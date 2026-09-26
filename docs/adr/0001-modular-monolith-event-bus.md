# ADR 0001 · 采用「模块化单体 + 事件总线 + Trait 接口」作为目标架构

- **状态**：已接受（Accepted）
- **日期**：2026-09-26
- **决策者**：项目 owner
- **影响范围**：全仓库（`ServerCode` / `HostCode` / 文档 / 协议）

---

## 背景（Context）

项目当前是「ServerCode 权威模拟 + HostCode 纯表现」的双 crate 结构，功能推进很快，但边界在持续失守：

1. **两套背包并存**：`inventory`（索引制 + `equipment` 注册表）与 `items`（格位制）都声称拥有物品数据。
2. **客户端直连服务端具体类型**：`hud` / `menu` / `world` 大量 `use cute_of_duty_server::{items::LootItem, interact::*, ...}`，
   服务端任何内部重构都会打断客户端，且没有任何兼容性承诺。
3. **同层横向依赖**：`hud_bigmap.rs` 直接读 `crate::menu::PauseMenu`。
4. **`launcher` 职责膨胀**：既是装配层又是表现代码宿主。
5. **文档与代码脱节**：没有模块边界文档、没有变更兼容性记录、协议语义靠口头约定。

若不先锁边界，继续叠加功能只会让"屎山"复利增长，且将来**拆微服务必然重写**。

## 决策（Decision）

采用 **模块化单体 + 事件总线 + Trait 接口** 作为目标架构，并配套四条强制纪律：

1. **模块间禁止直接调用**：跨模块只允许 ①经 Trait 接口 ②经事件总线。同层禁止横向调用。
2. **数据所有权唯一**：每份可变状态只有一个 owning 模块；他人只持快照/句柄，不持 `&mut`。
3. **依赖无环**，且严格分层单向：表现层 → 契约层 → 领域层 → 基础设施层。
4. **契约机器可读**：跨模块/跨进程载荷用 YAML 描述（`docs/contracts/`），协议版本 `x.y.z` —
   `x+1` 不兼容**必须附迁移指南**，`y+1` 加性，`z+1` 仅兼容性修复。

配套工程规范（同时写入 [CONTRIBUTING.md](../../CONTRIBUTING.md)）：
- Rust：禁 `unwrap`（用 `?`）、错误统一 `thiserror`、内部字段 `pub(crate)`、异步保 `Send + Sync`、配置 `serde` 外部化。
- 文档：每模块 `module.md`（边界/数据/接口/事件/成熟度 L0–L3）、`barek-history.md`（变更类型 + 兼容性 + 迁移指南）、
  `docs/contracts/*.yaml`（契约）、`docs/adr/*`（决策）。
- 模块分级：L2 及以上变更**必须审核**（maintainer 参与）。

## 后果（Consequences）

**正向**
- 代码边界锁死 → 将来按模块拆微服务**无需重写**，只需把 Trait 调用换成 RPC、把事件换成消息队列。
- 模块可 mock、可独立测试；编译单元清晰。

**代价 / 风险**
- 短期内需要先**停功能、补边界**（已冻结，见 [stop-doing.md](../stop-doing.md)）。
- 引入事件总线会带来"事件风暴 / 时序难追踪"的风险，需配套：事件命名规范（过去式事实）、关联 ID、事件日志。
- 现有的大批直接 `use` 需要逐块迁移，迁移期间会存在"新旧并存"的灰色地带，必须用 ADR 记录每一块的迁移边界。

## 未决事项（Open Questions）

引用 [module-boundaries.md 第八节](../architecture/module-boundaries.md) 的 5 条：
`items`/`inventory` 裁决、客户端契约适配层、`ModalState` 模态仲裁、`launcher` 瘦身时机、补文档与改代码的先后顺序。

## 替代方案（Alternatives considered）

- **A：继续按现状堆功能** — 已否决：复利式技术债，终局必须重写。
- **B：立刻拆成多 crate（每模块一个 crate）** — 否决：当前边界尚未确定，过早物理拆分会把错误边界焊死。
  模块化单体是"逻辑先拆、物理后拆"的正确中间态。
- **C：只加文档不加纪律** — 否决：文档会与代码漂移（本项目已发生过 config 硬编码漂移）。
