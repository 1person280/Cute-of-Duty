<div align="center">

# Cute Of Duty 1: Simple

**战术撤离射击游戏** · 核心差异化 **元素互斥生态 + 反护航经济架构**

基于 Rust + Bevy 0.16 的 3D 像素风 FPS · 服务端权威模拟与客户端表现层双 crate
配置文件表驱动的全部玩法规则 · 单一事实来源

![Cute Of Duty](docs/media/cover_1280x720.png)

**三步跑起来**：① 下载 [最新 Release](https://github.com/1person280/Cute-of-Duty/releases) 的双端 zip 并解压 → ② 先运行 `cod_server.exe` → ③ 再运行 `cod1.exe` 进入训练场（详见[快速开始](#一快速开始)）

[![CI](https://github.com/1person280/Cute-of-Duty/actions/workflows/rust.yml/badge.svg)](https://github.com/1person280/Cute-of-Duty/actions/workflows/rust.yml)
[![License: GPL-3.0 (code)](https://img.shields.io/badge/License-GPL--3.0--linking--exception-blue.svg)](LICENSE)
[![License: CC BY-NC-SA 4.0 (assets)](https://img.shields.io/badge/License-CC_BY--NC--SA_4.0-lightgrey.svg)](LICENSE-ASSETS)
[![Version](https://img.shields.io/badge/Version-0.14.1-blue.svg)](#五版本历史)
[![Rust](https://img.shields.io/badge/Rust-stable%20%28edition%202021%29-orange.svg)](Cargo.toml)
[![Server Safe](https://img.shields.io/badge/Server_Safe-by_Let%27s_Encrypt-green.svg)](https://letsencrypt.org)

**外部依赖 · 站在开源社区的肩膀上** · [![by Bevy](https://img.shields.io/badge/by-Bevy-E90000)](https://bevyengine.org)
[![by Tokio](https://img.shields.io/badge/by-Tokio-blue)](https://tokio.rs)
[![by Serde](https://img.shields.io/badge/by-Serde-white)](https://serde.rs)
[![by Tracing](https://img.shields.io/badge/by-Tracing-black)](https://github.com/tokio-rs/tracing)
[![by Rand](https://img.shields.io/badge/by-Rand-4B8BBE)](https://crates.io/crates/rand)
[![by Crossbeam](https://img.shields.io/badge/by-Crossbeam-8E44AD)](https://crates.io/crates/crossbeam)
[![by BLAKE3](https://img.shields.io/badge/by-BLAKE3-2EA44F)](https://crates.io/crates/blake3)

**AI 协作方法论** · [![skill by GrillMe](https://img.shields.io/badge/skill_by-GrillMe-4B3FE3)](https://github.com/mattpocock/skills)

> 完整依赖清单与各库许可证见游戏内「设置 → 开源代码鸣谢」面板。

</div>

<!-- 维护者注意：下方「免责声明」在本项目成为热门项目之前不得删除或弱化（规约见 CONTRIBUTING 与 compliant-delivery 技能）。 -->

> ⚠️ **免责声明**：本项目是**个人独立开发的非商业同人 / 学习项目**，与 **Activision Publishing, Inc.** 及《Call of Duty》系列**无任何关联**，未获其授权、赞助或认可；名称与题材上的相似**仅属致敬**，不代表任何官方关系。代码以 [GPL-3.0 + Linking Exception](LICENSE) 发布，美术资产以 [CC BY-NC-SA 4.0](LICENSE-ASSETS)（**禁止商业使用**）发布；本项目按「现状」提供，**不附任何明示或暗示的担保**。

---

> **一句话定位**：Cute Of Duty 1 站在"生态可互斥、经济反护航"的路线上——每种元素既是
> 收益也可能是反噬，装备等级越高越失控，玩家要做的是**在收益与风险之间权衡**，
> 而不是照搬传统 FPS 的"枪械数值堆叠"。

---

## 一、快速开始

### 直接试玩（无需编译）

**方式一（推荐）：双击 `启动游戏.bat`**——Release zip 内自带（源码见 [`tools/启动游戏.bat`](tools/启动游戏.bat)）：
自动先起服务器、等端口就绪后再启动客户端；客户端退出时自动收尾服务器进程。

**方式二（手动）**：先运行 **`cod_server.exe`**（服务端权威模拟），再运行
**`cod1.exe`**（客户端表现层）。**顺序不可颠倒**——服务端未起时客户端不会进入训练场
（客户端已实现每 2s 自动重连，服务端起来后会自动接入）。

> **单人即可完整游玩**：本地起一个服务器就是完整的单机体验——训练场 CQB 与 1×1km
> 露天搜打撤大场内的靶机、物资箱、功能台均由服务端权威模拟并经快照下发，
> **不需要其他玩家**也能完成「进图 → 搜刮 → 交战 → 撤离」整局循环。

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

### 自建服务器（联机）

服务端与客户端都支持用**第一个命令行参数**指定地址（默认 `127.0.0.1:8888`，仅本机回环）：

```powershell
# 服务器侧：监听所有网卡，供局域网 / 公网联机
cod_server.exe 0.0.0.0:8888

# 玩家侧：连接指定服务器
cod1.exe 服务器地址:8888
```

服务端进程还**内置 Web 入口**（服务器门户 / 运维 API / 浏览器游玩桥），由 `ServerCode/config/web.yaml` 驱动（运行时同路径文件可覆盖，语义同 `element_reactions.yaml`）：

| 项 | 说明 |
|---|---|
| 门户 / 状态页 | `http://服务器IP:8080`（在线人数 / 版本 / 公告） |
| HTTPS | 端口 `8443`，内置证书加载，路径 `certs/fullchain.pem` + `certs/privkey.pem`（`web.yaml` 可改） |
| 运维 API | `/api/status` · `/api/players` · `/api/announce` · `/api/kick`，Bearer token 鉴权 |
| token 配置 | **只从环境变量读取**（变量名默认 `COD_WEB_TOKEN`，可在 `web.yaml` 改名）——**不落配置文件、不写日志** |

> 安全建议：Web 明文端口（8080）建议仅内网使用或置于反向代理后；token 不要写进任何仓库、脚本或聊天窗口。

### Demo 操作方式

<details>
<summary><b>完整操作表（点开展开）</b></summary>

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

</details>

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
| **Cute Of Duty**（Rust + Bevy 0.16） | ✅ 全开源 GPL-3.0-with-linking-exception | ✅ **配置表驱动**：改玩法 = 改 YAML，社区即可做平衡 Mod | **核心零 bevy + 改表即生效**，`cargo test` 秒级验证 | **服务端权威 + 核心逻辑与服务端物理分离**：核心零 bevy、可无头确定性模拟，渲染为客户端表现层 |
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

Cargo Workspace（虚拟 manifest）下平级三个 crate：

| Crate | 角色定位 | 内容 | 依赖 bevy |
|---|---|---|---|
| **ServerCode** | 领域层 + 基础设施（**权威真理源**） | 权威 60Hz Tick · TCP 网络层 · 配置 · 存档 | ❌ 核心零 bevy |
| **HostCode** | 客户端表现层（**只吃快照 + 画**） | 动态装载 bevy 0.16 · `launcher` 纯装配 + HUD / 相机 Rig | ✅ 独占 bevy |
| **ContractCode** | 契约层（被两端共用，**不依赖两端**） | 线格式类型 + Port Trait + 共享常量 | ❌ |

> 数据流向：`ServerCode`（权威算）— 线格式/Port Trait → `HostCode`（只画，消费快照）。

**三条铁律**：

1. **横切红线——服务端算、客户端显示**：任何"应该算什么"（血量、背包、CD、战局、归属判定、模型身份）
   必须服务端权威；客户端只求快照 + 画。
2. **客户端不硬编码规则**：数值、反应、地图、干员档案全部配置表/纯数据驱动。
3. **核心库不依赖 bevy**：`ServerCode` 与 `ContractCode` 永不编译 bevy，可无头确定性模拟。

### 目录结构总览

| 模块 | 职责 | 关键文件 | 统计数据（行） |
|---|---|---|---|
| **客户端 · `HostCode/`** | | | **合计 10,093** |
| `launcher` | 纯装配层：动态装载 bevy 0.16+ 承载渲染表现全套（相机 Rig / 体素绘制 / HUD / 小地图 / 背包 UI），只吃快照 + 画 | `mod.rs` | 264 |
| `flow` | AppState 状态机（Loading / MainMenu / InGame）、加载屏、状态迁移、唯一 `ModalState` | `state.rs` / `loading.rs` / `modal.rs` | 592 |
| `net` | mpsc 后台线程消费服务端快照 + 上行 NetOut 命令通道 + 16MB 定址远程对象池 | `network.rs` / `remote.rs` / `snapshot.rs` | 1,353 |
| `menu` | 主菜单 / 仓库·携带物资（拖拽 + Shift 选装）/ 模式 / 设置 / 暂停 | `mod.rs` / `arsenal/` / `pause.rs` | 2,172 |
| `hud` | 血条 / 护甲 / 弹药 / 技能 CD / 小地图 / 战术大地图 / 背包面板 / 径向轮盘 / 交互·物资箱 / 击杀通告 / 撤离提示 | `vitals.rs` / `minimap.rs` / `bigmap.rs` / `backpack/panel.rs` / `item/wheel.rs` / `loot/panel.rs` / `kill/counter.rs` | 4,393 |
| `world` | 训练场几何 / 材质 / 光照 / 手雷弹道预览 / 体素模型绘制 | `scene.rs` / `camera.rs` / `grenade/preview.rs` / `voxel/` | 1,163 |
| `shared` | 主题色板、字体句柄、干员元数据、UI 资源 | `theme.rs` / `operator/meta.rs` / `ui/assets.rs` | 135 |
| **服务端 · `ServerCode/`** | | | **合计 10,465** |
| `engine` | 权威 60Hz 固定 Tick + 确定性双缓冲快照 + 爆炸缓存 | `mod.rs` / `double/buffer.rs` / `explosion/cache.rs` | 650 |
| `entity` | 自研 ECS（实体 = 组件容器） | `mod.rs` | 544 |
| `combat` | 射击 / 手雷 / 技能 / 区域 / 干员切换·战斗判定 | `shooter.rs` / `grenade.rs` / `skill.rs` / `zone.rs` / `range.rs` / `combatant.rs` | 1,767 |
| `damage` | 伤害结算流水线 | `packet.rs` / `resolver.rs` / `effect.rs` | 692 |
| `element` | 元素反应系统 | `mod.rs` | 372 |
| `map` | 纯网关（真身在契约 `ContractCode/map/`） | `mod.rs` | 9 |
| `model` | 模型文件（易变化资源）经快照下发客户端 | `loader.rs` / `mod.rs` | 235 |
| `net` | TCP 网络层（AOI / 会话 / 广播 / 预取 / 运行时 / 阶段编排）+ 原生 Web 子域（HTTP / WS / TLS） | `runtime.rs` / `session.rs` / `broadcaster.rs` / `prefetch.rs` / `stages.rs` / `web/` | 2,798 |
| `config` | 配置表（含加载器，与核心代码物理相邻） | `mod.rs` | 197 |
| `operator` | 干员（网关，真身在契约 `ContractCode/operator.rs`） | `mod.rs` | 9 |
| `player` | 档案 | `mod.rs` | 415 |
| `equipment` | 装备 | `mod.rs` | 375 |
| `gamemode` | 模式 | `mod.rs` | 394 |
| `hal` | 时钟 | `mod.rs` | 354 |
| `interact` | 交互菜单 / 供应品类规则（类型真身在契约） | `mod.rs` | 329 |
| `inventory` | 背包服务 | `service.rs` | 258 |
| `items` | 物品逻辑（类型真身在契约） | `mod.rs` | 418 |
| `storage` | 存档（JSONL 日志 + 冷库） | `log.rs` / `error.rs` / `cold/repo.rs` | 335 |
| **契约 · `ContractCode/`** | 线格式类型 + 跨域载荷 + 共享常量 + Port Trait（不依赖两端）；`docs/contracts/protocol.yaml` / `web.yaml` 为其机器可读描述 | | **合计 4,095** |
| `net/` | 定长包 / 指令优先调度 / 帧编解码 / 资源流（TCP 与浏览器桥共用） | `packet.rs` / `codec.rs` / `protocol.rs` / `scheduler.rs` / `stream.rs` | 1,784 |
| `map/` | 纯数据地图（训练场 CQB + `lawn` 1×1km 露天搜打撤大场） | `training/mod.rs` / `lawn/mod.rs` | 1,461 |
| 根类型文件 | 共享值类型（items / interact / equipment / model / operator / combat / element）+ Port Trait | `lib.rs` / `items.rs` / `interact.rs` / `equipment.rs` / `model.rs` / `operator.rs` / `port.rs` | 850 |
| **全仓合计** | 三个 crate 全部 `.rs` 行数 | — | **合计 24,653** |

### 外部资源与配置

| 目录 | 用途 | 说明 |
|---|---|---|
| `HostCode/menu/` | 客户端美术资源（随 exe 相对包根加载） | `icon/settings.png`（菜单齿轮图标）、中文字体 `zcool_kuaile.ttf`（附 OFL 许可） |
| `ServerCode/config/` | **配置表（含加载器，与核心代码物理相邻）** | `element_reactions.yaml`：**单一事实来源**——默认值由 `include_str!` 编译期嵌入，运行时同路径文件作为设计师热改覆盖 |
| `ServerCode/model/` | 模型文件（易变化资源） | 体素几何/动画 JSON；握手后经 `ModelCatalog` 下发客户端渲染本人模型 |
| `tools/` | 开发辅助 | `cargo-wrap`（编译封装，产物不入库）+ PowerShell 辅助脚本 |
| `.agents/skills/` | AI 协作工作流文档 | 改动前拷问 / 美术创作 / 地图验收 / 合规交付 四份技能说明 |
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

### 4.6 传输层 · 小定长包 + 指令优先组包 + 双通道（0.12.0 起）

0.11 的恒定 64KB 槽帧对"高频小控制帧"极不划算（一条 ~200B 上行意图占满 64KB）。0.12 改为**小定长包 + 指令优先组包 + 双通道**（`ServerCode/net/packet.rs`）：

- **主通道恒 256B**：= 32B 头 + **7×32B 单元**，每个"指令或数据"占 1 个 32B 单元；`SendScheduler` **指令优先**——每轮先把指令装满 7 个单元，无指令时才发数据切片，一条数据流连续发完，**指令永不被大数据饿死**。
- **指令二进制紧凑**（`codec.rs`）：热路径 `PlayerInput` 把 15 个 bool 位打包进单 32B 单元；含字符串的控制消息降级为 `DataKind::Control` 数据流（仍属指令类、仍优先）。
- **资源通道恒 4096B**：走**独立第二条 TCP 连接**，= 32B 头 + 4064B 负载，单份资源跨多包分片、收侧按 `key` 重组；`ModelCatalog` 展开为"逐份资源 + `ResourceEnd`"。
- **同端口双通道**：单一监听端口，连接**首条包恒为 256B 绑定包**（`sub_kind` = 角色 0=Control / 1=Resource、负载 = 档案名），据此决定此后按 256B 还是 4096B 读取；**两条线程不合并数据包**。
- **客户端 16MB 固定地址远程对象池**（`HostCode/net/remote.rs`，语义不变）：256 个 64KB 槽 —— 前 250 槽（16000KB）**在用缓冲**、末 6 槽（384KB）**预取区**；资源命中即复用，实体因 AOI 突现时**零等待**。淘汰经主通道上行 `PoolSync`。
- **AOI 边缘预取**（`ServerCode/net/prefetch.rs`）：按「到 AOI 边界距离 ÷ 速度」预测最可能进入视野的 6 个实体（算法就绪，端到端推送待接线）。

> 决策记录：[ADR 0006](docs/adr/0006-small-fixed-packet-dual-channel.md)（取代 [ADR 0005](docs/adr/0005-slot-frame-transport.md)）；线格式契约：[protocol.yaml](docs/contracts/protocol.yaml) `transport` 节。

---

## 五、版本历史

> **版本号规则（自 `0.10.0` 起）**：统一 `x.y.z`，协议版本与游戏版本不再分离 —— `x` 游戏内核版本（严重破坏 +1）、`y` 协议版本（不兼容 +1）、`z` 细节版本（无兼容性变化可 +1）；tag / Release / README / Cargo / 契约 YAML 同号。详见 [CONTRIBUTING 第六节](CONTRIBUTING.md)。

### 网络游戏时代（0.6 起）

| 版本 | 日期 | 说明 |
|---|---|---|
| **0.14.2** | 2026-10-06 | **傻瓜上手**（**正式发布** · 仓库首个非预发布版本）：无代码更改的发布治理版，主题是「下载即玩」。①**分发包一键启动**——`CuteOfDuty-*-win64.zip` 新增 `启动游戏.bat`（自动先起服务器、就绪探测后拉起客户端，客户端退出自动收尾服务器进程），`使用说明.txt` 重写（一键步骤 + **资源路径编译期绑定**的运行限制声明，不宣称解压即用）；②**GitHub 宣发基建收口**（计划 [0005](docs/plans/0005-GitHub宣发计划.md)）——README 首屏商店化（头图 / 三步跑起来 / CI 动态徽章 / 长表折叠）、仓库 topics 补全、[bevy-assets#618](https://github.com/bevyengine/bevy-assets/pull/618) 收录已合并、Discussions 开通 + Bug/Mod 提案两类 issue 表单、自建服务器文档化（含 `COD_WEB_TOKEN` 环境变量纪律）；③**契约 YAML 版本号漏网修正**——`protocol.yaml`（`0.14.0`）/ `web.yaml`（`0.13.0`）随动同步为 `0.14.2`（同号规约，属文档修正，非线格式变化）。**代码更改**：无 `.rs`——仅三端 `Cargo.toml` / `Cargo.lock` / 契约 YAML 版本号 + 文档与 `tools/启动游戏.bat`。**扁平化更新**：无。**兼容性**：**不触碰线格式**，`wire_version` 仍为 `14`，双端 `0.14.2` 与 `0.14.x` **互通**（`z+1`）。**未做**：<br>**来自 0.12.2**：<br>①新增玩法/内容<br>②渲染内存增长复验<br><br>**来自 0.12.3**：<br>①浏览器 3D 客户端（→ 迁至 [0006 暂缓项](docs/plans/0006-GitHub宣发暂缓项.md)）<br>②证书热重载 + 运维 API 细权限<br>③HTTPS 冒烟 + `ring` 的 `--release` 编译<br><br>**来自 0.12.4**：<br>①`get_single*` 仍弃用未迁，按纪律不动<br><br>**来自 0.14.0**：<br>①预设不接玩家档案/等级/经济（全局固定）<br>②预设内每项至多 1 件（不做数量语义）<br>③仓库 7 项物资池的手动拖拽模型不变<br>④返回主菜单时客户端背包显示被重置（服务端清单仍保留），两端不一致。<br><br>**已了结**（附凭据）：①「契约 YAML `version:` 未随动」—— 本版把 `protocol.yaml` / `web.yaml` 同步为 `0.14.2`。**下一版本目标**：见〈六、已采纳未来形态〉。**本轮冻结（下次修）**：无。**验证**：零 `.rs` 变更（`git diff 0.14.1` 仅版本号 / 文档 / 工具），按「纯文档 + 版本号变更不跑编译」惯例免编译；zip 复用 `0.14.1` release exe（tag 后游戏代码零 diff），一键启动流程经 owner 实机验收通过（2026-10-06）。**关联**：[BarekHistory 0.14.2](docs/barek-history.md)、[计划 0005](docs/plans/0005-GitHub宣发计划.md)、[0006 暂缓项](docs/plans/0006-GitHub宣发暂缓项.md)。 |
| **0.14.1** | 2026-10-03 | **贝维升级**（**预发布**）：完成 **Bevy 0.15 → 0.16** 引擎换代，并一并交付全仓文档规范化大扫除。①**Bevy 0.16 强制破坏项迁移**——只迁「编译不过」项：`ChildBuilder` → `ChildSpawnerCommands`（24 处 / 18 文件）、`despawn_recursive()` → `despawn()`（5 处）、`despawn_descendants()` → `despawn_related::<Children>()`（2 处）、`AmbientLight` 新增字段 `affects_lightmapped_meshes`（1 处）；`get_single*` 弃用告警按纪律保留不动。②**文档扫除**——`module.md` 统一迁至 **B 规范 `docs/module/<Crate>.<module>.md`**（旧 `ServerCode/net/module.md` → `docs/module/Server.net.md`），**新增 14 份模块文档**（Host 7 + Server 5 + Contract 1 + Server.net 迁移）；全仓旧路径引用（README/CONTRIBUTING/web.yaml/plan 0001/ADR 0002·0004·0008/barek-history）一并改指新路径；`module-boundaries.md` 第七节改写为 B 规范模板 + 已完成清单；`CONTRIBUTING.md` 文档表 / 过渡纪律同步（`module.md` 数量 0 → 14）。**代码更改**：`HostCode/Cargo.toml`（`bevy` `0.15` → `0.16`）+ 18 份 `HostCode` `.rs`（0.16 强制破坏项迁移）。**扁平化更新**：无（本轮为 API 层迁移，未改文件结构）。**兼容性**：**不触碰线格式**，`wire_version` 仍为 `14`，双端 `0.14.1` 与 `0.14.0` **仍互通**（`z+1`）。**未做**：<br>**来自 0.12.2**：<br>①新增玩法/内容<br>②渲染内存增长复验<br><br>**来自 0.12.3**：<br>①浏览器 3D 客户端<br>②证书热重载 + 运维 API 细权限<br>③HTTPS 冒烟 + `ring` 的 `--release` 编译<br><br>**来自 0.12.4**：<br>①`get_single*` 仍弃用未迁，按纪律不动<br><br>**来自 0.14.0**：<br>①预设不接玩家档案/等级/经济（全局固定）<br>②预设内每项至多 1 件（不做数量语义）<br>③仓库 7 项物资池的手动拖拽模型不变<br>④返回主菜单时客户端背包显示被重置（服务端清单仍保留），两端不一致。<br><br>**已了结**（附凭据）：<br>①「Bevy 未升级（仍 `0.15`）」（自 0.12.4 结转）—— 本版迁至 `0.16`（凭据：`cargo check --workspace --all-targets` 退出码 `0`）<br>②「Bevy 0.16 源码迁移未实施」（本版早期声明）—— 本版实施完毕并经 owner 实机测试通过。**下一版本目标**：见〈六、已采纳未来形态〉。**本轮冻结（下次修）**：无。**验证**：`cargo check --workspace --all-targets` 退出码 `0`、0 error（本机无 `cargo-wrap`，按 0.12.4 / 0.14.0 先例经 owner 确认以裸 `cargo` 替代）；`cargo build --workspace` dev 双端 exe 产出并**经 owner 实机测试通过**（暂停菜单开关 / 连服实体增删 / 小地图动态点 / 交互面板刷新 / 光照观感 / HUD·菜单·面板·体素生成均正常；无 `error`/panic/`B0003`）。**关联**：[BarekHistory 0.14.1](docs/barek-history.md)、[计划 0002](docs/plans/0002-bevy-0.16-强制破坏项迁移.md)。 |
| **0.14.0** | 2026-10-02 | **世界目录下发**（**预发布** · **协议不兼容** `y+1`）：结清自 0.12.2 结转的两项。①**「`roster`/地图布局改下发」**——干员名册与活动地图布局（皆为整局不变的静态表）改由服务端在握手后经**新下行消息 `ServerMessage::WorldCatalog`** 一次性下发；客户端不再直读契约静态表（`operator::roster()` / `map::lawn::layout()`），改从新资源 `flow::WorldCatalog` 消费（落地 ADR 0003「服务端权威、单一事实来源」）。②**「`launcher` 资源瘦身」**——移除 `launcher/mod.rs` 的本地 `spawn_scene`（其 `insert_resource(CubeMesh/AmbientLight)` 违 ADR 0004「launcher 不得定义组件/资源」），拆为 `world::spawn_scene_baseline`（`Startup`：相机 + `CubeMesh` + `AmbientLight`，不依赖布局）与 `world::spawn_world_when_ready`（`Update`：布局就绪首帧生成光照 + 静态地图），launcher 归零资源定义。③**修「镜头避障生硬」**（表现层，**既有问题非本轮回归**）——SpringArm 贴墙避障原为「瞬时收缩 + 线性回伸」，视角扫过掩体边缘时臂长瞬跳；改为**非对称、帧率无关的指数平滑**（收缩 `25/s`、回伸 `10/s`），去掉瞬时硬钳，`WALL_MARGIN` 0.25 → 0.30 兜底防穿模。**代码更改**：`ContractCode`（`operator.rs` 名册类型 serde 化 + 标签 `&'static str`→`String` + `ROSTER` 改 `LazyLock`；`map/mod.rs` 布局类型 serde 化 + `PartialEq` + `Default`；`net/protocol.rs` 新增 `WorldCatalog`；`net/packet.rs` `WIRE_VERSION` 13→14；`net/codec.rs` 编码分支 + 往返测试）、`ServerCode`（`net/stages.rs` `Connect` 下发；`combat/range.rs` 靶标签改 `String`；`interact/mod.rs` `spawn_from_layout`；`net/web/portal.html` 版本号）、`HostCode`（`flow/state.rs` 新资源与路由、`flow/mod.rs`、`launcher`、`world/scene.rs`（`spawn_scene_baseline` + `spawn_world_when_ready`）、`world/camera.rs`（避障改指数平滑 + 单测）、`hud/{root,vitals,minimap,bigmap}` 与 `hud/interact` 消费点）。**兼容性**：**不兼容**（`y+1`）——新增下行变体，老客户端无法反序列化该 JSON 变体；`WIRE_VERSION` 13 → 14，双端必须同时升级。**迁移指南**：见 [BarekHistory 0.14.0](docs/barek-history.md) 与 [protocol.yaml](docs/contracts/protocol.yaml)（无渐进兼容路径）。**未做**：<br>**来自 0.12.2**：<br>①新增玩法/内容<br>②渲染内存增长复验<br><br>**来自 0.12.3**：<br>①浏览器 3D 客户端<br>②证书热重载 + 运维 API 细权限<br>③HTTPS 冒烟 + `ring` 的 `--release` 编译<br><br>**来自 0.12.4**：<br>①Bevy 未升级（仍 `0.15`）<br>②`get_single`/`despawn_*` 未弃用，按计划不动<br><br>**当前版本**：<br>①预设不接玩家档案/等级/经济（全局固定）<br>②预设内每项至多 1 件（不做数量语义）<br>③仓库 7 项物资池的手动拖拽模型不变<br>④返回主菜单时客户端背包显示被重置（服务端清单仍保留），两端不一致。<br><br>**已了结**（自 0.12.2 结转，附凭据）：<br>①「`roster`/地图布局改下发」—— 本版以 `ServerMessage::WorldCatalog` 下发落地（`net::codec` 往返测试通过）<br>②「`launcher` 资源瘦身」—— 本版把 `spawn_scene` 迁入 `world`，launcher 归零资源定义。**下一版本目标**：见〈六、已采纳未来形态〉。**本轮冻结（下次修）**：无。**验证**：`cargo check --workspace --all-targets` 退出码 `0`（本机无 `cargo-wrap`，按 0.12.4 先例经 owner 确认以裸 `cargo` 替代）；`cargo test -p cute_of_duty_contract` **51 passed**；`cargo test -p cute_of_duty_server` **128 passed**（与 0.13.0 基线一致，服务端零回归）；`net::codec::tests::world_catalog_becomes_control_data` 往返无损；`cargo test -p cute_of_duty_host` 新增镜头平滑两条确定性单测（帧率无关 / 不超调）。**实机验证（owner，2026-10-02）**：进对局世界渲染、HUD 干员名/技能冷却、切换台列表、小地图/全景图静态层均正常；无服务器时 Loading 超时降级不崩溃；**镜头避障平滑后的贴墙手感经 owner 复测确认正常**。**磁盘约束**：`target/` ≤ 4GB（`[profile.dev] debug=false` + `incremental=false`；守卫 `tools/target_guard.ps1`）。**关联**：[BarekHistory 0.14.0](docs/barek-history.md)、[protocol.yaml](docs/contracts/protocol.yaml)。 |
| **0.13.0** | 2026-10-02 | **物库修复**（**预发布** · **协议不兼容** `y+1`）：①**修「仓库带入的物资进不了对局」**——根因是服务端 `conn_loadout` **只存不消费**（`Connect` 写入、仅 `Disconnect` 移除，从未重播种背包；玩家实体在 `Connect` 已按写死的 `Backpack::starting()` 出生）。现 `StartTraining` 取清单并调用新增 `combat::apply_loadout` **整包重建** 4×3 背包（**完全替换**语义：有记录用记录、无记录即空背包）。②**新增选装预设**——服务端权威定义 3 套全局固定预设（`items/presets.rs`：标准 / 生存 / 爆破），连接时经**新下行消息 `ServerMessage::PresetCatalog`** 一次性下发；仓库浮层新增「预设」栏，点击即以该预设内容覆盖携带清单。预设**另开物品池**（可用仓库 7 项之外的物资）。③**新增权威物品名映射** `items::catalog::item_from_name`（物资名 → `LootItem` 唯一映射；非法名忽略、超 12 格溢出丢弃，均告警）。④**修「主菜单版本号陈旧」**——`menu/mod.rs` 硬编码 `"PRE-ALPHA v0.3.0"` 改为 `env!("CARGO_PKG_VERSION")` 派生，此后随 `Cargo.toml` 自动同步。⑤**修「仓库浮层点击无反应」**——浮层 `ArsenalRoot` 为**独立 UI 根节点**，与整屏不透明主菜单根同级时 Bevy 根节点排序不稳定、可能被完全盖住；给 `ArsenalRoot` 挂 `GlobalZIndex(30)`、拖拽幽灵挂 `GlobalZIndex(40)`。**代码更改**：`ContractCode`（`net/protocol.rs` 新增 `LoadoutPreset` + `ServerMessage::PresetCatalog`、`net/packet.rs` `WIRE_VERSION` 12→13、`net/codec.rs` 编码分支+往返测试）、`ServerCode`（新增 `items/catalog.rs`、`items/presets.rs`；`items/mod.rs`、`combat/mod.rs` 新增 `apply_loadout`、`net/stages.rs` 下发与应用、`net/web/portal.html` 绑定包版本 12→13）、`HostCode`（`flow/state.rs` 新增 `LoadoutPresets` 资源与路由分支、`launcher` 注册、`menu/arsenal/{layout,interaction,state}.rs` 预设栏）。**兼容性**：**不兼容**（`y+1`）——新增下行变体，老客户端无法反序列化该 JSON 变体；`WIRE_VERSION` 12 → 13，双端必须同时升级到 `0.13.0`。**迁移指南**：见 [BarekHistory 0.13.0](docs/barek-history.md) 与 [protocol.yaml](docs/contracts/protocol.yaml)（老端需同步升级，无渐进兼容路径）。**未做**：<br>**来自 0.12.2**：<br>①`roster`/地图布局改下发<br>②`launcher` 资源瘦身<br>③新增玩法/内容<br>④渲染内存增长复验<br>**来自 0.12.3**：<br>①浏览器 3D 客户端<br>②证书热重载 + 运维 API 细权限<br>③HTTPS 冒烟 + `ring` 的 `--release` 编译<br>**来自 0.12.4**：<br>①Bevy 未升级（仍 `0.15`）<br>②`get_single`/`despawn_*` 未弃用，按计划不动<br>**当前版本**：<br>①预设不接玩家档案/等级/经济（全局固定）<br>②预设内每项至多 1 件（不做数量语义）<br>③仓库 7 项物资池的手动拖拽模型不变<br>④返回主菜单时客户端背包显示被重置（服务端清单仍保留），两端不一致。**已了结**（自 0.12.4 结转，附凭据）：<br>①「仓库（携带物资）浮层点击无反应」—— 本版以 `GlobalZIndex(30/40)` 修复<br>②「主菜单版本号陈旧」—— 本版改为 `env!("CARGO_PKG_VERSION")` 派生<br>③「契约 YAML `version:` 未随动」—— 本版已把 `protocol.yaml` / `web.yaml` 同步为 `0.13.0`。**下一版本目标**：见〈六、已采纳未来形态〉。**本轮冻结（下次修）**：无（自 0.12.4 结转）。**扁平化更新**：把 `ServerCode/entity/mod.rs`（改动前既有 **604 行**，超「单文件 ≤ 600 行」红线）中的 `impl Entity`（**195 行**）拆到同目录 `entity/create.rs`，`mod.rs` **604 → 410 行**、`create.rs` **208 行**，二者均回到红线内（纯结构搬移，无行为变更）。**验证**：`cargo test -p cute_of_duty_contract` **50 passed**；`cargo test -p cute_of_duty_server` **128 passed**；`cargo check --workspace` 退出码 `0`；`cargo build --workspace --release` 产出双端 exe 并**经 owner 实机复测确认无大问题**（2026-10-02）。**关联**：[BarekHistory 0.13.0](docs/barek-history.md)、[protocol.yaml](docs/contracts/protocol.yaml)。 |
| **0.12.4** | 2026-10-01 | **弃用 API 清零**（**预发布**）：把 Bevy 0.15 中**已弃用但仍可编译**的 6 类 Bundle 全部迁到 required-components 新写法，构建期弃用告警 **276 → 0**，一次性清掉自 0.12.1 起挂账的「留待 0.16」项。**代码更改**：`NodeBundle`(79)/`PbrBundle`(7)/`Camera3dBundle`(1)/`DirectionalLightBundle`(2)/`PointLightBundle`(1)/`ButtonBundle`(1) 共 **91 处构造点 / 17 文件**「脱壳」为直接 spawn 组件元组（Bundle 的字段本身就是组件），丢弃 `..default()` 交由 required components 补齐；`hud/crosshair.rs::bar()` 返回类型由 `NodeBundle` 改为 `impl Bundle`（5 处调用点零改动）。**关键保全**：`Node` 的 `#[require]` 集与 `NodeBundle` 全字段同集，故 `BackgroundColor`/`BorderColor`/`BorderRadius`/`ZIndex`/`Visibility` 无需手工补也不会丢；既有显式 `Interaction::default()` 全部保留（`Node` 不含它），`Button` 处不重复补 `Interaction`（自动补齐）；`ZIndex(10/15/20)`、`Visibility::Hidden` 等数值**零改动**；`Camera3d` require `Projection`，FOV 写入路径不受影响。**兼容性**：**不触碰线格式**，`wire_version` 仍为 `12`，双端 `0.12.4` 与 `0.12.3`/`0.12.2` **仍互通**（`z+1`）。**未做**：<br>**来自 0.12.2**：<br>①`roster`/地图布局改下发<br>②`launcher` 资源瘦身<br>③新增玩法/内容<br>④渲染内存增长复验<br>**来自 0.12.3**：<br>①浏览器 3D 客户端<br>②证书热重载 + 运维 API 细权限<br>③HTTPS 冒烟 + `ring` 的 `--release` 编译<br>**当前版本**：<br>①Bevy 未升级（仍 `0.15`）<br>②`get_single`/`despawn_*` 未弃用，按计划不动<br>③契约 YAML `version:` 未随动<br>**已了结**：来自 0.12.2 的「弃用 bundle 迁移」——本版已完成（凭据：弃用告警 `276 → 0`、六类 Bundle 名零命中）。**已知问题（待修复·非"已完成"）**：①**仓库（携带物资）浮层点击无反应**——实机录屏确认悬停有高亮但点击不出浮层；已排除浮层创建问题（插桩实测 `fonts_ready=true existing_roots=1`，根节点唯一已建），`main_menu_loadout`→`arsenal_interaction` 开关链路**根因未定位，未修复**；②**主菜单版本号陈旧**——显示 `PRE-ALPHA v0.3.0`，应为 `0.12.4`，**未修复**。**下一版本目标**：见〈六、已采纳未来形态〉。**本轮冻结（下次修）**：无。**验证**：`cargo check --workspace` 弃用告警 **276 → 0**、退出码 `0`；全仓六类 Bundle 名**零命中**；最大文件仍 < 600 行（`menu/mod.rs` 580 → 500）。 |
| **0.12.3** | 2026-09-29 | **内置网页**（**预发布**）：服务端 `cod_server` 进程内**自带 Web 入口**，一次交付三件事 —— ①**服务器门户/状态页**（浏览器打开即见在线人数/版本/公告）；②**运维 API**（`/api/status`·`/api/players`·`/api/announce`·`/api/kick`，Bearer token 鉴权，token **只从环境变量读**）；③**浏览器游玩桥**（`/ws` RFC6455，浏览器按与原生客户端**完全相同的 256B/4096B 包格式**收发 + 极简 JS 骨架页）。HTTP `8080` / HTTPS `8443` 内置证书加载，游戏 TCP `8888` 不动。**代码更改**：①`net/session.rs` 抽出**传输无关核心** `net/runtime.rs`（`NetRuntime`/`NetCommand`/`ControlSession`），TCP 与 WS 共用同一套定长包解析与 `NetCommand` 分发（**零改动复用**）；②新增 `net/web/` 子域（纯 tokio **手写** HTTP/1.1 + WebSocket，SHA-1/Base64 手写不引 crate；`WebOpsPort` trait 经 `main.rs` 注入领域能力，web 对领域**零耦合**）；③新增 `ServerCode/config/web.yaml`（`include_str!` 嵌入 + 运行时覆盖）；④`ServerCode/main.rs` 676→192 行，主循环阶段抽到 `net/stages.rs`、web 装配抽到 `net/web/spawn.rs`；⑤web 模块统一 `thiserror` 的 `WebError`。**扁平化更新**：全仓「**文件名禁下划线**」公约落地 —— 45+ 个 `x_y.rs` 重构为 `x.rs` 或 `x/y.rs`（地图五文件融合进 `map/training/mod.rs`、四文件融合进 `map/lawn/mod.rs`；`menu_main.rs` 并入 `menu/mod.rs`；`net/runtime.rs` 跨模块引用改直取契约 crate）；补 [ADR 0007](docs/adr/0007-native-web-service.md) + [web.yaml](docs/contracts/web.yaml) + [Server.net.md](docs/module/Server.net.md)。**兼容性**：**不触碰线格式**，`wire_version` 仍为 `12`，双端 `0.12.3` 与 `0.12.2`/`0.12.1` **仍互通**（`z+1`）。**未做**：①（自 0.12.2 **结转**）`roster`/地图布局的「改为下发」仍是跟进项（本期以契约只读副本过渡）；②（自 0.12.2 **结转**）`launcher` 资源定义待随 ADR 0004 决策 2 继续瘦身；③（自 0.12.2 **结转**）已弃用但仍可编译的 `NodeBundle`/`PbrBundle`/`Camera3dBundle`/`DirectionalLightBundle`/`PointLightBundle` 均未迁移，留待 0.16（构建期 ~276 条弃用告警属预期）；④（自 0.12.2 **结转**）未做新增玩法/内容；⑤（自 0.12.2 **结转**·非冻结挂账）「渲染内存缓慢增长」仍待长时间复验；⑥（**本版新增·填未做**）网页版**不能像客户端一样渲染游戏**——浏览器端本期只做到「能连上、握手、收快照」的极简 JS 骨架，体素渲染/输入/资源池**全无**；完整浏览器 3D 客户端**以后再做**（另立版本）；⑦（**本版新增**）证书热重载、运维 API 细权限模型（只读/读写分权、审计日志）；⑧（**本版新增·待实机验证**）内置 HTTPS 端到端冒烟与 Windows 工具链对 `ring` 的 `--release` 编译可行性。**下一版本目标**：见〈六、已采纳未来形态〉。**本轮冻结（下次修）**：无（自 0.12.2 **结转**：0.12.2 亦为「无」）。 |
| **0.12.2** | 2026-09-29 | **契约拆分**（**预发布**）：抽出 workspace 第 3 个 crate **`ContractCode`**（`cute_of_duty_contract`），把线格式类型 / 跨域语义 / 共享常量从 `ServerCode` 物理迁出，使客户端与服务端只共享契约、**物理解耦**。**代码更改**：①`net::{protocol,packet,codec,scheduler,resource_stream}`、`map`（含 `lawn`/`training`）、`operator` **整模块迁入**契约；`element`/`items`/`interact`/`equipment`/`model` 拆分——类型进契约、逻辑留服务端，原位置留 `pub use` 垫片**公开路径不变**；②`ModelPreset::from_entity_type` 因依赖服务端 `EntityType` 改为服务端自由函数 `model::preset_for_entity_type`；③`HostCode/Cargo.toml` **删除 `cute_of_duty_server` 依赖**，改依赖 `cute_of_duty_contract`，客户端 ~60 处 `use` 全量改指契约；④同批落地 ADR 0004——`flow` 持唯一 `ModalState`，`hud`/`net` 输入门控统一读 `blocks_gameplay_input()`，`hud` 横向 `use crate::menu::` **清零**。**扁平化更新**：契约 crate 按语义单文件组织（`items.rs`/`interact.rs`/`combat.rs`/…），服务端原模块瘦身为纯 `pub use` 网关，公开路径零改动。**兼容性**：线格式 `wire_version` 仍为 `12`、类型字节完全一致，**双端 `0.12.2` 与 `0.12.1` 仍互通**（`z+1`，非协议不兼容）。**未做**：①（本期）`roster`/地图布局的「改为下发」仍是跟进项（本期以契约只读副本过渡）；②（本期）`launcher` 资源定义待随 ADR 0004 决策 2 继续瘦身；③（自 0.12.1 **结转**）已弃用但仍可编译的 `NodeBundle`/`PbrBundle`/`Camera3dBundle`/`DirectionalLightBundle`/`PointLightBundle` 均未迁移，留待 0.16，构建期 ~276 条弃用告警属预期；④（自 0.12.1 **结转**）未做新增玩法/内容；⑤（自 0.12.1 **结转**·非冻结挂账）「渲染内存缓慢增长」仍待长时间复验。**下一版本目标**：见〈六、已采纳未来形态〉。**本轮冻结（下次修）**：无（自 0.12.1 **结转**：0.12.1 亦为「无」，0.6 冻结区已于 2026-09-29 清零归档）。详见 [ADR 0003](docs/adr/0003-contract-crate.md) / [ADR 0004](docs/adr/0004-client-layer-convergence.md)。 |
| **0.12.1** | 2026-09-29 | **引擎升级**：客户端底层 Bevy `0.14 → 0.15`（**预发布**，线格式与协议**不变**，双端仍互通）。**只迁强制破坏项**，弃用但可编译的 bundle 一律不动。**代码更改**：①**文本 API 换代**——0.15 删除了 `TextBundle`/`TextStyle`，新增本地适配层 `flow::text()`（返回 `Text + TextFont + TextColor` 组合），全项目 ~120 处生成侧、~40 处更新侧平移，调用点仅改名不改参数；②**UI 类型改名**——`Style` → `Node`、`NodeBundle.style` → `node`（含 `ButtonBundle`）；③**`SpatialBundle` 被引擎移除**——改用 `Transform` + `Visibility`（引擎 `require` 自动补齐 `GlobalTransform` / 可见性链），共 3 处；④**`PbrBundle` 字段类型变化**——`mesh: Mesh3d(..)`、`material: MeshMaterial3d(..)`，共 7 处；⑤**其它 0.15 改名**——`ZIndex::Global(n)` → `ZIndex(n)`、`UiImage` → `ImageNode`、`Window.cursor` → `cursor_options`、`Time::delta_seconds/elapsed_seconds` → `delta_secs/elapsed_secs`、`Gizmos::sphere` 去掉旋转参数。**扁平化更新**：新增 `flow::text()` 单点收敛文本生成，取代散落各处的 `TextBundle::from_section`。**未做**：已弃用的 `NodeBundle`/`PbrBundle`/`Camera3dBundle`/`DirectionalLightBundle`/`PointLightBundle` 留待 0.16；276 条弃用告警属预期。**下一版本目标**：见〈六、已采纳未来形态〉。**本轮冻结（下次修）**：无；0.6 冻结区已清零——快照8 反馈 A 节 6 条 + D 节 5 条于 2026-09-29 实机全部解除冻结，legacy 操作表 14 行由 owner 于 2026-09-29 确认全部可用。 |
| **0.12.0** | 2026-09-28 | **更快传输（协议不兼容）**：线格式由恒定 64KB 槽帧改为**小定长包 + 指令优先组包 + 双通道**。①**主通道恒 256B**（32B 头 + 7×32B 单元），`SendScheduler` **指令优先**——装满 7 指令再发数据切片，指令永不被大数据饿死；②**指令二进制紧凑**——热路径 `PlayerInput` 15 个 bool 位打包进单 32B 单元，含字符串控制消息降级为 `DataKind::Control` 数据流；③**资源通道恒 4096B** 走独立第二条 TCP 连接，资源跨包分片、收侧按 `key` 重组，**客户端 16MB 定址对象池语义不变**；④**同端口双通道**——首条 256B 绑定包按 `sub_kind` 分角色（0=Control / 1=Resource），两条线程**不合并数据包**；⑤对象池淘汰经主通道 `PoolSync` 上报。**代码更改**：`ServerCode/net/packet.rs` 改小定长包、新增 `scheduler.rs`/`resource_stream.rs`/`codec.rs`、重写 `session.rs`（同端口分角色）；`HostCode/net` 重写 `network`/`uplink`/`downlink`、新增 `resource_downlink.rs`。**扁平化更新**：`HostCode/net/remote.rs` 槽位常量统一为 `SLOT_BYTES`、`ServerCode/net/mod.rs` 网关重导出新增子模块。**未做**：预取推送端到端接线、客户端 LRU 淘汰触发。详见 [ADR 0006](docs/adr/0006-small-fixed-packet-dual-channel.md)（取代 ADR 0005）。 |
| **0.11.0** | 2026-09-28 | **通信优化（协议不兼容）**：线格式由 NDJSON 行帧改为**统一固定 64KB 槽帧**；客户端新增**固定 16MB（250 在用 + 6 预取，各 64KB 固定地址）远程对象池**，资源按 `key` 落槽、实体突现即复用；**上传/下载双线单线程**（各持 `try_clone` 句柄，上传不阻塞下载）；**传输不设时钟**；新增 AOI 边缘预取算法（预测 6 个即将进入视野的实体）。**代码更改**：移除 `Combatant::ammo_pool` 中间弹池，换弹改为计时耗尽后直接从背包弹药堆抽满 —— 修「备弹诡异归零 / 要多按一次 R」（协议 `ammo_pool` → `ammo_reserve`）。**扁平化更新**：`ServerCode/net/packet.rs` 统一帧编解码、`HostCode/net/remote.rs` 定址对象池、`downlink.rs`/`uplink.rs` 双线拆分。**未做**：预取推送端到端接线。**下一版本目标**：见〈六、已采纳未来形态〉。详见 [ADR 0005](docs/adr/0005-slot-frame-transport.md)。 |
| **0.10.0** | 2026-09-28 | **模型修复**：客户端首次可见本人「焰狐」体素模型。①**服务端下发模型目录**（几何+动画随握手一次性下行，协议 `0.10.0`，仅增不改、向后兼容）；②**焰狐几何重建**——修「四肢左右镜像颠倒 / 枪悬空 1.15m / 狐耳内折」三处硬伤，35→43 盒并优化造型；③**逐盒材质键**（`VoxelCube.mat`）——此前只按骨名着色致细节全被抹平、模型退化为一坨纯色方块，现按 0.3.2 色板逐盒上色；④**朝向随视线**（修「永远向北」）+ 相机按 3.52m 体型重新标定；⑤**扁平化**——解析器 `voxel_spec.rs`→`loader.rs`、两份 JSON 合并为单文件 `FireFox.json`、`mod.rs` 网关重导出保路径稳定。**未做**：多人联机 / 匹配机制 / 无掩体竞技场。**下一版本目标**：见〈六、已采纳未来形态〉。**本轮冻结（下次修）**：无新增（该台账已于 2026-09-29 归档并删除）。 |
| **0.6（Release）** | 2026-09-27 | **单机落幕 · 正式发布**：收官补齐手雷「先瞄准后释放」持雷态、可点击操作按钮组（`B`）、legacy 操作表逐行核对；修复三条实机缺陷（手雷重力未写回致走直线、新增抛物线预览、释放光标后视角仍转）——协议 `0.9.1`。**下一版本目标**：见〈六、已采纳未来形态〉。 |
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

## 六、已采纳未来形态

> **本节是全仓唯一权威的路线图**（"已采纳的未来形态"）。自本节起，此前散落在各版 Release note 与版本表格中**重复的多目标清单**统一收敛于此；
> Release note 与 README 版本行不再逐版复制路线图全文，只保留「**未做**」「**本轮冻结**」两份欠账清单（规约见 [compliant-delivery 3.3](.agents/skills/compliant-delivery/SKILL.md)）。

### 未开工 / 未彻底决策

| # | 已采纳的未来形态 | 状态 | 详情 |
|---|---|---|---|
| 2 | **完整浏览器 3D 客户端**（网页版做到与原生端同样的渲染 / 输入 / 资源池） | 未做（自 `0.12.2` 结转） | 0.12.3 已交付"能连上"骨架，渲染未做 |
| 5 | **三角形区域光线追踪着色器套件**（像素风专属区域光照：区域阴影 / 区域 AO / 低精度反射 / 简单间接光；WGSL 单源并入 Bevy/wgpu，剔除 DX11） | 已采纳 · 计划中（目标 `0.15.0`） | [计划 0001](docs/plans/0001-区域光照着色器套件.md) · [ADR 0008](docs/adr/0008-triangle-region-radiance.md) |
| 7 | **移动端支持**：以**纯 Rust 框架**优先（winit/wgpu 移动后端路线；仅当确有硬依赖时才引入 Flutter 作壳）；目标对标**小米澎湃OS 4 首次加入的自研「专属应用运行环境」**（2026-08 发布：前台 UI 路径 100% 全机器码执行、后台按需启用） | 未彻底决策（依赖上游开放程度） | 该框架 2026-08 调研时**仅覆盖系统自带应用，未向第三方 App 开放接入文档**——开放前只做技术调研，不投入实现 |

### 进行中

| # | 已采纳的未来形态 | 状态 | 详情 |
|---|---|---|---|
| 1 | **继续小步升级 Bevy（0.16+）**，或转回 `0.6.1` 网游版本（①多玩家 → ②匹配机制 → ③无掩体竞技场） | 跟进中（自 `0.12.1`） | 见「八、开发环境说明与已知坑」底层冻结红线 |
| 3 | **跟进 OpenWRC**（对比 EA WRC 更容易上手） | 跟进中 | — |
| 4 | **借鉴明日方舟式角色模式**（目标：进入游戏即大世界场景） | 跟进中 | — |
| 6 | **自研优秀 Rust 编译器（RustJ）**：以 RustJ 参考为起点，改造成**能真正进入真实 Rust 项目**的编译器 —— **保留 Java + JVM/ZGC 原案**（一套内存管理代码全架构通吃）、**GPLv3**、单个 `RustJ.jar` 分发、统一缓存块与自适应内存预算；以**真实工程三期递进**为验收靶 | **实现中 · 二期 2b.3 已实现**（一期 hello 闭环 + 二期最小原生链路 / 算术 / 控制流；自 `0.14.1` 采纳） | **已拆出独立仓库 `Desktop/RustJ`**（源码 `Code/` + 计划 0003/0004 原件随迁其 `docs/`）· 详见〈九、RustJ 编译器〉 |

### 已完成

> 暂无 —— 条目完成后移入本表（附版本号凭据），编号保留不回收。

> 第 5 条目前只有设计文档、尚无代码；其落地属**协议不兼容**变更（`y+1`），实现时按发布五件套正式发布。
> 第 6 条一期 hello 闭环与二期最小原生链路已实现；RustJ 已**拆出独立仓库 `Desktop/RustJ`**（含源码与计划 0003/0004 原件），后续演进在新仓库进行，不阻塞游戏主线。
> 第 7 条为本次新采纳：**移动端以纯 Rust 框架为首选**（仅当 Flutter 成为不可绕过的硬依赖时才引入）。目标跟随小米澎湃OS 4 自研「专属应用运行环境」的开放节奏——其开放开发文档与第三方 App 接入前，仅做技术调研不写代码。

---

## 七、文档

- **项目规范**
  - [贡献指南（反屎山公约：600 行上限 / 无循环依赖 / 文件名禁下划线）](CONTRIBUTING.md)
  - [代码许可 GPL-3.0-with-linking-exception](LICENSE) —— 适用于全部源代码与配置
  - [资产许可 CC BY-NC-SA 4.0](LICENSE-ASSETS) —— 适用于美术 / 模型 / 音频 / 自有字体
  - [贡献者许可协议 CLA](CLA.md) —— 提交 PR 前必读并同意
- **架构与契约**
  - [模块边界总览](docs/architecture/module-boundaries.md) / [ADR 0001–0008](docs/adr/)
  - [计划 0001 · 区域光照着色器套件](docs/plans/0001-区域光照着色器套件.md) —— 已采纳未来形态第 5 条的设计文档
  - [计划 0003 / 0004 · RustJ 编译器（原件已随拆出迁至独立仓库）](docs/plans/) → `Desktop/RustJ/docs/`
  - [线格式契约 protocol.yaml](docs/contracts/protocol.yaml) / [Web 服务契约 web.yaml](docs/contracts/web.yaml) / [BarekHistory 变更台账](docs/barek-history.md)（冻结区已并入其中：改了但未验证的改动直接写入 BarekHistory 条目并标注「待实机验证」）
- **AI 协作工作流**：`.agents/skills/`（[改动前拷问](.agents/skills/plan-interrogation/) / [游戏美术创作](.agents/skills/game-art-creation/) / [地图建模验收](.agents/skills/map-acceptance/) / [合规交付](.agents/skills/compliant-delivery/)）

---

## 八、开发环境说明与已知坑

1. 项目为 **Cargo Workspace**：`cargo build` 并行编译出 `cod1.exe` 与 `cod_server.exe`；`ServerCode` **不依赖 bevy**，核心模拟秒级增量迭代。
2. 编译一律走 **`tools/cargo-wrap.exe`**：把 cargo/rustc 归入「Rust 编译器」作业以便任务管理器折叠，并把并行度钳制为 **`-j4`**（防 CPU / 进程数爆炸）。并行度可用环境变量 `CARGO_WRAP_JOBS` 覆盖。
3. **磁盘硬约束：`target/` 不得超过 4GB**。`target/` 与 `tools/cargo-wrap/target/` 均不入库。两道保障：① 根 `Cargo.toml` 的 `[profile.dev]` 设 `debug = false` + `incremental = false`（`test` 继承 `dev`），从源头压小临时产物；② 构建前先跑 **`powershell -File tools/target_guard.ps1`** —— 它体检 `target/`，超 4GB 即清掉临时产物（`debug`/`flycheck0`/`tmp`/`doc`）但**保留 `target/release`** 复用缓存，避免整树重编（`-LimitGB` 可调上限，`-WhatIf` 只报告）。若 release 复用缓存自身超限，才需人工 `cargo clean`。
4. **CI**：`.github/workflows/rust.yml` 在 push / PR 到 `main` 时自动跑 `cargo build` + `cargo test`。

### 已知问题（未解决 / 待复测）

- **视野受限（AOI 60m）**：AOI 兴趣区域半径 60m，大场内远处靶机不进快照因而不可见。场上物资（拾取物 / 功能台 / 出生点物资箱）已由服务端按 `map::lawn` 落成权威实体并经快照下发，出生点即可见可交互。
- **运行顺序**：必须先启动 `cod_server.exe` 再启动 `cod1.exe`（客户端已能自动重连，但服务端未起时不会进入训练场）。
- **渲染内存缓慢增长（已定向缓解，仍待长时间确认）**：长时间游玩（数分钟级）GPU 内存仍会缓慢累积，可能最终 OOM。已做定向修复（弹字回收失效点 + `effect_guard.rs` 硬性存活上限清道夫 + 特效密度/寿命收敛 + `net/snapshot.rs` 实体材质按 `ModelPreset` 缓存根治重复创建），**尚待长时间复验**。诊断可用 `debug_tracer.rs`（每 5s 打印特效实体存活数与 `Mesh`/`Material` 资产表容量）。
- **0.6 冻结区已清零（2026-09-29）**：原 `docs/stop-doing.md` 与 `docs/frozen-tasks/` 两份台账的全部条目已取得实机回执并确认通过，两文件已删除，**仓库不再跟踪 0.6 版本遗留问题**。曾结转的两条架构裁决已**全部落地**：ADR 0003 契约 crate（`ContractCode`，`HostCode` 已不再依赖 `ServerCode`）与 ADR 0004 `flow` 唯一 `ModalState`（`hud`/`net` 横向依赖清零）——欠账条目已从本栏与 BarekHistory 移除。详见 [ADR 0003](docs/adr/0003-contract-crate.md) / [ADR 0004](docs/adr/0004-client-layer-convergence.md)。
- **自动化试玩提示**：若用外部自动化驱动 Demo，winit 可能拦截合成鼠标事件，可用系统级 `mouse_event` 绕过。

### 已知坑（开发 / 部署实测）

- **底层冻结红线（2026-09-25 起生效，2026-09-29 修订为「小步升级」）**：仍**禁止一次性大跳**——曾把 bevy 直接升到 0.19，因大量 API 变动与稳定性问题回退到 0.14。自 `0.12.1` 起改为**逐个小版本推进**（`0.14 → 0.15 → …`）：每次只迁「不迁就编译不过」的**强制破坏项**（如 0.15 的 `TextBundle`/`TextStyle` 删除、`Style` 改名 `Node`、`SpatialBundle` 移除、`PbrBundle` 字段类型改 `Mesh3d`/`MeshMaterial3d`），已弃用但仍可编译的 bundle 留待下一小步，每步都要实机验证后再发。`ServerCode` 不依赖 bevy，升级不影响服务端分离。
- **release 构建偶发 `os error 3`（路径找不到）**：编译 bevy crate 写 `.fingerprint` 时失败，非代码错误，疑似 target 残留 + LTO / `codegen-units=1` 重负载；重试会触发整树重建，必要时先 `cargo clean`。
- **ServerCode 遗留 dead_code 告警**：`combat/shooter.rs` 的 `Vec3Helper::dot` 暂未被调用，属无碍告警，后续接入近战/命中反馈时可复用。

---

## 九、RustJ 编译器（已拆出独立仓库）

> 对应〈六、已采纳未来形态〉第 6 条。RustJ 原为随本仓库分发的独立工具（不绑定游戏版本号、不碰线格式），现已**整体拆出为独立仓库**：
>
> **仓库位置：`Desktop/RustJ`（`C:\Users\yxhcf\Desktop\RustJ`）** —— 源码 `Code/`（`ast` / `frontend` / `backend` / `error` / `examples` + `main.java`）、上游设计文档 `docs/`（原计划 0003 / 0004 原件随迁）、README 含快速开始与代码结构。
> 本仓库不再保留 RustJ 代码与计划文档；如需共享，直接克隆该仓库。

一句话链路：

```
Rust 源码 → 词法/语法 → 纯数据 AST → 自产 x86-64 机器码 → 标准 PE/COFF 目标文件 → 自研 Java 链接器 → win-x64 PE exe
```

---

> 一位开发者接手前，只需读三份：**本 README（概览）** → **CONTRIBUTING.md（公约）** → **`ServerCode/` 各模块的 `mod.rs` Why 注释**。
> 核心业务模块保持 ≤ 2 层深度、每个 `.rs` ≤ 600 行、文件名不含下划线（`x_y.rs` 必重构为 `x/y.rs` 或 `x.rs`）、禁止 `utils.rs` 之类的语义化空壳——这些是硬约束，不是建议。
