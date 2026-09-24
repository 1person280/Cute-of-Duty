//! 步枪射击：冷却节拍、弹药消耗、射线命中、元素结算、击杀/命中事件
//!
//! 权威半边：不再生成曳光/枪口火花/弹孔网格（那是 launcher 表现层的活），
//! 只决定“这一枪是否命中、打掉多少血、是否致死”，并据此产出 [`CombatEvent`]。

use crate::combat::{Combatant, CombatEvent, RAY_MAX_RANGE, RELOAD_TIME_SECS, WEAKPOINT_CORE_R, WEAKPOINT_MULT};
use crate::combat::range::TARGET_HIT_RADIUS;
use crate::damage::{DamagePacket, DamageResolver, Vec3};
use crate::element::{ElementType, EntityElementState};
use crate::entity::{EntityId, EntityType, World};

/// 尝试换弹：满足条件才开始，计时完成后由 `Combatant::on_tick` 补弹。
pub fn try_reload(world: &mut World, eid: EntityId) {
    let Some(cb) = world.get_entity_mut(eid).and_then(|e| e.get_component_mut::<Combatant>()) else {
        return;
    };
    if cb.reload_timer <= 0.0 && cb.ammo < cb.max_ammo && cb.ammo_pool > 0 {
        cb.reload_timer = RELOAD_TIME_SECS;
    }
}

/// 尝试开火一枪。
///
/// 受冷却（自动射速节拍）与换弹互斥限制；命中由射线对 AI 目标结算，元素修正与
/// 减伤经 `DamageResolver` 统一裁决（沿用服务端权威伤害管线）。
pub fn try_fire(
    world: &mut World,
    resolver: &DamageResolver,
    env: &EntityElementState,
    events: &mut Vec<CombatEvent>,
    eid: EntityId,
    origin: Vec3,
    yaw: f32,
    pitch: f32,
    element: ElementType,
) {
    // 阶段1：射手状态裁决（冷却 / 换弹 / 弹药），只改射手本身
    let profile = crate::operator::rifle_profile(element);
    let (can_fire, origin_hit) = {
        let Some(cb) = world.get_entity_mut(eid).and_then(|e| e.get_component_mut::<Combatant>()) else {
            return;
        };
        if cb.reload_timer > 0.0 {
            (false, Vec3::default())
        } else if cb.shoot_cooldown > 0.0 {
            (false, Vec3::default())
        } else if cb.ammo <= 0 {
            // 打空自动换弹
            if cb.ammo_pool > 0 {
                cb.reload_timer = RELOAD_TIME_SECS;
            }
            (false, Vec3::default())
        } else {
            cb.ammo -= 1;
            cb.shoot_cooldown = cb.fire_interval;
            let o = Vec3::new(origin.x, origin.y + 1.5, origin.z);
            (true, o)
        }
    };
    if !can_fire {
        return;
    }

    // 视线方向（服务端仅服从此角度，客户端无权校订弹道）
    let dir = {
        let cy = yaw.sin() * pitch.cos();
        Vec3::new(cy, pitch.sin(), yaw.cos() * pitch.cos()).try_normalize()
    };
    if dir.x == 0.0 && dir.y == 0.0 && dir.z == 0.0 {
        return;
    }

    // 阶段2：射线命中判定（只读遍历目标，选最近命中点）
    let mut best_t = RAY_MAX_RANGE;
    let mut target: Option<(EntityId, bool)> = None;
    let base_damage = profile.damage;
    for entity in world.get_all_entities() {
        if !entity.is_alive || entity.id == eid {
            continue;
        }
        let base = entity.position;
        match entity.entity_type {
            // 敌人：躯干大球 + 头部核心小球（弱点）
            EntityType::AI => {
                if let Some(t) = ray_sphere_hit(origin_hit, dir, Vec3::new(base.x, base.y + 1.5, base.z), 1.5) {
                    if t <= best_t {
                        best_t = t;
                        target = Some((entity.id, false));
                    }
                }
                if let Some(t) = ray_sphere_hit(origin_hit, dir, base, WEAKPOINT_CORE_R) {
                    if t <= best_t {
                        best_t = t;
                        target = Some((entity.id, true));
                    }
                }
            }
            // 训练靶：靶面球体，无弱点
            EntityType::Target => {
                if let Some(t) = ray_sphere_hit(origin_hit, dir, base, TARGET_HIT_RADIUS) {
                    if t <= best_t {
                        best_t = t;
                        target = Some((entity.id, false));
                    }
                }
            }
            _ => {}
        }
    }

    let Some((target_id, is_headshot)) = target else {
        return;
    };
    let damage = base_damage * if is_headshot { WEAKPOINT_MULT } else { 1.0 };

    // 阶段3：结算（沿用权威伤害管线）
    // 训练靶：只计分、不致死。先判型再计分，避免对 `world` 的双重可变借用。
    let is_target = world
        .get_entity(target_id)
        .map(|e| e.entity_type == EntityType::Target)
        .unwrap_or(false);
    if is_target {
        crate::combat::range::record_hit(world, target_id);
        // 命中靶仍要回传射手（source），事件由 main.rs 转投给射手连接
        events.push(CombatEvent::Hit { source: eid.as_u64(), target: target_id.as_u64(), is_headshot });
        return;
    }
    let Some(target_entity) = world.get_entity_mut(target_id) else {
        return;
    };
    let packet = DamagePacket::new(element, damage, eid).with_source_pos(origin_hit);
    resolver.resolve(target_entity, &packet, env);

    let victim_dead = target_entity.hp <= 0.0;
    events.push(CombatEvent::Hit { source: eid.as_u64(), target: target_id.as_u64(), is_headshot });
    if victim_dead {
        target_entity.is_alive = false;
        events.push(CombatEvent::Kill { killer: eid.as_u64(), victim: target_id.as_u64() });
    }
}

/// 射线与球面的最近正交点，返回 `t`（>0）；无交点返回 None。
///
/// 推导：`|O + t·D − C|² = r²`（D 归一化）展开为 `t² − 2(D·(C−O))t + (|C−O|² − r²) = 0`，
/// 令 `p = D·(C−O)`、`b = |C−O|² − r²`，则近交点 `t = p − √(p² − b)`。
pub fn ray_sphere_hit(origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    let oc = Vec3::new(center.x - origin.x, center.y - origin.y, center.z - origin.z);
    let proj = oc.x * dir.x + oc.y * dir.y + oc.z * dir.z;
    let b = (oc.x * oc.x + oc.y * oc.y + oc.z * oc.z) - radius * radius;
    let disc = proj * proj - b;
    if disc < 0.0 {
        return None;
    }
    let sqrt = disc.sqrt();
    let t0 = proj - sqrt;
    if t0 > 0.0 {
        return Some(t0);
    }
    let t1 = proj + sqrt;
    if t1 > 0.0 { Some(t1) } else { None }
}

trait Vec3Helper {
    fn dot(&self, o: &Vec3) -> f32;
    fn try_normalize(&self) -> Vec3;
}

impl Vec3Helper for Vec3 {
    fn dot(&self, o: &Vec3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    fn try_normalize(&self) -> Vec3 {
        let len = (self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        if len < 1e-6 {
            Vec3::default()
        } else {
            Vec3::new(self.x / len, self.y / len, self.z / len)
        }
    }
}