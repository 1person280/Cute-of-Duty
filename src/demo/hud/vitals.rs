//! 生命/护甲/弹药/技能状态栏的每帧更新与数值文本、干员名

use bevy::prelude::*;
use crate::operator::roster;
use crate::model::{Player, OperatorState, PlayerMovement};
use crate::demo::frontend::*;
use crate::demo::components::*;

/// 生命/护甲条、弹药、武器槽位、技能冷却、换弹状态、技能名的每帧刷新
#[allow(clippy::type_complexity)]
pub(crate) fn hud_update_system(
    player_query: Query<(&Health, &Armor, &WeaponSlot, &OperatorState, &PlayerMovement, &Inventory), With<Player>>,
    mut styles: ParamSet<(
        Query<&mut Style, With<HudHealthBarFill>>,
        Query<&mut Style, With<HudArmorBarFill>>,
        Query<&mut Style, With<HudSkillQFill>>,
        Query<&mut Style, With<HudSkillEFill>>,
    )>,
    mut texts: ParamSet<(
        Query<&mut Text, With<HudAmmoMain>>,
        Query<&mut Text, With<HudWeaponSlot1>>,
        Query<&mut Text, With<HudWeaponSlot2>>,
        Query<&mut Text, With<HudSkillQText>>,
        Query<&mut Text, With<HudSkillEText>>,
        Query<&mut Text, With<HudReloadText>>,
        Query<&mut Text, With<HudSkillQLabel>>,
        Query<&mut Text, With<HudSkillELabel>>,
    )>,
    mut q_fill_bg: Query<&mut BackgroundColor, (With<HudSkillQFill>, Without<HudSkillEFill>)>,
    mut e_fill_bg: Query<&mut BackgroundColor, (With<HudSkillEFill>, Without<HudSkillQFill>)>,
) {
    let Ok((health, armor, weapon_slot, op, movement, inventory)) = player_query.get_single() else { return };
    let op_def = &roster()[op.active];

    // 固定双主武器：slot 0/1 即 1/2 号位
    let cur_w = &inventory.weapons[weapon_slot.current];
    let w0 = &inventory.weapons[0];
    let w1 = &inventory.weapons[1];

    {
        let mut health_bar = styles.p0();
        if let Ok(mut style) = health_bar.get_single_mut() {
            style.width = Val::Percent((health.current / health.max * 100.0).clamp(0.0, 100.0));
        }
    }
    {
        let mut armor_bar = styles.p1();
        if let Ok(mut style) = armor_bar.get_single_mut() {
            style.width = Val::Percent((armor.current / armor.max * 100.0).clamp(0.0, 100.0));
        }
    }

    // 弹药主显示：弹匣 / 背包弹药池
    {
        let mut ammo_main = texts.p0();
        if let Ok(mut text) = ammo_main.get_single_mut() {
            text.sections[0].value = format!("{} / {}", cur_w.ammo, inventory.ammo_pool);
        }
    }

    {
        let mut weapon_slot1 = texts.p1();
        if let Ok(mut text) = weapon_slot1.get_single_mut() {
            let color = if weapon_slot.current == 0 { w0.element.color() } else { Color::srgb(0.4, 0.4, 0.4) };
            text.sections[0].value = format!("[1] {}", w0.name);
            text.sections[0].style.color = color;
        }
    }
    {
        let mut weapon_slot2 = texts.p2();
        if let Ok(mut text) = weapon_slot2.get_single_mut() {
            let color = if weapon_slot.current == 1 { w1.element.color() } else { Color::srgb(0.4, 0.4, 0.4) };
            text.sections[0].value = format!("[2] {}", w1.name);
            text.sections[0].style.color = color;
        }
    }

    // Skill Q（技能元素 = 干员亲和元素）
    let q_pct = (op.q.elapsed_secs() / op_def.q.cooldown_secs).clamp(0.0, 1.0);
    {
        let mut skill_q_fill = styles.p2();
        if let Ok(mut style) = skill_q_fill.get_single_mut() {
            style.height = Val::Percent(q_pct * 100.0);
        }
    }
    if let Ok(mut bg) = q_fill_bg.get_single_mut() {
        bg.0 = op_def.element.color();
    }
    {
        let mut skill_q_text = texts.p3();
        if let Ok(mut text) = skill_q_text.get_single_mut() {
            if q_pct >= 1.0 {
                text.sections[0].value = "Q".to_string();
                text.sections[0].style.color = op_def.element.color();
                text.sections[1].value = String::new();
            } else {
                // 字母保持可见（变灰），冷却秒数作为旁注追加
                text.sections[0].value = "Q".to_string();
                text.sections[0].style.color = Color::srgb(0.45, 0.45, 0.45);
                let remaining = op_def.q.cooldown_secs - op.q.elapsed_secs();
                text.sections[1].value = format!(" {:.0}", remaining.ceil());
            }
        }
    }

    // Skill E
    let e_pct = (op.e.elapsed_secs() / op_def.e.cooldown_secs).clamp(0.0, 1.0);
    {
        let mut skill_e_fill = styles.p3();
        if let Ok(mut style) = skill_e_fill.get_single_mut() {
            style.height = Val::Percent(e_pct * 100.0);
        }
    }
    if let Ok(mut bg) = e_fill_bg.get_single_mut() {
        bg.0 = op_def.element.color();
    }
    {
        let mut skill_e_text = texts.p4();
        if let Ok(mut text) = skill_e_text.get_single_mut() {
            if e_pct >= 1.0 {
                text.sections[0].value = "E".to_string();
                text.sections[0].style.color = op_def.element.color();
                text.sections[1].value = String::new();
            } else {
                text.sections[0].value = "E".to_string();
                text.sections[0].style.color = Color::srgb(0.45, 0.45, 0.45);
                let remaining = op_def.e.cooldown_secs - op.e.elapsed_secs();
                text.sections[1].value = format!(" {:.0}", remaining.ceil());
            }
        }
    }

    // Reload status: show prominently when reloading
    {
        let mut reload_text = texts.p5();
        if let Ok(mut text) = reload_text.get_single_mut() {
            if let Some(ref timer) = movement.reload_timer {
                let remaining = timer.duration().as_secs_f32() - timer.elapsed_secs();
                text.sections[0].value = format!("RELOADING {:.1}s", remaining.max(0.0));
                text.sections[0].style.color = Color::srgb(1.0, 0.6, 0.1);
                text.sections[0].style.font_size = 18.0;
            } else {
                text.sections[0].value = "".to_string();
                text.sections[0].style.font_size = 14.0;
            }
        }
    }

    // Skill bottom labels: 技能名随干员变化
    {
        let mut skill_q_label = texts.p6();
        if let Ok(mut text) = skill_q_label.get_single_mut() {
            text.sections[0].value = op_def.q.name.to_string();
            text.sections[0].style.color = op_def.element.color();
        }
    }
    {
        let mut skill_e_label = texts.p7();
        if let Ok(mut text) = skill_e_label.get_single_mut() {
            text.sections[0].value = op_def.e.name.to_string();
            text.sections[0].style.color = op_def.element.color();
        }
    }
}

