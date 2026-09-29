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
use crate::operator::{roster, SkillKind};

/// 护甲显示上限（与 `interact::ARMOR_CAP`、表现层同源）：速用护甲片钳制用。
const ARMOR_CAP: f32 = 100.0;

// 公开 grenade 子模块：客户端轨迹预览需复用其弹道常数（初速/重力/出手点），
// 保证预览与实际结算**同源**、不各自硬编码副本（见 `combat::grenade` 顶部常量）。
pub mod grenade;
mod shooter;
mod skill;
mod zone;

pub use combatant::Combatant;
pub use grenade::HeldGrenade;
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
    /// 使用背包第 `slot` 格物品（边沿量）：医疗包回血、护甲片加甲、手雷进入**持握**。
    pub use_slot: Option<u8>,
    /// 取消持握中的手雷（边沿量）：原样放回背包、不消耗。
    pub grenade_cancel: bool,
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
        // 持雷（服务端权威）——手雷"先瞄准后释放"：持握时左键=投掷、Esc=取消，并**抑制
        // 枪械射击**（否则投出后会紧接着用枪开火）；其余意图（换弹/技能/切枪）照常。
        let holding = world
            .get_entity(eid)
            .and_then(|e| e.get_component::<HeldGrenade>())
            .map(|h| h.is_holding())
            .unwrap_or(false);
        if holding {
            if intent.grenade_cancel {
                grenade::cancel_held_grenade(world, eid);
            } else if intent.shoot {
                grenade::throw_held_grenade(world, eid);
            }
        } else if intent.shoot {
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
            use_item_at(world, eid, slot as usize);
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
        // 换弹推进：计时耗尽后直接从背包抽弹补满弹夹（无中间弹池）。
        shooter::tick_reloads(world, dt);
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
    // 持雷态宿主：初始未持雷（手雷"先瞄准后释放"的中间状态，见 `grenade`）。
    entity.add_component(Box::new(HeldGrenade::default()));
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
/// 恢复类（医疗包回血 / 护甲片加甲）按物品自身数值钳制到上限；战术类（手雷）**进入持握**
/// ——"先瞄准后释放"的中间态，取出既不消耗也不生成投射物，待左键释放才投出、或取消放回
/// （见 [`grenade::HeldGrenade`]）。空位或不可速用物（弹药/武器）静默忽略。
pub fn use_item_at(world: &mut World, eid: EntityId, index: usize) {
    let Some(item) = ({
        let Some(entity) = world.get_entity_mut(eid) else { return };
        let Some(bp) = entity.get_component_mut::<Backpack>() else { return };
        bp.take_one_at(index)
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
        PickupKind::Grenade { .. } => {
            // 手雷取出即进入持握；已持雷则本次使用作废，把刚取出的这件**原样放回**背包
            // （持握组件缺失同理——不能让它凭空消失）。
            let taken = match world.get_entity_mut(eid).and_then(|e| e.get_component_mut::<HeldGrenade>()) {
                Some(h) if !h.is_holding() => {
                    h.item = Some(item.clone());
                    true
                }
                _ => false,
            };
            if !taken {
                let _ = push_item(world, eid, item);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests;