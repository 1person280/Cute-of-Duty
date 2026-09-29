//! 仓库选装浮层的**交互与拖拽**：拖拽会话状态机 + Shift 快捷移动 + 确认/返回。
//!
//! 设计动机（Why）：把「谁被拖起、松手落在哪个容器、有没有按 Shift」全部收敛在此——
//! 共享状态定义在 `state`、绘制在 `refresh`，本模块只做输入判定与清单增删，且只上行
//! 选装意图，绝不本地裁决资格。

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use cute_of_duty_server::net::protocol::ClientMessage;

use crate::flow::flow_state::AppState;
use crate::net::network::NetOut;

use super::refresh::{carried_index, pool_color, write_loadout_texts};
use super::state::*;

/// 拖拽开始：显示幽灵物资名并定位到光标。
fn set_ghost(
    ghost: &mut GhostQ,
    ghost_text: &mut GhostTextQ,
    sel: &ArsenalSelection,
    source: &ArsenalDragSource,
    window: &mut Query<&mut Window, With<PrimaryWindow>>,
) {
    if let Ok((mut gv, mut gs)) = ghost.get_single_mut() {
        *gv = Visibility::Visible;
        if let Ok(w) = window.get_single_mut() {
            if let Some(cursor) = w.cursor_position() {
                gs.left = Val::Px(cursor.x - 70.0);
                gs.top = Val::Px(cursor.y - 20.0);
            }
        }
    }
    if let Ok((mut text, mut text_color)) = ghost_text.get_single_mut() {
        let (name, color) = match source {
            ArsenalDragSource::Warehouse(i) => (MVP_ITEMS[*i].to_string(), ITEM_COLORS[*i]),
            ArsenalDragSource::Carried(i) => match sel.0.get(*i) {
                Some(n) => (n.clone(), pool_color(n)),
                None => return,
            },
        };
        text.0 = name;
        text_color.0 = color;
    }
}

