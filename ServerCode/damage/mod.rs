//! 伤害结算系统
//!
//! 伤害结算流水线：
//! Step 1: 元素修正 ──► 查 ElementMatrix
//! Step 2: 数值计算 ──► base_value × 元素倍率 × 防御减伤 × 环境修正
//! Step 3: 副作用附加 ──► 根据结果标签，给实体挂组件
//! Step 4: 生命扣除 ──► hp -= final_damage
//! Step 5: 事件广播 ──► 推送到渲染（播特效）、网络（同步状态）
//!
//! 模块被拆分到以下语义化子模块：
//! - `packet`  ：数据包/标签/命令（纯数据）
//! - `resolver`：伤害结算流水线
//! - `effect`  ：副作用组件

mod packet;
mod resolver;
mod effect;

pub use packet::*;
pub use resolver::*;
pub use effect::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use crate::element::{ElementConfig, ElementSystem, ElementType, EntityElementState};
    use crate::entity::{Component, Entity, EntityId};

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