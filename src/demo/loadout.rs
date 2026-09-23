//! 仓库（对局外物资池）与携带背包（赛前选装）：主菜单"仓库"面板。
//! 左侧为仓库物资池，右侧为携带背包；两条目互相拖拽（按住左键拖到目标区松开）
//! 或 Shift+左键 快捷移动，即可切换"是否带进对局"。选择存于 `Loadout` 资源，
//! `OnEnter(InGame)` 时由 `apply_loadout` 写入玩家背包。
//!
//! 为什么单列成模块：携带物资是"带装进入对局"环节的数据与 UI 唯一来源，
//! 独立文件避免把选装逻辑塞进主菜单行为层（菜单文件已接近行数上限）。

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::element::ElementType;
use crate::demo::MenuButton;
use crate::demo::components::{Armor, Inventory, PickupItem, PickupType};

/// 携带背包容量上限（与对局内背包 max_slots 一致）
pub(crate) const LOADOUT_CAPACITY: usize = 8;

/// 仓库物资池的一条物资：名称 + 效果 + 主题色
#[derive(Clone)]
pub(crate) struct WarehouseItem {
    pub(crate) name: &'static str,
    pub(crate) item_type: PickupType,
    pub(crate) color: Color,
}

/// 仓库物资池（扩展现有补给池，含弹药/医疗/护甲/四系手雷）
pub(crate) const WAREHOUSE_POOL: [WarehouseItem; 7] = [
    WarehouseItem { name: "医疗包",   item_type: PickupType::Health { amount: 30.0 }, color: Color::srgb(0.9, 0.2, 0.2) },
    WarehouseItem { name: "护甲板",   item_type: PickupType::Armor { amount: 20.0 }, color: Color::srgb(0.2, 0.5, 0.9) },
    WarehouseItem { name: "额外弹药", item_type: PickupType::Ammo { amount: 150 },   color: Color::srgb(0.9, 0.7, 0.2) },
    WarehouseItem { name: "烈焰手雷", item_type: PickupType::Grenade { element: ElementType::Fire }, color: Color::srgb(1.0, 0.5, 0.1) },
    WarehouseItem { name: "冰霜手雷", item_type: PickupType::Grenade { element: ElementType::Ice }, color: Color::srgb(0.5, 0.9, 1.0) },
    WarehouseItem { name: "雷电手雷", item_type: PickupType::Grenade { element: ElementType::Electric }, color: Color::srgb(0.85, 0.75, 0.2) },
    WarehouseItem { name: "毒素手雷", item_type: PickupType::Grenade { element: ElementType::Poison }, color: Color::srgb(0.5, 0.85, 0.4) },
];

/// 本次进入对局要携带的物资清单（默认携医疗/护甲/弹药/火/冰手雷，与旧版一致）
#[derive(Resource)]
pub(crate) struct Loadout {
    pub(crate) carried: Vec<WarehouseItem>,
}

impl Default for Loadout {
    fn default() -> Self {
        let carried = [
            WAREHOUSE_POOL[0].clone(), // 医疗包
            WAREHOUSE_POOL[1].clone(), // 护甲板
            WAREHOUSE_POOL[2].clone(), // 额外弹药
            WAREHOUSE_POOL[3].clone(), // 烈焰手雷
            WAREHOUSE_POOL[4].clone(), // 冰霜手雷
        ].to_vec();
        Self { carried }
    }
}

// ---- 面板标记 ----
#[derive(Component)]
pub(crate) struct LoadoutButton;
#[derive(Component)]
pub(crate) struct LoadoutPanelRoot;
#[derive(Component)]
pub(crate) struct LoadoutBackdrop;
#[derive(Component)]
pub(crate) struct LoadoutCloseButton;
#[derive(Component)]
pub(crate) struct WarehouseRow(pub(crate) usize);
#[derive(Component)]
pub(crate) struct WarehouseRowText(pub(crate) usize);
#[derive(Component)]
pub(crate) struct CarriedSlot(pub(crate) usize);
#[derive(Component)]
pub(crate) struct CarriedSlotText(pub(crate) usize);
/// 携带背包容器（也作为拖拽落点检测）
#[derive(Component)]
pub(crate) struct BackpackZone;
/// 仓库容器（也作为拖拽落点检测）
#[derive(Component)]
pub(crate) struct WarehouseZone;
#[derive(Component)]
pub(crate) struct LoadoutGhost;
#[derive(Component)]
pub(crate) struct LoadoutGhostText;
#[derive(Component)]
pub(crate) struct LoadoutCapacityText;

