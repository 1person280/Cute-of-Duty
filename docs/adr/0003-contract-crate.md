# ADR 0003 · 抽出独立契约 crate（`cute_of_duty_contract`）

- **状态**：已接受（Accepted）
- **日期**：2026-09-26
- **决策者**：项目 owner
- **影响范围**：workspace 根 `Cargo.toml`、新增 `ContractCode`、`ServerCode/net`、`ServerCode/{items,interact,map,model,operator}`、`HostCode/*`（全部跨 crate 引用点）
- **依据**：[ADR 0001](0001-modular-monolith-event-bus.md) 铁律 1「禁跨模块直接调用」、铁律 5「契约机器可读」；[module-boundaries.md 冲突 2](../architecture/module-boundaries.md)

---

## 背景（Context）

当前 workspace 只有两个成员 crate：`HostCode`（客户端）与 `ServerCode`（服务端）。
客户端为了拿到线格式与共享枚举，直接依赖**服务端 crate 的内部类型**：

```rust
// HostCode 内的现状（违规示例）
use cute_of_duty_server::items::{ItemCategory, LootItem};
use cute_of_duty_server::interact::{InteractKind, SupplyKind};
use cute_of_duty_server::map::StationKind;
use cute_of_duty_server::operator::roster;
use cute_of_duty_server::net::protocol::*;
```

后果：
1. **违反铁律 1**：客户端直接调用服务端，而没有经过契约。
2. **无兼容承诺**：服务端任意内部重构（如 `EntitySnapshot` 删 `medkit`/`grenade`）都会**直接打断客户端编译** —— 这就是 0.8-Snapshot-8 现在编译失败的原因。
3. **"拆微服务无需重写"不可达**：两边共享一个 crate 的类型定义，物理上永远无法独立部署。
4. **契约 YAML 名不副实**：`docs/contracts/protocol.yaml` 只是文档，代码里的真定义在 `ServerCode/net/protocol.rs`。

## 决策（Decision）

**抽出一个三方共享的契约 crate，作为两端唯一的共同依赖。**

```
cute_of_duty_contract   （新，零 bevy、零模拟逻辑）
        ▲                    ▲
        │                    │
   ServerCode            HostCode
        └──── 不再存在 ────────┘   （HostCode 的 Cargo.toml 中删除对 ServerCode 的依赖）
```

### 1. crate 内放什么（**只放契约，不放逻辑**）

| 类别 | 内容 | 说明 |
|---|---|---|
| **线格式类型** | `ClientMessage` / `PlayerInput` / `EntitySnapshot` / `ServerEvent` … | 即 `docs/contracts/protocol.yaml` 的 Rust 实现 |
| **跨域载荷类型** | `LootItem` / `ItemCategory` / `PickupKind` / `TransferDir` / `StationKind` / `InteractKind` | 会出现在快照/事件载荷里的枚举与结构，**它们本来就是契约** |
| **共享常量** | `BACKPACK_SLOTS` / `CONTAINER_SLOTS` / `GRID_COLS` / `GRID_ROWS` / `INTERACT_RANGE` | 两端必须一致的数值，禁止各自硬编码 |
| **Port Trait** | `SnapshotSource` 等 `XxxPort` | 跨模块/跨进程能力的抽象，实现可替换、可 mock |

**禁止放入**：任何 `std::fs` / 网络 / 模拟计算 / bevy 类型。契约 crate 的依赖只有 `serde` 与 `thiserror`。

### 2. 依赖方向（单向，不可逆）

- `ContractCode` 不依赖 `ServerCode`、不依赖 `HostCode`。
- `ServerCode → ContractCode`（服务端实现契约）。
- `HostCode → ContractCode`（客户端消费契约）。
- **`HostCode` 不再依赖 `ServerCode`**（本次收敛的判据）。
- `ServerCode/net` 仍保留协议编解码与会话，但**类型定义**改为 `pub use cute_of_duty_contract::protocol::*;` 以维持公开路径不变。

### 3. 静态表怎么办（`roster` / `model_preset` / 地图布局）

这些目前由客户端直接读服务端常量。收敛原则：**能下发的一律下发，不能下发的进契约 crate 做只读副本并注明来源**。

| 数据 | 处置 |
|---|---|
| 干员名册 `operator::roster` | 经握手/快照下发（服务端权威），客户端不再直读 |
| 模型预设 `model::ModelPreset` | 已经由快照下发（`model_preset` 字段），客户端删掉直读 |
| 地图布局 `map::layout()` | 客户端 `world` 的静态场景数据**过渡期**保留只读副本，标注"以服务端为准，待改为下发" |

### 4. 契约的定义方式：**手写类型 + 一致性测试**，暂不引入代码生成

- 权威描述是 `docs/contracts/*.yaml`；Rust 类型是它的实现。
- 增加一个测试：校验 YAML 与 Rust 类型的字段集合一致（字段名/可空性），不一致即测试失败。
- 不引入 `prost`/`schemars` 之类生成工具链（收益小于维护成本），后续如需再单开 ADR。

### 5. 迁移路径（**文档先行阶段不实施**，待全部文档补齐后统一开工）

1. 在 workspace 根 `Cargo.toml` 的 `members` 增加 `ContractCode`；新建 `ContractCode/Cargo.toml`（仅 `serde` + `thiserror`）。
2. 把 `ServerCode/net/protocol.rs` 的类型迁入 `ContractCode`；`ServerCode/net/protocol.rs` 改为 `pub use cute_of_duty_contract::protocol::*;`。
3. 迁移跨域枚举与常量（`LootItem` / `ItemCategory` / `StationKind` / `InteractKind` / 各 `*_SLOTS`）。
4. 定义 Port Trait（首批：`SnapshotSource`）。
5. `HostCode` 逐文件把 `use cute_of_duty_server::…` 换成 `use cute_of_duty_contract::…`；跑 `cargo-wrap check --workspace`，直到 `HostCode/Cargo.toml` 可删掉 `ServerCode` 依赖。
6. 删除 `HostCode/Cargo.toml` 对 `cute_of_duty_server` 的依赖 —— **这一步通过 = 冲突 2 收敛完成**。
7. 补契约一致性测试与 `docs/contracts/items.yaml`。

## 后果（Consequences）

**正向**
- 客户端与服务端**物理解耦**：服务端内部重构不再打断客户端（只要契约不变）。
- 「拆微服务无需重写」达成前提：契约 crate 就是未来的 IDL。
- 兼容承诺可执行：契约变更 = 版本号变更 = BarekHistory 条目。

**代价**
- 多一个 crate 的编译单元与一次跨 crate 引用搬迁（改动面覆盖客户端 12+ 文件）。
- "客户端只依赖契约"意味着**客户端不能再直接读服务端静态表**，需要补齐下发链路（roster 等）。

## 未决事项（Open Questions）

- 契约 crate 的 crate 名与目录名（`ContractCode` / `cute_of_duty_contract`）最终定名。
- 地图静态布局是否也要下发（涉及带宽与加载时机），还是永久保留客户端只读副本。
- YAML ↔ Rust 一致性测试的具体形式（自定义断言 / 第三方 schema 校验）。

## 替代方案（Alternatives considered）

- **方案 B（收敛到 `HostCode/net` 适配层）** — 否决：只是把违规 `use` 集中到一处，客户端仍在编译期依赖服务端 crate，**无法拆微服务**，只是把问题藏起来。
- **方案 C（保留现状，记为已知债）** — 否决：0.8-Snapshot-8 的编译断层已经证明这条路的代价——每次服务端改字段都砸客户端。
