# 计划 0003 · RustJ 编译器

> **状态：已采纳 · 一期原型已落地（二期起仍为设计）** —— 一期 hello 闭环原型见 [`RustJCode/main.java`](../../RustJCode/main.java)，**为临时代码**，勿依赖。
> **归属版本：不绑定游戏版本号** —— RustJ 是随仓库分发的**独立工具**（Java 产物），不触碰线格式 → **不触发 `y+1`**；
> 若未来并入某期发布的「五件套」，届时随当期版本号正式发布。
> **归属**：仓库根 `./RustJ.jar`（单文件分发）+ 源码目录 `./RustJCode/` + 编译工作目录 `./RustJ/`；**不进入** `ServerCode` / `HostCode` 双 crate 的依赖关系。
> **范式去向**：README〈六、已采纳未来形态〉第 6 条。

---

## 零、一句话定义

以 **RustJ 参考为起点**，做一个**能真正进入真实 Rust 项目**的编译器工具 —— **保留其 Java + JVM/ZGC 的原案实现**
（跨架构靠 JVM 一套内存管理代码），以**单个 `RustJ.jar`** 分发、一行命令启动，
用「统一缓存块 + 自适应内存预算 + 内置 sysroot + 单文件分发」把编译体验做到傻瓜化，
并把**真实工程**（终靶为本仓 `ServerCode` 纯逻辑部分）作为唯一验收标准。

---

## 一、设计定位与取舍

| 决策 | 结论 | 理由 |
|---|---|---|
| 实现语言 | **保留 Java + JVM/ZGC 原案**，不重写为原生 Rust | Java 兼容性好，**一套内存管理代码可全架构运行**；ZGC 自动回收编译期短命对象，省去手写缓存失效/释放逻辑 |
| 许可证 | **GPLv3**（与游戏一致） | 与 Cute of Duty 同许可、同目录层，复用现有 `LICENSE`；不引入第二套许可治理成本 |
| 分发形态 | **单个 `RustJ.jar`** | 「一个 jar + 一条命令」——对齐原案的傻瓜化分发体验 |
| 落位 | 工具 `./RustJ.jar`（仓库根）；源码 `./RustJCode/`；产物 → `./RustJ/out/` | 仓库根单文件分发；Java 源码目录 `./RustJCode/`（扁平、**禁 `src/` 套娃嵌套**），编译工作目录 `./RustJ/`（原案 `javac/` 的更名），二者**分名不冲突** |
| 运行方式 | `java -Xms1G -Xmx4G -RJT=16 RustJ.jar` | `-Xms/-Xmx` 控堆、`-RJT` 控**线程数**、`-RJCC` 控**缓存块**、默认以当前目录为工程根 |
| 接入方式 | **独立 CLI**（不侵入用户 `cargo`/`rustc` 环境） | 落地最快、风险最低；`RUSTC_WRAPPER` 适配层列为远期可选，本期不承诺 |
| 验收靶子 | **真实工程三期递进**（见第四节） | 避免沦为「只能编译玩具语言」的空壳 |

**核心取舍**：RustJ 的「快」本质是**用 JVM 的运行时复杂度换编译器实现的简洁度**（内存自动管理 vs rustc 的显式控制）。
本计划接受这一取舍 —— 换来的是跨架构一致性与分发简单，代价是 JVM 启动开销与对 Java 生态的依赖。

---

## 二、分发与目录形态

### 2.1 单文件启动

```powershell
java -Xms1G -Xmx4G -RJT=16 -RJCC="16M" RustJ.jar
```

| 参数 | 含义 |
|---|---|
| `-Xms1G` | JVM 初始堆 1GB（低配机器可调小，如 `512m`） |
| `-Xmx4G` | JVM 最大堆 4GB（编译期内存预算的经济型 knob） |
| `-RJT=16` | RustJ 自定义参数：**并行线程数**（本处 = 16） |
| `-RJCC="<档位>"` | RustJ Cache Chunky：**单文件缓存块**，仅接受三档 —— `"64K"` / `"16M"`（默认）/ `"4G"` |
| 默认工程根 | 当前工作目录（不强制 `--project`） |

