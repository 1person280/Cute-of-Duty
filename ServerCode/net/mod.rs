//! 网络层（服务端权威）
//!
//! 严格对齐 README「四、网络架构规划」：
//! - 基于 TCP 的服务器权威架构（参考 Minecraft 可靠同步），而非 UDP 快节奏同步；
//! - 服务端是唯一真理源，客户端只发输入、只收服务端算好的结果（快照）；
//! - `aoi` 兴趣区域剔除：每个客户端只收到其视野范围内的实体数据（根治 ESP 透视）。
//!
//! 线格式采用 **JSON 行（NDJSON）** 帧协议，便于无头阶段调试与后续升级。

pub mod protocol;
pub mod aoi;
pub mod broadcaster;
pub mod session;

pub use broadcaster::build_snapshot;
pub use protocol::{ClientMessage, EntitySnapshot, PlayerInput, ServerMessage};
pub use session::{NetCommand, NetRuntime, accept_loop};