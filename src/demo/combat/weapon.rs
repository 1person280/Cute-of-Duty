//! 武器：换弹计时与主武器射击（命中判定/元素反应/曳光/枪口火光/命中反馈）

use bevy::prelude::*;
use crate::element::ElementType;
use crate::model::{Player, PlayerCamera, PlayerMovement};
use crate::demo::camera::{lerp, ray_sphere_hit, WEAKPOINT_CORE_R, WEAKPOINT_MULT};
use crate::demo::components::*;
use crate::demo::frontend::*;
use crate::demo::hud::{EffectAssets, EffectMatKind};
use crate::demo::inventory::HeldGrenade;
use super::*;

pub(crate) fn reload_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<(&mut PlayerMovement, &mut WeaponSlot, &mut Inventory), With<Player>>,
    time: Res<Time>,
    input_state: Res<InputState>,
) {
    let Ok((mut movement, weapon_slot, mut inventory)) = player_query.get_single_mut() else { return };

    let current_idx = weapon_slot.current;

    // Tick reload timer if active
    if let Some(ref mut timer) = movement.reload_timer {
        timer.tick(time.delta());
        if timer.finished() {
            // 换弹从背包弹药池取弹
            let widx = current_idx;
            let needed = inventory.weapons[widx].max_ammo - inventory.weapons[widx].ammo;
            if needed > 0 {
                let take = needed.min(inventory.ammo_pool);
                inventory.weapons[widx].ammo += take;
                inventory.ammo_pool -= take;
            }
            movement.reload_timer = None;
        }
    }

    // UI 打开（光标解锁）时 R 留给背包道具使用，不触发换弹
    if !input_state.cursor_locked { return; }

    // Start reload on R key press
    if keyboard.just_pressed(KeyCode::KeyR) && movement.reload_timer.is_none() {
        if inventory.weapons[weapon_slot.current].ammo < inventory.weapons[weapon_slot.current].max_ammo
            && inventory.ammo_pool > 0
        {
            let reload_time = 1.8; // seconds
            movement.reload_timer = Some(Timer::from_seconds(reload_time, TimerMode::Once));
        }
    }
}

