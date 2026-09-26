# 模块边界总览（v0.2 · 5 项冲突已全部裁决）

> **状态：第五节 5 项冲突已全部裁决（2026-09-26）；代码全面暂停，正在按第七节清单补 `module.md` 与契约。**
> 本文档回答三个问题：**每个模块拥有什么数据**、**模块之间允许怎么说话**、**它现在有多成熟（L0–L3）**。
>
> 目标架构一句话：**模块化单体 + 事件总线 + Trait 接口；模块间禁止直接调用；数据所有权划清；依赖无环。**
> 终局判据：**代码边界锁死，将来拆微服务无需重写。**

相关文档：[stop-doing.md](../stop-doing.md) · [barek-history.md](../barek-history.md) · [ADR 0001](../adr/0001-modular-monolith-event-bus.md) · [ADR 0002](../adr/0002-backpack-domain-split.md) · [ADR 0003](../adr/0003-contract-crate.md) · [ADR 0004](../adr/0004-client-layer-convergence.md) · [契约 protocol.yaml](../contracts/protocol.yaml)

---

## 一、五条铁律（与 CONTRIBUTING 同源）

| # | 铁律 | 落地判据 |
|---|---|---|
| 1 | **禁直接调用** | A 模块不得 `use B::内部类型` 去调用 B 的函数。跨模块只允许经 ①Trait 接口 ②事件。 |
| 2 | **数据所有权唯一** | 每份可变状态有且只有一个 owning 模块；其它模块只能持有快照/句柄，不能持有 `&mut`。 |
| 3 | **依赖无环** | 模块依赖图必须是 DAG。新增依赖前必须画图，禁"互相 use"。 |
| 4 | **接口先于实现** | 对外能力先定义 Trait（`pub trait XxxPort`），实现可换、可 mock。 |
| 5 | **契约机器可读** | 跨模块/跨进程的载荷用 YAML 契约描述（`docs/contracts/`），代码是契约的实现而非定义。 |

### 成熟度分级（L0–L3）

| 级别 | 含义 | 变更纪律 | 审核 |
|---|---|---|---|
| **L0** | 试验田：结构随时推翻 | 随便改 | 无 |
| **L1** | 内部稳定：crate 内可依赖 | 破坏性变更须写 ADR | 1 名 reviewer |
| **L2** | 对外契约：被其它 crate / 线格式依赖 | 破坏性变更须**迁移指南** + 契约 YAML 同步 + `x+1` 版本号 | **≥2 名 reviewer（maintainer 必须参与）** |
| **L3** | 冻结：只允许加性变更 | **语义不可改**，只可增字段/增事件 | 同 L2 |

> 协议版本号 `x.y.z`：**`x+1` = 不兼容，必须附迁移指南**；**`y+1` = 加性兼容**；**`z+1` = 修 bug，语义不变**。
> 详见 [契约](../contracts/protocol.yaml) 与 [BarekHistory](../barek-history.md)。

---

## 二、分层（依赖只许自上而下）

```
┌─ 表现层（HostCode/cod1，Bevy 0.14）────────────────────────────┐
│  launcher(纯装配)  flow  menu  hud  world  shared  net(适配)   │
└───────────────────────────┬───────────────────────────────────┘
                            │ 只依赖契约 crate（不得依赖 ServerCode）
┌─ 契约层 ContractCode ──────┴───────────────────────────────────┐
│  cute_of_duty_contract：线格式类型 + 跨域载荷类型 + 共享常量    │
│  + Port Trait（`XxxPort`）。零 bevy、零模拟逻辑、零 fs。        │
│  权威描述：docs/contracts/*.yaml                               │
└───────────────────────────┬───────────────────────────────────┘
                            │
┌─ 领域层（ServerCode/cute_of_duty_server，零 bevy）─────────────┐
│  combat  damage  element  items  inventory  equipment  operator│
│  player  gamemode  interact  map  model  entity  engine        │
└───────────────────────────┬───────────────────────────────────┘
                            │
┌─ 基础设施层 infra ─────────┴───────────────────────────────────┐
│  config  storage  hal  net(协议编解码/序列化/传输)              │
└───────────────────────────────────────────────────────────────┘
```

**依赖方向规则**
- 表现层 → 契约层 → 领域层 → 基础设施层，**单向**；反向依赖一律违规。
- **`HostCode` 不得依赖 `ServerCode`**（判据：`HostCode/Cargo.toml` 里没有 `cute_of_duty_server`）。
  见 [ADR 0003](../adr/0003-contract-crate.md)。
