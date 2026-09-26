//! HUD 全屏告警层：低血量屏幕边缘红光
//!
//! 设计动机（Why）：血量是**服务端权威**字段，本层只按快照 `hp` 渲染视觉告警，
//! 不做任何"是否濒死"的判定——阈值 30（满血 100 的 30%）只是表现层的取景参数，
//! 改这一处即可，不影响玩法裁决（旧版 `flash.rs` 的 `alpha=0.15*(1-hp/0.3)` 同源）。
//!
//! 边缘光用**三层同心边框**近似渐晕：越靠外越宽越淡、最内层最亮。bevy UI 无原生
//! 径向渐变，用三层不同宽度/透明度的方框叠加即可得到接近"边缘渗透红光"的观感，
//! 且全程无贴图，符合本项目过程化美术方向。三层共用组件 [`HudEdgeGlow`]（带权重），
//! 由同一系统一次性刷新，避免多查询。

use bevy::prelude::*;

use crate::flow::flow_state::LocalPlayer;
use crate::net::snapshot::SnapshotBuffer;

/// 屏幕边缘红光层（`weight` 越大越亮，用于三层同心叠加出渐晕）。
#[derive(Component, Clone, Copy)]
pub struct HudEdgeGlow {
    /// 该层相对强度权重（最内层最亮）。
    pub weight: f32,
}

/// 低血告警阈值：血量低于该值开始出现边缘红光（满血 100 的 30%）。
const HP_CRIT: f32 = 30.0;
/// 最深处的红光峰值透明度。
const PEAK_ALPHA: f32 = 0.5;
/// 红光基色（暗红，压在画面上不刺眼）。
const ALERT_RED: (f32, f32, f32) = (0.85, 0.08, 0.08);

/// 单层边框宽度与权重（外→内：宽而淡 → 窄而亮）。
const LAYERS: [(f32, f32); 3] = [(72.0, 0.30), (44.0, 0.62), (18.0, 1.0)];

/// 装配三层边缘光（默认全透明；低血时由 `update_alert` 点亮）。
pub fn spawn_alert_overlay(p: &mut ChildBuilder<'_>) {
    for (width, weight) in LAYERS {
        p.spawn((
            HudEdgeGlow { weight },
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    border: UiRect::all(Val::Px(width)),
                    ..default()
                },
                background_color: Color::NONE.into(),
                border_color: BorderColor(Color::NONE),
                visibility: Visibility::Hidden,
                ..default()
            },
        ));
    }
}

/// 每帧按血量刷新边缘红光强度（血量越低越亮；高于阈值时整层隐藏）。
///
/// `hp` 直接取权威快照中本人条目；无快照（未进场）时视为满血 → 不显示告警。
pub fn update_alert(
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    mut glows: Query<(&HudEdgeGlow, &mut BorderColor, &mut Visibility)>,
) {
    let hp = snap
        .current
        .iter()
        .find(|e| e.entity_id == player.entity_id)
        .map(|e| e.hp)
        .unwrap_or(100.0);

    // 强度：hp ∈ [0, HP_CRIT) 线性映射到 (0, 1]；hp ≥ HP_CRIT 时不告警。
    let intensity = if hp < HP_CRIT { (1.0 - hp / HP_CRIT).clamp(0.0, 1.0) } else { 0.0 };

    for (glow, mut border, mut vis) in &mut glows {
        if intensity <= 0.0 {
            *vis = Visibility::Hidden;
            continue;
        }
        let alpha = (PEAK_ALPHA * intensity * glow.weight).clamp(0.0, 1.0);
        border.0 = Color::srgba(ALERT_RED.0, ALERT_RED.1, ALERT_RED.2, alpha);
        *vis = Visibility::Visible;
    }
}
