//! 特效实体兜底清道夫：为高频一次性特效设定硬性存活上限
//!
//! 为什么存在：正常路径下 `BulletHit`/`DamagePopup` 等由各自的 timer 系统到期回收，
//! 但**任何**一条回收路径失效（系统提前 return、相机缺失、组件读写冲突等）都可能让
//! 某类实体无限堆积——这是 GPU 内存缓慢增长的真实候选来源之一。本模块是防呆兜底：
//! 不取代 timer，只保证某类特效实体数永远不会超过上限，超出的就地 despawn。
//! 上限取远高于正常游玩的量级，正常情况永不触发。

use bevy::prelude::*;
use super::components::*;

/// 各类特效实体的存活上限（远超正常峰值，仅防失效路径导致的无限堆积）
const MAX_BULLET_HIT: usize = 512;
const MAX_DAMAGE_POPUP: usize = 512;
const MAX_DAMAGE_PARTICLE: usize = 2048;
const MAX_FLOATING_REACTION: usize = 256;
const MAX_EXPLOSION: usize = 128;

/// 若某一类瞬态特效实体数量超过上限，将超出的就地 despawn。
/// 超出上限只发生在对应 timer 回收路径失效时，裁掉谁都不影响正确性。
/// （Bevy 的 Query 无法统一泛型抽象，故逐类展开，逻辑一致。）
pub(crate) fn effect_guard(
    mut commands: Commands,
    bullet: Query<Entity, With<BulletHit>>,
    popup: Query<Entity, With<DamagePopup>>,
    particle: Query<Entity, With<DamageParticle>>,
    reaction: Query<Entity, With<FloatingReaction>>,
    explosion: Query<Entity, With<ExplosionEffect>>,
) {
    for entity in bullet.iter().skip(MAX_BULLET_HIT) {
        commands.entity(entity).despawn();
    }
    for entity in popup.iter().skip(MAX_DAMAGE_POPUP) {
        commands.entity(entity).despawn();
    }
    for entity in particle.iter().skip(MAX_DAMAGE_PARTICLE) {
        commands.entity(entity).despawn();
    }
    for entity in reaction.iter().skip(MAX_FLOATING_REACTION) {
        commands.entity(entity).despawn();
    }
    for entity in explosion.iter().skip(MAX_EXPLOSION) {
        commands.entity(entity).despawn();
    }
}