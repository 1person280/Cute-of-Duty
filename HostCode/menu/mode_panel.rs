//! 模式/分类选择面板的数据与组件：模式清单、分类标签、选中态资源与查找。
//!
//! 设计动机（Why）：模式面板是主菜单的选择模式入口。本模块只静态声明模式元数据
//! （哪些已开放、归到哪个分类）并持有会话内选中态。「开始游戏」的进场资格仍在
//! 服务端裁决：客户端把选中的「训练场」吸收为 `StartTraining` 上行，不可用模式
//! 只做「敬请期待」占位、绝不发送协议 —— 本模块不参与任何本地玩法裁决，保持
//! 服务端权威红线。

use bevy::prelude::*;

/// 游戏模式 id：面板选中项 + 「开始游戏」的入口分发。
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum GameModeId {
    #[default]
    Training,
    Campaign,
    Evacuation,
}

/// 模式元数据：名称 / 描述 / 分类下标 / 是否已开放。
pub struct ModeSpec {
    pub id: GameModeId,
    pub name: &'static str,
    pub desc: &'static str,
    pub category: usize,
    pub available: bool,
}

/// 全部游戏模式（分类下标对应 MODE_CATEGORIES，0 为「全部」不过滤）。
pub const GAME_MODES: [ModeSpec; 3] = [
    ModeSpec { id: GameModeId::Training, name: "训练场", desc: "单人 · 射击与元素反应演练", category: 1, available: true },
    ModeSpec { id: GameModeId::Campaign, name: "战役模式", desc: "章节化 PVE 战役", category: 2, available: false },
    ModeSpec { id: GameModeId::Evacuation, name: "多人撤离", desc: "组队搜刮 · 带装撤离", category: 3, available: false },
];

/// 模式分类标签，0 号为「全部」（不过滤）。
pub const MODE_CATEGORIES: [&str; 4] = ["全部", "演练", "战役", "撤离"];

/// 当前选中的游戏模式（会话内保留，返回主界面后记住上次选择）。
#[derive(Resource, Default)]
pub struct SelectedMode(pub GameModeId);

/// 当前选中的分类下标。
#[derive(Resource, Default)]
pub struct SelectedCategory(pub usize);

/// 「切换模式」按钮：呼出/收起右侧模式面板。
#[derive(Component)]
pub struct SwitchModeButton;

/// 「开始游戏」按钮：进入当前选中的模式。
#[derive(Component)]
pub struct StartGameButton;

/// 模式分类按钮：按下标过滤模式列表。
#[derive(Component)]
pub struct CategoryButton(pub usize);

/// 模式面板根节点标记（样式系统读取其显隐，联动模式行可见性）。
#[derive(Component)]
pub struct ModePanelRoot;

/// 模式列表中的一行：点击选中该模式。
#[derive(Component)]
pub struct ModeRow(pub GameModeId);

/// 按模式 id 查元数据表。
pub fn game_mode_spec(id: GameModeId) -> &'static ModeSpec {
    GAME_MODES
        .iter()
        .find(|spec| spec.id == id)
        .expect("未知的游戏模式 id")
}