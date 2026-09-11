//! 功能站点：补给台/干员切换台面板、点击发放、UI 刷新

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use crate::element::ElementType;
use crate::map::StationKind;
use crate::operator::roster;
use crate::model::{
    OperatorAccent, Player, OperatorState,
};
use super::inventory::HeldGrenade;
use super::common::*;
use super::components::*;

/// 构建两张功能台的交互面板（隐藏，由 station_system 控制显隐）
pub(crate) fn setup_station_ui(mut commands: Commands) {
    // --- 干员切换台面板 ---
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
            visibility: Visibility::Hidden,
            ..default()
        },
        OperatorUIRoot,
    )).with_children(|root| {
        root.spawn(NodeBundle {
            style: Style {
                width: Val::Px(520.0),
                height: Val::Auto,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(8.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.06, 0.06, 0.08, 0.95)),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            ..default()
        }).with_children(|panel| {
            panel.spawn(TextBundle {
                text: Text::from_section(
                    "干员切换台",
                    TextStyle { font_size: 20.0, color: Color::srgb(0.9, 0.9, 0.9), ..default() }
                ),
                ..default()
            });
            panel.spawn(NodeBundle {
                style: Style { width: Val::Percent(100.0), height: Val::Px(2.0), ..default() },
                background_color: BackgroundColor(Color::srgba(0.4, 0.4, 0.5, 0.3)),
                ..default()
            });
            for i in 0..roster().len() {
                panel.spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            height: Val::Auto,
                            padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::srgba(0.10, 0.10, 0.13, 0.92)),
                        border_color: BorderColor(Color::srgba(0.35, 0.35, 0.4, 0.6)),
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        ..default()
                    },
                    Interaction::default(),
                    OperatorCard(i),
                )).with_children(|card| {
                    card.spawn((
                        TextBundle {
                            text: Text::from_sections([
                                TextSection::new("", TextStyle { font_size: 15.0, color: Color::WHITE, ..default() }),
                                TextSection::new("", TextStyle { font_size: 12.0, color: Color::srgb(0.75, 0.75, 0.78), ..default() }),
                                TextSection::new("", TextStyle { font_size: 12.0, color: Color::srgb(0.75, 0.75, 0.78), ..default() }),
                                TextSection::new("", TextStyle { font_size: 11.0, color: Color::srgb(0.5, 0.5, 0.55), ..default() }),
                            ]),
                            ..default()
                        },
                        OperatorCardText(i),
                    ));
                });
            }
            panel.spawn((
                TextBundle {
                    text: Text::from_section(
                        "",
                        TextStyle { font_size: 13.0, color: Color::srgb(0.6, 0.9, 0.6), ..default() }
                    ),
                    ..default()
                },
                OperatorStatusText,
            ));
            panel.spawn(TextBundle {
                text: Text::from_section(
                    "点击卡片切换干员（Q/E 技能组与元素随之更换） · F / Esc 关闭",
                    TextStyle { font_size: 12.0, color: Color::srgb(0.5, 0.5, 0.55), ..default() }
                ),
                ..default()
            });
        });
    });

    // --- 无限物资补给台面板 ---
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
            visibility: Visibility::Hidden,
            ..default()
        },
        SupplyUIRoot,
    )).with_children(|root| {
        root.spawn(NodeBundle {
            style: Style {
                width: Val::Px(420.0),
                height: Val::Auto,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(8.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.06, 0.06, 0.08, 0.95)),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            ..default()
        }).with_children(|panel| {
            panel.spawn(TextBundle {
                text: Text::from_section(
                    "补给台 · 无限物资",
                    TextStyle { font_size: 20.0, color: Color::srgb(0.9, 0.9, 0.9), ..default() }
                ),
                ..default()
            });
            panel.spawn(NodeBundle {
                style: Style { width: Val::Percent(100.0), height: Val::Px(2.0), ..default() },
                background_color: BackgroundColor(Color::srgba(0.4, 0.4, 0.5, 0.3)),
                ..default()
            });
            for (i, (name, hint)) in SUPPLY_ROWS.iter().enumerate() {
                panel.spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            height: Val::Auto,
                            padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::srgba(0.10, 0.10, 0.13, 0.92)),
                        border_color: BorderColor(Color::srgba(0.35, 0.35, 0.4, 0.6)),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                    Interaction::default(),
                    SupplyRow(i),
                )).with_children(|row| {
                    row.spawn(TextBundle {
                        text: Text::from_sections([
                            TextSection::new(*name, TextStyle { font_size: 14.0, color: Color::srgb(0.9, 0.9, 0.9), ..default() }),
                            TextSection::new(format!("  —  {}", hint), TextStyle { font_size: 11.0, color: Color::srgb(0.5, 0.5, 0.55), ..default() }),
                        ]),
                        ..default()
                    });
                });
            }
            panel.spawn((
                TextBundle {
                    text: Text::from_section(
                        "",
                        TextStyle { font_size: 13.0, color: Color::srgb(0.6, 0.9, 0.6), ..default() }
                    ),
                    ..default()
                },
                SupplyStatusText,
            ));
            panel.spawn(TextBundle {
                text: Text::from_section(
                    "点击行领取（弹药入池、其余入背包） · F / Esc 关闭",
                    TextStyle { font_size: 12.0, color: Color::srgb(0.5, 0.5, 0.55), ..default() }
                ),
                ..default()
            });
        });
    });
    // 旧的"站点提示条"已并入底部统一交互菜单（interact_menu_update），不再单独生成
}

