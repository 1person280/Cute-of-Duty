//! 权威战斗系统（服务器结算，纯逻辑、不依赖 bevy）
//!
//! 架构约定：所有战斗真相（弹药、技能冷却、投射物、毒区、元素结算）都只在
//! 服务端 `CombatSystem` 内计算，客户端只上报 [`CombatIntent`]、只收结果。
//! 本模块由 `engine::GameLoop` 持有，每固定 Tick 调度一次；命中/击杀以
//! [`CombatEvent`] 出队，供网络层转成 `ServerMessage::Event` 播报。
//!
//! 移植来源：legacy `src/demo/combat/{weapon,skill,grenade,explosion,zone}.rs`
//! 的权威半边（抽掉全部 Bevy 表现：曳光/粒子/弹幕/网格材质）。

use crate::damage::{DamageResolver, Vec3};
use crate::element::{ElementType, EntityElementState};
use crate::entity::{Entity, EntityId, EntityType, World};
use crate::operator::{roster, rifle_profile, SkillKind};

mod grenade;
mod shooter;
mod skill;
mod zone;

pub use combatant::Combatant;
mod combatant;
pub mod range;

/// 运行时常量：单个 Tick 内技能/换弹判定用到的数值
pub const RAY_MAX_RANGE: f32 = 60.0;
/// 弱点（头部）核心半径
pub const WEAKPOINT_CORE_R: f32 = 0.6;
/// 弱点命中倍率
pub const WEAKPOINT_MULT: f32 = 1.8;
/// 换弹所需秒数（与 legacy 一致）
pub const RELOAD_TIME_SECS: f32 = 1.8;
/// 毒区周期性掉血的结算周期（秒）
pub const ZONE_TICK_SECS: f32 = 0.5;

/// 一次战斗结算对外暴露的瞬时事件（HUD 播报用），
/// 由 `GameLoop::drain_combat_events()` 出队并转成 `ServerMessage::Event`。
#[derive(Debug, Clone)]
pub enum CombatEvent {
    /// 击杀：`killer` 击杀 `victim`
    Kill { killer: u64, victim: u64 },
    /// 被命中：`source` 击中 `target`（`is_headshot` 供 HUD 爆头判定）
    Hit { source: u64, target: u64, is_headshot: bool },
}

/// 单 Tick 的战斗意图（由 `net::protocol::PlayerInput` 在 `main.rs` 映射而来）。
///
/// 为何单独定义而不直接依赖线格式：`engine` 是与网络层解耦的确定性模拟核心，
/// 不应强依赖 `net::protocol`；这里用最小意图集做边界。
#[derive(Debug, Clone, Copy, Default)]
pub struct CombatIntent {
    /// 扳机按下（持续射击由冷却节拍）
    pub shoot: bool,
    pub reload: bool,
    pub skill_q: bool,
    pub skill_e: bool,
    /// 视线：yaw（偏航，弧度）/ pitch（俯仰，弧度）
    pub yaw: f32,
    pub pitch: f32,
}

/// 权威战斗系统：持有干员名册引用与事件出队队列。
///
/// `GameLoop` 每固定 Tick 调用 [`Self::tick_world`] 推进投射物/毒区；
/// 服务端在主循环里对每个玩家调用 [`Self::apply_input`] 结算开火/技能。
pub struct CombatSystem {
    events: Vec<CombatEvent>,
}

impl CombatSystem {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    /// 结算单个玩家的本帧战斗意图（开火/换弹/技能）。
    ///
    /// 分派给 `shooter`/`skill` 子模块；结果（弹药、冷却、命中、击杀）直接写回
    /// `world`，命中/击杀压入事件队待出队。
    pub fn apply_input(
        &mut self,
        world: &mut World,
        resolver: &DamageResolver,
        eid: EntityId,
        _dt: f32,
        env: &EntityElementState,
        intent: CombatIntent,
    ) {
        // 读取射手当前朝向与干员元素，供射击/技能共用（不可变快照，避免借用冲突）
        let Some((origin, yaw, pitch, element, op_idx)) = fetch_aim(world, eid) else {
            return;
        };
        if intent.shoot {
            shooter::try_fire(
                world,
                resolver,
                env,
                &mut self.events,
                eid,
                origin,
                yaw,
                pitch,
                element,
            );
        }
        if intent.reload {
            shooter::try_reload(world, eid);
        }
        if intent.skill_q {
            self.cast_skill(world, resolver, env, eid, origin, yaw, pitch, element, op_idx, true);
        }
        if intent.skill_e {
            self.cast_skill(world, resolver, env, eid, origin, yaw, pitch, element, op_idx, false);
        }
    }

