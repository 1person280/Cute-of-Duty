//! 统一 64KB 槽帧编解码（线格式的传输层，双端共用）
//!
//! 设计动机（Why）：0.10 及以前线格式是 **NDJSON**（每行一条 JSON），资源、快照、事件混在
//! 同一条行流里，且资源是"握手时一次性灌入客户端内存的 Vec"。问题有二：
//! 1. 实体因 AOI 进出视野时，客户端对"没缓存过的造型"只能等下一批数据才能出画；
//! 2. 边长 JSON 行让"资源"与"瞬时消息"无法在接收侧分离到固定位置。
//!
//! 0.11 起统一为**固定 64KB 帧**：每一帧恒定 [`FRAME_BYTES`] 字节、64KB 对齐，
//! 一条资源 = 一帧（尾部零填充对齐），控制指令（~256B）也打包进 64KB 帧；超过单帧容量的
//! 消息（如实体很多的快照）按 [`FrameHeader::continuation`] **跨帧分片**重组。
//! 客户端据此把资源帧写进固定地址槽位池（见 HostCode `net::remote`），实体突现即复用，
//! 无需等待加载。
//!
//! 本模块只负责"字节 ↔ 帧 ↔ 消息"，不含任何业务裁决；资源内容（几何/动画）仍是服务端
//! 权威，客户端只做冷映射渲染。

use serde::{Deserialize, Serialize};

use crate::model::{VoxelAnimationSpec, VoxelModelSpec};
use crate::net::protocol::{ClientMessage, ServerMessage};

/// 一帧恒定字节数（64KB，线上 64KB 对齐）。
pub const FRAME_BYTES: usize = 64 * 1024;
/// 定长帧头字节数。
pub const HEADER_BYTES: usize = 32;
/// 单帧可承载的最大负载字节数。
pub const PAYLOAD_MAX: usize = FRAME_BYTES - HEADER_BYTES;
/// 帧魔数 "COD1"（小端读出 = 0x434F4431）。
pub const MAGIC: u32 = 0x434F_4431;
/// 线格式版本（与协议 y 位对齐；0.11 = 槽帧世代）。
pub const WIRE_VERSION: u16 = 11;

/// 帧类别：决定负载如何被接收侧消费。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameKind {
    /// 控制指令（握手/Pong/撤离回程/全部上行意图）；~256B，打包进 64KB 帧。
    Control = 0,
    /// 每 Tick 权威快照（可跨帧分片）。
    Snapshot = 1,
    /// 瞬时事件（击杀/受击/拾取/通告）。
    Event = 2,
    /// 单份远程资源（一资源一帧，64KB 对齐）。
    Resource = 3,
    /// 一批资源下发完毕（目录批次终止标记）。
    ResourceEnd = 4,
}

impl FrameKind {
    fn from_u8(v: u8) -> Result<Self, String> {
        match v {
            0 => Ok(Self::Control),
            1 => Ok(Self::Snapshot),
            2 => Ok(Self::Event),
            3 => Ok(Self::Resource),
            4 => Ok(Self::ResourceEnd),
            other => Err(format!("未知帧类别: {other}")),
        }
    }
}

/// 资源帧的落区：客户端据此决定写"在用区"还是"预取区"。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Region {
    /// 在用缓冲（已在视野/正被使用）。
    InUse = 0,
    /// 预取区（服务端预测即将进入视野的实体资源）。
    Prefetch = 1,
}

impl Region {
    fn from_u8(v: u8) -> Result<Self, String> {
        match v {
            0 => Ok(Self::InUse),
            1 => Ok(Self::Prefetch),
            other => Err(format!("未知资源区: {other}")),
        }
    }
}

/// 资源子类（仅 [`FrameKind::Resource`] 有意义）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ResourceKind {
    /// 体素模型几何。
    Model = 0,
    /// 体素动画 clip。
    Animation = 1,
}

impl ResourceKind {
    fn from_u8(v: u8) -> Result<Self, String> {
        match v {
            0 => Ok(Self::Model),
            1 => Ok(Self::Animation),
            other => Err(format!("未知资源子类: {other}")),
        }
    }
}

