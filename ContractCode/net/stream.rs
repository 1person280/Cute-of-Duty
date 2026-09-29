//! 资源通道（独立 TCP 连接 · 每包恒定 4096B）
//!
//! 设计动机（Why）：体素几何/动画 JSON 动辄数十 KB，塞进 256B 主通道要切上百片、且与
//! 延迟敏感的指令抢带宽。0.12 把资源拆到**第二条 TCP 连接**，每包恒定
//! [`RES_PACKET_BYTES`] = 4096B = 32B 头 + [`RES_PAYLOAD_BYTES`] 4064B 负载；一份资源按
//! `chunk_index` 切片、末片 `continuation=false` 收尾。客户端把重组后的整份负载落进
//! 64KB 固定槽位池（见 HostCode `net::remote`），实体突现即复用。
//!
//! 两条通道**各管各的包**：控制连接恒 256B、资源连接恒 4096B。二者同监听一个端口，
//! 由**首条绑定包**决定角色（见 `session::accept_loop`）。

use crate::model::{VoxelAnimationSpec, VoxelModelSpec};
use crate::net::packet::{
    animation_key, model_key, PacketHeader, PacketKind, PacketWriter, Region, ResourceKind,
    ResourcePayload, RES_PACKET_BYTES, RES_PAYLOAD_BYTES,
};

/// 一份待下发的资源（子类 + 键 + 区 + 负载）。
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceMessage {
    pub kind: ResourceKind,
    pub key: u64,
    pub region: Region,
    pub payload: Vec<u8>,
}

impl ResourceMessage {
    /// 由体素模型规格构造（键 = [`model_key`]，负载 = 模型 JSON）。
    pub fn from_model(spec: &VoxelModelSpec, region: Region) -> Result<Self, String> {
        Ok(Self {
            kind: ResourceKind::Model,
            key: model_key(spec),
            region,
            payload: crate::net::packet::to_json(spec)?,
        })
    }

    /// 由体素动画规格构造（键 = [`animation_key`]，负载 = 动画 JSON）。
    pub fn from_animation(spec: &VoxelAnimationSpec, region: Region) -> Result<Self, String> {
        Ok(Self {
            kind: ResourceKind::Animation,
            key: animation_key(spec),
            region,
            payload: crate::net::packet::to_json(spec)?,
        })
    }
}

/// 把整份模型/动画目录摊成待下发资源清单（顺序：模型在前、动画在后）。
pub fn split_catalog(
    models: &[VoxelModelSpec],
    animations: &[VoxelAnimationSpec],
    region: Region,
) -> Result<Vec<ResourceMessage>, String> {
    let mut out = Vec::with_capacity(models.len() + animations.len());
    for m in models {
        out.push(ResourceMessage::from_model(m, region)?);
    }
    for a in animations {
        out.push(ResourceMessage::from_animation(a, region)?);
    }
    Ok(out)
}

/// 把一份资源切片写入资源包（每片 4064B，`chunk_index` 递增，末片 `continuation=false`）。
pub fn encode_resource(
    msg: &ResourceMessage,
    seq: &mut u64,
    w: &mut PacketWriter,
) -> Result<(), String> {
    let chunks: Vec<&[u8]> = msg.payload.chunks(RES_PAYLOAD_BYTES).collect();
    let total = chunks.len().max(1);
    for (i, chunk) in chunks.iter().enumerate() {
        *seq += 1;
        w.push(
            PacketHeader {
                kind: PacketKind::Resource,
                continuation: i + 1 < total,
                unit_count: 0,
                sub_kind: msg.kind as u8,
                region: msg.region,
                chunk_index: i as u16,
                payload_len: 0,
                seq: *seq,
                key: msg.key,
            },
            chunk,
        )?;
    }
    if msg.payload.is_empty() {
        // 空负载也发一片，保证接收侧能按 key 落槽（避免"只有 ResourceEnd"歧义）。
        *seq += 1;
        w.push(
            PacketHeader {
                kind: PacketKind::Resource,
                continuation: false,
                unit_count: 0,
                sub_kind: msg.kind as u8,
                region: msg.region,
                chunk_index: 0,
                payload_len: 0,
                seq: *seq,
                key: msg.key,
            },
            &[],
        )?;
    }
    Ok(())
}

/// 写一批资源的终止标记（客户端据此把目录标为就绪）。
pub fn encode_batch_end(region: Region, seq: &mut u64, w: &mut PacketWriter) -> Result<(), String> {
    *seq += 1;
    w.push(
        PacketHeader {
            kind: PacketKind::ResourceEnd,
            continuation: false,
            unit_count: 0,
            sub_kind: 0,
            region,
            chunk_index: 0,
            payload_len: 0,
            seq: *seq,
            key: 0,
        },
        &[],
    )
}

/// 一份重组完成的资源（接收侧按 `key` 落槽）。
#[derive(Debug, Clone, PartialEq)]
pub struct AssembledResource {
    pub kind: ResourceKind,
    pub key: u64,
    pub region: Region,
    pub payload: Vec<u8>,
}

/// 资源重组器：把同一份资源的连续分片拼回完整负载。
///
/// 资源连接是**单资源顺序下发**（切片连续），故只维护一份活动缓冲；`chunk_index == 0`
/// 开启新资源，`continuation == false` 收尾并交付。
#[derive(Default)]
pub struct ResourceAssembler {
    kind: Option<ResourceKind>,
    key: u64,
    region: Region,
    acc: Vec<u8>,
}

