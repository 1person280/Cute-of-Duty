//! HUD 左上角小地图 + 罗盘：以玩家为中心的局部放大图（半径≈[`MINIMAP_RANGE`] 米）
//!
//! 设计动机：小地图是**纯表现层**——静态层直接读服务端 `map::lawn::layout()` 的纯数据
//! （不触碰任何模拟逻辑），动态层读权威 `EntitySnapshot`（位置/模型身份是服务端裁决值）。
//!
//! 投影采用**以玩家为中心的局部放大**：只画玩家周围约一个 AOI（60m）半径内的内容，
//! 玩家恒居中。相比旧版"整图全览"，1km 场地压到 180px 会把身边 15m 的补给箱压进
//! 玩家点自身的几像素内而被完全遮挡；局部放大后 1m≈1.47px，附近的拾取物/站点清晰可辨。
//! 静态掩体/站点随玩家滚动（每帧重投影），不随图缩放。
//!
//! 罗盘机制移植自 0.3.2 `demo/minimap.rs`：每 15° 一根刻度、45° 加高、正方位最亮；
//! 方位字（北/东/南/西）随刻度滚动；1px=1°；|相对角|<=95° 才显示；中央固定指针 +
//! 八方位读数。朝向换算 `heading_rad(yaw) = (π - yaw) mod 2π`，与相机 yaw 同源。

use bevy::prelude::*;
use cute_of_duty_server::map::{lawn, MaterialKind, StationKind};

use crate::flow::flow_state::{self as flow, AimRig, CjkFont, LocalPlayer};
use crate::world::model::voxel_for;
use crate::net::snapshot::SnapshotBuffer;

/// 小地图画布边长（px，旧版 180）。
const MAP_SIZE: f32 = 180.0;
/// 罗盘条高度（px，旧版 26）。
const COMPASS_H: f32 = 26.0;
/// 地图框边宽（px，绘制区内缩，避免掩体贴边）。
const MAP_BORDER: f32 = 2.0;
/// 小地图绘制区边长（扣除边框）。
const MAP_INNER: f32 = MAP_SIZE - 2.0 * MAP_BORDER;
/// 局部放大半径（米）：画的是一道 AOI 半径内的内容，玩家恒在正中。
const MINIMAP_RANGE: f32 = 60.0;
/// 世界米 → 小地图像素（176px 覆盖 120m）。
const MAP_SCALE: f32 = MAP_INNER / (2.0 * MINIMAP_RANGE);
/// 画布中心像素坐标（玩家恒在此处）。
const MAP_CENTER: f32 = MAP_BORDER + MAP_INNER * 0.5;

/// 小地图画布容器（静态层 + 动态点容器 + 玩家标记都挂其下）。
#[derive(Component)]
pub struct MinimapLayer;

/// 动态实体点容器（每帧被 `despawn_descendants` 掏空重建）。
#[derive(Component)]
pub struct MinimapDots;

/// 画布内的动态实体色点（每帧重建）。
#[derive(Component)]
pub struct MinimapDot;

/// 静态掩体/站点点位：缓存世界坐标与像素尺寸，每帧按玩家中心重投影（仅平移）。
#[derive(Component)]
pub struct MinimapStatic {
    pub wx: f32,
    pub wz: f32,
    pub w: f32,
    pub h: f32,
}

/// 玩家定位白点（恒居中，仅装配时定位）。
#[derive(Component)]
pub struct MinimapPlayerDot;

/// 玩家朝向菱形（恒居中，仅旋转）。
#[derive(Component)]
pub struct MinimapPlayerArrow;

/// 罗盘刻度（`angle` 为世界方位角，度，正北=0 顺时针）。
#[derive(Component)]
pub struct CompassTick {
    pub angle: f32,
}

/// 罗盘方位字（随刻度滚动）。
#[derive(Component)]
pub struct CompassLabel {
    pub angle: f32,
}

/// 罗盘中央方位读数文本。
#[derive(Component)]
pub struct CompassHeadingText;

/// 世界坐标 → 小地图像素：以 `center` 为中心按 [`MAP_SCALE`] 放大。
fn project(v: f32, center: f32) -> f32 {
    MAP_CENTER + (v - center) * MAP_SCALE
}

