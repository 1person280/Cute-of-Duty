# Host.flow · module.md

> 客户端游玩流程子模块（`HostCode/flow/`）。分层定位：**表现层 · 应用流程状态机与全局资源**。持有跨模块的应用级全局资源（状态机 / 字体 / 本人实体 / 目录副本 / 设置 / 模态仲裁），并是**模态仲裁资源 `ModalState` 的唯一所有者**。见 [ADR 0004](../adr/0004-client-layer-convergence.md)。

## 边界

**本模块负责**：
- **状态机**：`AppState`（`Loading` → `MainMenu` → `InGame`）与 `OnEnter`/`OnExit` 驱动。
- **全局资源**：`CjkFont`（内嵌中文字体）、`LocalPlayer`（本人权威实体 ID）、`ModelCatalog` / `LoadoutPresets` / `WorldCatalog`（服务端下发目录的内存副本）、`AimRig`（视角/瞄准）、`ConnectLatency` / `LiveLatency`、`Announcements` / `KillCount` / `SeqCounter`、`GameSettings`（会话内设置值域）。
- **下行控制消息路由**：`route_control_messages`——排空 `ControlBuffer`，把握手 / Rtt / Announce / Kill / ReturnToMenu / `PresetCatalog` / `WorldCatalog` 分派到对应资源或状态迁移；含 Loading 超时兜底。
- **Loading 屏**：`loading`（惰性生成 + 握手驱动进度 + 最短可见时长 + 任意键跳过）。
- **模态仲裁**：`ModalState`（唯一所有者）+ `ModalKind` / `ModalChange` / `PauseOpenRequest` 事件 + `apply_modal_changes` 唯一写入点；提供 `blocks_gameplay_input` / `releases_cursor` / `escape_consumed_by_modal` 单一判定入口。
- **设置数据与规则**：`GameSettings` 值域 / 步进 / 显示文本（UI 构建器留在 `menu`）。
- **文本适配层**：`style()` / `text()`（bevy 0.15 无 `TextBundle` 后的本地替代）。

**本模块不负责**（明确划出）：
- 任何**模拟判定**（选装资格 / 进场 / 撤离距离）——一律服务端权威。
- **不构建**菜单 / HUD / 设置面板 UI——归 `menu`、`hud`。
- **不直接读**任何模块的面板资源来判输入门控——统一经 `ModalState::blocks_gameplay_input()`。

## 数据所有权

| 数据 | 类型 | 是否可变 | 谁可写 |
|---|---|---|---|
| 应用流程状态 | `State<AppState>` | 可变 | 仅 `flow`（`route_control_messages` / `loading_tick`） |
| 模态仲裁 | `ModalState` | 可变 | **仅 `flow::apply_modal_changes`**（生产者只发 `ModalChange`） |
| 本人实体 / 视角 | `LocalPlayer` / `AimRig` | 可变 | `LocalPlayer` 仅 `route_control_messages`；`AimRig` 由 `world::camera` 写、`net::input_system` 只读 |
| 服务端目录副本 | `ModelCatalog` / `LoadoutPresets` / `WorldCatalog` | 可变 | `ModelCatalog` 经 `net::sync_catalog_from_pool`；后两者仅 `route_control_messages` |
| 设置 | `GameSettings` | 可变 | `menu` 设置面板写、`world` 读 |
| 字体 / 延迟 / 通告 / 计数 / 序号 | 各 `Resource` | 可变 | 见上「接口」对应系统 |

> **所有权原则**：`flow` 持应用级全局状态，其它模块只读或经事件写入；`ModalState` 是"新增模态只加一个变体、消费侧一行不改"的收敛点。

## 接口

| 接口 | 形式 | 契约 | 兼容性承诺 |
|---|---|---|---|
| `AppState`（`Loading`/`MainMenu`/`InGame`） | 枚举 | 三态游玩流程 | 表现层内部 |
| `setup_global` | 系统 | `Startup`：注册字体与全部全局资源 | 内部编排 |
| `route_control_messages` | 系统 | 下行控制通道 → 资源/状态 | 消费 `net::ClientInbound` |
| `ModalState::{get, set, blocks_gameplay_input, releases_cursor, escape_consumed_by_modal, other_modal_open}` | 方法 | 模态唯一查询/判定入口 | 被 `hud`/`net`/`menu` 只读 |
| `spawn_loading` / `loading_tick` | 系统 | Loading 屏生成与刷新 | 仅 `Loading` 态 |
| `style(fonts, size, color)` / `text(contents, style)` | 函数 | 文本样式/实体构造适配层 | 全表现层复用 |
| `setting_step` / `setting_label` / `apply_setting_step` | 函数 | 设置步进与文本规则 | 被 `menu` 消费 |

## 事件

| 事件名 | 方向 | 载荷契约 | 订阅者 |
|---|---|---|---|
| `ModalChange { kind, open }` | 生产者（`hud`/`menu`）→ `flow` | 模态差量事实 | `flow::apply_modal_changes`（唯一落账） |
| `PauseOpenRequest` | `hud`（操作按钮组）→ `menu` | 空载荷（请求打开暂停） | `menu::open_pause_on_request` |
| `ClientInbound::*`（消费） | `net` 网络线程 → `flow` | `Connected`/`Rtt`/`Server(..)` | `route_control_messages` |

> ⚠️ 跨模块事件清单与订阅关系图仍属 [module-boundaries 第七节](../architecture/module-boundaries.md) 待补项。

## 成熟度

**L1（内部稳定：crate 内可依赖）** —— 依据：`flow` 是客户端应用的全局状态与编排中枢，被全部表现层模块读取；不定义对外线格式/契约。

- 破坏性变更纪律：须写 ADR（如新增/删除 `ModalKind` 变体影响全部消费侧）+ 1 名 reviewer。
- 已发生示例：0.14.0 起 `GameSettings` 由 `menu` 迁至 `flow`（消除 `world` → `menu` 的反向 `use`）。
