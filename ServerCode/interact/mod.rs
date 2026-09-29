//! 交互域（服务端权威）—— 场上拾取物与功能站点的语义载体与结算。
//!
//! 设计动机（Why）："按 F 能拿到什么"（弹药数量、回血值、切到哪个干员、箱子给什么）
//! 全部属于**应该算什么**的服务端职责，客户端只上报意图（对哪个目标、选了哪一项）。
//! 因此本模块同时持有：
//! - [`Interactable`] 组件：挂在 Loot/Station 实体上，声明"这是什么、叫什么"；
//! - 线格式类型 [`InteractInfo`] / [`InteractChoice`]：由 `net::protocol` 直接复用，
//!   保证"服务端实体语义"与"快照/上行契约"是同一份定义，不会漂移；
//! - [`settle`]：权威结算一次交互（距离校验 + 效果发放 + 是否消耗该实体）。
//!
//! 依赖方向：`interact → map`（拾取/站点语义）与 `interact → combat`（干员/弹药），
//! 二者均不反向依赖本模块，无循环。

use std::any::Any;

use crate::combat;
use crate::damage::Vec3;
use crate::entity::{Component, Entity, EntityId, World};
use crate::items::{Container, LootItem};
use crate::map::{MapLayout, PickupKind, StationKind};

// 跨域线格式类型（交互半径、交互语义、快照下行信息、补给选择、上行选择）由契约 crate
// 提供；经 `pub use` 保持 `crate::interact::*` 公开路径不变，其余模块无需改引用。
pub use cute_of_duty_contract::interact::{
    InteractChoice, InteractInfo, InteractKind, SupplyKind, INTERACT_RANGE,
};

/// 玩家护甲显示上限（表现层 `MAX_ARMOR` 同源）。服务端实体无护甲上限字段，
/// 在此统一钳制，避免补给叠加出超过 UI 的数值。
const ARMOR_CAP: f32 = 100.0;

/// 补给台「领取弹药」一次补入备弹池的发数（3 个弹夹量级）。
const SUPPLY_AMMO: i32 = 90;
/// 补给台「领取医疗」一次回血值。
const SUPPLY_HEALTH: f32 = 50.0;
/// 补给台「领取护甲」一次加甲值。
const SUPPLY_ARMOR: f32 = 50.0;

/// 可交互标记组件：附在 Loot / Station 实体上，承载展示名与语义。
#[derive(Debug, Clone)]
pub struct Interactable {
    pub label: String,
    pub kind: InteractKind,
}

impl Interactable {
    /// 由展示名与语义构造（地图数据 → 组件）。
    pub fn new(label: impl Into<String>, kind: InteractKind) -> Self {
        Self {
            label: label.into(),
            kind,
        }
    }

    /// 生成随快照下行的 [`InteractInfo`]。
    pub fn info(&self) -> InteractInfo {
        InteractInfo {
            label: self.label.clone(),
            kind: self.kind,
        }
    }
}

