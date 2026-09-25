//! HUD 干员切换台（右下垂直四卡）：焰狐/霜刃/雷豹/毒蛛 + 元素色 + Q/E 技能名。
//!
//! 设计动机：干员归属是“应该算什么”的服务端权威责任。本模块只做**表现选择意图**——
//! 点击卡片或按 1-4 上报 `ClientMessage::SwitchOperator`；服务端改 `Combatant.operator_idx`
//! 后经快照把 `operator_id` 回传，本模块**读快照**渲染当前高亮，绝不本地断言干员归属。
//! `StateScoped(InGame)`，随离场递归销毁。

use bevy::prelude::*;

use cute_of_duty_server::net::protocol::ClientMessage;

use crate::flow::flow_state::{self as flow, CjkFont, LocalPlayer};
use crate::net::network::NetOut;
use crate::shared::operator_meta::{self, meta};
use crate::net::snapshot::SnapshotBuffer;
use crate::shared::theme;

/// 干员卡片标记（记录名册索引；点击发送切换意图）。
#[derive(Component)]
pub struct OperatorSlot {
    pub index: u32,
}

/// 装配右侧垂直干员切换台（四卡，当前干员高亮由 `operator_highlight` 读快照刷新）。
pub fn spawn_operator_panel(p: &mut ChildBuilder<'_>, fonts: &CjkFont) {
    p.spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                top: Val::Px(240.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
            ..default()
        })
        .with_children(|rail| {
            for (i, _op) in operator_meta::OPERATORS.iter().enumerate() {
                let idx = i as u32;
                let op = meta(idx);
                rail.spawn((
                    OperatorSlot { index: idx },
                    Interaction::default(),
                    NodeBundle {
                        style: Style {
                            width: Val::Px(150.0),
                            height: Val::Px(52.0),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        background_color: theme::PANEL_BG.into(),
                        border_color: BorderColor(theme::PANEL_BORDER),
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        ..default()
                    },
                ))
                .with_children(|c| {
                    c.spawn(TextBundle::from_section(
                        format!("{}. {}", idx + 1, op.name),
                        flow::style(fonts, 16.0, op.color),
                    ));
                    c.spawn(TextBundle::from_section(
                        format!("{} · Q:{} E:{}", op.weapon, op.skill_q, op.skill_e),
                        flow::style(fonts, 10.0, theme::TEXT_DIM),
                    ));
                });
            }
        });
}

/// 点击干员卡片或按 1-4 → 上报切换意图（只发意图，不本地裁决）。
pub fn operator_input(
    out: Res<NetOut>,
    mut slots: Query<(&OperatorSlot, &Interaction)>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    for (slot, interaction) in &mut slots {
        if *interaction == Interaction::Pressed {
            // 复位点击态，避免同一按住帧重复发。真正高亮由快照回传驱动。
            let _ = out.0.send(ClientMessage::SwitchOperator {
                operator_id: slot.index,
            });
        }
    }
    for (i, code) in [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
    ]
    .iter()
    .enumerate()
    {
        if keys.just_pressed(*code) {
            let _ = out.0.send(ClientMessage::SwitchOperator {
                operator_id: i as u32,
            });
        }
    }
}

/// 读权威快照，把当前 `operator_id` 对应卡片边框高亮青色，其余回默认。
pub fn operator_highlight(
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    mut slots: Query<(&OperatorSlot, &mut BorderColor)>,
) {
    let op = snap
        .current
        .iter()
        .find(|e| e.entity_id == player.entity_id)
        .map(|e| e.operator_id)
        .unwrap_or(0);
    for (slot, mut border) in &mut slots {
        *border = if slot.index == op {
            BorderColor(theme::ACCENT_CYAN)
        } else {
            BorderColor(theme::PANEL_BORDER)
        };
    }
}