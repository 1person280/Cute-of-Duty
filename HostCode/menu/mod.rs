//! 主菜单子模块 —— 主菜单、模式选择、仓库选装、设置浮层
//!
//! 设计动机（Why）：主菜单与仓库选装属于表现层 UI，但选装结果 / 进场资格由服务端权威
//! 裁决。本模块负责主界面布局、交互反馈、仓库双清单（仓库 + 背包）浮层，以及灵敏度 /
//! FOV / 环境亮度等设置的浮层与控制，仅把选装意图上行，不做本地判定。

pub(crate) mod arsenal;
pub(crate) mod game_settings;
pub(crate) mod menu_behaviour;
pub(crate) mod menu_main;
pub(crate) mod mode_panel;
pub(crate) mod pause;

pub(crate) use arsenal::{
    arsenal_drag_system, arsenal_interaction, ensure_overlay, ArsenalDrag, ArsenalSelection,
    ArsenalVisible,
};
pub(crate) use game_settings::{
    GameSettings, settings_apply_ambient, settings_apply_fov,
};
pub(crate) use menu_behaviour::{
    main_menu_interaction, main_menu_loadout, main_menu_style, tick_grace,
};
pub(crate) use menu_main::spawn_menu;
pub(crate) use mode_panel::{SelectedCategory, SelectedMode};
pub(crate) use pause::{
    pause_closed, pause_menu_interaction, pause_toggle, teardown_pause, PauseMenu,
};
