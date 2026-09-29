//! HUD 可点击操作按钮组（按 B 打开的模态面板）：把 demo 操作映射为一组可点击按钮。
//!
//! 设计动机（Why）：legacy demo 的操作方式全是键鼠（WASD / 左键 / R / Q / E / 3 / 4…），
//! 缺少"屏幕上点一下就触发"的等价入口（台账 B#1「按钮」缺口）。本面板把每一类 demo 操作
//! 摆成按钮，**鼠标点击即触发**——便于试玩核对与后续接入触屏/手柄。
//!
//! 注入路径（Why）：面板打开时 `gameplay_input_active` 为假、`net::input_system` 被门控冻结，
//! 故本面板**照抄背包面板先例**——在面板内直接构造 `PlayerInput`（seq 取自增、yaw/pitch 取
//! [`AimRig`]）经 [`NetOut`] 上行，绕开被冻结的常规输入系统；**本模块系统不挂
//! `gameplay_input_active`**，否则点不动。所有"算什么"仍由服务端裁决，客户端只报意图。
//!
//! 语义（Why）：移动/疾跑/瞄准为**点按切换**持续标志（鼠标无法长按拖出面板，故取切换而非
//! 按住）；跳跃/射击/换弹/技能/切枪/道具为**边沿动作**（点一下发一帧脉冲）。Esc / Tab 背包 /
//! M 大地图 / 暂停 / 关闭为**纯客户端**（切换本地面板，不上行）。

use bevy::prelude::*;
use cute_of_duty_contract::items::ItemCategory;
use cute_of_duty_contract::net::protocol::{ClientMessage, PlayerInput};

use crate::flow::flow_state::{self as flow, AimRig, CjkFont, LocalPlayer, SeqCounter};
use crate::net::network::NetOut;
use crate::net::snapshot::SnapshotBuffer;
use crate::shared::theme;

/// 按钮面板运行时状态（纯表现层编排；持续动作为切换标志，边沿动作为一次性队列）。
#[derive(Resource, Default)]
pub struct ButtonPanelState {
    /// 面板是否打开。
    pub open: bool,
    /// 持续移动/疾跑/瞄准的切换标志（点按切换，非按住）。
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub sprint: bool,
    pub aim: bool,
    /// 边沿动作（本帧待上报，`button_panel_emit` 消费后清空）。
    pub jump: bool,
    pub shoot: bool,
    pub reload: bool,
    pub skill_q: bool,
    pub skill_e: bool,
    /// 切枪目标槽（`Some(0/1)`）。
    pub weapon_slot: Option<u8>,
    /// 使用背包某格（恢复品/战术品取该类首格）。
    pub use_slot: Option<u8>,
}

impl ButtonPanelState {
    /// 复位全部标志（关闭面板 / 离开训练场时调用，避免"已按下"状态带到下一局）。
    fn reset_flags(&mut self) {
        self.forward = false;
        self.backward = false;
        self.left = false;
        self.right = false;
        self.sprint = false;
        self.aim = false;
        self.jump = false;
        self.shoot = false;
        self.reload = false;
        self.skill_q = false;
        self.skill_e = false;
        self.weapon_slot = None;
        self.use_slot = None;
    }
}

/// 面板整屏根节点。
#[derive(Component)]
pub struct ButtonPanelRoot;

/// 单个操作按钮（`kind` 决定点击行为）。
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    Sprint,
    Aim,
    Jump,
    Shoot,
    Reload,
    SkillQ,
    SkillE,
    Weapon1,
    Weapon2,
    UseConsumable,
    UseTactical,
    OpenBackpack,
    OpenBigMap,
    OpenPause,
    Close,
}

/// 按钮尺寸（px）。
const BTN_W: f32 = 132.0;
const BTN_H: f32 = 46.0;

/// 装配按钮面板（整屏遮罩 + 标题 + 自动换行的按钮组，默认隐藏）。
pub fn spawn_button_panel(p: &mut ChildBuilder<'_>, fonts: &CjkFont) {
    p.spawn((
        ButtonPanelRoot,
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
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.05, 0.68)),
            Visibility::Hidden,
        ),
    ))
    .with_children(|root| {
        root.spawn(flow::text(
            "操作按钮 · 按 B / Esc 关闭",
            flow::style(fonts, 24.0, theme::TASK_GOLD),
        ));
        root.spawn(Node {
            width: Val::Px(BTN_W * 5.0 + 12.0 * 4.0),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(12.0),
            column_gap: Val::Px(12.0),
            ..default()
        })
        .with_children(|grid| {
            for &(kind, label) in BUTTONS {
                spawn_button(grid, fonts, kind, label);
            }
        });
    });
}

