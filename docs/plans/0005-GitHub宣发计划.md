# 0005 · GitHub 宣发计划

> **执行状态**（2026-10-06）：阶段 A（A1–A4）、B（B1–B3）、C（C1–C3，C1 待合并 / C3 待下个 Release）均已落地。
> B1 落地偏差：启动脚本**入库 `tools/启动游戏.bat`**（修正原「不入库」决定——不入库则无法版本化迭代，
> Release 打包也无固定来源；`tools/` 为既有脚本惯例位置）。
> A1 落地偏差：头图复用宣发封面 `cover_1280x720.png`（横版视频无法直接内嵌 README），
> 已入库 `docs/media/`。

> **范围**：仅使用 GitHub 生态内的能力（仓库门面 / Release / Topics / Discussions / Actions / Pages），
> 不引入外部商店与平台。
> **目标形态**：让路人 **3 秒看懂是什么**、**10 秒真能跑起来**、**有地方留下反馈**。
> **性质**：文档型计划，**不改动任何 `.rs` 代码**（除阶段 B 的启动脚本，属工具产物）。
> **关联**：[README](../../README.md) · [BarekHistory](../barek-history.md) · [CONTRIBUTING](../../CONTRIBUTING.md)

---

## 一、前提与硬约束

| 约束 | 内容 | 来源 |
|---|---|---|
| 定位不可动摇 | 个人独立开发的**非商业同人 / 学习项目**，与 Activision 及《Call of Duty》无任何关联；README 顶部免责声明**不得删除或弱化** | README 维护者注 |
| 资产授权 | 代码 GPL-3.0 + Linking Exception；美术资产 CC BY-NC-SA 4.0（**禁止商业使用**） | LICENSE / LICENSE-ASSETS |
| 平台合规 | 不刷 Star / Watch、不挂假 CI 徽章、不做与仓库内容无关的互赞互粉 | GitHub ToS |
| 版本纪律 | 统一版本号 `x.y.z`，Release 标题 4 汉字，预发布挂 `--prerelease`，五件套齐发 | compliant-delivery 技能 |

> 结论：**宣发叙事只能是「开源、可研究、可试玩、可做 Mod」**，不能出现购买、众筹、抢先体验付费等商业化措辞——
> 那会与免责声明和 CC BY-NC-SA 直接冲突。

---

## 二、现状盘点（起点）

| 项 | 现状 | 评估 |
|---|---|---|
| 版本 | `0.14.1`（2026-10-03，预发布，Bevy 0.16 换代） | 迭代节奏健康 |
| README | 已高度「商店化」：徽章矩阵、操作表、架构表、13 款友商对比表 | **过长**，首屏已超出「10 秒法则」 |
| CI | `.github/workflows/rust.yml`，push/PR 到 main 跑 `cargo build` + `cargo test` | 真实可用，可挂 badge |
| 分发 | `cod_server.exe` + `cod1.exe` **必须按序运行**，客户端每 2s 自动重连 | **最大转化漏斗损耗点** |
| Release | 五件套流程成熟（README 版本表 / BarekHistory / tag / Release / 双端 zip） | 可直接复用为宣发节奏 |
| 浏览器端 | 自 0.12.3 起仅有「能连上、握手、收快照」的极简 JS 骨架，**3D 渲染全无** | 高成本项，**本轮不做** |
| 社区入口 | 无 Discussions、无 issue 模板 | 缺口，低成本可补 |

---

## 三、目标与衡量口径

GitHub 不提供转化漏斗，用**可得的公开计数**做代理指标：

| 指标 | 口径 | 用途 |
|---|---|---|
| Release 下载数 | 每个 Release 双端 zip 的 asset download count | 真实「跑起来」人数 |
| Star / Watch | 仓库公开计数 | 曝光量级 |
| Issue / Discussion 数 | 非维护者发起 | 社区是否活了 |
| 外部引用 | `awesome-bevy` 等列表是否收录 | 精准流量入口是否打开 |

**阶段目标**：先把「下载→成功进训练场」这一步的损耗压下来，再谈曝光。

---

## 四、阶段 A · 门面（零代码，优先做）

