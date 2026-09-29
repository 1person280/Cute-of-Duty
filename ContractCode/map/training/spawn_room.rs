//! 出生准备室（z 6..15）—— 玩家出生的安全区与补给枢纽。
//!
//! 包含：南墙三段门框（对齐中央三巷道）、东/西侧墙门洞（休息间/军械间）、
//! 出生光垫、物资横排（基础补给/手雷/武器）、两张功能台及对应站点/光条、
//! 出生准备线，以及桶/货架/桌椅等出生区装饰掩体。
//!
//! 相关常量（SUPPLY_LINE_Z / SUPPLY_TABLE_POS / OPERATOR_DESK_POS / WALL_H）由父模块定义。

use crate::element::ElementType;
use crate::map::{
    GlowKind, GlowSpec, MaterialKind, PickupKind, PickupSpec, Prop, Shape, StationKind, StationSpec,
};

use super::{OPERATOR_DESK_POS, SUPPLY_LINE_Z, SUPPLY_TABLE_POS, WALL_H};

/// 出生准备室隔断：南墙三段门框 + 东西侧墙门洞 + 对应踢脚线
pub(crate) fn spawn_partitions() -> Vec<Prop> {
    let mut v = Vec::new();

    // ---- 出生准备室南墙（z=6，三个门洞对齐 x = -3 / 0 / 3 巷道）----
    for (cx, hx) in [(-5.25, 0.75), (0.0, 1.5), (5.25, 0.75)] {
        v.push(Prop::solid([cx, WALL_H / 2.0, 6.0], [hx, WALL_H / 2.0, 0.15], MaterialKind::Concrete));
    }
    // 门框（decor 贴面：门楣 + 侧柱，无直角硬切观感）
    for cx in [-3.0, 0.0, 3.0] {
        v.push(Prop::decor([cx, 2.8, 6.0], [1.6, 0.25, 0.2], MaterialKind::Dark));
        v.push(Prop::decor([cx - 1.6, 1.4, 6.0], [0.15, 1.4, 0.2], MaterialKind::Dark));
        v.push(Prop::decor([cx + 1.6, 1.4, 6.0], [0.15, 1.4, 0.2], MaterialKind::Dark));
    }

    // ---- 东西侧墙（x=±6, z 6..15，军械间/休息间各留门洞）----
    // 西墙（军械间）：门洞 z 8..10
    v.push(Prop::solid([-6.0, WALL_H / 2.0, 7.0], [0.15, WALL_H / 2.0, 1.0], MaterialKind::Concrete));
    v.push(Prop::solid([-6.0, WALL_H / 2.0, 12.5], [0.15, WALL_H / 2.0, 2.5], MaterialKind::Concrete));
    v.push(Prop::decor([-6.0, 2.8, 9.0], [0.2, 0.25, 1.3], MaterialKind::Dark));
    // 东墙（休息间）：门洞 z 11..13
    v.push(Prop::solid([6.0, WALL_H / 2.0, 8.5], [0.15, WALL_H / 2.0, 2.5], MaterialKind::Concrete));
    v.push(Prop::solid([6.0, WALL_H / 2.0, 14.0], [0.15, WALL_H / 2.0, 1.0], MaterialKind::Concrete));
    v.push(Prop::decor([6.0, 2.8, 12.0], [0.2, 0.25, 1.3], MaterialKind::Dark));

    // ---- 踢脚线（decor，沿出生准备室诸墙根收边）----
    for (pos, half) in [
        ([-3.75, 0.06, 6.12], [2.25, 0.06, 0.04]),
        ([3.75, 0.06, 6.12], [2.25, 0.06, 0.04]),
        ([-6.12, 0.06, 7.0], [0.04, 0.06, 1.0]),
        ([-6.12, 0.06, 12.5], [0.04, 0.06, 2.5]),
        ([6.12, 0.06, 8.5], [0.04, 0.06, 2.5]),
        ([6.12, 0.06, 14.0], [0.04, 0.06, 1.0]),
    ] {
        v.push(Prop::decor(pos, half, MaterialKind::Dark));
    }
    v
}