    /// 推进世界内所有非玩家实体战斗状态：手雷飞行/引爆、毒区周期结算。
    pub fn tick_world(
        &mut self,
        world: &mut World,
        resolver: &DamageResolver,
        dt: f32,
        env: &EntityElementState,
    ) {
        grenade::tick_grenades(world, resolver, env, &mut self.events, dt);
        zone::tick_zones(world, resolver, env, dt);
    }

    /// 出队所有未播报的战斗事件（主循环 drain 后转为 `ServerMessage::Event`）。
    pub fn drain_events(&mut self) -> Vec<CombatEvent> {
        std::mem::take(&mut self.events)
    }

    /// 结算一次技能施放（Q/E 由 `is_q` 区分，档案取 `roster()[op_idx]`）。
    fn cast_skill(
        &mut self,
        world: &mut World,
        resolver: &DamageResolver,
        env: &EntityElementState,
        caster: EntityId,
        origin: Vec3,
        yaw: f32,
        _pitch: f32,
        element: ElementType,
        op_idx: usize,
        is_q: bool,
    ) {
        let ops = roster();
        if op_idx >= ops.len() {
            return;
        }
        let skill = if is_q { &ops[op_idx].q } else { &ops[op_idx].e };
        // 冷却校验 + 触发（读回当前冷却，未就绪则忽略）
        if !skill::consume_cooldown(world, caster, skill.cooldown_secs, is_q) {
            return;
        }

        match skill.kind {
            SkillKind::Grenade { damage, radius } => {
                grenade::spawn_projectile(world, caster, origin, yaw, element, damage, radius, skill.effect);
            }
            SkillKind::Burst { damage, radius } => {
                skill::burst(world, resolver, env, &mut self.events, caster, origin, element, damage, radius, skill.effect);
            }
            SkillKind::Dash { distance } => {
                skill::dash(world, caster, origin, yaw, distance);
            }
        }
    }
}

impl Default for CombatSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// 从世界读取射击/技能所需的射手只读快照：站位、朝向、干员元素。
///
/// 设计动机：`apply_input` 既要 mutate 射手（弹药/冷却）又要 mutate 目标（伤害），
/// 若先做只读快照再各行其是，可彻底规避对同一 `&mut Entity` 的双重借用。
fn fetch_aim(world: &World, eid: EntityId) -> Option<(Vec3, f32, f32, ElementType, usize)> {
    let entity = world.get_entity(eid)?;
    let cb = entity.get_component::<Combatant>()?;
    let op = roster().get(cb.operator_idx)?;
    Some((entity.position, cb.last_yaw, cb.last_pitch, op.element, cb.operator_idx))
}

