//! 玩家移动积分：水平位移 + 竖直跳落的**纯数学**。
//!
//! 设计动机（Why）：`main.rs` 是服务端唯一编排者，负责"把模块接起来"，不该承载
//! 玩法数学。把「移速档位 / 按 yaw 分解的 WASD 位移 / 重力与起跳的竖直积分」收敛到
//! 一个不依赖 `combat` / `engine` 的纯函数，好处有三：
//! 1. 积分结果只由输入决定，可脱离世界状态单测，天然确定性；
//! 2. 编排者只做"取值 → 纯函数 → 写回 → 结算战斗"，借用结构与职责边界清晰；
//! 3. 常量（`SPRINT_MULT` 等）成为唯一事实来源，不再散落在入口文件里。

use crate::damage::Vec3;
use crate::net::protocol::PlayerInput;

/// 疾跑相对基础移速的倍率（`Entity.move_speed × 此值`）。基础步速 4 m/s、疾跑 ×1.75
/// = 7 m/s，对齐 legacy 0.3.2 单机时代的走/跑档位（走 4、跑 7）。
pub const SPRINT_MULT: f32 = 1.75;

/// 越肩瞄准时的移速倍率（`速度 × 此值`）。瞄准是"用机动性换精度"的博弈位：
/// 按住右键即从常态 4 m/s 降到 2.2 m/s，与旧版手感一致。
pub const AIM_MULT: f32 = 0.55;

/// 重力加速度（m/s²，服务端竖直积分用）。
pub const GRAVITY: f32 = 22.0;
/// 起跳初速（m/s）：约 1.1m 跳高，贴合 legacy 轻跳手感。
pub const JUMP_SPEED: f32 = 7.0;
/// 地面高度（草坪训练场为平地，地面 y = 0）。
pub const GROUND_Y: f32 = 0.0;

/// 单 Tick 移动积分的结果：新位置 + 新竖直速度 + 新着地标志。
///
/// 以值类型整体返回（Why）：调用方一次性拿到全部新状态再写回实体，避免"边算边写"
/// 造成的中间态可见，也让积分本身保持为可单测的纯函数。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionStep {
    /// 本 Tick 结算后的世界坐标。
    pub position: Vec3,
    /// 本 Tick 结算后的竖直速度（m/s，正 = 上升）。
    pub vertical_velocity: f32,
    /// 本 Tick 结算后是否着地（仅着地可起跳）。
    pub grounded: bool,
}

/// 按输入意图积分一个 Tick 的位置与竖直状态（纯函数，无副作用）。
///
/// 轴系（与客户端 `AimRig` 同源）：视线水平分量 `(sinY, cosY)`，右向量
/// `(-cosY, sinY)`；`W/S` 沿视线前后、`D/A` 沿右手左右。位移量 = `速度 × dt`
/// （`dt` 为固定 Tick 步长），因此速度单位是 m/s、与消息频率无关。
///
/// 竖直运动：`jump` 仅在着地时生效（防空中二段跳），离地后由重力积分把 y 拉回
/// [`GROUND_Y`] 并复位 `grounded`。全部由服务端裁决，客户端只上报意图。
pub fn integrate_motion(
    move_speed: f32,
    position: Vec3,
    grounded: bool,
    vertical_velocity: f32,
    input: &PlayerInput,
    dt: f32,
) -> MotionStep {
    let yaw = input.aim_yaw;
    let (fx, fz) = (yaw.sin(), yaw.cos()); // 视线水平方向
    let (rx, rz) = (-yaw.cos(), yaw.sin()); // 右手方向 = forward × Y

    let mut mx = 0f32;
    let mut mz = 0f32;
    if input.move_forward { mx += fx; mz += fz; }
    if input.move_backward { mx -= fx; mz -= fz; }
    if input.move_right { mx += rx; mz += rz; }
    if input.move_left { mx -= rx; mz -= rz; }

    let mut position = position;
    let mut vertical_velocity = vertical_velocity;
    let mut grounded = grounded;

    let len = (mx * mx + mz * mz).sqrt();
    if len > 1e-6 {
        let mut speed = if input.sprint { move_speed * SPRINT_MULT } else { move_speed };
        // 瞄准优先于疾跑压制移速：按住右键即进入"慢走精度档"（见 `AIM_MULT`）。
        if input.aim {
            speed *= AIM_MULT;
        }
        let step = speed * dt;
        position.x += mx / len * step;
        position.z += mz / len * step;
    }

    // 竖直运动：Space 意图起跳 + 重力积分（服务端权威，客户端只上报 jump 意图）。
    // 仅着地可起跳——防止空中连按叠成二段跳；离地后由重力把 y 拉回地面并复位 grounded。
    if input.jump && grounded {
        vertical_velocity = JUMP_SPEED;
        grounded = false;
    }
    vertical_velocity -= GRAVITY * dt;
    position.y += vertical_velocity * dt;
    if position.y <= GROUND_Y {
        position.y = GROUND_Y;
        vertical_velocity = 0.0;
        grounded = true;
    }

    MotionStep {
        position,
        vertical_velocity,
        grounded,
    }
}