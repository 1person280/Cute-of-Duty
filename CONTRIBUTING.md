# 贡献指南 · Cute Of Duty 1: Simple

> 欢迎参与！本项目处于 Pre-Alpha，允许大胆改动，**但有底线**：宁可少合一个 PR，也不收一片屎山。
> **任何不符合下方规范的 PR 将被直接关闭**，不讨论、不妥协。
> 提交 PR 前，必须先同意 [贡献者许可协议（CLA）](CLA.md)。

---

## 一、代码洁癖红线（硬性 · 不可协商）

> CI（`cargo build` + `cargo test`）只保证"能编译能通过"，**不保证"不脏"**；红线靠自律 + Reviewer 把关。

| # | 规则 | 说明 |
|---|------|------|
| 1 | **单文件 ≤ 600 行** | 超过 = 上帝文件，必须拆成语义化子模块；UI 面板建议更早瘦身（~300 行内） |
| 2 | **禁止循环依赖** | 模块 A→B 且 B→A 即为坏味道；`element`/`config` 已单向依赖，新增依赖前先画依赖图 |
| 3 | **禁止垃圾桶文件名** | `common.rs`/`utils.rs`/`misc.rs`/`helpers.rs`/`stuff.rs` 一律禁止；文件名必须自解释，且是**单一语义词**（`vitals.rs`、`inventory.rs`） |
| 4 | **嵌套 ≤ 2 层 · 文件名禁下划线** | 默认上限 `src/module/file.rs`。文件名**不得含下划线**：`x_y.rs` 即违规，必须二选一 —— ①压成单一语义词 → `x.rs`；②单开一层子域目录 → `x/y.rs`，**且须在 PR 里证明该层是真子域、嵌套有意义**（证明不成立只能选①）。②是 2 层上限的**唯一例外**，且 `x/y.rs` 之下不得再向下嵌套。叶子数据地图允许放宽，但增长时优先提取独立 crate |
| 5 | **公开项必须有 Why 注释** | 所有 `pub struct` / `pub fn` 要有 Rustdoc，解释**"为什么这么设计"**，不是复述代码在做什么 |
| 6 | **配置单一事实来源** | 数值、反应、地图、档案全部配置表驱动（`ServerCode/config` 嵌入式默认），禁在代码里散落硬编码副本 |
| 7 | **核心库零 bevy** | `ServerCode` 永远不依赖 bevy；渲染相关类型由 HostCode 侧包装 |

---

## 二、目录规范（新增文件前必读）

**放哪里？**

- **客户端表现** → `HostCode/<模块>/`（只吃快照 + 画，零玩法判定）
- **服务端领域逻辑** → `ServerCode/<模块>/`（一切"应该算什么"的唯一权威）
- **契约**（线格式 / 跨域载荷 / Port Trait）→ `ContractCode/` + `docs/contracts/*.yaml`
- **新增地图** → `ServerCode/map/<名称>/`，实现 `layout() -> MapLayout`
- **新增 UI 面板** → `HostCode/<模块>/<面板>.rs`，一个面板一个语义化文件

**什么时候切目录（模块 → 目录）？** 当单个 `.rs` 逼近 **600 行**时，转成目录模块：

```
src/xxx/xxx.rs        # ✅ 主结构
src/xxx/mod.rs        # 只做 mod 声明 + pub(crate) use <子>::*;
src/xxx/<语义子文件>.rs
```

`mod.rs` 必须保持"薄"：只负责 `mod` 声明与重导出，不写实现。

**公开路径不变式**：把 `xxx.rs` 拆成 `xxx/` 目录时，**所有已暴露符号必须原封不动经 `mod.rs` 重导出**——原来 `pub` → `pub use <子>::*;`，原来 `pub(crate)` → `pub(crate) use <子>::*;`。这是拆分合法性的**唯一判据**：拆分后任何调用方无需改一行。

**文件名禁下划线（与红线 3/4 一脉相承）**：凡是需要下划线才说得清的文件名，说明它名字里混了**两个词**——这正是"单开一层子域"的信号：

- 语义上确实是两个词（`health_bar`、`item_wheel`）→ 单开子目录 → `health/bar.rs`、`item/wheel.rs`
- 其实可以合并成一个词（`world_scene`→`scene.rs`）→ 直接压平
- **禁止** `menu/menu_behaviour.rs` 这类"目录词 + 文件名词"凑出来的下划线——子目录名就是它的域，文件名只留**最后一个词**（→ `menu/behaviour.rs`）

---

## 三、提交前自查（命中任一条 → 先重构再提 PR）