impl ResourceAssembler {
    /// 投入一片资源包负载；若是该资源末片则返回重组结果，否则 `None`。
    pub fn push(
        &mut self,
        header: &PacketHeader,
        payload: &[u8],
    ) -> Result<Option<AssembledResource>, String> {
        if header.kind != PacketKind::Resource {
            return Ok(None);
        }
        if header.chunk_index == 0 {
            self.kind = Some(header.resource_kind()?);
            self.key = header.key;
            self.region = header.region;
            self.acc.clear();
        }
        self.acc.extend_from_slice(payload);
        if header.continuation {
            return Ok(None);
        }
        let kind = self.kind.ok_or_else(|| "资源末片缺少首片（chunk_index=0）".to_string())?;
        self.kind = None;
        Ok(Some(AssembledResource {
            kind,
            key: self.key,
            region: self.region,
            payload: std::mem::take(&mut self.acc),
        }))
    }
}

/// 把重组后的资源负载解码为强类型规格。
pub fn decode_resource(msg: &AssembledResource) -> Result<ResourcePayload, String> {
    match msg.kind {
        ResourceKind::Model => Ok(ResourcePayload::Model(crate::net::packet::decode_model(
            &msg.payload,
        )?)),
        ResourceKind::Animation => Ok(ResourcePayload::Animation(
            crate::net::packet::decode_animation(&msg.payload)?,
        )),
    }
}

/// 单包资源负载容量（供调用方核算切片数）。
pub const fn payload_capacity() -> usize {
    RES_PAYLOAD_BYTES
}

/// 资源包长（供调用方构造 `PacketWriter`）。
pub const fn packet_bytes() -> usize {
    RES_PACKET_BYTES
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::packet::{PacketReader, RES_PAYLOAD_BYTES};

    /// 小资源单片下发、单片重组。
    #[test]
    fn single_chunk_resource_roundtrips() {
        // 契约 crate 不含服务端几何目录（`catalog()` 在 ServerCode），故手工构造一份最小规格。
        let model = VoxelModelSpec {
            preset: crate::model::ModelPreset::OperativeFire,
            scale: crate::model::YANHU_SCALE,
            bones: vec![],
        };
        let msg = ResourceMessage::from_model(&model, Region::InUse).expect("构造应成功");
        let mut seq = 0;
        let mut w = PacketWriter::new(RES_PACKET_BYTES);
        encode_resource(&msg, &mut seq, &mut w).expect("编码应成功");
        encode_batch_end(Region::InUse, &mut seq, &mut w).expect("编码应成功");

        let bytes = w.take();
        assert_eq!(bytes.len() % RES_PACKET_BYTES, 0, "资源包应定长对齐");

        let mut r = PacketReader::new(RES_PACKET_BYTES);
        r.feed(&bytes);
        let mut asm = ResourceAssembler::default();
        let mut assembled = None;
        let mut batch_end = false;
        while let Some((header, payload)) = r.next_packet().expect("读包应成功") {
            match header.kind {
                PacketKind::Resource => assembled = asm.push(&header, &payload).expect("重组应成功"),
                PacketKind::ResourceEnd => batch_end = true,
                _ => {}
            }
        }
        let assembled = assembled.expect("应得到重组资源");
        assert_eq!(assembled.key, msg.key);
        assert_eq!(assembled.region, Region::InUse);
        match decode_resource(&assembled).expect("解码应成功") {
            ResourcePayload::Model(back) => assert_eq!(back, model),
            ResourcePayload::Animation(_) => panic!("应为模型"),
        }
        assert!(batch_end, "应收到批次终止标记");
    }

    /// 超单包容量的负载跨片下发并被完整重组。
    #[test]
    fn large_resource_spans_chunks() {
        let payload: Vec<u8> = (0..(RES_PAYLOAD_BYTES * 2 + 17)).map(|i| (i % 251) as u8).collect();
        let msg = ResourceMessage {
            kind: ResourceKind::Animation,
            key: 0xABCD,
            region: Region::Prefetch,
            payload: payload.clone(),
        };
        let mut seq = 0;
        let mut w = PacketWriter::new(RES_PACKET_BYTES);
        encode_resource(&msg, &mut seq, &mut w).expect("编码应成功");
        let bytes = w.take();
        assert_eq!(bytes.len(), 3 * RES_PACKET_BYTES, "应切成 3 片");

        let mut r = PacketReader::new(RES_PACKET_BYTES);
        r.feed(&bytes);
        let mut asm = ResourceAssembler::default();
        let mut last = None;
        while let Some((header, pl)) = r.next_packet().expect("读包应成功") {
            if let Some(a) = asm.push(&header, &pl).expect("重组应成功") {
                last = Some(a);
            }
        }
        let a = last.expect("应得到重组资源");
        assert_eq!(a.payload, payload, "重组负载应与原文一致");
        assert_eq!(a.key, 0xABCD);
        assert_eq!(a.region, Region::Prefetch);
    }

    /// 目录摊平：模型/动画各归其位、键非零。
    #[test]
    fn catalog_splits_into_resources() {
        // 契约 crate 不含服务端几何/动画目录，故手工构造最小规格验证摊平。
        let models = vec![VoxelModelSpec {
            preset: crate::model::ModelPreset::OperativeFire,
            scale: crate::model::YANHU_SCALE,
            bones: vec![],
        }];
        let anims = vec![VoxelAnimationSpec {
            clip: "animation.test.idle".to_string(),
            length: 1.0,
            tracks: vec![],
        }];
        let items = split_catalog(&models, &anims, Region::InUse).expect("摊平应成功");
        assert!(!items.is_empty(), "应有资源");
        assert!(items.iter().all(|i| i.key != 0), "资源键不应为保留值 0");
        assert!(items.iter().any(|i| i.kind == ResourceKind::Model));
    }
}
