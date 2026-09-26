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
- **迁移指南**：`x+1` 必填 —— 逐步操作 + 前后代码对照
- **验证**：如何验证（命令 / 手动步骤）+ 结果
- **关联**：ADR / Issue / 契约文件
```

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
> **Breaking（`x+1`）** 记录并附迁移指南——因为客户端与服务端的依赖拓扑发生物理变更。
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
