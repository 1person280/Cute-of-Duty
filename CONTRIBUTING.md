# 贡献指南 · Cute Of Duty 1: Simple

# ⚠️ 贡献前必读：反屎山公约

> 欢迎参与！本项目处于 Pre-Alpha，允许大胆改动，**但有底线**。
> 本项目宁可少合一个 PR，也不收一片屎山。
> **任何不符合下方目录 / 代码规范的 PR 将被直接关闭**，不讨论、不妥协。

---

## 一、代码洁癖标准（硬性红线）

以下为**不可协商**的硬性标准。CI（`cargo build` + `cargo test`）只保证"能编译能通过"，
**不保证"不脏"**——洁癖红线靠你的自律 + Reviewer 把关：

| # | 规则 | 说明 |
|---|------|------|
| 1 | **单文件 ≤ 600 行** | 超过 = 上帝文件，必须拆成语义化子模块。UI 面板建议更早瘦身（~300 行内） |
| 2 | **禁止循环依赖** | 模块 A→B 且 B→A 即为坏味道。`element`/`config` 已实现单向依赖，新增依赖前先画依赖图 |
| 3 | **禁止垃圾桶文件名** | `common.rs`、`utils.rs`、`misc.rs`、`helpers.rs`、`stuff.rs` 一律禁止。文件名必须自解释：`hud_health_bar.rs`、`reaction_damage.rs`、`spawn_room.rs` |
| 4 | **核心业务模块深度 ≤ 2 层** | `src/module/file.rs`。`demo`（应用壳，feature 门控）与 `map/training`（叶子数据地图）允许 3 层，但增长时优先提取独立 crate |
| 5 | **公开项必须有 Why 注释** | 所有 `pub struct` / `pub fn` 要有 Rustdoc，**解释"为什么这么设计"**，不是复述代码在做什么 |
| 6 | **配置单一事实来源** | 数值、反应、地图、档案全部配置表驱动，不允许在代码里散落硬编码副本（见 `config` 模块的嵌入式默认） |
| 7 | **核心库零 bevy** | 默认 `cargo build`/`cargo test` 永远不编译 bevy。渲染相关类型由 Demo 侧包装，不许把 bevy 拖进核心模块 |

---

## 二、目录规范（新增文件前必读）

> ⚠️ **本节正文仍是旧的单 crate 布局（`src/` + `src/demo` + feature `demo`）**，
> 与第六节起的 `HostCode` / `ServerCode` 双 crate 实际结构**不一致**。
> **通用规则**（≤600 行 / 语义化命名 / 无循环依赖 / 禁 `common`·`utils` / `mod.rs` 只做网关 / 公开路径不变式）**继续有效**；
> **目录与 feature 相关表述一律以第六节及之后为准。**

### 放哪里？

- **核心业务逻辑** → `src/<模块>/`，遵循 `lib.rs` 里声明的 11 个核心模块划分。
- **新增地图** → `src/map/<名称>/` 或 `src/map/<名称>.rs`，实现 `layout() -> MapLayout`。
- **新增 UI 面板** → `src/demo/<面板>/` 目录：一个面板一个子目录，`mod.rs` 做薄壳重导出。
- **3D 美术/模型** → `src/model/`（feature `"demo"` 门控）。

### 什么时候切目录（模块 → 目录）？

当单个 `.rs` 逼近 **600 行**时，把它转成目录模块：
```
src/demo/menu/menu.rs        # ✅ 好的示范：面板=目录，主结构+交互/加载屏/设置分开
src/demo/menu/mod.rs         # 只做 mod 声明 + pub(crate) use <子>::*;
src/demo/menu/loading_screen.rs  # 具体语义化子文件
```
`mod.rs` 必须保持"薄"：只负责 `mod` 声明与 `pub use` 重导出，不写实现。
文件名按"具体做什么"命名，禁止 `common` / `utils`。

### 公开路径不变式

把 `xxx.rs` 拆成 `xxx/` 目录时，**所有已对外（`pub` / `pub(crate)`，含父模块 `use xxx::*`）暴露的符号必须原封不动地经 `mod.rs` 重导出**：
- 原来 `pub` → `pub use <子>::*;`
- 原来 `pub(crate)` → `pub(crate) use <子>::*;`

