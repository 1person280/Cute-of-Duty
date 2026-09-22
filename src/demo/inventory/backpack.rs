//! 背包 UI：Tab 背包面板（固定双主武器架、弹药池、物资槽）、开关逻辑、刷新与悬停快速使用。

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use crate::model::Player;
use crate::demo::components::*;
use crate::demo::frontend::ElementVisual;
use super::held_grenade::{use_item_at, HeldGrenade};

pub(crate) fn setup_inventory_hud(mut commands: Commands) {
    // Centered inventory overlay (hidden by default)
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
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            visibility: Visibility::Hidden,
            ..default()
        },
        InventoryUI,
    )).with_children(|overlay| {
        // Panel background
        overlay.spawn(NodeBundle {
            style: Style {
                width: Val::Px(540.0),
                height: Val::Auto,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(8.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.06, 0.06, 0.08, 0.95)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        }).with_children(|panel| {
            // Title row: 背包 + 弹药池
            panel.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    margin: UiRect::bottom(Val::Px(4.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                ..default()
            }).with_children(|title_row| {
                title_row.spawn(TextBundle {
                    text: Text::from_section(
                        "背包",
                        TextStyle { font_size: 20.0, color: Color::srgb(0.9, 0.9, 0.9), ..default() }
                    ),
                    ..default()
                });
                title_row.spawn((
                    TextBundle {
                        text: Text::from_section(
                            "弹药池 150",
                            TextStyle { font_size: 14.0, color: Color::srgb(0.95, 0.78, 0.25), ..default() }
                        ),
                        ..default()
                    },
                    HudBackpackAmmo,
                ));
            });
            // Divider line
            panel.spawn(NodeBundle {
                style: Style { width: Val::Percent(100.0), height: Val::Px(2.0), ..default() },
                background_color: BackgroundColor(Color::srgba(0.4, 0.4, 0.5, 0.3)),
                ..default()
            });
            // 武器架：固定双主武器，金色边框
            panel.spawn(TextBundle {
                text: Text::from_section(
                    "主武器 ×2（场上拾取新武器将替换当前手持）",
                    TextStyle { font_size: 12.0, color: Color::srgb(0.5, 0.5, 0.55), ..default() }
                ),
                ..default()
            });
            panel.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(8.0),
                    margin: UiRect::top(Val::Px(4.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                ..default()
            }).with_children(|weapons| {
                for i in 0..MAX_WEAPONS {
                    weapons.spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Px(246.0),
                                height: Val::Px(64.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::srgba(0.14, 0.14, 0.16, 0.95)),
                            border_color: BorderColor(Color::srgba(0.3, 0.3, 0.35, 0.6)),
                            border_radius: BorderRadius::all(Val::Px(4.0)),
                            ..default()
                        },
                        Interaction::default(),
                        BackpackWeaponSlot(i),
                    )).with_children(|box_| {
                        box_.spawn((
                            TextBundle {
                                text: Text::from_sections([
                                    TextSection::new("空", TextStyle { font_size: 12.0, color: Color::srgb(0.85, 0.85, 0.85), ..default() }),
                                    TextSection::new("", TextStyle { font_size: 11.0, color: Color::srgb(0.8, 0.8, 0.82), ..default() }),
                                ]),
                                ..default()
                            },
                            BackpackWeaponText(i),
                        ));
                    });
                }
            });
            // Divider line
            panel.spawn(NodeBundle {
                style: Style { width: Val::Percent(100.0), height: Val::Px(2.0), margin: UiRect::top(Val::Px(2.0)), ..default() },
                background_color: BackgroundColor(Color::srgba(0.4, 0.4, 0.5, 0.3)),
                ..default()
            });
            // 物资区标题
            panel.spawn(TextBundle {
                text: Text::from_section(
                    "物资（悬停 + R 使用）",
                    TextStyle { font_size: 12.0, color: Color::srgb(0.5, 0.5, 0.55), ..default() }
                ),
                style: Style { margin: UiRect::top(Val::Px(4.0)), ..default() },
                ..default()
            });
            // Grid of slots
            panel.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(10.0),
                    row_gap: Val::Px(10.0),
                    margin: UiRect::top(Val::Px(6.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                ..default()
            }).with_children(|grid| {
                for i in 0..8 {
                    grid.spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Px(86.0),
                                height: Val::Px(86.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::srgba(0.14, 0.14, 0.16, 0.95)),
                            border_color: BorderColor(Color::srgba(0.3, 0.3, 0.35, 0.6)),
                            border_radius: BorderRadius::all(Val::Px(4.0)),
                            ..default()
                        },
                        InventorySlotUI(i),
                        Interaction::default(),
                    )).with_children(|slot| {
                        slot.spawn((
                            TextBundle {
                                text: Text::from_section(
                                    " ",
                                    TextStyle { font_size: 13.0, color: Color::srgb(0.85, 0.85, 0.85), ..default() }
                                ),
                                style: Style { width: Val::Percent(90.0), height: Val::Auto, justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                                ..default()
                            },
                            InventorySlotText(i),
                        ));
                    });
                }
            });
            // Instructions
            panel.spawn(TextBundle {
                text: Text::from_section(
                    "Tab/Esc 关闭 · 悬停物资+R 使用 · 3/4 快捷 / 长按呼出轮盘（点击中心撤销）\n手雷使用后持握瞄准：左键投掷 / Esc 取消 · 右键越肩瞄准 · 拾取武器替换当前手持 · 弹药拾取直接入弹药池",
                    TextStyle { font_size: 12.0, color: Color::srgb(0.5, 0.5, 0.55), ..default() }
                ),
                style: Style { margin: UiRect::top(Val::Px(6.0)), ..default() },
                ..default()
            });
        });
    });

    // 统一交互菜单（默认隐藏）——底部居中：站点条目在前 + 附近拾取物，滚轮选择 / F 确认
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::End,
                padding: UiRect::bottom(Val::Px(160.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            visibility: Visibility::Hidden,
            ..default()
        },
        InteractMenuUI,
    )).with_children(|parent| {
        parent.spawn((
            NodeBundle {
            style: Style {
                width: Val::Px(240.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                row_gap: Val::Px(4.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.78)),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            ..default()
        },
        InteractMenuPanel,
        )).with_children(|panel| {
            // 标题行：F 徽标 + 标题（收起时需随菜单一起显式隐藏）
            panel.spawn((
                NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        margin: UiRect::bottom(Val::Px(2.0)),
                        ..default()
                    },
                    ..default()
                },
                InteractMenuHeader,
            )).with_children(|header| {
                header.spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(22.0),
                        height: Val::Px(22.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.9, 0.85, 0.25, 0.9)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                }).with_children(|badge| {
                    badge.spawn(TextBundle {
                        text: Text::from_section("F", TextStyle { font_size: 13.0, color: Color::srgb(0.1, 0.1, 0.1), ..default() }),
                        ..default()
                    });
                });
                header.spawn(TextBundle {
                    text: Text::from_section("互动菜单", TextStyle { font_size: 15.0, color: Color::srgb(0.9, 0.9, 0.9), ..default() }),
                    ..default()
                });
            });
            // 正文行：左侧固定高度的条目列表 + 右侧滚动条（凹槽常驻占位，面板宽度稳定）
            panel.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(6.0),
                    align_items: AlignItems::FlexStart,
                    ..default()
                },
                ..default()
            }).with_children(|body| {
                // 条目列表：8 个行槽常驻占位（隐藏也保留空间），面板高度不随条目数变化；
                // 超出可见行数的条目折叠在滚动窗口外，由 interact_menu_update 映射显示
                body.spawn(NodeBundle {
                    style: Style {
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(INTERACT_ROW_GAP),
                        ..default()
                    },
                    ..default()
                }).with_children(|list| {
                    // 条目行：每行独立节点、固定行高 + 统一左内边距，保证各行文本严格左对齐；
                    // 选中高亮（背景/边框）由 interact_menu_update 每帧刷新
                    for i in 0..INTERACT_MENU_VISIBLE_ROWS {
                        list.spawn((
                            NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(INTERACT_ROW_H),
                                    align_items: AlignItems::Center,
                                    padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::srgba(0.10, 0.10, 0.13, 0.85)),
                                border_color: BorderColor(Color::srgba(0.3, 0.3, 0.35, 0.4)),
                                border_radius: BorderRadius::all(Val::Px(4.0)),
                                visibility: Visibility::Hidden,
                                ..default()
                            },
                            InteractRow(i),
                        )).with_children(|row| {
                            row.spawn((
                                TextBundle {
                                    text: Text::from_section(
                                        "",
                                        TextStyle { font_size: 14.0, color: Color::srgb(0.9, 0.9, 0.9), ..default() }
                                    ),
                                    ..default()
                                },
                                InteractRowText(i),
                            ));
                        });
                    }
                });
                // 滚动条凹槽：高度与条目列表等高；条目未超出可见行数时整体隐藏（占位不塌）
                body.spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Px(6.0),
                            height: Val::Px(INTERACT_SCROLL_TRACK_H),
                            margin: UiRect::top(Val::Px(1.0)),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.08)),
                        border_radius: BorderRadius::all(Val::Px(3.0)),
                        visibility: Visibility::Hidden,
                        ..default()
                    },
                    InteractScrollBar(ScrollbarPart::Track),
                )).with_children(|track| {
                    // 滑块：绝对定位在凹槽内，top/height 由 interact_menu_update 按窗口位置刷新
                    track.spawn((
                        NodeBundle {
                            style: Style {
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                width: Val::Percent(100.0),
                                top: Val::Px(0.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::srgba(0.6, 0.6, 0.65, 0.7)),
                            border_radius: BorderRadius::all(Val::Px(3.0)),
                            visibility: Visibility::Hidden,
                            ..default()
                        },
                        InteractScrollBar(ScrollbarPart::Thumb),
                    ));
                });
            });
            // 底部提示行：操作说明，选中条目有满载/替换警告时由 interact_menu_update 覆写
            panel.spawn((
                TextBundle {
                    text: Text::from_section(
                        "滚轮选择 · F 确认",
                        TextStyle { font_size: 11.0, color: Color::srgb(0.5, 0.5, 0.55), ..default() }
                    ),
                    ..default()
                },
                InteractMenuHintText,
            ));
        });
    });
}

