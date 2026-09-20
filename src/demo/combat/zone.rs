//! 持续区域（毒雾/毒区）：展开、周期性结算与到期消散

use bevy::prelude::*;
use bevy::pbr::NotShadowCaster;
use crate::element::ElementType;
use crate::operator::SkillEffect;
use crate::demo::frontend::*;
use crate::demo::components::*;
use crate::demo::hud::{EffectAssets, EffectMatKind};
use super::*;

/// 持续区域（毒雾）：周期性对圈内目标结算机制伤害
#[derive(Component)]
pub(crate) struct SkillZone {
    pub(crate) element: ElementType,
    pub(crate) dps: f32,
    pub(crate) radius: f32,
    pub(crate) tick: Timer,
    pub(crate) lifetime: Timer,
}

/// 在作用点展开持续毒雾区（毒蛛机制）：圈内目标每 DOT_TICK 秒掉血
pub(crate) fn spawn_skill_zone(
    commands: &mut Commands,
    effects: &mut EffectAssets,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
    effect: SkillEffect,
) {
    if effect.zone_secs <= 0.0 || effect.zone_dps <= 0.0 { return; }
    // 元素取毒系固定（当前仅毒蛛配置毒区；半径沿用档案缺省 3.5）
    let element = ElementType::Poison;
    let radius = 3.5;
    commands.spawn((
        PbrBundle {
            mesh: effects.zone_cylinder.clone(),
            material: effects.material(materials, element, EffectMatKind::Zone),
            transform: Transform::from_translation(pos.with_y(0.0) + Vec3::Y * 0.7)
                .with_scale(Vec3::new(radius, 1.0, radius)),
            ..default()
        },
        NotShadowCaster,
        SkillZone {
            element,
            dps: effect.zone_dps,
            radius,
            tick: Timer::from_seconds(DOT_TICK, TimerMode::Repeating),
            lifetime: Timer::from_seconds(effect.zone_secs, TimerMode::Once),
        },
    ));
}

/// 毒雾区每帧驱动：周期掉血结算 + 到期消散
pub(crate) fn zone_tick_system(
    mut commands: Commands,
    mut zones: Query<(Entity, &mut Transform, &mut SkillZone)>,
    mut targets: Query<(&Transform, &mut TargetDummy), (Without<SkillZone>, Without<GrenadeProjectile>)>,
    element_system: Res<ElementalSystem>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut zone) in zones.iter_mut() {
        zone.lifetime.tick(time.delta());
        zone.tick.tick(time.delta());
        // 消散前塌缩视觉
        let remain = zone.lifetime.remaining_secs();
        if remain < 0.4 {
            let s = (remain / 0.4).max(0.05);
            transform.scale = Vec3::new(transform.scale.x, s, transform.scale.z);
        }
        if zone.tick.just_finished() {
            for (t, mut dummy) in targets.iter_mut() {
                if dummy.down_timer.is_some() { continue; }
                let offset = t.translation - transform.translation;
                let flat_dist = Vec3::new(offset.x, 0.0, offset.z).length();
                if flat_dist > zone.radius { continue; }
                let (dmg, _) = element_reaction(
                    &element_system.0,
                    dummy.element_state,
                    zone.element,
                    zone.dps * DOT_TICK,
                );
                dummy.current_health -= dmg;
                dummy.element_state = Some(zone.element);
                dummy.state_timer = Some(Timer::from_seconds(3.5, TimerMode::Once));
                spawn_damage_popup(
                    &mut commands,
                    t.translation + Vec3::Y * 1.2,
                    format!("{}", dmg as i32),
                    zone.element.color(),
                );
            }
        }
        if zone.lifetime.finished() {
            commands.entity(entity).despawn();
        }
    }
}