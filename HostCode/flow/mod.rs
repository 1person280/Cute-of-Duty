//! 游玩流程子模块 —— 客户端表现层状态机与全局资源
//!
//! 设计动机（Why）：状态机（加载/主菜单/训练场）是纯客户端决策，但选装/进场/撤离
//! 的资格与距离判定一律由服务端权威裁决。本模块只持有全局资源（本人实体/延迟书签/
//! 通告/中文字体/上行序号）、消费下行控制消息并推动状态机，不做任何模拟判定。

pub(crate) mod flow_state;
pub(crate) mod loading;

pub(crate) use flow_state::{AppState, CjkFont, route_control_messages, setup_global};
pub(crate) use loading::{loading_tick, spawn_loading};
