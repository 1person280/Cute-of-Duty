# Contract.lib · module.md

> 契约 crate（`ContractCode/`，crate `cute_of_duty_contract`）。分层定位：**契约层**。客户端与服务端**唯一的共同依赖**，是 `docs/contracts/*.yaml` 的 Rust 实现，承载两端必须一致的**纯数据**。见 [ADR 0003](../adr/0003-contract-crate.md)。

> `ContractCode` 为单 crate、无子模块层，故按 B 规范取 `lib` 作模块名（`docs/module/Contract.lib.md`）。

## 边界

**本模块负责**：
- **线格式**（`net`）：256B 主通道 + 4096B 资源通道、指令二进制编解码（`codec`）、组包调度（`scheduler`）、资源分片重组（`stream`）、`protocol` 消息类型、`packet` 包帧。
- **跨域载荷类型与共享常量**：`map`（`MapLayout` / `StationKind` / `MaterialKind`…）、`model`（`VoxelModelSpec` / `ModelPreset`…）、`element`、`equipment`（`EquipmentElement` / `EquipmentTier` / `EquipmentType`）、`items`（`LootItem` / `ItemCategory` / `BACKPACK_SLOTS`…）、`interact`（`InteractChoice` / `InteractInfo` / `INTERACT_RANGE`…）、`operator`（`OperatorDef` / `SkillKind`）。
- **同源战斗常数**（`combat`）：客户端表现需与服务端一致的手雷弹道常数。
- **能力抽象**（`port`）：`SnapshotSource` 等 `XxxPort` Trait，实现可替换、可 mock。

**本模块不负责**（硬性边界，违反即破坏"拆微服务无需重写"的前提）：
- **零 bevy、零模拟逻辑、零 I/O**（无 `std::fs`、无网络、无随机、无时间）。
- **依赖仅** `serde` / `serde_json`。
- **只被依赖**，不依赖 `HostCode` 或 `ServerCode`。

## 数据所有权

| 数据 | 类型 | 是否可变 | 谁可写 |
|---|---|---|---|
| 线格式 / 载荷类型 / 常量 | `pub struct` / `pub enum` / `const` | 不可变（纯数据） | 无（编译期定义） |
| Port Trait | `pub trait` | 接口契约 | 实现方在各自 crate |

> **所有权原则**：契约层**零状态**——只定义类型与常量，不持有任何运行时数据；权威源是 `docs/contracts/*.yaml`（当前线格式权威为代码，YAML 反转为后续独立任务）。

## 接口

| 接口 | 形式 | 契约 | 兼容性承诺 |
|---|---|---|---|
| `net::{protocol, packet, codec, scheduler, stream}` | 模块 | 线格式类型与编解码 | **L2 线格式契约**，见 [protocol.yaml](../contracts/protocol.yaml) |
| `map` / `model` / `element` / `equipment` / `items` / `interact` / `operator` | 模块 | 跨域载荷类型 + 共享常量 | 两端共同依赖，破坏性变更走 L2 |
| `combat` | 模块 | 同源战斗常数（手雷弹道） | 两端同源 |
| `port::SnapshotSource` | **Trait** | 权威快照来源抽象（`latest_snapshot`） | 破坏性变更须走 L2 流程 |

## 事件

| 事件名 | 方向 | 载荷契约 | 订阅者 |
|---|---|---|---|
| —（本 crate 不产生/消费运行态事件，仅定义类型） | — | — | — |

## 成熟度

**L2（对外契约：被其它 crate / 线格式依赖）** —— 依据：`ContractCode` 定义/承载线格式，被 `HostCode` 与 `ServerCode` 双向依赖（`ServerCode → ContractCode ← HostCode`）。

- 破坏性变更流程：线格式不兼容必附**迁移指南** + [barek-history](../barek-history.md) 条目 + [protocol.yaml](../contracts/protocol.yaml) 同步 + `y+1` 版本号；**≥2 名 reviewer（maintainer 必须参与）**。
- 加性/细节变更：`z+1`，老端须能忽略新字段继续运行。
- 已发生示例：0.13.0 新增 `ServerMessage::PresetCatalog`（`y+1`，`wire_version` 12→13）；0.14.0 新增 `ServerMessage::WorldCatalog`（`y+1`，`wire_version` 13→14）。
