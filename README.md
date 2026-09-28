<div align="center">

# Cute Of Duty 1: Simple

**战术撤离射击游戏** · 核心差异化 **元素互斥生态 + 反护航经济架构**

基于 Rust + Bevy 0.14 的 3D 像素风 FPS · 服务端权威模拟与客户端表现层双 crate
配置文件表驱动的全部玩法规则 · 单一事实来源

[![License: GPL-3.0 (code)](https://img.shields.io/badge/License-GPL--3.0--linking--exception-blue.svg)](LICENSE)
[![License: CC BY-NC-SA 4.0 (assets)](https://img.shields.io/badge/License-CC_BY--NC--SA_4.0-lightgrey.svg)](LICENSE-ASSETS)
[![Version](https://img.shields.io/badge/Version-0.10.0-blue.svg)](#五版本历史)
[![Rust](https://img.shields.io/badge/Rust-stable%20%28edition%202021%29-orange.svg)](Cargo.toml)

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

> 先运行 **`target/release/cod_server.exe`**（服务端权威模拟），再运行
> **`target/release/cod1.exe`**（客户端表现层）。**顺序不可颠倒**——服务端未起时客户端不会进入训练场
> （客户端已实现每 2s 自动重连，服务端起来后会自动接入）。

### 本地编译

环境要求：**Rust stable**（edition 2021，无需 nightly），Windows 10/11 或 Linux 均可。
编译一律走封装工具 `tools/cargo-wrap.exe`（把 cargo/rustc 归入「Rust 编译器」作业便于任务管理器折叠，并行度钳制 `-j4`）：

```powershell
cargo-wrap check --workspace                # 全工作区检查
cargo-wrap test -p cute_of_duty_server      # 服务端全部测试（不编译 bevy，秒级完成）
cargo-wrap build --release --workspace      # 发布构建，产出 cod_server.exe + cod1.exe
```

> 项目为 **Cargo Workspace**：`ServerCode`（服务端权威模拟 + TCP 网络层）与
> `HostCode`（客户端表现层，动态装载 `bevy_dylib`）。默认构建**完全不编译 bevy**，核心逻辑秒级增量迭代；
> 只有构建客户端时才触发 Bevy 全量编译。Windows 下 debug 产物可达 >2GB 并触发 `os error 193`，故用 release。

### Demo 操作方式

| 按键组 | 具体按键 | 默认触发 |
|---|---|---|
| 视角 | 鼠标 | 自由视角（X 轴偏航 / Y 轴俯仰，灵敏度独立，俯仰限制仰 50° / 俯 70°） |
| 移动 | W / A / S / D | 前后左右位移（始终相对相机方向） |
| 跳跃 | Space | 起跳（仅在地面时） |
| 疾跑 | 左 Ctrl | 常速 4 → 7 单位/秒（按住） |
| 主武器切换 | 1 / 2 | 在两把主武器间切换 |
| 干员技能 | Q / E | 技能（点燃 DoT / 冰冻 / 位移冲刺 / 毒素领域） |
| 越肩瞄准 | 鼠标右键（按住） | SpringArm 由右肩后方 6.5m 过渡到 2.4m（0.22s），FOV 收窄 28%，准星琥珀，移速降至 55% |
| 射击 / 投掷 | 鼠标左键 | 射击（相机射线，靶心弱点 ×1.8）；持雷时改为投掷 |
| 交互 | F | 呼出**居中交互面板**（功能台 + 拾取物 + 物资箱，站点优先）；滚轮翻页切换高亮，F/回车确认，Esc 关闭 |
| 背包 | Tab | 打开背包（双武器 / 弹药池 / 补给品） |
| 使用物品 | R（悬停背包物品） | 使用悬停的背包物品 |
| 快捷道具 | 3 / 4 | 短按速用首件 / 长按开径向轮盘；3 恢复品（医疗包回血 +50）；4 战术手雷——**先持握并强制越肩**，左键投出（70 伤 / 5m 半径）/ Esc 取消放回背包 |
| 操作按钮组 | B | 打开可点击操作按钮面板（逐键触发移动/动作/使用/面板），打开即释放光标；Esc 或 B 关闭 |
| 关闭 / 取消 | Esc | 关闭背包 / 功能台 / 取消持雷 / 无 UI 时释放鼠标 |
| 暂停 | / 或 ~ | 暂停菜单（返回游戏 / 设置 / 回主界面） |
| 延迟面板 | CapsLock | 显示/隐藏到服务器的通信延迟列表（逐玩家毫秒） |

> **越肩瞄准（SpringArm 相机，参考原神弓手瞄准模式）**：相机层级为
> 脚底 Pivot（不随模型旋转）→ ShoulderPivot（Yaw）→ PitchPivot（Pitch）→ SpringArm → Camera；
> 默认机位右肩 +0.55 / 眼高 ≈2.95 / 后方 6.5，瞄准机位右肩 +1.0 / 后方 2.4（0.22s smoothstep）；
> SpringArm 撞墙缩回（贴墙最小 0.7m）、离墙缓伸，地面高度钳制 ≥0.35m；射击判定自相机视线出发（与准星一致），
> 曳光从枪口收敛到命中点；命中靶板中心红心判定弱点（×1.8 伤害，即时射线无下坠）；手雷保持 12 m/s² 重力抛物线，
> 且必须"先瞄准后释放"。

---

## 二、核心差异化卖点

| 卖点 | 说明 |
|---|---|
| **元素互斥生态** | 火 / 冰 / 电 / 毒等元素并非"越堆越强"。护甲与武器的元素互斥、同源元素协同增益、环境修正全部由配置表驱动——选型本身就是博弈 |
| **反护航经济架构** | 装备等级不是保障线而是风险线：1 级新手保护舱 → 2–6 级可指定元素（成本翻倍）→ 7–9 级真随机混沌区；转售 / 给予会重置元素。高等级=高收益+高不确定性 |
| **配置表驱动的全部规则** | 数值、元素反应、干员档案、地图布局一律沉淀为 YAML / 纯数据结构，核心库**不硬编码任何玩法**，改平衡不用动代码 |
| **零 bevy 的核心库** | 游戏逻辑与渲染彻底分离：`cargo test` 秒级完成，Bevy 由客户端独占，核心迭代几乎无编译负担 |

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
                    ┌────────────────────────────────────────────┐
                    │  Cargo Workspace（虚拟 manifest）            │
                    └───────┬──────────────────────────┬─────────┘
              ┌─────────────▼────────────┐  ┌──────────▼─────────────────────┐
              │ ServerCode（领域层 + 基础设施）│  │ HostCode（客户端表现层）          │
              │ 权威 60Hz Tick · 核心零 bevy │  │ 动态装载 bevy 0.14 · 只吃快照 + 画 │
              │ TCP 网络层 / 配置 / 存档     │  │ launcher 纯装配 + HUD / 相机 Rig  │
              └────────────────────────────┘  └────────────────────────────────┘
                              ▲                          ▲
                              └──── ContractCode（契约层：线格式 + Port Trait）┘
```

**三条铁律**：

1. **横切红线——服务端算、客户端显示**：任何"应该算什么"（血量、背包、CD、战局、归属判定、模型身份）
   必须服务端权威；客户端只求快照 + 画。
2. **客户端不硬编码规则**：数值、反应、地图、干员档案全部配置表/纯数据驱动。
3. **核心库不依赖 bevy**：`ServerCode` 与 `ContractCode` 永不编译 bevy，可无头确定性模拟。

### 目录结构总览

| 模块 | 职责 | 关键文件 |
|---|---|---|
| **客户端 · `HostCode/`** | | |
| `launcher` | 纯装配层：动态装载 bevy 0.14 + 承载渲染表现全套（相机 Rig / 体素绘制 / HUD / 小地图 / 背包 UI），只吃快照 + 画 | `mod.rs` |
| `flow` | AppState 状态机（Loading / MainMenu / InGame）、加载屏、状态迁移 | `flow_state.rs` / `loading.rs` |
| `net` | mpsc 后台线程消费服务端快照 + 上行 NetOut 命令通道 | `network.rs` |
| `menu` | 主菜单 / 仓库·携带物资（拖拽 + Shift 选装）/ 模式 / 设置 / 暂停 | `menu_main.rs` / `arsenal.rs` / `pause.rs` |
| `hud` | 血条 / 护甲 / 弹药 / 技能 CD / 小地图 / 战术大地图 / 背包面板 / 击杀通告 / 撤离提示 | `hud_vitals.rs` / `hud_minimap.rs` / `hud_bigmap.rs` / `hud_backpack_panel.rs` / `hud_feed.rs` |
| `world` | 训练场几何 / 材质 / 光照 / 手雷弹道预览 | `world_scene.rs` / `camera.rs` / `grenade_preview.rs` |
| `shared` | 主题色板、字体句柄 | `theme.rs` |
| **服务端 · `ServerCode/`** | | |
| `engine` | 权威 60Hz 固定 Tick + 确定性双缓冲快照 | `double_buffer.rs` |
| `entity` | 自研 ECS（实体 = 组件容器） | `mod.rs` |
| `combat` | 射击 / 手雷 / 技能 / 区域 / 干员切换·战斗判定 | `shooter.rs` / `grenade.rs` / `skill.rs` / `zone.rs` / `switch_operator.rs` |
| `damage` | 伤害结算流水线 | `packet.rs` / `resolver.rs` / `effect.rs` |
| `element` | 元素反应系统 | `mod.rs` |
| `map` | 纯数据地图（训练场 CQB + `lawn` 1×1km 露天搜打撤大场） | `training/` + `lawn/` |
| `model` | 模型文件（易变化资源）经快照下发客户端 | `mod.rs` |
| `net` | TCP 网络层（AOI / 会话 / 广播 / 协议） | `aoi.rs` / `session.rs` / `broadcaster.rs` / `protocol.rs` |
| `config` / `operator` / `player` / `equipment` / `gamemode` / `hal` | 配置 / 干员 / 档案 / 装备 / 模式 / 时钟 | 各自 `mod.rs` |
| **契约 · `ContractCode/`** | 线格式类型 + 跨域载荷 + 共享常量 + Port Trait（不依赖两端） | `docs/contracts/protocol.yaml` 为其机器可读描述 |

### 外部资源与配置

| 目录 | 用途 | 说明 |
|---|---|---|
| `HostCode/menu/` | 客户端美术资源（随 exe 相对包根加载） | `icon/settings.png`（菜单齿轮图标）、中文字体 `zcool_kuaile.ttf`（附 OFL 许可） |
| `ServerCode/config/` | **配置表（含加载器，与核心代码物理相邻）** | `element_reactions.yaml`：**单一事实来源**——默认值由 `include_str!` 编译期嵌入，运行时同路径文件作为设计师热改覆盖 |
| `ServerCode/model/` | 模型文件（易变化资源） | 体素几何/动画 JSON；握手后经 `ModelCatalog` 下发客户端渲染本人模型 |
| `tools/` | 开发辅助 | `cargo-wrap`（编译封装，产物不入库）+ PowerShell 辅助脚本 |
| `.agents/skills/` | AI 协作工作流文档 | 美术创作 / 地图验收等技能说明 |
| `.github/workflows/rust.yml` | CI | push/PR 到 `main` 自动跑 `cargo build` + `cargo test` |

**配置加载语义（`element_reactions.yaml`）**：**权威默认** = 编译期 `include_str!` 嵌入的同目录 YAML；
**运行时覆盖** = 从项目根向上搜索同路径 YAML（供热改表）；**文件缺失** → 回退嵌入默认；
**文件存在但解析失败** → 启动直接报错退出（改错表当场失败，而非静默用默认值）。

### 如何新增一张地图

1. 新建 `ServerCode/map/<名称>/mod.rs`（或单文件 `<名称>.rs`），实现返回 `MapLayout` 的 `layout()`（照抄 `training/` 写法）；
2. 在 `map/mod.rs` 里 `pub mod <名称>;`
3. 客户端无需改动渲染逻辑——通用渲染器 `spawn_map_layout` 自动处理网格、材质、光照；
4. 跑 `cargo-wrap test` 确认布局约束（边界、通道净空、遮挡不与掩体相交等）全部通过。

---

## 四、通信协议

> Cute Of Duty 的玩法底座决定它的网络层形态：**长 TTK + 元素反应 + 撤离式搜打撤**，
> 天然适合 **TCP 服务器权威的可靠同步模型**，而非毫秒级瞬时反应的 UDP 快节奏同步。

### 4.1 为什么是「长 TTK + TCP 服务器权威」

- 基础击杀时间约 **5 秒**，叠加元素反应后可拖到数分钟——战斗强调**战术拉扯、技能配合与持续输出**，
  而非毫秒级瞬时反应；每次命中的权重被稀释，玩家对瞬时同步的苛刻要求显著降低；
- 网络波动导致的短暂卡顿有充足时间调整战术，不会出现"见面即死"的恶性体验；
- 由此选定**基于 TCP 的服务器权威架构**（参考 Minecraft Java 版的可靠同步模型），而非 UDP + 快照插值。

### 4.2 服务器权威

服务端作为**游戏世界的唯一真理源**，掌握全部**热数据**：地图区块、实体位置与状态、
元素反应状态（DoT / 冰冻 / 毒域）、背包数据、战局 / 赛季 / 信誉进度。
客户端仅作**渲染与输入的表现层**，定期拉取状态快照保持同步；
TCP 的可靠传输更好保障**元素状态、技能效果、背包交互**等关键数据的准确送达。

### 4.3 AOI 兴趣区域与实体剔除

服务端实现**移动实体剔除（AOI / 兴趣区域）**：每个客户端只收到**其视野范围内**的实体数据，
超出范围的实体不参与同步——既降低 TCP 带宽压力，也从根源上杜绝 **ESP 透视类外挂**（客户端根本不知道视野外有什么）。

### 4.4 客户端开源 + 反作弊

全开源（GPL-3.0-with-linking-exception）本可被质疑"外挂层出不穷"，但**服务器权威架构天然化解矛盾**：
所有热数据由服务端管理，客户端代码开放不破坏世界完整性；对地图、实体、状态的任何修改请求都必须经**服务端校验**；
从根本上杜绝**地图篡改、穿墙、刷物品**等作弊；同时 TCP 的稳定协议降低了第三方工具与社区 MOD 的开发门槛，良性反哺开源生态。

### 4.5 网络层与玩法基座的协同

| 玩法/工程属性 | 与网络层的衔接 |
|---|---|
| 长 TTK（基础 ~5s / 元素反应可达数分钟） | 削弱对瞬时同步的依赖，TCP 可靠传输足以支撑 |
| 元素反应持续状态 | 依赖可靠的逐包送达，TCP 按序保数据一致 |
| 服务器权威（唯一真理源） | 热数据全在服务端，客户端仅表现层 |
| AOI 剔除 | 控带宽 + 根治 ESP 透视外挂 |
| 客户端全开源 | 服务器校验兜底，开源不破坏完整性，反哺 Mod 生态 |

---

## 五、版本历史

### 网络游戏时代（0.6 起）

| 版本 | 日期 | 说明 |
|---|---|---|
| **0.10.0** | 2026-09-28 | **模型修复**：客户端首次可见本人「焰狐」体素模型。①**服务端下发模型目录**（几何+动画随握手一次性下行，协议 `0.10.0`，y+1 加性）；②**焰狐几何重建**——修「四肢左右镜像颠倒 / 枪悬空 1.15m / 狐耳内折」三处硬伤，35→43 盒并优化造型；③**逐盒材质键**（`VoxelCube.mat`）——此前只按骨名着色致细节全被抹平、模型退化为一坨纯色方块，现按 0.3.2 色板逐盒上色；④**朝向随视线**（修「永远向北」）+ 相机按 3.52m 体型重新标定；⑤**扁平化**——解析器 `voxel_spec.rs`→`loader.rs`、两份 JSON 合并为单文件 `FireFox.json`、`mod.rs` 网关重导出保路径稳定。**未做**：多人联机 / 匹配机制 / 无掩体竞技场。**下一版本目标**：`0.6.1` 网游版本（①多玩家 → ②匹配机制 → ③无掩体竞技场）。**本轮冻结（下次修）**：无新增，遗留项见 [docs/frozen-tasks](docs/frozen-tasks/snapshot-8-playtest-feedback.md)。 |
| **0.6（Release）** | 2026-09-27 | **单机落幕 · 正式发布**：收官补齐手雷「先瞄准后释放」持雷态、可点击操作按钮组（`B`）、legacy 操作表逐行核对；修复三条实机缺陷（手雷重力未写回致走直线、新增抛物线预览、释放光标后视角仍转）——协议 `0.9.1`。**下一版本目标：0.6.1 网游版本**（①多玩家 → ②匹配机制 → ③无掩体竞技场） |
| 0.6-Snapshot-1…10（Pre-Release） | 2026-09-25 ~ 09-27 | **由单 crate 迁移为双 crate workspace，并逐快照还原玩法**：确立 `ServerCode`（服务端权威模拟 + TCP 网络层）/ `HostCode`（客户端表现层）物理分离；训练场迁移至 0.3.2 `map::lawn` 1×1km 露天搜打撤大场 + `~` 暂停菜单 + 断线自动重连；越肩瞄准全套（SpringArm + 撞墙避障 + 越肩取景）；地图交互端到端（`F` 交互面板 / 物资箱 / 补给台 4×3 双向格位 / Tab 背包总览）；消耗品 `3`/`4`（医疗包 / 手雷）与手雷「先瞄准后释放」；备用子弹改为可堆叠背包物品（弹药 ×64 / 恢复·战术 ×16 / 工具不可堆叠）；HUD 图标化 + 战术大地图；架构边界体系落地（ADR 0001–0004 / 模块边界 / 契约 YAML / 冻结区）。逐快照完整记录见 [barek-history.md](docs/barek-history.md) |

### 单机时代（0.1 – 0.3.2）

| 版本 | 日期 | 说明 |
|---|---|---|
| 0.3.2 | 2026-09-22 | 搜打撤：物资箱重塑为体素栅格木箱并接入统一交互菜单；对局仓库/背包拖拽选装 + Tab 背包；补「与热门友商 FPS 对比」定位表；操作表改矩阵排版 |
| 0.3.1 | 2026-09-21 | 渲染内存泄漏定向修复（弹字回收 + `effect_guard.rs` 特效硬性存活上限 + 密度/寿命收敛）+ README 翻新 |
| 0.3.0 | 2026-09-20 | 反屎山扁平化重构：`config/` 并入（`include_str!` 嵌入 + 运行时覆盖，单一事实来源）；全部上帝文件拆语义化子模块；新增 CONTRIBUTING.md |
| 未发布（main） | 2026-09-11 | GitHub Actions CI 接入；`src/map/training.rs` 扁平化重构；设置面板新增「开源代码鸣谢」；`src/demo` 由 7300 行拆为 15 个功能子模块 |
| 0.2.3 | 2026-09-10 | 首个真开源版本：demo3d 独立 crate 并回本包（bevy 改 feature 门控）；新增 `src/model` Yanhu 干员模型与动作系统；以 GPL-3.0-with-linking-exception 开源 |
| 0.2.1 | 2026-09-06 | 0.2 hotfix 1：主界面改版；workspace 解耦（核心库零 bevy、配置表全量生效） |
| 0.1.1 | 2026-09-05 | 首个对外分享打包版：补齐交接文档（本 README）、`.gitignore` |
| 0.1.0 | — | 内部开发版：核心库 + 无头模拟 + 3D Demo 全部跑通 |

---

## 六、文档

- **项目规范**
  - [贡献指南（反屎山公约：600 行上限 / 无循环依赖 / 语义化命名）](CONTRIBUTING.md)
  - [代码许可 GPL-3.0-with-linking-exception](LICENSE) —— 适用于全部源代码与配置
  - [资产许可 CC BY-NC-SA 4.0](LICENSE-ASSETS) —— 适用于美术 / 模型 / 音频 / 自有字体
  - [贡献者许可协议 CLA](CLA.md) —— 提交 PR 前必读并同意
- **架构与契约**
  - [模块边界总览](docs/architecture/module-boundaries.md) / [ADR 0001–0004](docs/adr/)
  - [线格式契约 protocol.yaml](docs/contracts/protocol.yaml) / [BarekHistory 变更台账](docs/barek-history.md) / [stop-doing.md 冻结区](docs/stop-doing.md)
- **AI 协作工作流**：`.agents/skills/`（[游戏美术创作](.agents/skills/) / [地图建模验收](.agents/skills/)）

---

## 七、开发环境说明与已知坑

1. 项目为 **Cargo Workspace**：`cargo build` 并行编译出 `cod1.exe` 与 `cod_server.exe`；`ServerCode` **不依赖 bevy**，核心模拟秒级增量迭代。
2. 编译一律走 **`tools/cargo-wrap.exe`**：把 cargo/rustc 归入「Rust 编译器」作业以便任务管理器折叠，并把并行度钳制为 **`-j4`**（防 CPU / 进程数爆炸）。并行度可用环境变量 `CARGO_WRAP_JOBS` 覆盖。
3. **编译缓存禁用以省磁盘**：`target/` 与 `tools/cargo-wrap/target/` 不入库；多轮构建会累积较大二进制（release 且 LTO 时每份可达数百 MB～GB），需定期 `cargo clean` 释放空间。
4. **CI**：`.github/workflows/rust.yml` 在 push / PR 到 `main` 时自动跑 `cargo build` + `cargo test`。

### 已知问题（未解决 / 待复测）

- **视野受限（AOI 60m）**：AOI 兴趣区域半径 60m，大场内远处靶机不进快照因而不可见。场上物资（拾取物 / 功能台 / 出生点物资箱）已由服务端按 `map::lawn` 落成权威实体并经快照下发，出生点即可见可交互。
- **运行顺序**：必须先启动 `cod_server.exe` 再启动 `cod1.exe`（客户端已能自动重连，但服务端未起时不会进入训练场）。
- **渲染内存缓慢增长（已定向缓解，仍待长时间确认）**：长时间游玩（数分钟级）GPU 内存仍会缓慢累积，可能最终 OOM。已做定向修复（弹字回收失效点 + `effect_guard.rs` 硬性存活上限清道夫 + 特效密度/寿命收敛 + `net/snapshot.rs` 实体材质按 `ModelPreset` 缓存根治重复创建），**尚待长时间复验**。诊断可用 `debug_tracer.rs`（每 5s 打印特效实体存活数与 `Mesh`/`Material` 资产表容量）。
- **待复测（0.6 收官批次）**：手雷「先瞄准后释放」持雷态、可点击操作按钮组（`B`）、投掷轨迹预览——均已实现，待实机复测。
- **自动化试玩提示**：若用外部自动化驱动 Demo，winit 可能拦截合成鼠标事件，可用系统级 `mouse_event` 绕过。

### 已知坑（开发 / 部署实测）

- **底层冻结红线（2026-09-25 起生效）**：在 bevy 及其大版本依赖（wgpu / naga / winit / glam 等）出稳定版本前**不要更新底层**。曾把 bevy 升到 0.19 又因大量 API 变动与稳定性问题回退到 0.14（本机离线缓存 0.14.2）。`ServerCode` 不依赖 bevy，回退不影响服务端分离。
- **release 构建偶发 `os error 3`（路径找不到）**：编译 bevy crate 写 `.fingerprint` 时失败，非代码错误，疑似 target 残留 + LTO / `codegen-units=1` 重负载；重试会触发整树重建，必要时先 `cargo clean`。
- **ServerCode 遗留 dead_code 告警**：`combat/shooter.rs` 的 `Vec3Helper::dot` 暂未被调用，属无碍告警，后续接入近战/命中反馈时可复用。

---

> 一位开发者接手前，只需读三份：**本 README（概览）** → **CONTRIBUTING.md（公约）** → **`ServerCode/` 各模块的 `mod.rs` Why 注释**。
> 核心业务模块保持 ≤ 2 层深度、每个 `.rs` ≤ 600 行、禁止 `utils.rs` 之类的语义化空壳——这些是硬约束，不是建议。