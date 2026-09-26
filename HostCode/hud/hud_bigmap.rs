//! 战术全景图（按 M 打开）：全屏俯瞰整张活动地图
//!
//! 设计动机：与左上角小地图职责分离——小地图是"我周围有什么"，全景图是"整张图长什么样"。
//! 本视图画出搜打撤四段分区色带、分区边界、撤离信标、全部静态掩体/目标/拾取物/功能站点，
//! 以及实时玩家定位点与朝向箭头。**全部静态内容一次摆位**（地图数据不变），只有玩家标记
//! 每帧跟权威快照更新。数据来源与服务端渲染同源（`map::lawn` 纯数据），地图改布局即同步。
//!
//! 打开态只是 `InGame` 内的一枚资源门控（同暂停菜单思路）：画面仍在跑，仅隐藏/显示整屏
//! 覆盖层，并冻结游戏内输入上报与相机朝向，避免"开着地图还在平移视角/走火"。

use bevy::prelude::*;
use cute_of_duty_server::element::ElementType;
use cute_of_duty_server::map::lawn;
use cute_of_duty_server::map::{PickupKind, StationKind};

use crate::flow::flow_state::{self as flow, AimRig, CjkFont, LocalPlayer};
use crate::net::snapshot::SnapshotBuffer;

/// 全景图是否打开（打开时冻结游戏内输入，仅允许关图按键）。
#[derive(Resource, Default)]
pub struct BigMapOpen(pub bool);

/// 全景图整屏覆盖层根（默认隐藏）。
#[derive(Component)]
pub struct BigMapRoot;

/// 全景图玩家定位白点。
#[derive(Component)]
pub struct BigMapPlayerDot;

/// 全景图玩家朝向菱形。
#[derive(Component)]
pub struct BigMapPlayerArrow;

/// 全景地图边长（px，旧版 600）。
const BIGMAP_PX: f32 = 600.0;
/// 地图框边宽（px）。
const BIGMAP_BORDER: f32 = 3.0;
/// 绘制区边长（扣除边框）。
const BIGMAP_INNER: f32 = BIGMAP_PX - 2.0 * BIGMAP_BORDER;

/// 世界坐标（x 东 / z 南）→ 全景图像素（左上角为西北角，正北朝上）。
fn world_to_px(v: f32, half: f32) -> f32 {
    BIGMAP_BORDER + (v + half) / (2.0 * half) * BIGMAP_INNER
}

/// 一个 z 区间色带的像素框（自 [z_bottom, z_top] → top/height）。
fn band_top_height(z_top: f32, z_bottom: f32, half: f32) -> (f32, f32) {
    let top = world_to_px(z_top, half);
    let bottom = world_to_px(z_bottom, half);
    (top, bottom - top)
}

/// 相机 yaw 弧度 → 方位角弧度（正北=0，顺时针）；与罗盘同源。
fn heading_rad(yaw: f32) -> f32 {
    (std::f32::consts::PI - yaw).rem_euclid(std::f32::consts::TAU)
}

/// 搜打撤四段分区色带（z 区间 + 标签 + 半透明底色）。
const ZONES: [(&str, f32, f32, [f32; 3]); 4] = [
    ("出生区", 500.0, 440.0, [0.35, 0.85, 0.35]),   // 南端绿
    ("搜索区·搜", 440.0, 200.0, [0.95, 0.75, 0.2]), // 黄
    ("射击区·打", 200.0, -200.0, [0.95, 0.4, 0.2]), // 橙
    ("撤离区·撤", -200.0, -500.0, [0.9, 0.2, 0.2]),  // 北端红
];

/// 分区边界线 z（出生/搜索、搜索/射击、射击/撤离）。
const BOUNDARY_Z: [f32; 3] = [440.0, 200.0, -200.0];

/// 元素展示配色（表现层冷映射，与服务端 element 语义无关）。
fn element_color(e: ElementType) -> Color {
    match e {
        ElementType::Fire => Color::srgb(0.95, 0.45, 0.15),
        ElementType::Ice => Color::srgb(0.35, 0.75, 0.95),
        ElementType::Electric => Color::srgb(0.90, 0.80, 0.25),
        ElementType::Poison => Color::srgb(0.45, 0.80, 0.30),
        ElementType::Physical => Color::srgb(0.75, 0.75, 0.78),
        ElementType::Water => Color::srgb(0.30, 0.55, 0.90),
    }
}

