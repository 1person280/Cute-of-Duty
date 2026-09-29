//! HUD 模态发布 —— 把各面板/持雷的开关状态对照仲裁态，有差异才向 `flow` 发 [`ModalChange`]。
//!
//! 设计动机（Why）：客户端有七个 HUD 侧模态（大地图 / F 交互二级面板 / 物资箱 / 径向轮盘 /
//! 背包 / 操作按钮组 / 持雷）。若在各开关点手工发事件，新增模态或多一条开关路径就会漏发。
//! 这里集中成**唯一发布点**：每帧读取各源资源、与 `flow::ModalState` 对照、只发差值。
//! 无论状态怎么变（按键 / 拖拽 / 快照派生 / OnExit 复位）都被自动捕获，杜绝漂移。
//!
//! 边界：本文件只读 HUD 自己的面板资源与 `flow::ModalState`（只读），不写任何状态。

use bevy::prelude::*;

use crate::flow::{ModalChange, ModalKind, ModalState};

use crate::hud::backpack::panel::BackpackPanelState;
use crate::hud::bigmap::BigMapOpen;
use crate::hud::button::panel::ButtonPanelState;
use crate::hud::grenade::hint::HeldGrenadeState;
use crate::hud::interact::InteractState;
use crate::hud::item::wheel::ItemWheelState;
use crate::hud::loot::panel::LootPanelState;

/// 对照七个 HUD 侧模态源与仲裁态，发射差异 [`ModalChange`]。
pub fn publish_modal_changes(
    modal: Res<ModalState>,
    mut out: EventWriter<ModalChange>,
    bigmap: Res<BigMapOpen>,
    interact: Res<InteractState>,
    loot: Res<LootPanelState>,
    wheel: Res<ItemWheelState>,
    backpack: Res<BackpackPanelState>,
    button: Res<ButtonPanelState>,
    held: Res<HeldGrenadeState>,
) {
    let pairs = [
        (ModalKind::BigMap, bigmap.0),
        (ModalKind::Interact, interact.panel_open),
        (ModalKind::Loot, loot.open),
        (ModalKind::Wheel, wheel.open),
        (ModalKind::Backpack, backpack.open),
        (ModalKind::Button, button.open),
        (ModalKind::HeldGrenade, held.element.is_some()),
    ];
    for (kind, open) in pairs {
        if modal.get(kind) != open {
            out.send(ModalChange { kind, open });
        }
    }
}