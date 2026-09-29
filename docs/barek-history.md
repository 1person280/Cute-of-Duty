# BarekHistory · 变更记录（变更类型 / 兼容性 / 迁移指南）

> **为什么叫 BarekHistory**：沿用项目 owner 的命名。它是**面向兼容性的变更台账**——
> 与普通 CHANGELOG 的区别在于：每一条都必须回答「**这是不是破坏性变更**」「**老代码/老客户端怎么办**」。
>
> **写入时机**：任何触碰 **L2 及以上**模块（见 [模块边界](architecture/module-boundaries.md)）的提交，
> 或任何改动线格式 / 配置语义 / 公共 Trait 的提交，**必须在同一 PR 内追加一条**。
>
> **生成顺序**：最新在最上。

---

## 记录格式

```markdown
## [版本 x.y.z] · YYYY-MM-DD · <标题>
- **变更类型**：Breaking / Additive / Fix / Refactor（不含语义变化）
- **影响模块**：module-a, module-b
- **兼容性**：兼容 / 不兼容（说明破坏点）
- **迁移指南**：`y+1`（协议不兼容）必填 —— 逐步操作 + 前后代码对照
- **验证**：如何验证（命令 / 手动步骤）+ 结果
- **关联**：ADR / Issue / 契约文件
```

---

## [0.12.1] · 2026-09-29 · 引擎底层小步升级 Bevy 0.14 → 0.15（仅迁强制破坏项）

- **变更类型**：Refactor（无协议语义变化；客户端依赖升级 + API 平移）
- **影响模块**：`HostCode`（`flow`/`hud`/`menu`/`net`/`world`/`launcher` 的生成侧与更新侧）；`ServerCode` **零改动**
- **兼容性**：**兼容**（`z+1`）
  - 线格式 / 契约 / `ServerMessage` / `ClientMessage` **完全不变**，`docs/contracts/protocol.yaml` 的 `wire_version` 仍为 `12`；双端 `0.12.1` 与 `0.12.0` 可互通。
  - `ServerCode` 不依赖 bevy，本次升级不触及服务端模拟与网络层。
  - 破坏点仅限**客户端源码编译面**（非对外契约）：0.15 删除了 `TextBundle`/`TextStyle`，`Style` 类型改名 `Node`，`SpatialBundle` 被移除，`PbrBundle` 字段类型改为 `Mesh3d`/`MeshMaterial3d`。这不是协议不兼容，故不升 `y`。
- **迁移指南**：不适用（`z+1`，非协议不兼容）。仅对**后续在本仓新增客户端 UI 代码**者提示：
  - 文本一律用适配层 `flow::text(内容, flow::style(&fonts, 字号, 颜色))` 生成，勿再用 `TextBundle::from_section`（0.15 已无此 API）。
  - UI 样式类型是 `Node`（不是 `Style`），`NodeBundle`/`ButtonBundle` 的样式字段名是 `node`（不是 `style`）。
  - 需要根节点带变换 + 可见性时用 `(Transform::.., Visibility::default())`，勿再找 `SpatialBundle`（0.15 已移除）。
  - 画网格用 `PbrBundle { mesh: Mesh3d(handle), material: MeshMaterial3d(handle), .. }`（字段已被新类型包装）。
- **迁移边界（只做强制项）**：已弃用但仍可编译的 `NodeBundle`/`PbrBundle`/`Camera3dBundle`/`DirectionalLightBundle`/`PointLightBundle` **一律未动**，留待 0.16；构建期 276 条弃用告警属预期。
- **验证**：
  - `cargo-wrap check --workspace` 退出码 `0`（无错误）
  - `cargo-wrap test -p cute_of_duty_server` **138 passed / 0 failed**
  - `cargo-wrap build --workspace --release` 成功产出双端 exe
  - **实机**：启动双端进训练场，确认 UI（主菜单 / HUD / 背包 / 轮盘 / 大地图）与 3D（场景光照 / 焰狐体素模型 / 相机）无回归
- **关联**：`HostCode/Cargo.toml`、README「已知坑 · 底层冻结红线（2026-09-29 修订为小步升级）」、[ADR 0004](../adr/0004-client-layer-convergence.md)

---

## [0.12.0] · 2026-09-28 · 线格式改为小定长包 + 指令优先组包 + 双通道

- **变更类型**：**Breaking**（线格式由恒定 64KB 槽帧改为 256B 主通道 + 4096B 资源通道，协议不兼容 → `y+1`）
- **影响模块**：`net`（`packet` 改写、`scheduler` **新增**、`resource_stream` **新增**、`codec` **新增**、`session` 重写、`main`）；客户端 `net`（`network`/`uplink`/`downlink` 重写、`resource_downlink` **新增**、`remote`）
- **兼容性**：**不兼容**
  - 线上不再是"恒定 64KB 帧"。老客户端连新服务端会在**首条绑定包**即因类别（`Bind`）/魔数/包长不符而断开；老服务端同样无法解析新客户端的小定长包。双端须同升 `0.12.0`。
  - `ServerMessage`/`ClientMessage` 的**数据模型不变**，仅**封装与传输方式**改变。客户端 16MB 对象池与 AOI 预取语义**不变**。
