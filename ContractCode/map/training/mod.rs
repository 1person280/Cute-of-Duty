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

use crate::map::MapLayout;

// 各分区子模块：按地图区域拆分，逐个生成布局分片
mod building_shell;
mod cqb_hall;
mod second_floor;
mod shooting_range;
mod spawn_room;

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
        name: "CQB室内训练场",
        half_extent: HALF,
        floor_tile: 1.0,
        player_spawn: [0.0, 0.0, 10.5],
        props: [
            building_shell::perimeter(),
            spawn_room::spawn_partitions(),
            cqb_hall::hall_partitions(),
            shooting_range::range_arch(),
            cqb_hall::hall_cover(),
            spawn_room::spawn_cover(),
            shooting_range::lane_markers(),
            cqb_hall::command_platform(),
            second_floor::walkways(),
            spawn_room::spawn_tables(),
            cqb_hall::hall_tables(),
            spawn_room::spawn_line_decor(),
            building_shell::shell_decor(),
        ]
        .concat(),
        targets: [
            shooting_range::static_targets(),
            cqb_hall::hall_mover(),
            shooting_range::range_movers(),
        ]
        .concat(),
        pickups: [
            spawn_room::supply_line(),
            cqb_hall::hall_supply(),
            shooting_range::range_supply(),
        ]
        .concat(),
        glows: [
            building_shell::neon(),
            spawn_room::beacon_and_spawn_pad(),
            spawn_room::station_glow(),
        ]
        .concat(),
        stations: spawn_room::stations(),
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