//! 出生区（z 440..500，南端）—— 玩家落地即到的安全区与补给枢纽。
//!
//! 包含出生光垫、出生/搜索区边界线、补给横排（弹药/医疗/护甲/四系手雷/
//! 双武器）、两张功能台及对应站点与光条。玩家自 [0,0,470] 面向 -z 起步，
//! 先向左近探搜索区，再进入射击区，最终撤到北端信标。

use crate::element::ElementType;
use crate::map::{
    GlowKind, GlowSpec, MaterialKind, PickupKind, PickupSpec, Prop, Shape, StationKind, StationSpec,
};

use super::{SPAWN_Z, SUPPLY_LINE_Z};

/// 出生区与搜索区分界标线（z=440 白色虚线）：过了这条线即进入"搜"阶段。
pub(crate) fn spawn_decor() -> Vec<Prop> {
    let mut v = Vec::new();
    for x in -12..=12 {
        v.push(Prop::decor([x as f32 * 20.0, 0.02, 440.0], [8.0, 0.02, 0.12], MaterialKind::PaintWhite));
    }
    v
}

/// 出生区两张功能台：台面（实心可碰撞）+ 四条桌腿，对齐补给线
pub(crate) fn spawn_tables() -> Vec<Prop> {
    let mut v = Vec::new();
    for (x, z) in [(-5.0, SUPPLY_LINE_Z), (5.0, SUPPLY_LINE_Z)] {
        v.push(Prop::solid([x, 0.72, z], [1.1, 0.06, 0.55], MaterialKind::Steel));
        for (dx, dz) in [(-0.9, -0.4), (0.9, -0.4), (-0.9, 0.4), (0.9, 0.4)] {
            v.push(Prop::solid([x + dx, 0.33, z + dz], [0.06, 0.33, 0.06], MaterialKind::Dark));
        }
    }
    v
}

/// 出生区补给横排：弹药/医疗/护甲/四系手雷/双武器，对齐补给线
pub(crate) fn supply_line() -> Vec<PickupSpec> {
    vec![
        PickupSpec {
            pos: [-2.5, 0.4, SUPPLY_LINE_Z],
            label: "冰霜手雷",
            kind: PickupKind::Grenade { element: ElementType::Ice },
        },
        PickupSpec { pos: [-1.0, 0.4, SUPPLY_LINE_Z], label: "步枪弹药", kind: PickupKind::Ammo { amount: 30 } },
        PickupSpec { pos: [0.0, 0.4, SUPPLY_LINE_Z], label: "医疗包", kind: PickupKind::Health { amount: 25.0 } },
        PickupSpec { pos: [1.0, 0.4, SUPPLY_LINE_Z], label: "护甲片", kind: PickupKind::Armor { amount: 20.0 } },
        PickupSpec {
            pos: [2.5, 0.4, SUPPLY_LINE_Z],
            label: "烈焰手雷",
            kind: PickupKind::Grenade { element: ElementType::Fire },
        },
        PickupSpec {
            pos: [-10.0, 0.4, SUPPLY_LINE_Z],
            label: "毒液步枪",
            kind: PickupKind::Weapon { element: ElementType::Poison },
        },
        PickupSpec {
            pos: [10.0, 0.4, SUPPLY_LINE_Z],
            label: "雷电步枪",
            kind: PickupKind::Weapon { element: ElementType::Electric },
        },
    ]
}

/// 出生点前方可交互物资箱的世界坐标（玩家自 [0,0,470] 面向 -z，箱子在其右前方 8m）。
pub const CRATE_POS: [f32; 3] = [4.0, 0.45, 462.0];

/// 功能站点登记：补给台 + 干员切换台 + 出生点物资箱（demo 侧据此生成可交互站点）
pub(crate) fn stations() -> Vec<StationSpec> {
    vec![
        StationSpec { pos: [-5.0, 0.78, SUPPLY_LINE_Z], kind: StationKind::SupplyTable, label: "补给台" },
        StationSpec { pos: [5.0, 0.78, SUPPLY_LINE_Z], kind: StationKind::OperatorDesk, label: "干员切换台" },
        // 出生点右前方的物资箱：落地可交互，开箱一次性发放弹药/医疗/护甲。
        StationSpec { pos: CRATE_POS, kind: StationKind::SupplyCrate, label: "物资箱" },
    ]
}

/// 功能台台面语义光条（琥珀=补给，青色=干员）+ 出生光垫
pub(crate) fn pad_glow() -> Vec<GlowSpec> {
    vec![
        GlowSpec { shape: Shape::Box, pos: [0.0, 0.02, SPAWN_Z], half: [2.5, 0.04, 2.5], glow: GlowKind::SpawnPad },
        GlowSpec { shape: Shape::Box, pos: [-5.0, 0.83, SUPPLY_LINE_Z], half: [1.0, 0.03, 0.45], glow: GlowKind::Supply },
        GlowSpec { shape: Shape::Box, pos: [5.0, 0.83, SUPPLY_LINE_Z], half: [1.0, 0.03, 0.45], glow: GlowKind::Operator },
    ]
}
