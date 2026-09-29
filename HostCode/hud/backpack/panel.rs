//! HUD 背包面板（按 Tab 打开）：双武器槽 + 弹药池 + 4×3 补给品格位
//!
//! 设计动机（Why，对齐 legacy 0.3.2 操作表第 10 行「背包 · Tab · 打开背包（双武器 /
//! 弹药池 / 补给品）」）：背包此前只能经 3/4 速用与物资箱面板间接看到，缺一块"看全局"
//! 的总览面板。本面板把服务端权威快照里的三份数据一次摆开——两把武器槽（元素 + 弹夹）、
//! 备用弹药池、12 格补给品——并用 `R` 直接使用鼠标悬停的消耗品。
//!
//! 服务器权威（Why）：面板只读 `EntitySnapshot`（背包格位/武器元素/弹药/池量均为服务端裁决
//! 值），使用物品也只上报 `PlayerInput.use_slot` 意图，"这一格是什么、用了扣多少/回多少"
//! 仍由服务端 `combat::use_item_at` 按格位内容裁决——客户端不本地扣减。
//!
//! 与物资箱面板（[`crate::hud::loot::panel`]）职责分离：后者是"容器 ↔ 背包"的双向搬运，
//! 本面板是"本人装备总览 + 速用"，二者互不重叠。

use bevy::prelude::*;
use cute_of_duty_contract::element::ElementType;
use cute_of_duty_contract::net::protocol::{ClientMessage, PlayerInput};

use crate::flow::flow_state::{self as flow, AimRig, CjkFont, LocalPlayer, SeqCounter};
use crate::net::network::NetOut;
use crate::net::snapshot::SnapshotBuffer;
use crate::shared::theme;

/// 背包网格列数 / 行数（4×3，与服务端 `BACKPACK_SLOTS` 同口径）。
const GRID_COLS: usize = 4;
const GRID_ROWS: usize = 3;
/// 背包格位数。
const BP_SLOTS: usize = GRID_COLS * GRID_ROWS;
/// 单格边长与格间距（px）。
const CELL: f32 = 76.0;
const CELL_GAP: f32 = 6.0;

/// 背包面板运行时状态（纯表现层编排）。
#[derive(Resource, Default)]
pub struct BackpackPanelState {
    /// 面板是否打开。
    pub open: bool,
    /// 鼠标悬停的背包格下标（`None` = 未悬停任何格）。
    pub hovered: Option<usize>,
}

/// 面板整屏根节点。
#[derive(Component)]
pub struct BackpackPanelRoot;
/// 背包格位（`index`：0..12）。
#[derive(Component)]
pub struct BpCell {
    pub index: usize,
}
/// 格位内物品名文本。
#[derive(Component)]
pub struct BpCellText {
    pub index: usize,
}
/// 双武器槽概览文本。
#[derive(Component)]
pub struct BpWeaponText;
/// 备用弹药池文本。
#[derive(Component)]
pub struct BpAmmoText;
/// 底部操作提示文本。
#[derive(Component)]
pub struct BpHintText;

