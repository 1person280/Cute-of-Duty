//! 设置面板 UI：设置行构建 + FOV / 环境亮度应用系统。
//!
//! 设计动机（Why）：设置项是**可调的表现层参数**（灵敏度 / 视野 / 环境亮度）。期间只把
//! FOV 真实作用到本端相机投影、把环境亮度写入世界 `AmbientLight` —— 它们不触碰任何
//! 服务端权威数据。灵敏度因当前相机为绕点轨道 + WASD（无越肩瞄准），仅在面板存储与
//! 显示，预留给日后的鼠标瞄准输入。
//!
//! 边界：设置**数据与规则**（[`GameSettings`] / [`SettingKind`] / 步进 / 文本）归 `flow`；
//! 本文件只做面板 UI 构建与把设置作用到世界的应用系统。

use bevy::prelude::*;

use crate::world::camera::{ChaseCamera, AIM_FOV_NARROW};
use crate::flow::flow_state::{self as flow, CjkFont};
use crate::flow::{setting_label, setting_step, GameSettings, SettingKind};
use super::{menu_accent, MenuButton};

/// ◀ / ▶ 步进按钮：kind 对应设置项，delta 为步进量（负为减小）。
#[derive(Component)]
pub struct SettingAdjust {
    pub kind: SettingKind,
    pub delta: f32,
}

/// 设置项当前值文本。
#[derive(Component)]
pub struct SettingValueText(pub SettingKind);

/// 设置行：标签 + ◀ 值 ▶。
pub fn spawn_setting_row(
    parent: &mut ChildSpawnerCommands,
    fonts: &CjkFont,
    settings: &GameSettings,
    kind: SettingKind,
    label: &str,
) {
    parent
        .spawn((
            Node {
                width: Val::Px(460.0),
                height: Val::Px(46.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::px(18.0, 12.0, 0.0, 0.0),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.10, 0.14, 0.20, 0.95)),
            BorderColor(Color::srgb(0.22, 0.28, 0.36)),
            BorderRadius::all(Val::Px(4.0)),
        ))
        .with_children(|row| {
            row.spawn(flow::text(
                label,
                flow::style(fonts, 19.0, Color::srgb(0.85, 0.89, 0.95)),
            ));
            row.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|ctrl| {
                spawn_step_button(ctrl, fonts, "<", kind, -setting_step(kind));
                ctrl.spawn((
                    flow::text(
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
    parent: &mut ChildSpawnerCommands,
    fonts: &CjkFont,
    glyph: &str,
    kind: SettingKind,
    delta: f32,
) {
    parent
        .spawn((
            Node {
                width: Val::Px(40.0),
                height: Val::Px(34.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.14, 0.20, 0.28, 0.98)),
            BorderColor(menu_accent()),
            BorderRadius::all(Val::Px(4.0)),
            Interaction::default(),
            MenuButton,
            SettingAdjust { kind, delta },
        ))
        .with_children(|btn| {
            btn.spawn(flow::text(
                glyph,
                flow::style(fonts, 18.0, Color::srgb(0.92, 0.95, 1.0)),
            ));
        });
}

/// 开源代码鸣谢面板：逐条列出核心开源库及其用途（与 v0.3.2 一致）。
pub fn spawn_credits_panel(parent: &mut ChildSpawnerCommands, fonts: &CjkFont) {
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
        (
            "ZCOOL KuaiLe 站酷快乐体",
            "中文字体（SIL OFL 1.1）—— 界面中文显示；来源 https://github.com/googlefonts/zcool-kuaile",
        ),
    ];

    parent
        .spawn((
            Node {
                width: Val::Px(760.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(4.0),
                padding: UiRect::px(20.0, 10.0, 14.0, 12.0),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.09, 0.13, 0.92)),
            BorderColor(Color::srgb(0.22, 0.28, 0.36)),
            BorderRadius::all(Val::Px(4.0)),
        ))
        .with_children(|panel| {
            panel.spawn(flow::text(
                "开 源 代 码 鸣 谢",
                flow::style(fonts, 17.0, menu_accent()),
            ));
            panel.spawn(flow::text(
                "本项目代码以 GPL-3.0 with linking exception 开源、美术资产以 CC BY-NC-SA 4.0 授权，站在下列开源库的肩膀上",
                flow::style(fonts, 12.0, Color::srgb(0.55, 0.62, 0.72)),
            ));
            panel.spawn(Node { height: Val::Px(4.0), ..default() });
            for (name, desc) in CREDITS {
                panel
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        column_gap: Val::Px(16.0),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn(flow::text(
                            *name,
                            flow::style(fonts, 13.0, Color::srgb(0.85, 0.89, 0.95)),
                        ));
                        row.spawn(flow::text(
                            *desc,
                            flow::style(fonts, 13.0, Color::srgb(0.62, 0.70, 0.80)),
                        ));
                    });
            }
        });
}

/// 把设置 FOV（叠加越肩瞄准收窄）幂等应用到轨道相机投影。
///
/// 不做 is_changed 短路：重建相机（返回主菜单再进游戏）会回到默认 FOV，每帧幂等
/// 应用才能让设置在重建后保持一致。
///
/// 设计动机（Why）：越肩瞄准的 FOV 收窄**必须**复用本系统作为唯一写入点——若在
/// `world::follow_system` 里另开一处 `&mut Projection`，两系统同帧争用同一组件会在
/// 调度期触发 B0001 冲突 panic。故此处读相机自己维护的 `aim_blend` 过渡进度来缩放 FOV。
pub fn settings_apply_fov(
    settings: Res<GameSettings>,
    mut cameras: Query<(&mut Projection, &ChaseCamera)>,
) {
    for (mut projection, camera) in &mut cameras {
        let blend = camera.aim_blend;
        let eased = blend * blend * (3.0 - 2.0 * blend);
        let target = settings.fov_deg.to_radians() * (1.0 - AIM_FOV_NARROW * eased);
        if let Projection::Perspective(perspective) = &mut *projection {
            if perspective.fov != target {
                perspective.fov = target;
            }
        }
    }
}

/// 把设置的环境亮度写入世界 `AmbientLight`（仅当值变化时）。
///
/// 量纲映射（Why）：设置面板存储的逻辑亮度 0.10–1.20 沿用 bevy 0.15 的倍率习惯；
/// bevy 0.16 起 `brightness` 与光照强度同量纲（lux），故统一 ×4000 映射到物理值
/// （默认 0.55 → 2200 lux，对齐 8000 lux 主光的露天观感）。基准线 `spawn_scene_baseline`
/// 的首帧兜底值 = 默认逻辑值 × 同一系数，两处需同步修改。
pub fn settings_apply_ambient(settings: Res<GameSettings>, mut ambient: ResMut<AmbientLight>) {
    if !settings.is_changed() {
        return;
    }
    ambient.brightness = settings.ambient_brightness * 4_000.0;
}