/// 拾取物 → 全景图点色（与场上发光色一致）。
fn pickup_color(kind: PickupKind) -> Color {
    match kind {
        PickupKind::Ammo { .. } => Color::srgba(0.9, 0.7, 0.2, 0.95),
        PickupKind::Health { .. } => Color::srgba(0.9, 0.25, 0.25, 0.95),
        PickupKind::Armor { .. } => Color::srgba(0.3, 0.55, 0.95, 0.95),
        PickupKind::Grenade { element } | PickupKind::Weapon { element } => {
            element_color(element).with_alpha(0.95)
        }
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

/// 装配全景图整屏覆盖层（默认隐藏；静态内容一次摆位）。
pub fn spawn_bigmap(p: &mut ChildBuilder<'_>, fonts: &CjkFont) {
    let layout = lawn::layout();
    let half = layout.half_extent;

    p.spawn((
        BigMapRoot,
        NodeBundle {
            style: Style {
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
            background_color: Color::srgba(0.02, 0.03, 0.05, 0.6).into(),
            visibility: Visibility::Hidden,
            ..default()
        },
    ))
    .with_children(|root| {
        // 标题栏
        root.spawn(TextBundle::from_section(
            "战术全景图 · 搜打撤草坪训练场（1×1km）· 按 M / Esc 关闭",
            flow::style(fonts, 16.0, Color::srgb(0.95, 0.95, 0.9)),
        ));

        // 方形地图
        root.spawn(NodeBundle {
            style: Style {
                width: Val::Px(BIGMAP_PX),
                height: Val::Px(BIGMAP_PX),
                border: UiRect::all(Val::Px(BIGMAP_BORDER)),
                overflow: Overflow::clip(),
                ..default()
            },
            background_color: Color::srgba(0.04, 0.05, 0.07, 0.9).into(),
            border_color: BorderColor(Color::srgba(0.6, 0.63, 0.67, 0.95)),
            ..default()
        })
        .with_children(|map| {
            // ---- 分区色带 + 标签（最底层）----
            for (label, z_top, z_bottom, rgb) in ZONES {
                let (top, height) = band_top_height(z_top, z_bottom, half);
                map.spawn(NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(BIGMAP_BORDER),
                        top: Val::Px(top),
                        width: Val::Px(BIGMAP_INNER),
                        height: Val::Px(height),
                        ..default()
                    },
                    background_color: Color::srgba(rgb[0], rgb[1], rgb[2], 0.16).into(),
                    ..default()
                });
                // 色带中央标签
                let z_mid = (z_top + z_bottom) * 0.5;
                map.spawn(NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(BIGMAP_BORDER + 6.0),
                        top: Val::Px(world_to_px(z_mid, half) - 8.0),
                        ..default()
                    },
                    background_color: Color::srgba(0.0, 0.0, 0.0, 0.45).into(),
                    ..default()
                })
                .with_children(|t| {
                    t.spawn(TextBundle::from_section(
                        label,
                        flow::style(fonts, 12.0, Color::srgb(0.95, 0.95, 0.9)),
                    ));
                });
            }

            // ---- 分区边界线 ----
            for z in BOUNDARY_Z {
                map.spawn(NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(BIGMAP_BORDER),
                        top: Val::Px(world_to_px(z, half) - 1.0),
                        width: Val::Px(BIGMAP_INNER),
                        height: Val::Px(2.0),
                        ..default()
                    },
                    background_color: Color::srgba(0.95, 0.95, 0.95, 0.7).into(),
                    ..default()
                });
            }

            // ---- 撤离信标（北端红色，比普通目标更大）----
            map.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(world_to_px(0.0, half) - 4.0),
                    top: Val::Px(world_to_px(-440.0, half) - 4.0),
                    width: Val::Px(8.0),
                    height: Val::Px(8.0),
                    ..default()
                },
                background_color: Color::srgb(1.0, 0.2, 0.15).into(),
                ..default()
            });

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
                map.spawn(NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(world_to_px(prop.pos[0], half) - w * 0.5),
                        top: Val::Px(world_to_px(prop.pos[2], half) - h * 0.5),
                        width: Val::Px(w),
                        height: Val::Px(h),
                        ..default()
                    },
                    background_color: Color::srgba(0.58, 0.6, 0.64, 0.85).into(),
                    ..default()
                });
            }

            // ---- 静态目标（红芯小点）----
            for t in &layout.targets {
                map.spawn(NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(world_to_px(t.pos[0], half) - 2.5),
                        top: Val::Px(world_to_px(t.pos[2], half) - 2.5),
                        width: Val::Px(5.0),
                        height: Val::Px(5.0),
                        ..default()
                    },
                    background_color: Color::srgba(0.95, 0.35, 0.3, 0.95).into(),
                    ..default()
                });
            }

            // ---- 拾取物（按类型配色）----
            for pk in &layout.pickups {
                map.spawn(NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(world_to_px(pk.pos[0], half) - 2.0),
                        top: Val::Px(world_to_px(pk.pos[2], half) - 2.0),
                        width: Val::Px(4.0),
                        height: Val::Px(4.0),
                        ..default()
                    },
                    background_color: pickup_color(pk.kind).into(),
                    ..default()
                });
            }

            // ---- 功能站点（补给=琥珀 / 干员=青）----
            for st in &layout.stations {
                map.spawn(NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(world_to_px(st.pos[0], half) - 3.0),
                        top: Val::Px(world_to_px(st.pos[2], half) - 3.0),
                        width: Val::Px(6.0),
                        height: Val::Px(6.0),
                        ..default()
                    },
                    background_color: station_color(st.kind).into(),
                    ..default()
                });
            }

            // ---- 玩家：白色定位点 + 朝向箭头（后生成者在上层）----
            let px = world_to_px(layout.player_spawn[0], half);
            let py = world_to_px(layout.player_spawn[2], half);
            map.spawn((
                BigMapPlayerDot,
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(px - 2.0),
                        top: Val::Px(py - 2.0),
                        width: Val::Px(4.0),
                        height: Val::Px(4.0),
                        ..default()
                    },
                    background_color: Color::srgb(0.95, 0.95, 0.95).into(),
                    ..default()
                },
            ));
            map.spawn((
                BigMapPlayerArrow,
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(px - 5.5),
                        top: Val::Px(py - 5.5),
                        width: Val::Px(11.0),
                        height: Val::Px(11.0),
                        ..default()
                    },
                    background_color: Color::srgba(0.35, 0.95, 0.6, 0.5).into(),
                    transform: Transform::from_rotation(Quat::from_rotation_z(
                        std::f32::consts::FRAC_PI_4,
                    )),
                    ..default()
                },
            ));
        });

        // 图例说明
        root.spawn(TextBundle::from_section(
            "绿=出生 · 黄=搜索(搜) · 橙=射击(打) · 红=撤离(撤)　白点=玩家 · 红点=目标 · 彩点=拾取物",
            flow::style(fonts, 12.0, Color::srgb(0.85, 0.87, 0.9)),
        ));
    });
}

