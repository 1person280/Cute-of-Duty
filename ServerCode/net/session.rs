//! 连接会话与会话运行时管理（0.12 双通道）
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
//! 计算永远发生在服务端主循环；本模块只负责连接建立、收发与扇出。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use crate::interact::InteractChoice;
use crate::items::TransferDir;
use crate::net::codec::{decode_client_data, decode_client_units, encode_server, units_from_payload};
use crate::net::packet::{
    ChunkAssembler, PacketHeader, PacketKind, PacketReader, PacketWriter, Region, Role,
    HEADER_BYTES, PACKET_BYTES, RES_PACKET_BYTES,
};
use crate::net::protocol::{ClientMessage, InventoryAction, PlayerInput, ServerMessage};
use crate::net::resource_stream::{encode_batch_end, encode_resource, split_catalog, ResourceMessage};
use crate::net::scheduler::SendScheduler;

/// 客户端连接向服务端权威主循环投递的事件。
#[derive(Debug)]
pub enum NetCommand {
    /// 新连接握手（含玩家档案名）
    Connect { conn_id: u64, name: String },
    /// 单帧玩家输入意图
    Input { conn_id: u64, player: PlayerInput },
    /// 背包 CRUD 意图（由权威主循环经 inventory 服务结算）
    Inventory { conn_id: u64, action: InventoryAction },
    /// 仓库选装确认（携带清单存会话热副本）
    Loadout { conn_id: u64, carried: Vec<String> },
    /// 进入训练场
    StartTraining { conn_id: u64 },
    /// 请求撤离（权威距离判定）
    ExtractRequest { conn_id: u64 },
    /// 切换干员（权威写回战斗组件索引）
    SwitchOperator { conn_id: u64, operator_id: u32 },
    /// 交互意图（权威做距离校验与效果发放）
    Interact { conn_id: u64, target: u64, choice: InteractChoice },
    /// 物资箱逐格转移意图（权威做距离校验 + 格位裁决）
    LootTransfer { conn_id: u64, target: u64, dir: TransferDir, index: usize },
    /// 延迟探测（原样回显 Pong）
    Ping { conn_id: u64, seq: u64 },
    /// 客户端对象池淘汰上报（0.12）：服务端从该连接常驻资源集合移除 `evicted` 键。
    PoolSync { conn_id: u64, region: u8, evicted: Vec<u64> },
    /// 连接断开（主动 Disconnect 或对端关闭/异常）
    Disconnect { conn_id: u64 },
}

/// 资源下发任务（服务端 → 资源连接写任务）。
#[derive(Debug)]
pub enum ResourceJob {
    /// 一批资源（顺序下发 + 末尾 `ResourceEnd` 终止标记）。
    Batch { region: Region, items: Vec<ResourceMessage> },
}

/// 会话运行时内部共享状态（短临界区：锁内不 await、不发包）。
#[derive(Default)]
struct Shared {
    /// conn_id → 控制通道写队列。
    control_writers: HashMap<u64, mpsc::UnboundedSender<ServerMessage>>,
    /// conn_id → 资源通道写队列。
    resource_writers: HashMap<u64, mpsc::UnboundedSender<ResourceJob>>,
    /// 档案名 → 控制连接 id（资源连接据绑定包档案名找到所属控制连接）。
    profile_to_control: HashMap<String, u64>,
    /// 资源连接尚未绑定前先攒下的资源任务（按 conn_id）。
    pending_resources: HashMap<u64, Vec<ResourceJob>>,
}

/// 会话运行时：持有命令队列发送端 + 各通道写任务队列表。
pub struct NetRuntime {
    cmd_tx: mpsc::UnboundedSender<NetCommand>,
    cmd_rx: Option<mpsc::UnboundedReceiver<NetCommand>>,
    shared: Arc<Mutex<Shared>>,
    next_id: AtomicU64,
}

