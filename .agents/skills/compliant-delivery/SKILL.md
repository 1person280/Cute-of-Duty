---
name: compliant-delivery
description: Cute Of Duty 合规交付技能——把改动按「三闸门」合规地走完 提交(commit) → 推送(push) → 发布(release)：提交前过 CLA 声明 + 洁癖/架构/协议/文档/测试五道红线；推送遵循分支与"不重写历史"纪律；发布采用**统一版本号 `x.y.z`**——协议版本与游戏版本不再分离（`x` 游戏内核 · `y` 协议 · `z` 细节），Release 标题格式 `x.y.z：<4字简述>`（**版本信息必须 4 个汉字**），并同步 README 版本表 + BarekHistory + tag + GitHub Release + 双端 exe 二进制 zip 五件套。凡用户要求提交、commit、推送、push、开 PR、打 tag、发版本、写 release note、打包 exe / 发分发包 / 补发 release 资产，或问"这个改动能不能提交/合规吗"时使用——即使用户只说"帮我提交""推一下""发个版本""把这个 release 做了""把 exe 打包装上"。
---

# Cute Of Duty 合规交付（提交 / 推送 / 发布）

本项目合规 + 洁癖要求极硬（见 [CONTRIBUTING.md](../../../CONTRIBUTING.md)）。本技能把「提交 → 推送 → 发布」拆成三道互相独立、**逐关放行**的闸门。**上一关未过，禁止进入下一关**；任何"先推了再说"的行为都视为违规。

一句话判据：**归属合法（CLA/署名）→ 红线全过（五道）→ 号对齐（统一版本号 `x.y.z`：`x` 内核 · `y` 协议 · `z` 细节，三段各按判据 +1）。**

---

## 零、三张通行证（每关开闸前先自问）

| 通行证 | 问什么 | 在哪一步用 | 缺了会怎样 |
|---|---|---|---|
| ① 授权 | 贡献者已签 CLA？作者身份是登记身份？ | 提交前 | PR 不予合入；署名丢归属 |
| ② 洁净 | 五道红线（洁癖/架构/协议/文档/测试）全过？ | 提交前 | 直接关 PR，不讨论 |
| ③ 对齐 | 统一版本号 `x.y.z` 三段判据正确？tag/Release/README/BarekHistory/Cargo 一致？ | 发布前 | 版本漂移，无法追责与兼容 |

---

## 第一关：合规提交（commit）

### 1.1 提交前五道红线（缺一不可）

| 红线 | 判据 | 检查方式 |
|---|---|---|
| **洁癖** | 单文件 ≤ 600 行；禁循环依赖；禁 `common.rs`/`utils.rs`/`misc.rs`/`helpers.rs`；嵌套 ≤ 2 层且**文件名禁下划线**（`x_y.rs` 必重构为 `x/y.rs` 或 `x.rs`，单开子域须证明有意义）；公开项有 **Why** Rustdoc（不是复述代码） | 逐文件核对 + `git diff --stat` |
| **架构** | 禁跨模块直接调用（三层判据：①`use crate::<别模块>::` ②`HostCode/Cargo.toml` 不得含 `cute_of_duty_server` ③同层横向 `use crate::menu::`）；数据所有权唯一（他人只持快照/句柄）；依赖无环、只许 `表现层→契约层→领域层→基础设施层`；跨 crate/跨进程先写 `XxxPort` trait | 搜引用 + `grep`；新增依赖必须贴依赖图 |
| **协议** | 碰 L2+ / 线格式 / 配置语义 / 公共 Trait → 必须同 PR 追加 [docs/barek-history.md](../../../docs/barek-history.md) 条目；`y+1`（协议不兼容）必附迁移指南；契约同步 `docs/contracts/*.yaml` | 对照 [模块边界](../../../docs/architecture/module-boundaries.md) 的 L2 集合 |
| **文档** | 碰某模块 → 同 PR 补该模块 `module.md`（现状 0，"碰到就补"）；改代码同更 BarekHistory / 契约 YAML；破坏性 L1 变更写 ADR | 见 CONTRIBUTING 第九节 |
| **测试** | 任何改动 `cargo-wrap check --workspace`；服务端逻辑 `cargo-wrap test -p cute_of_duty_server`；线格式/契约 另加契约一致性；客户端表现 `cargo-wrap build --workspace --release` | 编译**一律走 `tools/cargo-wrap/target/release/cargo-wrap.exe`**，禁裸 `cargo` |

