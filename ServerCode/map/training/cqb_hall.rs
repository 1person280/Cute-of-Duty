//! 中央 CQB 大厅（z -6..6）—— 三巷道隔断、跪姿矮墙阵、中央指挥台、
//! 大厅功能台、沙袋堆，以及大厅移动靶与纵深补给。
//!
//! 这是近距离战斗练习的主场地，布局约束（射击道畅通、掩体不与靶相交、
//! 高台可达）由父模块单元测试保证。WALL_H 由父模块提供。

use crate::map::{
    MaterialKind, Motion, PickupKind, PickupSpec, Prop, TargetSpec,
};

use super::WALL_H;

/// 大厅巷道隔断（x=±5.5, z -3..3，中央错位开口形成三巷道走位）
pub(crate) fn hall_partitions() -> Vec<Prop> {
    let mut v = Vec::new();
    for x in [-5.5, 5.5] {
        v.push(Prop::solid([x, WALL_H / 2.0, -2.0], [0.3, WALL_H / 2.0, 1.0], MaterialKind::Concrete));
        v.push(Prop::solid([x, WALL_H / 2.0, 1.6], [0.3, WALL_H / 2.0, 1.4], MaterialKind::Concrete));
    }
    v
}

/// 大厅掩体：左右对称的跪姿矮墙列 + 大厅沙袋堆，全部落在大厅与巷道里（净空由测试保证）
pub(crate) fn hall_cover() -> Vec<Prop> {
    let mut v = Vec::new();
    // 左右对称：跪姿射击矮墙（各 3 块，练依托射击）
    for x in [-5.5, 5.5] {
        for z in [-3.9, -4.7, -5.5] {
            v.push(Prop::solid([x, 0.6, z], [0.4, 0.6, 0.4], MaterialKind::Concrete));
        }
    }
    // 大厅中央两侧：沙袋堆（双箱错位叠放，避开 ±3 射击道）
    for sx in [-4.4, 4.4] {
        v.push(Prop::solid([sx, 0.45, 3.0], [0.8, 0.45, 0.5], MaterialKind::Rust));
        v.push(Prop::solid([sx * 1.09, 1.1, 3.0], [0.55, 0.3, 0.45], MaterialKind::Rust));
    }
    v
}

/// 中央指挥台：二层平台 + 四立柱 + 南侧四级楼梯 + 二层护栏（练垂直仰角射击）
pub(crate) fn command_platform() -> Vec<Prop> {
    let mut v = Vec::new();
    // 二层平台（x=0, z=0，落位在 ±3 射击道之间的空当）
    v.push(Prop::solid([0.0, 2.5, 0.0], [2.2, 0.15, 1.8], MaterialKind::Concrete));
    // 支撑立柱
    for (x, z) in [(-1.8, -1.4), (1.8, -1.4), (-1.8, 1.4), (1.8, 1.4)] {
        v.push(Prop::solid([x, 1.25, z], [0.2, 1.25, 0.2], MaterialKind::Concrete));
    }
    // 南侧四级楼梯（逐级可跳，0.53/1.05/1.58/2.1）
    for (i, z) in [2.6, 3.1, 3.6, 4.1].iter().enumerate() {
        let step_y = 0.2625 + (i as f32) * 0.525;
        v.push(Prop::solid([0.0, step_y, *z], [0.8, 0.2625, 0.25], MaterialKind::Concrete));
    }
    // 二层护栏（decor：不挡子弹）
    v.push(Prop::decor([-2.2, 3.0, 0.0], [0.05, 0.35, 1.8], MaterialKind::Steel));
    v.push(Prop::decor([2.2, 3.0, 0.0], [0.05, 0.35, 1.8], MaterialKind::Steel));
    v.push(Prop::decor([0.0, 3.0, -1.8], [2.2, 0.35, 0.05], MaterialKind::Steel));
    v
}

/// 大厅两张功能台（z=4.5，与回廊楼梯错位）
pub(crate) fn hall_tables() -> Vec<Prop> {
    let mut v = Vec::new();
    for x in [-6.5, 6.5] {
        v.push(Prop::solid([x, 0.72, 4.5], [1.1, 0.06, 0.55], MaterialKind::Steel));
        for (dx, dz) in [(-0.9, -0.4), (0.9, -0.4), (-0.9, 0.4), (0.9, 0.4)] {
            v.push(Prop::solid([x + dx, 0.33, 4.5 + dz], [0.06, 0.33, 0.06], MaterialKind::Dark));
        }
    }
    v
}

/// 大厅移动靶：北侧开阔带（巷道隔墙与跪姿矮墙之间）慢速横移
pub(crate) fn hall_mover() -> Vec<TargetSpec> {
    vec![
        TargetSpec {
            pos: [-2.5, 1.5, -4.0],
            label: "移动靶",
            motion: Some(Motion { speed: 2.0, range: 2.0, start_dir: 1.0 }),
        },
    ]
}

/// 大厅纵深补给：左右镜像一对（弹药箱 / 大医疗包）
pub(crate) fn hall_supply() -> Vec<PickupSpec> {
    vec![
        PickupSpec { pos: [-9.0, 0.4, -2.0], label: "弹药箱", kind: PickupKind::Ammo { amount: 60 } },
        PickupSpec { pos: [9.0, 0.4, -2.0], label: "大医疗包", kind: PickupKind::Health { amount: 50.0 } },
    ]
}