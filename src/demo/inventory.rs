//! 背包与交互菜单：Tab 背包 UI、F 交互菜单、物品轮盘、手雷投掷

use bevy::prelude::*;
use bevy::input::mouse::MouseWheel;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use crate::map::StationKind;
use crate::operator::rifle_profile;
use crate::model::{
    Player, PlayerCamera,
};
use super::hud::{EffectAssets, EffectMatKind};
use super::combat::{ITEM_GRENADE_DAMAGE, ITEM_GRENADE_RADIUS};
use super::common::*;
use super::components::*;

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

pub(crate) fn inventory_toggle(
    mut keyboard: ResMut<ButtonInput<KeyCode>>,
    mut inventory_ui: Query<&mut Visibility, With<InventoryUI>>,
    mut input_state: ResMut<InputState>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    wheel: Res<WheelState>,
    open_station: Res<OpenStation>,
) {
    // 轮盘打开/按住期间不响应 Tab/Esc，避免两层 UI 叠加
    if wheel.open || wheel.pending_key.is_some() { return; }
    // 站点面板打开时不响应（站点面板自带开关）
    if *open_station != OpenStation::None { return; }
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

pub(crate) fn interact_detection_system(
    player_query: Query<&Transform, With<Player>>,
    pickup_query: Query<(Entity, &Transform, &PickupItem)>,
    station_query: Query<(&Transform, &Station)>,
    mut nearby: ResMut<NearbyInteract>,
) {
    let Ok(player_transform) = player_query.get_single() else { return };
    let player_pos = player_transform.translation;

    // 站点条目在最前：靠近桌子时优先开台，不会被地上的散落物抢走 F
    let mut entries: Vec<InteractEntry> = Vec::new();
    let mut best = STATION_USE_RANGE;
    let mut near_station: Option<(&StationKind, &'static str)> = None;
    for (transform, station) in station_query.iter() {
        let dist = transform.translation.distance(player_pos);
        if dist < best {
            best = dist;
            near_station = Some((&station.kind, station.label));
        }
    }
    if let Some((kind, label)) = near_station {
        entries.push(InteractEntry::Station { kind: *kind, label });
    }

    // 拾取物按距离升序排在站点之后
    let mut items: Vec<(Entity, f32)> = Vec::new();
    for (entity, transform, _item) in pickup_query.iter() {
        let dist = (transform.translation - player_pos).length();
        if dist < 3.5 {
            items.push((entity, dist));
        }
    }
    items.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    entries.extend(items.into_iter().map(|(e, _)| InteractEntry::Pickup(e)));
    entries.truncate(INTERACT_MENU_MAX_ENTRIES);

    nearby.entries = entries;
    if nearby.selected >= nearby.entries.len() && !nearby.entries.is_empty() {
        nearby.selected = 0;
        // 选中项跳回开头时滚动窗口一并归位，保证选中条目可见
        nearby.scroll_start = 0;
    }
    // 条目缩水后钳制窗口起点：超出固定高度的条目折叠在窗口外
    nearby.scroll_start = nearby.scroll_start.min(nearby.entries.len().saturating_sub(INTERACT_MENU_VISIBLE_ROWS));
}

pub(crate) fn interact_scroll_system(
    mut scroll_events: EventReader<MouseWheel>,
    mut nearby: ResMut<NearbyInteract>,
    input_state: Res<InputState>,
    wheel: Res<WheelState>,
) {
    // 交互菜单未显示（光标解锁/轮盘占用）时滚轮不改变选中项
    if !input_state.cursor_locked || wheel.open || wheel.pending_key.is_some() { return; }
    // 先钳制窗口起点，防止上一帧条目缩水后窗口越界
    nearby.scroll_start = nearby.scroll_start.min(nearby.entries.len().saturating_sub(INTERACT_MENU_VISIBLE_ROWS));
    for ev in scroll_events.read() {
        if nearby.entries.len() <= 1 { continue; }
        if ev.y > 0.0 {
            nearby.selected = (nearby.selected + nearby.entries.len() - 1) % nearby.entries.len();
        } else if ev.y < 0.0 {
            nearby.selected = (nearby.selected + 1) % nearby.entries.len();
        }
        // 滚动窗口跟随选中项：仅在选中项离开固定可见范围时最小幅度移动
        if nearby.selected < nearby.scroll_start {
            nearby.scroll_start = nearby.selected;
        } else if nearby.selected >= nearby.scroll_start + INTERACT_MENU_VISIBLE_ROWS {
            nearby.scroll_start = nearby.selected + 1 - INTERACT_MENU_VISIBLE_ROWS;
        }
    }
}

/// 统一交互菜单刷新：显隐、滚动窗口逐行文本与选中高亮、滚动条位置、底部警告提示。
/// 面板高度固定（行槽常驻占位）：条目超出可见行数时折叠在窗口外，
/// 行槽 j 显示 entries[scroll_start + j]，滚动条滑块同步窗口位置。
#[allow(clippy::too_many_arguments)]
pub(crate) fn interact_menu_update(
    nearby: Res<NearbyInteract>,
    pickup_query: Query<&PickupItem>,
    player_query: Query<(&Inventory, &WeaponSlot), With<Player>>,
    input_state: Res<InputState>,
    wheel: Res<WheelState>,
    held: Res<HeldGrenade>,
    mut root_vis: Query<&mut Visibility, With<InteractMenuUI>>,
    // bevy 0.14 UI 不会因祖先 Hidden 剔除子节点：收起时面板/标题/行/滚动条/文本都要各自隐藏
    mut panel_vis: Query<&mut Visibility, (With<InteractMenuPanel>, Without<InteractMenuUI>, Without<InteractRow>, Without<InteractMenuHeader>)>,
    mut header_vis: Query<&mut Visibility, (With<InteractMenuHeader>, Without<InteractMenuUI>, Without<InteractRow>, Without<InteractMenuPanel>)>,
    mut rows: Query<(&InteractRow, &mut Visibility, &mut BackgroundColor, &mut BorderColor), (Without<InteractRowText>, Without<InteractMenuUI>, Without<InteractMenuPanel>, Without<InteractMenuHeader>)>,
    mut texts: Query<(&InteractRowText, &mut Text), Without<InteractMenuHintText>>,
    mut hint: Query<&mut Text, (With<InteractMenuHintText>, Without<InteractRowText>)>,
    mut scrollbar: Query<(&InteractScrollBar, &mut Style, &mut Visibility), (Without<InteractMenuUI>, Without<InteractMenuPanel>, Without<InteractMenuHeader>, Without<InteractRow>)>,
) {
    // 背包/站点/轮盘等任一 UI 占用（光标解锁）或持雷瞄准时收起菜单
    let show = !nearby.entries.is_empty()
        && input_state.cursor_locked
        && !wheel.open
        && wheel.pending_key.is_none()
        && held.item.is_none();
    if let Ok(mut vis) = root_vis.get_single_mut() {
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
    }
    if let Ok(mut vis) = panel_vis.get_single_mut() {
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
    }
    if let Ok(mut vis) = header_vis.get_single_mut() {
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
    }

    // 滚动窗口：行槽 j 显示 entries[scroll_start + j]；scroll_start 由滚轮/检测系统维护，
    // 这里做显示层钳制，避免条目缩水后窗口越界一帧
    let total = nearby.entries.len();
    let max_start = total.saturating_sub(INTERACT_MENU_VISIBLE_ROWS);
    let start = nearby.scroll_start.min(max_start);

    for (row, mut vis, mut bg, mut border) in rows.iter_mut() {
        let entry_idx = start + row.0;
        if !show || nearby.entries.get(entry_idx).is_none() {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Visible;
        let selected = entry_idx == nearby.selected;
        *bg = BackgroundColor(if selected {
            Color::srgba(0.26, 0.27, 0.32, 0.95)
        } else {
            Color::srgba(0.10, 0.10, 0.13, 0.85)
        });
        *border = BorderColor(if selected {
            Color::srgba(1.0, 0.8, 0.25, 0.95)
        } else {
            Color::srgba(0.3, 0.3, 0.35, 0.4)
        });
    }
    for (row, mut text) in texts.iter_mut() {
        let entry_idx = start + row.0;
        if !show {
            text.sections[0].value = String::new();
            continue;
        }
        let Some(entry) = nearby.entries.get(entry_idx) else {
            text.sections[0].value = String::new();
            continue;
        };
        match *entry {
            InteractEntry::Station { kind, label } => {
                text.sections[0].value = label.to_string();
                text.sections[0].style.color = station_accent(kind);
            }
            InteractEntry::Pickup(entity) => {
                if let Ok(item) = pickup_query.get(entity) {
                    text.sections[0].value = item.name.clone();
                    text.sections[0].style.color = pickup_text_color(&item.item_type);
                }
            }
        }
    }
    // 滚动条：条目超出可见行数才显示；滑块高度 = 可见占比，位置对应窗口起点
    let scrolling = show && total > INTERACT_MENU_VISIBLE_ROWS;
    for (part, mut style, mut vis) in scrollbar.iter_mut() {
        *vis = if scrolling { Visibility::Visible } else { Visibility::Hidden };
        if part.0 == ScrollbarPart::Thumb && scrolling {
            let thumb_h = INTERACT_SCROLL_TRACK_H * (INTERACT_MENU_VISIBLE_ROWS as f32 / total as f32);
            let travel = INTERACT_SCROLL_TRACK_H - thumb_h;
            style.height = Val::Px(thumb_h);
            style.top = Val::Px(if max_start > 0 { travel * (start as f32 / max_start as f32) } else { 0.0 });
        }
    }
    // 底部提示：操作说明 + 选中拾取物的满载/替换警告（弹药直接入池永不占槽）
    if let Ok(mut text) = hint.get_single_mut() {
        let mut value = if show { "滚轮选择 · F 确认".to_string() } else { String::new() };
        if show {
            if let Some(InteractEntry::Pickup(entity)) = nearby.entries.get(nearby.selected) {
                if let Ok(item) = pickup_query.get(*entity) {
                    if let Ok((inventory, weapon_slot)) = player_query.get_single() {
                        match item.item_type {
                            PickupType::Weapon { .. } => {
                                if inventory.weapons.len() >= inventory.max_weapons {
                                    let cur = &inventory.weapons[weapon_slot.current];
                                    value = format!("将替换当前武器 {}", cur.name);
                                }
                            }
                            PickupType::Ammo { .. } => {}
                            _ => {
                                if inventory.items.len() >= inventory.max_slots {
                                    value = "背包已满".to_string();
                                }
                            }
                        }
                    }
                }
            }
        }
        text.sections[0].value = value;
    }
}

/// 站点条目配色（与小地图一致：补给台琥珀 / 干员切换台青）
pub(crate) fn station_accent(kind: StationKind) -> Color {
    match kind {
        StationKind::SupplyTable => Color::srgb(1.0, 0.65, 0.15),
        StationKind::OperatorDesk => Color::srgb(0.2, 0.9, 0.95),
    }
}

/// 拾取物条目配色（与场上发光色一致）
pub(crate) fn pickup_text_color(item_type: &PickupType) -> Color {
    match item_type {
        PickupType::Ammo { .. } => Color::srgb(0.9, 0.7, 0.2),
        PickupType::Health { .. } => Color::srgb(0.9, 0.2, 0.2),
        PickupType::Armor { .. } => Color::srgb(0.2, 0.5, 0.9),
        PickupType::Grenade { element } | PickupType::Weapon { element } => element.color(),
    }
}

pub(crate) fn interact_execute_system(
    mut keyboard: ResMut<ButtonInput<KeyCode>>,
    mut nearby: ResMut<NearbyInteract>,
    mut player_query: Query<(&mut Inventory, &WeaponSlot), With<Player>>,
    pickup_query: Query<&PickupItem>,
    mut commands: Commands,
    input_state: Res<InputState>,
    wheel: Res<WheelState>,
    held: Res<HeldGrenade>,
) {
    // UI 打开（光标解锁）/轮盘占用/持雷瞄准时不拾取
    if !input_state.cursor_locked || wheel.open || wheel.pending_key.is_some() || held.item.is_some() { return; }
    if !keyboard.just_pressed(KeyCode::KeyF) || nearby.entries.is_empty() { return; }
    // 只处理拾取物条目；站点条目由 station_system 开面板
    let Some(InteractEntry::Pickup(selected_entity)) = nearby.entries.get(nearby.selected).copied() else { return; };
    // 本次 F 已被拾取消费：清掉按下态，避免同帧 station_system 再把面板打开
    keyboard.clear_just_pressed(KeyCode::KeyF);
    let Ok((mut inventory, weapon_slot)) = player_query.get_single_mut() else { return };

    if let Ok(item) = pickup_query.get(selected_entity) {
        // 弹药直接补充弹药池，武器替换当前手持，其余物资入背包槽
        let mut taken = false;
        match &item.item_type {
            PickupType::Ammo { amount } => {
                inventory.ammo_pool += *amount;
                taken = true;
            }
            PickupType::Weapon { element } => {
                // 双主武器固定：新枪替换当前手持的那一把（旧枪弃置）
                let slot = weapon_slot.current;
                inventory.weapons[slot] = WeaponData::from_profile(&rifle_profile(*element));
                taken = true;
            }
            _ => {
                if inventory.items.len() < inventory.max_slots {
                    inventory.items.push(item.clone());
                    taken = true;
                }
            }
        }

        if taken {
            // Remove pickup entity from world
            commands.entity(selected_entity).despawn_recursive();
            // Remove from nearby list
            nearby.entries.retain(|e| !matches!(e, InteractEntry::Pickup(e2) if *e2 == selected_entity));
            if nearby.selected >= nearby.entries.len() && !nearby.entries.is_empty() {
                nearby.selected = nearby.entries.len() - 1;
            }
            // 窗口钳制：条目缩水后不越界，且保证选中项仍留在可见窗口内
            nearby.scroll_start = nearby.scroll_start
                .min(nearby.entries.len().saturating_sub(INTERACT_MENU_VISIBLE_ROWS))
                .min(nearby.selected);
        }
    }
}

/// 背包 UI 刷新：物资槽文本、武器架文本（槽位标记/弹匣数）、弹药池、悬停高亮
#[allow(clippy::type_complexity)]
pub(crate) fn inventory_ui_update(
    player_query: Query<&Inventory, With<Player>>,
    mut slot_texts: Query<(&InventorySlotText, &mut Text), (Without<BackpackWeaponText>, Without<HudBackpackAmmo>)>,
    mut weapon_texts: Query<(&BackpackWeaponText, &mut Text), (Without<InventorySlotText>, Without<HudBackpackAmmo>)>,
    mut ammo_text: Query<&mut Text, (With<HudBackpackAmmo>, Without<InventorySlotText>, Without<BackpackWeaponText>)>,
    #[allow(clippy::type_complexity)] mut slot_bgs: Query<
        (&InventorySlotUI, &Interaction, &mut BorderColor, &mut BackgroundColor),
        Without<BackpackWeaponSlot>,
    >,
    #[allow(clippy::type_complexity)] mut weapon_bgs: Query<
        (&BackpackWeaponSlot, &Interaction, &mut BorderColor, &mut BackgroundColor),
        Without<InventorySlotUI>,
    >,
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

// =============================================================================
// Inventory Item Usage (hover+R / quick keys / radial wheel)
// =============================================================================

/// 已从背包取出、正在瞄准持握的手雷：进入越肩瞄准姿态，左键投出 / Esc 取消放回。
/// 投掷物必须"先瞄准后释放"，因此手雷不再有任何即时投掷路径。
#[derive(Resource, Default)]
pub(crate) struct HeldGrenade {
    pub(crate) item: Option<PickupItem>,
}

#[derive(Component)]
pub(crate) struct HeldHintRoot;

/// 持握手雷时的屏幕提示文案（固定，只切显隐）
pub(crate) const HELD_HINT_TEXT: &str = "手持手雷 — 左键投掷 · Esc 取消";

/// 使用背包第 index 个道具：恢复类立即生效；手雷取出持握（不立即消耗弹道），
/// 进入越肩瞄准后由 grenade_throw_system 投出，Esc 取消放回背包。
pub(crate) fn use_item_at(
    index: usize,
    items: &mut Vec<PickupItem>,
    health: &mut Health,
    armor: &mut Armor,
    held: &mut HeldGrenade,
) -> Option<String> {
    let item = items.get(index)?.clone();
    match &item.item_type {
        // 弹药拾取时已直接入弹药池，不会作为背包物品出现在这里
        PickupType::Ammo { .. } => {}
        PickupType::Health { amount } => {
            health.current = (health.current + *amount).min(health.max);
        }
        PickupType::Armor { amount } => {
            armor.current = (armor.current + *amount).min(armor.max);
        }
        PickupType::Grenade { .. } => {
            // 投掷物必须先瞄准再释放：取出持握并进入瞄准姿态。
            // 已持握时本次使用作废，道具留在背包（items.remove 不会执行）。
            if held.item.is_some() { return None; }
            held.item = Some(item.clone());
        }
        // 武器通过背包"悬停+R 装备"使用，不走道具消耗
        PickupType::Weapon { .. } => return None,
    }
    items.remove(index);
    Some(item.name)
}

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

/// 手雷投掷：持握状态下左键沿相机视线（含俯仰）投出，Esc 取消放回背包。
/// 投掷方向取自相机真实朝向，与越肩准星对齐；持握期间 aim_system 强制瞄准。
#[allow(clippy::too_many_arguments)]
pub(crate) fn grenade_throw_system(
    mut commands: Commands,
    mut effects: ResMut<EffectAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    input_state: Res<InputState>,
    mut held: ResMut<HeldGrenade>,
    cam_query: Query<&GlobalTransform, With<PlayerCamera>>,
    mut player_query: Query<(&Transform, &mut Inventory), With<Player>>,
    mut hint_vis: Query<&mut Visibility, With<HeldHintRoot>>,
) {
    // 持握提示：只在持握时显示（文案固定，创建时已写好）
    if let Ok(mut vis) = hint_vis.get_single_mut() {
        *vis = if held.item.is_some() { Visibility::Visible } else { Visibility::Hidden };
    }
    if held.item.is_none() { return; }
    // UI 打开（光标解锁）时不投掷也不取消：左键属于界面
    if !input_state.cursor_locked { return; }
    let Ok((player_transform, mut inventory)) = player_query.get_single_mut() else { return };

    // Esc 取消：手雷放回背包（cursor_grab_toggle 在持握期间跳过 Esc，由这里接管）
    if keyboard.just_pressed(KeyCode::Escape) {
        if let Some(item) = held.item.take() {
            inventory.items.push(item);
        }
        return;
    }

    // 左键释放投掷：方向 = 相机视线（含俯仰），与准星一致
    if mouse.just_pressed(MouseButton::Left) {
        let Some(item) = held.item.take() else { return };
        let PickupType::Grenade { element } = item.item_type else {
            // 理论不可达（只有手雷会进入持握）：异常物品放回背包
            inventory.items.push(item);
            return;
        };
        let Ok(cam_tf) = cam_query.get_single() else { return };
        let (_, rotation, _) = cam_tf.to_scale_rotation_translation();
        let dir = (rotation * Vec3::NEG_Z).normalize();
        let origin = player_transform.translation + Vec3::Y * 1.7 + dir * 0.4;
        commands.spawn((
            PbrBundle {
                mesh: effects.projectile.clone(),
                material: effects.material(&mut materials, element, EffectMatKind::Plain),
                transform: Transform::from_translation(origin),
                ..default()
            },
            GrenadeProjectile {
                velocity: dir * 13.0 + Vec3::Y * 3.0,
                element,
                damage: ITEM_GRENADE_DAMAGE,
                radius: ITEM_GRENADE_RADIUS,
                effect: crate::operator::SkillEffect::NONE,
                timer: Timer::from_seconds(1.5, TimerMode::Once),
            },
        ));
    }
}

/// 长按 3/4 呼出的道具轮盘 UI（隐藏，由 item_wheel_system 控制显隐与布局）
pub(crate) fn setup_item_wheel(mut commands: Commands) {
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
        WheelRoot,
    )).with_children(|root| {
        // 中心区：提示条 + 取消按钮（点击撤销使用）
        root.spawn((
            NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(10.0),
                    ..default()
                },
                ..default()
            },
        )).with_children(|hub| {
            // 选中信息条
            hub.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Px(300.0),
                        height: Val::Px(46.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.06, 0.06, 0.09, 0.92)),
                    border_color: BorderColor(Color::srgba(0.45, 0.45, 0.5, 0.8)),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    ..default()
                },
            )).with_children(|bar| {
                bar.spawn((
                    TextBundle {
                        text: Text::from_section(
                            "",
                            TextStyle { font_size: 15.0, color: Color::srgb(0.92, 0.92, 0.92), ..default() }
                        ),
                        ..default()
                    },
                    WheelHubText,
                ));
            });
            // 取消按钮：点击撤销使用，光标悬停时清空选卡
            hub.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Px(120.0),
                        height: Val::Px(34.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.12, 0.07, 0.07, 0.92)),
                    border_color: BorderColor(Color::srgba(0.85, 0.35, 0.3, 0.8)),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    ..default()
                },
                Interaction::default(),
                WheelCancelButton,
            )).with_children(|btn| {
                btn.spawn((
                    TextBundle {
                        text: Text::from_section(
                            "✕ 取消使用",
                            TextStyle { font_size: 14.0, color: Color::srgb(0.95, 0.6, 0.55), ..default() }
                        ),
                        ..default()
                    },
                ));
            });
        });
        // 轮盘卡片：绝对定位，最多展示 8 格（与背包槽位上限一致）
        for i in 0..8 {
            root.spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        width: Val::Px(WHEEL_CARD_W),
                        height: Val::Px(WHEEL_CARD_H),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.10, 0.10, 0.13, 0.92)),
                    border_color: BorderColor(Color::srgba(0.35, 0.35, 0.4, 0.6)),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                },
                WheelCard(i),
            )).with_children(|card| {
                card.spawn((
                    TextBundle {
                        text: Text::from_section(
                            "",
                            TextStyle { font_size: 14.0, color: Color::srgb(0.9, 0.9, 0.9), ..default() }
                        ),
                        ..default()
                    },
                    WheelCardText(i),
                ));
            });
        }
    });
}

