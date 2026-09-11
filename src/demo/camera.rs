//! 相机 Rig：越肩/瞄准位切换、鼠标视角、射线拾取数学

use bevy::prelude::*;
use crate::model::{
    Player, PlayerAimGun, PlayerCamera,
    PlayerHeadPivot,
};
use super::inventory::{HeldGrenade};
use super::components::*;
#[derive(Component)]
/// 顶级枢轴：位于角色脚底中心，不随角色模型旋转（TopLevel），每帧跟随角色位置
pub struct CamPivot;

#[derive(Component)]
/// 偏航控制节点（鼠标 X 轴）
pub struct ShoulderPivot;

#[derive(Component)]
/// 俯仰控制节点（鼠标 Y 轴）
pub struct PitchPivot;

#[derive(Component)]
/// 弹簧臂：本地 Z = 后方距离，相机挂在其末端
pub struct SpringArm;

#[derive(Component)]
pub struct SpringArmState { pub(crate) len: f32 }


// —— SpringArm 参数（占位值已按 3.7m 体型的体素模型校准） ——
/// 俯仰枢轴高度（相对脚底 Pivot，模型颈肩处；头中心在 3.0）
pub(crate) const PIVOT_HEIGHT: f32 = 2.6;
/// 默认机位：右肩水平偏移 / 垂直偏移（附加于枢轴高度）/ 后方距离
pub(crate) const ARM_SHOULDER_X_NORMAL: f32 = 0.55;
pub(crate) const ARM_EYE_Y_NORMAL: f32 = 0.35;
pub(crate) const ARM_LEN_NORMAL: f32 = 6.5;
/// 越肩机位（瞄准）
pub(crate) const ARM_SHOULDER_X_AIM: f32 = 1.0;
pub(crate) const ARM_EYE_Y_AIM: f32 = 0.3;
pub(crate) const ARM_LEN_AIM: f32 = 2.4;
/// 正常 ↔ 瞄准过渡耗时（秒），smoothstep
pub(crate) const ARM_TRANSITION_SECS: f32 = 0.22;
/// 弹簧臂碰撞：贴墙最小距离 / 相机最低高度
pub(crate) const ARM_MIN_LEN: f32 = 0.7;
pub(crate) const ARM_GROUND_MIN_Y: f32 = 0.35;
/// 弹簧臂伸缩平滑：缩回快（避障跟手）、伸出慢（防抖）
pub(crate) const ARM_RETRACT_RATE: f32 = 30.0;
pub(crate) const ARM_EXTEND_RATE: f32 = 5.0;

// —— 瞄准行为 ——
/// 瞄准时移动速度降至 55%
pub(crate) const AIM_MOVE_FACTOR: f32 = 0.55;
/// 瞄准时角色转身插值速率（指数平滑）
pub(crate) const AIM_TURN_RATE: f32 = 12.0;
/// 俯仰限制：仰视 50°（pitch 取负）/ 俯视 70°（pitch 正值 = 俯视）
pub(crate) const PITCH_UP_MAX: f32 = 0.873;   // 50°
pub(crate) const PITCH_DOWN_MAX: f32 = 1.217; // 70°
/// 鼠标灵敏度：X 轴（Yaw）/ Y 轴（Pitch），弧度/像素，再乘设置里的灵敏度
pub(crate) const MOUSE_SENS_X: f32 = 0.0024;
pub(crate) const MOUSE_SENS_Y: f32 = 0.0021;
/// 头部 Aim Offset 最大俯仰（约为视线俯仰的 45%，上限 25°）
pub(crate) const HEAD_AIM_RATIO: f32 = 0.45;
pub(crate) const HEAD_AIM_MAX: f32 = 0.44;

// —— 弱点打击 ——
/// 命中靶心核心的额外伤害倍率
pub(crate) const WEAKPOINT_MULT: f32 = 1.8;
/// 靶心核心判定球半径（对应靶板内环白圈；人形敌人的头部弱点半径同为 0.3~0.55）
pub(crate) const WEAKPOINT_CORE_R: f32 = 0.3;

pub fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }

