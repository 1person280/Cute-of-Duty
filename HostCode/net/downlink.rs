//! 下载线程：把服务端下行的 64KB 槽帧解成资源/快照/控制三路
//!
//! 设计动机（Why）：0.11 起下行统一为固定 64KB 帧（见服务端 `net::packet`）。本线程是
//! **阻塞单线程循环**：`read_exact` 恰好一帧 → `FrameReader` 解头 → 按 `FrameKind` 分路：
//! - `Resource` 帧投资源池通道（`Region` 决定在用/预取区）；
//! - `ResourceEnd` 投批次终止标记（目录就绪）；
//! - `Snapshot`/`Event`/`Control` 经 `ChunkAssembler` 重组后 `decode_server`，快照灌
//!   [`SnapshotBuffer`]、其余灌控制通道。
//!
//! **不设时钟、不 sleep**：收完一帧立即读下一帧，传输完成即是下一次的起点。上传位于
//! 独立线程与独立 socket 句柄（见 [`super::uplink`]），单线程内的读阻塞不会拖住上行。

use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cute_of_duty_server::net::packet::{
    ChunkAssembler, FRAME_BYTES, FrameKind, FrameReader, decode_server, resource_kind_of,
};

use crate::net::network::{ClientInbound, SnapshotChannel};
use crate::net::remote::{PoolMessage, ResourceFrame};

/// 下载循环：阻塞读帧直到连接关闭或出错。
///
/// `start` 为连接起点，用于握手时折算 `connect_ms`；`pending` 与上传线程共享，
/// 记录 Ping 写入 socket 的时刻，收到 Pong 时折现真实往返。
pub fn downlink_loop(
    mut stream: TcpStream,
    snapshot_tx: SnapshotChannel,
    control_tx: std::sync::mpsc::Sender<ClientInbound>,
    resource_tx: std::sync::mpsc::Sender<PoolMessage>,
    start: Instant,
    shutdown: Arc<AtomicBool>,
    pending: Arc<Mutex<Option<Instant>>>,
) {
    let mut reader = FrameReader::new();
    let mut buf = vec![0u8; FRAME_BYTES];
    let mut asm = ChunkAssembler::default();

    loop {
        if shutdown.load(Ordering::Relaxed) {
            return;
        }
        // 恒定 64KB 整帧读：线格式保证帧对齐，read_exact 恰好取满一帧。
        if std::io::Read::read_exact(&mut stream, &mut buf).is_err() {
            shutdown.store(true, Ordering::Relaxed);
            return;
        }
        reader.feed(&buf);
        loop {
            match reader.next_frame() {
                Ok(Some((header, payload))) => {
                    if dispatch(
                        &header,
                        &payload,
                        &mut asm,
                        &snapshot_tx,
                        &control_tx,
                        &resource_tx,
                        start,
                        &pending,
                    )
                    .is_err()
                    {
                        shutdown.store(true, Ordering::Relaxed);
                        return;
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    eprintln!("[网络] 下行帧解析失败: {e}");
                    shutdown.store(true, Ordering::Relaxed);
                    return;
                }
            }
        }
    }
}

/// 按帧类别分路；返回 `Err` 表示应断开重连（畸形帧/解码失败）。
#[allow(clippy::too_many_arguments)]
fn dispatch(
    header: &cute_of_duty_server::net::packet::FrameHeader,
    payload: &[u8],
    asm: &mut ChunkAssembler,
    snapshot_tx: &SnapshotChannel,
    control_tx: &std::sync::mpsc::Sender<ClientInbound>,
    resource_tx: &std::sync::mpsc::Sender<PoolMessage>,
    start: Instant,
    pending: &Arc<Mutex<Option<Instant>>>,
) -> Result<(), ()> {
    match header.kind {
        FrameKind::Resource => {
            let kind = resource_kind_of(header).map_err(|e| eprintln!("[网络] {e}"))?;
            let frame = ResourceFrame {
                kind,
                key: header.key,
                region: header.region,
                bytes: payload.to_vec(),
            };
            let _ = resource_tx.send(PoolMessage::Resource(frame));
        }
        FrameKind::ResourceEnd => {
            let _ = resource_tx.send(PoolMessage::BatchEnd);
        }
        FrameKind::Snapshot => {
            if let Some(full) = asm.push(header, payload).map_err(|e| eprintln!("[网络] {e}"))? {
                let msg = decode_server(&full).map_err(|e| eprintln!("[网络] {e}"))?;
                if let cute_of_duty_server::net::protocol::ServerMessage::Snapshot { entries, .. } = msg
                {
                    let _ = snapshot_tx.send(entries);
                }
            }
        }
        FrameKind::Event | FrameKind::Control => {
            if let Some(full) = asm.push(header, payload).map_err(|e| eprintln!("[网络] {e}"))? {
                let msg = decode_server(&full).map_err(|e| eprintln!("[网络] {e}"))?;
                match msg {
                    cute_of_duty_server::net::protocol::ServerMessage::Handshake { assigned_id, .. } => {
                        let connect_ms = start.elapsed().as_secs_f32() * 1000.0;
                        let _ = control_tx.send(ClientInbound::Connected { assigned_id, connect_ms });
                    }
                    cute_of_duty_server::net::protocol::ServerMessage::Pong { .. } => {
                        if let Some(t0) = pending.lock().ok().and_then(|mut p| p.take()) {
                            let _ = control_tx.send(ClientInbound::Rtt(t0.elapsed().as_secs_f32() * 1000.0));
                        }
                    }
                    other => {
                        let _ = control_tx.send(ClientInbound::Server(other));
                    }
                }
            }
        }
    }
    Ok(())
}