- **迁移指南**：见 [契约 `protocol.yaml` 的 `compat.migration_guide`](contracts/protocol.yaml)（双通道各自固定包长、同端口绑定包分角色、指令优先组包、资源 4096B 分片）。核心五步：① 服务端控制连接用 `SendScheduler` 指令优先组包；② 服务端资源连接用 `resource_stream` 按 4096B 分片；③ 服务端 `session` 由"一帧一消息"改为"同端口首条 256B 绑定包分角色"；④ 客户端两条线程分别恒 256B / 恒 4096B，**不合并数据包**；⑤ 建连时各发一条 256B 绑定包声明角色。
- **内容**：
  - **主通道 256B · 32B 单元 · 指令优先**（`ServerCode/net/packet.rs` 改写 + `scheduler.rs` **新增**）：`PACKET_BYTES = 256`、`HEADER_BYTES = 32`、`UNIT_BYTES = 32`、`UNITS_PER_PACKET = 7`；`SendScheduler` 每轮**优先把指令装满 7 个单元**，无指令时才发数据切片，一条数据流连续发完——**指令永不被大数据饿死**。
  - **指令二进制紧凑**（`ServerCode/net/codec.rs`，**新增**）：热路径 `PlayerInput` 把 15 个 bool 位打包进单 32B 单元；含字符串的控制消息降级为 `DataKind::Control` 数据流（仍属指令类、仍优先）。`PoolSync` 每单元 ≤3 个键、可跨单元（连续单元解码时收拢为一条）。
  - **资源通道 4096B**（`ServerCode/net/resource_stream.rs`，**新增**）：`RES_PACKET_BYTES = 4096`、`RES_PAYLOAD_BYTES = 4064`；单份资源按 `chunk_index` 跨包分片，收侧 `ResourceAssembler` 按 `key` 重组；`ModelCatalog` 展开为"逐份资源 + `ResourceEnd`"。客户端 16MB / 256×64KB 定址池、250 在用 + 6 预取、`promote` 语义**全部保留**。
  - **同端口双通道分角色**（`ServerCode/net/session.rs` 重写）：单一 `TcpListener`，首条 `read_exact(256)` 读**绑定包**（`kind == Bind`，`sub_kind` = 角色 0=Control / 1=Resource，负载 = 档案名）；判定后控制连接恒 256B、资源连接恒 4096B。资源连接凭档案名经 `profile_to_control` 关联控制 `conn_id`，早到则暂存 `pending_resources` 待补齐。
  - **对象池淘汰上报**（`ClientMessage::PoolSync`，经主通道）：服务端 `main.rs` 维护 `conn_resident` 常驻资源集合，收到淘汰清单即移除（幂等）。
  - **客户端双连接**（`HostCode/net/network.rs`/`uplink.rs`/`downlink.rs` 重写、`resource_downlink.rs` **新增**）：控制连接恒 256B（上下行单线程）、资源连接恒 4096B（下行单线程），**两条线程不合并数据包**；建连时各发一条 256B 绑定包声明角色。
- **验证**：`cargo-wrap check -p cute_of_duty_server` / `-p cute_of_duty_host` 退出码 **0**；`cargo-wrap test -p cute_of_duty_server` **138 passed**（新增 scheduler 组包 / resource_stream 分片重组 / codec 二进制往返 / PoolSync 跨单元 等单测）。
- **关联**：[ADR 0006](adr/0006-small-fixed-packet-dual-channel.md)（取代 [ADR 0005](adr/0005-slot-frame-transport.md)）、[契约 `protocol.yaml`](contracts/protocol.yaml)

---

## [0.11.0] · 2026-09-28 · 移除备弹中间池：换弹直抽背包弹药堆

- **变更类型**：**Breaking**（快照删字段 `ammo_pool`；既有 `0.11.0` 不兼容窗口内一并收口）
- **影响模块**：`combat`（`combatant`/`shooter`/`mod`）、`net`（`protocol`/`broadcaster`）、`items`；客户端 `hud`
- **兼容性**：**不兼容**
  - `EntitySnapshot` **删除** `ammo_pool`，**新增** `ammo_reserve`（`#[serde(default)]`）：老客户端读不到 `ammo_pool`、也拿不到 `ammo_reserve`。双端须同升 `0.11.0`。
- **迁移指南**：HUD「备用」改读 `ammo_reserve`（= 背包内全部弹药堆合计）；服务端不再维护 `Combatant::ammo_pool`。
- **内容**：
  - **删除中间弹池**：移除 `Combatant::ammo_pool` 字段、`DEFAULT_AMMO_POOL`、`combat::add_ammo_pool`、`combat::pull_ammo_from_backpack`。备弹的**唯一权威宿主回归背包弹药堆**（可堆叠物品）。
  - **换弹改为直抽背包**：换弹计时与补弹从组件 `on_tick` 迁到世界级 `combat::shooter::tick_reloads`（组件 `on_tick` 期间组件表被临时取出、拿不到兄弟组件 `Backpack`）。计时耗尽即按 `max_ammo - ammo` 从背包 `draw_ammo` 直接补满弹夹；`try_reload`/打空自动换弹的判据改为「背包 `ammo_total() > 0`」。
  - **修复「多按一次 R」**：旧实现每开一枪会把**整弹夹量**折进池，而换弹只消耗"池里现有"的量；池不满时一次换弹只补一部分弹夹，须再按一次 R 补足。直抽背包后一次换弹必然全额补满，也省去了将来多种子弹时无谓的池区分。
  - **HUD 语义修正**：`ammo_reserve` 由服务端按 `Backpack::ammo_total()` 合计下发，右下 HUD 与背包面板的「备用」改读它——不再显示换弹后归零的瞬时池。
