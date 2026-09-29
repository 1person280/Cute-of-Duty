//! 地图模块（契约只读副本）
//!
//! 设计动机（Why）：地图定义是**纯数据**（网格/材质/光照语义，无渲染、无 I/O），
//! 已整体迁至契约 crate `cute_of_duty_contract::map`，使客户端无需依赖服务端 crate
//! 即可获得静态场景布局。
//!
//! **以服务端为准，待改为下发**：见 [ADR 0003](../../docs/adr/0003-contract-crate.md)
//! 第 3 节——过渡期保留契约内只读副本，后续版本可改为服务端下发。

pub use cute_of_duty_contract::map::*;