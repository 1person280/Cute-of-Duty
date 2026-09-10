//! Cute Of Duty 1: Simple
//! 战术撤离射击游戏 - 库入口
//!
//! 核心差异化：元素互斥生态 + 反护航经济架构
//!
//! 所有游戏逻辑模块集中在此，供 `cod1`（无头模拟）与
//! `cod1-demo`（Bevy 3D Demo）共用，
//! 保证两边的元素反应、伤害结算走同一套配置表驱动代码。
//!
//! 3D Demo 与干员模型/动作模块位于 `demo` / `model`，
//! 由 feature "demo" 门控：默认 `cargo build/test` 不编译 bevy，
//! `cargo run --features demo` 才构建 3D Demo。

pub mod config;
pub mod damage;
pub mod element;
pub mod engine;
pub mod entity;
pub mod equipment;
pub mod gamemode;
pub mod hal;
pub mod map;
pub mod operator;
pub mod player;

#[cfg(feature = "demo")]
pub mod demo;
#[cfg(feature = "demo")]
pub mod model;