Rust 硬规范：生产路径禁 `unwrap()`/`expect()`（用 `?`）；错误统一 `thiserror`（禁 `Box<dyn Error>` 穿模块边界）；内部字段 `pub(crate)`；跨 `await` 保 `Send + Sync`（禁 `Rc`/`RefCell` 跨界）；可调数值走 `src/config/`（禁硬编码副本）。

### 1.2 授权与署名（合规的资格线）

- **首次贡献者**必须在 PR 描述原样写入声明，否则不予合入：

  ```
  I have read the CLA Document and I hereby sign the CLA.
  ```

  维护者据 [docs/cla-signatures.md](../../../docs/cla-signatures.md) 登记（身份 / 邮箱 / CLA 版本 / 日期 / 覆盖提交范围）；历史贡献走"追溯补签"，联系不上或拒签的第三方须 clean-room 重写。
- **作者身份必须等于登记身份** `1person280 <1975383276@qq.com>`；提交前先核对：

  ```
  git config user.name
  git config user.email
  ```

  **AI/工具不得代填 author**：历史事故 `d857e61` 被工具误填成 `Bzhan-3493264141322312 <3493264141322312@users.noreply.github.com>`，已用根目录 [.mailmap](../../../.mailmap) 在显示层校正归属——**不重写历史、不 force-push**。

### 1.3 提交信息规范

```
<type>(<scope>): <一句话为什么>

<可选正文：为什么这么改，而不是怎么改>

关联: ADR-xxxx / Issue #xx
```

- `type` ∈ `feat` / `fix` / `refactor` / `docs` / `test` / `chore` / `perf`。
- `<scope>` 用**版本号**（如 `0.10.0`）或**模块名**（如 `net`）。仓库既有的 `feat(0.10): …`、`refactor(menu): …`、`docs(skill): …` 即范本。
- **一次提交只做一件事**；**重构与功能不得混在同一提交**。
- 正文写"为什么"（Why），不是"改了什么"（What）。

### 1.4 入库 / 不入库

入库前 `git status --short` 逐条确认。**禁止提交**（见 [.gitignore](../../../.gitignore)）：

`/target/`、`/tools/cargo-wrap/target/`、`/ServerCode/data/`（运行时档案）、`/_ref/`（1.6GB legacy 对照，不入库）、`/.trae/`、`* .zip`、`*.log`、`check_out.txt`、`smoke_out.log`、`/HostCode/check*.txt`。

### 1.5 未验证改动只能进冻结区

改了但没验证的功能，**只能**写进 [docs/barek-history.md](../../../docs/barek-history.md) 的条目（标题标注「**待实机验证**」）；**不得**在 README / CHANGELOG / release note 里被描述为"已完成"，也**不得**在其上继续叠加新代码。（原 `docs/stop-doing.md` 与 `docs/frozen-tasks/` 已于 2026-09-29 随 0.6 冻结区清零删除，冻结落点统一收敛到 BarekHistory。）

---

## 第二关：合规推送（push）

### 2.1 分支纪律

- 从 `main` 拉分支，命名语义化：`feat/xxx`、`refactor/xxx`。
- **未验证的玩法**留在 `wip/*`（如 `wip/0.8-snapshot-8`），**不合入 `main`**。

### 2.2 推送动作

```
git push -u origin <branch>          # origin = https://github.com/1person280/Cute-of-Duty.git
```

