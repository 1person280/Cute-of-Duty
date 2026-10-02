//! CQB 室内训练场 —— 声明式数据定义（0.3.1 全室内重做版）
//!
//! 取代旧版露天训练场：30×30m 全室内 CQB 训练设施，按功能分区组织——
//!
//! - 出生准备室（z 6..15）：出生光垫、物资横排（基础补给/手雷）、
//!   两侧隔断墙（西侧军械间 / 东侧休息间）+ 门框 + 踢脚线
//! - 中央 CQB 大厅（z -6..6）：三巷道隔断、跪姿矮墙阵、中央指挥台
//!   （二层平台 + 四级楼梯）、东西二层回廊（楼梯登顶）、功能台
//! - 北侧射击馆（z -15..-6）：拱形隔墙三开口衔接大厅、4 条射击道
//!   （x = ±3 / ±9）、5/10/15 米标线 + 分层靶位 + 移动靶
//! - 天花板：横梁阵列 + 灯带（室内氛围），墙体全封闭 + 立柱
//!
//! 坐标约定：玩家出生在 [0, 0, 10.5] 面向 -Z。所有布局约束（边界、
//! 出生区净空、射击道畅通、靶不与掩体相交、移动靶扫掠不出界、
//! 高台可达性）由本模块单元测试保证 —— 改坐标前先跑 `cargo test --lib`。
//!
//! 布局按区块融合进本模块（`layout()` 后的分节函数）：早先拆成 `spawn_room`
//! / `cqb_hall` / `shooting_range` / `second_floor` / `building_shell` 等下划线
//! 文件，按反屎山公约（文件名禁下划线 · 嵌套 ≤2 层）统一融为模块内分节函数。

use crate::element::ElementType;
use crate::map::{
    GlowKind, GlowSpec, MapLayout, MaterialKind, Motion, PickupKind, PickupSpec, Prop, Shape,
    StationKind, StationSpec, TargetSpec,
};

/// 场地半径（米）
const HALF: f32 = 15.0;

/// 内墙净高（米）；隔断墙统一 3m 高
const WALL_H: f32 = 3.0;

/// 射击道中心线（靶位摆放与净空测试共用）
pub const LANES: [f32; 4] = [-9.0, -3.0, 3.0, 9.0];

/// 出生区物资线纵横坐标（补给横排 / 功能台 / 站点共用），基准 z
pub const SUPPLY_LINE_Z: f32 = 8.5;

/// 站点坐标：补给台在出生准备室西侧，干员切换台在东侧（与物资线齐平）
pub const SUPPLY_TABLE_POS: [f32; 3] = [-4.5, 0.78, SUPPLY_LINE_Z];
pub const OPERATOR_DESK_POS: [f32; 3] = [4.5, 0.78, SUPPLY_LINE_Z];

/// 生成完整 CQB 室内训练场布局。每次调用构建全新数据，可安全修改。
pub fn layout() -> MapLayout {
    MapLayout {
        name: "CQB室内训练场".into(),
        half_extent: HALF,
        floor_tile: 1.0,
        player_spawn: [0.0, 0.0, 10.5],
        props: [
            perimeter(),
            spawn_partitions(),
            hall_partitions(),
            range_arch(),
            hall_cover(),
            spawn_cover(),
            lane_markers(),
            command_platform(),
            walkways(),
            spawn_tables(),
            hall_tables(),
            spawn_line_decor(),
            shell_decor(),
        ]
        .concat(),
        targets: [
            static_targets(),
            hall_mover(),
            range_movers(),
        ]
        .concat(),
        pickups: [
            supply_line(),
            hall_supply(),
            range_supply(),
        ]
        .concat(),
        glows: [
            neon(),
            beacon_and_spawn_pad(),
            station_glow(),
        ]
        .concat(),
        stations: stations(),
    }
}

// ============================================================================
// 出生准备室（z 6..15）—— 玩家出生的安全区与补给枢纽
//
// 包含：南墙三段门框（对齐中央三巷道）、东/西侧墙门洞（休息间/军械间）、
// 出生光垫、物资横排（基础补给/手雷/武器）、两张功能台及对应站点/光条、
// 出生准备线，以及桶/货架/桌椅等出生区装饰掩体。
// ============================================================================

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
            label: "冰霜手雷".into(),
            kind: PickupKind::Grenade { element: ElementType::Ice },
        },
        PickupSpec { pos: [-1.0, 0.4, SUPPLY_LINE_Z], label: "步枪弹药".into(), kind: PickupKind::Ammo { amount: 30 } },
        PickupSpec { pos: [0.0, 0.4, SUPPLY_LINE_Z], label: "医疗包".into(), kind: PickupKind::Health { amount: 25.0 } },
        PickupSpec { pos: [1.0, 0.4, SUPPLY_LINE_Z], label: "护甲片".into(), kind: PickupKind::Armor { amount: 20.0 } },
        PickupSpec {
            pos: [2.5, 0.4, SUPPLY_LINE_Z],
            label: "烈焰手雷".into(),
            kind: PickupKind::Grenade { element: ElementType::Fire },
        },
        PickupSpec {
            pos: [-8.5, 0.4, SUPPLY_LINE_Z],
            label: "毒液步枪".into(),
            kind: PickupKind::Weapon { element: ElementType::Poison },
        },
        PickupSpec {
            pos: [8.5, 0.4, SUPPLY_LINE_Z],
            label: "雷电步枪".into(),
            kind: PickupKind::Weapon { element: ElementType::Electric },
        },
    ]
}