### 2.2 编译工作目录 `./RustJ/`

```
RustJ/
├── out/            # 最终产物：bin / lib / cdylib / staticlib
├── deps/           # 依赖 crate 的 .rlib / .rmeta
├── sysroot/        # 首次运行从 jar 解压：core / alloc / std
├── linker/         # rust-lld 等平台链接器（按平台分目录）
├── incremental/    # 增量编译缓存（working / finalized 会话隔离）
└── cache/          # 缓存块、元数据、指纹
```

- **sysroot 自包含**：`core/alloc/std` 的 `.rlib` + 元数据随 jar 打包，首次运行解压，此后纯本地编译。
- **rlib 即 ar 归档**：需能读写归档内的 `.o` / `.rmeta` / 符号表 / 元数据。
- **会话隔离**：增量目录用 working/finalized 会话，避免并发编译损坏缓存。
- **链接器按平台分离**：`linker/win-x64/rust-lld.exe` 等。

### 2.3 源码目录 `./RustJCode/`

Java 源码（编译器主体）置于 `./RustJCode/`，**扁平结构、禁 `src/` 等 Java 老传统套娃嵌套**
（对齐本项目「语义化文件名 + 最多 2 级深度」的洁癖红线）。
与编译工作目录 `./RustJ/` **分名**，避免「输出目录 = 源码目录」的重名混淆。

### 2.4 运行流程

1. 启动自检 `RustJ/sysroot/`、`RustJ/linker/` 是否存在；
2. 缺失则从 jar 资源解压（首次多花几秒）；
3. 解析工程 `Cargo.toml`；
4. 从 `RustJ/deps/` 或远程缓存加载 `.rlib` / `.rmeta`；
5. 编译前端：词法 → 语法 → HIR/MIR → 借用检查 → 宏展开；
6. 代码生成中间产物；
7. 调用内置 `rust-lld` 链接，配合 sysroot 产出可执行文件 / 库；
8. 产物写入 `RustJ/out/`，增量缓存写入 `RustJ/incremental/`。

---

## 三、可移植策略（从 RustJ 搬运的重点）

| 策略 | 做法 | 相对 rustc 的差异 |
|---|---|---|
| **统一缓存块** | `-RJCC` 三档（`"64K"` / `"16M"` / `"4G"`）控**单文件缓存块**；不做任意 GB 硬切 | rustc 的磁盘增量是复杂指纹链 + 会话目录 |
| **自适应内存预算** | `-Xms/-Xmx` 显式控制；远期=`RUSTC_CACHE_LIMIT` 式环境变量 + 按可用内存/项目规模自动决定 | rustc 需手写失效与释放 |
| **sysroot 内置分发** | 预编译 sysroot 打进 jar，首次解压到缓存目录 | 官方靠 rustup 组件分发 |
| **增量会话隔离** | working/finalized 会话目录，防并发损坏 | 借鉴 rustc 更稳的隔离思路 |
| **并行调度** | `-RJT` 控**线程数** + 更细粒度任务拆分 + 自适应并行度 | 参考 rustc 并行前端 |
| **单文件分发** | 一个 `RustJ.jar` + 系统 JVM | rustup + sysroot + cargo 多件套 |

> **缓存失效拆两层**：① 逻辑失效（依赖指纹 / SVH 判断条目能否用）② 物理回收（交给 ZGC）。
> 磁盘层面仍要靠缓存键 / 依赖指纹 / 会话目录 —— ZGC 管内存，不管磁盘。

---

## 四、分期路线（验收靶子逐期逼近真实工程）

