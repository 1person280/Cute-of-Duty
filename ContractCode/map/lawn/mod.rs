//! 搜打撤草坪训练场 —— 声明式数据定义（1×1km 平地大场）
//!
//! 专用于验证"搜 → 打 → 撤"三段式战术循环能否正常运作。玩家出生在
//! 南端（z=470）面向 -z，自南向北依次经过：
//!
//! - 出生区（z 440..500）：出生光垫、补给横排、两张功能台
//! - 搜索区（搜，z 200..440）：散布搜索目标与拾取物，全向视野
//! - 射击区（打，z -200..200）：每 100m 警戒色距离标线 + 三排射击道
//!   （x=-200/0/200）静态靶 + 横移/巡逻移动目标，支持 400m 级直射
//! - 撤离区（撤，z -500..-200）：撤离光垫 + 红色提取信标 + 撤离目标
//!
//! 场地 1×1km（半场 500m），地面用 25m 大地砖棋盘（grass 色板），
//! 全场仅四面围墙 + 两桌为 solid 掩体，实体/Draw Call 在核显可控范围。
//!
//! 布局约束（边界、出生区净空、射击道畅通、靶不埋掩体、移动靶扫掠
//! 不越界）由本模块单元测试保证 —— 改坐标前先跑 `cargo test --lib`。
//!
//! 各分区按区块融合进本模块（`layout()` 后的分节函数）：早先拆成 `spawn_zone`
//! / `search_zone` / `engage_zone` / `extract_zone` 等下划线文件，按反屎山公约
//! （文件名禁下划线 · 嵌套 ≤2 层）统一融为模块内分节函数；外圈围墙仍在
//! [`perimeter`]（单词名，无需改名）。

use crate::element::ElementType;
use crate::map::{
    GlowKind, GlowSpec, MapLayout, MaterialKind, Motion, PickupKind, PickupSpec, Prop, Shape,
    StationKind, StationSpec, TargetSpec,
};

mod perimeter;

/// 场地半径（米）：场地 1×1km
pub const HALF: f32 = 500.0;

/// 围墙净高（米）
pub const WALL_H: f32 = 3.0;

/// 出生点 z（南端）
pub const SPAWN_Z: f32 = 470.0;

/// 出生区补给线 z
pub const SUPPLY_LINE_Z: f32 = 455.0;

/// 射击区边界 z（搜索区/射击区分界）
pub const ENGAGE_BOUND_Z: f32 = 200.0;

/// 生成完整搜打撤草坪训练场布局。每次调用构建全新数据，可安全修改。
pub fn layout() -> MapLayout {
    MapLayout {
        name: "搜打撤草坪训练场".into(),
        half_extent: HALF,
        floor_tile: 25.0,
        player_spawn: [0.0, 0.0, SPAWN_Z],
        props: [
            perimeter::walls(),
            spawn_tables(),
            spawn_decor(),
            zone_markers(),
            extract_boundary(),
        ]
        .concat(),
        targets: [
            search_targets(),
            engage_targets(),
            engage_movers(),
            extract_targets(),
        ]
        .concat(),
        pickups: [
            supply_line(),
            search_pickups(),
        ]
        .concat(),
        glows: [
            pad_glow(),
            extract_glow(),
        ]
        .concat(),
        stations: stations(),
    }
}

// ============================================================================
// 出生区（z 440..500，南端）—— 玩家落地即到的安全区与补给枢纽。
//
// 包含出生光垫、出生/搜索区边界线、补给横排（弹药/医疗/护甲/四系手雷/
// 双武器）、两张功能台及对应站点与光条。玩家自 [0,0,470] 面向 -z 起步，
// 先向左近探搜索区，再进入射击区，最终撤到北端信标。
// ============================================================================

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
            pos: [-10.0, 0.4, SUPPLY_LINE_Z],
            label: "毒液步枪".into(),
            kind: PickupKind::Weapon { element: ElementType::Poison },
        },
        PickupSpec {
            pos: [10.0, 0.4, SUPPLY_LINE_Z],
            label: "雷电步枪".into(),
            kind: PickupKind::Weapon { element: ElementType::Electric },
        },
    ]
}

