//! 远程资源对象池（固定 16MB · 256 × 64KB 固定地址槽位）
//!
//! 设计动机（Why）：0.10 及以前，客户端要等资源随快照/握手消息抵达才知道"这个实体长什么样"，
//! 实体因 AOI 突现时只能干等。0.12 起资源走独立 4096B 包通道（见服务端 `net::resource_stream`），
//! 客户端按 `key` 把重组后的整份负载落进**固定容量、固定地址**的对象池：
//!
//! - 总容量恒 [`POOL_BYTES`] = 16MB，切成 [`SLOT_COUNT`] = 256 个 [`SLOT_BYTES`] = 64KB 槽；
//! - 单块连续分配、**永不重分配**，故第 `i` 槽的数据地址恒为 `base + i × 64KB`；
//! - 区划：前 [`IN_USE_SLOTS`] = 250 槽为**在用缓冲**（16000KB），末 [`PREFETCH_SLOTS`] = 6 槽
//!   为**预取区**（384KB，服务端据 AOI 边缘预测提前下发）。
//!
//! 语义上是**通用对象池 / 资源缓存**：资源按 `key` 落槽，实体突现时按 key 命中即复用，
//! 而不是等加载；预取区命中则 [`RemotePool::promote`] 提升进在用区。
//!
//! 纯客户端本地内存，**不引入共享内存**（保跨机可移植）；下行仍严格走单线程 TCP。

use std::sync::mpsc;

use bevy::prelude::{ResMut, Resource};

use cute_of_duty_contract::net::packet::{
    Region, ResourceKind, decode_animation, decode_model, resource_kind_from_byte,
};

use crate::flow::ModelCatalog;

/// 单槽字节数（64KB：一份资源在资源通道上跨 4096B 包重组后整体落槽）。
pub const SLOT_BYTES: usize = 64 * 1024;
/// 槽位总数（16MB / 64KB）。
pub const SLOT_COUNT: usize = 256;
/// 池总字节数（固定 16MB）。
pub const POOL_BYTES: usize = 16 * 1024 * 1024;
/// 在用缓冲槽数（16000KB / 64KB）。
pub const IN_USE_SLOTS: usize = 250;
/// 预取区槽数（384KB / 64KB）。
pub const PREFETCH_SLOTS: usize = 6;

/// 槽位目录元数据（`key == 0` 表示空槽；资源键经 FNV-1a 计算，恒非 0）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SlotMeta {
    /// 资源键（0 = 空槽）。
    pub key: u64,
    /// 资源子类。
    pub kind: u8,
    /// 负载实际字节数。
    pub len: u32,
}

/// 固定地址远程资源池。
pub struct RemotePool {
    /// 单块连续 16MB，基址固定；第 `i` 槽数据在 `base + i × SLOT_BYTES`。
    bytes: Box<[u8]>,
    /// 256 个槽的目录。
    slots: Vec<SlotMeta>,
}

impl Default for RemotePool {
    fn default() -> Self {
        Self::new()
    }
}

/// 池的固定地址读写 API。
///
/// 其中若干只读/提升接口（`base_ptr`/`slot_ptr`/`get`/`promote` 等）当前仅由**单测**与
/// 即将接线的预取提升路径使用，故对「未在 bin 路径调用」放宽 dead_code 检查——它们是池
/// 契约的一部分（固定地址、命中复用、预取提升），非临时残留。
#[allow(dead_code)]
impl RemotePool {
    /// 一次性分配 16MB（用 `vec!` 构造再转 slice，避免在栈上构造 16MB 数组导致爆栈）。
    pub fn new() -> Self {
        debug_assert_eq!(IN_USE_SLOTS + PREFETCH_SLOTS, SLOT_COUNT);
        debug_assert_eq!(SLOT_BYTES * SLOT_COUNT, POOL_BYTES);
        Self {
            bytes: vec![0u8; POOL_BYTES].into_boxed_slice(),
            slots: vec![SlotMeta::default(); SLOT_COUNT],
        }
    }

    /// 池基址（固定，跨插入不变）。
    pub fn base_ptr(&self) -> *const u8 {
        self.bytes.as_ptr()
    }

    /// 第 `i` 槽在池内的固定字节偏移。
    pub fn slot_offset(i: usize) -> usize {
        i * SLOT_BYTES
    }

    /// 第 `i` 槽的固定地址（`base + i × 64KB`）。
    pub fn slot_ptr(&self, i: usize) -> *const u8 {
        self.bytes[Self::slot_offset(i)..].as_ptr()
    }