- **验证**：`cargo-wrap check --workspace` 退出码 **0**；`cargo-wrap test -p cute_of_duty_server` **通过**（含 `reload_works_with_third_ammo_stack`：3 叠弹药一次按 R 补满弹夹、背包恰好减 30；`snapshot_reports_true_ammo_reserve`）。
- **关联**：[契约 `protocol.yaml`](contracts/protocol.yaml)

---

## [0.11.0] · 2026-09-28 · 线格式改为统一 64KB 槽帧 + 客户端 16MB 远程对象池 + 双线单线程传输

- **变更类型**：**Breaking**（线格式由 NDJSON 行帧改为统一固定 64KB 槽帧，协议不兼容 → `y+1`）
- **影响模块**：`net`（`packet` **新增**、`prefetch` **新增**、`session`、`main`）；客户端 `net`（`remote`/`downlink`/`uplink` **新增**，改 `network`/`snapshot`）、`flow`、`launcher`
- **兼容性**：**不兼容**
  - 线上不再是"每行一条 JSON"。老客户端（NDJSON）连新服务端会在首帧即因魔数/长度不符而断开；老服务端同样无法解析新客户端的 64KB 帧。双端须同升 `0.11.0`。
  - `ServerMessage`/`ClientMessage` 的**数据模型与 JSON 载荷不变**，仅**封装与传输方式**改变。
- **迁移指南**：见 [契约 `protocol.yaml` 的 `compat.migration_guide`](contracts/protocol.yaml)（服务端写/读循环改帧、客户端拆双线程 + 对象池、前后帧格式对照）。核心五步：① `session::write_loop` 改 `FrameWriter + encode_server`；② `session::read_loop` 改 `read_exact(64KB) + FrameReader + ChunkAssembler + decode_client`；③ 客户端拆 `downlink`/`uplink` 两线程（各持 `try_clone` 句柄）；④ 下行资源帧写 16MB 固定地址对象池；⑤ `ModelCatalog` 由池增量解码。
- **内容**：
  - **统一 64KB 槽帧**（`ServerCode/net/packet.rs`，**新增**）：`FRAME_BYTES = 64KB`、`HEADER_BYTES = 32`、`PAYLOAD_MAX = 65504`；定长头 `FrameHeader{ magic/version/kind/flags/region/sub_kind/seq/key/payload_len }`；`FrameKind = Control | Snapshot | Event | Resource | ResourceEnd`。超单帧容量的消息按 `flags.continuation` **跨帧分片**、接收侧 `ChunkAssembler` 重组；单份资源**一资源一帧**、一批资源以 `ResourceEnd` 收尾。`ModelCatalog` 被展开为"一资源一帧 + ResourceEnd"下发。附 10 项单测（64KB 对齐、小控制帧入 64KB、超长分片重组、资源帧往返、目录展开、坏帧拒绝等）。
  - **客户端 16MB 远程对象池**（`HostCode/net/remote.rs`，**新增**）：`POOL_BYTES = 16MB`、`SLOT_COUNT = 256`、`SLOT_BYTES = 64KB`；**一次性分配、永不重分配**，第 `i` 槽地址恒为 `base + i×64KB`。区划：前 `IN_USE_SLOTS = 250`（16000KB）在用缓冲、末 `PREFETCH_SLOTS = 6`（384KB）预取区（250+6=256、16000KB+384KB=16MB）。API `insert/get/get_in/promote/clear_region/slot_ptr`；资源按 `key`（FNV-1a）落槽、命中即复用，预取命中经 `promote` 提升进在用区。**纯客户端本地内存，不引入共享内存**。
  - **池 → 视图**：`sync_catalog_from_pool` 把池中在用区资源**增量解码**进 `ModelCatalog`（下游 `apply_entities`/`voxel_model`/`voxel_idle` 的消费视图），实体突现即命中、不必等加载。`route_control_messages` 的 `ModelCatalog` 分支随之移除（目录不再经控制通道抵达）。
  - **双线单线程传输**（`HostCode/net/downlink.rs`、`uplink.rs`，**新增**）：下载/上传各为**阻塞单线程**，各持 `try_clone` 独立 socket 句柄 —— **上传不阻塞下载**。收/发完一帧立即处理下一帧，**无 sleep、无传输时钟**；仅上传线程以极短 `recv_timeout` 兼顾每秒 Ping 探测（延迟面板数据源）。连接级 `shutdown` 标志保证断链时两线程同步退出，`run_network` 统一 2s 重连（保留原 `RECONNECT_INTERVAL`）。
  - **服务端会话改帧**（`ServerCode/net/session.rs`）：`write_loop` 由 `to_line` 改为 `FrameWriter + encode_server`；`read_loop` 由 `read_line + from_line` 改为 `read_exact(64KB) + FrameReader + ChunkAssembler + decode_client`；客户端消息投递抽为 `dispatch_client_message`。
  - **AOI 边缘预取算法**（`ServerCode/net/prefetch.rs`，**新增**）：对 AOI 外实体按「到 AOI 边界距离 ÷ max(速度, ε)」升序取前 6，产预取资源集（`PREFETCH_COUNT = 6`、`MIN_SPEED = 0.5`）。**注**：端到端推送接线（Tick 内按预测标 `region=1` 下发 + 客户端 `promote`）留待后续版本。
