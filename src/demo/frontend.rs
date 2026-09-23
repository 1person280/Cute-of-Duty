//! 跨模块共享的基础设施：光标锁定、CJK 字体、元素系统胶水、全局状态
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use crate::element::{ElementConfig, ElementSystem, ElementType};
use super::components::*;
use super::inventory::HeldGrenade;

/// 核心库不依赖bevy，Resource trait由Demo侧的newtype提供
#[derive(Resource)]
pub(crate) struct ElementalSystem(pub(crate) ElementSystem);

/// 前端流程状态机：启动先过加载页，主菜单选择训练场后才搭建游戏世界
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) enum AppState {
    #[default]
    Loading,
    MainMenu,
    InGame,
}

/// 回到前端界面时释放鼠标（返回主菜单时复用）
pub(crate) fn release_cursor(
    mut window_query: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut input_state: ResMut<InputState>,
) {
    unlock_cursor(&mut window_query, &mut input_state);
}

pub(crate) fn lock_cursor(windows: &mut Query<&mut CursorOptions, With<PrimaryWindow>>, input_state: &mut InputState) {
    if let Ok(mut cursor) = windows.single_mut() {
        cursor.visible = false;
        cursor.grab_mode = CursorGrabMode::Locked;
    }
    input_state.cursor_locked = true;
}

pub(crate) fn unlock_cursor(windows: &mut Query<&mut CursorOptions, With<PrimaryWindow>>, input_state: &mut InputState) {
    if let Ok(mut cursor) = windows.single_mut() {
        cursor.visible = true;
        cursor.grab_mode = CursorGrabMode::None;
    }
    input_state.cursor_locked = false;
}

pub(crate) const CJK_FONT_BYTES: &[u8] = include_bytes!("../../assets/fonts/simhei.ttf");

pub(crate) fn load_cjk_font(mut fonts: ResMut<Assets<Font>>) {
    // Bevy 0.19 中 TextFont.font 默认取 FontSource::default()，
    // 其解析到 Handle<Font>::default() 这个默认字体资产；
    // 以 CJK 字体原地覆盖该资产 id 即可让所有未显式指定字体的文本获得中文支持。
    let font = Font::from_bytes(CJK_FONT_BYTES.to_vec());
    let _ = fonts.insert(Handle::<Font>::default().id(), font);
}

/// bevy_log 的订阅者在 DefaultPlugins 构建时才安装，
/// App::new() 之前用 bevy::log 打印的内容会被静默丢弃；
/// 因此启动期中文日志统一放到 Startup 系统里输出。
pub(crate) fn log_element_config(system: Res<ElementalSystem>) {
    bevy::log::info!("元素配置加载完成: {} 种反应规则", system.0.reaction_count());
}

// =============================================================================
// Element Definitions
// =============================================================================

// 元素类型直接复用核心库（crate::element::ElementType），
// 与cod1共享同一套配置表驱动的反应逻辑；
// 颜色/发光等纯表现信息由Demo通过扩展trait提供。

/// 元素表现层扩展（Demo专用）：核心库不含渲染信息
pub(crate) trait ElementVisual {
    fn color(&self) -> Color;
    fn emissive(&self) -> LinearRgba;
}

impl ElementVisual for ElementType {
    fn color(&self) -> Color {
        match self {
            ElementType::Fire => Color::srgb(1.0, 0.45, 0.12),
            ElementType::Ice => Color::srgb(0.35, 0.80, 1.0),
            ElementType::Electric => Color::srgb(0.95, 1.0, 0.15),
            ElementType::Poison => Color::srgb(0.50, 0.95, 0.20),
            ElementType::Physical => Color::srgb(0.75, 0.75, 0.75),
            ElementType::Water => Color::srgb(0.30, 0.55, 0.95),
        }
    }

    fn emissive(&self) -> LinearRgba {
        self.color().to_linear() * 6.0
    }
}

/// 加载元素配置文件（核心库统一加载器：自动定位项目根目录）。
/// 此函数在 App::new() 之前执行，bevy_log 订阅者尚未安装，
/// 必须用 eprintln! 输出，否则日志会被静默丢弃。
/// 文件缺失回退内置默认配置；解析失败直接退出——
/// 改错配置表应当大声失败，而不是静默用默认值让设计师误以为改动生效。
pub(crate) fn load_element_config() -> ElementConfig {
    match crate::config::load_element_config() {
        Ok((config, Some(path))) => {
            eprintln!(
                "元素配置加载完成: {} ({} 种反应规则)",
                path.display(),
                config.reactions.len()
            );
            config
        }
        Ok((config, None)) => {
            eprintln!("配置文件未找到(config/element_reactions.yaml)，使用内置默认配置");
            config
        }
        Err(e) => {
            eprintln!("{e}");
            eprintln!("请修正 config/element_reactions.yaml 后重新启动");
            std::process::exit(1);
        }
    }
}

pub(crate) fn grab_cursor(
    mut window_query: Query<&mut CursorOptions, With<PrimaryWindow>>,
    mut input_state: ResMut<InputState>,
) {
    let mut cursor = window_query.single_mut().unwrap();
    cursor.visible = false;
    cursor.grab_mode = CursorGrabMode::Locked;
    input_state.cursor_locked = true;
}

pub(crate) fn cursor_grab_toggle(
    mut window_query: Query<&mut CursorOptions, With<PrimaryWindow>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut input_state: ResMut<InputState>,
    wheel: Res<WheelState>,
    held: Res<HeldGrenade>,
    open_station: Res<OpenStation>,
    backpack_ui: Query<&Visibility, With<InventoryUI>>,
    crate_win: Res<super::supply_crate::CrateWindow>,
) {
    // 轮盘打开时 Esc 由轮盘负责（取消并收起），这里跳过避免抢占光标状态
    if wheel.open || wheel.pending_key.is_some() { return; }
    // 手雷持握时 Esc 由 grenade_throw_system 负责（取消持握放回背包），光标保持锁定
    if held.item.is_some() { return; }
    // 站点面板打开时 Esc 由 station_system 负责（本系统在它之前运行，看到打开态直接跳过）
    if *open_station != OpenStation::None { return; }
    // 物资箱窗口打开时 Esc 由 station_system 负责关闭，这里跳过避免抢占光标
    if crate_win.crate_entity.is_some() { return; }
    // 背包打开时 Esc 由 inventory_toggle 负责关闭背包；
    // 若在这里把光标锁回，随后 station_system 会看到"光标已锁 + Esc"而误开补给台
    if backpack_ui.single().map_or(false, |vis| *vis == Visibility::Visible) { return; }
    if keyboard.just_pressed(KeyCode::Escape) {
        let mut cursor = window_query.single_mut().unwrap();
        if cursor.grab_mode == CursorGrabMode::Locked {
            cursor.visible = true;
            cursor.grab_mode = CursorGrabMode::None;
            input_state.cursor_locked = false;
        } else {
            cursor.visible = false;
            cursor.grab_mode = CursorGrabMode::Locked;
            input_state.cursor_locked = true;
        }
    }
}