| 期 | 靶子 | 交付判据 | 覆盖 Rust 子集 |
|---|---|---|---|
| **一期** | `hello world` 单文件闭环 | `RustJ.jar` 能编出可运行的单文件程序（**已实现临时原型**） | 字面量 / 变量 / 函数 / 基本表达式 / `println!` |
| **二期** | **无宏、无外部 crate 的小 crate** | 能编译含多模块 / 结构体 / `trait` / 基础泛型的单 crate 工程 | 模块系统 / 结构体 / trait / 泛型 / 借用检查基础 |
| **三期（终靶）** | **本仓 `ServerCode` 纯逻辑部分** | 用 RustJ 编译 `ServerCode` 中**无 bevy、无 `proc_macro`** 的逻辑部分并通过其确定性测试 | 前述 + 真实工程规模的模块与类型系统 |

**分期纪律**：每期以「能编出**可运行且行为正确**的真实产物」为准，**不用「能解析」冒充「能编译」**；
未达终靶前，本文不标「已发布」。一期已落地**临时原型**（Rust→Java→外部 `javac`，产物为 JVM `.class`，**非〈2.4〉原生 exe**），二期起须把后端重做为真实 codegen。

### 参照的原案优先级（P0–P3）

| 优先级 | 内容 | 对应本计划分期 |
|---|---|---|
| P0 | 核心编译器（词法/语法/类型检查/代码生成）+ sysroot 打包 | 一期 |
| P1 | 链接器集成（rust-lld）+ 增量缓存 | 二期 |
| P2 | `proc_macro` 支持 + Cargo 兼容 | 三期及之后 |
| P3 | 跨平台分发（Linux / Windows / macOS） | 远期 |

---

## 五、明确不做

- **一期仅交付临时原型**（[`RustJCode/main.java`](../../RustJCode/main.java)）：只跑通 hello 子集闭环，**产物为 JVM `.class`**（**非**〈2.4〉原生 exe）；**不做** rust-lld / sysroot / 对象文件 / 增量缓存 / `-RJT`·`-RJCC` 实际生效（留待二期）。
- **不改为 MIT/Apache 双许可**（与 README 早期措辞相反，本计划定为 **GPLv3**）。
- **不做原生 Rust 重写**（定位即为保留 Java 实现）。
- **不侵入 `cargo`/`rustc` 环境**：不实现 `RUSTC_WRAPPER`、不替换 PATH 中的 `rustc`（远期可选，本期不列）。
- **不承诺 release 构建**：优化成熟的发布构建仍归 rustc/LLVM，本工具定位为 **dev 构建 / 本地迭代加速**。
- **不触碰线格式 / 契约 YAML / 服务端 / net 模块**，不升游戏版本号、不打 tag、不发 Release。

---

## 六、验证

- **一期原型验证命令**（本机 Java 25；仅编译 Java 原型，**无需 cargo-wrap**、不改三个游戏 crate）：
  ```powershell
  javac -d build RustJCode/main.java          # 编译原型
  jar cfe RustJ.jar main -C build .           # 打包根 ./RustJ.jar
  java -jar RustJ.jar hello.rs                # 编译 hello.rs -> ./RustJ/out/Hello.class
  java -cp RustJ/out Hello                    # 运行产物
  ```
  > 若 PATH 中无 `jar`（本机即如此：`javac` 走 Oracle `javapath` 而 `jar` 不在其中），改用 `"<JDK>\bin\jar"`，如 `"C:\Program Files\Java\jdk-25.0.4\bin\jar"`。
- 文档落地时的自查：README〈六〉第 6 条链接与〈七〉索引指向本文件的**链接可达性**。
- 各期实现阶段的验证命令（届时另开实现计划）：
  - 产物可运行性：`RustJ/out/` 下产物能启动并输出预期结果；
  - 终靶确定性：以 `ServerCode` 现有测试为准，行为须与 `cargo test` 一致。

---

## 七、关联

- [README〈六、已采纳未来形态〉第 6 条](../../README.md)（本计划对应的路线图条目）
- 参考材料：《RustJ 技术对话记录与开发参考》（外部设计参考，非本仓文件）
- [计划 0001 · 三角形区域光线追踪着色器套件](0001-区域光照着色器套件.md) / [计划 0002 · Bevy 0.16 强制破坏项迁移](0002-bevy-0.16-强制破坏项迁移.md)