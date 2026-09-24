//! 实体/组件系统 (ECS架构)
//!
//! 设计原则：
//! - 实体是组件的容器，无自身逻辑
//! - 组件是自洽的副作用单元，各自维护自身规则
//! - 系统按固定顺序处理实体，保证确定性

use crate::damage::Vec3;
use crate::element::EntityElementState;
use std::any::{Any, TypeId};
use std::collections::HashMap;

/// 实体ID（唯一标识）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct EntityId(pub u64);

impl EntityId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

/// 组件trait
/// 
/// 每个副作用是一个独立组件，内部封装自身规则，不感知其他组件存在。
/// 互克逻辑分散到各组件内部，而非集中在一个巨型函数中。
pub trait Component: Any + Send + Sync {
    /// 每Tick更新
    fn on_tick(&mut self, _owner: &mut Entity, _delta_time: f32) {}
    
    /// 组件被附加到实体时
    fn on_apply(&mut self, _owner: &mut Entity) {}
    
    /// 组件从实体移除时
    fn on_remove(&mut self, _owner: &mut Entity) {}
    
    /// 收到incoming元素时（互克反应）
    fn on_incoming_element(&mut self, _element: crate::element::ElementType, _owner: &mut Entity) {}
    
    /// 组件名称
    fn name(&self) -> &'static str;
    
    /// 转为Any引用（用于类型转换）
    fn as_any(&self) -> &dyn Any;
    
    /// 转为Any可变引用
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// 实体
/// 
/// 实体是组件的容器 + 基础属性集合。
/// 所有状态变更通过组件系统管理。
pub struct Entity {
    pub id: EntityId,
    pub position: Vec3,
    pub rotation: Vec3,
    pub hp: f32,
    pub max_hp: f32,
    pub armor: f32,
    pub move_speed: f32,
    pub base_move_speed: f32,
    pub element_state: EntityElementState,
    
    // 组件存储
    components: HashMap<TypeId, Box<dyn Component>>,

    // 待移除组件队列（避免在遍历期间修改components）
    pending_removals: Vec<TypeId>,
    
    // 输入队列（用于接收伤害指令等）
    inbox: Vec<crate::damage::DamagePacket>,
    
    // 实体类型标记
    pub entity_type: EntityType,
    
    // 存活状态
    pub is_alive: bool,
}

/// 实体类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityType {
    Player,
    AI,
    Grenade,
    Loot,
    Obstacle,
    /// 训练靶（实弹射击靶机）：可被射线命中并计分，填弹不致死
    Target,
}

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
            move_speed: 5.0,
            base_move_speed: 5.0,
            element_state: EntityElementState::Normal,
            components: HashMap::new(),
            pending_removals: Vec::new(),
            inbox: Vec::new(),
            entity_type: EntityType::Player,
            is_alive: true,
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

impl Drop for Entity {
    fn drop(&mut self) {
        self.clear_components();
    }
}

/// 世界（实体管理器）
/// 
/// 管理所有实体的创建、销毁和查询。
/// 保证确定性：实体按ID排序处理。
pub struct World {
    entities: HashMap<EntityId, Entity>,
    next_id: u64,
}