- 文件 > **600 行**？函数 > **30 行**或一眼看不出在干嘛？→ 拆
- `if-else` 嵌套 > **3 层**？→ 提取守卫子句 `if !x { return }`，或拆状态机
- 在复制粘贴前一段逻辑、仅参数不同？→ 提取公共函数/泛型
- 写了叫 `common`/`utils`/`helpers` 的文件？→ 按"它到底在帮什么忙"改名
- 文件名里带下划线（`x_y.rs`）？→ 必重构：压成单一语义词 `x.rs`，或单开子域目录 `x/y.rs` 并在 PR 里证明该层是真子域
- 模块 A 用 B、B 又反过来用 A？→ 循环依赖，先解耦
- 代码里硬编码了本应在 YAML/配置表里的数值？→ 移进配置并走 `ServerCode/config` 加载
- 让你的改动在**不带 feature** 的 `cargo build` 里开始编译 bevy？→ 退回，核心库不许依赖 bevy
- 写了 `pub` 却没 Why 注释，或注释只是在翻译代码？→ 补
- 改了代码却没同步 `module.md` / `barek-history.md` / 契约 YAML？→ 补

---

## 四、开发流程与测试要求

```bash
cargo-wrap check --workspace                 # 任何改动必跑
cargo-wrap test -p cute_of_duty_server       # 服务端逻辑改动
cargo-wrap build --release --workspace       # 客户端表现改动（debug 产物 >2GB 会报 os error 193）
```

1. 从 `main` 拉语义化分支（`feat/xxx`、`refactor/xxx`）。
2. 改动前先读相关模块 `mod.rs` 的 Why 注释。
3. 拆文件 / 改目录遵守上面的公开路径不变式。
4. 提 PR 写清：改了哪个模块、为什么、如何验证。

| 改动类型 | 必须跑 |
|---|---|
| 任何改动 | `cargo-wrap check --workspace` |
| 服务端逻辑 | `cargo-wrap test -p cute_of_duty_server` |
| 线格式 / 契约 | 上述 + 契约 YAML 一致性检查（待建） |
| 客户端表现 | 上述 + `cargo-wrap build --workspace --release` |
| 手工验收项 | PR 描述写**步骤 + 预期 + 实际**；未实测的标注"未验证" |

> **未验证的改动不得写进 README 的"已完成"**，只能进 [docs/barek-history.md](docs/barek-history.md) 的条目（标题标注「待实机验证」）。

**什么时候必须先开 Issue 对齐**：新增顶层模块 / 把模块提取为独立 crate / 改配置加载语义（`ServerCode/config`）/ 改 L2 模块（`net` 线格式、`ContractCode`、公共 Trait）。

---

## 五、目标架构：模块化单体 + 事件总线 + Trait 接口

> **一句话**：模块间**禁止直接调用**，数据所有权划清，依赖无环。
> **终局判据**：**代码边界锁死，将来拆微服务无需重写。**
> 权威细则见 [module-boundaries.md](docs/architecture/module-boundaries.md) 与 [ADR 0001](docs/adr/0001-modular-monolith-event-bus.md)。

**五条铁律**

| # | 铁律 | 判定方式（Reviewer 怎么查） |
|---|---|---|
| 1 | **禁跨模块直接调用** | 三层判据：①**同 crate 跨模块** —— 搜 `use crate::<别的模块>::`；②**跨 crate** —— `HostCode/` 不得出现 `cute_of_duty_server`（见 [ADR 0003](docs/adr/0003-contract-crate.md)）；③**同层横向** —— `hud/`、`net/` 中搜 `use crate::menu::`，命中即违规。跨模块只允许经 ①Trait（`XxxPort`）②事件 |
| 2 | **数据所有权唯一** | 每份可变状态有且只有一个 owning 模块；他人只能持**快照/句柄**，不得持 `&mut`、不得回写 |
| 3 | **依赖无环 + 单向分层** | 只许 `表现层 → 契约层 → 领域层 → 基础设施层`；新增依赖前**必须**在 PR 里贴依赖图 |
| 4 | **接口先于实现** | **跨 crate / 跨进程**能力先写 `pub trait XxxPort`；**crate 内** L0/L1 允许普通函数（如 `layout()`），但**升 L2 前必须先 Trait 化** |
| 5 | **契约机器可读** | 跨模块/跨进程载荷必须在 `docs/contracts/*.yaml` 有定义；代码是契约的实现，不是定义 |

**分层归属**

