//! 小地图与罗盘：标记组件、坐标换算、构建与每帧更新系统

use bevy::prelude::*;
use bevy::ecs::system::SystemParam;
use crate::map::{MaterialKind, StationKind};
use crate::model::{
    Player, PlayerCamera,
};
use crate::map::lawn;
use super::frontend::*;
use super::components::*;

pub(crate) const MINIMAP_PX: f32 = 180.0;
/// 罗盘条高度（像素）
pub(crate) const COMPASS_H: f32 = 26.0;
/// 地图框边宽：绘制区内缩，避免掩体贴到边框上
pub(crate) const MAP_BORDER: f32 = 2.0;
/// 小地图绘制区边长（扣除边框）
pub(crate) const MAP_INNER: f32 = MINIMAP_PX - 2.0 * MAP_BORDER;

#[derive(Component)]
pub(crate) struct MinimapRoot;

/// 角色阵营：小地图上敌我异色
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Faction { Enemy, Teammate }

/// 小地图需要的地图数据快照。
/// 核心库不依赖 bevy，MapLayout 本身不是 Resource，由 demo 侧包一层。
#[derive(Resource)]
pub(crate) struct MinimapMap { pub(crate) half_extent: f32 }

/// 玩家定位点（白色小点）
#[derive(Component)]
pub(crate) struct MinimapPlayerDot;

/// 玩家朝向菱形（绕自身中心旋转，UI 节点 Transform 平移基准为节点中心）
#[derive(Component)]
pub(crate) struct MinimapPlayerArrow;

/// 小地图动态点池的一格：靶/拾取物/敌人/队友共用，按 kind 分组编号
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MinimapDot { pub(crate) kind: DotKind, pub(crate) slot: usize }

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DotKind { Target, Pickup, Enemy, Teammate }

/// 罗盘刻度：angle 为世界方位角（度，正北=0，顺时针增加）
#[derive(Component)]
pub(crate) struct CompassTick { pub(crate) angle: f32 }

/// 罗盘方位字（北/东/南/西），随刻度一起滚动
#[derive(Component)]
pub(crate) struct CompassLabel { pub(crate) angle: f32 }

/// 罗盘中央方位读数（如"正北 0°"）
#[derive(Component)]
pub(crate) struct CompassHeadingText;

/// 世界坐标（x 向东 / z 向南）→ 小地图像素 x/y（左上角为西北角，正北朝上）
pub(crate) fn world_to_map(v: f32, half_extent: f32) -> f32 {
    MAP_BORDER + (v + half_extent) / (2.0 * half_extent) * MAP_INNER
}

/// 归一化角度差到 [-180, 180)
pub(crate) fn wrap_deg(a: f32) -> f32 {
    a - 360.0 * ((a + 180.0) / 360.0).floor()
}

/// 相机 yaw 弧度 → 方位角弧度（正北=0，顺时针）。
/// yaw=π 时面向 -Z（正北）；yaw=π/2 面向 +X（正东）。
pub(crate) fn heading_rad(yaw: f32) -> f32 {
    (std::f32::consts::PI - yaw).rem_euclid(std::f32::consts::TAU)
}

