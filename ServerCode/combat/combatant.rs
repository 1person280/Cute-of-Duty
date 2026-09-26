//! 权威战斗运行时状态（挂到玩家实体上的组件）
//!
//! 承担双武器槽弹药、换弹、射击/技能冷却的按帧衰减（`on_tick`），以及干员索引与朝向缓存。
//! 具体结算（开火/技能/投射物）由 `CombatSystem` 调度，这里只做“状态在哪里、何时衰减”。
//!
//! 双武器设计（对齐 legacy `Inventory.weapons`）：玩家默认携带两把独立步枪（火/冰），
//! `active_slot` 决定当前手持；`ammo_pool` 为两把武器共享的备弹池。武器与干员彻底解耦——
//! 射击元素取当前武器槽（`weapons[active_slot].element`），技能元素仍取干员（`roster()`）。

use crate::element::ElementType;
use crate::entity::{Component, Entity};
use crate::operator::rifle_profile;
use std::any::Any;

/// 武器槽数量（对齐 legacy 双主武器：1/2 两把）
pub const WEAPON_SLOTS: usize = 2;
/// 共享备弹池初始值（对齐 legacy `Inventory.ammo_pool = 150`）
pub const DEFAULT_AMMO_POOL: i32 = 150;

/// 手雷爆炸基础伤害 / 半径（服务端权威数值）。
pub const GRENADE_DAMAGE: f32 = 70.0;
pub const GRENADE_RADIUS: f32 = 5.0;

/// 单个武器槽：元素身份 + 弹夹状态。
///
/// 为何把元素放在槽里：武器与干员解耦后，射击元素由手持武器决定，
/// 换枪即换弹道；干员只影响技能（Q/E）。
#[derive(Clone, Copy, Debug)]
pub struct WeaponSlot {
    /// 武器元素（决定弹道数值与武器名，取自 `rifle_profile`）
    pub element: ElementType,
    /// 当前弹夹剩余
    pub ammo: i32,
    /// 弹夹容量
    pub max_ammo: i32,
}

impl WeaponSlot {
    /// 按元素生成满弹武器槽（容量取该元素步枪档案）。
    pub fn from_element(element: ElementType) -> Self {
        let profile = rifle_profile(element);
        Self { element, ammo: profile.max_ammo, max_ammo: profile.max_ammo }
    }
}

/// 玩家战斗运行时
///
/// 为何这类状态要走实体组件 ECS：切换干员（改技能）只需替换组件中的干员索引，
/// 多玩家各持一份，天然隔离；也便于 `broadcaster` 按实体读双武器槽/冷却给 HUD。
pub struct Combatant {
    /// 干员名册索引（`roster()[i]`）——决定技能与元素亲和
    pub operator_idx: usize,
    /// 携带的两把武器（1/2 对应槽 0/1）
    pub weapons: [WeaponSlot; WEAPON_SLOTS],
    /// 当前手持槽索引（[`Self::weapons`] 下标）
    pub active_slot: usize,
    /// 共享备弹池（换弹时从池中取弹补满当前槽）
    pub ammo_pool: i32,
    /// 距下次可开火的剩余冷却
    pub shoot_cooldown: f32,
    /// 换弹剩余时间（>0 表示正在换弹）
    pub reload_timer: f32,
    /// Q 技能剩余冷却
    pub skill_q_cd: f32,
    /// E 技能剩余冷却
    pub skill_e_cd: f32,
    /// 缓存本帧偏航（弧度）——技能方向 / 射线用
    pub last_yaw: f32,
    /// 缓存本帧俯仰（弧度）
    pub last_pitch: f32,
}

