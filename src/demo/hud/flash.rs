//! 低弹药闪烁 / 屏幕边缘红光（低血量告警）/ 重生闪屏

use bevy::prelude::*;
use crate::model::Player;
use crate::demo::components::*;

/// 弹药不足时主弹药显示红色闪烁提示
pub(crate) fn low_ammo_blink(
    mut ammo_main: Query<(&mut Text, &mut TextColor), With<HudAmmoMain>>,
    player_query: Query<(&WeaponSlot, &Inventory), With<Player>>,
    time: Res<Time>,
) {
    let Ok((weapon_slot, inventory)) = player_query.single() else { return };
    let weapon = &inventory.weapons[weapon_slot.current];
    let Ok((mut _text, mut color)) = ammo_main.single_mut() else { return };

    if weapon.ammo <= 5 && weapon.ammo > 0 {
        let flash = (time.elapsed_secs() * 6.0).sin() > 0.0;
        color.0 = if flash { Color::srgb(1.0, 0.2, 0.2) } else { Color::srgb(0.95, 0.95, 0.95) };
    } else {
        color.0 = Color::srgb(0.95, 0.95, 0.95);
    }
}

/// 血量过低时屏幕边缘泛起红色光晕
pub(crate) fn screen_edge_glow(
    mut edge_glow: Query<&mut BackgroundColor, With<HudEdgeGlow>>,
    player_query: Query<&Health, With<Player>>,
) {
    let Ok(health) = player_query.single() else { return };
    let Ok(mut bg) = edge_glow.single_mut() else { return };

    let hp_ratio = health.current / health.max;
    if hp_ratio < 0.3 {
        bg.0 = Color::srgba(1.0, 0.1, 0.1, 0.15 * (1.0 - hp_ratio / 0.3));
    } else {
        bg.0 = Color::srgba(1.0, 0.0, 0.0, 0.0);
    }
}

pub(crate) fn respawn_flash_system(
    _query: Query<&mut BackgroundColor, With<HudEdgeGlow>>,
) {
}