//! 客户端网络表现层：连接服务端、上发输入意图、拉取权威快照灌入通道
//!
//! 服务器权威架构下，本模块只做三件事：连接、发输入、拉快照。**不做任何模拟/校订**，
//! 把服务端算好的权威快照经 mpsc 推给 Bevy 表现主循环去画。

use std::sync::mpsc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use cute_of_duty_server::net::protocol::{ClientMessage, EntitySnapshot, PlayerInput, ServerMessage};

/// 客户端会话：持有已建立的连接与读取缓冲。
pub struct Session {
    reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: tokio::net::tcp::OwnedWriteHalf,
}

impl Session {
    /// 连接服务器并发起握手（携带玩家档案名）。
    pub async fn connect(addr: &str, profile: &str) -> Result<Self, String> {
        let stream = tokio::net::TcpStream::connect(addr)
            .await
            .map_err(|e| format!("连接服务器失败 {addr}: {e}"))?;
        let (reader, writer) = stream.into_split();
        let mut session = Self {
            reader: BufReader::new(reader),
            writer,
        };
        session
            .send(&ClientMessage::Connect {
                profile: profile.to_string(),
            })
            .await?;
        Ok(session)
    }

    /// 发送一帧客户端输入意图。
    pub async fn send_input(&mut self, input: PlayerInput) -> Result<(), String> {
        self.send(&ClientMessage::Input { player: input }).await
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
    async fn send(&mut self, msg: &ClientMessage) -> Result<(), String> {
        let mut line = serde_json::to_string(msg).map_err(|e| format!("序列化客户端消息失败: {e}"))?;
        line.push('\n');
        self.writer
            .write_all(line.as_bytes())
            .await
            .map_err(|e| format!("发送到服务器失败: {e}"))
    }
}

/// 后台网络拉取循环：连接、上发输入、把最新权威快照推入通道。
///
/// 连接失败或意外断线时打印原因后静默退出 —— 此时渲染层仍能展示空场景，
/// 保证"只剩一个静止画面"而非直接崩溃，便于与本机服务端分开排查。
/// 演示运动：让玩家在 X 轴往复，以肉眼证明"服务端权威回传"驱动的位移。
pub fn run_pull_loop(addr: &str, snapshot_tx: mpsc::Sender<Vec<EntitySnapshot>>) {
    let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("[网络] 创建运行时失败: {e}");
            return;
        }
    };

    if let Err(e) = rt.block_on(async move {
        let mut session = Session::connect(addr, "元素大师").await?;
        println!("已连接服务器，等待快照灌入场景...");

        let mut input_seq = 0u64;
        loop {
            input_seq += 1;
            let move_right = input_seq / 30 % 2 == 0;
            session
                .send_input(PlayerInput {
                    seq: input_seq,
                    move_right,
                    ..Default::default()
                })
                .await?;

            match session.recv().await? {
                ServerMessage::Snapshot { entries, .. } => {
                    let _ = snapshot_tx.send(entries);
                }
                _ => {}
            }
        }
        #[allow(unreachable_code)]
        Ok::<(), String>(())
    }) {
        eprintln!("[网络] {e}");
    }
}