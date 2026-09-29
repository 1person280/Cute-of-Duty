//! TCP 传输：连接接受与双通道读写（0.12 双通道）
//!
//! 服务器权威架构的网络骨架：单一监听端口，每条连接**首条包恒为 256B 绑定包**，据其中
//! 角色字段把该连接固定为两条通道之一：
//!
//! - **控制通道**（[`Role::Control`]，双向，恒 256B 包）：客户端上行意图 → `NetCommand`
//!   命令队列，供服务端权威主循环每个固定 Tick 消费；服务端下行指令/快照/事件 → `SendScheduler`
//!   指令优先组包后写回。
//! - **资源通道**（[`Role::Resource`]，只下行，恒 4096B 包）：服务端按 `key` 顺序下发体素
//!   几何/动画，客户端重组后落固定槽位池。
//!
//! 两条通道**不合并包**，各按固定包长读写；资源连接凭绑定包里的档案名关联到同名控制连接。
//! 本模块只负责 TCP 连接建立、收发与扇出；**协议解析与命令分发在 [`super::runtime`]**，
//! 与 WebSocket 传输共用同一套（见 `super::web::bridge`）。

use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use crate::net::codec::encode_server;
use crate::net::packet::{
    PacketHeader, PacketKind, PacketWriter, Role, HEADER_BYTES, PACKET_BYTES, RES_PACKET_BYTES,
};
use crate::net::protocol::ServerMessage;
use crate::net::resource_stream::{encode_batch_end, encode_resource};
use crate::net::runtime::{ControlSession, NetRuntime, ResourceJob};
use crate::net::scheduler::SendScheduler;

/// 接受连接主循环：同端口收连接，每条连接按首条绑定包分角色。
pub async fn accept_loop(addr: String, rt: Arc<NetRuntime>) -> std::io::Result<()> {
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("服务器权威已监听: {addr}");
    loop {
        let (stream, peer) = listener.accept().await?;
        // TCP_NODELAY：大量小定长包，禁用 Nagle 合并避免人为攒包拖慢往返（延迟面板可见）。
        let _ = stream.set_nodelay(true);
        let rt_child = Arc::clone(&rt);
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, rt_child).await {
                tracing::warn!("连接处理结束（{peer}）: {e}");
            }
        });
    }
}

/// 处理一条新连接：读首条 256B 绑定包定角色，转入对应通道处理。
async fn handle_connection(
    stream: tokio::net::TcpStream,
    rt: Arc<NetRuntime>,
) -> Result<(), String> {
    let (mut reader, writer) = stream.into_split();
    let mut bind = [0u8; PACKET_BYTES];
    reader
        .read_exact(&mut bind)
        .await
        .map_err(|e| format!("读取绑定包失败: {e}"))?;
    let mut hbuf = [0u8; HEADER_BYTES];
    hbuf.copy_from_slice(&bind[..HEADER_BYTES]);
    let header = PacketHeader::decode(&hbuf, PACKET_BYTES)?;
    if header.kind != PacketKind::Bind {
        return Err(format!("首条消息不是绑定包: {:?}", header.kind));
    }
    let role = header.role()?;
    let plen = header.payload_len as usize;
    let name = String::from_utf8(bind[HEADER_BYTES..HEADER_BYTES + plen].to_vec())
        .map_err(|e| format!("绑定包档案名非法: {e}"))?;
    match role {
        Role::Control => handle_control(reader, writer, rt, name).await,
        Role::Resource => handle_resource(reader, writer, rt, name).await,
    }
}

/// 控制通道：绑定档案 → 投递 Connect → 读写循环。
async fn handle_control(
    mut reader: OwnedReadHalf,
    writer: OwnedWriteHalf,
    rt: Arc<NetRuntime>,
    name: String,
) -> Result<(), String> {
    // open_control 一次完成"分配 conn_id + 绑定档案 + 注册控制写队列 + 投递 Connect"。
    let (conn_id, control_rx) = rt.open_control(&name);
    tracing::info!("控制连接 #{conn_id} 绑定档案「{name}」");

    let writer_task = tokio::spawn(control_write_loop(writer, control_rx));

    control_read_loop(&mut reader, conn_id, &rt).await;
    rt.notify_disconnect(conn_id);
    rt.remove_control_writer(conn_id);
    let _ = writer_task.await;
    Ok(())
}

