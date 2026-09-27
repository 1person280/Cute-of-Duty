//! 持雷提示（HUD）：按本人快照 `held_grenade` 派生「手持手雷 — 左键投掷 · Esc 取消」。
//!
//! 设计动机（Why）：手雷采用 legacy「先瞄准后释放」流程——服务端把取出后的手雷置入
//! **持握态**（快照字段 `held_grenade`），此时左键=投掷、Esc=取消放回。玩家必须有明确
//! 反馈，否则"取出后没有手雷飞出"会被误读为无响应。文案直接沿用 legacy 0.3.2
//! `demo/inventory/held_grenade.rs` 的固定提示，保持还原度。
//!
//! 顺带承载派生资源 [`HeldGrenadeState`]：供相机（强制越肩）、上行（左键脉冲 / 取消）、
//! 门控（Esc 不被软开关抢走）三处共用，避免各自重复解析快照、口径漂移。

use bevy::prelude::*;
use cute_of_duty_server::element::ElementType;

use crate::flow::flow_state::{self as flow, CjkFont, LocalPlayer};
use crate::net::snapshot::SnapshotBuffer;
use crate::shared::theme;

/// legacy 0.3.2 持雷提示文案（逐字沿用）。
const HINT: &str = "手持手雷 — 左键投掷 · Esc 取消";

/// 本人是否正持握手雷（由快照逐帧同步；`element` 为 `Some` 即持握中）。
///
/// 设计动机（Why）：持雷态是三处表现的公共输入——相机越肩 / 上行左键脉冲 / Esc 门控。
/// 收敛为单一资源，避免各系统各读一遍快照导致"同一帧三种结论"。
#[derive(Resource, Default)]
pub struct HeldGrenadeState {
    /// 持握手雷的元素（`None` = 未持雷）。
    pub element: Option<ElementType>,
}

/// 持雷提示整行根节点（默认隐藏）。
#[derive(Component)]
pub struct GrenadeHintRoot;

/// 装配持雷提示（屏幕下方居中，默认隐藏；显隐由 [`sync_held_grenade`] 派生）。
pub fn spawn_grenade_hint(p: &mut ChildBuilder<'_>, fonts: &CjkFont) {
    p.spawn((
        GrenadeHintRoot,
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                bottom: Val::Px(126.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            visibility: Visibility::Hidden,
            ..default()
        },
    ))
    .with_children(|row| {
        row.spawn(NodeBundle {
            style: Style {
                padding: UiRect::axes(Val::Px(16.0), Val::Px(6.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            background_color: theme::PANEL_BG.into(),
            border_color: BorderColor(theme::ACCENT_AMBER),
            ..default()
        })
        .with_children(|card| {
            card.spawn(TextBundle::from_section(
                HINT,
                flow::style(fonts, 16.0, theme::ACCENT_AMBER),
            ));
        });
    });
}

/// 由本人快照同步 [`HeldGrenadeState`] 并派生提示显隐（每帧；本人条目缺失时视为未持雷）。
pub fn sync_held_grenade(
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    mut state: ResMut<HeldGrenadeState>,
    mut root: Query<&mut Visibility, With<GrenadeHintRoot>>,
) {
    let held = snap
        .current
        .iter()
        .find(|e| e.entity_id == player.entity_id)
        .and_then(|e| e.held_grenade);
    state.element = held;
    if let Ok(mut vis) = root.get_single_mut() {
        *vis = if held.is_some() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// 离开训练场复位持雷态（否则下次进场可能带着"持雷中"影响相机越肩与 Esc 门控）。
pub fn reset_held_grenade(mut state: ResMut<HeldGrenadeState>) {
    state.element = None;
}
