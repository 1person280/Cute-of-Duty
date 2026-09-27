//! 暂停菜单：`~`(Backquote) 开关的游戏内浮层 + 设置子面板
//!
//! 设计动机（Why）：暂停是**纯客户端表现层门控**——它只冻结本地输入上报与相机朝向，
//! 绝不向服务端发送"暂停"概念（服务器权威下，本地不再上报移动意图即等价于暂停）。
//! 刻意不做成 `States`：从暂停回 InGame 若走状态切换会再次触发 `OnEnter(InGame)`
//! 重建 HUD，因此暂停只是 `InGame` 内的一枚资源门控，只有"返回主界面"才切状态。
//! 移植自 0.3.2 `demo/pause.rs`（主面板「返回游戏 / 游戏设置 / 返回主界面」+ 设置子面板
//! ＋开源鸣谢），并按本架构去掉轮盘与站点面板门控等不存在的依赖；鼠标锁定见
//! [`cursor_lock_system`]（游戏内锁定供自由视角，暂停/菜单态释放供点按）。

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};

use crate::flow::flow_state::{self as flow, AppState, CjkFont};
use super::game_settings::{
    apply_setting_step, setting_label, spawn_credits_panel, spawn_setting_row,
    GameSettings, SettingAdjust, SettingKind, SettingValueText,
};
use super::menu_main::{menu_button_palette, spawn_action_button, MenuButton};

/// 暂停菜单状态：关闭 / 主面板 / 设置面板。
#[derive(Resource, Default, Clone, Copy, PartialEq)]
pub enum PauseMenu {
    #[default]
    Closed,
    Main,
    Settings,
}

/// 暂停 UI 各面板句柄（关闭时随根节点一并销毁，资源同时移除）。
#[derive(Resource)]
pub struct PauseMenuUi {
    pub root: Entity,
    pub main_panel: Entity,
    pub settings_panel: Entity,
}

/// 暂停菜单刚打开时的输入保护期：拦截开菜单瞬间按住的按键误触按钮。
#[derive(Resource)]
pub struct PauseGrace(pub Timer);

/// 暂停面板按钮动作（挂在按钮实体上，交互系统统一分发）。
#[derive(Component, Clone, Copy, PartialEq)]
pub enum PauseAction {
    Resume,
    OpenSettings,
    BackToPause,
    ReturnMainMenu,
}

/// 运行条件：暂停菜单关闭（游戏内输入 / 相机正常运行）。
///
/// 这是唯一的门控条件——暂停打开时不跑输入上报与相机朝向，本地即"冻结"。
pub fn pause_closed(pause: Res<PauseMenu>) -> bool {
    *pause == PauseMenu::Closed
}

/// `~` 键开关暂停（打开弹主面板，再按则关闭并销毁面板）。
///
/// 每次调用先推进保护期计时；保护期由 `pause_menu_interaction` 消费，
/// 避免"开菜单那一帧仍按着的键"直接触发按钮。
pub fn pause_toggle(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut grace: Option<ResMut<PauseGrace>>,
    mut pause: ResMut<PauseMenu>,
    ui: Option<Res<PauseMenuUi>>,
    fonts: Res<CjkFont>,
    settings: Res<GameSettings>,
    mut commands: Commands,
) {
    if let Some(grace) = grace.as_mut() {
        grace.0.tick(time.delta());
    }
    if !keyboard.just_pressed(KeyCode::Backquote) {
        return;
    }
    match *pause {
        PauseMenu::Closed => {
            // 字体未就绪则不弹（空白面板不如不弹）；等字体就绪后按键即可打开。
            if fonts.0.is_none() {
                return;
            }
            *pause = PauseMenu::Main;
            spawn_pause_ui(&mut commands, &fonts, &settings);
        }
        _ => {
            *pause = PauseMenu::Closed;
            if let Some(ui) = ui {
                // bevy 0.14 `despawn()` 不递归，UI 树必须 `despawn_recursive`。
                commands.entity(ui.root).despawn_recursive();
                commands.remove_resource::<PauseMenuUi>();
                commands.remove_resource::<PauseGrace>();
            }
        }
    }
}

