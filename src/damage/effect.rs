//! 副作用组件实现
//!
//! 攻击/元素反应后附加到实体的状态组件，
//! 每个组件实现 `crate::entity::Component` trait，
//! 由实体在 tick 时统一驱动。

use crate::entity::{Entity, Component};
use crate::element::{ElementType, EntityElementState};

/// 燃烧组件
#[derive(Debug)]
pub struct BurningComponent {
    pub duration: f32,
    pub dps: f32,
    pub elapsed: f32,
}

impl BurningComponent {
    pub fn new(duration: f32, dps: f32) -> Self {
        Self { duration, dps, elapsed: 0.0 }
    }
}

impl Component for BurningComponent {
    fn on_tick(&mut self, owner: &mut Entity, delta_time: f32) {
        self.elapsed += delta_time;
        owner.hp -= self.dps * delta_time;
        
        if self.elapsed >= self.duration {
            owner.remove_component::<BurningComponent>();
        }
    }

    fn on_incoming_element(&mut self, element: ElementType, owner: &mut Entity) {
        // 水熄灭火
        if matches!(element, ElementType::Water) {
            owner.remove_component::<BurningComponent>();
            // 产生蒸汽效果（简化）
            owner.element_state = EntityElementState::Normal;
        }
    }

    fn name(&self) -> &'static str {
        "Burning"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// 冰冻组件
#[derive(Debug)]
pub struct FrozenComponent {
    pub duration: f32,
    pub elapsed: f32,
    pub original_speed: f32,
}

impl FrozenComponent {
    pub fn new(duration: f32) -> Self {
        Self { duration, elapsed: 0.0, original_speed: -1.0 }
    }
}

impl Component for FrozenComponent {
    fn on_apply(&mut self, owner: &mut Entity) {
        self.original_speed = owner.move_speed;
        owner.move_speed = 0.0; // 冻结时无法移动
    }

    fn on_remove(&mut self, owner: &mut Entity) {
        if self.original_speed >= 0.0 {
            owner.move_speed = self.original_speed;
        }
    }

    fn on_tick(&mut self, owner: &mut Entity, delta_time: f32) {
        self.elapsed += delta_time;
        if self.elapsed >= self.duration {
            owner.remove_component::<FrozenComponent>();
        }
    }

    fn on_incoming_element(&mut self, element: ElementType, owner: &mut Entity) {
        // 火解除冰冻并产生融化
        if matches!(element, ElementType::Fire) {
            owner.remove_component::<FrozenComponent>();
            owner.add_component(Box::new(MeltComponent::new()));
        }
    }

    fn name(&self) -> &'static str {
        "Frozen"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// 融化组件（临时状态）
#[derive(Debug)]
pub struct MeltComponent;

impl MeltComponent {
    pub fn new() -> Self {
        Self
    }
}

impl Component for MeltComponent {
    fn name(&self) -> &'static str {
        "Melt"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// 中毒组件
#[derive(Debug)]
pub struct PoisonedComponent {
    pub duration: f32,
    pub dps: f32,
    pub elapsed: f32,
}

impl PoisonedComponent {
    pub fn new(duration: f32, dps: f32) -> Self {
        Self { duration, dps, elapsed: 0.0 }
    }
}

impl Component for PoisonedComponent {
    fn on_tick(&mut self, owner: &mut Entity, delta_time: f32) {
        self.elapsed += delta_time;
        owner.hp -= self.dps * delta_time;
        owner.armor -= 1.0 * delta_time; // 腐蚀护甲
        
        if self.elapsed >= self.duration {
            owner.remove_component::<PoisonedComponent>();
        }
    }

    fn name(&self) -> &'static str {
        "Poisoned"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// 护甲碎裂组件
#[derive(Debug)]
pub struct ArmorShatterComponent {
    pub duration: f32,
    pub elapsed: f32,
}

impl ArmorShatterComponent {
    pub fn new(duration: f32) -> Self {
        Self { duration, elapsed: 0.0 }
    }
}

impl Component for ArmorShatterComponent {
    fn on_tick(&mut self, owner: &mut Entity, delta_time: f32) {
        self.elapsed += delta_time;
        owner.armor = owner.armor.max(0.0) * 0.5; // 护甲减半
        
        if self.elapsed >= self.duration {
            owner.remove_component::<ArmorShatterComponent>();
        }
    }

    fn name(&self) -> &'static str {
        "ArmorShatter"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// 物理易伤组件
#[derive(Debug)]
pub struct PhysicalVulnerabilityComponent {
    pub multiplier: f32,
    pub duration: f32,
    pub elapsed: f32,
}

impl PhysicalVulnerabilityComponent {
    pub fn new(multiplier: f32, duration: f32) -> Self {
        Self { multiplier, duration, elapsed: 0.0 }
    }
}

impl Component for PhysicalVulnerabilityComponent {
    fn on_tick(&mut self, owner: &mut Entity, delta_time: f32) {
        self.elapsed += delta_time;
        if self.elapsed >= self.duration {
            owner.remove_component::<PhysicalVulnerabilityComponent>();
        }
    }

    fn name(&self) -> &'static str {
        "PhysicalVulnerability"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// 感电组件
#[derive(Debug)]
pub struct ElectrifiedComponent {
    pub duration: f32,
    pub elapsed: f32,
}

impl ElectrifiedComponent {
    pub fn new(duration: f32) -> Self {
        Self { duration, elapsed: 0.0 }
    }
}

impl Component for ElectrifiedComponent {
    fn on_tick(&mut self, owner: &mut Entity, delta_time: f32) {
        self.elapsed += delta_time;
        // 感电期间受到电系伤害增加
        if self.elapsed >= self.duration {
            owner.remove_component::<ElectrifiedComponent>();
        }
    }

    fn name(&self) -> &'static str {
        "Electrified"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// 眩晕组件
#[derive(Debug)]
pub struct StunComponent {
    pub duration: f32,
    pub elapsed: f32,
    pub original_speed: f32,
}

impl StunComponent {
    pub fn new(duration: f32) -> Self {
        Self { duration, elapsed: 0.0, original_speed: -1.0 }
    }
}

impl Component for StunComponent {
    fn on_apply(&mut self, owner: &mut Entity) {
        self.original_speed = owner.move_speed;
        owner.move_speed = 0.0;
    }

    fn on_remove(&mut self, owner: &mut Entity) {
        if self.original_speed >= 0.0 {
            owner.move_speed = self.original_speed;
        }
    }

    fn on_tick(&mut self, owner: &mut Entity, delta_time: f32) {
        self.elapsed += delta_time;
        if self.elapsed >= self.duration {
            owner.remove_component::<StunComponent>();
        }
    }

    fn name(&self) -> &'static str {
        "Stun"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// 毒云组件（范围持续伤害）
#[derive(Debug)]
pub struct PoisonCloudComponent {
    pub radius: f32,
    pub duration: f32,
    pub dps: f32,
    pub elapsed: f32,
}

impl PoisonCloudComponent {
    pub fn new(radius: f32, duration: f32) -> Self {
        Self { radius, duration, dps: 15.0, elapsed: 0.0 }
    }
}

impl Component for PoisonCloudComponent {
    fn on_tick(&mut self, owner: &mut Entity, delta_time: f32) {
        self.elapsed += delta_time;
        if self.elapsed >= self.duration {
            owner.remove_component::<PoisonCloudComponent>();
        }
    }

    fn name(&self) -> &'static str {
        "PoisonCloud"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}