- **验证**：`cargo-wrap check -p cute_of_duty_server -p cute_of_duty_host` 退出码 **0**（无警告）；`cargo-wrap test -p cute_of_duty_server` **127 passed**（既有 114 + 新增帧编解码/对象池/预取）；`cargo-wrap test -p cute_of_duty_host` 通过。
- **关联**：[ADR 0005](adr/0005-slot-frame-transport.md)、[契约 `protocol.yaml`](contracts/protocol.yaml)

---

## [0.10.0] · 2026-09-27 · 服务端下发体素模型目录（客户端可见焰狐模型）+ 客户端资源目录归位

- **变更类型**：**Additive**（仅新增下行消息，无字段删改、无语义变更，向后兼容）
- **影响模块**：`model`（服务端权威几何解析，**新增** `loader`）；`net/protocol`（服务端契约）；`main`（握手后下发）；客户端 `world`（**新增** `voxel_model`/`voxel_idle`/`voxel_facing`，改 `camera`）、`net/snapshot`、`flow`、`launcher`、`menu`、`shared`
- **兼容性**：**兼容**
  - `ServerMessage` **新增** `ModelCatalog { models, animations }`（握手后一次性下发；老客户端收到未知变体按既有 `_ => {}` 分支忽略即可，行为不变）。
  - 无既有字段删改或语义改动；客户端资源目录搬迁不触线格式。
- **迁移指南**：无需迁移。老客户端连新服务端：忽略 `ModelCatalog` 后仍按原 `ModelPreset` 方块回退渲染，功能不受影响。
- **内容**：
  - **模型源文件归位与合并**：`ServerCode/assets/model/yanhu.{geometry,animation}.json` → `ServerCode/model/`，再**合并为单份** `ServerCode/model/FireFox.json`（几何在顶层 `minecraft:geometry`、动画在顶层 `animations`），删除空目录 `ServerCode/assets/`（模型属服务端易变内容，对齐既有约定）。
  - **加载器重命名**（`model/voxel_spec.rs` → `model/loader.rs`，**新增**）：以 `include_str!` 编译期嵌入**单份** `FireFox.json`，解析为可序列化的
    `VoxelModelSpec`（骨/枢轴/旋转/盒）与 `VoxelAnimationSpec`（clip/时长/骨名→旋转表达式），`OnceLock` 缓存；
    `catalog()` 当前仅 `OperativeFire → firefox`。附解析单测（12 骨 / 43 盒 / 3 clip，并断言**每盒均标注 `mat`** 与**左右侧位不颠倒**）。
    **命名免责声明**：`FireFox` 为本作我方火系干员「焰狐」（Flame Fox）之建模命名，与 Mozilla Firefox 浏览器无关；clip/几何 identifier 由 `yanhu` 统一改为 `firefox`（客户端 `IDLE_CLIP` 同步）。
  - **逐盒材质键**（`VoxelCube.mat`，**新增可选字段**）：几何 JSON 每个盒子新增 `mat`（语义名 `skin`/`green`/`dark`/`armor`/`boot`/`hair`/`hair_dark`/`cream`/`eye`/`gun`/`accent`），
    随模型目录下行。**根因修复**：此前客户端只按**骨名**着色，同一根骨上的发冠/刘海/腰带/护甲/靴子等细节全被抹成同色，
    模型退化为一坨纯色方块（0.3.2 参考实现是逐盒材质，故有层次）；现客户端按 `mat` 取 0.3.2 色板（`_ref/.../palette.rs`）逐盒上色，缺 `mat` 时仍按骨名兜底。
  - **焰狐几何重建**（`model/FireFox.json`）：修正三处与 0.3.2 参考不符的缺陷——①**四肢左右镜像颠倒**（`arm_right`/`leg_right` 误置于 -X，
    与动画摆臂相位「右臂与右腿反相」自相矛盾）；②**枪挂错骨**（`gun` 挂 `arm_right` 却把盒子放在 +X，导致枪悬空约 1.15m）；
    ③**狐耳倾角内折**（`-15/+15` 应为外张 `+12/-12`）。同时优化造型：狐吻 + 鼻头、眼高光、腮毛、尾巴加粗。骨名与父子关系保持不变，动画不受影响。
  - **协议下发**（`net/protocol.rs` + `main.rs`）：`ServerMessage` 新增 `ModelCatalog`，`NetCommand::Connect` 握手帧之后向该连接下发一次目录（静态数据，不进每帧快照）。
  - **客户端渲染**（`world/voxel_model.rs` + `voxel_idle.rs`，**新增**）：复用 `spawn_scene` 的单位 `CubeMesh`，按骨建 `SpatialBundle` 枢轴树、
    盒作子实体缩放着色（坐标 `S = 0.11`、`z` 取负，对齐 0.3.2 参考实现）；`voxel_idle` 求值 `idle` clip 表达式驱动枢轴旋转。
    `snapshot::spawn_body` 对**本人实体**改用体素模型（目录未到时暂缓本人生成），敌人/靶机/道具仍走方块回退。
    另加两处时序修复：`route_control_messages` 提到 `apply_entities` 之前（否则首帧快照会把本人按方块落成且此后不再重建），
    并以 `VoxelRendered` 标记做**幂等升级**（目录迟到时把已落成的方块本人拆掉重建），二者保证本人必为体素模型。
  - **朝向同步**（`world/voxel_facing.rs`，**新增**）：每帧把本人模型根旋转对齐 `AimRig.yaw`（视线偏航），
    **修复「角色永远向北」**——此前根旋转只在生成时写死一次 `from_rotation_y(π)`，此后无人更新，镜头转动角色不转。
  - **相机按 3.52m 体型标定**（`world/camera.rs`）：`CAMERA_DIST` 4.2→**6.5**、`PIVOT_Y` 1.55→**2.6**、`SHOULDER_OFFSET` 0.65→**0.55**，
    回到 0.3.2 为 3.5m 体型标定的取值。原值是按旧方块回退角色（≈2.67m）调的，套到 32px×0.11≈3.52m 的焰狐身上会过近、仰视，模型怼满屏幕。
  - **客户端资源目录归位**：取消 `HostCode/assets/`——`ui/gear_icon.png` → `menu/icon/settings.png`，
    `zcool_kuaile.ttf` / `zcool_kuaile_OFL.txt` → `HostCode/menu/`；`AssetPlugin.file_path` 改指 HostCode 根；同步 `LICENSE` / `LICENSE-ASSETS` / `README` / skill 文档路径引用。