/// 最短角差：把 to - from 收拢到 [-π, π]，供转身平滑使用
pub(crate) fn shortest_angle(from: f32, to: f32) -> f32 {
    let d = (to - from).rem_euclid(std::f32::consts::TAU);
    if d > std::f32::consts::PI { d - std::f32::consts::TAU } else { d }
}

/// 射线 vs AABB（slab 法），返回最近正向命中距离
pub(crate) fn ray_aabb_hit(origin: Vec3, dir: Vec3, half: Vec3, center: Vec3) -> Option<f32> {
    let mut tmin = 0.0f32;
    let mut tmax = f32::MAX;
    for axis in 0..3 {
        let o = origin[axis]; let d = dir[axis]; let h = half[axis]; let c = center[axis];
        if d.abs() < 1e-6 {
            if (o - c).abs() > h { return None; }
            continue;
        }
        let mut t1 = (c - h - o) / d;
        let mut t2 = (c + h - o) / d;
        if t1 > t2 { std::mem::swap(&mut t1, &mut t2); }
        tmin = tmin.max(t1);
        tmax = tmax.min(t2);
        if tmin > tmax { return None; }
    }
    if tmin > 0.0 { Some(tmin) } else { None }
}

/// 射线 vs 球，返回最近正向命中距离
pub(crate) fn ray_sphere_hit(origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    let oc = center - origin;
    let proj = oc.dot(dir);
    if proj < 0.0 { return None; }
    let d2 = oc.length_squared() - proj * proj;
    let r2 = radius * radius;
    if d2 > r2 { return None; }
    Some(proj - (r2 - d2).sqrt())
}

