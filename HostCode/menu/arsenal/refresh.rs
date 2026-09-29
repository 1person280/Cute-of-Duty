//! 仓库选装浮层的**刷新与绘制**：把携带态写成文本/配色，不改变任何玩法状态。
//!
//! 设计动机（Why）：每帧只「读选择、出画」，与服务端权威裁决彻底解耦——这里改的全是
//! UI 文本与颜色，选择清单本身只由交互子模块增删，职责单一便于独立演进。

use bevy::prelude::*;

use super::state::*;

/// 物资池下标 `pool_idx` 当前是否已携带；是则返回其在携带清单中的序位。
///
/// 以物资名而非下标做匹配（Why）：清单是"按携带顺序"的物资名序列，名到序位的映射
/// 才是绿标与拖回所需的一致口径。
pub(super) fn carried_index(sel: &ArsenalSelection, pool_idx: usize) -> Option<usize> {
    let name = MVP_ITEMS.get(pool_idx)?;
    sel.0.iter().position(|it| it.as_str() == *name)
}

/// 按物资名取行内配色（未登记则退化为白色）。
pub(super) fn pool_color(name: &str) -> Color {
    MVP_ITEMS
        .iter()
        .position(|it| *it == name)
        .map(|i| ITEM_COLORS[i])
        .unwrap_or(Color::WHITE)
}

/// 按携带态刷仓库行绿标 + 背包槽文案/配色 + 容量计数。仅面板可见时由拖拽系统逐帧调用。
pub(super) fn write_loadout_texts(sel: &ArsenalSelection, wh: &mut WhTextQ, bp: &mut BpTextQ, cap: &mut CapTextQ) {
    for (status, mut text) in wh.iter_mut() {
        let on = carried_index(sel, status.0).is_some();
        text.0 = if on { "已携带 ✓".to_string() } else { String::new() };
    }
    for (slot, mut text, mut text_color) in bp.iter_mut() {
        match sel.0.get(slot.0) {
            Some(name) => {
                text.0 = name.clone();
                text_color.0 = pool_color(name);
            }
            None => {
                text.0 = "空".to_string();
                text_color.0 = Color::srgb(0.85, 0.85, 0.85);
            }
        }
    }
    if let Ok(mut cap) = cap.get_single_mut() {
        cap.0 = format!("{} / {}", sel.0.len(), LOADOUT_CAPACITY);
    }
}