    /// 某区的槽位下标范围。
    pub fn range_of(region: Region) -> std::ops::Range<usize> {
        match region {
            Region::InUse => 0..IN_USE_SLOTS,
            Region::Prefetch => IN_USE_SLOTS..SLOT_COUNT,
        }
    }

    /// 槽下标所属区。
    pub fn region_of(i: usize) -> Region {
        if i < IN_USE_SLOTS { Region::InUse } else { Region::Prefetch }
    }

    /// 某区已用槽数。
    pub fn used(&self, region: Region) -> usize {
        Self::range_of(region).filter(|&i| self.slots[i].key != 0).count()
    }

    /// 资源是否已在池中（任意区）。
    pub fn contains(&self, key: u64) -> bool {
        key != 0 && self.slots.iter().any(|s| s.key == key)
    }

    /// 按 key 取负载（返回切片与槽下标）；未命中返回 `None`。
    pub fn get(&self, key: u64) -> Option<(&[u8], usize)> {
        let i = self.slots.iter().position(|s| s.key == key && s.key != 0)?;
        Some((self.slot_bytes(i), i))
    }

    /// 按 key 在指定区取负载。
    pub fn get_in(&self, region: Region, key: u64) -> Option<(&[u8], usize)> {
        let i = Self::range_of(region).find(|&i| self.slots[i].key == key && key != 0)?;
        Some((self.slot_bytes(i), i))
    }

    /// 取某槽的元数据与负载切片。
    pub fn slot(&self, i: usize) -> (SlotMeta, &[u8]) {
        (self.slots[i], self.slot_bytes(i))
    }

    /// 写入一个资源：同区同 key 已存在则覆盖该槽，否则取同区首个空槽。
    ///
    /// 返回落槽下标；负载超槽容量、key 为保留值 0、或该区已满时返回 `None`。
    pub fn insert(
        &mut self,
        region: Region,
        key: u64,
        kind: ResourceKind,
        payload: &[u8],
    ) -> Option<usize> {
        if key == 0 || payload.len() > SLOT_BYTES {
            return None;
        }
        if let Some(i) = Self::range_of(region).find(|&i| self.slots[i].key == key) {
            self.write_slot(i, key, kind, payload);
            return Some(i);
        }
        let free = Self::range_of(region).find(|&i| self.slots[i].key == 0)?;
        self.write_slot(free, key, kind, payload);
        Some(free)
    }

    /// 把预取槽提升进在用区（命中即复用）：返回在用区槽下标。
    ///
    /// 在用区已有同 key 时直接丢弃预取副本并返回既有槽；预取槽为空或下标不属于预取区、
    /// 或在用区已满时返回 `None`。
    pub fn promote(&mut self, prefetch_idx: usize) -> Option<usize> {
        if Self::region_of(prefetch_idx) != Region::Prefetch {
            return None;
        }
        let meta = self.slots[prefetch_idx];
        if meta.key == 0 {
            return None;
        }
        if let Some(existing) = Self::range_of(Region::InUse).find(|&i| self.slots[i].key == meta.key)
        {
            self.slots[prefetch_idx] = SlotMeta::default();
            return Some(existing);
        }
        let free = Self::range_of(Region::InUse).find(|&i| self.slots[i].key == 0)?;
        let src = Self::slot_offset(prefetch_idx);
        let dst = Self::slot_offset(free);
        self.bytes.copy_within(src..src + SLOT_BYTES, dst);
        self.slots[free] = meta;
        self.slots[prefetch_idx] = SlotMeta::default();
        Some(free)
    }

    /// 清空某区的目录（字节不动，仅标记空槽）。
    pub fn clear_region(&mut self, region: Region) {
        for i in Self::range_of(region) {
            self.slots[i] = SlotMeta::default();
        }
    }

    /// 写入槽内容并清零尾部（保证同 key 覆盖后旧字节不残留）。
    fn write_slot(&mut self, i: usize, key: u64, kind: ResourceKind, payload: &[u8]) {
        let start = Self::slot_offset(i);
        self.bytes[start..start + payload.len()].copy_from_slice(payload);
        for b in &mut self.bytes[start + payload.len()..start + SLOT_BYTES] {
            *b = 0;
        }
        self.slots[i] = SlotMeta { key, kind: kind as u8, len: payload.len() as u32 };
    }

    /// 第 `i` 槽的有效负载切片。
    fn slot_bytes(&self, i: usize) -> &[u8] {
        let len = self.slots[i].len as usize;
        let start = Self::slot_offset(i);
        &self.bytes[start..start + len]
    }
}

