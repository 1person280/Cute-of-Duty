//! 资源通道下载线程：把服务端下行的恒 4096B 包重组为整份资源落池
//!
//! 设计动机（Why）：0.12 资源走**独立 TCP 连接**、每包恒定 4096B（= 32B 头 + 4064B 负载）。
//! 本线程是独立的**阻塞单线程**，与控制通道（[`super::downlink`]）互不干扰：资源洪峰不会
//! 挤占延迟敏感的指令/快照带宽。
//!
//! 一份资源按 `chunk_index` 跨多包，[`ResourceAssembler`] 拼回整份负载后经
//! [`PoolMessage::Resource`] 投资源池通道（写进 64KB 固定槽位）；`ResourceEnd` 投批次终止
//! 标记令目录就绪。读取带短超时以便周期性检查 `shutdown`（控制线程断链时同步退出）。

use std::io::Read;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use cute_of_duty_server::net::packet::{PacketKind, PacketReader, RES_PACKET_BYTES};
use cute_of_duty_server::net::resource_stream::ResourceAssembler;

use crate::net::remote::{PoolMessage, ResourceFrame};

/// 资源连接读超时：仅供周期性检查 `shutdown`，不做传输节流。
const READ_TIMEOUT: Duration = Duration::from_millis(200);

/// 资源下载循环：阻塞读 4096B 包直到连接关闭、出错或 `shutdown` 置位。
pub fn resource_downlink_loop(
    mut stream: TcpStream,
    resource_tx: std::sync::mpsc::Sender<PoolMessage>,
    shutdown: Arc<AtomicBool>,
) {
    let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
    let mut reader = PacketReader::new(RES_PACKET_BYTES);
    let mut buf = vec![0u8; RES_PACKET_BYTES];
    let mut asm = ResourceAssembler::default();

    loop {
        if shutdown.load(Ordering::Relaxed) {
            return;
        }
        // 手工累积一整包：`read_exact` 遇超时中途报错会丢半包，故按 `read` 续读。
        let mut filled = 0usize;
        while filled < RES_PACKET_BYTES {
            if shutdown.load(Ordering::Relaxed) {
                return;
            }
            match stream.read(&mut buf[filled..]) {
                Ok(0) => return,
                Ok(n) => filled += n,
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    continue
                }
                Err(_) => return,
            }
        }
        reader.feed(&buf);
        loop {
            let next = match reader.next_packet() {
                Ok(next) => next,
                Err(e) => {
                    eprintln!("[网络] 资源包解析失败: {e}");
                    return;
                }
            };
            let Some((header, payload)) = next else {
                break;
            };
            match header.kind {
                PacketKind::Resource => match asm.push(&header, &payload) {
                    Ok(Some(assembled)) => {
                        let frame = ResourceFrame {
                            kind: assembled.kind,
                            key: assembled.key,
                            region: assembled.region,
                            bytes: assembled.payload,
                        };
                        let _ = resource_tx.send(PoolMessage::Resource(frame));
                    }
                    Ok(None) => {}
                    Err(e) => eprintln!("[网络] 资源重组失败: {e}"),
                },
                PacketKind::ResourceEnd => {
                    let _ = resource_tx.send(PoolMessage::BatchEnd);
                }
                // 资源通道只承载资源包；其余忽略。
                _ => {}
            }
        }
    }
}