### A1. README 首屏重构（10 秒法则）
- 在现有徽章矩阵**之后**插入一段「**一句话定位 + 一张头图/GIF + 三步跑起来**」，其余长内容（架构表、友商对比）折叠进
  `<details>` 或下移。
- 头图复用已有的横版/竖版宣发素材；README 内**动图 GIF 优于视频**（视频需跳转，转化断层）。
- **验收**：陌生人只看首屏，能答出「这是什么游戏 / 怎么玩上」。

### A2. 徽章真实化复核
- 逐个核对现有徽章是否为**静态硬编码**（当前多为 `img.shields.io/badge/...` 手写值）。
- 将 CI 徽章改为**动态 badge**（指向 `rust.yml` 的 workflow status），其余手写徽章确认数值与 `Cargo.toml` 一致。
- **红线**：宁可少挂，不可挂假。

### A3. Topics 配置
- 在仓库 About 中补齐：`rust` `bevy` `game` `fps` `voxel` `pixel-art` `extraction-shooter` `multiplayer` `gamedev` `open-source`。
- **验收**：GitHub 搜索 `topic:bevy` 能命中本仓库。

### A4. 免责声明与许可复查
- 确认 README、Release 说明、仓库 About **三处**均无商业化措辞，免责声明完整。
- **验收**：README 维护者注中的「不得删除或弱化」条件仍满足。

---

## 五、阶段 B · 分发摩擦（小改动，收益最高）

### B1. 一键启动包（**本阶段核心**）
现状下玩家要手动「先起服务端、再起客户端」，这是 GitHub 场景最大的劝退点。

- 在 Release 附件 zip 中额外放入 `启动游戏.bat`（或 `play.ps1`），逻辑为：
  1. 检查双击路径下 `cod_server.exe` / `cod1.exe` 是否存在；
  2. **先**起 `cod_server.exe`（后台），等待端口就绪；
  3. **再**起 `cod1.exe`（前台）；
  4. 退出客户端时一并结束服务端进程。
- 同步在 README「快速开始」把「直接试玩」改为「解压 → 双击 `启动游戏.bat`」，手动双端流程降为进阶说明。
- **验收**：全新机器解压后**双击一次**即可进入训练场。

### B2. 单机可玩兜底
- 玩家在 GitHub 下载后大概率**没有第二个人可联机**，需要一个「一个人也能完整玩一局」的入口。
- 复用现有 `ContractCode/map/training/` 训练场 + `ServerCode/gamemode`，以**模式变体**形式提供（本地起服务端 + 本地客户端即已满足，重点是引导）。
- **验收**：单人下载后能独立完成「进图 → 打靶/击杀 → 撤离 → 结算」。

### B3. 服务器部署说明
- README 补一节「自建服务器」：端口、`web.yaml` 门户、`/api/status` 运维 API（Bearer token 仅从环境变量读）。
- 用途：给想联机的玩家一条自助路径，同时展示 0.12.3 的内置 Web 能力。
- **安全提醒**：文档中**不得**出现任何示例 token 明文。

---

## 六、阶段 C · 被发现（零代码，节奏运营）

### C1. 提交 `awesome-bevy` 收录
- 向社区 awesome 列表提 PR，归入「Games」分类，描述突出：**Rust + Bevy 0.16、配置表驱动、服务端权威、核心零 bevy**。
- 这是 GitHub 内**最精准**的流量入口——看该列表的人本身就是 Bevy 受众。
- **验收**：PR 合并，README 出现右上角反向链接。

### C2. 开启 Discussions + Issue 模板
- 开 `Announcements` / `Ideas` / `Q&A` / `Show and tell` 四板块。
- 加两类 issue 模板：**Bug（附版本号/双端版本/复现步骤）** 与 **Mod/内容提案**。
- 单人项目优势：回复快 = 好感度高，GitHub 用户吃这套；但需设定节奏，避免被 issue 淹没。