/// 定长 32 字节帧头。
///
/// 布局（offset, width，小端）：
/// `0/4 magic · 4/2 version · 6/1 kind · 7/1 flags · 8/1 region · 9/1 sub_kind ·
/// 10/2 reserved · 12/8 seq · 20/8 key · 28/4 payload_len`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeader {
    pub kind: FrameKind,
    /// `true` = 本帧不是最后一片，后续还有同消息的分片。
    pub continuation: bool,
    pub region: Region,
    /// 资源子类（非资源帧填 0）。
    pub sub_kind: u8,
    /// 帧序号（单调递增，接收侧据此判重/丢旧）。
    pub seq: u64,
    /// 资源/实体键（0 = 无）。
    pub key: u64,
    /// 负载实际字节数（≤ [`PAYLOAD_MAX`]）。
    pub payload_len: u32,
}

impl FrameHeader {
    /// 编码为定长字节数组。
    pub fn encode(&self) -> [u8; HEADER_BYTES] {
        let mut buf = [0u8; HEADER_BYTES];
        buf[0..4].copy_from_slice(&MAGIC.to_le_bytes());
        buf[4..6].copy_from_slice(&WIRE_VERSION.to_le_bytes());
        buf[6] = self.kind as u8;
        buf[7] = self.continuation as u8;
        buf[8] = self.region as u8;
        buf[9] = self.sub_kind;
        // 10..12 reserved
        buf[12..20].copy_from_slice(&self.seq.to_le_bytes());
        buf[20..28].copy_from_slice(&self.key.to_le_bytes());
        buf[28..32].copy_from_slice(&self.payload_len.to_le_bytes());
        buf
    }

    /// 从定长字节数组解码（校验魔数/版本/类别/长度）。
    pub fn decode(buf: &[u8; HEADER_BYTES]) -> Result<Self, String> {
        let magic = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        if magic != MAGIC {
            return Err(format!("帧魔数不符: 0x{magic:08X}"));
        }
        let version = u16::from_le_bytes([buf[4], buf[5]]);
        if version != WIRE_VERSION {
            return Err(format!("线格式版本不符: {version}（期望 {WIRE_VERSION}）"));
        }
        let kind = FrameKind::from_u8(buf[6])?;
        let continuation = buf[7] & 0x01 != 0;
        let region = Region::from_u8(buf[8])?;
        let payload_len = u32::from_le_bytes([buf[28], buf[29], buf[30], buf[31]]);
        if payload_len as usize > PAYLOAD_MAX {
            return Err(format!("负载超长: {payload_len} > {PAYLOAD_MAX}"));
        }
        Ok(Self {
            kind,
            continuation,
            region,
            sub_kind: buf[9],
            seq: u64::from_le_bytes([buf[12], buf[13], buf[14], buf[15], buf[16], buf[17], buf[18], buf[19]]),
            key: u64::from_le_bytes([buf[20], buf[21], buf[22], buf[23], buf[24], buf[25], buf[26], buf[27]]),
            payload_len,
        })
    }
}

/// 帧写出器：把若干帧追加进一段连续缓冲（每帧恒定 64KB，尾部零填充）。
#[derive(Default)]
pub struct FrameWriter {
    out: Vec<u8>,
}

impl FrameWriter {
    pub fn new() -> Self {
        Self { out: Vec::new() }
    }

    /// 追加一帧（负载不足 64KB 时尾部零填充对齐）。
    pub fn push_frame(&mut self, header: FrameHeader, payload: &[u8]) -> Result<(), String> {
        if payload.len() > PAYLOAD_MAX {
            return Err(format!("负载超单帧容量: {} > {PAYLOAD_MAX}", payload.len()));
        }
        let mut h = header;
        h.payload_len = payload.len() as u32;
        self.out.extend_from_slice(&h.encode());
        self.out.extend_from_slice(payload);
        self.out.resize(self.out.len() + (PAYLOAD_MAX - payload.len()), 0);
        Ok(())
    }

    /// 取走已写字节（调用后缓冲清空）。
    pub fn take(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.out)
    }

    pub fn is_empty(&self) -> bool {
        self.out.is_empty()
    }
}