- **禁 force-push 到 `main`/`master`**；**禁重写公共历史**（署名/归属问题一律用 `.mailmap`，不用 amend / rebase / reset 修公共历史）。

### 2.3 PR 与评审门槛

- **L2 模块**（`net` 线格式 / 契约层 `ContractCode` / 公共 Trait）或**新增顶层模块 / 提取独立 crate** → **先开 Issue 对齐**。
- L2 变更须 **≥2 名 reviewer + maintainer 参与**；破坏性变更先写**迁移指南 + ADR**，再动代码。
- PR 描述必附：改了哪个模块 / 为什么 / **依赖图**（证明无环）/ **验证命令 + 结果**；手工验收写清 **步骤 + 预期 + 实际**，未实测标注"未验证"。
- 绕过 [BarekHistory](../../../docs/barek-history.md) 中标注「待实机验证」的冻结项 → 直接关闭。

### 2.4 CI

`push` / `pull_request` 到 `main` 触发 [.github/workflows/rust.yml](../../../.github/workflows/rust.yml)（装 Linux bevy 系统库 → 缓存 → `cargo build` + `cargo test`）。**推送前先本地跑通 `cargo-wrap`**，不要拿 CI 当第一次编译。

---

## 第三关：合规发布（release）

### 3.1 一套号，`x.y.z` 三段各司其职

**协议版本与游戏版本不再分离**：同一个 `x.y.z` 同时是**游戏版本**、**协议版本**、**发布号**。tag / GitHub Release / README 版本表 / `Cargo.toml`（ServerCode + HostCode）/ `docs/contracts/*.yaml` 一律写这一个号。

| 段 | 名称 | 何时 +1 | 触发条件 |
|---|---|---|---|
| **`x`** | **游戏内核版本** | **严重破坏性更新**必须 +1 | 内核 / 架构级颠覆，老客户端整体不可用（如渲染或内核重写） |
| **`y`** | **协议版本** | **不兼容**部分必须 +1 | L2 线格式 / 契约不兼容 → 必须附迁移指南 + BarekHistory 条目 |
| **`z`** | **细节版本** | **无兼容性变化**时可 +1 | 加性 / 修复 / 造型细节，向后兼容，老客户端无损 |

示例：`0.10.0` = 内核 `0` · 协议 `10` · 细节 `0`。当前协议 `0.10.0` ⇒ 发布号同样写 **`0.10.0`**。

- 改动**只增不改**、向后兼容 → `z+1`（如 `0.10.1`）。
- 改动**不兼容**现有线格式 → `y+1`，`z` 归零（如 `0.11.0`）。
- 改动**内核级破坏**、老客户端整体不可用 → `x+1`，`y` 与 `z` 归零（如 `1.0.0`）。

### 3.2 发布前门禁（全绿才可发）

- [ ] `cargo-wrap check --workspace` 退出码 **0**
- [ ] `cargo-wrap test -p cute_of_duty_server` **全绿**
- [ ] 客户端改动：`cargo-wrap build --workspace --release`（**debug 产物 >2GB 会触发 `os error 193`**）
- [ ] **冻结项审计**：[docs/barek-history.md](../../../docs/barek-history.md) 里标注「待实机验证」的条目，未了结的一律不得写成"已完成"
- [ ] **清单结转审计**：上一版的「**未做 / 下一版本目标 / 本轮冻结**」三份清单已**逐条**处理 —— 已了结者附凭据移除，未了结者结转进本版对应清单并标注来源版本；**无静默丢失**；且「未做」已**按来源版本分组、一条一行**（格式规约见 3.3）
- [ ] 协议 / L2 改动：BarekHistory 条目**已追加**（变更类型 / 兼容性 / 迁移指南 / 验证 / 关联）

### 3.3 发布五件套（必须一次性对齐）

