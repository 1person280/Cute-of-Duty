//! HUD 交互：**就近常显的附近列表**（滚轮翻页 + F/回车确认）+ 站点二级选项面板
//!
//! 设计动机（Why）：交互是"服务端算、客户端看"的典型场景——客户端只负责①根据快照里
//! 服务端下发的 `interact` 信息列出**附近可交互目标**；②把玩家的选择（拾取 / 领取哪项
//! 补给 / 开箱 / 切干员）作为意图上报。距离校验、效果发放、是否消耗实体全部由服务端
//! `interact::settle` 裁决，客户端不参与判定。
//!
//! 还原老版形态（legacy `demo/inventory/interact_menu.rs`）：
//! - **就近常显**：只要有目标进入 `INTERACT_RANGE` 就自动列出（不需先按 F，也不阻塞游玩）；
//! - **滚轮翻页**：滚轮改高亮项（环绕），窗口 `scroll_start` 跟随，右侧滚动条示位置；
//! - **F/回车确认**：拾取物 → 直接 `Take`；干员台 → 转出二级选项面板；物资箱/补给台 →
//!   打开双向 4×3 格位面板（见 `hud_loot_panel`，拖拽 / Shift+左键搬运）。
//! - **二级面板 Esc**：返回就近列表。
//!
//! 门控：仅**二级面板 / 物资箱面板 / 径向轮盘**打开时冻结玩法输入；就近列表本身不冻结，
//! 玩家可边跑边看列表（这也是老版手感的关键区别）。

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use cute_of_duty_server::interact::{InteractChoice, InteractKind, INTERACT_RANGE};
use cute_of_duty_server::map::StationKind;
use cute_of_duty_server::net::protocol::ClientMessage;
use cute_of_duty_server::operator::roster;

use crate::flow::flow_state::{self as flow, CjkFont, LocalPlayer};
use crate::net::network::NetOut;
use crate::net::snapshot::SnapshotBuffer;
use crate::shared::operator_meta::meta;
use crate::shared::theme;

use super::hud_item_wheel::ItemWheelState;
use super::hud_loot_panel::{open_loot_panel, open_supply_panel, LootPanelState};

/// 列表可见行数（老版 `INTERACT_MENU_VISIBLE_ROWS` 的量级）。
pub const INTERACT_VISIBLE_ROWS: usize = 5;
/// 行高 / 行间距（px，列表与滚动条共用一套几何）。
pub const INTERACT_ROW_H: f32 = 26.0;
pub const INTERACT_ROW_GAP: f32 = 4.0;
/// 列表最多列出的附近目标数（超出靠滚轮翻页）。
const MAX_ENTRIES: usize = 8;

/// 附近可交互条目：目标 ID + 展示名 + 语义（决定 F 确认后走哪条链路）。
#[derive(Clone)]
pub struct InteractEntry {
    pub id: u64,
    pub label: String,
    pub kind: InteractKind,
}

/// 交互面板的运行时状态（客户端表现层编排，非权威数据）。
///
/// `entries` 每帧由快照对账刷新；`selected/scroll_start` 由滚轮维护；
/// `panel_open/target/options/option_selected/pending` 为二级面板与上报队列。
#[derive(Resource, Default)]
pub struct InteractState {
    /// 附近可交互条目（站点优先 + 拾取物按距离升序，已截断）。
    pub entries: Vec<InteractEntry>,
    /// 列表当前高亮项下标。
    pub selected: usize,
    /// 列表首行窗口偏移（滚轮翻页时跟随 `selected`）。
    pub scroll_start: usize,
    /// 站点二级选项面板是否展开（展开时冻结玩法输入）。
    pub panel_open: bool,
    /// 二级面板目标实体 ID。
    pub target: u64,
    /// 二级面板标题（展示名）。
    pub title: String,
    /// 二级面板选项列表。
    pub options: Vec<InteractOption>,
    /// 二级面板当前高亮项下标。
    pub option_selected: usize,
    /// 待上报的动作（由输入/点击系统置位，由 `interact_commit` 发送后清空）。
    pub pending: Option<(u64, InteractAction)>,
}

/// 菜单选项：一项人类可读文案 + 一项`客户端→服务端`意图。
#[derive(Clone)]
pub struct InteractOption {
    pub label: String,
    pub action: InteractAction,
}

