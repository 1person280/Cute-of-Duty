//! 客户端网络装配：连接服务端、起上行/下行两条线程、断线自动重连
//!
//! 服务器权威架构下，本模块只做：连接、转发 bevy 侧上行意图、接收下行消息并分发。
//! **不做任何模拟/校订**。0.11 起线格式为统一固定 64KB 槽帧（见服务端 `net::packet`），
//! 上下行分处**两条阻塞单线程**（[`super::downlink`] / [`super::uplink`]），各持
//! `try_clone` 独立句柄，故上传不阻塞下载。下行分三路：
//! - 资源帧 → 远程对象池（`net::remote`）；
//! - 权威快照 → [`SnapshotBuffer`]（Bevy 场景对账用）；
//! - 其它（握手/事件/延迟回显/撤离回程）→ [`ControlBuffer`]（Bevy 状态机/延迟面板用）。

use std::net::TcpStream;
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use bevy::prelude::Resource;

use cute_of_duty_server::net::packet::{FrameWriter, encode_client};
use cute_of_duty_server::net::protocol::{ClientMessage, EntitySnapshot, ServerMessage};

/// 下行快照通道类型（渲染对账消费）。
pub type SnapshotChannel = std::sync::mpsc::Sender<Vec<EntitySnapshot>>;

/// 握手玩家名（与延迟面板的 test 行呼应）。
const PROFILE_NAME: &str = "test";

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
pub(crate) const PING_INTERVAL: Duration = Duration::from_secs(1);

/// 网络装配循环：连接 → 起上下行双线程 → 等其结束 → 重连。
///
/// 设计动机（Why）：客户端与服务端是两个独立进程，玩家先开客户端是常见操作。若首次连接
/// 失败就永久退出，画面会停在没有本人实体的空场景里（相机无处跟随、WASD 无处上报），
/// 玩家只会看到"无法移动"。因此这里改为无限重连：失败后打印原因、`RECONNECT_INTERVAL`
/// 后重试；重连成功会再收一次握手，`route_control_messages` 以新 `assigned_id` 覆盖本人
/// 实体（幂等），渲染层无需特殊处理。
///
/// 线程模型（owner 拍板）：**上传/下载两条独立线程、双线单线程**，上传不得影响下载；
/// 连接级 `shutdown` 标志与 `try_clone` 句柄共同保证断链时两线程同步退出后重连。
pub fn run_network(
    addr: &str,
    snapshot_tx: SnapshotChannel,
    control_tx: mpsc::Sender<ClientInbound>,
    resource_tx: mpsc::Sender<crate::net::remote::PoolMessage>,
    up_rx: mpsc::Receiver<ClientMessage>,
) {
    // 上行队列跨多次重连存活（放 Arc<Mutex> 内），避免重连时丢失排队意图。
    let up_rx = Arc::new(Mutex::new(up_rx));

    loop {
        // 阶段1：建连（失败则等待后重试；不 panic、不退出线程）。
        let start = Instant::now();
        let stream = match TcpStream::connect(addr) {
            Ok(stream) => stream,
            Err(e) => {
                eprintln!("[网络] 连接服务器失败 {addr}: {e}（{RECONNECT_INTERVAL:?} 后重试；请确认已先启动 cod_server.exe）");
                std::thread::sleep(RECONNECT_INTERVAL);
                continue;
            }
        };
        // TCP_NODELAY：禁用 Nagle 合并，避免上行输入/探测被攒包拖慢往返。
        let _ = stream.set_nodelay(true);
        let read_half = match stream.try_clone() {
            Ok(half) => half,
            Err(e) => {
                eprintln!("[网络] 复制读句柄失败: {e}");
                std::thread::sleep(RECONNECT_INTERVAL);
                continue;
            }
        };

        // 阶段2：先发握手，再起两个线程（读/写各自独立句柄）。
        if send_handshake(&stream).is_err() {
            eprintln!("[网络] 握手发送失败（{RECONNECT_INTERVAL:?} 后重试）");
            std::thread::sleep(RECONNECT_INTERVAL);
            continue;
        }
        println!("已连接服务器 {addr}，等待快照灌入场景...");

        let shutdown = Arc::new(AtomicBool::new(false));
        // Ping 打点：上传线程写入时刻，下载线程收 Pong 折现真实往返。
        let pending = Arc::new(Mutex::new(None::<Instant>));

        let down = std::thread::spawn({
            let snapshot_tx = snapshot_tx.clone();
            let control_tx = control_tx.clone();
            let resource_tx = resource_tx.clone();
            let shutdown = Arc::clone(&shutdown);
            let pending = Arc::clone(&pending);
            move || {
                crate::net::downlink::downlink_loop(
                    read_half,
                    snapshot_tx,
                    control_tx,
                    resource_tx,
                    start,
                    shutdown,
                    pending,
                )
            }
        });
        let up = std::thread::spawn({
            let up_rx = Arc::clone(&up_rx);
            let shutdown = Arc::clone(&shutdown);
            let pending = Arc::clone(&pending);
            move || crate::net::uplink::uplink_loop(stream, up_rx, shutdown, pending)
        });

        // 下行先退出即视为断链：置位 shutdown 让上行随短超时退出，再重连。
        let _ = down.join();
        shutdown.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = up.join();
        eprintln!("[网络] 连接断开（{RECONNECT_INTERVAL:?} 后重连）");
        std::thread::sleep(RECONNECT_INTERVAL);
    }
}

/// 发送握手控制帧（连接起点第一条上行，服务端据此建实体并回握手）。
fn send_handshake(stream: &TcpStream) -> Result<(), String> {
    let mut w = FrameWriter::new();
    encode_client(
        &mut w,
        &ClientMessage::Connect {
            profile: PROFILE_NAME.to_string(),
        },
        0,
    )?;
    std::io::Write::write_all(&mut &*stream, &w.take()).map_err(|e| format!("握手写入失败: {e}"))
}