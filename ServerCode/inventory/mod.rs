//! 背包 CRUD 域模块（热数据操作 + 一致性守卫）。
//!
//! Why: 背包的正误（元素/上限/余额）由服务端权威裁决，本模块是提纯后的领域服务，
//! 只依赖 `equipment`（装备热实例注册表）与 `player`（冷档案热副本），不感知网络
//! 线格式。主循环在此结算后经 Event 回执播报，断开时把同一份热副本落盘到冷仓库。

mod service;

pub use service::{adjust_currency, craft, discard_by_index, CraftRequest, InventoryError};