/// 站点面板开关：F 打开统一交互菜单（NearbyInteract）中选中的站点条目，
/// F / Esc 关闭已开面板并锁回光标。Esc 不再直接开站点（开面板统一走 F + 交互菜单），
/// 避免背包关闭同帧 Esc 被本系统抢走而误开面板。
/// 运行顺序在 cursor_grab_toggle 之后；靠近检测由 interact_detection_system 负责。
#[allow(clippy::too_many_arguments)]
pub(crate) fn station_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    mut input_state: ResMut<InputState>,
    mut open: ResMut<OpenStation>,
    wheel: Res<WheelState>,
    held: Res<HeldGrenade>,
    nearby: Res<NearbyInteract>,
    mut supply_vis: Query<&mut Visibility, (With<SupplyUIRoot>, Without<OperatorUIRoot>)>,
    mut operator_vis: Query<&mut Visibility, (With<OperatorUIRoot>, Without<SupplyUIRoot>)>,
) {
    // 轮盘打开/按住期间、手雷持握时不处理：开会在轮盘/瞄准上叠面板；关会把光标锁回
    let wheel_busy = wheel.open || wheel.pending_key.is_some();
    if wheel_busy || held.item.is_some() { return; }

    if keyboard.just_pressed(KeyCode::KeyF) {
        if *open == OpenStation::None {
            // 无 UI 占用（光标锁定）且选中条目是站点时才开面板；拾取物条目交给 interact_execute_system
            if input_state.cursor_locked {
                if let Some(InteractEntry::Station { kind, .. }) = nearby.entries.get(nearby.selected) {
                    *open = match kind {
                        StationKind::SupplyTable => OpenStation::Supply,
                        StationKind::OperatorDesk => OpenStation::Operator,
                    };
                    if let Ok(mut window) = window_query.get_single_mut() {
                        window.cursor.visible = true;
                        window.cursor.grab_mode = CursorGrabMode::None;
                    }
                    input_state.cursor_locked = false;
                }
            }
        } else {
            close_station_panel(&mut window_query, &mut input_state, &mut open);
        }
    } else if keyboard.just_pressed(KeyCode::Escape) {
        // Esc 只负责关闭已打开的面板
        if *open != OpenStation::None {
            close_station_panel(&mut window_query, &mut input_state, &mut open);
        }
    }

    // 面板显隐兜底：每帧按状态刷新，任何提前 return 都不会把面板留在屏幕上
    if let Ok(mut vis) = supply_vis.get_single_mut() {
        *vis = if *open == OpenStation::Supply { Visibility::Visible } else { Visibility::Hidden };
    }
    if let Ok(mut vis) = operator_vis.get_single_mut() {
        *vis = if *open == OpenStation::Operator { Visibility::Visible } else { Visibility::Hidden };
    }
}

