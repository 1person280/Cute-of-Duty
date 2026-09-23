//! 生命/护甲/弹药/技能状态栏的每帧更新与数值文本、干员名

use bevy::prelude::*;
use crate::operator::roster;
use crate::model::{Player, OperatorState, PlayerMovement};
use crate::demo::frontend::*;
use crate::demo::components::*;

/// 仅写文本值的查询别名：ParamSet 必须保持内联，仅把单个查询项
/// 用标记参数化即可收窄类型、规避 clippy::type_complexity。
type TextMarked<'w, 's, M> = Query<'w, 's, &'static mut Text, With<M>>;
/// 需同时写文本值与其颜色的查询（0.19 中 Text 与 TextColor 分离）
type TextColorMarked<'w, 's, M> = Query<'w, 's, (&'static mut Text, &'static mut TextColor), With<M>>;
/// 布局相关（0.19 中 Style 更名为 Node）
type NodeMarked<'w, 's, M> = Query<'w, 's, &'static mut Node, With<M>>;

/// 生命/护甲条宽度：随当前值/上限缩放。
/// 两个可变 Node 查询必须装进 ParamSet（组件同型需互斥访问），ParamSet 保持内联。
pub(crate) fn hud_health_bar_system(
    player_query: Query<(&Health, &Armor), With<Player>>,
    mut fills: ParamSet<(NodeMarked<HudHealthBarFill>, NodeMarked<HudArmorBarFill>)>,
) {
    let Ok((health, armor)) = player_query.single() else { return };
    if let Ok(mut node) = fills.p0().single_mut() {
        node.width = Val::Percent((health.current / health.max * 100.0).clamp(0.0, 100.0));
    }
    if let Ok(mut node) = fills.p1().single_mut() {
        node.width = Val::Percent((armor.current / armor.max * 100.0).clamp(0.0, 100.0));
    }
}

/// 弹药主读数：`弹匣 / 背包弹药池`
pub(crate) fn hud_ammo_main_system(
    player_query: Query<(&WeaponSlot, &Inventory), With<Player>>,
    mut ammo_main: TextMarked<HudAmmoMain>,
) {
    let Ok((weapon_slot, inventory)) = player_query.single() else { return };
    let cur_w = &inventory.weapons[weapon_slot.current];
    if let Ok(mut text) = ammo_main.single_mut() {
        text.0 = format!("{} / {}", cur_w.ammo, inventory.ammo_pool);
    }
}

/// 武器槽 1/2 名与元素色：当前手持槽位高亮
pub(crate) fn hud_weapon_slots_system(
    player_query: Query<(&WeaponSlot, &Inventory), With<Player>>,
    mut slot_texts: ParamSet<(TextColorMarked<HudWeaponSlot1>, TextColorMarked<HudWeaponSlot2>)>,
) {
    let Ok((weapon_slot, inventory)) = player_query.single() else { return };
    let w0 = &inventory.weapons[0];
    let w1 = &inventory.weapons[1];
    if let Ok((mut text, mut color)) = slot_texts.p0().single_mut() {
        text.0 = format!("[1] {}", w0.name);
        color.0 = if weapon_slot.current == 0 { w0.element.color() } else { Color::srgb(0.4, 0.4, 0.4) };
    }
    if let Ok((mut text, mut color)) = slot_texts.p1().single_mut() {
        text.0 = format!("[2] {}", w1.name);
        color.0 = if weapon_slot.current == 1 { w1.element.color() } else { Color::srgb(0.4, 0.4, 0.4) };
    }
}

/// 换弹读秒条：换弹进行中显示进度并放大，否则隐藏复位
pub(crate) fn hud_reload_system(
    player_query: Query<&PlayerMovement, With<Player>>,
    mut reload: Query<(&mut Text, &mut TextFont, &mut TextColor), With<HudReloadText>>,
) {
    let Ok(movement) = player_query.single() else { return };
    if let Ok((mut text, mut font, mut color)) = reload.single_mut() {
        if let Some(ref timer) = movement.reload_timer {
            let remaining = timer.duration().as_secs_f32() - timer.elapsed_secs();
            text.0 = format!("RELOADING {:.1}s", remaining.max(0.0));
            color.0 = Color::srgb(1.0, 0.6, 0.1);
            font.font_size = FontSize::Px(18.0);
        } else {
            text.0 = "".to_string();
            font.font_size = FontSize::Px(14.0);
        }
    }
}

/// 技能冷却条高度 + 亲和元素底色（Q/E 双技能；底色查询互斥，无需 ParamSet）
pub(crate) fn hud_skill_cd_system(
    player_query: Query<&OperatorState, With<Player>>,
    mut fills: ParamSet<(NodeMarked<HudSkillQFill>, NodeMarked<HudSkillEFill>)>,
    mut q_fill_bg: Query<&mut BackgroundColor, (With<HudSkillQFill>, Without<HudSkillEFill>)>,
    mut e_fill_bg: Query<&mut BackgroundColor, (With<HudSkillEFill>, Without<HudSkillQFill>)>,
) {
    let Ok(op) = player_query.single() else { return };
    let op_def = &roster()[op.active];
    let q_pct = (op.q.elapsed_secs() / op_def.q.cooldown_secs).clamp(0.0, 1.0);
    let e_pct = (op.e.elapsed_secs() / op_def.e.cooldown_secs).clamp(0.0, 1.0);
    if let Ok(mut node) = fills.p0().single_mut() {
        node.height = Val::Percent(q_pct * 100.0);
    }
    if let Ok(mut node) = fills.p1().single_mut() {
        node.height = Val::Percent(e_pct * 100.0);
    }
    if let Ok(mut bg) = q_fill_bg.single_mut() {
        bg.0 = op_def.element.color();
    }
    if let Ok(mut bg) = e_fill_bg.single_mut() {
        bg.0 = op_def.element.color();
    }
}