- **验证**：`cargo-wrap check --workspace` 退出码 **0**；`cargo-wrap test -p cute_of_duty_server` 通过（新增 loader 解析 + 协议往返用例）；`cargo-wrap build --release --workspace` 产出双端 exe；实机进训练场可见本人完整焰狐模型。
- **关联**：[契约 `protocol.yaml`](contracts/protocol.yaml)

---

## [0.9.1] · 2026-09-27 · 修手雷直线飞行 + 投掷轨迹预览 + 释放光标冻结视角

- **变更类型**：**Fix**（z+1 兼容性修复；无字段增删、无语义变更）
- **影响模块**：`combat/grenade`（服务端弹道）；客户端 `world/grenade_preview`(**新增**)、`hud/hud_bigmap`
- **兼容性**：**兼容**（线格式未变。弹道由直线纠正为抛物线属**修复**——此前是缺陷，非既定语义）
- **迁移指南**：无需迁移。
- **内容**：
  - **修「投掷物为直线、无视重力」**（`combat/grenade.rs::tick_grenades`）：根因是重力只扣在**局部副本**
    `let mut vel = g.velocity` 上、**从未写回组件**，于是每 Tick 都从初速重新起步、竖直速度恒定 →
    位置对时间呈线性（直线）。现把更新后的速度与剩余引信一并写回 `GrenadeState`，弹道恢复为
    12 m/s² 重力抛物线。顺带把飞行常数（初速/重力/引信/落点高度/出手点）抽为 `pub const`，
    供客户端预览**同源**复用。
  - **新增投掷轨迹预览**（`HostCode/world/grenade_preview.rs`）：持雷越肩时用 `Gizmos` 画点状预测弧线 +
    落点标记；弹道常数直接 `use` 服务端 `combat::grenade` 的常量，预览与实际结算不漂移。
  - **修「释放鼠标会移动视角」**（`hud/hud_bigmap.rs::gameplay_input_active`）：Esc「无 UI 时交还光标」
    的软开关 `CursorReleased` 此前未计入玩法门控，光标释放后 `mouse_look_system` 仍在跑、鼠标一动
    视角就转。现并入该门控（释放光标即冻结视角与上报），与暂停/各面板口径一致。
- **验证**：`cargo-wrap check --workspace` 退出码 **0**；`cargo-wrap test -p cute_of_duty_server`
  **111 passed / 0 failed**（新增 `thrown_grenade_follows_parabola` 弹性回归用例）。
- **关联**：[冻结任务 · 快照8 实机反馈](frozen-tasks/snapshot-8-playtest-feedback.md)（F-5）、
  [契约 `protocol.yaml`](contracts/protocol.yaml)

---

## [0.9.0] · 2026-09-27 · 持雷态权威 + 可点击操作按钮组（0.6 遗留项补齐）

- **变更类型**：**Additive**（仅新增字段 / 新增客户端能力，无既有语义改动）
- **影响模块**：`net/protocol`（服务端契约）；`combat`（服务端持雷权威）；客户端 `hud`(grenade_hint/button_panel)、
  `world/camera`、`menu/pause`、`net/pilot`、`launcher`
- **兼容性**：**兼容**（y+1 加性）
  - `PlayerInput` **新增** `grenade_cancel: bool`（`#[serde(default)]`，老客户端缺省即 `false`，行为不变）；
  - `EntitySnapshot` **新增** `held_grenade: Option<ElementType>`（老客户端忽略该字段即可）。
- **迁移指南**：无需迁移。老客户端连新服务端：不含 `grenade_cancel` / `held_grenade` 时按缺省处理，
  持雷态对老客户端不可见（仍可通过 `use_slot` 投掷），无破坏性影响。
