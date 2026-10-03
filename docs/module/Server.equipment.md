# Server.equipment · module.md

> 服务端**局外装备域**（`ServerCode/equipment/`）。分层定位：**领域层 · 局外（out-of-match）**。承载装备实例注册表（索引制）与等级/元素规则系统，跨局落盘。见 [ADR 0002](../adr/0002-backpack-domain-split.md)。

## 边界

**本模块负责**：
- **装备实例注册表**：`EquipmentSystem`——全局 `HashMap<u64, Equipment>` 热注册表 + `TradeRecord` 交易历史 + `seed_counter`（确定性可审计随机）。
- **等级规则**：`TierRules`——1 级无元素（保护舱）；2–6 级真随机元素且可付费指定（成本翻倍，博弈区）；7–9 级真随机且不可指定（混沌区）。
- **生成与转移**：`create_equipment`（按等级裁决元素）、`transfer_equipment`（直接交易/跨账号重随元素、市场购买固定、同账号不变）、`calculate_equipment_value`（含元素相性）。
- **注册表 CRUD**：`register_equipment` / `get_equipment` / `remove_equipment` / `trade_count`。

**本模块不负责**（明确划出，ADR 0002 三条硬边界）：
- **不持手持槽 / 弹夹**——手持武器槽与弹夹权威在 `combat::Combatant`；本模块只持**局外装备实例**。
- **不与 `items` 互相 `use`**——局外注册表与局内格位永不直接调用。
- **不做货币结算**——制作/丢弃/货币由 `inventory` 编排调用（`equipment` 只提供实例 API）。

## 数据所有权

| 数据 | 类型 | 是否可变 | 谁可写 |
|---|---|---|---|
| 装备注册表 | `EquipmentSystem.equipment_registry`（`RwLock<HashMap<u64, Equipment>>`） | 跨局可变 | 仅 `equipment` |
| 交易历史 | `EquipmentSystem.trade_history` | 跨局追加 | 仅 `equipment` |
| 种子计数 | `EquipmentSystem.seed_counter`（`AtomicU64`） | 跨局递增 | 仅 `equipment` |

> **所有权原则**：装备实例是**全局唯一**注册表（跨连接唯一 ID 由 `main.rs` 的分配器 `next_id` 提供）；背包只持 `equipment_ids` 索引，不复制实例。

## 接口

| 接口 | 形式 | 契约 | 兼容性承诺 |
|---|---|---|---|
| `EquipmentSystem::{new, get_tier_rules, create_equipment, transfer_equipment, register_equipment, get_equipment, remove_equipment, calculate_equipment_value, trade_count}` | 方法 | 装备实例与规则 | 局外装备域 |
| `Equipment` | 结构 | 装备实例（id/name/tier/eq_type/element/…） | `serde` 可序列化（落盘） |
| `TierRules` / `TransferType` / `EquipmentError` | 类型 | 等级规则 / 转移类型 / 错误 | 域内 |
| `EquipmentElement` / `EquipmentTier` / `EquipmentType` | 类型 | 权威见 `ContractCode/equipment`；本模块 `pub use` | **契约类型** |

## 事件

| 事件名 | 方向 | 载荷契约 | 订阅者 |
|---|---|---|---|
| —（本模块不产生事件；交易经 `TradeRecord` 内部留痕） | — | — | — |

## 成熟度

**L1（内部稳定：crate 内可依赖）** —— 依据：`equipment` 被 `inventory`/`main.rs` 依赖，跨局落盘；不定义对外线格式。

- 破坏性变更纪律：须写 ADR（触碰 ADR 0002 硬边界时）+ 1 名 reviewer。
