//! 第三人称越肩镜头 + 鼠标自由视角
///
/// 设计动机（Why）：0.6 早期版本朝向由 WASD 位移方向推导，既没有鼠标自由视角、
/// 也无法把瞄准朝向报给服务端（弹道与移动轴系都因此失真）。本模块改为**鼠标驱动**：
/// 偏航/俯仰写入共享资源 [`AimRig`]（`flow_state`），镜头据此摆放；`net::input_system`
/// 再把同一份 `AimRig` 作为 `aim_yaw/aim_pitch` 上报 —— 视角与移动/弹道轴系从此同源。
///
/// 取景方式（Why，对齐 0.3.2 `demo/camera.rs` 的越肩 SpringArm）：镜头挂在角色**右肩**后方，
/// 并**沿视线方向平行注视**（而非回看角色）——角色因此稳定落在画面**偏左**，正是"第三人称
/// 越肩"的标志观感；此前"纯环绕 + 始终看向角色胸口"的取景会让角色正中、并在贴墙时糊脸，
/// 观感偏近于第一人称。轴系与旧版一致：右肩水平偏移 `SHOULDER_OFFSET`，臂长 `CAMERA_DIST`。
///
/// 轴系约定（与 `flow_state::AimRig` 及服务端 `shooter` 完全一致）：
/// - 方向 = `(sinY·cosP, sinP, cosY·cosP)`，pitch 为正 = 抬头；
/// - 默认 yaw = π 面向 -Z（北/撤离区），镜头落在玩家 +Z 侧后方，与服务端出生朝向一致。
///
/// 视角/距离/FOV 均为纯表现层设定，不属于任何服务端权威数据（`game_settings` 据此改 FOV）。
/// 镜头位置取自权威快照（服务端唯一真相源），客户端不做本地位置校订。

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

use crate::flow::flow_state::{AimRig, LocalPlayer};
use crate::menu::GameSettings;
use crate::net::snapshot::SnapshotBuffer;

/// 第三人称越肩镜头标记（每帧由 `follow_system` 重写世界变换；`game_settings` 据此改 FOV）。
///
/// `aim_blend` 为越肩瞄准的过渡进度（0 = 常态取景，1 = 瞄准取景），由 `follow_system`
/// 按时间朝目标值收敛后写入；投影侧（`settings_apply_fov`）读它同步收窄 FOV，
/// 从而**全工程只有一处写 `Projection`**，避免两系统同帧争用同一组件的调度冲突。
#[derive(Component)]
pub struct ChaseCamera {
    pub aim_blend: f32,
}

/// 镜头到角色的臂长（米，旧版 `ARM_LEN_NORMAL` 的收敛值）。
const CAMERA_DIST: f32 = 4.2;
/// 越肩机位的右肩水平偏移（米）：镜头右移后角色落于画面偏左，形成越肩观感。
const SHOULDER_OFFSET: f32 = 0.65;
/// 瞄准时的臂长（米，镜头贴近右肩，旧版 `ARM_LEN_AIM`）。
const AIM_DIST: f32 = 2.4;
/// 瞄准时的右肩偏移（米）：比常态更外扩，避免贴脸时角色糊住画面。
const AIM_SHOULDER: f32 = 1.0;
/// 常态 ↔ 瞄准取景的过渡时长（秒，smoothstep），沿 0.3.2 手感。
const AIM_BLEND_SECS: f32 = 0.22;
/// 瞄准时 FOV 收窄比例（28%），由投影侧读取（见 [`ChaseCamera::aim_blend`]）。
pub const AIM_FOV_NARROW: f32 = 0.28;
/// 视线锚点高度（角色肩颈处；角色整体高约 2.67m，取肩部保证全身在框）。
const PIVOT_Y: f32 = 1.55;
/// 俯仰限位：抬头不高于 +0.55（约 31°）、低头不低于 -0.75（约 -43°），
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