// =============================================================================
// 统一交互菜单 UI 已由上方 setup_inventory_hud 生成；其显隐、滚动窗口、
// 选中高亮、滚动条位置与底部提示均在 interact_menu::interact_menu_update 每帧刷新。
// =============================================================================

pub(crate) fn inventory_toggle(
    mut keyboard: ResMut<ButtonInput<KeyCode>>,
    mut inventory_ui: Query<&mut Visibility, With<InventoryUI>>,
    mut input_state: ResMut<InputState>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    wheel: Res<WheelState>,
    open_station: Res<OpenStation>,
    crate_win: Res<super::super::supply_crate::CrateWindow>,
) {
    // 轮盘打开/按住期间不响应 Tab/Esc，避免两层 UI 叠加
    if wheel.open || wheel.pending_key.is_some() { return; }
    // 站点面板打开时不响应（站点面板自带开关）
    if *open_station != OpenStation::None { return; }
    // 物资箱窗口打开时不响应（关闭走 station_system）
    if crate_win.crate_entity.is_some() { return; }
    let tab = keyboard.just_pressed(KeyCode::Tab);
    let esc = keyboard.just_pressed(KeyCode::Escape);
    if !tab && !esc { return; }
    let Ok(mut vis) = inventory_ui.get_single_mut() else { return };
    let mut window = window_query.single_mut();
    if *vis == Visibility::Visible {
        // 背包打开时 Tab / Esc 均关闭；消费掉 Esc 按下态，
        // 防止同帧更晚运行的 cursor_grab_toggle 再把光标解锁
        *vis = Visibility::Hidden;
        window.cursor.visible = false;
        window.cursor.grab_mode = CursorGrabMode::Locked;
        input_state.cursor_locked = true;
        keyboard.clear_just_pressed(KeyCode::Escape);
    } else if tab {
        // 仅 Tab 打开；Esc 留给自由光标切换
        *vis = Visibility::Visible;
        window.cursor.visible = true;
        window.cursor.grab_mode = CursorGrabMode::None;
        input_state.cursor_locked = false;
    }
}

