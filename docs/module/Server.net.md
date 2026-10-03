# Server.net · module.md

> 服务端网络层（`ServerCode/net/`）。分层定位：**基础设施层**。线格式契约（`codec`/`packet`/`protocol`/`resource_stream`/`scheduler`）已于 0.12.2 随 [ADR 0003](../adr/0003-contract-crate.md) 迁至 `cute_of_duty_contract::net`，本模块经 `pub use` 保持 `crate::net::*` 公开路径不变，并只保留**服务端专属编排**。

## 边界

**本模块负责**：
- 定长包的**收发与接线**：控制通道（恒定 256B）与资源通道（恒定 4096B）的 TCP 读写循环（`session`）、同端口按首条绑定包分角色。
- **传输无关核心**：`runtime`（`NetRuntime`/`NetCommand`/`ControlSession`）——为一个连接登记控制+资源两条写队列、投递连接/断开/输入/背包命令、把上行定长包解析为 `NetCommand`。TCP 与 WebSocket 两条载体共用同一核心。
- **权威快照构建与分发**：`broadcaster`（`build_snapshot`，含 AOI 过滤）+ `aoi`（兴趣区域剔除）。
- **AOI 边缘预取**：`prefetch`（按距离÷速度预测即将进入视野的实体，取 6 个）。
- **主循环阶段编排**：`stages`（消费命令 → 应用到权威世界 → 回执播报，从 `main.rs` 抽出）。
- **内置 Web 服务**：`web` 子域（HTTP/HTTPS 门户、运维 API、WebSocket 游玩桥，见 [ADR 0007](../adr/0007-native-web-service.md)）。

**本模块不负责**（明确划出）：
- 线格式的**类型定义与纯编解码**——归 `ContractCode/net`（`protocol`/`packet`/`codec`/`scheduler`/`resource_stream`）。
- 任何**玩法/规则/判定**——血量、背包、CD、战局、归属、模型身份一律服务端领域模块权威，`net` 不裁决，只搬运。
- 领域数据的**真相**——`web` 读取的状态经 `WebOpsPort` 由 `main.rs` 注入，`net` 不持有领域真相。

## 数据所有权

| 数据 | 类型 | 是否可变 | 谁可写 |
|---|---|---|---|
| 连接运行时（写队列 / 档案名 / 连接表） | `NetRuntime` / `Shared` | 可变 | 仅 `net`（TCP 会话、web 桥经 `open_transport`/`close_transport`） |
| 连接内网状态（`conn_entity`/`conn_input`/`conn_profiles`/`conn_resident`…） | `main.rs` 主循环局部持有 | 可变 | 仅 `main.rs` 主循环（`stages` 以参数借入） |
| 权威快照 | `ServerMessage`（契约类型） | 每 Tick 重建 | `net::broadcaster`（只读 `sim.world`） |
| Web 展示槽（在线玩家姓名/等级） | `PlayerSlot` | 可变 | `net::stages::refresh_web_players` |
| Web 服务配置 | `WebConfig` | 启动期只读 | `config::load_web_config` |

> **所有权原则**：热数据（实体/血量/CD）属 `sim.world`，`net` 只读并构建快照；冷数据（玩家档案）由 `main.rs` 持热副本、`storage` 落盘，`net` 不直接访问仓库。

## 接口

