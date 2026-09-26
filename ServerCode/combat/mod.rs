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
use crate::items::Backpack;
use crate::map::PickupKind;
use crate::operator::{roster, SkillEffect, SkillKind};

/// 护甲显示上限（与 `interact::ARMOR_CAP`、表现层同源）：速用护甲片钳制用。
const ARMOR_CAP: f32 = 100.0;

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
    /// 切枪请求（Some = 本 Tick 请求切换到该武器槽；None = 无请求）
    pub weapon_slot: Option<u8>,
    /// 使用背包第 `slot` 格物品（边沿量）：医疗包回血、护甲片加甲、手雷投掷。
    pub use_slot: Option<u8>,
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
        // 切枪先于开火：确保本 Tick 的射击用新武器元素与弹夹
        if let Some(slot) = intent.weapon_slot {
            switch_weapon(world, eid, slot as usize);
        }
        // 读取射手当前朝向与手持武器元素（不可变快照，避免借用冲突）。
        // 射击元素来自手持武器，技能元素来自干员——两者解耦（见 `combatant` 模块头注释）。
        let Some((origin, yaw, pitch, weapon_element, op_idx)) = fetch_aim(world, eid) else {
            return;
        };
        let op_element = roster()[op_idx].element;
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
                weapon_element,
            );
        }
        if intent.reload {
            shooter::try_reload(world, eid);
        }
        if intent.skill_q {
            self.cast_skill(world, resolver, env, eid, origin, yaw, pitch, op_element, op_idx, true);
        }
        if intent.skill_e {
            self.cast_skill(world, resolver, env, eid, origin, yaw, pitch, op_element, op_idx, false);
        }
        if let Some(slot) = intent.use_slot {
            use_item_at(world, eid, slot as usize, origin, yaw);
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

/// 从世界读取射击/技能所需的射手只读快照：站位、朝向、手持武器元素、干员索引。
///
/// 设计动机：`apply_input` 既要 mutate 射手（弹药/冷却）又要 mutate 目标（伤害），
/// 若先做只读快照再各行其是，可彻底规避对同一 `&mut Entity` 的双重借用。
fn fetch_aim(world: &World, eid: EntityId) -> Option<(Vec3, f32, f32, ElementType, usize)> {
    let entity = world.get_entity(eid)?;
    let cb = entity.get_component::<Combatant>()?;
    roster().get(cb.operator_idx)?;
    Some((entity.position, cb.last_yaw, cb.last_pitch, cb.element(), cb.operator_idx))
}

/// 生成一个带权威战斗状态的玩家实体并塞入世界。
///
/// 默认配发火/冰两把制式步枪（与干员解耦）；`operator_idx` 只决定技能组（Q/E）。
pub fn spawn_player(world: &mut World, position: Vec3, operator_idx: usize) -> EntityId {
    let mut entity = Entity::new_player(0, position);
    entity.entity_type = EntityType::Player;
    entity.add_component(Box::new(Combatant::new(operator_idx)));
    // 开局携带 2 医疗包 + 2 手雷（宿主为 4×3 背包组件，权威格位见 `items`）。
    entity.add_component(Box::new(Backpack::starting()));
    world.spawn(entity)
}

/// 权威切换玩家手持武器槽：越界/同槽由 [`Combatant::switch_slot`] 忽略。
///
/// 武器与干员解耦：切枪只改 `active_slot`，不影响 `operator_idx`（技能组保持不变）。
pub fn switch_weapon(world: &mut World, eid: EntityId, slot: usize) {
    let Some(entity) = world.get_entity_mut(eid) else { return };
    let Some(cb) = entity.get_component_mut::<Combatant>() else { return };
    cb.switch_slot(slot);
}

/// 权威切换玩家干员：改写战斗组件里的名册索引（决定技能组 Q/E 与技能元素）。
///
/// 设计动机：切换干员属于“应该算什么”的服务端权威责任，客户端只上报所选索引；
/// 此处按名册长度对越界索引进裁切，确保非法输入不 panic。武器与干员解耦——切干员
/// 不动 `weapons`/`active_slot`。快照 `operator_id` 随 `Combatant.operator_idx` 自动
/// 更新，客户端据此渲染干员面板高亮。
pub fn switch_operator(world: &mut World, eid: EntityId, operator_id: u32) {
    let Some(entity) = world.get_entity_mut(eid) else { return };
    let Some(cb) = entity.get_component_mut::<Combatant>() else { return };
    let roster_len = crate::operator::roster().len().max(1) as u32;
    cb.operator_idx = (operator_id % roster_len) as usize;
}

/// 拾取到的武器装进**当前手持槽**（覆盖原武器并补满弹夹），不改变干员。
///
/// 设计动机（Why）：武器归属属"应该算什么"——拾取武器只应换枪，不该连带换干员；
/// 干员只决定技能组。客户端仅上报"拾取哪个实体"，换成哪把枪由服务端裁决。
pub fn equip_weapon(world: &mut World, eid: EntityId, element: ElementType) {
    let Some(entity) = world.get_entity_mut(eid) else { return };
    let Some(cb) = entity.get_component_mut::<Combatant>() else { return };
    cb.equip_active(element);
}

/// 备弹池追加（拾取弹药 / 补给 / 物资箱取弹的权威落点）；无战斗组件的实体静默忽略。
pub fn add_ammo_pool(world: &mut World, eid: EntityId, amount: i32) {
    let Some(entity) = world.get_entity_mut(eid) else { return };
    let Some(cb) = entity.get_component_mut::<Combatant>() else { return };
    cb.ammo_pool += amount;
}

/// 把一件可携带物品放回玩家背包（拾取医疗/护甲/手雷的权威落点）。
///
/// 返回 `true` 表示已入格；背包满则返回 `false`（调用方据此拒绝拾取、不消耗拾取物）。
/// 设计动机（Why）：物品归属属"应该算什么"——`items::Backpack` 是唯一权威格位宿主，
/// 拾取只做 push，数量/格位完全由服务端维护，客户端只收快照。
pub fn push_item(world: &mut World, eid: EntityId, item: crate::items::LootItem) -> bool {
    let Some(entity) = world.get_entity_mut(eid) else { return false };
    let Some(bp) = entity.get_component_mut::<Backpack>() else { return false };
    bp.push(item)
}

/// 使用背包第 `index` 格的物品（3/4 速用与径向轮盘的统一权威入口）。
///
/// 设计动机（Why）：3/4 号速用既要支持"短按用首件"，也要支持"长按径向轮盘精确选格"，
/// 二者在服务端收敛为同一件事——按**背包格位下标**取用。效果仍由服务端裁决：
/// 恢复类（医疗包回血 / 护甲片加甲）按物品自身数值钳制到上限；战术类（手雷）按当前
/// 朝向抛出，元素取自该颗手雷本身。空位或不可速用物（弹药/武器不占格）静默忽略。
pub fn use_item_at(world: &mut World, eid: EntityId, index: usize, origin: Vec3, yaw: f32) {
    let Some(item) = ({
        let Some(entity) = world.get_entity_mut(eid) else { return };
        let Some(bp) = entity.get_component_mut::<Backpack>() else { return };
        bp.take_at(index)
    }) else {
        return;
    };
    match item.kind {
        PickupKind::Health { amount } => {
            let Some(entity) = world.get_entity_mut(eid) else { return };
            entity.hp = (entity.hp + amount).min(entity.max_hp);
        }
        PickupKind::Armor { amount } => {
            let Some(entity) = world.get_entity_mut(eid) else { return };
            entity.armor = (entity.armor + amount).min(ARMOR_CAP);
        }
        PickupKind::Grenade { element } => {
            grenade::spawn_projectile(
                world,
                eid,
                origin,
                yaw,
                element,
                combatant::GRENADE_DAMAGE,
                combatant::GRENADE_RADIUS,
                SkillEffect::NONE,
            );
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::{ElementConfig, ElementSystem};
    use crate::entity::Entity;
    use crate::items::ItemCategory;

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

        let ammo_before = h.world.get_entity(pid).unwrap().get_component::<Combatant>().unwrap().active().ammo;

        h.system.apply_input(
            &mut h.world,
            &h.resolver,
            pid,
            0.016,
            &h.env,
            CombatIntent { shoot: true, ..CombatIntent::default() },
        );

        let ammo_now = h.world.get_entity(pid).unwrap().get_component::<Combatant>().unwrap().active().ammo;
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

        let ammo_before = h.world.get_entity(pid).unwrap().get_component::<Combatant>().unwrap().active().ammo;
        h.system.apply_input(
            &mut h.world,
            &h.resolver,
            pid,
            0.016,
            &h.env,
            CombatIntent { shoot: true, ..CombatIntent::default() },
        );

        let ammo_now = h.world.get_entity(pid).unwrap().get_component::<Combatant>().unwrap().active().ammo;
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

    /// 背包内某速用类别的数量（3/4 号槽计数来源）。
    fn category_count(world: &World, pid: EntityId, cat: ItemCategory) -> i32 {
        world
            .get_entity(pid)
            .and_then(|e| e.get_component::<Backpack>())
            .map(|bp| bp.count_category(cat))
            .unwrap_or(0)
    }

    /// 3 号消耗品（按格位）：开局第 0 格医疗包回血并清空该格，钳制到上限，空格不再生效。
    #[test]
    fn medkit_use_heals_and_consumes() {
        let mut h = CombatHarness::new();
        let pid = spawn_player(&mut h.world, Vec3::default(), 0);
        h.world.get_entity_mut(pid).unwrap().hp = 20.0;
        assert_eq!(category_count(&h.world, pid, ItemCategory::Consumable), 2, "开局应带 2 个恢复类");

        h.system.apply_input(
            &mut h.world,
            &h.resolver,
            pid,
            0.016,
            &h.env,
            CombatIntent { use_slot: Some(0), ..CombatIntent::default() },
        );
        let e = h.world.get_entity(pid).unwrap();
        assert!((e.hp - 70.0).abs() < 1e-3, "应回血一个医疗包量（开局医疗包 50），实际 {}", e.hp);
        assert_eq!(category_count(&h.world, pid, ItemCategory::Consumable), 1, "背包计数应递减");

        // 用尽剩余恢复类（第 1 格）：血量钳制到上限，计数归零。
        h.system.apply_input(
            &mut h.world,
            &h.resolver,
            pid,
            0.016,
            &h.env,
            CombatIntent { use_slot: Some(1), ..CombatIntent::default() },
        );
        // 再对已清空的第 0 格速用：应无任何反应、计数不为负。
        h.system.apply_input(
            &mut h.world,
            &h.resolver,
            pid,
            0.016,
            &h.env,
            CombatIntent { use_slot: Some(0), ..CombatIntent::default() },
        );
        let e = h.world.get_entity(pid).unwrap();
        assert!(e.hp <= e.max_hp + 1e-3, "回血不得越过上限，实际 {}", e.hp);
        assert_eq!(category_count(&h.world, pid, ItemCategory::Consumable), 0, "背包不应为负");
    }

    /// 4 号消耗品（按格位）：开局第 2 格手雷生成投射物并清空该格，取尽后不再生成。
    #[test]
    fn grenade_use_throws_and_consumes() {
        let mut h = CombatHarness::new();
        let pid = spawn_player(&mut h.world, Vec3::default(), 0);
        assert_eq!(category_count(&h.world, pid, ItemCategory::Tactical), 2, "开局应带 2 颗手雷");

        h.system.apply_input(
            &mut h.world,
            &h.resolver,
            pid,
            0.016,
            &h.env,
            CombatIntent { use_slot: Some(2), ..CombatIntent::default() },
        );
        assert_eq!(count_grenades(&h.world), 1, "应生成一颗手雷投射物");
        assert_eq!(category_count(&h.world, pid, ItemCategory::Tactical), 1, "计数应递减");

        // 用尽剩余手雷（第 3 格）：恰好再生成 1 颗；再对已清空格速用不再有反应。
        h.system.apply_input(
            &mut h.world,
            &h.resolver,
            pid,
            0.016,
            &h.env,
            CombatIntent { use_slot: Some(3), ..CombatIntent::default() },
        );
        h.system.apply_input(
            &mut h.world,
            &h.resolver,
            pid,
            0.016,
            &h.env,
            CombatIntent { use_slot: Some(2), ..CombatIntent::default() },
        );
        assert_eq!(count_grenades(&h.world), 2, "生成总数应恰好等于开局手雷数");
        assert_eq!(category_count(&h.world, pid, ItemCategory::Tactical), 0, "背包不应为负");
    }
}