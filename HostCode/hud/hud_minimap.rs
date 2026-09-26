//! HUD 右下小地图：把服务端快照所有实体位置投影到一张 160×160 小图
//!
//! 设计动机：小地图是**纯表现层**——把所有 `EntitySnapshot` 的 x/z 相对本人实体做一次
//! 直角投影（-Z 为北/上），画成色点。这里不关心实体"是什么角色"，只关心"在哪"，
//! 位置来源是服务端权威快照，客户端只做地图投影，绝不本地推测实体身份/状态。

use bevy::prelude::*;

use crate::flow::flow_state::{self as flow, CjkFont, LocalPlayer};
use crate::world::model::voxel_for;
use crate::net::snapshot::SnapshotBuffer;

/// 小地图画布容器。
#[derive(Component)]
pub struct MinimapLayer;

/// 画布内的实体色点（每帧重建）。
#[derive(Component)]
pub struct MinimapDot;

/// 小地图边长（px，旧版 180）。
const MAP_SIZE: f32 = 180.0;
/// 罗盘条高度（px，旧版 26）。
const COMPASS_H: f32 = 26.0;
/// 米/像素 比例（每像素代表 2 米）。
const METERS_PER_PX: f32 = 2.0;

/// 装配小地图（左上角，旧版锚点 + 上部罗盘条）。
pub fn spawn_minimap(p: &mut ChildBuilder<'_>, fonts: &CjkFont) {
    // 罗盘条（地图上方，方位刻度文案 + 深底；不带 MinimapLayer，避免被当成画布）
    p.spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                top: Val::Px(14.0),
                width: Val::Px(MAP_SIZE),
                height: Val::Px(COMPASS_H),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::horizontal(Val::Px(10.0)),
                ..default()
            },
            background_color: Color::srgb(0.08, 0.09, 0.10).into(),
            ..default()
        })
    .with_children(|c| {
        c.spawn(TextBundle::from_section(
            "西",
            flow::style(fonts, 13.0, Color::srgb(0.6, 0.68, 0.72)),
        ));
        c.spawn(TextBundle::from_section(
            "北",
            flow::style(fonts, 15.0, Color::srgb(0.95, 0.6, 0.2)),
        ));
        c.spawn(TextBundle::from_section(
            "东",
            flow::style(fonts, 13.0, Color::srgb(0.6, 0.68, 0.72)),
        ));
    });

    // 地图画布（罗盘下方）；外框贴图已废弃，画布底色 + 过程化点位出画。
    p.spawn((
        MinimapLayer,
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                top: Val::Px(14.0 + COMPASS_H),
                width: Val::Px(MAP_SIZE),
                height: Val::Px(MAP_SIZE),
                ..default()
            },
            background_color: Color::srgb(0.08, 0.09, 0.10).into(),
            ..default()
        },
    ));
}

/// 每帧重建小地图点：清空旧点 → 读快照 → 相对本人投影画点。
///
/// 清空用 [`DespawnRecursiveExt::despawn_descendants`]（一次性掏空容器 `Children`），
/// **不能**逐个 `commands.entity(dot).despawn()`：bevy 0.14 的单实体 `despawn` 不维护父子
/// 关系，被杀的点仍留在 `MinimapLayer` 的 `Children` 列表里；每帧一轮就会累积成百上千个
/// 失效实体 ID，待递归销毁 HUD 树时逐个 `World::despawn` 命中不存在实体，刷屏
/// `error[B0003]`（`bevy_ecs-0.14.2/src/world/mod.rs`）。`despawn_descendants` 以
/// `mem::take` 摘走整份列表，既不残留失效 ID、也无需逐点排队。
pub fn update_minimap(
    mut commands: Commands,
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    layer: Query<Entity, With<MinimapLayer>>,
) {
    let Ok(container) = layer.get_single() else {
        return;
    };

    // 清空上一帧的色点（连同容器 Children 一并摘走，见函数文档）。
    commands.entity(container).despawn_descendants();

    // 本人中心（含本人投影之后，若本人消失则整图无中心，仅显示原点污点）。
    let me = snap.current.iter().find(|e| e.entity_id == player.entity_id);

    // 重新以本人为中心画全部实体点。
    commands.entity(container).with_children(|p| {
        for e in &snap.current {
            let (dx, dz) = match me {
                Some(m) => (e.x - m.x, e.z - m.z),
                None => (e.x, e.z),
            };
            // 直角投影：x→右 px，-z→up px。
            let px = MAP_SIZE / 2.0 + dx / METERS_PER_PX;
            let py = MAP_SIZE / 2.0 - dz / METERS_PER_PX;
            if px < 0.0 || px > MAP_SIZE || py < 0.0 || py > MAP_SIZE {
                continue;
            }
            let (size, color) = dot_style(e.entity_id, me.map(|m| m.entity_id), e.model_preset);
            p.spawn((
                MinimapDot,
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(px - size / 2.0),
                        top: Val::Px(py - size / 2.0),
                        width: Val::Px(size),
                        height: Val::Px(size),
                        ..default()
                    },
                    background_color: color.into(),
                    ..default()
                },
            ));
        }
    });

    let _ = &player;
}

/// 实体点样式：本人高亮青色大点；靶红；敌人深灰红；其余黄白小点。
fn dot_style(id: u64, self_id: Option<u64>, preset: cute_of_duty_server::model::ModelPreset) -> (f32, Color) {
    if Some(id) == self_id {
        return (7.0, Color::srgb(0.3, 0.9, 1.0));
    }
    let body = voxel_for(preset);
    let color = match set_preset_group(preset) {
        DotKind::Target => Color::srgb(0.95, 0.25, 0.2),
        DotKind::Enemy => Color::srgb(0.55, 0.2, 0.18),
        DotKind::Other => body.primary,
    };
    (5.0, color)
}

/// 把模型身份粗分三类，决定小地图点色（Keep 冷映射，不涉及任何状态判定）。
enum DotKind {
    Target,
    Enemy,
    Other,
}

fn set_preset_group(preset: cute_of_duty_server::model::ModelPreset) -> DotKind {
    use cute_of_duty_server::model::ModelPreset;
    match preset {
        ModelPreset::AimTarget => DotKind::Target,
        ModelPreset::EnemyThug => DotKind::Enemy,
        _ => DotKind::Other,
    }
}