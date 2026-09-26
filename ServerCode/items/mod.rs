//! 对局内物品域（服务端权威）：可携带物品、背包格位、容器格位与逐格转移裁决。
//!
//! 设计动机（Why）：物资箱里"每一格是什么、取走后还剩什么、背包是否放得下"都属于
//! 应该算什么的服务端职责。客户端只上报"取哪一格 / 放哪一格"（[`TransferDir`]），
//! 服务端经 [`transfer`] 裁决后，把两侧格位内容随快照回传——客户端只画两个 4×3 网格。
//!
//! 依赖方向：`items → map`（复用 [`PickupKind`] 作为物品语义）与 `entity`（组件宿主），
//! **不依赖 `combat`**；"弹药入池 / 武器换手"这类即时效果由调用方（主循环）依据
//! [`TransferResult::Apply`] 施加，从而杜绝 `items ↔ combat` 循环依赖。

use std::any::Any;

use serde::{Deserialize, Serialize};

use crate::element::ElementType;
use crate::entity::Component;
use crate::map::PickupKind;

/// 背包格位数（4 列 × 3 行）。
pub const BACKPACK_SLOTS: usize = 12;
/// 物资箱格位数（4 列 × 3 行）。
pub const CONTAINER_SLOTS: usize = 12;

/// 一件可携带物品（占据一个格位：展示名 + 语义/数值）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LootItem {
    pub label: String,
    pub kind: PickupKind,
}

