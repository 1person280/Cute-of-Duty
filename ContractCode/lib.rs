//! Cute Of Duty 契约 crate —— 客户端与服务端**唯一的共同依赖**
//!
//! 设计动机（Why）：见 [ADR 0003](../../docs/adr/0003-contract-crate.md)。本 crate 是
//! `docs/contracts/*.yaml` 的 Rust 实现，承载两端必须一致的**纯数据**：
//! 线格式类型、跨域载荷枚举、共享常量、Port Trait。
//!
//! 硬性边界（违反即破坏"拆微服务无需重写"的前提）：
//! - **零 bevy、零模拟逻辑、零 I/O**（无 `std::fs`、无网络、无随机、无时间）；
//! - 依赖仅 `serde` / `serde_json`；
//! - 只被依赖，不依赖 `HostCode` 或 `ServerCode`。
//!
//! 模块职责：
//! - [`net`]：线格式（256B 主通道 + 4096B 资源通道、指令二进制编解码、组包调度、资源重组）。
//! - [`map`] / [`model`] / [`element`] / [`equipment`] / [`items`] / [`interact`] / [`operator`]：
//!   跨域载荷类型与共享常量（`LootItem` / `StationKind` / `InteractChoice` / 名册 / 弹道常数…）。
//! - [`combat`]：客户端表现需与服务端**同源**的战斗常数（手雷弹道）。
//! - [`port`]：跨模块/跨进程能力抽象（`XxxPort`），实现可替换、可 mock。

pub mod combat;
pub mod element;
pub mod equipment;
pub mod interact;
pub mod items;
pub mod map;
pub mod model;
pub mod net;
pub mod operator;
pub mod port;