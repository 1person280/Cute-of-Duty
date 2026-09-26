//! HUD 右下：武器槽 [1]/[2] + 大字弹药 + 备用弹池 + 换弹提示 + 元素状态；以及顶部公告流
//!
//! 设计动机：右下面板对齐旧版"武器槽 → 大字弹药 → 状态行"的三段结构（[1]/[2] 槽位、
//! `当前 / 容量` 大字、`RELOADING` 提示），但配色/字号仍走本项目的 `theme` 过程化风格。
//! 所有数值（弹药/弹夹容量/备用池/换弹剩余/武器归属/元素附着）都来自**权威快照**——
//! `ammo`/`ammo_max`/`ammo_pool`/`reload_remaining`/`operator_id`/`element_state`，
//! 客户端只做显示，绝不推演（换弹进度条也照读服务端剩余秒数）。
//!
//! 低弹告警（[`LOW_AMMO`] 以下）在此就地做红色闪烁：它同时需要弹药值与文本句柄，
//! 放在同一系统内可避免再起一个查询与门控，也符合"告警贴在它关心的控件旁"的组织原则。
//!
//! 面板内可变文本访问全部收敛到单一组件枚举 [`RightText`]，一个系统一种组件类型，
//! 从结构上规避 bevy 0.14 同组件 `&mut` 多查询导致的 B0001 冲突。

use bevy::prelude::*;
use cute_of_duty_server::element::ElementType;
use cute_of_duty_server::operator::rifle_profile;

use crate::flow::flow_state::{self as flow, Announcements, CjkFont, LocalPlayer};
use crate::net::snapshot::SnapshotBuffer;
use crate::shared::theme;

/// 右下面板内各文本（单一枚举 + 单查询，规避可变文本查询冲突）。
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum RightText {
    /// 武器槽位（1 = 主武器名，2 = 副武器占位）
    WeaponSlot(u8),
    /// 大字弹药当前值
    AmmoMain,
    /// 大字弹药分母 ` / 容量`
    AmmoMax,
    /// 备用弹药池
    AmmoPool,
    /// 换弹提示行（`RELOADING x.xs`）
    Reload,
    /// 元素附着状态
    Element,
}

/// 通告/击杀流文本。
#[derive(Component)]
pub struct FeedText;

/// 低弹阈值：弹药低于等于该值时大字弹药红色闪烁告警（对齐旧版 `<=5`）。
const LOW_AMMO: i32 = 5;
/// 低弹闪烁频率（Hz）—— 0.5s 一次明暗翻转，兼顾醒目与不刺眼。
const BLINK_HZ: f32 = 3.0;

/// 大字弹药告警红（比血条红略亮，便于在面板底色上读清）。
const AMMO_ALERT: Color = Color::srgb(0.95, 0.25, 0.20);

/// 装配右下武器/弹药面板 + 顶部公告流。
pub fn spawn_skills(p: &mut ChildBuilder<'_>, fonts: &CjkFont) {
    p.spawn(NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            right: Val::Px(16.0),
            bottom: Val::Px(16.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        background_color: theme::PANEL_BG.into(),
        ..default()
    })
    .with_children(|s| {
        // 武器槽行：[1] 主武器名 / [2] 副武器占位
        s.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(6.0),
                ..default()
            },
            ..default()
        })
        .with_children(|row| {
            spawn_weapon_slot(row, fonts, 1, "烈焰步枪");
            spawn_weapon_slot(row, fonts, 2, "冰霜步枪");
        });

        // 大字弹药行：`当前` (24px) + ` / 容量` (13px，基线对齐更稳)
        s.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::FlexEnd,
                column_gap: Val::Px(4.0),
                ..default()
            },
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                RightText::AmmoMain,
                TextBundle::from_section("--", flow::style(fonts, 34.0, theme::TEXT_WHITE)),
            ));
            row.spawn((
                RightText::AmmoMax,
                TextBundle::from_section("/ --", flow::style(fonts, 16.0, theme::TEXT_DIM)),
            ));
        });

        // 备用弹药池
        s.spawn((
            RightText::AmmoPool,
            TextBundle::from_section("备用 --", flow::style(fonts, 13.0, theme::TEXT_DIM)),
        ));

        // 换弹提示（未换弹时留空，避免面板抖动）
        s.spawn((
            RightText::Reload,
            TextBundle::from_section("", flow::style(fonts, 14.0, theme::ACCENT_AMBER)),
        ));

        // 元素附着状态
        s.spawn((
            RightText::Element,
            TextBundle::from_section("元素 · 无", flow::style(fonts, 13.0, theme::TEXT_DIM)),
        ));
    });

    // 顶部公告流：右上为击杀数、左上为小地图，故移至左中上（避开两者）。
    p.spawn((
        FeedText,
        TextBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(24.0),
                top: Val::Px(230.0),
                max_width: Val::Px(420.0),
                ..default()
            },
            text: Text::from_section("", flow::style(fonts, 16.0, theme::TEXT_DIM)),
            ..default()
        },
    ));
}

/// 单个武器槽：方形键位徽标 + 槽内武器名。
fn spawn_weapon_slot(row: &mut ChildBuilder<'_>, fonts: &CjkFont, slot: u8, name: &str) {
    row.spawn(NodeBundle {
        style: Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(4.0),
            padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        background_color: Color::srgba(0.14, 0.16, 0.20, 0.9).into(),
        border_color: BorderColor(theme::PANEL_BORDER),
        ..default()
    })
    .with_children(|b| {
        b.spawn(TextBundle::from_section(
            format!("{slot}"),
            flow::style(fonts, 12.0, theme::ACCENT_AMBER),
        ));
        b.spawn((
            RightText::WeaponSlot(slot),
            TextBundle::from_section(name, flow::style(fonts, 15.0, theme::TEXT_DIM)),
        ));
    });
}

