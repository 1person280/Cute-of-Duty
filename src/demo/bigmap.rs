//! 战术全景图（按 M 打开）—— 全屏俯瞰当前活动地图。
//!
//! 与左上角小地图不同，本视图是"全貌总览"：一整块放大的方形地图，画出
//! 搜打撤四段分区色带、边界线、撤离信标、全部静态目标/拾取物/功能站点，
//! 以及实时玩家定位点与朝向箭头。打开时释放光标（自动禁用射击/交互），
//! 再按 M 或 Esc 关闭并锁回光标。
//!
//! 数据来源同世界渲染：`crate::map::lawn`（纯数据，无 bevy 依赖），
//! 因此地图改布局后总览自动同步。

use bevy::prelude::*;
use bevy::window::{CursorOptions, PrimaryWindow};
use crate::map::{PickupKind, StationKind};
use crate::map::lawn;
use crate::model::{Player, PlayerCamera};
use super::frontend::*;
use super::components::*;
use super::inventory::HeldGrenade;
use super::minimap::{heading_rad, MinimapMap};

/// 全景图是否打开（打开时释放光标，禁用射击/交互）
#[derive(Resource, Default)]
pub(crate) struct BigMapOpen {
    pub(crate) open: bool,
}

/// 全景图根节点（整屏覆盖层）
#[derive(Component)]
pub(crate) struct BigMapRoot;

/// 全景图玩家定位点（白色小点）
#[derive(Component)]
pub(crate) struct BigMapPlayerDot;

/// 全景图玩家朝向箭头（菱形，绕自身中心旋转）
#[derive(Component)]
pub(crate) struct BigMapPlayerArrow;

/// 全景地图边长（像素）
const BIGMAP_PX: f32 = 600.0;
/// 地图框边宽
const BIGMAP_BORDER: f32 = 3.0;
/// 绘制区边长（扣除边框）
const BIGMAP_INNER: f32 = BIGMAP_PX - 2.0 * BIGMAP_BORDER;

/// 世界坐标（x 向东 / z 向南）→ 全景图内像素（左上角为西北，正北朝上）
fn world_to_px(v: f32, half: f32) -> f32 {
    BIGMAP_BORDER + (v + half) / (2.0 * half) * BIGMAP_INNER
}

/// 一个 z 区间色带的像素框（自 [z_bottom, z_top] → top/height）
fn band_top_height(z_top: f32, z_bottom: f32, half: f32) -> (f32, f32) {
    let top = world_to_px(z_top, half);
    let bottom = world_to_px(z_bottom, half);
    (top, bottom - top)
}

/// 搜打撤四段分区的色带（z 区间 + 标签 + 半透明底色）
const ZONES: [(&str, f32, f32, [f32; 3]); 4] = [
    ("出生区", 500.0, 440.0, [0.35, 0.85, 0.35]),   // 南端绿
    ("搜索区·搜", 440.0, 200.0, [0.95, 0.75, 0.2]), // 黄
    ("射击区·打", 200.0, -200.0, [0.95, 0.4, 0.2]), // 橙
    ("撤离区·撤", -200.0, -500.0, [0.9, 0.2, 0.2]),  // 北端红
];

/// 分区边界线 z（出生/搜索、搜索/射击、射击/撤离）
const BOUNDARY_Z: [f32; 3] = [440.0, 200.0, -200.0];

pub(crate) fn setup_bigmap(mut commands: Commands, mut open: ResMut<BigMapOpen>) {
    *open = BigMapOpen { open: false };
    let layout = lawn::layout();
    let half = layout.half_extent;

    // 整屏覆盖层：暗色背景 + 垂直居中
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.6)),
            Visibility::Hidden,
            BigMapRoot,
        ))
        .with_children(|root| {
            // 标题栏
            root.spawn((
                Text::new("战术全景图 · 搜打撤草坪训练场（1×1km）· 按 M / Esc 关闭"),
                TextFont { font_size: FontSize::Px(16.0), ..default() },
                TextColor(Color::srgb(0.95, 0.95, 0.9)),
            ));

            // 方形地图
            root.spawn((
                Node {
                    width: Val::Px(BIGMAP_PX),
                    height: Val::Px(BIGMAP_PX),
                    border: UiRect::all(Val::Px(BIGMAP_BORDER)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.04, 0.05, 0.07, 0.9)),
                BorderColor::all(Color::srgba(0.6, 0.63, 0.67, 0.95)),
            ))
            .with_children(|map| {
                // ---- 分区色带 + 标签（最底层）----
                for (label, z_top, z_bottom, rgb) in ZONES {
                    let (top, height) = band_top_height(z_top, z_bottom, half);
                    map.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(BIGMAP_BORDER),
                            top: Val::Px(top),
                            width: Val::Px(BIGMAP_INNER),
                            height: Val::Px(height),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(rgb[0], rgb[1], rgb[2], 0.16)),
                    ));
                    // 色带中央标签
                    let z_mid = (z_top + z_bottom) * 0.5;
                    map.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(BIGMAP_BORDER + 6.0),
                            top: Val::Px(world_to_px(z_mid, half) - 8.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
                    ))
                    .with_children(|t| {
                        t.spawn((
                            Text::new(label),
                            TextFont { font_size: FontSize::Px(12.0), ..default() },
                            TextColor(Color::srgb(0.95, 0.95, 0.9)),
                        ));
                    });
                }

                // ---- 分区边界线（虚线状细条）----
                for z in BOUNDARY_Z {
                    map.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(BIGMAP_BORDER),
                            top: Val::Px(world_to_px(z, half) - 1.0),
                            width: Val::Px(BIGMAP_INNER),
                            height: Val::Px(2.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.95, 0.95, 0.95, 0.7)),
                    ));
                }

                // ---- 撤离信标（北端红色，比普通目标更大）----
                map.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(world_to_px(0.0, half) - 4.0),
                        top: Val::Px(world_to_px(-440.0, half) - 4.0),
                        width: Val::Px(8.0),
                        height: Val::Px(8.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(1.0, 0.2, 0.15)),
                ));

                // ---- 围墙（有碰撞且高过膝的 solid 掩体）----
                for prop in &layout.props {
                    if !prop.solid {
                        continue;
                    }
                    let aabb = prop.aabb_half();
                    if prop.pos[1] + aabb[1] <= 0.5 {
                        continue;
                    }
                    let scale = BIGMAP_INNER / (2.0 * half);
                    let w = (aabb[0] * 2.0 * scale).max(2.0);
                    let h = (aabb[2] * 2.0 * scale).max(2.0);
                    map.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(world_to_px(prop.pos[0], half) - w * 0.5),
                            top: Val::Px(world_to_px(prop.pos[2], half) - h * 0.5),
                            width: Val::Px(w),
                            height: Val::Px(h),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.58, 0.6, 0.64, 0.85)),
                    ));
                }

                // ---- 静态目标（红芯小点）----
                for t in &layout.targets {
                    map.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(world_to_px(t.pos[0], half) - 2.5),
                            top: Val::Px(world_to_px(t.pos[2], half) - 2.5),
                            width: Val::Px(5.0),
                            height: Val::Px(5.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.95, 0.35, 0.3, 0.95)),
                    ));
                }

                // ---- 拾取物（按类型配色）----
                for pk in &layout.pickups {
                    let color = match pk.kind {
                        PickupKind::Ammo { .. } => Color::srgba(0.9, 0.7, 0.2, 0.95),
                        PickupKind::Health { .. } => Color::srgba(0.9, 0.25, 0.25, 0.95),
                        PickupKind::Armor { .. } => Color::srgba(0.45, 0.6, 0.95, 0.95),
                        PickupKind::Grenade { element } | PickupKind::Weapon { element } => element.color().with_alpha(0.95),
                    };
                    map.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(world_to_px(pk.pos[0], half) - 2.0),
                            top: Val::Px(world_to_px(pk.pos[2], half) - 2.0),
                            width: Val::Px(4.0),
                            height: Val::Px(4.0),
                            ..default()
                        },
                        BackgroundColor(color),
                    ));
                }

                // ---- 功能站点（补给=琥珀 / 干员=青）----
                for st in &layout.stations {
                    let color = match st.kind {
                        StationKind::SupplyTable => Color::srgb(1.0, 0.65, 0.15),
                        StationKind::OperatorDesk => Color::srgb(0.2, 0.9, 0.95),
                        StationKind::SupplyCrate => Color::srgb(1.0, 0.55, 0.12),
                    };
                    map.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(world_to_px(st.pos[0], half) - 3.0),
                            top: Val::Px(world_to_px(st.pos[2], half) - 3.0),
                            width: Val::Px(6.0),
                            height: Val::Px(6.0),
                            ..default()
                        },
                        BackgroundColor(color),
                    ));
                }

                // ---- 玩家：白色定位点 + 朝向箭头（后生成者在上层）----
                let px = world_to_px(layout.player_spawn[0], half);
                let py = world_to_px(layout.player_spawn[2], half);
                map.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(px - 2.0),
                        top: Val::Px(py - 2.0),
                        width: Val::Px(4.0),
                        height: Val::Px(4.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.95, 0.95, 0.95)),
                    BigMapPlayerDot,
                ));
                map.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(px - 5.5),
                        top: Val::Px(py - 5.5),
                        width: Val::Px(11.0),
                        height: Val::Px(11.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.35, 0.95, 0.6, 0.5)),
                    Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
                    BigMapPlayerArrow,
                ));
            });

            // 图例说明
            root.spawn((
                Text::new("绿=出生 · 黄=搜索(搜) · 橙=射击(打) · 红=撤离(撤)　白点=玩家 · 红点=目标 · 彩点=拾取物"),
                TextFont { font_size: FontSize::Px(12.0), ..default() },
                TextColor(Color::srgb(0.85, 0.87, 0.9)),
            ));
        });
}