pub(crate) fn shooting_system(
    mut commands: Commands,
    mut effects: ResMut<EffectAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mouse: Res<ButtonInput<MouseButton>>,
    element_system: Res<ElementalSystem>,
    cam_query: Query<&PlayerCamera>,
    cam_gtransform: Query<&GlobalTransform, With<PlayerCamera>>,
    mut player_query: Query<(&Transform, &mut PlayerMovement, &mut WeaponSlot, &mut Inventory), With<Player>>,
    mut target_query: Query<(Entity, &mut TargetDummy, &Transform, &Children)>,
    time: Res<Time>,
    input_state: Res<InputState>,
    held: Res<HeldGrenade>,
) {
    let Ok((player_transform, mut movement, weapon_slot, mut inventory)) = player_query.get_single_mut() else { return };
    movement.shoot_cooldown.tick(time.delta());

    // 背包/轮盘/站点等 UI 打开（光标解锁）时不射击
    if !input_state.cursor_locked { return; }

    // 手雷持握（瞄准投掷姿态）时左键交给投掷，步枪不射击
    if held.item.is_some() { return; }

    // Cannot shoot while reloading
    if movement.reload_timer.is_some() { return; }

    let shooting = mouse.pressed(MouseButton::Left);
    if !shooting || !movement.shoot_cooldown.finished() { return; }

    let idx = weapon_slot.current;
    let empty = inventory.weapons[idx].ammo <= 0;
    if empty {
        // Auto-reload when empty（备弹来自背包弹药池）
        if inventory.ammo_pool > 0 && movement.reload_timer.is_none() {
            let reload_time = 1.8;
            movement.reload_timer = Some(Timer::from_seconds(reload_time, TimerMode::Once));
        }
        return;
    }
    inventory.weapons[idx].ammo -= 1;
    let element = inventory.weapons[idx].element;
    let damage = inventory.weapons[idx].damage;
    let fire_interval = inventory.weapons[idx].fire_interval;
    movement.shoot_cooldown = Timer::from_seconds(fire_interval, TimerMode::Once);

    let Ok(cam) = cam_query.get_single() else { return };

    // 命中判定以准星为准：射线从相机出发、沿视线方向（与屏幕中心一致）。
    // 角色与相机之间的物体不参与检测（SpringArm 已做相机避障，射线只查靶子）。
    let cam_gtf = cam_gtransform.get_single().copied()
        .map(|tf| tf.compute_transform())
        .unwrap_or_default();
    let origin = cam_gtf.translation;
    let dir = cam_gtf.forward().normalize();

    // Raycast：躯干大球（宽松判定）+ 靶心核心小球（弱点）
    let mut closest: Option<(Entity, f32, Vec3, bool)> = None;
    for (entity, dummy, transform, _children) in target_query.iter() {
        if dummy.down_timer.is_some() { continue; } // 已倒地的靶子不再受击
        let base = transform.translation;
        let mut hit_t = ray_sphere_hit(origin, dir, base + Vec3::Y * 1.5, 1.5);
        let mut weak = false;
        // 弱点：靶板中心红心（人形敌人则对应头部区域）
        if let Some(t_core) = ray_sphere_hit(origin, dir, base, WEAKPOINT_CORE_R) {
            if hit_t.map_or(true, |t| t_core < t) {
                hit_t = Some(t_core);
                weak = true;
            }
        }
        if let Some(t) = hit_t {
            if t <= 60.0 && closest.map_or(true, |(_, d, _, _)| t < d) {
                closest = Some((entity, t, origin + dir * t, weak));
            }
        }
    }

    let end = if let Some((_, d, _, _)) = closest { origin + dir * d } else { origin + dir * 60.0 };
    // 曳光从枪口出发、收敛到命中点（TPS 标准做法：判定跟准星，视觉跟枪口）
    let aim_t = cam.aim_lerp * cam.aim_lerp * (3.0 - 2.0 * cam.aim_lerp);
    let muzzle = player_transform.translation
        + Quat::from_rotation_y(movement.model_yaw)
            * Vec3::new(0.4, lerp(1.3, 2.35, aim_t) + 0.08, lerp(1.15, 1.05, aim_t));
    let mid = (muzzle + end) / 2.0;
    let len = (end - muzzle).length();

    // Laser trail（共享网格按弹道长度缩放 Z）
    let color = element.color();
    commands.spawn((
        PbrBundle {
            mesh: effects.tracer.clone(),
            material: effects.material(&mut materials, element, EffectMatKind::Plain),
            transform: Transform::from_translation(mid)
                .looking_at(end, Vec3::Y)
                .with_scale(Vec3::new(1.0, 1.0, len)),
            ..default()
        },
        BulletHit { timer: Timer::from_seconds(0.06, TimerMode::Once) },
    ));

    // Muzzle flash
    commands.spawn((
        PbrBundle {
            mesh: effects.spark.clone(),
            material: effects.muzzle_material.clone(),
            transform: Transform::from_translation(muzzle),
            ..default()
        },
        BulletHit { timer: Timer::from_seconds(0.04, TimerMode::Once) },
    ));

    // Hit processing
    if let Some((entity, _, hit_point, weak)) = closest {
        if let Ok((_, mut dummy, _, _)) = target_query.get_mut(entity) {
            // 元素反应：查询核心配置表（与cod1共用同一套规则）
            // 弱点打击：命中靶心核心，基础伤害 ×1.8
            let (dmg, reaction_name) = element_reaction(
                &element_system.0,
                dummy.element_state,
                element,
                damage * if weak { WEAKPOINT_MULT } else { 1.0 },
            );

            dummy.current_health -= dmg;
            dummy.hit_flash = Some(Timer::from_seconds(0.2, TimerMode::Once));
            // 若这一击致命，靶子朝来弹方向倒下（取弹道水平分量）
            dummy.fall_dir = Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();
            if element != ElementType::Physical {
                dummy.element_state = Some(element);
                dummy.state_timer = Some(Timer::from_seconds(3.5, TimerMode::Once));
            }

            // Damage popup (element damage / actual damage / weak point)
            let popup_text = if weak {
                if let Some(ref reaction) = reaction_name {
                    format!("弱点 {} ({})", dmg as i32, reaction)
                } else {
                    format!("弱点 {}", dmg as i32)
                }
            } else if let Some(ref reaction) = reaction_name {
                format!("{} ({})", dmg as i32, reaction)
            } else {
                format!("{}", dmg as i32)
            };
            let popup_color = if weak {
                Color::srgb(1.0, 0.45, 0.1) // hot orange for weak points
            } else if reaction_name.is_some() {
                Color::srgb(1.0, 0.85, 0.2) // gold for reactions
            } else {
                color
            };
            spawn_damage_popup(&mut commands, hit_point + Vec3::Y * 0.5, popup_text, popup_color);

            // Damage particles
            for _ in 0..5 {
                let dir = Vec3::new(
                    (rand::random::<f32>() - 0.5) * 2.0,
                    rand::random::<f32>(),
                    (rand::random::<f32>() - 0.5) * 2.0,
                ).normalize();
                commands.spawn((
                    PbrBundle {
                        mesh: effects.particle.clone(),
                        material: effects.material(&mut materials, element, EffectMatKind::Particle),
                        transform: Transform::from_translation(hit_point),
                        ..default()
                    },
                    DamageParticle {
                        velocity: dir * 3.0,
                        timer: Timer::from_seconds(0.5, TimerMode::Once),
                    },
                ));
            }

            // Hit explosion
            commands.spawn((
                PbrBundle {
                    mesh: effects.spark.clone(),
                    material: effects.material(&mut materials, element, EffectMatKind::HitFlash),
                    transform: Transform::from_translation(hit_point).with_scale(Vec3::splat(2.5)),
                    ..default()
                },
                BulletHit { timer: Timer::from_seconds(0.15, TimerMode::Once) },
            ));

            // Reaction text
            if let Some(name) = reaction_name {
                spawn_reaction_text(&mut commands, hit_point + Vec3::Y * 0.8, name, color);
            }
        }
    }
}