/// 帧读出器：累积 TCP 字节流，逐帧解析。
#[derive(Default)]
pub struct FrameReader {
    buf: Vec<u8>,
    cursor: usize,
}

impl FrameReader {
    pub fn new() -> Self {
        Self { buf: Vec::new(), cursor: 0 }
    }

    /// 投入一段 TCP 收到的字节。
    pub fn feed(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// 取下一帧；不足一帧返回 `Ok(None)`（等待更多字节）。
    pub fn next_frame(&mut self) -> Result<Option<(FrameHeader, Vec<u8>)>, String> {
        if self.buf.len() - self.cursor < FRAME_BYTES {
            self.compact();
            return Ok(None);
        }
        let start = self.cursor;
        let mut hbuf = [0u8; HEADER_BYTES];
        hbuf.copy_from_slice(&self.buf[start..start + HEADER_BYTES]);
        let header = FrameHeader::decode(&hbuf)?;
        let plen = header.payload_len as usize;
        let payload = self.buf[start + HEADER_BYTES..start + HEADER_BYTES + plen].to_vec();
        self.cursor += FRAME_BYTES;
        Ok(Some((header, payload)))
    }

    fn compact(&mut self) {
        if self.cursor > 0 {
            self.buf.drain(..self.cursor);
            self.cursor = 0;
        }
    }
}

/// 分片重组器：把同一消息的连续分片拼回完整负载。
#[derive(Default)]
pub struct ChunkAssembler {
    acc: Vec<u8>,
    active: bool,
}

impl ChunkAssembler {
    /// 投入一帧；若该帧是最后一片则返回完整负载（非最后一片返回 `Ok(None)`）。
    pub fn push(&mut self, header: &FrameHeader, payload: &[u8]) -> Result<Option<Vec<u8>>, String> {
        if header.continuation {
            self.active = true;
            self.acc.extend_from_slice(payload);
            return Ok(None);
        }
        if !self.active {
            // 单片消息：直接返回。
            return Ok(Some(payload.to_vec()));
        }
        self.acc.extend_from_slice(payload);
        self.active = false;
        Ok(Some(std::mem::take(&mut self.acc)))
    }
}

/// 计算资源键（FNV-1a，稳定跨端；0 保留给"无键"，故映射到 1）。
pub fn resource_key(kind: ResourceKind, name: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in (kind as u8).to_le_bytes().iter().chain(name.as_bytes()) {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    if h == 0 { 1 } else { h }
}

/// 体素模型的资源键（造型身份稳定，故用 preset 的调试名）。
pub fn model_key(spec: &VoxelModelSpec) -> u64 {
    resource_key(ResourceKind::Model, &format!("{:?}", spec.preset))
}

/// 体素动画的资源键（按 clip 名）。
pub fn animation_key(spec: &VoxelAnimationSpec) -> u64 {
    resource_key(ResourceKind::Animation, &spec.clip)
}

/// 按负载长度切成若干帧写入（超长自动分片，`continuation` 标记是否还有后续）。
pub fn encode_chunked(
    w: &mut FrameWriter,
    kind: FrameKind,
    region: Region,
    seq: u64,
    key: u64,
    payload: &[u8],
) -> Result<(), String> {
    let mut rest = payload;
    loop {
        let take = rest.len().min(PAYLOAD_MAX);
        let (chunk, tail) = rest.split_at(take);
        let continuation = !tail.is_empty();
        w.push_frame(
            FrameHeader { kind, continuation, region, sub_kind: 0, seq, key, payload_len: 0 },
            chunk,
        )?;
        rest = tail;
        if !continuation {
            break;
        }
    }
    Ok(())
}

/// 编码一条完整资源帧（一资源一帧，超单帧容量直接报错）。
pub fn encode_resource(
    w: &mut FrameWriter,
    kind: ResourceKind,
    key: u64,
    region: Region,
    seq: u64,
    payload: &[u8],
) -> Result<(), String> {
    if payload.len() > PAYLOAD_MAX {
        return Err(format!("资源 {key} 超单帧容量: {} > {PAYLOAD_MAX}", payload.len()));
    }
    w.push_frame(
        FrameHeader {
            kind: FrameKind::Resource,
            continuation: false,
            region,
            sub_kind: kind as u8,
            seq,
            key,
            payload_len: 0,
        },
        payload,
    )
}

/// 编码资源批次终止帧。
pub fn encode_resource_end(w: &mut FrameWriter, seq: u64) -> Result<(), String> {
    w.push_frame(
        FrameHeader {
            kind: FrameKind::ResourceEnd,
            continuation: false,
            region: Region::InUse,
            sub_kind: 0,
            seq,
            key: 0,
            payload_len: 0,
        },
        &[],
    )
}

/// 把一条**服务端下行消息**编码为帧。
///
/// `ModelCatalog` 特殊：按"一资源一帧"拆为若干 [`FrameKind::Resource`] 帧并以
/// [`FrameKind::ResourceEnd`] 收尾（客户端据此写资源池）；其余消息按类别整体可选分片。
pub fn encode_server(w: &mut FrameWriter, msg: &ServerMessage, seq: u64) -> Result<(), String> {
    match msg {
        ServerMessage::ModelCatalog { models, animations } => {
            for m in models {
                let bytes = to_json(m)?;
                encode_resource(w, ResourceKind::Model, model_key(m), Region::InUse, seq, &bytes)?;
            }
            for a in animations {
                let bytes = to_json(a)?;
                encode_resource(w, ResourceKind::Animation, animation_key(a), Region::InUse, seq, &bytes)?;
            }
            encode_resource_end(w, seq)
        }
        ServerMessage::Snapshot { .. } => {
            encode_chunked(w, FrameKind::Snapshot, Region::InUse, seq, 0, &to_json(msg)?)
        }
        ServerMessage::Event { .. } => {
            encode_chunked(w, FrameKind::Event, Region::InUse, seq, 0, &to_json(msg)?)
        }
        // 握手/Pong/撤离回程：小控制帧，打包进 64KB。
        ServerMessage::Handshake { .. } | ServerMessage::Pong { .. } | ServerMessage::ReturnToMenu => {
            encode_chunked(w, FrameKind::Control, Region::InUse, seq, 0, &to_json(msg)?)
        }
    }
}

/// 把一条**客户端上行消息**编码为帧（全部走控制帧，~256B 打包进 64KB）。
pub fn encode_client(w: &mut FrameWriter, msg: &ClientMessage, seq: u64) -> Result<(), String> {
    encode_chunked(w, FrameKind::Control, Region::InUse, seq, 0, &to_json(msg)?)
}

/// 从完整负载解码服务端下行消息（不含资源帧——资源走池，见 `encode_server`）。
pub fn decode_server(payload: &[u8]) -> Result<ServerMessage, String> {
    serde_json::from_slice(payload).map_err(|e| format!("解析服务端消息失败: {e}"))
}

/// 从完整负载解码客户端上行消息。
pub fn decode_client(payload: &[u8]) -> Result<ClientMessage, String> {
    serde_json::from_slice(payload).map_err(|e| format!("解析客户端消息失败: {e}"))
}

/// 解码资源帧负载为体素模型规格。
pub fn decode_model(payload: &[u8]) -> Result<VoxelModelSpec, String> {
    serde_json::from_slice(payload).map_err(|e| format!("解析体素模型失败: {e}"))
}

/// 解码资源帧负载为体素动画规格。
pub fn decode_animation(payload: &[u8]) -> Result<VoxelAnimationSpec, String> {
    serde_json::from_slice(payload).map_err(|e| format!("解析体素动画失败: {e}"))
}

/// 从资源帧头解析子类。
pub fn resource_kind_of(header: &FrameHeader) -> Result<ResourceKind, String> {
    ResourceKind::from_u8(header.sub_kind)
}

/// 由槽位目录里的子类字节解析资源子类（供客户端对象池把槽内容解码为型号/动画）。
///
/// 返回 `None` 表示该字节不是已知子类（畸形/旧版本）。
pub fn resource_kind_from_byte(v: u8) -> Option<ResourceKind> {
    ResourceKind::from_u8(v).ok()
}

/// 序列化辅助（统一错误文案，避免裸 unwrap）。
fn to_json<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    serde_json::to_vec(value).map_err(|e| format!("消息序列化失败: {e}"))
}

/// 便于测试/无头工具构造资源负载。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResourcePayload {
    Model(VoxelModelSpec),
    Animation(VoxelAnimationSpec),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 每帧恒定 64KB，且总字节数为帧数 × 64KB（线上 64KB 对齐）。
    #[test]
    fn frames_are_64kb_aligned() {
        let mut w = FrameWriter::new();
        encode_client(&mut w, &ClientMessage::Ping { seq: 7 }, 1).expect("编码应成功");
        let bytes = w.take();
        assert_eq!(bytes.len(), FRAME_BYTES, "单帧应恰为 64KB");
        assert_eq!(bytes.len() % FRAME_BYTES, 0);
    }

