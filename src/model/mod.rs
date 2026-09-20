//! 干员模型与动作模块 —— 玩家角色体素建模、程序化动作与换装系统
//!
//! 从 3D Demo（src/demo）解耦而来，feature "demo" 门控（仅 Demo 需要 bevy）。
//! 职责：
//! - 四名干员的专属体素模型（焰狐/霜刃/雷豹/毒蛛），共用同一方块骨架与枢轴约定
//! - yanhu_action_system：行走/疾跑/跳跃/瞄准持枪/开火后坐/尾巴摇摆（全干员通用）
//! - operator_model_swap_system：切换干员时整体换模型、按干员重着色
//!
//! 与 demo 的接口：demo 玩法层通过本模块的组件（PlayerMovement/PlayerCamera/
//! OperatorState 等）驱动模型，本模块不包含任何玩法逻辑。
//!
//! 组织（反屎山扁平化拆分，见各子文件）：
//! - palette：干员/场景统一色板常量
//! - components：玩家/干员玩法驱动组件（PlayerCamera/PlayerMovement/OperatorState 等）
//! - rig：骨骼枢轴组件（头部/持枪/四肢/尾链 + PlayerModelRoot）
//! - operator_models：四名干员的体素模型构建
//! - operator_swap：切换干员时的模型置换系统
//! - yanhu_action：全干员通用的程序化动作系统

pub mod palette;

mod components;
mod rig;
mod operator_models;
mod operator_swap;
mod yanhu_action;

pub use components::*;
pub use rig::*;
pub use operator_models::*;
pub use operator_swap::*;
pub use yanhu_action::*;