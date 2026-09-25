//! 共享资源子模块 —— 配色、UI 贴图、干员元数据
//!
//! 设计动机（Why）：主题色板 / UI 贴图 / 干员静态元数据是跨多个表现层模块复用的稳定
//! 冷资源，独立成模块避免循环依赖，集中管理本地资源加载。

pub(crate) mod operator_meta;
pub(crate) mod theme;
pub(crate) mod ui_assets;

pub(crate) use ui_assets::{init_ui_assets, refresh_ui_ready};