/// 关闭站点面板：锁回光标并清空打开态
pub(crate) fn close_station_panel(
    window_query: &mut Query<&mut Window, With<PrimaryWindow>>,
    input_state: &mut InputState,
    open: &mut OpenStation,
) {
    *open = OpenStation::None;
    if let Ok(mut window) = window_query.get_single_mut() {
        window.cursor.visible = false;
        window.cursor.grab_mode = CursorGrabMode::Locked;
    }
    input_state.cursor_locked = true;
}

/// 无限补给台：按行发放物资（弹药直接入弹药池，其余入背包物资槽）
pub(crate) fn grant_supply(row: usize, inventory: &mut Inventory) -> String {
    match row {
        0 => {
            inventory.ammo_pool += 60;
            "已领取：步枪弹药 ×60（入弹药池）".to_string()
        }
        1 => push_supply(inventory, "医疗包", PickupType::Health { amount: 25.0 }),
        2 => push_supply(inventory, "护甲片", PickupType::Armor { amount: 20.0 }),
        3 => push_supply(inventory, "烈焰手雷", PickupType::Grenade { element: ElementType::Fire }),
        4 => push_supply(inventory, "冰霜手雷", PickupType::Grenade { element: ElementType::Ice }),
        5 => push_supply(inventory, "雷电手雷", PickupType::Grenade { element: ElementType::Electric }),
        6 => push_supply(inventory, "毒素手雷", PickupType::Grenade { element: ElementType::Poison }),
        _ => String::new(),
    }
}

pub(crate) fn push_supply(inventory: &mut Inventory, name: &str, item_type: PickupType) -> String {
    if inventory.items.len() >= inventory.max_slots {
        return "背包已满（先在背包中使用道具）".to_string();
    }
    inventory.items.push(PickupItem { name: name.to_string(), item_type });
    format!("已领取：{}", name)
}

/// 补给台点击：悬停行 + 左键领取一行物资（无限次）
pub(crate) fn supply_station_click_system(
    mouse: Res<ButtonInput<MouseButton>>,
    open: Res<OpenStation>,
    rows: Query<(&SupplyRow, &Interaction)>,
    mut player_query: Query<&mut Inventory, With<Player>>,
    mut status: Query<&mut Text, (With<SupplyStatusText>, Without<OperatorStatusText>)>,
) {
    if *open != OpenStation::Supply { return; }
    if !mouse.just_pressed(MouseButton::Left) { return; }
    let Some((row, _)) = rows.iter().find(|(_, interaction)| **interaction == Interaction::Pressed) else { return };
    let Ok(mut inventory) = player_query.get_single_mut() else { return };
    let msg = grant_supply(row.0, &mut inventory);
    if let Ok(mut text) = status.get_single_mut() {
        text.sections[0].value = msg;
    }
}

