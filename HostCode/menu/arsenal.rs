//! 仓库选装浮层：100% 对齐 0.3.2 参考版的双清单交互「拖拽 + Shift+左键」。
//!
//! 设计动机：这是玩法决策入口。语义与旧版逐条一致——
//! - 左键按住仓库行 / 背包槽 = 开始拖拽，跟随光标出现「幽灵物资名」；
//! - 松手落在**对方容器**上才移动（拖到背包=携带，拖回仓库=取消），否则撤销；
//! - `Shift+左键` = 快捷移动（不拖拽，直接携带/取消）；
//! - 背包容许数 = `LOADOUT_CAPACITY`，仓库行内用「已携带 ✓」绿标。
//! 携带结果仅存 `ArsenalSelection` 会话热副本，确认时把清单原样上报服务端，由服务端裁决资格。
//! 客户端绝不自行结算；离开主菜单由 `StateScoped` 统一递归销毁浮层。
//!
//! 系统间冲突规避：所有 `&mut Text` / `&mut Visibility` / `&mut BackgroundColor` 查询
//! 彼此加交叉 `Without` 过滤器，保证 bevy 可在编译期证明互不相交，避免 B0001 调度 panic。

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use cute_of_duty_server::net::protocol::ClientMessage;

use crate::flow::flow_state::{self as flow, AppState, CjkFont};
use crate::net::network::NetOut;
use crate::shared::theme;

/// 仓库物资清单（对齐 0.3.2 参考版：生存/防御/补给 + 四系元素手雷）。
/// 名字原样经 `Loadout { carried }` 上报服务端；带入进图后的「生效结算」属训练场待办。
const MVP_ITEMS: [&str; 7] = [
    "医疗包", "护甲板", "额外弹药", "烈焰手雷", "冰霜手雷", "雷电手雷", "毒素手雷",
];

/// 与 `MVP_ITEMS` 顺序一致的行内文字配色（对齐参考版按物资类型着色）。
const ITEM_COLORS: [Color; 7] = [
    Color::srgb(0.85, 0.22, 0.22), // 医疗包·红
    Color::srgb(0.25, 0.55, 0.90), // 护甲板·蓝
    Color::srgb(0.90, 0.78, 0.30), // 额外弹药·金
    Color::srgb(0.92, 0.55, 0.20), // 烈焰手雷·橙
    Color::srgb(0.35, 0.80, 0.85), // 冰霜手雷·青
    Color::srgb(0.90, 0.78, 0.30), // 雷电手雷·黄
    Color::srgb(0.40, 0.80, 0.42), // 毒素手雷·绿
];

/// 背包容纳上限（与服务端 4×3 背包格位同口径）。
const LOADOUT_CAPACITY: usize = 12;

/// 仓库 / 背包网格几何：4 列 × 3 行（对齐服务端 `BACKPACK_SLOTS` 的 4×3 形态）。
const ARSENAL_COLS: usize = 4;
const ARSENAL_ROWS: usize = 3;
const ARSENAL_SLOTS: usize = ARSENAL_COLS * ARSENAL_ROWS;
const CELL_W: f32 = 132.0;
const CELL_H: f32 = 56.0;
const CELL_GAP: f32 = 8.0;

/// 浮层左上角偏移（相对屏幕）。
const OVERLAY_OFFSET: f32 = 120.0;

// ——— 资源 ———

/// 浮层当前是否展开（主菜单按钮 / 确认 / 返回 共同维护）。
#[derive(Resource, Default)]
pub struct ArsenalVisible(pub bool);

/// 本局已携带的物资名（确认时上报服务端；即「背包」清单，按携带顺序）。
#[derive(Resource, Default)]
pub struct ArsenalSelection(pub Vec<String>);

/// 拖拽会话状态：谁被拖起。
#[derive(Resource, Default)]
pub struct ArsenalDrag {
    pub source: Option<ArsenalDragSource>,
}

/// 拖拽源：来自仓库池（按下标）或背包槽（按下标）。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ArsenalDragSource {
    Warehouse(usize),
    Carried(usize),
}