/// 菜单选项对应的上行意图（交互 or 切干员）。
#[derive(Clone)]
pub enum InteractAction {
    /// 交互意图（拾取 / 领取补给 / 开箱）
    Interact(InteractChoice),
    /// 切换干员（干员切换台使用既有的权威切干员链路）
    SwitchOperator(u32),
}

/// 常驻接近列表根节点（无附近目标时隐藏）。
#[derive(Component)]
pub struct InteractPanel;
/// 列表行容器框（按高亮态改底色/边框）。
#[derive(Component)]
pub struct InteractRowSlot(pub usize);
/// 列表行文本（显示 `entries[scroll_start + j]`）。
#[derive(Component)]
pub struct InteractRowText(pub usize);
/// 滚动条滑块。
#[derive(Component)]
pub struct InteractScrollThumb;
/// 列表底部提示行（翻页页码）。
#[derive(Component)]
pub struct InteractHintText;

/// 二级选项面板整屏遮罩根节点。
#[derive(Component)]
pub struct InteractMenuRoot;
/// 二级面板标题。
#[derive(Component)]
pub struct InteractMenuTitle;
/// 二级面板选项容器（选项行随（目标,高亮）变化重建）。
#[derive(Component)]
pub struct InteractMenuList;
/// 选项行 → 选项下标（供鼠标点击选择）。
#[derive(Component)]
pub struct InteractOptionIndex(pub usize);

/// 装配就近交互列表（**常显小面板**，无附近目标时隐藏）与站点二级选项面板（默认隐藏）。
pub fn spawn_interact_ui(p: &mut ChildBuilder<'_>, fonts: &CjkFont) {
    // ---- 就近列表：小面板贴在准星右下侧，**不铺满全屏、不加整屏遮罩**（避免常显时糊住画面） ----
    p.spawn((
        InteractPanel,
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Percent(55.0),
                top: Val::Percent(40.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(INTERACT_ROW_GAP),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            background_color: Color::srgba(0.04, 0.06, 0.09, 0.72).into(),
            visibility: Visibility::Hidden,
            ..default()
        },
    ))
    .with_children(|panel| {
        panel.spawn(TextBundle::from_section(
            "附近可交互",
            flow::style(fonts, 20.0, theme::TEXT_WHITE),
        ));
        // 行区（左） + 滚动条（右）并排
        panel
            .spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(6.0),
                    align_items: AlignItems::FlexStart,
                    ..default()
                },
                ..default()
            })
            .with_children(|body| {
                body.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(INTERACT_ROW_GAP),
                        min_width: Val::Px(260.0),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|list| {
                    for j in 0..INTERACT_VISIBLE_ROWS {
                        list.spawn((
                            InteractRowSlot(j),
                            NodeBundle {
                                style: Style {
                                    padding: UiRect::axes(Val::Px(10.0), Val::Px(3.0)),
                                    height: Val::Px(INTERACT_ROW_H),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                background_color: theme::PANEL_BG.into(),
                                border_color: BorderColor(theme::PANEL_BORDER),
                                ..default()
                            },
                        ))
                        .with_children(|row| {
                            row.spawn((
                                InteractRowText(j),
                                TextBundle::from_section(
                                    "",
                                    flow::style(fonts, 15.0, theme::TEXT_WHITE),
                                ),
                            ));
                        });
                    }
                });
                // 滚动条：定高轨道 + 按比例定位的滑块
                let track_h = INTERACT_VISIBLE_ROWS as f32 * (INTERACT_ROW_H + INTERACT_ROW_GAP)
                    - INTERACT_ROW_GAP;
                body.spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(4.0),
                        height: Val::Px(track_h),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    background_color: Color::srgba(1.0, 1.0, 1.0, 0.10).into(),
                    ..default()
                })
                .with_children(|track| {
                    track.spawn((
                        InteractScrollThumb,
                        NodeBundle {
                            style: Style {
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                top: Val::Px(0.0),
                                width: Val::Px(4.0),
                                height: Val::Px(track_h),
                                ..default()
                            },
                            background_color: theme::ACCENT_AMBER.into(),
                            ..default()
                        },
                    ));
                });
            });
        panel.spawn((
            InteractHintText,
            TextBundle::from_section(
                "滚轮翻页 · F 确认",
                flow::style(fonts, 13.0, theme::TEXT_DIM),
            ),
        ));
    });

    // ---- 站点二级选项面板：整屏半透明遮罩 + 居中面板（标题 / 选项列表 / 操作提示） ----
    p.spawn((
        InteractMenuRoot,
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
                row_gap: Val::Px(10.0),
                ..default()
            },
            background_color: Color::srgba(0.02, 0.03, 0.05, 0.55).into(),
            visibility: Visibility::Hidden,
            ..default()
        },
    ))
    .with_children(|root| {
        root.spawn((
            InteractMenuTitle,
            TextBundle::from_section("交互", flow::style(fonts, 24.0, theme::TEXT_WHITE)),
        ));
        root.spawn((
            InteractMenuList,
            NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    min_width: Val::Px(260.0),
                    ..default()
                },
                ..default()
            },
        ));
        root.spawn(TextBundle::from_section(
            "滚轮选择 · F 确认 · Esc 关闭",
            flow::style(fonts, 13.0, theme::TEXT_DIM),
        ));
    });
}