这是拆分合法性的**唯一判据**：拆分后任何调用方无需改一行。

---

## 三、坏味道自查表

> 提交前，逐条问自己。**命中任何一条 → 停下，先重构再提 PR。**

- [ ] 我刚写的文件超过 **600 行**了？（→ 拆目录模块）
- [ ] `if-else` 嵌套超过 **3 层**？（→ 提取守卫子句 `if !x { return }`，或拆成小函数/状态机）
- [ ] 某个函数超过 **30 行**、或一眼看不出在干嘛？（→ 拆函数，用好的命名）
- [ ] 我在复制粘贴前一段逻辑，仅参数不同？（→ 提取公共函数/泛型）
- [ ] 我写了一个叫 `common` / `utils` / `helpers` 的文件？（→ 改名：它到底在帮什么忙，就叫什么）
- [ ] 我想让模块 A 用模块 B 的东西，B 反过来也换 A？（→ 停止，这是循环依赖，先重构解耦）
- [ ] 我在代码里硬编码了一个本应在 YAML / 配置表里的数值？（→ 移进配置并走 `src/config` 加载）
- [ ] 我的改动让 `cargo build`（不带 feature）开始编译 bevy？（→ 退回，核心库不许依赖 bevy）
- [ ] 我写 `pub fn` / `pub struct` 却没写注释，或注释只是在翻译代码？（→ 补 Why 注释）
- [ ] 我把多个无关职责塞进一个文件"图省事"？（→ 按内聚拆开）

---

## 四、开发流程

```bash
cargo test                      # 全量测试（不编译 bevy，秒级），必跑
cargo check --features demo     # 3D Demo 侧改动后必跑（bevy 已缓存，增量快）
```

1. 从 `main` 拉分支，命名尽量语义化（`feat/xxx`、`refactor/xxx`）。
2. 改动前先读相关模块的 `mod.rs` 注释（多数模块有"为什么这样设计"的说明）。
3. 拆文件/改目录，遵守上面的公开路径不变式。
4. 本地跑通 `cargo test`（涉及 demo 再跑 `cargo check --features demo`）。
5. 提 PR 时写清：改了哪个模块、为什么、如何验证。CI 会自动跑 `cargo build` + `cargo test`。

### 什么时候必须问

- 想**新增核心模块**（在 `lib.rs` 加 `pub mod`）→ 先和 maintainer 对齐职责边界。
- 想把某模块**提取成独立 crate** → 值得做，但先讨论（本项目已把 demo 从独立 crate 并回，避免重复再造）。
- 想改配置加载语义（`src/config`）→ 先读该模块 Why 注释，别破坏"单一事实来源"。

---

## 五、约定速记（TL;DR）

- 文件 ≤ 600 行；目录 `mod.rs` 只做网关。
- 文件名 = 它做的事；禁止 `common` / `utils`。
- 公开项带 Why 注释；找不到 Why 就别急着写注释。
- 数值走配置表；核心库零 bevy；模块无环。
- **PR 不合规范 → 直接关闭。**

---

# 六、目标架构：模块化单体 + 事件总线 + Trait 接口

> **一句话**：模块间**禁止直接调用**，数据所有权划清，依赖无环。
> **终局判据**：**代码边界锁死，将来拆微服务无需重写。**

权威细则见 [docs/architecture/module-boundaries.md](docs/architecture/module-boundaries.md) 与 [ADR 0001](docs/adr/0001-modular-monolith-event-bus.md)。

## 6.1 五条铁律

| # | 铁律 | 判定方式（Reviewer 怎么查） |
|---|---|---|
| 1 | **禁跨模块直接调用** | **三层判据缺一不可**：①**同 crate 跨模块** —— 搜 `use crate::<别的模块>::`；②**跨 crate** —— `HostCode/Cargo.toml` 不得含 `cute_of_duty_server`，`grep -r "cute_of_duty_server" HostCode/` 须为空（见 [ADR 0003](docs/adr/0003-contract-crate.md)）；③**同层横向** —— 搜 `use crate::menu::` 于 `hud/`、`net/`，命中即违规。跨模块只允许 ①经 Trait（`XxxPort`）②经事件。 |
| 2 | **数据所有权唯一** | 每份可变状态有且只有一个 owning 模块。他人只能持**快照/句柄**，不得持 `&mut`，不得回写。 |
| 3 | **依赖无环 + 单向分层** | 只许 `表现层 → 契约层 → 领域层 → 基础设施层`。新增依赖前**必须**在 PR 里贴依赖图。 |
| 4 | **接口先于实现** | **跨 crate / 跨进程**能力先写 `pub trait XxxPort`，再写实现；实现必须可替换、可 mock。**crate 内**模块间在 L0/L1 允许普通函数（如 `layout()`、`model_preset()`），但**升 L2 前必须先 Trait 化**（见第九节成熟度）。 |
| 5 | **契约机器可读** | 跨模块/跨进程载荷必须在 `docs/contracts/*.yaml` 有定义；代码是契约的实现，不是定义。 |

