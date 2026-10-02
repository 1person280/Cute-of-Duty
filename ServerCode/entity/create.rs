//! `Entity` 的构造与生命周期：新实体工厂、组件挂载/卸载、伤害收件箱与逐 Tick 推进。
//!
//! 设计动机（Why）：把 `Entity` 的固有实现从模块根拆出，模块根只保留类型定义与 `World`
//! 管理；构造 / 组件 / 推进三条职责仍同处一个实现块，避免跨文件反复跳读。

use std::any::TypeId;
use std::collections::HashMap;

use crate::damage::Vec3;
use crate::element::EntityElementState;

use super::{Component, Entity, EntityId, EntityType};

impl Entity {
    /// 创建新实体
    pub fn new(id: u64, position: Vec3) -> Self {
        Self {
            id: EntityId::new(id),
            position,
            rotation: Vec3::default(),
            hp: 100.0,
            max_hp: 100.0,
            armor: 0.0,
            move_speed: 4.0,
            base_move_speed: 4.0,
            element_state: EntityElementState::Normal,
            components: HashMap::new(),
            pending_removals: Vec::new(),
            inbox: Vec::new(),
            entity_type: EntityType::Player,
            is_alive: true,
            vertical_velocity: 0.0,
            grounded: true,
        }
    }

    /// 创建玩家实体
    pub fn new_player(id: u64, position: Vec3) -> Self {
        let mut entity = Self::new(id, position);
        entity.entity_type = EntityType::Player;
        entity.max_hp = 100.0;
        entity.hp = 100.0;
        entity
    }

    /// 创建AI实体
    pub fn new_ai(id: u64, position: Vec3) -> Self {
        let mut entity = Self::new(id, position);
        entity.entity_type = EntityType::AI;
        entity.max_hp = 80.0;
        entity.hp = 80.0;
        entity
    }

    /// 创建手雷实体
    pub fn new_grenade(id: u64, position: Vec3) -> Self {
        let mut entity = Self::new(id, position);
        entity.entity_type = EntityType::Grenade;
        entity.is_alive = true;
        entity
    }

    /// 创建训练靶实体（不可移动核的静止靶默认姿态）
    ///
    /// 训练靶由 `combat::range` 生成并附加运动/计分组件；此处仅固定类型与血量，
    /// 使 `is_alive` 恒为 true（靶永不因掉血被销毁）。
    pub fn new_target(id: u64, position: Vec3) -> Self {
        let mut entity = Self::new(id, position);
        entity.entity_type = EntityType::Target;
        entity.max_hp = f32::MAX;
        entity.hp = f32::MAX;
        entity.is_alive = true;
        entity
    }

    /// 创建场上拾取物实体（弹药/医疗/护甲/手雷/武器）。
    ///
    /// 设计动机（Why）：拾取物的"是什么"（数值/元素）属于战局内容，由服务端权威裁决，
    /// 故实体本身只固定类型与不可破坏性，具体语义挂在 `interact::Interactable` 组件上，
    /// 由快照下发给客户端展示、由交互命令结算。
    pub fn new_loot(id: u64, position: Vec3) -> Self {
        let mut entity = Self::new(id, position);
        entity.entity_type = EntityType::Loot;
        // 拾取物不参与伤害结算（`shooter` 只认 AI/Target），血量取上限避免被误判死亡。
        entity.max_hp = f32::MAX;
        entity.hp = f32::MAX;
        entity.is_alive = true;
        entity
    }

    /// 创建功能站点实体（补给台/干员切换台/物资箱）：恒存活、不可移动，交互语义同样由组件承载。
    pub fn new_station(id: u64, position: Vec3) -> Self {
        let mut entity = Self::new(id, position);
        entity.entity_type = EntityType::Station;
        entity.max_hp = f32::MAX;
        entity.hp = f32::MAX;
        entity.is_alive = true;
        entity
    }

    /// 添加组件
    pub fn add_component(&mut self, mut component: Box<dyn Component>) {
        self.flush_pending_removals();

        let type_id = component.as_any().type_id();
        
        // 如果已存在同类型组件，先移除旧的
        if let Some(mut old) = self.components.remove(&type_id) {
            old.on_remove(self);
        }
        
        component.on_apply(self);
        self.components.insert(type_id, component);
    }

    /// 移除组件（延迟执行，避免遍历期间修改）
    pub fn remove_component<T: Component>(&mut self) {
        let type_id = TypeId::of::<T>();
        self.pending_removals.push(type_id);
    }

    /// 刷新待移除组件队列
    fn flush_pending_removals(&mut self) {
        let type_ids: Vec<TypeId> = self.pending_removals.drain(..).collect();
        for type_id in type_ids {
            if let Some(mut component) = self.components.remove(&type_id) {
                component.on_remove(self);
            }
        }
    }

    /// 获取组件引用
    pub fn get_component<T: Component>(&self) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        self.components.get(&type_id)
            .and_then(|c| c.as_any().downcast_ref::<T>())
    }

    /// 获取组件可变引用
    pub fn get_component_mut<T: Component>(&mut self) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        self.components.get_mut(&type_id)
            .and_then(|c| c.as_any_mut().downcast_mut::<T>())
    }

    /// 检查是否有指定组件（排除待移除的）
    pub fn has_component<T: Component>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        self.components.contains_key(&type_id) && !self.pending_removals.contains(&type_id)
    }

    /// 推送伤害包到收件箱
    pub fn push_damage(&mut self, packet: crate::damage::DamagePacket) {
        self.inbox.push(packet);
    }

    /// 处理收件箱（消费所有待处理的伤害包）
    pub fn process_inbox(&mut self, resolver: &crate::damage::DamageResolver, environment: &EntityElementState) {
        let packets: Vec<_> = self.inbox.drain(..).collect();
        for packet in packets {
            resolver.resolve(self, &packet, environment);
        }
    }

    /// 执行Tick更新（所有组件）
    pub fn tick(&mut self, delta_time: f32) {
        if !self.is_alive {
            return;
        }

        // 临时取出components，避免遍历期间与self产生借用冲突
        let mut components = std::mem::take(&mut self.components);
        for component in components.values_mut() {
            component.on_tick(self, delta_time);
        }
        self.components = components;
        self.flush_pending_removals();

        // 检查死亡
        if self.hp <= 0.0 {
            self.hp = 0.0;
            self.is_alive = false;
        }
    }

    /// 获取组件数量（排除待移除的）
    pub fn component_count(&self) -> usize {
        self.components.len() - self.pending_removals.iter().filter(|id| self.components.contains_key(id)).count()
    }

    /// 获取所有组件名称
    pub fn component_names(&self) -> Vec<&'static str> {
        self.components.values()
            .map(|c| c.name())
            .collect()
    }

    /// 清理所有组件
    pub fn clear_components(&mut self) {
        self.flush_pending_removals();
        
        let mut components = std::mem::take(&mut self.components);
        for (_, mut component) in components.drain() {
            component.on_remove(self);
        }
        self.components = components;
    }
}
