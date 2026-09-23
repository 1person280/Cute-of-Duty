//! 底部快捷道具图标（3 恢复 / 4 战术）：随背包数量刷新，无货变灰

use bevy::prelude::*;
use crate::model::Player;
use crate::demo::components::*;

/// 显示背包中对应类别的数量，无货时按键变灰。
/// 0.19 中图标为父 Text（按键字母，查 Text+TextColor）+ 子 TextSpan（数量）两段，
/// 二者共用同一 HudItemSlotText 标记，按 Text/TextSpan 组件区分父与子。
pub(crate) fn hud_item_slots_system(
    player_query: Query<&Inventory, With<Player>>,
    mut slot_texts: Query<(&HudItemSlotText, &mut Text, &mut TextColor), (Without<HudItemSlotLabel>, Without<TextSpan>)>,
    mut slot_spans: Query<(&HudItemSlotText, &mut TextSpan), (Without<HudItemSlotLabel>, Without<Text>)>,
    mut slot_labels: Query<(&HudItemSlotLabel, &mut TextColor), (Without<HudItemSlotText>, Without<TextSpan>)>,
) {
    let mut counts = [0usize; 2];
    if let Ok(inventory) = player_query.single() {
        for item in &inventory.items {
            match item.item_type.category() {
                ItemCategory::Consumable => counts[0] += 1,
                ItemCategory::Tactical => counts[1] += 1,
            }
        }
    }

    let colors = [ITEM_RECOVERY_COLOR, ITEM_TACTICAL_COLOR];
    for (slot, _text, mut color) in slot_texts.iter_mut() {
        color.0 = if counts[slot.0] > 0 {
            colors[slot.0]
        } else {
            Color::srgb(0.35, 0.35, 0.35)
        };
    }
    for (slot, mut span) in slot_spans.iter_mut() {
        span.0 = if counts[slot.0] > 0 { format!(" ×{}", counts[slot.0]) } else { String::new() };
    }
    for (slot, mut color) in slot_labels.iter_mut() {
        color.0 = if counts[slot.0] > 0 {
            Color::srgb(0.15, 0.15, 0.15)
        } else {
            Color::srgba(0.15, 0.15, 0.15, 0.45)
        };
    }
}