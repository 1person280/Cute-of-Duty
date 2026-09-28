//! 控制通道上传线程：把 Bevy 侧排队的上行意图编成恒 256B 包写回服务端
//!
//! 设计动机（Why）：上行（移动/瞄准/开火/交互意图）与下行必须**互不阻塞**——同一条 TCP
//! 连接上，若读线程因写阻塞而停摆，画面会卡住。故上行独占一个**阻塞单线程**，持有
//! `try_clone` 得到的独立 socket 句柄（见 [`super::downlink`] 的读句柄）。
//!
//! 组包走服务端同款 [`SendScheduler`]：指令优先、每包至多 7 条 32B 单元。热路径
//! `PlayerInput` 位打包进单单元，故一条移动意图恰占 1/7 包而非旧版的整 64KB。
//!
//! `shutdown` 由下载线程在断链时置位，本线程据此退出（连接级生命周期对齐）。

use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cute_of_duty_server::net::codec::encode_client;
use cute_of_duty_server::net::protocol::ClientMessage;
use cute_of_duty_server::net::scheduler::SendScheduler;

use crate::net::network::PING_INTERVAL;

/// 上传循环：阻塞等待上行队列，组包写回；顺带按间隔发 Ping（延迟面板数据源）。
pub fn uplink_loop(
    mut stream: TcpStream,
    up_rx: Arc<Mutex<Receiver<ClientMessage>>>,
    shutdown: Arc<AtomicBool>,
    pending: Arc<Mutex<Option<Instant>>>,
) {
    let mut sched = SendScheduler::new();
    let mut next_ping = Instant::now() + PING_INTERVAL;
    loop {
        if shutdown.load(Ordering::Relaxed) {
            return;
        }
        // 阻塞等待上行意图；超时只为周期性插入 Ping，不做传输节流。
        let first = {
            let rx = match up_rx.lock() {
                Ok(rx) => rx,
                Err(_) => return,
            };
            rx.recv_timeout(Duration::from_millis(20))
        };
        match first {
            Ok(msg) => push(&mut sched, &msg),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        // 抽干同批已排队意图：多条指令可拼进同一 256B 包，且指令优先于数据。
        {
            let rx = match up_rx.lock() {
                Ok(rx) => rx,
                Err(_) => return,
            };
            while let Ok(more) = rx.try_recv() {
                push(&mut sched, &more);
            }
        }

        let now = Instant::now();
        if now >= next_ping {
            push(&mut sched, &ClientMessage::Ping { seq: 0 });
            if let Ok(mut p) = pending.lock() {
                *p = Some(Instant::now());
            }
            next_ping = now + PING_INTERVAL;
        }

        match sched.flush() {
            Ok(bytes) if !bytes.is_empty() => {
                if std::io::Write::write_all(&mut stream, &bytes).is_err() {
                    shutdown.store(true, Ordering::Relaxed);
                    return;
                }
            }
            Ok(_) => {}
            Err(e) => eprintln!("[网络] 上行组包失败: {e}"),
        }
    }
}

/// 把一条上行消息编为线上形态并投入调度器（编码失败仅告警，不拖垮连接）。
fn push(sched: &mut SendScheduler, msg: &ClientMessage) {
    match encode_client(msg) {
        Ok(wire) => sched.push(wire),
        Err(e) => eprintln!("[网络] 上行编码失败: {e}"),
    }
}
