//! 统一定长包线格式（双端共用的传输层）
//!
//! 设计动机（Why）：0.11 的**恒定 64KB 槽帧**对"高频小控制帧"极不划算——一条 200B 的
//! 上行意图要占满 64KB，带宽利用率不到 0.4%。0.12 改为**小定长包 + 指令优先组包**：
//!
//! - **主通道**：每包恒定 [`PACKET_BYTES`] = 256B = [`HEADER_BYTES`] 32B + [`UNITS_PER_PACKET`]
//!   个 [`UNIT_BYTES`] 32B 单元；一包最多装 7 条**指令**（或 7 段**数据**切片）。
//! - **资源通道**：独立 TCP 连接，每包恒定 [`RES_PACKET_BYTES`] = 4096B = 32B 头 + 4064B 负载，
//!   由 `resource_stream` 多次拼接落进客户端 64KB 固定槽位。
//!
//! 两条通道**各管各的包，互不合并**：控制连接恒 256B、资源连接恒 4096B。二者同监听一个端口，
//! 由**首条绑定包**（恒 256B，见 [`PacketKind::Bind`]）决定该连接此后按哪种包长读取。
//!
//! 本模块只负责"字节 ↔ 包 ↔ 头"，不含业务裁决；指令/数据的语义编解码见 `codec`。

use serde::{Deserialize, Serialize};

use crate::model::{VoxelAnimationSpec, VoxelModelSpec};

/// 主通道包长（256B：32B 头 + 7×32B 单元）。
pub const PACKET_BYTES: usize = 256;
/// 资源通道包长（4096B：32B 头 + 4064B 负载）。
pub const RES_PACKET_BYTES: usize = 4096;
/// 定长包头字节数（两条通道一致）。
pub const HEADER_BYTES: usize = 32;
/// 单个"指令或数据"单元的字节数。
pub const UNIT_BYTES: usize = 32;
/// 主通道每包可承载的单元数（(256-32)/32 = 7）。
pub const UNITS_PER_PACKET: usize = (PACKET_BYTES - HEADER_BYTES) / UNIT_BYTES;
/// 单个"指令或数据"单元的定长数组类型。
pub type Unit = [u8; UNIT_BYTES];
/// 主通道包内负载区字节数（224）。
pub const MAIN_PAYLOAD_BYTES: usize = PACKET_BYTES - HEADER_BYTES;
/// 资源通道包内负载区字节数（4064）。
pub const RES_PAYLOAD_BYTES: usize = RES_PACKET_BYTES - HEADER_BYTES;
/// 帧魔数 "COD1"（小端读出 = 0x434F4431）。
pub const MAGIC: u32 = 0x434F_4431;
/// 线格式版本（与协议 y 位对齐；0.12 = 小定长包 + 指令优先世代）。
pub const WIRE_VERSION: u16 = 12;

/// 连接角色：由首条绑定包决定，之后固定该连接的包长。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Role {
    /// 控制连接（双向，恒 256B 包）。
    Control = 0,
    /// 资源连接（只下行，恒 4096B 包）。
    Resource = 1,
}

impl Role {
    fn from_u8(v: u8) -> Result<Self, String> {
        match v {
            0 => Ok(Self::Control),
            1 => Ok(Self::Resource),
            other => Err(format!("未知连接角色: {other}")),
        }
    }

    /// 该角色固定使用的包长。
    pub fn packet_bytes(self) -> usize {
        match self {
            Role::Control => PACKET_BYTES,
            Role::Resource => RES_PACKET_BYTES,
        }
    }
}

/// 包类别：决定负载如何被接收侧消费。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PacketKind {
    /// 首条绑定包（恒 256B）：`sub_kind` = 角色，负载 = 玩家档案名。
    Bind = 0,
    /// 指令包：负载为 1..7 个自包含 32B 指令单元。
    Command = 1,
    /// 数据包：`sub_kind` 标明消息类（快照/事件/超长控制），负载为某条消息字节流的连续 32B 切片。
    Data = 2,
    /// 资源包（仅资源通道）：单份资源的第 `chunk_index` 段。
    Resource = 3,
    /// 一批资源下发完毕（目录批次终止标记）。
    ResourceEnd = 4,
}

