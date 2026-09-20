//! 元素反应结算：把"已附着元素"映射为核心实体状态，查询反应配置表结算伤害与文案

use crate::element::{ElementSystem, ElementType, EntityElementState, ReactionResult};

/// 将"已附着的元素"映射为核心系统的实体元素状态
pub(crate) fn applied_element_state(element: ElementType) -> Option<EntityElementState> {
    match element {
        ElementType::Fire => Some(EntityElementState::Burning),
        ElementType::Ice => Some(EntityElementState::Frozen),
        ElementType::Electric => Some(EntityElementState::Electrified),
        ElementType::Poison => Some(EntityElementState::Poisoned),
        ElementType::Water => Some(EntityElementState::Wet),
        ElementType::Physical => None,
    }
}

/// 反应结果的中文名（弹字显示）
pub(crate) fn reaction_result_label(result: &ReactionResult) -> &'static str {
    match result {
        ReactionResult::Vaporize => "蒸发！",
        ReactionResult::Melt => "融化！",
        ReactionResult::Burning => "燃烧！",
        ReactionResult::Electrolysis => "电解！",
        ReactionResult::Superconduct => "超导！",
        ReactionResult::PoisonExplosion => "毒爆！",
        ReactionResult::PoisonCloud => "毒云！",
        ReactionResult::ShatterFreeze => "爆裂冻结！",
        ReactionResult::PhysicalVulnerability => "物理易伤！",
        ReactionResult::RainSuppressed => "雨天压制",
        ReactionResult::RainAmplified => "雨天增强",
        ReactionResult::Overheat => "过热",
        ReactionResult::OverheatRisk => "过热风险",
        ReactionResult::SnowAmplified => "雪地增强",
        ReactionResult::SnowSuppressed => "雪地压制",
        ReactionResult::ConductiveRisk => "导电风险",
    }
}

/// 元素反应结算：查询核心配置表（config/element_reactions.yaml）
///
/// 返回 (最终伤害, 反应名称)。伤害 = 基础伤害 × 反应倍率，
/// 与cod1的伤害结算走同一份YAML规则，新增反应无需改代码。
pub(crate) fn element_reaction(
    system: &ElementSystem,
    existing: Option<ElementType>,
    incoming: ElementType,
    base_damage: f32,
) -> (f32, Option<String>) {
    let Some(existing) = existing else { return (base_damage, None); };
    let Some(state) = applied_element_state(existing) else { return (base_damage, None); };
    let Some(reaction) = system.query_reaction(&state, &incoming) else {
        return (base_damage, None);
    };

    let damage = base_damage * reaction.damage_multiplier;
    (damage, Some(reaction_result_label(&reaction.result).to_string()))
}