//! 北侧射击馆（z -15..-6）—— 拱形隔墙三开口衔接大厅、4 条射击道
//! （x = ±3 / ±9）、5/10/15 米标线 + 分层靶位 + 移动靶 + 纵深手雷。
//!
//! 射击道中心线（LANES）与净空/畅通性由父模块测试保证，改靶位/掩体前先跑测试。

use crate::element::ElementType;
use crate::map::{
    MaterialKind, Motion, PickupKind, PickupSpec, Prop, TargetSpec,
};

use super::WALL_H;

/// 射击馆拱形隔墙（z=-6，三条 3.6m 开口对齐中央三巷道）+ 开口门楣 + 墙根踢脚线
pub(crate) fn range_arch() -> Vec<Prop> {
    let mut v = Vec::new();
    // 拱墙墙段中心/半长：[-12.6,2.4] [-6,1.8] [0,1.8] [6,1.8] [12.6,2.4]
    for (cx, hx) in [(-12.6, 2.4), (-6.0, 1.8), (0.0, 1.8), (6.0, 1.8), (12.6, 2.4)] {
        v.push(Prop::solid([cx, WALL_H / 2.0, -6.0], [hx, WALL_H / 2.0, 0.15], MaterialKind::Concrete));
    }
    // 拱墙开口门楣（decor）
    for cx in [-9.0, -3.0, 3.0, 9.0] {
        v.push(Prop::decor([cx, 2.8, -6.0], [1.6, 0.25, 0.2], MaterialKind::Dark));
    }
    // 拱墙北侧墙根踢脚线（decor 收边）
    for (pos, half) in [
        ([-6.0, 0.06, -6.12], [1.8, 0.06, 0.04]),
        ([0.0, 0.06, -6.12], [1.8, 0.06, 0.04]),
        ([6.0, 0.06, -6.12], [1.8, 0.06, 0.04]),
    ] {
        v.push(Prop::decor(pos, half, MaterialKind::Dark));
    }
    v
}

/// 距离标线（虚线）与标牌（出生点 [0,0,10.5] 起算：约 18.5 / 21.5 / 24.8m，
/// 即射击馆内三段纵深）
pub(crate) fn lane_markers() -> Vec<Prop> {
    let mut v = Vec::new();
    let lines = [
        (-8.0, MaterialKind::WarningYellow),
        (-11.0, MaterialKind::WarningOrange),
        (-14.3, MaterialKind::WarningRed),
    ];
    for (z, mat) in lines {
        for i in -4..=4 {
            v.push(Prop::decor([i as f32 * 1.5, -0.1, z], [0.45, 0.11, 0.075], mat));
        }
    }
    // 标牌（立柱 + 白板），贴拱墙北侧一列
    for z in [-8.0, -11.0, -14.3] {
        v.push(Prop::decor([-4.4, 0.9, z], [0.075, 0.9, 0.075], MaterialKind::Steel));
        v.push(Prop::decor([-4.4, 1.5, z + 0.1], [0.4, 0.25, 0.04], MaterialKind::PaintWhite));
    }
    v
}

/// 静态靶：随距离升高，内外侧道分层布置 + 台顶/回廊垂直仰角靶
pub(crate) fn static_targets() -> Vec<TargetSpec> {
    vec![
        // 近线：内侧双靶（大厅拱门内侧可直射）
        TargetSpec { pos: [-3.0, 1.5, -8.0], label: "近距靶", motion: None },
        TargetSpec { pos: [3.0, 1.5, -8.0], label: "近距靶", motion: None },
        // 中线：外侧双靶
        TargetSpec { pos: [-9.0, 1.8, -11.0], label: "中距靶", motion: None },
        TargetSpec { pos: [9.0, 1.8, -11.0], label: "中距靶", motion: None },
        // 远线：外侧双靶（贴后墙净空）
        TargetSpec { pos: [-9.0, 2.2, -14.3], label: "远距靶", motion: None },
        TargetSpec { pos: [9.0, 2.2, -14.3], label: "远距靶", motion: None },
        // 指挥台顶靶：练垂直仰角射击
        TargetSpec { pos: [0.0, 3.4, 0.0], label: "台顶靶", motion: None },
        // 回廊靶：东侧二层回廊上层位
        TargetSpec { pos: [13.0, 3.4, -2.0], label: "回廊靶", motion: None },
    ]
}

/// 射击馆移动靶：中段快速横移 + 远端中速巡逻（扫掠范围避开全部掩体，由测试保证）
pub(crate) fn range_movers() -> Vec<TargetSpec> {
    vec![
        // 射击馆中段快速横移
        TargetSpec {
            pos: [6.5, 1.8, -9.5],
            label: "移动靶",
            motion: Some(Motion { speed: 3.0, range: 3.0, start_dir: -1.0 }),
        },
        // 射击馆远端中速巡逻
        TargetSpec {
            pos: [-9.0, 1.5, -13.0],
            label: "移动靶",
            motion: Some(Motion { speed: 2.5, range: 2.5, start_dir: 1.0 }),
        },
    ]
}

/// 射击馆纵深手雷：左右镜像一对（毒素 / 雷电，补足四系元素）
pub(crate) fn range_supply() -> Vec<PickupSpec> {
    vec![
        PickupSpec {
            pos: [-8.0, 0.4, -4.5],
            label: "毒素手雷",
            kind: PickupKind::Grenade { element: ElementType::Poison },
        },
        PickupSpec {
            pos: [8.0, 0.4, -4.5],
            label: "雷电手雷",
            kind: PickupKind::Grenade { element: ElementType::Electric },
        },
    ]
}