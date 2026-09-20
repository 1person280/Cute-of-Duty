//! 干员技能：Q/E 手感调度、冷却节拍与技能形态施放

use bevy::prelude::*;
use crate::element::{ElementSystem, ElementType};
use crate::model::{OperatorState, Player, PlayerCamera};
use crate::operator::{roster, SkillKind};
use crate::demo::frontend::*;
use crate::demo::components::*;
use crate::demo::controller::collides;
use crate::demo::hud::{EffectAssets, EffectMatKind};
use super::*;

/// 背包战术手雷的伤害与半径（干员 Q 手雷的数值由核心库干员档案提供）
pub(crate) const ITEM_GRENADE_DAMAGE: f32 = 40.0;
pub(crate) const ITEM_GRENADE_RADIUS: f32 = 3.5;

pub(crate) fn skill_system(
    mut commands: Commands,
    mut effects: ResMut<EffectAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    element_system: Res<ElementalSystem>,
    mut player_query: Query<(&mut Transform, &mut OperatorState), With<Player>>,
    mut target_query: Query<(Entity, &mut TargetDummy, &Transform, &Children), (Without<GrenadeProjectile>, Without<Player>)>,
    colliders: Query<(&Transform, &Collider), Without<Player>>,
    cam_query: Query<&PlayerCamera>,
    input_state: Res<InputState>,
) {
    // 背包/站点等 UI 打开（光标解锁）时不触发技能
    if !input_state.cursor_locked { return; }
    let Ok((mut player_transform, mut op)) = player_query.get_single_mut() else { return };
    let Ok(cam) = cam_query.get_single() else { return };

    // 技能定义来自核心库干员名册：形态/机制/冷却全部随干员切换
    let op_def = &roster()[op.active];
    let element = op_def.element;
    if keyboard.just_pressed(KeyCode::KeyQ) && op.q.finished() {
        op.q.reset();
        cast_skill(&mut commands, &mut effects, &mut materials, &element_system.0,
            &mut target_query, &colliders, &mut player_transform, cam.yaw, element, op_def.q);
    }
    if keyboard.just_pressed(KeyCode::KeyE) && op.e.finished() {
        op.e.reset();
        cast_skill(&mut commands, &mut effects, &mut materials, &element_system.0,
            &mut target_query, &colliders, &mut player_transform, cam.yaw, element, op_def.e);
    }
}

/// 施放一个技能定义：形态由核心配置（SkillKind）决定，
/// 独特机制（点燃/冰冻/毒区/位移）由 SkillEffect/Kind 驱动，元素取干员亲和元素。
#[allow(clippy::too_many_arguments)]
pub(crate) fn cast_skill(
    commands: &mut Commands,
    effects: &mut EffectAssets,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    element_system: &ElementSystem,
    target_query: &mut Query<(Entity, &mut TargetDummy, &Transform, &Children), (Without<GrenadeProjectile>, Without<Player>)>,
    colliders: &Query<(&Transform, &Collider), Without<Player>>,
    player_transform: &mut Transform,
    yaw: f32,
    element: ElementType,
    skill: crate::operator::SkillDef,
) {
    let fwd = Vec3::new(yaw.sin(), 0.0, yaw.cos());
    match skill.kind {
        SkillKind::Grenade { damage, radius } => {
            commands.spawn((
                PbrBundle {
                    mesh: effects.projectile.clone(),
                    material: effects.material(materials, element, EffectMatKind::Plain),
                    transform: Transform::from_translation(player_transform.translation + Vec3::new(0.0, 2.5, 0.0) + fwd * 0.5),
                    ..default()
                },
                GrenadeProjectile {
                    velocity: fwd * 12.0 + Vec3::Y * 6.0,
                    element,
                    damage,
                    radius,
                    effect: skill.effect,
                    timer: Timer::from_seconds(1.5, TimerMode::Once),
                },
            ));
        }
        SkillKind::Burst { damage, radius } => {
            let burst_pos = player_transform.translation + Vec3::new(0.0, 1.0, 0.0);
            // 范围伤害 + 机制（点燃/冰冻）+ 视觉特效
            apply_explosion_damage(commands, effects, element_system, target_query,
                burst_pos, element, damage, radius, skill.effect);
            spawn_explosion(commands, effects, materials, burst_pos, element, radius + 0.5);
            // 机制：毒雾区在脚下展开
            spawn_skill_zone(commands, effects, materials, player_transform.translation, skill.effect);
        }
        SkillKind::Dash { distance } => {
            // 机制：沿视线水平疾冲，逐段碰撞检测，撞障碍即停
            let start = player_transform.translation;
            let half = Vec3::new(0.4, 1.0, 0.4);
            let step = 0.25;
            let mut moved = 0.0f32;
            while moved < distance {
                let next = (moved + step).min(distance);
                if collides(start + fwd * next, half, colliders) { break; }
                moved = next;
            }
            player_transform.translation = start + fwd * moved;
            // 冲刺残影：起点与终点各撒一把元素火花
            for pos in [start + Vec3::Y, start + fwd * moved + Vec3::Y] {
                for _ in 0..6 {
                    let dir = Vec3::new(
                        (rand::random::<f32>() - 0.5) * 2.0,
                        rand::random::<f32>(),
                        (rand::random::<f32>() - 0.5) * 2.0,
                    ).normalize();
                    commands.spawn((
                        PbrBundle {
                            mesh: effects.particle.clone(),
                            material: effects.material(materials, element, EffectMatKind::Particle),
                            transform: Transform::from_translation(pos),
                            ..default()
                        },
                        DamageParticle {
                            velocity: dir * 2.5,
                            timer: Timer::from_seconds(0.35, TimerMode::Once),
                        },
                    ));
                }
            }
        }
    }
}

pub(crate) fn operator_cooldown_tick(mut query: Query<&mut OperatorState, With<Player>>, time: Res<Time>) {
    let Ok(mut op) = query.get_single_mut() else { return };
    op.q.tick(time.delta());
    op.e.tick(time.delta());
}