//! HUD 左下 vitals：干员名 / 血条 / 护甲条 / Q·E 技能图标 / 道具槽
//!
//! 设计动机：Vitals 只读权威快照中本人条目。旧版的血条底图贴图已废弃（1920² 概念美术与
//! 体素低模方向相悖），改为**过程化**填充条 + 数值文本；技能图标同样是过程化方块
//! （底色 + 自下而上的冷却填充 + 读秒），不依赖任何贴图，风格与全局 theme 一致。
//!
//! 冷却填充比例由**客户端读服务端干员名册** `operator::roster()` 的 `cooldown_secs` 作分母
//! 算出（名册是冷数据，与 `operator_id` 一一对应）；剩余冷却值本身来自权威快照
//! `skill_cd_q/e`，客户端不推演游戏状态。
//!
//! 面板内所有可变访问都收敛到两个组件枚举（[`VitalsBar`] / [`VitalsText`]）上，每个系统
//! 只查询一种组件类型，从结构上杜绝 bevy 0.14 同一组件 `&mut` 多查询导致的 B0001 冲突。

use bevy::prelude::*;
use cute_of_duty_server::items::{ItemCategory, LootItem};
use cute_of_duty_server::operator::roster;

use crate::flow::flow_state::{self as flow, CjkFont, LocalPlayer};
use crate::net::snapshot::SnapshotBuffer;
use crate::shared::operator_meta::meta;
use crate::shared::theme;

/// 血条 / 护甲 / 技能冷却填充条（按变体分别更新宽度或高度）。
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum VitalsBar {
    /// 血条填充（宽度 = hp%）
    Health,
    /// 护甲条填充（宽度 = armor%）
    Armor,
    /// Q 技能冷却填充（高度 = cd/max_cd，自下而上）
    SkillQ,
    /// E 技能冷却填充（高度 = cd/max_cd，自下而上）
    SkillE,
}

/// 面板内各文本（按变体刷新内容/配色）。
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum VitalsText {
    /// 干员名
    OperatorName,
    /// `HP cur / max` 数值
    Hp,
    /// `ARMOR cur / max` 数值
    Armor,
    /// Q 技能图标中央读数
    SkillQ,
    /// E 技能图标中央读数
    SkillE,
    /// 道具槽（键位 3 / 4）：显示该类消耗品首件名 + 计数
    ItemSlot(u8),
}

/// HP / 护甲显示上限（表现层母板）。服务端玩家 `max_hp = 100`，护甲无独立上限字段，
/// 此处按同一量级取 100 显示；若服务端调整上限，改这一处即可。
const MAX_HP: f32 = 100.0;
const MAX_ARMOR: f32 = 100.0;
/// 技能图标边长（px，旧版 52）。
const SKILL_ICON: f32 = 52.0;
/// 道具槽尺寸（px）：够宽以容纳"物品名 ×N"。
const ITEM_W: f32 = 86.0;
const ITEM_H: f32 = 44.0;

/// 血条红。
const HP_RED: Color = Color::srgb(0.90, 0.18, 0.18);
/// 护甲蓝。
const ARMOR_BLUE: Color = Color::srgb(0.25, 0.55, 0.95);
/// 技能冷却填充色（琥珀，半透明）。
const SKILL_CD_FILL: Color = Color::srgba(1.0, 0.72, 0.2, 0.55);

