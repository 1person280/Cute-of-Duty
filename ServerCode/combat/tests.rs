//! `combat` 模块的验收测试（与实现分离，避免 `mod.rs` 越过单文件行数红线）。
//!
//! 覆盖：开火命中/击杀、技能冷却与范围爆发、毒区周期结算、3/4 号速用消耗，
//! 以及手雷"先瞄准后释放"的**持握/投掷/取消**三段式权威流程。

use super::*;
use crate::element::{ElementConfig, ElementSystem};
use crate::entity::Entity;
use crate::items::{Backpack, ItemCategory};

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

/// 玩家当前是否处于"持握手雷"状态（服务端权威 `HeldGrenade`）。
fn holding_grenade(world: &World, pid: EntityId) -> bool {
    world
        .get_entity(pid)
        .and_then(|e| e.get_component::<HeldGrenade>())
        .map(|h| h.is_holding())
        .unwrap_or(false)
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
    // 0.15.0：命中事件须携带服务端权威的伤害数值与反应结果（本枪无元素附着，反应应为 None）。
    let hit = events
        .iter()
        .find_map(|e| match e {
            CombatEvent::Hit { source, target, is_headshot, damage, reaction }
                if *source == pid.as_u64() && *target == aid.as_u64() => {
                Some((*is_headshot, *damage, reaction.clone()))
            }
            _ => None,
        })
        .expect("应有以玩家为 source、AI 为 target 的命中事件");
    assert!(hit.1 > 0.0, "命中事件应携带正的伤害数值，实际 {}", hit.1);
    assert!(hit.2.is_none(), "对普通 AI 的直射不应触发反应，实际 {:?}", hit.2);
    assert!(!hit.0, "躯干命中不应判定爆头");
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

/// 背包内第一个属于 `cat` 类别的格位下标。
///
/// 设计动机：速用测试要按"语义（哪个类别）"取格，而非硬编码下标——堆叠布局会随
/// 物品上限调整而变化（同类消耗品会并堆），硬编码下标极易随之失效。
fn first_slot_of(world: &World, pid: EntityId, cat: ItemCategory) -> Option<usize> {
    world
        .get_entity(pid)
        .and_then(|e| e.get_component::<Backpack>())
        .and_then(|bp| {
            bp.slots
                .iter()
                .position(|s| s.as_ref().map(|it| it.kind.category()) == Some(Some(cat)))
        })
}

/// 3 号消耗品（按格位）：开局医疗包回血并在用尽后清格，钳制到上限，空格不再生效。
#[test]
fn medkit_use_heals_and_consumes() {
    let mut h = CombatHarness::new();
    let pid = spawn_player(&mut h.world, Vec3::default(), 0);
    h.world.get_entity_mut(pid).unwrap().hp = 20.0;
    assert_eq!(category_count(&h.world, pid, ItemCategory::Consumable), 2, "开局应带 2 个恢复类");

    let slot = first_slot_of(&h.world, pid, ItemCategory::Consumable).expect("应有恢复类格位");
    h.system.apply_input(
        &mut h.world,
        &h.resolver,
        pid,
        0.016,
        &h.env,
        CombatIntent { use_slot: Some(slot as u8), ..CombatIntent::default() },
    );
    let e = h.world.get_entity(pid).unwrap();
    assert!((e.hp - 70.0).abs() < 1e-3, "应回血一个医疗包量（开局医疗包 50），实际 {}", e.hp);
    assert_eq!(category_count(&h.world, pid, ItemCategory::Consumable), 1, "背包计数应递减");

    // 用尽剩余恢复类（同一格内的第二件）：血量钳制到上限，计数归零。
    h.system.apply_input(
        &mut h.world,
        &h.resolver,
        pid,
        0.016,
        &h.env,
        CombatIntent { use_slot: Some(slot as u8), ..CombatIntent::default() },
    );
    // 再对已清空的格速用：应无任何反应、计数不为负。
    h.system.apply_input(
        &mut h.world,
        &h.resolver,
        pid,
        0.016,
        &h.env,
        CombatIntent { use_slot: Some(slot as u8), ..CombatIntent::default() },
    );
    let e = h.world.get_entity(pid).unwrap();
    assert!(e.hp <= e.max_hp + 1e-3, "回血不得越过上限，实际 {}", e.hp);
    assert_eq!(category_count(&h.world, pid, ItemCategory::Consumable), 0, "背包不应为负");
}

/// 手雷"先瞄准后释放"：速用只**进入持握**（取出但不消耗投射物、袋内递减），
/// 左键释放才真正投出（生成投射物并清空持握态）。
#[test]
fn grenade_use_holds_then_shoot_throws() {
    let mut h = CombatHarness::new();
    let pid = spawn_player(&mut h.world, Vec3::default(), 0);
    assert_eq!(category_count(&h.world, pid, ItemCategory::Tactical), 2, "开局应带 2 颗手雷");

    let slot = first_slot_of(&h.world, pid, ItemCategory::Tactical).expect("应有手雷格位");
    h.system.apply_input(
        &mut h.world,
        &h.resolver,
        pid,
        0.016,
        &h.env,
        CombatIntent { use_slot: Some(slot as u8), ..CombatIntent::default() },
    );
    assert_eq!(count_grenades(&h.world), 0, "速用只应进入持握，不生成投射物");
    assert_eq!(category_count(&h.world, pid, ItemCategory::Tactical), 1, "取出即从背包递减");
    assert!(holding_grenade(&h.world, pid), "应处于持握手雷状态");

    // 左键释放：生成一颗投射物并退出持握态（不再抑制后续枪械射击）。
    h.system.apply_input(
        &mut h.world,
        &h.resolver,
        pid,
        0.016,
        &h.env,
        CombatIntent { shoot: true, ..CombatIntent::default() },
    );
    assert_eq!(count_grenades(&h.world), 1, "左键释放应投出一颗手雷");
    assert!(!holding_grenade(&h.world, pid), "投出后应退出持握态");

    // 对已清空的格再速用：无任何反应；背包计数不为负。
    h.system.apply_input(
        &mut h.world,
        &h.resolver,
        pid,
        0.016,
        &h.env,
        CombatIntent { use_slot: Some(slot as u8), ..CombatIntent::default() },
    );
    assert_eq!(category_count(&h.world, pid, ItemCategory::Tactical), 0, "背包不应为负");
}

/// 持雷时 Esc 取消：手雷**原样放回背包**（不消耗）、不生成投射物、退出持握态。
#[test]
fn grenade_cancel_returns_to_backpack() {
    let mut h = CombatHarness::new();
    let pid = spawn_player(&mut h.world, Vec3::default(), 0);
    let slot = first_slot_of(&h.world, pid, ItemCategory::Tactical).expect("应有手雷格位");

    // 进入持握。
    h.system.apply_input(
        &mut h.world,
        &h.resolver,
        pid,
        0.016,
        &h.env,
        CombatIntent { use_slot: Some(slot as u8), ..CombatIntent::default() },
    );
    assert!(holding_grenade(&h.world, pid), "应先进入持握态");
    let after_take = category_count(&h.world, pid, ItemCategory::Tactical);
    assert_eq!(after_take, 1, "持握中袋内应少一颗");

    // Esc 取消：放回背包。
    h.system.apply_input(
        &mut h.world,
        &h.resolver,
        pid,
        0.016,
        &h.env,
        CombatIntent { grenade_cancel: true, ..CombatIntent::default() },
    );
    assert!(!holding_grenade(&h.world, pid), "取消后应退出持握态");
    assert_eq!(count_grenades(&h.world), 0, "取消不应生成投射物");
    assert_eq!(category_count(&h.world, pid, ItemCategory::Tactical), 2, "取消后手雷应原样放回");
}

/// 持雷时左键**不**触发枪械射击：弹药不减少（投掷取代开火）。
#[test]
fn holding_grenade_suppresses_gun_fire() {
    let mut h = CombatHarness::new();
    let pid = spawn_player(&mut h.world, Vec3::default(), 0);
    let slot = first_slot_of(&h.world, pid, ItemCategory::Tactical).expect("应有手雷格位");
    h.system.apply_input(
        &mut h.world,
        &h.resolver,
        pid,
        0.016,
        &h.env,
        CombatIntent { use_slot: Some(slot as u8), ..CombatIntent::default() },
    );

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
    assert_eq!(ammo_now, ammo_before, "持雷时左键应投雷而非开枪，弹药不应变化");
}

/// 手雷弹道必须**累积重力**：竖直速度逐 Tick 递减、高度先升后降（抛物线）。
///
/// 回归背景：`grenade::tick_grenades` 曾只在局部副本上扣重力、**未写回组件**，
/// 导致每 Tick 都从初速重新起步、竖直速度恒定 → 手雷走直线（实测"投掷物为直线，无视重力"）。
#[test]
fn thrown_grenade_follows_parabola() {
    let mut h = CombatHarness::new();
    let pid = spawn_player(&mut h.world, Vec3::default(), 0);
    let slot = first_slot_of(&h.world, pid, ItemCategory::Tactical).expect("应有手雷格位");
    h.system.apply_input(
        &mut h.world, &h.resolver, pid, 0.016, &h.env,
        CombatIntent { use_slot: Some(slot as u8), ..CombatIntent::default() },
    );
    h.system.apply_input(
        &mut h.world, &h.resolver, pid, 0.016, &h.env,
        CombatIntent { shoot: true, ..CombatIntent::default() },
    );
    assert_eq!(count_grenades(&h.world), 1, "应已投出一颗手雷");
    let gid = h.world
        .get_all_entities()
        .iter()
        .find(|e| e.entity_type == EntityType::Grenade)
        .map(|e| e.id)
        .expect("投射物应存在");

    let dt = 1.0 / 60.0;
    let vel_y = |w: &World| {
        w.get_entity(gid).and_then(|e| e.get_component::<super::grenade::GrenadeState>()).map(|g| g.velocity.y)
    };
    let pos_y = |w: &World| w.get_entity(gid).map(|e| e.position.y);

    let v0 = vel_y(&h.world).expect("投射物应有速度组件");
    h.system.tick_world(&mut h.world, &h.resolver, dt, &h.env);
    let v1 = vel_y(&h.world).expect("投射物应有速度组件");
    assert!(v1 < v0, "竖直速度应随重力递减（{v0} -> {v1}）");

    let start_y = pos_y(&h.world).expect("投射物应有位置");
    let mut peak = start_y;
    for _ in 0..60 {
        h.system.tick_world(&mut h.world, &h.resolver, dt, &h.env);
        match pos_y(&h.world) {
            Some(y) => peak = peak.max(y),
            None => break, // 已触地引爆
        }
    }
    assert!(peak > start_y, "重力弹道应出现上升段（顶点 {peak} 应高于出手 {start_y}）");
}

/// 回归复现：背包出现**第 3 叠弹药**后按 R 应一次换弹到位（直抽背包、无中间弹池残留）。
#[test]
fn reload_works_with_third_ammo_stack() {
    let mut h = CombatHarness::new();
    let pid = spawn_player(&mut h.world, Vec3::default(), 0);
    // 开局两叠各 64（已满）→ 再补 30 会另占一格，形成第 3 叠（合计 158 发）。
    {
        let e = h.world.get_entity_mut(pid).unwrap();
        let bp = e.get_component_mut::<Backpack>().unwrap();
        assert!(
            bp.push(crate::items::LootItem::with_count(
                "步枪弹药",
                crate::map::PickupKind::Ammo { amount: 30 },
                30,
            )),
            "第 3 叠弹药应能入格"
        );
        let stacks = bp
            .slots
            .iter()
            .flatten()
            .filter(|i| matches!(i.kind, crate::map::PickupKind::Ammo { .. }))
            .count();
        assert_eq!(stacks, 3, "背包内应有 3 叠弹药");
        assert_eq!(bp.ammo_total(), 158, "备用弹药合计应为 158");
    }
    // 打空当前弹夹。
    {
        let e = h.world.get_entity_mut(pid).unwrap();
        let cb = e.get_component_mut::<Combatant>().unwrap();
        cb.active_mut().ammo = 0;
    }
    h.system.apply_input(
        &mut h.world,
        &h.resolver,
        pid,
        0.016,
        &h.env,
        CombatIntent { reload: true, ..CombatIntent::default() },
    );
    // 一次按 R 即应起计时（能换弹的判据是背包有余弹）。
    let cb = h.world.get_entity(pid).unwrap().get_component::<Combatant>().unwrap();
    assert!(cb.reload_timer > 0.0, "有第 3 叠弹药时应能换弹（reload_timer={}）", cb.reload_timer);
    let max_ammo = cb.active().max_ammo;
    // 走完换弹计时（世界级 tick_reloads 直接抽背包补满弹夹）。
    let dt = 1.0 / 60.0;
    for _ in 0..(RELOAD_TIME_SECS / dt) as usize + 5 {
        h.system.tick_world(&mut h.world, &h.resolver, dt, &h.env);
    }
    let e = h.world.get_entity(pid).unwrap();
    let cb = e.get_component::<Combatant>().unwrap();
    assert_eq!(cb.active().ammo, max_ammo, "换弹结束后弹夹应一次补满");
    assert_eq!(cb.reload_timer, 0.0, "换弹计时应归零");
    // 弹夹补满 30 发来自背包：158 - 30 = 128（无中间池残留、无需再按一次 R）。
    let reserve = e.get_component::<Backpack>().unwrap().ammo_total();
    assert_eq!(reserve, 128, "换弹应恰好从背包抽走补齐弹夹的 30 发（剩余 {reserve}）");
}

// ——— 选装清单进图（`apply_loadout`）：完全替换语义 ———

/// 空清单 → 空背包：旧的 `Backpack::starting()` 固定配发必须被完全清掉。
#[test]
fn apply_loadout_empty_clears_backpack() {
    let mut h = CombatHarness::new();
    let pid = spawn_player(&mut h.world, Vec3::default(), 0);
    let placed = apply_loadout(&mut h.world, pid, &[]);
    assert_eq!(placed, 0, "空清单不应入格任何物品");
    let e = h.world.get_entity(pid).unwrap();
    let bp = e.get_component::<Backpack>().unwrap();
    assert!(bp.slots.iter().all(|s| s.is_none()), "空清单应得空背包，旧配发不得残留");
    assert_eq!(bp.ammo_total(), 0, "备用弹药应随旧配发一并清空");
}

/// 清单里的名字经**权威映射表**入格，并按类别落到不同格位。
#[test]
fn apply_loadout_places_known_items() {
    let mut h = CombatHarness::new();
    let pid = spawn_player(&mut h.world, Vec3::default(), 0);
    let carried = vec!["大型医疗包".to_string(), "破片手雷".to_string()];
    assert_eq!(apply_loadout(&mut h.world, pid, &carried), 2);
    let e = h.world.get_entity(pid).unwrap();
    let bp = e.get_component::<Backpack>().unwrap();
    assert_eq!(bp.count_category(ItemCategory::Consumable), 1, "大型医疗包应入恢复类");
    assert_eq!(bp.count_category(ItemCategory::Tactical), 1, "破片手雷应入战术类");
}

/// 未登记的名字忽略（客户端不可信），但不阻断其余合法物资入格。
#[test]
fn apply_loadout_ignores_unknown_names() {
    let mut h = CombatHarness::new();
    let pid = spawn_player(&mut h.world, Vec3::default(), 0);
    let carried = vec!["核弹".to_string(), "医疗包".to_string(), "".to_string()];
    assert_eq!(apply_loadout(&mut h.world, pid, &carried), 1, "非法名应被忽略");
    let e = h.world.get_entity(pid).unwrap();
    let bp = e.get_component::<Backpack>().unwrap();
    assert_eq!(bp.count_category(ItemCategory::Consumable), 1);
}

/// 超出 4×3 格位的溢出部分丢弃：14 个**不同文案**的物资只能占 12 格。
#[test]
fn apply_loadout_drops_overflow() {
    let mut h = CombatHarness::new();
    let pid = spawn_player(&mut h.world, Vec3::default(), 0);
    let carried: Vec<String> = [
        "医疗包", "大型医疗包", "急救包", "护甲片", "护甲板", "重型护甲板", "额外弹药",
        "步枪弹药", "烈焰手雷", "冰霜手雷", "雷电手雷", "毒素手雷", "破片手雷", "水压手雷",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let placed = apply_loadout(&mut h.world, pid, &carried);
    assert_eq!(placed, 12, "溢出应丢弃至恰好占满 12 格（实际入格 {placed}）");
    let e = h.world.get_entity(pid).unwrap();
    let bp = e.get_component::<Backpack>().unwrap();
    assert_eq!(bp.slots.iter().filter(|s| s.is_some()).count(), 12);
}