pub(crate) fn setup_minimap(mut commands: Commands) {
    let layout = lawn::layout();
    let half = layout.half_extent;

    // 根节点：左上角，纵向排列 罗盘条 + 方形地图（边距与右上角击杀播报一致）
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(14.0),
                left: Val::Px(16.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            background_color: BackgroundColor(Color::NONE),
            ..default()
        },
        MinimapRoot,
    )).with_children(|root| {
        // ---- 罗盘条：刻度滚动，中央指针 + 读数固定 ----
        root.spawn(NodeBundle {
            style: Style {
                width: Val::Px(MINIMAP_PX),
                height: Val::Px(COMPASS_H),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.7)),
            ..default()
        }).with_children(|strip| {
            // 每 15° 一根刻度；45° 倍数加高，正方位最亮（方位字单列）
            for deg in (0..360).step_by(15) {
                let major = deg % 45 == 0;
                let cardinal = deg % 90 == 0;
                strip.spawn((
                    NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            top: Val::Px(0.0),
                            width: Val::Px(if cardinal { 3.0 } else { 2.0 }),
                            height: Val::Px(if major { 9.0 } else { 6.0 }),
                            ..default()
                        },
                        background_color: BackgroundColor(if cardinal {
                            Color::srgba(0.95, 0.95, 0.95, 0.9)
                        } else if major {
                            Color::srgba(0.85, 0.85, 0.85, 0.55)
                        } else {
                            Color::srgba(0.7, 0.7, 0.7, 0.3)
                        }),
                        ..default()
                    },
                    CompassTick { angle: deg as f32 },
                ));
            }
            // 四个方位字随刻度滚动
            for (deg, name) in [(0.0, "北"), (90.0, "东"), (180.0, "南"), (270.0, "西")] {
                strip.spawn((
                    NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            top: Val::Px(9.0),
                            width: Val::Px(14.0),
                            height: Val::Px(12.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: BackgroundColor(Color::NONE),
                        ..default()
                    },
                    CompassLabel { angle: deg },
                )).with_children(|label| {
                    label.spawn(TextBundle {
                        text: Text::from_section(
                            name,
                            TextStyle { font_size: 10.0, color: Color::srgb(0.95, 0.95, 0.95), ..default() },
                        ),
                        ..default()
                    });
                });
            }
            // 中央指针（固定）
            strip.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.0),
                    left: Val::Px(MINIMAP_PX * 0.5 - 1.0),
                    width: Val::Px(2.0),
                    height: Val::Px(8.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgb(1.0, 0.8, 0.25)),
                ..default()
            });
            // 中央方位读数（in-flow 子节点，由 justify_content 居中）
            strip.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(64.0),
                    height: Val::Px(14.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)),
                ..default()
            }).with_children(|readout| {
                readout.spawn((
                    TextBundle {
                        text: Text::from_section(
                            "正北 0°",
                            TextStyle { font_size: 10.0, color: Color::srgb(0.95, 0.95, 0.9), ..default() },
                        ),
                        ..default()
                    },
                    CompassHeadingText,
                ));
            });
        });

        // ---- 方形小地图 ----
        root.spawn(NodeBundle {
            style: Style {
                width: Val::Px(MINIMAP_PX),
                height: Val::Px(MINIMAP_PX),
                border: UiRect::all(Val::Px(MAP_BORDER)),
                overflow: Overflow::clip(),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.04, 0.05, 0.07, 0.78)),
            border_color: BorderColor(Color::srgba(0.6, 0.63, 0.67, 0.9)),
            ..default()
        }).with_children(|map| {
            // 静态掩体：一次性摆位（障碍不动）；只画有碰撞且高过膝的，
            // 标线/管道等无碰撞装饰不上图
            for prop in &layout.props {
                if !prop.solid { continue; }
                let aabb = prop.aabb_half();
                if prop.pos[1] + aabb[1] <= 0.5 { continue; }
                let scale = MAP_INNER / (2.0 * half);
                let w = (aabb[0] * 2.0 * scale).max(2.0);
                let h = (aabb[2] * 2.0 * scale).max(2.0);
                map.spawn(NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(world_to_map(prop.pos[0], half) - w * 0.5),
                        top: Val::Px(world_to_map(prop.pos[2], half) - h * 0.5),
                        width: Val::Px(w),
                        height: Val::Px(h),
                        ..default()
                    },
                    background_color: BackgroundColor(minimap_material_color(prop.material)),
                    ..default()
                });
            }
            // 功能站点：补给台（琥珀）/ 干员切换台（青）
            for station in &layout.stations {
                let color = match station.kind {
                    StationKind::SupplyTable => Color::srgb(1.0, 0.65, 0.15),
                    StationKind::OperatorDesk => Color::srgb(0.2, 0.9, 0.95),
                    StationKind::SupplyCrate => Color::srgb(1.0, 0.55, 0.12),
                };
                map.spawn(NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(world_to_map(station.pos[0], half) - 3.0),
                        top: Val::Px(world_to_map(station.pos[2], half) - 3.0),
                        width: Val::Px(6.0),
                        height: Val::Px(6.0),
                        ..default()
                    },
                    background_color: BackgroundColor(color),
                    ..default()
                });
            }
            // 动态点池：初始隐藏，minimap_update_system 每帧填充
            for (kind, count) in [
                (DotKind::Target, 12),
                (DotKind::Pickup, 16),
                (DotKind::Enemy, 4),
                (DotKind::Teammate, 4),
            ] {
                for slot in 0..count {
                    let size = dot_size(kind);
                    map.spawn((
                        NodeBundle {
                            style: Style {
                                position_type: PositionType::Absolute,
                                width: Val::Px(size),
                                height: Val::Px(size),
                                display: Display::None,
                                ..default()
                            },
                            background_color: BackgroundColor(Color::NONE),
                            ..default()
                        },
                        MinimapDot { kind, slot },
                    ));
                }
            }
            // 玩家：白色定位点 + 朝向菱形（后生成者在上层）
            map.spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(world_to_map(layout.player_spawn[0], half) - 2.0),
                        top: Val::Px(world_to_map(layout.player_spawn[2], half) - 2.0),
                        width: Val::Px(4.0),
                        height: Val::Px(4.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgb(0.95, 0.95, 0.95)),
                    ..default()
                },
                MinimapPlayerDot,
            ));
            map.spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(world_to_map(layout.player_spawn[0], half) - 5.5),
                        top: Val::Px(world_to_map(layout.player_spawn[2], half) - 5.5),
                        width: Val::Px(11.0),
                        height: Val::Px(11.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.35, 0.95, 0.6, 0.5)),
                    transform: Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
                    ..default()
                },
                MinimapPlayerArrow,
            ));
        });
    });

    commands.insert_resource(MinimapMap { half_extent: layout.half_extent });
}