impl PacketKind {
    fn from_u8(v: u8) -> Result<Self, String> {
        match v {
            0 => Ok(Self::Bind),
            1 => Ok(Self::Command),
            2 => Ok(Self::Data),
            3 => Ok(Self::Resource),
            4 => Ok(Self::ResourceEnd),
            other => Err(format!("未知包类别: {other}")),
        }
    }
}

/// 数据包的子类（仅 [`PacketKind::Data`] 有意义）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DataKind {
    /// 权威快照。
    Snapshot = 0,
    /// 瞬时事件。
    Event = 1,
    /// 超长控制消息（含字符串的握手/选装/背包 CRUD 等，压不进 32B 指令单元）。
    Control = 2,
}

impl DataKind {
    fn from_u8(v: u8) -> Result<Self, String> {
        match v {
            0 => Ok(Self::Snapshot),
            1 => Ok(Self::Event),
            2 => Ok(Self::Control),
            other => Err(format!("未知数据子类: {other}")),
        }
    }
}

/// 资源帧的落区：客户端据此决定写"在用区"还是"预取区"。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Region {
    /// 在用缓冲（已在视野/正被使用）。
    #[default]
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

/// 资源子类（仅 [`PacketKind::Resource`] 有意义）。
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

/// 定长 32 字节包头。
///
/// 布局（offset, width，小端）：
/// `0/4 magic · 4/2 version · 6/1 kind · 7/1 flags · 8/1 unit_count · 9/1 sub_kind ·
/// 10/1 region · 11/1 reserved · 12/2 chunk_index · 14/2 payload_len · 16/8 seq · 24/8 key`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketHeader {
    pub kind: PacketKind,
    /// `true` = 本包不是最后一片，后续还有同消息的分片（仅 Data/Resource 有意义）。
    pub continuation: bool,
    /// 本包使用的单元数（1..=7；仅 Command 有意义）。
    pub unit_count: u8,
    /// 子类：Bind=角色、Data=DataKind、Resource=ResourceKind、其余填 0。
    pub sub_kind: u8,
    /// 资源落区（仅 Resource 有意义）。
    pub region: Region,
    /// 资源分片序号（仅 Resource 有意义）。
    pub chunk_index: u16,
    /// 负载实际字节数。
    pub payload_len: u16,
    /// 包序号（单调递增）。
    pub seq: u64,
    /// 资源/消息键（0 = 无）。
    pub key: u64,
}

impl PacketHeader {
    /// 编码为定长字节数组。
    pub fn encode(&self) -> [u8; HEADER_BYTES] {
        let mut buf = [0u8; HEADER_BYTES];
        buf[0..4].copy_from_slice(&MAGIC.to_le_bytes());
        buf[4..6].copy_from_slice(&WIRE_VERSION.to_le_bytes());
        buf[6] = self.kind as u8;
        buf[7] = self.continuation as u8;
        buf[8] = self.unit_count;
        buf[9] = self.sub_kind;
        buf[10] = self.region as u8;
        // 11 reserved
        buf[12..14].copy_from_slice(&self.chunk_index.to_le_bytes());
        buf[14..16].copy_from_slice(&self.payload_len.to_le_bytes());
        buf[16..24].copy_from_slice(&self.seq.to_le_bytes());
        buf[24..32].copy_from_slice(&self.key.to_le_bytes());
        buf
    }

    /// 从定长字节数组解码（校验魔数/版本/类别/长度）。
    ///
    /// `packet_bytes` 为该连接固定包长，用于校验 `payload_len` 不越界。
    pub fn decode(buf: &[u8; HEADER_BYTES], packet_bytes: usize) -> Result<Self, String> {
        let magic = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        if magic != MAGIC {
            return Err(format!("包魔数不符: 0x{magic:08X}"));
        }
        let version = u16::from_le_bytes([buf[4], buf[5]]);
        if version != WIRE_VERSION {
            return Err(format!("线格式版本不符: {version}（期望 {WIRE_VERSION}）"));
        }
        let kind = PacketKind::from_u8(buf[6])?;
        let payload_len = u16::from_le_bytes([buf[14], buf[15]]);
        if payload_len as usize > packet_bytes - HEADER_BYTES {
            return Err(format!("负载超长: {payload_len} > {}", packet_bytes - HEADER_BYTES));
        }
        Ok(Self {
            kind,
            continuation: buf[7] & 0x01 != 0,
            unit_count: buf[8],
            sub_kind: buf[9],
            region: Region::from_u8(buf[10])?,
            chunk_index: u16::from_le_bytes([buf[12], buf[13]]),
            payload_len,
            seq: u64::from_le_bytes([
                buf[16], buf[17], buf[18], buf[19], buf[20], buf[21], buf[22], buf[23],
            ]),
            key: u64::from_le_bytes([
                buf[24], buf[25], buf[26], buf[27], buf[28], buf[29], buf[30], buf[31],
            ]),
        })
    }

