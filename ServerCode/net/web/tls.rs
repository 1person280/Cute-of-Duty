//! TLS 承载：由 PEM 证书/私钥构建 `rustls` 监听器（HTTPS 用）。
//!
//! 设计动机（Why）：纯 tokio 不含 TLS，故引入 `rustls + tokio-rustls + ring`——这是
//! 0.12.3 唯一的第三方新增依赖族（首个 C/汇编依赖，见 [ADR 0007]）。证书由外部签发
//! （推荐 Let's Encrypt：`fullchain.pem` + `privkey.pem`），服务端只做**启动时加载**，
//! 不依赖反向代理终止 TLS。
//!
//! 容错策略：证书缺失/解析失败一律返回 `Err(WebError::Tls)`，由 `serve` 降级为"仅 HTTP"并告警——
//! 契合"环境无法编译 ring 或未签证书时改走反向代理"的回退路径，而非让服务端启动失败。
//!
//! [ADR 0007]: ../../docs/adr/0007-native-web-service.md

use std::sync::Arc;

use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;

use super::error::WebError;
use super::HttpsConfig;

/// 由 PEM 证书链 + 私钥构建 TLS 接受器。
pub fn build_acceptor(config: &HttpsConfig) -> Result<TlsAcceptor, WebError> {
    // 安装 ring 加密提供者（幂等：重复安装返回 Err，直接忽略）。
    let _ = tokio_rustls::rustls::crypto::ring::default_provider().install_default();

    let cert_pem = std::fs::read(&config.cert_path)
        .map_err(|e| WebError::Tls(format!("读取证书失败 {}: {e}", config.cert_path)))?;
    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_pem.as_slice())
        .collect::<Result<_, _>>()
        .map_err(|e| WebError::Tls(format!("解析证书失败 {}: {e}", config.cert_path)))?;
    if certs.is_empty() {
        return Err(WebError::Tls(format!("证书文件不含任何证书: {}", config.cert_path)));
    }

    let key_pem = std::fs::read(&config.key_path)
        .map_err(|e| WebError::Tls(format!("读取私钥失败 {}: {e}", config.key_path)))?;
    let key: PrivateKeyDer<'static> = rustls_pemfile::private_key(&mut key_pem.as_slice())
        .map_err(|e| WebError::Tls(format!("解析私钥失败 {}: {e}", config.key_path)))?
        .ok_or_else(|| WebError::Tls(format!("私钥文件不含可用私钥: {}", config.key_path)))?;

    let server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| WebError::Tls(format!("TLS 配置失败（证书与私钥不匹配？）: {e}")))?;

    Ok(TlsAcceptor::from(Arc::new(server_config)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 证书/私钥文件缺失时必须报错（供 `serve` 降级为仅 HTTP，而不是崩溃）。
    #[test]
    fn missing_cert_files_error() {
        let cfg = HttpsConfig {
            enabled: true,
            addr: "0.0.0.0".into(),
            port: 8443,
            cert_path: "no/such/fullchain.pem".into(),
            key_path: "no/such/privkey.pem".into(),
        };
        let err = build_acceptor(&cfg);
        assert!(err.is_err());
    }
}