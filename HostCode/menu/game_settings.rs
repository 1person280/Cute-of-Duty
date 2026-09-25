//! 游玩设置：`GameSettings` 资源 + 设置行 UI + 应用系统。
//!
//! 设计动机（Why）：设置项是可调的表现层参数（灵敏度 / 视野 / 环境亮度）。期间只把
//! FOV 真实作用到本端相机投影、把环境亮度写入世界 `AmbientLight` —— 它们不触碰任何
//! 服务端权威数据。灵敏度因当前相机为绕点轨道 + WASD（无越肩瞄准），仅在面板存储与
//! 显示，预留给日后的鼠标瞄准输入。

use bevy::prelude::*;

use crate::world::camera::ChaseCamera;
use crate::flow::flow_state::{self as flow, CjkFont};
use super::menu_main::{menu_accent, MenuButton};

/// 可调设置项：每帧由「值文本」与「步进按钮」共同呈现。
#[derive(Component, Clone, Copy, PartialEq)]
pub enum SettingKind {
    Sensitivity,
    Fov,
    Ambient,
}

/// ◀ / ▶ 步进按钮：kind 对应设置项，delta 为步进量（负为减小）。
#[derive(Component)]
pub struct SettingAdjust {
    pub kind: SettingKind,
    pub delta: f32,
}

/// 设置项当前值文本。
#[derive(Component)]
pub struct SettingValueText(pub SettingKind);

/// 局内可调设置值（会话内保留；默认值与 v0.3.2 一致）。
#[derive(Resource)]
pub struct GameSettings {
    pub mouse_sensitivity: f32,
    pub fov_deg: f32,
    pub ambient_brightness: f32,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self { mouse_sensitivity: 1.0, fov_deg: 45.0, ambient_brightness: 0.55 }
    }
}

/// 设置行：标签 + ◀ 值 ▶。
pub fn spawn_setting_row(
    parent: &mut ChildBuilder,
    fonts: &CjkFont,
    settings: &GameSettings,
    kind: SettingKind,
    label: &str,
) {
    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Px(460.0),
                height: Val::Px(46.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::px(18.0, 12.0, 0.0, 0.0),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.10, 0.14, 0.20, 0.95)),
            border_color: BorderColor(Color::srgb(0.22, 0.28, 0.36)),
            border_radius: BorderRadius::all(Val::Px(4.0)),
            ..default()
        })
        .with_children(|row| {
            row.spawn(TextBundle::from_section(
                label,
                flow::style(fonts, 19.0, Color::srgb(0.85, 0.89, 0.95)),
            ));
            row.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    ..default()
                },
                ..default()
            })
            .with_children(|ctrl| {
                spawn_step_button(ctrl, fonts, "<", kind, -setting_step(kind));
                ctrl.spawn((
                    TextBundle::from_section(
                        setting_label(settings, kind),
                        flow::style(fonts, 18.0, menu_accent()),
                    ),
                    SettingValueText(kind),
                ));
                spawn_step_button(ctrl, fonts, ">", kind, setting_step(kind));
            });
        });
}

/// 步进按钮：构建即挂 `Interaction`，命中经 `main_menu_behaviour` 修改设置值。
pub fn spawn_step_button(
    parent: &mut ChildBuilder,
    fonts: &CjkFont,
    glyph: &str,
    kind: SettingKind,
    delta: f32,
) {
    parent
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Px(40.0),
                    height: Val::Px(34.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.14, 0.20, 0.28, 0.98)),
                border_color: BorderColor(menu_accent()),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            Interaction::default(),
            MenuButton,
            SettingAdjust { kind, delta },
        ))
        .with_children(|btn| {
            btn.spawn(TextBundle::from_section(
                glyph,
                flow::style(fonts, 18.0, Color::srgb(0.92, 0.95, 1.0)),
            ));
        });
}

