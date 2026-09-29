//! 线格式契约（双端共用的传输层）
//!
//! 0.12 线格式为**小定长包 + 双通道**（见 [`packet`]）：
//! - **主/控制通道**：每包恒定 256B = 32B 头 + 7×32B 单元，指令优先于数据组包（[`scheduler`]）；
//! - **资源通道**：独立 TCP 连接，每包恒定 4096B，由 [`resource_stream`] 多次切片后落进客户端
//!   64KB 固定槽位池（见 HostCode `net::remote`）。
//!
//! 本模块只含**纯字节 ↔ 消息**的编解码与组包/重组，不含会话、不做 I/O。
//! 服务端会话编排（`accept_loop` / AOI / 预取）留在 `ServerCode::net`。

pub mod codec;
pub mod packet;
pub mod protocol;
pub mod scheduler;
pub mod stream;

/// 旧公开路径别名：`net::resource_stream` 经 `pub use ... as ...` 保持可用
/// （拆分/改名不破坏既有公开路径，见反屎山公约「公开路径不变式」）。
pub use stream as resource_stream;

pub use packet::{Region, ResourceKind, ResourcePayload, Unit};
pub use protocol::{ClientMessage, EntitySnapshot, PlayerInput, ServerMessage};