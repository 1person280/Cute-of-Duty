//! 服务端内置 Web 服务（门户页 / 运维 API / 浏览器游玩桥）。
//!
//! 设计动机（Why）：0.12.3 让 `cod_server` 不再只是 TCP 权威模拟器，而是**自带 web 入口**：
//! 运维用浏览器即可看状态、踢人、广播；玩家用浏览器即可接入同一套线格式试玩。三件事
//! 共用同一个极简 HTTP/1.1 + WebSocket 承载（手写，见 [`http`] / [`ws`]），不引 web 框架。
//!
//! 边界（铁律 4）：本模块属**基础设施层**，对领域（combat/items/…）**一无所知**——所有
//! 需要"领域真相"的数据都经 [`WebOpsPort`] 由 `main.rs`（唯一编排者）注入。这正是
//! 「服务端算、前端只看」横切红线的落点。
//!
//! 目录说明：`net/web/` 为 3 级路径，是「嵌套 ≤2 层」红线的**唯一合法例外**（`x/y.rs` 单开子域）：
//! `web` 是 `net` 下真实存在的子域，嵌套有意义，故合规；各文件仍 ≤600 行、语义化命名、无下划线。

pub mod bridge;
pub mod error;
pub mod http;
pub mod ops;
pub mod portal;
pub mod router;
pub mod spawn;
pub mod tls;
pub mod ws;

pub use error::WebError;
pub use ops::{AuthorityOps, PlayerSlot};

use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::net::runtime::NetRuntime;

/// 门户/运维 API 对外暴露的只读状态快照（经 [`WebOpsPort::status`] 取得）。
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct WebStatus {
    pub version: String,
    pub online: usize,
    pub map: String,
    pub uptime_secs: u64,
    pub motd: String,
}

/// 单个在线玩家的展示信息。
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct WebPlayerInfo {
    pub conn_id: u64,
    pub name: String,
    pub level: u32,
}

/// web 服务所需的领域能力 Port（由 `main.rs` 实现，见 [`AuthorityOps`]）。
///
/// 只暴露 web 真正需要的四件事；web 模块据此对领域保持零耦合。
pub trait WebOpsPort: Send + Sync {
    /// 当前服务端状态快照。
    fn status(&self) -> WebStatus;
    /// 当前在线玩家列表。
    fn players(&self) -> Vec<WebPlayerInfo>;
    /// 向所有在线连接广播一条公告。
    fn announce(&self, text: &str) -> Result<(), WebError>;
    /// 踢下指定连接。
    fn kick(&self, conn_id: u64) -> Result<(), WebError>;
}

/// web 服务配置（权威默认见 `ServerCode/config/web.yaml`，经 `config::load_web_config` 加载）。
#[derive(Debug, Clone, serde::Deserialize, PartialEq)]
pub struct WebConfig {
    pub enabled: bool,
    pub http: HttpConfig,
    pub https: HttpsConfig,
    pub admin: AdminConfig,
    pub portal: PortalConfig,
}

/// HTTP 明文端口配置。
#[derive(Debug, Clone, serde::Deserialize, PartialEq)]
pub struct HttpConfig {
    pub addr: String,
    pub port: u16,
}

/// HTTPS 端口与证书配置（TLS 承载在第二步接入，见 ADR 0007）。
#[derive(Debug, Clone, serde::Deserialize, PartialEq)]
pub struct HttpsConfig {
    pub enabled: bool,
    pub addr: String,
    pub port: u16,
    pub cert_path: String,
    pub key_path: String,
}

/// 运维 API 鉴权配置（token 只从环境变量读，不落配置文件）。
#[derive(Debug, Clone, serde::Deserialize, PartialEq)]
pub struct AdminConfig {
    pub token_env: String,
}

/// 门户页展示配置。
#[derive(Debug, Clone, serde::Deserialize, PartialEq)]
pub struct PortalConfig {
    pub title: String,
    pub motd: String,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            http: HttpConfig { addr: "0.0.0.0".into(), port: 8080 },
            https: HttpsConfig {
                enabled: true,
                addr: "0.0.0.0".into(),
                port: 8443,
                cert_path: "certs/fullchain.pem".into(),
                key_path: "certs/privkey.pem".into(),
            },
            admin: AdminConfig { token_env: "COD_WEB_TOKEN".into() },
            portal: PortalConfig {
                title: "Cute Of Duty · 服务器门户".into(),
                motd: "欢迎来到 Cute Of Duty".into(),
            },
        }
    }
}

