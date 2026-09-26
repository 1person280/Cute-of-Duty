//! 第三人称环绕镜头 + 鼠标自由视角
//!
//! 设计动机（Why）：0.6 早期版本朝向由 WASD 位移方向推导，既没有鼠标自由视角、
//! 也无法把瞄准朝向报给服务端（弹道与移动轴系都因此失真）。本模块改为**鼠标驱动**：
//! 偏航/俯仰写入共享资源 [`AimRig`]（`flow_state`），镜头据此摆放；`net::input_system`
//! 再把同一份 `AimRig` 作为 `aim_yaw/aim_pitch` 上报 —— 视角与移动/弹道轴系从此同源。
//!
//! 取景方式（Why）：采用**环绕相机**——镜头沿视线反方向退到角色后方 `CAMERA_DIST`，
//! 并**始终看向角色胸口** `center + Y*LOOK_TARGET_Y`。这样无论鼠标俯仰多大，角色都稳定
//! 居于画面中央；此前「看向角色前方数米」的取景在窄 FOV 下会让角色头部贴边、俯视即出画。
//!
//! 轴系约定（与 `flow_state::AimRig` 及服务端 `shooter` 完全一致）：
//! - 方向 = `(sinY·cosP, sinP, cosY·cosP)`，pitch 为正 = 抬头；
//! - 默认 yaw = π 面向 -Z（北/撤离区），镜头落在玩家 +Z 侧后方，与服务端出生朝向一致。
//!
//! 视角/距离/FOV 均为纯表现层设定，不属于任何服务端权威数据（`game_settings` 据此改 FOV）。
//! 镜头位置取自权威快照（服务端唯一真相源），客户端不做本地位置校订。

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

use crate::flow::flow_state::{AimRig, LocalPlayer};
use crate::menu::GameSettings;
use crate::net::snapshot::SnapshotBuffer;

/// 第三人称环绕镜头标记（每帧由 `follow_system` 重写世界变换；`game_settings` 据此改 FOV）。
#[derive(Component)]
pub struct ChaseCamera;

/// 镜头到角色的环绕半径（米）。
const CAMERA_DIST: f32 = 4.2;
/// 视线锁定的角色高度（胸口偏上；角色整体高约 2.67m，取中部保证全身在框）。
const LOOK_TARGET_Y: f32 = 1.4;
/// 环绕俯仰限位：抬头不高于 +0.55（约 31°）、低头不低于 -0.75（约 -43°），
/// 避免镜头钻到角色脚下/贴地。瞄准用 `AimRig.pitch` 另有更宽的限位。
const ORBIT_PITCH_MIN: f32 = -0.75;
const ORBIT_PITCH_MAX: f32 = 0.55;
/// 镜头最低高度（米），防止大角度俯视时穿到地面以下。
const MIN_CAM_Y: f32 = 0.5;

/// 鼠标灵敏度（弧度/像素），再乘设置里的灵敏度；数值沿 0.3.2 手感。
const MOUSE_SENS_X: f32 = 0.0024;
const MOUSE_SENS_Y: f32 = 0.0021;
/// 俯仰限制：抬头 50° / 低头 70°（与服务端视线方向同号：pitch 正 = 抬头）。
const PITCH_UP_MAX: f32 = 0.873;
const PITCH_DOWN_MAX: f32 = 1.217;
/// 单帧鼠标位移上限（像素）：光标刚锁定时系统可能上报一次“从旧位置跳到中心”的巨大位移，
/// 限幅可防止视角被该跳变甩飞（正常鼠标移动远低于此值）。
const MAX_FRAME_DELTA: f32 = 200.0;

/// 生成环绕摄像机（初始面向 -Z 北侧；握手前停在原点后方，仍在场内）。
///
/// bevy 0.14 用 `Camera3dBundle` 承载相机（渲染图/投影/可见性一并装配）。
pub fn spawn_camera(commands: &mut Commands) {
    let look = Vec3::new(0.0, LOOK_TARGET_Y, 0.0);
    commands.spawn((
        ChaseCamera,
        Camera3dBundle {
            transform: Transform::from_translation(look + Vec3::new(0.0, 0.0, CAMERA_DIST))
                .looking_at(look, Vec3::Y),
            ..default()
        },
    ));
}

/// 鼠标自由视角：把本帧鼠标位移折进共享的 [`AimRig`]（偏航/俯仰）。
///
/// 只在游戏内且未暂停时运行（`launcher` 以 `pause_closed` 门控），且仅在光标锁定
/// （`menu::pause::cursor_lock_system` 保证）时才有意义——菜单态鼠标用于点按 UI。
pub fn mouse_look_system(
    mut motion: EventReader<MouseMotion>,
    settings: Res<GameSettings>,
    mut rig: ResMut<AimRig>,
) {
    let mut delta = Vec2::ZERO;
    for ev in motion.read() {
        delta += ev.delta;
    }
    if delta == Vec2::ZERO {
        return;
    }
    // 限幅：抵消光标锁定瞬间的巨大跳变（见 `MAX_FRAME_DELTA`）。
    delta = delta.clamp(Vec2::splat(-MAX_FRAME_DELTA), Vec2::splat(MAX_FRAME_DELTA));
    // 鼠标右移 → 视线右转（yaw 减小，见模块头轴系推导）；鼠标下移 → 低头（pitch 减小）。
    rig.yaw -= delta.x * MOUSE_SENS_X * settings.mouse_sensitivity;
    rig.pitch -= delta.y * MOUSE_SENS_Y * settings.mouse_sensitivity;
    rig.pitch = rig.pitch.clamp(-PITCH_DOWN_MAX, PITCH_UP_MAX);
}

/// 每帧把镜头环绕到角色后方、始终注视角色胸口；握手前无本人实体则停在原点。
///
/// 目标位置取自权威快照（服务端唯一真相源），客户端不做本地校订；镜头姿态纯属表现层。
pub fn follow_system(
    mut query: Query<(&mut Transform, &ChaseCamera)>,
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    rig: Res<AimRig>,
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

    // 环绕注视点固定在角色胸口：角色因此永远居于画面中央，俯仰也不会把它甩出画面。
    let look = center + Vec3::Y * LOOK_TARGET_Y;
    // 镜头沿「视线反方向」退到身后（含俯仰的环轨道），再钳制俯仰与最低高度。
    let pitch = rig.pitch.clamp(ORBIT_PITCH_MIN, ORBIT_PITCH_MAX);
    let cos_p = pitch.cos();
    let dir = Vec3::new(rig.yaw.sin() * cos_p, pitch.sin(), rig.yaw.cos() * cos_p);
    let mut cam_pos = look - dir * CAMERA_DIST;
    if cam_pos.y < MIN_CAM_Y {
        cam_pos.y = MIN_CAM_Y;
    }
    for (mut tf, _) in &mut query {
        tf.translation = cam_pos;
        tf.look_at(look, Vec3::Y);
    }
}