- **内容**：
  - **服务端持雷权威**：`combat` 新增 `HeldGrenade` 组件与结算——`use_item_at` 选中战术类手雷时**先握持、
    不立即投掷**，左键 `shoot` 释放投掷并扣件、`grenade_cancel` 取消放回；持雷期间抑制常规射击。
    快照下行 `held_grenade` 供表现层读取。
  - **客户端持雷表现**（`hud_grenade_hint.rs`）：新增 `HeldGrenadeState` 派生资源，持雷时强制越肩
    （`sync_grenade_aim` 叠加 `AimRig::aiming`）并屏幕下方常显「手持手雷 — 左键投掷 · Esc 取消」提示；
    上行 `shoot` 语义在持雷态改为**左键边沿**、`grenade_cancel` 绑 Esc。
  - **可点击操作按钮组**（`hud_button_panel.rs`，补齐台账 B#1）：`B` 打开模态面板，逐键合成 `PlayerInput`
    上行（移动类点按切换、动作类边沿、使用类取该类别首格下标）；打开时释放光标并纳入
    `gameplay_input_active` / `cursor_lock_system` / `cursor_release_toggle` / `item_wheel_input.blocked` /
    `interact_input` 五处门控；关闭当帧补发全零输入以停止持续动作。
- **验证**：`cargo-wrap check --workspace` 退出码 **0**；`cargo-wrap test -p cute_of_duty_server`
  **110 passed / 0 failed**；`cargo-wrap build --workspace --release` 退出码 **0**。
- **关联**：[冻结任务 · 快照8 实机反馈](frozen-tasks/snapshot-8-playtest-feedback.md)（B#1/B#2/F）、
  [契约 `protocol.yaml`](contracts/protocol.yaml)

---

## [0.6-SnapShot-10] · 2026-09-27 · 手雷可见性根因修复 + 备用子弹改背包可堆叠物品 + legacy 操作表核对析出项（D-1）（**待实机验证**）

- **变更类型**：**Breaking**（线格式 + 输入语义 + 玩法行为）
- **影响模块**：`combat`(mod/combatant/shooter), `items`, `interact`, `entity`, `main.rs`（服务端）；
  `hud_item_wheel`, `hud_loot_panel`, `hud_backpack_panel`(**新增**), `hud_root`, `hud_bigmap`, `world/camera`,
  `menu/pause`, `menu/mod`, `net/pilot`, `launcher`（客户端）
- **兼容性**：**不兼容**（服务端与客户端必须**同版本**部署）
  - `PlayerInput` **新增** `jump: bool`（跳跃意图，持续量）；
  - `EntitySnapshot` **新增** `backpack` 内 `LootItem.count` 语义（同格堆叠计数，`display_label` 供 HUD）；
  - **玩法语义变更**：备弹权威宿主由 `Combatant.ammo_pool` 迁至背包格位（`ammo_pool` 降级为换弹中转）；
    疾跑键位 `ShiftLeft → ControlLeft`、常速 `5 → 4`、疾跑倍率 `1.6 → 1.75`（= 4 → 7 单位/秒）。
- **迁移指南**（不提供跨版本兼容）：
  1. 客户端输入映射删除 `ShiftLeft` 疾跑绑定，改绑 `ControlLeft`；新增 `Space → PlayerInput.jump`；
  2. 3/4 槽 / HUD 读取备弹时改按 `EntitySnapshot.backpack` 中 `ItemCategory::Ammo` 的 `count` 求和，
     不再读 `Combatant.ammo_pool`；
  3. 任何直接写弹药池的下游逻辑改为写背包物品（`Backpack::draw_ammo` / `push`）。
- **内容**：
  - **修「手雷无可见投射物」**：根因**不在快照/渲染**（`combat/grenade.rs::spawn_projectile` 与快照
    `ModelPreset::Grenade` 链路经复核本就正确），而在客户端 `hud_item_wheel.rs::item_wheel_input`
    松开分支先写 `pending_slot` 再 `*state = ItemWheelState::default()`，`default()` 把 `pending_slot`
    一并抹掉 → `use_slot` 从未上报（同因导致 3/4「无响应」）。修法：先求值 → 复位会话 → **最后回填**
    `pending_slot`，保证脉冲被 `net::input_system` 取走。
  - **备用子弹改背包物品（可堆叠 64）**：`PickupKind::max_stack()`（弹药 64 / 恢复·战术类 16 / 武器 1）；
    `LootItem` 增 `count`/`display_label`；`Backpack`/`Container::push` 改**堆叠感知**并新增
    `same_stack_kind()`（忽略逐件 `amount`，以 `label` 兜底区分同名不同档）；
    `combat::pull_ammo_from_backpack` 在换弹/自动换弹前折现备弹。
  - **D-1 legacy 操作表核对落地六条**：跳跃整链路（`JUMP_SPEED/GRAVITY/GROUND_Y` 竖直积分 +
    `Entity.vertical_velocity/grounded`，仅着地可起跳）、疾跑键位与速度对齐、越肩 SpringArm 碰撞避障
    （`map::lawn` 静态 solid props → AABB 缓存 + slab 射线-AABB 扫掠，撞墙缩回 ≥0.7m / 离墙缓伸 /
    地面钳制 ≥0.35m）、Esc「无 UI 时释放鼠标」（`CursorReleased` + `cursor_release_toggle`）、
    暂停键补 `/`（`pause_toggle` 接受 `Backquote | Slash`）；`R` 键位裁决**随 Tab 背包落地**（背包内
    悬停消耗品按 `R` 使用）。
  - **新增 Tab 背包总览面板**（`hud_backpack_panel.rs`，对齐 legacy 操作表第 10 行）：左列双武器槽
    （元素 + 手持槽弹夹、`▸` 标注）+ 备用弹药池，右列 **4×3 补给品格位**（`×N` 堆叠数）；**悬停 + `R`
    使用**该格消耗品（仅上报 `PlayerInput.use_slot` 意图，扣/回仍由服务端 `combat::use_item_at` 裁决）；
    打开时释放光标并纳入 `gameplay_input_active` / `cursor_lock_system` / `cursor_release_toggle` /
    `item_wheel_input.blocked` 五处门控。
  - **分支治理**：把快照9（`0f7f9bd`）**合入 `main`** 并**删除 `wip/0.8-snapshot-8`**（本地 + 远程）——
    此前成批"返祖"的根因是快照 8/9 修复只存在于该 wip 分支、从未合入 main（详见冻结台账 E-1）。
  - **堆叠数量显示统一**：`hud_backpack_panel` / `hud_loot_panel` / `hud_item_wheel::category_slots`
    统一走 `LootItem::display_label()`。
