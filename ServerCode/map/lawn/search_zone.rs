//! 搜索区（搜，z 200..440）—— 玩家离开出生区后先在本区"搜索"。
//!
//! 平坦草坪上散布搜索目标（静态靶）与拾取物（弹药/医疗/护甲/手雷），
//! 模拟侦察阶段逐一清除与收刮。本区无实心掩体，保证全向视野与直射。

use crate::element::ElementType;
use crate::map::{PickupKind, PickupSpec, TargetSpec};

/// 搜索目标：两列纵向散布，间距错开避免排成直线
pub(crate) fn search_targets() -> Vec<TargetSpec> {
    vec![
        TargetSpec { pos: [-360.0, 1.5, 400.0], label: "搜索目标", motion: None },
        TargetSpec { pos: [360.0, 1.5, 400.0], label: "搜索目标", motion: None },
        TargetSpec { pos: [-240.0, 1.5, 360.0], label: "搜索目标", motion: None },
        TargetSpec { pos: [240.0, 1.5, 360.0], label: "搜索目标", motion: None },
        TargetSpec { pos: [-120.0, 1.5, 320.0], label: "搜索目标", motion: None },
        TargetSpec { pos: [120.0, 1.5, 320.0], label: "搜索目标", motion: None },
        TargetSpec { pos: [-300.0, 1.5, 280.0], label: "搜索目标", motion: None },
        TargetSpec { pos: [300.0, 1.5, 280.0], label: "搜索目标", motion: None },
        TargetSpec { pos: [-60.0, 1.5, 240.0], label: "搜索目标", motion: None },
        TargetSpec { pos: [60.0, 1.5, 240.0], label: "搜索目标", motion: None },
    ]
}

/// 搜索区拾取物：左右镜像成对散布，模拟"搜刮战利品"
pub(crate) fn search_pickups() -> Vec<PickupSpec> {
    vec![
        PickupSpec { pos: [-180.0, 0.4, 380.0], label: "弹药箱", kind: PickupKind::Ammo { amount: 30 } },
        PickupSpec { pos: [180.0, 0.4, 380.0], label: "大医疗包", kind: PickupKind::Health { amount: 50.0 } },
        PickupSpec {
            pos: [-40.0, 0.4, 300.0],
            label: "毒素手雷",
            kind: PickupKind::Grenade { element: ElementType::Poison },
        },
        PickupSpec {
            pos: [40.0, 0.4, 300.0],
            label: "雷电手雷",
            kind: PickupKind::Grenade { element: ElementType::Electric },
        },
        PickupSpec { pos: [-260.0, 0.4, 260.0], label: "护甲片", kind: PickupKind::Armor { amount: 30.0 } },
        PickupSpec { pos: [260.0, 0.4, 260.0], label: "弹药箱", kind: PickupKind::Ammo { amount: 60 } },
    ]
}
