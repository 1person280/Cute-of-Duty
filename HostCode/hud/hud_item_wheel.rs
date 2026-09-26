//! HUD 消耗品径向轮盘：**长按 3/4 呼出、滚轮/鼠标方向选格、松开速用；短按直接速用首件**
//!
//! 设计动机（Why）：3/4 号槽不再是"固定的医疗包/手雷"两个硬编码位，而是**按类别索引进
//! 背包**——服务端权威的 4×3 背包里凡是恢复类（[`ItemCategory::Consumable`]）即可用 3 调用，
//! 战术类（[`ItemCategory::Tactical`]）用 4 调用。客户端只做"选哪一格"的表现与交互，
//! "用了扣什么/回多少"由服务端按该格内容裁决（见 `combat::use_item_at`）。
//!
//! 交互还原 legacy `item_wheel`：
//! - **短按**（< [`WHEEL_OPEN_DELAY`]）= 速用该类**首件**；
//! - **长按** = 呼出径向轮盘，鼠标离中心方向决定高亮扇区，靠中心死区 = 松开取消；
//! - **松开** = 把选中格下标写入 [`ItemWheelState::pending_slot`]，由 `net::input_system`
//!   折进 `PlayerInput.use_slot` 上报（客户端不本地扣减库存）。
//!
//! 门控：轮盘打开期间由 `gameplay_input_active` 冻结移动/开火与鼠标视角，避免"边选边打"。

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use cute_of_duty_server::items::ItemCategory;

use crate::flow::flow_state::{self as flow, CjkFont, LocalPlayer};
use crate::net::snapshot::SnapshotBuffer;
use crate::shared::theme;

/// 长按超过该秒数即呼出轮盘（低于此值视为短按速用）。
const WHEEL_OPEN_DELAY: f32 = 0.25;
/// 中心死区半径（px）：光标离中心小于它时视为"取消选择"。
const DEAD_ZONE: f32 = 34.0;
/// 扇区环绕半径（px）。
const RING_RADIUS: f32 = 118.0;
/// 单个扇区卡片尺寸（px）。
const SECTOR_W: f32 = 108.0;
const SECTOR_H: f32 = 34.0;
/// 预生成的扇区节点数（覆盖 4×3 背包内同类物品的理论上限）。
const MAX_SECTORS: usize = 12;

/// 轮盘运行时状态（纯表现层编排，非权威数据）。
#[derive(Resource)]
pub struct ItemWheelState {
    /// 当前按住的键（3 = 恢复类、4 = 战术类；`None` = 未按住）。
    pub held_key: Option<u8>,
    /// 已按住时长（秒）。
    pub held: f32,
    /// 轮盘是否已呼出。
    pub open: bool,
    /// 该类别在背包中的格位下标（升序）。
    pub slots: Vec<usize>,
    /// 与 `slots` 一一对应的物品展示名。
    pub labels: Vec<String>,
    /// 当前高亮扇区下标。
    pub selected: usize,
    /// 光标处于中心死区（松开会取消本次使用）。
    pub cancel: bool,
    /// 打开期间累计的鼠标位移（决定方向）。
    pub cursor: Vec2,
    /// 待上报的背包格下标（由 `net::input_system` 消费后清空）。
    pub pending_slot: Option<u8>,
}

impl Default for ItemWheelState {
    fn default() -> Self {
        Self {
            held_key: None,
            held: 0.0,
            open: false,
            slots: Vec::new(),
            labels: Vec::new(),
            selected: 0,
            cancel: false,
            cursor: Vec2::ZERO,
            pending_slot: None,
        }
    }
}

/// 轮盘整屏根节点。
#[derive(Component)]
pub struct ItemWheelRoot;
/// 第 `i` 个扇区卡片节点。
#[derive(Component)]
pub struct ItemWheelSector(pub usize);
/// 第 `i` 个扇区卡片内的物品名文本。
#[derive(Component)]
pub struct ItemWheelSectorText(pub usize);
/// 中心卡片的物品名文本。
#[derive(Component)]
pub struct ItemWheelCenterText;
/// 顶部键位提示文本。
#[derive(Component)]
pub struct ItemWheelKeyText;

