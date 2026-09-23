//! 暂停菜单与设置：GameSettings、设置行/鸣谢面板 UI、暂停交互、teardown_game

use bevy::prelude::*;
use bevy::window::{CursorOptions, PrimaryWindow};
use crate::model::PlayerCamera;
use super::menu::{spawn_menu_button, menu_accent, menu_button_palette, MenuButton};
use super::inventory::HeldGrenade;
use super::hud::EffectAssets;
use super::frontend::*;
use super::components::*;

#[derive(Resource)]
pub(crate) struct GameSettings {
    pub(crate) mouse_sensitivity: f32,
    pub(crate) fov_deg: f32,
    pub(crate) ambient_brightness: f32,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self { mouse_sensitivity: 1.0, fov_deg: 45.0, ambient_brightness: 0.55 }
    }
}

/// 暂停菜单状态：关闭 / 主面板 / 设置面板。
/// 刻意不做成 States：从 Paused 回 InGame 会再次触发 OnEnter(InGame) 重建整个世界，
/// 因此暂停只是 InGame 内的资源门控，只有"返回主界面"才真正切状态。
#[derive(Resource, Default, Clone, Copy, PartialEq)]
pub(crate) enum PauseMenu {
    #[default]
    Closed,
    Main,
    Settings,
}

#[derive(Resource)]
pub(crate) struct PauseMenuUi {
    pub(crate) root: Entity,
    pub(crate) main_panel: Entity,
    pub(crate) settings_panel: Entity,
}

/// 暂停菜单刚打开时的输入保护期：拦截开菜单瞬间按住的射击键误触按钮
#[derive(Resource)]
pub(crate) struct PauseGrace(pub(crate) Timer);

/// 暂停面板按钮动作（挂在按钮实体上，交互系统统一分发）
#[derive(Component, Clone, Copy, PartialEq)]
pub(crate) enum PauseAction {
    Resume,
    OpenSettings,
    BackToPause,
    ReturnMainMenu,
}

/// 可调设置项类别
#[derive(Component, Clone, Copy, PartialEq)]
pub(crate) enum SettingKind {
    Sensitivity,
    Fov,
    Ambient,
}

/// ◀/▶ 步进按钮：kind 对应设置项，delta 为步进量（负为减小）
#[derive(Component)]
pub(crate) struct SettingAdjust {
    pub(crate) kind: SettingKind,
    pub(crate) delta: f32,
}

/// 设置项当前值文本
#[derive(Component)]
pub(crate) struct SettingValueText(pub(crate) SettingKind);

/// 游戏逻辑门控条件：暂停菜单打开时不跑（挂在三个游戏系统元组上）
pub(crate) fn pause_open(pause: Res<PauseMenu>) -> bool {
    *pause != PauseMenu::Closed
}

pub(crate) fn pause_toggle(
    keyboard: Res<ButtonInput<KeyCode>>,
    wheel: Res<WheelState>,
    open_station: Res<OpenStation>,
    time: Res<Time>,
    mut grace: Option<ResMut<PauseGrace>>,
    mut pause: ResMut<PauseMenu>,
    ui: Option<Res<PauseMenuUi>>,
    settings: Res<GameSettings>,
    mut commands: Commands,
    mut windows: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut input_state: ResMut<InputState>,
) {
    if let Some(grace) = grace.as_mut() {
        grace.0.tick(time.delta());
    }
    // 轮盘/站点面板打开时 / 键由它们负责（与 Esc 的跳过规则一致）
    if wheel.open || wheel.pending_key.is_some() || *open_station != OpenStation::None {
        return;
    }
    if !keyboard.just_pressed(KeyCode::Backquote) {
        return;
    }
    match *pause {
        PauseMenu::Closed => {
            *pause = PauseMenu::Main;
            spawn_pause_ui(&mut commands, &settings);
            unlock_cursor(&mut windows, &mut input_state);
        }
        _ => {
            *pause = PauseMenu::Closed;
            if let Some(ui) = ui {
                // bevy 0.19 的 despawn() 已递归销毁子节点
                commands.entity(ui.root).despawn();
                commands.remove_resource::<PauseMenuUi>();
                commands.remove_resource::<PauseGrace>();
            }
            lock_cursor(&mut windows, &mut input_state);
        }
    }
}


