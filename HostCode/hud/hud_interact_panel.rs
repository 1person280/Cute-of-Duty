//! 交互面板渲染（纯表现）：一级居中列表（行 / 滚动条 / 页码）+ 站点二级选项面板
//!
//! 设计动机（Why）：把"画"与"算"分开——`hud_interact` 维护状态与输入（列出附近目标、
//! 维护高亮/窗口、生成上报意图），本模块只把状态映射成 UI 节点属性，不含任何判定。
//! 列表行是**固定槽位**（装配时一次生成 `INTERACT_VISIBLE_ROWS` 行），每帧只改文本与
//! 高亮色，避免每帧增删节点带来的 `Children` 抖动；二级面板选项数少（≤4）且随高亮变化，
//! 按 `(open, target, 高亮）` 变化重建即可。

use bevy::prelude::*;

use crate::flow::flow_state::{self as flow, CjkFont};
use crate::shared::theme;

use super::hud_interact::{
    InteractHintText, InteractMenuList, InteractMenuRoot, InteractMenuTitle, InteractOptionIndex,
    InteractPanel, InteractRowSlot, InteractRowText, InteractScrollThumb, InteractState,
    INTERACT_ROW_GAP, INTERACT_ROW_H, INTERACT_VISIBLE_ROWS,
};

/// 列表滚动条定高（与行区同高）。
fn track_height() -> f32 {
    INTERACT_VISIBLE_ROWS as f32 * (INTERACT_ROW_H + INTERACT_ROW_GAP) - INTERACT_ROW_GAP
}

/// 每帧刷新常驻接近列表：可见性、行文本、高亮色、滚动条位置、页码提示。
#[allow(clippy::type_complexity)]
pub fn sync_interact_panel(
    state: Res<InteractState>,
    mut root: Query<&mut Visibility, With<InteractPanel>>,
    mut rows: Query<(&InteractRowSlot, &mut BackgroundColor, &mut BorderColor, &mut Style)>,
    mut texts: Query<(&InteractRowText, &mut Text)>,
    mut thumb: Query<&mut Style, (With<InteractScrollThumb>, Without<InteractRowSlot>)>,
    mut hint: Query<&mut Text, (With<InteractHintText>, Without<InteractRowText>)>,
) {
    let n = state.entries.len();
    // 就近列表常显：附近有目标且未进入二级面板即可见（无需先按 F）。
    let show = n > 0 && !state.panel_open;
    if let Ok(mut v) = root.get_single_mut() {
        *v = if show { Visibility::Visible } else { Visibility::Hidden };
    }
    if !show {
        return;
    }

    for (slot, mut bg, mut border, mut style) in &mut rows {
        let idx = state.scroll_start + slot.0;
        if idx >= n {
            style.display = Display::None;
            continue;
        }
        style.display = Display::Flex;
        let selected = idx == state.selected;
        *bg = if selected { theme::ROW_HOVER.into() } else { theme::PANEL_BG.into() };
        *border = BorderColor(if selected { theme::ACCENT_AMBER } else { theme::PANEL_BORDER });
    }

    for (slot, mut text) in &mut texts {
        let idx = state.scroll_start + slot.0;
        match state.entries.get(idx) {
            Some(entry) => {
                let selected = idx == state.selected;
                text.sections[0].value =
                    format!("{}{}", if selected { "▶ " } else { "  " }, entry.label);
                text.sections[0].style.color =
                    if selected { theme::TEXT_WHITE } else { theme::TEXT_DIM };
            }
            None => text.sections[0].value = String::new(),
        }
    }

    if let Ok(mut style) = thumb.get_single_mut() {
        let track_h = track_height();
        let total = n as f32;
        let visible = INTERACT_VISIBLE_ROWS.min(n) as f32;
        let h = (track_h * visible / total).max(8.0);
        let top = if n > INTERACT_VISIBLE_ROWS {
            (track_h - h) * state.scroll_start as f32 / (total - visible)
        } else {
            0.0
        };
        style.height = Val::Px(h);
        style.top = Val::Px(top);
    }

    if let Ok(mut text) = hint.get_single_mut() {
        let pages = n.div_ceil(INTERACT_VISIBLE_ROWS);
        text.sections[0].value = if pages > 1 {
            let page = state.scroll_start / INTERACT_VISIBLE_ROWS + 1;
            format!("滚轮翻页（{page}/{pages}）· F 确认")
        } else {
            "滚轮翻页 · F 确认".to_string()
        };
    }
}

/// 二级选项面板：可见性 + 标题 + 选项行（按 `(open, target, 高亮)` 变化重建，选中行高亮）。
pub fn sync_interact_menu(
    mut commands: Commands,
    fonts: Res<CjkFont>,
    state: Res<InteractState>,
    mut last: Local<(bool, u64, usize)>,
    mut root: Query<&mut Visibility, With<InteractMenuRoot>>,
    list: Query<Entity, With<InteractMenuList>>,
    mut title: Query<&mut Text, With<InteractMenuTitle>>,
) {
    if let Ok(mut v) = root.get_single_mut() {
        *v = if state.panel_open { Visibility::Visible } else { Visibility::Hidden };
    }

    let key = (state.panel_open, state.target, state.option_selected);
    if *last == key {
        return;
    }
    *last = key;
    if !state.panel_open {
        return;
    }

    if let Ok(mut t) = title.get_single_mut() {
        t.sections[0].value = state.title.clone();
    }

    let Ok(list_entity) = list.get_single() else {
        return;
    };
    commands.entity(list_entity).despawn_descendants();
    for (i, opt) in state.options.iter().enumerate() {
        let selected = i == state.option_selected;
        let label = opt.label.clone();
        commands.entity(list_entity).with_children(|p| {
            p.spawn((
                ButtonBundle {
                    style: Style {
                        padding: UiRect::new(
                            Val::Px(14.0),
                            Val::Px(14.0),
                            Val::Px(8.0),
                            Val::Px(8.0),
                        ),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    background_color: if selected {
                        theme::ROW_HOVER.into()
                    } else {
                        theme::PANEL_BG.into()
                    },
                    border_color: BorderColor(if selected {
                        theme::ACCENT_AMBER
                    } else {
                        theme::PANEL_BORDER
                    }),
                    ..default()
                },
                InteractOptionIndex(i),
            ))
            .with_children(|b| {
                b.spawn(TextBundle::from_section(
                    format!("{}{}", if selected { "▶ " } else { "  " }, label),
                    flow::style(
                        &fonts,
                        18.0,
                        if selected { theme::TEXT_WHITE } else { theme::TEXT_DIM },
                    ),
                ));
            });
        });
    }
}
