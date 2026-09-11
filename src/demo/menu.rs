//! 主菜单与加载屏：状态数据、UI 构建、交互与样式系统

use bevy::prelude::*;
use super::pause::{GameSettings, SettingKind, spawn_setting_row};
use super::pause::{spawn_credits_panel, SettingValueText, SettingAdjust, apply_setting_step, setting_label};
use super::common::*;

#[derive(Resource)]
pub(crate) struct MenuCamera(pub(crate) Entity);

/// 加载页可更新节点的句柄
#[derive(Resource)]
pub(crate) struct LoadingScreen {
    pub(crate) root: Entity,
    pub(crate) step: Entity,
    pub(crate) fill: Entity,
    pub(crate) percent: Entity,
}

/// 加载计时器
#[derive(Resource)]
pub(crate) struct LoadingTimer(pub(crate) Timer);

/// 主菜单各可更新节点句柄（进入游戏时随根节点统一销毁）
#[derive(Resource)]
pub(crate) struct MainMenuUi {
    pub(crate) root: Entity,
    /// 右侧 60% 模式面板（"切换模式"呼出/收起）
    pub(crate) mode_panel: Entity,
    /// 齿轮呼出的设置浮层
    pub(crate) settings_overlay: Entity,
    /// 设置浮层的全屏压暗层（与浮层同步显隐）
    pub(crate) settings_backdrop: Entity,
    /// 右下角"当前模式"状态行
    pub(crate) status_text: Entity,
}

/// 菜单刚打开时的输入保护期：拦截启动瞬间残留的按键/按下状态误触按钮
#[derive(Resource)]
pub(crate) struct MenuGrace(pub(crate) Timer);

/// 可交互菜单按钮（无此标记的按钮为占位项，不参与交互）
#[derive(Component)]
pub(crate) struct MenuButton;

/// 退出按钮
#[derive(Component)]
pub(crate) struct QuitButton;

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

/// 右上角齿轮按钮：打开设置浮层
#[derive(Component)]
pub(crate) struct GearButton;

/// 齿轮图标图片节点（悬停变色用）
#[derive(Component)]
pub(crate) struct GearIcon;

/// 设置浮层的"返回"按钮
#[derive(Component)]
pub(crate) struct SettingsCloseButton;

/// 模式分类按钮：按下标过滤模式列表
#[derive(Component)]
pub(crate) struct CategoryButton(pub(crate) usize);

/// 模式面板根节点标记（样式系统读取其显隐，联动模式行可见性）
#[derive(Component)]
pub(crate) struct ModePanelRoot;

/// 模式列表中的一行：点击选中该模式
#[derive(Component)]
pub(crate) struct ModeRow(pub(crate) GameModeId);

/// 右下角"当前模式"状态行（选中变化/非法进入时更新）
#[derive(Component)]
pub(crate) struct StatusText;

/// 加载分步文案：真实初始化很轻，按统一节奏展示各子系统就位
pub(crate) const LOADING_STEPS: [&str; 5] = [
    "初始化引擎核心…",
    "加载元素反应配置…",
    "构建训练场地图…",
    "准备干员与武器档案…",
    "校准 HUD 与小地图…",
];

pub(crate) const LOADING_DURATION_SECS: f32 = 2.8;

pub(crate) fn menu_accent() -> Color {
    Color::srgb(0.30, 0.78, 1.0)
}

/// 可交互按钮的两态配色（构建与交互系统共用，保证视觉一致）
pub(crate) fn menu_button_palette(hovered: bool) -> (BackgroundColor, BorderColor) {
    if hovered {
        (
            BackgroundColor(Color::srgba(0.14, 0.20, 0.28, 0.98)),
            BorderColor(menu_accent()),
        )
    } else {
        (
            BackgroundColor(Color::srgba(0.10, 0.14, 0.20, 0.95)),
            BorderColor(Color::srgb(0.22, 0.28, 0.36)),
        )
    }
}