impl NetRuntime {
    /// 创建会话运行时。
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        Self {
            cmd_tx,
            cmd_rx: Some(cmd_rx),
            shared: Arc::new(Mutex::new(Shared::default())),
            next_id: AtomicU64::new(100),
        }
    }

    /// 取走命令队列接收端（仅由服务端权威主循环调用一次）。
    pub fn take_command_receiver(&mut self) -> mpsc::UnboundedReceiver<NetCommand> {
        match self.cmd_rx.take() {
            Some(rx) => rx,
            None => mpsc::unbounded_channel().1,
        }
    }

    /// 分配连接 id（控制连接用；资源连接复用其控制连接的 id）。
    fn next_conn_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// 登记档案名 → 控制连接 id。
    fn bind_profile(&self, name: &str, conn_id: u64) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.profile_to_control.insert(name.to_string(), conn_id);
        }
    }

    /// 按档案名查控制连接 id（资源连接绑定用）。
    fn control_id_for_profile(&self, name: &str) -> Option<u64> {
        self.shared.lock().ok()?.profile_to_control.get(name).copied()
    }

    /// 注册控制通道写队列。
    fn register_control_writer(&self, conn_id: u64, tx: mpsc::UnboundedSender<ServerMessage>) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.control_writers.insert(conn_id, tx);
        }
    }

    /// 控制连接结束：移除写队列（丢弃发送端即令写任务收尾）。
    fn remove_control_writer(&self, conn_id: u64) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.control_writers.remove(&conn_id);
        }
    }

    /// 注册资源通道写队列，并把此前攒下的资源任务补齐。
    fn register_resource_writer(&self, conn_id: u64, tx: mpsc::UnboundedSender<ResourceJob>) {
        let Ok(mut shared) = self.shared.lock() else { return };
        if let Some(pending) = shared.pending_resources.remove(&conn_id) {
            for job in pending {
                let _ = tx.send(job);
            }
        }
        shared.resource_writers.insert(conn_id, tx);
    }

    /// 资源连接结束：移除写队列。
    fn remove_resource_writer(&self, conn_id: u64) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.resource_writers.remove(&conn_id);
        }
    }

    /// 向单个连接投递服务端消息。
    ///
    /// `ModelCatalog` 归资源通道（4096B 包）；资源连接未绑定时先攒着，绑定后自动补齐。
    pub fn send_to(&self, conn_id: u64, msg: ServerMessage) {
        match msg {
            ServerMessage::ModelCatalog { models, animations } => {
                let items = match split_catalog(&models, &animations, Region::InUse) {
                    Ok(items) => items,
                    Err(e) => {
                        tracing::warn!("模型目录摊平失败: {e}");
                        return;
                    }
                };
                self.push_resource_job(conn_id, ResourceJob::Batch { region: Region::InUse, items });
            }
            other => {
                if let Ok(shared) = self.shared.lock() {
                    if let Some(tx) = shared.control_writers.get(&conn_id) {
                        let _ = tx.send(other);
                    }
                }
            }
        }
    }

    /// 向所有在线控制连接扇出消息。
    pub fn send_to_all(&self, msg: ServerMessage) {
        let Ok(shared) = self.shared.lock() else { return };
        for tx in shared.control_writers.values() {
            let _ = tx.send(msg.clone());
        }
    }

    /// 投递一批资源任务：资源连接已绑定则直接下发，否则攒入待发队列。
    fn push_resource_job(&self, conn_id: u64, job: ResourceJob) {
        let Ok(mut shared) = self.shared.lock() else { return };
        if let Some(tx) = shared.resource_writers.get(&conn_id) {
            let _ = tx.send(job);
            return;
        }
        shared.pending_resources.entry(conn_id).or_default().push(job);
    }
}

impl Default for NetRuntime {
    fn default() -> Self {
        Self::new()
    }
}

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
    let conn_id = rt.next_conn_id();
    tracing::info!("控制连接 #{conn_id} 绑定档案「{name}」");
    rt.bind_profile(&name, conn_id);

    let (tx, rx) = mpsc::unbounded_channel();
    rt.register_control_writer(conn_id, tx);
    let writer_task = tokio::spawn(control_write_loop(writer, rx));

    // 绑定包即握手：直接投递 Connect（无需再发一条控制消息）。
    let _ = rt.cmd_tx.send(NetCommand::Connect { conn_id, name });

    control_read_loop(&mut reader, conn_id, &rt).await;
    let _ = rt.cmd_tx.send(NetCommand::Disconnect { conn_id });
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
        match sched.flush() {
            Ok(bytes) if !bytes.is_empty() => {
                if writer.write_all(&bytes).await.is_err() {
                    break;
                }
            }
            Ok(_) => {}
            Err(e) => tracing::warn!("下行编码失败: {e}"),
        }
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