/// 装配 vitals 面板（左下角：干员名 → HP 180×22 → 护甲 140×10 → Q/E 图标 → 道具槽）。
pub fn spawn_vitals(p: &mut ChildBuilder<'_>, fonts: &CjkFont) {
    p.spawn(NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            bottom: Val::Px(16.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        background_color: theme::PANEL_BG.into(),
        ..default()
    })
    .with_children(|v| {
        // 干员名
        v.spawn((
            VitalsText::OperatorName,
            TextBundle::from_section("干员 #0", flow::style(fonts, 20.0, theme::TEXT_WHITE)),
        ));

        // 血条 180×22 + 居中数值
        let mut hp = v.spawn(NodeBundle {
            style: Style {
                width: Val::Px(180.0),
                height: Val::Px(22.0),
                justify_content: JustifyContent::FlexStart,
                ..default()
            },
            background_color: Color::srgb(0.15, 0.13, 0.13).into(),
            ..default()
        });
        hp.with_children(|h| {
            h.spawn((
                VitalsBar::Health,
                NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    background_color: HP_RED.into(),
                    ..default()
                },
            ));
            // 数值覆盖层（绝对定位铺满血条、内容居中）
            h.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ..default()
            })
            .with_children(|c| {
                c.spawn((
                    VitalsText::Hp,
                    TextBundle::from_section(
                        "HP 100 / 100",
                        flow::style(fonts, 13.0, theme::TEXT_WHITE),
                    ),
                ));
            });
        });

        // 护甲条 140×10 + 数值
        v.spawn(NodeBundle {
            style: Style {
                width: Val::Px(140.0),
                height: Val::Px(10.0),
                justify_content: JustifyContent::FlexStart,
                ..default()
            },
            background_color: Color::srgb(0.10, 0.13, 0.17).into(),
            ..default()
        })
        .with_children(|a| {
            a.spawn((
                VitalsBar::Armor,
                NodeBundle {
                    style: Style {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    background_color: ARMOR_BLUE.into(),
                    ..default()
                },
            ));
        });
        v.spawn((
            VitalsText::Armor,
            TextBundle::from_section("ARMOR 0 / 100", flow::style(fonts, 12.0, theme::TEXT_DIM)),
        ));

        // 技能图标行：Q / E 两个 52×52 方块（冷却填充 + 读秒 + 底部键位标签）
        v.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                ..default()
            },
            ..default()
        })
        .with_children(|row| {
            spawn_skill_icon(row, fonts, "Q", VitalsBar::SkillQ, VitalsText::SkillQ);
            spawn_skill_icon(row, fonts, "E", VitalsBar::SkillE, VitalsText::SkillE);
        });

        // 道具槽行：键位 3 / 4（物品名 + 计数来自权威快照的 4×3 背包格位）
        v.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                ..default()
            },
            ..default()
        })
        .with_children(|row| {
            for slot in [3u8, 4u8] {
                row.spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Px(ITEM_W),
                            height: Val::Px(ITEM_H),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        background_color: Color::srgba(0.14, 0.16, 0.20, 0.9).into(),
                        border_color: BorderColor(theme::PANEL_BORDER),
                        ..default()
                    },
                ))
                .with_children(|c| {
                    c.spawn(TextBundle::from_section(
                        format!("{slot}"),
                        flow::style(fonts, 11.0, theme::TEXT_DIM),
                    ));
                    c.spawn((
                        VitalsText::ItemSlot(slot),
                        TextBundle::from_section("—", flow::style(fonts, 13.0, theme::TEXT_DIM)),
                    ));
                });
            }
        });
    });
}

/// 单个技能图标：面板底 + 自下而上的冷却填充 + 中央读数 + 底部键位标签。
fn spawn_skill_icon(
    row: &mut ChildBuilder<'_>,
    fonts: &CjkFont,
    key: &str,
    bar: VitalsBar,
    text: VitalsText,
) {
    row.spawn(NodeBundle {
        style: Style {
            width: Val::Px(SKILL_ICON),
            height: Val::Px(SKILL_ICON),
            border: UiRect::all(Val::Px(1.0)),
            overflow: Overflow::clip(),
            ..default()
        },
        background_color: Color::srgba(0.10, 0.12, 0.16, 0.9).into(),
        border_color: BorderColor(theme::PANEL_BORDER),
        ..default()
    })
    .with_children(|icon| {
        // 冷却填充：贴底、高度随剩余冷却比例
        icon.spawn((
            bar,
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(0.0),
                    ..default()
                },
                background_color: SKILL_CD_FILL.into(),
                ..default()
            },
        ));
        // 中央读数（钟面）
        icon.spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ..default()
        })
        .with_children(|c| {
            c.spawn((
                text,
                TextBundle::from_section("就绪", flow::style(fonts, 13.0, theme::TEXT_WHITE)),
            ));
            c.spawn(TextBundle::from_section(
                key,
                flow::style(fonts, 11.0, theme::ACCENT_AMBER),
            ));
        });
    });
}

/// 每帧刷新条类控件：血条/护甲宽度、技能冷却填充高度。
pub fn update_vitals_bars(
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    mut bars: Query<(&VitalsBar, &mut Style)>,
) {
    let snap_entry = snap.current.iter().find(|e| e.entity_id == player.entity_id);
    let (hp_f, armor_f, cd_q, cd_e, op) = match snap_entry {
        Some(e) => (
            (e.hp / MAX_HP).clamp(0.0, 1.0),
            (e.armor / MAX_ARMOR).clamp(0.0, 1.0),
            e.skill_cd_q,
            e.skill_cd_e,
            e.operator_id,
        ),
        None => (1.0, 0.0, 0.0, 0.0, 0),
    };
    let (max_q, max_e) = skill_cooldowns(op);

    for (bar, mut style) in &mut bars {
        match bar {
            VitalsBar::Health => style.width = Val::Percent(hp_f * 100.0),
            VitalsBar::Armor => style.width = Val::Percent(armor_f * 100.0),
            VitalsBar::SkillQ => style.height = Val::Percent(cd_fraction(cd_q, max_q) * 100.0),
            VitalsBar::SkillE => style.height = Val::Percent(cd_fraction(cd_e, max_e) * 100.0),
        }
    }
}