// =============================================================================
// 背包内的道具使用 / 刷新
// =============================================================================

/// 背包内鼠标悬停物资槽 + 按 R：直接使用（武器固定双槽，无需背包内装备）
pub(crate) fn inventory_item_use_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    inventory_ui: Query<&Visibility, With<InventoryUI>>,
    item_slots: Query<(&InventorySlotUI, &Interaction)>,
    mut player_query: Query<(&mut Inventory, &mut Health, &mut Armor), With<Player>>,
    mut held: ResMut<HeldGrenade>,
) {
    let Ok(vis) = inventory_ui.get_single() else { return };
    if *vis != Visibility::Visible { return; }
    if !keyboard.just_pressed(KeyCode::KeyR) { return; }

    let Ok((mut inventory, mut health, mut armor)) = player_query.get_single_mut() else { return };

    // 悬停物资：使用
    let hovered_item = item_slots.iter()
        .find(|(_, interaction)| **interaction == Interaction::Hovered)
        .map(|(slot, _)| slot.0);
    let Some(slot_index) = hovered_item else { return };
    if slot_index >= inventory.items.len() { return; }
    use_item_at(slot_index, &mut inventory.items, &mut health, &mut armor, &mut held);
}

/// 物资槽/武器架的样式查询别名：槽位标记 + 交互 + 边框/底色。
/// 用类型别名收窄长 Query 元组，规避 clippy::type_complexity。
type InventorySlotStyle<'w, 's> = Query<
    'w, 's,
    (&'static InventorySlotUI, &'static Interaction, &'static mut BorderColor, &'static mut BackgroundColor),
    Without<BackpackWeaponSlot>,
