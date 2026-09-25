//! 主菜单：结构 + 完整 UI 树构建 + 共享样式助手与销毁。
//!
//! 设计动机（Why）：主菜单把玩家从「已连接」推进到可选模式、进场、设置、撤离返回。
//! 本模块只构建 UI 树与持有各浮层/节点的句柄资源；交互与样式刷新收敛到
//! `menu_behaviour`，模式数据见 `mode_panel`，设置/鸣谢见 `game_settings`。进场资格
//! 由服务端裁决：仓库浮层由 `arsenal` 复用、`LoadoutButton` 只翻其显隐，绝不本地裁决。
//! 根节点挂 `StateScoped(MainMenu)`，离开状态自动递归销毁。

use bevy::prelude::*;

use super::arsenal::{ArsenalSelection, ArsenalVisible};
use crate::flow::flow_state::{self as flow, AppState, CjkFont};
use super::game_settings::{self, GameSettings, SettingKind};
use super::mode_panel::{self, game_mode_spec, CategoryButton, ModePanelRoot, ModeRow, SelectedMode};
use crate::shared::theme;

/// 主菜单各可更新节点句柄（离开状态时随根节点统一销毁；资源在下次进入时重建）。
#[derive(Resource)]
pub struct MainMenuUi {
    /// 右侧 60% 模式面板（「切换模式」呼出/收起）。
    pub mode_panel: Entity,
    /// 齿轮呼出的设置浮层。
    pub settings_overlay: Entity,
    /// 设置浮层的全屏压暗层（与浮层同步显隐）。
    pub settings_backdrop: Entity,
    /// 右下角「当前模式」状态行。
    pub status_text: Entity,
}

/// 菜单刚打开时的输入保护期：拦截启动瞬间残留的按键/按下状态误触按钮。
#[derive(Resource)]
pub struct MenuGrace(pub Timer);

/// 可交互菜单按钮（无此标记的按钮为占位项，不参与交互）。
#[derive(Component)]
pub struct MenuButton;

/// 退出按钮。
#[derive(Component)]
pub struct QuitButton;

/// 右下角「当前模式」状态行（选中变化/非法进入时更新）。
#[derive(Component)]
pub struct StatusText;

/// 右上角齿轮按钮：打开设置浮层。
#[derive(Component)]
pub struct GearButton;

/// 齿轮图标实体（悬停时给它着色）。
#[derive(Component)]
pub struct GearIcon;

/// 设置浮层「返回」按钮：关闭设置浮层。
#[derive(Component)]
pub struct SettingsCloseButton;

/// 「仓库 · 携带物资」按钮：开关仓库浮层（复用 `arsenal`）。
#[derive(Component)]
pub struct LoadoutButton;

/// 主菜单强调色（交互、设置与加载屏共用；旧版 `#4dd0e1` 青色）。
pub fn menu_accent() -> Color {
    theme::ACCENT_CYAN
}