/// 拖拽来源：被拖条目是仓库侧还是背包侧
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DragSource {
    Warehouse(usize),
    Carried(usize),
}

/// 仓库拖拽状态机：记录当前来源（None = 无拖拽）
#[derive(Resource, Default)]
pub(crate) struct LoadoutDrag {
    pub(crate) source: Option<DragSource>,
}

/// 把 `Loadout` 落地到玩家背包（弹药/护甲按其类型直写池/甲，其余入物资槽）。
pub(crate) fn apply_loadout(inv: &mut Inventory, armor: &mut Armor, load: &Loadout) {
    inv.items.clear();
    inv.ammo_pool = 60;
    armor.current = 0.0;
    for item in &load.carried {
        match item.item_type {
            PickupType::Health { amount } => {
                inv.items.push(PickupItem { name: item.name.to_string(), item_type: PickupType::Health { amount } });
            }
            PickupType::Grenade { element } => {
                inv.items.push(PickupItem { name: item.name.to_string(), item_type: PickupType::Grenade { element } });
            }
            PickupType::Armor { .. } => { armor.current = Armor::default().current; }
            PickupType::Ammo { .. } => { inv.ammo_pool = 150; }
            PickupType::Weapon { .. } => {}
        }
    }
}

/// 在主菜单根节点下构建仓库面板（覆盖层 + 压暗层），返回 (panel, backdrop)。
/// `root` 来自主菜单根 `commands.entity(root).with_children(...)`，闭包参数为 ChildSpawnerCommands。
pub(crate) fn spawn_loadout_panel(root: &mut ChildSpawnerCommands) -> (Entity, Entity) {
    let backdrop = root
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.02, 0.60)),
            Visibility::Hidden,
            GlobalZIndex(15),
            LoadoutBackdrop,
        ))
        .id();

    let panel = root
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(12.0),
                ..default()
            },
            Visibility::Hidden,
            GlobalZIndex(20),
            LoadoutPanelRoot,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("仓 库 · 携带物资"),
                TextFont { font_size: FontSize::Px(34.0), ..default() },
                TextColor(Color::srgb(0.92, 0.95, 1.0)),
            ));
            panel.spawn((
                Text::new("拖拽仓库物资到右侧背包=携带 · 拖回左侧=不带 · Shift+左键 快捷移动"),
                TextFont { font_size: FontSize::Px(13.0), ..default() },
                TextColor(Color::srgb(0.55, 0.62, 0.72)),
            ));
            panel.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(28.0),
                align_items: AlignItems::FlexStart,
                ..default()
            }).with_children(|cols| {
                // ---- 左侧：仓库物资池 ----
                cols.spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(9.0),
                        width: Val::Px(330.0),
                        ..default()
                    },
                    Interaction::default(),
                    WarehouseZone,
                )).with_children(|wh| {
                    wh.spawn((
                        Text::new("仓 库（物资池）"),
                        TextFont { font_size: FontSize::Px(18.0), ..default() },
                        TextColor(Color::srgb(0.92, 0.95, 1.0)),
                    ));
                    for (i, item) in WAREHOUSE_POOL.iter().enumerate() {
                        wh.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(50.0),
                                justify_content: JustifyContent::SpaceBetween,
                                align_items: AlignItems::Center,
                                padding: UiRect::axes(Val::Px(22.0), Val::Px(0.0)),
                                border: UiRect::all(Val::Px(2.0)),
                                border_radius: BorderRadius::all(Val::Px(4.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.10, 0.14, 0.20, 0.95)),
                            BorderColor::all(Color::srgb(0.22, 0.28, 0.36)),
                            Interaction::default(),
                            WarehouseRow(i),
                        )).with_children(|row| {
                            row.spawn((
                                Text::new(item.name),
                                TextFont { font_size: FontSize::Px(19.0), ..default() },
                                TextColor(item.color),
                            ));
                            row.spawn((
                                Text::new(" "),
                                TextFont { font_size: FontSize::Px(12.0), ..default() },
                                TextColor(Color::srgb(0.4, 0.85, 0.4)),
                                WarehouseRowText(i),
                            ));
                        });
                    }
                });
                // ---- 右侧：携带背包 ----
                cols.spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(9.0),
                        width: Val::Px(330.0),
                        ..default()
                    },
                    Interaction::default(),
                    BackpackZone,
                )).with_children(|bp| {
                    bp.spawn((
                        Text::new("携带背包"),
                        TextFont { font_size: FontSize::Px(18.0), ..default() },
                        TextColor(Color::srgb(0.92, 0.95, 1.0)),
                    ));
                    bp.spawn((
                        Text::new("0 / 8"),
                        TextFont { font_size: FontSize::Px(13.0), ..default() },
                        TextColor(Color::srgb(0.55, 0.62, 0.72)),
                        LoadoutCapacityText,
                    ));
                    for i in 0..LOADOUT_CAPACITY {
                        bp.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(44.0),
                                justify_content: JustifyContent::SpaceBetween,
                                align_items: AlignItems::Center,
                                padding: UiRect::axes(Val::Px(22.0), Val::Px(0.0)),
                                border: UiRect::all(Val::Px(2.0)),
                                border_radius: BorderRadius::all(Val::Px(4.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.14, 0.14, 0.16, 0.95)),
                            BorderColor::all(Color::srgba(0.3, 0.3, 0.35, 0.6)),
                            Interaction::default(),
                            CarriedSlot(i),
                        )).with_children(|slot| {
                            slot.spawn((
                                Text::new("空"),
                                TextFont { font_size: FontSize::Px(17.0), ..default() },
                                TextColor(Color::srgb(0.85, 0.85, 0.85)),
                                CarriedSlotText(i),
                            ));
                        });
                    }
                });
            });
            // 拖拽幽灵：面板内绝对定位跟随光标
            panel.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(140.0),
                    height: Val::Px(40.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.06, 0.06, 0.09, 0.95)),
                BorderColor::all(Color::srgb(0.6, 0.6, 0.65)),
                Visibility::Hidden,
                GlobalZIndex(30),
                LoadoutGhost,
            )).with_children(|g| {
                g.spawn((
                    Text::new(""),
                    TextFont { font_size: FontSize::Px(15.0), ..default() },
                    TextColor(Color::WHITE),
                    LoadoutGhostText,
                ));
            });
            panel.spawn(Node { height: Val::Px(8.0), ..default() });
            panel.spawn((
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(46.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.10, 0.14, 0.20, 0.95)),
                BorderColor::all(Color::srgb(0.22, 0.28, 0.36)),
                Interaction::default(),
                MenuButton,
                LoadoutCloseButton,
            )).with_children(|btn| {
                btn.spawn((
                    Text::new("返 回"),
                    TextFont { font_size: FontSize::Px(19.0), ..default() },
                    TextColor(Color::srgb(0.92, 0.95, 1.0)),
                ));
            });
        })
        .id();

    (panel, backdrop)
}

