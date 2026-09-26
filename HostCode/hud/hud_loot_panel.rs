//! HUD 物资箱面板：**左「容器」右「背包」两个 4×3 网格 + 逐格转移**
//!
//! 设计动机（Why）：物资箱不再"开启即打包拿走"，而是 legacy 形态的**双向格位面板**——
//! 玩家在容器与背包之间逐格搬运，随时可反悔（放回）。"这一格是什么、搬不搬得动、弹药是否
//! 入池、武器是否换手"全部由服务端按格位内容裁决（`items::transfer` + `combat` 即时效果），
//! 客户端只画两个网格、上报"哪一边、第几格"（[`ClientMessage::LootTransfer`]）。
//!
//! 交互：`A/D`（或 ←/→）切换活动侧；`W/S`（或 ↑/↓）按行移动；`F/回车` 转移当前格；`Esc` 关闭。
//! 当前格高亮为琥珀描边，空格显示"空"。面板关闭条件：按 Esc、或箱子离开 AOI 视野。

use bevy::prelude::*;
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

/// 物资箱面板运行时状态（纯表现层编排）。
#[derive(Resource, Default)]
pub struct LootPanelState {
    /// 面板是否打开。
    pub open: bool,
    /// 目标物资箱实体 ID。
    pub target: u64,
    /// 活动侧：0 = 容器、1 = 背包。
    pub side: u8,
    /// 活动格下标（0..12）。
    pub index: usize,
}

/// 面板整屏根节点。
#[derive(Component)]
pub struct LootPanelRoot;
/// 一侧的网格格位（`side`：0 容器 / 1 背包）。
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
/// 底部操作提示文本。
#[derive(Component)]
pub struct LootHintText;

/// 装配物资箱面板（标题 + 两个 4×3 网格 + 操作提示，默认隐藏）。
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
        root.spawn(TextBundle::from_section(
            "物资箱",
            flow::style(fonts, 26.0, theme::TASK_GOLD),
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
}

/// 生成一侧网格（标题 + `GRID_ROWS × GRID_COLS` 格位）。
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
        col.spawn(TextBundle::from_section(
            title,
            flow::style(fonts, 18.0, theme::ACCENT_CYAN),
        ));
        col.spawn(NodeBundle {
            style: Style {
                width: Val::Px(GRID_COLS as f32 * CELL + (GRID_COLS - 1) as f32 * CELL_GAP),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                row_gap: Val::Px(CELL_GAP),
                column_gap: Val::Px(CELL_GAP),
                ..default()
            },
            ..default()
        })
        .with_children(|grid| {
            for index in 0..LOOT_SLOTS {
                grid.spawn((
                    LootCell { side, index },
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

/// 键鼠输入：切侧 / 移格 / 转移 / 关闭；并在目标箱子离开视野时自动收起。
pub fn loot_panel_input(
    keys: Res<ButtonInput<KeyCode>>,
    snap: Res<SnapshotBuffer>,
    mut state: ResMut<LootPanelState>,
    out: Res<NetOut>,
) {
    if !state.open {
        return;
    }
    // 目标箱子消失（被 AOI 过滤或销毁）时自动收起，避免对着空气搬运。
    let target_present = snap
        .current
        .iter()
        .any(|e| e.entity_id == state.target && e.container.is_some());
    if !target_present {
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
        let dir = if state.side == 0 {
            TransferDir::Take
        } else {
            TransferDir::Put
        };
        let _ = out.0.send(ClientMessage::LootTransfer {
            target: state.target,
            dir,
            index: state.index,
        });
    }
}

/// 打开面板（由交互链在确认物资箱时调用）。
pub fn open_loot_panel(state: &mut LootPanelState, target: u64) {
    state.open = true;
    state.target = target;
    state.side = 0;
    state.index = 0;
}

/// 收起面板并清空目标。
fn close(state: &mut LootPanelState) {
    state.open = false;
    state.target = 0;
    state.side = 0;
    state.index = 0;
}

/// 离开训练场时复位物资箱面板。
pub fn reset_loot_panel(mut state: ResMut<LootPanelState>) {
    close(&mut state);
}

/// 每帧刷新两个网格：可见性、格内物品名、当前格高亮、操作提示。
#[allow(clippy::type_complexity)]
pub fn sync_loot_panel(
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    state: Res<LootPanelState>,
    mut root: Query<&mut Visibility, With<LootPanelRoot>>,
    mut cells: Query<(&LootCell, &mut BackgroundColor, &mut BorderColor)>,
    mut texts: Query<(&LootCellText, &mut Text)>,
    mut hint: Query<&mut Text, (With<LootHintText>, Without<LootCellText>)>,
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

    // 容器内容取自目标箱子的快照；背包内容取自本人快照。
    let container: &[Option<LootItem>] = snap
        .current
        .iter()
        .find(|e| e.entity_id == state.target)
        .and_then(|e| e.container.as_deref())
        .unwrap_or(&[]);
    let backpack: &[Option<LootItem>] = snap
        .current
        .iter()
        .find(|e| e.entity_id == player.entity_id)
        .and_then(|e| e.backpack.as_deref())
        .unwrap_or(&[]);

    for (cell, mut bg, mut border) in &mut cells {
        let active = cell.side == state.side && cell.index == state.index;
        // 高亮当前格：活动侧琥珀描边，非活动侧保持面板底。
        *bg = if active {
            theme::ROW_HOVER.into()
        } else {
            Color::srgba(0.10, 0.12, 0.16, 0.9).into()
        };
        *border = BorderColor(if active {
            theme::ACCENT_AMBER
        } else {
            theme::PANEL_BORDER
        });
    }

    for (cell, mut text) in &mut texts {
        let source = if cell.side == 0 { container } else { backpack };
        match source.get(cell.index).and_then(|s| s.as_ref()) {
            Some(item) => {
                text.sections[0].value = item.label.clone();
                text.sections[0].style.color = theme::TEXT_WHITE;
            }
            None => {
                text.sections[0].value = "空".to_string();
                text.sections[0].style.color = theme::TEXT_DIM;
            }
        }
    }

    if let Ok(mut t) = hint.get_single_mut() {
        let side = if state.side == 0 { "容器" } else { "背包" };
        let action = if state.side == 0 { "取出到背包" } else { "放回容器" };
        t.sections[0].value = format!("A/D 切侧 · W/S 移格 · F 转移（当前 {side} 第 {} 格 → {action}）· Esc 关闭", state.index + 1);
    }
}
