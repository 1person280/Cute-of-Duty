//! WASD 移动 + 鼠标视角输入系统：把本地意图折成 `PlayerInput` 上报服务端
//!
//! 设计动机（Why）：服务端权威架构下客户端绝不本地移动，只把按键/视角意图折成
//! `PlayerInput` 上报，位移由服务端按 Tick 结算后经快照回显。移动轴系取自视角
//! （[`AimRig`]）：`W` = 面朝方向、`D` = 右手方向，与服务端弹道同一 yaw —— 客户端
//! 报「朝向 + 按键」，服务端算「方向 × 速度 × dt」，天然免疫坐标篡改。
//!
//! 发包策略：仅当与上一帧**确实不同**时才发送（按 W 不动鼠标时不发，服务端每 Tick
//! 用最近一次输入继续推进；松开 W 或转动视角都会产生变化并即时上报），避免无意义噪音。

use bevy::prelude::*;
use cute_of_duty_server::net::protocol::{ClientMessage, PlayerInput};

use crate::flow::flow_state::{AimRig, SeqCounter};
use super::network::NetOut;

/// WASD + Shift 疾跑的位移意图与视角朝向上报。
pub fn input_system(
    keys: Res<ButtonInput<KeyCode>>,
    rig: Res<AimRig>,
    out: Res<NetOut>,
    mut seq: ResMut<SeqCounter>,
    mut last: Local<Option<PlayerInput>>,
) {
    let mut input = PlayerInput::default();
    input.move_forward = keys.pressed(KeyCode::KeyW);
    input.move_backward = keys.pressed(KeyCode::KeyS);
    input.move_left = keys.pressed(KeyCode::KeyA);
    input.move_right = keys.pressed(KeyCode::KeyD);
    input.sprint = keys.pressed(KeyCode::ShiftLeft);
    // 视角即瞄准意图：服务端据此旋转权威实体、结算弹道，并作为移动轴系的基准。
    input.aim_yaw = rig.yaw;
    input.aim_pitch = rig.pitch;

    if last.as_ref() == Some(&input) {
        return;
    }

    seq.0 += 1;
    input.seq = seq.0;
    *last = Some(input);
    let _ = out.0.send(ClientMessage::Input { player: input });
}