/// 装配径向轮盘（整屏遮罩 + 顶部键位提示 + 中心卡片 + N 个扇区卡片，默认隐藏）。
pub fn spawn_item_wheel(p: &mut ChildBuilder<'_>, fonts: &CjkFont) {
    p.spawn((
        ItemWheelRoot,
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            background_color: Color::srgba(0.02, 0.03, 0.05, 0.35).into(),
            visibility: Visibility::Hidden,
            ..default()
        },
    ))
    .with_children(|root| {
        // 顶部键位提示（水平居中）。
        root.spawn((
            ItemWheelKeyText,
            TextBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Px(88.0),
                    margin: UiRect {
                        left: Val::Px(-160.0),
                        ..default()
                    },
                    width: Val::Px(320.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                text: Text::from_section("", flow::style(fonts, 18.0, theme::TEXT_WHITE)),
                ..default()
            },
        ));

        // 中心卡片（屏幕正中，显示当前高亮物品名）。
        root.spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-90.0),
                    top: Val::Px(-32.0),
                    ..default()
                },
                width: Val::Px(180.0),
                height: Val::Px(64.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            background_color: theme::PANEL_BG.into(),
            border_color: BorderColor(theme::ACCENT_AMBER),
            ..default()
        })
        .with_children(|c| {
            c.spawn((
                ItemWheelCenterText,
                TextBundle::from_section("", flow::style(fonts, 17.0, theme::ACCENT_AMBER)),
            ));
        });

        // 扇区卡片：一次性生成上限数量，运行时按需显示与定位。
        for i in 0..MAX_SECTORS {
            root.spawn((
                ItemWheelSector(i),
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(50.0),
                        top: Val::Percent(50.0),
                        width: Val::Px(SECTOR_W),
                        height: Val::Px(SECTOR_H),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        display: Display::None,
                        ..default()
                    },
                    background_color: Color::srgba(0.10, 0.12, 0.16, 0.9).into(),
                    border_color: BorderColor(theme::PANEL_BORDER),
                    ..default()
                },
            ))
            .with_children(|s| {
                s.spawn((
                    ItemWheelSectorText(i),
                    TextBundle::from_section("", flow::style(fonts, 14.0, theme::TEXT_WHITE)),
                ));
            });
        }
    });
}

/// 轮盘键鼠输入：长按呼出、方向选格、松开速用/取消；短按速用首件。
///
/// 系统参数刻意包含各模态资源：其它模态打开时（暂停/全景图/交互面板/物资箱）忽略轮盘输入，
/// 但若轮盘**已经**处于打开态则继续处理，避免中途弹窗导致轮盘卡在屏幕上。
#[allow(clippy::too_many_arguments)]
pub fn item_wheel_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut motion: EventReader<MouseMotion>,
    time: Res<Time>,
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    pause: Res<crate::menu::PauseMenu>,
    bigmap: Res<crate::hud::BigMapOpen>,
    interact: Res<crate::hud::InteractState>,
    loot: Res<crate::hud::LootPanelState>,
    mut state: ResMut<ItemWheelState>,
) {
    // 先消费本帧鼠标事件（即使被门控丢弃），避免门控解除后第一帧突然跳到选。
    let delta: Vec2 = motion.read().map(|e| e.delta).fold(Vec2::ZERO, |a, b| a + b);

    let blocked = *pause != crate::menu::PauseMenu::Closed
        || bigmap.0
        || interact.panel_open
        || loot.open;
    if !state.open && blocked {
        return;
    }

    // 起始：按下 3 / 4 记录本次会话。
    if state.held_key.is_none() {
        if keys.just_pressed(KeyCode::Digit3) {
            state.held_key = Some(3);
            state.held = 0.0;
        } else if keys.just_pressed(KeyCode::Digit4) {
            state.held_key = Some(4);
            state.held = 0.0;
        }
    }

    let Some(key) = state.held_key else {
        return;
    };
    state.held += time.delta_seconds();
    let category = if key == 3 {
        ItemCategory::Consumable
    } else {
        ItemCategory::Tactical
    };

    if state.open {
        // 打开中：累计方向 → 选扇区；靠中心 = 取消。
        state.cursor += delta;
        let n = state.slots.len();
        if n > 0 {
            if state.cursor.length() < DEAD_ZONE {
                state.cancel = true;
            } else {
                state.cancel = false;
                let step = std::f32::consts::TAU / n as f32;
                let ang = state.cursor.y.atan2(state.cursor.x);
                let rel = (ang + std::f32::consts::FRAC_PI_2)
                    .rem_euclid(std::f32::consts::TAU);
                state.selected = (((rel + step * 0.5) / step).floor() as usize) % n;
            }
        }
    } else if state.held >= WHEEL_OPEN_DELAY {
        // 达到长按阈值：固化该类别的背包格与物品名，再决定是否真的呼出。
        let (slots, labels) = category_slots(&snap, &player, category);
        state.slots = slots;
        state.labels = labels;
        state.open = !state.slots.is_empty();
        state.selected = 0;
        state.cancel = false;
        state.cursor = Vec2::ZERO;
    }

    // 松开：打开态 → 用选中格（中心死区则取消）；未打开 → 短按速用首件。
    let released = if key == 3 {
        keys.just_released(KeyCode::Digit3)
    } else {
        keys.just_released(KeyCode::Digit4)
    };
    if released {
        if state.open {
            if !state.cancel {
                let picked = state.slots.get(state.selected).map(|i| *i as u8);
                state.pending_slot = picked;
            }
        } else {
            state.pending_slot = category_slots(&snap, &player, category)
                .0
                .first()
                .map(|i| *i as u8);
        }
        *state = ItemWheelState::default();
    }
}

