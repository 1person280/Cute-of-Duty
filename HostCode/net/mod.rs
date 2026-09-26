//! 网络子模块 —— 后台快照/事件拉取、输入上行、延迟面板
//!
//! 设计动机（Why）：客户端只消费服务端权威快照与事件，绝不本地模拟实体状态。本模块
//! 承载三路通道（快照/下行控制/上行意图）的消费端：拉快照出画、握手/事件/撤离回程
//! 路由、CapsLock 延迟面板，以及把玩家输入上行给服务端。

pub(crate) mod latency;
pub(crate) mod network;
pub(crate) mod pilot;
pub(crate) mod snapshot;

pub(crate) use latency::{caps_toggle, panel_update, spawn_panel};
pub(crate) use network::{ClientInbound, ControlBuffer, NetOut, run_pull_loop};
pub(crate) use pilot::input_system;
pub(crate) use snapshot::{
    CubeMesh, EntityMaterials, SnapshotBuffer, apply_entities, receive_snapshots,
};
