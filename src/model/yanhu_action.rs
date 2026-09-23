//! 程序化动作系统（来自 src/model/mod.rs 原"程序化动作系统"段）
//! 焰狐模型的行走 / 疾跑 / 跳跃 / 瞄准持枪 / 开火后坐 / 尾巴摇摆。
//! 全部在 YanhuLimb 枢轴（肩/髋/尾根）上做旋转合成，不触碰四肢盒子本身：
//! - 行走摆臂摆腿：正弦步态，步频与摆幅随 planar_speed 增长（疾跑明显加快加大）；
//! - 待机呼吸：手臂极小幅摆动 + 尾巴慢速摇摆（三节尾巴相位递延，链式跟随）；
//! - 跳跃滞空：双腿前后分张、左臂后摆的空中姿态（瞄准态保留持枪臂）；
//! - 瞄准：按 aim_lerp 混入持枪姿态（双臂前举托枪、微微内收），与步态平滑过渡；
//! - 开火后坐：射击冷却刚重启时给持枪臂与枪身一个衰减上抬（约 0.25s 归零）。
//! 必须排在 aim_rig_system 之后：枪枢轴的旋转由瞄准系统写入，本系统在其上叠加后坐。

use bevy::prelude::*;

use crate::demo::camera::{CamPivot, PitchPivot, ShoulderPivot, SpringArm, lerp};

use super::components::{Player, PlayerCamera, PlayerMovement};
use super::rig::{LimbKind, PlayerAimGun, PlayerHeadPivot, YanhuLimb};

pub fn yanhu_action_system(
    mut player_query: Query<&mut PlayerMovement, With<Player>>,
    cam_query: Query<&PlayerCamera>,
    mut limb_query: Query<(&YanhuLimb, &mut Transform), (
        Without<Player>, Without<PlayerHeadPivot>, Without<PlayerAimGun>,
        Without<CamPivot>, Without<ShoulderPivot>, Without<PitchPivot>, Without<SpringArm>,
    )>,
    mut gun_query: Query<&mut Transform, (
        With<PlayerAimGun>, Without<Player>, Without<PlayerHeadPivot>, Without<YanhuLimb>,
        Without<CamPivot>, Without<ShoulderPivot>, Without<PitchPivot>, Without<SpringArm>,
    )>,
    time: Res<Time>,
) {
    let Ok(cam) = cam_query.single() else { return };
    // 与 aim_rig_system 相同的 smoothstep 过渡系数
    let a = cam.aim_lerp;
    let aim_t = a * a * (3.0 - 2.0 * a);
    let t = time.elapsed_secs();
    // 尾巴慢摆相位（待机也在摇）
    let tail_idle = t * 1.7;

    for movement in player_query.iter_mut() {
        let speed_ratio = (movement.planar_speed / 4.0).min(1.4);
        let stride = t * (5.0 + 4.0 * speed_ratio); // 步频：走 5，疾跑约 10.8
        let amp = speed_ratio.min(1.15);            // 摆幅随速度增长
        let s = stride.sin();
        // 开火后坐强度：射击冷却刚重启≈1，随冷却流逝平方衰减归零
        let kick: f32 = (1.0 - movement.shoot_cooldown.fraction()).powi(2);
        let airborne = !movement.is_grounded;

        for (limb, mut tf) in limb_query.iter_mut() {
            let (mut rx, mut ry, mut rz) = (0.0f32, 0.0f32, 0.0f32);
            match limb.0 {
                LimbKind::ArmL => {
                    rx = 0.65 * amp * s + 0.04 * (t * 2.0).sin(); // 与右臂反相摆动 + 呼吸
                    rz = 0.10 * aim_t;                            // 内收扶枪
                    if airborne { rx = -0.85; }
                    rx = lerp(rx, 1.05, aim_t) + 0.08 * kick;
                }
                LimbKind::ArmR => {
                    rx = -0.65 * amp * s + 0.04 * (t * 2.0).sin();
                    rz = -0.10 * aim_t;
                    if airborne { rx = -0.85; }
                    rx = lerp(rx, 1.25, aim_t) + 0.30 * kick;     // 持枪臂吃主要后坐
                }
                LimbKind::LegL => {
                    rx = -0.55 * amp * s;
                    if airborne { rx = -0.55; }                   // 滞空前腿抬起
                }
                LimbKind::LegR => {
                    rx = 0.55 * amp * s;
                    if airborne { rx = 0.40; }                    // 滞空后腿后蹬
                }
                LimbKind::TailA => {
                    ry = 0.26 * tail_idle.sin() + 0.22 * amp * s; // 待机慢摆 + 跑动甩尾
                    rx = -0.22 * amp;                             // 疾跑时尾巴微微上扬
                }
                LimbKind::TailB => { ry = 0.32 * (tail_idle - 0.6).sin() + 0.22 * amp * s; }
                LimbKind::TailC => { ry = 0.36 * (tail_idle - 1.2).sin(); }
            }
            tf.rotation = Quat::from_euler(EulerRot::XYZ, rx, ry, rz);
        }

        // 枪身后坐：在 aim_rig_system 写入的瞄准旋转上叠加一个上抬增量
        if let Ok(mut gun_t) = gun_query.single_mut() {
            gun_t.rotation = gun_t.rotation * Quat::from_rotation_x(0.10 * kick);
        }
    }
}