impl LootItem {
    /// 由展示名与语义构造。
    pub fn new(label: impl Into<String>, kind: PickupKind) -> Self {
        Self {
            label: label.into(),
            kind,
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
    /// 是否占用背包格位：弹药直接入弹药池、武器直接换手，二者均不占格。
    pub fn occupies_slot(self) -> bool {
        matches!(
            self,
            PickupKind::Health { .. } | PickupKind::Armor { .. } | PickupKind::Grenade { .. }
        )
    }

    /// 速用类别（弹药/武器不可速用，返回 `None`）。
    pub fn category(self) -> Option<ItemCategory> {
        match self {
            PickupKind::Health { .. } | PickupKind::Armor { .. } => Some(ItemCategory::Consumable),
            PickupKind::Grenade { .. } => Some(ItemCategory::Tactical),
            _ => None,
        }
    }
}

/// 玩家背包（4×3 格位容器）。
pub struct Backpack {
    pub slots: Vec<Option<LootItem>>,
}

impl Backpack {
    /// 空背包。
    pub fn new() -> Self {
        Self {
            slots: vec![None; BACKPACK_SLOTS],
        }
    }

    /// 开局携带：两只医疗包 + 两颗手雷（对齐 legacy 起始消耗品）。
    pub fn starting() -> Self {
        let mut bp = Self::new();
        let _ = bp.push(LootItem::new("医疗包", PickupKind::Health { amount: 50.0 }));
        let _ = bp.push(LootItem::new("医疗包", PickupKind::Health { amount: 50.0 }));
        let _ = bp.push(LootItem::new("烈焰手雷", PickupKind::Grenade { element: ElementType::Fire }));
        let _ = bp.push(LootItem::new("烈焰手雷", PickupKind::Grenade { element: ElementType::Fire }));
        bp
    }

    /// 第一个空格下标；背包已满返回 `None`。
    pub fn first_empty(&self) -> Option<usize> {
        self.slots.iter().position(|s| s.is_none())
    }

    /// 统计某速用类别的物品数（HUD 3/4 计数）。
    pub fn count_category(&self, category: ItemCategory) -> i32 {
        self.slots
            .iter()
            .flatten()
            .filter(|it| it.kind.category() == Some(category))
            .count() as i32
    }

    /// 取出第一个指定类别的物品（3/4 速用消费）。
    pub fn take_first_of(&mut self, category: ItemCategory) -> Option<LootItem> {
        let idx = self
            .slots
            .iter()
            .position(|s| s.as_ref().map(|it| it.kind.category()) == Some(Some(category)))?;
        self.slots[idx].take()
    }

    /// 放入一格（取第一个空格）；满则返回 false 且不改变内容。
    pub fn push(&mut self, item: LootItem) -> bool {
        match self.first_empty() {
            Some(i) => {
                self.slots[i] = Some(item);
                true
            }
            None => false,
        }
    }

    /// 取出指定格（空返回 None）。公开给"按格位速用"（3/4 号消耗品）与逐格转移。
    pub fn take_at(&mut self, index: usize) -> Option<LootItem> {
        self.slots.get_mut(index).and_then(|s| s.take())
    }

    /// 把物品放回指定格（转移失败回滚用）。
    fn put_back(&mut self, index: usize, item: LootItem) {
        if let Some(s) = self.slots.get_mut(index) {
            *s = Some(item);
        }
    }
}

impl Default for Backpack {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Backpack {
    fn name(&self) -> &'static str {
        "Backpack"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// 场景物资箱的战利品格位（4×3 容器）。
pub struct Container {
    pub slots: Vec<Option<LootItem>>,
}

impl Container {
    /// 空容器。
    pub fn new() -> Self {
        Self {
            slots: vec![None; CONTAINER_SLOTS],
        }
    }

    /// 按固定掉落池生成一个物资箱战利品（确定性：以箱序号轮转池位，无随机依赖）。
    ///
    /// 设计动机（Why）：服务端权威要求同一份地图布局产生可复现的战局，不引入
    /// 真随机源；以 `seed` 轮转固定池即可让不同箱子内容互不相同且完全确定。
    pub fn rolled(seed: usize) -> Self {
        let mut ct = Self::new();
        for i in 0..CONTAINER_SLOTS {
            ct.slots[i] = Some(pool_item((i + seed) % POOL.len()));
        }
        ct
    }

    /// 第一个空格下标；箱内已满返回 `None`。
    pub fn first_empty(&self) -> Option<usize> {
        self.slots.iter().position(|s| s.is_none())
    }

    /// 放入一格（取第一个空格）；满则返回 false。
    pub fn push(&mut self, item: LootItem) -> bool {
        match self.first_empty() {
            Some(i) => {
                self.slots[i] = Some(item);
                true
            }
            None => false,
        }
    }

    /// 取出指定格（空返回 None）。
    fn take(&mut self, index: usize) -> Option<LootItem> {
        self.slots.get_mut(index).and_then(|s| s.take())
    }

    /// 把物品放回指定格（转移失败回滚用）。
    fn put_back(&mut self, index: usize, item: LootItem) {
        if let Some(s) = self.slots.get_mut(index) {
            *s = Some(item);
        }
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for Container {
    fn name(&self) -> &'static str {
        "Container"
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
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

/// 一次转移的裁决结果。
pub enum TransferResult {
    /// 已在两侧格位间移动（文本回执）。
    Moved(String),
    /// 需调用方施加即时效果（弹药入池 / 武器换手），文本回执。
    Apply(PickupKind, String),
    /// 被拒绝（满格 / 空格 / 越界），文本回执。
    Rejected(String),
}

/// 权威裁决一次格位转移（不触碰战斗数值，即时效果交由调用方按 [`TransferResult`] 施加）。
pub fn transfer(
    bp: &mut Backpack,
    ct: &mut Container,
    dir: TransferDir,
    index: usize,
) -> TransferResult {
    match dir {
        TransferDir::Take => {
            let Some(item) = ct.take(index) else {
                return TransferResult::Rejected("该格为空".to_string());
            };
            let label = item.label.clone();
            if item.kind.occupies_slot() {
                // 先探空位再移动：满则把物品放回原格、不消耗（避免"已移走才失败"）。
                if bp.first_empty().is_some() {
                    let _ = bp.push(item);
                    TransferResult::Moved(format!("已取走：{label}"))
                } else {
                    ct.put_back(index, item);
                    TransferResult::Rejected("背包已满".to_string())
                }
            } else {
                TransferResult::Apply(item.kind, format!("已取走：{label}"))
            }
        }
        TransferDir::Put => {
            let Some(item) = bp.take_at(index) else {
                return TransferResult::Rejected("该格为空".to_string());
            };
            let label = item.label.clone();
            if ct.first_empty().is_some() {
                let _ = ct.push(item);
                TransferResult::Moved(format!("已放回：{label}"))
            } else {
                bp.put_back(index, item);
                TransferResult::Rejected("箱内已满".to_string())
            }
        }
    }
}

/// 物资箱战利品掉落池（对齐 legacy `supply_crate` 的 12 项候选）。
const POOL: [fn() -> LootItem; 12] = [
    || LootItem::new("步枪弹药 ×60", PickupKind::Ammo { amount: 60 }),
    || LootItem::new("步枪弹药 ×60", PickupKind::Ammo { amount: 60 }),
    || LootItem::new("医疗包", PickupKind::Health { amount: 25.0 }),
    || LootItem::new("医疗包", PickupKind::Health { amount: 25.0 }),
    || LootItem::new("大型医疗包", PickupKind::Health { amount: 50.0 }),
    || LootItem::new("护甲片", PickupKind::Armor { amount: 20.0 }),
    || LootItem::new("烈焰手雷", PickupKind::Grenade { element: ElementType::Fire }),
    || LootItem::new("冰霜手雷", PickupKind::Grenade { element: ElementType::Ice }),
    || LootItem::new("雷电手雷", PickupKind::Grenade { element: ElementType::Electric }),
    || LootItem::new("毒素手雷", PickupKind::Grenade { element: ElementType::Poison }),
    || LootItem::new("烈焰步枪", PickupKind::Weapon { element: ElementType::Fire }),
    || LootItem::new("冰霜步枪", PickupKind::Weapon { element: ElementType::Ice }),
];

/// 取掉落池第 `i` 项并克隆为一件物品。
fn pool_item(i: usize) -> LootItem {
    POOL[i % POOL.len()]()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 取弹药不占格（返回 Apply），且容器该格被清空。
    #[test]
    fn take_ammo_does_not_occupy_slot() {
        let mut bp = Backpack::new();
        let mut ct = Container::new();
        ct.slots[0] = Some(LootItem::new("步枪弹药 ×60", PickupKind::Ammo { amount: 60 }));

        let r = transfer(&mut bp, &mut ct, TransferDir::Take, 0);
        assert!(matches!(r, TransferResult::Apply(PickupKind::Ammo { .. }, _)));
        assert!(ct.slots[0].is_none());
        assert!(bp.first_empty() == Some(0), "弹药不占背包格");
    }

    /// 取医疗包占一格；背包满时拒绝且把物品放回原格（不消耗）。
    #[test]
    fn take_health_respects_capacity() {
        let mut bp = Backpack::new();
        for _ in 0..BACKPACK_SLOTS {
            assert!(bp.push(LootItem::new("护甲片", PickupKind::Armor { amount: 20.0 })));
        }
        let mut ct = Container::new();
        ct.slots[0] = Some(LootItem::new("医疗包", PickupKind::Health { amount: 50.0 }));

        let r = transfer(&mut bp, &mut ct, TransferDir::Take, 0);
        assert!(matches!(r, TransferResult::Rejected(_)));
        assert!(ct.slots[0].is_some(), "拒绝后物品应回到容器原格");
    }

    /// 放回：背包格 → 容器空位，背包该格清空。
    #[test]
    fn put_moves_back_to_container() {
        let mut bp = Backpack::new();
        let mut ct = Container::new();
        bp.slots[3] = Some(LootItem::new("烈焰手雷", PickupKind::Grenade { element: ElementType::Fire }));

        let r = transfer(&mut bp, &mut ct, TransferDir::Put, 3);
        assert!(matches!(r, TransferResult::Moved(_)));
        assert!(bp.slots[3].is_none());
        assert!(ct.first_empty() == Some(1), "应放入容器第一个空位");
    }

    /// 速用类别计数与首个取出。
    #[test]
    fn category_count_and_take() {
        let mut bp = Backpack::starting();
        assert_eq!(bp.count_category(ItemCategory::Consumable), 2);
        assert_eq!(bp.count_category(ItemCategory::Tactical), 2);

        let item = bp.take_first_of(ItemCategory::Tactical).expect("应有手雷");
        assert!(matches!(item.kind, PickupKind::Grenade { .. }));
        assert_eq!(bp.count_category(ItemCategory::Tactical), 1);
    }

    /// 轮转生成的容器内容确定且各不相同。
    #[test]
    fn rolled_container_is_deterministic() {
        let a = Container::rolled(0);
        let b = Container::rolled(0);
        let c = Container::rolled(1);
        assert_eq!(a.slots, b.slots);
        assert_ne!(a.slots, c.slots);
        assert!(a.slots.iter().all(|s| s.is_some()));
    }
}
