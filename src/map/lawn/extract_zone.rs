//! 撤离区（撤，z -500..-200，北端）—— 玩家打完纵深后撤到信标完成循环。
//!
//! 包含撤离光垫、红色提取信标（柱 + 顶灯）、射击区/撤离区分界标线，
//! 以及两具"撤离途中仍需压制"的撤离目标，测试撤退阶段的战斗反馈。

use crate::map::{GlowKind, GlowSpec, MaterialKind, Prop, Shape, TargetSpec};

/// 射击区与撤离区分界标线（z=-200 白色虚线）
pub(crate) fn extract_boundary() -> Vec<Prop> {
    let mut v = Vec::new();
    for x in -12..=12 {
        v.push(Prop::decor([x as f32 * 40.0, 0.02, -200.0], [16.0, 0.02, 0.12], MaterialKind::PaintWhite));
    }
    v
}

/// 撤离光垫 + 红色提取信标（柱 + 顶灯）
pub(crate) fn extract_glow() -> Vec<GlowSpec> {
    vec![
        GlowSpec { shape: Shape::Box, pos: [0.0, 0.02, -440.0], half: [3.0, 0.04, 3.0], glow: GlowKind::SpawnPad },
        GlowSpec { shape: Shape::Box, pos: [0.0, 2.0, -420.0], half: [0.2, 2.0, 0.2], glow: GlowKind::Red },
        GlowSpec { shape: Shape::Box, pos: [0.0, 4.2, -420.0], half: [0.6, 0.15, 0.6], glow: GlowKind::Red },
    ]
}

/// 撤离目标：撤离途中侧翼压制
pub(crate) fn extract_targets() -> Vec<TargetSpec> {
    vec![
        TargetSpec { pos: [-150.0, 1.5, -320.0], label: "撤离目标", motion: None },
        TargetSpec { pos: [150.0, 1.5, -320.0], label: "撤离目标", motion: None },
    ]
}
