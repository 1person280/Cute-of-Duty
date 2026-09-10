//! 伤害结算系统
//!
//! 伤害结算流水线：
//! Step 1: 元素修正 ──► 查 ElementMatrix
//! Step 2: 数值计算 ──► base_value × 元素倍率 × 防御减伤 × 环境修正
//! Step 3: 副作用附加 ──► 根据结果标签，给实体挂组件
//! Step 4: 生命扣除 ──► hp -= final_damage
//! Step 5: 事件广播 ──► 推送到渲染（播特效）、网络（同步状态）

use std::sync::Arc;
use crate::element::{ElementSystem, ElementType, EntityElementState, ReactionResult};
use crate::entity::{Entity, Component, EntityId};

/// 3D向量（简化版）
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// 计算两点距离
    pub fn distance(&self, other: &Vec3) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2) + (self.z - other.z).powi(2)).sqrt()
    }

    /// 线性插值
    pub fn lerp(&self, other: &Vec3, t: f32) -> Vec3 {
        Vec3 {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            z: self.z + (other.z - self.z) * t,
        }
    }
}

/// 纯数据伤害包
/// 
/// 关键：DamagePacket 是纯数据，零逻辑。
/// 它不携带"目标应该如何响应"的信息。
#[derive(Debug, Clone)]
pub struct DamagePacket {
    /// 攻击元素类型
    pub element: ElementType,
    /// 基础伤害值
    pub base_value: f32,
    /// 伤害源位置（用于计算距离衰减）
    pub source_pos: Vec3,
    /// 伤害源实体ID
    pub source_id: EntityId,
    /// 攻击自带的副作用标签
    pub inherent_effects: Vec<EffectTag>,
    /// 是否暴击
    pub is_critical: bool,
    /// 暴击倍率
    pub critical_multiplier: f32,
}

impl DamagePacket {
    /// 创建基础伤害包
    pub fn new(element: ElementType, base_value: f32, source_id: EntityId) -> Self {
        Self {
            element,
            base_value,
            source_pos: Vec3::default(),
            source_id,
            inherent_effects: Vec::new(),
            is_critical: false,
            critical_multiplier: 1.5,
        }
    }

    /// 设置源位置
    pub fn with_source_pos(mut self, pos: Vec3) -> Self {
        self.source_pos = pos;
        self
    }

    /// 添加副作用标签
    pub fn with_effect(mut self, effect: EffectTag) -> Self {
        self.inherent_effects.push(effect);
        self
    }

    /// 设置暴击
    pub fn with_critical(mut self, multiplier: f32) -> Self {
        self.is_critical = true;
        self.critical_multiplier = multiplier;
        self
    }
}

/// 效果标签
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectTag {
    Burning,           // 燃烧
    Frozen,            // 冰冻
    Poisoned,          // 中毒
    Wet,               // 潮湿
    Electrified,       // 感电
    Vaporize,          // 蒸发
    Melt,              // 融化
    ShatterFreeze,     // 爆裂冻结
    PhysicalVulnerability, // 物理易伤
    ArmorShatter,      // 护甲碎裂
    HealBlock,         // 治疗阻断
    Stun,              // 眩晕
}

/// 爆炸指令（预爆炸缓存推送用）
#[derive(Debug, Clone)]
pub struct ExplosionCmd {
    /// 元素类型
    pub element: ElementType,
    /// 基础伤害
    pub base_damage: f32,
    /// 冲击方向
    pub impulse_direction: Vec3,
    /// 冲量大小
    pub impulse_magnitude: f32,
    /// 爆炸中心
    pub explosion_center: Vec3,
    /// 来源实体ID
    pub source_id: EntityId,
    /// 爆炸半径
    pub radius: f32,
    /// 衰减曲线类型
    pub falloff: FalloffType,
}

/// 伤害衰减类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FalloffType {
    Linear,      // 线性衰减
    Quadratic,   // 二次衰减
    Constant,    // 恒定（无衰减）
}

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

// ========== 副作用组件实现 ==========

/// 燃烧组件
#[derive(Debug)]
pub struct BurningComponent {
    pub duration: f32,
    pub dps: f32,
    pub elapsed: f32,
}

impl BurningComponent {
    pub fn new(duration: f32, dps: f32) -> Self {
        Self { duration, dps, elapsed: 0.0 }
    }
}

impl Component for BurningComponent {
    fn on_tick(&mut self, owner: &mut Entity, delta_time: f32) {
        self.elapsed += delta_time;
        owner.hp -= self.dps * delta_time;
        
        if self.elapsed >= self.duration {
            owner.remove_component::<BurningComponent>();
        }
    }