/// 出生区掩体/家具：门侧桶 + 军械间货架 + 休息间桌椅（装饰性掩体）
pub(crate) fn spawn_cover() -> Vec<Prop> {
    let mut v = Vec::new();
    // 桶：出生门两侧对称一对，不挡门与 ±3 射击道
    v.push(Prop::solid_cyl([4.4, 0.35, 5.2], 0.25, 0.7, MaterialKind::Rust));
    v.push(Prop::solid_cyl([-4.4, 0.35, 5.2], 0.25, 0.7, MaterialKind::Rust));
    // 军械间货架（靠西墙，装饰掩体）
    v.push(Prop::solid([-13.0, 0.9, 8.0], [1.2, 0.9, 0.35], MaterialKind::Steel));
    v.push(Prop::solid([-13.0, 0.9, 11.0], [1.2, 0.9, 0.35], MaterialKind::Steel));
    // 休息间桌椅（靠东墙）
    v.push(Prop::solid([13.0, 0.5, 9.0], [0.9, 0.5, 0.5], MaterialKind::Steel));
    v
}

/// 出生区两张功能台：台面（实心可碰撞）+ 四条桌腿，对齐物资线
pub(crate) fn spawn_tables() -> Vec<Prop> {
    let mut v = Vec::new();
    for (x, z) in [(-4.5, SUPPLY_LINE_Z), (4.5, SUPPLY_LINE_Z)] {
        // 台面（高度 0.72±0.06，站台上沿即站点交互位）
        v.push(Prop::solid([x, 0.72, z], [1.1, 0.06, 0.55], MaterialKind::Steel));
        // 桌腿
        for (dx, dz) in [(-0.9, -0.4), (0.9, -0.4), (-0.9, 0.4), (0.9, 0.4)] {
            v.push(Prop::solid([x + dx, 0.33, z + dz], [0.06, 0.33, 0.06], MaterialKind::Dark));
        }
    }
    v
}

/// 出生准备线（白色虚线，z = 9.5）—— 玩家落地即到的起跑线装饰
pub(crate) fn spawn_line_decor() -> Vec<Prop> {
    let mut v = Vec::new();
    for x in -3..=3 {
        v.push(Prop::decor([x as f32, 0.02, 9.5], [0.45, 0.025, 0.075], MaterialKind::PaintWhite));
    }
    v
}

/// 出生区物资线补给：基础补给/手雷/武器排成一列（与功能台同一排）
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
            pos: [-8.5, 0.4, SUPPLY_LINE_Z],
            label: "毒液步枪",
            kind: PickupKind::Weapon { element: ElementType::Poison },
        },
        PickupSpec {
            pos: [8.5, 0.4, SUPPLY_LINE_Z],
            label: "雷电步枪",
            kind: PickupKind::Weapon { element: ElementType::Electric },
        },
    ]
}

/// 功能站点登记：补给台 + 干员切换台（demo 侧据此生成可交互站点）
pub(crate) fn stations() -> Vec<StationSpec> {
    vec![
        StationSpec { pos: SUPPLY_TABLE_POS, kind: StationKind::SupplyTable, label: "补给台" },
        StationSpec { pos: OPERATOR_DESK_POS, kind: StationKind::OperatorDesk, label: "干员切换台" },
    ]
}

/// 功能台台面的语义光条（琥珀=补给，青色=干员）
pub(crate) fn station_glow() -> Vec<GlowSpec> {
    vec![
        GlowSpec {
            shape: Shape::Box,
            pos: [-4.5, 0.83, SUPPLY_LINE_Z],
            half: [1.0, 0.03, 0.45],
            glow: GlowKind::Supply,
        },
        GlowSpec {
            shape: Shape::Box,
            pos: [4.5, 0.83, SUPPLY_LINE_Z],
            half: [1.0, 0.03, 0.45],
            glow: GlowKind::Operator,
        },
    ]
}

/// 出生光垫 + 提取信标（西南角）
pub(crate) fn beacon_and_spawn_pad() -> Vec<GlowSpec> {
    vec![
        GlowSpec {
            shape: Shape::Box,
            pos: [0.0, 0.02, 10.5],
            half: [1.25, 0.04, 1.25],
            glow: GlowKind::SpawnPad,
        },
        GlowSpec {
            shape: Shape::Box,
            pos: [-12.0, 1.75, 12.0],
            half: [0.15, 1.75, 0.15],
            glow: GlowKind::Red,
        },
        GlowSpec {
            shape: Shape::Box,
            pos: [-12.0, 3.6, 12.0],
            half: [0.4, 0.1, 0.4],
            glow: GlowKind::Red,
        },
    ]
}