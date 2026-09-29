//! 装备等级与元素规则系统
//!
//! 反RMT/反护航核心机制：
//! - 1级：无元素、无副作用、无组合（新手保护舱）
//! - 2-6级：获得时「真随机」元素，可付费指定（成本翻倍）
//! - 7-9级：获得时「真随机」元素，不可指定（混沌区）
//! - 转售/给予：元素属性重新真随机生成

use serde::{Deserialize, Serialize};
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_pcg::Pcg64Mcg;

/// 装备等级/类型/元素枚举（迁至契约 crate `cute_of_duty_contract::equipment`，此处保持公开路径不变）。
pub use cute_of_duty_contract::equipment::{EquipmentElement, EquipmentTier, EquipmentType};

/// 装备实例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Equipment {
    pub id: u64,
    pub name: String,
    pub tier: EquipmentTier,
    pub eq_type: EquipmentType,
    pub element: EquipmentElement,
    pub base_value: u32,
    pub durability: f32, // 0.0 ~ 1.0
    pub owner_id: Option<u64>,
    pub generation_seed: u64, // 生成时使用的种子，用于审计
}

/// 装备等级规则
#[derive(Debug, Clone)]
pub struct TierRules {
    pub tier: EquipmentTier,
    pub has_element: bool,
    pub can_specify_element: bool,
    pub specify_cost_multiplier: f32,
    pub description: &'static str,
}

/// 装备交易系统
#[derive(Debug, Clone)]
pub struct TradeRecord {
    pub equipment_id: u64,
    pub from_player: u64,
    pub to_player: u64,
    pub price: u32,
    pub timestamp: u64,
    pub element_before: EquipmentElement,
    pub element_after: EquipmentElement,
    pub transaction_hash: String,
}

/// 装备系统
pub struct EquipmentSystem {
    /// 装备注册表
    equipment_registry: std::sync::RwLock<std::collections::HashMap<u64, Equipment>>,
    /// 交易记录
    trade_history: std::sync::RwLock<Vec<TradeRecord>>,
    /// 全局种子计数器（用于确定性随机）
    seed_counter: std::sync::atomic::AtomicU64,
}

