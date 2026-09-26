# ADR 0002 · 背包域拆分：`items` 拥有战局内格位，`inventory` 收敛为局外经济

- **状态**：已接受（Accepted）
- **日期**：2026-09-26
- **决策者**：项目 owner
- **影响范围**：`ServerCode/items`、`ServerCode/inventory`、`ServerCode/equipment`、`ServerCode/combat`、`ServerCode/main.rs`（编排）
- **依据**：[ADR 0001](0001-modular-monolith-event-bus.md) 铁律 2「数据所有权唯一」；[module-boundaries.md 冲突 1](../architecture/module-boundaries.md)

---

## 背景（Context）

仓库里同时存在两套"背包"，都声称拥有物品数据：

| | `items`（本轮新增） | `inventory`（既有） |
|---|---|---|
| 模型 | **格位制**：`Backpack{slots:12}` / `Container{slots:12}` / `transfer()` | **索引制 + `equipment` 注册表**：`adjust_currency` / `craft` / `discard_by_index` |
| 作用域 | 战局内（一局的生命周期） | 局外档案与经济 |
| 生命周期 | 随战局创建/销毁 | 落盘到 `storage` 冷仓库 |
| 已知消费方 | `combat::use_item_at`、`interact::settle` | 无（当前为孤立域服务） |

另有第二处重叠：`equipment` 持"装备热实例注册表"，而 `combat/combatant.rs` 持 `WEAPON_SLOTS` / `DEFAULT_AMMO_POOL`（手持槽 / 弹夹）。两者都涉及"武器"。

按 ADR 0001 铁律 2，同一份可变状态不得有两个 owning 模块。必须裁决。

## 决策（Decision）

采用**方案 A：按域切开**。

### 1. 轴心：**局内（in-match） vs 局外（out-of-match）**

| 归属 | 模块 | 拥有的数据 | 生命周期 | 是否落盘 |
|---|---|---|---|---|
| **局内** | `items` | `Backpack{slots:12}`、`Container{slots:12}`、`LootItem`、`POOL` | 一局 | ❌ 不落盘 |
| **局内** | `combat` | 手持武器槽（`WEAPON_SLOTS`）、弹夹 / `DEFAULT_AMMO_POOL` | 一局 | ❌ 不落盘 |
| **局外** | `inventory` | 货币、制作、丢弃等**经济操作**及其一致性守卫 | 跨局 | ✅ 落 `storage` |
| **局外** | `equipment` | 装备实例注册表（等级 / 元素 / 归属） | 跨局 | ✅ 落 `storage` |

### 2. 三条硬边界

1. **`items` 不依赖 `inventory` / `equipment`，反之亦然。** 两者永不互相 `use`。
2. **`inventory` 不得再引入任何"战局内格位"语义**（格位是 `items` 的独占职责）。
3. **`equipment` 不得持有"手持/弹夹"**（那是 `combat` 的战局内状态）；`combat` 不得持有"装备等级/元素"（那是 `equipment` 的局外状态）。

### 3. 唯一编排者与两条数据流

`ServerCode/main.rs` 是唯一编排者，跨域同步只走它：

```
入局：equipment（局外装备）--(main.rs 选取/装配)--> combat 手持槽 + items 初始背包
局内：items/combat 的变化【不回写】局外
撤离/结算：combat|items --(事件)--> main.rs --(调用)--> inventory/equipment 落盘
```

- 局内变更**不得**直接回写 `inventory` / `equipment`（违反铁律 2）。
- 唯一的跨域写回路径是**结算事件**，由 `main.rs` 消费后调用局外模块的接口。

### 4. 接口与所有权

| 模块 | 对外接口（形式） | 谁可写其数据 |
|---|---|---|
| `items` | `transfer(bp, ct, dir, index) -> TransferResult`、`Backpack` / `Container` 的类型与只读访问器 | 仅 `items` 自身 + `main.rs` 编排 |
| `inventory` | `adjust_currency` / `craft` / `discard_by_index`、`InventoryError`、`CraftRequest` | 仅 `inventory` 自身 + `main.rs` 编排 |
| `equipment` | 注册 / 查询 | 仅 `equipment` 自身 + `main.rs` 编排 |
| `combat` | `CombatIntent`、`use_item_at` | 仅 `combat` 自身 + `main.rs` 编排 |

> 上述"仅 `main.rs` 编排"是**过渡期**形态。终局形态是：`items` ↔ `combat` 之间也改由**事件**通信（`combat.item_applied`、`items.transfer_committed`），`main.rs` 只做装配。

### 5. 迁移路径（**代码暂停期间只落文档，不实施**）

1. 补 `ServerCode/items/module.md`、`ServerCode/inventory/module.md`、`ServerCode/equipment/module.md`，写明上表。
2. 在 `inventory/module.md` 的「边界」节显式写"**不负责战局内格位**"。
3. 在 `equipment/module.md` 的「边界」节显式写"**不负责手持/弹夹**"。
4. 在 `combat/module.md` 的「边界」节显式写"**不负责装备等级/元素**"。
5. 把 `docs/contracts/` 补 `items.yaml`（格位载荷契约），与 `protocol.yaml` 的 `backpack` / `container` 字段对齐。
6. 上述完成后，才允许在 `items` / `inventory` 上新增代码。

## 后果（Consequences）

**正向**
- 两套背包不再竞争"所有权"，`items`（局内格位）与 `inventory`（局外经济）各自边界清晰。
- 顺带解决了 `equipment` ↔ `combat` 的武器重叠。
- 将来拆微服务时，局内/局外天然是两个可独立部署的域。

**代价**
- `inventory` 当前是"孤立域服务"（无消费方），拆分后它更需要一个明确的入局/结算接线点 —— 该接线点在 `main.rs`，需在实施时补上。
- `equipment` → `combat` 的入局装配逻辑目前是隐式的，需显式化。

## 未决事项（Open Questions）

- 撤离结算的具体事件名与载荷（待 `docs/architecture/module-boundaries.md` 第六节事件清单补齐后定）。
- `equipment` 的"元素"是否也应参与局内伤害结算（当前 `element` 模块独立负责规则）—— 需另一条 ADR。

## 替代方案（Alternatives considered）

- **方案 B（`items` 并入 `inventory`）** — 否决：`inventory` 是**落盘的局外经济**，`items` 是**易失的局内格位**，
  合并会把"落盘 vs 不落盘""一局 vs 跨局"两种生命周期混进一个模块，是新的边界污染。
- **方案 C（双轨 + Trait 定权威）** — 否决：两套模型长期并存，边界模糊，正是当前问题的来源；Trait 只能掩盖而不能消除所有权冲突。