- **验证**：`cargo-wrap check --workspace` 退出码 **0**；`cargo-wrap test -p cute_of_duty_server`
  **107 passed / 0 failed**；`cargo-wrap build --workspace --release` 退出码 **0**。
  ✅ **实机验证通过**（owner 复测：手雷可见投射物 / 跳跃 / Esc 呼出鼠标 / Tab 背包 /
  物资箱·消耗品·轮盘 全部正常）。**注**：先前一轮反馈"问题依旧"经排查为运行了 `target/release/`
  下 9:39 的**旧产物**（本轮改动只进了 debug），非代码缺陷——发布以 `--release` 重建为准。
- **关联**：[冻结任务 · 快照8 实机反馈](frozen-tasks/snapshot-8-playtest-feedback.md)（C.5/C.6/D-1/E）、
  `_ref/Cute-of-Duty-0.3.2/README.md`

---

## [0.6-Snapshot-9] · 2026-09-26 · 缝缝补补又一版——快照8 冻结项解冻修复

- **变更类型**：Fix（冻结项解冻修复 + 物品搬运形态统一；**含输入门控与 UI 行为变更**）
- **影响模块**：`hud_item_wheel`, `hud_interact`, `hud_loot_panel`, `hud_vitals`, `menu/pause`,
  `net/pilot`, `launcher`（客户端）；`interact`, `items`（服务端）
- **兼容性**：**兼容**（无协议字段/线格式变更；仅客户端 UI 形态与输入门控行为调整）
- **迁移指南**：不适用（非 `x+1`）
- **内容**：
  - 修 **WASD 冻结**（轮盘 `held_key` 残留 + Esc 提升为二级面板优先关闭）；
  - 修 **3/4 消耗品无响应**（短按速用 / 长按轮盘，无可用物品推 HUD 播报）；
  - **统一物品搬运**：物资箱 / 补给台改 **4×3 双向格位面板**（左键拖拽 + Shift+左键），
    与仓库选装同形态，补拖拽幽灵（`LootGhost`）；
  - 格位面板打开即释放鼠标（`cursor_lock_system` 覆盖全部指针型面板）；
  - 根治「3/4 用后数量不减」（会话收尾改走 `reset_session` 保留 `pending_slot`）；
  - **扩物品表**：掉落池 **12 → 20**。
- **验证**：`cargo-wrap check --workspace` 退出码 **0**（发布时随 tag `0.6-Snapshot-9` 打点）
- **关联**：[冻结任务 · 快照8 实机反馈](frozen-tasks/snapshot-8-playtest-feedback.md)

---

## [0.6-Snapshot-7] · 2026-09-26 · 架构边界体系落地 + 扁平化清理（**无玩法变更**）

- **变更类型**：Refactor（规范修订 + 目录清理 + 冷数据路径修正；**不含任何玩法行为变更**）
- **影响模块**：全仓库（规范层）、`ServerCode/storage`、`ServerCode/main.rs`、`HostCode/launcher`（删除空目录）
- **兼容性**：**兼容** —— 无协议字段/语义变更，客户端与服务端无需同版本强绑
  - **例外（数据布局，非线格式）**：**默认**冷数据目录由 `ServerCode/data/profiles/**profiles**/` 修正为
    `ServerCode/data/profiles/`（原代码默认路径多嵌套一层，与其自述布局矛盾）。现存档案已随本快照迁移。
- **迁移指南**：不适用（非 `x+1`）。**仅默认路径受影响**：仓库内现存档案已上移一格。
  设了 `COD_DATA_DIR` 的用户**不受影响**（环境变量路径直通，未参与本次修正）。
- **内容**：
  - CONTRIBUTING 第六～十二章**逐条封闭 8 条边界漏洞**（判据/命名/契约层载体/L2 集合/铁律4/`module.md` 过渡态/版本号映射/事件清单责任）。
  - 同步 `module-boundaries.md`（`interact` 定级、事件清单责任、过渡纪律；冲突 4 复核标注"前提已不成立"）。
  - 目录清理：删空目录 `HostCode/render/`；修冷数据双重嵌套。
- **验证**：`cargo-wrap check --workspace` 退出码 **0**；`cargo-wrap test -p cute_of_duty_server` **94 passed / 0 failed**
- **关联**：CONTRIBUTING.md、[module-boundaries](architecture/module-boundaries.md)、[ADR 0001–0004](adr/)

---

