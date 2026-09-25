//! 背包 CRUD 服务（服务端权威的热数据操作 + 落盘前的一致性守卫）
//!
//! Why: 将"背包该算什么"收口到此域。客户端只上报意图（`protocol::InventoryAction`），
//! 由主循环在单一权威线程内调用本模块结算：元素/上限/余额校验全在此裁决，且所有写
//! 操作都带**回滚语义**（先改热数据、失败即回滚、绝不留下半成品），保证与随后
//! Disconnect 落盘到冷仓库的口径一致。

use crate::equipment::{
    Equipment, EquipmentElement, EquipmentSystem, EquipmentTier, EquipmentType,
};
use crate::player::PlayerProfile;

/// 背包 CRUD 错误（区分可预知守卫失败与内部不一致）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryError {
    /// 背包格位已满
    SlotsFull,
    /// 丢弃下标越界
    IndexOutOfBounds,
    /// 货币余额不足
    InsufficientFunds,
    /// 背包引用的装备实例已丢失（全局注册表与背包不一致，属内部错误）
    EquipmentMissing,
}

impl std::fmt::Display for InventoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InventoryError::SlotsFull => write!(f, "背包格位已满"),
            InventoryError::IndexOutOfBounds => write!(f, "丢弃下标越界"),
            InventoryError::InsufficientFunds => write!(f, "货币余额不足"),
            InventoryError::EquipmentMissing => write!(f, "背包引用的装备实例已丢失"),
        }
    }
}

impl std::error::Error for InventoryError {}

/// 锻造操作的域内请求（主循环从 `protocol::InventoryAction` 映射而来，域不依赖线格式）。
pub struct CraftRequest {
    pub name: String,
    pub eq_type: EquipmentType,
    pub tier: EquipmentTier,
    pub base_value: u32,
    pub specified_element: Option<EquipmentElement>,
}

/// 锻造一件装备并加入玩家背包。
///
/// - 元素由 `EquipmentSystem::create_equipment` 按等级规则裁决（权威）；
/// - `next_id` 为全局装备 ID 分配器（跨连接唯一，主循环持有）；
/// - 守卫：先入全局注册表，再加背包；背包满则回滚注册表并拒绝，保证热数据一致。
pub fn craft(
    eqsys: &EquipmentSystem,
    profile: &mut PlayerProfile,
    next_id: &mut u64,
    req: CraftRequest,
) -> Result<Equipment, InventoryError> {
    let id = *next_id;
    *next_id += 1;

    let mut equipment = eqsys.create_equipment(
        id,
        req.name,
        req.tier,
        req.eq_type,
        req.base_value,
        req.specified_element,
    );
    equipment.owner_id = Some(profile.player_id);

    eqsys.register_equipment(equipment.clone());
    if !profile.inventory.add_equipment(equipment.id) {
        // 回滚热数据：背包满，注册表不再持有该件
        let _ = eqsys.remove_equipment(equipment.id);
        return Err(InventoryError::SlotsFull);
    }
    Ok(equipment)
}

/// 按背包下标丢弃装备（回收实例与格位）。
///
/// 守卫：先移除全局注册表（热数据），失败则背包不动，避免背包引用悬空实例。
pub fn discard_by_index(
    eqsys: &EquipmentSystem,
    profile: &mut PlayerProfile,
    index: usize,
) -> Result<(), InventoryError> {
    let equipment_id = *profile
        .inventory
        .equipment_ids
        .get(index)
        .ok_or(InventoryError::IndexOutOfBounds)?;

    eqsys
        .remove_equipment(equipment_id)
        .map_err(|_| InventoryError::EquipmentMissing)?;
    profile.inventory.remove_equipment(equipment_id);
    Ok(())
}