pub(crate) fn setup_menu_camera(menu: Option<Res<MenuCamera>>, mut commands: Commands) {
    // 幂等：首次启动由 Startup 链创建；从游戏返回主菜单时这里重建
    if menu.is_some() {
        return;
    }
    let id = commands.spawn(Camera2dBundle::default()).id();
    commands.insert_resource(MenuCamera(id));
}

pub(crate) fn setup_loading_screen(mut commands: Commands) {
    let mut step_id = Entity::PLACEHOLDER;
    let mut pct_id = Entity::PLACEHOLDER;
    let mut fill_id = Entity::PLACEHOLDER;

    let root = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgb(0.04, 0.06, 0.09)),
            ..default()
        })
        .with_children(|root| {
            root.spawn(TextBundle::from_section(
                "CUTE OF DUTY",
                TextStyle { font_size: 84.0, color: Color::srgb(0.92, 0.95, 1.0), ..default() },
            ));
            root.spawn(TextBundle::from_section(
                "SIMPLE · 像素战术撤离 · PRE-ALPHA",
                TextStyle { font_size: 20.0, color: Color::srgb(0.55, 0.62, 0.72), ..default() },
            ));
            root.spawn(NodeBundle {
                style: Style { height: Val::Px(56.0), ..default() },
                ..default()
            });
            step_id = root.spawn(TextBundle::from_section(
                LOADING_STEPS[0],
                TextStyle { font_size: 18.0, color: Color::srgb(0.70, 0.78, 0.88), ..default() },
            )).id();
        })
        .id();

    commands.entity(root).with_children(|root| {
        // 进度条：容器 + 百分比宽度的填充条
        root.spawn(NodeBundle {
            style: Style {
                width: Val::Px(520.0),
                height: Val::Px(16.0),
                padding: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::srgb(0.10, 0.13, 0.18)),
            ..default()
        }).with_children(|bar| {
            fill_id = bar.spawn(NodeBundle {
                style: Style { width: Val::Percent(0.0), height: Val::Percent(100.0), ..default() },
                background_color: BackgroundColor(menu_accent()),
                ..default()
            }).id();
        });
        pct_id = root.spawn(TextBundle::from_section(
            "0%",
            TextStyle { font_size: 15.0, color: menu_accent(), ..default() },
        )).id();
        root.spawn(TextBundle::from_section(
            "首次启动需要编译渲染管线，请稍候 · 按任意键跳过",
            TextStyle { font_size: 13.0, color: Color::srgb(0.40, 0.46, 0.55), ..default() },
        ));
    });

    commands.insert_resource(LoadingScreen { root, step: step_id, fill: fill_id, percent: pct_id });
    commands.insert_resource(LoadingTimer(Timer::from_seconds(LOADING_DURATION_SECS, TimerMode::Once)));
}

pub(crate) fn loading_tick(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut timer: ResMut<LoadingTimer>,
    screen: Res<LoadingScreen>,
    mut texts: Query<&mut Text>,
    mut styles: Query<&mut Style>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    timer.0.tick(time.delta());
    // 任意键/点击跳过加载动画
    if keys.get_just_pressed().next().is_some() || mouse.get_just_pressed().next().is_some() {
        let total = timer.0.duration();
        timer.0.set_elapsed(total);
    }

    let t = (timer.0.elapsed_secs() / timer.0.duration().as_secs_f32()).clamp(0.0, 1.0);
    let step_index = ((t * LOADING_STEPS.len() as f32) as usize).min(LOADING_STEPS.len() - 1);

    if let Ok(mut text) = texts.get_mut(screen.step) {
        text.sections[0].value = LOADING_STEPS[step_index].to_string();
    }
    if let Ok(mut style) = styles.get_mut(screen.fill) {
        style.width = Val::Percent(t * 100.0);
    }
    if let Ok(mut text) = texts.get_mut(screen.percent) {
        text.sections[0].value = format!("{:.0}%", t * 100.0);
    }

    if timer.0.finished() {
        next_state.set(AppState::MainMenu);
    }
}

