//! 仓库选装浮层：双清单「仓库 + 背包」点击选装 + 确认进场
//!
//! 设计动机：这是玩法决策入口——玩家从「仓库」全量物资池点击携带进「背包」，
//! 确认后把**已携带清单**上报服务端。「能带几件 / 带什么」的资格判定由服务端裁决
//! （本阶段仅存会话热副本 + 播报）；客户端只维护浮层显隐与本地选择高亮，绝不携带
//! 结算结果。浮层以 `StateScoped(MainMenu)` 挂载，仅用 `ArsenalVisible` 资源切换显隐，
//! 离开主菜单时由 state-scoped 统一递归销毁（服务端/本模块都不手动拆）。
//!
//! 双清单布局：左「仓库」列 = 全量物资池（本阶段 MVP 固定清单 `MVP_ITEMS`），每行
//! 点击在携带/取消间切换；右「背包」列 = 当前已携带项（`ArsenalSelection`），点击已
//! 携带的行即取消携带。背包行全程存在、仅按携带态切换 `Visibility`（避免动态增删
//! 实体），携带数由顶部 `BackpackCapacity` 计数文本展示。

use bevy::prelude::*;
use cute_of_duty_server::net::protocol::ClientMessage;

use crate::flow::flow_state::{self as flow, AppState, CjkFont};
use crate::net::network::NetOut;
use crate::shared::theme;

/// MVP 固定物资清单（名字即服务端 `conn_loadout` 里可识别项；本局结束即弃）。
const MVP_ITEMS: [&str; 4] = ["医疗包", "步枪弹药", "护甲片", "破片手雷"];
/// 浮层左上角偏移（相对屏幕）。
const OVERLAY_OFFSET: f32 = 120.0;

/// 浮层当前是否展开（主菜单按钮 / 确认按钮共同维护）。
#[derive(Resource, Default)]
pub struct ArsenalVisible(pub bool);

/// 本局已勾选携带的物资名（确认时上报服务端；即「背包」清单）。
#[derive(Resource, Default)]
pub struct ArsenalSelection(pub Vec<String>);

/// 浮层根节点标记。
#[derive(Component)]
pub struct ArsenalRoot;

/// 左「仓库」单个物资行标记（记录清单索引；背景色与 ✓ 由系统按选择态刷新）。
#[derive(Component)]
pub struct ArsenalRow {
    pub index: usize,
}

/// 右「背包」已携带行标记（同样按下标对应 `MVP_ITEMS`；点击即取消携带）。
#[derive(Component)]
pub struct BackpackRow {
    pub index: usize,
}

/// 「背包」容量计数文本（随携带数变化更新）。
#[derive(Component)]
pub struct BackpackCapacity;

/// 行内文本标签（供选择态刷新文本）。
#[derive(Component)]
pub struct RowLabel;

/// 「确认进入训练场」按钮。
#[derive(Component)]
pub struct ConfirmEntryButton;

/// 在 `MainMenu` 状态下确保浮层根存在（无则建，字体句柄未就绪时跳过本帧）。
pub fn ensure_overlay(mut commands: Commands, fonts: Res<CjkFont>, exists: Query<(), With<ArsenalRoot>>) {
    if fonts.0.is_none() || !exists.is_empty() {
        return;
    }
    let accent = theme::ACCENT_CYAN;
    let row_hover_bg = theme::ROW_HOVER;
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
                "仓 库 选 装",
                flow::style(&fonts, 28.0, theme::TASK_GOLD),
            ));

            // 左右双列：左「仓库」= 全量物资池；右「背包」= 已携带清单。
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
                // 左：仓库池
                cols.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|wa| {
                    wa.spawn(TextBundle::from_section(
                        "仓 库",
                        flow::style(&fonts, 20.0, accent),
                    ));
                    for (i, name) in MVP_ITEMS.iter().enumerate() {
                        wa.spawn((
                            ArsenalRow { index: i },
                            Interaction::default(),
                            NodeBundle {
                                style: Style {
                                    width: Val::Px(240.0),
                                    height: Val::Px(40.0),
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                background_color: row_hover_bg.into(),
                                ..default()
                            },
                        ))
                        .with_children(|r| {
                            r.spawn((
                                RowLabel,
                                TextBundle::from_section(
                                    format!("○ {name}"),
                                    flow::style(&fonts, 22.0, Color::WHITE),
                                ),
                            ));
                        });
                    }
                });

                // 右：背包（已携带）
                cols.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|bk| {
                    bk.spawn(TextBundle::from_section(
                        "背 包",
                        flow::style(&fonts, 20.0, accent),
                    ));
                    bk.spawn((
                        BackpackCapacity,
                        TextBundle::from_section(
                            format!("已携带 0 / {}", MVP_ITEMS.len()),
                            flow::style(&fonts, 16.0, theme::TEXT_DIM),
                        ),
                    ));
                    for (i, name) in MVP_ITEMS.iter().enumerate() {
                        bk.spawn((
                            BackpackRow { index: i },
                            Interaction::default(),
                            NodeBundle {
                                visibility: Visibility::Hidden,
                                style: Style {
                                    width: Val::Px(240.0),
                                    height: Val::Px(40.0),
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                background_color: row_hover_bg.into(),
                                ..default()
                            },
                        ))
                        .with_children(|r| {
                            r.spawn((
                                RowLabel,
                                TextBundle::from_section(
                                    name.to_string(),
                                    flow::style(&fonts, 22.0, Color::WHITE),
                                ),
                            ));
                        });
                    }
                });
            });

            o.spawn((
                ConfirmEntryButton,
                Interaction::default(),
                NodeBundle {
                    style: Style {
                        width: Val::Px(508.0),
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
                    "确认进入训练场",
                    flow::style(&fonts, 22.0, Color::WHITE),
                ));
            });
        });
}