/// 生成越肩摄像机（初始面向 -Z 北侧；握手前停在原点后方，仍在场内）。
///
/// bevy 0.14 用 `Camera3dBundle` 承载相机（渲染图/投影/可见性一并装配）。
pub fn spawn_camera(commands: &mut Commands) {
    let look = Vec3::new(0.0, PIVOT_Y, 0.0);
    commands.spawn((
        ChaseCamera { aim_blend: 0.0 },
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
    mouse: Res<ButtonInput<MouseButton>>,
    settings: Res<GameSettings>,
    mut rig: ResMut<AimRig>,
) {
    // 瞄准态由右键按住决定（相机取景过渡 + 上行 `aim` 意图同源，见 `net::input_system`）。
    rig.aiming = mouse.pressed(MouseButton::Right);

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

/// 每帧把镜头摆到角色右肩后方并沿视线平行注视（越肩第三人称）；握手前无本人实体则停在原点。
///
/// 目标位置取自权威快照（服务端唯一真相源），客户端不做本地校订；镜头姿态纯属表现层。
/// 注意：本人实体若**暂时**不在本帧快照（AOI/对账间隙），直接保持上一帧机位而非退回世界原点——
/// 否则镜头会瞬移到 (0,0,0)，表现为"看不见自己、场景乱飘"。
pub fn follow_system(
    mut query: Query<(&mut Transform, &mut ChaseCamera)>,
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    rig: Res<AimRig>,
    time: Res<Time>,
) {
    let center = if player.entity_id != 0 {
        match snap
            .current
            .iter()
            .find(|e| e.entity_id == player.entity_id)
        {
            Some(e) => Vec3::new(e.x, e.y, e.z),
            // 已握手但本帧快照缺本人条目：保持上一帧机位，避免跳回原点。
            None => return,
        }
    } else {
        Vec3::ZERO
    };

    // 视线锚点固定在角色肩部；镜头绕其沿视线反方向退到身后（含俯仰的环轨道）。
    let pitch = rig.pitch.clamp(ORBIT_PITCH_MIN, ORBIT_PITCH_MAX);
    let cos_p = pitch.cos();
    // 视线方向（与服务端弹道同号：pitch 正 = 抬头）。
    let dir = Vec3::new(rig.yaw.sin() * cos_p, pitch.sin(), rig.yaw.cos() * cos_p);
    // 水平右向量 = normalize(cross(forward, Y))，用于右肩平移（越肩机位）。
    let right = Vec3::new(-rig.yaw.cos(), 0.0, rig.yaw.sin());

    let anchor = center + Vec3::Y * PIVOT_Y;

    // 越肩瞄准过渡：`aim_blend` 按「dt / 过渡时长」朝目标推进（帧率无关），
    // 再取 smoothstep 缓动，使收臂 / 收 FOV 起步与收尾都不生硬。
    let target = if rig.aiming { 1.0 } else { 0.0 };
    let step = time.delta_seconds() / AIM_BLEND_SECS;
    for (mut tf, mut cam) in &mut query {
        cam.aim_blend = (cam.aim_blend + (target - cam.aim_blend).clamp(-step, step)).clamp(0.0, 1.0);
        let b = cam.aim_blend;
        let eased = b * b * (3.0 - 2.0 * b);

        let dist = CAMERA_DIST + (AIM_DIST - CAMERA_DIST) * eased;
        let shoulder = SHOULDER_OFFSET + (AIM_SHOULDER - SHOULDER_OFFSET) * eased;
        let mut cam_pos = anchor + right * shoulder - dir * dist;
        if cam_pos.y < MIN_CAM_Y {
            cam_pos.y = MIN_CAM_Y;
        }
        tf.translation = cam_pos;
        // 沿视线**平行**注视（不回看角色）：角色因此稳定落在画面偏左，形成越肩第三人称观感。
        tf.look_at(cam_pos + dir, Vec3::Y);
    }
}