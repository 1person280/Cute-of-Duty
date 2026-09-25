<div align="center">

# Cute Of Duty 1: Simple

**战术撤离射击游戏** · 核心差异化 **元素互斥生态 + 反护航经济架构**

基于 Rust + Bevy 0.14 的 3D 像素风 FPS · 无头确定性模拟与 3D Demo 双入口
配置文件表驱动的全部玩法规则 · 单一事实来源

[![License](https://img.shields.io/badge/License-GPL--3.0--linking--exception-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/Version-0.6.0-SnapShot-2-blue.svg)](#六版本历史)
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
| 延迟面板 | CapsLock | 显示/隐藏到服务器的通信延迟列表（逐玩家毫秒） | — | — |

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

##### 技术对比（引擎 / 开源 / 可 Mod / 平衡调整成本 / 架构）

> 核心命题：**源码是否开放、Mod 是否可做、改平衡要动代码还是动数据、核心逻辑与渲染是否分离**。

| 游戏 / 技术栈 | 源码开源 | 可 Mod / 社区内容 | 平衡调整成本 | 核心与渲染架构 |
|---|---|---|---|---|
| **Cute Of Duty**（Rust + Bevy 0.14） | ✅ 全开源 GPL-3.0-with-linking-exception | ✅ **配置表驱动**：改玩法 = 改 YAML，社区即可做平衡 Mod | **核心零 bevy + 改表即生效**，`cargo test` 秒级验证 | **服务端权威 + 核心逻辑与服务端物理分离**：核心零 bevy、可无头确定性模拟，渲染为客户端表现层 |
| **CS:GO / CS2**（Source 2） | ❌ 闭源 | ✅ 创意工坊（地图/皮肤） | 官方平衡，改引擎/服务器逻辑 | 引擎一体，无逻辑分离 |
| **Valorant**（Unreal 魔改自研） | ❌ 闭源 | ❌ 官方严格管控 | 官方改技能数值包 | 引擎一体，无逻辑分离 |
| **Overwatch 2**（自研引擎） | ❌ 闭源 | ❌ | 官方改英雄平衡 | 引擎一体，无逻辑分离 |
| **Apex Legends**（Source 魔改） | ❌ 闭源 | ❌ | 官方改英雄数值包 | 引擎一体，无逻辑分离 |
| **PUBG**（Unreal Engine） | ❌ 闭源 | ❌ | 官方数值调整 | 引擎一体，无逻辑分离 |
| **Fortnite**（Unreal Engine 5） | ❌ 闭源 | ✅ UEFN / 创意模式 | 官方 + 创意模式创作者 | 引擎一体，无逻辑分离 |
| **Call of Duty（Warzone）**（自研 IW 系） | ❌ 闭源 | ❌ | 官方平衡补丁 | 引擎一体，无逻辑分离 |
| **Rainbow Six Siege**（自研引擎） | ❌ 闭源 | ❌ | 官方平衡干员 | 引擎一体，无逻辑分离 |
| **Destiny 2**（Tiger 引擎） | ❌ 闭源 | ❌ | 官方季度平衡 | 引擎一体，无逻辑分离 |
| **Battlefield**（Frostbite） | ❌ 闭源 | ❌ | 官方平衡补丁 | 引擎一体，无逻辑分离 |
| **Halo Infinite**（Slipspace） | ❌ 闭源 | ✅ Forge 自定义模式 | 官方 + 社区 Forge | 引擎一体，无逻辑分离 |
| **逃离塔科夫**（Unity） | ❌ 闭源 | ❌ | 官方改数值 + 掉落表，需重进服 | 引擎一体，无逻辑分离 |

##### 商业化对比（经济 / 技能元素 / 撤离循环 / 付费模式）

> 核心命题：**装备经济是护航还是反护航、技能元素是否可配置化、有无撤离式长线循环、付费是否影响游戏性**。
> Cute Of Duty 的商业化定位是 **无影响月卡制**——自愿订阅支持开发，**对玩法/数值/胜负零影响**，不做数值售卖、不开箱抽卡、无 P2W。

| 游戏 | 装备经济 | 技能 / 元素系统 | 撤离式循环 | 付费模式 |
|---|---|---|---|---|
| **Cute Of Duty** | **反护航**：1级新手舱→2–6级指定元素(成本翻倍)→7–9级真随机，转售/给予重置元素；局内 + 局外 | **元素互斥生态**：火/冰/电/毒非堆强，护甲/武器互斥、同源协同、环境修正，配置表驱动 | ✅ **核心循环**「搜→打→撤」 | **无影响月卡制**：自愿订阅支持开发，对玩法零影响，非数值售卖/开箱 |
| **CS:GO / CS2** | 回合局内经济（买枪起甲，击杀省） | 无元素，纯枪械胜率 | ❌ | 买断 + 饰品开箱 |
| **Valorant** | 回合局内经济（每回合攒钱买武器） | 固定特工技能（元素近似"角色定位"） | ❌ | F2P + 内购 |
| **Overwatch 2** | 无购买，直接选角 | 英雄技能 + 职责体系 | ❌ | F2P + 通行证/皮肤 |
| **Apex Legends** | 落地拾取制大逃杀 | 传说技能 + 传说护甲 | ❌ | F2P + 内购 |
| **PUBG** | 落地拾取制大逃杀 | 无元素系统 | ❌ | 买断后转 F2P |
| **Fortnite** | 大逃杀拾取 + 建筑资源 | 无元素（基建制，章节技能） | ❌ | F2P + 皮肤 |
| **Call of Duty（Warzone）** | 局外枪匠配装 + 局内拾取 | 连杀奖励，无元素互斥 | ❌ | 买断 + F2P 内购 |
| **Rainbow Six Siege** | 局外解锁干员 | 干员道具（战术配置） | ❌ | 买断 + 内购 |
| **Destiny 2** | 局外装备/技能池 + 局内掉落 | 职业技能 + 元素属性 | ❌（副本/幽灵撤离） | F2P + 资料片 |
| **Battlefield** | 兵种装备制，局内重生部署 | 兵种能力，无元素互斥 | ❌ | 买断 |
| **Halo Infinite** | 拾取制（武器架） | 能量护盾 + 装备 | ❌ | F2P（多人）+ 战役买断 |
| **逃离塔科夫** | **局外持久仓库 + 局内搜刮**，死亡装备丢失 | 弹药/护甲分级，无元素互斥 | ✅ **核心循环**「搜→打→撤」 | 买断 |

> 定位差异一句话：**在塔科夫式「搜→打→撤」长线收益循环里，用反转风险的等级曲线取代纯拼枪/纯拼装**，
> 通过**配置表驱动**让"改玩法"从改代码变成改数据，并以**核心零 bevy + 全开源**换取社区 Mod 与秒级验证，
> 商业化上坚持**无影响月卡制**——对玩法/数值/胜负零影响。

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

### 目录结构总览（客户端结构 · HostCode）

> 动态装载 bevy 0.14 的客户端表现层，**只吃快照 + 画**（服务端算、客户端显示）。
> 模块深度 ≤ 2 层（`module/file.rs`），扁平化/拆分规范见 [CONTRIBUTING.md](CONTRIBUTING.md)。

| 模块 | 职责 | 关键文件 |
|---|---|---|
| `launcher` | 纯装配层：动态装载 bevy 0.14 + 装载渲染表现全套（相机 Rig / 体素绘制 / HUD / 小地图 / 背包 UI 绘制），只吃快照 + 画 | `mod.rs` |
| `flow` | AppState 状态机（Loading / MainMenu / InGame）、加载屏、状态迁移 | `flow_state.rs` / `loading.rs` |
| `net` | mpsc 后台线程消费服务端快照 + 上行 NetOut 命令通道 | `network.rs` |
| `menu` | 主菜单 / 仓库·携带物资（拖拽 + Shift 选装）/ 模式 / 设置 / 加载 | `menu_main.rs` / `arsenal.rs` / `mode_panel.rs` / `game_settings.rs` |
| `hud` | 血条 / 护甲量 / 弹药、技能 CD、小地图、击杀与通告、撤离提示 | `hud_vitals.rs` / `hud_minimap.rs` / `hud_skills.rs` / `hud_feed.rs`（语义化子文件） |
| `world` | 训练场几何 / 材质 / 光照（服务端重画，客户端只摆） | `world_assets.rs` |
| `shared` | 主题色板「无影响月卡制」、字体句柄 | `theme.rs` |

### 目录结构总览（服务端结构 · ServerCode）

> 服务端权威模拟 + TCP 网络层，**核心零 bevy**、可无头确定性模拟（详见「五、服务端进度」）。

| 模块 | 职责 |
|---|---|
| `engine` | 权威 60Hz 固定 Tick + 确定性双缓冲快照 |
| `entity` | 自研 ECS（实体 = 组件容器） |
| `combat` | 射击 / 手雷 / 技能 / 区域 / 干员切换·战斗判定（`shooter.rs` / `range.rs` / `skill.rs` / `zone.rs` / `switch_operator`） |
| `damage` | 伤害结算流水线（packet / resolver / effect） |
| `element` | 元素反应系统 |
| `map` | 纯数据地图（`map::training::layout()` 直接渲染完整 CQB 室内训练场） |
| `model` | 模型文件（易变化资源）经快照下发客户端 |
| `net` | TCP 网络层（AOI / 会话 / 广播 / 协议：Loadout / StartTraining / ExtractRequest / Ping） |
| `config` / `operator` / `player` / `equipment` / `gamemode` / `hal` | 配置 / 干员 / 档案 / 装备 / 模式 / 时钟 |

### 目录结构总览（表3 · 外部资源与配置）

| 目录名 | 用途 | 文件格式 / 注意事项 |
|---|---|---|
| `assets/` | 美术资源（游戏内加载） | 子目录：`characters/` `environment/` `fonts/` `ui/` `weapons/`；`.jpg`/`.png`/`.ttf`（中文字体 `simhei.ttf`）+ `model/*.json`（体素模型） |
| `ServerCode/config/` | **配置表（含加载器，与核心代码物理相邻）** | `element_reactions.yaml`：无头模拟与主机端共用；**单一事实来源**——默认值由 `include_str!` 编译期嵌入，运行时同路径文件作为设计师热改覆盖；改表需同步重编译默认或改同文件 |
| `tools/` | 开发辅助脚本 | PowerShell（图标生成、窗口截图、UI 自动测试等） |
| `.github/workflows/` | CI | `rust.yml`：push/PR 到 `main` 自动跑 `cargo build` + `cargo test`（不带 demo） |
| `.agents/skills/` | AI 协作工作流文档 | 美术创作 / 地图验收等技能的说明文档 |
| `CuteOfDuty_Demo.exe` | 预编译 3D Demo | 双击即玩；启动自动定位项目根目录（向上搜索 `ServerCode/config/element_reactions.yaml`）|

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

## 四、网络架构规划（未来部署上线）

> **本节为未来线上部署的技术选型与架构蓝图**（部分已在 ServerCode 落地，见 [五、服务端进度](#五服务端进度servercode)）。
> Cute Of Duty 的玩法底座决定它的网络层形态：**长 TTK + 元素反应 + 撤离式搜打撤**，
> 天然适合 TCP 服务器权威的可靠同步模型，而非毫秒级瞬时反应的 UDP 快节奏同步。

### 4.1 设计目标：为什么是「长 TTK + TCP 服务器权威」

Cute Of Duty 1 采用**长 TTK** 设计——基础击杀时间约 **5 秒**，叠加元素反应后可拖到数分钟。
这不同于传统 FPS 的"见面即死"：

- 战斗强调**战术拉扯、技能配合与持续输出**，而非毫秒级瞬时反应；
- 每一次命中的权重都被稀释，玩家对瞬时网络同步的苛刻要求显著降低；
- 网络波动导致的短暂卡顿，玩家有充足时间调整战术，不会出现"见面即死"的恶性体验。

这一设计直接决定了网络层选型：**基于 TCP 的服务器权威架构**（参考 Minecraft Java 版的
可靠同步模型），而非 UDP + 快照插值。

### 4.2 服务器权威架构

服务端作为**游戏世界的唯一真理源**，掌握全部**热数据**：

- 地图区块与方块状态
- 实体位置与状态
- 元素反应状态（DoT / 冰冻 / 毒域等持续效果）
- 背包数据
- 战局 / 赛季 / 信誉进度

客户端仅作为**渲染与输入的表现层**，定期从服务端拉取最新状态快照以保持同步。
TCP 的可靠传输能更好地保障**元素状态、技能效果、背包交互**等关键数据的准确同步。

### 4.3 AOI 兴趣区域与移动实体剔除

服务端实现**移动实体剔除（AOI / 兴趣区域）机制**：

- 每个客户端只收到**其视野范围内**或 **AOI 兴趣区域内**的实体数据；
- 超出范围的实体不参与该客户端的同步；
- 既降低 TCP 带宽压力，也从根源上杜绝 **ESP 透视类外挂**——客户端根本不知道视野外有什么。

### 4.4 客户端开源 + 反作弊：开源不会破坏完整性

Cute Of Duty 是全开源（GPL-3.0-with-linking-exception）。客户端开源本可被质疑"外挂层出不穷"，
但**服务器权威架构天然化解这一矛盾**：

- 所有**热数据由服务端权威管理**，客户端代码的开放不会破坏游戏世界的完整性；
- 对地图、实体、状态的任何修改请求，都必须经过**服务端校验**；
- 从根本上杜绝：**地图篡改、穿墙、刷物品**等作弊行为；
- TCP 的稳定协议同时降低了第三方工具与社区 MOD 的开发门槛，良性反哺开源生态。

### 4.5 网络层与玩法基座的协同

| 玩法/工程属性 | 与网络层的衔接 |
|---|---|
| 长 TTK（基础 ~5s / 元素反应可达数分钟） | 削弱对瞬时同步的依赖，TCP 可靠传输足以支撑 |
| 元素反应持续状态 | 依赖可靠的逐包送达，TCP 按序保数据一致 |
| 服务器权威（唯一直理源） | 热数据全在服务端，客户端仅表现层 |
| AOI 剔除 | 控带宽 + 根治 ESP 透视外挂 |
| 客户端全开源 | 服务器校验兜底，开源不破坏完整性，反哺 Mod 生态 |

---

## 五、服务端进度（ServerCode）

> 自 **0.6.0** 起，项目由「单 crate + feature 门控 Bevy Demo」重构为 **Cargo Workspace 物理分离**：
> `ServerCode`（服务端权威模拟 + TCP 网络层）与 `HostCode`（客户端表现层）。
> 横切红线：**服务端算、客户端显示**——任何"应该算什么"（血量、背包、CD、战局、归属判定、
> 模型身份）必须服务端权威；客户端只求快照 + 画。

### 5.1 服务端模块与职责

| 模块 | 职责 | 关键文件 |
|---|---|---|
| `engine` | 权威游戏循环（60Hz 固定 Tick）+ 确定性双缓冲快照 | `double_buffer.rs` / `pre_explosion_cache.rs` |
| `entity` | 自研 ECS：实体 = 组件容器 | `mod.rs`（`EntityType` 等） |
| `combat` | 战斗判定：射击 / 手雷 / 技能 / 区域 / 战斗者 | `shooter.rs`（相机射线）+ `range.rs`（靶）+ `grenade.rs`/`skill.rs`/`zone.rs`/`combatant.rs` |
| `damage` | 伤害结算流水线 | `packet.rs` / `resolver.rs` / `effect.rs` |
| `element` | 元素反应系统 | `mod.rs` |
| `map` | 纯数据地图定义（训练场 + 草坪四区） | `training/` + `lawn/`（spawn / engage / search / extract / perimeter） |
| `model` | **模型文件放服务端**（易变化资源） | `mod.rs`（`ModelPreset` 经快照下发客户端） |
| `net` | TCP 网络层：AOI / 会话 / 广播 / 协议 | `aoi.rs` / `session.rs` / `broadcaster.rs` / `protocol.rs` |
| `config` / `operator` / `player` / `equipment` / `gamemode` / `hal` | 配置 / 干员 / 档案 / 装备 / 模式 / 时钟 | 各自 `mod.rs` |

### 5.2 当前进度

> 现状：**0.6-SnapShot-2**（架构仍为服务端权威 + 客户端表现层，核心零 bevy）。

- 服务器权威模拟 + TCP 网络层已落地（`net/`：AOI 兴趣区域剔除、会话管理、状态广播、协议编解码）。
- **网络协议已落地**：`Loadout` / `StartTraining` / `ExtractRequest` / `Ping` 及 serde 用例；
  `session.rs` 映射 4 个新 NetCommand 到对应处理。
- **干员切换 `combat::switch_operator`**（四名干员档案）+ **撤离距离权威判定 `handle_extract`**。
- `map::training::layout()` **直接渲染完整 CQB 室内训练场**（棋盘格地板 + 多材质 + 发光光源）。
- 模型 `model/` 经快照下发 `ModelPreset` 到客户端（易变化资源归服务端）。
- `cargo test --offline` **全绿（94 个用例通过）**。
- 与 HostCode **彻底解耦**：ServerCode **不依赖 bevy**，分离不受渲染层升级影响。
- 构建产物 `cod_server.exe`（服务端）+ `cod1.exe`（客户端），由 workspace 一次并行编译产出。
- **待办（明示）**：仓库携带「带入进图后的生效结算」`apply_loadout` 属训练场后续，
  当前加载仅存会话热副本（自带风险提示）。

### 5.3 运行

- 服务端：`cargo run --bin cod_server`（默认 `cargo build` 已并行产出）
- 客户端（表现层）：`HostCode` 消费服务端权威快照

---

## 六、版本历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 0.6-Snapshot-2（Pre-Release） | 2026-09-25 | **仓库·携带物资 UI 100% 还原 0.3.2**（拖拽 + Shift 左键 + 已携带✓ + 容量计数 + 返回/开始游戏）；协议 `Loadout`/`StartTraining`/`ExtractRequest`/`Ping` 与干员切换、撤离判定落地；launcher 拆平级模块 + 扁平化清理；`cargo test` 94 用例全绿；**带仓库带入属于训练场后续**（`apply_loadout`，当前加载仅存会话热副本，自带风险提示） |
| 0.6.0 | 2026-09-25 | **双 crate workspace + 服务端落地**：重构为 `ServerCode`（服务端权威模拟 + TCP 网络层，含 AOI/会话/广播/协议）与 `HostCode`（客户端表现层），Model 文件归服务端并经快照下发；射击场射线检测系统完成（目标可射击/命中计分/自动往返）；`cargo test` 81 用例全绿；bevy 因底层稳定性问题由 0.19 回退至 0.14（详见「已知坑」底层冻结红线） |
| 0.3.2 | 2026-09-22 | **搜打撤**：物资箱重塑为体素栅格木箱（四角立柱 + 四面通板 + 平顶盖）并接入统一交互菜单（站点优先于拾取，F 必开箱不误拾，弃用自建触发）；对局仓库/背包拖拽选装落地并打通 Tab 背包；核心差异化补「与热门友商 FPS 对比」定位表；Demo 操作方式改为「按键组 × 触发环境」矩阵排版（A/B 环境列为玩法环境预留，当前标 `—`） |
| 0.3.1 | 2026-09-21 | **渲染内存泄漏定向修复 + README 翻新**：修复 `damage_popup_system` 相机缺失时弹字永久存活的确定性泄漏；新增 `effect_guard.rs`（五类高频特效硬性存活上限兜底）；收敛特效密度/寿命（命中粒子 5→3、爆炸碎块 10→4 等）；新增 `debug_tracer.rs`（每 5s 实体/资产采样，供定位残余增长） |
| 0.3.0 | 2026-09-20 | **反屎山扁平化重构**：`config/` 并入 `src/config/`（YAML `include_str!` 嵌入 + 运行时覆盖，单一事实来源）；全部 >500 行上帝文件拆成语义化子模块（`damage`→packet/resolver/effect，`map/training`→分区分文件，`demo` 的 `inventory`/`menu`/`hud`/`combat`→面板目录，`model`→operator_models 等）；`common.rs`→`frontend.rs`；新增 [CONTRIBUTING.md（反屎山公约）](CONTRIBUTING.md) 与目录结构表格 |
| 未发布（main） | 2026-09-11 | GitHub Actions CI 接入；`src/map/training.rs` 扁平化重构并修复 CI 构建错误；设置面板新增「开源代码鸣谢」；`src/demo` 由 7300 行单文件拆分为 15 个功能子模块 |
| 0.2.3 | 2026-09-10 | 首个真开源版本：demo3d 独立 crate 并回本包（bevy 改由 feature `"demo"` 门控）；新增 `src/model` 干员模型（Yanhu）与动作系统；仓库以 GPL-3.0-with-linking-exception 开源 |
| 0.2.1 | 2026-09-06 | 0.2 hotfix 1：主界面改版（右下角「切换模式/开始游戏」、右侧 60% 分类+模式选择面板、右上角齿轮设置浮层）；workspace 解耦（核心库零 bevy、配置表全量生效） |
| 0.1.1 | 2026-09-05 | 首个对外分享打包版：补齐交接文档（本 README）、`.gitignore` |
| 0.1.0 | — | 内部开发版：核心库 + 无头模拟 + 3D Demo 全部跑通 |

---

## 七、文档

* **项目规范**
  * [贡献指南（反屎山公约：600 行上限 / 无循环依赖 / 语义化命名）](CONTRIBUTING.md)
  * [开源许可证 GPL-3.0-with-linking-exception（原文）](LICENSE)
* **AI 协作工作流（`.agents/skills/`）**
  * [游戏美术创作指南](.agents/skills/)
  * [地图建模验收流程](.agents/skills/)

---

## 八、开发环境说明与已知坑

1. 项目为 **Cargo Workspace**：`HostCode`（客户端表现层）+ `ServerCode`（服务端权威模拟 + 网络层）。
   `cargo build` 会并行编译出 `cod1.exe` 与 `cod_server.exe`；ServerCode **不依赖 bevy**，核心模拟秒级增量迭代。
2. 编译一律走 **`tools/cargo-wrap.exe`**：把 cargo/rustc 归入「Rust 编译器」作业以便任务管理器折叠，
   并把并行度钳制为 **`-j4`**（防 CPU / 进程数爆炸）。并行度用环境变量 `CARGO_WRAP_JOBS` 覆盖。
3. **编译缓存禁用以省磁盘**：`target/` 与 `tools/cargo-wrap/target/` 不入库；多轮构建会累积较大
   二进制（release 且 LTO 时每份可达数百 MB～GB），需定期 `cargo clean` 释放空间。
4. **CI**：`.github/workflows/rust.yml` 在 push / PR 到 `main` 时自动跑 `cargo build` + `cargo test`
   （提交前本地跑一遍同样命令可提前发现问题）。

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

### 已知坑（开发 / 部署实测）

- **底层冻结红线（2026-09-25 起生效）**：在 bevy 及其大版本依赖（wgpu / naga / winit / glam 等）出稳定
  版本之前，**不要更新底层**。曾把 bevy 升到 0.19 又因大量 API 变动与稳定性问题回退到 0.14
  （本机离线缓存 0.14.2，`cargo check --offline` 通过；渲染代码用 0.14 的
  `MaterialMeshBundle` / `PbrBundle` / `DirectionalLightBundle` / `Camera3dBundle`、
  `Time::elapsed_seconds()`）。ServerCode 不依赖 bevy，回退不影响服务端分离。
- **Windows debug 构建 Bevy 0.19 可能产出 >2GB 可执行文件，报 `os error 193`（无效 Win32 程序）**：
  用 release 构建或优化 dev profile 规避。
- **release 构建偶发 `os error 3`（路径找不到）**：编译 bevy crate 写 `.fingerprint` 时失败，非代码错误，
  疑似 target 残留 + LTO / `codegen-units=1` 重负载；重试会触发整树重建（约 40 分钟），必要时先 `cargo clean`。
- **ServerCode 遗留 dead_code 告警**：`combat/shooter.rs` 的 `Vec3Helper::dot` 暂未被调用，属无碍告警，
  后续接入近战/命中反馈时可复用。
- **历史遗留（0.5 单 crate 架构，已随重构隔离到 `legacy/0.1-0.5` 分支）**：返回主菜单时曾崩溃/连带销毁窗口、
  进场即现 `B0004` 层级损坏洪水，根因系聚合根 / 层级挂接与 teardown 方案冲突，已在新架构中改用
  服务端权威 + 客户端快照模式规避。

---

## 九、目录结构

```
CuteOfDutyAlpha/
├── Cargo.toml / Cargo.lock       # Cargo Workspace 根（虚拟 manifest，声明 HostCode + ServerCode 成员）
├── CONTRIBUTING.md               # ⚠️ 反屎山公约（贡献前必读）
├── README.md                     # 本档案（交接文档）
├── .github/workflows/rust.yml    # CI：push/PR 到 main 跑 cargo build + cargo test
├── .agents/skills/               # AI 协作工作流文档（美术创作 / 地图验收）
├── tools/                        # cargo-wrap（编译封装，产物不入库）+ PowerShell 辅助脚本
├── HostCode/                     # 客户端表现层（launcher 模块：动态装载 bevy_dylib + 渲染/快照消费）
└── ServerCode/                   # 服务端权威模拟 + TCP 网络层（详见「五、服务端进度」）
    ├── lib.rs / main.rs          # 核心库 + cod_server 入口
    ├── config/                   # 配置加载器 + element_reactions.yaml（单一事实来源）
    ├── engine/ entity/ combat/ damage/ element/ map/ model/ net/
    ├── operator/ player/ equipment/ gamemode/ hal/
    └── (构建产物 target/ 已 git 忽略)
```

> 一位开发者接手前，只需要读三份：**本 README（概览）** → **服务端进度与架构表** → **CONTRIBUTING.md（公约）**。
> 核心业务模块保持 ≤ 2 层深度、每个 `.rs` ≤ 600 行、禁止 `utils.rs` 之类的语义化空壳——这些是硬约束，不是建议。