- 同层之间**禁止横向直接调用**，只能通过事件总线。
- `config` 是**只读单例**：任何模块可读，**禁止任何模块写入**。
- `main.rs`（两端的 binary 入口）是**唯一的编排者**：只有它可以同时持有多个模块的句柄并把它们接起来。
  客户端的装配者 `launcher` 只允许 `add_systems`（引用各模块导出的系统函数），**不得定义组件/资源、不得写表现实现**。
  见 [ADR 0004](../adr/0004-client-layer-convergence.md)。

---

## 三、服务端模块边界表

| 模块 | 职责（边界内） | 拥有的数据（所有权） | 对外接口 | 发出 / 消费事件 | 成熟度 |
|---|---|---|---|---|---|
| `config` | 配置加载（`include_str!` 嵌入 + 运行时覆盖） | 全部配置表（只读） | `Config` 只读访问器 | 无 | L1 |
| `element` | 元素互斥/反应规则 | 反应规则表 | 纯函数 | 无 | L1 |
| `map` | 地图布局（lawn / training）；叶子数据 | `MapLayout`（静态） | `layout()`、`StationKind` | 无 | L1 |
| `model` | 体素模型预设（冷数据，供客户端取用） | `ModelPreset` | `model_preset()` | 无 | L1 |
| `entity` | 世界实体与组件容器 | `World`、全部实体组件 | `World` 读写 | 无 | L1 |
| `engine` | 模拟引擎原语（双缓冲、爆炸预计算缓存） | 缓存 | 纯结构 | 无 | L0 |
| `player` | 玩家冷档案的热副本 | 玩家档案 | 查询/更新 | 无 | L1 |
| `operator` | 干员名册 + 权威切干员 | 名册 | `roster()`、切换 | 无 | L1 |
| `equipment` | **局外**装备实例注册表（**索引制**，跨局落盘） | 装备实例表 | 注册/查询 | 无 | L1 |
| `inventory` | **局外经济域**：货币 / 制作 / 丢弃（跨局落盘）；**不含战局格位** | 货币与制作一致性守卫 | `InventoryError`、`CraftRequest` | Event 回执 | L1 |
| `items` | **局内物资域**：`Backpack`(12) / `Container`(12) / `transfer()` / 速用类别（一局，不落盘） | 格位内容 | `transfer`、`Backpack` | 无（被编排） | **L0（本轮新增）** |
| `combat` | 战斗：命中/弹道/技能/手雷 | 战斗意图与结算结果 | `CombatIntent`、`use_item_at` | Event 回执 | L1 |
| `damage` | 伤害管线（packet → resolver → effect） | 伤害结算中间态 | 纯函数 + 事件 | 消费伤害包 | L1 |
| `gamemode` | 玩法模式规则 | 模式状态 | 模式查询 | 无 | L0 |
| `interact` | 站点/拾取交互与结算（`settle`、`InteractChoice`） | 交互目标与结算 | `INTERACT_RANGE`、`SupplyKind` | Event 回执 | **L1→L2 待定**（定级前按 L2 流程，见 [CONTRIBUTING 第九节](../../CONTRIBUTING.md)） |
| `net` | 协议线格式、会话、AOI、广播 | 连接会话 | `protocol`、`session`、`broadcaster` | 收发消息 | **L2（线格式）** |
| `storage` | 冷数据持久化（json_log / cold_repo） | 落盘数据 | `thiserror` 错误类型 | 无 | L1 |
| `hal` | 硬件抽象（平台相关） | 无 | Trait | 无 | L0 |

> ⚠️ 表中 `?` 缺项：事件名称清单与订阅关系**待补**（需通读 `ServerCode/net/` 与 `main.rs` 的编排段）。这一栏填完之前，事件总线不得动工。

---

## 四、客户端模块边界表