/// 浮层交互：显隐、仓库行勾选、背包行取消、确认进场（上报选装 + 请求进场）。
#[allow(clippy::too_many_arguments)]
pub fn arsenal_interaction(
    out: Res<NetOut>,
    mut next_state: ResMut<NextState<AppState>>,
    mut vis: ResMut<ArsenalVisible>,
    mut selection: ResMut<ArsenalSelection>,
    mut root_q: Query<&mut Visibility, With<ArsenalRoot>>,
    mut rows: Query<(&ArsenalRow, &Interaction, &Children, &mut BackgroundColor)>,
    mut backpack: Query<
        (&BackpackRow, &Interaction, &Children, &mut Visibility, &mut BackgroundColor),
        (Without<ArsenalRow>, Without<ArsenalRoot>),
    >,
    mut capacity: Query<&mut Text, (With<BackpackCapacity>, Without<RowLabel>)>,
    confirm: Query<(&Interaction, &Children), (With<ConfirmEntryButton>, Changed<Interaction>)>,
    mut labels: Query<(&mut Text, &RowLabel)>,
) {
    // 1) 浮层显隐跟随资源。
    if let Ok(mut visibility) = root_q.get_single_mut() {
        *visibility = if vis.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    // 2) 左「仓库」行点击切换携带。
    for (row, interaction, _, _) in &rows {
        if *interaction == Interaction::Pressed {
            let name = MVP_ITEMS[row.index].to_string();
            match selection.0.iter().position(|c| *c == name) {
                Some(pos) => {
                    selection.0.remove(pos);
                }
                None => selection.0.push(name),
            }
        }
    }

    // 3) 右「背包」行点击：取消对应携带。
    for (row, interaction, _, _, _) in &backpack {
        if *interaction == Interaction::Pressed {
            let name = MVP_ITEMS[row.index].to_string();
            if let Some(pos) = selection.0.iter().position(|c| *c == name) {
                selection.0.remove(pos);
            }
        }
    }

    // 4) 刷新左「仓库」行高亮（背景色 + 行首勾选标记）。
    for (row, _, children, mut bg) in &mut rows {
        let selected = selection.0.iter().any(|c| *c == MVP_ITEMS[row.index]);
        *bg = if selected {
            Color::srgb(0.55, 0.46, 0.16).into()
        } else {
            Color::srgb(0.20, 0.22, 0.25).into()
        };
        let mark = if selected { "●" } else { "○" };
        for child in children {
            if let Ok((mut text, _)) = labels.get_mut(*child) {
                if !text.sections.is_empty() {
                    text.sections[0].value = format!("{mark} {}", MVP_ITEMS[row.index]);
                }
            }
        }
    }

    // 5) 刷新右「背包」行：仅在携带态显示、置底/隐藏切换；并更新容量计数。
    for (row, _, children, mut v, mut bg) in &mut backpack {
        let carried = selection.0.iter().any(|c| *c == MVP_ITEMS[row.index]);
        *v = if carried { Visibility::Visible } else { Visibility::Hidden };
        *bg = if carried {
            Color::srgb(0.28, 0.40, 0.30).into()
        } else {
            Color::srgb(0.20, 0.22, 0.25).into()
        };
        for child in children {
            if let Ok((mut text, _)) = labels.get_mut(*child) {
                if !text.sections.is_empty() {
                    text.sections[0].value = MVP_ITEMS[row.index].to_string();
                }
            }
        }
    }
    if let Ok(mut text) = capacity.get_single_mut() {
        if !text.sections.is_empty() {
            text.sections[0].value =
                format!("已携带 {} / {}", selection.0.len(), MVP_ITEMS.len());
        }
    }

    // 6) 确认进场。
    for (interaction, _) in &confirm {
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