**分层归属**（新增文件前先想清楚它属于哪层）

| 层 | 归属 | 硬性约束 |
|---|---|---|
| 表现层 | `HostCode/*` | 只准"读快照 + 出画"；**零玩法判定** |
| 契约层 | `ContractCode`（crate `cute_of_duty_contract`）+ `docs/contracts/*.yaml` | **物理载体是独立 crate**（线格式类型 + 跨域载荷 + 共享常量 + Port Trait），YAML 是其机器可读描述。`ContractCode` 不得依赖 `ServerCode`/`HostCode`，依赖只有 `serde`/`thiserror`（见 [ADR 0003](docs/adr/0003-contract-crate.md)）。只增不减；改语义必须升版本 |
| 领域层 | `ServerCode/*`（除 net/config/storage/hal） | **零 bevy**；一切"应该算什么"的唯一权威 |
| 基础设施层 | `ServerCode/{config,storage,hal,net}` | `config` 只读单例，**任何模块禁写** |

**唯一的编排者**：服务端 `ServerCode/main.rs`；客户端装配者 `HostCode/launcher`（`HostCode/main.rs` 只是入口：做 `mod` 声明 + `launcher::run`，**不参与编排**）。只有它们可以同时持有多个模块的句柄并把它们接起来；库内模块之间不许互相接线。

**编排者的唯一豁免**：`launcher` 允许 `use crate::hud::xxx_system` 等**引用各模块导出的系统函数**以执行 `add_systems` —— 这是装配者唯一被允许的跨模块动作；但**不得** `use crate::hud::<组件/资源类型>` 去自己写逻辑（见 [ADR 0004](docs/adr/0004-client-layer-convergence.md)）。

## 6.2 事件总线约定

- **命名**：`<域>.<过去式事实>`（`combat.damage_applied`、`items.transfer_committed`）。**事件表达已发生的事实，不是命令**。
- **载荷**：必须是契约类型（YAML 有定义）；字段只增不减、语义不改。
- **时序**：发布者**不等待**订阅者。需要回执的场景 → "事件 + 关联 ID"，**不得用返回值回传**。
- **所有权**：载荷按值传递（`Clone` / `Arc`）；订阅者**不得回写**发布者数据。
- **迁移纪律**：现有直接调用改事件**必须**先补齐事件清单与订阅关系图（见 module-boundaries 第六节），否则不许动。
  **责任与时序**：事件清单由**服务端 owner** 在解冻前补齐（module-boundaries 第七节待补项）；补齐前"把现有调用改事件"冻结。
  **已知豁免**：ADR 0004 已登记的 4 个客户端模态事件（`menu`/`hud` 的 `modal_opened`/`modal_closed`）不受此限——它们已有清单。

---

# 七、协议版本与兼容承诺

版本号 `x.y.z`，**每一次协议改动都必须同时更新 [docs/barek-history.md](docs/barek-history.md)**：

| 位 | 含义 | 强制要求 |
|---|---|---|
| `x+1` | **不兼容**变更（删字段、改语义、改类型） | **必须**写迁移指南（逐步操作 + 前后对照）；客户端与服务端须同版本部署 |
| `y+1` | 加性兼容（只增字段 / 只增消息） | 老端必须能忽略新字段继续运行 |
| `z+1` | 仅修 bug，语义不变 | 无需迁移指南 |

**禁止事项（直接关闭 PR）**
- 悄悄改字段**语义**而不升 `x`（例如把 `use_medkit: bool` 直接复用成别的含义）；
- 删字段却不写迁移指南；
- 客户端/服务端协议版本不一致时仍声称"兼容"。