/// 控制通道读循环：按恒 256B 包解析上行意图并投递。
async fn control_read_loop(reader: &mut OwnedReadHalf, conn_id: u64, rt: &Arc<NetRuntime>) {
    let mut packet_reader = PacketReader::new(PACKET_BYTES);
    let mut buf = vec![0u8; PACKET_BYTES];
    let mut asm = ChunkAssembler::default();
    loop {
        if reader.read_exact(&mut buf).await.is_err() {
            return;
        }
        packet_reader.feed(&buf);
        loop {
            match packet_reader.next_packet() {
                Ok(Some((header, payload))) => {
                    match handle_client_packet(rt, conn_id, &header, &payload, &mut asm) {
                        Ok(true) => {}
                        Ok(false) => return,
                        Err(e) => {
                            tracing::warn!("连接 #{conn_id} 上行包处理失败: {e}");
                            return;
                        }
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    tracing::warn!("连接 #{conn_id} 上行包解析失败: {e}");
                    return;
                }
            }
        }
    }
}

/// 处理一个上行包；`Ok(false)` 表示客户端主动断开、连接应结束。
fn handle_client_packet(
    rt: &Arc<NetRuntime>,
    conn_id: u64,
    header: &PacketHeader,
    payload: &[u8],
    asm: &mut ChunkAssembler,
) -> Result<bool, String> {
    match header.kind {
        PacketKind::Command => {
            let units = units_from_payload(header.unit_count, payload)?;
            for msg in decode_client_units(&units)? {
                if !dispatch_client_message(rt, conn_id, msg) {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        PacketKind::Data => {
            let kind = header.data_kind()?;
            if let Some(full) = asm.push(header.continuation, payload) {
                let msg = decode_client_data(kind, &full)?;
                if !dispatch_client_message(rt, conn_id, msg) {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        PacketKind::Bind => Ok(true),
        other => Err(format!("控制通道收到非预期包: {other:?}")),
    }
}

/// 把一条已解码的客户端消息投递为 `NetCommand`；返回 `false` 表示连接应终止。
fn dispatch_client_message(rt: &Arc<NetRuntime>, conn_id: u64, msg: ClientMessage) -> bool {
    match msg {
        ClientMessage::Connect { profile } => {
            let _ = rt.cmd_tx.send(NetCommand::Connect { conn_id, name: profile });
        }
        ClientMessage::Input { player } => {
            let _ = rt.cmd_tx.send(NetCommand::Input { conn_id, player });
        }
        ClientMessage::Inventory { action } => {
            let _ = rt.cmd_tx.send(NetCommand::Inventory { conn_id, action });
        }
        ClientMessage::Loadout { carried } => {
            let _ = rt.cmd_tx.send(NetCommand::Loadout { conn_id, carried });
        }
        ClientMessage::StartTraining => {
            let _ = rt.cmd_tx.send(NetCommand::StartTraining { conn_id });
        }
        ClientMessage::ExtractRequest => {
            let _ = rt.cmd_tx.send(NetCommand::ExtractRequest { conn_id });
        }
        ClientMessage::SwitchOperator { operator_id } => {
            let _ = rt.cmd_tx.send(NetCommand::SwitchOperator { conn_id, operator_id });
        }
        ClientMessage::Interact { target, choice } => {
            let _ = rt.cmd_tx.send(NetCommand::Interact { conn_id, target, choice });
        }
        ClientMessage::LootTransfer { target, dir, index } => {
            let _ = rt.cmd_tx.send(NetCommand::LootTransfer { conn_id, target, dir, index });
        }
        ClientMessage::Ping { seq } => {
            let _ = rt.cmd_tx.send(NetCommand::Ping { conn_id, seq });
        }
        ClientMessage::PoolSync { region, evicted } => {
            let _ = rt.cmd_tx.send(NetCommand::PoolSync { conn_id, region, evicted });
        }
        ClientMessage::Disconnect => {
            let _ = rt.cmd_tx.send(NetCommand::Disconnect { conn_id });
            return false;
        }
    }
    true
}
