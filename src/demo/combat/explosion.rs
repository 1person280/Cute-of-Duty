//! 爆炸：范围伤害结算（含点燃/冰冻机制）、爆炸视觉扩展与消散

use bevy::prelude::*;
use bevy::pbr::NotShadowCaster;
use crate::element::{ElementSystem, ElementType};
use crate::model::Player;
use crate::operator::SkillEffect;
use crate::demo::frontend::*;
use crate::demo::components::*;
use crate::demo::hud::{EffectAssets, EffectMatKind};
use super::*;

/// 对爆炸点周围的目标结算范围伤害 + 干员机制（点燃/冰冻）；
/// 毒雾区域由调用方在作用点另行生成（spawn_skill_zone）。
/// 线性距离衰减（边缘保底30%）+ 元素反应，规则查核心配置表。
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_explosion_damage(
    commands: &mut Commands,
    effects: &EffectAssets,
    element_system: &ElementSystem,
    targets: &mut Query<(Entity, &mut TargetDummy, &Transform, &Children), (Without<GrenadeProjectile>, Without<Player>)>,
    pos: Vec3,
    element: ElementType,
    base_damage: f32,
    radius: f32,
    effect: SkillEffect,
) {
    for (entity, mut dummy, transform, _) in targets.iter_mut() {
        if dummy.down_timer.is_some() { continue; } // 已倒地的靶子不再受击
        let dist = transform.translation.distance(pos);
        if dist > radius { continue; }
        let falloff = (1.0 - dist / radius).max(0.3);
        let (dmg, reaction_name) = element_reaction(
            element_system,
            dummy.element_state,
            element,
            base_damage * falloff,
        );

        dummy.current_health -= dmg;
        dummy.hit_flash = Some(Timer::from_seconds(0.2, TimerMode::Once));
        // 若这一击致命，靶子被冲击波掀翻：沿爆炸中心指向靶子的方向倒下
        let blast_dir = (transform.translation - pos).with_y(0.0);
        if blast_dir.length_squared() > 1e-6 {
            dummy.fall_dir = blast_dir.normalize();
        }
        if element != ElementType::Physical {
            dummy.element_state = Some(element);
            dummy.state_timer = Some(Timer::from_seconds(3.5, TimerMode::Once));
        }

        // 机制：点燃（DoT 挂到目标身上）
        if effect.burn_secs > 0.0 && effect.burn_dps > 0.0 {
            dummy.dots.push(DamageOverTime {
                element,
                dps: effect.burn_dps,
                tick: Timer::from_seconds(DOT_TICK, TimerMode::Repeating),
                remaining: Timer::from_seconds(effect.burn_secs, TimerMode::Once),
            });
        }
        // 机制：冰冻/电麻（目标停止行动 + 冰块视觉；已冻结则只刷新时长）
        if effect.freeze_secs > 0.0 {
            if dummy.frozen.is_none() {
                dummy.frozen = Some(Timer::from_seconds(effect.freeze_secs, TimerMode::Once));
                let ice = commands.spawn((
                    PbrBundle {
                        mesh: effects.frost_cube.clone(),
                        material: effects.frost_material.clone(),
                        transform: Transform::from_translation(Vec3::new(0.0, 0.4, 0.0)),
                        ..default()
                    },
                    NotShadowCaster,
                )).id();
                commands.entity(entity).add_child(ice);
                dummy.frozen_visual = Some(ice);
            } else if let Some(t) = dummy.frozen.as_mut() {
                t.reset();
            }
        }

        let popup_text = if let Some(ref reaction) = reaction_name {
            format!("{} ({})", dmg as i32, reaction)
        } else {
            format!("{}", dmg as i32)
        };
        let popup_color = if reaction_name.is_some() {
            Color::srgb(1.0, 0.85, 0.2)
        } else {
            element.color()
        };
        spawn_damage_popup(commands, transform.translation + Vec3::Y * 0.8, popup_text, popup_color);
    }
}

pub(crate) fn spawn_explosion(
    commands: &mut Commands,
    effects: &mut EffectAssets,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
    element: ElementType,
    max_scale: f32,
) {
    commands.spawn((
        PbrBundle {
            mesh: effects.explosion_sphere.clone(),
            material: effects.material(materials, element, EffectMatKind::Explosion),
            transform: Transform::from_translation(pos).with_scale(Vec3::splat(0.1)),
            ..default()
        },
        ExplosionEffect { timer: Timer::from_seconds(0.6, TimerMode::Once), max_scale },
    ));
    for _ in 0..10 {
        let dir = Vec3::new(
            (rand::random::<f32>() - 0.5) * 2.0,
            rand::random::<f32>() * 0.8 + 0.2,
            (rand::random::<f32>() - 0.5) * 2.0,
        ).normalize();
        commands.spawn((
            PbrBundle {
                mesh: effects.explosion_debris.clone(),
                material: effects.material(materials, element, EffectMatKind::Plain),
                transform: Transform::from_translation(pos + dir * 0.5),
                ..default()
            },
            BulletHit { timer: Timer::from_seconds(0.5, TimerMode::Once) },
        ));
    }
}

pub(crate) fn explosion_expand(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut ExplosionEffect)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut effect) in query.iter_mut() {
        effect.timer.tick(time.delta());
        let t = effect.timer.elapsed_secs() / effect.timer.duration().as_secs_f32();
        let scale = effect.max_scale * t.min(1.0);
        transform.scale = Vec3::splat(scale);
        if effect.timer.finished() {
            commands.entity(entity).despawn();
        }
    }
}