//! 网络层（服务端权威）
//!
//! 严格对齐 README「四、网络架构规划」：
//! - 基于 TCP 的服务器权威架构（参考 Minecraft 可靠同步），而非 UDP 快节奏同步；
//! - 服务端是唯一真理源，客户端只发输入、只收服务端算好的结果（快照）；
//! - `aoi` 兴趣区域剔除：每个客户端只收到其视野范围内的实体数据（根治 ESP 透视）。
//!
//! 线格式采用**统一固定 64KB 槽帧**（见 [`packet`]）：资源一资源一帧、控制指令
//! （~256B）也打包进 64KB 帧，超长消息按 `continuation` 分片重组。客户端据此把资源帧
//! 写入固定地址槽位池（见 HostCode `net::remote`），实体因 AOI 突现即复用、不等加载。

pub mod protocol;
pub mod aoi;
pub mod broadcaster;
pub mod packet;
pub mod prefetch;
pub mod session;

pub use broadcaster::build_snapshot;
pub use packet::{
    FrameHeader, FrameKind, FrameReader, FrameWriter, Region, ResourceKind, encode_client,
    encode_server, decode_client, decode_server,
};
pub use prefetch::predict_prefetch;
pub use protocol::{ClientMessage, EntitySnapshot, PlayerInput, ServerMessage};
pub use session::{NetCommand, NetRuntime, accept_loop};