/// 开源代码鸣谢面板：逐条列出核心开源库及其用途（与 v0.3.2 一致）。
pub fn spawn_credits_panel(parent: &mut ChildBuilder, fonts: &CjkFont) {
    /// (库名 · 版本, 一句话说明"是什么、用在哪")。
    const CREDITS: &[(&str, &str)] = &[
        ("Bevy 0.14", "3D 游戏引擎（MIT / Apache-2.0）—— 渲染、输入、UI、ECS 场景调度"),
        ("Tokio 1.35", "异步运行时（MIT）—— 驱动固定 Tick 主循环的实时节流与定时"),
        ("Serde / serde_yaml", "序列化框架（MIT / Apache-2.0）—— 解析 config/element_reactions.yaml 配置"),
        ("Tracing", "结构化日志（MIT）—— 主循环、战局与档案系统的运行日志输出"),
        ("Rand / rand_pcg", "随机数（MIT / Apache-2.0）—— 装备元素掉落与 AI 行为的确定性随机"),
        ("Crossbeam-queue", "无锁队列（MIT / Apache-2.0）—— HAL 层环形缓冲的低延迟通信"),
        ("BLAKE3 1.5", "密码学哈希（CC0 / Apache-2.0）—— 交易记录审计与确定性验证的状态哈希"),
        ("Criterion 0.5", "基准测试框架（MIT / Apache-2.0，仅 dev 依赖）—— 性能回归基准"),
    ];

    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Px(760.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(4.0),
                padding: UiRect::px(20.0, 10.0, 14.0, 12.0),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.06, 0.09, 0.13, 0.92)),
            border_color: BorderColor(Color::srgb(0.22, 0.28, 0.36)),
            border_radius: BorderRadius::all(Val::Px(4.0)),
            ..default()
        })
        .with_children(|panel| {
            panel.spawn(TextBundle::from_section(
                "开 源 代 码 鸣 谢",
                flow::style(fonts, 17.0, menu_accent()),
            ));
            panel.spawn(TextBundle::from_section(
                "本项目是开源软件（GPL-3.0 with linking exception），站在下列开源库的肩膀上",
                flow::style(fonts, 12.0, Color::srgb(0.55, 0.62, 0.72)),
            ));
            panel.spawn(NodeBundle {
                style: Style { height: Val::Px(4.0), ..default() },
                ..default()
            });
            for (name, desc) in CREDITS {
                panel
                    .spawn(NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceBetween,
                            column_gap: Val::Px(16.0),
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn(TextBundle::from_section(
                            *name,
                            flow::style(fonts, 13.0, Color::srgb(0.85, 0.89, 0.95)),
                        ));
                        row.spawn(TextBundle::from_section(
                            *desc,
                            flow::style(fonts, 13.0, Color::srgb(0.62, 0.70, 0.80)),
                        ));
                    });
            }
        });
}

/// 每项设置的步进量。
pub fn setting_step(kind: SettingKind) -> f32 {
    match kind {
        SettingKind::Sensitivity => 0.1,
        SettingKind::Fov => 5.0,
        SettingKind::Ambient => 0.05,
    }
}

/// 设置项当前值的显示文本。
pub fn setting_label(settings: &GameSettings, kind: SettingKind) -> String {
    match kind {
        SettingKind::Sensitivity => format!("x{:.1}", settings.mouse_sensitivity),
        SettingKind::Fov => format!("{:.0}", settings.fov_deg),
        SettingKind::Ambient => format!("{:.2}", settings.ambient_brightness),
    }
}

/// 应用一步增量，并 clamp 到合理区间。
pub fn apply_setting_step(settings: &mut GameSettings, kind: SettingKind, delta: f32) {
    match kind {
        SettingKind::Sensitivity => settings.mouse_sensitivity = (settings.mouse_sensitivity + delta).clamp(0.2, 3.0),
        SettingKind::Fov => settings.fov_deg = (settings.fov_deg + delta).clamp(40.0, 110.0),
        SettingKind::Ambient => settings.ambient_brightness = (settings.ambient_brightness + delta).clamp(0.10, 1.20),
    }
}

/// 把设置 FOV 幂等应用到轨道相机投影。
///
/// 不做 is_changed 短路：重建相机（返回主菜单再进游戏）会回到默认 FOV，每帧幂等
/// 应用才能让设置在重建后保持一致。
pub fn settings_apply_fov(
    settings: Res<GameSettings>,
    mut cameras: Query<&mut Projection, With<ChaseCamera>>,
) {
    let target = settings.fov_deg.to_radians();
    for mut projection in &mut cameras {
        if let Projection::Perspective(perspective) = &mut *projection {
            if perspective.fov != target {
                perspective.fov = target;
            }
        }
    }
}

/// 把设置的环境亮度写入世界 `AmbientLight`（仅当值变化时）。
pub fn settings_apply_ambient(settings: Res<GameSettings>, mut ambient: ResMut<AmbientLight>) {
    if !settings.is_changed() {
        return;
    }
    ambient.brightness = settings.ambient_brightness;
}