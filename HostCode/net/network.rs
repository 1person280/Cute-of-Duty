//! 客户端网络表现层：连接服务端、转发上行意图、拉取权威快照/事件灌入通道
//!
//! 服务器权威架构下，本模块只做：连接、转发 bevy 侧上行意图、接收下行消息并分发。
//! **不做任何模拟/校订**。下行分两路：
//! - 权威快照 `Vec<EntitySnapshot>` → `snapshot_tx`（Bevy 场景对账用）；
//! - 其它（握手/事件/延迟回显/撤离回程）→ `control_tx`（Bevy 状态机/延迟面板用）。

use std::sync::mpsc;
use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use bevy::prelude::Resource;

use cute_of_duty_server::net::protocol::{ClientMessage, EntitySnapshot, ServerMessage};

/// 客户端会话：持有已建立的连接与读写缓冲。
pub struct Session {
    reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: tokio::net::tcp::OwnedWriteHalf,
}

impl Session {
    /// 连接服务器并发起握手（当前统一用 "test" 玩家名，与延迟面板的 test 行呼应）。
    pub async fn connect(addr: &str) -> Result<Self, String> {
        let stream = tokio::net::TcpStream::connect(addr)
            .await
            .map_err(|e| format!("连接服务器失败 {addr}: {e}"))?;
        // TCP_NODELAY：NDJSON 是一行一小帧，禁用 Nagle 合并，避免上行输入/探测被攒包拖慢往返。
        let _ = stream.set_nodelay(true);
        let (reader, writer) = stream.into_split();
        let mut session = Self {
            reader: BufReader::new(reader),
            writer,
        };
        session
            .send(&ClientMessage::Connect {
                profile: "test".to_string(),
            })
            .await?;
        Ok(session)
    }

    /// 接收并解析一行服务端消息。
    pub async fn recv(&mut self) -> Result<ServerMessage, String> {
        let mut line = String::new();
        let n = self
            .reader
            .read_line(&mut line)
            .await
            .map_err(|e| format!("读取服务器消息失败: {e}"))?;
        if n == 0 {
            return Err("服务器已关闭连接".to_string());
        }
        serde_json::from_str(line.trim_end()).map_err(|e| format!("解析服务器消息失败: {e}"))
    }

    /// 发送一条上行消息（JSON 行帧）。
    pub async fn send(&mut self, msg: &ClientMessage) -> Result<(), String> {
        let mut line = serde_json::to_string(msg).map_err(|e| format!("序列化客户端消息失败: {e}"))?;
        line.push('\n');
        self.writer
            .write_all(line.as_bytes())
            .await
            .map_err(|e| format!("发送到服务器失败: {e}"))
    }
}

/// 非快照的下行事件（供 Bevy 状态机/延迟面板消费）。
///
/// 用独立的客户端侧枚举而非裸 `ServerMessage`，是为了在**网络线程**里精确测量
/// 「连接起点 → 收到握手」的耗时（含 TCP 三次握手 + 首轮往返），Bevy 侧只消费结果。
#[derive(Debug)]
pub enum ClientInbound {
    /// 连接建立并收到握手：`assigned_id` 为本人权威实体 ID，`connect_ms` 含握手往返耗时。
    Connected { assigned_id: u64, connect_ms: f32 },
    /// 常态 Ping/Pong 往返延迟（毫秒）。由**网络线程**打点测量，不含 Bevy 帧时间。
    Rtt(f32),
    /// 其它服务端下行消息（事件/延迟回显/撤离回程）。
    Server(ServerMessage),
}

/// 下行控制通道（`Receiver` 非 Sync，用 `Mutex` 包裹以作为 Bevy 资源）。
#[derive(Resource)]
pub struct ControlBuffer(
    pub std::sync::Mutex<std::sync::mpsc::Receiver<ClientInbound>>,
);

/// 上行意图通道：Bevy 系统据此把 `ClientMessage`（移动/选装/进场/撤离/延迟探测）发给网络线程。
#[derive(Resource, Clone)]
pub struct NetOut(pub std::sync::mpsc::Sender<ClientMessage>);

/// 重连间隔：连接失败/掉线后等待多久再试（避免忙轮询，也给用户留出启服务端的时间）。
const RECONNECT_INTERVAL: Duration = Duration::from_secs(2);