/// 一帧远程资源（网络线程解出的下行对象，交给 ECS 侧写池）。
#[derive(Debug, Clone)]
pub struct ResourceFrame {
    pub kind: ResourceKind,
    pub key: u64,
    pub region: Region,
    pub bytes: Vec<u8>,
}

/// 下载线程 → 池的投递消息。
///
/// 资源帧与批次终止标记走同一条通道，是为了让 ECS 侧按**到达顺序**处理：先写完本批全部
/// 资源、再据 [`PoolMessage::BatchEnd`] 把目录标记为就绪（`catalog_ready`），从而保证
/// 「目录就绪」不会早于该批资源落池。
#[derive(Debug, Clone)]
pub enum PoolMessage {
    /// 一份资源（写入对应区的槽位）。
    Resource(ResourceFrame),
    /// 一批资源（服务端 `ResourceEnd`）下发完毕。
    BatchEnd,
}

/// 远程资源池的 Bevy 资源：持有池本体 + 网络线程投递通道。
///
/// 设计动机（Why）：池由 Bevy 主线程独占（无锁、无跨线程别名），网络线程只把解出的资源帧
/// 经通道投递过来，由 `receive_resources` 排空写池——与既有快照/控制通道同一模式。
#[derive(Resource)]
pub struct RemoteObjects {
    /// 固定 16MB 槽位池。
    pub pool: RemotePool,
    /// 网络线程 → ECS 的池消息通道（`Receiver` 非 Sync，故以 Mutex 包裹）。
    pub rx: std::sync::Mutex<mpsc::Receiver<PoolMessage>>,
    /// 目录批次是否已收完（收到服务端 `ResourceEnd` 后置位）。
    pub catalog_ready: bool,
    /// 已从池同步进 [`ModelCatalog`] 的资源键，避免每帧重复解码。
    synced_keys: std::collections::HashSet<u64>,
}

impl RemoteObjects {
    /// 由网络线程的池消息接收端构造。
    pub fn new(rx: mpsc::Receiver<PoolMessage>) -> Self {
        Self {
            pool: RemotePool::new(),
            rx: std::sync::Mutex::new(rx),
            catalog_ready: false,
            synced_keys: std::collections::HashSet::new(),
        }
    }

    /// 直接写入一个资源帧（测试与内部复用）。
    pub fn insert_frame(&mut self, frame: &ResourceFrame) -> Option<usize> {
        self.pool
            .insert(frame.region, frame.key, frame.kind, &frame.bytes)
    }

    /// 把池中**尚未同步**的在用区资源解码进 `catalog`（模型/动画各归其位）。
    ///
    /// 设计动机（Why）：下游渲染（`apply_entities`/`voxel_model`/`voxel_idle`）消费的是
    /// `ModelCatalog` 视图；池是传输侧的固定地址缓存。此系统做一次"池 → 视图"的增量搬运，
    /// 使「实体突现」直接命中已解码目录，而不是等下一批握手数据。仅遍历在用区（预取区资源
    /// 由 [`RemotePool::promote`] 提升后再进目录）。
    pub fn sync_catalog(&mut self, catalog: &mut ModelCatalog) {
        for i in RemotePool::range_of(Region::InUse) {
            let meta = self.pool.slot(i).0;
            if meta.key == 0 || !self.synced_keys.insert(meta.key) {
                continue;
            }
            let bytes = self.pool.slot(i).1;
            match resource_kind_from_byte(meta.kind) {
                Some(ResourceKind::Model) => {
                    if let Ok(spec) = decode_model(bytes) {
                        if !catalog.models.iter().any(|m| m.preset == spec.preset) {
                            catalog.models.push(spec);
                        }
                    }
                }
                Some(ResourceKind::Animation) => {
                    if let Ok(spec) = decode_animation(bytes) {
                        if !catalog.animations.iter().any(|a| a.clip == spec.clip) {
                            catalog.animations.push(spec);
                        }
                    }
                }
                None => {}
            }
        }
    }
}

/// 每帧排空池消息通道并写入池（资源）/置位目录就绪（批次终止）。
pub fn receive_resources(mut objects: ResMut<RemoteObjects>) {
    // 持锁只做通道读取，释放借用后再写池，避免与 `pool` 可变借用冲突。
    let mut drained: Vec<PoolMessage> = Vec::new();
    {
        let rx = objects.rx.lock().expect("资源通道锁不应中毒");
        while let Ok(msg) = rx.try_recv() {
            drained.push(msg);
        }
    }
    for msg in drained {
        match msg {
            PoolMessage::Resource(frame) => {
                let _ = objects.insert_frame(&frame);
            }
            PoolMessage::BatchEnd => objects.catalog_ready = true,
        }
    }
}