/// 可交互按钮的两态配色（构建与交互系统共用，保证视觉一致）。
pub fn menu_button_palette(hovered: bool) -> (BackgroundColor, BorderColor) {
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

/// 生成主菜单整屏 UI 树（OnEnter(MainMenu) 调用；字体未就绪则跳过本帧）。
#[allow(clippy::too_many_arguments)]
pub fn spawn_menu(
    mut commands: Commands,
    fonts: Res<CjkFont>,
    settings: Res<GameSettings>,
    selected: Res<SelectedMode>,
    assets: Res<AssetServer>,
) {
    if fonts.0.is_none() {
        return;
    }
    commands.insert_resource(ArsenalVisible(false));
    commands.insert_resource(ArsenalSelection::default());

    let mut quit_btn = Entity::PLACEHOLDER;
    let mut switch_btn = Entity::PLACEHOLDER;
    let mut start_btn = Entity::PLACEHOLDER;
    let mut close_btn = Entity::PLACEHOLDER;
    let mut loadout_btn = Entity::PLACEHOLDER;
    let mut status_text = Entity::PLACEHOLDER;
    let mut mode_panel = Entity::PLACEHOLDER;
    let mut settings_overlay = Entity::PLACEHOLDER;
    let mut settings_backdrop = Entity::PLACEHOLDER;

    let gear_texture: Handle<Image> = assets.load("ui/gear_icon.png");
    let (base_bg, base_border) = menu_button_palette(false);

    let root = commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                background_color: BackgroundColor(theme::BG_DEEP),
                ..default()
            },
            StateScoped(AppState::MainMenu),
        ))
        .id();

    commands.entity(root).with_children(|root| {
        // 左上角角标
        root.spawn(TextBundle::from_section(
            "PRE-ALPHA v0.3.0",
            flow::style(&fonts, 14.0, Color::srgb(0.42, 0.48, 0.56)),
        ))
        .insert(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(18.0),
            left: Val::Px(24.0),
            ..default()
        });
        root.spawn(TextBundle::from_section(
            "CUTE OF DUTY 1: SIMPLE",
            flow::style(&fonts, 14.0, Color::srgb(0.42, 0.48, 0.56)),
        ))
        .insert(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(40.0),
            left: Val::Px(24.0),
            ..default()
        });

        // 左侧 40% 标题区（模式面板展开时标题仍完整可见）
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
        })
        .with_children(|left| {
            left.spawn(TextBundle::from_section(
                "CUTE OF DUTY",
                flow::style(&fonts, 64.0, Color::srgb(0.92, 0.95, 1.0)),
            ));
            left.spawn(TextBundle::from_section(
                "SIMPLE · 像素战术撤离",
                flow::style(&fonts, 18.0, menu_accent()),
            ));
            left.spawn(TextBundle::from_section(
                "点右下角「切换模式」选择作战模式",
                flow::style(&fonts, 14.0, Color::srgb(0.42, 0.48, 0.56)),
            ));
        });

        // 底部左下：操作提示 + 退出
        root.spawn(TextBundle::from_section(
            "WASD 移动 · 左键射击 · 右键越肩瞄准 · R 换弹\nTab/Esc 背包 · Q/E 干员技能 · F 互动（拾取/站点）",
            flow::style(&fonts, 13.0, Color::srgb(0.42, 0.48, 0.56)),
        ))
        .insert(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(80.0),
            left: Val::Px(24.0),
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
        })
        .with_children(|wrap| {
            quit_btn = spawn_action_button(wrap, &fonts, "退 出 游 戏", 170.0, 44.0, 18.0);
        });

        // 右下角：当前模式状态行 + 切换模式/开始游戏/仓库
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
        })
        .with_children(|col| {
            status_text = col
                .spawn((
                    TextBundle::from_section(
                        format!("当前模式：{}", game_mode_spec(selected.0).name),
                        flow::style(&fonts, 15.0, menu_accent()),
                    ),
                    StatusText,
                ))
                .id();
            col.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(12.0),
                    ..default()
                },
                ..default()
            })
            .with_children(|row| {
                switch_btn = spawn_action_button(row, &fonts, "切 换 模 式", 176.0, 54.0, 20.0);
                start_btn = spawn_action_button(row, &fonts, "开 始 游 戏", 216.0, 54.0, 20.0);
            });
            loadout_btn = spawn_action_button(col, &fonts, "仓 库 · 携带物资", 404.0, 46.0, 18.0);
        });

        // 右上角齿轮：打开设置浮层
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
        ))
        .with_children(|btn| {
            btn.spawn((
                NodeBundle {
                    style: Style { width: Val::Px(26.0), height: Val::Px(26.0), ..default() },
                    ..default()
                },
                UiImage::new(gear_texture.clone()),
                GearIcon,
            ));
        });

        // 右侧 60% 模式面板：「切换模式」呼出/收起
        mode_panel = root
            .spawn((
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
            ))
            .with_children(|panel| {
                panel.spawn(TextBundle::from_section(
                    "选 择 作 战 模 式",
                    flow::style(&fonts, 30.0, Color::srgb(0.92, 0.95, 1.0)),
                ));
                panel.spawn(TextBundle::from_section(
                    "按分类筛选 · 选中后点右下角「开始游戏」",
                    flow::style(&fonts, 14.0, Color::srgb(0.55, 0.62, 0.72)),
                ));
                panel
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(28.0),
                            flex_grow: 1.0,
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|body| {
                        // 分类列
                        body.spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(10.0),
                                width: Val::Px(150.0),
                                ..default()
                            },
                            ..default()
                        })
                        .with_children(|cats| {
                            cats.spawn(TextBundle::from_section(
                                "分 类",
                                flow::style(&fonts, 14.0, Color::srgb(0.55, 0.62, 0.72)),
                            ));
                            for (index, name) in mode_panel::MODE_CATEGORIES.iter().enumerate() {
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
                                ))
                                .with_children(|b| {
                                    b.spawn(TextBundle::from_section(
                                        *name,
                                        flow::style(&fonts, 17.0, Color::srgb(0.85, 0.89, 0.95)),
                                    ));
                                });
                            }
                        });
                        // 模式列
                        body.spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(12.0),
                                flex_grow: 1.0,
                                ..default()
                            },
                            ..default()
                        })
                        .with_children(|modes| {
                            modes.spawn(TextBundle::from_section(
                                "游 戏 模 式",
                                flow::style(&fonts, 14.0, Color::srgb(0.55, 0.62, 0.72)),
                            ));
                            for spec in mode_panel::GAME_MODES.iter() {
                                let (name_color, desc_color, tag, tag_color) = if spec.available {
                                    (Color::srgb(0.92, 0.95, 1.0), Color::srgb(0.55, 0.62, 0.72), "可 用", menu_accent())
                                } else {
                                    (Color::srgb(0.45, 0.50, 0.58), Color::srgb(0.30, 0.35, 0.42), "敬请期待", Color::srgb(0.34, 0.39, 0.46))
                                };
                                modes
                                    .spawn((
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
                                    ))
                                    .with_children(|row| {
                                        row.spawn(NodeBundle {
                                            style: Style {
                                                flex_direction: FlexDirection::Column,
                                                row_gap: Val::Px(3.0),
                                                ..default()
                                            },
                                            ..default()
                                        })
                                        .with_children(|l| {
                                            l.spawn(TextBundle::from_section(
                                                spec.name,
                                                flow::style(&fonts, 21.0, name_color),
                                            ));
                                            l.spawn(TextBundle::from_section(
                                                spec.desc,
                                                flow::style(&fonts, 13.0, desc_color),
                                            ));
                                        });
                                        row.spawn(TextBundle::from_section(
                                            tag,
                                            flow::style(&fonts, 15.0, tag_color),
                                        ));
                                    });
                            }
                        });
                    });
            })
            .id();

        // 设置浮层 + 独立全屏压暗层。
        // 0.14 里带 Global Z 的父节点下 absolute 背景子节点与兄弟子树的遮挡关系不可靠，
        // 压暗层必须自己占一层（ZIndex 15/20，盖过 z10 的主操作区）。
        settings_backdrop = root
            .spawn(NodeBundle {
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
            })
            .id();
        settings_overlay = root
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
                visibility: Visibility::Hidden,
                z_index: ZIndex::Global(20),
                ..default()
            })
            .with_children(|overlay| {
                overlay.spawn(TextBundle::from_section(
                    "游 戏 设 置",
                    flow::style(&fonts, 36.0, Color::srgb(0.92, 0.95, 1.0)),
                ));
                overlay.spawn(NodeBundle {
                    style: Style { height: Val::Px(8.0), ..default() },
                    ..default()
                });
                game_settings::spawn_setting_row(overlay, &fonts, &settings, SettingKind::Sensitivity, "鼠标灵敏度");
                game_settings::spawn_setting_row(overlay, &fonts, &settings, SettingKind::Fov, "视野 (FOV)");
                game_settings::spawn_setting_row(overlay, &fonts, &settings, SettingKind::Ambient, "环境亮度");
                overlay.spawn(NodeBundle {
                    style: Style { height: Val::Px(10.0), ..default() },
                    ..default()
                });
                game_settings::spawn_credits_panel(overlay, &fonts);
                close_btn = spawn_action_button(overlay, &fonts, "返 回", 200.0, 48.0, 20.0);
            })
            .id();
    });

    commands.entity(quit_btn).insert(QuitButton);
    commands.entity(switch_btn).insert(mode_panel::SwitchModeButton);
    commands.entity(start_btn).insert(mode_panel::StartGameButton);
    commands.entity(loadout_btn).insert(LoadoutButton);
    commands.entity(close_btn).insert(SettingsCloseButton);
    commands.insert_resource(MainMenuUi {
        mode_panel,
        settings_overlay,
        settings_backdrop,
        status_text,
    });
    commands.insert_resource(MenuGrace(Timer::from_seconds(0.4, TimerMode::Once)));
}

/// 右下角/底部小号操作按钮：居中单行标签，构建即挂 `Interaction`（悬停样式统一）。
pub fn spawn_action_button(
    parent: &mut ChildBuilder,
    fonts: &CjkFont,
    label: &str,
    width: f32,
    height: f32,
    font_size: f32,
) -> Entity {
    let (bg, border) = menu_button_palette(false);
    parent
        .spawn((
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
        ))
        .with_children(|btn| {
            btn.spawn(TextBundle::from_section(
                label,
                flow::style(fonts, font_size, Color::srgb(0.92, 0.95, 1.0)),
            ));
        })
        .id()
}