## 7.1 发布号 与 协议版本 是两套号，不得混用

| 号 | 形态 | 用途 | 驱动什么 |
|---|---|---|---|
| **发布号** | `<版本线>-Snapshot-<N>`（如 `0.6-Snapshot-7`） | 面向玩家的里程碑标记（GitHub Release / README 版本表） | **不驱动**兼容性，纯发布标记 |
| **协议版本** | `x.y.z`（记录在 `docs/contracts/*.yaml` 的 `version`） | 线格式 / 契约兼容性 | `x+1`/`y+1`/`z+1` 规则 + BarekHistory 条目 |

> `Snapshot-N` **不映射**到 `x.y.z`。改协议 → 动 `x.y.z` 并写 BarekHistory；改发布 → 只动发布号。
> （历史遗留：README / barek-history 曾出现 `0.7-Snapshot-7` / `[0.8.0-Snapshot-8]` 的 WIP 号，
> 那批快照**未发布且已被冻结**；正式发布线以 `0.6-Snapshot-N` 连续编号为准。）

---

# 八、Rust 规范（硬性）

| # | 规则 | 说明 |
|---|---|---|
| 1 | **禁 `unwrap()` / `expect()`** | 生产路径一律 `?` 向上传播。`unwrap` 只允许出现在 `#[cfg(test)]` 与"逻辑上不可能失败且有注释证明"处 |
| 2 | **错误类型统一 `thiserror`** | 每模块一个 `XxxError`（`thiserror::Error`），禁用 `Box<dyn Error>` 穿透模块边界 |
| 3 | **内部字段 `pub(crate)`** | 字段默认私有；跨模块可见性最小化，`pub` 必须是契约的一部分 |
| 4 | **异步保 `Send + Sync`** | 跨 `await` 持有的类型必须 `Send`；共享状态用 `Arc<...>` 且保证 `Sync`，禁用 `Rc`/`RefCell` 跨界 |
| 5 | **配置 `serde` 外部化** | 一切可调数值走 `src/config/`（`include_str!` 嵌入 + 运行时覆盖），禁在代码里散落硬编码副本 |

---

# 九、文档要求（缺一不可）

| 文档 | 位置 | 必含内容 |
|---|---|---|
| `module.md` | **每个模块目录下** | `边界` / `数据所有权` / `接口` / `事件` / `成熟度(L0–L3)` |
| `barek-history.md` | `docs/` | 变更类型 / 兼容性 / 迁移指南（每次协议或 L2+ 改动追加一条） |
| 契约 | `docs/contracts/*.yaml` | 机器可读的线格式与跨模块载荷定义 |
| ADR | `docs/adr/*.md` | 重大架构决策：背景 / 决策 / 后果 / 未决事项 / 替代方案 |
| `stop-doing.md` | `docs/` | 冻结区：未验证、不许动、已确认不动的事项 |

**成熟度分级与审核门槛**

| 级别 | 含义 | 变更纪律 | 审核要求 |
|---|---|---|---|
| L0 | 试验田 | 随便改 | 无 |
| L1 | 内部稳定 | 破坏性变更须写 ADR | 1 名 reviewer |
| L2 | 对外契约（被其它 crate / 线格式依赖） | 破坏性变更须迁移指南 + 契约同步 + `x+1` | **≥2 名 reviewer，maintainer 必须参与** |
| L3 | 冻结 | **只允许加性变更，语义不可改** | 同 L2 |

**`module.md` 过渡态（当前落地 0，必须带纪律）**

> 实测：仓库现有 `module.md` 数量 = **0**。因此第九节不是"锦上添花"，必须带过渡纪律：
> 1. **优先补**清单见 [module-boundaries 第七节](docs/architecture/module-boundaries.md)；
> 2. **未补期间**：任何触碰某模块的 PR，必须**同时**补上该模块的 `module.md`，否则不予合入（"碰到就补，不碰不堵"）；
> 3. 待清单全部勾掉后，本条降级为常规要求。

**`L2` 集合以权威表为准（禁用"等"字兜底）**

> L2 集合 = [module-boundaries 成熟度列](docs/architecture/module-boundaries.md) 中标记为 L2 的模块。
> 标注为 `待定` 的模块（当前：`interact`）在**定级前按 L2 流程处理**（保守优先），避免"待定"成为免审通道。