| 模块 | 职责（边界内） | 拥有的数据 | 对外接口 | 成熟度 |
|---|---|---|---|---|
| `launcher` | **纯装配**：加载 `bevy_dylib`、把各模块系统挂进 App、承载全局渲染表现 | Bevy `App` | `run()` | L1 |
| `flow` | 应用流程状态机（`AppState`、加载屏） | `AppState`、`CjkFont`、`LocalPlayer` | 状态资源 | L1 |
| `net` | 客户端网络适配：快照缓冲、意图上报、延迟 | `SnapshotBuffer`、`NetOut`、`SeqCounter` | `input_system` | **L2（吃线格式）** |
| `menu` | 主菜单 / 设置 / 暂停 / 模式面板 / arsenal | 各面板资源 | 面板系统 | L1 |
| `hud` | HUD 面板族（vitals / minimap / bigmap / interact / loot / wheel / alert …） | 各 HUD 资源 | 面板系统 | L1 |
| `world` | 3D 世界表现（相机 Rig、场景、体素绘制） | `AimRig` | 相机系统 | L1 |
| `shared` | 共享主题/资产/干员元数据（**纯只读**） | `theme`、`operator_meta` | 常量与查表 | L1 |

---

## 五、已识别的边界冲突（**必须先裁决，否则新代码继续堆**）

### 冲突 1 · `items` 与 `inventory` 两套背包并存 【🟢 已裁决 · ADR 0002 · 方案 A】

- `inventory` = **索引制 + `equipment` 注册表**（货币 / 制作 / 丢弃），`pub use service::{adjust_currency, craft, discard_by_index, CraftRequest, InventoryError}`。
- `items` = **格位制**（`Backpack{slots:12}` / `Container` / `transfer()`），本轮为 3/4 速用与物资箱而新增。
- 两者都叫"背包"，都声称拥有物品数据 → **违反铁律 2（数据所有权唯一）**。

**裁决（2026-09-26，owner）：方案 A —— 按域切开。**

| 域 | 归属模块 | 数据 | 生命周期 | 是否落盘 |
|---|---|---|---|---|
| **局内（in-match）** | `items` | `Backpack{slots:12}`、`Container{slots:12}`、`LootItem`、`POOL` | 一局 | ❌ 不落盘 |
| **局内（in-match）** | `combat` | `WEAPON_SLOTS`、`DEFAULT_AMMO_POOL`（手持槽 / 弹夹） | 一局 | ❌ 不落盘 |
| **局外（out-of-match）** | `inventory` | 货币、制作、丢弃（**经济域**） | 跨局 | ✅ 落 storage |
| **局外（out-of-match）** | `equipment` | 装备实例注册表 | 跨局 | ✅ 落 storage |

**三条硬边界**
1. `items` 与 `inventory` / `equipment` **永不互相 `use`**。
2. `inventory` 不得再引入"战局内格位"语义。
3. `equipment` 不得持手持槽 / 弹夹；`combat` 不得持装备等级 / 元素。

**唯一编排者**：`ServerCode/main.rs` 把两者接起来（入局装配 → 撤离结算经事件回写）。

详见 [ADR 0002 · 背包按域切开](../adr/0002-backpack-domain-split.md)。

### 冲突 2 · 客户端跨 crate 直接依赖服务端**具体类型** 【🟢 已裁决 · ADR 0003 · 抽独立契约 crate】

- 现状：`hud_*` / `menu` / `world` 大量 `use cute_of_duty_server::{items::LootItem, interact::*, map::StationKind, operator::roster, net::protocol::*}`。
- 违反铁律 1（禁直接调用）。后果：服务端内部重构必然打断客户端 → 无兼容承诺。

**裁决（2026-09-26，owner）：抽出独立契约 crate `cute_of_duty_contract`。**

| 项 | 结论 |
|---|---|
| 新增 crate | `ContractCode`（workspace 第 3 个成员），依赖只有 `serde` + `thiserror` |
| 内容 | 线格式类型 + 跨域载荷类型（`LootItem`/`StationKind`/…）+ 共享常量 + Port Trait |
| 禁止 | 任何 bevy / fs / 网络 / 模拟逻辑 |
| 依赖方向 | `ServerCode → ContractCode ← HostCode`；**`HostCode` 不再依赖 `ServerCode`** |
| 静态表 | roster / model preset 改为下发；地图布局过渡期保留客户端只读副本并注明"以服务端为准" |
| 定义方式 | 手写类型 + YAML↔Rust 一致性测试（暂不引入代码生成） |

详见 [ADR 0003 · 抽出独立契约 crate](../adr/0003-contract-crate.md)。

### 冲突 3 · `hud` 内部横切依赖 `menu` 【🟢 已裁决 · ADR 0004 · `flow` 持 `ModalState`】