/// 归一化角度差到 [-180, 180)。
fn wrap_deg(a: f32) -> f32 {
    a - 360.0 * ((a + 180.0) / 360.0).floor()
}

/// 相机 yaw 弧度 → 方位角弧度（正北=0，顺时针）。`yaw=π` 面向 -Z（正北）。
fn heading_rad(yaw: f32) -> f32 {
    (std::f32::consts::PI - yaw).rem_euclid(std::f32::consts::TAU)
}

/// 掩体材质 → 小地图配色（数据层只声明材质语义，颜色由表现层映射）。
fn material_color(m: MaterialKind) -> Color {
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

/// 功能站点 → 配色（补给台琥珀 / 干员切换台青 / 物资箱橙）。
fn station_color(kind: StationKind) -> Color {
    match kind {
        StationKind::SupplyTable => Color::srgb(1.0, 0.65, 0.15),
        StationKind::OperatorDesk => Color::srgb(0.2, 0.9, 0.95),
        StationKind::SupplyCrate => Color::srgb(1.0, 0.55, 0.12),
    }
}

/// 装配小地图（罗盘条 + 方形局部图 + 静态掩体/站点 + 居中玩家标记）。
pub fn spawn_minimap(p: &mut ChildBuilder<'_>, fonts: &CjkFont) {
    let layout = lawn::layout();

    // ---- 罗盘条：刻度滚动，中央指针 + 读数固定 ----
    p.spawn(NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            top: Val::Px(14.0),
            width: Val::Px(MAP_SIZE),
            height: Val::Px(COMPASS_H),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            overflow: Overflow::clip(),
            ..default()
        },
        background_color: Color::srgba(0.05, 0.06, 0.08, 0.7).into(),
        ..default()
    })
    .with_children(|strip| {
        // 每 15° 一根刻度；45° 倍数加高，正方位最亮
        for deg in (0..360).step_by(15) {
            let major = deg % 45 == 0;
            let cardinal = deg % 90 == 0;
            strip.spawn((
                CompassTick { angle: deg as f32 },
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        top: Val::Px(0.0),
                        width: Val::Px(if cardinal { 3.0 } else { 2.0 }),
                        height: Val::Px(if major { 9.0 } else { 6.0 }),
                        ..default()
                    },
                    background_color: if cardinal {
                        Color::srgba(0.95, 0.95, 0.95, 0.9)
                    } else if major {
                        Color::srgba(0.85, 0.85, 0.85, 0.55)
                    } else {
                        Color::srgba(0.7, 0.7, 0.7, 0.3)
                    }
                    .into(),
                    ..default()
                },
            ));
        }
        // 四个方位字随刻度滚动
        for (deg, name) in [(0.0, "北"), (90.0, "东"), (180.0, "南"), (270.0, "西")] {
            strip
                .spawn((
                    CompassLabel { angle: deg },
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
                        ..default()
                    },
                ))
                .with_children(|label| {
                    label.spawn(TextBundle::from_section(
                        name,
                        flow::style(fonts, 10.0, Color::srgb(0.95, 0.95, 0.95)),
                    ));
                });
        }
        // 中央固定指针
        strip.spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(MAP_SIZE * 0.5 - 1.0),
                width: Val::Px(2.0),
                height: Val::Px(8.0),
                ..default()
            },
            background_color: Color::srgb(1.0, 0.8, 0.25).into(),
            ..default()
        });
        // 中央方位读数（in-flow 子节点，由 justify_content 居中）
        strip
            .spawn(NodeBundle {
                style: Style {
                    width: Val::Px(64.0),
                    height: Val::Px(14.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: Color::srgba(0.0, 0.0, 0.0, 0.75).into(),
                ..default()
            })
            .with_children(|readout| {
                readout.spawn((
                    CompassHeadingText,
                    TextBundle::from_section(
                        "正北 0°",
                        flow::style(fonts, 10.0, Color::srgb(0.95, 0.95, 0.9)),
                    ),
                ));
            });
    });

    // ---- 方形小地图（局部放大，玩家居中，随玩家滚动） ----
    p.spawn((
        MinimapLayer,
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                top: Val::Px(14.0 + COMPASS_H),
                width: Val::Px(MAP_SIZE),
                height: Val::Px(MAP_SIZE),
                border: UiRect::all(Val::Px(MAP_BORDER)),
                overflow: Overflow::clip(),
                ..default()
            },
            background_color: Color::srgba(0.04, 0.05, 0.07, 0.78).into(),
            border_color: BorderColor(Color::srgba(0.6, 0.63, 0.67, 0.9)),
            ..default()
        },
    ))
    .with_children(|map| {
        // 静态掩体：只画有碰撞且高过膝的，标线/管道等装饰不上图；位置每帧重投影
        for prop in &layout.props {
            if !prop.solid {
                continue;
            }
            let aabb = prop.aabb_half();
            if prop.pos[1] + aabb[1] <= 0.5 {
                continue;
            }
            let w = (aabb[0] * 2.0 * MAP_SCALE).max(2.0);
            let h = (aabb[2] * 2.0 * MAP_SCALE).max(2.0);
            map.spawn((
                MinimapStatic { wx: prop.pos[0], wz: prop.pos[2], w, h },
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(MAP_CENTER - w * 0.5),
                        top: Val::Px(MAP_CENTER - h * 0.5),
                        width: Val::Px(w),
                        height: Val::Px(h),
                        ..default()
                    },
                    background_color: material_color(prop.material).into(),
                    ..default()
                },
            ));
        }
        // 功能站点：补给台（琥珀）/ 干员切换台（青）/ 物资箱（橙）
        for station in &layout.stations {
            map.spawn((
                MinimapStatic { wx: station.pos[0], wz: station.pos[2], w: 6.0, h: 6.0 },
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(MAP_CENTER - 3.0),
                        top: Val::Px(MAP_CENTER - 3.0),
                        width: Val::Px(6.0),
                        height: Val::Px(6.0),
                        ..default()
                    },
                    background_color: station_color(station.kind).into(),
                    ..default()
                },
            ));
        }
        // 动态点容器：每帧被 `update_minimap` 清空重建
        map.spawn((MinimapDots, NodeBundle::default()));
        // 玩家：白色定位点 + 朝向菱形（恒居中，后生成者在上层）
        map.spawn((
            MinimapPlayerDot,
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(MAP_CENTER - 2.0),
                    top: Val::Px(MAP_CENTER - 2.0),
                    width: Val::Px(4.0),
                    height: Val::Px(4.0),
                    ..default()
                },
                background_color: Color::srgb(0.95, 0.95, 0.95).into(),
                ..default()
            },
        ));
        map.spawn((
            MinimapPlayerArrow,
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(MAP_CENTER - 5.5),
                    top: Val::Px(MAP_CENTER - 5.5),
                    width: Val::Px(11.0),
                    height: Val::Px(11.0),
                    ..default()
                },
                background_color: Color::srgba(0.35, 0.95, 0.6, 0.5).into(),
                transform: Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
                ..default()
            },
        ));
    });
}

