//! WASD 移动输入系统：把本地按键折成 `PlayerInput` 上报服务端
//!
//! 设计动机：服务端权威架构下客户端绝不本地移动，只把按键意图折成 `PlayerInput`
//! 上报，位移由服务端结算后经快照回显。按 W 前进（-Z 朝向射击馆/北端撤离区）。
//! 只有在探测到本帧确有位移意图时才发包（减少无意义的网络噪音）。

use bevy::prelude::*;
use cute_of_duty_server::net::protocol::{ClientMessage, PlayerInput};

use crate::flow::flow_state::SeqCounter;
use super::network::NetOut;

/// WASD + Shift 疾跑的位移意图上报。
pub fn input_system(
    keys: Res<ButtonInput<KeyCode>>,
    out: Res<NetOut>,
    mut seq: ResMut<SeqCounter>,
) {
    let mut input = PlayerInput::default();
    input.move_forward = keys.pressed(KeyCode::KeyW);
    input.move_backward = keys.pressed(KeyCode::KeyS);
    input.move_left = keys.pressed(KeyCode::KeyA);
    input.move_right = keys.pressed(KeyCode::KeyD);
    input.sprint = keys.pressed(KeyCode::ShiftLeft);

    let moving =
        input.move_forward || input.move_backward || input.move_left || input.move_right || input.sprint;
    if !moving {
        return;
    }

    seq.0 += 1;
    input.seq = seq.0;
    let _ = out.0.send(ClientMessage::Input { player: input });
}