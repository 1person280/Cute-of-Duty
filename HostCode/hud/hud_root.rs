//! 训练场 HUD：整屏根节点装配 + 撤离引导与撤离请求
//!
//! 设计动机：HUD 只读权威快照中本人条目（`LocalPlayer.entity_id`）来展示，血量/护甲/
//! 弹药/技能 CD 全部为服务端裁决值，客户端不加改。本文件只做「装配根节点 + 撤离引导」；
//! vitals / minimap / skills 的具体刷新各拆到独立子模块（保持每个 .rs ≤600 行）。
//! 撤离的距离提示为纯表现层引导，**进入判定**由服务端按玩家世界坐标裁决。

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy::ui::UiImage;
use cute_of_duty_server::net::protocol::ClientMessage;

use crate::flow::flow_state::{self as flow, AppState, CjkFont, KillCount, LocalPlayer, EXTRACTION_POINT, EXTRACTION_RANGE};
use crate::net::network::NetOut;
use crate::net::snapshot::SnapshotBuffer;
use crate::shared::ui_assets::UiAssets;

/// HUD 整屏根标记（`StateScoped(InGame)` 随离场自动销毁）。
#[derive(Component)]
pub struct HudRoot;

/// 撤离引导文本（顶部中央，指示是否已进入撤离区）。
#[derive(Component)]
pub struct ExtractLabel;

/// 生成 HUD：左下 vitals、右上击杀、右下弹药/技能、左上小地图、居中准星、顶部撤离引导。
pub fn spawn_hud(mut commands: Commands, fonts: Res<CjkFont>, ui: Res<UiAssets>, kills: Res<KillCount>) {
    if fonts.0.is_none() {
        return;
    }
    commands
        .spawn((
            HudRoot,
            StateScoped(AppState::InGame),
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                ..default()
            },
        ))
        .with_children(|p| {
            super::hud_vitals::spawn_vitals(p, &fonts, &ui);
            super::hud_skills::spawn_skills(p, &fonts, &ui);
            super::hud_minimap::spawn_minimap(p, &fonts, &ui);
            super::hud_crosshair::spawn_crosshair(p);
            super::hud_kill_counter::spawn_kill_counter(p, &fonts, &kills);
            super::hud_operator_panel::spawn_operator_panel(p, &fonts);
            spawn_extract_label(p, &fonts);
        });
}

/// 撤离引导：顶部中央，图标（可无）+ 距离/提示文本。
fn spawn_extract_label(p: &mut ChildBuilder, fonts: &CjkFont) {
    p.spawn((
        ExtractLabel,
        UiImage::default(),
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(48.0),
                left: Val::Percent(50.0),
                ..default()
            },
            ..default()
        },
    ))
    .with_children(|label| {
        label.spawn(TextBundle::from_section(
            "前往北端撤离区",
            flow::style(&fonts, 20.0, Color::srgb(1.0, 0.95, 0.6)),
        ));
    });
}

/// 每帧刷新撤离引导（距离/进入提示）。共用本模块，避免跨模块再读一遍快照。
pub fn update_extract(
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    mut extract: Query<&mut Text, With<ExtractLabel>>,
) {
    let snapshot = snap
        .current
        .iter()
        .find(|e| e.entity_id == player.entity_id);

    let (dx, dz) = match snapshot {
        Some(e) => (e.x - EXTRACTION_POINT.0, e.z - EXTRACTION_POINT.1),
        None => (0.0, 0.0),
    };
    let dist = (dx * dx + dz * dz).sqrt();
    let in_zone = dist <= EXTRACTION_RANGE;

    if let Ok(mut text) = extract.get_single_mut() {
        text.sections[0].value = if in_zone {
            "已到撤离区 · 按 Enter 撤离".to_string()
        } else {
            format!("前往北端撤离区  距离 {dist:.0} m")
        };
    }
}

/// InGame 内按 Enter 发起撤离请求（是否成功由服务端裁决）。
pub fn extract_interaction(
    keys: Res<ButtonInput<KeyCode>>,
    out: Res<NetOut>,
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
) {
    if !keys.just_pressed(KeyCode::Enter) {
        return;
    }
    let Some(e) = snap
        .current
        .iter()
        .find(|e| e.entity_id == player.entity_id)
    else {
        return;
    };
    let dx = e.x - EXTRACTION_POINT.0;
    let dz = e.z - EXTRACTION_POINT.1;
    if (dx * dx + dz * dz).sqrt() <= EXTRACTION_RANGE {
        let _ = out.0.send(ClientMessage::ExtractRequest);
    }
}

/// 挂接贴图：把可选项变成一个图片子节点（未就绪返回空节点不显示图，数值照常）。
///
/// 0.14 的图片节点由独立的 `UiImage` 组件承载（`NodeBundle` 无 `image` 字段），
/// 故这里把 `UiImage` 与 `NodeBundle` 组合成一个子实体。`p` 为容器 `EntityCommands`，
/// 通过 `with_children` 挂子级。
pub fn optional_image(
    p: &mut EntityCommands,
    ui: &UiAssets,
    handle: Option<Handle<Image>>,
    size: Val,
    height: Val,
) {
    let image = if ui.ready {
        handle.map(UiImage::new).unwrap_or_default()
    } else {
        UiImage::default()
    };
    p.with_children(|c| {
        c.spawn((
            NodeBundle {
                style: Style {
                    width: size,
                    height,
                    ..default()
                },
                ..default()
            },
            image,
        ));
    });
}