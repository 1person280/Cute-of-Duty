//! 游玩流程子模块 —— 客户端表现层状态机与全局资源
//!
//! 设计动机（Why）：状态机（加载/主菜单/训练场）是纯客户端决策，但选装/进场/撤离
//! 的资格与距离判定一律由服务端权威裁决。本模块只持有全局资源（本人实体/延迟书签/
//! 通告/中文字体/上行序号）、消费下行控制消息并推动状态机，不做任何模拟判定。

pub(crate) mod loading;
pub(crate) mod modal;
pub(crate) mod settings;
pub(crate) mod state;

// 旧公开路径别名：改名前的 `flow::flow_state` 经 `pub use ... as ...` 保持可用，
// 避免全仓调用方被动改动（公开路径不变式）。
pub(crate) use state as flow_state;

pub(crate) use state::{
    route_control_messages, setup_global, AppState, CjkFont, LoadoutPresets, LocalPlayer,
    ModelCatalog,
};
pub(crate) use settings::{apply_setting_step, setting_label, setting_step, GameSettings, SettingKind};
pub(crate) use loading::{loading_tick, spawn_loading};
pub(crate) use modal::{
    apply_modal_changes, ModalChange, ModalKind, ModalState, PauseOpenRequest,
};