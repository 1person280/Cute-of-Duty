//! 持握手雷（服务端权威）：手雷"先瞄准后释放"的中间状态与两个结算出口。
//!
//! 设计动机（Why）：手雷不再是"按下即抛出"的即时技能，而是先进入**持握**姿态——
//! 取出时既不消耗也不生成投射物；持握期间服务端把该玩家视为越肩瞄准（移速压到
//! [`crate::motion::AIM_MULT`]）；直到左键释放才真正消耗一件并生成投射物，或取消把
//! 整件原样放回背包。持握与否、投掷方向、消耗数量都属"应该算什么"，故状态落在服务端
//! 组件上，客户端只按快照表现。
//!
//! 移植来源：legacy `src/demo/inventory/held_grenade.rs`（`HeldGrenade` +
//! `grenade_throw_system`），剥掉全部 Bevy 表现，只留权威半边。

use std::any::Any;

use crate::damage::Vec3;
use crate::entity::{Component, EntityId, World};
use crate::items::LootItem;

/// 玩家正持握的手雷（`None` = 未持雷）。
///
/// 存**整件 [`LootItem`]** 而非仅元素：取消时需把原物原样放回背包（保留展示名与堆叠量），
/// 释放时元素信息用于生成对应属性的投射物。
#[derive(Default)]
pub struct HeldGrenade {
    pub item: Option<LootItem>,
}

impl HeldGrenade {
    /// 是否正在持雷（服务端据此分派"左键=投掷 / Esc=取消"并抑制枪械射击）。
    pub fn is_holding(&self) -> bool {
        self.item.is_some()
    }
}

impl Component for HeldGrenade {
    fn name(&self) -> &'static str {
        "HeldGrenade"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// 释放持握中的手雷：沿权威朝向生成投射物，并真正消耗这一件。
///
/// 返回 `true` 表示确有一次投掷发生。未持雷时为无副作用的 no-op。
pub fn throw_held_grenade(world: &mut World, eid: EntityId) -> bool {
    let Some(item) = take_held(world, eid) else { return false };
    let crate::map::PickupKind::Grenade { element } = item.kind else {
        // 理论不可达（只有手雷会进入持握）：异常物品放回背包，不凭空消失。
        super::push_item(world, eid, item);
        return false;
    };
    let Some((origin, yaw)) = aim_of(world, eid) else {
        super::push_item(world, eid, item);
        return false;
    };
    super::grenade::spawn_projectile(
        world,
        eid,
        origin,
        yaw,
        element,
        super::combatant::GRENADE_DAMAGE,
        super::combatant::GRENADE_RADIUS,
        crate::operator::SkillEffect::NONE,
    );
    true
}

/// 取消持握：把手雷原样放回背包（不消耗、不生成投射物）。
pub fn cancel_held_grenade(world: &mut World, eid: EntityId) -> bool {
    let Some(item) = take_held(world, eid) else { return false };
    super::push_item(world, eid, item);
    true
}

/// 取出持握件（结算入口共用）：未持雷返回 `None`。
fn take_held(world: &mut World, eid: EntityId) -> Option<LootItem> {
    world.get_entity_mut(eid)?.get_component_mut::<HeldGrenade>()?.item.take()
}

/// 读取投掷所需的权威位姿：站位 + 本帧朝向（服务端写回，客户端无法伪造）。
fn aim_of(world: &World, eid: EntityId) -> Option<(Vec3, f32)> {
    let entity = world.get_entity(eid)?;
    let cb = entity.get_component::<super::Combatant>()?;
    Some((entity.position, cb.last_yaw))
}