/// M 打开/关闭全景图；打开时释放光标（自动禁用射击/交互），关闭时锁回。
pub(crate) fn bigmap_toggle(
    mut keyboard: ResMut<ButtonInput<KeyCode>>,
    mut bigmap: ResMut<BigMapOpen>,
    mut root: Query<&mut Visibility, With<BigMapRoot>>,
    mut windows: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut input_state: ResMut<InputState>,
    wheel: Res<WheelState>,
    held: Res<HeldGrenade>,
    open_station: Res<OpenStation>,
    crate_win: Res<super::supply_crate::CrateWindow>,
    backpack_ui: Query<&Visibility, (With<InventoryUI>, Without<BigMapRoot>)>,
) {
    if bigmap.open {
        // 打开态：M 或 Esc 关闭；消费按键避免与全局光标切换/其它 Esc 处理冲突
        if keyboard.just_pressed(KeyCode::KeyM) || keyboard.just_pressed(KeyCode::Escape) {
            keyboard.clear_just_pressed(KeyCode::KeyM);
            keyboard.clear_just_pressed(KeyCode::Escape);
            bigmap.open = false;
            lock_cursor(&mut windows, &mut input_state);
            if let Ok(mut v) = root.single_mut() {
                *v = Visibility::Hidden;
            }
        }
        return;
    }
    // 关闭态：仅 M 打开，且不与其它 UI 状态叠加
    if !keyboard.just_pressed(KeyCode::KeyM) {
        return;
    }
    if wheel.open || held.item.is_some()
        || *open_station != OpenStation::None
        || crate_win.crate_entity.is_some()
        || backpack_ui.single().map_or(false, |v| *v == Visibility::Visible)
    {
        return;
    }
    keyboard.clear_just_pressed(KeyCode::KeyM);
    bigmap.open = true;
    unlock_cursor(&mut windows, &mut input_state);
    if let Ok(mut v) = root.single_mut() {
        *v = Visibility::Visible;
    }
}

/// 每帧更新玩家定位点与朝向箭头（地图静态内容不重复摆位）。
pub(crate) fn bigmap_update_system(
    cam_query: Query<&PlayerCamera>,
    player_query: Query<&Transform, (With<Player>, Without<BigMapPlayerArrow>)>,
    map_half: Option<Res<MinimapMap>>,
    mut dot: Query<&mut Node, (With<BigMapPlayerDot>, Without<BigMapPlayerArrow>)>,
    mut arrow: Query<(&mut Node, &mut Transform), (With<BigMapPlayerArrow>, Without<BigMapPlayerDot>)>,
) {
    let (Ok(cam), Ok(player)) = (cam_query.single(), player_query.single()) else { return };
    let half = map_half.map_or(lawn::layout().half_extent, |m| m.half_extent);
    let px = world_to_px(player.translation.x, half);
    let py = world_to_px(player.translation.z, half);
    if let Ok(mut node) = dot.single_mut() {
        node.left = Val::Px(px - 2.0);
        node.top = Val::Px(py - 2.0);
    }
    if let Ok((mut node, mut transform)) = arrow.single_mut() {
        node.left = Val::Px(px - 5.5);
        node.top = Val::Px(py - 5.5);
        let heading = heading_rad(cam.yaw);
        transform.rotation = Quat::from_rotation_z(heading + std::f32::consts::FRAC_PI_4);
    }
}