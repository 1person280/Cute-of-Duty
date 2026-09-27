//! 战斗内 HUD 子模块 —— 血条/技能/小地图/准星/击杀/交互/撤离
//!
//! 设计动机（Why）：HUD 是纯表现层，所有数值（血量/弹药/CD/击杀数）都来自服务端快照，
//! 本模块只负责把快照画到 UI，并就地触发撤离请求（距离判定由服务端权威裁决）。

pub(crate) mod hud_alert;
pub(crate) mod hud_backpack_panel;
pub(crate) mod hud_bigmap;
pub(crate) mod hud_button_panel;
pub(crate) mod hud_crosshair;
pub(crate) mod hud_grenade_hint;
pub(crate) mod hud_interact;
pub(crate) mod hud_interact_panel;
pub(crate) mod hud_item_wheel;
pub(crate) mod hud_kill_counter;
pub(crate) mod hud_loot_panel;
pub(crate) mod hud_minimap;
pub(crate) mod hud_root;
pub(crate) mod hud_skills;
pub(crate) mod hud_vitals;

pub(crate) use hud_alert::update_alert;
pub(crate) use hud_backpack_panel::{
    backpack_toggle, backpack_use_hovered, reset_backpack_panel, sync_backpack_panel,
    BackpackPanelRoot, BackpackPanelState,
};
pub(crate) use hud_bigmap::{
    bigmap_toggle, gameplay_input_active, reset_bigmap, update_bigmap, BigMapOpen, BigMapRoot,
};
pub(crate) use hud_button_panel::{
    button_panel_click, button_panel_emit, button_panel_toggle, reset_button_panel,
    sync_button_panel, ButtonPanelState,
};
pub(crate) use hud_root::{extract_interaction, spawn_hud, update_extract};
pub(crate) use hud_crosshair::update_crosshair;
pub(crate) use hud_grenade_hint::{
    reset_held_grenade, sync_held_grenade, HeldGrenadeState,
};
pub(crate) use hud_interact::{
    interact_commit, interact_input, interact_menu_click, reset_interact,
    update_interact_entries, InteractState,
};
pub(crate) use hud_interact_panel::{sync_interact_menu, sync_interact_panel};
pub(crate) use hud_item_wheel::{
    item_wheel_input, reset_item_wheel, update_item_wheel, ItemWheelState,
};
pub(crate) use hud_kill_counter::update_kill;
pub(crate) use hud_loot_panel::{
    loot_panel_drag, loot_panel_input, reset_loot_panel, sync_loot_panel, LootPanelState,
};
pub(crate) use hud_minimap::update_minimap;
pub(crate) use hud_skills::{update_feed, update_skills};
pub(crate) use hud_vitals::{update_vitals_bars, update_vitals_text};
