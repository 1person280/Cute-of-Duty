//! 第三人称越肩跟随镜头：贴在本地玩家身后/肩上、朝移动方向看
//!
//! 设计动机（Why）：训练场是 30×30 全封闭室内 CQB（3m 内墙 + 天花板横梁）。旧版「绕玩家
//! 自动环绕」镜头半径 22/高 14，落在建筑外墙/屋顶之外——从外面看不见室内，造成「3D
//! 黑屏、看不到自己干员」。改为**越肩跟随**：镜头始终保持在玩家背后肩上、面朝玩家前进
//! 方向，从而镜头一直待在室内、能看到干员与内部光照。朝向由 WASD 位移方向平滑推导
//! （与服务端移动同一轴系：W=-Z 北/撤离、S=+Z、A=-X、D=+X），静止时保持上一朝向。
//! 视角/距离均为纯表现层设定，不属于任何服务端权威数据。FOV 由 `game_settings` 按月
//! 此项（`With<ChaseCamera>`）改写投影。

use bevy::prelude::*;

use crate::flow::flow_state::LocalPlayer;
use crate::net::snapshot::SnapshotBuffer;

/// 越肩跟随镜头标记（每帧由 `follow_system` 重写世界变换；`game_settings` 据此改 FOV）。
#[derive(Component)]
pub struct ChaseCamera;

/// 镜头在玩家背后的水平距离、肩上高度（越肩感）。
const CAMERA_DIST: f32 = 3.2;
const CAMERA_HEIGHT: f32 = 1.7;
/// 看向玩家前方多远、视线对准的高度（头部/肩部）。
const LOOK_AHEAD: f32 = 3.0;
const LOOK_EYE: f32 = 1.4;
/// 朝向平滑转向速率（弧度/秒）；WASD 变向时镜头不至于瞬间跳转。
const TURN_RATE: f32 = 8.0;

/// 生成越肩跟拍摄像机（初始面向 -Z 北侧；握手前退到原点，仍在室内）。
///
/// bevy 0.14 用 `Camera3dBundle` 承载相机（渲染图/投影/可见性一并装配）。
pub fn spawn_camera(commands: &mut Commands) {
    commands.spawn((
        ChaseCamera,
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, CAMERA_HEIGHT, CAMERA_DIST)
                .looking_at(Vec3::new(0.0, LOOK_EYE, -LOOK_AHEAD), Vec3::Y),
            ..default()
        },
    ));
}

/// 每帧让镜头贴玩家背后/肩上、朝移动方向看；握手前无本人实体则停在原点（室内视角）。
///
/// 目标位置取自权威快照（服务端唯一真相源），客户端不做本地校订；镜头朝向由本地输入
/// 平滑推导，属表现层。
fn shortest_angle_horizontal(cur: f32, target: f32) -> f32 {
    let mut d = (target - cur) % std::f32::consts::TAU;
    if d > std::f32::consts::PI {
        d -= std::f32::consts::TAU;
    }
    if d < -std::f32::consts::PI {
        d += std::f32::consts::TAU;
    }
    d
}

pub fn follow_system(
    mut query: Query<(&mut Transform, &ChaseCamera)>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    // 当前朝向（世界 yaw，弧度）；初始未定值时先置为面向 -Z。
    mut heading: Local<f32>,
    mut init: Local<bool>,
) {
    let center = if player.entity_id != 0 {
        snap.current
            .iter()
            .find(|e| e.entity_id == player.entity_id)
            .map(|e| Vec3::new(e.x, e.y, e.z))
            .unwrap_or(Vec3::ZERO)
    } else {
        Vec3::ZERO
    };

    if !*init {
        *heading = std::f32::consts::PI; // 面向 -Z（北/撤离区）
        *init = true;
    }

    // 由 WASD 推导期望朝向（与服务端移动同一轴系）。
    let mut move_vec = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        move_vec.z -= 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        move_vec.z += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        move_vec.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        move_vec.x += 1.0;
    }
    if move_vec.length_squared() > 1e-6 {
        let target_yaw = move_vec.x.atan2(move_vec.z);
        let step = (TURN_RATE * time.delta_seconds()).min(1.0);
        *heading += shortest_angle_horizontal(*heading, target_yaw) * step;
    }

    // 沿朝向算出镜头在背后/肩上、视线朝前的越肩位。
    let fwd = Vec3::new(heading.sin(), 0.0, heading.cos());
    let back = -fwd;
    let cam_pos = center + back * CAMERA_DIST + Vec3::Y * CAMERA_HEIGHT;
    let look = center + fwd * LOOK_AHEAD + Vec3::Y * LOOK_EYE;
    for (mut tf, _) in &mut query {
        tf.translation = cam_pos;
        tf.look_at(look, Vec3::Y);
    }
}