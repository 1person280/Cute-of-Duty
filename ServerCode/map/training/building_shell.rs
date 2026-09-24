//! 建筑外壳 —— 四面封闭围墙与立柱、天花板横梁阵列、南墙观察窗、室内灯带。
//!
//! 属于 CQB 室内训练场的"容器"层：把整块 30×30m 场地围成封闭房间，
//! 并提供室内氛围所需的梁柱结构。相对坐标常量（HALF）由父模块提供。

use crate::map::{
    GlowKind, GlowSpec, MaterialKind, Prop, Shape,
};

use super::HALF;

/// 室内四壁：全封闭（区别于旧版露天三面墙），立柱沿四壁每 5m 一根（框架结构感）
pub(crate) fn perimeter() -> Vec<Prop> {
    let mut v = vec![
        Prop::solid([0.0, 1.5, -HALF], [HALF + 0.5, 1.5, 0.15], MaterialKind::Steel),
        Prop::solid([0.0, 1.5, HALF], [HALF + 0.5, 1.5, 0.15], MaterialKind::Steel),
        Prop::solid([-HALF, 1.5, 0.0], [0.15, 1.5, HALF + 0.5], MaterialKind::Steel),
        Prop::solid([HALF, 1.5, 0.0], [0.15, 1.5, HALF + 0.5], MaterialKind::Steel),
    ];
    for x in [-15, -10, -5, 0, 5, 10, 15] {
        v.push(Prop::solid([x as f32, 1.5, -HALF], [0.25, 1.5, 0.25], MaterialKind::Steel));
        v.push(Prop::solid([x as f32, 1.5, HALF], [0.25, 1.5, 0.25], MaterialKind::Steel));
    }
    for z in [-10, -5, 0, 5, 10] {
        v.push(Prop::solid([-HALF, 1.5, z as f32], [0.25, 1.5, 0.25], MaterialKind::Steel));
        v.push(Prop::solid([HALF, 1.5, z as f32], [0.25, 1.5, 0.25], MaterialKind::Steel));
    }
    v
}

/// 外壳装饰：天花板横梁阵列（东西向，每 4m 一根）+ 南墙观察窗（窗框 + 白色玻璃面）
pub(crate) fn shell_decor() -> Vec<Prop> {
    let mut v = Vec::new();
    // 天花板横梁阵列（顶不封保持采光，梁柱撑起室内感）
    for z in [-12, -8, -4, 0, 4, 8, 12] {
        v.push(Prop::decor([0.0, 4.6, z as f32], [HALF, 0.15, 0.2], MaterialKind::Steel));
    }
    // 南墙观察窗（decor 窗框 + 白色玻璃面，各两扇）
    for x in [-11.0, -8.0, 8.0, 11.0] {
        v.push(Prop::decor([x, 1.8, HALF - 0.28], [0.9, 0.9, 0.08], MaterialKind::PaintWhite));
        v.push(Prop::decor([x, 1.8, HALF - 0.22], [1.0, 1.0, 0.05], MaterialKind::Dark));
    }
    v
}

/// 室内灯带：天花板梁下绿色灯带 + 侧壁橙色灯带（硬光氛围）
pub(crate) fn neon() -> Vec<GlowSpec> {
    let mut v = Vec::new();
    // 天花板灯带（沿横梁下沿，每 4m 一条）
    for z in [-12, -8, -4, 0, 4, 8, 12] {
        v.push(GlowSpec {
            shape: Shape::Box,
            pos: [0.0, 4.4, z as f32],
            half: [HALF - 0.5, 0.05, 0.08],
            glow: GlowKind::Green,
        });
    }
    // 东西侧壁灯带（两段，避开回廊）
    for x in [-14.8, 14.8] {
        for z in [-10.0, 10.0] {
            v.push(GlowSpec {
                shape: Shape::Box,
                pos: [x, 2.4, z],
                half: [0.06, 0.06, 1.8],
                glow: GlowKind::Orange,
            });
        }
    }
    v
}