/// 面板按钮清单（顺序即布局顺序）。
const BUTTONS: &[(ButtonKind, &str)] = &[
    (ButtonKind::MoveForward, "前进 W"),
    (ButtonKind::MoveBackward, "后退 S"),
    (ButtonKind::MoveLeft, "左移 A"),
    (ButtonKind::MoveRight, "右移 D"),
    (ButtonKind::Sprint, "疾跑 Ctrl"),
    (ButtonKind::Aim, "瞄准"),
    (ButtonKind::Jump, "跳跃 Space"),
    (ButtonKind::Shoot, "射击 / 投掷"),
    (ButtonKind::Reload, "换弹 R"),
    (ButtonKind::SkillQ, "技能 Q"),
    (ButtonKind::SkillE, "技能 E"),
    (ButtonKind::Weapon1, "武器 1"),
    (ButtonKind::Weapon2, "武器 2"),
    (ButtonKind::UseConsumable, "恢复品 3"),
    (ButtonKind::UseTactical, "战术品 4"),
    (ButtonKind::OpenBackpack, "背包 Tab"),
    (ButtonKind::OpenBigMap, "大地图 M"),
    (ButtonKind::OpenPause, "暂停 /"),
    (ButtonKind::Close, "关闭 B"),
];

/// 生成单个可点击按钮。
fn spawn_button(p: &mut ChildBuilder<'_>, fonts: &CjkFont, kind: ButtonKind, label: &str) {
    p.spawn((
        kind,
        Interaction::default(),
        (
            Node {
                width: Val::Px(BTN_W),
                height: Val::Px(BTN_H),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.10, 0.12, 0.16, 0.92)),
            BorderColor(theme::PANEL_BORDER),
        ),
    ))
    .with_children(|btn| {
        btn.spawn(flow::text(
            label,
            flow::style(fonts, 15.0, theme::TEXT_WHITE),
        ));
    });
}

