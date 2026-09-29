//! WebSocket ↔ 控制/资源通道桥（浏览器游玩的接入点）。
//!
//! 设计动机（Why）：浏览器只有一条 WebSocket 连接，而原生客户端用两条 TCP 连接分别承载
//! 256B 控制包与 4096B 资源包。本模块把**同一条 WS 连接**当成"两条通道的复用载体"：
//! 上行把 WS 二进制帧喂给传输无关的 [`ControlSession`]；下行把控制的 256B 包、资源的
//! 4096B 包各编成一个 WS 二进制帧下发——前端按**帧长**（256 / 4096）区分通道。
//!
//! 首帧约定与 TCP 完全一致：必须是 256B **绑定包**（`PacketKind::Bind`，负载 = 档案名）；
//! 据此经 [`NetRuntime::open_transport`] 登记传输（投递 `Connect`，主循环回 `Handshake`
//! 与 `ModelCatalog`——后者经资源通道以 4096B 帧下发）。
//!
//! 本模块只做"字节搬运 + 编解码"，不含业务裁决；连接的生命周期经 `close_transport` 收口。

use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, WriteHalf};
use tokio::sync::mpsc;

use crate::net::codec::encode_server;
use crate::net::packet::{
    PacketHeader, PacketKind, PacketWriter, HEADER_BYTES, PACKET_BYTES, RES_PACKET_BYTES,
};
use crate::net::resource_stream::{encode_batch_end, encode_resource};
use crate::net::runtime::{NetRuntime, ResourceJob};
use crate::net::scheduler::SendScheduler;

use super::error::WebError;
use super::ws;

/// 驱动一条已完成握手的 WebSocket 连接，直到任一端关闭。
pub async fn run<S>(stream: S, rt: Arc<NetRuntime>) -> Result<(), WebError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut read_half, write_half) = tokio::io::split(stream);

    // 首帧 = 256B 绑定包：取档案名并登记传输（语义与 TCP `handle_control` 一致）。
    let first = ws::read_frame(&mut read_half)
        .await?
        .ok_or_else(|| WebError::Ws("WS 在发送绑定包前关闭".to_string()))?;
    if first.opcode == ws::OP_CLOSE {
        return Ok(());
    }
    let name =
        bind_name(&first.payload).ok_or_else(|| WebError::Ws("WS 首帧不是合法 256B 绑定包".to_string()))?;
    let handles = rt.open_transport(&name);
    let mut session = rt.control_session(handles.conn_id);
    tracing::info!("WS 连接 #{} 绑定档案「{name}」", handles.conn_id);

    // 下行任务独占写半；上行读循环把 ping 的 pong、close 回执经 raw 通道交给它发送。
    let (raw_tx, raw_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let write_task = tokio::spawn(downstream(
        write_half,
        handles.control_rx,
        handles.resource_rx,
        raw_rx,
    ));

    loop {
        let frame = match ws::read_frame(&mut read_half).await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(e) => {
                tracing::debug!("WS 读帧失败: {e}");
                break;
            }
        };
        match frame.opcode {
            ws::OP_BINARY | ws::OP_TEXT => {
                if !session.feed(&rt, &frame.payload)? {
                    break; // 客户端主动 Disconnect
                }
            }
            ws::OP_PING => {
                let _ = raw_tx.send(ws::encode_frame(ws::OP_PONG, &frame.payload));
            }
            ws::OP_CLOSE => {
                let _ = raw_tx.send(ws::encode_frame(ws::OP_CLOSE, &frame.payload));
                break;
            }
            _ => {}
        }
    }

    // 收尾：投递 Disconnect + 摘除两条写队列（下行任务据此自然结束），再关闭 raw 通道。
    rt.close_transport(handles.conn_id);
    drop(raw_tx);
    let _ = write_task.await;
    Ok(())
}

