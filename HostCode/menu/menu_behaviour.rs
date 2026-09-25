//! 主菜单行为层：输入分发、按钮动作、模式/分类选择与样式刷新。
//!
//! 设计动机（Why）：把主菜单 UI 树的交互与样式从 `menu_main` 的构建中分离，保持每个
//! 模块职责单一。进场（`开始游戏`）只把选中的「训练场」吸收为 `StartTraining` 上行、
//! 由服务端裁决；不可用模式只刷状态行「敬请期待」、不发任何协议。仓库浮层由一个纯
//! 显隐（`ArsenalVisible`）开关驱动，其自身交互仍在 `arsenal`。

use bevy::prelude::*;
use cute_of_duty_server::net::protocol::ClientMessage;

use super::arsenal::ArsenalVisible;
use crate::flow::flow_state::AppState;
use super::game_settings::{
    apply_setting_step, setting_label, GameSettings, SettingAdjust, SettingValueText,
};
use super::menu_main::{
    menu_accent, menu_button_palette, GearButton, GearIcon, LoadoutButton, MainMenuUi, MenuButton,
    MenuGrace, QuitButton, SettingsCloseButton, StatusText,
};
use super::mode_panel::{
    game_mode_spec, CategoryButton, ModePanelRoot, ModeRow, SelectedCategory, SelectedMode,
    StartGameButton, SwitchModeButton,
};
use crate::net::network::NetOut;

/// 主菜单按钮动作查询：五种按钮标记各为 Optional，借类型别名收窄长元组。
type MenuButtons<'w, 's> = Query<
    'w,
    's,
    (
        &'static Interaction,
        Option<&'static SwitchModeButton>,
        Option<&'static StartGameButton>,
        Option<&'static GearButton>,
        Option<&'static SettingsCloseButton>,
        Option<&'static QuitButton>,
    ),
    Changed<Interaction>,
>;

/// 主菜单样式层长可变查询别名：以互斥 With/Without 标记隔离，规避 clippy::type_complexity。
type MenuHoverQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Interaction, &'static mut BackgroundColor, &'static mut BorderColor),
    (Changed<Interaction>, With<MenuButton>),
>;
type MenuModeRowQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static ModeRow,
        &'static Interaction,
        &'static mut BackgroundColor,
        &'static mut BorderColor,
        &'static mut Visibility,
    ),
    (Without<MenuButton>, Without<CategoryButton>, Without<ModePanelRoot>),
>;
type MenuCategoryQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static CategoryButton,
        &'static Interaction,
        &'static mut BackgroundColor,
        &'static mut BorderColor,
    ),
    (Without<MenuButton>, Without<ModeRow>),
>;

/// 推进主菜单输入保护期计时器（在交互系统之前运行）。
pub fn tick_grace(time: Res<Time>, mut grace: ResMut<MenuGrace>) {
    grace.0.tick(time.delta());
}

