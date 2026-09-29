//! 门户页渲染与状态 JSON。
//!
//! 设计动机（Why）：门户 HTML 以 `include_str!` 编译期嵌入（与配置表同款语义），
//! 随二进制分发、无源码也能出页；标题/公告经占位符替换注入，避免在 Rust 里拼 HTML。
//! 状态 JSON 走 `serde_json` 序列化 `WebStatus`，形状与契约 [`web.yaml`] 一致。

use super::{PortalConfig, WebStatus};

/// 编译期嵌入的门户 HTML 模板（占位符 `{{TITLE}}` / `{{MOTD}}`）。
pub const PORTAL_HTML: &str = include_str!("portal.html");

/// 用配置填充门户模板。
pub fn render_portal(config: &PortalConfig) -> String {
    PORTAL_HTML
        .replace("{{TITLE}}", &escape_html(&config.title))
        .replace("{{MOTD}}", &escape_html(&config.motd))
}

/// 序列化状态快照为 JSON。
pub fn status_json(status: &WebStatus) -> String {
    serde_json::to_string(status).unwrap_or_else(|_| "{}".into())
}

/// 最小 HTML 转义（`< > & "`），防止配置文案注入标签。
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portal_replaces_placeholders() {
        let cfg = PortalConfig { title: "我的服".into(), motd: "欢迎".into() };
        let html = render_portal(&cfg);
        assert!(html.contains("我的服"));
        assert!(html.contains("欢迎"));
        assert!(!html.contains("{{TITLE}}"));
        assert!(!html.contains("{{MOTD}}"));
    }

    #[test]
    fn portal_escapes_config_text() {
        let cfg = PortalConfig { title: "<script>".into(), motd: "a&b".into() };
        let html = render_portal(&cfg);
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("a&amp;b"));
    }

    #[test]
    fn status_json_shape_is_stable() {
        let status = WebStatus {
            version: "0.12.3".into(),
            online: 3,
            map: "lawn".into(),
            uptime_secs: 42,
            motd: "m".into(),
        };
        let json = status_json(&status);
        assert_eq!(
            json,
            "{\"version\":\"0.12.3\",\"online\":3,\"map\":\"lawn\",\"uptime_secs\":42,\"motd\":\"m\"}"
        );
    }
}