/// 技能 Q/E 冷却读秒文本 + 下方技能名标签。
/// 技能图标文本为父 Text（字母）+ 子 TextSpan（冷却秒数）两段，二者共用同一
/// 标记：父实体仅含 Text、子实体仅含 TextSpan，故按组件类型即可区分。
/// 四个 `&mut Text` 查询必须塞进同一个 ParamSet（p0..p3），
/// 否则任何两个 `&mut Text` 查询跨 ParamSet 依旧构成 B0001 冲突。
pub(crate) fn hud_skill_text_system(
    player_query: Query<&OperatorState, With<Player>>,
    mut texts: ParamSet<(
        TextColorMarked<HudSkillQText>,
        TextColorMarked<HudSkillQLabel>,
        TextColorMarked<HudSkillEText>,
        TextColorMarked<HudSkillELabel>,
        Query<&mut TextSpan, With<HudSkillQText>>,
        Query<&mut TextSpan, With<HudSkillEText>>,
    )>,
) {
    let Ok(op) = player_query.single() else { return };
    let op_def = &roster()[op.active];
    let q_pct = (op.q.elapsed_secs() / op_def.q.cooldown_secs).clamp(0.0, 1.0);
    let e_pct = (op.e.elapsed_secs() / op_def.e.cooldown_secs).clamp(0.0, 1.0);
    // Q 字母：冷却期变灰；冷却秒数由子 TextSpan 追加
    if let Ok((mut text, mut color)) = texts.p0().single_mut() {
        text.0 = "Q".to_string();
        color.0 = if q_pct >= 1.0 { op_def.element.color() } else { Color::srgb(0.45, 0.45, 0.45) };
    }
    if let Ok(mut cd) = texts.p4().single_mut() {
        if q_pct >= 1.0 {
            cd.0 = String::new();
        } else {
            let remaining = op_def.q.cooldown_secs - op.q.elapsed_secs();
            cd.0 = format!(" {:.0}", remaining.ceil());
        }
    }
    // E 字母
    if let Ok((mut text, mut color)) = texts.p2().single_mut() {
        text.0 = "E".to_string();
        color.0 = if e_pct >= 1.0 { op_def.element.color() } else { Color::srgb(0.45, 0.45, 0.45) };
    }
    if let Ok(mut cd) = texts.p5().single_mut() {
        if e_pct >= 1.0 {
            cd.0 = String::new();
        } else {
            let remaining = op_def.e.cooldown_secs - op.e.elapsed_secs();
            cd.0 = format!(" {:.0}", remaining.ceil());
        }
    }
    // 技能名称标签随干员切换
    if let Ok((mut text, mut color)) = texts.p1().single_mut() {
        text.0 = op_def.q.name.to_string();
        color.0 = op_def.element.color();
    }
    if let Ok((mut text, mut color)) = texts.p3().single_mut() {
        text.0 = op_def.e.name.to_string();
        color.0 = op_def.element.color();
    }
}

/// 血条/护甲条数值文本（同一系统内两个可变 Text 查询必须装进 ParamSet，
/// 否则 B0001 冲突）
pub(crate) fn hud_vitals_text_system(
    player_query: Query<(&Health, &Armor), With<Player>>,
    mut vitals: ParamSet<(
        TextMarked<HudHealthText>,
        TextMarked<HudArmorText>,
    )>,
) {
    let Ok((health, armor)) = player_query.single() else { return };
    {
        let mut hp = vitals.p0();
        if let Ok(mut text) = hp.single_mut() {
            text.0 = format!("HP {}/{}", health.current.round() as i32, health.max as i32);
        }
    }
    {
        let mut armor_text = vitals.p1();
        if let Ok(mut text) = armor_text.single_mut() {
            text.0 = format!("ARMOR {}/{}", armor.current.round() as i32, armor.max as i32);
        }
    }
}

/// 当前干员名 HUD（独立系统，与读秒文本系统分开以避免 Text 可变访问竞争）
pub(crate) fn hud_operator_name_system(
    player_query: Query<&OperatorState, With<Player>>,
    mut operator_name: Query<(&mut Text, &mut TextColor), With<HudOperatorName>>,
) {
    let Ok(op) = player_query.single() else { return };
    let Ok((mut text, mut color)) = operator_name.single_mut() else { return };
    let op_def = &roster()[op.active];
    text.0 = format!("干员 · {}", op_def.name);
    color.0 = op_def.element.color();
}