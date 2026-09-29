//! 会话运行时（传输无关核心）
//!
//! 设计动机（Why）：0.12 的线格式只有一套（256B 控制包 / 4096B 资源包），但承载它的
//! 传输不止一种——原生客户端走 TCP 双连接，浏览器走 WebSocket 单连接。若把"协议解析 +
//! 命令分发"焊死在 TCP 读写循环里，第二条传输就只能复制一遍。故本模块抽出**传输无关**
//! 的三件事，TCP（[`super::session`]）与 WebSocket（`super::web::bridge`）共用：
//!
//! 1. [`NetRuntime`]：连接注册表 + 两条通道的写队列（`conn_id → tx`）+ 下行投递
//!    （[`NetRuntime::send_to`] / [`NetRuntime::send_to_all`]）；
//! 2. [`ChannelHandles`] / [`NetRuntime::open_transport`]：让任意传输层登记一对上下行端点，
//!    无需经过 TCP；
//! 3. [`ControlSession`]：把一段上行字节喂进去，它负责按 256B 定长包解析、解码、投递
//!    [`NetCommand`]——TCP 与 WebSocket 都只是"把收到的字节喂给它"。
//!
//! 本模块只做连接编解码与分发，不含任何业务裁决（那些在服务端主循环 `main.rs` 里）。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;

use crate::net::codec::{decode_client_data, decode_client_units, units_from_payload};
use crate::net::packet::{
    ChunkAssembler, PacketHeader, PacketKind, PacketReader, Region, PACKET_BYTES,
};
use crate::net::protocol::{ClientMessage, InventoryAction, PlayerInput, ServerMessage};
use crate::net::resource_stream::ResourceMessage;

// 跨域值类型直取契约 crate，不经领域模块网关（避免"基础设施层→领域层"的反向依赖观感）。
use cute_of_duty_contract::interact::InteractChoice;
use cute_of_duty_contract::items::TransferDir;

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

/// 资源下发任务（服务端 → 资源通道写任务）。
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

