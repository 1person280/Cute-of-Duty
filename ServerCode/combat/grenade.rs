//! 手雷/投掷物与范围爆炸：飞行物理、引爆、区域伤害、点燃 DoT
//!
//! 权威半边：只做弹道/伤害/机制（点燃、毒区落点），曳光与爆裂网格交给 launcher。
//! 范围伤害沿用 legacy `apply_explosion_damage` 的线性距离衰减（边缘保底 30%）。

use crate::combat::{CombatEvent, ZONE_TICK_SECS};
use crate::damage::{DamagePacket, DamageResolver, Vec3};
use crate::element::{ElementType, EntityElementState};
use crate::entity::{Component, Entity, EntityId, EntityType, World};
use crate::operator::SkillEffect;
use std::any::Any;

/// 飞行中手雷（组件）：携带速度、元素、伤害、半径与附加机制
pub struct GrenadeState {
    pub velocity: Vec3,
    pub element: ElementType,
    pub damage: f32,
    pub radius: f32,
    pub effect: SkillEffect,
    pub timer: f32,
}

/// 点燃/灼烧 DoT：命中目标周期性掉血（焰狐机制）
pub struct BurnDot {
    pub dps: f32,
    pub remaining: f32,
}

impl Component for GrenadeState {
    fn name(&self) -> &'static str {
        "GrenadeState"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Component for BurnDot {
    fn on_tick(&mut self, owner: &mut Entity, delta_time: f32) {
        if self.remaining <= 0.0 {
            return;
        }
        let step = delta_time.min(self.remaining);
        owner.hp -= self.dps * step;
        self.remaining -= step;
    }
    fn name(&self) -> &'static str {
        "BurnDot"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// 干员 Q 手雷：在视线前方抛出手雷投射物。
pub fn spawn_projectile(
    world: &mut World,
    _caster: EntityId,
    origin: Vec3,
    yaw: f32,
    element: ElementType,
    damage: f32,
    radius: f32,
    effect: SkillEffect,
) {
    let fwd = Vec3::new(yaw.sin(), 0.0, yaw.cos());
    let mut entity = Entity::new_grenade(0, Vec3::new(
        origin.x + fwd.x * 0.5,
        origin.y + 2.5,
        origin.z + fwd.z * 0.5,
    ));
    entity.entity_type = EntityType::Grenade;
    entity.add_component(Box::new(GrenadeState {
        velocity: Vec3::new(fwd.x * 12.0, 6.0, fwd.z * 12.0),
        element,
        damage,
        radius,
        effect,
        timer: 1.5,
    }));
    world.spawn(entity);
}

/// 每 Tick 推进所有手雷：抛物线飞行、落地/引信引爆。
pub fn tick_grenades(
    world: &mut World,
    resolver: &DamageResolver,
    env: &EntityElementState,
    events: &mut Vec<CombatEvent>,
    dt: f32,
) {
    // 阶段1：只读积分，收集每个手雷的下一帧位姿与是否引爆
    let mut plan: Vec<(EntityId, Vec3, Option<Vec3>)> = Vec::new();
    for entity in world.get_all_entities() {
        if entity.entity_type != EntityType::Grenade {
            continue;
        }
        let Some(g) = entity.get_component::<GrenadeState>() else { continue };
        let mut vel = g.velocity;
        vel.y -= 12.0 * dt;
        let pos = Vec3::new(
            entity.position.x + vel.x * dt,
            entity.position.y + vel.y * dt,
            entity.position.z + vel.z * dt,
        );
        let mut impact: Option<Vec3> = None;
        if pos.y <= 0.15 {
            impact = Some(Vec3::new(pos.x, 0.15, pos.z));
        } else if g.timer - dt <= 0.0 {
            impact = Some(pos);
        }
        plan.push((entity.id, pos, impact));
    }
    // 阶段2：应用——引爆结算或推进位置
    let mut to_despawn: Vec<EntityId> = Vec::new();
    for (id, pos, impact) in plan {
        if let Some(spot) = impact {
            if let Some(g) = world.get_entity(id).and_then(|e| e.get_component::<GrenadeState>()) {
                let effect = g.effect;
                apply_explosion(world, resolver, env, events, id, spot, g.element, g.damage, g.radius, effect);
            }
            to_despawn.push(id);
        } else if let Some(e) = world.get_entity_mut(id) {
            e.position = pos;
        }
    }
    for id in to_despawn {
        world.despawn(id);
    }
}

/// 在 `pos` 结算以 `element` 为属性的范围内所有 AI 目标，施加距离衰减 + 机制。
///
/// 供手雷引爆与 `Burst` 技能共用，保证两处范围伤害规则完全一致。
pub fn apply_explosion(
    world: &mut World,
    resolver: &DamageResolver,
    env: &EntityElementState,
    events: &mut Vec<CombatEvent>,
    source: EntityId,
    pos: Vec3,
    element: ElementType,
    base_damage: f32,
    radius: f32,
    effect: SkillEffect,
) {
    // 毒雾区在落点展开（毒蛛机制），与逐目标结算解耦，避免对 world 的双重可变借用
    if effect.zone_secs > 0.0 && effect.zone_dps > 0.0 {
        crate::combat::zone::spawn_zone(world, pos, effect.zone_secs, effect.zone_dps, ZONE_TICK_SECS);
    }

    // 只读收集范围内 AI 目标（含本实体按类型筛选；排除非 AI）
    let targets: Vec<EntityId> = world
        .get_all_entities()
        .iter()
        .filter(|e| e.is_alive && e.entity_type == EntityType::AI)
        .map(|e| e.id)
        .collect();

    for tid in targets {
        let Some(dist) = world.get_entity(tid).map(|e| e.position.distance(&pos)) else { continue };
        if dist > radius {
            continue;
        }
        let falloff = (1.0 - dist / radius).max(0.3);
        let Some(target) = world.get_entity_mut(tid) else { continue };
        let packet = DamagePacket::new(element, base_damage * falloff, source).with_source_pos(pos);
        resolver.resolve(target, &packet, env);

        // 机制：点燃 DoT 挂到目标
        if effect.burn_secs > 0.0 && effect.burn_dps > 0.0 {
            target.add_component(Box::new(BurnDot {
                dps: effect.burn_dps,
                remaining: effect.burn_secs,
            }));
        }
        // 机制：冰冻/电麻（记录元素状态；硬控的移动遏制由 launcher 表现）
        if effect.freeze_secs > 0.0 {
            target.element_state = EntityElementState::Frozen;
        }

        if target.hp <= 0.0 {
            target.is_alive = false;
            events.push(CombatEvent::Kill { killer: source.as_u64(), victim: tid.as_u64() });
        } else {
            // 载具伤害事件仅统计命中（爆炸没有“弱点头”概念）
            events.push(CombatEvent::Hit { source: source.as_u64(), target: tid.as_u64(), is_headshot: false });
        }
    }
}