    fn on_incoming_element(&mut self, element: ElementType, owner: &mut Entity) {
        // 水熄灭火
        if matches!(element, ElementType::Water) {
            owner.remove_component::<BurningComponent>();
            // 产生蒸汽效果（简化）
            owner.element_state = EntityElementState::Normal;
        }
    }

    fn name(&self) -> &'static str {
        "Burning"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// 冰冻组件
#[derive(Debug)]
pub struct FrozenComponent {
    pub duration: f32,
    pub elapsed: f32,
    pub original_speed: f32,
}

impl FrozenComponent {
    pub fn new(duration: f32) -> Self {
        Self { duration, elapsed: 0.0, original_speed: -1.0 }
    }
}

impl Component for FrozenComponent {
    fn on_apply(&mut self, owner: &mut Entity) {
        self.original_speed = owner.move_speed;
        owner.move_speed = 0.0; // 冻结时无法移动
    }

    fn on_remove(&mut self, owner: &mut Entity) {
        if self.original_speed >= 0.0 {
            owner.move_speed = self.original_speed;
        }
    }

    fn on_tick(&mut self, owner: &mut Entity, delta_time: f32) {
        self.elapsed += delta_time;
        if self.elapsed >= self.duration {
            owner.remove_component::<FrozenComponent>();
        }
    }

    fn on_incoming_element(&mut self, element: ElementType, owner: &mut Entity) {
        // 火解除冰冻并产生融化
        if matches!(element, ElementType::Fire) {
            owner.remove_component::<FrozenComponent>();
            owner.add_component(Box::new(MeltComponent::new()));
        }
    }

    fn name(&self) -> &'static str {
        "Frozen"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// 融化组件（临时状态）
#[derive(Debug)]
pub struct MeltComponent;

impl MeltComponent {
    pub fn new() -> Self {
        Self
    }
}

impl Component for MeltComponent {
    fn name(&self) -> &'static str {
        "Melt"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// 中毒组件
#[derive(Debug)]
pub struct PoisonedComponent {
    pub duration: f32,
    pub dps: f32,
    pub elapsed: f32,
}

impl PoisonedComponent {
    pub fn new(duration: f32, dps: f32) -> Self {
        Self { duration, dps, elapsed: 0.0 }
    }
}

impl Component for PoisonedComponent {
    fn on_tick(&mut self, owner: &mut Entity, delta_time: f32) {
        self.elapsed += delta_time;
        owner.hp -= self.dps * delta_time;
        owner.armor -= 1.0 * delta_time; // 腐蚀护甲
        
        if self.elapsed >= self.duration {
            owner.remove_component::<PoisonedComponent>();
        }
    }

    fn name(&self) -> &'static str {
        "Poisoned"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// 护甲碎裂组件
#[derive(Debug)]
pub struct ArmorShatterComponent {
    pub duration: f32,
    pub elapsed: f32,
}

impl ArmorShatterComponent {
    pub fn new(duration: f32) -> Self {
        Self { duration, elapsed: 0.0 }
    }
}

impl Component for ArmorShatterComponent {
    fn on_tick(&mut self, owner: &mut Entity, delta_time: f32) {
        self.elapsed += delta_time;
        owner.armor = owner.armor.max(0.0) * 0.5; // 护甲减半
        
        if self.elapsed >= self.duration {
            owner.remove_component::<ArmorShatterComponent>();
        }
    }

    fn name(&self) -> &'static str {
        "ArmorShatter"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// 物理易伤组件
#[derive(Debug)]
pub struct PhysicalVulnerabilityComponent {
    pub multiplier: f32,
    pub duration: f32,
    pub elapsed: f32,
}

impl PhysicalVulnerabilityComponent {
    pub fn new(multiplier: f32, duration: f32) -> Self {
        Self { multiplier, duration, elapsed: 0.0 }
    }
}

impl Component for PhysicalVulnerabilityComponent {
    fn on_tick(&mut self, owner: &mut Entity, delta_time: f32) {
        self.elapsed += delta_time;
        if self.elapsed >= self.duration {
            owner.remove_component::<PhysicalVulnerabilityComponent>();
        }
    }

    fn name(&self) -> &'static str {
        "PhysicalVulnerability"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// 感电组件
#[derive(Debug)]
pub struct ElectrifiedComponent {
    pub duration: f32,
    pub elapsed: f32,
}

impl ElectrifiedComponent {
    pub fn new(duration: f32) -> Self {
        Self { duration, elapsed: 0.0 }
    }
}

impl Component for ElectrifiedComponent {
    fn on_tick(&mut self, owner: &mut Entity, delta_time: f32) {
        self.elapsed += delta_time;
        // 感电期间受到电系伤害增加
        if self.elapsed >= self.duration {
            owner.remove_component::<ElectrifiedComponent>();
        }
    }