/// 货币增减（负数表示扣除）。
///
/// 守卫：任一币种扣除后为负 → 拒绝且整笔不生效（不半生半熟地扣一部分）。
pub fn adjust_currency(
    profile: &mut PlayerProfile,
    soft: i64,
    hard: i64,
    season: i64,
) -> Result<(), InventoryError> {
    let w = &profile.inventory.currency;
    // 先校验全部币种，余额不足整笔拒绝
    if (w.soft_currency as i64) + soft < 0
        || (w.hard_currency as i64) + hard < 0
        || (w.season_tokens as i64) + season < 0
    {
        return Err(InventoryError::InsufficientFunds);
    }
    let w = &mut profile.inventory.currency;
    w.soft_currency = (w.soft_currency as i64 + soft) as u32;
    w.hard_currency = (w.hard_currency as i64 + hard) as u32;
    w.season_tokens = (w.season_tokens as i64 + season) as u32;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(id: u64) -> PlayerProfile {
        PlayerProfile::new(id, "Tester")
    }

    #[test]
    fn craft_adds_equipment_and_owner() {
        let eqsys = EquipmentSystem::new();
        let mut p = profile(1);
        let mut next_id = 100;

        let eq = craft(
            &eqsys,
            &mut p,
            &mut next_id,
            CraftRequest {
                name: "新手护甲".into(),
                eq_type: EquipmentType::Armor,
                tier: EquipmentTier::Tier1,
                base_value: 100,
                specified_element: None,
            },
        )
        .unwrap();

        assert_eq!(eq.owner_id, Some(1));
        assert_eq!(p.inventory.used_slots, 1);
        assert_eq!(p.inventory.equipment_ids, vec![eq.id]);
        assert_eq!(next_id, 101, "装备 ID 应递增");
    }

    #[test]
    fn craft_rejects_when_slots_full() {
        let eqsys = EquipmentSystem::new();
        let mut p = profile(2);
        // 手动占满格位
        for suit in 0..p.inventory.max_slots {
            assert!(p.inventory.add_equipment(suit.into()));
        }
        let mut next_id = 1;

        let result = craft(
            &eqsys,
            &mut p,
            &mut next_id,
            CraftRequest {
                name: "超员".into(),
                eq_type: EquipmentType::Weapon,
                tier: EquipmentTier::Tier2,
                base_value: 10,
                specified_element: None,
            },
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), InventoryError::SlotsFull);
        // 回滚：注册表不应残留该件
        assert!(eqsys.get_equipment(1).is_none(), "背包满应回滚注册表");
    }

    #[test]
    fn discard_removes_instance_and_slot() {
        let eqsys = EquipmentSystem::new();
        let mut p = profile(3);
        let mut next_id = 10;
        craft(
            &eqsys,
            &mut p,
            &mut next_id,
            CraftRequest {
                name: "弃件".into(),
                eq_type: EquipmentType::Grenade,
                tier: EquipmentTier::Tier3,
                base_value: 1,
                specified_element: Some(EquipmentElement::Fire),
            },
        )
        .unwrap();
        assert_eq!(p.inventory.used_slots, 1);

        discard_by_index(&eqsys, &mut p, 0).unwrap();
        assert_eq!(p.inventory.used_slots, 0);
        assert!(eqsys.get_equipment(10).is_none(), "丢弃后实例应移除");
    }

    #[test]
    fn discard_out_of_bounds_is_rejected() {
        let eqsys = EquipmentSystem::new();
        let mut p = profile(4);
        assert_eq!(
            discard_by_index(&eqsys, &mut p, 5),
            Err(InventoryError::IndexOutOfBounds)
        );
    }

    #[test]
    fn adjust_currency_guards_insufficient() {
        let mut p = profile(5);
        assert_eq!(p.inventory.currency.soft_currency, 10000, "默认初始软币");

        // 扣到负数 → 拒绝，且整笔不动
        let err = adjust_currency(&mut p, -10001, 0, 0).unwrap_err();
        assert_eq!(err, InventoryError::InsufficientFunds);
        assert_eq!(p.inventory.currency.soft_currency, 10000);

        // 合法增减
        adjust_currency(&mut p, -1000, 500, 7).unwrap();
        assert_eq!(p.inventory.currency.soft_currency, 9000);
        assert_eq!(p.inventory.currency.hard_currency, 500);
        assert_eq!(p.inventory.currency.season_tokens, 7);
    }

    #[test]
    fn tier_rules_drive_craft_element() {
        let eqsys = EquipmentSystem::new();
        let mut p = profile(6);
        let mut next_id = 1u64;

        // 1 级：无元素
        let eq = craft(
            &eqsys,
            &mut p,
            &mut next_id,
            CraftRequest {
                name: "保护舱".into(),
                eq_type: EquipmentType::Armor,
                tier: EquipmentTier::Tier1,
                base_value: 10,
                specified_element: Some(EquipmentElement::Fire),
            },
        )
        .unwrap();
        assert_eq!(eq.element, EquipmentElement::None, "1 级强制无元素");

        // 5 级博弈区：可指定元素
        let eq = craft(
            &eqsys,
            &mut p,
            &mut next_id,
            CraftRequest {
                name: "指定火".into(),
                eq_type: EquipmentType::Weapon,
                tier: EquipmentTier::Tier5,
                base_value: 50,
                specified_element: Some(EquipmentElement::Fire),
            },
        )
        .unwrap();
        assert_eq!(eq.element, EquipmentElement::Fire);
    }
}