pub(crate) fn spawn_pause_ui(commands: &mut Commands, settings: &GameSettings) {
    let mut resume_btn = Entity::PLACEHOLDER;
    let mut settings_btn = Entity::PLACEHOLDER;
    let mut return_btn = Entity::PLACEHOLDER;
    let mut back_btn = Entity::PLACEHOLDER;

    let root = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .id();

    let mut main_panel = Entity::PLACEHOLDER;
    let mut settings_panel = Entity::PLACEHOLDER;

    commands.entity(root).with_children(|root| {
        // 全屏压暗层：独立绝对定位节点（与 HUD 边缘光同款写法，可靠渲染）
        root.spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.02, 0.55)),
        ));
        main_panel = root.spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(14.0),
            ..default()
        }).with_children(|panel| {
            panel.spawn((
                Text::new("游 戏 暂 停"),
                TextFont { font_size: FontSize::Px(46.0), ..default() },
                TextColor(Color::srgb(0.92, 0.95, 1.0)),
            ));
            panel.spawn((
                Text::new("按 / 或 ~ 键继续游戏"),
                TextFont { font_size: FontSize::Px(14.0), ..default() },
                TextColor(Color::srgb(0.55, 0.62, 0.72)),
            ));
            panel.spawn(Node { height: Val::Px(18.0), ..default() });
            resume_btn = spawn_menu_button(panel, "返 回 游 戏", "继续当前训练", true);
            settings_btn = spawn_menu_button(panel, "游 戏 设 置", "灵敏度 · 视野 · 亮度", true);
            return_btn = spawn_menu_button(panel, "返 回 主 界 面", "结束本次训练", true);
        }).id();

        settings_panel = root.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            Visibility::Hidden,
        )).with_children(|panel| {
            panel.spawn((
                Text::new("游 戏 设 置"),
                TextFont { font_size: FontSize::Px(40.0), ..default() },
                TextColor(Color::srgb(0.92, 0.95, 1.0)),
            ));
            panel.spawn(Node { height: Val::Px(10.0), ..default() });
            spawn_setting_row(panel, settings, SettingKind::Sensitivity, "鼠标灵敏度");
            spawn_setting_row(panel, settings, SettingKind::Fov, "视野 (FOV)");
            spawn_setting_row(panel, settings, SettingKind::Ambient, "环境亮度");
            panel.spawn(Node { height: Val::Px(12.0), ..default() });
            spawn_credits_panel(panel);
            back_btn = spawn_menu_button(panel, "返 回", "回到暂停菜单", true);
        }).id();
    });

    commands.entity(resume_btn).insert(PauseAction::Resume);
    commands.entity(settings_btn).insert(PauseAction::OpenSettings);
    commands.entity(return_btn).insert(PauseAction::ReturnMainMenu);
    commands.entity(back_btn).insert(PauseAction::BackToPause);

    commands.insert_resource(PauseMenuUi { root, main_panel, settings_panel });
    commands.insert_resource(PauseGrace(Timer::from_seconds(0.25, TimerMode::Once)));
}

