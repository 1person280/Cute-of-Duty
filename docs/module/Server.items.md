# Server.items · module.md

> 服务端**局内物资域**（`ServerCode/items/`）。分层定位：**领域层 · 局内（in-match）**。承载可携带物品、背包格位、容器格位与逐格转移裁决。见 [ADR 0002](../adr/0002-backpack-domain-split.md)。

## 边界

**本模块负责**：
- **格位容器**：`Backpack`（4×3 = [`BACKPACK_SLOTS`]）与 `Container`（4×3 = [`CONTAINER_SLOTS`]）的格位内容、堆叠语义（先并入同类未满堆、再占空格，上限 `PickupKind::max_stack`）。
- **权威转移**：`transfer(bp, ct, dir, index)`——逐格取/放裁决，失败回滚（满格/空格/越界拒绝且不消耗来源）。
- **开局携带与速用**：`Backpack::starting()`（2 医疗包 + 2 手雷 + 2×64 弹药）；`count_category` / `take_first_of` / `take_one_at`（3/4 速用与径向轮盘按格消费）。
- **备弹权威**：备用子弹是**可堆叠背包物品**，`ammo_total()` / `draw_ammo()` 是备弹的唯一权威存储与抽取入口。
- **掉落池**：`Container::rolled(seed)` 以箱序号轮转固定 [`POOL`]（确定性、无真随机），`catalog` / `presets` 提供名称解析与预设。

**本模块不负责**（明确划出，ADR 0002 三条硬边界）：
- **不与 `inventory` / `equipment` 互相 `use`**——局内格位与局外经济/装备注册表永不直接调用。
- **不触碰战斗数值**——"弹药入池 / 武器换手"这类即时效果由调用方（主循环）按 `TransferResult::Apply` 施加，从而杜绝 `items ↔ combat` 循环依赖。
- **不落盘**——局内数据只活一局。

## 数据所有权

| 数据 | 类型 | 是否可变 | 谁可写 |
|---|---|---|---|
| 玩家背包格位 | `Backpack`（`Component`） | 一局可变 | 仅 `items`（宿主挂在玩家实体上） |
| 物资箱格位 | `Container`（`Component`） | 一局可变 | 仅 `items`（`spawn_from_layout` 挂载） |
| 掉落池 | `POOL` | 静态 | 编译期常量 |

> **所有权原则**：`items` 与 `combat` 均为局内域——`combat` 持手持槽/弹夹，`items` 持背包/容器格位（备弹权威在 `Backpack`）；二者由 `main.rs` 编排，不互相 `use`。

## 接口

| 接口 | 形式 | 契约 | 兼容性承诺 |
|---|---|---|---|
| `Backpack::{new, starting, push, take_at, take_one_at, draw_ammo, ammo_total, count_category, take_first_of, first_empty}` | 方法 | 格位增删查与备弹抽取 | 局内域内 |
| `Container::{new, rolled, push, first_empty}` | 方法 | 容器生成与放置 | 局内域内 |
| `transfer(bp, ct, dir, index) -> TransferResult` | 函数 | 权威逐格转移裁决 | 纯逻辑，无网络感知 |
| `TransferResult::{Moved, Apply, Rejected}` | 枚举 | 已移动 / 需调用方施加即时效果 / 拒绝 | 即时效果边界 |
| `BACKPACK_SLOTS` / `CONTAINER_SLOTS` / `ItemCategory` / `LootItem` / `TransferDir` | 类型 | 权威见 `ContractCode/items`；本模块 `pub use` | **契约类型（线格式）** |

## 事件

| 事件名 | 方向 | 载荷契约 | 订阅者 |
|---|---|---|---|
| —（本模块不产生事件；被主循环编排） | — | — | — |

> 交互/转移的上行经 `interact::settle` 与 `main.rs` 编排；`items` 自身无可订阅事件。

## 成熟度

**L0（试验田：结构随时推翻）** —— 依据：`items` 为本轮新增的局内物资域（对应 [module-boundaries 第三节](../architecture/module-boundaries.md) 标注「L0（本轮新增）」）。

- 变更纪律：随便改，无 reviewer 要求。
- 稳定后（如跨局也依赖其语义）再按流程升 L1。
