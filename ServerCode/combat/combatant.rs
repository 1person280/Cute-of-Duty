//! 权威战斗运行时状态（挂到玩家实体上的组件）
//!
//! 承担弹药、换弹、射击/技能冷却的按帧衰减（`on_tick`），以及干员索引与朝向缓存。
//! 具体结算（开火/技能/投射物）由 `CombatSystem` 调度，这里只做“状态在哪里、何时衰减”。

use crate::entity::{Component, Entity};
use std::any::Any;

/// 玩家战斗运行时
///
/// 为何这类状态要走实体组件 ECS：切换干员（改弹道/技能）只需替换组件中的干员索引，
/// 多玩家各持一份，天然隔离；也便于 `broadcaster` 按实体读弹夹/冷却给 HUD。
pub struct Combatant {
    /// 干员名册索引（`roster()[i]`）——决定弹道数值、技能与元素亲和
    pub operator_idx: usize,
    /// 当前武器弹夹剩余弹药
    pub ammo: i32,
    /// 弹夹容量
    pub max_ammo: i32,
    /// 备弹池（换弹时从池中取弹补满）
    pub ammo_pool: i32,
    /// 射出间隔（秒）→ 自动射速节拍
    pub fire_interval: f32,
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
    /// 新建战斗状态：`ammo_capacity` 为弹夹容量，`pool` 为备弹池初始值。
    pub fn new(operator_idx: usize, ammo_capacity: i32, pool: i32) -> Self {
        Self {
            operator_idx,
            ammo: ammo_capacity,
            max_ammo: ammo_capacity,
            ammo_pool: pool,
            fire_interval: 0.14,
            shoot_cooldown: 0.0,
            reload_timer: 0.0,
            skill_q_cd: 0.0,
            skill_e_cd: 0.0,
            last_yaw: 0.0,
            last_pitch: 0.0,
        }
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
        // 换弹计时耗尽 → 从备弹池补满
        if self.reload_timer > 0.0 {
            self.reload_timer -= delta_time;
            if self.reload_timer <= 0.0 {
                self.reload_timer = 0.0;
                let needed = self.max_ammo - self.ammo;
                let take = needed.min(self.ammo_pool);
                self.ammo += take;
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