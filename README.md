<div align="center">

# Cute Of Duty 1: Simple

**战术撤离射击游戏** · 核心差异化 **元素互斥生态 + 反护航经济架构**

基于 Rust + Bevy 0.14 的 3D 像素风 FPS · 无头确定性模拟与 3D Demo 双入口
配置文件表驱动的全部玩法规则 · 单一事实来源

[![License](https://img.shields.io/badge/License-GPL--3.0--linking--exception-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/Version-0.3.2--%E6%90%9C%E6%89%93%E6%92%A4-red.svg)](#四版本历史)
[![Rust](https://img.shields.io/badge/Rust-stable%20%28edition%202021%29-orange.svg)](Cargo.toml)
[![Headless](https://img.shields.io/badge/%E6%97%A0%E5%A4%B4%E6%A8%A1%E6%8B%9F-passing-2ea44f.svg)](#一快速开始)
[![Demo](https://img.shields.io/badge/3D%20Demo-Bevy%200.14-2ea44f.svg)](#一快速开始)

**外部依赖 · 站在开源社区的肩膀上** · [![by Bevy](https://img.shields.io/badge/by-Bevy-E90000)](https://bevyengine.org)
[![by Tokio](https://img.shields.io/badge/by-Tokio-blue)](https://tokio.rs)
[![by Serde](https://img.shields.io/badge/by-Serde-white)](https://serde.rs)
[![by Tracing](https://img.shields.io/badge/by-Tracing-black)](https://github.com/tokio-rs/tracing)
[![by Rand](https://img.shields.io/badge/by-Rand-4B8BBE)](https://crates.io/crates/rand)
[![by Crossbeam](https://img.shields.io/badge/by-Crossbeam-8E44AD)](https://crates.io/crates/crossbeam)
[![by BLAKE3](https://img.shields.io/badge/by-BLAKE3-2EA44F)](https://crates.io/crates/blake3)

> 完整依赖清单与各库许可证见游戏内「设置 → 开源代码鸣谢」面板。

</div>

---

> **一句话定位**：Cute Of Duty 1 站在"生态可互斥、经济反护航"的路线上——每种元素既是
> 收益也可能是反噬，装备等级越高越失控，玩家要做的是**在收益与风险之间权衡**，
> 而不是照搬传统 FPS 的"枪械数值堆叠"。

---

## 一、快速开始

### 直接试玩（无需编译）

> 双击项目根目录下的 **`CuteOfDuty_Demo.exe`** 即可进入 3D 像素风 FPS Demo。
>
> 程序启动时会自动定位项目目录（从工作目录逐级向上搜索，找不到再从 exe 所在目录搜索），
> 因此从任意位置启动都能加载 `assets/` 与 `src/config/element_reactions.yaml`。

### 本地编译

环境要求：**Rust stable**（edition 2021，无需 nightly），Windows 10/11 或 Linux 均可。

```powershell
cargo run --bin cod1          # 无头模拟：60Hz 固定 Tick + 确定性重放验证 + 战局/档案演示
cargo run --features demo     # 3D 像素风 FPS Demo（Bevy 0.14）
cargo test                    # 全部测试（不编译 bevy，秒级完成）
cargo build --release         # 发布构建（已开启 LTO + strip）
```

> **单 crate 双入口**：核心库与 3D Demo 同在一个包内，bevy 是被 feature `"demo"` 门控的
> 可选依赖——默认 `cargo build` / `cargo test` **完全不编译 bevy**，核心逻辑秒级增量迭代；
> 只有带 `--features demo` 的命令才触发 Bevy 全量编译（首次约 20 分钟以上，后续增量很快）。

### Demo 操作方式

> 下表为「按键组 × 触发环境」矩阵排版示意：`A 环境` / `B 环境` 两列用于承载后续玩法环境（如不同模式/天气）下的
> 触发覆盖，当前版本尚未落地该维度，一律标注 `—`（沿用默认触发）。

| 按键组 | 具体按键 | 默认触发 | A 环境触发 · 是否忽略默认 | B 环境触发 · 是否忽略默认 |
|---|---|---|---|---|
| 视角 | 鼠标 | 自由视角（X 轴偏航 / Y 轴俯仰，灵敏度独立，俯仰限制仰 50° / 俯 70°） | — | — |
| 移动 | W / A / S / D | 前后左右位移（始终相对相机方向） | — | — |
| 跳跃 | Space | 起跳（仅在地面时） | — | — |
| 疾跑 | 左 Ctrl | 常速 4 → 7 单位/秒（按住） | — | — |
| 主武器切换 | 1 / 2 | 在两把主武器间切换 | — | — |
| 干员技能 | Q / E | 技能（点燃 DoT / 冰冻 / 位移冲刺 / 毒素领域） | — | — |
| 越肩瞄准 | 鼠标右键（按住） | SpringArm 由右肩后方 6.5m 过渡到 2.4m（0.22s），FOV 收窄 28%，准星琥珀，移速降至 55% | — | — |
| 射击 / 投掷 | 鼠标左键 | 射击（相机射线，靶心弱点 ×1.8）；持雷时改为投掷 | — | — |
| 交互 | F | 呼出统一交互菜单（功能台 + 拾取物，站点优先）；滚轮选择，F 确认 | — | — |
| 背包 | Tab | 打开背包（双武器 / 弹药池 / 补给品） | — | — |
| 使用物品 | R（悬停背包物品） | 使用悬停的背包物品 | — | — |
| 快捷道具 | 3 / 4 | 快捷使用恢复品 / 战术品（按住打开轮盘，点轮盘中心撤销） | — | — |
| 关闭 / 取消 | Esc | 关闭背包 / 功能台 / 取消持雷 / 无 UI 时释放鼠标 | — | — |
| 暂停 | / 或 ~ | 暂停菜单（返回游戏 / 设置 / 回主界面） | — | — |

> **越肩瞄准（SpringArm 相机架构，参考原神弓手瞄准模式）**
> - 相机层级：脚底 Pivot（TopLevel，不随模型旋转）→ ShoulderPivot（Yaw）→ PitchPivot（Pitch）→ SpringArm（右肩偏移 + 后方距离）→ Camera；
> - 默认机位：右肩 +0.55 / 眼高 ≈2.95 / 后方 6.5；瞄准机位：右肩 +1.0 / 后方 2.4，过渡 0.22s smoothstep；
> - SpringArm 碰撞避障（原神方案）：撞墙缩回（贴墙最小 0.7m）、离墙缓伸，地面高度钳制 ≥0.35m；角色与相机之间不阻挡射击射线；
> - 射击判定从相机视线出发（与准星一致），曳光从枪口收敛到命中点；命中靶板中心红心判定弱点，伤害 ×1.8（无弹道下坠，为即时射线；手雷保持 12 m/s² 重力抛物线）；
> - 无蓄力机制（步枪保持连发手感）；不支持左右肩切换（固定右肩）；瞄准中跳跃保持瞄准；
> - 手雷必须"先瞄准后释放"：任意途径使用后进入持握并强制越肩瞄准，左键投出 / Esc 取消放回背包。

---

## 二、核心差异化卖点

| 卖点 | 说明 |
|---|---|
| **元素互斥生态** | 火 / 冰 / 电 / 毒等元素并非"越堆越强"。护甲与武器的元素互斥、同源元素协同增益、环境修正全部由配置表驱动——选型本身就是博弈 |
| **反护航经济架构** | 装备等级不是保障线而是风险线：1 级新手保护舱 → 2–6 级可指定元素（成本翻倍）→ 7–9 级真随机混沌区；转售 / 给予会重置元素。高等级=高收益+高不确定性 |
| **配置表驱动的全部规则** | 数值、元素反应、干员档案、地图布局一律沉淀为 YAML / 纯数据结构，核心库**不硬编码任何玩法**，改平衡不用动代码 |
| **零 bevy 的核心库** | 游戏逻辑与渲染彻底分离：`cargo test` 秒级完成，Bezy 由 feature 门控，带来的直接好处是核心迭代几乎无编译负担 |

> 三条铁律（改动前请先理解，这是项目约定）详见 [架构说明](#三架构总览)。

#### 与热门友商 FPS 的差异化定位

| 维度 | Cute Of Duty | CS:GO / CS2 | Valorant | APEX Legends | 逃离塔科夫 |
|---|---|---|---|---|---|
| 装备经济 | **反护航**：等级越高风险越高（1 级新手舱 → 2–6 级指定元素 → 7–9 级真随机），转售/给予重置元素 | 回合局内经济（买枪起甲，击杀省） | 技能 + 局内经济（每回合攒钱买武器） | 落地拾取制（无购买） | 局外持久仓库 + 局内搜刮撤离 |
| 元素玩法 | **互斥生态**：火/冰/电/毒非堆强，护甲与武器互斥、同源协同、环境修正全由配置表驱动 | 无元素系统，纯枪械胜率 | 固定特工技能（元素近似"角色定位"） | 英雄技能 + 传说护甲 | 弹种/护甲分级，无元素互斥 |
| 对该玩法的调整成本 | **零 bevy + 配置表驱动**：改平衡只动 YAML/数据结构，`cargo test` 秒级验证 | 需改引擎局内逻辑 | 需改技能数值包 | 需改英雄数值包 | 需改数值 + 掉落表 |
| 撤离目标 | 核心循环之一 | 无撤离，回合胜率 | 无撤离，回合胜率 | 无撤离，大逃杀 | 核心循环（战利品带出） |

> 定位差异一句话：**在塔科夫式「搜→打→撤」长线收益循环里，用反转风险的等级曲线取代纯拼枪/纯拼装**，
> 并把全部平衡规则下沉到配置表，让"改玩法"从改代码变成改数据。

---

## 三、架构总览（接手前必读）

```
                ┌──────────────────────────────────┐
                │  核心库（src/lib.rs 引出的 11 模块）│
                │      ★ Cargo 层面零 bevy ★       │
                │      配置表驱动的全部游戏逻辑      │
                └───────┬──────────────────┬───────┘
                        │ 共用同一套规则    │ 共用同一套规则
            ┌───────────▼──────────┐  ┌────▼─────────────────────┐
            │ cod1（src/main.rs）  │  │ 3D Demo                  │
            │ 无头模拟 · 默认构建   │  │ src/demo + src/model     │
            │ 固定Tick+确定性验证  │  │ feature "demo" · Bevy 0.14│
            └──────────────────────┘  └──────────────────────────┘
```

**三条铁律（改动前请先理解，这是项目约定）：**

1. **核心逻辑全部在 `src/lib.rs` 引出的库模块中**，3D Demo 与无头模拟只是两个"入口"。
2. **Demo 不硬编码规则**：数值、反应、地图、干员档案全部配置表/纯数据驱动。
3. **核心库不依赖 bevy**：bevy 是根包的可选依赖，由 feature `"demo"` 门控；
   默认 `cargo build`/`cargo test` 永远不编译 bevy；需要渲染的资源（Resource trait 等）
   由 Demo 侧 newtype 包装（如 `ElementalSystem`）。

### 目录结构总览（表1 · Src 内部核心模块）

> 公开符号全部经各目录 `mod.rs` 薄壳重导出；**核心业务模块深度 ≤ 2 层**（`src/module/file.rs`）。
> 扁平化/拆分规范见 [CONTRIBUTING.md](CONTRIBUTING.md)。

| 模块名 | 核心职责 | 关键文件 / 结构体说明 |
|---|---|---|
| `config` | 配置加载与单一事实来源 | `mod.rs`（目录探测 + `include_str!` 嵌入默认 + 运行时覆盖）；`element_reactions.yaml`（随源码提交的元素反应表） |
| `element` | 元素反应系统 | `ElementType` / `EntityElementState` / `ReactionResult`（元素/状态模型）；`ElementConfig`（YAML 反序列化目标）；`ElementSystem`（反应查询、互斥惩罚、协同增益、环境修正） |
| `damage` | 伤害结算流水线 | `packet.rs`（`DamagePacket`/`Vec3` 纯数据）；`resolver.rs`（`DamageResolver` 结算 Step1–9）；`effect.rs`（燃烧/冰冻/中毒等 9 个副作用组件） |
| `engine` | 游戏循环 | `GameLoop`（60Hz 固定 Tick 主时钟）；`TickConfig`；确定性双缓冲快照 + 重放验证 |
| `entity` | 自研 ECS | 实体=组件容器；组件自治前作用；系统按固定顺序处理 |
| `equipment` | 装备等级与元素规则（反护航经济核心） | 1 级新手保护舱；2-6 级可指定元素（成本翻倍）；7-9 级真随机混沌区；转售/给予重置元素 |
| `gamemode` | 游戏模式 | `MatchConfig` / `MatchManager`：战术撤离（核心）、团队死斗（练习）、合约 |
| `hal` | 硬件抽象层 | 单调时钟；中断消解为带时间戳的环形缓冲（零分配，`crossbeam-queue`） |
| `map` | **纯数据地图定义**（零 bevy） | `MapLayout`；`training/`：`layout()` 组装出生准备室/CQB 大厅/射击馆/二层回廊（`building_shell.rs`/`spawn_room.rs`/`cqb_hall.rs`/`shooting_range.rs`/`second_floor.rs`） |
| `operator` | 干员与武器档案（纯数据） | Q/E 技能归属干员（焦狸/霜吻/雷豹/毒蜨）；步枪数值集中于此 |
| `player` | 玩家档案 | 信誉系统、赛季进度、统计数据 |
| `demo` *(feature `"demo"` 门控)* | 3D FPS Demo | `mod.rs` 纯组装层；面板已扁平化：`frontend`(引导)/`components`/`menu/`(主菜单+加载+设置)/`pause`/`world`/`character`/`controller`/`camera`/`combat/`(武器/技能/手雷/爆炸/区域/反馈)/`targets`/`minimap`/`hud/`(血条/弹药/击杀播报/伤害跳字/闪烁)/`inventory/`(背包/交互菜单/物品轮盘/持握手雷)/`stations` |
| `model` *(feature `"demo"` 门控)* | 干员模型与动作 | `operator_models.rs`（四名干员体素模型）；`operator_swap.rs`（模型置换）；`yanhu_action.rs`（动作系统）；`rig.rs`/`palette.rs`/`components.rs` |

### 目录结构总览（表2 · 外部资源与配置）

| 目录名 | 用途 | 文件格式 / 注意事项 |
|---|---|---|
| `assets/` | 美术资源（游戏内加载） | 子目录：`characters/` `environment/` `fonts/` `ui/` `weapons/`；`.jpg`/`.png`/`.ttf`（中文字体 `simhei.ttf`）+ `model/*.json`（体素模型） |
| `src/config/` | **配置表（含加载器，与核心代码物理相邻）** | `element_reactions.yaml`：无头模拟与 3D Demo 共用；**单一事实来源**——默认值由 `include_str!` 编译期嵌入，运行时同路径文件作为设计师热改覆盖；改表需同步重编译默认或改同文件 |
| `tools/` | 开发辅助脚本 | PowerShell（图标生成、窗口截图、UI 自动测试等） |
| `.github/workflows/` | CI | `rust.yml`：push/PR 到 `main` 自动跑 `cargo build` + `cargo test`（不带 demo） |
| `.agents/skills/` | AI 协作工作流文档 | 美术创作 / 地图验收等技能的说明文档 |
| `CuteOfDuty_Demo.exe` | 预编译 3D Demo | 双击即玩；启动自动定位项目根目录（向上搜索 `src/config/element_reactions.yaml`）|

### 配置表

- `src/config/element_reactions.yaml`：元素反应表，无头模拟与 3D Demo **共用同一份**
  （核心库 `cute_of_duty::config` 模块统一加载，自动定位项目根目录）。
  改平衡直接改表，不用动代码。**单一事实来源**的加载语义：
  - **权威默认** = 编译期 `include_str!` 嵌入的同目录 YAML，随二进制分发、无源码也能拿到一致默认值；
    改表时同步改该文件即可（默认值由同一文件驱动，永不漂移）；
  - **运行时覆盖** = 从项目根目录向上搜索同路径 YAML，供设计师不改代码热改表；
  - **文件缺失** → 回退嵌入默认（与 YAML 恒一致），日志会提示；
  - **文件存在但解析失败** → 启动直接报错退出（两个入口均如此）——改错表应当场失败，
    而不是静默用默认值让改动"看起来生效了"。
- 表内环境修正（`RainEnvironment`/`HighTemperature`/`SnowEnvironment` 各行）、
  互斥规则（`mutual_exclusion`）、协同增益（`synergies`）均由运行时真实读取：
  - 环境修正 = 以 (环境状态, 元素) 查反应表取 `damage_multiplier`；
  - 互斥规则的 `elements` 字符串按**元素英文键名前缀**匹配（"IceArmor" 以 "Ice" 开头）；
    `self_conflicts.elements` 顺序约定为 `[护甲, 武器]`，`teammate_conflicts` 顺序无关；
  - 协同增益数值（如 `fire_damage_bonus`）通过 `ElementSystem::synergy_effect()` 读取。

### 如何新增一张地图

1. 新建 `src/map/<名称>/mod.rs`（或单文件 `<名称>.rs`），实现一个返回 `MapLayout` 的 `layout()` 函数（照抄 `training/mod.rs` 的写法）；
2. 在 `src/map/mod.rs` 里 `pub mod <名称>;`
3. Demo 侧无需改动渲染逻辑——通用渲染器 `spawn_map_layout` 会自动处理网格、材质、光照；
4. 跑 `cargo test` 确认布局约束（边界、通道净空、遮挡不与掩体相交等）全部通过。

---

## 四、版本历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 0.3.2 | 2026-09-22 | **搜打撤**：物资箱重塑为体素栅格木箱（四角立柱 + 四面通板 + 平顶盖）并接入统一交互菜单（站点优先于拾取，F 必开箱不误拾，弃用自建触发）；对局仓库/背包拖拽选装落地并打通 Tab 背包；核心差异化补「与热门友商 FPS 对比」定位表；Demo 操作方式改为「按键组 × 触发环境」矩阵排版（A/B 环境列为玩法环境预留，当前标 `—`） |
| 0.3.1 | 2026-09-21 | **渲染内存泄漏定向修复 + README 翻新**：修复 `damage_popup_system` 相机缺失时弹字永久存活的确定性泄漏；新增 `effect_guard.rs`（五类高频特效硬性存活上限兜底）；收敛特效密度/寿命（命中粒子 5→3、爆炸碎块 10→4 等）；新增 `debug_tracer.rs`（每 5s 实体/资产采样，供定位残余增长） |
| 0.3.0 | 2026-09-20 | **反屎山扁平化重构**：`config/` 并入 `src/config/`（YAML `include_str!` 嵌入 + 运行时覆盖，单一事实来源）；全部 >500 行上帝文件拆成语义化子模块（`damage`→packet/resolver/effect，`map/training`→分区分文件，`demo` 的 `inventory`/`menu`/`hud`/`combat`→面板目录，`model`→operator_models 等）；`common.rs`→`frontend.rs`；新增 [CONTRIBUTING.md（反屎山公约）](CONTRIBUTING.md) 与目录结构表格 |
| 未发布（main） | 2026-09-11 | GitHub Actions CI 接入；`src/map/training.rs` 扁平化重构并修复 CI 构建错误；设置面板新增「开源代码鸣谢」；`src/demo` 由 7300 行单文件拆分为 15 个功能子模块 |
| 0.2.3 | 2026-09-10 | 首个真开源版本：demo3d 独立 crate 并回本包（bevy 改由 feature `"demo"` 门控）；新增 `src/model` 干员模型（Yanhu）与动作系统；仓库以 GPL-3.0-with-linking-exception 开源 |
| 0.2.1 | 2026-09-06 | 0.2 hotfix 1：主界面改版（右下角「切换模式/开始游戏」、右侧 60% 分类+模式选择面板、右上角齿轮设置浮层）；workspace 解耦（核心库零 bevy、配置表全量生效） |
| 0.1.1 | 2026-09-05 | 首个对外分享打包版：补齐交接文档（本 README）、`.gitignore` |
| 0.1.0 | — | 内部开发版：核心库 + 无头模拟 + 3D Demo 全部跑通 |

---

## 五、文档

* **项目规范**
  * [贡献指南（反屎山公约：600 行上限 / 无循环依赖 / 语义化命名）](CONTRIBUTING.md)
  * [开源许可证 GPL-3.0-with-linking-exception（原文）](LICENSE)
* **AI 协作工作流（`.agents/skills/`）**
  * [游戏美术创作指南](.agents/skills/)
  * [地图建模验收流程](.agents/skills/)

---

## 六、开发环境说明与已知坑

1. 首次 `cargo run --features demo` 需要 20+ 分钟（Bevy/wgpu 全量编译），请耐心等待；
   不带 feature 的命令不编译 bevy，秒级完成。建议保持 `Cargo.lock` 以获得与开发一致的依赖版本。
2. dev profile 已按 Bevy 官方建议把依赖设为 O3（否则试玩帧率明显下降），游戏代码本身保持 O1
   以加快增量编译——不要改 `[profile.dev.package."*"]`。
3. **CI**：`.github/workflows/rust.yml` 在 push / PR 到 `main` 时自动跑 `cargo build` + `cargo test`
   （不带 demo feature，验证核心库；提交前本地跑一遍同样命令可提前发现问题）。

### 已知问题（试玩实测）

- ~~手雷爆炸内存飙升 / OOM~~：2026-09-05 已修复（爆炸/枪口特效网格与材质入池共享，不再逐发新建资产）。
- ~~术能锁定后相机冻结~~：实为玩家初始 yaw 朝向问题（背对靶场），已修复（默认面向靶场出生）。
- **渲染内存缓慢增长（已定向缓解，仍待长时间确认）**：长时间游玩（数分钟级）GPU 内存仍会缓慢累积，
  最终可能 OOM。2026-09-21 起做了定向修复（仍在观察是否彻底根治，见下）：
  - 修复「弹字清理依赖相机，相机暂不可用时弹字永久存活」的确定性回收失效点（`damage_popup_system`）；
  - 为曳光/弹字/粒子/反应文字/爆炸等高频特效增加**硬性存活上限兜底清道夫**（`effect_guard.rs`），
    杜绝任何回收路径失效导致的无限堆积；
  - 收敛高频特效的生成密度与寿命（命中粒子 5→3、爆炸碎块 10→4、曳光/枪口/爆闪寿命收短等），
    减少 Bevy 每帧反复 spawn/despawn 造成的渲染 batch 抖动。
  - 定位其余增长点时，可用内置诊断采样（`cargo run --features demo` 游玩后看日志，`debug_tracer.rs`
    每 5s 打印各类特效实体存活数与 `Mesh`/`Material` 资产表容量）。
- 自动化试玩提示：若用外部自动化驱动本 Demo，winit 可能拦截合成鼠标事件，可用系统级 `mouse_event` 绕过。

---

## 七、目录结构

```
CuteOfDutyAlpha/
├── Cargo.toml / Cargo.lock     # 包清单（核心库 + cod1 bin；bevy 为 feature 门控的可选依赖）
├── CuteOfDuty_Demo.exe         # 预编译 3D Demo，双击即玩
├── CONTRIBUTING.md             # ⚠️ 反屎山公约（贡献前必读）
├── README.md                   # 本档案（交接文档）
├── .github/workflows/rust.yml  # CI：push/PR 到 main 跑 cargo build + cargo test
├── .agents/skills/             # AI 协作工作流文档（美术创作 / 地图验收）
├── assets/                     # 美术资源：characters / environment / fonts / ui / weapons
├── tools/                      # 开发辅助脚本（PowerShell：图标生成、窗口截图、UI 测试等）
├── src/
│   ├── lib.rs                  # 核心库入口（11 个核心模块，见表1）
│   ├── main.rs                 # cod1 入口：默认无头模拟；--features demo 时为 3D Demo
│   ├── config/                 # 配置加载器 + element_reactions.yaml（单一事实来源）
│   ├── damage/ element/ engine/ entity/ equipment/ gamemode/ hal/ operator/ player/
│   ├── map/                    # 纯数据地图定义（map/training/ 为训练场分区分文件）
│   ├── demo/                   # 3D Demo（feature "demo"）：组装层 + 面板化子目录
│   └── model/                  # 干员模型与动作（feature "demo"）
└── target/                     # 构建产物（git 忽略）
```

> 一位开发者接手前，只需要读三份：**本 README（概览）** → **架构表 1/2** → **CONTRIBUTING.md（公约）**。
> 核心业务模块保持 ≤ 2 层深度、每个 `.rs` ≤ 600 行、禁止 `utils.rs` 之类的语义化空壳——这些是硬约束，不是建议。