    /// 由子类字节解析数据类（仅 Data）。
    pub fn data_kind(&self) -> Result<DataKind, String> {
        DataKind::from_u8(self.sub_kind)
    }

    /// 由子类字节解析资源子类（仅 Resource）。
    pub fn resource_kind(&self) -> Result<ResourceKind, String> {
        ResourceKind::from_u8(self.sub_kind)
    }

    /// 由子类字节解析连接角色（仅 Bind）。
    pub fn role(&self) -> Result<Role, String> {
        Role::from_u8(self.sub_kind)
    }
}

/// 定长包写出器：把若干包追加进一段连续缓冲（每包恒定 `packet_bytes`，尾部零填充）。
pub struct PacketWriter {
    packet_bytes: usize,
    out: Vec<u8>,
}

impl PacketWriter {
    /// 按固定包长构造（控制连接用 [`PACKET_BYTES`]，资源连接用 [`RES_PACKET_BYTES`]）。
    pub fn new(packet_bytes: usize) -> Self {
        Self { packet_bytes, out: Vec::new() }
    }

    /// 追加一包（负载不足固定包长时尾部零填充对齐）。
    pub fn push(&mut self, header: PacketHeader, payload: &[u8]) -> Result<(), String> {
        let cap = self.packet_bytes - HEADER_BYTES;
        if payload.len() > cap {
            return Err(format!("负载超单包容量: {} > {cap}", payload.len()));
        }
        let mut h = header;
        h.payload_len = payload.len() as u16;
        self.out.extend_from_slice(&h.encode());
        self.out.extend_from_slice(payload);
        self.out.resize(self.out.len() + (cap - payload.len()), 0);
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

/// 定长包读出器：累积 TCP 字节流，按固定包长逐包解析。
#[derive(Default)]
pub struct PacketReader {
    packet_bytes: usize,
    buf: Vec<u8>,
    cursor: usize,
}

impl PacketReader {
    /// 按固定包长构造。
    pub fn new(packet_bytes: usize) -> Self {
        Self { packet_bytes, buf: Vec::new(), cursor: 0 }
    }

    /// 投入一段 TCP 收到的字节。
    pub fn feed(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// 取下一包；不足一包返回 `Ok(None)`（等待更多字节）。
    pub fn next_packet(&mut self) -> Result<Option<(PacketHeader, Vec<u8>)>, String> {
        if self.buf.len() - self.cursor < self.packet_bytes {
            self.compact();
            return Ok(None);
        }
        let start = self.cursor;
        let mut hbuf = [0u8; HEADER_BYTES];
        hbuf.copy_from_slice(&self.buf[start..start + HEADER_BYTES]);
        let header = PacketHeader::decode(&hbuf, self.packet_bytes)?;
        let plen = header.payload_len as usize;
        let payload = self.buf[start + HEADER_BYTES..start + HEADER_BYTES + plen].to_vec();
        self.cursor += self.packet_bytes;
        Ok(Some((header, payload)))
    }

    fn compact(&mut self) {
        if self.cursor > 0 {
            self.buf.drain(..self.cursor);
            self.cursor = 0;
        }
    }
}

/// 分片重组器：把同一消息/资源流的连续分片拼回完整负载。
#[derive(Default)]
pub struct ChunkAssembler {
    acc: Vec<u8>,
    active: bool,
}

impl ChunkAssembler {
    /// 投入一片；若该片是最后一片则返回完整负载（非最后一片返回 `Ok(None)`）。
    pub fn push(&mut self, continuation: bool, payload: &[u8]) -> Option<Vec<u8>> {
        if continuation {
            self.active = true;
            self.acc.extend_from_slice(payload);
            return None;
        }
        if !self.active {
            // 单片消息：直接返回。
            return Some(payload.to_vec());
        }
        self.acc.extend_from_slice(payload);
        self.active = false;
        Some(std::mem::take(&mut self.acc))
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

/// 解码资源包负载为体素模型规格。
pub fn decode_model(payload: &[u8]) -> Result<VoxelModelSpec, String> {
    serde_json::from_slice(payload).map_err(|e| format!("解析体素模型失败: {e}"))
}

/// 解码资源包负载为体素动画规格。
pub fn decode_animation(payload: &[u8]) -> Result<VoxelAnimationSpec, String> {
    serde_json::from_slice(payload).map_err(|e| format!("解析体素动画失败: {e}"))
}

/// 由槽位目录里的子类字节解析资源子类（供客户端对象池把槽内容解码为型号/动画）。
///
/// 返回 `None` 表示该字节不是已知子类（畸形/旧版本）。
pub fn resource_kind_from_byte(v: u8) -> Option<ResourceKind> {
    ResourceKind::from_u8(v).ok()
}

/// 便于测试/无头工具构造资源负载。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResourcePayload {
    Model(VoxelModelSpec),
    Animation(VoxelAnimationSpec),
}

/// 序列化辅助（统一错误文案，避免裸 unwrap）。
pub fn to_json<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    serde_json::to_vec(value).map_err(|e| format!("消息序列化失败: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 包长恒等式：256 = 32 + 7×32；4096 = 32 + 4064。
    #[test]
    fn packet_layout_constants_are_exact() {
        assert_eq!(PACKET_BYTES, 256);
        assert_eq!(HEADER_BYTES + UNITS_PER_PACKET * UNIT_BYTES, PACKET_BYTES);
        assert_eq!(UNITS_PER_PACKET, 7);
        assert_eq!(MAIN_PAYLOAD_BYTES, 224);
        assert_eq!(RES_PACKET_BYTES, 4096);
        assert_eq!(HEADER_BYTES + RES_PAYLOAD_BYTES, RES_PACKET_BYTES);
        assert_eq!(RES_PAYLOAD_BYTES, 4064);
    }

    /// 主通道单包恰为 256B（尾部零填充对齐）。
    #[test]
    fn main_packet_is_256_bytes() {
        let mut w = PacketWriter::new(PACKET_BYTES);
        let units = [[7u8; UNIT_BYTES]];
        w.push(
            PacketHeader {
                kind: PacketKind::Command,
                continuation: false,
                unit_count: 1,
                sub_kind: 0,
                region: Region::InUse,
                chunk_index: 0,
                payload_len: 0,
                seq: 1,
                key: 0,
            },
            &units.concat(),
        )
        .expect("写包应成功");
        let bytes = w.take();
        assert_eq!(bytes.len(), PACKET_BYTES, "单包应恰为 256B");
        assert_eq!(bytes.len() % PACKET_BYTES, 0);
    }

    /// 头往返一致（含 continuation / 子类 / 区 / 分片号 / 键）。
    #[test]
    fn header_roundtrip() {
        let h = PacketHeader {
            kind: PacketKind::Resource,
            continuation: true,
            unit_count: 3,
            sub_kind: ResourceKind::Animation as u8,
            region: Region::Prefetch,
            chunk_index: 5,
            payload_len: 0,
            seq: 42,
            key: 0xDEAD_BEEF,
        };
        let back = PacketHeader::decode(&h.encode(), RES_PACKET_BYTES).expect("解码应成功");
        assert_eq!(back, h);
        assert_eq!(back.resource_kind().unwrap(), ResourceKind::Animation);
    }

    /// 数据包按 continuation 分片后重组，与原文一致。
    #[test]
    fn data_chunks_reassemble() {
        let payload: Vec<u8> = (0..(MAIN_PAYLOAD_BYTES + 5)).map(|i| (i % 251) as u8).collect();
        // 切成 224B 一片：两片。
        let mut w = PacketWriter::new(PACKET_BYTES);
        for (i, chunk) in payload.chunks(MAIN_PAYLOAD_BYTES).enumerate() {
            let last = (i + 1) * MAIN_PAYLOAD_BYTES >= payload.len();
            w.push(
                PacketHeader {
                    kind: PacketKind::Data,
                    continuation: !last,
                    unit_count: 0,
                    sub_kind: DataKind::Snapshot as u8,
                    region: Region::InUse,
                    chunk_index: 0,
                    payload_len: 0,
                    seq: i as u64 + 1,
                    key: 7,
                },
                chunk,
            )
            .expect("写包应成功");
        }
        let bytes = w.take();
        assert_eq!(bytes.len(), 2 * PACKET_BYTES, "应恰为两包");

        let mut r = PacketReader::new(PACKET_BYTES);
        r.feed(&bytes);
        let mut asm = ChunkAssembler::default();
        let (h1, p1) = r.next_packet().expect("读包应成功").expect("应有第一片");
        assert!(h1.continuation);
        assert!(asm.push(h1.continuation, &p1).is_none());
        let (h2, p2) = r.next_packet().expect("读包应成功").expect("应有第二片");
        assert!(!h2.continuation);
        let joined = asm.push(h2.continuation, &p2).expect("应得到完整负载");
        assert_eq!(joined, payload, "重组结果应与原文一致");
        assert!(r.next_packet().expect("读包应成功").is_none());
    }

    /// 资源包恰为 4096B，chunk_index/键原样保留。
    #[test]
    fn resource_packet_is_4096_bytes() {
        let payload = vec![9u8; RES_PAYLOAD_BYTES];
        let key = resource_key(ResourceKind::Model, "FireFox");
        let mut w = PacketWriter::new(RES_PACKET_BYTES);
        w.push(
            PacketHeader {
                kind: PacketKind::Resource,
                continuation: false,
                unit_count: 0,
                sub_kind: ResourceKind::Model as u8,
                region: Region::InUse,
                chunk_index: 3,
                payload_len: 0,
                seq: 1,
                key,
            },
            &payload,
        )
        .expect("写包应成功");
        let bytes = w.take();
        assert_eq!(bytes.len(), RES_PACKET_BYTES, "资源包应恰为 4096B");

        let mut r = PacketReader::new(RES_PACKET_BYTES);
        r.feed(&bytes);
        let (h, p) = r.next_packet().expect("读包应成功").expect("应有一包");
        assert_eq!(h.kind, PacketKind::Resource);
        assert_eq!(h.chunk_index, 3);
        assert_eq!(h.key, key);
        assert_eq!(h.resource_kind().unwrap(), ResourceKind::Model);
        assert_eq!(p, payload);
        assert_ne!(key, 0, "资源键不应为保留值 0");
    }

    /// 坏魔数 / 超长负载必须被拒绝。
    #[test]
    fn malformed_packets_are_rejected() {
        let mut bad = [0u8; HEADER_BYTES];
        bad[0..4].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        assert!(PacketHeader::decode(&bad, PACKET_BYTES).is_err(), "坏魔数应被拒");

        let mut w = PacketWriter::new(PACKET_BYTES);
        let too_long = vec![0u8; MAIN_PAYLOAD_BYTES + 1];
        assert!(
            w.push(
                PacketHeader {
                    kind: PacketKind::Data,
                    continuation: false,
                    unit_count: 0,
                    sub_kind: 0,
                    region: Region::InUse,
                    chunk_index: 0,
                    payload_len: 0,
                    seq: 1,
                    key: 0,
                },
                &too_long,
            )
            .is_err(),
            "超单包容量的负载应被拒"
        );
    }

    /// 资源负载枚举可往返（供无头工具/测试构造）。
    #[test]
    fn resource_payload_enum_roundtrip() {
        // 契约 crate 不含服务端几何目录（`catalog()` 在 ServerCode），故手工构造一份最小规格。
        let spec = VoxelModelSpec {
            preset: crate::model::ModelPreset::OperativeFire,
            scale: crate::model::YANHU_SCALE,
            bones: vec![crate::model::VoxelBone {
                name: "body".to_string(),
                parent: None,
                pivot: [0.0; 3],
                rotation: [0.0; 3],
                cubes: vec![],
            }],
        };
        let payload = ResourcePayload::Model(spec.clone());
        let bytes = serde_json::to_vec(&payload).expect("序列化应成功");
        match serde_json::from_slice::<ResourcePayload>(&bytes).expect("反序列化应成功") {
            ResourcePayload::Model(back) => assert_eq!(back, spec),
            ResourcePayload::Animation(_) => panic!("应为模型负载"),
        }
    }
}
