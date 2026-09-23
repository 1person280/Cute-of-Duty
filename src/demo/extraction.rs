//! 撤离区确认：玩家贴近北端红色提取信标时，弹出"按 Enter 确认撤离"提示，
//! 回车即判定撤离成功、整局结束返回主菜单。用于搜打撤流程的"撤"阶段验收。

use bevy::prelude::*;
use crate::model::Player;
use crate::demo::frontend::AppState;

/// 撤离光垫中心（与 lawn/extract_zone 绿色撤离垫一致，z=-440）
pub(crate) const EXTRACTION_POINT: Vec3 = Vec3::new(0.0, 0.0, -440.0);
/// 判定"进入撤离区"的触发半径（米，按平面距离算，忽略玩家站立高度）
pub(crate) const EXTRACTION_RANGE: f32 = 12.0;

/// 撤离提示容器（整局常驻、默认隐藏）
#[derive(Component)]
pub(crate) struct ExtractionHintRoot;
/// 撤离提示文本
#[derive(Component)]
pub(crate) struct ExtractionHintText;

/// 靠近撤离区时显示提示，回车确认后整局结束回主菜单。
/// 提示显隐与文本各挂独立标记，互斥隔离以避免 `&mut Text`/`&mut Visibility` 冲突。
pub(crate) fn extraction_zone_system(
    player_query: Query<&Transform, With<Player>>,
    mut keyboard: ResMut<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut root_vis: Query<&mut Visibility, (With<ExtractionHintRoot>, Without<ExtractionHintText>)>,
    mut hint: Query<(&mut Text, &mut TextColor), (With<ExtractionHintText>, Without<ExtractionHintRoot>)>,
) {
    let Ok(player) = player_query.single() else { return };
    // 平面距离（忽略 Y）：站上光垫即触发，不受角色站立高度影响
    let planar = Vec2::new(player.translation.x - EXTRACTION_POINT.x, player.translation.z - EXTRACTION_POINT.z);
    let in_zone = planar.length() <= EXTRACTION_RANGE;

    if let Ok(mut vis) = root_vis.single_mut() {
        *vis = if in_zone { Visibility::Visible } else { Visibility::Hidden };
    }
    if let Ok((mut text, mut color)) = hint.single_mut() {
        if in_zone {
            text.0 = "已抵达撤离区 · 按 Enter 确认撤离".to_string();
            color.0 = Color::srgb(0.9, 0.2, 0.2);
        } else {
            text.0 = String::new();
        }
    }

    // 确认撤离：回车判定成功，整局结束回主菜单
    if in_zone && keyboard.just_pressed(KeyCode::Enter) {
        keyboard.clear_just_pressed(KeyCode::Enter);
        next_state.set(AppState::MainMenu);
    }
}