| 接口 | 形式 | 契约 | 兼容性承诺 |
|---|---|---|---|
| `accept_loop(addr, rt)` | 函数 | TCP 监听入口（同端口按绑定包分角色） | 公开路径稳定（`crate::net::accept_loop`） |
| `NetRuntime::{send_to, send_to_all, open_transport, close_transport}` | 函数 | 连接编号 → 下行 | 公开路径稳定 |
| `ControlSession::feed(&mut self, rt, bytes) -> Result<bool, _>` | 函数 | 上行定长包 → `NetCommand`；`false` 表示请求断开 | 传输无关，TCP/WS 共用 |
| `build_snapshot(world, tick, observer)` | 函数 | 权威世界 → AOI 过滤后的 `ServerMessage` | 公开路径稳定 |
| `predict_prefetch(...)` | 函数 | 世界 → 预取键集合 | 公开路径稳定 |
| `WebOpsPort`（`status`/`players`/`announce`/`kick`） | **Trait** | 领域能力 Port，由 `main.rs` 实现（`AuthorityOps`） | 破坏性变更须走 L2 流程 |
| `web::serve(config, ops, rt)` | 函数 | 装配并拉起 HTTP/HTTPS 监听 | 由 `web::spawn::spawn_web` 调用 |
| 线格式类型（`ServerMessage`/`ClientMessage`/`PacketHeader`…） | 类型 | 权威见 `ContractCode/net`；本模块 `pub use` | **L2 线格式契约**，见 [protocol.yaml](../contracts/protocol.yaml) |

## 事件

| 事件名 | 方向 | 载荷契约 | 订阅者 |
|---|---|---|---|
| `NetCommand::Connect` | 会话 → 主循环 | `{ conn_id, name }` | `main.rs` → `net::stages::drain_commands` |
| `NetCommand::Input` | 会话 → 主循环 | `{ conn_id, PlayerInput }` | 同上（连续量记最新 / 边沿量锁存） |
| `NetCommand::Inventory` | 会话 → 主循环 | `{ conn_id, InventoryAction }` | 同上 |
| `NetCommand::Disconnect` | 会话 → 主循环 | `{ conn_id }` | 同上 |
| `ServerMessage::{Handshake, ModelCatalog, PresetCatalog, WorldCatalog, Snapshot, Event, Control}` | 主循环 → 会话 | 契约类型 | 原生客户端 / 浏览器（经 `web` 桥） |

> ⚠️ 本表为 `net` 内部命令/消息流；**服务端跨模块事件清单与订阅关系图**仍属 [module-boundaries 第七节](../architecture/module-boundaries.md) 待补项（`?` 列），补齐前不得把既有直接调用改造成事件。

## 成熟度

**L2（对外契约：被其它 crate / 线格式依赖）** —— 依据：`net` 定义/承载线格式，被 `HostCode` 与 `ContractCode` 依赖。

- 破坏性变更流程：`y+1`（协议不兼容）必附**迁移指南** + [barek-history](../barek-history.md) 条目 + [protocol.yaml](../contracts/protocol.yaml) 同步；**≥2 名 reviewer（maintainer 必须参与）**。
- 加性/细节变更：`z+1`，老端必须能忽略新字段继续运行（如 0.12.3 内置 Web 服务：**不触碰线格式**，`wire_version` 仍为 12）。
- 已发生示例：**0.13.0 新增下行变体 `ServerMessage::PresetCatalog`（选装预设目录）→ `y+1`**，`wire_version` 12 → 13，
  附迁移指南（[BarekHistory 0.13.0](../barek-history.md)）与 [protocol.yaml](../contracts/protocol.yaml) 同步；
  该消息经控制类数据流（`DataKind::Control` + JSON）承载，包帧结构未变，破坏点仅在新增变体。
- 已发生示例：**0.14.0 新增下行变体 `ServerMessage::WorldCatalog`（干员名册 + 活动地图布局）→ `y+1`**，`wire_version` 13 → 14，
  由 `net::stages::drain_commands` 的 `Connect` 分支在握手后一次性下发（`crate::operator::roster()` 与
  `crate::map::lawn::layout()`），客户端据此不再直读契约静态表；配套契约类型 serde 化见
  [BarekHistory 0.14.0](../barek-history.md) 与 [protocol.yaml](../contracts/protocol.yaml)。
- 线格式权威源当前为**代码**（`ContractCode/net/protocol.rs`）；"YAML 为准、代码由 YAML 校验"的权威源反转列为后续独立任务。
