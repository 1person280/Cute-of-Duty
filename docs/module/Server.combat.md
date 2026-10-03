# Server.combat · module.md

> 服务端**权威战斗域**（`ServerCode/combat/`）。分层定位：**领域层 · 局内（in-match）**。所有战斗真相（弹药、技能冷却、投射物、毒区、元素结算）都只在 `CombatSystem` 内计算；客户端只上报 `CombatIntent`、只收结果。纯逻辑、零 bevy。

## 边界

**本模块负责**：
- **战斗结算**：`CombatSystem::apply_input`（开火 / 换弹 / 技能 Q/E / 切枪 / 速用，分派给 `shooter`/`skill`）与 `tick_world`（手雷飞行引爆 / 毒区周期结算 / 换弹推进）。
- **战斗组件**：`Combatant`（`operator_idx` + `weapons` + `active_slot` + 弹夹 + 冷却）、`HeldGrenade`（持雷中间态）。
- **弹道与数值**：`shooter`（`try_fire` / `try_reload`，含弱点 ×1.8）、`grenade`（弹道常数 + 投掷/取消，**公开**供客户端预览同源复用）、`skill`（Grenade/Burst/Dash）、`zone`（周期掉血）。
- **实体与装配**：`spawn_player`、`apply_loadout`（选装清单重播种背包，未知名忽略）、`switch_weapon` / `switch_operator` / `equip_weapon` / `push_item` / `use_item_at`。
- **事件出队**：`CombatEvent::{Kill, Hit}` 经 `drain_events()` 出队，供网络层转 `ServerMessage::Event`。

**本模块不负责**（明确划出，ADR 0002 三条硬边界）：
- **不持装备等级 / 元素**——局外装备实例与等级规则在 `equipment`；本模块只按干员与手持武器裁决战斗元素。
- **不持背包格位**——备弹权威在 `items::Backpack`（换弹直接从背包抽取，无中间弹池）；`combat` 只持手持槽 / 弹夹。
- **不依赖线格式**——用最小意图集 `CombatIntent` 做边界，由 `main.rs` 从 `PlayerInput` 映射。

## 数据所有权

| 数据 | 类型 | 是否可变 | 谁可写 |
|---|---|---|---|
| 战斗组件（弹药/冷却/弹夹/持雷） | `Combatant` / `HeldGrenade`（`Component`） | 每 Tick 可变 | 仅 `combat` |
| 战斗事件队列 | `CombatSystem.events` | 每 Tick 出队 | 仅 `combat` |
| 世界实体 | `World` | 每 Tick 可变 | `combat` 经 `entity` API 增改 |

> **所有权原则**：热战斗数据挂在玩家实体组件上、由 `CombatSystem` 单点结算；背包格位归 `items`，`combat` 只经 `items` API 抽取，不直接持格位。

## 接口

| 接口 | 形式 | 契约 | 兼容性承诺 |
|---|---|---|---|
| `CombatSystem::{new, apply_input, tick_world, drain_events}` | 方法 | 战斗结算主入口 | 被 `engine::GameLoop` / `main.rs` 调度 |
| `CombatIntent` | 结构 | 单 Tick 最小意图集（不依赖线格式） | 边界契约 |
| `CombatEvent::{Kill, Hit}` | 枚举 | HUD 播报事件 | 出队供网络层 |
| `spawn_player` / `apply_loadout` / `switch_weapon` / `switch_operator` / `equip_weapon` / `push_item` / `use_item_at` | 函数 | 玩家战斗态装配与操作入口 | 被 `interact`/`main.rs` 调用 |
| `combat::grenade`（公开子模块） | 模块 | 弹道常数，供客户端预览同源复用 | 公开 |
| `RAY_MAX_RANGE` / `WEAKPOINT_MULT` / `RELOAD_TIME_SECS` / `ZONE_TICK_SECS` | 常量 | 战斗数值 | 域内 |

## 事件

| 事件名 | 方向 | 载荷契约 | 订阅者 |
|---|---|---|---|
| `CombatEvent::Kill { killer, victim }` | `combat` → 主循环 | 击杀事实 | `GameLoop::drain_combat_events` → `ServerMessage::Event` |
| `CombatEvent::Hit { source, target, is_headshot }` | `combat` → 主循环 | 命中事实 | 同上 |

## 成熟度

**L1（内部稳定：crate 内可依赖）** —— 依据：`combat` 被 `interact`/`main.rs`/`engine` 依赖，是服务端权威战斗核心；不定义对外线格式（走 `CombatIntent` 边界）。

- 破坏性变更纪律：须写 ADR + 1 名 reviewer。
- 确定性要求：服务端权威逻辑须可复现（同输入同输出），测试见 [combat/tests.rs](file:///c:/Users/yxhcf/Desktop/Cute-of-Duty/ServerCode/combat/tests.rs)。