/// 每帧刷新武器槽 / 大字弹药 / 备用池 / 换弹提示 / 元素状态。
///
/// 低弹闪烁在此就地结算：仅当"有弹且低于阈值且未在换弹"时按 3Hz 翻转告警色，
/// 其余情况恢复常色——读的是服务端弹药，不本地扣减。
pub fn update_skills(
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    time: Res<Time>,
    mut texts: Query<(&RightText, &mut Text)>,
) {
    let snapshot = snap.current.iter().find(|e| e.entity_id == player.entity_id);
    let (ammo, ammo_max, ammo_pool, reload, weapons, active_slot, elem_state) = match snapshot {
        Some(e) => (
            e.ammo,
            e.ammo_max,
            e.ammo_pool,
            e.reload_remaining,
            e.weapon_elements,
            e.active_slot,
            Some(e.element_state.clone()),
        ),
        None => (-1, -1, -1, 0.0, None, 0, None),
    };
    // 闪烁相位：3Hz 方波（亮 1/6s、暗 1/6s 交替）。
    let blink_on = ((time.elapsed_seconds() * BLINK_HZ * 2.0) as u32) % 2 == 0;
    let low_ammo = ammo > 0 && ammo <= LOW_AMMO && reload <= 0.0;

    for (kind, mut text) in &mut texts {
        match *kind {
            // 双武器槽：按服务端下发的武器元素取名/配色，当前手持槽高亮（元素亮色），
            // 另一槽压暗——直观体现"同一时间只有一把可用"。
            RightText::WeaponSlot(n) => {
                let idx = n.saturating_sub(1) as usize;
                let (value, color) = match weapons.and_then(|w| w.get(idx).copied()) {
                    Some(element) => {
                        let name = rifle_profile(element).name.to_string();
                        let color = if idx as u8 == active_slot {
                            weapon_color(element)
                        } else {
                            theme::TEXT_DIM
                        };
                        (name, color)
                    }
                    None => ("—".to_string(), theme::TEXT_DIM),
                };
                text.sections[0].value = value;
                text.sections[0].style.color = color;
            }
            RightText::AmmoMain => {
                text.sections[0].value = if ammo < 0 { "--".to_string() } else { format!("{ammo}") };
                text.sections[0].style.color = if low_ammo && blink_on {
                    AMMO_ALERT
                } else if low_ammo {
                    theme::KILL_AMBER
                } else {
                    theme::TEXT_WHITE
                };
            }
            RightText::AmmoMax => {
                text.sections[0].value =
                    if ammo_max < 0 { "/ --".to_string() } else { format!("/ {ammo_max}") };
            }
            RightText::AmmoPool => {
                text.sections[0].value =
                    if ammo_pool < 0 { "备用 --".to_string() } else { format!("备用 {ammo_pool}") };
            }
            RightText::Reload => {
                text.sections[0].value = if reload > 0.0 {
                    format!("RELOADING {reload:.1}s")
                } else {
                    String::new()
                };
                text.sections[0].style.color = theme::ACCENT_AMBER;
            }
            RightText::Element => {
                let (value, color) = element_readout(elem_state.as_ref());
                text.sections[0].value = value;
                text.sections[0].style.color = color;
            }
        }
    }
}

/// 元素 → 武器槽高亮色（与场景元素配色一致，用于区分当前手持武器）。
fn weapon_color(e: ElementType) -> Color {
    match e {
        ElementType::Fire => Color::srgb(0.95, 0.45, 0.15),
        ElementType::Ice => Color::srgb(0.35, 0.75, 0.95),
        ElementType::Electric => Color::srgb(0.90, 0.80, 0.25),
        ElementType::Poison => Color::srgb(0.45, 0.80, 0.30),
        ElementType::Physical => Color::srgb(0.75, 0.75, 0.78),
        ElementType::Water => Color::srgb(0.30, 0.55, 0.90),
    }
}

/// 元素附着读数：`Normal`（或无快照）显示"无"，其余显示状态名并配元素色。
fn element_readout(state: Option<&cute_of_duty_server::element::EntityElementState>) -> (String, Color) {
    use cute_of_duty_server::element::EntityElementState as S;
    match state {
        Some(S::Burning) => ("元素 · 燃烧".to_string(), Color::srgb(1.0, 0.45, 0.2)),
        Some(S::Frozen) => ("元素 · 冰冻".to_string(), Color::srgb(0.5, 0.85, 1.0)),
        Some(S::Electrified) => ("元素 · 感电".to_string(), Color::srgb(0.75, 0.6, 1.0)),
        Some(S::Poisoned) => ("元素 · 中毒".to_string(), Color::srgb(0.6, 0.95, 0.4)),
        Some(S::Wet) => ("元素 · 潮湿".to_string(), Color::srgb(0.5, 0.7, 1.0)),
        Some(other) => (format!("元素 · {other:?}"), theme::TEXT_DIM),
        None => ("元素 · 无".to_string(), theme::TEXT_DIM),
    }
}

/// 刷新通告/击杀流：取 `Announcements` 队列末尾最近几条。
pub fn update_feed(
    announces: Res<Announcements>,
    mut feed: Query<&mut Text, (With<FeedText>, Without<RightText>)>,
) {
    let Ok(mut text) = feed.get_single_mut() else {
        return;
    };
    let msg = announces
        .0
        .iter()
        .rev()
        .take(3)
        .rev()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    text.sections[0].value = msg;
}