/// 求仓库物资池下标对应的条目是否已在携带清单中
fn carried_index(load: &Loadout, pool_idx: usize) -> Option<usize> {
    load.carried.iter().position(|it| it.name == WAREHOUSE_POOL[pool_idx].name)
}

// 三类文本查询别名：以互斥 With/Without 标记隔离可变 Text/TextColor 访问，规避 clippy::type_complexity
type WhTextQ<'w, 's> = Query<
    'w, 's,
    (&'static WarehouseRowText, &'static mut Text, &'static mut TextColor),
    (Without<CarriedSlotText>, Without<LoadoutCapacityText>, Without<LoadoutGhostText>),
>;
type BpTextQ<'w, 's> = Query<
    'w, 's,
    (&'static CarriedSlotText, &'static mut Text, &'static mut TextColor),
    (Without<WarehouseRowText>, Without<LoadoutCapacityText>, Without<LoadoutGhostText>),
>;
type CapTextQ<'w, 's> = Query<
    'w, 's,
    &'static mut Text,
    (With<LoadoutCapacityText>, Without<WarehouseRowText>, Without<CarriedSlotText>, Without<LoadoutGhostText>),
>;
type GhostTextQ<'w, 's> = Query<
    'w, 's,
    (&'static mut Text, &'static mut TextColor),
    (With<LoadoutGhostText>, Without<LoadoutCapacityText>, Without<WarehouseRowText>, Without<CarriedSlotText>),
>;

/// 刷新仓库行勾选态 / 背包槽文本 / 容量计数。暴露给主菜单行为层与拖拽系统共用。
fn write_loadout_texts(load: &Loadout, wh_texts: &mut WhTextQ, bp_texts: &mut BpTextQ, capacity: &mut CapTextQ) {
    for (row, mut text, mut color) in wh_texts.iter_mut() {
        let on = carried_index(load, row.0).is_some();
        text.0 = if on { "已携带 ✓".to_string() } else { String::new() };
        color.0 = if on { Color::srgb(0.4, 0.85, 0.4) } else { Color::srgb(0.5, 0.5, 0.55) };
    }
    for (slot, mut text, mut color) in bp_texts.iter_mut() {
        match load.carried.get(slot.0) {
            Some(item) => {
                text.0 = item.name.to_string();
                color.0 = item.color;
            }
            None => {
                text.0 = "空".to_string();
                color.0 = Color::srgb(0.85, 0.85, 0.85);
            }
        }
    }
    if let Ok(mut cap) = capacity.single_mut() {
        cap.0 = format!("{} / {}", load.carried.len(), LOADOUT_CAPACITY);
    }
}

/// 仓库拖拽 + Shift+左键快捷移动主系统。仅在面板打开时生效。
/// 拖拽语义：按住左键点在行/槽上开始 → 幽灵跟随光标 → 松手落在对方容器则移动，否则撤销。
#[allow(clippy::type_complexity)]
pub(crate) fn loadout_drag_system(
    mouse: Res<ButtonInput<MouseButton>>,
    shift: Res<ButtonInput<KeyCode>>,
    mut drag: ResMut<LoadoutDrag>,
    mut load: ResMut<Loadout>,
    panel_vis: Query<&Visibility, With<LoadoutPanelRoot>>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    mut ghost: Query<(&mut Visibility, &mut Node), (With<LoadoutGhost>, Without<LoadoutPanelRoot>)>,
    mut ghost_text: GhostTextQ,
    wh_rows: Query<(&WarehouseRow, &Interaction), Without<CarriedSlot>>,
    bp_slots: Query<(&CarriedSlot, &Interaction), Without<WarehouseRow>>,
    zone_hover: Query<
        (&Interaction, Option<&WarehouseZone>, Option<&BackpackZone>),
        (Without<WarehouseRow>, Without<CarriedSlot>),
    >,
    mut wh_texts: WhTextQ,
    mut bp_texts: BpTextQ,
    mut capacity: CapTextQ,
) {
    let Ok(vis) = panel_vis.single() else { return };
    if *vis != Visibility::Visible {
        if drag.source.take().is_some() {
            if let Ok((mut gv, _)) = ghost.single_mut() { *gv = Visibility::Hidden; }
        }
        return;
    }

    // 面板打开期间每帧刷新文本（覆盖打开瞬间、拖拽/快移后的变化，避免跨模块耦合）
    write_loadout_texts(&load, &mut wh_texts, &mut bp_texts, &mut capacity);

    // 松手落点：光标是否停在仓库容器 / 背包容器上
    let mut over_warehouse = false;
    let mut over_backpack = false;
    for (inter, whz, bpz) in zone_hover.iter() {
        if *inter != Interaction::Hovered { continue; }
        if whz.is_some() { over_warehouse = true; }
        if bpz.is_some() { over_backpack = true; }
    }

    // ---- 拖拽开始 ----
    if drag.source.is_none() && mouse.just_pressed(MouseButton::Left) && !shift.pressed(KeyCode::ShiftLeft) {
        if let Some((row, _)) = wh_rows.iter().find(|(_, it)| **it == Interaction::Pressed) {
            if carried_index(&load, row.0).is_none() {
                drag.source = Some(DragSource::Warehouse(row.0));
                set_ghost(&mut ghost, &mut ghost_text, &load, drag.source.as_ref().unwrap(), &mut window_query);
            }
        } else if let Some((slot, _)) = bp_slots.iter().find(|(_, it)| **it == Interaction::Pressed) {
            drag.source = Some(DragSource::Carried(slot.0));
            set_ghost(&mut ghost, &mut ghost_text, &load, drag.source.as_ref().unwrap(), &mut window_query);
        }
    }

    // ---- 拖拽中：幽灵跟随光标 ----
    if let Some(source) = drag.source {
        if let Ok((_, mut style)) = ghost.single_mut() {
            if let Ok(window) = window_query.single_mut() {
                if let Some(cursor) = window.cursor_position() {
                    style.left = Val::Px(cursor.x - 70.0);
                    style.top = Val::Px(cursor.y - 20.0);
                }
            }
        }
        if mouse.just_released(MouseButton::Left) {
            match source {
                DragSource::Warehouse(idx) if over_backpack => {
                    if load.carried.len() < LOADOUT_CAPACITY && carried_index(&load, idx).is_none() {
                        load.carried.push(WAREHOUSE_POOL[idx].clone());
                    }
                }
                DragSource::Carried(idx) if over_warehouse => {
                    if idx < load.carried.len() {
                        load.carried.remove(idx);
                    }
                }
                _ => {} // 松开在无效处：撤销
            }
            drag.source = None;
            if let Ok((mut gv, _)) = ghost.single_mut() { *gv = Visibility::Hidden; }
        }
        return;
    }

    // ---- 快移：Shift+左键 直接移动 ----
    if shift.pressed(KeyCode::ShiftLeft) && mouse.just_pressed(MouseButton::Left) {
        if let Some((row, _)) = wh_rows.iter().find(|(_, it)| **it == Interaction::Pressed) {
            if load.carried.len() < LOADOUT_CAPACITY && carried_index(&load, row.0).is_none() {
                load.carried.push(WAREHOUSE_POOL[row.0].clone());
            }
        } else if let Some((slot, _)) = bp_slots.iter().find(|(_, it)| **it == Interaction::Pressed) {
            if slot.0 < load.carried.len() {
                load.carried.remove(slot.0);
            }
        }
    }
}

/// 拖拽开始时初始化幽灵：显示被拖物资名并定位到光标处
fn set_ghost(
    ghost: &mut Query<(&mut Visibility, &mut Node), (With<LoadoutGhost>, Without<LoadoutPanelRoot>)>,
    ghost_text: &mut GhostTextQ,
    load: &Loadout,
    source: &DragSource,
    window_query: &mut Query<&mut Window, With<PrimaryWindow>>,
) {
    if let Ok((mut gv, mut gs)) = ghost.single_mut() {
        *gv = Visibility::Visible;
        if let Ok(window) = window_query.single_mut() {
            if let Some(cursor) = window.cursor_position() {
                gs.left = Val::Px(cursor.x - 70.0);
                gs.top = Val::Px(cursor.y - 20.0);
            }
        }
    }
    if let Ok((mut text, mut color)) = ghost_text.single_mut() {
        match source {
            DragSource::Warehouse(i) => {
                if let Some(item) = WAREHOUSE_POOL.get(*i) {
                    text.0 = item.name.to_string();
                    color.0 = item.color;
                }
            }
            DragSource::Carried(i) => {
                if let Some(item) = load.carried.get(*i) {
                    text.0 = item.name.to_string();
                    color.0 = item.color;
                }
            }
        }
    }
}