- 现状：`hud_bigmap.rs` 的 `gameplay_input_active` 直接读 `crate::menu::PauseMenu`；`launcher` 的 Update 链把 `pause_toggle` 与 HUD 系统混排。
- 违反"同层禁横向调用"。

**裁决（2026-09-26，owner）：`flow` 拥有唯一 `ModalState`，其余模块只读，写入靠事件。**

- `flow` 的定义：[`ModalState`](../adr/0004-client-layer-convergence.md)（`pause` / `bigmap` / `interact` / `loot` / `wheel`）+
  `blocks_gameplay_input()` 单一判定入口。
- `menu` / `hud` 开关面板时发事件（`ModalKind` 载荷），`flow` 订阅并落状态。
- `hud` / `net` 只读 `Res<ModalState>`；**`use crate::menu::` 出现即为违规**。
- 收益：新增模态只需"加 variant + 发事件"，输入门控一行不改。

详见 [ADR 0004 · 客户端表现层收敛](../adr/0004-client-layer-convergence.md)。

### 冲突 4 · `launcher` 承载全部渲染表现 【🟢 已裁决 · ADR 0004 · 本轮一起瘦身】

- 现状：`launcher/mod.rs` 同时是装配层**和**表现代码宿主，且直接 `use crate::hud::*` / `crate::menu::*` / `crate::net::*`。

**裁决（2026-09-26，owner）：本轮一起瘦身。**

- `launcher/mod.rs` 只允许：①装载 `bevy_dylib` ②`init_state`/`init_resource` ③`add_systems` ④`.run()`。
- 禁止：定义组件/资源、写渲染实现、写玩法判定。
- 表现代码去向：相机 → `world/camera.rs`；体素绘制 → `world`；HUD → `hud/*`；面板装配态 → `menu/*`。
- 收官判据：`launcher/mod.rs` 不含任何组件/资源定义；仍超 600 行则按语义化子文件拆分，`mod.rs` 保持薄网关。

**实测现状（2026-09-26 复核）**：`launcher/mod.rs` 已缩至 **147 行**，相机/世界/HUD/面板的**表现代码均已迁出**（改为委托 `world::*` / `hud::*` / `menu::*`）——"承载全部渲染表现"的前提**已不成立**。**残留 2 处未达标**（待 ADR 0003/0004 实施）：
1. `use cute_of_duty_server::net::protocol::{ClientMessage, EntitySnapshot}` —— 跨 crate 直连服务端类型（违铁律 1 判据②）；
2. `spawn_scene` 内 `insert_resource(CubeMesh / AmbientLight)` —— 定义资源（违 ADR 0004「launcher 禁定义组件/资源」）。

详见 [ADR 0004 · 客户端表现层收敛](../adr/0004-client-layer-convergence.md)。

### 冲突 5 · `combat` 与 `items` 的编排位置

- 现状：`items` 不依赖 `combat`（✅ 正确）；`combat::use_item_at` 依赖 `items`（✅ 单向）；但"施加返回值"的粘合写在 `main.rs`。
- 判定：**这是正确形态**，但需把"`main.rs` 是唯一编排者"写进规范，防止后续有人把 `combat` 反向塞进 `items`。

---

## 六、事件总线约定（待落地）

- **命名**：`<域>.<过去式事实>`（如 `combat.damage_applied`、`items.transfer_committed`）。事件表达**已发生的事实**，不是命令。
- **载荷**：必须是契约类型（YAML 有定义），字段只增不减、不改语义。
- **时序**：发布者**不等待**订阅者；需要回执的场景走"事件 + 关联 ID"，不使用返回值。
- **所有权**：事件载荷**按值传递**（clone 或 Arc），订阅者不得回写发布者数据。

### 已裁决的客户端模态事件（ADR 0004）

| 事件名 | 发布者 | 载荷 | 订阅者 | 效果 |
|---|---|---|---|---|
| `menu.modal_opened` | `menu` | `ModalKind` | `flow` | 置 `ModalState` 对应位 = true |
| `menu.modal_closed` | `menu` | `ModalKind` | `flow` | 置 `ModalState` 对应位 = false |
| `hud.modal_opened` | `hud` | `ModalKind` | `flow` | 同上（交互面板 / 物资箱 / 轮盘） |
| `hud.modal_closed` | `hud` | `ModalKind` | `flow` | 同上 |

> `hud` / `net` 的输入门控一律读 `ModalState::blocks_gameplay_input()`，**不得**各自拼 `&& !a.open && !b.open`。

