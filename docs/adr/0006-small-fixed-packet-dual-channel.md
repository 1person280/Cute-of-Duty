# ADR 0006 · 线格式改为小定长包 + 指令优先组包 + 双通道（同端口按绑定包分角色）

- **状态**：已接受（Accepted）—— **取代** [ADR 0005](0005-slot-frame-transport.md)
- **日期**：2026-09-28
- **决策者**：项目 owner
- **影响范围**：`ServerCode/net`（`packet` 改写、`scheduler` **新增**、`resource_stream` **新增**、`codec` **新增**、`session` 重写、`main`）、`HostCode/net`（`network`/`uplink`/`downlink` 重写、`resource_downlink` **新增**、`remote`）
- **依据**：[ADR 0001](0001-modular-monolith-event-bus.md)「服务端权威」铁律；[ADR 0005](0005-slot-frame-transport.md) 的 16MB 对象池与预取语义（保留）；`docs/contracts/protocol.yaml`；`docs/architecture/module-boundaries.md` L2 `net` 契约纪律

---

## 背景（Context）

0.11 的**恒定 64KB 槽帧**对"高频小控制帧"极不划算：一条约 200B 的上行意图（`PlayerInput`）要占满 64KB，带宽利用率不到 0.4%；而快照与资源又混在同一包长里，收侧无法既省小包又搬大资源。

owner 逐条拍板：
1. **主/控制通道**每包恒定 **256B** = 32B 头 + **7×32B 单元**，每个"指令或数据"占 1 个 32B 单元；**指令优先于数据**组包。
2. **资源通道**走**独立第二条 TCP 连接**，每包恒定 **4096B**（32B 头 + 4064B 负载），多次拼接落进客户端 64KB 固定槽位。
3. **两条线程/通道不合并数据包**，各按固定包长读写。
4. **同一监听端口**：连接首条包恒为 256B **绑定包**，按其中角色字段决定此后按 256B（Control）还是 4096B（Resource）读取。

## 决策（Decision）

### 决策 1 · 主通道：恒定 256B、32B 单元、指令优先（`ServerCode/net/packet.rs` + `scheduler.rs`）

- `PACKET_BYTES = 256`、`HEADER_BYTES = 32`、`UNIT_BYTES = 32`、`UNITS_PER_PACKET = 7`、`MAIN_PAYLOAD_BYTES = 224`；每包**恒定 256B**（尾部零填充对齐）。
- 定长头 `PacketHeader{ magic/version/kind/flags/unit_count/sub_kind/region/chunk_index/payload_len/seq/key }`（布局见契约 `transport` 节）。
- `PacketKind = Bind | Command | Data | Resource | ResourceEnd`；`DataKind = Snapshot | Event | Control`。
- `SendScheduler`（**新增**）：把消息分为**指令类**（控制消息 + 全部上行意图 + 事件）与**数据类**（快照）。**每轮组包优先把指令装满 7 个单元**；无指令时才发数据切片。一条数据流必须**连续发完**（`active` 未发完不开下一条），而指令包不经重组器，故可穿插在数据切片之间——**指令永不被大数据饿死**。
- 指令编码走**二进制紧凑**（`codec.rs`）：热路径 `PlayerInput` 把 15 个 bool 位打包进单 32B 单元；含字符串的控制消息降级为 `DataKind::Control` 数据流（仍属指令类、仍优先）。

### 决策 2 · 资源通道：恒定 4096B（`resource_stream.rs`）

- `RES_PACKET_BYTES = 4096`、`RES_PAYLOAD_BYTES = 4064`；单份资源（体素几何/动画）按 `chunk_index` **跨多包分片**，`ResourceAssembler` 在收侧按 `key` 重组。
- `ModelCatalog` 被展开为"逐份资源 + `ResourceEnd` 批次终止"下发（与 0.11 语义一致，只是换了包长与通道）。
- 客户端 `remote.rs` 的 **16MB / 256×64KB 定址池、`base+i×64KB`、250 在用 + 6 预取、`promote`** 语义**全部保留**；分包拼接结果落进 64KB 槽位 JSON。

### 决策 3 · 同端口双通道：首条绑定包分角色（`session.rs`）

- 服务端**单一 `TcpListener`** `accept` 每条连接，先 `read_exact([0u8;256])` 读**首条绑定包**：校验 `kind == Bind`，`sub_kind` 即角色（`Role::Control=0` / `Role::Resource=1`），负载为玩家档案名。
- 判定后转入 `handle_control`（此后恒 256B，双向）或 `handle_resource`（此后恒 4096B，只下行）。资源连接凭档案名经 `profile_to_control` 关联控制连接的 `conn_id`；若资源连接早于控制 `Connect` 处理，暂存 `pending_resources`，待控制写入器注册后补齐。

### 决策 4 · 对象池淘汰经主通道上报（`PoolSync`）

- 客户端对象池淘汰资源时，经**主通道**上行 `ClientMessage::PoolSync{ region, evicted }`（二进制紧凑、每单元 3 个键、可跨单元）；服务端据此更新该连接的**常驻资源集合**，未来该键需要时再经资源通道重发。

## 后果（Consequences）

**正向**
- 小控制帧带宽利用率由 <0.4% 提升到接近 100%；快照/事件/意图各占 32B 单元、可同包混装。
- 资源与指令**物理分通道**，大资源的 4096B 分片不阻塞指令的 256B 小包，指令**永不被饿死**。
- 同端口即可容纳两类连接，部署无需两个监听端口。

**代价 / 风险**
- 指令包不经重组器、数据流必须连续，故**一条数据流不能与"同类的另一条数据流"交错**（靠 `active` 串行化保证）。
- `PoolSync` 若跨包（一包 7 单元上限）会被切成多条消息：因"移除淘汰键"是**幂等**操作，语义不受影响。
- `net` 为 L2 模块，**协议不兼容 → `y+1`**：本次升 `0.12.0`（`wire_version = 12`），须附迁移指南（见 `docs/barek-history.md`）与 `protocol.yaml` 更新。

## 未决事项（Open Questions）

- 预取推送的 Tick 接线（按 `prefetch.rs` 预测把 6 个实体资源标 `region=Prefetch` 经资源通道下发）仍在算法层就绪，端到端推送与客户端 `promote` 触发点待后续版本接通。
- 客户端对象池 **LRU 淘汰**的实际触发逻辑尚未实现（当前仅保留协议与服务端接收 `PoolSync` 的能力）。

## 替代方案（Alternatives considered）

- **维持统一 64KB 槽帧** — 否决：小控制帧浪费带宽，违背 owner「更快传输」目标。
- **两条通道合并为一条多路复用流** — 否决：违反 owner「两个线程不要试图合并数据包」，且复用会重新引入大小包互相阻塞。
- **两个独立监听端口** — 否决：owner 明确要求**同一监听端口**按首条绑定消息区分角色。