/// 功能站点登记：补给台 + 干员切换台（demo 侧据此生成可交互站点）
pub(crate) fn stations() -> Vec<StationSpec> {
    vec![
        StationSpec { pos: SUPPLY_TABLE_POS, kind: StationKind::SupplyTable, label: "补给台".into() },
        StationSpec { pos: OPERATOR_DESK_POS, kind: StationKind::OperatorDesk, label: "干员切换台".into() },
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

// ============================================================================
// 中央 CQB 大厅（z -6..6）—— 三巷道隔断、跪姿矮墙阵、中央指挥台、
// 大厅功能台、沙袋堆，以及大厅移动靶与纵深补给。
//
// 这是近距离战斗练习的主场地，布局约束（射击道畅通、掩体不与靶相交、
// 高台可达）由本模块单元测试保证。
// ============================================================================

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
            label: "移动靶".into(),
            motion: Some(Motion { speed: 2.0, range: 2.0, start_dir: 1.0 }),
        },
    ]
}

/// 大厅纵深补给：左右镜像一对（弹药箱 / 大医疗包）
pub(crate) fn hall_supply() -> Vec<PickupSpec> {
    vec![
        PickupSpec { pos: [-9.0, 0.4, -2.0], label: "弹药箱".into(), kind: PickupKind::Ammo { amount: 60 } },
        PickupSpec { pos: [9.0, 0.4, -2.0], label: "大医疗包".into(), kind: PickupKind::Health { amount: 50.0 } },
    ]
}

// ============================================================================
// 北侧射击馆（z -15..-6）—— 拱形隔墙三开口衔接大厅、4 条射击道
// （x = ±3 / ±9）、5/10/15 米标线 + 分层靶位 + 移动靶 + 纵深手雷。
//
// 射击道中心线（LANES）与净空/畅通性由本模块测试保证，改靶位/掩体前先跑测试。
// ============================================================================

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
        TargetSpec { pos: [-3.0, 1.5, -8.0], label: "近距靶".into(), motion: None },
        TargetSpec { pos: [3.0, 1.5, -8.0], label: "近距靶".into(), motion: None },
        // 中线：外侧双靶
        TargetSpec { pos: [-9.0, 1.8, -11.0], label: "中距靶".into(), motion: None },
        TargetSpec { pos: [9.0, 1.8, -11.0], label: "中距靶".into(), motion: None },
        // 远线：外侧双靶（贴后墙净空）
        TargetSpec { pos: [-9.0, 2.2, -14.3], label: "远距靶".into(), motion: None },
        TargetSpec { pos: [9.0, 2.2, -14.3], label: "远距靶".into(), motion: None },
        // 指挥台顶靶：练垂直仰角射击
        TargetSpec { pos: [0.0, 3.4, 0.0], label: "台顶靶".into(), motion: None },
        // 回廊靶：东侧二层回廊上层位
        TargetSpec { pos: [13.0, 3.4, -2.0], label: "回廊靶".into(), motion: None },
    ]
}

/// 射击馆移动靶：中段快速横移 + 远端中速巡逻（扫掠范围避开全部掩体，由测试保证）
pub(crate) fn range_movers() -> Vec<TargetSpec> {
    vec![
        // 射击馆中段快速横移
        TargetSpec {
            pos: [6.5, 1.8, -9.5],
            label: "移动靶".into(),
            motion: Some(Motion { speed: 3.0, range: 3.0, start_dir: -1.0 }),
        },
        // 射击馆远端中速巡逻
        TargetSpec {
            pos: [-9.0, 1.5, -13.0],
            label: "移动靶".into(),
            motion: Some(Motion { speed: 2.5, range: 2.5, start_dir: 1.0 }),
        },
    ]
}

/// 射击馆纵深手雷：左右镜像一对（毒素 / 雷电，补足四系元素）
pub(crate) fn range_supply() -> Vec<PickupSpec> {
    vec![
        PickupSpec {
            pos: [-8.0, 0.4, -4.5],
            label: "毒素手雷".into(),
            kind: PickupKind::Grenade { element: ElementType::Poison },
        },
        PickupSpec {
            pos: [8.0, 0.4, -4.5],
            label: "雷电手雷".into(),
            kind: PickupKind::Grenade { element: ElementType::Electric },
        },
    ]
}

// ============================================================================
// 二层立体结构（东西回廊）—— 训练场垂直机动的另一半：指挥台（中央）在大厅分节，
// 东西回廊在此。台阶高差 ≤1.1m 保证可跳登上层（由 vertical_structures_climbable 测试保证）。
// ============================================================================

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

// ============================================================================
// 建筑外壳 —— 四面封闭围墙与立柱、天花板横梁阵列、南墙观察窗、室内灯带。
//
// 属于 CQB 室内训练场的"容器"层：把整块 30×30m 场地围成封闭房间，并提供室内氛围
// 所需的梁柱结构。
// ============================================================================

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

#[cfg(test)]
mod tests;