pub(crate) fn despawn_loading_screen(mut commands: Commands, screen: Res<LoadingScreen>) {
    // bevy 0.14 的 despawn() 不递归销毁子节点，UI 树必须用 despawn_recursive
    commands.entity(screen.root).despawn_recursive();
    commands.remove_resource::<LoadingScreen>();
    commands.remove_resource::<LoadingTimer>();
}

pub(crate) fn setup_main_menu(
    mut commands: Commands,
    assets: Res<AssetServer>,
    settings: Res<GameSettings>,
    selected: Res<SelectedMode>,
) {
    let mut quit_btn = Entity::PLACEHOLDER;
    let mut switch_btn = Entity::PLACEHOLDER;
    let mut start_btn = Entity::PLACEHOLDER;
    let mut close_btn = Entity::PLACEHOLDER;
    let mut status_text = Entity::PLACEHOLDER;
    let mut mode_panel = Entity::PLACEHOLDER;
    let mut settings_overlay = Entity::PLACEHOLDER;
    let mut settings_backdrop = Entity::PLACEHOLDER;

    let gear_texture: Handle<Image> = assets.load("ui/gear_icon.png");
    let (base_bg, base_border) = menu_button_palette(false);

    let root = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgb(0.04, 0.06, 0.09)),
            ..default()
        })
        .id();

    commands.entity(root).with_children(|root| {
        // 左上角角标
        root.spawn(TextBundle {
            text: Text::from_section(
                "PRE-ALPHA v0.2.1",
                TextStyle { font_size: 14.0, color: Color::srgb(0.42, 0.48, 0.56), ..default() },
            ),
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(18.0),
                left: Val::Px(24.0),
                ..default()
            },
            ..default()
        });
        root.spawn(TextBundle {
            text: Text::from_section(
                "CUTE OF DUTY 1: SIMPLE",
                TextStyle { font_size: 14.0, color: Color::srgb(0.42, 0.48, 0.56), ..default() },
            ),
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(40.0),
                left: Val::Px(24.0),
                ..default()
            },
            ..default()
        });

        // 左侧标题区（40% 宽：模式面板展开时标题仍完整可见）
        root.spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(40.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            ..default()
        }).with_children(|left| {
            left.spawn(TextBundle::from_section(
                "CUTE OF DUTY",
                TextStyle { font_size: 64.0, color: Color::srgb(0.92, 0.95, 1.0), ..default() },
            ));
            left.spawn(TextBundle::from_section(
                "SIMPLE · 像素战术撤离",
                TextStyle { font_size: 18.0, color: menu_accent(), ..default() },
            ));
            left.spawn(TextBundle::from_section(
                "点右下角「切换模式」选择作战模式",
                TextStyle { font_size: 14.0, color: Color::srgb(0.42, 0.48, 0.56), ..default() },
            ));
        });

        // 底部左侧：操作提示 + 退出
        root.spawn(TextBundle {
            text: Text::from_section(
                "WASD 移动 · 左键射击 · 右键越肩瞄准 · R 换弹\nTab/Esc 背包 · Q/E 干员技能 · F 互动（拾取/站点）",
                TextStyle { font_size: 13.0, color: Color::srgb(0.42, 0.48, 0.56), ..default() },
            ),
            style: Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(80.0),
                left: Val::Px(24.0),
                ..default()
            },
            ..default()
        });
        root.spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(24.0),
                bottom: Val::Px(24.0),
                ..default()
            },
            ..default()
        }).with_children(|wrap| {
            quit_btn = spawn_action_button(wrap, "退 出 游 戏", 170.0, 44.0, 18.0);
        });

        // 右下角：当前模式状态行 + 切换模式/开始游戏（ZIndex 压在模式面板之上）
        root.spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                right: Val::Px(24.0),
                bottom: Val::Px(24.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                row_gap: Val::Px(10.0),
                ..default()
            },
            z_index: ZIndex::Global(10),
            ..default()
        }).with_children(|col| {
            status_text = col.spawn((
                TextBundle::from_section(
                    format!("当前模式：{}", game_mode_spec(selected.0).name),
                    TextStyle { font_size: 15.0, color: menu_accent(), ..default() },
                ),
                StatusText,
            )).id();
            col.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(12.0),
                    ..default()
                },
                ..default()
            }).with_children(|row| {
                switch_btn = spawn_action_button(row, "切 换 模 式", 176.0, 54.0, 20.0);
                start_btn = spawn_action_button(row, "开 始 游 戏", 216.0, 54.0, 20.0);
            });
        });

        // 右上角齿轮：打开设置浮层（ZIndex 压在模式面板之上）
        root.spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(16.0),
                    right: Val::Px(24.0),
                    width: Val::Px(46.0),
                    height: Val::Px(46.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                background_color: base_bg,
                border_color: base_border,
                border_radius: BorderRadius::all(Val::Px(6.0)),
                z_index: ZIndex::Global(10),
                ..default()
            },
            Interaction::default(),
            MenuButton,
            GearButton,
        )).with_children(|btn| {
            btn.spawn((
                NodeBundle {
                    style: Style { width: Val::Px(26.0), height: Val::Px(26.0), ..default() },
                    ..default()
                },
                UiImage::new(gear_texture.clone()),
                GearIcon,
            ));
        });

        // 右侧 60% 模式面板："切换模式"呼出/收起
        mode_panel = root.spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(60.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect {
                        left: Val::Px(44.0),
                        right: Val::Px(44.0),
                        top: Val::Px(32.0),
                        bottom: Val::Px(110.0),
                    },
                    border: UiRect { left: Val::Px(2.0), ..default() },
                    row_gap: Val::Px(14.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.05, 0.07, 0.11, 0.985)),
                border_color: BorderColor(menu_accent()),
                visibility: Visibility::Hidden,
                ..default()
            },
            ModePanelRoot,
        )).with_children(|panel| {
            panel.spawn(TextBundle::from_section(
                "选 择 作 战 模 式",
                TextStyle { font_size: 30.0, color: Color::srgb(0.92, 0.95, 1.0), ..default() },
            ));
            panel.spawn(TextBundle::from_section(
                "按分类筛选 · 选中后点右下角「开始游戏」",
                TextStyle { font_size: 14.0, color: Color::srgb(0.55, 0.62, 0.72), ..default() },
            ));
            panel.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(28.0),
                    flex_grow: 1.0,
                    ..default()
                },
                ..default()
            }).with_children(|body| {
                // 分类列：0 号"全部"不过滤，其余按下标过滤模式行
                body.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        width: Val::Px(150.0),
                        ..default()
                    },
                    ..default()
                }).with_children(|cats| {
                    cats.spawn(TextBundle::from_section(
                        "分 类",
                        TextStyle { font_size: 14.0, color: Color::srgb(0.55, 0.62, 0.72), ..default() },
                    ));
                    for (index, name) in MODE_CATEGORIES.iter().enumerate() {
                        cats.spawn((
                            NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(44.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                background_color: base_bg,
                                border_color: base_border,
                                border_radius: BorderRadius::all(Val::Px(4.0)),
                                ..default()
                            },
                            Interaction::default(),
                            CategoryButton(index),
                        )).with_children(|b| {
                            b.spawn(TextBundle::from_section(
                                *name,
                                TextStyle { font_size: 17.0, color: Color::srgb(0.85, 0.89, 0.95), ..default() },
                            ));
                        });
                    }
                });
                // 模式列：一行一个模式（名称+描述 | 状态标签）
                body.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(12.0),
                        flex_grow: 1.0,
                        ..default()
                    },
                    ..default()
                }).with_children(|modes| {
                    modes.spawn(TextBundle::from_section(
                        "游 戏 模 式",
                        TextStyle { font_size: 14.0, color: Color::srgb(0.55, 0.62, 0.72), ..default() },
                    ));
                    for spec in GAME_MODES.iter() {
                        let (name_color, desc_color, tag, tag_color) = if spec.available {
                            (Color::srgb(0.92, 0.95, 1.0), Color::srgb(0.55, 0.62, 0.72), "可 用", menu_accent())
                        } else {
                            (Color::srgb(0.45, 0.50, 0.58), Color::srgb(0.30, 0.35, 0.42), "敬请期待", Color::srgb(0.34, 0.39, 0.46))
                        };
                        modes.spawn((
                            NodeBundle {
                                style: Style {
                                    height: Val::Px(68.0),
                                    justify_content: JustifyContent::SpaceBetween,
                                    align_items: AlignItems::Center,
                                    padding: UiRect {
                                        left: Val::Px(22.0),
                                        right: Val::Px(22.0),
                                        top: Val::Px(0.0),
                                        bottom: Val::Px(0.0),
                                    },
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                background_color: base_bg,
                                border_color: base_border,
                                border_radius: BorderRadius::all(Val::Px(4.0)),
                                ..default()
                            },
                            Interaction::default(),
                            ModeRow(spec.id),
                        )).with_children(|row| {
                            row.spawn(NodeBundle {
                                style: Style {
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(3.0),
                                    ..default()
                                },
                                ..default()
                            }).with_children(|left| {
                                left.spawn(TextBundle::from_section(
                                    spec.name,
                                    TextStyle { font_size: 21.0, color: name_color, ..default() },
                                ));
                                left.spawn(TextBundle::from_section(
                                    spec.desc,
                                    TextStyle { font_size: 13.0, color: desc_color, ..default() },
                                ));
                            });
                            row.spawn(TextBundle::from_section(
                                tag,
                                TextStyle { font_size: 15.0, color: tag_color, ..default() },
                            ));
                        });
                    }
                });
            });
        }).id();

        // 设置浮层（齿轮呼出；ZIndex 盖在模式面板与右下角按钮之上）
        // 压暗层用独立兄弟节点 + 显式 ZIndex：0.14 里带 Global Z 的父节点下，
        // absolute 背景子节点与兄弟子树的遮挡关系不可靠，压暗层必须自己占一层
        settings_backdrop = root.spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.02, 0.60)),
            visibility: Visibility::Hidden,
            z_index: ZIndex::Global(15),
            ..default()
        }).id();
        settings_overlay = root.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            visibility: Visibility::Hidden,
            z_index: ZIndex::Global(20),
            ..default()
        }).with_children(|overlay| {
            overlay.spawn(TextBundle::from_section(
                "游 戏 设 置",
                TextStyle { font_size: 36.0, color: Color::srgb(0.92, 0.95, 1.0), ..default() },
            ));
            overlay.spawn(NodeBundle {
                style: Style { height: Val::Px(8.0), ..default() },
                ..default()
            });
            spawn_setting_row(overlay, &settings, SettingKind::Sensitivity, "鼠标灵敏度");
            spawn_setting_row(overlay, &settings, SettingKind::Fov, "视野 (FOV)");
            spawn_setting_row(overlay, &settings, SettingKind::Ambient, "环境亮度");
            overlay.spawn(NodeBundle {
                style: Style { height: Val::Px(10.0), ..default() },
                ..default()
            });
            spawn_credits_panel(overlay);
            close_btn = spawn_action_button(overlay, "返 回", 200.0, 48.0, 20.0);
        }).id();
    });

    commands.entity(close_btn).insert(SettingsCloseButton);
    commands.entity(quit_btn).insert(QuitButton);
    commands.entity(switch_btn).insert(SwitchModeButton);
    commands.entity(start_btn).insert(StartGameButton);
    commands.insert_resource(MainMenuUi {
        root,
        mode_panel,
        settings_overlay,
        settings_backdrop,
        status_text,
    });
    commands.insert_resource(MenuGrace(Timer::from_seconds(0.4, TimerMode::Once)));
}

