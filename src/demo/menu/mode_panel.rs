//! 模式/分类选择面板的数据与组件：模式清单、分类标签、选中态资源与查找。

use bevy::prelude::*;

/// 游戏模式 id：面板选中项 + "开始游戏"的入口分发
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub(crate) enum GameModeId {
    #[default]
    Training,
    Campaign,
    Evacuation,
}

/// 模式元数据：名称 / 描述 / 分类下标 / 是否已开放
pub(crate) struct ModeSpec {
    pub(crate) id: GameModeId,
    pub(crate) name: &'static str,
    pub(crate) desc: &'static str,
    pub(crate) category: usize,
    pub(crate) available: bool,
}

/// 全部游戏模式（分类下标对应 MODE_CATEGORIES，0 为"全部"）
pub(crate) const GAME_MODES: [ModeSpec; 3] = [
    ModeSpec { id: GameModeId::Training, name: "训练场", desc: "单人 · 射击与元素反应演练", category: 1, available: true },
    ModeSpec { id: GameModeId::Campaign, name: "战役模式", desc: "章节化 PVE 战役", category: 2, available: false },
    ModeSpec { id: GameModeId::Evacuation, name: "多人撤离", desc: "组队搜刮 · 带装撤离", category: 3, available: false },
];

/// 模式分类标签，0 号为"全部"（不过滤）
pub(crate) const MODE_CATEGORIES: [&str; 4] = ["全部", "演练", "战役", "撤离"];

/// 当前选中的游戏模式（会话内保留，返回主界面后记住上次选择）
#[derive(Resource, Default)]
pub(crate) struct SelectedMode(pub(crate) GameModeId);

/// 当前选中的分类下标
#[derive(Resource, Default)]
pub(crate) struct SelectedCategory(pub(crate) usize);

/// "切换模式"按钮：呼出/收起右侧模式面板
#[derive(Component)]
pub(crate) struct SwitchModeButton;

/// "开始游戏"按钮：进入当前选中的模式
#[derive(Component)]
pub(crate) struct StartGameButton;

/// 模式分类按钮：按下标过滤模式列表
#[derive(Component)]
pub(crate) struct CategoryButton(pub(crate) usize);

/// 模式面板根节点标记（样式系统读取其显隐，联动模式行可见性）
#[derive(Component)]
pub(crate) struct ModePanelRoot;

/// 模式列表中的一行：点击选中该模式
#[derive(Component)]
pub(crate) struct ModeRow(pub(crate) GameModeId);

/// 按模式 id 查元数据表
pub(crate) fn game_mode_spec(id: GameModeId) -> &'static ModeSpec {
    GAME_MODES.iter().find(|spec| spec.id == id).expect("未知的游戏模式 id")
}