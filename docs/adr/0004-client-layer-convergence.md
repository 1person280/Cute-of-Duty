# ADR 0004 · 客户端表现层收敛：`ModalState` 事件仲裁 + `launcher` 瘦身

- **状态**：已接受（Accepted）
- **日期**：2026-09-26
- **决策者**：项目 owner
- **影响范围**：`HostCode/flow`、`HostCode/hud`、`HostCode/menu`、`HostCode/net`、`HostCode/launcher`、`HostCode/world`
- **依据**：[ADR 0001](0001-modular-monolith-event-bus.md) 铁律 1「禁跨模块直接调用」与「同层禁横向调用」；[module-boundaries.md 冲突 3 / 冲突 4](../architecture/module-boundaries.md)

---

## 背景（Context）

客户端表现层有两处边界失守：

**其一 · 同层横向调用（冲突 3）**
`hud_bigmap.rs` 的 `gameplay_input_active` 直接读 `crate::menu::PauseMenu` 来判断"暂停中/大地图开启时不响应玩法输入"。
`hud` 与 `menu` 是**同一层**的两个模块，`hud → menu` 是横向依赖。它带来的连锁问题：
每新增一个模态面板（暂停 / 大地图 / 交互面板 / 物资箱 / 轮盘），都要在 `hud` 与 `net` 里再补一条 `use crate::menu::…` 与一个 `&& !xxx.open` 条件 —— 判定逻辑散落各方，漏一处就是"暂停时还能开枪"。

**其二 · 装配层兼任表现宿主（冲突 4）**
`launcher/mod.rs` 既是 `App` 的装配者（`add_systems`），又承载了几乎全部渲染表现代码（相机 Rig、体素绘制、HUD 绘制等），且直接 `use crate::hud::*` / `crate::menu::*` / `crate::net::*`。它是全客户端最大的文件，也是"上帝文件"风险最高的地方。

## 决策（Decision）

### 决策 1 · `flow` 拥有唯一 `ModalState`，其余模块只读，写入靠事件

- **所有者**：`flow`（应用流程状态机，已拥有 `AppState`），新增资源：

```rust
// HostCode/flow —— 唯一所有者，其它模块只读
pub struct ModalState {
    pub pause: bool,          // 暂停菜单
    pub bigmap: bool,         // 战术大地图
    pub interact: bool,       // F 交互面板
    pub loot: bool,           // 物资箱 4×3 面板
    pub wheel: bool,          // 3/4 速用轮盘
}
impl ModalState {
    /// Why: 任何模态打开都意味着"玩法输入必须冻结"，
    /// 由本类型统一回答，避免各模块各自拼 `&& !a && !b`。
    pub fn blocks_gameplay_input(&self) -> bool { /* … */ }
}
```

- **写入路径**：`menu` / `hud` 打开或关闭面板时**发事件**（`<域>.modal_opened` / `<域>.modal_closed`），载荷为 `ModalKind`；`flow` 订阅并把状态写进 `ModalState`。
- **读取路径**：`hud` / `net` 只读 `Res<ModalState>`，调 `blocks_gameplay_input()`；**不再 `use crate::menu::…`**。
- **违反判定**：任何模块出现 `use crate::menu::` 即为违规（`menu` 是面板实现，不是仲裁者）。

### 决策 2 · `launcher` 本轮瘦身为"纯装配"，表现代码归还各模块

`launcher/mod.rs` 只允许做四件事：

1. 装载 `bevy_dylib` 动态库（`bevy_dynamic_plugin`）；
2. `init_state::<AppState>()`（含 `enable_state_scoped_entities`）+ `init_resource`；
3. 把各模块的 `pub(crate) fn` 系统按顺序 `add_systems`；
4. `.run()`。

**禁止**：定义组件 / 资源、写渲染绘制实现、写玩法判定、持有相机或 HUD 的具体状态。

