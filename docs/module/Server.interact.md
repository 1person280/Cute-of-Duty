# Server.interact · module.md

> 服务端**交互域**（`ServerCode/interact/`）。分层定位：**领域层 · 局内（in-match）**。"按 F 能拿到什么"（弹药、回血、切干员、箱子内容）全部是服务端职责，客户端只上报意图（对哪个目标、选了哪一项）。

## 边界

**本模块负责**：
- **交互语义载体**：`Interactable` 组件（挂在 Loot / Station 实体上，声明"是什么、叫什么"）+ `info()` 生成随快照下行的 `InteractInfo`。
- **实体落成**：`spawn_from_layout`——把地图数据（`map::lawn`）的拾取物 / 功能站点落成权威实体；物资箱额外挂 `Container::rolled(seed)`（确定性掉落）。
- **权威结算**：`settle(world, player, target, choice)`——**平面距离校验**（`INTERACT_RANGE`，忽略 Y）+ 效果发放 + 是否消耗目标。
- **效果发放**：`apply_pickup`（武器装当前手持槽不换干员；消耗品入背包、满则拒收不消耗）、`grant_supply`（补给台弹药入背包 / 医疗回血 / 护甲加甲，钳制上限）。
- **线格式类型**：`InteractChoice` / `InteractInfo` / `InteractKind` / `SupplyKind` / `INTERACT_RANGE`（`pub use` 自 `ContractCode/interact`）。

**本模块不负责**（明确划出）：
- **不裁决战斗数值**——回血/加甲经 `combat` API，元素/弹道归 `combat`。
- **不持格位内容**——箱子内容在 `items::Container`，逐格转移见 `items::transfer`；本模块只"打开面板"。
- **不重复地图定义**——地图是数据（`map`），实体是运行态，二者在此对接，保持单一来源。

## 数据所有权

| 数据 | 类型 | 是否可变 | 谁可写 |
|---|---|---|---|
| 交互标记 | `Interactable`（`Component`） | 生成期 | 仅 `interact::spawn_from_layout` |
| 交互目标/结算 | 世界实体 + 结算回执 | 每次交互 | 仅 `interact::settle` |
| 线格式类型 | `pub use ContractCode/interact` | — | 只读导出 |

> **所有权原则**：交互目标语义（标签 + 种类）归 `interact`；具体物品/格位归 `items`、数值归 `combat`，本模块只做距离校验与效果编排。

## 接口

| 接口 | 形式 | 契约 | 兼容性承诺 |
|---|---|---|---|
| `spawn_from_layout(world, layout) -> (usize, usize)` | 函数 | 地图数据 → 权威实体 | 启动时调用 |
| `settle(world, player, target, choice) -> (String, bool)` | 函数 | 权威结算一次交互 | 被 `net::stages` 调用 |
| `Interactable::{new, info}` | 方法 | 交互标记与快照信息 | 域内 |
| `InteractChoice` / `InteractInfo` / `InteractKind` / `SupplyKind` / `INTERACT_RANGE` | 类型 | 权威见 `ContractCode/interact`；本模块 `pub use` | **契约类型（含线格式）** |

## 事件

| 事件名 | 方向 | 载荷契约 | 订阅者 |
|---|---|---|---|
| `InteractChoice`（上行，消费） | 客户端 → `interact::settle` | 选择项（Take / Supply / OpenCrate） | `net::stages` 编排 |
| `InteractInfo`（下行） | `interact` → 快照 | 目标标签 + 种类 | 客户端 HUD |

## 成熟度

**L1 → L2 待定（定级前按 L2 流程）** —— 依据：[module-boundaries 第三节](../architecture/module-boundaries.md) 明确标注 `interact` 为「L1→L2 待定」，其交互语义/线格式类型经 `ContractCode` 承载。

- **定级前一律按 L2 处理**：破坏性变更须**迁移指南** + 契约 YAML 同步 + `y+1` 版本号 + **≥2 名 reviewer（maintainer 必须参与）**，见 [CONTRIBUTING 第九节](../../CONTRIBUTING.md)。
- 定级完成后回写本节与 module-boundaries。
