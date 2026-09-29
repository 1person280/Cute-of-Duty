# ADR 0007 · 服务端内置原生 Web 服务（HTTP/HTTPS 门户 + 运维 API + WebSocket 游玩桥）

- **状态**：已接受（Accepted）
- **日期**：2026-09-29
- **决策者**：项目 owner
- **影响范围**：`ServerCode/net`（`runtime` **新增**、`session` 精简为纯 TCP、`stages` **新增**、`web/*` **新增**）、`ServerCode/config`（`web.yaml` **新增** + `load_web_config`）、`ServerCode/Cargo.toml`（新增 `tokio-rustls`/`rustls`/`rustls-pemfile`/`thiserror`）、`ServerCode/main.rs`（装配外移到 `net::web::spawn`）
- **依据**：[ADR 0001](0001-modular-monolith-event-bus.md)「服务端权威」铁律；[ADR 0006](0006-small-fixed-packet-dual-channel.md) 的 256B/4096B 双通道线格式；`docs/contracts/protocol.yaml` `transport` 节；`docs/architecture/module-boundaries.md` 分层纪律（`net` 属基础设施层）

---

## 背景（Context）

`cod_server` 此前只有 TCP 双通道线格式（256B 控制包 / 4096B 资源包），没有任何 Web 能力：运维看状态要登机跑命令，公告/踢人要改代码，也没有任何"浏览器接入"的路径。

0.12.3 目标是在**服务端进程内**一次交付三件事：**①服务器门户/状态页**（浏览器打开即见在线人数/版本/公告）、**②管理运维 API**（REST 查询/广播/踢人，token 鉴权）、**③浏览器内游玩首期**（WebSocket 桥，浏览器按与原生客户端**完全相同的包格式**收发 + 极简 JS 骨架页）。

关键可行性结论（已核对源码）：浏览器端可**直接复用现有线格式**——WebSocket 二进制帧承载同样的定长包；服务端只需把"TCP 读写"换成"可注入的传输端点"，`session` 的协议解析与 `NetCommand` 分发**零改动复用**。因此本版**不改线格式、`wire_version` 仍为 12**，属 `z+1`（加性/细节）。

## 决策（Decision）

### 决策 1 · 抽出"传输无关"核心（`net/runtime.rs`）

把 `NetRuntime`/`Shared`/`NetCommand`/`ResourceJob` 从 `session.rs` 迁入新文件 `net/runtime.rs`，并新增传输无关能力：`NetRuntime::open_transport`（为一个非 TCP 连接登记控制+资源两条写队列、绑定档案名、投递 `Connect`，返回两条下行接收端）、`close_transport`（供踢人复用）、`ControlSession`（复用 `PacketReader`，`feed()` 驱动定长包解析）。

`net/session.rs` 随之精简为**纯 TCP 收发**（`accept_loop` + 控制/资源读写循环），`control_read_loop` 改由 `ControlSession::feed` 驱动，消除 TCP 与 WS 两条路径的解析重复。

### 决策 2 · web 子模块（纯 tokio 手写，不引 web 框架）

新增 `net/web/` 子域（3 级路径，是「嵌套 ≤2 层」红线的**唯一合法例外**——`web` 是 `net` 下真实子域，嵌套有意义）：

| 文件 | 职责 |
|---|---|
| `web/mod.rs` | 网关：`WebConfig` 类型、`WebOpsPort` trait、`serve()` 启动编排、`WebError` 重导出 |
| `web/error.rs` | 统一 `WebError`（`thiserror`），`From<std::io::Error>` / `From<String>` |
| `web/http.rs` | HTTP/1.1 请求解析 + 响应编码（状态行/头/Content-Length/keep-alive） |
| `web/tls.rs` | 由 PEM 证书/私钥构建 `rustls::ServerConfig` + `TlsAcceptor`（ring provider） |
| `web/ws.rs` | RFC6455 握手（**手写** SHA-1 + Base64）+ 帧编解码（text/binary/ping/pong/close、掩码） |
| `web/router.rs` | 路由：`/`、`/api/*`（Bearer token 鉴权）、`/ws`；404/405 |
| `web/portal.rs` + `portal.html` | 门户页 HTML（`include_str!` 嵌入）+ 状态 JSON 渲染 |
| `web/bridge.rs` | WS ↔ 控制/资源通道桥：收 WS 二进制帧 → `ControlSession::feed`；下行 → WS 二进制帧 |
| `web/spawn.rs` | 装配：`load_web_config` + `AuthorityOps::new` + `tokio::spawn(serve)` |
| `web/ops.rs` | `AuthorityOps`（持 `Arc<NetRuntime>`）实现 `WebOpsPort` |