> ⚠️ 服务端的现有事件枚举清单与订阅关系图**待补**（见第三节 `?`）。本栏填完前，不得把现有直接调用改成事件。
> **责任与时序**：该清单由**服务端 owner** 在解冻前补齐（第七节待补项）；补齐前"把现有调用改事件"冻结。
> **已知豁免**：上表 4 个客户端模态事件已登记，不受此限。

---

## 七、每个模块必须补 `module.md`（模板 + 待办清单）

**位置**：`ServerCode/<模块>/module.md`、`HostCode/<模块>/module.md`
**必含 5 节**：`## 边界` / `## 数据所有权` / `## 接口` / `## 事件` / `## 成熟度(L0–L3)`
**模板**：

```markdown
# <模块名> · module.md
## 边界
本模块负责：…（明确写出"不负责"的部分）
## 数据所有权
| 数据 | 类型 | 是否可变 | 谁可写 |
## 接口
| 接口 | 形式(Trait/函数) | 契约 | 兼容性承诺 |
## 事件
| 事件名 | 方向 | 载荷契约 | 订阅者 |
## 成熟度
L? — <依据>；破坏性变更流程…
```

**待补清单（按本文件优先级排序）**
- [ ] `ServerCode/items/module.md`（L0 → 先定边界）〔依据 ADR 0002〕
- [ ] `ServerCode/inventory/module.md`〔依据 ADR 0002：显式写"不负责战局内格位"〕
- [ ] `ServerCode/equipment/module.md`〔依据 ADR 0002：显式写"不负责手持/弹夹"〕
- [ ] `ServerCode/combat/module.md`〔依据 ADR 0002：显式写"不负责装备等级/元素"〕
- [ ] `ServerCode/interact/module.md`
- [ ] `ServerCode/net/module.md`（L2，须与 protocol.yaml 同步）
- [ ] `ContractCode/module.md`（新 crate，依据 ADR 0003）
- [ ] `HostCode/launcher/module.md`（依据 ADR 0004：只写装配职责）
- [ ] `HostCode/flow/module.md`（依据 ADR 0004：`ModalState` 唯一所有者）
- [ ] `HostCode/net/module.md`（依据 ADR 0003：只吃契约 crate）
- [ ] `HostCode/hud/module.md`（依据 ADR 0004：禁 `use crate::menu::`）
- [ ] 其余模块（见第三节/第四节表）
- [ ] 服务端事件清单与订阅关系图（补第三节 `?` 列）

**过渡纪律（当前 `module.md` 落地 = 0）**
- 未补期间：任何触碰某模块的 PR，必须**同时**补该模块 `module.md`，否则不予合入（"碰到就补，不碰不堵"）。
- `L2` 集合以本文件成熟度列为准；标注 `待定` 的模块（`interact`）定级前按 L2 流程处理。

---

## 八、裁决结果（全部完成）

> **代码全面暂停**（owner 裁决 2026-09-26）：只允许补文档（`module.md`）+ 契约 YAML，不写重构代码。
> 已建在未编译代码上的 A1/A3/A4 三批改动**保持冻结**（见 [stop-doing.md](../stop-doing.md)）。

| # | 冲突 | 裁决 | 记录 |
|---|---|---|---|
| 1 | `items` / `inventory` 双背包 | **方案 A「按域切开」**（局内 vs 局外） | [ADR 0002](../adr/0002-backpack-domain-split.md) |
| 2 | 客户端跨 crate 直连服务端类型 | **抽独立契约 crate `cute_of_duty_contract`** | [ADR 0003](../adr/0003-contract-crate.md) |
| 3 | `hud` 横向依赖 `menu` | **`flow` 持唯一 `ModalState` + 事件仲裁** | [ADR 0004](../adr/0004-client-layer-convergence.md) |
| 4 | `launcher` 装配层兼任表现宿主 | **本轮一起瘦身为纯装配** | [ADR 0004](../adr/0004-client-layer-convergence.md) |
| 5 | 推进顺序 | **先补文档（`module.md` + 契约 + 事件清单），再动代码** | 本文件第七节待补清单 |

**下一步（唯一允许的推进方向）**：按第七节待补清单写各模块 `module.md`；补 `docs/contracts/items.yaml`；
补服务端事件清单。全部补齐后，才统一开工实施 ADR 0002 / 0003 / 0004。