/// 采集某速用类别在背包中的（格位下标, 物品名）列表（升序）。
fn category_slots(
    snap: &SnapshotBuffer,
    player: &LocalPlayer,
    category: ItemCategory,
) -> (Vec<usize>, Vec<String>) {
    let mut slots = Vec::new();
    let mut labels = Vec::new();
    if let Some(me) = snap.current.iter().find(|e| e.entity_id == player.entity_id) {
        if let Some(bp) = &me.backpack {
            for (i, slot) in bp.iter().enumerate() {
                if let Some(item) = slot {
                    if item.kind.category() == Some(category) {
                        slots.push(i);
                        labels.push(item.label.clone());
                    }
                }
            }
        }
    }
    (slots, labels)
}

/// 每帧刷新轮盘绘制：根可见性、各扇区位置/高亮/文案、中心卡片与键位提示。
#[allow(clippy::type_complexity)]
pub fn update_item_wheel(
    state: Res<ItemWheelState>,
    mut root: Query<&mut Visibility, With<ItemWheelRoot>>,
    mut sectors: Query<(&ItemWheelSector, &mut Style, &mut BackgroundColor, &mut BorderColor)>,
    mut sec_text: Query<
        (&ItemWheelSectorText, &mut Text),
        (Without<ItemWheelCenterText>, Without<ItemWheelKeyText>),
    >,
    mut center: Query<
        &mut Text,
        (With<ItemWheelCenterText>, Without<ItemWheelSectorText>, Without<ItemWheelKeyText>),
    >,
    mut key_text: Query<
        &mut Text,
        (With<ItemWheelKeyText>, Without<ItemWheelSectorText>, Without<ItemWheelCenterText>),
    >,
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

    let n = state.slots.len();
    for (sector, mut style, mut bg, mut border) in &mut sectors {
        if sector.0 >= n {
            style.display = Display::None;
            continue;
        }
        style.display = Display::Flex;
        // 0 号扇区置于正上方，顺时针均分圆周。
        let step = std::f32::consts::TAU / n as f32;
        let ang = -std::f32::consts::FRAC_PI_2 + step * sector.0 as f32;
        style.margin.left = Val::Px(ang.cos() * RING_RADIUS - SECTOR_W / 2.0);
        style.margin.top = Val::Px(ang.sin() * RING_RADIUS - SECTOR_H / 2.0);

        let selected = sector.0 == state.selected && !state.cancel;
        *bg = if selected {
            theme::ROW_HOVER.into()
        } else {
            Color::srgba(0.10, 0.12, 0.16, 0.9).into()
        };
        *border = BorderColor(if selected {
            theme::ACCENT_AMBER
        } else {
            theme::PANEL_BORDER
        });
    }

    for (sector, mut text) in &mut sec_text {
        text.sections[0].value = state.labels.get(sector.0).cloned().unwrap_or_default();
        let selected = sector.0 == state.selected && !state.cancel;
        text.sections[0].style.color = if selected {
            theme::TEXT_WHITE
        } else {
            theme::TEXT_DIM
        };
    }

    if let Ok(mut t) = center.get_single_mut() {
        t.sections[0].value = if state.cancel {
            "松开取消".to_string()
        } else {
            state.labels.get(state.selected).cloned().unwrap_or_default()
        };
    }

    if let Ok(mut t) = key_text.get_single_mut() {
        let cat = if state.held_key == Some(3) {
            "3 恢复类"
        } else {
            "4 战术类"
        };
        t.sections[0].value = format!("{cat} · 滚轮/方向选择 · 松开使用");
    }
}

/// 离开训练场时复位轮盘（否则下次进场可能带着"打开中"冻结输入）。
pub fn reset_item_wheel(mut state: ResMut<ItemWheelState>) {
    *state = ItemWheelState::default();
}