/// 干员切换台点击：点击卡片切换当前干员（技能组/冷却/角色饰条色一并更新）
pub(crate) fn operator_station_click_system(
    mouse: Res<ButtonInput<MouseButton>>,
    open: Res<OpenStation>,
    cards: Query<(&OperatorCard, &Interaction)>,
    mut player_query: Query<(&mut OperatorState, &OperatorAccent), With<Player>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut status: Query<&mut Text, (With<OperatorStatusText>, Without<SupplyStatusText>)>,
) {
    if *open != OpenStation::Operator { return; }
    if !mouse.just_pressed(MouseButton::Left) { return; }
    let Some((card, _)) = cards.iter().find(|(_, interaction)| **interaction == Interaction::Pressed) else { return };
    let Ok((mut op, accent)) = player_query.get_single_mut() else { return };
    if card.0 == op.active {
        if let Ok(mut text) = status.get_single_mut() {
            text.sections[0].value = format!("当前已是 {}", roster()[op.active].name);
        }
        return;
    }
    switch_operator(&mut op, accent, &mut materials, card.0);
    if let Ok(mut text) = status.get_single_mut() {
        text.sections[0].value = format!("已切换至 {}", roster()[card.0].name);
    }
}

/// 补给台面板：悬停行高亮
#[allow(clippy::type_complexity)]
pub(crate) fn supply_ui_update_system(
    mut rows: Query<(&SupplyRow, &Interaction, &mut BorderColor, &mut BackgroundColor), Without<OperatorCard>>,
) {
    for (_row, interaction, mut border, mut bg) in rows.iter_mut() {
        if *interaction == Interaction::Hovered {
            border.0 = Color::srgba(1.0, 0.8, 0.25, 0.95);
            bg.0 = Color::srgba(0.26, 0.27, 0.32, 0.95);
        } else {
            border.0 = Color::srgba(0.35, 0.35, 0.4, 0.6);
            bg.0 = Color::srgba(0.10, 0.10, 0.13, 0.92);
        }
    }
}

/// 干员切换台面板：卡片高亮（当前干员金色 + 元素底色，悬停白框）与文本刷新
#[allow(clippy::type_complexity)]
pub(crate) fn operator_ui_update_system(
    mut cards: Query<(&OperatorCard, &Interaction, &mut BorderColor, &mut BackgroundColor), Without<SupplyRow>>,
    mut texts: Query<(&OperatorCardText, &mut Text)>,
    player_query: Query<&OperatorState, With<Player>>,
) {
    let active = player_query.get_single().map(|op| op.active).unwrap_or(usize::MAX);
    for (card, interaction, mut border, mut bg) in cards.iter_mut() {
        let selected = card.0 == active;
        let hovered = *interaction == Interaction::Hovered;
        border.0 = if hovered {
            Color::srgba(0.95, 0.95, 0.95, 0.95)
        } else if selected {
            Color::srgba(1.0, 0.8, 0.25, 0.95)
        } else {
            Color::srgba(0.35, 0.35, 0.4, 0.6)
        };
        bg.0 = if selected {
            roster().get(card.0).map(|op| op.element.color().with_alpha(0.28)).unwrap_or(Color::srgba(0.10, 0.10, 0.13, 0.92))
        } else if hovered {
            Color::srgba(0.3, 0.3, 0.35, 0.92)
        } else {
            Color::srgba(0.10, 0.10, 0.13, 0.92)
        };
    }
    for (ct, mut text) in texts.iter_mut() {
        let Some(op) = roster().get(ct.0) else { continue };
        let mark = if ct.0 == active { "   ✓ 当前" } else { "" };
        text.sections[0].value = format!("{} · {}{}", op.name, op.title, mark);
        text.sections[0].style.color = op.element.color();
        text.sections[1].value = format!(
            "\nQ {}（{}s 冷却）：{}",
            op.q.name, op.q.cooldown_secs, op.q.desc
        );
        text.sections[1].style.color = Color::srgb(0.75, 0.75, 0.78);
        text.sections[2].value = format!(
            "\nE {}（{}s 冷却）：{}",
            op.e.name, op.e.cooldown_secs, op.e.desc
        );
        text.sections[2].style.color = Color::srgb(0.75, 0.75, 0.78);
        text.sections[3].value = format!("\n{}", op.passive);
        text.sections[3].style.color = Color::srgb(0.5, 0.5, 0.55);
    }
}

// =============================================================================
// Kill Feed (top-right)
// =============================================================================