/// 主菜单按钮/输入分发：设置开关、模式切换、进场、退出与设置步进。
pub fn main_menu_interaction(
    mut next_state: ResMut<NextState<AppState>>,
    mut app_exit: EventWriter<AppExit>,
    out: Res<NetOut>,
    grace: Res<MenuGrace>,
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<GameSettings>,
    mut selected: ResMut<SelectedMode>,
    mut category: ResMut<SelectedCategory>,
    arsenal_vis: Res<ArsenalVisible>,
    ui: Res<MainMenuUi>,
    mut visibility: Query<&mut Visibility>,
    mut value_texts: Query<(&SettingValueText, &mut Text), Without<StatusText>>,
    mut status_texts: Query<&mut Text, (With<StatusText>, Without<SettingValueText>)>,
    buttons: MenuButtons,
    adjust_buttons: Query<(&SettingAdjust, &Interaction), Changed<Interaction>>,
    select_buttons: Query<(&Interaction, Option<&CategoryButton>, Option<&ModeRow>), Changed<Interaction>>,
) {
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
    // 仓库浮层由独立 `ArsenalVisible` 资源驱动。
    let loadout_open = arsenal_vis.0;

    // 按钮动作分发（任一浮层打开时，除各自「返回」外全部拦截）
    for (interaction, switch, start, gear, close, quit) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if loadout_open {
            continue;
        }
        if settings_open && close.is_none() {
            continue;
        }
        if close.is_some() {
            if let Ok(mut vis) = visibility.get_mut(ui.settings_overlay) {
                *vis = Visibility::Hidden;
            }
            if let Ok(mut vis) = visibility.get_mut(ui.settings_backdrop) {
                *vis = Visibility::Hidden;
            }
        } else if switch.is_some() {
            if let Ok(mut vis) = visibility.get_mut(ui.mode_panel) {
                let open = matches!(*vis, Visibility::Visible);
                *vis = if open { Visibility::Hidden } else { Visibility::Visible };
            }
        } else if start.is_some() {
            let spec = game_mode_spec(selected.0);
            if spec.available {
                let _ = out.0.send(ClientMessage::StartTraining);
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
            if let Ok(mut vis) = visibility.get_mut(ui.settings_backdrop) {
                *vis = Visibility::Visible;
            }
        } else if quit.is_some() {
            app_exit.send(AppExit::Success);
            return;
        }
    }

    // 分类 / 模式选择（任一浮层打开时不响应）
    let any_overlay = settings_open || loadout_open;
    if !any_overlay {
        for (interaction, cat, row) in &select_buttons {
            if *interaction != Interaction::Pressed {
                continue;
            }
            if let Some(cat) = cat {
                category.0 = cat.0;
            } else if let Some(row) = row {
                selected.0 = row.0;
            }
        }
    }

    // 设置浮层里的步进调节
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

/// 仓库（携带物资）面板行为：`LoadoutButton` 开关 `ArsenalVisible`。
pub fn main_menu_loadout(
    mut arsenal_vis: ResMut<ArsenalVisible>,
    loadout_btn: Query<&Interaction, (With<LoadoutButton>, Changed<Interaction>)>,
) {
    for interaction in &loadout_btn {
        if *interaction == Interaction::Pressed {
            arsenal_vis.0 = !arsenal_vis.0;
        }
    }
}

/// 主菜单样式层：普通按钮悬停高亮、齿轮图标变色、模式行选中/置灰/分类过滤、状态行同步。
/// 只在有输入或选中态变化时重刷，避免每帧覆写样式导致变更检测空转。
#[allow(clippy::too_many_arguments)]
pub fn main_menu_style(
    ui: Res<MainMenuUi>,
    selected: Res<SelectedMode>,
    category: Res<SelectedCategory>,
    panel_vis: Query<&Visibility, (With<ModePanelRoot>, Without<ModeRow>)>,
    gear_hover: Query<&Interaction, (With<GearButton>, Changed<Interaction>)>,
    mut gear_icon: Query<&mut UiImage, With<GearIcon>>,
    mut hover_buttons: MenuHoverQuery,
    row_changed: Query<&Interaction, (With<ModeRow>, Changed<Interaction>)>,
    category_changed: Query<&Interaction, (With<CategoryButton>, Changed<Interaction>)>,
    mut mode_rows: MenuModeRowQuery,
    mut categories: MenuCategoryQuery,
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

    if let Ok(interaction) = gear_hover.get_single() {
        if let Ok(mut image) = gear_icon.get_single_mut() {
            image.color = if *interaction == Interaction::Hovered {
                menu_accent()
            } else {
                Color::WHITE
            };
        }
    }

    // 模式行：面板打开时按分类过滤可见；选中高亮、未开放置灰。
    // bevy 0.14 中被显式写成 Visible 的子节点会在隐藏父节点下漏渲染，因此行可见性
    // 必须与面板显隐联动，不能只看分类。
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

    if selected.is_changed() {
        if let Ok(mut text) = status_texts.get_mut(ui.status_text) {
            text.sections[0].value = format!("当前模式：{}", game_mode_spec(selected.0).name);
        }
    }
}