/// 把池中在用区资源增量同步进 [`ModelCatalog`]（下游渲染视图）。
///
/// 须排在 `receive_resources` 之后、`apply_entities` 之前：同一帧收下的资源帧可被本帧
/// 的实体对账立即复用（"突现零等待"）。
pub fn sync_catalog_from_pool(
    mut objects: ResMut<RemoteObjects>,
    mut catalog: ResMut<ModelCatalog>,
) {
    objects.sync_catalog(&mut catalog);
}

#[cfg(test)]
mod tests {
    use super::*;
    use cute_of_duty_contract::net::packet;

    /// 区划恒等式：250 + 6 = 256；16000KB + 384KB = 16MB；每槽 64KB。
    #[test]
    fn layout_constants_are_exact() {
        assert_eq!(SLOT_BYTES, 64 * 1024);
        assert_eq!(IN_USE_SLOTS + PREFETCH_SLOTS, SLOT_COUNT);
        assert_eq!(SLOT_COUNT, 256);
        assert_eq!(SLOT_BYTES * SLOT_COUNT, POOL_BYTES);
        assert_eq!(IN_USE_SLOTS * SLOT_BYTES, 16_000 * 1024, "在用区应恰为 16000KB");
        assert_eq!(PREFETCH_SLOTS * SLOT_BYTES, 384 * 1024, "预取区应恰为 384KB");
        assert_eq!(POOL_BYTES, 16 * 1024 * 1024);
    }

    /// 槽地址恒为 base + i × 64KB，且跨多次插入不变（固定地址）。
    #[test]
    fn slot_addresses_are_fixed() {
        let mut pool = RemotePool::new();
        let base = pool.base_ptr();
        assert_eq!(pool.slot_ptr(0), base);
        assert_eq!(pool.slot_ptr(7) as usize - base as usize, 7 * SLOT_BYTES);

        for i in 0..20u64 {
            let payload = vec![(i % 251) as u8; 128];
            let idx = pool
                .insert(Region::InUse, 1000 + i, ResourceKind::Model, &payload)
                .expect("应可落槽");
            assert_eq!(idx, i as usize, "应顺序取空槽");
        }
        assert_eq!(pool.base_ptr(), base, "基址跨插入应不变");
        assert_eq!(pool.slot_ptr(7) as usize - base as usize, 7 * SLOT_BYTES);
    }

    /// 写入/读取往返一致，同 key 覆盖不新增槽。
    #[test]
    fn insert_and_get_roundtrip_with_overwrite() {
        let mut pool = RemotePool::new();
        let key = packet::resource_key(ResourceKind::Model, "OperativeFire");
        let first = b"first-payload".to_vec();
        let i = pool.insert(Region::InUse, key, ResourceKind::Model, &first).expect("应可落槽");
        assert_eq!(pool.get(key).expect("应命中").0, first.as_slice());

        let second = b"second".to_vec();
        let j = pool.insert(Region::InUse, key, ResourceKind::Model, &second).expect("应可覆盖");
        assert_eq!(i, j, "同 key 应覆盖同槽");
        assert_eq!(pool.get(key).expect("应命中").0, second.as_slice(), "应读到新负载");
        assert_eq!(pool.used(Region::InUse), 1, "覆盖不应新增槽");
    }

    /// 区划隔离：预取区的资源不占在用区，且可用 `get_in` 单独检索。
    #[test]
    fn regions_are_isolated() {
        let mut pool = RemotePool::new();
        let key = 42u64;
        let i = pool.insert(Region::Prefetch, key, ResourceKind::Animation, b"anim").expect("预取区应可落槽");
        assert_eq!(RemotePool::region_of(i), Region::Prefetch);
        assert_eq!(pool.used(Region::InUse), 0, "预取不应计入在用区");
        assert_eq!(pool.used(Region::Prefetch), 1);
        assert!(pool.get_in(Region::InUse, key).is_none(), "在用区不应命中");
        assert!(pool.get_in(Region::Prefetch, key).is_some(), "预取区应命中");
    }