/// 右下角/底部小号操作按钮：居中单行标签，构建即挂 Interaction（悬停样式与主菜单统一）
pub(crate) fn spawn_action_button(
    parent: &mut ChildBuilder,
    label: &str,
    width: f32,
    height: f32,
    font_size: f32,
) -> Entity {
    let (bg, border) = menu_button_palette(false);
    parent.spawn((
        NodeBundle {
            style: Style {
                width: Val::Px(width),
                height: Val::Px(height),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            background_color: bg,
            border_color: border,
            border_radius: BorderRadius::all(Val::Px(4.0)),
            ..default()
        },
        Interaction::default(),
        MenuButton,
    )).with_children(|btn| {
        btn.spawn(TextBundle::from_section(
            label,
            TextStyle { font_size, color: Color::srgb(0.92, 0.95, 1.0), ..default() },
        ));
    }).id()
}

/// 构建单个菜单按钮；interactive=false 时为灰态占位（不挂 Interaction，不响应悬停/点击）
pub(crate) fn spawn_menu_button(
    parent: &mut ChildBuilder,
    label: &str,
    desc: &str,
    interactive: bool,
) -> Entity {
    let (bg, border) = if interactive {
        menu_button_palette(false)
    } else {
        (
            BackgroundColor(Color::srgba(0.06, 0.08, 0.11, 0.85)),
            BorderColor(Color::srgb(0.13, 0.16, 0.21)),
        )
    };
    let label_color = if interactive { Color::srgb(0.92, 0.95, 1.0) } else { Color::srgb(0.36, 0.40, 0.46) };
    let desc_color = if interactive { Color::srgb(0.55, 0.62, 0.72) } else { Color::srgb(0.26, 0.30, 0.36) };

    let mut button = parent.spawn(NodeBundle {
        style: Style {
            width: Val::Px(460.0),
            height: Val::Px(64.0),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            padding: UiRect::axes(Val::Px(24.0), Val::Px(0.0)),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        background_color: bg,
        border_color: border,
        border_radius: BorderRadius::all(Val::Px(4.0)),
        ..default()
    });
    button.with_children(|row| {
        row.spawn(TextBundle::from_section(
            label,
            TextStyle { font_size: 26.0, color: label_color, ..default() },
        ));
        row.spawn(TextBundle::from_section(
            desc,
            TextStyle { font_size: 14.0, color: desc_color, ..default() },
        ));
    });

    if interactive {
        button.insert(Interaction::default());
        button.insert(MenuButton);
    }
    button.id()
}

/// 按模式 id 查元数据表
pub(crate) fn game_mode_spec(id: GameModeId) -> &'static ModeSpec {
    GAME_MODES.iter().find(|spec| spec.id == id).expect("未知的游戏模式 id")
}

#[allow(clippy::type_complexity)]
pub(crate) fn main_menu_interaction(
    mut next_state: ResMut<NextState<AppState>>,
    mut app_exit: EventWriter<AppExit>,
    time: Res<Time>,
    mut grace: ResMut<MenuGrace>,
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<GameSettings>,
    mut selected: ResMut<SelectedMode>,
    mut category: ResMut<SelectedCategory>,
    ui: Res<MainMenuUi>,
    mut visibility: Query<&mut Visibility>,
    mut value_texts: Query<(&SettingValueText, &mut Text), Without<StatusText>>,
    mut status_texts: Query<&mut Text, (With<StatusText>, Without<SettingValueText>)>,
    buttons: Query<
        (
            &Interaction,
            Option<&SwitchModeButton>,
            Option<&StartGameButton>,
            Option<&GearButton>,
            Option<&SettingsCloseButton>,
            Option<&QuitButton>,
        ),
        Changed<Interaction>,
    >,
    adjust_buttons: Query<(&SettingAdjust, &Interaction), Changed<Interaction>>,
    category_buttons: Query<(&CategoryButton, &Interaction), Changed<Interaction>>,
    mode_rows: Query<(&ModeRow, &Interaction), Changed<Interaction>>,
) {
    grace.0.tick(time.delta());
    if !grace.0.finished() {
        return;
    }

    // Esc 关闭设置浮层（连同压暗层）
    if keys.just_pressed(KeyCode::Escape) {
        if let Ok(mut vis) = visibility.get_mut(ui.settings_overlay) {
            if matches!(*vis, Visibility::Visible) {
                *vis = Visibility::Hidden;
                if let Ok(mut backdrop_vis) = visibility.get_mut(ui.settings_backdrop) {
                    *backdrop_vis = Visibility::Hidden;
                }
            }
        }
    }
    let settings_open = visibility
        .get(ui.settings_overlay)
        .map_or(false, |vis| matches!(*vis, Visibility::Visible));

    // 按钮动作分发（设置浮层打开时，除"返回"外全部拦截）
    for (interaction, switch, start, gear, close, quit) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if settings_open && close.is_none() {
            continue;
        }
        if close.is_some() {
            if let Ok(mut vis) = visibility.get_mut(ui.settings_overlay) {
                *vis = Visibility::Hidden;
            }
            if let Ok(mut backdrop_vis) = visibility.get_mut(ui.settings_backdrop) {
                *backdrop_vis = Visibility::Hidden;
            }
        } else if switch.is_some() {
            if let Ok(mut vis) = visibility.get_mut(ui.mode_panel) {
                let open = matches!(*vis, Visibility::Visible);
                *vis = if open { Visibility::Hidden } else { Visibility::Visible };
            }
        } else if start.is_some() {
            let spec = game_mode_spec(selected.0);
            if spec.available {
                next_state.set(AppState::InGame);
                return;
            }
            if let Ok(mut text) = status_texts.get_mut(ui.status_text) {
                text.sections[0].value = format!("「{}」尚未开放，敬请期待", spec.name);
            }
        } else if gear.is_some() {
            if let Ok(mut vis) = visibility.get_mut(ui.settings_overlay) {
                *vis = Visibility::Visible;
            }
            if let Ok(mut backdrop_vis) = visibility.get_mut(ui.settings_backdrop) {
                *backdrop_vis = Visibility::Visible;
            }
        } else if quit.is_some() {
            app_exit.send(AppExit::Success);
            return;
        }
    }

    // 分类 / 模式选择（面板内部互斥：设置浮层打开时不响应）
    if !settings_open {
        for (cat, interaction) in &category_buttons {
            if *interaction == Interaction::Pressed {
                category.0 = cat.0;
            }
        }
        for (row, interaction) in &mode_rows {
            if *interaction == Interaction::Pressed {
                selected.0 = row.0;
            }
        }
    }

    // 设置浮层里的步进调节（与暂停菜单共用一套设置行控件）
    let mut adjusted = false;
    for (adjust, interaction) in &adjust_buttons {
        if *interaction == Interaction::Pressed {
            apply_setting_step(&mut settings, adjust.kind, adjust.delta);
            adjusted = true;
        }
    }
    if adjusted {
        for (value, mut text) in value_texts.iter_mut() {
            text.sections[0].value = setting_label(&settings, value.0);
        }
    }
}

