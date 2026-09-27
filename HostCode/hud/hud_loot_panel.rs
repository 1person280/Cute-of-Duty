//! HUD 格位搬运面板（统一形态）：**左「来源」右「背包」两个 4×3 网格 + 鼠标拖拽 / Shift 快捷**
//!
//! 设计动机（Why）：物资箱与补给台此前是两套别扭的操作（键盘逐格搬运 / 二级选项菜单）。
//! 0.6-Snapshot-9 统一为**与仓库选装完全一致**的形态（见 `menu/arsenal.rs`）：
//! - **左键按住来源格 = 拖拽**，松手落在「背包」区 = 取入，落在「来源」区 = 放回；
//! - **Shift+左键** = 快捷移动（不拖拽，直接取入/放回）；
//! - 键盘 `W/S/A/D + F` 仍作后备。
//!
//! 面板两种形态（[`LootMode`]）共用同一套网格与拖拽逻辑：
//! - [`LootMode::Crate`] 物资箱：左 = 权威 4×3 容器（双向转移，服务端 [`items::transfer`] 裁决）；
//! - [`LootMode::Supply`] 补给台：左 = 固定补给项（[`SUPPLY_OFFERINGS`]），拖入背包即发出
//!   `InteractChoice::Supply`，领取与否/发放多少仍由服务端 `interact::settle` 裁决。
//!
//! 服务器权威（Why）：客户端只上报"哪一侧、第几格、哪个方向"，"这一格是什么、搬不搬得动、
//! 弹药是否入池、武器是否换手"全部由服务端裁决后经快照回传——客户端只画两个网格。

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use cute_of_duty_server::interact::{InteractChoice, SupplyKind};
use cute_of_duty_server::items::{LootItem, TransferDir};
use cute_of_duty_server::net::protocol::ClientMessage;

use crate::flow::flow_state::{self as flow, CjkFont, LocalPlayer};
use crate::net::network::NetOut;
use crate::net::snapshot::SnapshotBuffer;
use crate::shared::theme;

/// 每侧网格列数 / 行数（4×3，与服务端 `BACKPACK_SLOTS` / `CONTAINER_SLOTS` 同口径）。
const GRID_COLS: usize = 4;
const GRID_ROWS: usize = 3;
/// 每侧格位数。
const LOOT_SLOTS: usize = GRID_COLS * GRID_ROWS;
/// 单格边长与格间距（px）。
const CELL: f32 = 76.0;
const CELL_GAP: f32 = 6.0;

/// 补给台固定提供的物资（左网格前几格；可无限领取）。
///
/// 设计动机：补给台语义是"照名领取"的发放点而非有限容器，故不带 `Container`；
/// 其可领取项以本表为表现层清单，**领取本身仍由服务端 `settle` 权威结算**。
pub const SUPPLY_OFFERINGS: [SupplyKind; 3] =
    [SupplyKind::Ammo, SupplyKind::Health, SupplyKind::Armor];

/// 面板形态：物资箱（容器↔背包，双向）或补给台（固定补给项→背包，单向）。
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum LootMode {
    #[default]
    Crate,
    Supply,
}

/// 面板运行时状态（纯表现层编排）。
#[derive(Resource, Default)]
pub struct LootPanelState {
    /// 面板是否打开。
    pub open: bool,
    /// 目标站点实体 ID（物资箱 / 补给台）。
    pub target: u64,
    /// 当前形态（决定左网格内容与转移语义）。
    pub mode: LootMode,
    /// 键盘活动侧：0 = 来源、1 = 背包。
    pub side: u8,
    /// 键盘活动格下标（0..12）。
    pub index: usize,
    /// 鼠标拖拽源 `(side, index)`；`None` = 未在拖拽。
    pub drag: Option<(u8, usize)>,
}

