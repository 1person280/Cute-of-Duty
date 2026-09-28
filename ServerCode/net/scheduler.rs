//! 控制通道发送调度器（指令优先组包）
//!
//! 设计动机（Why）：0.12 主通道每包恒定 256B = 32B 头 + 7×32B 单元。上行意图/控制消息
//! 是**延迟敏感**的短指令，快照/事件是**吞吐敏感**的长数据流；二者混在一包里会互相拖累。
//! 故本调度器把两类分开排队，并约定：
//!
//! 1. **指令绝对优先**：只要有待发指令，就先行成包（每包最多 [`UNITS_PER_PACKET`] 条）；
//! 2. **数据流分片连续**：一条数据消息的 224B 切片必须连续下发（接收侧 `ChunkAssembler`
//!    按 `continuation` 拼回），故一条数据流开始后先发完它，再开下一条；
//! 3. 指令包可穿插在数据切片之间——指令不经 `ChunkAssembler`，不会破坏数据连续性。
//!
//! 本模块只做"排队 → 出包"，不含裁决；消息编解码见 `codec`。

use std::collections::VecDeque;

use crate::net::codec::WireMessage;
use crate::net::packet::{
    DataKind, PacketHeader, PacketKind, PacketWriter, Region, Unit, MAIN_PAYLOAD_BYTES,
    PACKET_BYTES, UNITS_PER_PACKET, UNIT_BYTES,
};

/// 进行中的一条数据流（按负载区分片续发；切片必须连续）。
struct ActiveData {
    kind: DataKind,
    bytes: Vec<u8>,
    offset: usize,
}

/// 控制通道发送调度器：指令优先，数据随后；每条数据流分片连续。
pub struct SendScheduler {
    /// 已出包数（填包头 `seq`，单调递增）。
    seq: u64,
    /// 待发指令单元队列（FIFO）。
    commands: VecDeque<Unit>,
    /// 待发数据消息队列（按到达顺序）。
    data: VecDeque<(DataKind, Vec<u8>)>,
    /// 正在分片下发的数据消息。
    active: Option<ActiveData>,
}

impl Default for SendScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl SendScheduler {
    /// 新建空调度器。
    pub fn new() -> Self {
        Self { seq: 0, commands: VecDeque::new(), data: VecDeque::new(), active: None }
    }

    /// 投入一条已编码的线上消息（指令入指令队列，数据入数据队列）。
    pub fn push(&mut self, msg: WireMessage) {
        match msg {
            WireMessage::Command(units) => self.commands.extend(units),
            WireMessage::Data(kind, bytes) => self.data.push_back((kind, bytes)),
        }
    }

    /// 是否还有待发内容（指令/数据/进行中的数据流）。
    pub fn has_pending(&self) -> bool {
        !self.commands.is_empty() || !self.data.is_empty() || self.active.is_some()
    }

    /// 出一包。返回 `Ok(false)` 表示已无内容可发。
    ///
    /// 优先级：指令包 → 进行中的数据切片 → 新数据流首片。一包恰为 [`PACKET_BYTES`]。
    pub fn pump_once(&mut self, w: &mut PacketWriter) -> Result<bool, String> {
        if self.emit_command(w)? {
            return Ok(true);
        }
        self.emit_data(w)
    }

    /// 抽干当前全部待发包（一次 `write_all` 写完）。
    pub fn flush(&mut self) -> Result<Vec<u8>, String> {
        let mut w = PacketWriter::new(PACKET_BYTES);
        while self.pump_once(&mut w)? {}
        Ok(w.take())
    }

    /// 出一条指令包（最多 7 单元）；无指令返回 `Ok(false)`。
    fn emit_command(&mut self, w: &mut PacketWriter) -> Result<bool, String> {
        if self.commands.is_empty() {
            return Ok(false);
        }
        let n = self.commands.len().min(UNITS_PER_PACKET);
        let mut payload = Vec::with_capacity(n * UNIT_BYTES);
        for _ in 0..n {
            if let Some(unit) = self.commands.pop_front() {
                payload.extend_from_slice(&unit);
            }
        }
        self.seq += 1;
        w.push(
            PacketHeader {
                kind: PacketKind::Command,
                continuation: false,
                unit_count: n as u8,
                sub_kind: 0,
                region: Region::InUse,
                chunk_index: 0,
                payload_len: 0,
                seq: self.seq,
                key: 0,
            },
            &payload,
        )?;
        Ok(true)
    }

