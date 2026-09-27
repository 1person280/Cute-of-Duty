//! 持雷投掷轨迹预览（表现层）：持握手雷时按**与服务器同源**的弹道常数预测抛物线，
//! 用点状弧线画出预计落点，供玩家判断投掷距离。
//!
//! 设计动机（Why）：手雷"先瞄准后释放"，没有预览就只能盲投。弹道常数（水平/竖直初速、
//! 重力、出手点）直接复用 `ServerCode::combat::grenade` 导出的常量，保证预览与实际结算
//! **同源不漂移**；本系统纯为表现，不参与任何判定——真正落点仍由服务端结算。

use bevy::prelude::*;
use cute_of_duty_server::combat::grenade::{
    FUSE_SECS, GRAVITY, GROUND_Y, SPAWN_FWD, SPAWN_HEIGHT, THROW_SPEED, THROW_UP,
};

use crate::flow::flow_state::{AimRig, LocalPlayer};
use crate::hud::HeldGrenadeState;
use crate::net::snapshot::SnapshotBuffer;

/// 积分步长（秒）：`MAX_STEPS * STEP = 2s`，覆盖引信 1.5s。
const STEP: f32 = 0.05;
const MAX_STEPS: usize = 40;
/// 弧线采样点半径与落点标记半径（米）。
const DOT_R: f32 = 0.06;
const LANDING_R: f32 = 0.16;

/// 持雷时每帧绘制预测抛物线（未持雷不画）。
///
/// 出手点/初速逐项对齐 [`cute_of_duty_server::combat::grenade::spawn_projectile`]：
/// 起点 = 玩家脚底 + 沿朝向 `SPAWN_FWD` + 抬高 `SPAWN_HEIGHT`；初速 = 朝向水平 `THROW_SPEED`
/// + 竖直 `THROW_UP`；每步先从竖直速度扣 `GRAVITY * STEP`，再按速度推进。
pub fn draw_grenade_preview(
    held: Res<HeldGrenadeState>,
    rig: Res<AimRig>,
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    mut gizmos: Gizmos,
) {
    if held.element.is_none() {
        return;
    }
    // 本人条目暂缺（AOI/对账间隙）时不画，避免用错误起点误导。
    let Some(me) = snap.current.iter().find(|e| e.entity_id == player.entity_id) else {
        return;
    };
    // 方向只用 yaw（与服务端 `aim_of` 一致，当前弹道不随俯仰角变化）。
    let fwd = Vec3::new(rig.yaw.sin(), 0.0, rig.yaw.cos());
    let mut pos = Vec3::new(
        me.x + fwd.x * SPAWN_FWD,
        me.y + SPAWN_HEIGHT,
        me.z + fwd.z * SPAWN_FWD,
    );
    let mut vel = Vec3::new(fwd.x * THROW_SPEED, THROW_UP, fwd.z * THROW_SPEED);
    let color = Color::srgb(1.0, 0.78, 0.22);

    let mut t = 0.0_f32;
    for _ in 0..MAX_STEPS {
        vel.y -= GRAVITY * STEP;
        pos += vel * STEP;
        t += STEP;
        // 触地或引信到点：在落点画一个更大的标记，结束。
        if pos.y <= GROUND_Y {
            gizmos.sphere(Vec3::new(pos.x, GROUND_Y, pos.z), Quat::IDENTITY, LANDING_R, color);
            break;
        }
        if t >= FUSE_SECS {
            gizmos.sphere(pos, Quat::IDENTITY, LANDING_R, color);
            break;
        }
        gizmos.sphere(pos, Quat::IDENTITY, DOT_R, color);
    }
}