// ——— 组件标记 ———
/// 浮层根节点。
#[derive(Component)]
pub struct ArsenalRoot;
/// 左「仓库」单个物资行（记录清单索引）。
#[derive(Component)]
pub struct ArsenalRow {
    pub index: usize,
}
/// 右「背包」槽（按下标 0..CAPACITY 定位携带序位）。
#[derive(Component)]
pub struct BackpackRow {
    pub index: usize,
}
/// 左列容器：作为拖拽「仓库落区」勾选目标。
#[derive(Component)]
pub struct WarehouseZone;
/// 右列容器：作为拖拽「背包落区」勾选目标。
#[derive(Component)]
pub struct BackpackZone;
/// 仓库行的「已携带 ✓」绿标文本（记录清单索引）。
#[derive(Component)]
pub struct WhStatusText(pub usize);
/// 背包槽的物资名/空 文本（记录槽位）。
#[derive(Component)]
pub struct CarriedSlotText(pub usize);
/// 背包容量计数文本。
#[derive(Component)]
pub struct BackpackCapacity;
/// 拖拽「幽灵」根节点（跟随光标）。
#[derive(Component)]
pub struct LoadoutGhost;
/// 拖拽「幽灵」物资名文本。
#[derive(Component)]
pub struct LoadoutGhostText;
/// 「开始游戏」按钮。
#[derive(Component)]
pub struct JinButton;
/// 「返回」按钮：仅收起浮层。
#[derive(Component)]
pub struct BackButton;

// ——— 定制查询类型（互斥；供拖拽系统与刷新函数共用）———
/// 仓库行绿标文本（与其余 &mut Text 互斥）。
type WhTextQ<'w, 's> = Query<
    'w,
    's,
    (&'static WhStatusText, &'static mut Text),
    (Without<CarriedSlotText>, Without<BackpackCapacity>, Without<LoadoutGhostText>),
>;
/// 背包槽文本（与其余 &mut Text 互斥）。
type BpTextQ<'w, 's> = Query<
    'w,
    's,
    (&'static CarriedSlotText, &'static mut Text),
    (Without<WhStatusText>, Without<BackpackCapacity>, Without<LoadoutGhostText>),
>;
/// 容量计数文本（与其余 &mut Text 互斥）。
type CapTextQ<'w, 's> = Query<
    'w,
    's,
    &'static mut Text,
    (With<BackpackCapacity>, Without<WhStatusText>, Without<CarriedSlotText>, Without<LoadoutGhostText>),
>;
/// 拖拽幽灵文本（与其余 &mut Text 互斥）。
type GhostTextQ<'w, 's> = Query<
    'w,
    's,
    &'static mut Text,
    (With<LoadoutGhostText>, Without<WhStatusText>, Without<CarriedSlotText>, Without<BackpackCapacity>),
>;
/// 幽灵根（&mut Visibility/Style；与背包行 Visibility 互斥）。
type GhostQ<'w, 's> = Query<
    'w,
    's,
    (&'static mut Visibility, &'static mut Style),
    (With<LoadoutGhost>, Without<ArsenalRoot>, Without<BackpackRow>),
>;

// ——— 工具 ———
fn carried_index(sel: &ArsenalSelection, pool_idx: usize) -> Option<usize> {
    let name = MVP_ITEMS.get(pool_idx)?;
    sel.0.iter().position(|it| it.as_str() == *name)
}

/// 4 列网格容器样式（自动换行成 3 行）。
fn grid_style() -> Style {
    Style {
        width: Val::Px(ARSENAL_COLS as f32 * CELL_W + (ARSENAL_COLS - 1) as f32 * CELL_GAP),
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        row_gap: Val::Px(CELL_GAP),
        column_gap: Val::Px(CELL_GAP),
        ..default()
    }
}

/// 单个格位样式（内容居中）。
fn cell_style() -> Style {
    Style {
        width: Val::Px(CELL_W),
        height: Val::Px(CELL_H),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        column_gap: Val::Px(6.0),
        ..default()
    }
}

