# 计划 0004 · RustJ 二期实现（最小原生链路）

> **状态：执行中** —— 二期 2a「最小原生链路」已完成（Rust 极小子集 → 自产 COFF → 自研 Java 链接器 → win-x64 PE exe 端到端闭环）；
> 二期 2b.1「变量绑定 + 算术表达式」、2b.2「表达式补全与比较」、2b.3「发射层重构 + 控制流」已完成并验证。
> **归属版本：不绑定游戏版本号** —— 沿用 [`0003-RustJ编译器.md`](0003-RustJ编译器.md) 的定位：RustJ 是随仓库分发的独立工具，
> 不触碰线格式 → **不触发 `y+1`**、不打 tag、不写 BarekHistory。
> **归属**：仓库根 `./RustJ.jar`（单文件分发）+ 源码目录 `./RustJCode/` + 编译工作目录 `./RustJ/`。
> **上游设计**：本计划是 0003〈四、分期路线〉中「二期」的**落地实现计划**；0003 的 linker 描述已按本计划就地修正。

---

## 零、一句话定义

把一期的「转译 Java」后端整体重写为**纯 Java 的原生后端**：`fn main() -> i32 { <整数> }`
→ 自产标准 PE/COFF 目标文件 → 自研 Java 链接器 → **win-x64 PE exe**，
用退出码断言证明「真 codegen → 真链接 → 真执行」，**全程不依赖任何非 Java 工具链**。

---

## 一、已定决策（经 `plan-interrogation` 逐条确认）

| # | 决策 | 结论 |
|---|---|---|
| 1 | 起手式 | **后端优先**：先打通最小原生链路，再横向扩语言特性 |
| 2 | codegen 机制 | **自研机器码发射**（Java 直接编码 x86-64），先只覆盖 **win-x64**；无 LLVM / clang |
| 3 | 运行时 | **零 sysroot**：自写最小 runtime stub，不碰 `core`/`std` 的 `.rlib` |
| 4 | 链接器 | **放弃 `rust-lld` 二进制**，自研 Java 链接器 `lld.java` |
| 5 | lld 输入格式 | **标准 PE/COFF 最小子集**（保住未来接 sysroot / 真实 `.o` 的路） |
| 6 | 验收 | **端到端 + 退出码断言**（`{0}`→0、`{42}`→42） |
| 7 | 依赖铁律 | 整条工具链**纯 Java、零外部工具链、尽量不链接任何非 Java 代码** |
| 8 | 退出机制 | **零 import**（PE 导入表为空，靠入口返回码作退出码；不符则回退最小 `ExitProcess` 导入） |
| 9 | 文档 | 就地修正 0003 过时条目 + 本文件 |
| 10 | 洁癖（更严） | **一 class 一文件**；**单文件 < 100 行（≤99）**；入口**仍 `main.java`**；类/文件**统一小写** |

---

## 二、源码组织与类拆分

组织范式：**`RustJCode/<宽泛目的>/<具体实现>`** —— 子目录即 Java 包，带 `package` 声明；
**一 class 一文件、单文件 < 100 行、类与文件名统一小写**；每个 class 头部注释写明「做什么」与「对外提供什么功能」。
入口 `main.java` 留在 `RustJCode/` 根、属默认包（入口位置固定不变）。

| 包（目录） | 宽泛目的 | 文件（class） |
|---|---|---|
| 根（默认包） | 入口与流水线编排 | `main` |
| `ast/` | 纯数据语法树节点 | 表达式 `expr` `intlit` `ident` `binop` `unop` `op`；语句 `stmt` `block` `letstmt` `assignstmt` `ifstmt` `whilestmt` `returnstmt`；函数 `function` |
| `frontend/` | 词法、语法、符号表 | `token` `lexer` `cursor` `parser` `stmts` `exprs` `locals` |
| `backend/` | 机器码、目标文件、链接、PE | `arch`（发射接口）`x64`（win-x64 实现）`codebuffer` `codegen` `eval` `coff` `lld` `pe` |
| `error/` | 前后端共用的编译期错误 | `rustjerror` |

**依赖方向（无环）**：`frontend → ast`、`backend → ast`；`frontend`、`backend` 各自单向依赖 `error`；`main` 依赖全部。
`ast` 为**纯数据**（不含任何机器码）；机器码发射全在 `backend`，故依赖为 `backend → ast`（2b.3 由原 `ast → backend` 反转而来），
新增硬件架构只需再实现 `arch` 接口一个类。`rustjerror` 独立成包，是为了让前后端都只单向依赖它，避免 `frontend ↔ backend` 互引。

---

## 三、里程碑与执行顺序

**子期 2a · 最小原生链路**（已完成）：

| 步 | 内容 | 产出判据 |
|---|---|---|
| 2a.1 | `rustjerror` / `token` / `lexer` | 能把源码切成 token 流 ✅ |
| 2a.2 | `function` / `parser` | 能解析单个 `fn main() -> i32` ✅ |
| 2a.3 | `x64` / `coff` | 能产出标准 COFF `.o` ✅ |
| 2a.4 | `lld` / `pe` | 能把 `.o` 链接成 win-x64 PE exe ✅ |
| 2a.5 | `main` 编排 + 端到端验证 | 退出码断言通过（`min.rs` 0 / 42）✅ |

**子期 2b · 语言特性**（进行中）：

