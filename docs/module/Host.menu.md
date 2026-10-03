# Host.menu · module.md

> 客户端主菜单子模块（`HostCode/menu/`）。分层定位：**表现层 · 局外 UI**。负责主界面布局、模式选择、仓库选装浮层、设置浮层与暂停菜单，仅把选装/进场意图上行，不做本地判定（资格由服务端权威裁决）。

## 边界

**本模块负责**：
- **主菜单**（`mod.rs`）：整屏 UI 树（标题区 / 模式面板 / 状态行 / 退出），根节点挂 `StateScoped(MainMenu)`。
- **交互与样式**：`behaviour`（`main_menu_interaction` / `main_menu_style` / `main_menu_loadout` / `tick_grace` 输入保护期）。
- **模式选择**：`mode/panel`（分类 + 模式清单 + `SelectedMode` / `SelectedCategory`）。
- **仓库选装浮层**：`arsenal`（复用双清单浮层 + 拖拽/选中 `ArsenalDrag` / `ArsenalSelection` / `ArsenalVisible` + `refresh`）。
- **设置浮层**：`settings`（面板构建 + `settings_apply_fov` / `settings_apply_ambient` 应用系统；数据规则在 `flow::GameSettings`）。
- **暂停菜单与光标**：`pause`（`pause_toggle` / `pause_menu_interaction` / `teardown_pause` / `pause_closed` 运行条件；`cursor_lock_system` / `cursor_release_toggle` 光标锁定；`open_pause_on_request` 消费暂停请求）。

**本模块不负责**（明确划出）：
- **不裁决**进场资格 / 选装合法性——服务端权威。
- **不持设置真相**——`GameSettings` 归 `flow`；本模块只构建 UI 并转出口旧路径 `crate::menu::GameSettings`。
- **不被 `hud` 直连**——`hud` 打开暂停经 `PauseOpenRequest` 事件（见 [Host.hud](./Host.hud.md)）。

## 数据所有权

| 数据 | 类型 | 是否可变 | 谁可写 |
|---|---|---|---|
| 主菜单节点句柄 | `MainMenuUi` / `MenuGrace` | 生成期 | 仅 `menu`（`spawn_menu` / OnEnter） |
| 选装选择/拖拽 | `ArsenalSelection` / `ArsenalDrag` / `ArsenalVisible` | 可变 | 仅 `menu::arsenal` |
| 模式选择 | `SelectedMode` / `SelectedCategory` | 可变 | 仅 `menu::mode` |
| 暂停态 | `PauseMenu` | 可变 | 仅 `menu::pause` |
| 光标软开关 | `CursorReleased` | 可变 | 仅 `menu::pause` |
| 设置值 | `GameSettings` | 可变 | 面板写、`world` 读（所有者 `flow`） |

> **所有权原则**：菜单内部 UI 状态自持；跨模块全局态（设置 / 模态）分别归 `flow`，本模块只读转出口或发事件。

## 接口

| 接口 | 形式 | 契约 | 兼容性承诺 |
|---|---|---|---|
| `spawn_menu` | 系统 | `OnEnter(MainMenu)` 构建整屏 | 表现层内部 |
| `pause_closed` | 运行条件 | 暂停时冻结视角/相机 | 被 `launcher` 作 `run_if` |
| `cursor_lock_system` / `cursor_release_toggle` | 系统 | 光标锁定随状态/暂停翻转 | 各状态下运行 |
| `settings_apply_fov` / `settings_apply_ambient` | 系统 | 应用设置到相机 FOV / 环境光 | 常驻 |
| `open_pause_on_request` | 系统 | 消费 `PauseOpenRequest` 装配暂停 UI | 同帧落账 |
| `publish_modal_changes` | 系统 | 发布 `Pause` / `CursorReleased` 差量 | 与 `hud` 同批 |

## 事件

| 事件名 | 方向 | 载荷契约 | 订阅者 |
|---|---|---|---|
| `ModalChange { kind, open }` | `menu` → `flow` | `Pause` / `CursorReleased` 差量 | `flow::apply_modal_changes` |
| `PauseOpenRequest` | `hud` → `menu` | 空载荷 | `menu::open_pause_on_request` |

> ⚠️ 跨模块事件清单与订阅关系图仍属 [module-boundaries 第七节](../architecture/module-boundaries.md) 待补项。

## 成熟度

**L1（内部稳定：crate 内可依赖）** —— 依据：`menu` 是局外 UI 的完整实现，被 `launcher` 接线；不定义对外线格式。

- 破坏性变更纪律：须写 ADR + 1 名 reviewer。
- 已发生示例：0.14.0 起 `GameSettings` 迁 `flow`，本模块经 `pub(crate) use crate::flow::GameSettings` 保持旧路径。