>;
type BackpackWeaponStyle<'w, 's> = Query<
    'w, 's,
    (&'static BackpackWeaponSlot, &'static Interaction, &'static mut BorderColor, &'static mut BackgroundColor),
    Without<InventorySlotUI>,
>;

/// 背包文本/弹药池文本的可变查询别名：以互斥 With/Without 标记隔离，
/// 收窄长 Query 元组，规避 clippy::type_complexity。
type SlotTextQuery<'w, 's> = Query<
    'w, 's,
    (&'static InventorySlotText, &'static mut Text),
    (Without<BackpackWeaponText>, Without<HudBackpackAmmo>),
>;
type WeaponTextQuery<'w, 's> = Query<
    'w, 's,
    (&'static BackpackWeaponText, &'static mut Text),
    (Without<InventorySlotText>, Without<HudBackpackAmmo>),
>;
type BackpackAmmoTextQuery<'w, 's> = Query<
    'w, 's,
    &'static mut Text,
    (With<HudBackpackAmmo>, Without<InventorySlotText>, Without<BackpackWeaponText>),
>;

/// 背包 UI 刷新：物资槽文本、武器架文本（槽位标记/弹匣数）、弹药池、悬停高亮
pub(crate) fn inventory_ui_update(
    player_query: Query<&Inventory, With<Player>>,
    mut slot_texts: SlotTextQuery,
    mut weapon_texts: WeaponTextQuery,
    mut ammo_text: BackpackAmmoTextQuery,
    mut slot_bgs: InventorySlotStyle,
    mut weapon_bgs: BackpackWeaponStyle,
) {
    let Ok(inventory) = player_query.get_single() else { return };

    // 物资槽
    for (slot, mut text) in slot_texts.iter_mut() {
        if let Some(item) = inventory.items.get(slot.0) {
            text.sections[0].value = item.name.clone();
        } else {
            text.sections[0].value = "".to_string();
        }
    }
    // 武器架：固定双主武器，槽位标记 + 名称 + 弹匣
    for (bw, mut text) in weapon_texts.iter_mut() {
        if let Some(w) = inventory.weapons.get(bw.0) {
            text.sections[0].value = format!("[{}] {}", bw.0 + 1, w.name);
            text.sections[0].style.color = w.element.color();
            text.sections[1].value = format!("\n{}/{} 弹", w.ammo, w.max_ammo);
            text.sections[1].style.color = Color::srgb(0.8, 0.8, 0.82);
        } else {
            text.sections[0].value = "空".to_string();
            text.sections[0].style.color = Color::srgb(0.35, 0.35, 0.35);
            text.sections[1].value = String::new();
        }
    }
    // 弹药池
    if let Ok(mut text) = ammo_text.get_single_mut() {
        text.sections[0].value = format!("弹药池 {}", inventory.ammo_pool);
    }
    // 物资槽悬停高亮：亮色边框 + 提亮底色，提示可按 R 使用
    for (_slot_ui, interaction, mut border, mut bg) in slot_bgs.iter_mut() {
        if *interaction == Interaction::Hovered {
            border.0 = Color::srgba(1.0, 0.8, 0.25, 0.95);
            bg.0 = Color::srgba(0.26, 0.27, 0.32, 0.95);
        } else {
            border.0 = Color::srgba(0.3, 0.3, 0.35, 0.6);
            bg.0 = Color::srgba(0.14, 0.14, 0.16, 0.95);
        }
    }
    // 武器架高亮：双主武器恒在架，金色边框；悬停白色
    for (bw, interaction, mut border, mut bg) in weapon_bgs.iter_mut() {
        let equipped = bw.0 < inventory.weapons.len();
        let hovered = *interaction == Interaction::Hovered;
        border.0 = if hovered {
            Color::srgba(0.95, 0.95, 0.95, 0.95)
        } else if equipped {
            Color::srgba(1.0, 0.8, 0.25, 0.95)
        } else {
            Color::srgba(0.3, 0.3, 0.35, 0.6)
        };
        bg.0 = if hovered {
            Color::srgba(0.26, 0.27, 0.32, 0.95)
        } else if equipped {
            Color::srgba(0.2, 0.18, 0.1, 0.95)
        } else {
            Color::srgba(0.14, 0.14, 0.16, 0.95)
        };
    }
}