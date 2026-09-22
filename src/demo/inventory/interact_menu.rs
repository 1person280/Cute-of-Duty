//! 统一交互菜单拾取：可交互条目检测（站点优先 + 附近拾取物）、滚轮选择、
//! 显隐/滚动窗口刷新、F 确认拾取执行。UI 节点在 backpack::setup_inventory_hud 中创建。

use bevy::prelude::*;
use bevy::ecs::system::SystemParam;
use bevy::input::mouse::MouseWheel;
use crate::map::StationKind;
use crate::operator::rifle_profile;
use crate::model::Player;
use crate::demo::components::*;
use crate::demo::frontend::ElementVisual;
use super::held_grenade::HeldGrenade;

pub(crate) fn interact_detection_system(
    player_query: Query<&Transform, With<Player>>,
    pickup_query: Query<(Entity, &Transform, &PickupItem)>,
    station_query: Query<(Entity, &Transform, &Station)>,
    mut nearby: ResMut<NearbyInteract>,
) {
    let Ok(player_transform) = player_query.get_single() else { return };
    let player_pos = player_transform.translation;

    // 站点条目在最前：靠近桌子时优先开台，不会被地上的散落物抢走 F
    let mut entries: Vec<InteractEntry> = Vec::new();
    let mut best = STATION_USE_RANGE;
    let mut near_station: Option<(Entity, &StationKind, &'static str)> = None;
    for (entity, transform, station) in station_query.iter() {
        let dist = transform.translation.distance(player_pos);
        if dist < best {
            best = dist;
            near_station = Some((entity, &station.kind, station.label));
        }
    }
    if let Some((entity, kind, label)) = near_station {
        entries.push(InteractEntry::Station { kind: *kind, label, entity });
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

/// 交互菜单的输入上下文：条目/资源状态 + 只读查询打包成 SystemParam，
/// 可变 UI 查询在系统参数中直连以规避 SystemParam 的可变查询生命周期限制。
#[derive(SystemParam)]
pub(crate) struct InteractMenuContext<'w, 's> {
    nearby: Res<'w, NearbyInteract>,
    pickup_query: Query<'w, 's, &'static PickupItem>,
    player_query: Query<'w, 's, (&'static Inventory, &'static WeaponSlot), With<Player>>,
    input_state: Res<'w, InputState>,
    wheel: Res<'w, WheelState>,
    held: Res<'w, HeldGrenade>,
}

/// 交互菜单刷新所需的若干可变 UI 查询别名：以互斥 With/Without 标记隔离，
/// 收窄长 Query 元组，规避 clippy::type_complexity。
type InteractHeaderVisQuery<'w, 's> = Query<
    'w, 's,
    &'static mut Visibility,
    (
        Or<(With<InteractMenuPanel>, With<InteractMenuHeader>)>,
        Without<InteractMenuUI>,
        Without<InteractRow>,
    ),
>;
type InteractRowsQuery<'w, 's> = Query<
    'w, 's,
    (
        &'static InteractRow,
        &'static mut Visibility,
        &'static mut BackgroundColor,
        &'static mut BorderColor,
    ),
    (
        Without<InteractRowText>,
        Without<InteractMenuUI>,
        Without<InteractMenuPanel>,
        Without<InteractMenuHeader>,
    ),
>;
type InteractScrollbarQuery<'w, 's> = Query<
    'w, 's,
    (
        &'static InteractScrollBar,
        &'static mut Style,
        &'static mut Visibility,
    ),
    (
        Without<InteractMenuUI>,
        Without<InteractMenuPanel>,
        Without<InteractMenuHeader>,
        Without<InteractRow>,
    ),
>;

/// 统一交互菜单刷新：显隐、滚动窗口逐行文本与选中高亮、滚动条位置、底部警告提示。
/// 面板高度固定（行槽常驻占位）：条目超出可见行数时折叠在窗口外，
/// 行槽 j 显示 entries[scroll_start + j]，滚动条滑块同步窗口位置。
pub(crate) fn interact_menu_update(
    r: InteractMenuContext,
    mut root_vis: Query<&mut Visibility, With<InteractMenuUI>>,
    // bevy 0.14 UI 不会因祖先 Hidden 剔除子节点：收起时面板/标题/行/滚动条/文本都要各自隐藏
    mut panel_header_vis: InteractHeaderVisQuery,
    mut rows: InteractRowsQuery,
    mut texts: Query<(&InteractRowText, &mut Text), Without<InteractMenuHintText>>,
    mut hint: Query<&mut Text, (With<InteractMenuHintText>, Without<InteractRowText>)>,
    mut scrollbar: InteractScrollbarQuery,
) {
    // 背包/站点/轮盘等任一 UI 占用（光标解锁）或持雷瞄准时收起菜单
    let show = !r.nearby.entries.is_empty()
        && r.input_state.cursor_locked
        && !r.wheel.open
        && r.wheel.pending_key.is_none()
        && r.held.item.is_none();
    if let Ok(mut vis) = root_vis.get_single_mut() {
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
    }
    for mut vis in &mut panel_header_vis {
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
    }

    // 滚动窗口：行槽 j 显示 entries[scroll_start + j]；scroll_start 由滚轮/检测系统维护，
    // 这里做显示层钳制，避免条目缩水后窗口越界一帧
    let total = r.nearby.entries.len();
    let max_start = total.saturating_sub(INTERACT_MENU_VISIBLE_ROWS);
    let start = r.nearby.scroll_start.min(max_start);

    for (row, mut vis, mut bg, mut border) in rows.iter_mut() {
        let entry_idx = start + row.0;
        if !show || r.nearby.entries.get(entry_idx).is_none() {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Visible;
        let selected = entry_idx == r.nearby.selected;
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
        let Some(entry) = r.nearby.entries.get(entry_idx) else {
            text.sections[0].value = String::new();
            continue;
        };
        match *entry {
            InteractEntry::Station { kind, label, .. } => {
                text.sections[0].value = label.to_string();
                text.sections[0].style.color = station_accent(kind);
            }
            InteractEntry::Pickup(entity) => {
                if let Ok(item) = r.pickup_query.get(entity) {
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
            if let Some(InteractEntry::Pickup(entity)) = r.nearby.entries.get(r.nearby.selected) {
                if let Ok(item) = r.pickup_query.get(*entity) {
                    if let Ok((inventory, weapon_slot)) = r.player_query.get_single() {
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

/// 站点条目配色（与小地图一致：补给台琥珀 / 干员切换台青 / 物资箱橙）
pub(crate) fn station_accent(kind: StationKind) -> Color {
    match kind {
        StationKind::SupplyTable => Color::srgb(1.0, 0.65, 0.15),
        StationKind::OperatorDesk => Color::srgb(0.2, 0.9, 0.95),
        StationKind::SupplyCrate => Color::srgb(1.0, 0.55, 0.12),
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

/// 拾取执行的输入资源：键盘 + 交互条目 + 光标态 + 轮盘/持雷占用。
/// 用 SystemParam 收拢资源，规避 too_many_arguments；玩家/拾取查询因含可变项直连。
#[derive(SystemParam)]
pub(crate) struct InteractExecuteContext<'w> {
    keyboard: ResMut<'w, ButtonInput<KeyCode>>,
    nearby: ResMut<'w, NearbyInteract>,
    input_state: Res<'w, InputState>,
    wheel: Res<'w, WheelState>,
    held: Res<'w, HeldGrenade>,
}

pub(crate) fn interact_execute_system(
    mut commands: Commands,
    mut ctx: InteractExecuteContext,
    mut player_query: Query<(&mut Inventory, &WeaponSlot), With<Player>>,
    pickup_query: Query<&PickupItem>,
) {
    // UI 打开（光标解锁）/轮盘占用/持雷瞄准时不拾取
    if !ctx.input_state.cursor_locked || ctx.wheel.open || ctx.wheel.pending_key.is_some() || ctx.held.item.is_some() { return; }
    if !ctx.keyboard.just_pressed(KeyCode::KeyF) || ctx.nearby.entries.is_empty() { return; }
    // 只处理拾取物条目；站点条目由 station_system 开面板
    let Some(InteractEntry::Pickup(selected_entity)) = ctx.nearby.entries.get(ctx.nearby.selected).copied() else { return };
    // 本次 F 已被拾取消费：清掉按下态，避免同帧 station_system 再把面板打开
    ctx.keyboard.clear_just_pressed(KeyCode::KeyF);
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
            ctx.nearby.entries.retain(|e| !matches!(e, InteractEntry::Pickup(e2) if *e2 == selected_entity));
            if ctx.nearby.selected >= ctx.nearby.entries.len() && !ctx.nearby.entries.is_empty() {
                ctx.nearby.selected = ctx.nearby.entries.len() - 1;
            }
            // 窗口钳制：条目缩水后不越界，且保证选中项仍留在可见窗口内
            ctx.nearby.scroll_start = ctx.nearby.scroll_start
                .min(ctx.nearby.entries.len().saturating_sub(INTERACT_MENU_VISIBLE_ROWS))
                .min(ctx.nearby.selected);
        }
    }
}