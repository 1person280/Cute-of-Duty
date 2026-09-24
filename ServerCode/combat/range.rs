//! 训练靶（实弹射击靶机）—— 服务的权威半边
//!
//! 架构约定：靶位与运动参数来自 `crate::map::*::layout().targets`（纯数据），
//! 本模块负责把 `TargetSpec` 实体化进权威世界、驱动移动靶运动，并累计命中计分。
//! 靶机 `hp` 为最大值（见 `Entity::new_target`），命中只计分、绝不致死，因此
//! 常规的死亡清理不会误删它——这是与 AI 敌人最本质的差别。
//!
//! 命中判定（射线对靶面球体求交）由 `shooter::try_fire` 统一完成；这里只在一个
//! 训练靶被射中时被回调 `record_hit` 累加次数。

use crate::damage::Vec3;
use crate::entity::{Component, Entity, EntityId, World};
use std::any::Any;

/// 往返运动参数（X 轴向 `center.x ± range` 来回）
#[derive(Clone, Copy, Debug)]
pub struct TargetMotion {
    pub speed: f32,
    pub range: f32,
    pub start_dir: f32,
}

/// 训练靶抗打击半径（靶面 1.2×1.2 取外接球约 0.7，留一点命中宽容）
pub const TARGET_HIT_RADIUS: f32 = 0.7;

/// 训练靶运行时：标签、累计命中数、可选往返运动
///
/// 运动实现在 [`Component::on_tick`]，位置写回 `owner`——和 AI 巡逻同构，
/// 服务端权威推进后经快照回传，客户端只画。
pub struct RangeTarget {
    /// 击杀/命中播报中展示的名称（如“近距靶”）
    pub label: &'static str,
    /// 累计命中次数（HUD 计分）
    pub hits: u32,
    /// 往返运动；None = 静态靶
    pub motion: Option<TargetMotion>,
    /// 移动靶当前 x 相位中心/方向（内部分运动状态）
    pub base_x: f32,
    pub dir: f32,
    pub phase_x: f32,
}

impl RangeTarget {
    pub fn static_target(label: &'static str) -> Self {
        Self {
            label,
            hits: 0,
            motion: None,
            base_x: 0.0,
            dir: 1.0,
            phase_x: 0.0,
        }
    }

    pub fn moving_target(label: &'static str, motion: TargetMotion) -> Self {
        Self {
            label,
            hits: 0,
            motion: Some(motion),
            base_x: 0.0,
            dir: motion.start_dir,
            phase_x: 0.0,
        }
    }
}

impl Component for RangeTarget {
    /// 移动靶：每 Tick 沿 X 轴向往返扫掠，超出范围反向。
    fn on_tick(&mut self, owner: &mut Entity, delta_time: f32) {
        let Some(m) = self.motion else { return };
        self.phase_x += self.dir * m.speed * delta_time;
        if self.phase_x >= m.range {
            self.phase_x = m.range;
            self.dir = -1.0;
        } else if self.phase_x <= -m.range {
            self.phase_x = -m.range;
            self.dir = 1.0;
        }
        owner.position.x = self.base_x + self.phase_x;
    }