    fn name(&self) -> &'static str {
        "Electrified"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// 眩晕组件
#[derive(Debug)]
pub struct StunComponent {
    pub duration: f32,
    pub elapsed: f32,
    pub original_speed: f32,
}

impl StunComponent {
    pub fn new(duration: f32) -> Self {
        Self { duration, elapsed: 0.0, original_speed: -1.0 }
    }
}

impl Component for StunComponent {
    fn on_apply(&mut self, owner: &mut Entity) {
        self.original_speed = owner.move_speed;
        owner.move_speed = 0.0;
    }

    fn on_remove(&mut self, owner: &mut Entity) {
        if self.original_speed >= 0.0 {
            owner.move_speed = self.original_speed;
        }
    }

    fn on_tick(&mut self, owner: &mut Entity, delta_time: f32) {
        self.elapsed += delta_time;
        if self.elapsed >= self.duration {
            owner.remove_component::<StunComponent>();
        }
    }

    fn name(&self) -> &'static str {
        "Stun"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// 毒云组件（范围持续伤害）
#[derive(Debug)]
pub struct PoisonCloudComponent {
    pub radius: f32,
    pub duration: f32,
    pub dps: f32,
    pub elapsed: f32,
}

impl PoisonCloudComponent {
    pub fn new(radius: f32, duration: f32) -> Self {
        Self { radius, duration, dps: 15.0, elapsed: 0.0 }
    }
}

impl Component for PoisonCloudComponent {
    fn on_tick(&mut self, owner: &mut Entity, delta_time: f32) {
        self.elapsed += delta_time;
        if self.elapsed >= self.duration {
            owner.remove_component::<PoisonCloudComponent>();
        }
    }

    fn name(&self) -> &'static str {
        "PoisonCloud"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::ElementConfig;

    #[test]
    fn test_damage_calculation_basic() {
        let element_system = Arc::new(ElementSystem::new(ElementConfig::default()));
        let resolver = DamageResolver::new(element_system);

        let mut target = Entity::new(1, Vec3::new(0.0, 0.0, 0.0));
        target.hp = 100.0;
        target.armor = 50.0;

        let damage = DamagePacket::new(ElementType::Fire, 50.0, EntityId(2));

        let final_damage = resolver.resolve(&mut target, &damage, &EntityElementState::Normal);

        // 基础50伤害，无元素反应倍率1.0，防御减伤 100/(100+50) = 0.667
        // 期望伤害约 50 * 0.667 = 33.3
        assert!(final_damage > 0.0);
        assert!(target.hp < 100.0);
    }

    #[test]
    fn test_element_reaction_damage() {
        let element_system = Arc::new(ElementSystem::new(ElementConfig::default()));
        let resolver = DamageResolver::new(element_system);

        let mut target = Entity::new(1, Vec3::new(0.0, 0.0, 0.0));
        target.hp = 200.0;
        target.armor = 0.0;
        target.element_state = EntityElementState::Wet;

        // 潮湿目标受到火攻击 = 蒸发 (1.5倍)
        let damage = DamagePacket::new(ElementType::Fire, 100.0, EntityId(2));
        let final_damage = resolver.resolve(&mut target, &damage, &EntityElementState::Normal);

        // 期望: 100 * 1.5 (蒸发倍率) * 1.0 (无防御) = 150
        assert!((final_damage - 150.0).abs() < 1.0);
    }

    #[test]
    fn test_burning_component() {
        let mut entity = Entity::new(1, Vec3::new(0.0, 0.0, 0.0));
        entity.hp = 100.0;

        let mut burning = BurningComponent::new(2.0, 10.0); // 2秒，每秒10伤害
        
        // 模拟2秒Tick
        burning.on_tick(&mut entity, 1.0);
        assert_eq!(entity.hp, 90.0);
        
        burning.on_tick(&mut entity, 1.0);
        assert_eq!(entity.hp, 80.0);
    }

    #[test]
    fn test_frozen_movement() {
        let mut entity = Entity::new(1, Vec3::new(0.0, 0.0, 0.0));
        entity.move_speed = 5.0;

        let mut frozen = FrozenComponent::new(3.0);
        frozen.on_apply(&mut entity);
        assert_eq!(entity.move_speed, 0.0);

        frozen.on_remove(&mut entity);
        assert_eq!(entity.move_speed, 5.0);
    }
}
