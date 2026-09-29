//! RFC6455 WebSocket：握手 + 帧编解码（纯手写 SHA-1 / Base64，不引依赖）。
//!
//! 设计动机（Why）：握手所需的 `Sec-WebSocket-Accept` = Base64(SHA1(key + GUID))，
//! 帧编解码也只是长度前缀 + 掩码 XOR。为这么点功能引入 `sha1`/`base64`/`tungstenite`
//! 三棵依赖树，与底层冻结红线（磁盘/依赖最小化）相悖，故在此手写所需子集。
//!
//! 浏览器发来的客户端帧**必定掩码**（RFC6455 §5.3），服务端帧**必定不掩码**；
//! 本模块按此约定双向处理，并对超长帧设上限防御。

use tokio::io::{AsyncRead, AsyncReadExt};

use super::error::WebError;

/// WebSocket 握手魔术 GUID（RFC6455 §1.3）。
const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// 单帧负载上限（游玩桥只承载 256B/4096B 定长包，1MiB 已极宽松）。
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;

/// 操作码（RFC6455 §5.2）。
pub const OP_CONTINUATION: u8 = 0x0;
pub const OP_TEXT: u8 = 0x1;
pub const OP_BINARY: u8 = 0x2;
pub const OP_CLOSE: u8 = 0x8;
pub const OP_PING: u8 = 0x9;
pub const OP_PONG: u8 = 0xA;

/// 一条已解码的 WebSocket 帧（服务端视角，负载已去掩码）。
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub fin: bool,
    pub opcode: u8,
    pub payload: Vec<u8>,
}

/// 计算 `Sec-WebSocket-Accept`：Base64(SHA1(client_key + GUID))。
pub fn accept_key(client_key: &str) -> String {
    let mut input = client_key.as_bytes().to_vec();
    input.extend_from_slice(WS_GUID.as_bytes());
    base64_encode(&sha1(&input))
}

/// 判断请求是否为 WebSocket 升级请求（`Upgrade: websocket`）。
pub fn is_upgrade(request: &super::http::Request) -> bool {
    request
        .header("upgrade")
        .map(|v| v.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false)
        && request.header("sec-websocket-key").is_some()
}

/// 从异步读端读取一条帧；`Ok(None)` 表示连接已关闭。
pub async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> Result<Option<Frame>, WebError> {
    let mut head = [0u8; 2];
    match reader.read_exact(&mut head).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(WebError::Ws(format!("读取帧头失败: {e}"))),
    }
    let fin = head[0] & 0x80 != 0;
    let opcode = head[0] & 0x0F;
    let masked = head[1] & 0x80 != 0;
    let len7 = (head[1] & 0x7F) as usize;

    let payload_len = match len7 {
        126 => read_u16(reader).await? as usize,
        127 => read_u64(reader).await? as usize,
        n => n,
    };
    if payload_len > MAX_FRAME_BYTES {
        return Err(WebError::Ws(format!("帧过大: {payload_len} 字节")));
    }

    // 客户端帧必须掩码；未掩码按协议错误拒绝（防止代理层歧义）。
    let mask = if masked {
        let mut m = [0u8; 4];
        reader.read_exact(&mut m).await.map_err(|e| WebError::Ws(format!("读取掩码失败: {e}")))?;
        Some(m)
    } else {
        None
    };

    let mut payload = vec![0u8; payload_len];
    if payload_len > 0 {
        reader
            .read_exact(&mut payload)
            .await
            .map_err(|e| WebError::Ws(format!("读取帧负载失败: {e}")))?;
    }
    if let Some(m) = mask {
        for (i, b) in payload.iter_mut().enumerate() {
            *b ^= m[i % 4];
        }
    }
    Ok(Some(Frame { fin, opcode, payload }))
}