/// 罗盘刻度样式查询别名（互斥 With/Without，规避 clippy 复杂度与 B0001 冲突）。
type TicksQuery<'w, 's> = Query<
    'w,
    's,
    (&'static CompassTick, &'static mut Style),
    (Without<CompassLabel>, Without<MinimapStatic>),
>;
type LabelsQuery<'w, 's> = Query<
    'w,
    's,
    (&'static CompassLabel, &'static mut Style),
    (Without<CompassTick>, Without<MinimapStatic>),
>;
type StaticsQuery<'w, 's> = Query<
    'w,
    's,
    (&'static MinimapStatic, &'static mut Style),
    (Without<CompassTick>, Without<CompassLabel>),
>;

/// 每帧更新：罗盘滚动 + 方位读数 + 静态层重投影 + 玩家标记旋转 + 动态实体点重建。
///
/// 清空动态点用 [`DespawnRecursiveExt::despawn_descendants`]（一次性掏空 `MinimapDots`
/// 容器）而非逐点 `despawn`：bevy 0.14 单实体 `despawn` 不维护父子关系，被杀的点仍留在
/// `Children` 列表里，每帧累积成百上千失效 ID，最终递归销毁 HUD 树时刷屏 `error[B0003]`。
#[allow(clippy::type_complexity)]
pub fn update_minimap(
    mut commands: Commands,
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    rig: Res<AimRig>,
    layer: Query<Entity, With<MinimapDots>>,
    mut ticks: TicksQuery,
    mut labels: LabelsQuery,
    mut statics: StaticsQuery,
    mut player_arrow: Query<&mut Transform, With<MinimapPlayerArrow>>,
    mut heading_text: Query<&mut Text, With<CompassHeadingText>>,
) {
    let me = snap.current.iter().find(|e| e.entity_id == player.entity_id);
    // 玩家中心：优先用权威快照；首帧无快照时退回地图默认出生点（仅影响滚动起点）。
    let (cx, cz) = me
        .map(|m| (m.x, m.z))
        .unwrap_or_else(|| {
            let sp = lawn::layout().player_spawn;
            (sp[0], sp[2])
        });

    // ---- 罗盘：刻度/方位字按 1px=1° 滚动，超出可视范围隐藏 ----
    let heading = heading_rad(rig.yaw);
    let heading_deg = heading.to_degrees();
    for (tick, mut style) in ticks.iter_mut() {
        let rel = wrap_deg(tick.angle - heading_deg);
        style.left = Val::Px(MAP_SIZE * 0.5 + rel - 1.0);
        style.display = if rel.abs() <= 95.0 { Display::Flex } else { Display::None };
    }
    for (label, mut style) in labels.iter_mut() {
        let rel = wrap_deg(label.angle - heading_deg);
        style.left = Val::Px(MAP_SIZE * 0.5 + rel - 7.0);
        style.display = if rel.abs() <= 95.0 { Display::Flex } else { Display::None };
    }
    if let Ok(mut text) = heading_text.get_single_mut() {
        const DIR_NAMES: [&str; 8] =
            ["正北", "东北", "正东", "东南", "正南", "西南", "正西", "西北"];
        let dir = DIR_NAMES[((heading_deg + 22.5) / 45.0) as usize % 8];
        text.sections[0].value = format!("{} {:.0}°", dir, heading_deg);
    }

    // ---- 静态层：随玩家中心重投影（尺寸不变，仅平移；越界由画布 clip 裁掉） ----
    for (s, mut style) in statics.iter_mut() {
        style.left = Val::Px(project(s.wx, cx) - s.w * 0.5);
        style.top = Val::Px(project(s.wz, cz) - s.h * 0.5);
    }

    // ---- 玩家朝向菱形：恒居中，仅按相机朝向旋转 ----
    // UI 屏幕 y 向下，rotation.z 正值在屏上表现为顺时针，恰与罗盘方位一致。
    if let Ok(mut transform) = player_arrow.get_single_mut() {
        transform.rotation = Quat::from_rotation_z(heading + std::f32::consts::FRAC_PI_4);
    }

    // ---- 动态实体点重建（以玩家为原点局部投影，越界剔除） ----
    let Ok(container) = layer.get_single() else {
        return;
    };
    commands.entity(container).despawn_descendants();
    commands.entity(container).with_children(|p| {
        for e in &snap.current {
            let px = project(e.x, cx);
            let py = project(e.z, cz);
            if px < 0.0 || px > MAP_SIZE || py < 0.0 || py > MAP_SIZE {
                continue;
            }
            let (size, color) =
                dot_style(e.entity_id, me.map(|m| m.entity_id), e.model_preset);
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
}

/// 实体点样式：本人高亮青色大点；靶红；敌人深灰红；其余按模型主色小点。
fn dot_style(
    id: u64,
    self_id: Option<u64>,
    preset: cute_of_duty_server::model::ModelPreset,
) -> (f32, Color) {
    if Some(id) == self_id {
        return (7.0, Color::srgb(0.3, 0.9, 1.0));
    }
    let body = voxel_for(preset);
    let color = match preset_group(preset) {
        DotKind::Target => Color::srgb(0.95, 0.25, 0.2),
        DotKind::Enemy => Color::srgb(0.55, 0.2, 0.18),
        DotKind::Other => body.primary,
    };
    (5.0, color)
}

/// 把模型身份粗分三类，决定小地图点色（冷映射，不涉及任何状态判定）。
enum DotKind {
    Target,
    Enemy,
    Other,
}

fn preset_group(preset: cute_of_duty_server::model::ModelPreset) -> DotKind {
    use cute_of_duty_server::model::ModelPreset;
    match preset {
        ModelPreset::AimTarget => DotKind::Target,
        ModelPreset::EnemyThug => DotKind::Enemy,
        _ => DotKind::Other,
    }
}
