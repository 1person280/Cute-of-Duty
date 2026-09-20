//! 底部快捷道具图标（3 恢复 / 4 战术）：随背包数量刷新，无货变灰

use bevy::prelude::*;
use crate::model::Player;
use crate::demo::components::*;

/// 显示背包中对应类别的数量，无货时按键变灰
pub(crate) fn hud_item_slots_system(
    player_query: Query<&Inventory, With<Player>>,
    mut slot_texts: Query<(&HudItemSlotText, &mut Text), Without<HudItemSlotLabel>>,
    mut slot_labels: Query<(&HudItemSlotLabel, &mut Text), Without<HudItemSlotText>>,
) {
    let mut counts = [0usize; 2];
    if let Ok(inventory) = player_query.get_single() {
        for item in &inventory.items {
            match item.item_type.category() {
                ItemCategory::Consumable => counts[0] += 1,
                ItemCategory::Tactical => counts[1] += 1,
            }
        }
    }

    let colors = [ITEM_RECOVERY_COLOR, ITEM_TACTICAL_COLOR];
    for (slot, mut text) in slot_texts.iter_mut() {
        let n = counts[slot.0];
        text.sections[0].style.color = if n > 0 {
            colors[slot.0]
        } else {
            Color::srgb(0.35, 0.35, 0.35)
        };
        text.sections[1].value = if n > 0 { format!(" ×{}", n) } else { String::new() };
    }
    for (slot, mut text) in slot_labels.iter_mut() {
        text.sections[0].style.color = if counts[slot.0] > 0 {
            Color::srgb(0.15, 0.15, 0.15)
        } else {
            Color::srgba(0.15, 0.15, 0.15, 0.45)
        };
    }
}