/// 装配背包面板（标题 + 左「武器/弹药」右「4×3 补给品」+ 操作提示，默认隐藏）。
pub fn spawn_backpack_panel(p: &mut ChildBuilder<'_>, fonts: &CjkFont) {
    p.spawn((
        BackpackPanelRoot,
        (
            Node {
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
            BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.62)),
            Visibility::Hidden,
        ),
    ))
    .with_children(|root| {
        root.spawn(flow::text(
            "背 包",
            flow::style(fonts, 26.0, theme::TASK_GOLD),
        ));
        root.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(48.0),
            align_items: AlignItems::FlexStart,
            ..default()
        })
        .with_children(|cols| {
            // 左列：双武器槽 + 弹药池概览。
            cols.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                align_items: AlignItems::FlexStart,
                ..default()
            })
            .with_children(|col| {
                col.spawn(flow::text(
                    "武器",
                    flow::style(fonts, 18.0, theme::ACCENT_CYAN),
                ));
                col.spawn((
                    BpWeaponText,
                    flow::text("", flow::style(fonts, 15.0, theme::TEXT_WHITE)),
                ));
                col.spawn(flow::text(
                    "备用弹药",
                    flow::style(fonts, 18.0, theme::ACCENT_CYAN),
                ));
                col.spawn((
                    BpAmmoText,
                    flow::text("", flow::style(fonts, 15.0, theme::TEXT_WHITE)),
                ));
            });
            // 右列：4×3 补给品格位。
            cols.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|col| {
                col.spawn(flow::text(
                    "补给品",
                    flow::style(fonts, 18.0, theme::ACCENT_CYAN),
                ));
                col.spawn(Node {
                    width: Val::Px(GRID_COLS as f32 * CELL + (GRID_COLS - 1) as f32 * CELL_GAP),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    row_gap: Val::Px(CELL_GAP),
                    column_gap: Val::Px(CELL_GAP),
                    ..default()
                })
                .with_children(|grid| {
                    for index in 0..BP_SLOTS {
                        grid.spawn((
                            BpCell { index },
                            Interaction::default(),
                            (
                                Node {
                                    width: Val::Px(CELL),
                                    height: Val::Px(CELL),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.10, 0.12, 0.16, 0.9)),
                                BorderColor(theme::PANEL_BORDER),
                            ),
                        ))
                        .with_children(|cell| {
                            cell.spawn((
                                BpCellText { index },
                                flow::text("空", flow::style(fonts, 12.0, theme::TEXT_DIM)),
                            ));
                        });
                    }
                });
            });
        });
        root.spawn((
            BpHintText,
            flow::text("", flow::style(fonts, 14.0, theme::TEXT_DIM)),
        ));
    });
}

