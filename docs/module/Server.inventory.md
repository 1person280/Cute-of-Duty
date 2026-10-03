# Server.inventory · module.md

> 服务端**局外经济域**（`ServerCode/inventory/`）。分层定位：**领域层 · 局外（out-of-match）**。承载货币 / 制作 / 丢弃的 CRUD 服务与一致性守卫，跨局落盘。见 [ADR 0002](../adr/0002-backpack-domain-split.md)。

## 边界

**本模块负责**：
- **制作**：`craft(eqsys, profile, next_id, CraftRequest)`——按 `EquipmentSystem::create_equipment` 的等级规则裁决元素，先入全局注册表再加背包，背包满则**回滚注册表**。
- **丢弃**：`discard_by_index(eqsys, profile, index)`——先移除全局注册表，失败则背包不动（防引用悬空实例）。
- **货币**：`adjust_currency(profile, soft, hard, season)`——任一币种扣为负则**整笔拒绝**（不半生半熟）。
- **错误语义**：`InventoryError`（`SlotsFull` / `IndexOutOfBounds` / `InsufficientFunds` / `EquipmentMissing`）。

**本模块不负责**（明确划出，ADR 0002 三条硬边界）：
- **不含战局内格位**——`Backpack` / `Container` / `transfer` 属 `items`（局内域）；本模块**永不引入**"战局内格位"语义。
- **不与 `items` 互相 `use`**——只在 `main.rs` 编排下经事件回写接起来。
- **不感知线格式**——`InventoryAction` 由 `main.rs` 映射为 `CraftRequest`，域不依赖协议。

## 数据所有权

| 数据 | 类型 | 是否可变 | 谁可写 |
|---|---|---|---|
| 货币与背包引用 | `PlayerProfile.inventory`（`Currency` / `equipment_ids` / `used_slots`） | 跨局可变 | 仅 `inventory`（经 `PlayerProfile` 热副本） |
| 装备实例 | `EquipmentSystem` 注册表 | 跨局可变 | `craft`/`discard` 经 `equipment` API |

> **所有权原则**：冷档案热副本由 `player` 持、`storage` 落盘；`inventory` 只在其上做守卫式 CRUD，所有写操作带回滚语义，保证与 Disconnect 落盘口径一致。

## 接口

| 接口 | 形式 | 契约 | 兼容性承诺 |
|---|---|---|---|
| `craft(...) -> Result<Equipment, InventoryError>` | 函数 | 制作并入背包（带回滚） | 局外经济域 |
| `discard_by_index(...) -> Result<(), InventoryError>` | 函数 | 按下标丢弃（先移注册表） | 局外经济域 |
| `adjust_currency(...) -> Result<(), InventoryError>` | 函数 | 货币增减（整笔拒绝守卫） | 局外经济域 |
| `CraftRequest` | 结构 | 域内制作请求（从 `InventoryAction` 映射） | 不依赖线格式 |
| `InventoryError` | 枚举 | 领域错误 | `Display` + `Error` |

## 事件

| 事件名 | 方向 | 载荷契约 | 订阅者 |
|---|---|---|---|
| —（本模块不产生事件；结算经 `main.rs` 以 Event 回执播报） | — | — | — |

## 成熟度

**L1（内部稳定：crate 内可依赖）** —— 依据：`inventory` 被 `main.rs` 编排调用，持有跨局经济一致性守卫；不定义对外线格式。

- 破坏性变更纪律：须写 ADR（尤其触碰 ADR 0002 三条硬边界时）+ 1 名 reviewer。