impl Component for Interactable {
    fn name(&self) -> &'static str {
        "Interactable"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// 把地图数据里的拾取物/功能站点落成权威实体，并挂上 [`Interactable`] 语义。
///
/// 设计动机（Why）：地图是**数据**（`map::lawn`），实体是**运行态**。二者在服务端启动时
/// 于此对接：数据改布局，实体随之变化，客户端仅通过快照感知——保持"地图定义单一来源"。
/// 返回 `(拾取物数, 站点数)` 供启动日志。
pub fn spawn_from_layout(world: &mut World, layout: &MapLayout) -> (usize, usize) {
    for p in &layout.pickups {
        let mut e = Entity::new_loot(0, Vec3::new(p.pos[0], p.pos[1], p.pos[2]));
        e.add_component(Box::new(Interactable::new(
            p.label,
            InteractKind::Pickup(p.kind),
        )));
        world.spawn(e);
    }
    for (seed, s) in layout.stations.iter().enumerate() {
        let mut e = Entity::new_station(0, Vec3::new(s.pos[0], s.pos[1], s.pos[2]));
        e.add_component(Box::new(Interactable::new(
            s.label,
            InteractKind::Station(s.kind),
        )));
        // 物资箱：挂上权威 4×3 战利品格位（以站点序号轮转固定掉落池，确定性可复现）。
        // 逐个"开箱取物"由此组件承载——不再开箱即打包，而是逐格转移到玩家背包。
        if s.kind == StationKind::SupplyCrate {
            e.add_component(Box::new(Container::rolled(seed)));
        }
        world.spawn(e);
    }
    (layout.pickups.len(), layout.stations.len())
}

/// 权威结算一次交互。返回 `(回执文本, 是否应销毁目标实体)`。
///
/// 距离校验在服务端做（客户端只上报意图）：即便客户端伪造目标 ID，也必须站到
/// `INTERACT_RANGE` 内才会生效——这是"应该算什么"归服务端的直接落点。
pub fn settle(
    world: &mut World,
    player: EntityId,
    target: EntityId,
    choice: InteractChoice,
) -> (String, bool) {
    let Some(t) = world.get_entity(target) else {
        return ("交互目标不存在".to_string(), false);
    };
    let Some(info) = t.get_component::<Interactable>().cloned() else {
        return ("该目标不可交互".to_string(), false);
    };
    let Some(p) = world.get_entity(player) else {
        return ("玩家实体不存在".to_string(), false);
    };

    // 平面距离校验（忽略 Y，与撤离判定口径一致）
    let dx = t.position.x - p.position.x;
    let dz = t.position.z - p.position.z;
    if (dx * dx + dz * dz).sqrt() > INTERACT_RANGE {
        return ("距离太远，无法交互".to_string(), false);
    }

    match (info.kind, choice) {
        (InteractKind::Pickup(kind), InteractChoice::Take) => {
            apply_pickup(world, player, kind, &info.label)
        }
        (InteractKind::Station(StationKind::SupplyTable), InteractChoice::Supply { kind }) => {
            let msg = grant_supply(world, player, kind);
            (msg, false)
        }
        (InteractKind::Station(StationKind::SupplyCrate), InteractChoice::OpenCrate) => {
            // 物资箱：仅"打开面板"，内容物经逐格转移协议取走，箱子本身不消耗、可反复开。
            ("物资箱已打开".to_string(), false)
        }
        _ => ("无效的交互操作".to_string(), false),
    }
}

/// 结算地面拾取物效果，返回 `(回执文本, 是否应消耗该拾取物)`。
///
/// 设计动机（Why）：武器拾取只换枪不换干员（武器与干员解耦，见 `combat` 模块头注释）；
/// 医疗包/护甲片/手雷/弹药一律入 **4×3 背包格位**（弹药为可堆叠背包物品，由玩家按 3/4 主动使用）。
/// 背包满时**拒绝拾取且不消耗**地面物品（返回 `false`），避免物品凭空蒸发。
fn apply_pickup(
    world: &mut World,
    player: EntityId,
    kind: PickupKind,
    label: &str,
) -> (String, bool) {
    match kind {
        PickupKind::Ammo { amount } => {
            // 备用子弹按**可堆叠背包物品**入格（每叠上限 64）；背包满则拒收、不消耗地面物品。
            let item = LootItem::with_count(label, kind, amount.max(1) as u32);
            if combat::push_item(world, player, item) {
                (format!("拾取 {label}（备用子弹 +{amount}）"), true)
            } else {
                ("背包已满".to_string(), false)
            }
        }
        PickupKind::Health { .. } | PickupKind::Armor { .. } | PickupKind::Grenade { .. } => {
            // 占格物品：入背包（满则拒收、不消耗地面物品）。
            if combat::push_item(world, player, LootItem::new(label, kind)) {
                (format!("拾取 {label}"), true)
            } else {
                ("背包已满".to_string(), false)
            }
        }
        PickupKind::Weapon { element } => {
            // 武器拾取 → 装进当前手持武器槽（覆盖并补满弹药），不动干员（技能组不变）。
            combat::equip_weapon(world, player, element);
            (format!("拾取 {label}（已装备到当前武器槽）"), true)
        }
    }
}

/// 发放一项补给（补给台/物资箱共用）。
fn grant_supply(world: &mut World, player: EntityId, kind: SupplyKind) -> String {
    match kind {
        SupplyKind::Ammo => {
            // 补给弹药同样按可堆叠背包物品发放（备弹的权威宿主是背包格位）。
            let item = LootItem::with_count(
                "步枪弹药",
                PickupKind::Ammo { amount: SUPPLY_AMMO },
                SUPPLY_AMMO.max(1) as u32,
            );
            if combat::push_item(world, player, item) {
                format!("补给弹药 +{SUPPLY_AMMO}（入背包）")
            } else {
                "背包已满，无法领取弹药".to_string()
            }
        }
        SupplyKind::Health => {
            heal(world, player, SUPPLY_HEALTH);
            format!("补给医疗 +{SUPPLY_HEALTH:.0}")
        }
        SupplyKind::Armor => {
            add_armor(world, player, SUPPLY_ARMOR);
            format!("补给护甲 +{SUPPLY_ARMOR:.0}")
        }
    }
}

/// 回血（钳制到 `max_hp`）。
fn heal(world: &mut World, player: EntityId, amount: f32) {
    if let Some(e) = world.get_entity_mut(player) {
        e.hp = (e.hp + amount).min(e.max_hp);
    }
}

/// 加甲（钳制到 [`ARMOR_CAP`]）。
fn add_armor(world: &mut World, player: EntityId, amount: f32) {
    if let Some(e) = world.get_entity_mut(player) {
        e.armor = (e.armor + amount).min(ARMOR_CAP);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::Combatant;
    use crate::damage::Vec3;
    use crate::element::ElementType;

    /// 站得够近拾取弹药：以**可堆叠背包物品**入格（备弹宿主为背包），返回"应消耗"。
    #[test]
    fn take_ammo_within_range_consumes() {
        let mut world = World::new();
        let pid = combat::spawn_player(&mut world, Vec3::default(), 0);
        let mut loot = Entity::new_loot(0, Vec3::new(1.0, 0.0, 0.0));
        loot.add_component(Box::new(Interactable::new(
            "步枪弹药",
            InteractKind::Pickup(PickupKind::Ammo { amount: 30 }),
        )));
        let lid = world.spawn(loot);

        // 统计背包内弹药总发数（弹药的权威宿主是背包格位里的可堆叠物品，无中间弹池）。
        let ammo_of = |w: &World| -> i32 {
            w.get_entity(pid)
                .and_then(|e| e.get_component::<crate::items::Backpack>())
                .map(|bp| bp.ammo_total())
                .unwrap_or(0)
        };
        let before = ammo_of(&world);
        let (msg, consumed) = settle(&mut world, pid, lid, InteractChoice::Take);
        let after = ammo_of(&world);

        assert!(consumed, "地面拾取物应被消耗");
        assert_eq!(after, before + 30, "背包弹药应 +30");
        assert!(msg.contains("备用子弹"));
    }

    /// 距目标过远（超出 INTERACT_RANGE）时拒绝交互，且不消耗实体。
    #[test]
    fn interact_out_of_range_refused() {
        let mut world = World::new();
        let pid = combat::spawn_player(&mut world, Vec3::default(), 0);
        let mut loot = Entity::new_loot(0, Vec3::new(20.0, 0.0, 0.0));
        loot.add_component(Box::new(Interactable::new(
            "医疗包",
            InteractKind::Pickup(PickupKind::Health { amount: 25.0 }),
        )));
        let lid = world.spawn(loot);

        let (msg, consumed) = settle(&mut world, pid, lid, InteractChoice::Take);
        assert!(!consumed);
        assert!(msg.contains("距离太远"));
    }

    /// 补给台反复补给：一次性不消耗，且血量钳制到上限。
    #[test]
    fn supply_table_reusable_and_clamped() {
        let mut world = World::new();
        let pid = combat::spawn_player(&mut world, Vec3::default(), 0);
        // 先扣血，便于观察回血
        world.get_entity_mut(pid).unwrap().hp = 10.0;
        let mut st = Entity::new_station(0, Vec3::new(1.0, 0.0, 0.0));
        st.add_component(Box::new(Interactable::new(
            "补给台",
            InteractKind::Station(StationKind::SupplyTable),
        )));
        let sid = world.spawn(st);

        let (_, consumed) = settle(
            &mut world,
            pid,
            sid,
            InteractChoice::Supply { kind: SupplyKind::Health },
        );
        assert!(!consumed, "补给台不应被消耗");
        let hp = world.get_entity(pid).unwrap().hp;
        assert!((hp - 60.0).abs() < 1e-3, "回血应为 10+50=60，实际 {hp}");

        // 再补两次：钳制到 100
        settle(&mut world, pid, sid, InteractChoice::Supply { kind: SupplyKind::Health });
        settle(&mut world, pid, sid, InteractChoice::Supply { kind: SupplyKind::Health });
        assert!((world.get_entity(pid).unwrap().hp - 100.0).abs() < 1e-3);
    }

    /// 武器拾取只换当前手持槽的武器元素，不改变干员（武器与干员解耦）。
    #[test]
    fn weapon_pickup_equips_slot_not_operator() {
        let mut world = World::new();
        let pid = combat::spawn_player(&mut world, Vec3::default(), 0); // 焰狐（火系）
        let mut loot = Entity::new_loot(0, Vec3::new(1.0, 0.0, 0.0));
        loot.add_component(Box::new(Interactable::new(
            "毒液步枪",
            InteractKind::Pickup(PickupKind::Weapon { element: ElementType::Poison }),
        )));
        let lid = world.spawn(loot);

        let (_, consumed) = settle(&mut world, pid, lid, InteractChoice::Take);
        assert!(consumed);

        let cb = world.get_entity(pid).unwrap().get_component::<Combatant>().unwrap();
        assert_eq!(cb.operator_idx, 0, "拾取武器不应切换干员");
        assert_eq!(cb.active_slot, 0, "武器应装进当前手持槽");
        assert_eq!(cb.element(), ElementType::Poison, "手持武器元素应被替换");
        assert_eq!(cb.weapons[1].element, ElementType::Ice, "另一槽位武器不应受影响");
    }

    /// 医疗包/护甲片/手雷拾取进入 4×3 背包格位（而非拾取即结算）。
    #[test]
    fn consumable_pickups_accumulate() {
        use crate::items::ItemCategory;

        let mut world = World::new();
        let pid = combat::spawn_player(&mut world, Vec3::default(), 0);
        let count = |w: &World, cat| {
            w.get_entity(pid).unwrap().get_component::<crate::items::Backpack>().unwrap().count_category(cat)
        };
        let before_cons = count(&world, ItemCategory::Consumable);
        let before_tac = count(&world, ItemCategory::Tactical);

        let mut med = Entity::new_loot(0, Vec3::new(1.0, 0.0, 0.0));
        med.add_component(Box::new(Interactable::new(
            "医疗包",
            InteractKind::Pickup(PickupKind::Health { amount: 25.0 }),
        )));
        let mid = world.spawn(med);

        let mut gre = Entity::new_loot(0, Vec3::new(1.0, 0.0, 0.0));
        gre.add_component(Box::new(Interactable::new(
            "冰霜手雷",
            InteractKind::Pickup(PickupKind::Grenade { element: ElementType::Ice }),
        )));
        let gid = world.spawn(gre);

        assert!(settle(&mut world, pid, mid, InteractChoice::Take).1, "拾取应成功并消耗地面物品");
        assert!(settle(&mut world, pid, gid, InteractChoice::Take).1);

        assert_eq!(count(&world, ItemCategory::Consumable), before_cons + 1, "医疗包应入背包恢复类");
        assert_eq!(count(&world, ItemCategory::Tactical), before_tac + 1, "手雷应入背包战术类");
    }
}
