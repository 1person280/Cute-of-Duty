//! 上传线程：把 Bevy 侧排队的上行意图编成 64KB 槽帧并写回服务端
//!
//! 设计动机（Why）：上行（移动/瞄准/开火/交互意图）与下行必须**互不阻塞**——同一条 TCP
//! 连接上，若读线程因写阻塞而停摆，画面会卡住。故上行独占一个 **阻塞单线程**，持有
//! `try_clone` 得到的独立 socket 句柄（见 [`super::downlink`] 的读句柄）。写完整帧即取
//! 下一帧，**不为传输人为计时**；仅以极短 `recv_timeout` 兼顾常态 Ping 探测。
//!
//! `shutdown` 由下载线程在断链时置位，本线程据此退出（连接级生命周期对齐）。

use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cute_of_duty_server::net::packet::{FrameWriter, encode_client};
use cute_of_duty_server::net::protocol::ClientMessage;

use crate::net::network::PING_INTERVAL;

/// 上传循环：阻塞等待上行队列，写帧；顺带按间隔发 Ping（延迟面板数据源）。
pub fn uplink_loop(
    mut stream: TcpStream,
    up_rx: Arc<Mutex<Receiver<ClientMessage>>>,
    shutdown: Arc<AtomicBool>,
    pending: Arc<Mutex<Option<Instant>>>,
) {
    let mut seq: u64 = 0;
    let mut next_ping = Instant::now() + PING_INTERVAL;
    loop {
        if shutdown.load(Ordering::Relaxed) {
            return;
        }
        // 阻塞等待上行意图；超时只为周期性插入 Ping，不做传输节流。
        let msg = {
            let rx = match up_rx.lock() {
                Ok(rx) => rx,
                Err(_) => return,
            };
            rx.recv_timeout(Duration::from_millis(20))
        };
        match msg {
            Ok(msg) => {
                seq += 1;
                if write_message(&mut stream, &msg, seq).is_err() {
                    shutdown.store(true, Ordering::Relaxed);
                    return;
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }

        let now = Instant::now();
        if now >= next_ping {
            seq += 1;
            if write_message(&mut stream, &ClientMessage::Ping { seq }, seq).is_ok() {
                if let Ok(mut p) = pending.lock() {
                    *p = Some(Instant::now());
                }
            }
            next_ping = now + PING_INTERVAL;
        }
    }
}

/// 把一条上行消息编成整帧并写全（帧恒为 64KB，写失败即断链）。
fn write_message(stream: &mut TcpStream, msg: &ClientMessage, seq: u64) -> Result<(), ()> {
    let mut w = FrameWriter::new();
    encode_client(&mut w, msg, seq).map_err(|e| eprintln!("[网络] 上行编码失败: {e}"))?;
    std::io::Write::write_all(stream, &w.take()).map_err(|e| eprintln!("[网络] 上行写入失败: {e}"))
}