//! 毒雾/毒区：周期性范围内结算（毒蛛机制）
//!
//! 权威半边：只做周期掉血与到期消散，柱状视觉网格交给 launcher。
//! 半径沿用 legacy 的固定 3.5；元素固定毒系。

use crate::damage::{DamagePacket, DamageResolver, Vec3};
use crate::element::{ElementType, EntityElementState};
use crate::entity::{Component, Entity, EntityId, EntityType, World};
use std::any::Any;

/// 持续毒区（组件）：记录 DPS、作用半径、结算周期与存活时长
pub struct ZoneState {
    pub dps: f32,
    pub radius: f32,
    pub tick_accum: f32,
    pub tick_interval: f32,
    pub lifetime: f32,
}

impl Component for ZoneState {
    fn name(&self) -> &'static str {
        "ZoneState"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// 在 `pos` 展开一片持续毒雾。
pub fn spawn_zone(world: &mut World, pos: Vec3, zone_secs: f32, zone_dps: f32, tick_interval: f32) {
    let radius = 3.5;
    let mut entity = Entity::new(0, Vec3::new(pos.x, 0.0, pos.z));
    entity.entity_type = EntityType::Obstacle;
    entity.add_component(Box::new(ZoneState {
        dps: zone_dps,
        radius,
        tick_accum: 0.0,
        tick_interval,
        lifetime: zone_secs,
    }));
    world.spawn(entity);
}

/// 每 Tick 推进所有毒区：周期结算圈内 AI、到期消散。
pub fn tick_zones(world: &mut World, resolver: &DamageResolver, env: &EntityElementState, dt: f32) {
    // 阶段1：只读收集毒区快照
    struct Rec {
        id: EntityId,
        pos: Vec3,
        dps: f32,
        radius: f32,
        tick_interval: f32,
        lifetime: f32,
        accum: f32,
    }
    let mut recs: Vec<Rec> = Vec::new();
    for e in world.get_all_entities() {
        if !e.has_component::<ZoneState>() {
            continue;
        }
        if let Some(z) = e.get_component::<ZoneState>() {
            recs.push(Rec {
                id: e.id,
                pos: e.position,
                dps: z.dps,
                radius: z.radius,
                tick_interval: z.tick_interval,
                lifetime: z.lifetime,
                accum: z.tick_accum,
            });
        }
    }

    let mut to_despawn: Vec<EntityId> = Vec::new();
    for rec in &mut recs {
        rec.lifetime -= dt;
        rec.accum += dt;
        let mut fire_tick = false;
        if rec.accum >= rec.tick_interval {
            rec.accum -= rec.tick_interval;
            fire_tick = true;
        }

        // 回写衰减后的状态（供组件保持周期/寿命连续性）
        if let Some(z) = world.get_entity_mut(rec.id).and_then(|e| e.get_component_mut::<ZoneState>()) {
            z.tick_accum = rec.accum;
            z.lifetime = rec.lifetime;
        }

        // 周期结算：圈内 AI 每周期掉血（沿用权威伤害管线）
        if fire_tick {
            let targets: Vec<EntityId> = world
                .get_all_entities()
                .iter()
                .filter(|e| e.is_alive && e.entity_type == EntityType::AI)
                .map(|e| e.id)
                .collect();
            for tid in targets {
                let Some(dx) = world.get_entity(tid).map(|e| e.position.x - rec.pos.x) else { continue };
                let Some(dz) = world.get_entity(tid).map(|e| e.position.z - rec.pos.z) else { continue };
                let flat = (dx * dx + dz * dz).sqrt();
                if flat > rec.radius {
                    continue;
                }
                let Some(target) = world.get_entity_mut(tid) else { continue };
                // 毒区元素固定毒系（spawn_zone 已裁死），周期结算的伤害基底 = dps × 周期
                let packet = DamagePacket::new(ElementType::Poison, rec.dps * rec.tick_interval, rec.id)
                    .with_source_pos(rec.pos);
                let _ = resolver.resolve(target, &packet, env);
                target.element_state = EntityElementState::Poisoned;
                if target.hp <= 0.0 {
                    target.is_alive = false;
                }
            }
        }

        if rec.lifetime <= 0.0 {
            to_despawn.push(rec.id);
        }
    }
    for id in to_despawn {
        world.despawn(id);
    }
}