| 层 | 归属 | 硬性约束 |
|---|---|---|
| 表现层 | `HostCode/*` | 只准"读快照 + 出画"；**零玩法判定** |
| 契约层 | `ContractCode` + `docs/contracts/*.yaml` | 独立 crate，不得依赖 `ServerCode`/`HostCode`；只增不减，改语义必须升版本 |
| 领域层 | `ServerCode/*`（除 net/config/storage/hal） | **零 bevy**；一切"应该算什么"的唯一权威 |
| 基础设施层 | `ServerCode/{config,storage,hal,net}` | `config` 只读单例，**任何模块禁写** |

**编排者**：服务端 `ServerCode/main.rs`；客户端装配者 `HostCode/launcher`（`main.rs` 只是入口）。只有它们可同时持多个模块句柄并接起来；库内模块之间不许互相接线。装配者的唯一豁免：允许 `use crate::hud::<系统函数>` 以执行 `add_systems`，但**不得** `use` 组件/资源类型自己写逻辑（见 [ADR 0004](docs/adr/0004-client-layer-convergence.md)）。

**事件总线约定**：命名 `<域>.<过去式事实>`（`combat.damage_applied`）——**表达已发生的事实，不是命令**；载荷必须是契约类型，字段只增不减、语义不改；发布者**不等待**订阅者，需回执走"事件 + 关联 ID"，不得用返回值回传；订阅者**不得回写**发布者数据；现有直接调用改事件**必须**先补齐事件清单与订阅关系图（清单由服务端 owner 补齐）。

---

## 六、统一版本号 `x.y.z`（协议 = 游戏 = 发布号）

自 `0.10.0` 起，**协议版本与游戏版本不再分离**：同一个 `x.y.z` 既是**游戏版本**、**协议版本**，也是**发布号** —— README 版本表 / tag / GitHub Release / `Cargo.toml`（ServerCode + HostCode）/ `docs/contracts/*.yaml` 一律写这同一个号。

| 位 | 名称 | 含义 | 强制要求 |
|---|---|---|---|
| `x` | **游戏内核版本** | **严重破坏性更新**（内核 / 架构级颠覆，老客户端整体不可用） | **必须** `x+1`，`y`、`z` 归零 |
| `y` | **协议版本** | **不兼容**变更（删字段、改语义、改类型） | **必须** `y+1`，`z` 归零；**必须**写迁移指南；两端须同版本部署 |
| `z` | **细节版本** | **无兼容性变化**（加性 / 修 bug / 造型细节） | 可 `z+1`；加性变更老端必须能忽略新字段继续运行 |

每一次版本变动都必须同步 [docs/barek-history.md](docs/barek-history.md) 条目（变更类型 / 兼容性 / 迁移指南 / 验证 / 关联）。

**禁止事项（直接关闭 PR）**：悄悄改字段**语义**却不升 `y`；删字段却不写迁移指南；内核级破坏却只升 `y`；两端协议版本不一致却声称"兼容"。

---

## 七、Rust 规范（硬性）

| # | 规则 | 说明 |
|---|---|---|
| 1 | **禁 `unwrap()` / `expect()`** | 生产路径一律 `?` 向上传播；只允许出现在 `#[cfg(test)]` 与"逻辑上不可能失败且有注释证明"处 |
| 2 | **错误类型统一 `thiserror`** | 每模块一个 `XxxError`，禁用 `Box<dyn Error>` 穿透模块边界 |
| 3 | **内部字段 `pub(crate)`** | 字段默认私有；跨模块可见性最小化，`pub` 必须是契约的一部分 |
| 4 | **异步保 `Send + Sync`** | 跨 `await` 持有的类型必须 `Send`；共享状态用 `Arc<...>`，禁用 `Rc`/`RefCell` 跨界 |
| 5 | **配置 `serde` 外部化** | 一切可调数值走 `ServerCode/config/`（`include_str!` 嵌入 + 运行时覆盖），禁硬编码副本 |

---

## 八、文档要求与成熟度分级

| 文档 | 位置 | 必含内容 |
|---|---|---|
| `module.md` | **每个模块目录下** | `边界` / `数据所有权` / `接口` / `事件` / `成熟度(L0–L3)` |
| `barek-history.md` | `docs/` | 变更类型 / 兼容性 / 迁移指南；**冻结区**（未验证 / 不许动 / 已确认不动）亦标注于此，标题加「待实机验证」 |
| 契约 | `docs/contracts/*.yaml` | 机器可读的线格式与跨模块载荷定义 |
| ADR | `docs/adr/*.md` | 重大决策：背景 / 决策 / 后果 / 未决事项 / 替代方案 |