/// `B` 开 / `B` 或 `Esc` 关按钮面板（打开即释放光标，与背包面板同形态）。
pub fn button_panel_toggle(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<ButtonPanelState>,
    mut root: Query<&mut Visibility, With<ButtonPanelRoot>>,
) {
    let toggle = if state.open {
        keys.just_pressed(KeyCode::KeyB) || keys.just_pressed(KeyCode::Escape)
    } else {
        keys.just_pressed(KeyCode::KeyB)
    };
    if !toggle {
        return;
    }
    state.open = !state.open;
    state.reset_flags();
    if let Ok(mut vis) = root.get_single_mut() {
        *vis = if state.open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// 处理按钮点击：切换持续标志 / 压入边沿动作 / 执行纯客户端面板切换。
#[allow(clippy::too_many_arguments)]
pub fn button_panel_click(
    mut state: ResMut<ButtonPanelState>,
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    mut backpack: ResMut<crate::hud::BackpackPanelState>,
    mut bigmap: ResMut<crate::hud::BigMapOpen>,
    mut pause_req: EventWriter<crate::flow::PauseOpenRequest>,
    mut root: Query<&mut Visibility, With<ButtonPanelRoot>>,
    mut backpack_root: Query<
        &mut Visibility,
        (With<crate::hud::BackpackPanelRoot>, Without<ButtonPanelRoot>),
    >,
    mut bigmap_root: Query<
        &mut Visibility,
        (
            With<crate::hud::BigMapRoot>,
            Without<ButtonPanelRoot>,
            Without<crate::hud::BackpackPanelRoot>,
        ),
    >,
    buttons: Query<(&ButtonKind, &Interaction), Changed<Interaction>>,
) {
    if !state.open {
        return;
    }
    for (kind, interaction) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match kind {
            ButtonKind::MoveForward => state.forward = !state.forward,
            ButtonKind::MoveBackward => state.backward = !state.backward,
            ButtonKind::MoveLeft => state.left = !state.left,
            ButtonKind::MoveRight => state.right = !state.right,
            ButtonKind::Sprint => state.sprint = !state.sprint,
            ButtonKind::Aim => state.aim = !state.aim,
            ButtonKind::Jump => state.jump = true,
            ButtonKind::Shoot => state.shoot = true,
            ButtonKind::Reload => state.reload = true,
            ButtonKind::SkillQ => state.skill_q = true,
            ButtonKind::SkillE => state.skill_e = true,
            ButtonKind::Weapon1 => state.weapon_slot = Some(0),
            ButtonKind::Weapon2 => state.weapon_slot = Some(1),
            ButtonKind::UseConsumable => {
                state.use_slot = first_slot(&snap, &player, ItemCategory::Consumable);
            }
            ButtonKind::UseTactical => {
                state.use_slot = first_slot(&snap, &player, ItemCategory::Tactical);
            }
            // —— 纯客户端面板切换：先收起本面板，再打开目标面板 ——
            ButtonKind::OpenBackpack => {
                close_self(&mut state, &mut root);
                backpack.open = true;
                if let Ok(mut vis) = backpack_root.get_single_mut() {
                    *vis = Visibility::Visible;
                }
            }
            ButtonKind::OpenBigMap => {
                close_self(&mut state, &mut root);
                bigmap.0 = true;
                if let Ok(mut vis) = bigmap_root.get_single_mut() {
                    *vis = Visibility::Visible;
                }
            }
            ButtonKind::OpenPause => {
                close_self(&mut state, &mut root);
                // 请求 `menu` 装配暂停 UI（字体未就绪与否由 `menu` 自行判定，与本模块解耦）。
                pause_req.send(crate::flow::PauseOpenRequest);
            }
            ButtonKind::Close => close_self(&mut state, &mut root),
        }
    }
}

/// 每帧合成一条 [`PlayerInput`] 上报（面板打开期间由按钮驱动；关闭当帧补发全零输入停止动作）。
///
/// 设计动机（Why）：面板打开时常规 `input_system` 被 `gameplay_input_active` 冻结，服务端每 Tick
/// 复用**最近一次**输入——若关闭时不补发一条全零输入，服务端会继续执行最后一次"按钮驱动的持续
/// 移动"。故关闭当帧补发一条中性输入，确保动作立即停止。
pub fn button_panel_emit(
    mut state: ResMut<ButtonPanelState>,
    rig: Res<AimRig>,
    mut seq: ResMut<SeqCounter>,
    out: Res<NetOut>,
    mut last: Local<Option<PlayerInput>>,
    mut was_open: Local<bool>,
) {
    if !state.open {
        if *was_open {
            seq.0 += 1;
            let input = PlayerInput {
                seq: seq.0,
                aim_yaw: rig.yaw,
                aim_pitch: rig.pitch,
                ..default()
            };
            let _ = out.0.send(ClientMessage::Input { player: input });
            *last = Some(input);
            *was_open = false;
        }
        return;
    }
    *was_open = true;

    let mut input = PlayerInput {
        move_forward: state.forward,
        move_backward: state.backward,
        move_left: state.left,
        move_right: state.right,
        sprint: state.sprint,
        aim: state.aim,
        jump: state.jump,
        shoot: state.shoot,
        reload: state.reload,
        skill_q: state.skill_q,
        skill_e: state.skill_e,
        weapon_slot: state.weapon_slot,
        use_slot: state.use_slot,
        aim_yaw: rig.yaw,
        aim_pitch: rig.pitch,
        ..default()
    };
    // 边沿量消费后清空 → 天然形成"一帧 true → 下一帧 false"脉冲（服务端锁存消费一次）。
    state.jump = false;
    state.shoot = false;
    state.reload = false;
    state.skill_q = false;
    state.skill_e = false;
    state.weapon_slot = None;
    state.use_slot = None;

    if last.as_ref() == Some(&input) {
        return;
    }
    seq.0 += 1;
    input.seq = seq.0;
    *last = Some(input);
    let _ = out.0.send(ClientMessage::Input { player: input });
}

/// 每帧刷新面板：根可见性 + 按钮高亮（持续动作开启 / 悬停）。
pub fn sync_button_panel(
    state: Res<ButtonPanelState>,
    mut root: Query<&mut Visibility, With<ButtonPanelRoot>>,
    mut buttons: Query<(&ButtonKind, &Interaction, &mut BackgroundColor, &mut BorderColor)>,
) {
    if let Ok(mut vis) = root.get_single_mut() {
        *vis = if state.open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !state.open {
        return;
    }
    for (kind, interaction, mut bg, mut border) in &mut buttons {
        let active = is_active(kind, &state);
        let hovered = matches!(interaction, Interaction::Hovered | Interaction::Pressed);
        *bg = if active {
            theme::ROW_HOVER.into()
        } else if hovered {
            Color::srgba(0.18, 0.21, 0.27, 0.95).into()
        } else {
            Color::srgba(0.10, 0.12, 0.16, 0.92).into()
        };
        *border = BorderColor(if active {
            theme::ACCENT_AMBER
        } else {
            theme::PANEL_BORDER
        });
    }
}

/// 离开训练场复位按钮面板（否则下次进场会带着"已打开"冻结输入）。
pub fn reset_button_panel(mut state: ResMut<ButtonPanelState>) {
    *state = ButtonPanelState::default();
}

/// 该按钮是否为"已开启的持续动作"（用于高亮）。
fn is_active(kind: &ButtonKind, state: &ButtonPanelState) -> bool {
    match kind {
        ButtonKind::MoveForward => state.forward,
        ButtonKind::MoveBackward => state.backward,
        ButtonKind::MoveLeft => state.left,
        ButtonKind::MoveRight => state.right,
        ButtonKind::Sprint => state.sprint,
        ButtonKind::Aim => state.aim,
        _ => false,
    }
}

/// 收起本面板（清标志 + 隐藏根节点）。
fn close_self(state: &mut ButtonPanelState, root: &mut Query<&mut Visibility, With<ButtonPanelRoot>>) {
    state.open = false;
    state.reset_flags();
    if let Ok(mut vis) = root.get_single_mut() {
        *vis = Visibility::Hidden;
    }
}

/// 取某速用类别在本人背包中的**首格下标**（无则 `None`）。
fn first_slot(
    snap: &SnapshotBuffer,
    player: &LocalPlayer,
    category: ItemCategory,
) -> Option<u8> {
    snap.current
        .iter()
        .find(|e| e.entity_id == player.entity_id)
        .and_then(|e| e.backpack.as_ref())
        .and_then(|bp| {
            bp.iter()
                .position(|s| s.as_ref().map(|i| i.kind.category() == Some(category)).unwrap_or(false))
        })
        .map(|i| i as u8)
}