/// 常态 RTT 探测间隔（秒）。
const PING_INTERVAL: Duration = Duration::from_secs(1);

/// 后台网络拉取循环：连接、转发上行、把下行分路推入通道，**断线自动重连**。
///
/// 设计动机（Why）：客户端与服务端是两个独立进程，玩家先开客户端是常见操作。若首次连接
/// 失败就永久退出，画面会停在没有本人实体的空场景里（相机无处跟随、WASD 无处上报），
/// 玩家只会看到"无法移动"。因此这里改为无限重连：失败后打印原因、`RECONNECT_INTERVAL`
/// 后重试；重连成功会再收一次握手，`route_control_messages` 以新 `assigned_id` 覆盖本人
/// 实体（幂等），渲染层无需特殊处理。
pub fn run_pull_loop(
    addr: &str,
    snapshot_tx: mpsc::Sender<Vec<EntitySnapshot>>,
    control_tx: mpsc::Sender<ClientInbound>,
    up_rx: mpsc::Receiver<ClientMessage>,
) {
    let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("[网络] 创建运行时失败: {e}");
            return;
        }
    };

    rt.block_on(async move {
        loop {
            // 阶段1：建连（失败则等待后重试；不 panic、不退出线程）。
            let start = Instant::now();
            let mut session = match Session::connect(addr).await {
                Ok(session) => session,
                Err(e) => {
                    eprintln!("[网络] {e}（{RECONNECT_INTERVAL:?} 后重试；请确认已先启动 cod_server.exe）");
                    tokio::time::sleep(RECONNECT_INTERVAL).await;
                    continue;
                }
            };
            println!("已连接服务器 {addr}，等待快照灌入场景...");

            // 常态 RTT 探测全部在**网络线程**打点：`pending` 记录 Ping 真正写入 socket 的时刻，
            // 收到 Pong 折现。这样面板显示的是真实链路往返，绝不会把 Bevy 帧时间算进去
            // （此前在 Bevy 帧内打点，低帧率时会把 ~5ms 的网络往返放大成几百 ms 的假延迟）。
            let mut ping_seq: u64 = 0;
            let mut next_ping = Instant::now() + PING_INTERVAL;
            let mut pending: Option<Instant> = None;

            // 阶段2：会话循环；任何收发错误都退到外层重连（而非终止线程）。
            loop {
                // 1) 转发 bevy 侧排队的上行意图（移动/选装/进场/撤离）
                while let Ok(msg) = up_rx.try_recv() {
                    if let Err(e) = session.send(&msg).await {
                        eprintln!("[网络] {e}");
                        break;
                    }
                }

                // 2) 定时 Ping（探测间隔到了就发；快照 60Hz 驱动本循环，实际抖动 < 1 帧）
                let now = Instant::now();
                if now >= next_ping {
                    ping_seq += 1;
                    if session
                        .send(&ClientMessage::Ping { seq: ping_seq })
                        .await
                        .is_ok()
                    {
                        pending = Some(Instant::now());
                    }
                    next_ping = now + PING_INTERVAL;
                }

                // 3) 接收一行下行并按类型分路
                match session.recv().await {
                    Ok(ServerMessage::Snapshot { entries, .. }) => {
                        let _ = snapshot_tx.send(entries);
                    }
                    Ok(ServerMessage::Pong { .. }) => {
                        if let Some(start) = pending.take() {
                            let _ = control_tx.send(ClientInbound::Rtt(
                                start.elapsed().as_secs_f32() * 1000.0,
                            ));
                        }
                    }
                    Ok(ServerMessage::Handshake { assigned_id, .. }) => {
                        let connect_ms = start.elapsed().as_secs_f32() * 1000.0;
                        let _ = control_tx.send(ClientInbound::Connected {
                            assigned_id,
                            connect_ms,
                        });
                    }
                    Ok(other) => {
                        let _ = control_tx.send(ClientInbound::Server(other));
                    }
                    Err(e) => {
                        eprintln!("[网络] {e}（{RECONNECT_INTERVAL:?} 后重连）");
                        break;
                    }
                }

                // 避免忙轮询
                std::thread::sleep(Duration::from_millis(1));
            }

            tokio::time::sleep(RECONNECT_INTERVAL).await;
        }
    });
}