//! 手雷/投掷物：飞行物理、落地引爆（范围伤害 + 机制 + 视觉）

use bevy::prelude::*;
use crate::model::Player;
use crate::demo::frontend::*;
use crate::demo::components::*;
use crate::demo::hud::EffectAssets;
use super::*;

pub(crate) fn grenade_physics(
    mut commands: Commands,
    mut effects: ResMut<EffectAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    element_system: Res<ElementalSystem>,
    mut query: Query<(Entity, &mut Transform, &mut GrenadeProjectile), Without<TargetDummy>>,
    mut target_query: Query<(Entity, &mut TargetDummy, &Transform, &Children), (Without<GrenadeProjectile>, Without<Player>)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut grenade) in query.iter_mut() {
        grenade.timer.tick(time.delta());
        grenade.velocity.y -= 12.0 * time.delta_secs();
        transform.translation += grenade.velocity * time.delta_secs();

        // 落地或引信耗尽时引爆：范围伤害 + 机制 + 视觉特效（数值由投掷物自带）
        if transform.translation.y <= 0.15 {
            transform.translation.y = 0.15;
            let impact = transform.translation;
            apply_explosion_damage(&mut commands, &effects, &element_system.0, &mut target_query, ExplosionSpec {
                pos: impact,
                element: grenade.element,
                base_damage: grenade.damage,
                radius: grenade.radius,
                effect: grenade.effect,
            });
            spawn_skill_zone(&mut commands, &mut effects, &mut materials, impact, grenade.effect);
            spawn_explosion(&mut commands, &mut effects, &mut materials, impact, grenade.element, grenade.radius + 1.5);
            commands.entity(entity).despawn();
            continue;
        }
        if grenade.timer.is_finished() {
            let impact = transform.translation;
            apply_explosion_damage(&mut commands, &effects, &element_system.0, &mut target_query, ExplosionSpec {
                pos: impact,
                element: grenade.element,
                base_damage: grenade.damage,
                radius: grenade.radius,
                effect: grenade.effect,
            });
            spawn_skill_zone(&mut commands, &mut effects, &mut materials, impact, grenade.effect);
            spawn_explosion(&mut commands, &mut effects, &mut materials, impact, grenade.element, grenade.radius + 1.5);
            commands.entity(entity).despawn();
        }
    }
}