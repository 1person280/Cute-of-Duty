//! 控制通道下载线程：把服务端下行的恒 256B 包解成快照/控制两路
//!
//! 设计动机（Why）：0.12 控制通道每包恒定 256B = 32B 头 + 7×32B 单元。本线程是**阻塞
//! 单线程循环**：`read_exact` 恰好一包 → `PacketReader` 解头 → 按 [`PacketKind`] 分路：
//! - `Command`：`unit_count` 个 32B 指令单元 → [`decode_server_units`]（握手/回显/回主菜单）；
//! - `Data`：`Snapshot`/`Event`/`Control` 经 [`ChunkAssembler`] 重组后 [`decode_server_data`]，
//!   快照灌 [`SnapshotBuffer`]、其余灌控制通道。
//!
//! 资源不在此处：资源走独立 4096B 连接（见 [`super::resource_downlink`]）。**不设时钟、
//! 不 sleep**：收完一包立即读下一包。上传位于独立线程与独立 socket 句柄（见 [`super::uplink`]）。

use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cute_of_duty_server::net::codec::{decode_server_data, decode_server_units, units_from_payload};
use cute_of_duty_server::net::packet::{
    ChunkAssembler, PacketHeader, PacketKind, PacketReader, PACKET_BYTES,
};
use cute_of_duty_server::net::protocol::ServerMessage;

use crate::net::network::{ClientInbound, SnapshotChannel};

/// 下载循环：阻塞读 256B 包直到连接关闭或出错。
///
/// `start` 为连接起点，用于握手时折算 `connect_ms`；`pending` 与上传线程共享，
/// 记录 Ping 写入 socket 的时刻，收到 Pong 时折现真实往返。
pub fn downlink_loop(
    mut stream: TcpStream,
    snapshot_tx: SnapshotChannel,
    control_tx: std::sync::mpsc::Sender<ClientInbound>,
    start: Instant,
    shutdown: Arc<AtomicBool>,
    pending: Arc<Mutex<Option<Instant>>>,
) {
    let mut reader = PacketReader::new(PACKET_BYTES);
    let mut buf = vec![0u8; PACKET_BYTES];
    let mut asm = ChunkAssembler::default();

    loop {
        if shutdown.load(Ordering::Relaxed) {
            return;
        }
        // 恒定 256B 整包读：线格式保证包对齐，read_exact 恰好取满一包。
        if std::io::Read::read_exact(&mut stream, &mut buf).is_err() {
            shutdown.store(true, Ordering::Relaxed);
            return;
        }
        reader.feed(&buf);
        loop {
            match reader.next_packet() {
                Ok(Some((header, payload))) => {
                    if dispatch(
                        &header,
                        &payload,
                        &mut asm,
                        &snapshot_tx,
                        &control_tx,
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
                    eprintln!("[网络] 控制包解析失败: {e}");
                    shutdown.store(true, Ordering::Relaxed);
                    return;
                }
            }
        }
    }
}

/// 按包类别分路；返回 `Err` 表示应断开重连（畸形包/解码失败）。
#[allow(clippy::too_many_arguments)]
fn dispatch(
    header: &PacketHeader,
    payload: &[u8],
    asm: &mut ChunkAssembler,
    snapshot_tx: &SnapshotChannel,
    control_tx: &std::sync::mpsc::Sender<ClientInbound>,
    start: Instant,
    pending: &Arc<Mutex<Option<Instant>>>,
) -> Result<(), ()> {
    match header.kind {
        PacketKind::Command => {
            let units = units_from_payload(header.unit_count, payload)
                .map_err(|e| eprintln!("[网络] {e}"))?;
            for msg in decode_server_units(&units).map_err(|e| eprintln!("[网络] {e}"))? {
                route(msg, snapshot_tx, control_tx, start, pending);
            }
        }
        PacketKind::Data => {
            let kind = header.data_kind().map_err(|e| eprintln!("[网络] {e}"))?;
            if let Some(full) = asm.push(header.continuation, payload) {
                let msg =
                    decode_server_data(kind, &full).map_err(|e| eprintln!("[网络] {e}"))?;
                route(msg, snapshot_tx, control_tx, start, pending);
            }
        }
        // 控制通道不应出现资源/绑定包：忽略（不视为致命，保持连接）。
        PacketKind::Bind | PacketKind::Resource | PacketKind::ResourceEnd => {}
    }
    Ok(())
}

/// 把一条服务端消息路由到对应消费端（快照灌快照缓冲，其余灌控制通道）。
fn route(
    msg: ServerMessage,
    snapshot_tx: &SnapshotChannel,
    control_tx: &std::sync::mpsc::Sender<ClientInbound>,
    start: Instant,
    pending: &Arc<Mutex<Option<Instant>>>,
) {
    match msg {
        ServerMessage::Snapshot { entries, .. } => {
            let _ = snapshot_tx.send(entries);
        }
        ServerMessage::Handshake { assigned_id, .. } => {
            let connect_ms = start.elapsed().as_secs_f32() * 1000.0;
            let _ = control_tx.send(ClientInbound::Connected { assigned_id, connect_ms });
        }
        ServerMessage::Pong { .. } => {
            if let Some(t0) = pending.lock().ok().and_then(|mut p| p.take()) {
                let _ = control_tx.send(ClientInbound::Rtt(t0.elapsed().as_secs_f32() * 1000.0));
            }
        }
        other => {
            let _ = control_tx.send(ClientInbound::Server(other));
        }
    }
}
