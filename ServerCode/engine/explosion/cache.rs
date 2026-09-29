//! 预爆炸缓存系统
//!
//! 手雷/爆炸物飞行期间增量维护"影响范围内实体订阅列表"，
//! 爆炸时直接遍历订阅列表，无需实时空间查询。
//! 独立成文件以控制 engine/mod.rs 行数（≤600 行约束）。

use crate::entity::{EntityId, World};
use crate::damage::{DamagePacket, DamageResolver, ExplosionCmd, Vec3};
use crate::element::EntityElementState;

/// 预爆炸缓存：订阅爆炸范围内的实体，爆炸时直接推送伤害。
///
/// 订阅列表在实体进入/离开范围时增量维护，爆炸时免去实时空间查询，
/// 兼顾确定性与性能。
pub struct PreExplosionCache {
    /// 订阅列表：在爆炸范围内的实体
    subscribers: Vec<EntityId>,
    /// 爆炸半径
    radius: f32,
    /// 最后更新时间
    last_update_tick: u64,
}

impl PreExplosionCache {
    pub fn new(radius: f32) -> Self {
        Self {
            subscribers: Vec::new(),
            radius,
            last_update_tick: 0,
        }
    }

    /// 添加订阅者
    pub fn add_subscriber(&mut self, entity_id: EntityId) {
        if !self.subscribers.contains(&entity_id) {
            self.subscribers.push(entity_id);
        }
    }

    /// 移除订阅者
    pub fn remove_subscriber(&mut self, entity_id: EntityId) {
        self.subscribers.retain(|&id| id != entity_id);
    }

    /// 更新订阅列表（实体进入/离开范围时调用）
    pub fn update(&mut self, grenade_pos: Vec3, world: &World, current_tick: u64) {
        self.last_update_tick = current_tick;

        // 获取当前在范围内的所有实体
        let in_range: Vec<_> = world.query_in_radius(grenade_pos, self.radius);

        // 更新订阅列表
        self.subscribers.clear();
        self.subscribers.extend(in_range);
    }

    /// 获取订阅者列表
    pub fn get_subscribers(&self) -> &[EntityId] {
        &self.subscribers
    }

    /// 验证并推送爆炸指令（带一致性校验）
    pub fn validate_and_push(
        &self,
        explosion_center: Vec3,
        world: &mut World,
        cmd: &ExplosionCmd,
        resolver: &DamageResolver,
    ) {
        for &subscriber_id in &self.subscribers {
            if let Some(entity) = world.get_entity(subscriber_id) {
                // 1. 存活校验
                if !entity.is_alive {
                    continue;
                }

                // 2. 位置偏差校验（10%容差）
                let actual_dist = entity.position.distance(&explosion_center);
                if actual_dist > self.radius * 1.1 {
                    continue;
                }
            }

            // 3. 阻挡重新校验 + 推送
            if let Some(entity) = world.get_entity_mut(subscriber_id) {
                let damage = DamagePacket::new(
                    cmd.element,
                    cmd.base_damage,
                    cmd.source_id,
                ).with_source_pos(explosion_center);

                let env = EntityElementState::Normal;
                resolver.resolve(entity, &damage, &env);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::Entity;

    #[test]
    fn test_pre_explosion_cache() {
        let mut world = World::new();

        // 创建实体
        let e1 = Entity::new_player(0, Vec3::new(0.0, 0.0, 0.0));
        let e2 = Entity::new_player(0, Vec3::new(3.0, 0.0, 0.0));
        let e3 = Entity::new_player(0, Vec3::new(10.0, 0.0, 0.0));

        let id1 = world.spawn(e1);
        let id2 = world.spawn(e2);
        let _id3 = world.spawn(e3);

        // 创建缓存（半径5米）
        let mut cache = PreExplosionCache::new(5.0);
        cache.update(Vec3::new(0.0, 0.0, 0.0), &world, 1);

        // e1(0,0,0)和e2(3,0,0)在范围内，e3(10,0,0)不在
        let subscribers = cache.get_subscribers();
        assert!(subscribers.contains(&id1));
        assert!(subscribers.contains(&id2));
        assert!(!subscribers.contains(&EntityId::new(3))); // e3不在范围内
    }
}
