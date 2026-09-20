# Cute Of Duty 1: Simple

**版本 0.3.0 (Pre-Alpha)** · 战术撤离射击游戏

核心差异化设计：**元素互斥生态 + 反护航经济架构**。

作者：B站@3493264141322312
## 📜 License / 授权协议

本项目采用 **GPL-3.0-with-linking-exception** 协议开源。

这意味着：
- ✅ **你可以自由使用**：你可以将本项目的代码库链接到你的独立模块（包括商业闭源软件）中，而无需将你的整个项目开源。
- ✅ **修改需保留声明**：如果你直接修改了本项目的源代码，修改后的部分仍需保持 GPL-3.0 开源协议。
- ⚖️ **详细说明**：请查看仓库根目录下的 [LICENSE](./LICENSE) 文件以获取完整的法律文本和例外条款。


> 本项目站在开源社区的肩膀上：Bevy / Tokio / Serde / Tracing / Rand / Crossbeam / BLAKE3 等
> （完整清单与许可证见游戏内「设置 → 开源代码鸣谢」面板）。

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

| 按键 | 功能 |
|------|------|
| 鼠标 | 自由视角（X 轴偏航 / Y 轴俯仰，灵敏度独立，俯仰限制仰 50° / 俯 70°） |
| WASD / Space / Shift | 移动 / 跳跃 / 疾跑（移动始终相对相机方向） |
| 1 / 2 | 切换两把主武器 |
| Q / E | 干员技能（每名干员机制不同：点燃 DoT / 冰冻 / 位移冲刺 / 毒素领域） |
| 鼠标右键（按住） | 越肩第三人称瞄准：SpringArm 从右肩后方 6.5m 过渡到 2.4m（0.22s），FOV 收窄 28%，准星变琥珀色，移速降至 55%，角色平滑转向相机朝向并举枪至肩 |
| 鼠标左键 | 射击（相机射线判定，靶心核心弱点 ×1.8）；持握手雷时改为投掷 |
| F | 呼出统一交互菜单（附近功能台 + 拾取物，站点条目优先）；滚轮选择条目，F 确认：拾取直接生效（新武器替换手中武器），功能台打开面板 |
| Tab | 打开背包（双主武器、弹药池、补给品） |
| R（悬停背包物品） | 使用物品 |
| 3 / 4 | 快捷使用恢复品 / 战术品（按住打开轮盘；点击轮盘中心撤销使用） |
| Esc | 关闭背包 / 关闭功能台面板 / 取消持握的手雷（放回背包）/ 无 UI 时释放鼠标 |
| / 或 ~ | 暂停菜单（返回游戏 / 游戏设置 / 返回主界面） |

> **越肩瞄准（SpringArm 相机架构，参考原神弓手瞄准模式）**
> - 相机层级：脚底 Pivot（TopLevel，不随模型旋转）→ ShoulderPivot（Yaw）→ PitchPivot（Pitch）→ SpringArm（右肩偏移 + 后方距离）→ Camera；
> - 默认机位：右肩 +0.55 / 眼高 ≈2.95 / 后方 6.5；瞄准机位：右肩 +1.0 / 后方 2.4，过渡 0.22s smoothstep；
> - SpringArm 碰撞避障（原神方案）：撞墙缩回（贴墙最小 0.7m）、离墙缓伸，地面高度钳制 ≥0.35m；角色与相机之间不阻挡射击射线；
> - 射击判定从相机视线出发（与准星一致），曳光从枪口收敛到命中点；命中靶板中心红心判定弱点，伤害 ×1.8（无弹道下坠，为即时射线；手雷保持 12 m/s² 重力抛物线）；
> - 无蓄力机制（步枪保持连发手感）；不支持左右肩切换（固定右肩）；瞄准中跳跃保持瞄准；
> - 手雷必须"先瞄准后释放"：任意途径使用后进入持握并强制越肩瞄准，左键投出 / Esc 取消放回背包。

---

## 二、架构总览（接手前必读）

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

## 三、开发环境说明与已知坑

1. 首次 `cargo run --features demo` 需要 20+ 分钟（Bevy/wgpu 全量编译），请耐心等待；
   不带 feature 的命令不编译 bevy，秒级完成。建议保持 `Cargo.lock` 以获得与开发一致的依赖版本。
2. dev profile 已按 Bevy 官方建议把依赖设为 O3（否则试玩帧率明显下降），游戏代码本身保持 O1
   以加快增量编译——不要改 `[profile.dev.package."*"]`。
3. **CI**：`.github/workflows/rust.yml` 在 push / PR 到 `main` 时自动跑 `cargo build` + `cargo test`
   （不带 demo feature，验证核心库；提交前本地跑一遍同样命令可提前发现问题）。

### 已知问题（试玩实测）

- ~~手雷爆炸内存飙升 / OOM~~：2026-09-05 已修复（爆炸/枪口特效网格与材质入池共享，不再逐发新建资产）。
- ~~术能锁定后相机冻结~~：实为玩家初始 yaw 朝向问题（背对靶场），已修复（默认面向靶场出生）。
- **渲染内存缓慢增长（未修）**：长时间游玩（数分钟级）GPU 内存仍会缓慢累积，最终可能 OOM；
  自动化/长时间试玩建议分段进行。
- 自动化试玩提示：若用外部自动化驱动本 Demo，winit 可能拦截合成鼠标事件，可用系统级 `mouse_event` 绕过。

---

## 四、版本历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 0.3.0 | 2026-09-20 | **反屎山扁平化重构**：`config/` 并入 `src/config/`（YAML `include_str!` 嵌入 + 运行时覆盖，单一事实来源）；全部 >500 行上帝文件拆成语义化子模块（`damage`→packet/resolver/effect，`map/training`→分区分文件，`demo` 的 `inventory`/`menu`/`hud`/`combat`→面板目录，`model`→operator_models 等）；`common.rs`→`frontend.rs`；新增 [CONTRIBUTING.md（反屎山公约）](CONTRIBUTING.md) 与目录结构表格 |
| 未发布（main） | 2026-09-11 | GitHub Actions CI 接入；`src/map/training.rs` 扁平化重构并修复 CI 构建错误；设置面板新增「开源代码鸣谢」；`src/demo` 由 7300 行单文件拆分为 15 个功能子模块 |
| 0.2.3 | 2026-09-10 | 首个真开源版本：demo3d 独立 crate 并回本包（bevy 改由 feature `"demo"` 门控）；新增 `src/model` 干员模型（Yanhu）与动作系统；仓库以 GPL-3.0-with-linking-exception 开源 |
| 0.2.1 | 2026-09-06 | 0.2 hotfix 1：主界面改版（右下角「切换模式/开始游戏」、右侧 60% 分类+模式选择面板、右上角齿轮设置浮层）；workspace 解耦（核心库零 bevy、配置表全量生效） |
| 0.1.1 | 2026-09-05 | 首个对外分享打包版：补齐交接文档（本 README）、`.gitignore` |
| 0.1.0 | — | 内部开发版：核心库 + 无头模拟 + 3D Demo 全部跑通 |

---

## 五、目录结构

```
CuteOfDutyAlpha/
├── Cargo.toml / Cargo.lock     # 包清单（核心库 + cod1 bin；bevy 为 feature 门控的可选依赖）
├── CuteOfDuty_Demo.exe         # 预编译 3D Demo，双击即玩
├── CONTRIBUTING.md             # ⚠️ 反屎山公约（贡献前必读）
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
└── target/                     # 构建产物
```