/// 运行条件：游玩输入可用 = 未暂停 **且** 全景图未打开 **且** 交互二级面板/物资箱面板/径向轮盘
/// 均未打开。
///
/// 把四个模态门控合成单一条件，避免在 `run_if` 处用 `Condition::and` 组合（bevy 0.14 的
/// `Condition` 组合器不在 prelude，直接在函数项上调用 `.and` 无法解析）。
/// 注意：**就近交互列表本身不冻结输入**——它是常显的提示性列表，玩家可边跑边看（对齐 legacy）。
pub fn gameplay_input_active(
    pause: Res<crate::menu::PauseMenu>,
    open: Res<BigMapOpen>,
    interact: Res<crate::hud::InteractState>,
    loot: Res<crate::hud::LootPanelState>,
    wheel: Res<crate::hud::ItemWheelState>,
) -> bool {
    *pause == crate::menu::PauseMenu::Closed
        && !open.0
        && !interact.panel_open
        && !loot.open
        && !wheel.open
}

/// 离开训练场时复位全景图门控（否则下次进场会带着"已打开"状态冻结输入）。
pub fn reset_bigmap(mut open: ResMut<BigMapOpen>) {
    open.0 = false;
}

/// M 打开 / M 或 Esc 关闭全景图（仅切换可见性，静态内容不重建）。
pub fn bigmap_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut open: ResMut<BigMapOpen>,
    mut root: Query<&mut Visibility, With<BigMapRoot>>,
) {
    let toggle = if open.0 {
        keys.just_pressed(KeyCode::KeyM) || keys.just_pressed(KeyCode::Escape)
    } else {
        keys.just_pressed(KeyCode::KeyM)
    };
    if !toggle {
        return;
    }
    open.0 = !open.0;
    if let Ok(mut vis) = root.get_single_mut() {
        *vis = if open.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// 每帧更新玩家定位点与朝向箭头（地图静态内容不重复摆位）。
///
/// 位置取权威快照（服务端裁决）；朝向取本地 `AimRig`（纯表现视角，与上行瞄准同源）。
pub fn update_bigmap(
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    rig: Res<AimRig>,
    mut dot: Query<&mut Style, (With<BigMapPlayerDot>, Without<BigMapPlayerArrow>)>,
    mut arrow: Query<(&mut Style, &mut Transform), (With<BigMapPlayerArrow>, Without<BigMapPlayerDot>)>,
) {
    let half = lawn::HALF;
    let Some(m) = snap.current.iter().find(|e| e.entity_id == player.entity_id) else {
        return;
    };
    let px = world_to_px(m.x, half);
    let py = world_to_px(m.z, half);
    if let Ok(mut style) = dot.get_single_mut() {
        style.left = Val::Px(px - 2.0);
        style.top = Val::Px(py - 2.0);
    }
    if let Ok((mut style, mut transform)) = arrow.get_single_mut() {
        style.left = Val::Px(px - 5.5);
        style.top = Val::Px(py - 5.5);
        transform.rotation = Quat::from_rotation_z(heading_rad(rig.yaw) + std::f32::consts::FRAC_PI_4);
    }
}