impl EquipmentSystem {
    /// 创建新的装备系统
    pub fn new() -> Self {
        Self {
            equipment_registry: std::sync::RwLock::new(std::collections::HashMap::new()),
            trade_history: std::sync::RwLock::new(Vec::new()),
            seed_counter: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// 获取等级规则
    pub fn get_tier_rules(&self, tier: EquipmentTier) -> TierRules {
        match tier {
            EquipmentTier::Tier1 => TierRules {
                tier,
                has_element: false,
                can_specify_element: false,
                specify_cost_multiplier: 1.0,
                description: "新手保护舱 - 无元素、无副作用、无组合",
            },
            EquipmentTier::Tier2 | EquipmentTier::Tier3 | EquipmentTier::Tier4 |
            EquipmentTier::Tier5 | EquipmentTier::Tier6 => TierRules {
                tier,
                has_element: true,
                can_specify_element: true,
                specify_cost_multiplier: 2.0,
                description: "博弈区 - 获得时真随机元素，可付费指定（成本翻倍）",
            },
            EquipmentTier::Tier7 | EquipmentTier::Tier8 | EquipmentTier::Tier9 => TierRules {
                tier,
                has_element: true,
                can_specify_element: false,
                specify_cost_multiplier: 0.0,
                description: "混沌区 - 获得时真随机元素，不可指定",
            },
        }
    }

    /// 生成随机元素（真随机）
    /// 
    /// 使用确定性RNG：基于Tick种子的确定性RNG
    /// 保证可审计、可复现
    fn generate_random_element(&self, seed: u64) -> EquipmentElement {
        let mut rng = Pcg64Mcg::seed_from_u64(seed);
        let elements = [
            EquipmentElement::Fire,
            EquipmentElement::Ice,
            EquipmentElement::Electric,
            EquipmentElement::Poison,
        ];
        elements.choose(&mut rng).copied().unwrap_or(EquipmentElement::Fire)
    }

    /// 创建新装备
    /// 
    /// # 参数
    /// - `id`: 装备唯一ID
    /// - `name`: 装备名称
    /// - `tier`: 装备等级
    /// - `eq_type`: 装备类型
    /// - `base_value`: 基础价值
    /// - `specified_element`: 指定元素（仅在博弈区且愿意支付双倍成本时有效）
    ///
    /// # 返回
    /// 新创建的装备实例
    pub fn create_equipment(
        &self,
        id: u64,
        name: String,
        tier: EquipmentTier,
        eq_type: EquipmentType,
        base_value: u32,
        specified_element: Option<EquipmentElement>,
    ) -> Equipment {
        let rules = self.get_tier_rules(tier);
        let seed = self.seed_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let element = if rules.has_element {
            if rules.can_specify_element && specified_element.is_some() {
                // 博弈区指定元素（成本已在调用处计算）
                specified_element.unwrap()
            } else {
                // 真随机生成
                self.generate_random_element(seed)
            }
        } else {
            EquipmentElement::None
        };

        Equipment {
            id,
            name,
            tier,
            eq_type,
            element,
            base_value,
            durability: 1.0,
            owner_id: None,
            generation_seed: seed,
        }
    }

    /// 转移装备（给予/交易/市场购买）
    ///
    /// 关键规则：
    /// - 玩家间直接交易：元素重随
    /// - 市场购买：元素固定（卖家上架时的随机结果）
    /// - 同账号内转移：不变
    /// - 跨账号/给予他人：重随
    pub fn transfer_equipment(
        &self,
        equipment_id: u64,
        from_player: u64,
        to_player: u64,
        price: u32,
        transfer_type: TransferType,
    ) -> Result<Equipment, EquipmentError> {
        let mut registry = self.equipment_registry.write().unwrap();
        
        let equipment = registry.get_mut(&equipment_id)
            .ok_or(EquipmentError::EquipmentNotFound)?;

        let element_before = equipment.element.clone();

        // 根据转移类型决定是否重随元素
        match transfer_type {
            TransferType::DirectTrade | TransferType::CrossAccount => {
                // 直接交易或跨账号：元素重随
                let seed = self.seed_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                equipment.element = self.generate_random_element(seed);
            }
            TransferType::MarketPurchase => {
                // 市场购买：元素固定（卖家上架时的结果）
                // 不做修改
            }
            TransferType::SameAccountTransfer => {
                // 同账号转移：元素不变
                // 不做修改
            }
        }

        let element_after = equipment.element.clone();
        equipment.owner_id = Some(to_player);

        // 记录交易
        let record = TradeRecord {
            equipment_id,
            from_player,
            to_player,
            price,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            element_before,
            element_after,
            transaction_hash: blake3::hash(format!("{}:{}:{}:{}", 
                equipment_id, from_player, to_player, equipment.generation_seed).as_bytes()).to_hex().to_string(),
        };

        self.trade_history.write().unwrap().push(record);

        Ok(equipment.clone())
    }

    /// 计算装备价值（考虑元素相性）
    ///
    /// COD1 价值公式:
    /// 价值 = 等级 × 基础价值 × 【与当前队友/自身Build的元素相性】
    pub fn calculate_equipment_value(
        &self,
        equipment: &Equipment,
        team_elements: &[EquipmentElement],
    ) -> f32 {
        let tier_multiplier = equipment.tier.as_u8() as f32;
        let base = equipment.base_value as f32;

        // 计算元素相性
        let synergy = if equipment.tier.is_safe_zone() {
            1.0 // 1级装备无元素，无 synergy 变化
        } else {
            self.calculate_element_synergy(&equipment.element, team_elements)
        };

        tier_multiplier * base * synergy
    }

    /// 计算元素相性系数
    fn calculate_element_synergy(
        &self,
        equipment_element: &EquipmentElement,
        team_elements: &[EquipmentElement],
    ) -> f32 {
        if matches!(equipment_element, EquipmentElement::None) {
            return 1.0;
        }

        let mut synergy: f32 = 1.0;

        for team_elem in team_elements {
            match (equipment_element, team_elem) {
                // 同元素正向加成
                (EquipmentElement::Fire, EquipmentElement::Fire) => synergy += 0.1,
                (EquipmentElement::Ice, EquipmentElement::Ice) => synergy += 0.1,
                (EquipmentElement::Electric, EquipmentElement::Electric) => synergy += 0.1,
                (EquipmentElement::Poison, EquipmentElement::Poison) => synergy += 0.1,

                // 互斥元素负向惩罚
                (EquipmentElement::Fire, EquipmentElement::Ice) => synergy -= 0.2,
                (EquipmentElement::Ice, EquipmentElement::Fire) => synergy -= 0.2,
                (EquipmentElement::Electric, EquipmentElement::Poison) => synergy -= 0.15,
                (EquipmentElement::Poison, EquipmentElement::Electric) => synergy -= 0.2,

                _ => {}
            }
        }

        synergy.max(0.1) // 最低保留10%价值
    }

    /// 注册装备到系统
    pub fn register_equipment(&self, equipment: Equipment) {
        let mut registry = self.equipment_registry.write().unwrap();
        registry.insert(equipment.id, equipment);
    }

    /// 获取装备
    pub fn get_equipment(&self, id: u64) -> Option<Equipment> {
        let registry = self.equipment_registry.read().unwrap();
        registry.get(&id).cloned()
    }

    /// 移除装备（丢弃/上架成交/守卫失败回滚）。
    ///
    /// 语义（Why）：装备实例是全局注册表内的热数据；背包丢弃或 CRUD 守卫回滚时
    /// 需从注册表移除该实例，同时反馈原值供调用方核对（不存在时给出明确错误）。
    pub fn remove_equipment(&self, id: u64) -> Result<Equipment, EquipmentError> {
        let mut registry = self.equipment_registry.write().unwrap();
        registry
            .remove(&id)
            .ok_or(EquipmentError::EquipmentNotFound)
    }

    /// 获取交易记录数量
    pub fn trade_count(&self) -> usize {
        self.trade_history.read().unwrap().len()
    }
}

/// 转移类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferType {
    DirectTrade,        // 玩家间直接交易
    MarketPurchase,     // 市场购买
    SameAccountTransfer, // 同账号转移
    CrossAccount,       // 跨账号转移
}

/// 装备错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EquipmentError {
    EquipmentNotFound,
    InvalidTier,
    CannotSpecifyElement,
    InsufficientFunds,
    TradeBlocked,
}