    /// 预取命中提升进在用区，内容与键保持不变，预取槽被释放。
    #[test]
    fn promote_moves_prefetch_into_in_use() {
        let mut pool = RemotePool::new();
        let key = 777u64;
        let payload = b"prefetched-model-bytes".to_vec();
        let p = pool.insert(Region::Prefetch, key, ResourceKind::Model, &payload).expect("预取区应可落槽");
        let used = pool.promote(p).expect("应可提升");
        assert_eq!(RemotePool::region_of(used), Region::InUse);
        assert_eq!(pool.get_in(Region::InUse, key).expect("应命中在用区").0, payload.as_slice());
        assert_eq!(pool.used(Region::Prefetch), 0, "提升后预取槽应释放");
    }

    /// 预取区容量恰为 6：第 7 个应被拒。
    #[test]
    fn prefetch_region_holds_exactly_six() {
        let mut pool = RemotePool::new();
        for i in 0..PREFETCH_SLOTS as u64 {
            assert!(pool.insert(Region::Prefetch, i + 1, ResourceKind::Model, b"x").is_some());
        }
        assert!(pool.insert(Region::Prefetch, 999, ResourceKind::Model, b"y").is_none(), "预取区应已满");
    }

    /// 超槽容量的负载 / 保留键 0 必须被拒。
    #[test]
    fn oversized_or_zero_key_rejected() {
        let mut pool = RemotePool::new();
        let too_big = vec![0u8; SLOT_BYTES + 1];
        assert!(pool.insert(Region::InUse, 5, ResourceKind::Model, &too_big).is_none());
        assert!(pool.insert(Region::InUse, 0, ResourceKind::Model, b"x").is_none(), "键 0 为保留值");
    }

    /// 接收系统：网络线程投递的资源帧写进池，批次终止标记置位目录就绪。
    #[test]
    fn resource_frames_land_in_pool() {
        let (tx, rx) = mpsc::channel();
        let mut objects = RemoteObjects::new(rx);
        let frame = ResourceFrame {
            kind: ResourceKind::Model,
            key: 123,
            region: Region::InUse,
            bytes: b"payload".to_vec(),
        };
        tx.send(PoolMessage::Resource(frame)).expect("发送应成功");
        tx.send(PoolMessage::BatchEnd).expect("发送应成功");
        // 模拟 `receive_resources` 的排空逻辑（不依赖 Bevy 调度）：
        // 持锁只读通道、释放借用后再写池，避免与 `pool` 可变借用冲突。
        let mut drained: Vec<PoolMessage> = Vec::new();
        {
            let rx = objects.rx.lock().expect("锁不应中毒");
            while let Ok(msg) = rx.try_recv() {
                drained.push(msg);
            }
        }
        for msg in drained {
            match msg {
                PoolMessage::Resource(f) => assert!(objects.insert_frame(&f).is_some()),
                PoolMessage::BatchEnd => objects.catalog_ready = true,
            }
        }
        assert_eq!(
            objects.pool.get(123).expect("应命中").0,
            b"payload".as_slice()
        );
        assert!(objects.catalog_ready, "批次终止后目录应标就绪");
    }

    /// 池内模型资源可增量同步进 `ModelCatalog` 视图（同键只搬一次）。
    #[test]
    fn pool_resources_sync_into_catalog() {
        use cute_of_duty_contract::model::{
            ModelPreset, VoxelBone, VoxelCube, VoxelModelSpec, YANHU_SCALE,
        };

        let (_tx, rx) = mpsc::channel();
        let mut objects = RemoteObjects::new(rx);

        // 手工构造一份可解码的模型负载（契约层结构；不依赖服务端的造型解析）。
        let model = VoxelModelSpec {
            preset: ModelPreset::OperativeFire,
            scale: YANHU_SCALE,
            bones: vec![VoxelBone {
                name: "body".to_string(),
                parent: None,
                pivot: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                cubes: vec![VoxelCube {
                    origin: [0.0, 0.0, 0.0],
                    size: [1.0, 1.0, 1.0],
                    mat: Some("cream".to_string()),
                }],
            }],
        };
        let key = cute_of_duty_contract::net::packet::model_key(&model);
        let bytes = serde_json::to_vec(&model).expect("序列化应成功");
        objects
            .insert_frame(&ResourceFrame {
                kind: ResourceKind::Model,
                key,
                region: Region::InUse,
                bytes,
            })
            .expect("应可落槽");

        let mut catalog = crate::flow::ModelCatalog::default();
        objects.sync_catalog(&mut catalog);
        assert_eq!(catalog.models.len(), 1, "应同步一件模型");
        // 再次同步不应重复搬运。
        objects.sync_catalog(&mut catalog);
        assert_eq!(catalog.models.len(), 1, "同键只搬一次");
    }
}
