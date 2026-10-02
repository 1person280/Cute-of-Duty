//! 选装预设：服务端权威的起始携带模板（全局固定 3 套）。
//!
//! 设计动机（Why）：预设内容属"应该算什么"的服务端职责——客户端不内嵌任何预设数据，
//! 只展示服务端下发的目录（[`ServerMessage::PresetCatalog`]）并回传选中项的名字清单，
//! 避免双端漂移与本地篡改。预设另开物品池，允许使用仓库物资池 7 项之外的物资。

use cute_of_duty_contract::net::protocol::LoadoutPreset;

/// 全局固定 3 套预设：均衡（标准）/ 续航（生存）/ 元素火力（爆破）。
///
/// 名称与清单均为展示口径的物资名，由 [`crate::items::catalog::item_from_name`] 解析为
/// 权威物品；此处不写数值，数值只由映射表裁决。
pub fn all() -> Vec<LoadoutPreset> {
    vec![
        LoadoutPreset {
            name: "标准".to_string(),
            items: vec![
                "大型医疗包".to_string(),
                "重型护甲板".to_string(),
                "步枪弹药".to_string(),
                "烈焰手雷".to_string(),
                "雷电手雷".to_string(),
            ],
        },
        LoadoutPreset {
            name: "生存".to_string(),
            items: vec![
                "急救包".to_string(),
                "大型医疗包".to_string(),
                "医疗包".to_string(),
                "护甲片".to_string(),
                "毒素手雷".to_string(),
            ],
        },
        LoadoutPreset {
            name: "爆破".to_string(),
            items: vec![
                "破片手雷".to_string(),
                "烈焰手雷".to_string(),
                "冰霜手雷".to_string(),
                "水压手雷".to_string(),
                "步枪弹药".to_string(),
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::catalog::item_from_name;

    /// 三套预设的每一个物资名都必须能被权威映射表解析（否则下发即含非法名）。
    #[test]
    fn every_preset_item_is_resolvable() {
        let presets = all();
        assert_eq!(presets.len(), 3, "应为 3 套预设");
        for p in &presets {
            assert!(!p.items.is_empty(), "预设「{}」不应为空", p.name);
            for name in &p.items {
                assert!(item_from_name(name).is_some(), "「{}」中的「{name}」无法解析", p.name);
            }
        }
    }

    /// 每套预设不超过 12 格（4×3 背包容量的权威上限）。
    #[test]
    fn presets_fit_backpack() {
        for p in all() {
            assert!(p.items.len() <= 12, "预设「{}」超出背包容量", p.name);
        }
    }
}