/// 3/4 键状态机：
/// 短按 → 快速使用背包中第一个对应类别道具；
/// 长按 WHEEL_OPEN_DELAY 秒 → 呼出轮盘（解锁光标），光标指向选卡，松开按键确认使用；
/// 光标停在中心或点击中心"取消"键 → 撤销使用（不消耗道具）。
#[allow(clippy::too_many_arguments)]
pub(crate) fn item_wheel_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    mut input_state: ResMut<InputState>,
    mut wheel: ResMut<WheelState>,
    mut player_query: Query<(&mut Inventory, &mut Health, &mut Armor), With<Player>>,
    mut held: ResMut<HeldGrenade>,
    mut root_vis: Query<&mut Visibility, (With<WheelRoot>, Without<WheelCard>)>,
    mut hub_text: Query<&mut Text, (With<WheelHubText>, Without<WheelCardText>)>,
    cancel_btn: Query<&Interaction, With<WheelCancelButton>>,
    mut cards: Query<(&WheelCard, &mut Style, &mut BorderColor, &mut Visibility, &mut BackgroundColor), Without<WheelRoot>>,
    mut card_texts: Query<(&WheelCardText, &mut Text), Without<WheelHubText>>,
) {
    // 0) 收起判定最先执行：松开 3/4 / Esc / 失焦 / 超时 → 本帧立刻收起并使用选中道具。
    //    必须放在显隐刷新与卡片布局之前——低端机上单帧渲染可达数秒，若先布局后判定，
    //    松手那一帧会带着整圈卡片多渲染数秒，看起来就是“松手后轮盘滞留”。
    if wheel.open {
        wheel.open_secs += time.delta_seconds();
        let wheel_key = if wheel.category == ItemCategory::Consumable { KeyCode::Digit3 } else { KeyCode::Digit4 };
        let window_focused = window_query.get_single().map(|w| w.focused).unwrap_or(true);
        let force_cancel = keyboard.just_pressed(KeyCode::Escape)
            || !window_focused
            || wheel.open_secs >= WHEEL_MAX_OPEN_SECS;
        if force_cancel || !keyboard.pressed(wheel_key) {
            let confirmed = !force_cancel && keyboard.just_released(wheel_key);
            if confirmed {
                // 中心撤销：无选中（光标停在中心/取消键上）→ 不消耗道具直接收起
                if let Some(sel) = wheel.selected {
                    if let Some(&inv_idx) = wheel.filtered.get(sel) {
                        if let Ok((mut inventory, mut health, mut armor)) = player_query.get_single_mut() {
                            use_item_at(inv_idx, &mut inventory.items, &mut health, &mut armor, &mut held);
                        }
                    }
                }
            }
            wheel.open = false;
            wheel.selected = None;
            if let Ok(mut window) = window_query.get_single_mut() {
                window.cursor.visible = false;
                window.cursor.grab_mode = CursorGrabMode::Locked;
            }
            input_state.cursor_locked = true;
        }
    }

    // 1) 根节点与卡片显隐兜底：按【收起判定之后】的状态刷新，任何提前 return
    //    （快速使用/判定中）都不会把轮盘留在屏幕上；卡片显式隐藏，不依赖父继承
    let Ok(mut root_vis) = root_vis.get_single_mut() else { return };
    *root_vis = if wheel.open { Visibility::Visible } else { Visibility::Hidden };
    if !wheel.open {
        for (_, _, _, mut visibility, _) in cards.iter_mut() {
            *visibility = Visibility::Hidden;
        }
    }

    // 1) 开始按住 3/4（仅游戏进行中、无 UI 占用时）
    if !wheel.open && wheel.pending_key.is_none() && input_state.cursor_locked {
        if keyboard.just_pressed(KeyCode::Digit3) {
            wheel.pending_key = Some(KeyCode::Digit3);
            wheel.pending_hold = 0.0;
        } else if keyboard.just_pressed(KeyCode::Digit4) {
            wheel.pending_key = Some(KeyCode::Digit4);
            wheel.pending_hold = 0.0;
        }
    }

    // 2) 待判定：短按快速使用 / 长按呼出轮盘
    if let Some(key) = wheel.pending_key {
        // 按住期间其他 UI（站点/背包）抢走了光标 → 放弃本次判定：
        // 既不快速使用也不呼出轮盘，避免轮盘叠在面板上、松键后滞留
        if !input_state.cursor_locked {
            wheel.pending_key = None;
            return;
        }
        wheel.pending_hold += time.delta_seconds();
        if keyboard.just_released(key) {
            wheel.pending_key = None;
            let category = if key == KeyCode::Digit3 { ItemCategory::Consumable } else { ItemCategory::Tactical };
            if let Ok((mut inventory, mut health, mut armor)) = player_query.get_single_mut() {
                if let Some(idx) = inventory.items.iter().position(|it| it.item_type.category() == category) {
                    use_item_at(idx, &mut inventory.items, &mut health, &mut armor, &mut held);
                }
            }
            return;
        } else if wheel.pending_hold >= WHEEL_OPEN_DELAY {
            wheel.pending_key = None;
            let category = if key == KeyCode::Digit3 { ItemCategory::Consumable } else { ItemCategory::Tactical };
            // 该类别没有道具时不呼出轮盘（快速轻点同样是无操作）
            let has_items = player_query.get_single()
                .map(|(inv, _, _)| inv.items.iter().any(|it| it.item_type.category() == category))
                .unwrap_or(false);
            if has_items {
                wheel.open = true;
                wheel.category = category;
                wheel.selected = None;
                wheel.open_secs = 0.0;
                if let Ok(mut window) = window_query.get_single_mut() {
                    window.cursor.visible = true;
                    window.cursor.grab_mode = CursorGrabMode::None;
                }
                input_state.cursor_locked = false;
            }
        } else {
            return; // 仍在判定中，本轮不动轮盘 UI
        }
    }

    // 轮盘未打开时到此为止（显隐已在顶部按状态刷新）。
    // 必须有此守卫：否则下方“打开态”逻辑会在正常游戏时每帧执行，
    // 把轮盘点亮又立刻取消，表现为“没长按 3/4 也常驻屏幕”。
    if !wheel.open { return; }

    // 3) 轮盘打开：收集同类道具、光标选卡（收起判定已前移到系统开头）
    let Ok((inventory, ..)) = player_query.get_single_mut() else { return };
    let filtered: Vec<usize> = inventory.items.iter().enumerate()
        .filter(|(_, it)| it.item_type.category() == wheel.category)
        .map(|(i, _)| i)
        .collect();
    wheel.filtered = filtered;
    if wheel.filtered.is_empty() {
        wheel.open = false;
        *root_vis = Visibility::Hidden;
        for (_, _, _, mut visibility, _) in cards.iter_mut() {
            *visibility = Visibility::Hidden;
        }
        if let Ok(mut window) = window_query.get_single_mut() {
            window.cursor.visible = false;
            window.cursor.grab_mode = CursorGrabMode::Locked;
        }
        input_state.cursor_locked = true;
        return;
    }
    *root_vis = Visibility::Visible;

    let Ok(mut window) = window_query.get_single_mut() else { return };
    let center = Vec2::new(window.width() * 0.5, window.height() * 0.5);

    // 点击轮盘中心"取消"键：撤销使用（不消耗道具），收起轮盘并锁定光标
    let hover_cancel = cancel_btn.iter().any(|i| *i != Interaction::None);
    if hover_cancel && mouse.just_pressed(MouseButton::Left) {
        wheel.open = false;
        wheel.selected = None;
        window.cursor.visible = false;
        window.cursor.grab_mode = CursorGrabMode::Locked;
        input_state.cursor_locked = true;
        return;
    }

    // 光标选卡：移出中心死区才选中；回到中心或悬停取消键即清空选择（此时松开 3/4 = 撤销）
    if let Some(cursor) = window.cursor_position() {
        let offset = cursor - center;
        if offset.length() > 40.0 && !hover_cancel {
            let angle = offset.y.atan2(offset.x);
            let n = wheel.filtered.len();
            let sector = std::f32::consts::TAU / n as f32;
            let mut best = 0usize;
            let mut best_dist = f32::MAX;
            for i in 0..n {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 * sector;
                let mut d = (a - angle).rem_euclid(std::f32::consts::TAU);
                if d > std::f32::consts::PI { d = std::f32::consts::TAU - d; }
                if d < best_dist { best_dist = d; best = i; }
            }
            wheel.selected = Some(best);
        } else {
            // 中心区 = 撤销位：不选中任何卡片
            wheel.selected = None;
        }
    }

    // 卡片环形布局 + 选中高亮 + 显隐 + 文本；超出道具数量的卡片隐藏
    let n = wheel.filtered.len();
    let sector = std::f32::consts::TAU / n as f32;
    for (card, mut style, mut border, mut visibility, mut bg) in cards.iter_mut() {
        let i = card.0;
        if i >= n {
            *visibility = Visibility::Hidden;
            continue;
        }
        *visibility = Visibility::Visible;
        let a = -std::f32::consts::FRAC_PI_2 + i as f32 * sector;
        let cx = center.x + a.cos() * WHEEL_RADIUS - WHEEL_CARD_W * 0.5;
        let cy = center.y + a.sin() * WHEEL_RADIUS - WHEEL_CARD_H * 0.5;
        style.position_type = PositionType::Absolute;
        style.left = Val::Px(cx);
        style.top = Val::Px(cy);
        let selected = Some(i) == wheel.selected;
        border.0 = if selected {
            Color::srgba(1.0, 0.8, 0.25, 0.95)
        } else {
            Color::srgba(0.35, 0.35, 0.4, 0.6)
        };
        bg.0 = if selected {
            Color::srgba(0.30, 0.28, 0.14, 0.95)
        } else {
            Color::srgba(0.10, 0.10, 0.13, 0.92)
        };
    }
    for (card_text, mut text) in card_texts.iter_mut() {
        if let Some(&inv_idx) = wheel.filtered.get(card_text.0) {
            if let Some(item) = inventory.items.get(inv_idx) {
                text.sections[0].value = item.name.clone();
                continue;
            }
        }
        text.sections[0].value = "".to_string();
    }
    if let Ok(mut hub) = hub_text.get_single_mut() {
        let sel_name = wheel.selected
            .and_then(|s| wheel.filtered.get(s))
            .and_then(|&idx| inventory.items.get(idx))
            .map(|item| item.name.as_str())
            .unwrap_or("移出中心选择道具");
        let cat_label = match wheel.category {
            ItemCategory::Consumable => "恢复",
            ItemCategory::Tactical => "战术",
        };
        hub.sections[0].value = format!("[{}] {} — 松开使用", cat_label, sel_name);
    }
}

// =============================================================================
// 功能站点：无限物资补给台 / 干员切换台
// =============================================================================


