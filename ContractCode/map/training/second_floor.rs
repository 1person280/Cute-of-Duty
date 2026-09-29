//! 二层立体结构 —— 东西两侧的二层回廊（贴墙），各带支撑立柱、登顶楼梯与护栏。
//!
//! 这是训练场垂直机动的另一半：指挥台（中央）在 cqb_hall 模块，东西回廊在此。
//! 台阶高差 ≤1.1m 保证可跳登上层（由父模块 vertical_structures_climbable 测试保证）。

use crate::map::{MaterialKind, Prop};

/// 东西二层回廊：平台 + 立柱 + 从大厅南端逐级登顶的楼梯 + 护栏。全轴对齐盒体（AABB 友好）
pub(crate) fn walkways() -> Vec<Prop> {
    let mut v = Vec::new();

    // ---- 东侧二层回廊（贴东墙，楼梯从大厅南端登顶）----
    v.push(Prop::solid([13.8, 2.5, 0.0], [0.8, 0.15, 4.0], MaterialKind::Steel));
    for z in [-3.6, -1.2, 1.2, 3.6] {
        v.push(Prop::solid([13.8, 1.25, z], [0.18, 1.25, 0.18], MaterialKind::Steel));
    }
    v.push(Prop::solid([13.8, 0.3, 5.2], [0.8, 0.3, 0.4], MaterialKind::Steel));
    v.push(Prop::solid([13.8, 1.0, 4.6], [0.8, 1.0, 0.4], MaterialKind::Steel));
    v.push(Prop::solid([13.8, 1.9, 4.0], [0.8, 1.9, 0.4], MaterialKind::Steel));
    v.push(Prop::decor([13.0, 3.0, 0.0], [0.05, 0.35, 4.0], MaterialKind::Steel));

    // ---- 西侧二层回廊（镜像）----
    v.push(Prop::solid([-13.8, 2.5, 0.0], [0.8, 0.15, 4.0], MaterialKind::Steel));
    for z in [-3.6, -1.2, 1.2, 3.6] {
        v.push(Prop::solid([-13.8, 1.25, z], [0.18, 1.25, 0.18], MaterialKind::Steel));
    }
    v.push(Prop::solid([-13.8, 0.3, 5.2], [0.8, 0.3, 0.4], MaterialKind::Steel));
    v.push(Prop::solid([-13.8, 1.0, 4.6], [0.8, 1.0, 0.4], MaterialKind::Steel));
    v.push(Prop::solid([-13.8, 1.9, 4.0], [0.8, 1.9, 0.4], MaterialKind::Steel));
    v.push(Prop::decor([-13.0, 3.0, 0.0], [0.05, 0.35, 4.0], MaterialKind::Steel));

    v
}