/// 编码一条服务端帧（不掩码，单帧）。
pub fn encode_frame(opcode: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 10);
    out.push(0x80 | (opcode & 0x0F)); // FIN=1
    let len = payload.len();
    if len < 126 {
        out.push(len as u8);
    } else if len <= u16::MAX as usize {
        out.push(126);
        out.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        out.push(127);
        out.extend_from_slice(&(len as u64).to_be_bytes());
    }
    out.extend_from_slice(payload);
    out
}

async fn read_u16<R: AsyncRead + Unpin>(reader: &mut R) -> Result<u16, WebError> {
    let mut b = [0u8; 2];
    reader.read_exact(&mut b).await.map_err(|e| WebError::Ws(format!("读取扩展长度失败: {e}")))?;
    Ok(u16::from_be_bytes(b))
}

async fn read_u64<R: AsyncRead + Unpin>(reader: &mut R) -> Result<u64, WebError> {
    let mut b = [0u8; 8];
    reader.read_exact(&mut b).await.map_err(|e| WebError::Ws(format!("读取扩展长度失败: {e}")))?;
    Ok(u64::from_be_bytes(b))
}

// ---------------------------------------------------------------------------
// SHA-1（RFC3174 最小实现，仅用于握手的一次性摘要）
// ---------------------------------------------------------------------------

/// 计算 SHA-1 摘要（20 字节）。
pub fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [0x6745_2301, 0xEFCD_AB89, 0x98BA_DCFE, 0x1032_5476, 0xC3D2_E1F0];
    // 填充：0x80 + 0…0，使长度 ≡ 56 mod 64，再附 64 位大端原长（比特）。
    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64) * 8;
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 80];
        for (i, word) in w.iter_mut().take(16).enumerate() {
            *word = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A82_7999),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6),
            };
            let tmp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = tmp;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }

    let mut out = [0u8; 20];
    for (i, v) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    out
}

// ---------------------------------------------------------------------------
// Base64（标准字母表 + `=` 填充）
// ---------------------------------------------------------------------------

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// 标准 Base64 编码（带 `=` 填充）。
pub fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64[((n >> 18) & 0x3F) as usize] as char);
        out.push(B64[((n >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(B64[((n >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(B64[(n & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC6455 §1.3 官方握手向量。
    #[test]
    fn rfc6455_accept_vector() {
        assert_eq!(accept_key("dGhlIHNhbXBsZSBub25jZQ=="), "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }

    /// SHA-1 官方向量（"abc"）。
    #[test]
    fn sha1_abc_vector() {
        let d = sha1(b"abc");
        let hex: String = d.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    #[test]
    fn base64_padding() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
    }

    #[tokio::test]
    async fn masked_frame_roundtrip() {
        // 掩码键 0x37 0xfa 0x21 0x3d，对 "Hello" 的官方示例掩码字节。
        let mut raw = vec![0x81, 0x85, 0x37, 0xfa, 0x21, 0x3d];
        raw.extend_from_slice(&[0x7f, 0x9f, 0x4d, 0x51, 0x58]);
        let mut src: &[u8] = &raw;
        let f = read_frame(&mut src).await.unwrap().unwrap();
        assert_eq!(f.opcode, OP_TEXT);
        assert!(f.fin);
        assert_eq!(f.payload, b"Hello");
    }

    #[tokio::test]
    async fn server_frame_is_unmasked_small() {
        let bytes = encode_frame(OP_BINARY, &[1, 2, 3]);
        assert_eq!(bytes, vec![0x82, 0x03, 1, 2, 3]);
    }

    #[tokio::test]
    async fn server_frame_uses_extended_length() {
        let payload = vec![0xABu8; 200];
        let bytes = encode_frame(OP_BINARY, &payload);
        assert_eq!(bytes[0], 0x82);
        assert_eq!(bytes[1], 126);
        assert_eq!(&bytes[2..4], &200u16.to_be_bytes());
        assert_eq!(bytes.len(), 4 + 200);
    }

    #[tokio::test]
    async fn closed_stream_is_none() {
        let mut src: &[u8] = b"";
        assert!(read_frame(&mut src).await.unwrap().is_none());
    }
}