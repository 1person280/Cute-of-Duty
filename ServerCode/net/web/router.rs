//! 路由分发：门户页、运维 API（token 鉴权）、WebSocket 升级入口。
//!
//! 设计动机（Why）：本服务路由极少且稳定，用一张 `match` 表比引路由框架更清晰可控；
//! 鉴权只在 `/api/*` 前置一层 Bearer token 校验，门户页与静态资源一律公开。
//!
//! `/ws` 在本版（第一步）返回 501：WebSocket 升级与桥接在第二步接入（见 ADR 0007）；
//! 届时由 `mod.rs` 在路由前拦截 `/ws` 完成握手，本模块无需改动。

use std::sync::Arc;

use super::http::{Request, Response};
use super::portal;
use super::{PortalConfig, WebOpsPort};

/// 路由处理器（持领域 Port + 运维 token + 门户展示配置）。
pub struct Router {
    ops: Arc<dyn WebOpsPort>,
    token: Option<String>,
    portal: PortalConfig,
}

impl Router {
    /// 构造路由。
    pub fn new(ops: Arc<dyn WebOpsPort>, token: Option<String>, portal: PortalConfig) -> Self {
        Self { ops, token, portal }
    }

    /// 处理一条请求，返回响应。
    pub fn handle(&self, req: &Request) -> Response {
        match (req.method.as_str(), req.path.as_str()) {
            ("GET", "/") => Response::html(200, portal::render_portal(&self.portal)),
            ("GET", "/api/status") => self.with_auth(req, || {
                Response::json(200, portal::status_json(&self.ops.status()))
            }),
            ("GET", "/api/players") => self.with_auth(req, || {
                let players = self.ops.players();
                let body = serde_json::to_string(&players).unwrap_or_else(|_| "[]".into());
                Response::json(200, body)
            }),
            ("POST", "/api/announce") => self.with_auth(req, || self.announce(req)),
            ("POST", "/api/kick") => self.with_auth(req, || self.kick(req)),
            ("GET", "/ws") => Response::text(
                501,
                "WebSocket 桥在 0.12.3 第二步接入（当前仅 HTTP 门户/运维 API 可用）",
            ),
            // 路径存在但方法不对 → 405；完全未知 → 404。
            (_, "/") | (_, "/api/status") | (_, "/api/players") | (_, "/api/announce")
            | (_, "/api/kick") | (_, "/ws") => Response::text(405, "方法不允许"),
            _ => Response::text(404, "未找到"),
        }
    }

    /// `/api/*` 统一鉴权：未配置 token → 503；缺/错 token → 401。
    fn with_auth(&self, req: &Request, ok: impl FnOnce() -> Response) -> Response {
        let Some(expected) = &self.token else {
            return Response::json(503, "{\"error\":\"admin token 未配置\"}");
        };
        let provided = req
            .header("authorization")
            .and_then(|v| v.strip_prefix("Bearer "))
            .unwrap_or("");
        if provided == expected {
            ok()
        } else {
            Response::json(401, "{\"error\":\"未授权\"}")
        }
    }

    /// `POST /api/announce`：请求体为纯文本公告。
    fn announce(&self, req: &Request) -> Response {
        let text = String::from_utf8_lossy(&req.body).trim().to_string();
        if text.is_empty() {
            return Response::json(400, "{\"error\":\"公告内容为空\"}");
        }
        match self.ops.announce(&text) {
            Ok(()) => Response::json(200, "{\"ok\":true}"),
            Err(e) => Response::json(500, json_error(&e.to_string())),
        }
    }

    /// `POST /api/kick`：请求体为 JSON `{"conn_id": 123}`。
    fn kick(&self, req: &Request) -> Response {
        let value: serde_json::Value = match serde_json::from_slice(&req.body) {
            Ok(v) => v,
            Err(_) => return Response::json(400, "{\"error\":\"请求体须为 JSON\"}"),
        };
        let Some(conn_id) = value.get("conn_id").and_then(|v| v.as_u64()) else {
            return Response::json(400, "{\"error\":\"缺少 conn_id\"}");
        };
        match self.ops.kick(conn_id) {
            Ok(()) => Response::json(200, "{\"ok\":true}"),
            Err(e) => Response::json(500, json_error(&e.to_string())),
        }
    }
}

/// 把错误文本包成 JSON（经 serde_json 转义，避免引号破坏 JSON 结构）。
fn json_error(msg: &str) -> String {
    serde_json::json!({ "error": msg }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::web::{WebError, WebPlayerInfo, WebStatus};

    struct StubOps;

    impl WebOpsPort for StubOps {
        fn status(&self) -> WebStatus {
            WebStatus {
                version: "0.12.3".into(),
                online: 2,
                map: "lawn".into(),
                uptime_secs: 7,
                motd: "hi".into(),
            }
        }
        fn players(&self) -> Vec<WebPlayerInfo> {
            vec![WebPlayerInfo { conn_id: 100, name: "a".into(), level: 1 }]
        }
        fn announce(&self, _text: &str) -> Result<(), WebError> {
            Ok(())
        }
        fn kick(&self, _conn_id: u64) -> Result<(), WebError> {
            Ok(())
        }
    }

    fn router(token: Option<&str>) -> Router {
        Router::new(
            Arc::new(StubOps),
            token.map(|s| s.to_string()),
            PortalConfig { title: "T".into(), motd: "M".into() },
        )
    }

    fn req(method: &str, path: &str, auth: Option<&str>, body: &[u8]) -> Request {
        Request {
            method: method.into(),
            path: path.into(),
            query: String::new(),
            headers: auth
                .map(|a| vec![("authorization".to_string(), a.to_string())])
                .unwrap_or_default(),
            body: body.to_vec(),
        }
    }

    #[test]
    fn portal_is_public() {
        let r = router(None).handle(&req("GET", "/", None, b""));
        assert_eq!(r.status, 200);
    }

    #[test]
    fn api_without_token_configured_is_503() {
        let r = router(None).handle(&req("GET", "/api/status", None, b""));
        assert_eq!(r.status, 503);
    }

    #[test]
    fn api_without_bearer_is_401() {
        let r = router(Some("secret")).handle(&req("GET", "/api/status", None, b""));
        assert_eq!(r.status, 401);
    }

    #[test]
    fn api_with_bearer_is_200() {
        let r = router(Some("secret")).handle(&req("GET", "/api/status", Some("Bearer secret"), b""));
        assert_eq!(r.status, 200);
        let body = String::from_utf8(r.body).unwrap();
        assert!(body.contains("\"online\":2"));
    }

    #[test]
    fn unknown_route_is_404_and_wrong_method_is_405() {
        assert_eq!(router(None).handle(&req("GET", "/nope", None, b"")).status, 404);
        assert_eq!(router(None).handle(&req("POST", "/", None, b"")).status, 405);
    }

    #[test]
    fn kick_parses_json_body() {
        let r = router(Some("t")).handle(&req("POST", "/api/kick", Some("Bearer t"), b"{\"conn_id\":123}"));
        assert_eq!(r.status, 200);
        let bad = router(Some("t")).handle(&req("POST", "/api/kick", Some("Bearer t"), b"not-json"));
        assert_eq!(bad.status, 400);
    }

    #[test]
    fn ws_route_reports_not_implemented_in_step1() {
        assert_eq!(router(None).handle(&req("GET", "/ws", None, b"")).status, 501);
    }
}