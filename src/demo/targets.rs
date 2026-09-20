//! 靶标：静态靶倒地/复位、移动靶巡逻、准星命中反馈

use bevy::prelude::*;
use crate::element::ElementType;
use crate::model::{
    Player, PlayerCamera, PlayerMovement,
};
use super::hud::EffectAssets;
use super::combat::{spawn_damage_popup, spawn_explosion};
use super::frontend::*;
use super::components::*;

pub(crate) fn target_dummy_logic(
    mut commands: Commands,
    mut effects: ResMut<EffectAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut query: Query<(Entity, &mut TargetDummy, &mut Transform)>,
    mut kill_events: EventWriter<KillEvent>,
    time: Res<Time>,
) {
    for (_entity, mut dummy, mut transform) in query.iter_mut() {
        let dt = time.delta();

        // 冰冻/电麻计时：到时解冻并移除冰块视觉
        if let Some(ref mut frozen) = dummy.frozen {
            frozen.tick(dt);
            if frozen.finished() {
                dummy.frozen = None;
                if let Some(ice) = dummy.frozen_visual.take() {
                    commands.entity(ice).despawn();
                }
            }
        }

        // 持续伤害（点燃等）：按 tick 结算并弹出伤害数字
        let mut pending_dmg = 0.0f32;
        let mut pending_popups: Vec<(f32, ElementType)> = Vec::new();
        for dot in dummy.dots.iter_mut() {
            dot.remaining.tick(dt);
            dot.tick.tick(dt);
            if dot.tick.just_finished() {
                let dmg = dot.dps * DOT_TICK;
                pending_dmg += dmg;
                pending_popups.push((dmg, dot.element));
            }
        }
        dummy.current_health -= pending_dmg;
        for (dmg, element) in pending_popups {
            spawn_damage_popup(&mut commands,
                transform.translation + Vec3::Y * 1.2,
                format!("{}", dmg as i32),
                element.color());
        }
        dummy.dots.retain(|dot| !dot.remaining.finished());

        let fall_dir = dummy.fall_dir;
        if let Some(ref mut down) = dummy.down_timer {
            // 倒地阶段：前 fall_time 秒播放翻倒动画，之后躺到计时结束原地复活
            down.tick(time.delta());
            let fall_progress = (down.elapsed_secs() / TARGET_FALL_TIME).min(1.0);
            let eased = fall_progress * fall_progress; // 加速下坠感
            let axis = Vec3::Y.cross(fall_dir);
            if axis.length_squared() > 1e-6 {
                transform.rotation = Quat::from_axis_angle(axis.normalize(), eased * std::f32::consts::FRAC_PI_2);
            }
            if down.finished() {
                dummy.current_health = dummy.max_health;
                dummy.element_state = None;
                dummy.state_timer = None;
                dummy.hit_flash = None;
                dummy.down_timer = None;
                dummy.dots.clear();
                dummy.frozen = None;
                if let Some(ice) = dummy.frozen_visual.take() {
                    commands.entity(ice).despawn();
                }
                transform.rotation = Quat::IDENTITY;
            }
        } else if dummy.current_health <= 0.0 {
            // 击倒瞬间：爆炸特效 + 击杀播报，进入倒地状态（不再满血瞬间重置）
            // 同时清空持续效果（点燃熄灭、冰块碎裂）
            kill_events.send(KillEvent { name: dummy.label.to_string() });
            dummy.current_health = 0.0;
            dummy.down_timer = Some(Timer::from_seconds(TARGET_DOWN_SECS, TimerMode::Once));
            dummy.element_state = None;
            dummy.state_timer = None;
            dummy.hit_flash = None;
            dummy.dots.clear();
            dummy.frozen = None;
            if let Some(ice) = dummy.frozen_visual.take() {
                commands.entity(ice).despawn();
            }
            spawn_explosion(&mut commands, &mut effects, &mut materials,
                transform.translation + Vec3::Y * 1.0, ElementType::Physical, 2.5);
        }
    }
}

pub(crate) fn moving_target_logic(
    mut query: Query<(&mut Transform, &mut MovingTarget, Option<&TargetDummy>)>,
    time: Res<Time>,
) {
    for (mut transform, mut target, dummy) in query.iter_mut() {
        // 倒地或被冰冻期间停止巡逻
        if dummy.map(|d| d.down_timer.is_some() || d.frozen.is_some()).unwrap_or(false) { continue; }
        let offset = transform.translation - target.origin;
        if offset.x.abs() > target.range { target.direction *= -1.0; }
        transform.translation.x += target.direction * target.speed * time.delta_seconds();
    }
}

// =============================================================================
// HUD Dynamic Systems
// =============================================================================

pub(crate) fn crosshair_hit_feedback(
    mut crosshair_lines: Query<&mut BackgroundColor, With<CrosshairLine>>,
    mut crosshair_center: Query<&mut BackgroundColor, (With<CrosshairCenter>, Without<CrosshairLine>)>,
    mut player_query: Query<&mut PlayerMovement, With<Player>>,
    cam_query: Query<&PlayerCamera>,
) {
    let Ok(movement) = player_query.get_single_mut() else { return };
    let t = movement.shoot_cooldown.elapsed_secs() / movement.shoot_cooldown.duration().as_secs_f32();
    // 命中反馈（红） > 瞄准态（琥珀） > 常态（白）
    let aiming = cam_query.get_single().map(|c| c.aim_lerp > 0.35).unwrap_or(false);
    let color = if t < 0.3 {
        Color::srgba(1.0, 0.3, 0.3, 0.9)
    } else if aiming {
        Color::srgba(1.0, 0.8, 0.25, 0.9)
    } else {
        Color::srgba(0.95, 0.95, 0.95, 0.9)
    };
    for mut bg in crosshair_lines.iter_mut() { bg.0 = color; }
    for mut bg in crosshair_center.iter_mut() { bg.0 = color; }
}

