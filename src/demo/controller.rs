//! FPS 控制器：移动与 AABB 碰撞、武器切换

use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use crate::model::{
    Player, PlayerCamera, PlayerMovement,
};
use super::camera::{MOUSE_SENS_X, MOUSE_SENS_Y, PITCH_UP_MAX, PITCH_DOWN_MAX, shortest_angle, AIM_TURN_RATE, AIM_MOVE_FACTOR};
use super::pause::GameSettings;
use super::components::*;

pub(crate) fn fps_controller(
    mut player_query: Query<(&mut Transform, &mut PlayerMovement), With<Player>>,
    mut cam_query: Query<&mut PlayerCamera>,
    mut mouse_events: EventReader<MouseMotion>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    input_state: Res<InputState>,
    settings: Res<GameSettings>,
    colliders: Query<(&Transform, &Collider), Without<Player>>,
) {
    if !input_state.cursor_locked {
        // Clear mouse events so they don't accumulate
        mouse_events.clear();
        return;
    }

    let dt = time.delta_seconds();
    let mut delta = Vec2::ZERO;
    for ev in mouse_events.read() { delta += ev.delta; }

    let Ok(mut cam) = cam_query.get_single_mut() else { return };
    if delta != Vec2::ZERO {
        // 鼠标 X → 肩轴 Yaw，Y → 俯仰 Pitch；两轴灵敏度独立
        cam.yaw -= delta.x * MOUSE_SENS_X * settings.mouse_sensitivity;
        cam.pitch += delta.y * MOUSE_SENS_Y * settings.mouse_sensitivity;
        // 俯仰限制：仰视 50° / 俯视 70°（pitch 正值 = 俯视）
        cam.pitch = cam.pitch.clamp(-PITCH_UP_MAX, PITCH_DOWN_MAX);
    }

    let yaw = cam.yaw;
    for (mut transform, mut movement) in player_query.iter_mut() {
        // 角色朝向：瞄准时以最短角差 + 指数平滑追相机朝向（等效球面插值，
        // 快速转身不抖动）；普通视角保持原有的即时同向
        if cam.aiming {
            let diff = shortest_angle(movement.model_yaw, yaw);
            movement.model_yaw += diff * (1.0 - (-AIM_TURN_RATE * dt).exp());
        } else {
            movement.model_yaw = yaw;
        }
        transform.rotation = Quat::from_rotation_y(movement.model_yaw);
        let mut input = Vec3::ZERO;
        if keyboard.pressed(KeyCode::KeyW) { input.z += 1.0; }
        if keyboard.pressed(KeyCode::KeyS) { input.z -= 1.0; }
        if keyboard.pressed(KeyCode::KeyA) { input.x -= 1.0; }
        if keyboard.pressed(KeyCode::KeyD) { input.x += 1.0; }

        // 移动输入始终相对相机方向（W = 相机前方）；瞄准时移速降至 55%
        let mut speed = if keyboard.pressed(KeyCode::ShiftLeft) { 7.0 } else { 4.0 };
        if cam.aiming { speed *= AIM_MOVE_FACTOR; }
        // 动作系统读这个值决定步频/摆幅（撞墙时仍保持走姿，输入在即视为移动）
        movement.planar_speed = if input != Vec3::ZERO { speed } else { 0.0 };
        if input != Vec3::ZERO {
            input = input.normalize();
            let forward = Vec3::new(yaw.sin(), 0.0, yaw.cos());
            let right = forward.cross(Vec3::Y);
            let desired_move = (forward * input.z + right * input.x) * speed * dt;
            let new_pos = transform.translation + desired_move;
            // Simple AABB collision: resolve X and Z separately
            let player_half = Vec3::new(0.4, 1.0, 0.4);
            let mut resolved = transform.translation;
            // Try X move
            let try_x = Vec3::new(new_pos.x, resolved.y, resolved.z);
            if !collides(try_x, player_half, &colliders) {
                resolved.x = try_x.x;
            }
            // Try Z move
            let try_z = Vec3::new(resolved.x, resolved.y, new_pos.z);
            if !collides(try_z, player_half, &colliders) {
                resolved.z = try_z.z;
            }
            transform.translation = resolved;
        }

        if keyboard.just_pressed(KeyCode::Space) && movement.is_grounded {
            movement.velocity.y = 6.5;
            movement.is_grounded = false;
        }
        if !movement.is_grounded {
            movement.velocity.y -= 18.0 * dt;
            let mut new_y = transform.translation + movement.velocity * dt;
            // Collision for vertical falling
            let player_half = Vec3::new(0.4, 1.0, 0.4);
            if collides(new_y, player_half, &colliders) && movement.velocity.y < 0.0 {
                movement.velocity.y = 0.0;
                movement.is_grounded = true;
                // Snap to nearest safe Y
                let step = 0.1;
                for _ in 0..20 {
                    new_y.y += step;
                    if !collides(new_y, player_half, &colliders) {
                        transform.translation = new_y;
                        break;
                    }
                }
            } else {
                transform.translation = new_y;
                if transform.translation.y <= 0.0 {
                    transform.translation.y = 0.0;
                    movement.velocity.y = 0.0;
                    movement.is_grounded = true;
                }
            }
        }
    }
}

pub(crate) fn collides(
    player_pos: Vec3,
    player_half: Vec3,
    colliders: &Query<(&Transform, &Collider), Without<Player>>,
) -> bool {
    for (t, c) in colliders.iter() {
        let min_a = player_pos - player_half;
        let max_a = player_pos + player_half;
        let min_b = t.translation - c.half_size;
        let max_b = t.translation + c.half_size;
        if min_a.x < max_b.x && max_a.x > min_b.x
            && min_a.y < max_b.y && max_a.y > min_b.y
            && min_a.z < max_b.z && max_a.z > min_b.z
        {
            return true;
        }
    }
    false
}

// =============================================================================
// 越肩第三人称瞄准（SpringArm 相机架构，参考原神弓手瞄准模式）
// =============================================================================

// —— 相机装配 ——

pub(crate) fn weapon_switch(keyboard: Res<ButtonInput<KeyCode>>, mut query: Query<&mut WeaponSlot, With<Player>>) {
    let Ok(mut slot) = query.get_single_mut() else { return };
    if keyboard.just_pressed(KeyCode::Digit1) && slot.current != 0 { slot.current = 0; }
    if keyboard.just_pressed(KeyCode::Digit2) && slot.current != 1 { slot.current = 1; }
}