/// 掩体材质 → 小地图配色（数据层只声明材质语义，颜色由表现层映射）
pub(crate) fn minimap_material_color(m: MaterialKind) -> Color {
    match m {
        MaterialKind::Concrete => Color::srgba(0.58, 0.60, 0.64, 0.85),
        MaterialKind::Rust => Color::srgba(0.56, 0.42, 0.30, 0.85),
        MaterialKind::Steel => Color::srgba(0.40, 0.43, 0.47, 0.85),
        MaterialKind::TargetRed => Color::srgba(0.85, 0.28, 0.22, 0.85),
        MaterialKind::TargetWhite | MaterialKind::PaintWhite => Color::srgba(0.80, 0.80, 0.82, 0.85),
        MaterialKind::Dark => Color::srgba(0.25, 0.26, 0.30, 0.85),
        MaterialKind::Pipe => Color::srgba(0.52, 0.47, 0.42, 0.85),
        MaterialKind::WarningYellow => Color::srgba(0.85, 0.75, 0.20, 0.85),
        MaterialKind::WarningOrange => Color::srgba(0.85, 0.50, 0.15, 0.85),
        MaterialKind::WarningRed => Color::srgba(0.85, 0.25, 0.15, 0.85),
    }
}

/// 拾取物 → 小地图点色（与场上发光色一致）
pub(crate) fn minimap_pickup_color(item: &PickupType) -> Color {
    match item {
        PickupType::Ammo { .. } => Color::srgba(0.9, 0.7, 0.2, 0.95),
        PickupType::Health { .. } => Color::srgba(0.9, 0.25, 0.25, 0.95),
        PickupType::Armor { .. } => Color::srgba(0.3, 0.55, 0.95, 0.95),
        PickupType::Grenade { element } | PickupType::Weapon { element } => element.color().with_alpha(0.95),
    }
}

pub(crate) fn dot_size(kind: DotKind) -> f32 {
    match kind {
        DotKind::Target => 5.0,
        DotKind::Pickup => 4.0,
        DotKind::Enemy | DotKind::Teammate => 6.0,
    }
}

