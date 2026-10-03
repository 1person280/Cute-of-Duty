# Host.hud · module.md

> 客户端战斗内 HUD 子模块（`HostCode/hud/`）。分层定位：**表现层 · 战斗内 UI**。所有数值（血量 / 弹药 / CD / 击杀数 / 交互目标）都来自服务端快照，本模块只把快照画到 UI，并就地触发意图（进场撤离 / 使用 / 交互由服务端裁决）。

## 边界

**本模块负责**：
- **常驻 HUD 面板族**：`vitals`（血条/文本）、`crosshair`（准星）、`minimap`、`bigmap`（战术大地图 + 撤离）、`skills`（技能/击杀播报）、`kill`（击杀计数）、`alert`（告警）、`root`（HUD 根 + 撤离交互）。
- **模态面板族**：`interact`（F 交互二级面板）、`loot`（物资箱 4×3 格位 + 拖拽）、`item/wheel`（3/4 径向轮盘）、`backpack`（Tab 背包总览）、`button/panel`（B 可点击操作按钮组）、`grenade/hint`（持雷提示）。
- **模态发布**：`modal::publish_modal_changes`——每帧对照七个 HUD 侧模态源资源与 `flow::ModalState`，有差异才发 `ModalChange`（唯一发布点，杜绝多路径漏发）。
- **输入门控**：`bigmap::gameplay_input_active`（统一取自 `flow::ModalState::blocks_gameplay_input`）。

**本模块不负责**（明确划出）：
- **不直连 `menu`**——打开暂停改发 `PauseOpenRequest` 事件（冲突 3 落地判据）；`hud` 内不得出现 `use crate::menu::`。
- **不裁决**任何数值——血量/弹药/CD/命中/撤离距离一律服务端权威。
- **不持模态真相**——`ModalState` 唯一所有者是 `flow`，本模块只投影为源资源并只读仲裁态。

## 数据所有权

| 数据 | 类型 | 是否可变 | 谁可写 |
|---|---|---|---|
| HUD 根与子节点句柄 | 各 `Component`（`BigMapRoot` 等） | 生成期 | 仅 `hud`（`spawn_hud` / OnEnter） |
| 模态门控源资源 | `BigMapOpen` / `InteractState` / `ItemWheelState` / `LootPanelState` / `BackpackPanelState` / `ButtonPanelState` / `HeldGrenadeState` | 可变 | 仅 `hud` 对应面板系统 |
| 模态仲裁只读视图 | `Res<ModalState>` | — | 只读（唯一写者 `flow`） |

> **所有权原则**：每个面板资源是"模态开关 + 面板局部态"的单一所有者；对仲裁态只读，写入一律发 `ModalChange`。

## 接口

| 接口 | 形式 | 契约 | 兼容性承诺 |
|---|---|---|---|
| `spawn_hud` | 系统 | `OnEnter(InGame)` 构建 HUD 树 | 表现层内部 |
| `gameplay_input_active` | 运行条件 | 唯一输入冻结判定（转发 `flow::ModalState`） | 被 `launcher` 作 `run_if` |
| 各面板 `*_toggle` / `*_input` / `sync_*` / `reset_*` | 系统 | 面板开关 / 输入 / 刷新 / 复位 | 由 `launcher` 接线 |
| `publish_modal_changes` | 系统 | HUD 侧模态差量发布 | 与 `menu::publish_modal_changes` 同批 |
| `HeldGrenadeState` | 资源 | 服务端权威持雷态的客户端派生（由快照写入） | 相机/上行/Esc 共读 |

## 事件

| 事件名 | 方向 | 载荷契约 | 订阅者 |
|---|---|---|---|
| `ModalChange { kind, open }` | `hud` → `flow` | 七个 HUD 侧模态的差量 | `flow::apply_modal_changes` |
| `PauseOpenRequest` | `hud`（按钮组）→ `menu` | 空载荷 | `menu::open_pause_on_request` |

> ⚠️ 跨模块事件清单与订阅关系图仍属 [module-boundaries 第七节](../architecture/module-boundaries.md) 待补项。

## 成熟度

**L1（内部稳定：crate 内可依赖）** —— 依据：`hud` 是战斗内 UI 的完整实现，被 `launcher` 接线；不定义对外线格式。

- 破坏性变更纪律：须写 ADR + 1 名 reviewer。
- 已发生示例：冲突 3 落地后 `hud`/`net` 输入门控统一改读 `blocks_gameplay_input()`，`use crate::menu::` 已清零。
