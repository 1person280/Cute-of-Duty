//! 伤害结算流水线
//!
//! `DamageResolver` 是伤害结算器的核心：
//! 内部持有 `Arc<ElementSystem>`，执行完整的分步结算
//! （元素修正 → 环境修正 → 距离衰减 → 防御减伤 → 暴击 → 副作用 → 生命扣除），
//! 并支持爆炸指令的批量处理。

use std::sync::Arc;
use crate::element::{ElementSystem, ElementType, EntityElementState, ReactionResult};
use crate::entity::Entity;
use super::*;

/// 伤害结算器
///
/// 内部持有Arc<ElementSystem>，克隆开销极小，
/// 可以被GameLoop等系统长期持有并按需克隆分发。
#[derive(Clone)]
pub struct DamageResolver {
    element_system: Arc<ElementSystem>,
}

impl DamageResolver {
    pub fn new(element_system: Arc<ElementSystem>) -> Self {
        Self { element_system }
    }

    /// 执行完整伤害结算流水线
    ///
    /// # 参数
    /// - `target`: 目标实体
    /// - `damage`: 伤害数据包
    /// - `environment`: 当前环境状态
    ///
    /// # 返回
    /// 最终伤害数值（已应用所有修正）
    pub fn resolve(&self, target: &mut Entity, damage: &DamagePacket, environment: &EntityElementState) -> f32 {
        // Step 1: 元素修正
        let element_multiplier = self.element_system.calculate_damage_multiplier(
            &target.element_state,
            &damage.element,
        );

        // Step 2: 环境修正
        let env_multiplier = self.element_system.get_environment_modifier(environment, &damage.element);

        // Step 3: 距离衰减
        let dist = damage.source_pos.distance(&target.position);
        let dist_multiplier = self.calculate_distance_falloff(dist, 50.0, FalloffType::Linear); // 50米衰减距离

        // Step 4: 防御减伤
        let defense_multiplier = self.calculate_defense_reduction(target);

        // Step 5: 暴击修正
        let crit_multiplier = if damage.is_critical { damage.critical_multiplier } else { 1.0 };

        // Step 6: 副作用计算（队友互斥等）
        let side_effect_multiplier = self.calculate_side_effects(target, &damage.element);

        // 最终伤害计算
        let final_damage = damage.base_value
            * element_multiplier
            * env_multiplier
            * dist_multiplier
            * defense_multiplier
            * crit_multiplier
            * side_effect_multiplier;

        // Step 7: 副作用附加（根据元素反应结果）
        if let Some(reaction) = self.element_system.query_reaction(&target.element_state, &damage.element) {
            for effect in &reaction.attach_effects {
                self.apply_reaction_effect(target, effect);
            }
            
            // 更新目标元素状态
            self.update_element_state(target, &reaction.result);
        }

        // Step 8: 附加固有副作用
        for effect in &damage.inherent_effects {
            self.apply_effect_tag(target, *effect);
        }

        // Step 9: 生命扣除
        target.hp -= final_damage;
        if target.hp < 0.0 {
            target.hp = 0.0;
        }

        final_damage
    }

    /// 计算距离衰减
    fn calculate_distance_falloff(&self, distance: f32, max_range: f32, falloff: FalloffType) -> f32 {
        if distance >= max_range {
            return 0.0;
        }

        let t = distance / max_range;

        match falloff {
            FalloffType::Constant => 1.0,
            FalloffType::Linear => 1.0 - t,
            FalloffType::Quadratic => 1.0 - t * t,
        }
    }

    /// 计算防御减伤
    fn calculate_defense_reduction(&self, target: &Entity) -> f32 {
        // 基础减伤公式: 100 / (100 + armor)
        let armor = target.armor;
        100.0 / (100.0 + armor)
    }

    /// 计算副作用乘数（队友互斥等）
    fn calculate_side_effects(&self, _target: &Entity, _element: &ElementType) -> f32 {
        // 简化的副作用计算
        // 实际实现应考虑目标周围队友的元素配置
        1.0
    }

    /// 应用反应效果到实体
    fn apply_reaction_effect(&self, target: &mut Entity, effect: &ReactionResult) {
        match effect {
            ReactionResult::Burning => {
                target.add_component(Box::new(BurningComponent::new(5.0, 10.0)));
            }
            ReactionResult::Melt => {
                // 融化立即造成额外伤害已在倍率中体现
                target.add_component(Box::new(MeltComponent::new()));
            }
            ReactionResult::Vaporize => {
                // 蒸发效果已在倍率中体现
            }
            ReactionResult::ShatterFreeze => {
                target.add_component(Box::new(ArmorShatterComponent::new(8.0)));
                target.add_component(Box::new(PhysicalVulnerabilityComponent::new(2.0, 5.0)));
            }
            ReactionResult::Electrolysis => {
                target.add_component(Box::new(ElectrifiedComponent::new(3.0)));
            }
            ReactionResult::PoisonCloud => {
                target.add_component(Box::new(PoisonCloudComponent::new(5.0, 8.0)));
            }
            _ => {}
        }
    }

    /// 应用效果标签
    fn apply_effect_tag(&self, target: &mut Entity, tag: EffectTag) {
        match tag {
            EffectTag::Burning => {
                target.add_component(Box::new(BurningComponent::new(5.0, 10.0)));
            }
            EffectTag::Frozen => {
                target.add_component(Box::new(FrozenComponent::new(3.0)));
            }
            EffectTag::Poisoned => {
                target.add_component(Box::new(PoisonedComponent::new(10.0, 5.0)));
            }
            EffectTag::Stun => {
                target.add_component(Box::new(StunComponent::new(1.5)));
            }
            _ => {}
        }
    }

    /// 更新实体元素状态
    fn update_element_state(&self, target: &mut Entity, reaction_result: &ReactionResult) {
        let new_state = match reaction_result {
            ReactionResult::Burning => Some(EntityElementState::Burning),
            ReactionResult::Melt => Some(EntityElementState::Normal),
            ReactionResult::Vaporize => Some(EntityElementState::Normal),
            ReactionResult::ShatterFreeze => Some(EntityElementState::Frozen),
            ReactionResult::Electrolysis => Some(EntityElementState::Normal),
            ReactionResult::Superconduct => Some(EntityElementState::Normal),
            _ => None,
        };

        if let Some(state) = new_state {
            target.element_state = state;
        }
    }

    /// 处理爆炸指令（预爆炸缓存推送）
    pub fn process_explosion(&self, explosion: &ExplosionCmd, targets: &mut [Entity]) {
        for target in targets.iter_mut() {
            let dist = explosion.explosion_center.distance(&target.position);
            
            if dist > explosion.radius * 1.1 {
                continue; // 超出范围+10%容差
            }

            // 重新校验阻挡
            let blocked = self.check_line_of_sight(&explosion.explosion_center, &target.position);
            let damage_multiplier = if blocked { 0.5 } else { 1.0 };

            // 距离衰减
            let falloff = self.calculate_distance_falloff(dist, explosion.radius, explosion.falloff);
            
            let damage = DamagePacket::new(
                explosion.element,
                explosion.base_damage * falloff * damage_multiplier,
                explosion.source_id,
            ).with_source_pos(explosion.explosion_center);

            // 获取环境状态（简化：使用目标当前元素状态作为环境）
            let env = EntityElementState::Normal;
            self.resolve(target, &damage, &env);
        }
    }

    /// 简化的视线检查
    fn check_line_of_sight(&self, _from: &Vec3, _to: &Vec3) -> bool {
        // 实际实现应使用射线检测
        false
    }
}