1. **README 版本表**：[README.md](../../../README.md)「六、版本历史」**顶部**加一行 ——
   `| **x.y.z** | YYYY-MM-DD | 说明… |`
   **硬规约（版本信息 4 个字）**：版本信息（Release 标题、README 版本行开头）**必须是 4 个汉字**，简洁点明本版干了什么（如 `通信优化`、`换弹精简`）。**不得**多字/少字/用英文/写空泛口号。卡片推送、README 版本表、release note 一处不得漂移。
   **硬规约（每次 release 必带代码更改）**：说明**必须**包含**简短的代码更改** —— 一句话讲清"**做了什么**"与"**做了什么扁平化更新**"（如文件重命名 / 合并 / 结构收敛），再覆盖：**未做**、**下一版本目标（roadmap）**、**本轮冻结（下次修）**。
   **硬规约（三份清单必须结转）**：上一版本的「**未做**」「**下一版本目标（roadmap）**」「**本轮冻结（下次修）**」三份清单，**每一条都必须逐条结转**进本版对应清单，并标注来源版本「（自 `x.y.z` 结转）」——**除非本版确实已了结**：已了结者须在"代码更改/验证"中给出**可核对凭据**，方可移除。**禁止**换版时静默丢弃任一份清单致欠账隐身；**不得**用"与上版一致"这类整段省略代替逐条结转（须保留条目原文 + 来源版本）。Release note 与 README 版本行**两处同源结转**，不得只在一处写。
   **硬规约（未做清单格式）**：「未做」**按来源版本分组**，分组标题用**该条目的原始出处版本**（依上一版条目里的既有标注判定：标「自 `x.y.z` 结转」者归 `x.y.z` 组、标「本版新增」者归上一版组、本版新产生的归「当前版本」组），**因此通常会有两组或更多**；**每组下每条未做项独占一行、一句话以内**，不得把多条挤成一行长句。README 版本表单元格内以 `<br>` 换行、Release note 用独立列表项 —— **两处同构**，不得一处分组一处扁平。已了结条目单列「**已了结**」并附凭据，不混进未做项里。
2. **BarekHistory**：协议 / L2 改动时在 [docs/barek-history.md](../../../docs/barek-history.md) 顶部追加条目（最新在最上）。
3. **打 tag**：
   ```
   git tag x.y.z                       # 无 v 前缀，如 0.10.0
   git push origin x.y.z
   ```
   （早期历史线 `0.3.x` / `0.5.x` 曾用 `v0.3.2` 形式；当前一律以统一版本号 `x.y.z` 为准。）
4. **GitHub Release**：以该 tag 建 Release，**标题格式 `x.y.z：<4字简述>`**（版本信息 = 4 个汉字，如 `0.11.0：通信优化`）。**正式版本（Release）不加 `--prerelease`**；确需预发布时才加：
   ```
   gh release create x.y.z --title "x.y.z：<4字简述>" --notes-file <说明文件>
   ```
5. **二进制分发包（zip 资产）**：把 `--release` 产出的双端 exe 打包成 zip 挂到该 Release —— 见 3.4。

### 3.4 双端 exe 打包（Release 资产）

**必须先 `cargo-wrap build --workspace --release` 产出** `target/release/cod_server.exe`（服务端）与
`target/release/cod1.exe`（客户端）；**禁止**打包 `target/debug/` 产物（>2GB 且非发布版本）。

打包内容（`原样打包` 口径：不改资源解析代码）：

| 入包项 | 来源 | 说明 |
|---|---|---|
| `cod_server.exe` | `target/release/` | 服务端，先启动（默认 `127.0.0.1:8888`） |
| `cod1.exe` | `target/release/` | 客户端，后启动 |
| `menu/` | `HostCode/menu/` | 客户端资源（`icon/settings.png` 图标 + `zcool_kuaile.ttf` 字体等） |
| `LICENSE` / `LICENSE-ASSETS` / `CLA.md` | 仓库根 | **许可随版本发布**，缺一不可 |
| `使用说明.txt` | 生成 | 运行顺序 + 资源路径提示（见下） |

命名规范：`CuteOfDuty-<版本号>-win64.zip`（如 `CuteOfDuty-0.10-win64.zip`）。

