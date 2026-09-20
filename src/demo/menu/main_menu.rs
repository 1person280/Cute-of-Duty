//! 主菜单结构：根界面构建、菜单相机、核心 UI 数据/标记、共享样式助手与销毁。
//! 交互与样式系统见 menu_behaviour，模式面板数据见 mode_panel，设置浮层标记见 settings_panel。

use bevy::prelude::*;
use super::mode_panel::*;
use super::settings_panel::*;
use crate::demo::pause::{spawn_credits_panel, spawn_setting_row, GameSettings, SettingKind};

#[derive(Resource)]
pub(crate) struct MenuCamera(pub(crate) Entity);

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

/// 右下角"当前模式"状态行（选中变化/非法进入时更新）
#[derive(Component)]
pub(crate) struct StatusText;

/// 主菜单强调色（交互与加载屏共用）
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
                "PRE-ALPHA v0.3.0",
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

pub(crate) fn despawn_main_menu(mut commands: Commands, ui: Res<MainMenuUi>) {
    commands.entity(ui.root).despawn_recursive();
    commands.remove_resource::<MainMenuUi>();
    commands.remove_resource::<MenuGrace>();
}

pub(crate) fn despawn_menu_camera(mut commands: Commands, cam: Res<MenuCamera>) {
    commands.entity(cam.0).despawn();
    commands.remove_resource::<MenuCamera>();
}