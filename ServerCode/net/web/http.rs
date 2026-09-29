//! 极简 HTTP/1.1 请求解析与响应编码（纯手写，不引 web 框架）。
//!
//! 设计动机（Why）：本项目「服务端内置 web」只服务三类轻量场景——门户页、运维 API、
//! WebSocket 升级握手——远不足以承担一个 web 框架的依赖树与磁盘占用（底层冻结红线）。
//! 故只手写所需子集：请求行 + 头 + `Content-Length` 定长体；响应恒定
//! `Content-Length` + `Connection: close`（每请求一连接，规避 keep-alive/分块的状态机复杂度）。
//!
//! 与传输无关：解析函数对 `AsyncRead` 泛型，故 TCP 与 TLS 两条承载可共用同一份解析。

use tokio::io::{AsyncRead, AsyncReadExt};

use super::error::WebError;

/// 请求头体积上限（防御超大头部拖垮内存）。
pub const MAX_HEADER_BYTES: usize = 16 * 1024;
/// 请求体体积上限（门户/运维 API 均为小 JSON）。
pub const MAX_BODY_BYTES: usize = 1024 * 1024;

/// 一条已解析的 HTTP 请求。
#[derive(Debug, Clone)]
pub struct Request {
    pub method: String,
    /// 路径（不含查询串）。
    pub path: String,
    /// 查询串（不含 `?`，可能为空）。
    pub query: String,
    /// 头（名字统一小写，便于大小写无关取用）。
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    /// 按小写名取头值。
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

/// 一条待发送的 HTTP 响应。
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    /// 纯文本响应。
    pub fn text(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".into(), "text/plain; charset=utf-8".into())],
            body: body.into().into_bytes(),
        }
    }

    /// HTML 响应。
    pub fn html(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".into(), "text/html; charset=utf-8".into())],
            body: body.into().into_bytes(),
        }
    }

    /// JSON 响应。
    pub fn json(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".into(), "application/json; charset=utf-8".into())],
            body: body.into().into_bytes(),
        }
    }

    /// 仅状态码、空体。
    pub fn empty(status: u16) -> Self {
        Self { status, headers: Vec::new(), body: Vec::new() }
    }

    /// 编码为线格式字节：状态行 + 头 + 定长体，固定 `Connection: close`。
    pub fn encode(&self) -> Vec<u8> {
        let mut head = format!("HTTP/1.1 {} {}\r\n", self.status, status_text(self.status));
        let mut has_ct = false;
        for (k, v) in &self.headers {
            if k.eq_ignore_ascii_case("content-type") {
                has_ct = true;
            }
            head.push_str(k);
            head.push_str(": ");
            head.push_str(v);
            head.push_str("\r\n");
        }
        if !has_ct {
            head.push_str("Content-Type: text/plain; charset=utf-8\r\n");
        }
        head.push_str(&format!("Content-Length: {}\r\n", self.body.len()));
        head.push_str("Connection: close\r\n\r\n");
        let mut bytes = head.into_bytes();
        bytes.extend_from_slice(&self.body);
        bytes
    }
}

/// 状态码的英文短语（仅覆盖本服务用到的集合，未知码回退 `Unknown`）。
fn status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        426 => "Upgrade Required",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
}

/// 从异步读端读取一条完整请求。
///
/// 返回 `Ok(None)` 表示对端在请求开始前已关闭（正常结束）。
/// 逐字节读到 `\r\n\r\n` 以避免过读（不吞掉后续字节），对本服务的低频短请求足够。
pub async fn read_request<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Option<Request>, WebError> {
    let mut head: Vec<u8> = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match reader.read(&mut byte).await {
            Ok(0) => return Ok(None),
            Ok(_) => head.push(byte[0]),
            Err(e) => return Err(WebError::Http(format!("读取请求失败: {e}"))),
        }
        if head.len() >= 4 && &head[head.len() - 4..] == b"\r\n\r\n" {
            break;
        }
        if head.len() > MAX_HEADER_BYTES {
            return Err(WebError::Http("请求头过大".to_string()));
        }
    }

    let text = String::from_utf8(head).map_err(|e| WebError::Http(format!("请求头非 UTF-8: {e}")))?;
    let mut lines = text.split("\r\n");
    let request_line = lines.next().ok_or_else(|| WebError::Http("缺少请求行".to_string()))?;
    let mut parts = request_line.split(' ');
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("").to_string();
    if method.is_empty() || target.is_empty() {
        return Err(WebError::Http("请求行非法".to_string()));
    }
    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (target, String::new()),
    };

    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
        }
    }

    let len = headers
        .iter()
        .find(|(k, _)| k == "content-length")
        .and_then(|(_, v)| v.parse::<usize>().ok())
        .unwrap_or(0);
    if len > MAX_BODY_BYTES {
        return Err(WebError::Http("请求体过大".to_string()));
    }
    let mut body = vec![0u8; len];
    if len > 0 {
        reader
            .read_exact(&mut body)
            .await
            .map_err(|e| WebError::Http(format!("读取请求体失败: {e}")))?;
    }

    Ok(Some(Request { method, path, query, headers, body }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn parse_get_with_headers() {
        let raw = b"GET /api/status?x=1 HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer abc\r\n\r\n";
        let mut src: &[u8] = raw;
        let req = read_request(&mut src).await.unwrap().unwrap();
        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/api/status");
        assert_eq!(req.query, "x=1");
        assert_eq!(req.header("authorization"), Some("Bearer abc"));
        assert!(req.body.is_empty());
    }

    #[tokio::test]
    async fn parse_post_with_body() {
        let raw = b"POST /api/kick HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello";
        let mut src: &[u8] = raw;
        let req = read_request(&mut src).await.unwrap().unwrap();
        assert_eq!(req.method, "POST");
        assert_eq!(req.body, b"hello");
    }

    #[tokio::test]
    async fn closed_before_request_is_none() {
        let mut src: &[u8] = b"";
        assert!(read_request(&mut src).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn oversized_header_rejected() {
        let mut raw = b"GET / HTTP/1.1\r\nX: ".to_vec();
        raw.extend(std::iter::repeat(b'a').take(MAX_HEADER_BYTES + 10));
        let mut src: &[u8] = &raw;
        assert!(read_request(&mut src).await.is_err());
    }

    #[test]
    fn response_encode_sets_length_and_close() {
        let resp = Response::json(200, "{\"ok\":true}");
        let bytes = resp.encode();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(text.contains("Content-Type: application/json; charset=utf-8\r\n"));
        assert!(text.contains("Content-Length: 11\r\n"));
        assert!(text.contains("Connection: close\r\n"));
        assert!(text.ends_with("{\"ok\":true}"));
    }
}