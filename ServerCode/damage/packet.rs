//! 伤害数据包与命令的定义（纯数据，零逻辑）
//!
//! 包含：
//! - `Vec3`：3D 向量（简化版）
//! - `DamagePacket`：纯数据伤害包
//! - `EffectTag`：效果标签（编译期副作用标识）
//! - `ExplosionCmd`：爆炸指令（预爆炸缓存推送用）
//! - `FalloffType`：伤害衰减类型

use crate::element::ElementType;
use crate::entity::EntityId;

/// 3D向量（简化版）
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// 计算两点距离
    pub fn distance(&self, other: &Vec3) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2) + (self.z - other.z).powi(2)).sqrt()
    }

    /// 线性插值
    pub fn lerp(&self, other: &Vec3, t: f32) -> Vec3 {
        Vec3 {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
            z: self.z + (other.z - self.z) * t,
        }
    }
}

/// 纯数据伤害包
/// 
/// 关键：DamagePacket 是纯数据，零逻辑。
/// 它不携带"目标应该如何响应"的信息。
#[derive(Debug, Clone)]
pub struct DamagePacket {
    /// 攻击元素类型
    pub element: ElementType,
    /// 基础伤害值
    pub base_value: f32,
    /// 伤害源位置（用于计算距离衰减）
    pub source_pos: Vec3,
    /// 伤害源实体ID
    pub source_id: EntityId,
    /// 攻击自带的副作用标签
    pub inherent_effects: Vec<EffectTag>,
    /// 是否暴击
    pub is_critical: bool,
    /// 暴击倍率
    pub critical_multiplier: f32,
}

impl DamagePacket {
    /// 创建基础伤害包
    pub fn new(element: ElementType, base_value: f32, source_id: EntityId) -> Self {
        Self {
            element,
            base_value,
            source_pos: Vec3::default(),
            source_id,
            inherent_effects: Vec::new(),
            is_critical: false,
            critical_multiplier: 1.5,
        }
    }

    /// 设置源位置
    pub fn with_source_pos(mut self, pos: Vec3) -> Self {
        self.source_pos = pos;
        self
    }

    /// 添加副作用标签
    pub fn with_effect(mut self, effect: EffectTag) -> Self {
        self.inherent_effects.push(effect);
        self
    }

    /// 设置暴击
    pub fn with_critical(mut self, multiplier: f32) -> Self {
        self.is_critical = true;
        self.critical_multiplier = multiplier;
        self
    }
}

/// 效果标签
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectTag {
    Burning,           // 燃烧
    Frozen,            // 冰冻
    Poisoned,          // 中毒
    Wet,               // 潮湿
    Electrified,       // 感电
    Vaporize,          // 蒸发
    Melt,              // 融化
    ShatterFreeze,     // 爆裂冻结
    PhysicalVulnerability, // 物理易伤
    ArmorShatter,      // 护甲碎裂
    HealBlock,         // 治疗阻断
    Stun,              // 眩晕
}

/// 爆炸指令（预爆炸缓存推送用）
#[derive(Debug, Clone)]
pub struct ExplosionCmd {
    /// 元素类型
    pub element: ElementType,
    /// 基础伤害
    pub base_damage: f32,
    /// 冲击方向
    pub impulse_direction: Vec3,
    /// 冲量大小
    pub impulse_magnitude: f32,
    /// 爆炸中心
    pub explosion_center: Vec3,
    /// 来源实体ID
    pub source_id: EntityId,
    /// 爆炸半径
    pub radius: f32,
    /// 衰减曲线类型
    pub falloff: FalloffType,
}

/// 伤害衰减类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FalloffType {
    Linear,      // 线性衰减
    Quadratic,   // 二次衰减
    Constant,    // 恒定（无衰减）
}