//! CapsLock 通信延迟面板（含 TCP 三次握手的往返指标）
//!
//! 设计动机：玩家开启大写锁定即希望看到自己到服务器的通信质量。本面板持续驻留
//! （不做状态限定，随时可看），仅在 CapsLock 打开时可见。延迟指标分两类：
//! - `ConnectLatency`：连接起点 → 握手完成（含 TCP 三次握手 + 首轮往返），握手中一次性测得；
//! - `LiveLatency`：常态 Ping/Pong 往返，由**网络线程**每 ~1s 探测一次并打点。
//! 当前仅显示 `test` 玩家一行（其余玩家接入后再扩展）。

use bevy::prelude::*;

use crate::flow::flow_state::{self as flow, CjkFont, ConnectLatency, LiveLatency};

/// 面板可见性开关（CapsLock 切换）。
#[derive(Resource, Default)]
pub struct LatencyShow(pub bool);

/// 面板根标记（持久驻留，非状态限定）。
#[derive(Component)]
pub struct LatencyPanel;

/// 面板内容文本（刷新延迟指标用）。
#[derive(Component)]
pub struct LatencyText;

/// 面板是否已生成（避免每帧重复 spawn 时重复建文本）。
#[derive(Resource, Default)]
pub struct PanelSpawned(pub bool);

/// 生成永久延迟面板（字体就绪且尚未生成时执行一次；不随状态销毁）。
pub fn spawn_panel(mut commands: Commands, fonts: Res<CjkFont>, spawned: Res<PanelSpawned>) {
    if fonts.0.is_none() || spawned.0 {
        return;
    }
    commands.insert_resource(PanelSpawned(true));
    commands
        .spawn((
            LatencyPanel,
            NodeBundle {
                visibility: Visibility::Hidden,
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(4.0),
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                ..default()
            },
        ))
        .with_children(|p| {
            p.spawn((
                LatencyText,
                TextBundle::from_section("", flow::style(&fonts, 18.0, Color::WHITE)),
            ));
        });
}

/// CapsLock 按下切换面板显隐。
pub fn caps_toggle(keys: Res<ButtonInput<KeyCode>>, mut show: ResMut<LatencyShow>) {
    if keys.just_pressed(KeyCode::CapsLock) {
        show.0 = !show.0;
    }
}

/// 同步面板显隐并刷新延迟文本（只显示 `test` 玩家一行）。
///
/// 探测由网络线程承担（见 `run_pull_loop`）——RTT 只在网络线程打点，不含 Bevy 帧时间，
/// 因此即便客户端掉帧，面板显示的仍是真实链路往返。
pub fn panel_update(
    show: Res<LatencyShow>,
    connect: Res<ConnectLatency>,
    live: Res<LiveLatency>,
    mut panelq: Query<&mut Visibility, With<LatencyPanel>>,
    mut text_q: Query<&mut Text, With<LatencyText>>,
) {
    // 1) 显隐跟随 CapsLock
    if let Ok(mut visibility) = panelq.get_single_mut() {
        *visibility = if show.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // 2) 刷新文本内容（连接含握手 + 常态 RTT）
    if let Ok(mut text) = text_q.get_single_mut() {
        text.sections[0].value = format!(
            "test   连接(含握手) {:.1} ms    RTT {:.1} ms",
            connect.ms, live.rtt_ms
        );
    }
}