//! 内存泄漏诊断：定时采样并打印各类特效实体存活数与资产表规模
//!
//! 为什么存在：README「已知问题」记载长时间游玩 GPU 内存仍缓慢增长（最终 OOM）。
//! 特效网格/材质虽已入池共享、实体大多靠 timer 销毁，但增长点尚不明确——本模块
//! 在不改变任何玩法行为的前提下，把"实体/资产是否在涨"变成可观测的日志，据此
//! 才能判断泄漏在实体层、`Assets` 层还是渲染层，避免盲目改代码。
//! 默认仅 `info!` 打印，对性能与手感无影响；定位完成后应随泄漏一起删除。

use bevy::prelude::*;
use crate::demo::hud::EffectAssets;
use crate::demo::combat::SkillZone;
use super::components::*;

/// 采样节拍：每 SAMPLE_INTERVAL_SECS 秒打印一组计数
#[derive(Resource)]
pub(crate) struct TracerTimer {
    acc: f32,
    samples: u32,
}

impl Default for TracerTimer {
    fn default() -> Self {
        Self { acc: 0.0, samples: 0 }
    }
}

const SAMPLE_INTERVAL_SECS: f32 = 5.0;

/// 眼花缭乱的特效实体计数：实时反映"有特效未回收"
#[derive(Default)]
struct EntityCounts {
    bullet_hit: usize,
    damage_popup: usize,
    floating_reaction: usize,
    damage_particle: usize,
    explosion: usize,
    skill_zone: usize,
    grenade: usize,
}

/// 每 5 秒采样一次：特效实体有几类在累积 + 网格/材质资产表增长趋势。
/// 用法：`cargo run --features demo` 游玩数分钟后看日志里各列是否持续上涨。
pub(crate) fn memory_tracer(
    mut timer: ResMut<TracerTimer>,
    time: Res<Time>,
    bullet: Query<Entity, With<BulletHit>>,
    popup: Query<Entity, With<DamagePopup>>,
    reaction: Query<Entity, With<FloatingReaction>>,
    particle: Query<Entity, With<DamageParticle>>,
    explosion: Query<Entity, With<ExplosionEffect>>,
    zone: Query<Entity, With<SkillZone>>,
    grenade: Query<Entity, With<GrenadeProjectile>>,
    meshes: Res<Assets<Mesh>>,
    materials: Res<Assets<StandardMaterial>>,
    effects: Res<EffectAssets>,
) {
    timer.acc += time.delta_secs();
    if timer.acc < SAMPLE_INTERVAL_SECS {
        return;
    }
    timer.acc = 0.0;
    timer.samples += 1;

    let c = EntityCounts {
        bullet_hit: bullet.iter().count(),
        damage_popup: popup.iter().count(),
        floating_reaction: reaction.iter().count(),
        damage_particle: particle.iter().count(),
        explosion: explosion.iter().count(),
        skill_zone: zone.iter().count(),
        grenade: grenade.iter().count(),
    };
    tracing::info!(
        sample = timer.samples,
        bullet_hit = c.bullet_hit,
        damage_popup = c.damage_popup,
        floating_reaction = c.floating_reaction,
        damage_particle = c.damage_particle,
        explosion = c.explosion,
        skill_zone = c.skill_zone,
        grenade = c.grenade,
        mesh_assets = meshes.iter().count(),
        material_assets = materials.iter().count(),
        cached_effect_materials = effects.element_materials.len(),
        "memory tracer sample"
    );
}