    /// 控制指令（~256B）打包进 64KB 帧后，接收侧仍能原样解码。
    #[test]
    fn small_control_frame_roundtrip() {
        let msg = ClientMessage::Ping { seq: 42 };
        let mut w = FrameWriter::new();
        encode_client(&mut w, &msg, 1).expect("编码应成功");
        let mut r = FrameReader::new();
        r.feed(&w.take());
        let (h, payload) = r.next_frame().expect("读帧应成功").expect("应有一帧");
        assert_eq!(h.kind, FrameKind::Control);
        assert!(!h.continuation);
        assert_eq!(decode_client(&payload).expect("解码应成功"), msg);
        assert!(r.next_frame().expect("读帧应成功").is_none());
    }

    /// 超长消息按 continuation 分片，重组后与原文一致。
    #[test]
    fn oversized_message_is_chunked_and_reassembled() {
        // 构造 >1 帧的伪负载（PAYLOAD_MAX + 5 字节 → 两片）。
        let payload: Vec<u8> = (0..(PAYLOAD_MAX + 5)).map(|i| (i % 251) as u8).collect();
        let mut w = FrameWriter::new();
        encode_chunked(&mut w, FrameKind::Snapshot, Region::InUse, 9, 0, &payload)
            .expect("分片编码应成功");
        let bytes = w.take();
        assert_eq!(bytes.len(), 2 * FRAME_BYTES, "应恰为两帧");

        let mut r = FrameReader::new();
        r.feed(&bytes);
        let mut asm = ChunkAssembler::default();
        let (h1, p1) = r.next_frame().expect("读帧应成功").expect("应有第一片");
        assert!(h1.continuation, "第一片应标记有后续");
        assert!(asm.push(&h1, &p1).expect("重组应成功").is_none());
        let (h2, p2) = r.next_frame().expect("读帧应成功").expect("应有第二片");
        assert!(!h2.continuation, "末片不应标记后续");
        let joined = asm.push(&h2, &p2).expect("重组应成功").expect("应得到完整负载");
        assert_eq!(joined, payload, "重组结果应与原文一致");
    }