/// 血条/护甲条数值文本（独立系统：hud_update_system 的 Text ParamSet 已满 8 席，
/// 且同一系统内两个可变 Text 查询必须装进 ParamSet，否则 B0001 冲突）
pub(crate) fn hud_vitals_text_system(
    player_query: Query<(&Health, &Armor), With<Player>>,
    mut vitals: ParamSet<(
        Query<&mut Text, With<HudHealthText>>,
        Query<&mut Text, With<HudArmorText>>,
    )>,
) {
    let Ok((health, armor)) = player_query.get_single() else { return };
    {
        let mut hp = vitals.p0();
        if let Ok(mut text) = hp.get_single_mut() {
            text.sections[0].value = format!("HP {}/{}", health.current.round() as i32, health.max as i32);
        }
    }
    {
        let mut armor_text = vitals.p1();
        if let Ok(mut text) = armor_text.get_single_mut() {
            text.sections[0].value = format!("ARMOR {}/{}", armor.current.round() as i32, armor.max as i32);
        }
    }
}

/// 当前干员名 HUD（独立系统：hud_update_system 的 Text ParamSet 已满 8 席）
pub(crate) fn hud_operator_name_system(
    player_query: Query<&OperatorState, With<Player>>,
    mut operator_name: Query<&mut Text, With<HudOperatorName>>,
) {
    let Ok(op) = player_query.get_single() else { return };
    let Ok(mut text) = operator_name.get_single_mut() else { return };
    let op_def = &roster()[op.active];
    text.sections[0].value = format!("干员 · {}", op_def.name);
    text.sections[0].style.color = op_def.element.color();
}