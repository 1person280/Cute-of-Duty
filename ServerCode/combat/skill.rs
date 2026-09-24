//! 干员技能结算：冷却消耗、范围爆发（Burst）、疾冲位移（Dash）
//!
//! 权威半边：只结算冷却/伤害/位移，特效交给 launcher。Grenade 形态的弹道在
//! [`super::grenade`] 处理（投掷物实体），这里聚焦玩家自身立即可见的效果。

use crate::combat::{CombatEvent, Combatant};
use crate::damage::{DamageResolver, Vec3};
use crate::element::{ElementType, EntityElementState};
use crate::entity::{EntityId, World};
use crate::operator::SkillEffect;

/// 尝试消耗一次技能冷却；就绪则置冷却并返回 true（未就绪返回 false 不触发）。
pub fn consume_cooldown(world: &mut World, caster: EntityId, cooldown_secs: f32, is_q: bool) -> bool {
    let Some(cb) = world.get_entity_mut(caster).and_then(|e| e.get_component_mut::<Combatant>()) else {
        return false;
    };
    if !cb.skill_ready(is_q) {
        return false;
    }
    cb.trigger_skill_cd(is_q, cooldown_secs);
    true
}

/// 以自身为中心的范围爆发：立即结算范围内伤的伤害 + 机制。
pub fn burst(
    world: &mut World,
    resolver: &DamageResolver,
    env: &EntityElementState,
    events: &mut Vec<CombatEvent>,
    caster: EntityId,
    origin: Vec3,
    element: ElementType,
    damage: f32,
    radius: f32,
    effect: SkillEffect,
) {
    let center = Vec3::new(origin.x, origin.y + 1.0, origin.z);
    super::grenade::apply_explosion(world, resolver, env, events, caster, center, element, damage, radius, effect);
}

/// 朝视线方向水平疾冲（雷豹 Q）；撞墙不做精细碰撞，直接滑到目标距离。
pub fn dash(world: &mut World, caster: EntityId, origin: Vec3, yaw: f32, distance: f32) {
    let fwd = Vec3::new(yaw.sin(), 0.0, yaw.cos());
    let next = Vec3::new(origin.x + fwd.x * distance, origin.y, origin.z + fwd.z * distance);
    if let Some(e) = world.get_entity_mut(caster) {
        e.position = next;
    }
}