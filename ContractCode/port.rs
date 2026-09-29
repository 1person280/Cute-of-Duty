//! Port Trait 集合（跨模块/跨进程能力的抽象接口）
//!
//! 设计动机（Why）：见 [ADR 0003](../../docs/adr/0003-contract-crate.md) 第 1 节——
//! 契约 crate 除了纯数据类型，还收敛"能力抽象"，使实现可替换、可 mock，
//! 为"拆微服务无需重写"预置 seam。

use crate::net::protocol::EntitySnapshot;

/// 权威快照来源（Port）。
///
/// 把"最新权威状态从哪来"抽象成接口：服务端实现为内存快照缓冲，客户端实现为
/// 下行解码结果，测试可用固定帧 mock。调用方只依赖本 trait，不感知具体传输。
pub trait SnapshotSource {
    /// 取最近一帧快照（`seq` + 实体条目）；尚无快照返回 `None`。
    fn latest_snapshot(&self) -> Option<(u64, Vec<EntitySnapshot>)>;
}