//! HUD 右上击杀计数（旧版右上橙黄计数）。
//!
//! 设计动机：纯展示聚合——计数值 `KillCount` 只由 `flow::route_control_messages` 在收到
//! 服务端 `EventKind::Kill { killer_id == 本人 }` 时递增（权威在服务端），本模块只把它
//! 画到右上角。`InGame` 态由 `hud.rs` 装配，`StateScoped` 随离场销毁。

use bevy::prelude::*;

use crate::flow::flow_state::{self as flow, CjkFont, KillCount};
use crate::shared::theme;

/// 击杀计数文本标记。
#[derive(Component)]
pub struct KillCounterText;

/// 装配击杀计数（右上，旧版 `top:14,right:16`）。
pub fn spawn_kill_counter(p: &mut ChildBuilder<'_>, fonts: &CjkFont, count: &KillCount) {
    p.spawn((
        KillCounterText,
        TextBundle {
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(14.0),
                right: Val::Px(16.0),
                ..default()
            },
            text: Text::from_section(
                format!("击杀 {}", count.0),
                flow::style(fonts, 18.0, theme::KILL_AMBER),
            ),
            ..default()
        },
    ));
}

/// 每帧刷新击杀数文本（仅本人计数）。
pub fn update_kill(mut counter: Query<&mut Text, With<KillCounterText>>, count: Res<KillCount>) {
    let Ok(mut text) = counter.get_single_mut() else {
        return;
    };
    text.sections[0].value = format!("击杀 {}", count.0);
}