//! 仓库选装浮层的**装配与布局**：4×3 网格几何 + `ensure_overlay` 一次性建出 UI 树。
//!
//! 设计动机（Why）：浮层是纯表现层实体树，只在字体就绪后建一次；把它与交互/刷新分开，
//! 装配代码可独立阅读，不必在大文件里与拖拽状态机交织。

use bevy::prelude::*;

use crate::flow::flow_state::{self as flow, AppState, CjkFont, LoadoutPresets};
use crate::shared::theme;

use super::state::*;

/// 4 列网格容器样式（自动换行成 3 行）。
fn grid_style() -> Node {
    Node {
        width: Val::Px(ARSENAL_COLS as f32 * CELL_W + (ARSENAL_COLS - 1) as f32 * CELL_GAP),
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::Wrap,
        row_gap: Val::Px(CELL_GAP),
        column_gap: Val::Px(CELL_GAP),
        ..default()
    }
}

/// 单个格位样式（内容居中）。
fn cell_style() -> Node {
    Node {
        width: Val::Px(CELL_W),
        height: Val::Px(CELL_H),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        column_gap: Val::Px(6.0),
        ..default()
    }
}

/// 预设按钮：点击后以服务端下发目录的第 `index` 套预设覆盖携带清单（内容归服务端权威）。
fn spawn_preset_button(parent: &mut ChildBuilder, fonts: &CjkFont, index: usize, label: &str) {
    parent
        .spawn((
            PresetButton { index },
            Interaction::default(),
            Node {
                padding: UiRect::px(14.0, 14.0, 6.0, 6.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.14, 0.18, 0.24)),
            BorderColor(theme::ACCENT_CYAN),
            BorderRadius::all(Val::Px(4.0)),
        ))
        .with_children(|b| {
            b.spawn(flow::text(label.to_string(), flow::style(fonts, 18.0, Color::WHITE)));
        });
}

/// 在 `MainMenu` 下确保浮层存在（含左右落区标记与拖拽幽灵，字体就绪后一次性建出）。
pub fn ensure_overlay(
    mut commands: Commands,
    fonts: Res<CjkFont>,
    presets: Res<LoadoutPresets>,
    exists: Query<(), With<ArsenalRoot>>,
) {
    if fonts.0.is_none() || !exists.is_empty() {
        return;
    }
    let accent = theme::ACCENT_CYAN;
    commands
        .spawn((
            ArsenalRoot,
            StateScoped(AppState::MainMenu),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(OVERLAY_OFFSET),
                top: Val::Px(OVERLAY_OFFSET),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                padding: UiRect::all(Val::Px(18.0)),
                ..default()
            },
            BackgroundColor(theme::PANEL_BG),
            Visibility::Hidden,
            // 浮层是独立 UI 根：与主菜单根（整屏不透明）同级时，Bevy 的根节点排序不稳定，
            // 可能被排到后面而被盖住。GlobalZIndex 专用于跨层级/跨根排序，确保浮层稳定在上。
            GlobalZIndex(30),
        ))
        .with_children(|o| {
            o.spawn(flow::text(
                "仓库 · 携带物资",
                flow::style(&fonts, 28.0, theme::TASK_GOLD),
            ));
            o.spawn(flow::text(
                "拖拽仓库物资到右侧背包=携带 · 拖回左侧=不带 · Shift+左键 快捷移动",
                flow::style(&fonts, 15.0, theme::TEXT_DIM),
            ));

            // 预设栏：服务端权威下发的一键选装（点击即以该预设内容覆盖携带清单）。
            if !presets.presets.is_empty() {
                o.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(10.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|row| {
                    row.spawn(flow::text("预设：", flow::style(&fonts, 18.0, theme::TEXT_DIM)));
                    for (i, p) in presets.presets.iter().enumerate() {
                        spawn_preset_button(row, &fonts, i, &p.name);
                    }
                });
            }

            o.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(28.0),
                align_items: AlignItems::FlexStart,
                ..default()
            })
            .with_children(|cols| {
                // 左：仓库池（4×3 格位；落区 = 取消携带）
                cols.spawn((
                    WarehouseZone,
                    Interaction::default(),
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .with_children(|wa| {
                    wa.spawn(flow::text(
                        "仓库（物资池）",
                        flow::style(&fonts, 20.0, accent),
                    ));
                    wa.spawn(grid_style())
                    .with_children(|grid| {
                        // 物资池按 4×3 铺格：有物资的格可拖拽，其余为空格位（不响应拖拽）。
                        for i in 0..ARSENAL_SLOTS {
                            match MVP_ITEMS.get(i) {
                                Some(name) => {
                                    grid.spawn((
                                        ArsenalRow { index: i },
                                        Interaction::default(),
                                        cell_style(),
                                        BackgroundColor(Color::srgb(0.20, 0.22, 0.25)),
                                    ))
                                    .with_children(|r| {
                                        r.spawn(flow::text(
                                            name.to_string(),
                                            flow::style(&fonts, 18.0, ITEM_COLORS[i]),
                                        ));
                                        r.spawn((
                                            WhStatusText(i),
                                            flow::text(
                                                String::new(),
                                                flow::style(&fonts, 15.0, Color::srgb(0.4, 0.85, 0.4)),
                                            ),
                                        ));
                                    });
                                }
                                None => {
                                    grid.spawn((
                                        cell_style(),
                                        BackgroundColor(Color::srgb(0.13, 0.14, 0.17)),
                                    ));
                                }
                            }
                        }
                    });
                });

                // 右：背包槽（4×3 格位；落区 = 携带）
                cols.spawn((
                    BackpackZone,
                    Interaction::default(),
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .with_children(|bk| {
                    bk.spawn(flow::text(
                        "携 带 背 包",
                        flow::style(&fonts, 20.0, accent),
                    ));
                    bk.spawn((
                        BackpackCapacity,
                        flow::text(
                            format!("0 / {}", LOADOUT_CAPACITY),
                            flow::style(&fonts, 16.0, theme::TEXT_DIM),
                        ),
                    ));
                    bk.spawn(grid_style())
                    .with_children(|grid| {
                        for s in 0..ARSENAL_SLOTS {
                            grid.spawn((
                                BackpackRow { index: s },
                                Interaction::default(),
                                cell_style(),
                                BackgroundColor(Color::srgb(0.20, 0.22, 0.25)),
                            ))
                            .with_children(|slot| {
                                slot.spawn((
                                    CarriedSlotText(s),
                                    flow::text(
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
            o.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(12.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    BackButton,
                    Interaction::default(),
                    Node {
                        width: Val::Px(120.0),
                        height: Val::Px(44.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.22, 0.22, 0.24)),
                ))
                .with_children(|b| {
                    b.spawn(flow::text(
                        "返 回",
                        flow::style(&fonts, 22.0, Color::WHITE),
                    ));
                });
                row.spawn((
                    JinButton,
                    Interaction::default(),
                    Node {
                        width: Val::Px(376.0),
                        height: Val::Px(44.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.30, 0.42, 0.28)),
                ))
                .with_children(|b| {
                    b.spawn(flow::text(
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
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                padding: UiRect::px(10.0, 10.0, 4.0, 4.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.13, 0.16)),
            Visibility::Hidden,
            // 幽灵是独立根，取比浮层更高的全局层级，保证始终压在浮层之上跟随光标。
            GlobalZIndex(40),
        ))
        .with_children(|g| {
            g.spawn((
                LoadoutGhostText,
                flow::text(String::new(), flow::style(&fonts, 22.0, Color::WHITE)),
            ));
        });
}