/// Tab 打开 / Tab 或 Esc 关闭背包面板（仅切换可见性，格位随快照每帧刷新）。
pub fn backpack_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<BackpackPanelState>,
    mut root: Query<&mut Visibility, With<BackpackPanelRoot>>,
) {
    let toggle = if state.open {
        keys.just_pressed(KeyCode::Tab) || keys.just_pressed(KeyCode::Escape)
    } else {
        keys.just_pressed(KeyCode::Tab)
    };
    if !toggle {
        return;
    }
    state.open = !state.open;
    if !state.open {
        state.hovered = None;
    }
    if let Ok(mut vis) = root.get_single_mut() {
        *vis = if state.open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// `R` 使用鼠标悬停的背包物品：上报 `PlayerInput.use_slot`（服务端按格位内容权威裁决）。
///
/// 设计动机（Why）：背包打开期间 `gameplay_input_active` 为假、常规 `input_system` 停摆，
/// 故由本系统直接发一条只带 `use_slot` 的输入意图——视角朝向仍取本地 `AimRig`（与上行同源），
/// 使手雷按当前朝向抛出；其余字段置零（面板打开时本就不应有移动/开火意图）。
pub fn backpack_use_hovered(
    keys: Res<ButtonInput<KeyCode>>,
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    rig: Res<AimRig>,
    state: Res<BackpackPanelState>,
    mut seq: ResMut<SeqCounter>,
    out: Res<NetOut>,
) {
    if !state.open || !keys.just_pressed(KeyCode::KeyR) {
        return;
    }
    let Some(index) = state.hovered else { return };
    // 仅当该格确有物品时才上报（空格静默忽略，避免无谓噪音）。
    let occupied = snap
        .current
        .iter()
        .find(|e| e.entity_id == player.entity_id)
        .and_then(|e| e.backpack.as_ref())
        .and_then(|bp| bp.get(index))
        .map(|s| s.is_some())
        .unwrap_or(false);
    if !occupied {
        return;
    }
    seq.0 += 1;
    let input = PlayerInput {
        seq: seq.0,
        use_slot: Some(index as u8),
        aim_yaw: rig.yaw,
        aim_pitch: rig.pitch,
        ..default()
    };
    let _ = out.0.send(ClientMessage::Input { player: input });
}

/// 每帧刷新面板：可见性、格内物品名（含堆叠数量）、悬停高亮、武器/弹药池概览、操作提示。
#[allow(clippy::type_complexity)]
pub fn sync_backpack_panel(
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    mut state: ResMut<BackpackPanelState>,
    mut root: Query<&mut Visibility, With<BackpackPanelRoot>>,
    mut cells: Query<(
        &BpCell,
        &Interaction,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
    mut texts: ParamSet<(
        Query<(&BpCellText, &mut Text, &mut TextColor)>,
        Query<&mut Text, With<BpWeaponText>>,
        Query<&mut Text, With<BpAmmoText>>,
        Query<&mut Text, With<BpHintText>>,
    )>,
) {
    if let Ok(mut vis) = root.get_single_mut() {
        *vis = if state.open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !state.open {
        state.hovered = None;
        return;
    }

    let me = snap.current.iter().find(|e| e.entity_id == player.entity_id);

    // 悬停格：读各格 `Interaction`，记录当前悬停下标并高亮。
    let hovered = cells
        .iter_mut()
        .find(|(_, it, _, _)| **it == Interaction::Hovered)
        .map(|(c, _, _, _)| c.index);
    state.hovered = hovered;
    for (cell, _, mut bg, mut border) in &mut cells {
        let active = Some(cell.index) == hovered;
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

    // 格内物品名（数量 > 1 时带 `×N`）。
    let backpack = me.and_then(|e| e.backpack.as_deref()).unwrap_or(&[]);
    for (cell, mut text, mut color) in &mut texts.p0() {
        match backpack.get(cell.index).and_then(|s| s.as_ref()) {
            Some(item) => {
                text.0 = item.display_label();
                color.0 = theme::TEXT_WHITE;
            }
            None => {
                text.0 = "空".to_string();
                color.0 = theme::TEXT_DIM;
            }
        }
    }

    // 双武器槽概览：手持槽以 `▸` 标注，弹夹数（服务端只下发挥当前手持槽）仅随手持槽显示。
    if let Ok(mut t) = texts.p1().get_single_mut() {
        t.0 = match me {
            Some(e) => match e.weapon_elements {
                Some([a, b]) => {
                    let line = |slot: u8, el: ElementType| {
                        let mark = if e.active_slot == slot { "▸" } else { " " };
                        let ammo = if e.active_slot == slot {
                            format!("  {}/{}", e.ammo, e.ammo_max)
                        } else {
                            String::new()
                        };
                        format!("{mark} 槽{} [{}]{ammo}", slot + 1, element_name(el))
                    };
                    format!("{}\n{}", line(0, a), line(1, b))
                }
                None => "（无武器）".to_string(),
            },
            None => "（等待快照）".to_string(),
        };
    }

    // 备用弹药总量（弹池 + 背包弹药合计，服务端权威合计）。
    if let Ok(mut t) = texts.p2().get_single_mut() {
        let reserve = me.map(|e| e.ammo_reserve).unwrap_or(-1);
        t.0 = if reserve < 0 {
            "—".to_string()
        } else {
            format!("备用 {reserve}")
        };
    }

    if let Ok(mut t) = texts.p3().get_single_mut() {
        t.0 =
            "鼠标悬停物品 · 按 R 使用 · Tab / Esc 关闭".to_string();
    }
}

/// 元素中文名（表现层冷映射）。
fn element_name(e: ElementType) -> &'static str {
    match e {
        ElementType::Fire => "火",
        ElementType::Ice => "冰",
        ElementType::Electric => "电",
        ElementType::Poison => "毒",
        ElementType::Physical => "物",
        ElementType::Water => "水",
    }
}

/// 离开训练场时复位面板（否则下次进场会带着"已打开"状态冻结输入）。
pub fn reset_backpack_panel(mut state: ResMut<BackpackPanelState>) {
    *state = BackpackPanelState::default();
}