/// 出生点前方可交互物资箱的世界坐标（玩家自 [0,0,470] 面向 -z，箱子在其右前方 8m）。
pub const CRATE_POS: [f32; 3] = [4.0, 0.45, 462.0];

/// 功能站点登记：补给台 + 干员切换台 + 出生点物资箱（demo 侧据此生成可交互站点）
pub(crate) fn stations() -> Vec<StationSpec> {
    vec![
        StationSpec { pos: [-5.0, 0.78, SUPPLY_LINE_Z], kind: StationKind::SupplyTable, label: "补给台".into() },
        StationSpec { pos: [5.0, 0.78, SUPPLY_LINE_Z], kind: StationKind::OperatorDesk, label: "干员切换台".into() },
        // 出生点右前方的物资箱：落地可交互，开箱一次性发放弹药/医疗/护甲。
        StationSpec { pos: CRATE_POS, kind: StationKind::SupplyCrate, label: "物资箱".into() },
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

// ============================================================================
// 搜索区（搜，z 200..440）—— 玩家离开出生区后先在本区"搜索"。
//
// 平坦草坪上散布搜索目标（静态靶）与拾取物（弹药/医疗/护甲/手雷），
// 模拟侦察阶段逐一清除与收刮。本区无实心掩体，保证全向视野与直射。
// ============================================================================

/// 搜索目标：两列纵向散布，间距错开避免排成直线
pub(crate) fn search_targets() -> Vec<TargetSpec> {
    vec![
        TargetSpec { pos: [-360.0, 1.5, 400.0], label: "搜索目标".into(), motion: None },
        TargetSpec { pos: [360.0, 1.5, 400.0], label: "搜索目标".into(), motion: None },
        TargetSpec { pos: [-240.0, 1.5, 360.0], label: "搜索目标".into(), motion: None },
        TargetSpec { pos: [240.0, 1.5, 360.0], label: "搜索目标".into(), motion: None },
        TargetSpec { pos: [-120.0, 1.5, 320.0], label: "搜索目标".into(), motion: None },
        TargetSpec { pos: [120.0, 1.5, 320.0], label: "搜索目标".into(), motion: None },
        TargetSpec { pos: [-300.0, 1.5, 280.0], label: "搜索目标".into(), motion: None },
        TargetSpec { pos: [300.0, 1.5, 280.0], label: "搜索目标".into(), motion: None },
        TargetSpec { pos: [-60.0, 1.5, 240.0], label: "搜索目标".into(), motion: None },
        TargetSpec { pos: [60.0, 1.5, 240.0], label: "搜索目标".into(), motion: None },
    ]
}

/// 搜索区拾取物：左右镜像成对散布，模拟"搜刮战利品"
pub(crate) fn search_pickups() -> Vec<PickupSpec> {
    vec![
        PickupSpec { pos: [-180.0, 0.4, 380.0], label: "弹药箱".into(), kind: PickupKind::Ammo { amount: 30 } },
        PickupSpec { pos: [180.0, 0.4, 380.0], label: "大医疗包".into(), kind: PickupKind::Health { amount: 50.0 } },
        PickupSpec {
            pos: [-40.0, 0.4, 300.0],
            label: "毒素手雷".into(),
            kind: PickupKind::Grenade { element: ElementType::Poison },
        },
        PickupSpec {
            pos: [40.0, 0.4, 300.0],
            label: "雷电手雷".into(),
            kind: PickupKind::Grenade { element: ElementType::Electric },
        },
        PickupSpec { pos: [-260.0, 0.4, 260.0], label: "护甲片".into(), kind: PickupKind::Armor { amount: 30.0 } },
        PickupSpec { pos: [260.0, 0.4, 260.0], label: "弹药箱".into(), kind: PickupKind::Ammo { amount: 60 } },
    ]
}

// ============================================================================
// 射击区（打，z -200..200）—— 玩家"搜"完后进入本区进行远程交战。
//
// 每 100m 一条警戒色距离标线（200/100/0/-100/-200），三排射击道
// （x=-200/0/200）逐级抬高的静态靶 + 两具横移/巡逻移动目标。标线为
// decor（无碰撞），射击道全净空，保证 400m 级直射视线不被切断。
// ============================================================================

/// 距离标线（虚线）+ 两侧标牌柱（立柱 + 白板标距）
pub(crate) fn zone_markers() -> Vec<Prop> {
    let mut v = Vec::new();
    let lines = [
        (200.0, MaterialKind::WarningYellow),
        (100.0, MaterialKind::WarningOrange),
        (0.0, MaterialKind::WarningRed),
        (-100.0, MaterialKind::WarningOrange),
        (-200.0, MaterialKind::WarningYellow),
    ];
    for (z, mat) in lines {
        for x in -12..=12 {
            v.push(Prop::decor([x as f32 * 40.0, 0.02, z], [16.0, 0.02, 0.12], mat));
        }
        // 两侧标牌柱（不影响中部射击道净空）
        for sx in [-460.0, 460.0] {
            v.push(Prop::decor([sx, 1.5, z], [0.12, 1.5, 0.12], MaterialKind::Steel));
            v.push(Prop::decor([sx, 2.4, z + 0.2], [0.6, 0.35, 0.06], MaterialKind::PaintWhite));
        }
    }
    v
}

/// 三排射击道静态靶：近（100m）/ 中（200m）/ 远（300m），逐级抬高
pub(crate) fn engage_targets() -> Vec<TargetSpec> {
    vec![
        TargetSpec { pos: [-200.0, 1.5, 100.0], label: "射击目标".into(), motion: None },
        TargetSpec { pos: [0.0, 1.5, 100.0], label: "射击目标".into(), motion: None },
        TargetSpec { pos: [200.0, 1.5, 100.0], label: "射击目标".into(), motion: None },
        TargetSpec { pos: [-200.0, 1.8, 0.0], label: "射击目标".into(), motion: None },
        TargetSpec { pos: [0.0, 1.8, 0.0], label: "射击目标".into(), motion: None },
        TargetSpec { pos: [200.0, 1.8, 0.0], label: "射击目标".into(), motion: None },
        TargetSpec { pos: [-200.0, 2.2, -100.0], label: "射击目标".into(), motion: None },
        TargetSpec { pos: [0.0, 2.2, -100.0], label: "射击目标".into(), motion: None },
        TargetSpec { pos: [200.0, 2.2, -100.0], label: "射击目标".into(), motion: None },
    ]
}

/// 移动目标：150m 处快速横移 + 250m 处中速巡逻（扫掠不越界不穿掩体）
pub(crate) fn engage_movers() -> Vec<TargetSpec> {
    vec![
        TargetSpec {
            pos: [0.0, 1.8, 50.0],
            label: "移动目标".into(),
            motion: Some(Motion { speed: 4.0, range: 40.0, start_dir: -1.0 }),
        },
        TargetSpec {
            pos: [0.0, 1.8, -50.0],
            label: "移动目标".into(),
            motion: Some(Motion { speed: 3.0, range: 60.0, start_dir: 1.0 }),
        },
    ]
}

// ============================================================================
// 撤离区（撤，z -500..-200，北端）—— 玩家打完纵深后撤到信标完成循环。
//
// 包含撤离光垫、红色提取信标（柱 + 顶灯）、射击区/撤离区分界标线，
// 以及两具"撤离途中仍需压制"的撤离目标，测试撤退阶段的战斗反馈。
// ============================================================================

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
        TargetSpec { pos: [-150.0, 1.5, -320.0], label: "撤离目标".into(), motion: None },
        TargetSpec { pos: [150.0, 1.5, -320.0], label: "撤离目标".into(), motion: None },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::ElementType;
    use crate::map::{PickupKind, Pos, Prop, StationKind};

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
            if p.pos[1] + half[1] <= 0.5 {
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

    /// 靶不埋进掩体：静态靶与所有 solid prop 不相交
    #[test]
    fn static_targets_clear_of_solids() {
        let l = layout();
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

    /// 移动靶整个扫掠体积不与掩体相交且不越界
    #[test]
    fn moving_targets_clear_and_in_bounds() {
        let l = layout();
        for t in &l.targets {
            let Some(m) = t.motion else { continue };
            assert!(
                t.pos[0].abs() + m.range + 0.5 <= HALF && t.pos[2].abs() + 0.5 <= HALF,
                "移动靶扫掠越界: {:?} range {}",
                t.pos,
                m.range
            );
            let sweep_pos = [t.pos[0], t.pos[1], t.pos[2]];
            let sweep_half = [m.range + 0.5, 1.0, 0.5];
            for p in solid_props() {
                assert!(
                    !overlap(sweep_pos, sweep_half, p.pos, p.aabb_half()),
                    "移动靶扫掠穿过掩体 {:?}",
                    p.pos
                );
            }
        }
    }

    /// 元素手雷覆盖四系元素（搜打撤要能体验全部反应）
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

    /// 功能站点：两类桌台各一（踩在桌面上沿）+ 出生点物资箱（落地）
    #[test]
    fn stations_cover_both_kinds_on_tables() {
        let l = layout();
        assert!(l.stations.len() >= 3, "应有补给台/干员切换台/物资箱三类站点");
        assert!(l.stations.iter().any(|s| s.kind == StationKind::SupplyTable));
        assert!(l.stations.iter().any(|s| s.kind == StationKind::OperatorDesk));
        assert!(l.stations.iter().any(|s| s.kind == StationKind::SupplyCrate), "缺出生点物资箱");
        for s in &l.stations {
            // 物资箱落地摆放，不受"必须在桌面上"约束；其余桌台型站点须踩在桌面上沿。
            if s.kind == StationKind::SupplyCrate {
                assert!(s.pos[1] < 0.6, "物资箱应落地摆放: {:?}", s.pos);
                assert!(s.pos[0].abs() <= HALF && s.pos[2].abs() <= HALF, "物资箱越界: {:?}", s.pos);
                continue;
            }
            assert!((0.5..=1.2).contains(&s.pos[1]), "站点交互位高度异常: {:?}", s.pos);
            let has_table = solid_props().iter().any(|p| {
                p.pos[1] + p.aabb_half()[1] > 0.5
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
            "草坪场缺少武器拾取物"
        );
    }

    /// 三段式分区齐备：搜/打/撤各区都有目标与对应标记
    #[test]
    fn three_phase_zones_laid_out() {
        let l = layout();
        // 搜：z 200..440 内存在搜索目标
        assert!(
            l.targets.iter().any(|t| (ENGAGE_BOUND_Z..SPAWN_Z).contains(&t.pos[2])),
            "搜索区无目标"
        );
        // 打：z -200..200 内至少 9 个静态射击目标（三排×三列）
        let engage_static = l
            .targets
            .iter()
            .filter(|t| t.motion.is_none() && (-ENGAGE_BOUND_Z..=ENGAGE_BOUND_Z).contains(&t.pos[2]))
            .count();
        assert!(engage_static >= 9, "射击区静态靶不足: {engage_static}");
        // 打：至少一具移动目标
        assert!(
            l.targets.iter().any(|t| t.motion.is_some() && (-ENGAGE_BOUND_Z..=ENGAGE_BOUND_Z).contains(&t.pos[2])),
            "射击区缺移动目标"
        );
        // 撤：z < -300 存在撤离目标与红色信标
        assert!(l.targets.iter().any(|t| t.pos[2] < -300.0), "撤离区无目标");
        // 撤信标：extract_glow 提供红色 beacon（此处校验发光件落在北端）
        assert!(l.glows.iter().any(|g| g.pos[2] < -300.0), "撤离区缺信标光");
        // 出生：补给线横排上有武器
        assert!(
            l.pickups
                .iter()
                .any(|pk| matches!(pk.kind, PickupKind::Weapon { .. }) && (pk.pos[2] - SUPPLY_LINE_Z).abs() < 1e-3),
            "出生区补给线上缺武器"
        );
    }
}