/// 每帧刷新文本类控件：干员名、HP/ARMOR 数值、技能读秒、道具槽计数。
pub fn update_vitals_text(
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    mut texts: Query<(&VitalsText, &mut Text)>,
) {
    let snap_entry = snap.current.iter().find(|e| e.entity_id == player.entity_id);
    let (hp, armor, cd_q, cd_e, op) = match snap_entry {
        Some(e) => (e.hp, e.armor, e.skill_cd_q, e.skill_cd_e, e.operator_id),
        None => (MAX_HP, 0.0, 0.0, 0.0, 0),
    };
    // 3/4 号槽的标签与计数由权威背包格位派生（3 = 恢复类、4 = 战术类）。
    let backpack = snap_entry.and_then(|e| e.backpack.as_deref());
    let om = meta(op);

    for (kind, mut text) in &mut texts {
        match *kind {
            VitalsText::OperatorName => {
                text.sections[0].value = om.name.to_string();
                text.sections[0].style.color = om.color;
            }
            VitalsText::Hp => {
                text.sections[0].value = format!("HP {:.0} / {:.0}", hp.max(0.0), MAX_HP);
            }
            VitalsText::Armor => {
                text.sections[0].value = format!("ARMOR {:.0} / {:.0}", armor.max(0.0), MAX_ARMOR);
            }
            VitalsText::SkillQ => {
                text.sections[0].value = cd_readout(cd_q, om.skill_q);
            }
            VitalsText::SkillE => {
                text.sections[0].value = cd_readout(cd_e, om.skill_e);
            }
            VitalsText::ItemSlot(slot) => {
                let category = if slot == 3 {
                    ItemCategory::Consumable
                } else {
                    ItemCategory::Tactical
                };
                let (name, n) = category_readout(backpack, category);
                text.sections[0].value = match name {
                    Some(name) if n > 0 => format!("{name} ×{n}"),
                    _ => "—".to_string(),
                };
                text.sections[0].style.color = if n == 0 {
                    theme::TEXT_DIM
                } else {
                    text_color_for(slot)
                };
            }
        }
    }
}

/// 统计背包中某速用类别的（首件展示名, 件数）；空背包返回 `(None, 0)`。
fn category_readout(
    backpack: Option<&[Option<LootItem>]>,
    category: ItemCategory,
) -> (Option<String>, i32) {
    let mut first: Option<String> = None;
    let mut n = 0i32;
    if let Some(slots) = backpack {
        for item in slots.iter().flatten() {
            if item.kind.category() == Some(category) {
                n += 1;
                if first.is_none() {
                    first = Some(item.label.clone());
                }
            }
        }
    }
    (first, n)
}

/// 消耗品计数文字色：医疗包偏绿、手雷偏琥珀，便于快速区分 3/4 号槽。
fn text_color_for(slot: u8) -> Color {
    if slot == 3 {
        Color::srgb(0.55, 0.9, 0.6)
    } else {
        Color::srgb(1.0, 0.78, 0.35)
    }
}

/// 取某干员的（Q, E）技能满冷却秒数（读服务端名册冷数据；越界回退一号干员）。
fn skill_cooldowns(operator_id: u32) -> (f32, f32) {
    let roster = roster();
    let op = roster.get(operator_id as usize).or_else(|| roster.first());
    match op {
        Some(op) => (op.q.cooldown_secs, op.e.cooldown_secs),
        None => (1.0, 1.0),
    }
}

/// 剩余冷却 → 填充比例（满冷却 = 1，就绪 = 0）。
fn cd_fraction(cd: f32, max_cd: f32) -> f32 {
    if max_cd <= 0.0 {
        return 0.0;
    }
    (cd / max_cd).clamp(0.0, 1.0)
}

/// 技能读数：冷却中显示秒数，就绪显示技能名。
fn cd_readout(cd: f32, skill_name: &str) -> String {
    if cd > 0.0 {
        format!("{cd:.1}")
    } else {
        skill_name.to_string()
    }
}