| 步 | 内容 | 产出判据 |
|---|---|---|
| 2b.1 | 变量绑定 + 算术表达式（`let`、`+ - *`、优先级、括号、`return`/末表达式） | `arith.rs` 退出码断言 7；`min.rs` 回归 0 ✅ |
| 2b.2 | 表达式补全与比较（`/ %`、一元负号、`== != < <= > >=`） | `ops.rs` 退出码断言 7；符号语义与 Rust 一致 ✅ |
| 2b.3 | 发射层重构（AST 纯数据）+ 控制流（`if/else`、`while`、`return`、赋值） | `flow.rs` 退出码断言 55；三例回归不变 ✅ |
| 2b.4+ | 后续特性（多函数/调用 → 结构体 → 模块 → 类型系统/泛型 → trait） | 待定 |

**2b.1 已定决策**（经 `plan-interrogation` 逐条确认）：
起手特性 = 变量绑定 + 算术表达式；子集边界 = `let` + i32 字面量 + `+ - *`（含优先级/括号）+ `return`/末表达式；
codegen = 栈帧局部变量 `[rbp-4n]` + 后序栈机求值；AST = 多态类层次（一节点一 class）；
语义检查 = 符号表（未声明 / 重复声明报错，类型一律 i32）；验收 = `arith.rs` 退出码 7 + `min.rs` 回归。

**2b.2 已定决策**：运算符 AST = 一运算符一 class，收进 `ast/compute/` 子包；表达式解析从 `parser` 拆出为 `frontend/exprs`（守 < 100 行）；
比较结果 = i32 的 0/1（`cmp` + `setcc` + `movzx`），不引入独立 bool；优先级 = 比较 < `+ -` < `* / %` < 一元 `-` < atom。
`/ %` 用 `cdq` + `idiv`，符号语义与 Rust 一致（向零截断、余数随被除数）。

**2b.3 已定决策**：发射从 AST 移出交后端（`arch` 接口 + `x64` 实现 + `codegen`/`eval` 走树），AST 变纯数据，
运算符由 12 个节点类收敛为 `binop`/`unop`/`op`（依赖方向反转为 `backend → ast`；2b.2 的「一运算符一 class / `ast/compute` 子包」由此收敛，该子包已删除）；
语句模型 = 统一 `block` + 语句列表（块可嵌套，尾表达式即块值）；控制流 = `if/else` + `while` + `return` + 赋值（`x = e;`，不引入 `mut`）；
分支 = 条件求值入 EAX 后 `test eax,eax` + `jz`，标号与 rel32 相对跳转由 `codebuffer` 回填。

后续子期（方向占位）：**2b.4+** 更多语言特性、**2c** sysroot 接入、**2d** 增量缓存。

---

## 四、验证

```powershell
javac -d build -sourcepath RustJCode (Get-ChildItem RustJCode -Recurse -Filter *.java).FullName
jar cfe RustJ.jar main -C build .
java -jar RustJ.jar RustJCode/examples/flow.rs
.\RustJ\out\main.exe; echo "exit=$LASTEXITCODE"   # 期望 55（arith.rs 期望 7、ops.rs 期望 7、min.rs 期望 0）
```

- **回归**：`min.rs` 期望 `exit=0`；把其末值改为 `42` 重编，期望 `exit=42` —— 证明是**真执行**而非占位。
- **优先级**：`arith.rs` 为 `let x = 1 + 2 * 3; x`，期望 `exit=7`（验证 `*` 高于 `+`）。
- **运算符**：`ops.rs` 覆盖 `/ %`、一元负号与 6 种比较，期望 `exit=7`。
- **符号语义**：`-7 / 2` 与 `-7 % 2` 应分别得 `-3` 与 `-1`（与 Rust 一致）。
- **控制流**：`flow.rs` 为 `while` 累加 1..10（应得 55），并用两个 `if/else` 分别覆盖真/假分支；分支未按预期执行则结果偏离 55，期望 `exit=55`。
- **提前返回**：`return`（含 `if/else` 中提前返回）经端到端验证（如 `x==1` 返回 1、`else` 返回 3）。
- **语义检查**：`let x = y;`（`y` 未声明）与重复 `let x` 均以带行号的 `rustjerror` 报错并退出码 1。

---

## 五、明确不做

- ❌ 不接 `sysroot` / `std` / `rlib`（下一条独立子期 2c）。
- ❌ 不做 ELF / Mach-O（0003 P3 后置）。
- ❌ 不导入任何 OS 函数（本切片零 import）。
- ❌ 不做增量缓存 / `-RJT` / `-RJCC` 实际生效（**继续占位**）。
- ❌ 本切片不含模块 / 结构体 / trait / 泛型（子期 2b 后续）。
- ❌ 2b.3 不含 `for`/`loop`/`break`/`continue`/`match`、函数调用、多函数（2b.4+）。
- ❌ 不引入 `bool` 类型 / 真值字面量（条件沿用比较节点产出的 i32 0/1）；赋值不引入 `mut`（对任意已声明变量可写）。
- ❌ 不侵入 `cargo`/`rustc`；不改游戏三 crate；不碰线格式 / 契约 YAML。
- ❌ 不改游戏版本号、不触发 `y+1`、不打 tag、不写 BarekHistory。

---

## 六、关联

- [`0003-RustJ编译器.md`](0003-RustJ编译器.md)（上游设计：定位 / 分发形态 / 分期路线）
- [`README.md`](../../README.md)〈六、已采纳未来形态〉第 6 条（对应路线图条目）