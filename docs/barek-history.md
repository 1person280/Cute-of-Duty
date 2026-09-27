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