/// 每帧刷新附近可交互条目列表（站点优先，其余按距离升序，截断到上限）。
pub fn update_interact_entries(
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    mut state: ResMut<InteractState>,
) {
    let mut found: Vec<(bool, f32, u64, String, InteractKind)> = Vec::new();
    if let Some(me) = snap.current.iter().find(|e| e.entity_id == player.entity_id) {
        for e in &snap.current {
            let Some(info) = &e.interact else { continue };
            let dx = e.x - me.x;
            let dz = e.z - me.z;
            let d = (dx * dx + dz * dz).sqrt();
            if d > INTERACT_RANGE {
                continue;
            }
            let is_pickup = matches!(info.kind, InteractKind::Pickup(_));
            found.push((is_pickup, d, e.entity_id, info.label.clone(), info.kind));
        }
    }
    // 站点（is_pickup=false）优先，同组内按距离升序。
    found.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)));
    found.truncate(MAX_ENTRIES);

    let ids_changed = found.len() != state.entries.len()
        || found
            .iter()
            .zip(state.entries.iter())
            .any(|(a, b)| a.2 != b.id);
    state.entries = found
        .into_iter()
        .map(|(_, _, id, label, kind)| InteractEntry { id, label, kind })
        .collect();

    // 条目集合变化后收敛高亮/窗口，避免越界或高亮漂移到别的目标上。
    if ids_changed {
        state.selected = 0;
        state.scroll_start = 0;
    }
    let n = state.entries.len();
    if n == 0 {
        state.selected = 0;
        state.scroll_start = 0;
    } else {
        state.selected = state.selected.min(n - 1);
        state.scroll_start = state.scroll_start.min(n.saturating_sub(INTERACT_VISIBLE_ROWS));
    }
    // 目标消失时收起二级面板（如物资箱被移出视野）；列表本身随 `entries` 为空自然隐藏。
    if state.panel_open && !state.entries.iter().any(|e| e.id == state.target) {
        close_panel(&mut state);
    }
}

/// 键鼠输入：滚轮移高亮、F/回车确认；二级面板滚轮/确认/Esc 返回。
///
/// 物资箱面板或径向轮盘打开时让位（由各自系统处理键鼠），避免 F/滚轮被重复消费；
/// 但 **Esc 关闭二级面板的优先级高于让位**——否则轮盘/物资箱一旦因异常残留 `open`，
/// 二级面板将无法收起，`gameplay_input_active` 恒假，WASD/开火/3-4 全灭（见 A-1）。
pub fn interact_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut wheel: EventReader<MouseWheel>,
    mut state: ResMut<InteractState>,
    mut loot: ResMut<LootPanelState>,
    wheel_state: Res<ItemWheelState>,
) {
    // 滚轮累计（一次事件批可能多帧滚动）。先读完事件，避免让位时把事件留在队列里。
    let scroll: f32 = wheel.read().map(|e| e.y).sum();
    let step: i32 = if scroll > 0.0 { -1 } else if scroll < 0.0 { 1 } else { 0 };

    // Esc 优先：无论轮盘/物资箱是否打开，二级面板都必须能收起，保证玩法输入可恢复。
    if state.panel_open && keys.just_pressed(KeyCode::Escape) {
        close_panel(&mut state);
        return;
    }

    if loot.open || wheel_state.open {
        return;
    }

    // —— 二级选项面板：滚轮移高亮、F/回车确认（Esc 关闭已在上方统一处理） ——
    if state.panel_open {
        let n = state.options.len();
        if n > 0 && step != 0 {
            state.option_selected =
                (state.option_selected as i32 + step).rem_euclid(n as i32) as usize;
        }
        if keys.just_pressed(KeyCode::KeyF) || keys.just_pressed(KeyCode::Enter) {
            if let Some(opt) = state.options.get(state.option_selected) {
                state.pending = Some((state.target, opt.action.clone()));
            }
        }
        return;
    }

    // —— 就近列表：常显，滚轮翻页 / F 确认；附近无目标时忽略 ——
    let n = state.entries.len();
    if n == 0 {
        return;
    }
    if step != 0 {
        state.selected = (state.selected as i32 + step).rem_euclid(n as i32) as usize;
        // 窗口跟随高亮，保证高亮项始终可见。
        if state.selected < state.scroll_start {
            state.scroll_start = state.selected;
        } else if state.selected >= state.scroll_start + INTERACT_VISIBLE_ROWS {
            state.scroll_start = state.selected + 1 - INTERACT_VISIBLE_ROWS;
        }
    }
    if keys.just_pressed(KeyCode::KeyF) || keys.just_pressed(KeyCode::Enter) {
        confirm_entry(&mut state, &mut loot);
    }
}