/// 越肩相机每帧驱动：
/// 1) Pivot 跟随角色脚底；ShoulderPivot/Yaw、PitchPivot/Pitch 写入鼠标视角
/// 2) SpringArm 在默认右肩机位与越肩机位间按 aim_lerp 过渡（smoothstep）
/// 3) 臂长做射线避障（撞墙缩回、离墙缓伸，最小距离钳制 + 地面钳制）
/// 4) 头部俯仰跟随视线、枪械从腰际举到肩上（程序化 Aim Offset）
pub(crate) fn aim_rig_system(
    player_query: Query<&Transform, With<Player>>,
    cam_query: Query<&PlayerCamera>,
    mut pivot_query: Query<&mut Transform, (With<CamPivot>, Without<Player>)>,
    mut yaw_query: Query<&mut Transform, (With<ShoulderPivot>, Without<CamPivot>, Without<Player>)>,
    mut pitch_query: Query<&mut Transform, (With<PitchPivot>, Without<CamPivot>, Without<ShoulderPivot>, Without<Player>)>,
    mut arm_query: Query<(&mut Transform, &mut SpringArmState), (With<SpringArm>, Without<CamPivot>, Without<ShoulderPivot>, Without<PitchPivot>, Without<Player>)>,
    mut head_query: Query<&mut Transform, (With<PlayerHeadPivot>, Without<CamPivot>, Without<ShoulderPivot>, Without<PitchPivot>, Without<SpringArm>, Without<PlayerAimGun>, Without<Player>)>,
    mut gun_query: Query<(&PlayerAimGun, &mut Transform), (Without<CamPivot>, Without<ShoulderPivot>, Without<PitchPivot>, Without<SpringArm>, Without<PlayerHeadPivot>, Without<Player>)>,
    colliders: Query<(&Transform, &Collider), (Without<CamPivot>, Without<ShoulderPivot>, Without<PitchPivot>, Without<SpringArm>, Without<PlayerHeadPivot>, Without<PlayerAimGun>, Without<Player>)>,
    time: Res<Time>,
) {
    let Ok(player_transform) = player_query.get_single() else { return };
    let Ok(cam) = cam_query.get_single() else { return };

    // 瞄准过渡系数（smoothstep）
    let step = time.delta_seconds() / ARM_TRANSITION_SECS;
    let aim_lerp = if cam.aiming { (cam.aim_lerp + step).min(1.0) } else { (cam.aim_lerp - step).max(0.0) };
    let t = aim_lerp * aim_lerp * (3.0 - 2.0 * aim_lerp);

    let pitch = cam.pitch.clamp(-PITCH_UP_MAX, PITCH_DOWN_MAX);
    // 俯仰旋转量（rotation_x 正值 = 抬头，取负与"pitch 正 = 俯视"约定对齐）
    let rot = Quat::from_rotation_y(cam.yaw + std::f32::consts::PI) * Quat::from_rotation_x(-pitch);

    if let Ok(mut pivot) = pivot_query.get_single_mut() {
        pivot.translation = player_transform.translation;
    }
    if let Ok(mut yaw_p) = yaw_query.get_single_mut() {
        yaw_p.rotation = Quat::from_rotation_y(cam.yaw + std::f32::consts::PI);
    }
    if let Ok(mut pitch_p) = pitch_query.get_single_mut() {
        pitch_p.rotation = Quat::from_rotation_x(-pitch);
    }

    // 机位参数插值
    let shoulder_x = lerp(ARM_SHOULDER_X_NORMAL, ARM_SHOULDER_X_AIM, t);
    let eye_y = lerp(ARM_EYE_Y_NORMAL, ARM_EYE_Y_AIM, t);
    let want_len = lerp(ARM_LEN_NORMAL, ARM_LEN_AIM, t);

    // 弹簧臂避障：从臂根（肩位）沿臂方向打射线，撞墙缩回，地面钳制
    let base = player_transform.translation + PIVOT_HEIGHT * Vec3::Y + rot * Vec3::new(shoulder_x, eye_y, 0.0);
    let back = rot * Vec3::Z;
    let mut blocked = want_len;
    for (col_t, col) in colliders.iter() {
        if let Some(d) = ray_aabb_hit(base, back, col.half_size, col_t.translation) {
            if d < blocked { blocked = d; }
        }
    }
    if back.y < -1e-4 {
        let t_ground = (ARM_GROUND_MIN_Y - base.y) / back.y;
        if t_ground > 0.0 && t_ground < blocked { blocked = t_ground; }
    }
    let target_len = blocked.max(ARM_MIN_LEN).min(want_len);

    if let Ok((mut arm, mut state)) = arm_query.get_single_mut() {
        let rate = if target_len < state.len { ARM_RETRACT_RATE } else { ARM_EXTEND_RATE };
        state.len = lerp(state.len, target_len, 1.0 - (-rate * time.delta_seconds()).exp());
        arm.translation = Vec3::new(shoulder_x, eye_y, state.len);
    }

    // 程序化 Aim Offset：头部随视线俯仰（约 45%，上限 25°），枪从腰际举到肩上
    let head_pitch = (-pitch * HEAD_AIM_RATIO).clamp(-HEAD_AIM_MAX, HEAD_AIM_MAX) * aim_lerp;
    if let Ok(mut head) = head_query.get_single_mut() {
        head.rotation = Quat::from_rotation_x(head_pitch);
    }
    if let Ok((gun, mut gun_t)) = gun_query.get_single_mut() {
        gun_t.translation = gun.base.lerp(gun.raised, t);
        gun_t.rotation = Quat::from_rotation_x(-pitch * 0.6 * aim_lerp);
    }
}

/// 推进 aim_lerp（与 aim_rig_system 拆开，避免只读/可变借用交织）
pub(crate) fn aim_lerp_system(
    mut cam_query: Query<&mut PlayerCamera>,
    time: Res<Time>,
) {
    let step = time.delta_seconds() / ARM_TRANSITION_SECS;
    for mut cam in &mut cam_query {
        cam.aim_lerp = if cam.aiming { (cam.aim_lerp + step).min(1.0) } else { (cam.aim_lerp - step).max(0.0) };
    }
}


/// 瞄准输入：按住右键进入越肩瞄准；手持手雷时强制瞄准（投掷姿态）。
/// UI 打开（光标解锁）且未持握时自动退出瞄准。
pub(crate) fn aim_system(
    mouse: Res<ButtonInput<MouseButton>>,
    input_state: Res<InputState>,
    held: Res<HeldGrenade>,
    mut cam_query: Query<&mut PlayerCamera>,
) {
    let Ok(mut cam) = cam_query.get_single_mut() else { return };
    cam.aiming = held.item.is_some()
        || (input_state.cursor_locked && mouse.pressed(MouseButton::Right));
}

