//! 物资名 → 权威物品构造（服务端单一事实来源）。
//!
//! 设计动机（Why）：客户端只上报物资**名字**（`ClientMessage::Loadout::carried`），
//! "这个名字对应什么语义 / 多少数值"必须由服务端裁决且全项目唯一——否则同一名字在
//! 掉落池、补给、选装三处各写一份，必然漂移。本表是名字到 [`LootItem`] 的唯一映射，
//! 未登记的名字一律判为非法（调用方据此忽略，绝不凭空造物）。

use crate::element::ElementType;
use crate::items::LootItem;
use crate::map::PickupKind;

/// 按物资名构造权威物品；未登记名字返回 `None`。
///
/// 覆盖范围：
/// - 仓库物资池 7 项：`医疗包` / `护甲板` / `额外弹药` + 四系元素手雷；
/// - 选装预设另开的物品池：`大型医疗包` / `急救包` / `护甲片` / `重型护甲板` /
///   `步枪弹药` / `破片手雷` / `水压手雷`。
pub fn item_from_name(name: &str) -> Option<LootItem> {
    let item = match name {
        "医疗包" => LootItem::new(name, PickupKind::Health { amount: 25.0 }),
        "大型医疗包" => LootItem::new(name, PickupKind::Health { amount: 50.0 }),
        "急救包" => LootItem::new(name, PickupKind::Health { amount: 75.0 }),
        "护甲片" => LootItem::new(name, PickupKind::Armor { amount: 20.0 }),
        // 「护甲板」（仓库物资池）与「重型护甲板」（预设池）同语义、同数值，仅文案不同。
        "护甲板" | "重型护甲板" => LootItem::new(name, PickupKind::Armor { amount: 50.0 }),
        // 「额外弹药」（仓库物资池）与「步枪弹药」（预设池）同为一叠 60 发。
        "额外弹药" | "步枪弹药" => LootItem::with_count(name, PickupKind::Ammo { amount: 60 }, 60),
        "烈焰手雷" => LootItem::new(name, PickupKind::Grenade { element: ElementType::Fire }),
        "冰霜手雷" => LootItem::new(name, PickupKind::Grenade { element: ElementType::Ice }),
        "雷电手雷" => LootItem::new(name, PickupKind::Grenade { element: ElementType::Electric }),
        "毒素手雷" => LootItem::new(name, PickupKind::Grenade { element: ElementType::Poison }),
        "破片手雷" => LootItem::new(name, PickupKind::Grenade { element: ElementType::Physical }),
        "水压手雷" => LootItem::new(name, PickupKind::Grenade { element: ElementType::Water }),
        _ => return None,
    };
    Some(item)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 仓库 7 项与预设池物品全部可解析，且类别正确。
    #[test]
    fn known_names_resolve() {
        for name in [
            "医疗包",
            "护甲板",
            "额外弹药",
            "烈焰手雷",
            "冰霜手雷",
            "雷电手雷",
            "毒素手雷",
            "大型医疗包",
            "急救包",
            "护甲片",
            "重型护甲板",
            "步枪弹药",
            "破片手雷",
            "水压手雷",
        ] {
            assert!(item_from_name(name).is_some(), "「{name}」应可解析");
        }
    }

    /// 未登记名字判为非法，不凭空造物。
    #[test]
    fn unknown_name_is_rejected() {
        assert!(item_from_name("核弹").is_none());
        assert!(item_from_name("").is_none());
    }

    /// 一叠弹药的堆叠数量为 60（而非单件）。
    #[test]
    fn ammo_preset_is_a_stack() {
        let ammo = item_from_name("额外弹药").expect("应有弹药");
        assert_eq!(ammo.count, 60);
    }
}