/// 鼠标点击选项行 → 直接置位待上报动作（并同步高亮）。
pub fn interact_menu_click(
    mut state: ResMut<InteractState>,
    q: Query<(&Interaction, &InteractOptionIndex), Changed<Interaction>>,
) {
    if !state.panel_open {
        return;
    }
    for (interaction, idx) in &q {
        if *interaction == Interaction::Pressed {
            let Some(action) = state.options.get(idx.0).map(|o| o.action.clone()) else {
                continue;
            };
            state.option_selected = idx.0;
            state.pending = Some((state.target, action));
        }
    }
}

/// 上报待处理动作并收起两级面板（意图由服务端权威结算）。
pub fn interact_commit(mut state: ResMut<InteractState>, out: Res<NetOut>) {
    let Some((target, action)) = state.pending.take() else {
        return;
    };
    let msg = match action {
        InteractAction::Interact(choice) => ClientMessage::Interact { target, choice },
        InteractAction::SwitchOperator(operator_id) => {
            ClientMessage::SwitchOperator { operator_id }
        }
    };
    let _ = out.0.send(msg);
    close_panel(&mut state);
}

/// 离开训练场时复位交互状态（否则下次进场会带着"面板已开"冻结输入）。
pub fn reset_interact(mut state: ResMut<InteractState>) {
    state.entries.clear();
    state.selected = 0;
    state.scroll_start = 0;
    state.pending = None;
    close_panel(&mut state);
}

/// 确认当前高亮条目：拾取物直接拾取；功能站点展开二级选项面板；物资箱打开双向格位面板。
fn confirm_entry(state: &mut InteractState, loot: &mut LootPanelState) {
    let Some(entry) = state.entries.get(state.selected) else { return };
    let (id, title, kind) = (entry.id, entry.label.clone(), entry.kind);
    match kind {
        InteractKind::Pickup(_) => {
            state.pending = Some((id, InteractAction::Interact(InteractChoice::Take)));
        }
        InteractKind::Station(StationKind::SupplyTable) => {
            // 补给台不再走"二级选项菜单"：与物资箱统一为 4×3 格位面板（拖拽领取）。
            open_supply_panel(loot, id);
        }
        InteractKind::Station(StationKind::OperatorDesk) => {
            let options = roster()
                .iter()
                .enumerate()
                .map(|(i, _)| InteractOption {
                    label: format!("切换至 {}", meta(i as u32).name),
                    action: InteractAction::SwitchOperator(i as u32),
                })
                .collect();
            open_panel(state, id, title, options);
        }
        InteractKind::Station(StationKind::SupplyCrate) => {
            // 物资箱不再"开启即拿走"：打开双向 4×3 格位面板逐格搬运（服务端裁决每次转移）。
            open_loot_panel(loot, id);
        }
    }
}

/// 展开二级面板：写入目标/标题/选项并复位高亮。
fn open_panel(state: &mut InteractState, target: u64, title: String, options: Vec<InteractOption>) {
    state.panel_open = true;
    state.target = target;
    state.title = title;
    state.options = options;
    state.option_selected = 0;
}

/// 收起二级面板并清空选项。
fn close_panel(state: &mut InteractState) {
    state.panel_open = false;
    state.title.clear();
    state.options.clear();
    state.option_selected = 0;
}
