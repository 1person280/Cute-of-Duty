# Host.net · module.md

> 客户端网络适配子模块（`HostCode/net/`）。分层定位：**表现层 · 网络适配（吃线格式）**。只消费服务端权威快照与事件、上行本地意图，**绝不做本地模拟或校订**。线格式类型（`protocol`/`packet`）归 [ContractCode](./Contract.lib.md)。

## 边界

**本模块负责**：
- **连接装配与重连**：`network`——同端口建**控制**（恒 256B）与**资源**（恒 4096B）双 TCP 连接，各首发角色绑定包；失败无限重连（间隔 2s，容忍先开客户端）。
- **下行消费**：`downlink`——控制下行线程（快照 / 控制消息 / Ping-Pong 打点）与资源下行线程（按 `key` 跨 4096B 包重组落池）。
- **上行意图**：`uplink`（线程侧发包 + Ping 打点）、`pilot`（`input_system`：把按键/视角折成 `PlayerInput`，仅在与上一帧不同才发）。
- **快照对账渲染**：`snapshot`——把 `EntitySnapshot` 映射为体素造型（更新坐标 / 生成缺失 / 递归销毁消失），按 `ModelPreset`+`tint` 缓存材质（防材质单调累积）。
- **远程资源对象池**：`remote`——固定 16MB · 256×64KB 固定地址槽（在位 250 + 预取 6），资源按 `key` 落槽、实体突现即复用；增量同步进 `flow::ModelCatalog`。
- **延迟面板**：`latency`——CapsLock 显隐；显示握手耗时 + 常态 RTT（均由网络线程打点，不含 Bevy 帧时间）。

**本模块不负责**（明确划出）：
- 线格式的**类型定义与编解码**——归 `ContractCode/net`（本模块只 `use`）。
- 任何**模拟/校订**——位置、存活、模型身份均为服务端裁决值，客户端只是"绘画镜面"。
- 门控判定——输入冻结结论取自 `flow::ModalState`（`launcher` 以 `run_if` 施加）。

## 数据所有权

| 数据 | 类型 | 是否可变 | 谁可写 |
|---|---|---|---|
| 最新一帧快照 | `SnapshotBuffer` | 每帧覆盖 | 仅 `net::receive_snapshots` |
| 下行控制缓冲 | `ControlBuffer` | 排空式 | 仅 `flow::route_control_messages` |
| 上行意图通道 | `NetOut` | 追加式 | 任意系统可 `send`（`pilot`/菜单/交互） |
| 远程资源池 | `RemoteObjects` | 可变 | 仅 `net::receive_resources` / `sync_catalog_from_pool` |
| 造型材质缓存 | `EntityMaterials` | 只增复用 | 仅 `net::snapshot` |
| 共享网格 | `CubeMesh` | 启动期 | `world::scene::spawn_scene_baseline` |
| 延迟面板态 | `LatencyShow` / `PanelSpawned` | 可变 | 仅 `net::latency` |

> **所有权原则**：网络线程只经 `mpsc` 投递，ECS 侧单点写资源；`Receiver` 非 `Sync` 故以 `Mutex` 包裹，锁只覆盖通道读取的短临界区。

## 接口

| 接口 | 形式 | 契约 | 兼容性承诺 |
|---|---|---|---|
| `run_network(addr, snapshot_tx, control_tx, resource_tx, up_rx)` | 函数 | 双连接装配 + 重连主循环 | 由 `launcher` 起线程调用 |
| `ClientInbound::{Connected, Rtt, Server}` | 枚举 | 网络线程 → Bevy 的下行事件 | 消费方 `flow` |
| `input_system(...)` | 系统 | 按键/视角 → `PlayerInput` 上行 | `run_if(gameplay_input_active)` |
| `receive_snapshots` / `apply_entities` | 系统 | 快照排空 + 实体对账 | 公开路径稳定 |
| `receive_resources` / `sync_catalog_from_pool` | 系统 | 资源帧落池 + 增量同步目录 | 须排在 `apply_entities` 之前 |
| `RemotePool::{insert, get, promote, contains, region_of, ...}` | 方法 | 固定地址对象池读写 | 池契约（固定地址/命中复用/预取提升） |
| `caps_toggle` / `spawn_panel` / `panel_update` | 系统 | 延迟面板 | 常驻，不随状态销毁 |
| 线格式类型（`ClientMessage`/`ServerMessage`/`EntitySnapshot`/`PacketHeader`…） | 类型 | 权威见 `ContractCode/net` | **L2 线格式契约** |

## 事件

| 事件名 | 方向 | 载荷契约 | 订阅者 |
|---|---|---|---|
| `PoolMessage::{Resource, BatchEnd}` | 网络线程 → ECS | 资源帧 / 批次终止 | `net::receive_resources` |
| `ClientInbound::{Connected, Rtt, Server}` | 网络线程 → ECS | 握手 / RTT / 服务端消息 | `flow::route_control_messages` |
| `ClientMessage::{Input, Ping, ...}` | ECS → 网络线程 | 上行意图 | `net::uplink` |
| `vec<EntitySnapshot>` | 网络线程 → ECS | 权威快照帧 | `net::receive_snapshots` |

## 成熟度

**L2（对外契约：吃线格式）** —— 依据：本模块消费/生产 `ContractCode` 的线格式类型，线格式不兼容即中断通信。

- 破坏性变更流程：线格式变更随 `ContractCode` 走 `y+1` + 迁移指南 + [protocol.yaml](../contracts/protocol.yaml) 同步；本模块适配代码随线格式同 PR 更新。
- 已发生示例：0.12 起小定长包 + 双通道（控制 256B / 资源 4096B）；0.14.0 `WorldCatalog` 由 `flow::route_control_messages` 消费。