/// 开源代码鸣谢面板：逐条列出本项目用到的核心开源库及其用途
pub(crate) fn spawn_credits_panel(parent: &mut ChildSpawnerCommands) {
    /// (库名 · 版本, 一句话说明"是什么、用在哪")
    const CREDITS: &[(&str, &str)] = &[
        ("Bevy 0.14", "3D 游戏引擎（MIT / Apache-2.0）—— 渲染、输入、UI、ECS 场景调度"),
        ("Tokio 1.35", "异步运行时（MIT）—— 驱动固定 Tick 主循环的实时节流与定时"),
        ("Serde / serde_yaml", "序列化框架（MIT / Apache-2.0）—— 解析 config/element_reactions.yaml 元素反应配置"),
        ("Tracing", "结构化日志（MIT）—— 主循环、战局与档案系统的运行日志输出"),
        ("Rand / rand_pcg", "随机数（MIT / Apache-2.0）—— 装备元素掉落与 AI 行为的确定性随机"),
        ("Crossbeam-queue", "无锁队列（MIT / Apache-2.0）—— HAL 层环形缓冲的低延迟通信"),
        ("BLAKE3 1.5", "密码学哈希（CC0 / Apache-2.0）—— 交易记录审计与确定性验证的状态哈希"),
        ("Criterion 0.5", "基准测试框架（MIT / Apache-2.0，仅 dev 依赖）—— 性能回归基准"),
    ];

    parent.spawn((
        Node {
            width: Val::Px(760.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(4.0),
            padding: UiRect::px(20.0, 10.0, 14.0, 12.0),
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.06, 0.09, 0.13, 0.92)),
        BorderColor::all(Color::srgb(0.22, 0.28, 0.36)),
    )).with_children(|panel| {
        panel.spawn((
            Text::new("开 源 代 码 鸣 谢"),
            TextFont { font_size: FontSize::Px(17.0), ..default() },
            TextColor(menu_accent()),
        ));
        panel.spawn((
            Text::new("本项目是开源软件（GPL-3.0 with linking exception），站在下列开源库的肩膀上"),
            TextFont { font_size: FontSize::Px(12.0), ..default() },
            TextColor(Color::srgb(0.55, 0.62, 0.72)),
        ));
        panel.spawn(Node { height: Val::Px(4.0), ..default() });
        for (name, desc) in CREDITS {
            panel.spawn(Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: Val::Px(16.0),
                ..default()
            }).with_children(|row| {
                // 左列定宽，保证右侧说明文字纵向对齐
                row.spawn((
                    Text::new(*name),
                    TextFont { font_size: FontSize::Px(13.0), ..default() },
                    TextColor(Color::srgb(0.85, 0.89, 0.95)),
                ));
                row.spawn((
                    Text::new(*desc),
                    TextFont { font_size: FontSize::Px(13.0), ..default() },
                    TextColor(Color::srgb(0.62, 0.70, 0.80)),
                ));
            });
        }
    });
}

/// 设置行：标签 + ◀ 值 ▶
pub(crate) fn spawn_setting_row(parent: &mut ChildSpawnerCommands, settings: &GameSettings, kind: SettingKind, label: &str) {
    parent.spawn((
        Node {
            width: Val::Px(460.0),
            height: Val::Px(46.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::px(18.0, 12.0, 0.0, 0.0),
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.10, 0.14, 0.20, 0.95)),
        BorderColor::all(Color::srgb(0.22, 0.28, 0.36)),
    )).with_children(|row| {
        row.spawn((
            Text::new(label),
            TextFont { font_size: FontSize::Px(19.0), ..default() },
            TextColor(Color::srgb(0.85, 0.89, 0.95)),
        ));
        row.spawn(Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        }).with_children(|ctrl| {
            spawn_step_button(ctrl, "<", kind, -setting_step(kind));
            ctrl.spawn((
                Text::new(setting_label(settings, kind)),
                TextFont { font_size: FontSize::Px(18.0), ..default() },
                TextColor(menu_accent()),
                SettingValueText(kind),
            ));
            spawn_step_button(ctrl, ">", kind, setting_step(kind));
        });
    });
}

pub(crate) fn spawn_step_button(parent: &mut ChildSpawnerCommands, glyph: &str, kind: SettingKind, delta: f32) {
    parent.spawn((
        Node {
            width: Val::Px(40.0),
            height: Val::Px(34.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(2.0)),
            border_radius: BorderRadius::all(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.14, 0.20, 0.28, 0.98)),
        BorderColor::all(menu_accent()),
        Interaction::default(),
        MenuButton,
        SettingAdjust { kind, delta },
    )).with_children(|btn| {
        btn.spawn((
            Text::new(glyph),
            TextFont { font_size: FontSize::Px(18.0), ..default() },
            TextColor(Color::srgb(0.92, 0.95, 1.0)),
        ));
    });
}

pub(crate) fn setting_step(kind: SettingKind) -> f32 {
    match kind {
        SettingKind::Sensitivity => 0.1,
        SettingKind::Fov => 5.0,
        SettingKind::Ambient => 0.05,
    }
}

pub(crate) fn setting_label(settings: &GameSettings, kind: SettingKind) -> String {
    match kind {
        SettingKind::Sensitivity => format!("x{:.1}", settings.mouse_sensitivity),
        SettingKind::Fov => format!("{:.0}", settings.fov_deg),
        SettingKind::Ambient => format!("{:.2}", settings.ambient_brightness),
    }
}