## [未发布] · 2026-09-26 · 模块化单体目标架构确立（文档先行，未改代码）

- **变更类型**：Refactor（仅新增文档与规范，**未改任何代码**）
- **影响模块**：全仓库（规范层）
- **兼容性**：**兼容**（本次不含任何行为变更）
- **迁移指南**：不适用（非 `x+1`）
- **内容**：
  - 新增 [ADR 0001](adr/0001-modular-monolith-event-bus.md)：确立「模块化单体 + 事件总线 + Trait 接口」，
    模块间禁直接调用、数据所有权唯一、依赖无环、契约机器可读。
  - 新增 [ADR 0002](adr/0002-backpack-domain-split.md)：裁决 `items` / `inventory` 双背包冲突，**按域切开**——
    局内（`items` 格位 / `combat` 手持槽，一局，不落盘）与局外（`inventory` 经济 / `equipment` 装备实例，跨局，落盘）分属不同模块，永不互相 `use`。
  - 新增 [ADR 0003](adr/0003-contract-crate.md)：抽出独立契约 crate `cute_of_duty_contract`（线格式类型 + 跨域载荷 + 共享常量 + Port Trait），
    `HostCode` 不再依赖 `ServerCode`。**将来会新增 workspace 第 3 个成员 crate。**
  - 新增 [ADR 0004](adr/0004-client-layer-convergence.md)：客户端表现层收敛——`flow` 拥有唯一 `ModalState`（`hud`/`net` 只读 `blocks_gameplay_input()`，禁 `use crate::menu::`）；
    `launcher` 本轮瘦身为纯装配。
  - 新增 [模块边界总览](architecture/module-boundaries.md)：服务端 18 个 / 客户端 7 个模块的边界、
    数据所有权、接口、成熟度 L0–L3；**5 项冲突已全部裁决**。
  - 新增 [stop-doing.md](stop-doing.md)：冻结区（本轮未验证的功能 + legacy 操作表未还原项）+ 本轮冻结裁决。
  - 新增 [契约 protocol.yaml](contracts/protocol.yaml)：线格式机器可读契约（v0.8.0 草案）。
  - 更新 `CONTRIBUTING.md`：增补架构/Rust/文档/开源四组规范与模块分级审核流程。
- **验证**：文档评审（owner 已裁决全部 5 项冲突）
- **关联**：ADR 0001、ADR 0002、ADR 0003、ADR 0004

> **预声明（尚未发生）**：采纳 ADR 0003 后，workspace 将新增成员 `ContractCode`（`cute_of_duty_contract`），
> 且 `HostCode/Cargo.toml` 将移除对 `cute_of_duty_server` 的依赖。实施时**必须**在本文件追加一条
> **Breaking（`y+1`）** 记录并附迁移指南——因为客户端与服务端的依赖拓扑发生物理变更。
> 当前**未实施**（代码全面暂停），故本条仅为预声明。

---

## [0.8.0-Snapshot-8] · 2026-09-26 · 战局内格位制物资（**未验证，不计入发布**）

- **变更类型**：**Breaking**（线格式 + 输入语义）
- **影响模块**：`items`(新), `combat`, `interact`, `net/protocol`, `net/session`, `net/broadcaster`（服务端）；
  `hud_item_wheel`(新), `hud_loot_panel`(新), `hud_interact`, `hud_vitals`, `hud_bigmap`, `menu/arsenal`, `net/pilot`, `launcher`（客户端）
- **兼容性**：**不兼容**
  - `EntitySnapshot` **删除** `medkit` / `grenade` 字段 → 老客户端读不到会直接编译失败（Rust 静态类型）；
  - `PlayerInput` **删除** `use_medkit` / `use_grenade`，**新增** `use_slot: Option<u8>`；
  - `EntitySnapshot` **新增** `backpack: Option<Vec<Option<LootItem>>>` / `container: ...`；
  - `ClientMessage` **新增** `LootTransfer { target, dir, index }`；
  - `SupplyKind::label()` **语义变更**：由动作词（"领取弹药"）改为物品名（"步枪弹药 ×90"）。
- **迁移指南**（服务端与客户端必须**同版本**部署，本版本不提供跨版本兼容）：
  1. 客户端删除对 `PlayerInput.use_medkit/use_grenade` 的一切引用，改为上报背包格位下标 `use_slot`；
  2. 3/4 号槽的 UI 不再读 `EntitySnapshot.medkit/grenade`，改为按 `ItemCategory` 统计 `EntitySnapshot.backpack`；
  3. 物资箱交互不再发送 `InteractChoice::OpenCrate`，改为打开本地 4×3 面板并发送 `LootTransfer` 逐格搬运；
  4. 服务端 `grant_supply` 的选项文案依赖 `SupplyKind::label()`，若下游有文案快照需同步刷新。
- **验证**：❌ **未验证** —— `cargo-wrap check --workspace` 尚**未成功跑过**；
  详见 [stop-doing.md A 节与 D 节](stop-doing.md)。**在通过 D 节清单前，本条不得升级为正式发布。**
- **关联**：stop-doing.md（A1/A3/A4）

---

## 历史记录（0.7 及更早）

> 0.7-Snapshot-7 及更早版本的变更记录在迁移前由 `README.md` 的版本表承载。
> **待办**：把它们按上述格式回填到本文件（`F` 表 1 类工作），回填完成前 README 版本表仍为准。