impl Combatant {
    /// 新建战斗状态：默认携带火/冰两把制式步枪、共享备弹池、手持槽 0。
    pub fn new(operator_idx: usize) -> Self {
        Self {
            operator_idx,
            weapons: [
                WeaponSlot::from_element(ElementType::Fire),
                WeaponSlot::from_element(ElementType::Ice),
            ],
            active_slot: 0,
            ammo_pool: DEFAULT_AMMO_POOL,
            shoot_cooldown: 0.0,
            reload_timer: 0.0,
            skill_q_cd: 0.0,
            skill_e_cd: 0.0,
            last_yaw: 0.0,
            last_pitch: 0.0,
        }
    }

    /// 当前手持武器槽（只读）
    pub fn active(&self) -> &WeaponSlot {
        &self.weapons[self.active_slot.min(WEAPON_SLOTS - 1)]
    }

    /// 当前手持武器槽（可变）
    pub fn active_mut(&mut self) -> &mut WeaponSlot {
        let idx = self.active_slot.min(WEAPON_SLOTS - 1);
        &mut self.weapons[idx]
    }

    /// 当前手持武器的射击元素（射击结算取此元素，而非干员元素）
    pub fn element(&self) -> ElementType {
        self.active().element
    }

    /// 当前手持武器的射速节拍（秒/发）
    pub fn fire_interval(&self) -> f32 {
        rifle_profile(self.active().element).fire_interval
    }

    /// 把一把新武器装进当前手持槽（拾取武器时调用）：换元素并按该元素步枪档案补满弹夹。
    ///
    /// 与干员解耦：只改 `weapons[active_slot]`，不动 `operator_idx`（技能组不变）。
    pub fn equip_active(&mut self, element: ElementType) {
        let idx = self.active_slot.min(WEAPON_SLOTS - 1);
        self.weapons[idx] = WeaponSlot::from_element(element);
        self.reload_timer = 0.0;
    }

    /// 切换到指定武器槽：越界或同槽忽略；换枪打断换弹并给出短射速节流。
    pub fn switch_slot(&mut self, slot: usize) {
        if slot >= WEAPON_SLOTS || slot == self.active_slot {
            return;
        }
        self.active_slot = slot;
        self.reload_timer = 0.0;
        self.shoot_cooldown = self.shoot_cooldown.max(0.25);
    }

    /// 向体内刷新已知朝向（服务端按输入写回，供本帧射线/技能方向用）。
    pub fn update_aim(&mut self, yaw: f32, pitch: f32) {
        self.last_yaw = yaw;
        self.last_pitch = pitch;
    }

    /// Q/E 是否就绪（冷却已到 0）。
    pub fn skill_ready(&self, is_q: bool) -> bool {
        if is_q { self.skill_q_cd <= 0.0 } else { self.skill_e_cd <= 0.0 }
    }

    /// 触发一次技能冷却（写入指定槽位）。
    pub fn trigger_skill_cd(&mut self, is_q: bool, cooldown_secs: f32) {
        if is_q { self.skill_q_cd = cooldown_secs } else { self.skill_e_cd = cooldown_secs }
    }
}

impl Component for Combatant {
    fn on_tick(&mut self, _owner: &mut Entity, delta_time: f32) {
        if self.shoot_cooldown > 0.0 {
            self.shoot_cooldown = (self.shoot_cooldown - delta_time).max(0.0);
        }
        // 换弹计时耗尽 → 从共享备弹池补满当前手持槽
        if self.reload_timer > 0.0 {
            self.reload_timer -= delta_time;
            if self.reload_timer <= 0.0 {
                self.reload_timer = 0.0;
                let idx = self.active_slot.min(WEAPON_SLOTS - 1);
                let needed = self.weapons[idx].max_ammo - self.weapons[idx].ammo;
                let take = needed.min(self.ammo_pool);
                self.weapons[idx].ammo += take;
                self.ammo_pool -= take;
            }
        }
        if self.skill_q_cd > 0.0 {
            self.skill_q_cd = (self.skill_q_cd - delta_time).max(0.0);
        }
        if self.skill_e_cd > 0.0 {
            self.skill_e_cd = (self.skill_e_cd - delta_time).max(0.0);
        }
    }

    fn name(&self) -> &'static str {
        "Combatant"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
