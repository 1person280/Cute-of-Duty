//! 干员技能：Q/E 手感调度、冷却节拍与技能形态施放

use bevy::prelude::*;
use bevy::ecs::system::SystemParam;
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

/// 技能系统的输入资源：特效/材质资产 + 键盘 + 元素系统 + 光标锁态。
/// 用 SystemParam 收拢只读/可变资源，规避 too_many_arguments。
#[derive(SystemParam)]
pub(crate) struct SkillContext<'w> {
    effects: ResMut<'w, EffectAssets>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    keyboard: Res<'w, ButtonInput<KeyCode>>,
    element_system: Res<'w, ElementalSystem>,
    input_state: Res<'w, InputState>,
}

pub(crate) fn skill_system(
    mut commands: Commands,
    mut ctx: SkillContext,
    mut player_query: Query<(&mut Transform, &mut OperatorState), With<Player>>,
    mut target_query: Query<(Entity, &mut TargetDummy, &Transform, &Children), (Without<GrenadeProjectile>, Without<Player>)>,
    colliders: Query<(&Transform, &Collider), Without<Player>>,
    cam_query: Query<&PlayerCamera>,
) {
    // 背包/站点等 UI 打开（光标解锁）时不触发技能
    if !ctx.input_state.cursor_locked { return; }
    let Ok((mut player_transform, mut op)) = player_query.single_mut() else { return };
    let Ok(cam) = cam_query.single() else { return };

    // 技能定义来自核心库干员名册：形态/机制/冷却全部随干员切换
    let op_def = &roster()[op.active];
    let element = op_def.element;
    if ctx.keyboard.just_pressed(KeyCode::KeyQ) && op.q.is_finished() {
        op.q.reset();
        cast_skill(&mut commands, &mut ctx.effects, &mut ctx.materials, &ctx.element_system.0,
            &mut target_query, &colliders, SkillCast { player_transform: &mut player_transform, yaw: cam.yaw, element, skill: op_def.q });
    }
    if ctx.keyboard.just_pressed(KeyCode::KeyE) && op.e.is_finished() {
        op.e.reset();
        cast_skill(&mut commands, &mut ctx.effects, &mut ctx.materials, &ctx.element_system.0,
            &mut target_query, &colliders, SkillCast { player_transform: &mut player_transform, yaw: cam.yaw, element, skill: op_def.e });
    }
}

/// 一次技能施放的上下文：玩家朝向（player_transform + yaw）、亲和元素与技能定义
pub(crate) struct SkillCast<'a> {
    pub(crate) player_transform: &'a mut Transform,
    pub(crate) yaw: f32,
    pub(crate) element: ElementType,
    pub(crate) skill: crate::operator::SkillDef,
}

/// 施放一个技能定义：形态由核心配置（SkillKind）决定，
/// 独特机制（点燃/冰冻/毒区/位移）由 SkillEffect/Kind 驱动，元素取干员亲和元素。
pub(crate) fn cast_skill(
    commands: &mut Commands,
    effects: &mut EffectAssets,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    element_system: &ElementSystem,
    target_query: &mut Query<(Entity, &mut TargetDummy, &Transform, &Children), (Without<GrenadeProjectile>, Without<Player>)>,
    colliders: &Query<(&Transform, &Collider), Without<Player>>,
    cast: SkillCast,
) {
    let fwd = Vec3::new(cast.yaw.sin(), 0.0, cast.yaw.cos());
    match cast.skill.kind {
        SkillKind::Grenade { damage, radius } => {
            commands.spawn((
                Mesh3d(effects.projectile.clone()),
                MeshMaterial3d(effects.material(materials, cast.element, EffectMatKind::Plain)),
                Transform::from_translation(cast.player_transform.translation + Vec3::new(0.0, 2.5, 0.0) + fwd * 0.5),
                GrenadeProjectile {
                    velocity: fwd * 12.0 + Vec3::Y * 6.0,
                    element: cast.element,
                    damage,
                    radius,
                    effect: cast.skill.effect,
                    timer: Timer::from_seconds(1.5, TimerMode::Once),
                },
            ));
        }
        SkillKind::Burst { damage, radius } => {
            let burst_pos = cast.player_transform.translation + Vec3::new(0.0, 1.0, 0.0);
            // 范围伤害 + 机制（点燃/冰冻）+ 视觉特效
            apply_explosion_damage(commands, effects, element_system, target_query, ExplosionSpec {
                pos: burst_pos,
                element: cast.element,
                base_damage: damage,
                radius,
                effect: cast.skill.effect,
            });
            spawn_explosion(commands, effects, materials, burst_pos, cast.element, radius + 0.5);
            // 机制：毒雾区在脚下展开
            spawn_skill_zone(commands, effects, materials, cast.player_transform.translation, cast.skill.effect);
        }
        SkillKind::Dash { distance } => {
            // 机制：沿视线水平疾冲，逐段碰撞检测，撞障碍即停
            let start = cast.player_transform.translation;
            let half = Vec3::new(0.4, 1.0, 0.4);
            let step = 0.25;
            let mut moved = 0.0f32;
            while moved < distance {
                let next = (moved + step).min(distance);
                if collides(start + fwd * next, half, colliders) { break; }
                moved = next;
            }
            cast.player_transform.translation = start + fwd * moved;
            // 冲刺残影：起点与终点各撒一把元素火花
            for pos in [start + Vec3::Y, start + fwd * moved + Vec3::Y] {
                for _ in 0..3 {
                    let dir = Vec3::new(
                        (rand::random::<f32>() - 0.5) * 2.0,
                        rand::random::<f32>(),
                        (rand::random::<f32>() - 0.5) * 2.0,
                    ).normalize();
                    commands.spawn((
                        Mesh3d(effects.particle.clone()),
                        MeshMaterial3d(effects.material(materials, cast.element, EffectMatKind::Particle)),
                        Transform::from_translation(pos),
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
    let Ok(mut op) = query.single_mut() else { return };
    op.q.tick(time.delta());
    op.e.tick(time.delta());
}