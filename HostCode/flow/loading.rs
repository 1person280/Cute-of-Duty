//! Loading 画面：连接握手过渡 + v0.3.2 视觉（大标题 / 进度条 / 分步文案 / 任意键跳过）。
//!
//! 设计动机（Why）：加载屏是启动到主菜单之间的过渡。与 v0.3.2 的"计时 + 任意键盲跳"
//! 不同——本端必须在**服务端握手**（`LocalPlayer.entity_id` 就位）后才可靠进入主菜单，
//! 否则服务器未启动时玩家会看到空主菜单。因此这里保留惰性生成 + 握手驱动的进度：未
//! 连接时进度随时间平移，握手后即时到 100%，再停留 `LOADING_MIN_SECS` 保证可见后切换，
//! 任意键/点击可提前跳过。根节点挂 `StateScoped(Loading)`，离开状态自动销毁。

use bevy::prelude::*;

use super::flow_state::{self, AppState, CjkFont, LocalPlayer};
use crate::menu::menu_main::menu_accent;

/// Loading 屏根节点标记（配合 `StateScoped` 惰性生成检测 + 离开状态自动销毁）。
#[derive(Component)]
pub struct LoadingScreenRoot;

/// Loading 屏可更新节点句柄。
#[derive(Resource)]
pub struct LoadingScreen {
    pub step: Entity,
    pub fill: Entity,
    pub percent: Entity,
}

/// 进度满量用时（未连接时作为平移斜率；握手后即时跳到 100%）。
const LOADING_DURATION_SECS: f32 = 2.8;

/// 握手后加载屏最短可见时长（防止握手极快时加载屏一闪而过、玩家看不到连接反馈）。
const LOADING_MIN_SECS: f32 = 0.8;

/// 加载分步文案：真实初始化很轻，按统一节奏展示各子系统就位。
const LOADING_STEPS: [&str; 5] = [
    "初始化引擎核心…",
    "加载元素反应配置…",
    "构建训练场地图…",
    "准备干员与武器档案…",
    "校准 HUD 与小地图…",
];

/// 惰性生成 Loading 屏：仅当尚未生成且中文字体已就绪时为一次。
pub fn spawn_loading(
    mut commands: Commands,
    fonts: Res<CjkFont>,
    exists: Query<(), With<LoadingScreenRoot>>,
) {
    if fonts.0.is_none() || !exists.is_empty() {
        return;
    }
    let mut step_id = Entity::PLACEHOLDER;
    let mut pct_id = Entity::PLACEHOLDER;
    let mut fill_id = Entity::PLACEHOLDER;

    commands
        .spawn((
            LoadingScreenRoot,
            StateScoped(AppState::Loading),
            NodeBundle {
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
            },
        ))
        .with_children(|root| {
            // 标题区
            root.spawn(TextBundle::from_section(
                "CUTE OF DUTY",
                flow_state::style(&fonts, 84.0, Color::srgb(0.92, 0.95, 1.0)),
            ));
            root.spawn(TextBundle::from_section(
                "SIMPLE · 像素战术撤离 · PRE-ALPHA",
                flow_state::style(&fonts, 20.0, Color::srgb(0.55, 0.62, 0.72)),
            ));
            root.spawn(NodeBundle {
                style: Style { height: Val::Px(40.0), ..default() },
                ..default()
            });
            step_id = root
                .spawn(TextBundle::from_section(
                    LOADING_STEPS[0],
                    flow_state::style(&fonts, 18.0, Color::srgb(0.70, 0.78, 0.88)),
                ))
                .id();
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
            })
            .with_children(|bar| {
                fill_id = bar
                    .spawn(NodeBundle {
                        style: Style { width: Val::Percent(0.0), height: Val::Percent(100.0), ..default() },
                        background_color: BackgroundColor(menu_accent()),
                        ..default()
                    })
                    .id();
            });
            pct_id = root
                .spawn(TextBundle::from_section(
                    "0%",
                    flow_state::style(&fonts, 15.0, menu_accent()),
                ))
                .id();
            root.spawn(TextBundle::from_section(
                "正在连接服务器…按任意键跳过（需已连接）",
                flow_state::style(&fonts, 13.0, Color::srgb(0.40, 0.46, 0.55)),
            ));
        });

    commands.insert_resource(LoadingScreen { step: step_id, fill: fill_id, percent: pct_id });
}

/// 每帧刷新进度条 / 分步文案 / 百分比；已握手后任意键/点击跳过。
pub fn loading_tick(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    player: Res<LocalPlayer>,
    screen: Option<Res<LoadingScreen>>,
    mut progress: Local<f32>,
    // 握手后停留计时（保证加载屏至少展示 `LOADING_MIN_SECS`）。
    mut held: Local<f32>,
    mut texts: Query<&mut Text>,
    mut styles: Query<&mut Style>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    // 保护：`spawn_loading` 需等字体就绪且可能与本系统同帧先后不一，资源未生成时跳过本帧。
    let Some(screen) = screen else {
        return;
    };
    let connected = player.entity_id != 0;
    // 未连接时进度随时间平移（封顶 90%）；握手后即时到 100%，并累计停留时长。
    *held = if connected { *held + time.delta_seconds() } else { 0.0 };
    *progress = if connected {
        1.0
    } else {
        (*progress + time.delta_seconds() / LOADING_DURATION_SECS).min(0.9)
    };
    let step_index = ((*progress * LOADING_STEPS.len() as f32) as usize).min(LOADING_STEPS.len() - 1);

    if let Ok(mut text) = texts.get_mut(screen.step) {
        text.sections[0].value = if connected {
            "已连接服务器…".to_string()
        } else {
            LOADING_STEPS[step_index].to_string()
        };
    }
    if let Ok(mut style) = styles.get_mut(screen.fill) {
        style.width = Val::Percent(*progress * 100.0);
    }
    if let Ok(mut text) = texts.get_mut(screen.percent) {
        text.sections[0].value = format!("{:.0}%", *progress * 100.0);
    }

    // 已握手后，停留满最短时长（或任意键/点击提前）才切入主菜单。
    if connected
        && (*held >= LOADING_MIN_SECS
            || keys.get_just_pressed().next().is_some()
            || mouse.get_just_pressed().next().is_some())
    {
        next_state.set(AppState::MainMenu);
    }
}