/// 面板整屏根节点。
#[derive(Component)]
pub struct LootPanelRoot;
/// 面板标题文本。
#[derive(Component)]
pub struct LootTitleText;
/// 一侧网格的列标题（`side`：0 来源 / 1 背包）。
#[derive(Component)]
pub struct LootSideTitle {
    pub side: u8,
}
/// 一侧的网格格位（`side`：0 来源 / 1 背包）。
#[derive(Component)]
pub struct LootCell {
    pub side: u8,
    pub index: usize,
}
/// 格位内物品名文本。
#[derive(Component)]
pub struct LootCellText {
    pub side: u8,
    pub index: usize,
}
/// 一侧网格容器（作为拖拽落区；`side` 同格位）。
#[derive(Component)]
pub struct LootZone {
    pub side: u8,
}
/// 底部操作提示文本。
#[derive(Component)]
pub struct LootHintText;
/// 拖拽「幽灵」根节点（跟随光标显示被拖物资名；与仓库选装同形态）。
#[derive(Component)]
pub struct LootGhost;
/// 拖拽「幽灵」内的物资名文本。
#[derive(Component)]
pub struct LootGhostText;

/// 装配格位搬运面板（标题 + 两个 4×3 网格 + 操作提示，默认隐藏）。
pub fn spawn_loot_panel(p: &mut ChildBuilder<'_>, fonts: &CjkFont) {
    p.spawn((
        LootPanelRoot,
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            background_color: Color::srgba(0.02, 0.03, 0.05, 0.62).into(),
            visibility: Visibility::Hidden,
            ..default()
        },
    ))
    .with_children(|root| {
        root.spawn((
            LootTitleText,
            TextBundle::from_section("物资箱", flow::style(fonts, 26.0, theme::TASK_GOLD)),
        ));
        root.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(48.0),
                align_items: AlignItems::FlexStart,
                ..default()
            },
            ..default()
        })
        .with_children(|cols| {
            spawn_grid(cols, fonts, 0, "容器");
            spawn_grid(cols, fonts, 1, "背包");
        });
        root.spawn((
            LootHintText,
            TextBundle::from_section("", flow::style(fonts, 14.0, theme::TEXT_DIM)),
        ));
    });

    // 拖拽幽灵：作为面板之后的兄弟节点（绘制在上层），不参与命中判定（无 `Interaction`）。
    p.spawn((
        LootGhost,
        NodeBundle {
            visibility: Visibility::Hidden,
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                padding: UiRect::px(10.0, 10.0, 4.0, 4.0),
                ..default()
            },
            background_color: Color::srgba(0.12, 0.13, 0.16, 0.95).into(),
            ..default()
        },
    ))
    .with_children(|g| {
        g.spawn((
            LootGhostText,
            TextBundle::from_section("", flow::style(fonts, 16.0, theme::TEXT_WHITE)),
        ));
    });
}

