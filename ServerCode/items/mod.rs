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
    /// 注意：`count` 可超过单格上限，表示"若干发待入库"——由 [`Backpack::push`] / [`Container::push`]
    /// 负责按上限**拆成多格堆叠**，因此构造出的物品不保证已是合法单格，落格后必然合法。
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

    /// 开局携带：两只医疗包 + 两颗手雷 + 两叠备用子弹（对齐 legacy 起始消耗品与备弹）。
    ///
    /// 设计动机（Why）：备用子弹不再是抽象弹药池，而是**背包里的可堆叠物品**（每叠上限 64），
    /// 换弹时由服务端按需从背包抽入弹夹——背包格位因而成为备弹的唯一权威存储。
    pub fn starting() -> Self {
        let mut bp = Self::new();
        let _ = bp.push(LootItem::new("医疗包", PickupKind::Health { amount: 50.0 }));
        let _ = bp.push(LootItem::new("医疗包", PickupKind::Health { amount: 50.0 }));
        let _ = bp.push(LootItem::new("烈焰手雷", PickupKind::Grenade { element: ElementType::Fire }));
        let _ = bp.push(LootItem::new("烈焰手雷", PickupKind::Grenade { element: ElementType::Fire }));
        let _ = bp.push(LootItem::with_count("步枪弹药", PickupKind::Ammo { amount: 64 }, 64));
        let _ = bp.push(LootItem::with_count("步枪弹药", PickupKind::Ammo { amount: 64 }, 64));
        bp
    }

    /// 第一个空格下标；背包已满返回 `None`。
    pub fn first_empty(&self) -> Option<usize> {
        self.slots.iter().position(|s| s.is_none())
    }

    /// 统计某速用类别的**总件数**（HUD 3/4 计数按堆叠内数量累加）。
    pub fn count_category(&self, category: ItemCategory) -> i32 {
        self.slots
            .iter()
            .flatten()
            .filter(|it| it.kind.category() == Some(category))
            .map(|it| it.count as i32)
            .sum()
    }

    /// 取出第一个指定类别的**单件**（3/4 速用消费；堆叠递减，取空清格）。
    pub fn take_first_of(&mut self, category: ItemCategory) -> Option<LootItem> {
        let idx = self
            .slots
            .iter()
            .position(|s| s.as_ref().map(|it| it.kind.category()) == Some(Some(category)))?;
        self.take_one_at(idx)
    }

    /// 放入物品（**先并入同类未满堆、再占空格**），返回是否**全部**放入。
    ///
    /// 设计动机（Why）：堆叠语义必须由权威格位宿主统一实现——弹药/消耗品进来时先补满已有
    /// 同类堆（上限 [`PickupKind::max_stack`]），剩余再占新格；放不下则返回 false（调用方据此
    /// 拒收、不消耗来源），保证"背包满"判定与真实格位一致。
    pub fn push(&mut self, item: LootItem) -> bool {
        let max = item.kind.max_stack().max(1);
        let mut remaining = item.count.max(1);
        for slot in self.slots.iter_mut() {
            if remaining == 0 {
                break;
            }
            if let Some(existing) = slot {
                if existing.label == item.label && existing.kind.same_stack_kind(&item.kind) && existing.count < max {
                    let add = (max - existing.count).min(remaining);
                    existing.count += add;
                    remaining -= add;
                }
            }
        }
        for slot in self.slots.iter_mut() {
            if remaining == 0 {
                break;
            }
            if slot.is_none() {
                let add = max.min(remaining);
                *slot = Some(LootItem {
                    label: item.label.clone(),
                    kind: item.kind,
                    count: add,
                });
                remaining -= add;
            }
        }
        remaining == 0
    }

    /// 取出指定格的**整叠**（空返回 None）。供逐格转移/放回使用。
    pub fn take_at(&mut self, index: usize) -> Option<LootItem> {
        self.slots.get_mut(index).and_then(|s| s.take())
    }

    /// 取出指定格的**单件**（堆叠递减，取空清格）。供 3/4 速用按格消费。
    pub fn take_one_at(&mut self, index: usize) -> Option<LootItem> {
        let slot = self.slots.get_mut(index)?;
        let existing = slot.as_mut()?;
        existing.count -= 1;
        let out = LootItem {
            label: existing.label.clone(),
            kind: existing.kind,
            count: 1,
        };
        if existing.count == 0 {
            *slot = None;
        }
        Some(out)
    }

    /// 从背包弹药堆中抽出至多 `want` 发（供换弹时倒入备弹池），返回实际抽出数。
    pub fn draw_ammo(&mut self, want: i32) -> i32 {
        let mut got = 0;
        for slot in self.slots.iter_mut() {
            if got >= want {
                break;
            }
            if let Some(existing) = slot {
                if matches!(existing.kind, PickupKind::Ammo { .. }) {
                    let take = ((want - got) as u32).min(existing.count);
                    existing.count -= take;
                    got += take as i32;
                    if existing.count == 0 {
                        *slot = None;
                    }
                }
            }
        }
        got
    }

    /// 背包内**全部弹药堆**的总发数（备用弹的权威合计）。
    ///
    /// 设计动机（Why）：备用子弹的权威宿主就是背包格位（可堆叠物品）——换弹直接从背包抽弹补满
    /// 弹夹，不经任何中间弹池。HUD 的「备用」由服务端用本方法合计后下发，客户端只显示。
    pub fn ammo_total(&self) -> i32 {
        self.slots
            .iter()
            .flatten()
            .filter(|it| matches!(it.kind, PickupKind::Ammo { .. }))
            .map(|it| it.count as i32)
            .sum()
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

    /// 放入物品（先并入同类未满堆、再占空格）；放不下返回 false。
    pub fn push(&mut self, item: LootItem) -> bool {
        let max = item.kind.max_stack().max(1);
        let mut remaining = item.count.max(1);
        for slot in self.slots.iter_mut() {
            if remaining == 0 {
                break;
            }
            if let Some(existing) = slot {
                if existing.label == item.label && existing.kind.same_stack_kind(&item.kind) && existing.count < max {
                    let add = (max - existing.count).min(remaining);
                    existing.count += add;
                    remaining -= add;
                }
            }
        }
        for slot in self.slots.iter_mut() {
            if remaining == 0 {
                break;
            }
            if slot.is_none() {
                let add = max.min(remaining);
                *slot = Some(LootItem {
                    label: item.label.clone(),
                    kind: item.kind,
                    count: add,
                });
                remaining -= add;
            }
        }
        remaining == 0
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

/// 物资箱战利品掉落池（对齐 legacy `supply_crate` 的候选项，并扩充物品种类）。
///
/// 设计动机（Why）：反馈"物品种类太少"的落点就在这张表——它是场景战利品的**唯一来源**，
/// 扩充此处即可让每个箱子的 4×3 内容更有变化，且不改任何协议（[`PickupKind`] 语义未变）。
/// 六种元素的手雷与步枪各占一项，另加弹药/医疗/护甲档位，兼顾"火力补充"与"元素收集"。
const POOL: [fn() -> LootItem; 20] = [
    || LootItem::with_count("步枪弹药", PickupKind::Ammo { amount: 30 }, 30),
    || LootItem::with_count("步枪弹药", PickupKind::Ammo { amount: 60 }, 60),
    || LootItem::with_count("步枪弹药", PickupKind::Ammo { amount: 90 }, 90),
    || LootItem::new("医疗包", PickupKind::Health { amount: 25.0 }),
    || LootItem::new("大型医疗包", PickupKind::Health { amount: 50.0 }),
    || LootItem::new("急救包", PickupKind::Health { amount: 75.0 }),
    || LootItem::new("护甲片", PickupKind::Armor { amount: 20.0 }),
    || LootItem::new("重型护甲板", PickupKind::Armor { amount: 50.0 }),
    || LootItem::new("烈焰手雷", PickupKind::Grenade { element: ElementType::Fire }),
    || LootItem::new("冰霜手雷", PickupKind::Grenade { element: ElementType::Ice }),
    || LootItem::new("雷电手雷", PickupKind::Grenade { element: ElementType::Electric }),
    || LootItem::new("毒素手雷", PickupKind::Grenade { element: ElementType::Poison }),
    || LootItem::new("破片手雷", PickupKind::Grenade { element: ElementType::Physical }),
    || LootItem::new("水压手雷", PickupKind::Grenade { element: ElementType::Water }),
    || LootItem::new("烈焰步枪", PickupKind::Weapon { element: ElementType::Fire }),
    || LootItem::new("冰霜步枪", PickupKind::Weapon { element: ElementType::Ice }),
    || LootItem::new("雷电步枪", PickupKind::Weapon { element: ElementType::Electric }),
    || LootItem::new("毒素步枪", PickupKind::Weapon { element: ElementType::Poison }),
    || LootItem::new("制式步枪", PickupKind::Weapon { element: ElementType::Physical }),
    || LootItem::new("潮汐步枪", PickupKind::Weapon { element: ElementType::Water }),
];

/// 取掉落池第 `i` 项并克隆为一件物品。
fn pool_item(i: usize) -> LootItem {
    POOL[i % POOL.len()]()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 备用子弹是**可堆叠的背包物品**：取走后入格（占位）、同类再取会并入同一堆（上限 64）。
    #[test]
    fn take_ammo_occupies_slot_and_stacks() {
        let mut bp = Backpack::new();
        let mut ct = Container::new();
        ct.slots[0] = Some(LootItem::with_count("步枪弹药", PickupKind::Ammo { amount: 60 }, 60));

        let r = transfer(&mut bp, &mut ct, TransferDir::Take, 0);
        assert!(matches!(r, TransferResult::Moved(_)), "弹药应入背包格（不再直接入池）");
        assert!(ct.slots[0].is_none());
        assert_eq!(bp.slots[0].as_ref().map(|i| i.count), Some(60), "第 0 格应为 60 发弹药");

        // 再取一叠 30 发：应并入第 0 格（60+30=90 超过上限 64 → 拆为 64 + 26 两格）。
        ct.slots[1] = Some(LootItem::with_count("步枪弹药", PickupKind::Ammo { amount: 30 }, 30));
        let _ = transfer(&mut bp, &mut ct, TransferDir::Take, 1);
        assert_eq!(bp.slots[0].as_ref().map(|i| i.count), Some(64), "先补满到上限 64");
        assert_eq!(bp.slots[1].as_ref().map(|i| i.count), Some(26), "溢出部分另占一格");
    }

    /// 取医疗包占一格；背包满时拒绝且把物品放回原格（不消耗）。
    #[test]
    fn take_health_respects_capacity() {
        let mut bp = Backpack::new();
        // 工具不可堆叠：用 12 把武器把 12 格各占 1 格（堆叠上限 1，不会并堆）。
        for _ in 0..BACKPACK_SLOTS {
            assert!(bp.push(LootItem::new("制式步枪", PickupKind::Weapon { element: ElementType::Fire })));
        }
        let mut ct = Container::new();
        ct.slots[0] = Some(LootItem::new("医疗包", PickupKind::Health { amount: 50.0 }));

        let r = transfer(&mut bp, &mut ct, TransferDir::Take, 0);
        assert!(matches!(r, TransferResult::Rejected(_)));
        assert!(ct.slots[0].is_some(), "拒绝后物品应回到容器原格");
    }

    /// 堆叠上限：可堆叠类先补堆再占格，工具不可堆叠；换弹抽弹按发数递减。
    #[test]
    fn stack_limits_and_draw_ammo() {
        assert_eq!(PickupKind::Ammo { amount: 1 }.max_stack(), 64);
        assert_eq!(PickupKind::Health { amount: 1.0 }.max_stack(), 16);
        assert_eq!(PickupKind::Weapon { element: ElementType::Ice }.max_stack(), 1);

        let mut bp = Backpack::new();
        assert!(bp.push(LootItem::with_count("步枪弹药", PickupKind::Ammo { amount: 64 }, 64)));
        assert!(bp.push(LootItem::with_count("步枪弹药", PickupKind::Ammo { amount: 64 }, 64)));
        assert_eq!(bp.draw_ammo(70), 70, "应抽出 70 发");
        assert!(bp.slots[0].is_none(), "首格应被抽空");
        assert_eq!(bp.slots[1].as_ref().map(|i| i.count), Some(58), "次格剩 58 发");
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