---

# 十、避坑清单（本项目已实际踩过）

- [ ] **循环依赖** —— 模块 A↔B 互相 `use`（新增依赖前先画图）。
- [ ] **跨模块写数据** —— 直接改别的模块拥有的状态，而不是发事件。
- [ ] **协议偷偷改语义** —— 复用字段/复用 variant 而不升 `x`。
- [ ] **文档不同步** —— 改了代码不更 `module.md` / `barek-history.md` / 契约 YAML。
- [ ] **无版本分发** —— 客户端与服务端版本漂移却无兼容声明。
- [ ] **无兼容承诺** —— 删除/重命名公开项却不给迁移指南。
- [ ] **在未验证的地基上盖楼** —— 见 [stop-doing.md](docs/stop-doing.md)，未验证项禁止继续叠加。

---

# 十一、开源贡献流程

## 11.1 贡献范围

| 范围 | 是否接受 | 说明 |
|---|---|---|
| 缺陷修复、文档补全、测试补充 | ✅ 直接提 PR | — |
| L0 / L1 模块内的重构与功能 | ✅ 提 PR | 需附依赖图与验证方式 |
| **L2 模块（`net` 线格式 / 契约层 `ContractCode` / 公共 Trait）改动** | ⚠️ **先开 Issue 对齐** | 必须附迁移指南，≥2 reviewer |
| 新增顶层模块 / 把模块提取为独立 crate | ⚠️ **先开 Issue 对齐** | 必须先在 `module-boundaries.md` 定边界 |
| 绕过 `stop-doing.md` 冻结项 | ❌ 直接关闭 | — |

## 11.2 接口变更流程

1. 开 Issue 说明：改哪个接口、为什么、影响哪些调用方、是否破坏兼容。
2. 若破坏兼容 → 先写**迁移指南**（进 `barek-history.md`）与 **ADR**，再动代码。
3. 同步更新 `docs/contracts/*.yaml` 与相关 `module.md`。
4. PR 内附**依赖图**（证明无环）与**验证命令 + 结果**。

## 11.3 提交规范

```
<type>(<模块>): <一句话为什么>

<可选正文：为什么这么改，而不是怎么改>

关联: ADR-xxxx / Issue #xx
```

`type` ∈ `feat` / `fix` / `refactor` / `docs` / `test` / `chore` / `perf`。
**一次提交只做一件事**；重构与功能**不得混在同一提交**。

## 11.4 测试要求

| 改动类型 | 必须跑 |
|---|---|
| 任何改动 | `cargo-wrap check --workspace` |
| 服务端逻辑 | `cargo-wrap test -p cute_of_duty_server` |
| 线格式 / 契约 | 上述 + 契约 YAML 一致性检查（待建） |
| 客户端表现 | 上述 + `cargo-wrap build --workspace --release`（debug 产物 >2GB 会触发 os error 193） |
| 手工验收项 | 必须写进 PR 描述：**步骤 + 预期 + 实际**；未实测的标注"未验证" |

> **未验证的改动不得写进 README 的"已完成"**，只能进 `docs/stop-doing.md`。

---

# 十二、文档索引

- [模块边界总览](docs/architecture/module-boundaries.md)（边界 / 数据 / 接口 / 成熟度 L0–L3 / 待裁决冲突）
- [ADR 0001 · 模块化单体 + 事件总线](docs/adr/0001-modular-monolith-event-bus.md)
- [BarekHistory · 变更与兼容性台账](docs/barek-history.md)
- [线格式契约 protocol.yaml](docs/contracts/protocol.yaml)
- [stop-doing.md · 冻结区](docs/stop-doing.md)

---

> **⚠️ 关于本文档第一~五节（"反屎山公约"）**：它们写于**旧的单 crate 布局**（`src/lib.rs` + `src/demo` + feature `demo`），
> 与当前 **`HostCode` / `ServerCode` 双 crate + Bevy 动态库** 的实际结构**已不一致**。
> 其中的**通用规则**（≤600 行 / 语义化命名 / 无循环依赖 / Why 注释 / 禁止 `common`·`utils`）**继续有效**；
> 但**目录与 feature 相关的表述以第六节及之后为准**，旧表述待回填修订（见 `docs/stop-doing.md`）。
