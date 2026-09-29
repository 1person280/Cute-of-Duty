//! 干员与武器档案系统（契约只读副本）
//!
//! 设计动机（Why）：干员名册与步枪档案已迁至契约 crate
//! `cute_of_duty_contract::operator`，使客户端不再直接依赖服务端 crate。
//!
//! **以服务端为准，待改为下发**：名册本应由服务端经握手/快照权威下发
//! （见 [ADR 0003](../../docs/adr/0003-contract-crate.md) 第 3 节）；本轮采用
//! "契约内只读副本"过渡，后续版本改为下发后再删本地副本。

pub use cute_of_duty_contract::operator::*;