/// 一条非 TCP 传输握在手里的两条下行端点（供 WebSocket 桥复用）。
///
/// 上行不经此处：调用方把收到的字节喂给 [`ControlSession::feed`] 即可。
pub struct ChannelHandles {
    /// 分配到的连接 id（与 TCP 连接同一命名空间）。
    pub conn_id: u64,
    /// 控制通道路（256B 包）下行消息。
    pub control_rx: mpsc::UnboundedReceiver<ServerMessage>,
    /// 资源通道路（4096B 包）下行任务。
    pub resource_rx: mpsc::UnboundedReceiver<ResourceJob>,
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
    pub(crate) fn bind_profile(&self, name: &str, conn_id: u64) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.profile_to_control.insert(name.to_string(), conn_id);
        }
    }

    /// 按档案名查控制连接 id（资源连接绑定用）。
    pub(crate) fn control_id_for_profile(&self, name: &str) -> Option<u64> {
        self.shared.lock().ok()?.profile_to_control.get(name).copied()
    }

    /// 注册控制通道写队列。
    pub(crate) fn register_control_writer(&self, conn_id: u64, tx: mpsc::UnboundedSender<ServerMessage>) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.control_writers.insert(conn_id, tx);
        }
    }

    /// 控制连接结束：移除写队列（丢弃发送端即令写任务收尾）。
    pub(crate) fn remove_control_writer(&self, conn_id: u64) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.control_writers.remove(&conn_id);
        }
    }

    /// 注册资源通道写队列，并把此前攒下的资源任务补齐。
    pub(crate) fn register_resource_writer(&self, conn_id: u64, tx: mpsc::UnboundedSender<ResourceJob>) {
        let Ok(mut shared) = self.shared.lock() else { return };
        if let Some(pending) = shared.pending_resources.remove(&conn_id) {
            for job in pending {
                let _ = tx.send(job);
            }
        }
        shared.resource_writers.insert(conn_id, tx);
    }

    /// 资源连接结束：移除写队列。
    pub(crate) fn remove_resource_writer(&self, conn_id: u64) {
        if let Ok(mut shared) = self.shared.lock() {
            shared.resource_writers.remove(&conn_id);
        }
    }

    /// 为一条 TCP 控制连接登记上行/下行端点（供 [`super::session`] 复用）。
    ///
    /// 与 [`NetRuntime::open_transport`] 的差别：TCP 控制连接之后会另开一条资源连接，
    /// 故此处**只**注册控制写队列；资源写队列由 `handle_resource` 稍后注册。
    pub(crate) fn open_control(
        &self,
        name: &str,
    ) -> (u64, mpsc::UnboundedReceiver<ServerMessage>) {
        let conn_id = self.next_conn_id();
        self.bind_profile(name, conn_id);
        let (control_tx, control_rx) = mpsc::unbounded_channel();
        self.register_control_writer(conn_id, control_tx);
        let _ = self.cmd_tx.send(NetCommand::Connect { conn_id, name: name.to_string() });
        (conn_id, control_rx)
    }

    /// 为非 TCP 传输登记一对上下行端点（WebSocket 游玩的接入点）。
    ///
    /// 语义（Why）：与 TCP `handle_control` 完全一致——分配 conn_id、绑定档案名、注册控制+
    /// 资源两条写队列、投递 `Connect`（主循环据此建权威实体并回 `Handshake` + `ModelCatalog`）。
    /// 差别只在于：下行不经 socket 写，而是交回两条 `rx` 由调用方（WS 桥）自行编码发送。
    /// 资源写队列先注册，故 `ModelCatalog` 不会落入 `pending_resources` 而丢失。
    pub fn open_transport(&self, name: &str) -> ChannelHandles {
        let conn_id = self.next_conn_id();
        self.bind_profile(name, conn_id);

        let (control_tx, control_rx) = mpsc::unbounded_channel();
        self.register_control_writer(conn_id, control_tx);
        let (resource_tx, resource_rx) = mpsc::unbounded_channel();
        self.register_resource_writer(conn_id, resource_tx);

        let _ = self.cmd_tx.send(NetCommand::Connect { conn_id, name: name.to_string() });
        ChannelHandles { conn_id, control_rx, resource_rx }
    }

    /// 结束一条非 TCP 传输：投递 `Disconnect` 并移除两条写队列（供运维 API「踢人」复用）。
    pub fn close_transport(&self, conn_id: u64) {
        let _ = self.cmd_tx.send(NetCommand::Disconnect { conn_id });
        self.remove_control_writer(conn_id);
        self.remove_resource_writer(conn_id);
    }

    /// 仅投递 `Disconnect` 到主循环（不触碰写队列）。
    ///
    /// 供 TCP 控制连接读循环结束时调用：写队列的清理由调用方各自负责，
    /// 以免与 `remove_control_writer` 的时序耦合。
    pub(crate) fn notify_disconnect(&self, conn_id: u64) {
        let _ = self.cmd_tx.send(NetCommand::Disconnect { conn_id });
    }

    /// 为该连接构造一个上行解析会话（TCP / WebSocket 共用）。
    pub fn control_session(&self, conn_id: u64) -> ControlSession {
        ControlSession::new(conn_id)
    }

    /// 向单个连接投递服务端消息。
    ///
    /// `ModelCatalog` 归资源通道（4096B 包）；资源连接未绑定时先攒着，绑定后自动补齐。
    pub fn send_to(&self, conn_id: u64, msg: ServerMessage) {
        match msg {
            ServerMessage::ModelCatalog { models, animations } => {
                let items = match crate::net::resource_stream::split_catalog(
                    &models,
                    &animations,
                    Region::InUse,
                ) {
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

    /// 当前在线控制连接数（门户/运维 API 的状态展示）。
    pub fn online_count(&self) -> usize {
        self.shared.lock().map(|s| s.control_writers.len()).unwrap_or(0)
    }

    /// 当前在线连接的 conn_id 列表（运维 API 用；顺序不保证）。
    pub fn online_ids(&self) -> Vec<u64> {
        self.shared
            .lock()
            .map(|s| s.control_writers.keys().copied().collect())
            .unwrap_or_default()
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

/// 传输无关的上行解析会话：把收到的字节喂进 [`ControlSession::feed`]，
/// 它按恒 256B 包解析并投递 [`NetCommand`]。
pub struct ControlSession {
    conn_id: u64,
    reader: PacketReader,
    asm: ChunkAssembler,
}

impl ControlSession {
    /// 按连接 id 构造（包长恒为控制通道 [`PACKET_BYTES`]）。
    pub fn new(conn_id: u64) -> Self {
        Self {
            conn_id,
            reader: PacketReader::new(PACKET_BYTES),
            asm: ChunkAssembler::default(),
        }
    }

    /// 投入一段上行字节，处理其中所有已到达的完整包。
    ///
    /// 返回 `Ok(false)` 表示客户端主动请求断开（收到 `Disconnect`），调用方应结束连接。
    pub fn feed(&mut self, rt: &NetRuntime, bytes: &[u8]) -> Result<bool, String> {
        self.reader.feed(bytes);
        loop {
            match self.reader.next_packet() {
                Ok(Some((header, payload))) => {
                    if !handle_client_packet(rt, self.conn_id, &header, &payload, &mut self.asm)? {
                        return Ok(false);
                    }
                }
                Ok(None) => return Ok(true),
                Err(e) => return Err(e),
            }
        }
    }
}

/// 处理一个上行包；`Ok(false)` 表示客户端主动断开、连接应结束。
pub(crate) fn handle_client_packet(
    rt: &NetRuntime,
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
fn dispatch_client_message(rt: &NetRuntime, conn_id: u64, msg: ClientMessage) -> bool {
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