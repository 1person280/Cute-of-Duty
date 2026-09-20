//! HUD：准星/血条/弹药/技能栏构建、每帧更新、底部物品栏、击杀播报
//!
//! 目录模块：按内聚主题拆分为多个子文件，本文件仅负责模块声明与逐层重新导出，
//! 保证父模块 `demo::mod.rs` 的 `use hud::*` 继续把所有 hud 符号带入作用域。

mod effect_assets;
mod flash;
mod hud_ui;
mod item_slots;
mod kill_feed;
mod vitals;

// 逐层重新导出：保持 `crate::demo::hud::{...}` 与 `use hud::*` 可见性一致
pub(crate) use effect_assets::*;
pub(crate) use flash::*;
pub(crate) use hud_ui::*;
pub(crate) use item_slots::*;
pub(crate) use kill_feed::*;
pub(crate) use vitals::*;