    /// 出一条数据切片（必要时开启下一条数据流）；无数据返回 `Ok(false)`。
    fn emit_data(&mut self, w: &mut PacketWriter) -> Result<bool, String> {
        if self.active.is_none() {
            let Some((kind, bytes)) = self.data.pop_front() else {
                return Ok(false);
            };
            self.active = Some(ActiveData { kind, bytes, offset: 0 });
        }
        // 先取出本片字节与标志，结束对 `self.active` 的借用后再自增 seq / 写包。
        let (chunk, continuation, sub_kind) = {
            let Some(active) = self.active.as_mut() else {
                return Ok(false);
            };
            let end = (active.offset + MAIN_PAYLOAD_BYTES).min(active.bytes.len());
            let continuation = end < active.bytes.len();
            let chunk = active.bytes[active.offset..end].to_vec();
            active.offset = end;
            (chunk, continuation, active.kind as u8)
        };
        if !continuation {
            self.active = None;
        }
        self.seq += 1;
        w.push(
            PacketHeader {
                kind: PacketKind::Data,
                continuation,
                unit_count: 0,
                sub_kind,
                region: Region::InUse,
                chunk_index: 0,
                payload_len: 0,
                seq: self.seq,
                key: 0,
            },
            &chunk,
        )?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::packet::{ChunkAssembler, PacketReader};

    /// 指令优先：先攒一堆数据，再塞一条指令，首包必为指令包。
    #[test]
    fn commands_are_emitted_before_data() {
        let mut s = SendScheduler::new();
        s.push(WireMessage::Data(DataKind::Snapshot, vec![7u8; 500]));
        s.push(WireMessage::Command(vec![[1u8; UNIT_BYTES]]));
        let mut w = PacketWriter::new(PACKET_BYTES);
        assert!(s.pump_once(&mut w).expect("应出包"));
        let bytes = w.take();
        let mut r = PacketReader::new(PACKET_BYTES);
        r.feed(&bytes);
        let (header, _) = r.next_packet().expect("读包应成功").expect("应有一包");
        assert_eq!(header.kind, PacketKind::Command, "指令应优先出包");
    }

    /// 一包最多 7 条指令；多余的进下一包。
    #[test]
    fn command_packet_holds_at_most_seven_units() {
        let mut s = SendScheduler::new();
        s.push(WireMessage::Command(vec![[2u8; UNIT_BYTES]; 10]));
        let out = s.flush().expect("应出包");
        assert_eq!(out.len(), 2 * PACKET_BYTES, "10 条指令应切成两包");
        let mut r = PacketReader::new(PACKET_BYTES);
        r.feed(&out);
        let (h1, _) = r.next_packet().expect("读包应成功").expect("应有第一包");
        let (h2, _) = r.next_packet().expect("读包应成功").expect("应有第二包");
        assert_eq!(h1.unit_count, 7);
        assert_eq!(h2.unit_count, 3);
    }

    /// 数据流按 224B 分片：500B → 3 片，前两片 continuation、末片收尾。
    #[test]
    fn data_stream_is_chunked_and_contiguous() {
        let mut s = SendScheduler::new();
        s.push(WireMessage::Data(DataKind::Event, vec![9u8; 500]));
        let out = s.flush().expect("应出包");
        assert_eq!(out.len(), 3 * PACKET_BYTES, "500B 应切成 3 片");

        let mut r = PacketReader::new(PACKET_BYTES);
        r.feed(&out);
        let mut asm = ChunkAssembler::default();
        let mut flags = Vec::new();
        while let Some((header, payload)) = r.next_packet().expect("读包应成功") {
            flags.push(header.continuation);
            if let Some(full) = asm.push(header.continuation, &payload) {
                assert_eq!(full, vec![9u8; 500], "重组应与原文一致");
            }
        }
        assert_eq!(flags, vec![true, true, false]);
    }

    /// 空调度器不出包。
    #[test]
    fn empty_scheduler_emits_nothing() {
        let mut s = SendScheduler::new();
        assert!(!s.has_pending());
        assert!(s.flush().expect("应成功").is_empty());
    }
}