/// 仓库拖拽 + Shift+左键 主系统（对齐 0.3.2）。仅面板可见时刷新与响应。
#[allow(clippy::type_complexity)]
pub fn arsenal_drag_system(
    mouse: Res<ButtonInput<MouseButton>>,
    shift: Res<ButtonInput<KeyCode>>,
    mut drag: ResMut<ArsenalDrag>,
    mut sel: ResMut<ArsenalSelection>,
    panel_vis: Query<&Visibility, (With<ArsenalRoot>, Without<BackpackRow>, Without<ArsenalRow>)>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
    mut ghost: GhostQ,
    mut ghost_text: GhostTextQ,
    wh_rows: Query<(&ArsenalRow, &Interaction), Without<BackpackRow>>,
    bp_slots: Query<(&BackpackRow, &Interaction), Without<ArsenalRow>>,
    mut wh_bg: Query<(&ArsenalRow, &mut BackgroundColor), Without<BackpackRow>>,
    mut bp_row: Query<(&BackpackRow, &mut Visibility, &mut BackgroundColor), Without<ArsenalRow>>,
    mut wh_status: WhTextQ,
    mut bp_text: BpTextQ,
    mut cap_text: CapTextQ,
    zone_hover: Query<
        (&Interaction, Option<&WarehouseZone>, Option<&BackpackZone>),
        (Without<ArsenalRow>, Without<BackpackRow>),
    >,
) {
    let Ok(vis) = panel_vis.get_single() else {
        return;
    };
    if *vis != Visibility::Visible {
        if drag.source.take().is_some() {
            if let Ok((mut gv, _)) = ghost.get_single_mut() {
                *gv = Visibility::Hidden;
            }
        }
        return;
    }

    write_loadout_texts(&sel, &mut wh_status, &mut bp_text, &mut cap_text);
    for (row, mut bg) in &mut wh_bg {
        let on = carried_index(&sel, row.index).is_some();
        *bg = if on {
            Color::srgb(0.55, 0.46, 0.16).into()
        } else {
            Color::srgb(0.20, 0.22, 0.25).into()
        };
    }
    for (row, mut v, mut bg) in &mut bp_row {
        // 4×3 网格格位恒显（空格位显示"空"），仅按是否携带改底色。
        *v = Visibility::Visible;
        let on = sel.0.get(row.index).is_some();
        *bg = if on {
            Color::srgb(0.28, 0.40, 0.30).into()
        } else {
            Color::srgb(0.20, 0.22, 0.25).into()
        };
    }

    // 松手落点：光标是否停在仓库 / 背包容器上。
    let mut over_warehouse = false;
    let mut over_backpack = false;
    for (inter, whz, bpz) in zone_hover.iter() {
        if *inter != Interaction::Hovered {
            continue;
        }
        if whz.is_some() {
            over_warehouse = true;
        }
        if bpz.is_some() {
            over_backpack = true;
        }
    }

    // ——— 拖拽开始（非 Shift 左键按住）———
    if drag.source.is_none() && mouse.just_pressed(MouseButton::Left) && !shift.pressed(KeyCode::ShiftLeft)
    {
        if let Some((row, _)) = wh_rows.iter().find(|(_, it)| **it == Interaction::Pressed) {
            if carried_index(&sel, row.index).is_none() {
                let s = ArsenalDragSource::Warehouse(row.index);
                drag.source = Some(s);
                set_ghost(&mut ghost, &mut ghost_text, &sel, &s, &mut window);
            }
        } else if let Some((slot, _)) = bp_slots.iter().find(|(_, it)| **it == Interaction::Pressed) {
            if slot.index < sel.0.len() {
                let s = ArsenalDragSource::Carried(slot.index);
                drag.source = Some(s);
                set_ghost(&mut ghost, &mut ghost_text, &sel, &s, &mut window);
            }
        }
    }

    // ——— 拖拽中：幽灵跟随光标；松手按落区移动 ———
    if let Some(source) = drag.source {
        if let Ok((_, mut style)) = ghost.get_single_mut() {
            if let Ok(w) = window.get_single_mut() {
                if let Some(cursor) = w.cursor_position() {
                    style.left = Val::Px(cursor.x - 70.0);
                    style.top = Val::Px(cursor.y - 20.0);
                }
            }
        }
        if mouse.just_released(MouseButton::Left) {
            match source {
                ArsenalDragSource::Warehouse(idx) if over_backpack => {
                    if sel.0.len() < LOADOUT_CAPACITY && carried_index(&sel, idx).is_none() {
                        sel.0.push(MVP_ITEMS[idx].to_string());
                    }
                }
                ArsenalDragSource::Carried(idx) if over_warehouse => {
                    if idx < sel.0.len() {
                        sel.0.remove(idx);
                    }
                }
                _ => {}
            }
            drag.source = None;
            if let Ok((mut gv, _)) = ghost.get_single_mut() {
                *gv = Visibility::Hidden;
            }
        }
        return;
    }

    // ——— Shift+左键 快捷移动 ———
    if shift.pressed(KeyCode::ShiftLeft) && mouse.just_pressed(MouseButton::Left) {
        if let Some((row, _)) = wh_rows.iter().find(|(_, it)| **it == Interaction::Pressed) {
            if sel.0.len() < LOADOUT_CAPACITY && carried_index(&sel, row.index).is_none() {
                sel.0.push(MVP_ITEMS[row.index].to_string());
            }
        } else if let Some((slot, _)) = bp_slots.iter().find(|(_, it)| **it == Interaction::Pressed) {
            if slot.index < sel.0.len() {
                sel.0.remove(slot.index);
            }
        }
    }
}

/// 浮层交互：显隐跟随、返回、确认进场（上报选装 + 请求进场）。拖拽由 `arsenal_drag_system` 负责。
pub fn arsenal_interaction(
    out: Res<NetOut>,
    mut next_state: ResMut<NextState<AppState>>,
    mut vis: ResMut<ArsenalVisible>,
    selection: Res<ArsenalSelection>,
    mut root_q: Query<&mut Visibility, With<ArsenalRoot>>,
    confirm: Query<&Interaction, (With<JinButton>, Changed<Interaction>)>,
    back: Query<&Interaction, (With<BackButton>, Changed<Interaction>)>,
) {
    if let Ok(mut visibility) = root_q.get_single_mut() {
        *visibility = if vis.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for interaction in &back {
        if *interaction == Interaction::Pressed {
            vis.0 = false;
        }
    }
    for interaction in &confirm {
        if *interaction == Interaction::Pressed {
            let _ = out.0.send(ClientMessage::Loadout {
                carried: selection.0.clone(),
            });
            let _ = out.0.send(ClientMessage::StartTraining);
            vis.0 = false;
            next_state.set(AppState::InGame);
        }
    }
}