/// 生成一个带权威战斗状态的玩家实体并塞入世界。
///
/// `operator_idx` 决定弹道/技能/元素亲和（取 `roster()[i]`），并提供制式弹夹。
pub fn spawn_player(world: &mut World, position: Vec3, operator_idx: usize) -> EntityId {
    let mut entity = Entity::new_player(0, position);
    entity.entity_type = EntityType::Player;
    let profile = rifle_profile(roster()[operator_idx].element);
    let mut cb = Combatant::new(operator_idx, profile.max_ammo, profile.max_ammo * 3);
    cb.fire_interval = profile.fire_interval;
    entity.add_component(Box::new(cb));
    world.spawn(entity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::{ElementConfig, ElementSystem};
    use crate::entity::Entity;

    /// 搭建最小战斗环境：空世界 + 权威结算器 + 战斗系统
    struct CombatHarness {
        world: World,
        resolver: DamageResolver,
        system: CombatSystem,
        env: EntityElementState,
    }

    impl CombatHarness {
        fn new() -> Self {
            let element_system = std::sync::Arc::new(ElementSystem::new(ElementConfig::default()));
            Self {
                world: World::new(),
                resolver: DamageResolver::new(element_system),
                system: CombatSystem::new(),
                env: EntityElementState::Normal,
            }
        }

        /// 在 `pos` 放一个默认 AI（max_hp 80）
        fn spawn_ai(&mut self, pos: Vec3) -> EntityId {
            let e = Entity::new_ai(0, pos);
            self.world.spawn(e)
        }
    }

    fn count_grenades(world: &World) -> usize {
        world.get_all_entities().iter().filter(|e| e.entity_type == EntityType::Grenade).count()
    }

    /// 玩家朝向 +Z，正前方放一个 AI：开火应命中、扣血、打出命中事件、消耗弹药。
    #[test]
    fn fire_hits_and_consumes_ammo() {
        let mut h = CombatHarness::new();
        let pid = spawn_player(&mut h.world, Vec3::default(), 0); // 焰狐：火步枪 12 伤
        let aid = h.spawn_ai(Vec3::new(0.0, 0.0, 8.0));

        let ammo_before = h.world.get_entity(pid).unwrap().get_component::<Combatant>().unwrap().ammo;

        h.system.apply_input(
            &mut h.world,
            &h.resolver,
            pid,
            0.016,
            &h.env,
            CombatIntent { shoot: true, ..CombatIntent::default() },
        );

        let ammo_now = h.world.get_entity(pid).unwrap().get_component::<Combatant>().unwrap().ammo;
        assert_eq!(ammo_now, ammo_before - 1, "开火应消耗一发弹药");

        let ai_hp = h.world.get_entity(aid).unwrap().hp;
        assert!(ai_hp < 80.0, "命中应使 AI 掉血，实际 {ai_hp}");

        let events = h.system.drain_events();
        assert!(events.iter().any(|e| matches!(e, CombatEvent::Hit { .. })), "应有命中事件");
    }

    /// 训练靶：开火命中靶机 → 消耗弹药、出命中事件、计分 +1，且靶机不致死。
    #[test]
    fn fire_hits_training_target() {
        let mut h = CombatHarness::new();
        let pid = spawn_player(&mut h.world, Vec3::default(), 0);
        // 玩家朝向 +Z（默认 yaw=0），靶放在正前方 8m
        let tid = super::range::spawn_target(&mut h.world, Vec3::new(0.0, 1.5, 8.0), "近距靶", None);

        let ammo_before = h.world.get_entity(pid).unwrap().get_component::<Combatant>().unwrap().ammo;
        h.system.apply_input(
            &mut h.world,
            &h.resolver,
            pid,
            0.016,
            &h.env,
            CombatIntent { shoot: true, ..CombatIntent::default() },
        );

        let ammo_now = h.world.get_entity(pid).unwrap().get_component::<Combatant>().unwrap().ammo;
        assert_eq!(ammo_now, ammo_before - 1, "射靶也应消耗一发弹药");

        let events = h.system.drain_events();
        assert!(
            events.iter().any(|e| matches!(e, CombatEvent::Hit { source, .. } if *source == pid.as_u64())),
            "应有以玩家为 source 的命中事件"
        );

        let target = h.world.get_entity(tid).unwrap();
        assert_eq!(target.get_component::<super::range::RangeTarget>().unwrap().hits, 1, "命中应计分置 1");
        assert!(target.is_alive, "靶机命中后仍须存活");
        assert!(target.hp == f32::MAX, "靶机血量不应被真实结算扣减");
    }

    /// 玩家连续开火直至击杀：AI 死亡、出 Kill 事件、尸体被回收。
    #[test]
    fn fire_kills_and_recycles() {
        let mut h = CombatHarness::new();
        let pid = spawn_player(&mut h.world, Vec3::default(), 0);
        let aid = h.spawn_ai(Vec3::new(0.0, 0.0, 8.0));

        for _ in 0..30 {
            if let Some(cb) = h.world.get_entity_mut(pid).and_then(|e| e.get_component_mut::<Combatant>()) {
                cb.shoot_cooldown = 0.0; // 测试里手动碾平冷却以便连发
            }
            h.system.apply_input(
                &mut h.world,
                &h.resolver,
                pid,
                0.016,
                &h.env,
                CombatIntent { shoot: true, ..CombatIntent::default() },
            );
        }

        let events = h.system.drain_events();
        assert!(
            events.iter().any(|e| matches!(e, CombatEvent::Kill { killer, victim } if *killer == pid.as_u64() && *victim == aid.as_u64())),
            "应产生含玩家与 AI 的击杀事件"
        );
        let alive = h.world.get_entity(aid).map(|e| e.is_alive).unwrap_or(false);
        assert!(!alive, "AI 不应存活");
    }

    /// 技能冷却：Q 手雷触发后 CD 置位，同帧再次 Q 不再生成第二颗手雷。
    #[test]
    fn skill_cooldown_gates_spawn() {
        let mut h = CombatHarness::new();
        let pid = spawn_player(&mut h.world, Vec3::default(), 0); // 焰狐 Q=爆燃弹 Grenade

        let skill = CombatIntent { skill_q: true, ..CombatIntent::default() };
        h.system.apply_input(&mut h.world, &h.resolver, pid, 0.016, &h.env, skill);

        let cd = h.world.get_entity(pid).unwrap().get_component::<Combatant>().unwrap().skill_q_cd;
        assert!(cd > 0.0, "施放后 Q 应进入冷却");

        let grenades_before = count_grenades(&h.world);
        h.system.apply_input(&mut h.world, &h.resolver, pid, 0.016, &h.env, skill);
        assert_eq!(grenades_before, count_grenades(&h.world), "冷却中的技能不应重复生效");
        assert!(grenades_before >= 1, "Q 手雷应被生成");
    }

    /// E 技能范围爆发：附近 AI 受范围伤害。
    #[test]
    fn burst_damages_area() {
        let mut h = CombatHarness::new();
        let pid = spawn_player(&mut h.world, Vec3::default(), 0); // 焰狐 E=焦土爆发 Burst
        let near = h.spawn_ai(Vec3::new(0.0, 0.0, 2.0));

        h.system.apply_input(
            &mut h.world,
            &h.resolver,
            pid,
            0.016,
            &h.env,
            CombatIntent { skill_e: true, ..CombatIntent::default() },
        );

        let hp = h.world.get_entity(near).unwrap().hp;
        assert!(hp < 80.0, "范围爆发应伤及圈内 AI，实际 {hp}");
    }

    /// 毒区：周期结算后圈内 AI 掉血、圈外不受影响、到期消散。
    #[test]
    fn zone_ticks_and_expires() {
        let mut h = CombatHarness::new();
        let inside = h.spawn_ai(Vec3::new(2.0, 0.0, 0.0)); // 圈内
        let far = h.spawn_ai(Vec3::new(50.0, 0.0, 50.0));   // 圈外

        super::zone::spawn_zone(&mut h.world, Vec3::default(), 6.0, 12.0, 0.5);

        for _ in 0..120 {
            h.system.tick_world(&mut h.world, &h.resolver, 1.0 / 60.0, &h.env);
        }

        let in_hp = h.world.get_entity(inside).map(|e| e.hp).expect("圈内 AI 仍在");
        assert!(in_hp < 80.0, "毒区应让圈内 AI 掉血，实际 {in_hp}");
        let far_hp = h.world.get_entity(far).unwrap().hp;
        assert_eq!(far_hp, 80.0, "圈外 AI 不应受影响");

        for _ in 0..360 {
            h.system.tick_world(&mut h.world, &h.resolver, 1.0 / 60.0, &h.env);
        }
        let zone_gone = h.world
            .get_all_entities()
            .iter()
            .all(|e| !e.has_component::<super::zone::ZoneState>());
        assert!(zone_gone, "毒区到期应消散");
    }
}