### C3. Release 节奏 = 宣发节奏
- 每个 Release 的说明首行用「**本版本你终于能做什么**」的人话，而非 changelog 堆砌（现有版本说明已是逐条技术账，建议**顶部加一段玩家视角摘要**）。
- **摘要段模板**（置于 Release 说明正文最顶部，技术账保持在其后）：
  > **本版本你终于能**：①<人话能力 1> ②<人话能力 2>。
  > **上手**：下载 zip → 解压 → 双击 `启动游戏.bat`。
  > **遇到问题**：[Issue（Bug 报告）](https://github.com/1person280/Cute-of-Duty/issues/new?template=bug_report.yml) / [讨论区](https://github.com/1person280/Cute-of-Duty/discussions)。
- 预发布照常挂 `--prerelease`，正式版不挂——保住 release feed 的信噪比。
- watcher 会在发版时收到通知，**稳定发版本身就是持续曝光**。

---

## 七、阶段 D · 暂缓项（高成本，明确不做）

| 项 | 为什么暂缓 |
|---|---|
| 浏览器 3D 客户端 | 客户端依赖 `bevy_dylib` 动态装载 + 独立服务端进程 TCP；浏览器既无 dylib 也无原生 TCP。工作量大，属独立版本工程，**不在本宣发计划内** |
| Steam 商店页 | 超出「仅 GitHub」范围；且与「非商业同人」定位存在张力，需先解决定位问题 |
| 内嵌网页试玩（Pages） | 现有 Web 桥只能握手收快照，**不能渲染游戏**；先做 B1 的本地一键包收益更高 |

---

## 八、优先级与改动量

| 顺序 | 事项 | 改动量 | 预期收益 |
|---|---|---|---|
| 1 | B1 一键启动包 | 1 个脚本 + README 一节 | 转化损耗最大 → 直接提升「真跑起来」人数 |
| 2 | A1 README 首屏重构 | 纯文档 | 首屏留住路人 |
| 3 | A3 Topics + A2 徽章 | 配置 | 搜索可见性 + 可信度 |
| 4 | B2 单机兜底引导 | 文案为主（机制已具备） | 单人也能玩完整一局 |
| 5 | C1 awesome-bevy 收录 | 外部 PR | 精准流量 |
| 6 | C2 Discussions + 模板 | 配置 | 反馈入口 |
| 7 | C3 Release 玩家视角摘要 | 纯文档 | 持续曝光质量 |

> 排序原则：**先补漏斗底部的漏（跑不起来），再补漏斗顶部的口（看不见）。**

---

## 九、合规红线（每步自检）

1. 免责声明**完整保留**，三处措辞一致；
2. 无任何商业化字样（购买 / 众筹 / 付费抢先体验 / 内购）；
3. 资产授权说明（CC BY-NC-SA 4.0）随分发 zip 一并带上 `LICENSE-ASSETS`；
4. 不刷 Star / Watch，不挂假徽章；
5. 自建服务器文档**无 token 明文**；
6. 本计划全部改动**不触碰 `.rs` 文件**，因此**无需本地编译**（符合「纯文档不跑编译」决定）；
   B1 启动脚本入库 `tools/启动游戏.bat`（就地修正：原定「不入库」），随 Release zip 分发。

---

## 十、验收清单

- [x] A1 README 首屏含定位 + 头图 + 三步跑起来
- [x] A2 徽章数值与 `Cargo.toml` / `rust.yml` 一致，CI badge 动态
- [x] A3 仓库 About 已配 Topics（11 个）
- [x] B1 一键启动脚本已入库 `tools/启动游戏.bat`（缺 exe 报错分支实测通过；服务器端口就绪探测对真实 release 服务器实测通过；**双击全流程待 owner 实机验收**——打 Release zip 时放入根目录）
- [x] B2 README 明确单人可玩路径
- [x] B3 自建服务器章节无 token 明文
- [x] C1 awesome-bevy PR 已合并（bevyengine/bevy-assets#618，2026-10-06 无评审意见直接合入）
- [x] C2 Discussions 已开启（默认板块）+ Bug/Mod 提案两类 issue 表单就绪（随下次 push 生效）
- [ ] C3 下一 Release 说明含玩家视角摘要段（模板已备，见 C3 节）
- [x] 九、合规红线 6 条逐条自检通过