    /// 资源帧携带子类/区/键，负载原样保留。
    #[test]
    fn resource_frame_roundtrip() {
        let payload = b"voxel-spec-bytes".to_vec();
        let key = resource_key(ResourceKind::Model, "OperativeFire");
        let mut w = FrameWriter::new();
        encode_resource(&mut w, ResourceKind::Model, key, Region::Prefetch, 3, &payload)
            .expect("资源编码应成功");
        let mut r = FrameReader::new();
        r.feed(&w.take());
        let (h, p) = r.next_frame().expect("读帧应成功").expect("应有一帧");
        assert_eq!(h.kind, FrameKind::Resource);
        assert_eq!(h.region, Region::Prefetch);
        assert_eq!(h.key, key);
        assert_eq!(resource_kind_of(&h).expect("子类应可解"), ResourceKind::Model);
        assert_eq!(p, payload);
        assert_ne!(key, 0, "资源键不应为保留值 0");
    }

    /// 模型目录按"一资源一帧 + 终止帧"下发，全部落在在用区。
    #[test]
    fn model_catalog_expands_to_resource_frames() {
        let msg = ServerMessage::ModelCatalog {
            models: crate::model::catalog(),
            animations: crate::model::animations(),
        };
        let expect_frames = match &msg {
            ServerMessage::ModelCatalog { models, animations } => models.len() + animations.len() + 1,
            _ => unreachable!(),
        };
        let mut w = FrameWriter::new();
        encode_server(&mut w, &msg, 5).expect("目录编码应成功");
        let bytes = w.take();
        assert_eq!(bytes.len(), expect_frames * FRAME_BYTES, "帧数应为 资源数 + 终止帧");

        let mut r = FrameReader::new();
        r.feed(&bytes);
        let mut seen = 0usize;
        let mut terminated = false;
        while let Some((h, p)) = r.next_frame().expect("读帧应成功") {
            match h.kind {
                FrameKind::Resource => {
                    assert_eq!(h.region, Region::InUse);
                    assert!(!p.is_empty(), "资源负载不应为空");
                    seen += 1;
                }
                FrameKind::ResourceEnd => terminated = true,
                other => panic!("不应出现 {other:?} 帧"),
            }
        }
        assert_eq!(seen + 1, expect_frames, "资源帧 + 终止帧 = 总帧数");
        assert!(terminated, "批次应以 ResourceEnd 收尾");
    }

