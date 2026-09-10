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

use crate::element::ElementType;
use crate::map::{
    GlowKind, GlowSpec, MapLayout, MaterialKind, Motion, PickupKind, PickupSpec, Prop, Shape,
    StationKind, StationSpec, TargetSpec,
};

/// 场地半径（米）
const HALF: f32 = 15.0;

/// 射击道中心线（靶位摆放与净空测试共用）
pub const LANES: [f32; 4] = [-9.0, -3.0, 3.0, 9.0];

/// 生成完整 CQB 室内训练场布局。每次调用构建全新数据，可安全修改。
pub fn layout() -> MapLayout {
    MapLayout {
        name: "CQB室内训练场",
        half_extent: HALF,
        floor_tile: 1.0,
        player_spawn: [0.0, 0.0, 10.5],
        props: [perimeter(), partitions(), lane_markers(), cqb_course(), vertical_structures(), furniture(), decor()].concat(),
        targets: [static_targets(), movers()].concat(),
        pickups: supply(),
        glows: [neon(), beacon_and_spawn_pad(), station_glow()].concat(),
        stations: stations(),
    }
}

// =============================================================================
// 围墙（全室内：四面封闭 + 立柱）
// =============================================================================

/// 室内四壁：全封闭（区别于旧版露天三面墙），立柱沿四壁排布
fn perimeter() -> Vec<Prop> {
    let mut v = vec![
        Prop::solid([0.0, 1.5, -HALF], [HALF + 0.5, 1.5, 0.15], MaterialKind::Steel),
        Prop::solid([0.0, 1.5, HALF], [HALF + 0.5, 1.5, 0.15], MaterialKind::Steel),
        Prop::solid([-HALF, 1.5, 0.0], [0.15, 1.5, HALF + 0.5], MaterialKind::Steel),
        Prop::solid([HALF, 1.5, 0.0], [0.15, 1.5, HALF + 0.5], MaterialKind::Steel),
    ];
    // 立柱：四壁每 5m 一根（室内框架结构感）
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

// =============================================================================
// 室内隔断（分区墙 + 门框 + 窗）
// =============================================================================

/// 内墙净高（米）；隔断墙统一 3m 高
const WALL_H: f32 = 3.0;

/// 分区隔断：出生准备室（x ±6 / z=6 以下）、射击馆拱墙（z=-6，三开口）、
/// 军械间/休息间隔墙（x=±6, z 6..15，各留门洞）
fn partitions() -> Vec<Prop> {
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

    // ---- 出生准备室侧墙（x=±6, z 6..15，军械间/休息间各留门洞 z 8..10 / 11..13）----
    // 西墙（军械间）：门洞 z 8..10
    v.push(Prop::solid([-6.0, WALL_H / 2.0, 7.0], [0.15, WALL_H / 2.0, 1.0], MaterialKind::Concrete));
    v.push(Prop::solid([-6.0, WALL_H / 2.0, 12.5], [0.15, WALL_H / 2.0, 2.5], MaterialKind::Concrete));
    v.push(Prop::decor([-6.0, 2.8, 9.0], [0.2, 0.25, 1.3], MaterialKind::Dark));
    // 东墙（休息间）：门洞 z 11..13
    v.push(Prop::solid([6.0, WALL_H / 2.0, 8.5], [0.15, WALL_H / 2.0, 2.5], MaterialKind::Concrete));
    v.push(Prop::solid([6.0, WALL_H / 2.0, 14.0], [0.15, WALL_H / 2.0, 1.0], MaterialKind::Concrete));
    v.push(Prop::decor([6.0, 2.8, 12.0], [0.2, 0.25, 1.3], MaterialKind::Dark));

    // ---- 射击馆拱墙（z=-6，三条 3.6m 开口对齐中央三巷道）----
    // 墙段中心/半长：[-12.6,2.4] [-6,1.8] [0,1.8] [6,1.8] [12.6,2.4]
    for (cx, hx) in [(-12.6, 2.4), (-6.0, 1.8), (0.0, 1.8), (6.0, 1.8), (12.6, 2.4)] {
        v.push(Prop::solid([cx, WALL_H / 2.0, -6.0], [hx, WALL_H / 2.0, 0.15], MaterialKind::Concrete));
    }
    // 拱墙开口门楣（decor）
    for cx in [-9.0, -3.0, 3.0, 9.0] {
        v.push(Prop::decor([cx, 2.8, -6.0], [1.6, 0.25, 0.2], MaterialKind::Dark));
    }

    // ---- 大厅巷道隔断（x=±5.5, z -3..3，中央错位开口）----
    for x in [-5.5, 5.5] {
        v.push(Prop::solid([x, WALL_H / 2.0, -2.0], [0.3, WALL_H / 2.0, 1.0], MaterialKind::Concrete));
        v.push(Prop::solid([x, WALL_H / 2.0, 1.6], [0.3, WALL_H / 2.0, 1.4], MaterialKind::Concrete));
    }

    // ---- 踢脚线（decor，墙根收边，沿主要内墙）----
    for (pos, half) in [
        ([-3.75, 0.06, 6.12], [2.25, 0.06, 0.04]),
        ([3.75, 0.06, 6.12], [2.25, 0.06, 0.04]),
        ([-6.12, 0.06, 7.0], [0.04, 0.06, 1.0]),
        ([-6.12, 0.06, 12.5], [0.04, 0.06, 2.5]),
        ([6.12, 0.06, 8.5], [0.04, 0.06, 2.5]),
        ([6.12, 0.06, 14.0], [0.04, 0.06, 1.0]),
        ([-6.0, 0.06, -6.12], [1.8, 0.06, 0.04]),
        ([0.0, 0.06, -6.12], [1.8, 0.06, 0.04]),
        ([6.0, 0.06, -6.12], [1.8, 0.06, 0.04]),
    ] {
        v.push(Prop::decor(pos, half, MaterialKind::Dark));
    }
    v
}

// =============================================================================
// 射击道标线与标牌
// =============================================================================

/// 距离标线（虚线）与标牌（出生点 [0,0,10.5] 起算：约 18.5 / 21.5 / 24.8m，
/// 即射击馆内三段纵深）
fn lane_markers() -> Vec<Prop> {
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
    // 标牌（立柱 + 白板），贴射击馆拱墙北侧一列
    for z in [-8.0, -11.0, -14.3] {
        v.push(Prop::decor([-4.4, 0.9, z], [0.075, 0.9, 0.075], MaterialKind::Steel));
        v.push(Prop::decor([-4.4, 1.5, z + 0.1], [0.4, 0.25, 0.04], MaterialKind::PaintWhite));
    }
    v
}

// =============================================================================
// 靶位
// =============================================================================

/// 静态靶：随距离升高，内外侧道分层布置
fn static_targets() -> Vec<TargetSpec> {
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

/// 移动靶：三档节奏，扫掠范围避开全部掩体（由测试保证）
fn movers() -> Vec<TargetSpec> {
    vec![
        // 大厅北侧开阔带：巷道隔墙与跪姿矮墙之间慢速横移
        TargetSpec {
            pos: [-2.5, 1.5, -4.0],
            label: "移动靶",
            motion: Some(Motion { speed: 2.0, range: 2.0, start_dir: 1.0 }),
        },
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

// =============================================================================
// 掩体训练区（CQB 大厅）
// =============================================================================

/// 掩体布置：左右对称的跪姿矮墙列 + 大厅沙袋堆 + 桶阵，
/// 全部落在大厅与巷道里（净空由测试保证）
fn cqb_course() -> Vec<Prop> {
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

// =============================================================================
// 立体结构（垂直机动）
// =============================================================================

/// 立体结构：中央指挥台（二层平台 + 四级楼梯）+ 东西二层回廊（楼梯登顶）。
/// 全部轴对齐盒体（AABB 碰撞友好），台阶高差 ≤1.1m 保证可跳上。
fn vertical_structures() -> Vec<Prop> {
    let mut v = Vec::new();

    // ---- 中央指挥台（x=0, z=0，落位在 ±3 射击道之间的空当）----
    v.push(Prop::solid([0.0, 2.5, 0.0], [2.2, 0.15, 1.8], MaterialKind::Concrete));
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

// =============================================================================
// 功能站点家具
// =============================================================================

/// 出生区物资横排：基础补给、手雷、武器与两张功能台全部对齐在这条线上
pub const SUPPLY_LINE_Z: f32 = 8.5;

/// 站点坐标：补给台在出生准备室西侧，干员切换台在东侧（与物资线齐平）
pub const SUPPLY_TABLE_POS: [f32; 3] = [-4.5, 0.78, SUPPLY_LINE_Z];
pub const OPERATOR_DESK_POS: [f32; 3] = [4.5, 0.78, SUPPLY_LINE_Z];

/// 两张功能台：台面（实心可碰撞）+ 四条桌腿
fn furniture() -> Vec<Prop> {
    let mut v = Vec::new();
    for (x, z) in [(-4.5, SUPPLY_LINE_Z), (4.5, SUPPLY_LINE_Z)] {
        // 台面（高度 0.72±0.06，站台上沿即站点交互位）
        v.push(Prop::solid([x, 0.72, z], [1.1, 0.06, 0.55], MaterialKind::Steel));
        // 桌腿
        for (dx, dz) in [(-0.9, -0.4), (0.9, -0.4), (-0.9, 0.4), (0.9, 0.4)] {
            v.push(Prop::solid([x + dx, 0.33, z + dz], [0.06, 0.33, 0.06], MaterialKind::Dark));
        }
    }
    // 大厅两张功能台（z=4.5 与回廊楼梯错位）
    for x in [-6.5, 6.5] {
        v.push(Prop::solid([x, 0.72, 4.5], [1.1, 0.06, 0.55], MaterialKind::Steel));
        for (dx, dz) in [(-0.9, -0.4), (0.9, -0.4), (-0.9, 0.4), (0.9, 0.4)] {
            v.push(Prop::solid([x + dx, 0.33, 4.5 + dz], [0.06, 0.33, 0.06], MaterialKind::Dark));
        }
    }
    v
}

/// 功能站点登记：demo 侧据此生成可交互站点（按 F 打开面板）
fn stations() -> Vec<StationSpec> {
    vec![
        StationSpec { pos: SUPPLY_TABLE_POS, kind: StationKind::SupplyTable, label: "补给台" },
        StationSpec { pos: OPERATOR_DESK_POS, kind: StationKind::OperatorDesk, label: "干员切换台" },
    ]
}

/// 功能台台面的语义光条（琥珀=补给，青色=干员）
fn station_glow() -> Vec<GlowSpec> {
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

// =============================================================================
// 装饰
// =============================================================================

/// 出生线、天花板横梁阵列与壁灯座
fn decor() -> Vec<Prop> {
    let mut v = Vec::new();
    // 出生准备线（白色虚线，z = 9.5）
    for x in -3..=3 {
        v.push(Prop::decor([x as f32, 0.02, 9.5], [0.45, 0.025, 0.075], MaterialKind::PaintWhite));
    }
    // 天花板横梁阵列（东西向，每 4m 一根，撑起室内感；顶不封保持采光）
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

// =============================================================================
// 发光件
// =============================================================================

/// 室内灯带：天花板梁下绿色灯带 + 侧壁橙色灯带（硬光氛围）
fn neon() -> Vec<GlowSpec> {
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

/// 出生光垫 + 提取信标（西南角）
fn beacon_and_spawn_pad() -> Vec<GlowSpec> {
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

// =============================================================================
// 补给
// =============================================================================

/// 补给与战术手雷：基础补给/手雷/武器在出生区物资线上排成一列，
/// 纵深处只留弹药箱/大医疗包与毒素/雷电手雷各一对镜像
fn supply() -> Vec<PickupSpec> {
    vec![
        // 出生区物资线（z = SUPPLY_LINE_Z，与功能台同一排）
        PickupSpec { pos: [-2.5, 0.4, SUPPLY_LINE_Z], label: "冰霜手雷", kind: PickupKind::Grenade { element: ElementType::Ice } },
        PickupSpec { pos: [-1.0, 0.4, SUPPLY_LINE_Z], label: "步枪弹药", kind: PickupKind::Ammo { amount: 30 } },
        PickupSpec { pos: [0.0, 0.4, SUPPLY_LINE_Z], label: "医疗包", kind: PickupKind::Health { amount: 25.0 } },
        PickupSpec { pos: [1.0, 0.4, SUPPLY_LINE_Z], label: "护甲片", kind: PickupKind::Armor { amount: 20.0 } },
        PickupSpec { pos: [2.5, 0.4, SUPPLY_LINE_Z], label: "烈焰手雷", kind: PickupKind::Grenade { element: ElementType::Fire } },
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
        // 大厅纵深补给：左右镜像一对
        PickupSpec { pos: [-9.0, 0.4, -2.0], label: "弹药箱", kind: PickupKind::Ammo { amount: 60 } },
        PickupSpec { pos: [9.0, 0.4, -2.0], label: "大医疗包", kind: PickupKind::Health { amount: 50.0 } },
        // 射击馆纵深手雷：左右镜像一对
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

// =============================================================================
// 布局约束测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::Pos;

    /// 两个轴对齐盒（位置 + 半尺寸）是否相交
    fn overlap(a_pos: Pos, a_half: [f32; 3], b_pos: Pos, b_half: [f32; 3]) -> bool {
        (0..3).all(|i| (a_pos[i] - b_pos[i]).abs() < a_half[i] + b_half[i])
    }

    fn solid_props() -> Vec<Prop> {
        layout().props.into_iter().filter(|p| p.solid).collect()
    }

    #[test]
    fn everything_within_arena_bounds() {
        let l = layout();
        let margin = 0.6;
        for p in &l.props {
            let half = p.aabb_half();
            assert!(p.pos[0].abs() + half[0] <= HALF + margin, "prop 超出 x 边界: {:?}", p.pos);
            assert!(p.pos[2].abs() + half[2] <= HALF + margin, "prop 超出 z 边界: {:?}", p.pos);
            assert!(p.pos[1] + half[1] <= 5.0, "prop 过高: y_top={}", p.pos[1] + half[1]);
            assert!(p.pos[1] - half[1] >= -0.3, "prop 陷入地下过深: y_bottom={}", p.pos[1] - half[1]);
        }
        for t in &l.targets {
            assert!(t.pos[0].abs() <= HALF && t.pos[2].abs() <= HALF, "靶超出边界: {:?}", t.pos);
        }
        for pk in &l.pickups {
            assert!(pk.pos[0].abs() <= HALF && pk.pos[2].abs() <= HALF, "拾取物超出边界: {:?}", pk.pos);
        }
        for g in &l.glows {
            assert!(g.pos[0].abs() + g.half[0] <= HALF + margin, "发光件超出 x 边界: {:?}", g.pos);
            assert!(g.pos[2].abs() + g.half[2] <= HALF + margin, "发光件超出 z 边界: {:?}", g.pos);
        }
    }

    /// 出生点周围 1.5m 内不能有高于 0.5m 的碰撞体（玩家出生即被卡死）
    #[test]
    fn spawn_zone_clear() {
        let l = layout();
        let [sx, _, sz] = l.player_spawn;
        for p in solid_props() {
            let half = p.aabb_half();
            let top = p.pos[1] + half[1];
            if top <= 0.5 {
                continue;
            }
            assert!(
                ((p.pos[0] - sx).abs() - half[0]) >= 1.5 || ((p.pos[2] - sz).abs() - half[2]) >= 1.5,
                "出生区被 solid prop 遮挡: pos={:?} half={:?}",
                p.pos,
                half
            );
        }
    }

    /// 靶不埋进掩体：静态靶与所有 solid prop 不相交
    #[test]
    fn static_targets_clear_of_solids() {
        let l = layout();
        // 靶视觉体近似半尺寸：面板 1.2×1.2×0.4
        let target_half = [0.6, 0.6, 0.25];
        for t in &l.targets {
            if t.motion.is_some() {
                continue;
            }
            for p in solid_props() {
                assert!(
                    !overlap(t.pos, target_half, p.pos, p.aabb_half()),
                    "静态靶 {:?} 与掩体 {:?} 相交",
                    t.pos,
                    p.pos
                );
            }
        }
    }

    /// 移动靶整个扫掠体积不与掩体相交（移动靶穿模问题的回归测试）
    #[test]
    fn moving_targets_clear_of_solids() {
        let l = layout();
        for t in &l.targets {
            let Some(m) = t.motion else { continue };
            // 扫掠盒：x 覆盖整个往返区间（含靶身余量），y/z 取靶身余量
            let sweep_pos = [t.pos[0], t.pos[1], t.pos[2]];
            let sweep_half = [m.range + 0.5, 1.0, 0.5];
            for p in solid_props() {
                assert!(
                    !overlap(sweep_pos, sweep_half, p.pos, p.aabb_half()),
                    "移动靶扫掠体积（中心 {:?}，range {}）穿过掩体 {:?}",
                    t.pos,
                    m.range,
                    p.pos
                );
            }
        }
    }

    /// 移动靶扫掠范围不越出场地
    #[test]
    fn moving_targets_stay_in_bounds() {
        let l = layout();
        for t in &l.targets {
            let Some(m) = t.motion else { continue };
            assert!(
                t.pos[0].abs() + m.range + 0.5 <= HALF,
                "移动靶 x 扫掠越界: {:?} range {}",
                t.pos,
                m.range
            );
            assert!(
                t.pos[2].abs() + 0.5 <= HALF,
                "移动靶 z 越界: {:?}",
                t.pos
            );
        }
    }

    /// 射击道中心线畅通：高于 0.4m 且低于 2.6m 的 solid prop 不得进入道轴 ±0.35m
    /// （保证从出生点到各距离线的直射视线不被掩体切断；桥底 ≥2.6m 的高架结构豁免）
    #[test]
    fn firing_lanes_clear() {
        let lane_z = [6.0, -14.5]; // 射击道 z 范围（出生准备室门洞到最远靶位）
        for lane in LANES {
            for p in solid_props() {
                let half = p.aabb_half();
                let top = p.pos[1] + half[1];
                let bottom = p.pos[1] - half[1];
                if top <= 0.4 || bottom >= 2.6 {
                    continue;
                }
                let z_near = p.pos[2] - half[2] - 0.2;
                let z_far = p.pos[2] + half[2] + 0.2;
                let z_overlaps = z_near <= lane_z[0] && z_far >= lane_z[1];
                if !z_overlaps {
                    continue;
                }
                assert!(
                    (lane - p.pos[0]).abs() >= half[0] + 0.35,
                    "射击道 x={} 被 prop {:?}（half {:?}）侵入",
                    lane,
                    p.pos,
                    half
                );
            }
        }
    }

    /// 靶与拾取物不埋地
    #[test]
    fn targets_and_pickups_grounded() {
        let l = layout();
        for t in &l.targets {
            assert!(t.pos[1] >= 0.5, "靶位过低: {:?}", t.pos);
        }
        for pk in &l.pickups {
            assert!(pk.pos[1] >= 0.3, "拾取物过低: {:?}", pk.pos);
        }
    }

    /// 每条射击道上至少有一个静态靶（道位不空置）
    #[test]
    fn lanes_have_targets() {
        let l = layout();
        for lane in LANES {
            assert!(
                l.targets.iter().any(|t| t.motion.is_none() && (t.pos[0] - lane).abs() < 0.5),
                "射击道 x={} 上没有任何靶",
                lane
            );
        }
    }

    /// 元素手雷覆盖四系元素（训练场要能体验全部反应）
    #[test]
    fn grenades_cover_all_elements() {
        let l = layout();
        let mut found = Vec::new();
        for pk in &l.pickups {
            if let PickupKind::Grenade { element } = pk.kind {
                found.push(element);
            }
        }
        for e in [
            ElementType::Fire,
            ElementType::Ice,
            ElementType::Electric,
            ElementType::Poison,
        ] {
            assert!(found.contains(&e), "缺少元素手雷: {:?}", e);
        }
    }

    /// 功能站点：两类各一，且踩在对应桌面的上沿（不是飘在半空或埋进地里）
    #[test]
    fn stations_cover_both_kinds_on_tables() {
        let l = layout();
        assert_eq!(l.stations.len(), 2, "应有补给台与干员切换台两个站点");
        assert!(l.stations.iter().any(|s| s.kind == StationKind::SupplyTable));
        assert!(l.stations.iter().any(|s| s.kind == StationKind::OperatorDesk));
        for s in &l.stations {
            assert!(
                (0.5..=1.2).contains(&s.pos[1]),
                "站点交互位高度异常: {:?}",
                s.pos
            );
            // 站点脚下应有台面（同 x/z 附近存在高于 0.5m 的 solid 桌面）
            let has_table = solid_props().iter().any(|p| {
                let top = p.pos[1] + p.aabb_half()[1];
                top > 0.5
                    && (p.pos[0] - s.pos[0]).abs() < 1.2
                    && (p.pos[2] - s.pos[2]).abs() < 0.6
            });
            assert!(has_table, "站点 {:?} 没有对应的桌子实体", s.pos);
        }
    }

    /// 场上应提供可拾取的武器（验证背包武器架玩法）
    #[test]
    fn weapon_pickups_available() {
        let l = layout();
        assert!(
            l.pickups.iter().any(|pk| matches!(pk.kind, PickupKind::Weapon { .. })),
            "训练场缺少武器拾取物"
        );
    }

    /// 拾取物成组排列：基础补给/手雷/武器对齐在出生区物资线上，
    /// 纵深补给与纵深手雷左右镜像成对（防止改坐标时散落回全场）
    #[test]
    fn pickups_grouped_neatly() {
        let l = layout();
        let pos = |label: &str| {
            l.pickups
                .iter()
                .find(|p| p.label == label)
                .unwrap_or_else(|| panic!("缺少拾取物: {label}"))
                .pos
        };
        for label in ["步枪弹药", "医疗包", "护甲片", "烈焰手雷", "冰霜手雷", "雷电步枪", "毒液步枪"] {
            assert!(
                (pos(label)[2] - SUPPLY_LINE_Z).abs() < 1e-3,
                "{label} 未对齐出生区物资线 z={SUPPLY_LINE_Z}"
            );
        }
        for (a, b) in [("弹药箱", "大医疗包"), ("雷电手雷", "毒素手雷")] {
            let (pa, pb) = (pos(a), pos(b));
            assert!((pa[0] + pb[0]).abs() < 1e-3, "{a} 与 {b} 未左右镜像");
            assert!((pa[2] - pb[2]).abs() < 1e-3, "{a} 与 {b} 未对齐同一纵深");
        }
    }

    /// CQB 掩体左右对称：跪姿矮墙两列镜像摆放（防止单侧加墙破坏对称）
    #[test]
    fn cqb_kneeling_walls_symmetric() {
        let count_at = |x: f32| {
            solid_props()
                .iter()
                .filter(|p| (p.pos[0] - x).abs() < 1e-3 && (p.pos[1] - 0.6).abs() < 1e-3)
                .count()
        };
        assert!(count_at(-5.5) >= 3, "左侧跪姿矮墙列缺失");
        assert_eq!(count_at(-5.5), count_at(5.5), "左右跪姿矮墙数量不一致");
    }

    /// 立体结构齐备且可攀爬：存在高台（top ≥ 2.3m）与逐级台阶，
    /// 且台阶高差不超过跳跃上限 1.1m（保证垂直机动可达）
    #[test]
    fn vertical_structures_climbable() {
        let solids = solid_props();
        // 有可站立的高台面（薄板：half_y ≤ 0.2 且 top ≥ 2.3）
        let platforms: Vec<&Prop> = solids
            .iter()
            .filter(|p| p.half[1] <= 0.2 && p.pos[1] + p.half[1] >= 2.3)
            .collect();
        assert!(platforms.len() >= 3, "高台数量不足（应含指挥台/东西回廊）");
        // 有逐级台阶（最高一级 top ≥ 1.0，且相邻台阶高差 ≤ 1.1）
        let mut steps: Vec<f32> = solids
            .iter()
            .filter(|p| p.half[1] >= 0.2 && p.pos[1] + p.half[1] <= 1.5 && p.pos[1] - p.half[1] < 0.05)
            .map(|p| p.pos[1] + p.half[1])
            .collect();
        steps.sort_by(|a, b| a.partial_cmp(b).unwrap());
        steps.dedup_by(|a, b| (*a - *b).abs() < 1e-3);
        assert!(steps.last().copied().unwrap_or(0.0) >= 1.0, "缺少可用的登台阶梯");
        for pair in steps.windows(2) {
            assert!(
                pair[1] - pair[0] <= 1.1 + 1e-3,
                "台阶高差 {} 超过跳跃上限",
                pair[1] - pair[0]
            );
        }
    }

    /// 全室内封闭性：四面边界墙齐全（覆盖原露天训练场的开口侧）
    #[test]
    fn indoor_perimeter_enclosed() {
        let walls = solid_props()
            .iter()
            .filter(|p| {
                let half = p.aabb_half();
                p.pos[1] >= 1.0 // 全高墙体
                    && (p.pos[2].abs() > HALF - 0.5 && half[0] > HALF - 0.5
                        || p.pos[0].abs() > HALF - 0.5 && half[2] > HALF - 0.5)
            })
            .count();
        assert_eq!(walls, 4, "四面边界墙应齐全，实际 {} 面", walls);
    }
}