fn pool_color(name: &str) -> Color {
    MVP_ITEMS
        .iter()
        .position(|it| *it == name)
        .map(|i| ITEM_COLORS[i])
        .unwrap_or(Color::WHITE)
}

/// 在 `MainMenu` 下确保浮层存在（含左右落区标记与拖拽幽灵，字体就绪后一次性建出）。
pub fn ensure_overlay(mut commands: Commands, fonts: Res<CjkFont>, exists: Query<(), With<ArsenalRoot>>) {
    if fonts.0.is_none() || !exists.is_empty() {
        return;
    }
    let accent = theme::ACCENT_CYAN;
    commands
        .spawn((
            ArsenalRoot,
            StateScoped(AppState::MainMenu),
            NodeBundle {
                visibility: Visibility::Hidden,
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(OVERLAY_OFFSET),
                    top: Val::Px(OVERLAY_OFFSET),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(12.0),
                    padding: UiRect::all(Val::Px(18.0)),
                    ..default()
                },
                background_color: theme::PANEL_BG.into(),
                ..default()
            },
        ))
        .with_children(|o| {
            o.spawn(TextBundle::from_section(
                "仓库 · 携带物资",
                flow::style(&fonts, 28.0, theme::TASK_GOLD),
            ));
            o.spawn(TextBundle::from_section(
                "拖拽仓库物资到右侧背包=携带 · 拖回左侧=不带 · Shift+左键 快捷移动",
                flow::style(&fonts, 15.0, theme::TEXT_DIM),
            ));

            o.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(28.0),
                    align_items: AlignItems::FlexStart,
                    ..default()
                },
                ..default()
            })
            .with_children(|cols| {
                // 左：仓库池（4×3 格位；落区 = 取消携带）
                cols.spawn((
                    WarehouseZone,
                    Interaction::default(),
                    NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(8.0),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        ..default()
                    },
                ))
                .with_children(|wa| {
                    wa.spawn(TextBundle::from_section(
                        "仓库（物资池）",
                        flow::style(&fonts, 20.0, accent),
                    ));
                    wa.spawn(NodeBundle {
                        style: grid_style(),
                        ..default()
                    })
                    .with_children(|grid| {
                        // 物资池按 4×3 铺格：有物资的格可拖拽，其余为空格位（不响应拖拽）。
                        for i in 0..ARSENAL_SLOTS {
                            match MVP_ITEMS.get(i) {
                                Some(name) => {
                                    grid.spawn((
                                        ArsenalRow { index: i },
                                        Interaction::default(),
                                        NodeBundle {
                                            style: cell_style(),
                                            background_color: Color::srgb(0.20, 0.22, 0.25).into(),
                                            ..default()
                                        },
                                    ))
                                    .with_children(|r| {
                                        r.spawn(TextBundle::from_section(
                                            name.to_string(),
                                            flow::style(&fonts, 18.0, ITEM_COLORS[i]),
                                        ));
                                        r.spawn((
                                            WhStatusText(i),
                                            TextBundle::from_section(
                                                String::new(),
                                                flow::style(&fonts, 15.0, Color::srgb(0.4, 0.85, 0.4)),
                                            ),
                                        ));
                                    });
                                }
                                None => {
                                    grid.spawn(NodeBundle {
                                        style: cell_style(),
                                        background_color: Color::srgb(0.13, 0.14, 0.17).into(),
                                        ..default()
                                    });
                                }
                            }
                        }
                    });
                });

                // 右：背包槽（4×3 格位；落区 = 携带）
                cols.spawn((
                    BackpackZone,
                    Interaction::default(),
                    NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(8.0),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        ..default()
                    },
                ))
                .with_children(|bk| {
                    bk.spawn(TextBundle::from_section(
                        "携 带 背 包",
                        flow::style(&fonts, 20.0, accent),
                    ));
                    bk.spawn((
                        BackpackCapacity,
                        TextBundle::from_section(
                            format!("0 / {}", LOADOUT_CAPACITY),
                            flow::style(&fonts, 16.0, theme::TEXT_DIM),
                        ),
                    ));
                    bk.spawn(NodeBundle {
                        style: grid_style(),
                        ..default()
                    })
                    .with_children(|grid| {
                        for s in 0..ARSENAL_SLOTS {
                            grid.spawn((
                                BackpackRow { index: s },
                                Interaction::default(),
                                NodeBundle {
                                    style: cell_style(),
                                    background_color: Color::srgb(0.20, 0.22, 0.25).into(),
                                    ..default()
                                },
                            ))
                            .with_children(|slot| {
                                slot.spawn((
                                    CarriedSlotText(s),
                                    TextBundle::from_section(
                                        "空".to_string(),
                                        flow::style(&fonts, 18.0, Color::srgb(0.85, 0.85, 0.85)),
                                    ),
                                ));
                            });
                        }
                    });
                });
            });

            // 底部操作行：返回 / 开始游戏。
            o.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(12.0),
                    ..default()
                },
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    BackButton,
                    Interaction::default(),
                    NodeBundle {
                        style: Style {
                            width: Val::Px(120.0),
                            height: Val::Px(44.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        background_color: Color::srgb(0.22, 0.22, 0.24).into(),
                        ..default()
                    },
                ))
                .with_children(|b| {
                    b.spawn(TextBundle::from_section(
                        "返 回",
                        flow::style(&fonts, 22.0, Color::WHITE),
                    ));
                });
                row.spawn((
                    JinButton,
                    Interaction::default(),
                    NodeBundle {
                        style: Style {
                            width: Val::Px(376.0),
                            height: Val::Px(44.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        background_color: Color::srgb(0.30, 0.42, 0.28).into(),
                        ..default()
                    },
                ))
                .with_children(|b| {
                    b.spawn(TextBundle::from_section(
                        "开始游戏",
                        flow::style(&fonts, 22.0, Color::WHITE),
                    ));
                });
            });
        });

    // 拖拽幽灵：独立于浮层根之外，避免被落区命中（`Without<ArsenalRoot>`）。
    commands
        .spawn((
            LoadoutGhost,
            StateScoped(AppState::MainMenu),
            NodeBundle {
                visibility: Visibility::Hidden,
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    padding: UiRect::px(10.0, 10.0, 4.0, 4.0),
                    ..default()
                },
                background_color: Color::srgb(0.12, 0.13, 0.16).into(),
                ..default()
            },
        ))
        .with_children(|g| {
            g.spawn((
                LoadoutGhostText,
                TextBundle::from_section(String::new(), flow::style(&fonts, 22.0, Color::WHITE)),
            ));
        });
}

