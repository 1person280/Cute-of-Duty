//! 主题色板：全 HUD / 主菜单 / 仓库共用，对齐旧版 v0.3.x 深海军蓝 + 青色点缀观感。
//!
//! 设计动机：把视觉身份集中到一处，各 UI 模块只引用这些常量，避免散落的硬编码 RGB
//! 导致风格漂移。色彩是稳定表现层资源，与服务端权威无关。

use bevy::prelude::Color;

/// 全局底色（深海军蓝，旧版 `srgb(0.04,0.06,0.09)`）。
pub const BG_DEEP: Color = Color::srgb(0.04, 0.06, 0.09);
/// 主点缀青（旧版 `#4dd0e1`）。
pub const ACCENT_CYAN: Color = Color::srgb(0.30, 0.82, 0.88);
/// 面板底（比全局底色略亮、半透明观感）。
pub const PANEL_BG: Color = Color::srgb(0.09, 0.11, 0.16);
/// 面板描边（低对比深青）。
pub const PANEL_BORDER: Color = Color::srgb(0.16, 0.24, 0.32);
/// 行高亮（选中/悬停）。
pub const ROW_HOVER: Color = Color::srgb(0.14, 0.20, 0.28);
/// 主文字白。
pub const TEXT_WHITE: Color = Color::srgb(0.92, 0.93, 0.95);
/// 次级文字灰。
pub const TEXT_DIM: Color = Color::srgb(0.58, 0.64, 0.72);
/// 击杀计数（右上橙黄，旧版 `palette` 击杀橙）。
pub const KILL_AMBER: Color = Color::srgb(0.95, 0.72, 0.25);
/// 任务/导航提示金黄。
pub const TASK_GOLD: Color = Color::srgb(1.00, 0.95, 0.60);
/// UI 强调琥珀（美术色板「UI 强调」#FFCC40）：越肩瞄准态准星等高优先级提示用色。
pub const ACCENT_AMBER: Color = Color::srgb(1.00, 0.80, 0.25);