SHA-1 / Base64 手写在 `web/ws.rs`，**不引** `sha1`/`base64`，压低依赖树与磁盘占用。

### 决策 3 · 领域解耦：`WebOpsPort`（铁律 4）

`web` 属**基础设施层**，对领域（combat/items/…）**一无所知**。web 真正需要的四件事（`status`/`players`/`announce`/`kick`）经 `WebOpsPort` trait 暴露，由 `main.rs`（唯一编排者）侧的 `AuthorityOps` 实现并注入。这正是「服务端算、前端只看」横切红线的落点。

### 决策 4 · 传输/TLS 与端口（唯一新增第三方依赖族）

- **HTTP + HTTPS 双端口**：HTTP 默认 `8080`、HTTPS 默认 `8443`（游戏 TCP `8888` 不动）；内置证书加载，**不依赖反向代理**。
- **TLS** 走 `rustls` + `tokio-rustls` + `ring`（+ `rustls-pemfile`）——纯 tokio 无法 TLS，这是**唯一新增依赖族**，也是本项目**首个 C/汇编依赖**（`ring`）。
- 证书由 **Let's Encrypt** 签发（Windows 用 win-acme，Linux 用 certbot），服务端只加载 `fullchain.pem` + `privkey.pem`（路径见 `web.yaml`）。
- **回退路径**：若目标 Windows 工具链无法编译 `ring`，置 `web.yaml` 的 `https.enabled: false`，只跑 HTTP，由前置反向代理（Caddy/nginx）终止 TLS。

### 决策 5 · 配置单一事实来源（`config/web.yaml`）

新增 `ServerCode/config/web.yaml`（权威默认，`include_str!` 嵌入 + 运行时覆盖），由 `config::load_web_config()` 加载（与 `load_element_config` 同款搜索/回退语义，含"嵌入默认须与仓库 YAML 一致"单测）。运维 token **只从环境变量读**（默认 `COD_WEB_TOKEN`），**不落配置文件**；token 未设时 `/api/*` 返回 503。

## 后果（Consequences）

**正向**
- 服务端由"只有 TCP 的权威模拟器"变为**自带 web 入口**：浏览器即可运维、即可接入试玩。
- 浏览器端**零新增线格式**：与原生端收发同一批 256B/4096B 包，服务端协议解析与 `NetCommand` 分发**零改动复用**。
- `web` 对领域零耦合，满足分层纪律；`main.rs` 装配外移到 `net::web::spawn` 后仍保持精简。

**代价 / 风险**
- **首个 C/汇编依赖 `ring`**：Windows 工具链需可用 C 工具链（GNU 通道通常需 clang/llvm 或 MSVC build tools）；不可编译时按决策 4 回退反向代理终止 TLS。
- HTTPS 证书缺失/解析失败或 `ring` 不可用 → **降级为告警**（HTTP 门户仍可用），不阻断服务端启动；仅主 HTTP 端口无法绑定时才返回 `Err`。
- `net` 为 L2 模块，本次**不触碰线格式**（`wire_version` 仍 12）→ **`z+1`**：`0.12.3`，双端 `0.12.3` 与 `0.12.2`/`0.12.1` 仍互通。

## 未决事项（Open Questions）

- **证书热重载**：续期后不重启（本期仅做"启动时加载"）。
- **运维 API 更细权限模型**：只读/读写分权、审计日志。
- **完整浏览器 3D 客户端**（体素渲染、输入、资源池）——另立版本。
- **`ring` 在目标 Windows 工具链的编译可行性**若受阻，转反向代理终止 TLS。

## 替代方案（Alternatives considered）

- **引入 web 框架（axum/actix）** — 否决：owner 明确要求纯 tokio 手写 HTTP/1.1 与 RFC6455，压低依赖树。
- **依赖反向代理终止 TLS（不内置 HTTPS）** — 否决（保留为回退）：owner 要求内置证书加载、HTTP+HTTPS 双端口。
- **新增 `sha1`/`base64` crate** — 否决：WebSocket 握手所需算法手写即可，避免为两处小算法扩依赖。
- **让 `web` 直接 `use` 领域模块读状态** — 否决：违反铁律 4 与分层纪律；改经 `WebOpsPort` 注入。