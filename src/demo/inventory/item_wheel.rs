//! 物品轮盘：3/4 键短按快速使用 / 长按呼出径向轮盘，光标指向选卡松开确认，
//! 光标停在中心或点击中心"取消"键则撤销使用（不消耗道具）。

use bevy::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use crate::model::Player;
use crate::demo::components::*;
use super::held_grenade::{use_item_at, HeldGrenade};

/// 长按 3/4 呼出的道具轮盘 UI（隐藏，由 item_wheel_system 控制显隐与布局）
pub(crate) fn setup_item_wheel(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
        Visibility::Hidden,
        WheelRoot,
    )).with_children(|root| {
        // 中心区：提示条 + 取消按钮（点击撤销使用）
        root.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(10.0),
                ..default()
            },
        )).with_children(|hub| {
            // 选中信息条
            hub.spawn((
                Node {
                    width: Val::Px(300.0),
                    height: Val::Px(46.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.06, 0.06, 0.09, 0.92)),
                BorderColor::all(Color::srgba(0.45, 0.45, 0.5, 0.8)),
            )).with_children(|bar| {
                bar.spawn((
                    Text::new(""),
                    TextFont { font_size: FontSize::Px(15.0), ..default() },
                    TextColor(Color::srgb(0.92, 0.92, 0.92)),
                    WheelHubText,
                ));
            });
            // 取消按钮：点击撤销使用，光标悬停时清空选卡
            hub.spawn((
                Node {
                    width: Val::Px(120.0),
                    height: Val::Px(34.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.12, 0.07, 0.07, 0.92)),
                BorderColor::all(Color::srgba(0.85, 0.35, 0.3, 0.8)),
                Interaction::default(),
                WheelCancelButton,
            )).with_children(|btn| {
                btn.spawn((
                    Text::new("✕ 取消使用"),
                    TextFont { font_size: FontSize::Px(14.0), ..default() },
                    TextColor(Color::srgb(0.95, 0.6, 0.55)),
                ));
            });
        });
        // 轮盘卡片：绝对定位，最多展示 8 格（与背包槽位上限一致）
        for i in 0..8 {
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(WHEEL_CARD_W),
                    height: Val::Px(WHEEL_CARD_H),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.10, 0.10, 0.13, 0.92)),
                BorderColor::all(Color::srgba(0.35, 0.35, 0.4, 0.6)),
                WheelCard(i),
            )).with_children(|card| {
                card.spawn((
                    Text::new(""),
                    TextFont { font_size: FontSize::Px(14.0), ..default() },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
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
/// 轮盘系统的输入上下文：输入事件、时间、光标/轮盘/持雷状态与中心"取消"键交互。
/// 纯资源 + 只读查询走 SystemParam，可变查询在系统参数中直连。
#[derive(SystemParam)]
pub(crate) struct WheelContext<'w, 's> {
    keyboard: Res<'w, ButtonInput<KeyCode>>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
    time: Res<'w, Time>,
    input_state: ResMut<'w, InputState>,
    wheel: ResMut<'w, WheelState>,
    held: ResMut<'w, HeldGrenade>,
    cancel_btn: Query<'w, 's, &'static Interaction, With<WheelCancelButton>>,
}

pub(crate) fn item_wheel_system(
    mut r: WheelContext,
    mut window_query: Query<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
    mut player_query: Query<(&mut Inventory, &mut Health, &mut Armor), With<Player>>,
    mut root_vis: Query<&mut Visibility, (With<WheelRoot>, Without<WheelCard>)>,
    mut hub_text: Query<&mut Text, (With<WheelHubText>, Without<WheelCardText>)>,
    mut cards: Query<(&WheelCard, &mut Node, &mut BorderColor, &mut Visibility, &mut BackgroundColor), Without<WheelRoot>>,
    mut card_texts: Query<(&WheelCardText, &mut Text), Without<WheelHubText>>,
) {
    // 0) 收起判定最先执行：松开 3/4 / Esc / 失焦 / 超时 → 本帧立刻收起并使用选中道具。
    //    必须放在显隐刷新与卡片布局之前——低端机上单帧渲染可达数秒，若先布局后判定，
    //    松手那一帧会带着整圈卡片多渲染数秒，看起来就是“松手后轮盘滞留”。
    if r.wheel.open {
        r.wheel.open_secs += r.time.delta_secs();
        let wheel_key = if r.wheel.category == ItemCategory::Consumable { KeyCode::Digit3 } else { KeyCode::Digit4 };
        let window_focused = window_query.single().map(|(w, _)| w.focused).unwrap_or(true);
        let force_cancel = r.keyboard.just_pressed(KeyCode::Escape)
            || !window_focused
            || r.wheel.open_secs >= WHEEL_MAX_OPEN_SECS;
        if force_cancel || !r.keyboard.pressed(wheel_key) {
            let confirmed = !force_cancel && r.keyboard.just_released(wheel_key);
            if confirmed {
                // 中心撤销：无选中（光标停在中心/取消键上）→ 不消耗道具直接收起
                if let Some(sel) = r.wheel.selected {
                    if let Some(&inv_idx) = r.wheel.filtered.get(sel) {
                        if let Ok((mut inventory, mut health, mut armor)) = player_query.single_mut() {
                            use_item_at(inv_idx, &mut inventory.items, &mut health, &mut armor, &mut r.held);
                        }
                    }
                }
            }
            r.wheel.open = false;
            r.wheel.selected = None;
            if let Ok((_, mut cursor)) = window_query.single_mut() {
                cursor.visible = false;
                cursor.grab_mode = CursorGrabMode::Locked;
            }
            r.input_state.cursor_locked = true;
        }
    }

    // 1) 根节点与卡片显隐兜底：按【收起判定之后】的状态刷新，任何提前 return
    //    （快速使用/判定中）都不会把轮盘留在屏幕上；卡片显式隐藏，不依赖父继承
    let Ok(mut root_vis) = root_vis.single_mut() else { return };
    *root_vis = if r.wheel.open { Visibility::Visible } else { Visibility::Hidden };
    if !r.wheel.open {
        for (_, _, _, mut visibility, _) in cards.iter_mut() {
            *visibility = Visibility::Hidden;
        }
    }

    // 1) 开始按住 3/4（仅游戏进行中、无 UI 占用时）
    if !r.wheel.open && r.wheel.pending_key.is_none() && r.input_state.cursor_locked {
        if r.keyboard.just_pressed(KeyCode::Digit3) {
            r.wheel.pending_key = Some(KeyCode::Digit3);
            r.wheel.pending_hold = 0.0;
        } else if r.keyboard.just_pressed(KeyCode::Digit4) {
            r.wheel.pending_key = Some(KeyCode::Digit4);
            r.wheel.pending_hold = 0.0;
        }
    }

    // 2) 待判定：短按快速使用 / 长按呼出轮盘
    if let Some(key) = r.wheel.pending_key {
        // 按住期间其他 UI（站点/背包）抢走了光标 → 放弃本次判定：
        // 既不快速使用也不呼出轮盘，避免轮盘叠在面板上、松键后滞留
        if !r.input_state.cursor_locked {
            r.wheel.pending_key = None;
            return;
        }
        r.wheel.pending_hold += r.time.delta_secs();
        if r.keyboard.just_released(key) {
            r.wheel.pending_key = None;
            let category = if key == KeyCode::Digit3 { ItemCategory::Consumable } else { ItemCategory::Tactical };
            if let Ok((mut inventory, mut health, mut armor)) = player_query.single_mut() {
                if let Some(idx) = inventory.items.iter().position(|it| it.item_type.category() == category) {
                    use_item_at(idx, &mut inventory.items, &mut health, &mut armor, &mut r.held);
                }
            }
            return;
        } else if r.wheel.pending_hold >= WHEEL_OPEN_DELAY {
            r.wheel.pending_key = None;
            let category = if key == KeyCode::Digit3 { ItemCategory::Consumable } else { ItemCategory::Tactical };
            // 该类别没有道具时不呼出轮盘（快速轻点同样是无操作）
            let has_items = player_query.single()
                .map(|(inv, _, _)| inv.items.iter().any(|it| it.item_type.category() == category))
                .unwrap_or(false);
            if has_items {
                r.wheel.open = true;
                r.wheel.category = category;
                r.wheel.selected = None;
                r.wheel.open_secs = 0.0;
                if let Ok((_, mut cursor)) = window_query.single_mut() {
                    cursor.visible = true;
                    cursor.grab_mode = CursorGrabMode::None;
                }
                r.input_state.cursor_locked = false;
            }
        } else {
            return; // 仍在判定中，本轮不动轮盘 UI
        }
    }

    // 轮盘未打开时到此为止（显隐已在顶部按状态刷新）。
    // 必须有此守卫：否则下方“打开态”逻辑会在正常游戏时每帧执行，
    // 把轮盘点亮又立刻取消，表现为“没长按 3/4 也常驻屏幕”。
    if !r.wheel.open { return; }

    // 3) 轮盘打开：收集同类道具、光标选卡（收起判定已前移到系统开头）
    let Ok((inventory, ..)) = player_query.single_mut() else { return };
    let filtered: Vec<usize> = inventory.items.iter().enumerate()
        .filter(|(_, it)| it.item_type.category() == r.wheel.category)
        .map(|(i, _)| i)
        .collect();
    r.wheel.filtered = filtered;
    if r.wheel.filtered.is_empty() {
        r.wheel.open = false;
        *root_vis = Visibility::Hidden;
        for (_, _, _, mut visibility, _) in cards.iter_mut() {
            *visibility = Visibility::Hidden;
        }
        if let Ok((_, mut cursor)) = window_query.single_mut() {
            cursor.visible = false;
            cursor.grab_mode = CursorGrabMode::Locked;
        }
        r.input_state.cursor_locked = true;
        return;
    }
    *root_vis = Visibility::Visible;

    let Ok((window, mut cursor)) = window_query.single_mut() else { return };
    let center = Vec2::new(window.width() * 0.5, window.height() * 0.5);

    // 点击轮盘中心"取消"键：撤销使用（不消耗道具），收起轮盘并锁定光标
    let hover_cancel = r.cancel_btn.iter().any(|i| *i != Interaction::None);
    if hover_cancel && r.mouse.just_pressed(MouseButton::Left) {
        r.wheel.open = false;
        r.wheel.selected = None;
        cursor.visible = false;
        cursor.grab_mode = CursorGrabMode::Locked;
        r.input_state.cursor_locked = true;
        return;
    }

    // 光标选卡：移出中心死区才选中；回到中心或悬停取消键即清空选择（此时松开 3/4 = 撤销）
    if let Some(cursor_pos) = window.cursor_position() {
        let offset = cursor_pos - center;
        if offset.length() > 40.0 && !hover_cancel {
            let angle = offset.y.atan2(offset.x);
            let n = r.wheel.filtered.len();
            let sector = std::f32::consts::TAU / n as f32;
            let mut best = 0usize;
            let mut best_dist = f32::MAX;
            for i in 0..n {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 * sector;
                let mut d = (a - angle).rem_euclid(std::f32::consts::TAU);
                if d > std::f32::consts::PI { d = std::f32::consts::TAU - d; }
                if d < best_dist { best_dist = d; best = i; }
            }
            r.wheel.selected = Some(best);
        } else {
            // 中心区 = 撤销位：不选中任何卡片
            r.wheel.selected = None;
        }
    }

    // 卡片环形布局 + 选中高亮 + 显隐 + 文本；超出道具数量的卡片隐藏
    let n = r.wheel.filtered.len();
    let sector = std::f32::consts::TAU / n as f32;
    for (card, mut node, mut border, mut visibility, mut bg) in cards.iter_mut() {
        let i = card.0;
        if i >= n {
            *visibility = Visibility::Hidden;
            continue;
        }
        *visibility = Visibility::Visible;
        let a = -std::f32::consts::FRAC_PI_2 + i as f32 * sector;
        let cx = center.x + a.cos() * WHEEL_RADIUS - WHEEL_CARD_W * 0.5;
        let cy = center.y + a.sin() * WHEEL_RADIUS - WHEEL_CARD_H * 0.5;
        node.position_type = PositionType::Absolute;
        node.left = Val::Px(cx);
        node.top = Val::Px(cy);
        let selected = Some(i) == r.wheel.selected;
        border.set_all(if selected {
            Color::srgba(1.0, 0.8, 0.25, 0.95)
        } else {
            Color::srgba(0.35, 0.35, 0.4, 0.6)
        });
        bg.0 = if selected {
            Color::srgba(0.30, 0.28, 0.14, 0.95)
        } else {
            Color::srgba(0.10, 0.10, 0.13, 0.92)
        };
    }
    for (card_text, mut text) in card_texts.iter_mut() {
        if let Some(&inv_idx) = r.wheel.filtered.get(card_text.0) {
            if let Some(item) = inventory.items.get(inv_idx) {
                text.0 = item.name.clone();
                continue;
            }
        }
        text.0 = "".to_string();
    }
    if let Ok(mut hub) = hub_text.single_mut() {
        let sel_name = r.wheel.selected
            .and_then(|s| r.wheel.filtered.get(s))
            .and_then(|&idx| inventory.items.get(idx))
            .map(|item| item.name.as_str())
            .unwrap_or("移出中心选择道具");
        let cat_label = match r.wheel.category {
            ItemCategory::Consumable => "恢复",
            ItemCategory::Tactical => "战术",
        };
        hub.0 = format!("[{}] {} — 松开使用", cat_label, sel_name);
    }
}