| 从 `launcher` 迁出 | 去向 |
|---|---|
| 相机 Rig（越肩/自由视角/过渡） | `world/camera.rs`（已存在，收编 `launcher` 内的相机代码） |
| 体素/模型绘制 | `world` 的绘制子模块 |
| HUD 绘制 | `hud/*`（各自面板文件） |
| 主菜单/暂停/设置装配态 | `menu/*` |
| 输入门控（冻结判定） | 统一改为"读 `ModalState`"，由各输入系统自己判 |

**装配即引用**：`launcher` 仍会 `use crate::hud::xxx_system`（这是装配者唯一被允许的跨模块动作），但**不得** `use crate::hud::某个组件类型` 去自己写逻辑。

### 决策 3 · 600 行红线对 `launcher` 同样生效

瘦身后若 `launcher/mod.rs` 仍逼近 600 行，拆为 `launcher/mod.rs`（装配入口）+ 语义化子文件（如 `launcher/dylib_loader.rs`），`mod.rs` 保持薄网关。

## 迁移路径（**文档先行阶段不实施**）

1. 在 `HostCode/flow/` 新增 `ModalState`（所有者）与其订阅系统；在 `flow/module.md` 写明所有权。
2. 在 `docs/architecture/module-boundaries.md` 第六节登记事件：`menu.modal_opened` / `menu.modal_closed` / `hud.modal_opened` / `hud.modal_closed`，载荷 `ModalKind`。
3. `menu` / `hud` 各面板的开关点改为发事件（不再直接改自己的 `open` 供他模块读）。
4. `hud_bigmap.rs` 的 `gameplay_input_active` 改为读 `ModalState::blocks_gameplay_input()`，删除 `use crate::menu::PauseMenu`。
5. `net/pilot.rs` 的输入门控同样改为读 `ModalState`（替换现有 `wheel.open` / `loot.open` 之类散落条件）。
6. `launcher/mod.rs` 逐段把表现代码搬回 `world` / `hud` / `menu`，只留装配。
7. 收官判据：`grep -r "use crate::menu" HostCode/hud` 为空；`launcher/mod.rs` 不含任何组件/资源定义。

## 后果（Consequences）

**正向**
- 新增模态面板**只需**：发事件 + 在 `ModalKind` 加一个 variant。`hud`/`net` 的输入门控一行不用改（这是本次决策最大的收益）。
- `launcher` 回归"装配层"定位，文件规模可控（符合 ≤600 行红线）。
- 与 ADR 0003 配合后，客户端形成 `launcher（装配）→ 各模块 → contract` 的清晰单向图。

**代价**
- `ModalState` 成为新的**热点资源**：必须在 `flow/module.md` 明确"只允许 `flow` 写"。
- 搬迁期间 `launcher` 与 `world`/`hud` 会短暂同时持有相关代码，需一次性搬完避免双份。

## 未决事项（Open Questions）

- `ModalKind` 的完整 variant 集合（是否把"设置子面板""加载屏"也纳入模态仲裁）。
- 事件命名最终形态：`menu.modal_opened` 还是统一归 `modal.opened`（发 `ModalKind` 区分来源）。
- `ModalState` 与 `AppState` 的关系：`AppState::InGame` 之外是否强制 `ModalState` 全 false。

## 替代方案（Alternatives considered）

- **共享只读 `PauseMenu` 类型（把"是否打开"抽成纯数据类型给 `hud` 读）** — 否决：只解决 `pause` 一个模态；每加一个面板就要再抽一个类型，`hud` 仍要知道 `menu` 的所有面板存在。
- **`hud` 保留现状，仅在文档记为已知债** — 否决：模态每增加一个就多一处漏判，正是"暂停时还能操作"这类 bug 的温床。
- **`launcher` 不瘦身，正式承认其为渲染宿主** — 否决：承认它就是承认"装配层可以写业务"，与铁律 1 直接冲突，且它会持续膨胀成上帝文件。
