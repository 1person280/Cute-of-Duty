//! web 服务装配（从 `main.rs` 抽出的启动段）。
//!
//! 设计动机（Why）：`main.rs` 是唯一编排者，但「加载 web 配置 → 注入 [`AuthorityOps`] →
//! 拉起 [`serve`]」这段装配高度内聚且不依赖任何领域状态，留在二进制入口里只会让入口文件
//! 膨胀。把它收进 `net::web` 自身，既让 web 子系统"自携装配"，又让 `main.rs` 只剩一行调用。
//!
//! 返回 `PlayerSlot` 供主循环每 Tick 刷新玩家展示槽（web 侧只读该槽）。

use std::sync::Arc;

use crate::config;
use crate::net::runtime::NetRuntime;

use super::error::WebError;
use super::{AuthorityOps, PlayerSlot, WebOpsPort};

/// 按配置装配并拉起 web 服务，返回主循环用于刷新玩家展示槽的句柄。
///
/// 返回 `Err` 于 **web 配置解析失败**（改错表就该当场失败）；HTTP/HTTPS 端口绑定或 TLS
/// 证书问题在 [`serve`] 内部降级为告警（HTTP 门户仍可用），不影响本函数的 `Ok`。
/// `tokio::spawn` 依赖调用方已处于 tokio 运行时上下文（`main` 为 `#[tokio::main]`）。
pub fn spawn_web(rt: Arc<NetRuntime>) -> Result<PlayerSlot, WebError> {
    let (web_cfg, web_cfg_path) = config::load_web_config()?;
    match &web_cfg_path {
        Some(path) => tracing::info!("Web 配置加载完成: {}", path.display()),
        None => tracing::warn!("未找到 Web 配置文件，使用内置默认配置"),
    }

    let (ops, player_slot) = AuthorityOps::new(
        Arc::clone(&rt),
        env!("CARGO_PKG_VERSION"),
        "lawn",
        web_cfg.portal.motd.clone(),
    );
    let web_ops: Arc<dyn WebOpsPort> = ops;
    tokio::spawn(async move {
        if let Err(e) = super::serve(web_cfg, web_ops, rt).await {
            tracing::warn!("web 服务退出: {e}");
        }
    });

    Ok(player_slot)
}