/// 按携带态刷仓库行绿标 + 背包槽文案/显隐 + 容量计数。仅面板可见时由拖拽系统逐帧调用。
fn write_loadout_texts(sel: &ArsenalSelection, wh: &mut WhTextQ, bp: &mut BpTextQ, cap: &mut CapTextQ) {
    for (status, mut text) in wh.iter_mut() {
        let on = carried_index(sel, status.0).is_some();
        text.sections[0].value = if on { "已携带 ✓".to_string() } else { String::new() };
    }
    for (slot, mut text) in bp.iter_mut() {
        match sel.0.get(slot.0) {
            Some(name) => {
                text.sections[0].value = name.clone();
                text.sections[0].style.color = pool_color(name);
            }
            None => {
                text.sections[0].value = "空".to_string();
                text.sections[0].style.color = Color::srgb(0.85, 0.85, 0.85);
            }
        }
    }
    if let Ok(mut cap) = cap.get_single_mut() {
        if !cap.sections.is_empty() {
            cap.sections[0].value = format!("{} / {}", sel.0.len(), LOADOUT_CAPACITY);
        }
    }
}

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
    if let Ok(mut text) = ghost_text.get_single_mut() {
        let (name, color) = match source {
            ArsenalDragSource::Warehouse(i) => (MVP_ITEMS[*i].to_string(), ITEM_COLORS[*i]),
            ArsenalDragSource::Carried(i) => match sel.0.get(*i) {
                Some(n) => (n.clone(), pool_color(n)),
                None => return,
            },
        };
        text.sections[0].value = name;
        text.sections[0].style.color = color;
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