/// 下行任务：把控制/资源两条通道的消息编成 WS 二进制帧，独占写半串行发送。
async fn downstream<S>(
    mut write: WriteHalf<S>,
    mut control_rx: mpsc::UnboundedReceiver<crate::net::protocol::ServerMessage>,
    mut resource_rx: mpsc::UnboundedReceiver<ResourceJob>,
    mut raw_rx: mpsc::UnboundedReceiver<Vec<u8>>,
) where
    S: AsyncWrite + Unpin,
{
    let mut sched = SendScheduler::new();
    loop {
        tokio::select! {
            msg = control_rx.recv() => {
                let Some(msg) = msg else { break };
                if let Ok(wire) = encode_server(&msg) {
                    sched.push(wire);
                }
                // 抽干当前已到的更多控制消息，令"指令优先组包"在单次脉冲内充分生效。
                while let Ok(more) = control_rx.try_recv() {
                    if let Ok(wire) = encode_server(&more) {
                        sched.push(wire);
                    }
                }
                if flush_control(&mut sched, &mut write).await.is_err() {
                    break;
                }
            }
            job = resource_rx.recv() => {
                let Some(job) = job else { break };
                if send_resource(job, &mut write).await.is_err() {
                    break;
                }
            }
            raw = raw_rx.recv() => {
                let Some(raw) = raw else { break };
                if write.write_all(&raw).await.is_err() {
                    break;
                }
            }
        }
    }
}

/// 冲刷控制调度器：把攒下的 256B 包逐个编成 WS 二进制帧写出。
async fn flush_control<S>(sched: &mut SendScheduler, write: &mut WriteHalf<S>) -> Result<(), WebError>
where
    S: AsyncWrite + Unpin,
{
    match sched.flush() {
        Ok(bytes) => {
            for chunk in bytes.chunks(PACKET_BYTES) {
                let frame = ws::encode_frame(ws::OP_BINARY, chunk);
                write.write_all(&frame).await?;
            }
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

/// 发送一批资源：按 4096B 定长片编成 WS 二进制帧。
async fn send_resource<S>(job: ResourceJob, write: &mut WriteHalf<S>) -> Result<(), WebError>
where
    S: AsyncWrite + Unpin,
{
    let mut seq: u64 = 0;
    let mut w = PacketWriter::new(RES_PACKET_BYTES);
    let ResourceJob::Batch { region, items } = job;
    for item in &items {
        if let Err(e) = encode_resource(item, &mut seq, &mut w) {
            tracing::warn!("WS 资源编码失败: {e}");
        }
    }
    if let Err(e) = encode_batch_end(region, &mut seq, &mut w) {
        tracing::warn!("WS 资源批次终止编码失败: {e}");
    }
    let bytes = w.take();
    for chunk in bytes.chunks(RES_PACKET_BYTES) {
        let frame = ws::encode_frame(ws::OP_BINARY, chunk);
        write.write_all(&frame).await?;
    }
    Ok(())
}

/// 从首帧负载解析绑定包中的档案名（校验魔数/版本/类别，与 TCP 同口径）。
fn bind_name(payload: &[u8]) -> Option<String> {
    if payload.len() < HEADER_BYTES {
        return None;
    }
    let mut hbuf = [0u8; HEADER_BYTES];
    hbuf.copy_from_slice(&payload[..HEADER_BYTES]);
    let header = PacketHeader::decode(&hbuf, PACKET_BYTES).ok()?;
    if header.kind != PacketKind::Bind {
        return None;
    }
    let end = HEADER_BYTES + header.payload_len as usize;
    if payload.len() < end {
        return None;
    }
    String::from_utf8(payload[HEADER_BYTES..end].to_vec()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::packet::{PacketHeader, PacketKind, Role};

    fn bind_payload(name: &str) -> Vec<u8> {
        let header = PacketHeader {
            kind: PacketKind::Bind,
            continuation: false,
            unit_count: 0,
            sub_kind: Role::Control as u8,
            region: Default::default(),
            chunk_index: 0,
            payload_len: name.len() as u16,
            seq: 0,
            key: 0,
        };
        let mut buf = vec![0u8; PACKET_BYTES];
        buf[..HEADER_BYTES].copy_from_slice(&header.encode());
        buf[HEADER_BYTES..HEADER_BYTES + name.len()].copy_from_slice(name.as_bytes());
        buf
    }

    #[test]
    fn bind_name_extracts_profile() {
        assert_eq!(bind_name(&bind_payload("alice")).as_deref(), Some("alice"));
    }

    #[test]
    fn bind_name_rejects_non_bind() {
        let mut payload = bind_payload("bob");
        payload[6] = PacketKind::Command as u8; // 覆写 kind
        assert!(bind_name(&payload).is_none());
    }

    #[test]
    fn bind_name_rejects_truncated() {
        assert!(bind_name(&[0u8; 4]).is_none());
    }
}