    /// 快照经统一帧通道往返一致（含实体字段）。
    #[test]
    fn snapshot_roundtrip_over_frames() {
        let msg = ServerMessage::Snapshot {
            seq: 11,
            entries: vec![crate::net::protocol::EntitySnapshot::placeholder(7)],
        };
        let mut w = FrameWriter::new();
        encode_server(&mut w, &msg, 11).expect("快照编码应成功");
        let mut r = FrameReader::new();
        r.feed(&w.take());
        let (h, p) = r.next_frame().expect("读帧应成功").expect("应有一帧");
        assert_eq!(h.kind, FrameKind::Snapshot);
        assert_eq!(decode_server(&p).expect("解码应成功"), msg);
    }

    /// 事件帧类别正确、可解码。
    #[test]
    fn event_frame_roundtrip() {
        let msg = ServerMessage::Event {
            kind: crate::net::protocol::EventKind::Announce { text: "撤离可用".into() },
        };
        let mut w = FrameWriter::new();
        encode_server(&mut w, &msg, 1).expect("事件编码应成功");
        let mut r = FrameReader::new();
        r.feed(&w.take());
        let (h, p) = r.next_frame().expect("读帧应成功").expect("应有一帧");
        assert_eq!(h.kind, FrameKind::Event);
        assert_eq!(decode_server(&p).expect("解码应成功"), msg);
    }

    /// 资源键随子类/名不同而不同，且稳定可复现。
    #[test]
    fn resource_keys_are_stable_and_distinct() {
        let a = resource_key(ResourceKind::Model, "OperativeFire");
        let b = resource_key(ResourceKind::Model, "OperativeFire");
        let c = resource_key(ResourceKind::Animation, "OperativeFire");
        assert_eq!(a, b, "同名同子类键应稳定");
        assert_ne!(a, c, "不同子类键应不同");
    }

    /// 坏魔数/超长负载必须被拒绝。
    #[test]
    fn malformed_frames_are_rejected() {
        let mut bad = [0u8; HEADER_BYTES];
        bad[0..4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        assert!(FrameHeader::decode(&bad).is_err(), "坏魔数应被拒");

        let mut w = FrameWriter::new();
        let too_long = vec![0u8; PAYLOAD_MAX + 1];
        assert!(
            encode_resource(&mut w, ResourceKind::Model, 1, Region::InUse, 1, &too_long).is_err(),
            "超单帧容量的资源应被拒"
        );
    }

    /// 资源负载枚举可往返（供无头工具/测试构造）。
    #[test]
    fn resource_payload_enum_roundtrip() {
        let spec = crate::model::catalog().into_iter().next().expect("应有焰狐模型");
        let payload = ResourcePayload::Model(spec.clone());
        let bytes = serde_json::to_vec(&payload).expect("序列化应成功");
        match serde_json::from_slice::<ResourcePayload>(&bytes).expect("反序列化应成功") {
            ResourcePayload::Model(back) => assert_eq!(back, spec),
            ResourcePayload::Animation(_) => panic!("应为模型负载"),
        }
    }
}
