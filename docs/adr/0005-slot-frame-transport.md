# ADR 0005 · 线格式改为统一 64KB 槽帧 + 客户端 16MB 远程对象池 + 双线单线程传输

- **状态**：**已被取代（Superseded by [ADR 0006](0006-small-fixed-packet-dual-channel.md)）** —— 0.12.0 起线格式改为"小定长包 + 指令优先组包 + 双通道"（256B 主通道 / 4096B 资源通道）。本 ADR 的**客户端 16MB 对象池与 AOI 预取语义仍有效**，被 0006 继承。
- **日期**：2026-09-28
- **决策者**：项目 owner
- **影响范围**：`ServerCode/net`（`packet` **新增**、`session`、`main`、`prefetch` **新增**）、`HostCode/net`（`remote` **新增**、`downlink` **新增**、`uplink` **新增**、`network`、`snapshot`）、`HostCode/flow`、`HostCode/launcher`
- **依据**：[ADR 0001](0001-modular-monolith-event-bus.md)「服务端权威」铁律；`docs/contracts/protocol.yaml`；`docs/architecture/module-boundaries.md` L2 `net` 契约纪律

---

## 背景（Context）

0.10 及以前，双端线格式是 **NDJSON**（TCP 上每行一条 `ServerMessage`/`ClientMessage`）：

1. **资源与瞬时消息混流**：体素几何/动画（`ModelCatalog`）与快照、事件挤在同一条行流里，握手时**一次性灌入**客户端内存；
2. **实体突现要等加载**：AOI（`net/aoi.rs`）裁决下实体进出视野频繁，而客户端对"没缓存过的造型"只能等下一批数据才知道长什么样；
3. **收侧无法按固定位置分离**：变长 JSON 行让"资源 vs 指令"无法在接收侧落进固定地址。

owner 逐条拍板要求：纯 TCP（**不引入共享内存**）、**全部下行统一固定 64KB 帧**、客户端维护**固定 16MB、固定地址**的通用远程对象池（资源缓存）、服务端做 **AOI 边缘预取**、**上传/下载双线单线程**、**不设传输时钟**。

## 决策（Decision）

### 决策 1 · 全部下行/上行统一为固定 64KB 槽帧（`ServerCode/net/packet.rs`）

- `FRAME_BYTES = 64KB`、`HEADER_BYTES = 32`、`PAYLOAD_MAX = 65504`，每帧**恒定 64KB**（尾部零填充对齐）；
- 定长头 `FrameHeader{ magic/version/kind/flags/region/sub_kind/seq/key/payload_len }`；
  `FrameKind = Control | Snapshot | Event | Resource | ResourceEnd`；
- 超单帧容量的消息（实体多的快照）按 `flags.continuation` **跨帧分片**，接收侧 `ChunkAssembler` 重组；
- 单份资源**一资源一帧**（`Resource`），一批资源以 `ResourceEnd` 收尾；`ModelCatalog` 即被展开为该形态。

**取消 NDJSON**：`to_line`/`from_line` 不再是线上路径（`ServerMessage`/`ClientMessage` 数据模型与 JSON 载荷本身保留）。

### 决策 2 · 客户端固定 16MB 固定地址对象池（`HostCode/net/remote.rs`）

- `POOL_BYTES = 16MB`、`SLOT_COUNT = 256`、`SLOT_BYTES = 64KB`；**一次性分配、永不重分配**，第 `i` 槽地址恒为 `base + i×64KB`；
- 区划：前 `IN_USE_SLOTS = 250`（16000KB）为**在用缓冲**、末 `PREFETCH_SLOTS = 6`（384KB）为**预取区**；250+6=256、16000KB+384KB=16MB 恒等；
- API：`insert`/`get`/`get_in`/`promote`（预取命中提升进在用区）/`clear_region`/`slot_ptr`；
- 池是**传输侧缓存**；下游渲染消费的 `ModelCatalog` 由 `sync_catalog_from_pool` 从池**增量解码**而成（池 → 视图），实体突现即命中、不用等加载。

**明确排除共享内存**：池纯客户端本地内存，保跨机可移植。

### 决策 3 · 上传/下载双线单线程，传输不设时钟

- `HostCode/net/downlink.rs`（下载）与 `uplink.rs`（上传）各为**阻塞单线程**，各持 `try_clone` 得到的独立 socket 句柄 —— **上传不阻塞下载**；
- 收/发完一帧立即处理下一帧（`read_exact` 整帧），**无 sleep、无传输节流时钟**；仅上传线程以极短 `recv_timeout` 兼顾每秒 Ping 探测（延迟面板数据源）；
- 连接级 `shutdown` 标志：下行断链即置位，上传随之退出，`run_network` 统一 2s 后重连。

## 后果（Consequences）

**正向**
- 资源与指令在接收侧**落到固定地址**，实体突现可**零等待复用**；预取区为"即将进入视野"预留位。
- 收侧解码路径固定（按 `kind` 分路），畸形帧可**立即断线重连**，不污染其它流。
- 上下行物理隔离，符合 owner「上传不得影响下载」。

**代价 / 风险**
- **带宽**：60Hz 满帧快照约 3.8MB/s/客户端（owner 已接受）；空载靠零填充对齐，不做压缩。
- 分片开销：超 64KB 的快照须按 `seq`+`continuation` 重组。
- `net` 为 L2 模块，**协议不兼容 → `y+1`**：本次升 `0.11.0`，须附迁移指南（见 `docs/barek-history.md`）与 `protocol.yaml` 更新。

## 未决事项（Open Questions）

- 预取推送的 Tick 接线（按 `prefetch.rs` 预测把 6 个实体资源标 `region=1` 下发）已在算法层就绪，端到端推送与客户端 `promote` 触发点待后续版本接通。
- 是否对高频小控制帧做"多帧合并写"（当前一条消息一次 `write_all`）。

## 替代方案（Alternatives considered）

- **共享内存 / 映射文件传资源** — 否决：破坏跨机可移植，owner 明确排除。
- **仅资源走 64KB 帧、其余保留 NDJSON** — 否决：无法在接收侧按固定位置分离，违背"统一槽帧"目标。
- **单线程收发（轮询 select）** — 否决：违反 owner「上传不影响下载」，写阻塞会拖住读。