# 冻结任务 · 0.6-Snapshot-8 实机反馈

> **本目录的用途**：`main` 分支的 [stop-doing.md](../stop-doing.md) 冻结区在当前工作分支
> （`wip/0.8-snapshot-8`）不存在，且本轮不合并 `main`。为避免"实测结论丢在聊天里"，
> 本目录专门承接**当前工作分支上的冻结条目**——语义与 stop-doing.md 一致：
>
> 1. 本目录列出的条目，在移出本目录前**不得**在 README / CHANGELOG / release note 里写成"已完成"；
> 2. **不得**在其基础上继续叠加新玩法代码（先验证，再往前走）；
> 3. 解除冻结必须补上"验证方式 + 实测结果"，并把条目迁回 stop-doing.md / barek-history。

最后更新：2026-09-26（快照8 实机反馈归集 · owner）

---

## A. 快照8 实机反馈 · 状态

| # | 反馈原文 | 代码状态 | 实测状态 | 遗留 / 卡在哪 |
|---|---|---|---|---|
| 1 | 「3/4 改成消耗品 **没有名字**，且**没有轮盘切换**」 | `HostCode/hud/hud_item_wheel.rs` 已实现长按径向轮盘 + 短按速用；`hud_vitals.rs` 3/4 槽应显示"物品名 ×N" | ❌ **未响应**（用户实测：**短按一次与长按均无任何反应**） | 需查：3/4 是否被上游门控吞掉、`category_slots` 是否取到空背包（背包 0/100 时轮盘 `open` 判为 false）、HUD 3/4 槽名字是否真的取到 `labels` |
| 2 | 「两个武器槽只能用一个且与干员耦合」 | 已与干员解耦 | ✅ 用户确认 | — |
| 3 | 「F 交互没有鼠标滚轮切换的翻页面板，**必须按 F 才能看到**；且补给台**不能按照物品名字给予**」 | `hud_interact.rs` 改就近常显 + 滚轮翻页；补给台按 `SupplyKind::label()` 物品名给予 | ✅ **已解决**（用户确认） | 遗留（不阻断）：**物品种类太少**——"算过但记录"，后续补物品表时一并扩 |
| 4 | 「起始点**没有可交互箱子**；不是开启直接拿走，而是**两个 4×3 界面**（仓库也改成两个 4×3）」 | 新增 `hud_loot_panel.rs`（容器↔背包 4×3 逐格转移）；`menu/arsenal.rs` 仓库/背包均改 4×3 | ✅ **已还原**（用户确认"问题不大"） | 遗留（**下次修**）：操作形态**别扭** → 改为**鼠标拖拽 / Shift+右键**移动物品；**补给台与仓库同样适用**（三者交互形态需统一） |
| 5 | 「去除窗口右边的若干干员」 | 已移除 | ✅ 用户确认 | — |
| 6 | 「**WASD 无法移动**」 | 未改 | ❌ **未修复**（本轮只记录，不修） | 根因线索见下节 A-1 |

### A-1. 「WASD 无法移动」根因线索（本轮实测，未修）

实机观察（快照8 release，服务端 `cod_server.exe` + 客户端 `cod1.exe`，全新进程、无任何按键输入）：

- 进入训练场后，**交互二级选项面板（`补给台`：步枪弹药 / 医疗包 / 护甲片 · "滚轮选择 · F 确认 · Esc 关闭"）在无输入的情况下自行展开**。
- 该面板置位 `InteractState.panel_open = true`，使运行条件
  `hud::gameplay_input_active`（`hud_bigmap.rs`）恒为假：
  `pause != Closed || bigmap.0 || interact.panel_open || loot.open || wheel.open` 中 `interact.panel_open` 为真。
- 后果：`net::input_system`（`HostCode/net/pilot.rs`）与 `world::mouse_look_system` 被 `run_if` 冻结
  → **WASD / 开火 / 换弹 / 技能 / 鼠标视角全部无响应**（与"WASD 无法移动"的用户现象一致）。
- 现有唯一置位路径：`hud_interact.rs::open_panel` ← `confirm_entry` ← `interact_input` 的
  `keys.just_pressed(KeyCode::KeyF) || keys.just_pressed(KeyCode::Enter)`。**全新进程未按任何键却已展开**，
  故需排查：① 是否 UI `Interaction`/`Changed<Interaction>` 误触发；② 是否按键状态在首帧被误判为 `just_pressed`；
  ③ 是否 `sync_interact_menu` 的可见性条件与 `panel_open` 不一致。

> 附带独立缺陷（同批，未修）：`hud_item_wheel.rs::item_wheel_input` 中 `held_key` 在
> `if !state.open && blocked { return; }` 提前返回时不会清除 —— 若在按住 3/4 期间被 F/M 等模态打断，
> 松开事件被吞掉，`held` 持续累积、`state.open` 会在**无按键**时自发置真，同样永久冻结玩法输入。

---

## B. 快照8 未完成项（发布时明确"还差"的）

| # | 项 | 状态 |
|---|---|---|
| 1 | **按钮**（demo 操作方法的 UI 触发） | ❌ 未实现（本轮定为"仅核对，不实现"） |
| 2 | 扁平化剩余目标：**B. legacy demo 操作表逐行核对**（基准 `_ref/Cute-of-Duty-0.3.2/README.md`） | ⏳ 进行中（逐行核对结论另记） |

---

## C. 下次要修（按用户本轮指示）

1. **修 WASD 冻结**：清掉"交互二级面板无故自开"与 `item_wheel` `held_key` 残留两条路径（见 A-1）。
2. **修 3/4 消耗品无响应**：短按/长按均需有反馈（见 A 表 #1）。
3. **统一物品搬运交互**：物资箱 / 仓库 / 补给台 全部改为**鼠标拖拽 或 Shift+右键**（见 A 表 #4）。
4. 扩物品表（见 A 表 #3 遗留）。
