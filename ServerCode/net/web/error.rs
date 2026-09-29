//! web 模块统一错误类型（`thiserror`）。
//!
//! 设计动机（Why）：公约红线七.2 要求「错误类型统一 `thiserror`，禁 `Box<dyn Error>` 穿模块
//! 边界」。本模块此前各函数一律返回 `Result<_, String>`，字符串无法区分"HTTP 解析失败 /
//! WS 帧损坏 / TLS 证书问题 / 纯 IO 错误"，调用方只能看文本猜测，且 `?` 不能跨层传播
//! 结构化错误。统一为 [`WebError`] 后：① 每类错误有明确变体，调用方（`serve` / `spawn_web`）
//! 可分辨；② 经 `#[from]`/`From<String>` 让 `?` 自动提升旧有的字符串错误，**不改动契约层
//! 仍返回 `String` 的现状**，迁移零破坏。

use std::io;

/// web 模块统一错误类型。
///
/// 变体按"子系统"划分：HTTP 请求解析/编码、WebSocket 帧与握手、TLS 证书加载、底层 IO。
/// [`WebError::Wire`] 收编来自契约层（仍返回 `String`）与遗留字符串错误，经 `From<String>` /
/// `From<&str>` 自动提升，使 `?` 在 web 代码里可以无缝穿过这两类来源。
#[derive(Debug, thiserror::Error)]
pub enum WebError {
    /// HTTP 请求解析或响应编码失败。
    #[error("HTTP 处理失败: {0}")]
    Http(String),
    /// WebSocket 帧编解码或握手失败。
    #[error("WebSocket 处理失败: {0}")]
    Ws(String),
    /// TLS 证书/私钥加载或配置失败。
    #[error("TLS 处理失败: {0}")]
    Tls(String),
    /// 底层 IO 错误（连接读写、端口绑定、文件读取等）。
    #[error("IO 错误: {0}")]
    Io(#[from] io::Error),
    /// 遗留/契约层字符串错误（尚未归入上述变体的统一收口）。
    #[error("web 运行错误: {0}")]
    Wire(String),
}

impl From<String> for WebError {
    /// 把遗留的 `Result<_, String>`（契约/领域层仍返回字符串）经 `?` 自动提升为 [`WebError`]。
    fn from(value: String) -> Self {
        WebError::Wire(value)
    }
}

impl From<&str> for WebError {
    /// 同上，覆盖字符串字面量错误来源（如 `.ok_or("…")` 风格）。
    fn from(value: &str) -> Self {
        WebError::Wire(value.to_string())
    }
}