/// 启动 web 服务：按配置分别拉起 HTTP 与 HTTPS 监听（各自独立任务）。
///
/// 返回 `Err` 仅在**主 HTTP 端口无法绑定**时（配置写错就该当场失败）；HTTPS 证书缺失/
/// 解析失败或 `ring` 不可用时降级为告警（HTTP 门户仍可用），契合"反向代理终止 TLS"的
/// 回退路径。`rt` 用于浏览器游玩桥（WS）复用同一套传输无关运行时。
pub async fn serve(
    config: WebConfig,
    ops: Arc<dyn WebOpsPort>,
    rt: Arc<NetRuntime>,
) -> Result<(), WebError> {
    if !config.enabled {
        tracing::info!("web 服务已禁用（web.yaml: enabled=false）");
        return Ok(());
    }

    let token = std::env::var(&config.admin.token_env)
        .ok()
        .filter(|s| !s.trim().is_empty());
    if token.is_none() {
        tracing::warn!(
            "运维 API 未启用鉴权：环境变量 {} 未设置，/api/* 将返回 503",
            config.admin.token_env
        );
    }

    let router = Arc::new(router::Router::new(
        Arc::clone(&ops),
        token,
        config.portal.clone(),
    ));

    if config.https.enabled {
        match tls::build_acceptor(&config.https) {
            Ok(acceptor) => {
                let addr = format!("{}:{}", config.https.addr, config.https.port);
                match TcpListener::bind(&addr).await {
                    Ok(listener) => {
                        tracing::info!("web 门户已监听: https://{addr}");
                        let router = Arc::clone(&router);
                        let rt = Arc::clone(&rt);
                        tokio::spawn(async move {
                            if let Err(e) = accept_https(listener, acceptor, router, rt).await {
                                tracing::warn!("HTTPS 监听退出: {e}");
                            }
                        });
                    }
                    Err(e) => tracing::warn!("HTTPS 端口 {addr} 绑定失败（降级仅 HTTP）: {e}"),
                }
            }
            Err(e) => tracing::warn!("HTTPS 证书加载失败（降级仅 HTTP）: {e}"),
        }
    }

    let addr = format!("{}:{}", config.http.addr, config.http.port);
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("web 门户已监听: http://{addr}");
    accept_http(listener, router, rt).await
}

/// HTTP 接受循环。
async fn accept_http(
    listener: TcpListener,
    router: Arc<router::Router>,
    rt: Arc<NetRuntime>,
) -> Result<(), WebError> {
    loop {
        let (stream, peer) = listener.accept().await?;
        let _ = stream.set_nodelay(true);
        let router = Arc::clone(&router);
        let rt = Arc::clone(&rt);
        tokio::spawn(async move {
            if let Err(e) = handle_plain(stream, router, rt).await {
                tracing::debug!("HTTP 连接结束（{peer}）: {e}");
            }
        });
    }
}

/// HTTPS 接受循环：每条 TCP 先过 TLS 握手，再走同一套请求处理（含 WS 升级）。
async fn accept_https(
    listener: TcpListener,
    acceptor: tokio_rustls::TlsAcceptor,
    router: Arc<router::Router>,
    rt: Arc<NetRuntime>,
) -> Result<(), WebError> {
    loop {
        let (stream, peer) = listener.accept().await?;
        let _ = stream.set_nodelay(true);
        let acceptor = acceptor.clone();
        let router = Arc::clone(&router);
        let rt = Arc::clone(&rt);
        tokio::spawn(async move {
            match acceptor.accept(stream).await {
                Ok(tls) => {
                    if let Err(e) = handle_tls(tls, router, rt).await {
                        tracing::debug!("HTTPS 连接结束（{peer}）: {e}");
                    }
                }
                Err(e) => tracing::debug!("TLS 握手失败（{peer}）: {e}"),
            }
        });
    }
}

/// 处理一条明文 TCP 连接：`/ws` 升级交给桥，其余走路由（单请求后关闭）。
async fn handle_plain(
    mut stream: tokio::net::TcpStream,
    router: Arc<router::Router>,
    rt: Arc<NetRuntime>,
) -> Result<(), WebError> {
    let req = match http::read_request(&mut stream).await? {
        Some(r) => r,
        None => return Ok(()),
    };
    if req.path == "/ws" {
        return do_ws_upgrade(stream, &req, rt).await;
    }
    let resp = router.handle(&req);
    stream.write_all(&resp.encode()).await?;
    Ok(())
}

/// 处理一条 TLS 连接（与明文共用路由与 WS 升级路径）。
async fn handle_tls(
    mut tls: tokio_rustls::server::TlsStream<tokio::net::TcpStream>,
    router: Arc<router::Router>,
    rt: Arc<NetRuntime>,
) -> Result<(), WebError> {
    let req = match http::read_request(&mut tls).await? {
        Some(r) => r,
        None => return Ok(()),
    };
    if req.path == "/ws" {
        return do_ws_upgrade(tls, &req, rt).await;
    }
    let resp = router.handle(&req);
    tls.write_all(&resp.encode()).await?;
    Ok(())
}

/// 完成 WebSocket 握手（101）后，把连接交给 [`bridge`] 驱动。
///
/// 非升级请求（缺少 `Upgrade: websocket`）返回 426；这正是浏览器骨架页与原生 TCP
/// 之外的第二条接入路径。
async fn do_ws_upgrade<S>(mut stream: S, req: &http::Request, rt: Arc<NetRuntime>) -> Result<(), WebError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    if !ws::is_upgrade(req) {
        let resp = http::Response::text(426, "需要 Upgrade: websocket");
        stream.write_all(&resp.encode()).await?;
        return Ok(());
    }
    let key = req.header("sec-websocket-key").unwrap_or("");
    let accept = ws::accept_key(key);
    let head = format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
    );
    stream
        .write_all(head.as_bytes())
        .await
        .map_err(|e| WebError::Ws(format!("写 101 握手失败: {e}")))?;
    bridge::run(stream, rt).await
}