//! 加载屏：占位启动过渡、进度条动画与"任意键跳过"逻辑。

use bevy::prelude::*;
use super::*;
use crate::demo::frontend::*;

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

/// 加载分步文案：真实初始化很轻，按统一节奏展示各子系统就位
pub(crate) const LOADING_STEPS: [&str; 5] = [
    "初始化引擎核心…",
    "加载元素反应配置…",
    "构建训练场地图…",
    "准备干员与武器档案…",
    "校准 HUD 与小地图…",
];

pub(crate) const LOADING_DURATION_SECS: f32 = 2.8;

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