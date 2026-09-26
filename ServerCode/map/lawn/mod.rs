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

use crate::map::MapLayout;

mod engage_zone;
mod extract_zone;
mod perimeter;
mod search_zone;
mod spawn_zone;

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
        name: "搜打撤草坪训练场",
        half_extent: HALF,
        floor_tile: 25.0,
        player_spawn: [0.0, 0.0, SPAWN_Z],
        props: [
            perimeter::walls(),
            spawn_zone::spawn_tables(),
            spawn_zone::spawn_decor(),
            engage_zone::zone_markers(),
            extract_zone::extract_boundary(),
        ]
        .concat(),
        targets: [
            search_zone::search_targets(),
            engage_zone::engage_targets(),
            engage_zone::engage_movers(),
            extract_zone::extract_targets(),
        ]
        .concat(),
        pickups: [
            spawn_zone::supply_line(),
            search_zone::search_pickups(),
        ]
        .concat(),
        glows: [
            spawn_zone::pad_glow(),
            extract_zone::extract_glow(),
        ]
        .concat(),
        stations: spawn_zone::stations(),
    }
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