步骤（PowerShell，暂存目录放 `$env:TEMP`，上传后**立即删除**以省磁盘）：

```powershell
$stage = Join-Path $env:TEMP "cod_pack"      # 暂存目录
# …Copy-Item 双端 exe + HostCode\menu + LICENSE/LICENSE-ASSETS/CLA.md + 生成 使用说明.txt…
$zip = Join-Path (Get-Location) "CuteOfDuty-<版本号>-win64.zip"
Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $zip -CompressionLevel Optimal

gh release upload <版本号> $zip --clobber    # 已发布则用 upload 补发；未发布可并入 release create
Remove-Item $stage -Recurse -Force; Remove-Item $zip -Force   # 省磁盘：GitHub 已是权威副本
```

**补发已有 Release**：release 已建好后补资产一律用 `gh release upload <版本号> <zip> --clobber`，
**不要**重建 release（重建会丢 tag 关联与历史）。上传后核对 `gh release view <版本号>` 的
`asset:` 行已出现该 zip。

**资产路径红线（重要）**：客户端 [launcher/mod.rs](../../../HostCode/launcher/mod.rs) 的
`AssetPlugin.file_path` 目前是**编译期绝对路径**（`env!("CARGO_MANIFEST_DIR")`，即 HostCode 包根），
并非相对 exe 解析。因此 `原样打包` 的 zip **只在"与构建机相同目录结构"的机器上可直接运行**；
换机器/换盘符需保持相同目录结构，或改为运行时解析后重新编译。`使用说明.txt` 必须**明写**此限制，
**不得**在 release note / README 里把该包描述成"任意机器解压即用"。

**zip 不入库**：`*.zip` 已在 [.gitignore](../../../.gitignore) 中忽略；资产只挂在 GitHub Release，
不提交进仓库。

### 3.5 发布红线

- **统一版本号 `x.y.z`**：`x`（内核）严重破坏 +1、`y`（协议）不兼容 +1、`z`（细节）无兼容变化可 +1；三段判据不得用错 —— 不兼容变更只升 `z`、或内核破坏只升 `y`，均属违规。
- **版本信息必须 4 个字**：Release 标题 / README 版本行开头的版本信息**必须是 4 个汉字**（如 `通信优化`），简洁点明本版干了什么；不得多字/少字/英文/空泛口号。
- **每次 release 必带代码更改**：release note / README 版本行必须含**简短代码更改**（做了什么 + 做了什么扁平化更新），不得只写空泛口号。
- **三份清单必须结转**：上一版「未做」「下一版本目标」「本轮冻结」三份清单，除非本版**确实已了结**（附可核对凭据），否则**必须逐条结转**进本版对应清单并标注来源版本；换版静默丢欠账（含用"与上版一致"整段省略）属违规。
- **未做清单格式**：「未做」必须**按来源版本分组**（分组标题 = 条目原始出处版本，通常 ≥2 组），**每组下一条一行、一句话以内**；README 用 `<br>` 换行、Release note 用独立列表项，两处同构。把多条未做挤成一行长句属违规。
- tag 名严格为 `x.y.z`（如 `0.10.0`），**无 v 前缀**（历史曾误写 `v0.6-SnapShot-1`，大小写错乱，**不要复现**）。
- **不得启用已冻结的历史 WIP 号**（如 `0.7-Snapshot-7` / `0.8.0-Snapshot-8` 那批未发布快照）。
- **许可随版本发布**：代码 `LICENSE`（GPLv3 + Linking Exception，覆盖 `HostCode/`·`ServerCode/`·`ContractCode/`·`tools/`）+ 资产 `LICENSE-ASSETS`（CC BY-NC-SA 4.0）+ `CLA.md` 必须齐全。
- **禁无版本分发**：任何 release 都必须对应明确 tag 与提交，客户端与服务端版本漂移须给兼容声明。

---

## 本项目已实测的踩坑（别重犯）

