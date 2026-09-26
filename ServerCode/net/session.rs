//! 连接会话与会话运行时管理
//!
//! 服务器权威架构的网络骨架：
//! - `accept_loop` 持续接受连接，每个连接拆成独立的读任务与写任务（tokio task）；
//! - 读任务解析客户端上行意图（`ClientMessage`）→ 投递到 `NetCommand` 命令队列，
//!   供服务端权威主循环（拥有模拟世界）在每个固定 Tick 消费并应用到模拟；
//! - 写任务从各自的 `UnboundedSender` 通道取服务端编好的权威快照写回客户端。
//!
//! 计算永远发生在服务端主循环；本模块只负责连接的建立、收发与扇出。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use crate::net::protocol::{ClientMessage, InventoryAction, PlayerInput, ServerMessage};

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
    /// 延迟探测（原样回显 Pong）
    Ping { conn_id: u64, seq: u64 },
    /// 连接断开（主动 Disconnect 或对端关闭/异常）
    Disconnect { conn_id: u64 },
}

/// 会话运行时：持有命令队列发送端 + 各连接写任务通道表。
///
/// `cmd_rx` 以 `Option` 封装，供服务端权威主循环调用 `take_command_receiver`
/// 一次性取走接收端；`writers` 用内部锁并保持短临界区（锁内不放 await），
/// 因此可在 tokio 任务间安全共享。
pub struct NetRuntime {
    cmd_tx: mpsc::UnboundedSender<NetCommand>,
    cmd_rx: Option<mpsc::UnboundedReceiver<NetCommand>>,
    writers: Arc<Mutex<HashMap<u64, mpsc::UnboundedSender<ServerMessage>>>>,
    next_id: AtomicU64,
}

impl NetRuntime {
    /// 创建会话运行时。
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        Self {
            cmd_tx,
            cmd_rx: Some(cmd_rx),
            writers: Arc::new(Mutex::new(HashMap::new())),
            next_id: AtomicU64::new(100),
        }
    }

    /// 取走命令队列接收端（仅由服务端权威主循环调用一次）。
    pub fn take_command_receiver(&mut self) -> mpsc::UnboundedReceiver<NetCommand> {
        self.cmd_rx
            .take()
            .expect("命令队列接收端只能被取走一次")
    }

    /// 注册某连接对应的写任务通道。
    fn register_writer(&self, conn_id: u64, tx: mpsc::UnboundedSender<ServerMessage>) {
        self.writers.lock().unwrap().insert(conn_id, tx);
    }

    /// 写任务结束后移除该连接通道，避免死写与无效投递。
    fn remove_writer(&self, conn_id: u64) {
        self.writers.lock().unwrap().remove(&conn_id);
    }

    /// 向单个连接投递权威快照（通道已关则忽略）。
    pub fn send_to(&self, conn_id: u64, msg: ServerMessage) {
        if let Some(tx) = self.writers.lock().unwrap().get(&conn_id) {
            let _ = tx.send(msg);
        }
    }

    /// 向所有在线连接扇出权威快照。
    pub fn send_to_all(&self, msg: ServerMessage) {
        let writers = self.writers.lock().unwrap();
        for tx in writers.values() {
            let _ = tx.send(msg.clone());
        }
    }
}

impl Default for NetRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// 接受连接主循环：每来一个连接即分配 ID 并展开读/写两个任务。
pub async fn accept_loop(addr: String, rt: Arc<NetRuntime>) -> std::io::Result<()> {
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("服务器权威已监听: {addr}");
    loop {
        let (stream, peer) = listener.accept().await?;
        // TCP_NODELAY：NDJSON 是许多小帧，禁用 Nagle 合并避免人为攒包拖慢往返（延迟面板可见）。
        let _ = stream.set_nodelay(true);
        let conn_id = rt.next_id.fetch_add(1, Ordering::Relaxed);
        tracing::info!("新连接 #{conn_id} 来自 {peer}");

        let (reader, writer) = stream.into_split();

        // 写任务：消费本连接的权威快照通道，逐行写回客户端
        let (tx, rx) = mpsc::unbounded_channel();
        rt.register_writer(conn_id, tx);
        let rt_w = Arc::clone(&rt);
        tokio::spawn(async move {
            write_loop(writer, rx).await;
            rt_w.remove_writer(conn_id);
        });

        // 读任务：解析客户端上行意图并投递到命令队列
        let rt_r = Arc::clone(&rt);
        tokio::spawn(async move {
            read_loop(reader, conn_id, rt_r).await;
        });
    }
}

/// 写回循环：将服务端权威快照逐行写入该连接的 TCP 流。
async fn write_loop(
    mut writer: OwnedWriteHalf,
    mut rx: mpsc::UnboundedReceiver<ServerMessage>,
) {
    while let Some(msg) = rx.recv().await {
        if writer.write_all(msg.to_line().as_bytes()).await.is_err() {
            // 对端关闭：结束写任务，通道由 remove_writer 清理
            break;
        }
    }
}

/// 读取循环：逐行解析客户端消息并投递为 `NetCommand`。
async fn read_loop(mut reader: OwnedReadHalf, conn_id: u64, rt: Arc<NetRuntime>) {
    let mut buf = BufReader::new(&mut reader);
    let mut line = String::new();
    loop {
        line.clear();
        match buf.read_line(&mut line).await {
            Ok(0) | Err(_) => {
                // EOF 或读错 → 视为断开
                let _ = rt.cmd_tx.send(NetCommand::Disconnect { conn_id });
                break;
            }
            Ok(_) => {
                let msg = match ClientMessage::from_line(line.trim_end()) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
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
                    ClientMessage::Ping { seq } => {
                        let _ = rt.cmd_tx.send(NetCommand::Ping { conn_id, seq });
                    }
                    ClientMessage::Disconnect => {
                        let _ = rt.cmd_tx.send(NetCommand::Disconnect { conn_id });
                        break;
                    }
                }
            }
        }
    }
}