//! 全局游玩设置 —— [`GameSettings`] 资源与步进/文本规则。
//!
//! 设计动机（Why）：设置是**跨模块的表现层全局状态**（`menu` 的设置面板改它，`world` 的鼠标视角
//! 读它，FOV/环境亮度应用系统消费它）。此前它挂在 `menu` 下，迫使 `world` 反向 `use crate::menu`，
//! 形成同层横向依赖。设置本身属"应用级全局资源"，与 `AppState` / `CjkFont` 同类，故归入 `flow`。
//!
//! 边界：本文件只承载**数据与规则**（值域、步进、显示文本）；设置面板的 UI 构建器留在 `menu`。

use bevy::prelude::*;

/// 可调设置项：每帧由「值文本」与「步进按钮」共同呈现。
#[derive(Component, Clone, Copy, PartialEq)]
pub enum SettingKind {
    Sensitivity,
    Fov,
    Ambient,
}

/// 局内可调设置值（会话内保留；默认值与 v0.3.2 一致）。
#[derive(Resource)]
pub struct GameSettings {
    pub mouse_sensitivity: f32,
    pub fov_deg: f32,
    pub ambient_brightness: f32,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self { mouse_sensitivity: 1.0, fov_deg: 45.0, ambient_brightness: 0.55 }
    }
}

/// 每项设置的步进量。
pub fn setting_step(kind: SettingKind) -> f32 {
    match kind {
        SettingKind::Sensitivity => 0.1,
        SettingKind::Fov => 5.0,
        SettingKind::Ambient => 0.05,
    }
}

/// 设置项当前值的显示文本。
pub fn setting_label(settings: &GameSettings, kind: SettingKind) -> String {
    match kind {
        SettingKind::Sensitivity => format!("x{:.1}", settings.mouse_sensitivity),
        SettingKind::Fov => format!("{:.0}", settings.fov_deg),
        SettingKind::Ambient => format!("{:.2}", settings.ambient_brightness),
    }
}

/// 应用一步增量，并 clamp 到合理区间。
pub fn apply_setting_step(settings: &mut GameSettings, kind: SettingKind, delta: f32) {
    match kind {
        SettingKind::Sensitivity => settings.mouse_sensitivity = (settings.mouse_sensitivity + delta).clamp(0.2, 3.0),
        SettingKind::Fov => settings.fov_deg = (settings.fov_deg + delta).clamp(40.0, 110.0),
        SettingKind::Ambient => settings.ambient_brightness = (settings.ambient_brightness + delta).clamp(0.10, 1.20),
    }
}