/// 资源通道：凭档案名关联控制连接 → 起只下行写循环 → 监测对端关闭。
async fn handle_resource(
    reader: OwnedReadHalf,
    writer: OwnedWriteHalf,
    rt: Arc<NetRuntime>,
    name: String,
) -> Result<(), String> {
    // 控制连接通常先到；短暂重试以容忍两连接建连顺序的竞态。
    let mut found = rt.control_id_for_profile(&name);
    let mut tries = 0;
    while found.is_none() && tries < 50 {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        found = rt.control_id_for_profile(&name);
        tries += 1;
    }
    let conn_id = found.ok_or_else(|| format!("资源连接找不到档案「{name}」的控制连接"))?;
    tracing::info!("资源连接绑定到控制连接 #{conn_id}（档案「{name}」）");

    let (tx, rx) = mpsc::unbounded_channel();
    rt.register_resource_writer(conn_id, tx);
    let writer_task = tokio::spawn(resource_write_loop(writer, rx));

    monitor_close(reader).await;
    rt.remove_resource_writer(conn_id);
    let _ = writer_task.await;
    Ok(())
}

/// 控制通道写循环：收消息 → 编线上形态 → 指令优先组包 → 一次写全。
async fn control_write_loop(
    mut writer: OwnedWriteHalf,
    mut rx: mpsc::UnboundedReceiver<ServerMessage>,
) {
    let mut sched = SendScheduler::new();
    while let Some(msg) = rx.recv().await {
        if let Ok(wire) = encode_server(&msg) {
            sched.push(wire);
        }
        // 抽干当前已到的更多消息，令"指令优先组包"在单次脉冲内充分生效。
        while let Ok(more) = rx.try_recv() {
            if let Ok(wire) = encode_server(&more) {
                sched.push(wire);
            }
        }
        flush_scheduler(&mut sched, &mut writer).await;
    }
}

/// 资源通道写循环：按批把资源切片写满 4096B 包，末尾附 `ResourceEnd`。
async fn resource_write_loop(
    mut writer: OwnedWriteHalf,
    mut rx: mpsc::UnboundedReceiver<ResourceJob>,
) {
    let mut seq: u64 = 0;
    while let Some(job) = rx.recv().await {
        let mut w = PacketWriter::new(RES_PACKET_BYTES);
        let ResourceJob::Batch { region, items } = job;
        for item in &items {
            if let Err(e) = encode_resource(item, &mut seq, &mut w) {
                tracing::warn!("资源编码失败: {e}");
            }
        }
        if let Err(e) = encode_batch_end(region, &mut seq, &mut w) {
            tracing::warn!("资源批次终止编码失败: {e}");
        }
        if !w.is_empty() && writer.write_all(&w.take()).await.is_err() {
            break;
        }
    }
}

/// 把调度器已攒的包一次写全（TCP 与 WebSocket 共用的下行冲刷语义）。
pub(crate) async fn flush_scheduler(
    sched: &mut SendScheduler,
    writer: &mut OwnedWriteHalf,
) {
    match sched.flush() {
        Ok(bytes) if !bytes.is_empty() => {
            if writer.write_all(&bytes).await.is_err() {
                tracing::warn!("下行写失败，连接将结束");
            }
        }
        Ok(_) => {}
        Err(e) => tracing::warn!("下行编码失败: {e}"),
    }
}

/// 监测资源连接对端关闭（资源连接为只下行，读到的字节一律忽略）。
async fn monitor_close(mut reader: OwnedReadHalf) {
    let mut buf = [0u8; 64];
    loop {
        match reader.read(&mut buf).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
    }
}

/// 控制通道读循环：按恒 256B 包读字节，交给传输无关的 [`ControlSession`] 解析与分发。
async fn control_read_loop(reader: &mut OwnedReadHalf, conn_id: u64, rt: &Arc<NetRuntime>) {
    let mut session = ControlSession::new(conn_id);
    let mut buf = vec![0u8; PACKET_BYTES];
    loop {
        if reader.read_exact(&mut buf).await.is_err() {
            return;
        }
        match session.feed(rt, &buf) {
            Ok(true) => {}
            Ok(false) => return, // 客户端主动断开
            Err(e) => {
                tracing::warn!("连接 #{conn_id} 上行包处理失败: {e}");
                return;
            }
        }
    }
}