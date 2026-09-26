//! HUD 右下：技能 CD + 元素状态 + 武器名/弹药；以及顶部公告/击杀流
//!
//! 设计动机：技能冷却/元素状态/弹药/武器归属都是**服务端裁决**的热字段（`skill_cd_q/e`、
//! `element_state`、`ammo`、`operator_id`），客户端只读快照展示；武器名/干员元素色来自
//! 稳定的 `operator_meta` 展示母板（不含任何判定）。通告流读 `flow::Announcements`。

use bevy::prelude::*;

use crate::flow::flow_state::{self as flow, Announcements, CjkFont, LocalPlayer};
use crate::shared::operator_meta::meta;
use crate::net::snapshot::SnapshotBuffer;
use crate::shared::theme;

/// Q 技能冷却文本。
#[derive(Component)]
pub struct SkillQText;
/// E 技能冷却文本。
#[derive(Component)]
pub struct SkillEText;
/// 元素附着状态文本。
#[derive(Component)]
pub struct ElemText;
/// 当前武器名文本。
#[derive(Component)]
pub struct WeaponText;
/// 弹药读数文本。
#[derive(Component)]
pub struct AmmoText;
/// 通告/击杀流文本。
#[derive(Component)]
pub struct FeedText;

/// 装配右下武器/弹药/技能面板 + 顶部公告流。
pub fn spawn_skills(p: &mut ChildBuilder<'_>, fonts: &CjkFont) {
    // 右下：武器名 → 弹药 → Q → E → 元素（旧版右下弹药/武器 + 技能指示）
    p.spawn(NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            right: Val::Px(16.0),
            bottom: Val::Px(16.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        background_color: theme::PANEL_BG.into(),
        ..default()
    })
    .with_children(|s| {
        s.spawn((
            WeaponText,
            TextBundle::from_section(
                "烈焰步枪",
                flow::style(fonts, 20.0, theme::ACCENT_CYAN),
            ),
        ));
        s.spawn((
            AmmoText,
            TextBundle::from_section(
                "弹药 --",
                flow::style(fonts, 30.0, theme::TEXT_WHITE),
            ),
        ));
        s.spawn((
            SkillQText,
            TextBundle::from_section(
                "Q 技能 · 就绪",
                flow::style(fonts, 16.0, theme::TEXT_DIM),
            ),
        ));
        s.spawn((
            SkillEText,
            TextBundle::from_section(
                "E 技能 · 就绪",
                flow::style(fonts, 16.0, theme::TEXT_DIM),
            ),
        ));
        s.spawn((
            ElemText,
            TextBundle::from_section(
                "元素 · 无",
                flow::style(fonts, 15.0, Color::srgb(0.7, 0.9, 0.7)),
            ),
        ));
    });

    // 顶部公告流：右上为击杀数、左上为小地图，故移至中上（避开两者）。
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

/// 每帧刷新武器名 / 弹药 / 技能 CD / 元素状态。
///
/// 各 `&mut Text` 查询以交叉 `Without` 互斥标记证明不相交，避免 B0001 可变冲突。
#[allow(clippy::type_complexity)]
pub fn update_skills(
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    mut weapon: Query<&mut Text, (With<WeaponText>, Without<AmmoText>, Without<SkillQText>, Without<SkillEText>, Without<ElemText>)>,
    mut ammo: Query<&mut Text, (With<AmmoText>, Without<WeaponText>, Without<SkillQText>, Without<SkillEText>, Without<ElemText>)>,
    mut q: Query<&mut Text, (With<SkillQText>, Without<SkillEText>, Without<ElemText>, Without<WeaponText>, Without<AmmoText>)>,
    mut e_q: Query<&mut Text, (With<SkillEText>, Without<SkillQText>, Without<ElemText>, Without<WeaponText>, Without<AmmoText>)>,
    mut elem: Query<&mut Text, (With<ElemText>, Without<SkillQText>, Without<SkillEText>, Without<WeaponText>, Without<AmmoText>)>,
) {
    let snapshot = snap
        .current
        .iter()
        .find(|e| e.entity_id == player.entity_id);
    let (cd_q, cd_e, elem_state, ammo_cur, op) = match snapshot {
        Some(e) => (
            e.skill_cd_q,
            e.skill_cd_e,
            Some(e.element_state.clone()),
            e.ammo,
            e.operator_id,
        ),
        None => (0.0, 0.0, None, -1, 0),
    };
    let om = meta(op);

    if let Ok(mut t) = weapon.get_single_mut() {
        t.sections[0].value = om.weapon.to_string();
        t.sections[0].style.color = om.color;
    }
    if let Ok(mut t) = ammo.get_single_mut() {
        let s = if ammo_cur < 0 { "--".to_string() } else { format!("{ammo_cur}") };
        t.sections[0].value = format!("弹药 {s}");
    }
    if let Ok(mut t) = q.get_single_mut() {
        t.sections[0].value = if cd_q <= 0.0 {
            "Q 技能 · 就绪".to_string()
        } else {
            format!("Q 技能 · {cd_q:.1}s")
        };
    }
    if let Ok(mut t) = e_q.get_single_mut() {
        t.sections[0].value = if cd_e <= 0.0 {
            "E 技能 · 就绪".to_string()
        } else {
            format!("E 技能 · {cd_e:.1}s")
        };
    }
    if let Ok(mut t) = elem.get_single_mut() {
        t.sections[0].value = match elem_state {
            Some(st) if st != default_element_state() => format!("元素 · {st:?}"),
            _ => "元素 · 无".to_string(),
        };
    }
}

/// `EntityElementState` 的"正常/无附着"默认值（此处只需判断是否出示，不构造全枚举）。
fn default_element_state() -> cute_of_duty_server::element::EntityElementState {
    use cute_of_duty_server::element::EntityElementState;
    EntityElementState::Normal
}

/// 刷新通告/击杀流：取 `Announcements` 队列末尾最近几条。
pub fn update_feed(
    announces: Res<Announcements>,
    mut feed: Query<&mut Text, (With<FeedText>, Without<SkillQText>, Without<SkillEText>, Without<ElemText>, Without<WeaponText>, Without<AmmoText>)>,
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