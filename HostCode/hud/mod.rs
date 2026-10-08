//! 战斗内 HUD 子模块 —— 血条/技能/小地图/准星/击杀/交互/撤离
//!
//! 设计动机（Why）：HUD 是纯表现层，所有数值（血量/弹药/CD/击杀数）都来自服务端快照，
//! 本模块只负责把快照画到 UI，并就地触发撤离请求（距离判定由服务端权威裁决）。
//!
//! 命名约定：单词语义概念直接作文件名；两词概念单开子目录（如 `kill/counter.rs`），
//! 不再用下划线拼接文件名。

pub(crate) mod alert;
pub(crate) mod backpack;
pub(crate) mod bigmap;
pub(crate) mod button;
pub(crate) mod crosshair;
pub(crate) mod feedback;
pub(crate) mod grenade;
pub(crate) mod interact;
pub(crate) mod item;
pub(crate) mod kill;
pub(crate) mod loot;
pub(crate) mod minimap;
pub(crate) mod modal;
pub(crate) mod root;
pub(crate) mod skills;
pub(crate) mod vitals;

pub(crate) use alert::update_alert;
pub(crate) use backpack::panel::{
    backpack_toggle, backpack_use_hovered, reset_backpack_panel, sync_backpack_panel,
    BackpackPanelRoot, BackpackPanelState,
};
pub(crate) use bigmap::{
    bigmap_toggle, gameplay_input_active, reset_bigmap, update_bigmap, BigMapOpen, BigMapRoot,
};
pub(crate) use button::panel::{
    button_panel_click, button_panel_emit, button_panel_toggle, reset_button_panel,
    sync_button_panel, ButtonPanelState,
};
pub(crate) use crosshair::update_crosshair;
pub(crate) use feedback::update_hit_feedback;
pub(crate) use grenade::hint::{reset_held_grenade, sync_held_grenade, HeldGrenadeState};
pub(crate) use interact::panel::{sync_interact_menu, sync_interact_panel};
pub(crate) use interact::{
    interact_commit, interact_input, interact_menu_click, reset_interact,
    update_interact_entries, InteractState,
};
pub(crate) use item::wheel::{
    item_wheel_input, reset_item_wheel, update_item_wheel, ItemWheelState,
};
pub(crate) use kill::counter::update_kill;
pub(crate) use loot::panel::{
    loot_panel_drag, loot_panel_input, reset_loot_panel, sync_loot_panel, LootPanelState,
};
pub(crate) use minimap::update_minimap;
pub(crate) use modal::publish::publish_modal_changes;
pub(crate) use root::{extract_interaction, spawn_hud, update_extract};
pub(crate) use skills::{update_feed, update_skills};
pub(crate) use vitals::{update_vitals_bars, update_vitals_text};