pub(crate) fn apply_setting_step(settings: &mut GameSettings, kind: SettingKind, delta: f32) {
    match kind {
        SettingKind::Sensitivity => settings.mouse_sensitivity = (settings.mouse_sensitivity + delta).clamp(0.2, 3.0),
        SettingKind::Fov => settings.fov_deg = (settings.fov_deg + delta).clamp(40.0, 110.0),
        SettingKind::Ambient => settings.ambient_brightness = (settings.ambient_brightness + delta).clamp(0.10, 1.20),
    }
}

pub(crate) fn pause_menu_interaction(
    mut commands: Commands,
    mut next_state: ResMut<NextState<AppState>>,
    mut pause: ResMut<PauseMenu>,
    mut settings: ResMut<GameSettings>,
    ui: Option<Res<PauseMenuUi>>,
    mut windows: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut input_state: ResMut<InputState>,
    mut visibility: Query<&mut Visibility>,
    mut hover_buttons: Query<
        (Entity, &Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<MenuButton>),
    >,
    action_buttons: Query<(&PauseAction, &Interaction), Changed<Interaction>>,
    adjust_buttons: Query<(&SettingAdjust, &Interaction), Changed<Interaction>>,
    mut value_texts: Query<(&SettingValueText, &mut Text)>,
    grace: Option<Res<PauseGrace>>,
) {
    if let Some(grace) = grace {
        if !grace.0.is_finished() {
            return;
        }
    }

    // 悬停高亮（与主菜单同一套配色）
    let (base_bg, base_border) = menu_button_palette(false);
    let (hover_bg, hover_border) = menu_button_palette(true);
    for (_, interaction, mut bg, mut border) in &mut hover_buttons {
        match *interaction {
            Interaction::Hovered => { *bg = hover_bg; *border = hover_border; }
            Interaction::None => { *bg = base_bg; *border = base_border; }
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
                    commands.entity(ui.root).despawn();
                    commands.remove_resource::<PauseMenuUi>();
                    commands.remove_resource::<PauseGrace>();
                }
                lock_cursor(&mut windows, &mut input_state);
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
                // 直接切状态；清场交给 OnExit(InGame) 的 teardown_game
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
            text.0 = setting_label(&settings, value.0);
        }
    }
}

/// 返回主界面时的清场：销毁除窗口外的所有实体并重置游戏态资源，
/// 下一次进入训练场由 OnEnter(InGame) 全量重建。
pub(crate) fn teardown_game(world: &mut World) {
    let window_entities: Vec<Entity> = world
        .query_filtered::<Entity, With<Window>>()
        .iter(world)
        .collect();
    let doomed: Vec<Entity> = world
        .iter_entities()
        .map(|entity| entity.id())
        .filter(|entity| !window_entities.contains(entity))
        .collect();
    for entity in doomed {
        world.despawn(entity);
    }

    world.insert_resource(NearbyInteract::default());
    world.insert_resource(WheelState::default());
    world.insert_resource(HeldGrenade::default());
    world.insert_resource(KillStats::default());
    world.insert_resource(OpenStation::None);
    world.insert_resource(InputState::default());
    world.insert_resource(PauseMenu::default());
    world.remove_resource::<PauseMenuUi>();
    world.remove_resource::<PauseGrace>();
    // setup_world 在下次进入时重新生成
    world.remove_resource::<EffectAssets>();
}

pub(crate) fn settings_apply_fov(
    settings: Res<GameSettings>,
    mut cameras: Query<(&mut Projection, &PlayerCamera)>,
) {
    // 不做 is_changed 短路：返回主界面再进游戏会生成新相机（默认 FOV），
    // 每帧幂等应用才能让设置在重建后保持一致
    for (mut projection, cam) in &mut cameras {
        if let Projection::Perspective(perspective) = &mut *projection {
            // 越肩瞄准时视野收窄（最多 -28%），与 aim_lerp 同步平滑过渡
            let target = settings.fov_deg.to_radians() * (1.0 - 0.28 * cam.aim_lerp);
            if perspective.fov != target {
                perspective.fov = target;
            }
        }
    }
}

pub(crate) fn settings_apply_ambient(settings: Res<GameSettings>, mut ambient: Query<&mut AmbientLight>) {
    if !settings.is_changed() {
        return;
    }
    if let Ok(mut ambient) = ambient.single_mut() {
        ambient.brightness = settings.ambient_brightness;
    }
}