impl World {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            next_id: 1,
        }
    }

    /// 创建实体
    pub fn spawn(&mut self, mut entity: Entity) -> EntityId {
        let id = EntityId::new(self.next_id);
        self.next_id += 1;
        entity.id = id;
        self.entities.insert(id, entity);
        id
    }

    /// 获取实体引用
    pub fn get_entity(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    /// 获取实体可变引用
    pub fn get_entity_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    /// 销毁实体
    pub fn despawn(&mut self, id: EntityId) -> Option<Entity> {
        self.entities.remove(&id)
    }

    /// 获取所有实体（按ID排序）
    pub fn get_all_entities(&self) -> Vec<&Entity> {
        let mut entities: Vec<_> = self.entities.values().collect();
        entities.sort_by_key(|e| e.id.0);
        entities
    }

    /// 获取所有实体可变引用（按ID排序）
    /// 
    /// 注意：由于Rust借用规则，此方法在需要同时修改多个实体时不适用。
    /// 实际实现应使用split_mut或unsafe代码，此处为简化版API。
    pub fn get_all_entities_mut(&mut self) -> std::collections::hash_map::ValuesMut<'_, EntityId, Entity> {
        self.entities.values_mut()
    }

    /// 按ID排序遍历所有实体（确定性保证）
    pub fn tick_all_entities(&mut self, delta_time: f32) {
        // 收集所有ID并排序（确定性关键）
        let mut ids: Vec<_> = self.entities.keys().copied().collect();
        ids.sort_by_key(|id| id.0);

        for id in ids {
            if let Some(entity) = self.entities.get_mut(&id) {
                entity.tick(delta_time);
            }
        }

        // 清理已死亡实体
        self.cleanup_dead_entities();
    }

    /// 清理已死亡实体
    fn cleanup_dead_entities(&mut self) {
        let dead_ids: Vec<_> = self.entities
            .iter()
            .filter(|(_, e)| !e.is_alive)
            .map(|(id, _)| *id)
            .collect();
        
        for id in dead_ids {
            self.entities.remove(&id);
        }
    }

    /// 获取实体数量
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// 空间查询：获取指定范围内的实体
    pub fn query_in_radius(&self, center: Vec3, radius: f32) -> Vec<EntityId> {
        self.entities
            .values()
            .filter(|e| {
                e.is_alive && e.position.distance(&center) <= radius
            })
            .map(|e| e.id)
            .collect()
    }

    /// 空间查询：获取指定范围内特定类型的实体
    pub fn query_in_radius_by_type(
        &self,
        center: Vec3,
        radius: f32,
        entity_type: EntityType,
    ) -> Vec<EntityId> {
        self.entities
            .values()
            .filter(|e| {
                e.is_alive 
                    && e.entity_type == entity_type 
                    && e.position.distance(&center) <= radius
            })
            .map(|e| e.id)
            .collect()
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

/// 渲染状态快照（双缓冲用）
#[derive(Debug, Clone)]
pub struct RenderSnapshot {
    pub tick: u64,
    pub entity_positions: HashMap<EntityId, (Vec3, Vec3)>, // (position, rotation)
    pub entity_hp: HashMap<EntityId, f32>,
    pub entity_states: HashMap<EntityId, EntityElementState>,
}

impl RenderSnapshot {
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            entity_positions: HashMap::new(),
            entity_hp: HashMap::new(),
            entity_states: HashMap::new(),
        }
    }

    /// 从世界状态生成快照
    pub fn from_world(world: &World, tick: u64) -> Self {
        let mut snapshot = Self::new(tick);
        
        for entity in world.get_all_entities() {
            snapshot.entity_positions.insert(
                entity.id,
                (entity.position, entity.rotation)
            );
            snapshot.entity_hp.insert(entity.id, entity.hp);
            snapshot.entity_states.insert(entity.id, entity.element_state.clone());
        }
        
        snapshot
    }

    /// 计算状态哈希（用于确定性验证）
    pub fn compute_hash(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        self.tick.hash(&mut hasher);
        
        // 按ID排序后哈希（确定性）
        let mut positions: Vec<_> = self.entity_positions.iter().collect();
        positions.sort_by_key(|(id, _)| id.0);
        for (id, (pos, rot)) in positions {
            id.hash(&mut hasher);
            pos.x.to_bits().hash(&mut hasher);
            pos.y.to_bits().hash(&mut hasher);
            pos.z.to_bits().hash(&mut hasher);
            rot.x.to_bits().hash(&mut hasher);
            rot.y.to_bits().hash(&mut hasher);
            rot.z.to_bits().hash(&mut hasher);
        }
        
        let mut hps: Vec<_> = self.entity_hp.iter().collect();
        hps.sort_by_key(|(id, _)| id.0);
        for (id, hp) in hps {
            id.hash(&mut hasher);
            hp.to_bits().hash(&mut hasher);
        }
        
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::damage::{BurningComponent, Vec3};

    #[test]
    fn test_entity_creation() {
        let entity = Entity::new_player(1, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(entity.id.as_u64(), 1);
        assert_eq!(entity.hp, 100.0);
        assert!(entity.is_alive);
    }

    #[test]
    fn test_component_management() {
        let mut entity = Entity::new_player(1, Vec3::new(0.0, 0.0, 0.0));
        
        // 添加燃烧组件
        entity.add_component(Box::new(BurningComponent::new(5.0, 10.0)));
        assert!(entity.has_component::<BurningComponent>());
        assert_eq!(entity.component_count(), 1);

        // 移除组件
        entity.remove_component::<BurningComponent>();
        assert!(!entity.has_component::<BurningComponent>());
        assert_eq!(entity.component_count(), 0);
    }

    #[test]
    fn test_world_spawn_despawn() {
        let mut world = World::new();
        
        let entity = Entity::new_player(0, Vec3::new(1.0, 2.0, 3.0));
        let id = world.spawn(entity);
        
        assert_eq!(world.entity_count(), 1);
        assert!(world.get_entity(id).is_some());
        
        world.despawn(id);
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn test_render_snapshot_hash() {
        let mut world = World::new();
        
        let entity1 = Entity::new_player(0, Vec3::new(1.0, 0.0, 0.0));
        let entity2 = Entity::new_player(0, Vec3::new(2.0, 0.0, 0.0));
        
        world.spawn(entity1);
        world.spawn(entity2);
        
        let snapshot = RenderSnapshot::from_world(&world, 1);
        let hash = snapshot.compute_hash();
        
        // 相同状态应产生相同哈希
        let snapshot2 = RenderSnapshot::from_world(&world, 1);
        let hash2 = snapshot2.compute_hash();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_entity_death_cleanup() {
        let mut world = World::new();
        
        let entity = Entity::new_player(0, Vec3::new(0.0, 0.0, 0.0));
        let id = world.spawn(entity);
        
        // 杀死实体
        if let Some(e) = world.get_entity_mut(id) {
            e.hp = 0.0;
        }
        
        world.tick_all_entities(0.01667); // 一帧
        
        // 死亡实体应被清理
        assert_eq!(world.entity_count(), 0);
    }
}
