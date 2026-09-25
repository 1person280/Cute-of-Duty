//! Cute Of Duty 1: Simple — 服务端权威模拟库
//!
//! 自 0.6.0 与客户端物理分离：**所有模拟真相源（元素反应、伤害、地图、战局、
//! 背包、信誉等热数据）只由本 crate 持有并计算**，客户端（`HostCode`）只做
//! 表现层，通过 `net::protocol` 定义的线格式接收服务端算好的结果来显示。
//!
//! 网络架构严格对齐 README「四、网络架构规划」：基于 TCP 的服务器权威架构 +
//! AOI 兴趣区域剔除（客户端只收到其视野范围内的实体数据，根治 ESP 透视外挂）。
//!
//! 核心业务模块与旧 `src/lib.rs` 逐一对应迁入；`net` 为 0.6.0 新增网络层。

pub mod combat;
pub mod config;
pub mod damage;
pub mod element;
pub mod engine;
pub mod entity;
pub mod equipment;
pub mod gamemode;
pub mod hal;
pub mod inventory;
pub mod map;
pub mod model;
pub mod net;
pub mod operator;
pub mod player;
pub mod storage;