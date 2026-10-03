# Host.launcher · module.md

> 客户端表现层装配入口（`HostCode/launcher/`）。分层定位：**表现层 · 装配者**。与 `flow`/`net`/`menu`/`hud`/`world`/`shared` 六个顶层模块平级，只把它们的系统挂进 Bevy App，不写任何表现实现。见 [ADR 0004](../adr/0004-client-layer-convergence.md)。

## 边界

**本模块负责**：
- **通道装配**：建快照 / 下行控制 / 远程资源 / 上行意图四路 `mpsc` 通道，起后台网络线程（`net::run_network`）。
- **App 装配**：`DefaultPlugins`（`AssetPlugin.file_path` 固定为编译期 `CARGO_MANIFEST_DIR`）+ `insert_resource` / `init_resource` / `add_event` / `init_state::<AppState>()` / `enable_state_scoped_entities`。
- **系统接线**：按 `Startup` / `Update`（全局常驻、Loading、MainMenu、InGame）/ `OnEnter` / `OnExit` 分组 `add_systems`，用 `.chain()` 锁定顺序敏感链。
- **运行**：`.run()` 拉起游玩主循环。

**本模块不负责**（明确划出，ADR 0004 收官判据）：
- **不定义**任何组件 / 资源（资源构造器 `::new()` 属各自模块；本模块只 `insert`/`init`）。
- **不写**渲染实现、玩法判定、面板布局。
- **不持有**任何模拟状态——热数据一律在服务端，本模块只做接线。

## 数据所有权

| 数据 | 类型 | 是否可变 | 谁可写 |
|---|---|---|---|
| Bevy `App` | `App` | 装配期 | 仅 `launcher`（`run` 内一次性） |
| 四路通道端点 | `mpsc::Sender/Receiver` | 装配期 | 仅 `launcher`（创建后移交给对应模块资源） |

> **所有权原则**：`launcher` 是**唯一编排者**——只有它可以同时持有多个模块的句柄并把它们接起来（同 [module-boundaries 第二节](../architecture/module-boundaries.md) 对 `main.rs` 的定位）。

## 接口

| 接口 | 形式 | 契约 | 兼容性承诺 |
|---|---|---|---|
| `run(addr: &str)` | 函数 | 唯一公开入口：`addr` 为服务端监听地址，装配并运行 App | 表现层内部，改动不涉外 |

## 事件

| 事件名 | 方向 | 载荷契约 | 订阅者 |
|---|---|---|---|
| —（本模块仅 `add_event` 注册，不生产/消费） | — | `ModalChange` / `PauseOpenRequest` 由 `flow` 定义，`hud`/`menu` 生产消费 | 见 [Host.flow](./Host.flow.md) |

## 成熟度

**L1（内部稳定：crate 内可依赖）** —— 依据：`launcher` 是客户端 binary 的唯一入口，被 `main.rs` 依赖；不定义对外线格式/契约，故非 L2。

- 破坏性变更纪律：须写 ADR + 1 名 reviewer。
- 收官判据（ADR 0004）：`launcher/mod.rs` 不含任何组件/资源定义；超 600 行则拆语义化子文件，`mod.rs` 保持薄网关。
