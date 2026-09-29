//! 可携带物品契约（`LootItem` 与格位常量）
//!
//! 设计动机（Why）：背包/容器格位内容随快照下发（`EntitySnapshot::backpack` /
//! `container`），逐格转移方向 `TransferDir` 随上行下发，故 [`LootItem`] / [`TransferDir`]
//! 与格位常量都是**契约**。格位的权威宿主（`Backpack` / `Container`）与裁决（`transfer`）
//! 属服务端模拟逻辑，留在 `ServerCode::items`。

use serde::{Deserialize, Serialize};

use crate::map::PickupKind;

/// 背包格位数（4 列 × 3 行）。
pub const BACKPACK_SLOTS: usize = 12;
/// 物资箱格位数（4 列 × 3 行）。
pub const CONTAINER_SLOTS: usize = 12;

/// 一件可携带物品（占据一个格位：展示名 + 语义/数值 + 堆叠数量）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LootItem {
    pub label: String,
    pub kind: PickupKind,
    /// 该格内的堆叠数量（≥1）。上限由 [`PickupKind::max_stack`] 决定。
    pub count: u32,
}

impl LootItem {
    /// 由展示名与语义构造（默认单件，`count = 1`）。
    pub fn new(label: impl Into<String>, kind: PickupKind) -> Self {
        Self {
            label: label.into(),
            kind,
            count: 1,
        }
    }

    /// 由展示名、语义与堆叠数量构造。
    ///
    /// 注意：`count` 可超过单格上限，表示"若干发待入库"——由权威格位宿主负责按上限
    /// **拆成多格堆叠**，因此构造出的物品不保证已是合法单格，落格后必然合法。
    pub fn with_count(label: impl Into<String>, kind: PickupKind, count: u32) -> Self {
        Self {
            label: label.into(),
            kind,
            count: count.max(1),
        }
    }

    /// 展示名（数量 > 1 时附 `×N`，供 HUD 格位/轮盘显示）。
    pub fn display_label(&self) -> String {
        if self.count > 1 {
            format!("{} ×{}", self.label, self.count)
        } else {
            self.label.clone()
        }
    }
}

/// 物品速用类别（3/4 号消耗品槽与径向轮盘的统一分类口径）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemCategory {
    /// 恢复类（医疗包 / 护甲片）→ 3 号槽
    Consumable,
    /// 战术类（手雷）→ 4 号槽
    Tactical,
}

impl PickupKind {
    /// 该物品单格堆叠上限。
    ///
    /// 设计动机（Why）：备用子弹改为**背包物品**后需要明确的堆叠口径——弹药 64（大弹夹量级），
    /// 恢复/战术类 16（小件消耗品），武器属**工具不可堆叠**（每把独占一格）。
    pub fn max_stack(self) -> u32 {
        match self {
            PickupKind::Ammo { .. } => 64,
            PickupKind::Health { .. } | PickupKind::Armor { .. } | PickupKind::Grenade { .. } => 16,
            // 工具不可堆叠：每件独占一格。
            PickupKind::Weapon { .. } => 1,
        }
    }

    /// 是否占用背包格位：仅武器走"直接换手"（不占格），其余（含弹药）均入背包格。
    pub fn occupies_slot(self) -> bool {
        !matches!(self, PickupKind::Weapon { .. })
    }

    /// 速用类别（弹药/武器不可速用，返回 `None`）。
    pub fn category(self) -> Option<ItemCategory> {
        match self {
            PickupKind::Health { .. } | PickupKind::Armor { .. } => Some(ItemCategory::Consumable),
            PickupKind::Grenade { .. } => Some(ItemCategory::Tactical),
            _ => None,
        }
    }

    /// 是否与另一件物品属**同一可堆叠种类**（可合并进同一堆）。
    ///
    /// 设计动机（Why）：堆叠判定必须**忽略逐件承载数量**——地面/箱子里的弹药条目各带不同
    /// `amount`（60/30/…），但它们都是"步枪弹药"，理应并堆；若直接比较 `kind` 相等会因
    /// `amount` 不同而永远无法合并。恢复类同理（同一展示名下数值一致，由调用方按 `label`
    /// 兜底区分如"医疗包/大型医疗包"），手雷/武器则按元素区分。
    pub fn same_stack_kind(&self, other: &Self) -> bool {
        match (self, other) {
            (PickupKind::Ammo { .. }, PickupKind::Ammo { .. }) => true,
            (PickupKind::Health { .. }, PickupKind::Health { .. }) => true,
            (PickupKind::Armor { .. }, PickupKind::Armor { .. }) => true,
            (PickupKind::Grenade { element: a }, PickupKind::Grenade { element: b }) => a == b,
            (PickupKind::Weapon { element: a }, PickupKind::Weapon { element: b }) => a == b,
            _ => false,
        }
    }
}

/// 转移方向（客户端只上报"哪一边取、哪一格"）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferDir {
    /// 容器格 → 背包
    Take,
    /// 背包格 → 容器
    Put,
}