/// 构建暂停整屏 UI（压暗层 + 主面板 + 设置子面板），并登记句柄/保护期资源。
pub fn spawn_pause_ui(commands: &mut Commands, fonts: &CjkFont, settings: &GameSettings) {
    let mut resume_btn = Entity::PLACEHOLDER;
    let mut settings_btn = Entity::PLACEHOLDER;
    let mut return_btn = Entity::PLACEHOLDER;
    let mut back_btn = Entity::PLACEHOLDER;
    let mut main_panel = Entity::PLACEHOLDER;
    let mut settings_panel = Entity::PLACEHOLDER;

    let root = commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        })
        .id();

    commands.entity(root).with_children(|root_node| {
        // 全屏压暗层：独立绝对定位节点（与 HUD 边缘光同款写法，可靠渲染）。
        root_node.spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.02, 0.55)),
            ..default()
        });

        main_panel = root_node
            .spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(14.0),
                    ..default()
                },
                ..default()
            })
            .with_children(|panel| {
                panel.spawn(TextBundle::from_section(
                    "游 戏 暂 停",
                    flow::style(fonts, 46.0, Color::srgb(0.92, 0.95, 1.0)),
                ));
                panel.spawn(TextBundle::from_section(
                    "按 / 或 ~ 键继续游戏",
                    flow::style(fonts, 14.0, Color::srgb(0.55, 0.62, 0.72)),
                ));
                panel.spawn(NodeBundle {
                    style: Style { height: Val::Px(18.0), ..default() },
                    ..default()
                });
                resume_btn = spawn_action_button(panel, fonts, "返 回 游 戏", 360.0, 52.0, 22.0);
                settings_btn = spawn_action_button(panel, fonts, "游 戏 设 置", 360.0, 52.0, 22.0);
                return_btn = spawn_action_button(panel, fonts, "返 回 主 界 面", 360.0, 52.0, 22.0);
            })
            .id();

        settings_panel = root_node
            .spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(14.0),
                    ..default()
                },
                visibility: Visibility::Hidden,
                ..default()
            })
            .with_children(|panel| {
                panel.spawn(TextBundle::from_section(
                    "游 戏 设 置",
                    flow::style(fonts, 40.0, Color::srgb(0.92, 0.95, 1.0)),
                ));
                panel.spawn(NodeBundle {
                    style: Style { height: Val::Px(10.0), ..default() },
                    ..default()
                });
                spawn_setting_row(panel, fonts, settings, SettingKind::Sensitivity, "鼠标灵敏度");
                spawn_setting_row(panel, fonts, settings, SettingKind::Fov, "视野 (FOV)");
                spawn_setting_row(panel, fonts, settings, SettingKind::Ambient, "环境亮度");
                panel.spawn(NodeBundle {
                    style: Style { height: Val::Px(12.0), ..default() },
                    ..default()
                });
                spawn_credits_panel(panel, fonts);
                back_btn = spawn_action_button(panel, fonts, "返 回", 360.0, 46.0, 20.0);
            })
            .id();
    });

    commands.entity(resume_btn).insert(PauseAction::Resume);
    commands.entity(settings_btn).insert(PauseAction::OpenSettings);
    commands.entity(return_btn).insert(PauseAction::ReturnMainMenu);
    commands.entity(back_btn).insert(PauseAction::BackToPause);

    commands.insert_resource(PauseMenuUi { root, main_panel, settings_panel });
    commands.insert_resource(PauseGrace(Timer::from_seconds(0.25, TimerMode::Once)));
}