/// 罗盘/落图的可变样式查询别名：各标记互斥的 With 过滤。
/// 用类型别名收窄长 Query 元组，规避 clippy::type_complexity。
type MinimapDotStyleQuery<'w, 's> = Query<
    'w, 's,
    (&'static MinimapDot, &'static mut Style, &'static mut BackgroundColor),
    (
        Without<MinimapPlayerDot>,
        Without<MinimapPlayerArrow>,
        Without<CompassTick>,
        Without<CompassLabel>,
    ),
>;
type MinimapTickStyleQuery<'w, 's> = Query<
    'w, 's,
    (&'static CompassTick, &'static mut Style),
    (
        Without<MinimapDot>,
        Without<MinimapPlayerDot>,
        Without<MinimapPlayerArrow>,
        Without<CompassLabel>,
    ),
>;
type MinimapLabelStyleQuery<'w, 's> = Query<
    'w, 's,
    (&'static CompassLabel, &'static mut Style),
    (
        Without<MinimapDot>,
        Without<MinimapPlayerDot>,
        Without<MinimapPlayerArrow>,
        Without<CompassTick>,
    ),
>;
type MinimapPlayerDotQuery<'w, 's> = Query<
    'w, 's,
    &'static mut Style,
    (
        With<MinimapPlayerDot>,
        Without<MinimapPlayerArrow>,
        Without<MinimapDot>,
        Without<CompassTick>,
        Without<CompassLabel>,
    ),
>;
type MinimapPlayerArrowQuery<'w, 's> = Query<
    'w, 's,
    (&'static mut Style, &'static mut Transform),
    (
        With<MinimapPlayerArrow>,
        Without<MinimapPlayerDot>,
        Without<MinimapDot>,
        Without<CompassTick>,
        Without<CompassLabel>,
        Without<Player>,
        Without<Faction>,
        Without<TargetDummy>,
        Without<PickupItem>,
    ),
>;

/// 每帧更新的只读"世界"上下文：相机、玩家、地图边界与动态实体查询。
/// 用 SystemParam 收拢只读查询与资源，规避 too_many_arguments。
#[derive(SystemParam)]
pub(crate) struct MinimapWorld<'w, 's> {
    cam_query: Query<'w, 's, &'static PlayerCamera>,
    player_query: Query<'w, 's, &'static Transform, With<Player>>,
    map: Res<'w, MinimapMap>,
    characters: Query<'w, 's, (&'static Transform, &'static Faction), Without<Player>>,
    targets: MinimapTargetsQuery<'w, 's>,
    pickups: MinimapPickupsQuery<'w, 's>,
}

/// 动态实体落图的只读查询别名：互斥 With/Without 标记隔离敌我/掩体，
/// 收窄长 Query 元组，规避 clippy::type_complexity。
type MinimapTargetsQuery<'w, 's> = Query<
    'w, 's,
    &'static Transform,
    (With<TargetDummy>, Without<Player>, Without<Faction>),
>;
type MinimapPickupsQuery<'w, 's> = Query<
    'w, 's,
    (&'static Transform, &'static PickupItem),
    (Without<Player>, Without<Faction>, Without<TargetDummy>),
>;

/// 每帧更新：罗盘滚动 + 方位读数 + 玩家/动态实体落图。
/// 各 Style 可变查询用互斥的 With 标记隔离，避免 B0001 运行时冲突。
pub(crate) fn minimap_update_system(
    world: MinimapWorld,
    mut dots: MinimapDotStyleQuery,
    mut ticks: MinimapTickStyleQuery,
    mut labels: MinimapLabelStyleQuery,
    mut player_dot: MinimapPlayerDotQuery,
    mut player_arrow: MinimapPlayerArrowQuery,
    mut heading_text: Query<&mut Text, With<CompassHeadingText>>,
) {
    let Ok(cam) = world.cam_query.get_single() else { return };
    let Ok(player) = world.player_query.get_single() else { return };
    let half = world.map.half_extent;

    // ---- 罗盘：刻度/方位字按 1px=1° 滚动，超出可视范围隐藏 ----
    let heading = heading_rad(cam.yaw);
    let heading_deg = heading.to_degrees();
    for (tick, mut style) in ticks.iter_mut() {
        let rel = wrap_deg(tick.angle - heading_deg);
        style.left = Val::Px(MINIMAP_PX * 0.5 + rel - 1.0);
        style.display = if rel.abs() <= 95.0 { Display::Flex } else { Display::None };
    }
    for (label, mut style) in labels.iter_mut() {
        let rel = wrap_deg(label.angle - heading_deg);
        style.left = Val::Px(MINIMAP_PX * 0.5 + rel - 7.0);
        style.display = if rel.abs() <= 95.0 { Display::Flex } else { Display::None };
    }

    // 中央读数：八方位名 + 角度
    const DIR_NAMES: [&str; 8] = ["正北", "东北", "正东", "东南", "正南", "西南", "正西", "西北"];
    if let Ok(mut text) = heading_text.get_single_mut() {
        let dir = DIR_NAMES[((heading_deg + 22.5) / 45.0) as usize % 8];
        text.sections[0].value = format!("{} {:.0}°", dir, heading_deg);
    }

    // ---- 玩家标记：定位点居中，菱形指向相机朝向 ----
    let px = world_to_map(player.translation.x, half);
    let py = world_to_map(player.translation.z, half);
    if let Ok(mut style) = player_dot.get_single_mut() {
        style.left = Val::Px(px - 2.0);
        style.top = Val::Px(py - 2.0);
    }
    if let Ok((mut style, mut transform)) = player_arrow.get_single_mut() {
        style.left = Val::Px(px - 5.5);
        style.top = Val::Px(py - 5.5);
        // UI 屏幕 y 向下，rotation.z 正值在屏上表现为顺时针，恰与罗盘方位一致
        transform.rotation = Quat::from_rotation_z(heading + std::f32::consts::FRAC_PI_4);
    }

    // ---- 动态实体 → 地图像素 ----
    let to_px = |t: &Transform| [world_to_map(t.translation.x, half), world_to_map(t.translation.z, half)];
    let target_px: Vec<[f32; 2]> = world.targets.iter().map(to_px).collect();
    let pickup_px: Vec<([f32; 2], Color)> = world.pickups
        .iter()
        .map(|(t, item)| (to_px(t), minimap_pickup_color(&item.item_type)))
        .collect();
    let mut enemy_px: Vec<[f32; 2]> = Vec::new();
    let mut teammate_px: Vec<[f32; 2]> = Vec::new();
    for (t, faction) in world.characters.iter() {
        match faction {
            Faction::Enemy => enemy_px.push(to_px(t)),
            Faction::Teammate => teammate_px.push(to_px(t)),
        }
    }

    for (dot, mut style, mut bg) in dots.iter_mut() {
        let entry = match dot.kind {
            DotKind::Target => target_px.get(dot.slot).map(|p| (*p, Color::srgba(0.95, 0.35, 0.3, 0.9))),
            DotKind::Pickup => pickup_px.get(dot.slot).map(|(p, c)| (*p, *c)),
            DotKind::Enemy => enemy_px.get(dot.slot).map(|p| (*p, Color::srgba(1.0, 0.3, 0.25, 0.95))),
            DotKind::Teammate => teammate_px.get(dot.slot).map(|p| (*p, Color::srgba(0.25, 0.9, 0.45, 0.95))),
        };
        if let Some(([x, y], color)) = entry {
            let s = dot_size(dot.kind) * 0.5;
            style.display = Display::Flex;
            style.left = Val::Px(x - s);
            style.top = Val::Px(y - s);
            bg.0 = color;
        } else {
            style.display = Display::None;
        }
    }
}

// =============================================================================
// HUD Setup - Polished FPS Interface
// =============================================================================

