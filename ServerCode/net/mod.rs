//! 网络层（服务端权威）
//!
//! 严格对齐 README「四、网络架构规划」：
//! - 基于 TCP 的服务器权威架构（参考 Minecraft 可靠同步），而非 UDP 快节奏同步；
//! - 服务端是唯一真理源，客户端只发输入、只收服务端算好的结果（快照）；
//! - `aoi` 兴趣区域剔除：每个客户端只收到其视野范围内的实体数据（根治 ESP 透视）。
//!
//! 0.12 线格式为**小定长包 + 双通道**（见 [`packet`]）：
//! - **主/控制通道**：每包恒定 256B = 32B 头 + 7×32B 单元，指令优先于数据组包（[`scheduler`]）；
//! - **资源通道**：独立 TCP 连接，每包恒定 4096B，由 [`resource_stream`] 多次切片后落进客户端
//!   64KB 固定槽位池（见 HostCode `net::remote`）。
//!
//! 两条通道同监听一个端口，按**首条绑定包**区分角色（见 [`session`]）。
//!
//! 0.13.0（[ADR 0003]）起，线格式的**类型与纯编解码**（`codec` / `packet` / `protocol` /
//! `resource_stream` / `scheduler`）迁至契约 crate `cute_of_duty_contract::net`；本模块
//! 通过 `pub use` 保持既有公开路径不变，并只保留**服务端专属编排**（会话 / AOI / 预取）。
//!
//! [ADR 0003]: ../../docs/adr/0003-contract-crate.md

pub mod aoi;
pub mod broadcaster;
pub mod prefetch;
pub mod session;

/// 线格式契约（迁至 `cute_of_duty_contract::net`，此处保持 `crate::net::*` 路径可用）。
pub use cute_of_duty_contract::net::{codec, packet, protocol, resource_stream, scheduler};

pub use broadcaster::build_snapshot;
pub use cute_of_duty_contract::net::packet::{Region, ResourceKind, ResourcePayload, Unit};
pub use cute_of_duty_contract::net::protocol::{
    ClientMessage, EntitySnapshot, PlayerInput, ServerMessage,
};
pub use prefetch::predict_prefetch;
pub use session::{NetCommand, NetRuntime, ResourceJob, accept_loop};