//! 装备等级/类型/元素枚举（线格式载荷契约）
//!
//! 设计动机（Why）：`InventoryAction::Craft` 载荷里带 [`EquipmentType`] / [`EquipmentTier`] /
//! [`EquipmentElement`]，故这三个枚举必须双端一致。装备实例、随机生成、交易与注册表
//! 属服务端模拟逻辑，留在 `ServerCode::equipment`。

use serde::{Deserialize, Serialize};

/// 装备等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipmentTier {
    Tier1 = 1,
    Tier2 = 2,
    Tier3 = 3,
    Tier4 = 4,
    Tier5 = 5,
    Tier6 = 6,
    Tier7 = 7,
    Tier8 = 8,
    Tier9 = 9,
}

impl EquipmentTier {
    /// 从数值创建等级
    pub fn from_u8(tier: u8) -> Option<Self> {
        match tier {
            1 => Some(EquipmentTier::Tier1),
            2 => Some(EquipmentTier::Tier2),
            3 => Some(EquipmentTier::Tier3),
            4 => Some(EquipmentTier::Tier4),
            5 => Some(EquipmentTier::Tier5),
            6 => Some(EquipmentTier::Tier6),
            7 => Some(EquipmentTier::Tier7),
            8 => Some(EquipmentTier::Tier8),
            9 => Some(EquipmentTier::Tier9),
            _ => None,
        }
    }

    /// 获取等级数值
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }

    /// 是否为新手保护舱（1级）
    pub fn is_safe_zone(&self) -> bool {
        matches!(self, EquipmentTier::Tier1)
    }

    /// 是否为博弈区（2-6级）
    pub fn is_gamble_zone(&self) -> bool {
        matches!(
            self,
            EquipmentTier::Tier2
                | EquipmentTier::Tier3
                | EquipmentTier::Tier4
                | EquipmentTier::Tier5
                | EquipmentTier::Tier6
        )
    }

    /// 是否为混沌区（7-9级）
    pub fn is_chaos_zone(&self) -> bool {
        matches!(self, EquipmentTier::Tier7 | EquipmentTier::Tier8 | EquipmentTier::Tier9)
    }

    /// 是否可指定元素
    pub fn can_specify_element(&self) -> bool {
        self.is_gamble_zone()
    }

    /// 是否拥有元素属性
    pub fn has_element(&self) -> bool {
        !self.is_safe_zone()
    }
}

/// 装备类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipmentType {
    /// 护甲
    Armor,
    /// 武器
    Weapon,
    /// 手雷/投掷物
    Grenade,
    /// 配件
    Accessory,
}

/// 装备元素属性
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipmentElement {
    /// 无元素
    None,
    /// 火
    Fire,
    /// 冰
    Ice,
    /// 电
    Electric,
    /// 毒
    Poison,
}

// EquipmentElement 扩展，用于相性计算
impl EquipmentElement {
    /// 映射到核心元素类型（无元素视为物理/中性）。
    pub fn to_element_type(&self) -> Option<crate::element::ElementType> {
        match self {
            EquipmentElement::Fire => Some(crate::element::ElementType::Fire),
            EquipmentElement::Ice => Some(crate::element::ElementType::Ice),
            EquipmentElement::Electric => Some(crate::element::ElementType::Electric),
            EquipmentElement::Poison => Some(crate::element::ElementType::Poison),
            EquipmentElement::None => Some(crate::element::ElementType::Physical),
        }
    }
}