impl std::fmt::Display for EquipmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EquipmentError::EquipmentNotFound => write!(f, "装备未找到"),
            EquipmentError::InvalidTier => write!(f, "无效的装备等级"),
            EquipmentError::CannotSpecifyElement => write!(f, "该等级装备不可指定元素"),
            EquipmentError::InsufficientFunds => write!(f, "资金不足"),
            EquipmentError::TradeBlocked => write!(f, "交易被阻止"),
        }
    }
}

impl std::error::Error for EquipmentError {}

// `EquipmentElement::to_element_type` 随枚举迁至契约 crate `cute_of_duty_contract::equipment`。



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_rules() {
        let system = EquipmentSystem::new();

        let tier1 = system.get_tier_rules(EquipmentTier::Tier1);
        assert!(!tier1.has_element);
        assert!(!tier1.can_specify_element);

        let tier5 = system.get_tier_rules(EquipmentTier::Tier5);
        assert!(tier5.has_element);
        assert!(tier5.can_specify_element);
        assert_eq!(tier5.specify_cost_multiplier, 2.0);

        let tier8 = system.get_tier_rules(EquipmentTier::Tier8);
        assert!(tier8.has_element);
        assert!(!tier8.can_specify_element);
    }

    #[test]
    fn test_equipment_creation() {
        let system = EquipmentSystem::new();

        // 1级装备无元素
        let eq1 = system.create_equipment(
            1, "新手护甲".to_string(), EquipmentTier::Tier1,
            EquipmentType::Armor, 100, None,
        );
        assert_eq!(eq1.element, EquipmentElement::None);

        // 5级装备随机元素
        let eq5 = system.create_equipment(
            2, "精英武器".to_string(), EquipmentTier::Tier5,
            EquipmentType::Weapon, 500, None,
        );
        assert_ne!(eq5.element, EquipmentElement::None);

        // 5级装备指定元素
        let eq5_specified = system.create_equipment(
            3, "精英武器(指定)".to_string(), EquipmentTier::Tier5,
            EquipmentType::Weapon, 500, Some(EquipmentElement::Fire),
        );
        assert_eq!(eq5_specified.element, EquipmentElement::Fire);

        // 8级装备不可指定
        let eq8 = system.create_equipment(
            4, "传说护甲".to_string(), EquipmentTier::Tier8,
            EquipmentType::Armor, 800, Some(EquipmentElement::Ice),
        );
        // 即使指定了也无效，应为随机
        assert_ne!(eq8.element, EquipmentElement::None);
    }

    #[test]
    fn test_trade_element_reroll() {
        let system = EquipmentSystem::new();

        // 创建一个装备
        let mut eq = system.create_equipment(
            1, "测试武器".to_string(), EquipmentTier::Tier5,
            EquipmentType::Weapon, 100, Some(EquipmentElement::Fire),
        );
        eq.owner_id = Some(100);
        system.register_equipment(eq.clone());

        // 直接交易：元素应重随
        let result = system.transfer_equipment(
            1, 100, 200, 100, TransferType::DirectTrade,
        );
        assert!(result.is_ok());
        let traded = result.unwrap();
        // 元素可能相同也可能不同，但seed不同
        assert_eq!(traded.owner_id, Some(200));
    }
}