/// 暂停面板交互：悬停高亮、按钮分发（继续 / 设置 / 返回主界面）、设置步进与值刷新。
#[allow(clippy::too_many_arguments)]
pub fn pause_menu_interaction(
    mut commands: Commands,
    mut next_state: ResMut<NextState<AppState>>,
    mut pause: ResMut<PauseMenu>,
    mut settings: ResMut<GameSettings>,
    ui: Option<Res<PauseMenuUi>>,
    mut visibility: Query<&mut Visibility>,
    mut hover_buttons: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<MenuButton>),
    >,
    action_buttons: Query<(&PauseAction, &Interaction), Changed<Interaction>>,
    adjust_buttons: Query<(&SettingAdjust, &Interaction), Changed<Interaction>>,
    mut value_texts: Query<(&SettingValueText, &mut Text)>,
    grace: Option<Res<PauseGrace>>,
) {
    // 保护期内不响应，避免开菜单瞬间的残留按下误触。
    if let Some(grace) = grace {
        if !grace.0.finished() {
            return;
        }
    }

    // 悬停高亮（与主菜单同一套配色）。
    let (base_bg, base_border) = menu_button_palette(false);
    let (hover_bg, hover_border) = menu_button_palette(true);
    for (interaction, mut bg, mut border) in &mut hover_buttons {
        match *interaction {
            Interaction::Hovered => {
                *bg = hover_bg;
                *border = hover_border;
            }
            Interaction::None => {
                *bg = base_bg;
                *border = base_border;
            }
            Interaction::Pressed => {}
        }
    }

    for (action, interaction) in &action_buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            PauseAction::Resume => {
                *pause = PauseMenu::Closed;
                if let Some(ui) = ui.as_ref() {
                    commands.entity(ui.root).despawn_recursive();
                    commands.remove_resource::<PauseMenuUi>();
                    commands.remove_resource::<PauseGrace>();
                }
                return;
            }
            PauseAction::OpenSettings | PauseAction::BackToPause => {
                let target = if *action == PauseAction::OpenSettings {
                    PauseMenu::Settings
                } else {
                    PauseMenu::Main
                };
                *pause = target;
                if let Some(ui) = ui.as_ref() {
                    if let Ok(mut vis) = visibility.get_mut(ui.main_panel) {
                        *vis = if target == PauseMenu::Main { Visibility::Visible } else { Visibility::Hidden };
                    }
                    if let Ok(mut vis) = visibility.get_mut(ui.settings_panel) {
                        *vis = if target == PauseMenu::Settings { Visibility::Visible } else { Visibility::Hidden };
                    }
                }
            }
            PauseAction::ReturnMainMenu => {
                // 切状态即可；暂停浮层的销毁交给 OnExit(InGame) 的 `teardown_pause`。
                *pause = PauseMenu::Closed;
                next_state.set(AppState::MainMenu);
                return;
            }
        }
    }

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

/// 游戏内锁定鼠标（隐藏光标并锁定到窗口中心，供 [`crate::world::mouse_look_system`]
/// 读鼠标位移做自由视角）；主菜单、暂停浮层，或**任一指针型面板**打开时释放光标，
/// 让光标回到可悬停/可拖拽状态。
///
/// 设计动机（Why）：鼠标锁定是「游戏内输入模态」的表现层开关。而 4×3 格位面板（物资箱/补给台）、
/// 交互二级选项面板、战术大地图都是**依赖指针悬停/点击**的 UI——若仍锁死光标，bevy 的
/// `ui_focus_system` 只会在窗口中心命中节点，玩家既无法拖拽格位、也点不到选项（实测反馈
/// "没有呼出鼠标让我拖拽"）。故这四类面板打开时一律释放光标。三者本就把 `gameplay_input_active`
/// 置假（相机不再跟随鼠标），释放光标不会引起视角乱转。
/// 注意：**径向轮盘除外**——它靠鼠标**位移**而非指针位置选格，保持锁定更符合 legacy 手感。
pub fn cursor_lock_system(
    state: Res<State<AppState>>,
    pause: Res<PauseMenu>,
    bigmap: Res<crate::hud::BigMapOpen>,
    interact: Res<crate::hud::InteractState>,
    loot: Res<crate::hud::LootPanelState>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let lock = *state.get() == AppState::InGame
        && *pause == PauseMenu::Closed
        && !bigmap.0
        && !interact.panel_open
        && !loot.open;
    for mut window in &mut windows {
        let grab = if lock { CursorGrabMode::Locked } else { CursorGrabMode::None };
        if window.cursor.grab_mode != grab {
            window.cursor.grab_mode = grab;
            window.cursor.visible = !lock;
        }
    }
}

/// 离开 InGame 时的兜底清理：关闭暂停态并销毁浮层（返回主界面 / 状态切换通用）。
pub fn teardown_pause(
    mut commands: Commands,
    ui: Option<Res<PauseMenuUi>>,
    mut pause: ResMut<PauseMenu>,
) {
    *pause = PauseMenu::Closed;
    if let Some(ui) = ui {
        commands.entity(ui.root).despawn_recursive();
    }
    commands.remove_resource::<PauseMenuUi>();
    commands.remove_resource::<PauseGrace>();
}