| 级别 | 含义 | 变更纪律 | 审核要求 |
|---|---|---|---|
| L0 | 试验田 | 随便改 | 无 |
| L1 | 内部稳定 | 破坏性变更须写 ADR | 1 名 reviewer |
| L2 | 对外契约 | 破坏性变更须迁移指南 + 契约同步 + `y+1` | **≥2 名 reviewer，maintainer 必须参与** |
| L3 | 冻结 | **只允许加性变更，语义不可改** | 同 L2 |

> **`module.md` 过渡纪律**：仓库现有 `module.md` 数量 = **0**。未补期间，任何触碰某模块的 PR 必须**同时**补上该模块的 `module.md`，否则不予合入（"碰到就补，不碰不堵"）；补全清单见 [module-boundaries 第七节](docs/architecture/module-boundaries.md)。
> **L2 集合**以 module-boundaries 成熟度列中标记 L2 的模块为准（禁用"等"字兜底）；标 `待定` 的模块（当前 `interact`）在定级前按 L2 流程处理。
> **单人维护期过渡条款**：仓库当前仅 **1 名登记贡献者**（见 [CLA 签署台账](docs/cla-signatures.md)）——该期间 L2 变更**豁免**上表「≥2 名 reviewer」，由 maintainer **自审**，但必须**同时**满足：① BarekHistory 条目含**迁移指南**；② `docs/contracts/*.yaml` 同步；③ 五道红线全绿且附**可核对凭据**；④ 该次提交/发布记录中**声明为豁免**。出现**第 2 名登记贡献者**后本条款**自动失效**，恢复「≥2 名 reviewer」门槛（届时须回溯复核豁免期内的 L2 变更）。

---

## 九、开源贡献流程

### 9.1 贡献者许可协议（CLA）

提交前必须先同意 [CLA](CLA.md)：未在 PR 中按第 7 节做出声明同意（`I have read the CLA Document and I hereby sign the CLA.`）的 PR **不予合入**。贡献代码 = 同意 CLA（第 2 条版权许可 + 第 3 条专利许可 + 第 4 条陈述保证）；你保留自己贡献的著作权，仅授予项目方**非独占的再许可权**。签署记录见 [CLA 签署台账](docs/cla-signatures.md)。

### 9.2 贡献范围

| 范围 | 是否接受 | 说明 |
|---|---|---|
| 缺陷修复、文档补全、测试补充 | ✅ 直接提 PR | — |
| L0 / L1 模块内的重构与功能 | ✅ 提 PR | 需附依赖图与验证方式 |
| **L2 模块（`net` 线格式 / `ContractCode` / 公共 Trait）** | ⚠️ **先开 Issue 对齐** | 必须附迁移指南，≥2 reviewer（单人维护期见第八节过渡条款） |
| 新增顶层模块 / 提取独立 crate | ⚠️ **先开 Issue 对齐** | 必须先在 `module-boundaries.md` 定边界 |
| 绕过 `barek-history.md` 中标注「待实机验证」的冻结项 | ❌ 直接关闭 | — |

### 9.3 接口变更流程

1. 开 Issue：改哪个接口、为什么、影响哪些调用方、是否破坏兼容。
2. 若破坏兼容 → 先写**迁移指南**（进 `barek-history.md`）与 **ADR**，再动代码。
3. 同步更新 `docs/contracts/*.yaml` 与相关 `module.md`。
4. PR 内附**依赖图**（证明无环）与**验证命令 + 结果**。

### 9.4 提交规范

```
<type>(<模块>): <一句话为什么>

关联: ADR-xxxx / Issue #xx
```

`type` ∈ `feat` / `fix` / `refactor` / `docs` / `test` / `chore` / `perf`。**一次提交只做一件事**；重构与功能**不得混在同一提交**。

---

## 十、文档索引

- [贡献者许可协议 CLA](CLA.md) / [CLA 签署台账](docs/cla-signatures.md)
- [代码许可 GPLv3 + Linking Exception](LICENSE)（`HostCode/`·`ServerCode/`·`ContractCode/`·`tools/`）
- [资产许可 CC BY-NC-SA 4.0](LICENSE-ASSETS)（美术 / 模型 / 音频 / 自有字体）
- [模块边界总览](docs/architecture/module-boundaries.md) / [ADR 0001](docs/adr/0001-modular-monolith-event-bus.md)
- [BarekHistory 变更台账](docs/barek-history.md)（含冻结区） / [protocol.yaml](docs/contracts/protocol.yaml)