- **发布号/文档漂移**：仓库出现过 tag 已存在而 README 版本表顶行仍停在旧版本的情况 —— 发布五件套必须**同一次**补齐，缺一即为漂移。
- **版本段误用**：`x.y.z` 三段各有判据 —— 不兼容协议变更只升 `z`（应 `y+1`）、内核级破坏只升 `y`（应 `x+1`）、或改了版本忘了同步 README / Cargo / 契约 YAML，均属版本漂移。
- **debug 构建爆盘**：Windows debug 版 bevy 可 >2GB → `os error 193`（invalid Win32 application）→ 发布构建**必须 `--release`**。
- **AI 工具误填 author**：见 1.2 的 `Bzhan-…` 事故 —— 提交前**必核对 `git config user.*`**。
- **想靠 force-push 改署名**：**不要** —— 用 `.mailmap` 做显示层校正，不重写历史、不影响任何已 clone/fork 的仓库。
- **把未验证改动写成"已完成"**：违反冻结区规则，属直接关闭 PR 的违规项。
- **补发资产去重建 release**：release 建好后补 zip 只能用 `gh release upload <版本号> <zip> --clobber`；重建 release 会丢 tag 关联与历史。
- **PowerShell 下 `gh --jq` 引号被拆**：`gh release view x --json assets --jq '.assets[] | ...'` 在 PowerShell 里单引号内管道会被拆成多参数报 `accepts at most 1 arg(s)`；直接看 `gh release view <版本号>` 原文即可，或把 jq 表达式用双引号包裹。
- **把 `原样打包` 的 zip 说成"解压即用"**：客户端资源根目录是编译期绝对路径（见 3.4 资产路径红线），换机换盘符会缺资源 —— 说明文件必须写明限制。
- **三份清单换版丢失**：上版「未做 / 下一版本目标 / 本轮冻结」未结转进新版对应清单 → 欠账隐身（0.12.2 曾丢 0.12.1 的弃用 bundle 迁移 / 渲染内存挂账）。每次发布必须逐条核对上版三份清单：已了结则附凭据移除，未了结则结转并标注来源版本。
- **未做清单写成一行长句**：把十几条未做挤进一个扁平长句（如 0.12.4 初稿的 ①~⑩ 挤在一行）可读性极差、来源也看不出。应**按来源版本分组 + 一条一行**；分组标题取原始出处版本，别把「自 0.12.2 结转」的条目错记成 0.12.3 的（0.12.4 修正过一次）。

---

## 快速检查表（TL;DR）

- 提交前：CLA 已签？`user.name/email` 对？五道红线过了？`git status` 无该入库之外的杂物？
- 提交信息：`<type>(<scope>): 为什么`，一次一件事，重构≠功能。
- 推送：分支语义化，`main` 不 force-push、不重写历史；L2 先开 Issue + ≥2 reviewer。
- 发布：统一版本号 `x.y.z`（`x` 内核 · `y` 协议 · `z` 细节，各按判据 +1），**无 v 前缀**；**版本信息必须 4 个汉字**（Release 标题 `x.y.z：<4字简述>`，如 `0.11.0：通信优化`）；五件套（README 版本表 / BarekHistory / tag / GitHub Release / 双端 exe 二进制 zip）一次对齐；**每次 release 必带简短代码更改 + 扁平化更新**；**上版「未做 / 下一版本目标 / 本轮冻结」三份清单未了结者必须逐条结转（标注来源版本，不得静默丢失 / 不得"与上版一致"整段省略），且「未做」须按来源版本分组、一条一行**；冻结项不写"已完成"。
- 打包：只打 `--release` 双端 exe；zip 含 exe + `menu/` + 三份许可 + 使用说明；命名 `CuteOfDuty-<版本号>-win64.zip`；补发用 `gh release upload --clobber`，上传后删本地 zip；说明须写明资源绝对路径限制。
- 编译一律 `cargo-wrap`；发布构建一律 `--release`。