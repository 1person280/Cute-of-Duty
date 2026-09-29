//! 交互载荷契约（`InteractInfo` / `InteractChoice` / `SupplyKind` / 交互半径）
//!
//! 设计动机（Why）：`EntitySnapshot::interact` 携带 [`InteractInfo`]、上行 `ClientMessage::Interact`
//! 携带 [`InteractChoice`]，二者都是快照/上行契约的一部分；交互半径 [`INTERACT_RANGE`] 两端
//! 判定口径必须一致。而"挂组件、判距离、发效果"（`Interactable` / `settle`）属服务端权威，
//! 留在 `ServerCode::interact`。

use serde::{Deserialize, Serialize};

use crate::map::{PickupKind, StationKind};

/// 交互触发半径（米，平面距离）。与客户端提示的判定口径一致。
pub const INTERACT_RANGE: f32 = 3.5;

/// 可交互语义（服务端权威）：地面拾取物 or 功能站点。
///
/// 直接内嵌 `map` 的数据层枚举，使"地图定义了哪些拾取物/站点"与"实体携带什么语义"
/// 是同一套类型，新增一种拾取物无需两处同步。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum InteractKind {
    /// 地面拾取物（一次性，拾取后消失）
    Pickup(PickupKind),
    /// 功能站点（可反复交互）
    Station(StationKind),
}

/// 快照下行的"可交互"信息：客户端据此显示 `[F] 提示` 与交互菜单。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteractInfo {
    /// 展示名（"补给台"/"医疗包"…）
    pub label: String,
    pub kind: InteractKind,
}

/// 补给台三项补给的选择项（客户端菜单选项 → 服务端发放对应物资）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupplyKind {
    Ammo,
    Health,
    Armor,
}

impl SupplyKind {
    /// 展示名（菜单选项文案）。
    ///
    /// 设计动机（Why）：玩家在补给台上是"照着物品名领取"——选项必须直接写出到手的
    /// 物资名与数量（对齐 legacy 补给菜单），而不是"领取弹药/领取医疗"这类动作词。
    pub fn label(self) -> &'static str {
        match self {
            SupplyKind::Ammo => "步枪弹药 ×90",
            SupplyKind::Health => "医疗包",
            SupplyKind::Armor => "护甲片",
        }
    }
}

/// 客户端交互选择（只给"选了哪一项"，具体数值由服务端裁决）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InteractChoice {
    /// 拾取地面物品（无需选择）
    Take,
    /// 补给台：领取指定物资
    Supply { kind: SupplyKind },
    /// 打开场景物资箱
    OpenCrate,
}