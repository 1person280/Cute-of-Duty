//! 主菜单行为层：输入分发、按钮动作、模式/分类选择与样式刷新。
//! 依赖 main_menu / mode_panel / settings_panel 提供的 UI 句柄、资源与标记。

use bevy::prelude::*;
use super::*;
use crate::demo::frontend::*;
use crate::demo::pause::*;

#[allow(clippy::type_complexity)]
pub(crate) fn main_menu_interaction(
    mut next_state: ResMut<NextState<AppState>>,
    mut app_exit: EventWriter<AppExit>,
    time: Res<Time>,
    mut grace: ResMut<MenuGrace>,
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<GameSettings>,
    mut selected: ResMut<SelectedMode>,
    mut category: ResMut<SelectedCategory>,
    ui: Res<MainMenuUi>,
    mut visibility: Query<&mut Visibility>,
    mut value_texts: Query<(&SettingValueText, &mut Text), Without<StatusText>>,
    mut status_texts: Query<&mut Text, (With<StatusText>, Without<SettingValueText>)>,
    buttons: Query<
        (
            &Interaction,
            Option<&SwitchModeButton>,
            Option<&StartGameButton>,
            Option<&GearButton>,
            Option<&SettingsCloseButton>,
            Option<&QuitButton>,
        ),
        Changed<Interaction>,
    >,
    adjust_buttons: Query<(&SettingAdjust, &Interaction), Changed<Interaction>>,
    category_buttons: Query<(&CategoryButton, &Interaction), Changed<Interaction>>,
    mode_rows: Query<(&ModeRow, &Interaction), Changed<Interaction>>,
) {
    grace.0.tick(time.delta());
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

    // 按钮动作分发（设置浮层打开时，除"返回"外全部拦截）
    for (interaction, switch, start, gear, close, quit) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if settings_open && close.is_none() {
            continue;
        }
        if close.is_some() {
            if let Ok(mut vis) = visibility.get_mut(ui.settings_overlay) {
                *vis = Visibility::Hidden;
            }
            if let Ok(mut backdrop_vis) = visibility.get_mut(ui.settings_backdrop) {
                *backdrop_vis = Visibility::Hidden;
            }
        } else if switch.is_some() {
            if let Ok(mut vis) = visibility.get_mut(ui.mode_panel) {
                let open = matches!(*vis, Visibility::Visible);
                *vis = if open { Visibility::Hidden } else { Visibility::Visible };
            }
        } else if start.is_some() {
            let spec = game_mode_spec(selected.0);
            if spec.available {
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
            if let Ok(mut backdrop_vis) = visibility.get_mut(ui.settings_backdrop) {
                *backdrop_vis = Visibility::Visible;
            }
        } else if quit.is_some() {
            app_exit.send(AppExit::Success);
            return;
        }
    }

    // 分类 / 模式选择（面板内部互斥：设置浮层打开时不响应）
    if !settings_open {
        for (cat, interaction) in &category_buttons {
            if *interaction == Interaction::Pressed {
                category.0 = cat.0;
            }
        }
        for (row, interaction) in &mode_rows {
            if *interaction == Interaction::Pressed {
                selected.0 = row.0;
            }
        }
    }

    // 设置浮层里的步进调节（与暂停菜单共用一套设置行控件）
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

/// 主菜单样式层：普通按钮悬停高亮、齿轮图标变色、模式行选中/置灰/分类过滤、状态行同步。
/// 只在有输入或选中态变化时重刷，避免每帧覆写样式导致变更检测空转。
pub(crate) fn main_menu_style(
    ui: Res<MainMenuUi>,
    selected: Res<SelectedMode>,
    category: Res<SelectedCategory>,
    panel_vis: Query<&Visibility, (With<ModePanelRoot>, Without<ModeRow>)>,
    gear_hover: Query<&Interaction, (With<GearButton>, Changed<Interaction>)>,
    mut gear_icon: Query<&mut UiImage, With<GearIcon>>,
    mut hover_buttons: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<MenuButton>),
    >,
    row_changed: Query<&Interaction, (With<ModeRow>, Changed<Interaction>)>,
    category_changed: Query<&Interaction, (With<CategoryButton>, Changed<Interaction>)>,
    mut mode_rows: Query<
        (&ModeRow, &Interaction, &mut BackgroundColor, &mut BorderColor, &mut Visibility),
        (Without<MenuButton>, Without<CategoryButton>, Without<ModePanelRoot>),
    >,
    mut categories: Query<
        (&CategoryButton, &Interaction, &mut BackgroundColor, &mut BorderColor),
        (Without<MenuButton>, Without<ModeRow>),
    >,
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

    // 普通按钮（切换/开始/退出/返回/齿轮底板）悬停高亮
    for (interaction, mut bg, mut border) in &mut hover_buttons {
        match *interaction {
            Interaction::Hovered => { *bg = hover_bg; *border = hover_border; }
            Interaction::None => { *bg = base_bg; *border = base_border; }
            Interaction::Pressed => {}
        }
    }

    // 齿轮图标随悬停着色
    if let Ok(interaction) = gear_hover.get_single() {
        if let Ok(mut image) = gear_icon.get_single_mut() {
            image.color = if *interaction == Interaction::Hovered {
                menu_accent()
            } else {
                Color::WHITE
            };
        }
    }

    // 模式行：面板打开时按分类过滤可见；未开放置灰、选中高亮。
    // 注意 bevy 0.14 中被显式写成 Visible 的子节点会在隐藏父节点下漏渲染，
    // 因此行可见性必须与面板显隐联动，不能只看分类。
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

    // 分类按钮：选中高亮
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

    // 选中模式变化 → 右下角状态行同步
    if selected.is_changed() {
        if let Ok(mut text) = status_texts.get_mut(ui.status_text) {
            text.sections[0].value = format!("当前模式：{}", game_mode_spec(selected.0).name);
        }
    }
}