    fn name(&self) -> &'static str {
        "RangeTarget"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// 生成一个训练靶实体并塞入世界。
///
/// `center_x` 同时写入实体初始位置与 `RangeTarget::base_x`，移动靶围绕它往返。
pub fn spawn_target(
    world: &mut World,
    pos: Vec3,
    label: &'static str,
    motion: Option<TargetMotion>,
) -> EntityId {
    let mut entity = Entity::new_target(0, pos);
    let mut rt = match motion {
        Some(m) => RangeTarget::moving_target(label, m),
        None => RangeTarget::static_target(label),
    };
    rt.base_x = pos.x;
    entity.add_component(Box::new(rt));
    world.spawn(entity)
}

/// 依据地图布局 `.targets` 批量生成训练靶。
///
/// 场景进场时调用一次；`MapLayout::targets` 里的运动参数从 `map::Motion` 转换而来。
pub fn spawn_range_targets(world: &mut World, targets: &[crate::map::TargetSpec]) -> Vec<EntityId> {
    let mut ids = Vec::with_capacity(targets.len());
    for t in targets {
        let motion = t.motion.map(|m| TargetMotion {
            speed: m.speed,
            range: m.range,
            start_dir: m.start_dir,
        });
        let id = spawn_target(
            world,
            Vec3::new(t.pos[0], t.pos[1], t.pos[2]),
            t.label,
            motion,
        );
        ids.push(id);
    }
    ids
}

/// 训练靶“命中一次”：累加计分，返回新累计次数（供测试断言）。
pub fn record_hit(world: &mut World, id: EntityId) -> u32 {
    let Some(rt) = world
        .get_entity_mut(id)
        .and_then(|e| e.get_component_mut::<RangeTarget>())
    else {
        return 0;
    };
    rt.hits += 1;
    rt.hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityType;
    use crate::combat::range::{record_hit, spawn_range_targets, spawn_target, TargetMotion};

    #[test]
    fn spawns_and_tags_target() {
        let mut world = World::new();
        let id = spawn_target(&mut world, Vec3::new(0.0, 1.5, -8.0), "近距靶", None);
        let e = world.get_entity(id).unwrap();
        assert_eq!(e.entity_type, EntityType::Target);
        assert!(e.is_alive, "靶机必须恒存活");
        assert_eq!(e.get_component::<RangeTarget>().unwrap().label, "近距靶");
    }

    #[test]
    fn record_hit_increments_score() {
        let mut world = World::new();
        let id = spawn_target(&mut world, Vec3::new(0.0, 1.5, -8.0), "近距靶", None);
        assert_eq!(record_hit(&mut world, id), 1);
        assert_eq!(record_hit(&mut world, id), 2);
    }

    #[test]
    fn moving_target_oscillates_within_range() {
        let mut world = World::new();
        let id = spawn_target(
            &mut world,
            Vec3::new(6.5, 1.8, -9.5),
            "移动靶",
            Some(TargetMotion { speed: 3.0, range: 3.0, start_dir: -1.0 }),
        );
        let start_x = world.get_entity(id).unwrap().position.x;
        // 推进半秒：应朝 -X 移动，但不超过初值（range=3 内）
        for _ in 0..30 {
            world.tick_all_entities(1.0 / 60.0);
        }
        let x = world.get_entity(id).unwrap().position.x;
        assert!(x < start_x, "移动靶应向 -X 运动，实际起点 {start_x} 现{x}");
        assert!((start_x - x) <= 3.0 + 1e-3, "移动靶扫掠不得越界");
    }

    #[test]
    fn static_target_does_not_migrate() {
        let mut world = World::new();
        let id = spawn_target(&mut world, Vec3::new(9.0, 2.2, -14.3), "远距靶", None);
        for _ in 0..30 {
            world.tick_all_entities(1.0 / 60.0);
        }
        let e = world.get_entity(id).unwrap();
        assert_eq!(e.position.x, 9.0);
        assert_eq!(e.position.z, -14.3);
        assert!(e.is_alive);
    }

    #[test]
    fn spawn_range_targets_from_layout_supports_motion() {
        let layout = crate::map::training::layout();
        let mut world = World::new();
        let ids = spawn_range_targets(&mut world, &layout.targets);
        assert_eq!(ids.len(), layout.targets.len());
        // 至少有一个移动靶与一个静态靶被正确实体化
        let movers = ids.iter().filter(|&&id| {
            let e = world.get_entity(id).unwrap();
            e.get_component::<RangeTarget>().unwrap().motion.is_some()
        }).count();
        let statics = ids.iter().filter(|&&id| {
            let e = world.get_entity(id).unwrap();
            e.get_component::<RangeTarget>().unwrap().motion.is_none()
        }).count();
        assert!(movers >= 2 && statics >= 3, "移动靶 {movers}、静态靶 {statics} 分布异常");
    }
}