/// 主菜单样式层：普通按钮悬停高亮、齿轮图标变色、模式行选中/置灰/分类过滤、状态行同步。
/// 只在有输入或选中态变化时重刷，避免每帧覆写样式导致变更检测空转。
pub(crate) fn main_menu_style(
    ui: Res<MainMenuUi>,
    selected: Res<SelectedMode>,
    category: Res<SelectedCategory>,
    panel_vis: Query<&Visibility, (With<ModePanelRoot>, Without<ModeRow>)>,
    gear_hover: Query<&Interaction, (With<GearButton>, Changed<Interaction>)>,
    mut gear_icon: Query<&mut UiImage, With<GearIcon>>,
    mut hover_buttons: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<MenuButton>),
    >,
    row_changed: Query<&Interaction, (With<ModeRow>, Changed<Interaction>)>,
    category_changed: Query<&Interaction, (With<CategoryButton>, Changed<Interaction>)>,
    mut mode_rows: Query<
        (&ModeRow, &Interaction, &mut BackgroundColor, &mut BorderColor, &mut Visibility),
        (Without<MenuButton>, Without<CategoryButton>, Without<ModePanelRoot>),
    >,
    mut categories: Query<
        (&CategoryButton, &Interaction, &mut BackgroundColor, &mut BorderColor),
        (Without<MenuButton>, Without<ModeRow>),
    >,
    mut status_texts: Query<&mut Text, (With<StatusText>, Without<SettingValueText>)>,
) {
    let touched = ui.is_changed()
        || selected.is_changed()
        || category.is_changed()
        || !gear_hover.is_empty()
        || !hover_buttons.is_empty()
        || !row_changed.is_empty()
        || !category_changed.is_empty();
    if !touched {
        return;
    }

    let (base_bg, base_border) = menu_button_palette(false);
    let (hover_bg, hover_border) = menu_button_palette(true);

    // 普通按钮（切换/开始/退出/返回/齿轮底板）悬停高亮
    for (interaction, mut bg, mut border) in &mut hover_buttons {
        match *interaction {
            Interaction::Hovered => { *bg = hover_bg; *border = hover_border; }
            Interaction::None => { *bg = base_bg; *border = base_border; }
            Interaction::Pressed => {}
        }
    }

    // 齿轮图标随悬停着色
    if let Ok(interaction) = gear_hover.get_single() {
        if let Ok(mut image) = gear_icon.get_single_mut() {
            image.color = if *interaction == Interaction::Hovered {
                menu_accent()
            } else {
                Color::WHITE
            };
        }
    }

    // 模式行：面板打开时按分类过滤可见；未开放置灰、选中高亮。
    // 注意 bevy 0.14 中被显式写成 Visible 的子节点会在隐藏父节点下漏渲染，
    // 因此行可见性必须与面板显隐联动，不能只看分类。
    let panel_open = panel_vis
        .get(ui.mode_panel)
        .map_or(false, |vis| matches!(*vis, Visibility::Visible));
    for (row, interaction, mut bg, mut border, mut vis) in &mut mode_rows {
        let spec = game_mode_spec(row.0);
        let hovered = *interaction == Interaction::Hovered;
        *vis = if panel_open && (category.0 == 0 || spec.category == category.0) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if row.0 == selected.0 {
            *bg = BackgroundColor(Color::srgba(0.12, 0.22, 0.30, 0.98));
            *border = BorderColor(menu_accent());
        } else if !spec.available {
            *bg = BackgroundColor(Color::srgba(0.07, 0.09, 0.12, 0.90));
            *border = BorderColor(Color::srgb(0.15, 0.18, 0.24));
        } else if hovered {
            *bg = hover_bg;
            *border = hover_border;
        } else {
            *bg = base_bg;
            *border = base_border;
        }
    }

    // 分类按钮：选中高亮
    for (cat, interaction, mut bg, mut border) in &mut categories {
        let hovered = *interaction == Interaction::Hovered;
        if cat.0 == category.0 {
            *bg = BackgroundColor(Color::srgba(0.12, 0.22, 0.30, 0.98));
            *border = BorderColor(menu_accent());
        } else if hovered {
            *bg = hover_bg;
            *border = hover_border;
        } else {
            *bg = base_bg;
            *border = base_border;
        }
    }

    // 选中模式变化 → 右下角状态行同步
    if selected.is_changed() {
        if let Ok(mut text) = status_texts.get_mut(ui.status_text) {
            text.sections[0].value = format!("当前模式：{}", game_mode_spec(selected.0).name);
        }
    }
}

pub(crate) fn despawn_main_menu(mut commands: Commands, ui: Res<MainMenuUi>) {
    commands.entity(ui.root).despawn_recursive();
    commands.remove_resource::<MainMenuUi>();
    commands.remove_resource::<MenuGrace>();
}

pub(crate) fn despawn_menu_camera(mut commands: Commands, cam: Res<MenuCamera>) {
    commands.entity(cam.0).despawn();
    commands.remove_resource::<MenuCamera>();
}