/// 生成一侧网格（标题 + `GRID_ROWS × GRID_COLS` 格位；网格容器本身是拖拽落区）。
fn spawn_grid(cols: &mut ChildBuilder<'_>, fonts: &CjkFont, side: u8, title: &str) {
    cols.spawn(NodeBundle {
        style: Style {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(8.0),
            align_items: AlignItems::Center,
            ..default()
        },
        ..default()
    })
    .with_children(|col| {
        col.spawn((
            LootSideTitle { side },
            TextBundle::from_section(title, flow::style(fonts, 18.0, theme::ACCENT_CYAN)),
        ));
        // 网格容器 = 落区：`Interaction` 让光标悬停可判定"松手落在哪一侧"。
        col.spawn((
            LootZone { side },
            Interaction::default(),
            NodeBundle {
                style: Style {
                    width: Val::Px(GRID_COLS as f32 * CELL + (GRID_COLS - 1) as f32 * CELL_GAP),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    row_gap: Val::Px(CELL_GAP),
                    column_gap: Val::Px(CELL_GAP),
                    ..default()
                },
                ..default()
            },
        ))
        .with_children(|grid| {
            for index in 0..LOOT_SLOTS {
                grid.spawn((
                    LootCell { side, index },
                    Interaction::default(),
                    NodeBundle {
                        style: Style {
                            width: Val::Px(CELL),
                            height: Val::Px(CELL),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        background_color: Color::srgba(0.10, 0.12, 0.16, 0.9).into(),
                        border_color: BorderColor(theme::PANEL_BORDER),
                        ..default()
                    },
                ))
                .with_children(|cell| {
                    cell.spawn((
                        LootCellText { side, index },
                        TextBundle::from_section("空", flow::style(fonts, 12.0, theme::TEXT_DIM)),
                    ));
                });
            }
        });
    });
}

/// 键盘后备输入：切侧 / 移格 / 转移 / 关闭；并在目标离开视野时自动收起。
pub fn loot_panel_input(
    keys: Res<ButtonInput<KeyCode>>,
    snap: Res<SnapshotBuffer>,
    mut state: ResMut<LootPanelState>,
    out: Res<NetOut>,
) {
    if !state.open {
        return;
    }
    // 目标消失（被 AOI 过滤或销毁）时自动收起，避免对着空气搬运。
    // 注意：补给台**没有** `Container`，故只按实体存在性判定（不能要求 container 存在）。
    if !snap.current.iter().any(|e| e.entity_id == state.target) {
        close(&mut state);
        return;
    }

    if keys.just_pressed(KeyCode::Escape) {
        close(&mut state);
        return;
    }

    if keys.just_pressed(KeyCode::KeyA) || keys.just_pressed(KeyCode::ArrowLeft) {
        state.side = 0;
    }
    if keys.just_pressed(KeyCode::KeyD) || keys.just_pressed(KeyCode::ArrowRight) {
        state.side = 1;
    }
    let row = state.index / GRID_COLS;
    let col = state.index % GRID_COLS;
    if keys.just_pressed(KeyCode::KeyW) || keys.just_pressed(KeyCode::ArrowUp) {
        state.index = row.saturating_sub(1) * GRID_COLS + col;
    }
    if keys.just_pressed(KeyCode::KeyS) || keys.just_pressed(KeyCode::ArrowDown) {
        state.index = (row + 1).min(GRID_ROWS - 1) * GRID_COLS + col;
    }

    if keys.just_pressed(KeyCode::KeyF) || keys.just_pressed(KeyCode::Enter) {
        commit_transfer(&state, &out, state.side, state.index);
    }
}

/// 鼠标搬运主系统：左键拖拽 + Shift+左键快捷移动（与仓库选装同形态）。
///
/// 拖拽期间跟随光标显示「幽灵物资名」——这是仓库选装已有的手感，格位面板此前缺失，
/// 表现为"拖着没反应"（实测反馈"没有预览，仓库是有的"）。
#[allow(clippy::too_many_arguments)]
pub fn loot_panel_drag(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<LootPanelState>,
    out: Res<NetOut>,
    cells: Query<(&LootCell, &Interaction)>,
    zones: Query<(&LootZone, &Interaction)>,
    texts: Query<(&LootCellText, &Text), Without<LootGhostText>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut ghost: Query<(&mut Visibility, &mut Style), With<LootGhost>>,
    mut ghost_text: Query<&mut Text, (With<LootGhostText>, Without<LootCellText>)>,
) {
    if !state.open {
        state.drag = None;
        if let Ok((mut vis, _)) = ghost.get_single_mut() {
            *vis = Visibility::Hidden;
        }
        return;
    }
    let shift =
        keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    // 松手落点所属侧（网格容器为落区；格位 `FocusPolicy` 默认 `Pass`，可穿透到容器）。
    let over_side = zones
        .iter()
        .find(|(_, it)| **it == Interaction::Hovered)
        .map(|(z, _)| z.side);

    // 拖拽开始 / Shift 快捷移动。
    if mouse.just_pressed(MouseButton::Left) {
        if let Some((cell, _)) = cells.iter().find(|(_, it)| **it == Interaction::Pressed) {
            if shift {
                commit_transfer(&state, &out, cell.side, cell.index);
            } else if can_pick_up(state.mode, cell.side) {
                state.drag = Some((cell.side, cell.index));
            }
        }
    }

    // 拖拽结束：落在对方侧才提交（落回原侧 = 撤销，与仓库拖拽一致）。
    if mouse.just_released(MouseButton::Left) {
        if let (Some((side, index)), Some(drop_side)) = (state.drag.take(), over_side) {
            if drop_side != side {
                commit_transfer(&state, &out, side, index);
            }
        }
    }

    // 拖拽中：幽灵物资名跟随光标；未拖拽则隐藏。
    match state.drag {
        Some((side, index)) => {
            let label = texts
                .iter()
                .find(|(c, _)| c.side == side && c.index == index)
                .map(|(_, t)| t.sections[0].value.clone())
                .unwrap_or_default();
            if let Ok((mut vis, mut style)) = ghost.get_single_mut() {
                *vis = Visibility::Visible;
                if let Ok(w) = window.get_single() {
                    if let Some(cur) = w.cursor_position() {
                        style.left = Val::Px(cur.x - 60.0);
                        style.top = Val::Px(cur.y - 18.0);
                    }
                }
            }
            if let Ok(mut t) = ghost_text.get_single_mut() {
                if !t.sections.is_empty() {
                    t.sections[0].value = label;
                }
            }
        }
        None => {
            if let Ok((mut vis, _)) = ghost.get_single_mut() {
                *vis = Visibility::Hidden;
            }
        }
    }
}

/// 该侧是否可作为拖拽源：补给台的「背包」侧拖入补给台无意义，禁止拖起。
fn can_pick_up(mode: LootMode, side: u8) -> bool {
    !(mode == LootMode::Supply && side == 1)
}

/// 依据形态与来源侧发出一次搬运意图（服务端权威裁决）。
fn commit_transfer(state: &LootPanelState, out: &NetOut, from_side: u8, index: usize) {
    match state.mode {
        LootMode::Crate => {
            // 物资箱：来源侧（容器）→ 背包 = Take；背包 → 容器 = Put。
            let dir = if from_side == 0 {
                TransferDir::Take
            } else {
                TransferDir::Put
            };
            let _ = out.0.send(ClientMessage::LootTransfer {
                target: state.target,
                dir,
                index,
            });
        }
        LootMode::Supply => {
            // 补给台：从补给项到背包 = 领取（单向，无"放回"语义）。
            if from_side == 0 {
                if let Some(kind) = SUPPLY_OFFERINGS.get(index).copied() {
                    let _ = out.0.send(ClientMessage::Interact {
                        target: state.target,
                        choice: InteractChoice::Supply { kind },
                    });
                }
            }
        }
    }
}

/// 打开物资箱面板（由交互链在确认物资箱时调用）。
pub fn open_loot_panel(state: &mut LootPanelState, target: u64) {
    open(state, target, LootMode::Crate);
}

/// 打开补给台面板（由交互链在确认补给台时调用；与物资箱共用同一套网格/拖拽形态）。
pub fn open_supply_panel(state: &mut LootPanelState, target: u64) {
    open(state, target, LootMode::Supply);
}

/// 打开面板并复位高亮/拖拽。
fn open(state: &mut LootPanelState, target: u64, mode: LootMode) {
    state.open = true;
    state.target = target;
    state.mode = mode;
    state.side = 0;
    state.index = 0;
    state.drag = None;
}

/// 收起面板并清空目标。
fn close(state: &mut LootPanelState) {
    state.open = false;
    state.target = 0;
    state.side = 0;
    state.index = 0;
    state.drag = None;
}

/// 离开训练场时复位面板。
pub fn reset_loot_panel(mut state: ResMut<LootPanelState>) {
    close(&mut state);
}

/// 每帧刷新两个网格：可见性、格内物品名、当前格高亮、拖拽源高亮、操作提示。
#[allow(clippy::type_complexity)]
pub fn sync_loot_panel(
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    state: Res<LootPanelState>,
    mut root: Query<&mut Visibility, With<LootPanelRoot>>,
    mut cells: Query<(&LootCell, &mut BackgroundColor, &mut BorderColor)>,
    mut texts: ParamSet<(
        Query<(&LootCellText, &mut Text)>,
        Query<&mut Text, With<LootHintText>>,
        Query<&mut Text, With<LootTitleText>>,
        Query<(&LootSideTitle, &mut Text)>,
    )>,
) {
    if let Ok(mut v) = root.get_single_mut() {
        *v = if state.open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !state.open {
        return;
    }

    // 来源侧内容：物资箱取目标容器格位；补给台取固定补给清单。
    let container: &[Option<LootItem>] = snap
        .current
        .iter()
        .find(|e| e.entity_id == state.target)
        .and_then(|e| e.container.as_deref())
        .unwrap_or(&[]);
    // 背包内容取自本人快照（两种形态共用右侧）。
    let backpack: &[Option<LootItem>] = snap
        .current
        .iter()
        .find(|e| e.entity_id == player.entity_id)
        .and_then(|e| e.backpack.as_deref())
        .unwrap_or(&[]);

    for (cell, mut bg, mut border) in &mut cells {
        let active = cell.side == state.side && cell.index == state.index;
        let dragging = state.drag == Some((cell.side, cell.index));
        // 高亮优先级：拖拽源（更醒目）> 键盘当前格。
        *bg = if dragging {
            theme::ACCENT_AMBER.into()
        } else if active {
            theme::ROW_HOVER.into()
        } else {
            Color::srgba(0.10, 0.12, 0.16, 0.9).into()
        };
        *border = BorderColor(if dragging || active {
            theme::ACCENT_AMBER
        } else {
            theme::PANEL_BORDER
        });
    }

    // 文本：来源侧按形态取容器格 / 补给清单，背包侧取本人背包。
    for (cell, mut text) in &mut texts.p0() {
        let label = if cell.side == 0 && state.mode == LootMode::Supply {
            SUPPLY_OFFERINGS
                .get(cell.index)
                .map(|k| k.label().to_string())
        } else {
            let source = if cell.side == 0 { container } else { backpack };
            source
                .get(cell.index)
                .and_then(|s| s.as_ref())
                .map(|item| item.label.clone())
        };
        match label {
            Some(label) => {
                text.sections[0].value = label;
                text.sections[0].style.color = theme::TEXT_WHITE;
            }
            None => {
                text.sections[0].value = "空".to_string();
                text.sections[0].style.color = theme::TEXT_DIM;
            }
        }
    }

    if let Ok(mut t) = texts.p2().get_single_mut() {
        t.sections[0].value = match state.mode {
            LootMode::Crate => "物资箱",
            LootMode::Supply => "补给台 · 领取补给",
        }
        .to_string();
    }
    for (title, mut text) in &mut texts.p3() {
        let name = match (state.mode, title.side) {
            (LootMode::Supply, 0) => "补给",
            (_, 0) => "容器",
            (_, _) => "背包",
        };
        text.sections[0].value = name.to_string();
    }

    if let Ok(mut t) = texts.p1().get_single_mut() {
        let verb = match state.mode {
            LootMode::Crate => {
                let side = if state.side == 0 { "容器" } else { "背包" };
                let action = if state.side == 0 { "取出到背包" } else { "放回容器" };
                format!(
                    "当前 {side} 第 {} 格 → {action}",
                    state.index + 1
                )
            }
            LootMode::Supply => format!("当前 第 {} 格 → 领取补给", state.index + 1),
        };
        let tail = match state.drag {
            Some(_) => "拖拽中：拖到「背包」松手即可",
            None => "鼠标拖拽 / Shift+左键 快捷移动 · W/S/A/D 移格 · F 确认 · Esc 关闭",
        };
        t.sections[0].value = format!("{verb} · {tail}");
    }
}