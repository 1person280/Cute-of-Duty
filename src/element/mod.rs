//! 元素系统核心模块
//! 
//! 核心游戏机制：元素互斥生态
//! 设计理念：将元素反应从 O(n²) 笛卡尔积压缩为 O(n) 线性扩展
//! 所有元素反应通过配置表驱动，新增元素只需添加配置行

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 基础元素类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ElementType {
    /// 火 - 放热：高爆发、持续燃烧
    Fire,
    /// 冰 - 吸热：减速/冻结控场
    Ice,
    /// 电 - 导电：连锁伤害、破盾效率
    Electric,
    /// 毒/酸 - 腐蚀：护甲穿透、持续削弱
    Poison,
    /// 物理/无 - 中性：稳定、无互斥
    Physical,
    /// 水 - 潮湿：环境互动
    Water,
}

impl ElementType {
    /// 获取元素名称
    pub fn name(&self) -> &'static str {
        match self {
            ElementType::Fire => "火",
            ElementType::Ice => "冰",
            ElementType::Electric => "电",
            ElementType::Poison => "毒",
            ElementType::Physical => "物理",
            ElementType::Water => "水",
        }
    }

    /// 是否为物理/中性元素
    pub fn is_neutral(&self) -> bool {
        matches!(self, ElementType::Physical)
    }

    /// 是否会产生互斥效果
    pub fn is_mutually_exclusive(&self) -> bool {
        !self.is_neutral()
    }

    /// 英文键名：互斥配置表的 `elements` 字符串以它开头
    /// （如 "IceArmor" 以 "Ice" 开头 → 冰系护甲）
    pub fn key(&self) -> &'static str {
        match self {
            ElementType::Fire => "Fire",
            ElementType::Ice => "Ice",
            ElementType::Electric => "Electric",
            ElementType::Poison => "Poison",
            ElementType::Physical => "Physical",
            ElementType::Water => "Water",
        }
    }
}

/// 实体当前元素状态
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityElementState {
    /// 正常状态
    Normal,
    /// 潮湿状态
    Wet,
    /// 冰冻状态
    Frozen,
    /// 燃烧状态
    Burning,
    /// 中毒状态
    Poisoned,
    /// 感电状态（电元素附着）
    Electrified,
    /// 草地/植被环境
    Grass,
    /// 雨天环境
    RainEnvironment,
    /// 高温环境
    HighTemperature,
    /// 雪地环境
    SnowEnvironment,
}

/// 元素反应结果类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReactionResult {
    Vaporize,        // 蒸发
    Melt,            // 融化
    Burning,         // 燃烧
    Electrolysis,    // 电解
    Superconduct,    // 超导
    PoisonExplosion, // 毒雾爆炸
    PoisonCloud,     // 毒云
    ShatterFreeze,   // 爆裂冻结
    PhysicalVulnerability, // 物理易伤
    RainSuppressed,  // 雨天压制
    RainAmplified,   // 雨天增强
    Overheat,        // 过热
    OverheatRisk,    // 过热风险
    SnowAmplified,   // 雪地增强
    SnowSuppressed,  // 雪地压制
    ConductiveRisk,  // 导电风险
}

/// 元素反应规则（配置表中的一行）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementReaction {
    pub target_state: EntityElementState,
    pub incoming_element: ElementType,
    pub result: ReactionResult,
    pub damage_multiplier: f32,
    #[serde(default)]
    pub attach_effects: Vec<ReactionResult>,
    #[serde(default)]
    pub description: String,
}

/// 互斥规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfConflictRule {
    pub elements: Vec<String>,
    pub effect: String,
    pub value: f32,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeammateConflictCombination {
    pub elements: Vec<String>,
    pub effect: String,
    pub max_value: f32,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeammateConflictRules {
    pub radius: f32,
    pub falloff: String,
    pub combinations: Vec<TeammateConflictCombination>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MutualExclusionRules {
    #[serde(default)]
    pub self_conflicts: Vec<SelfConflictRule>,
    #[serde(default)]
    pub teammate_conflicts: Option<TeammateConflictRules>,
}

/// 协同增益规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynergyRule {
    pub name: String,
    pub condition: String,
    pub trigger: String,
    pub effect: serde_yaml::Value,
    pub description: String,
}

/// 元素系统配置（YAML反序列化目标）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementConfig {
    #[serde(default)]
    pub reactions: Vec<ElementReaction>,
    #[serde(default)]
    pub mutual_exclusion: MutualExclusionRules,
    #[serde(default)]
    pub synergies: Vec<SynergyRule>,
}

impl Default for ElementConfig {
    /// 内置默认配置：与 config/element_reactions.yaml 保持一致
    /// （找不到配置文件时的回退值，两份数据必须同步修改）
    fn default() -> Self {
        let reaction = |target_state: EntityElementState,
                        incoming_element: ElementType,
                        result: ReactionResult,
                        damage_multiplier: f32,
                        attach_effects: Vec<ReactionResult>,
                        description: &str| {
            ElementReaction {
                target_state,
                incoming_element,
                result,
                damage_multiplier,
                attach_effects,
                description: description.to_string(),
            }
        };
        Self {
            reactions: vec![
                // ========== 基础元素反应 ==========
                reaction(
                    EntityElementState::Wet,
                    ElementType::Fire,
                    ReactionResult::Vaporize,
                    1.5,
                    vec![ReactionResult::Vaporize],
                    "水遇火蒸发，造成额外伤害",
                ),
                reaction(
                    EntityElementState::Frozen,
                    ElementType::Fire,
                    ReactionResult::Melt,
                    2.0,
                    vec![ReactionResult::Melt],
                    "冰遇火融化，造成巨额伤害",
                ),
                reaction(
                    EntityElementState::Grass,
                    ElementType::Fire,
                    ReactionResult::Burning,
                    1.2,
                    vec![ReactionResult::Burning],
                    "点燃植被，持续燃烧",
                ),
                reaction(
                    EntityElementState::Wet,
                    ElementType::Electric,
                    ReactionResult::Electrolysis,
                    1.8,
                    vec![ReactionResult::Electrolysis],
                    "电解潮湿目标，无视护盾直接伤害",
                ),
                reaction(
                    EntityElementState::Frozen,
                    ElementType::Electric,
                    ReactionResult::Superconduct,
                    1.0,
                    vec![ReactionResult::Superconduct],
                    "超导反应，降低目标物理抗性",
                ),
                reaction(
                    EntityElementState::Poisoned,
                    ElementType::Fire,
                    ReactionResult::PoisonExplosion,
                    2.5,
                    vec![ReactionResult::PoisonCloud],
                    "毒雾爆炸，产生范围毒火混合云",
                ),
                reaction(
                    EntityElementState::Burning,
                    ElementType::Ice,
                    ReactionResult::ShatterFreeze,
                    2.0,
                    vec![ReactionResult::ShatterFreeze, ReactionResult::PhysicalVulnerability],
                    "火后冰击，目标护甲碎裂，受到200%物理易伤",
                ),
                // ========== 环境互动修正 ==========
                reaction(
                    EntityElementState::RainEnvironment,
                    ElementType::Fire,
                    ReactionResult::RainSuppressed,
                    0.7,
                    vec![],
                    "雨天火系输出降低30%",
                ),
                reaction(
                    EntityElementState::RainEnvironment,
                    ElementType::Electric,
                    ReactionResult::RainAmplified,
                    1.2,
                    vec![ReactionResult::ConductiveRisk],
                    "雨天电系输出增加20%，但有自伤风险",
                ),
                reaction(
                    EntityElementState::HighTemperature,
                    ElementType::Fire,
                    ReactionResult::Overheat,
                    1.4,
                    vec![ReactionResult::OverheatRisk],
                    "高温区火系输出增加40%，但有过热自伤风险",
                ),
                reaction(
                    EntityElementState::SnowEnvironment,
                    ElementType::Ice,
                    ReactionResult::SnowAmplified,
                    1.3,
                    vec![],
                    "雪地冰系输出增加30%",
                ),
                reaction(
                    EntityElementState::SnowEnvironment,
                    ElementType::Electric,
                    ReactionResult::SnowSuppressed,
                    0.8,
                    vec![],
                    "雪地电系输出降低20%",
                ),
            ],
            mutual_exclusion: MutualExclusionRules {
                self_conflicts: vec![
                    SelfConflictRule {
                        elements: vec!["IceArmor".to_string(), "FireWeapon".to_string()],
                        effect: "fire_output_reduction".to_string(),
                        value: 0.15,
                        description: "冰甲+火枪：火枪输出降低15%（冰吸热）".to_string(),
                    },
                    SelfConflictRule {
                        elements: vec!["FireArmor".to_string(), "IceGrenade".to_string()],
                        effect: "ice_efficiency_reduction".to_string(),
                        value: 0.15,
                        description: "火甲+冰雷：冰雷冻结效率降低15%（火升温）".to_string(),
                    },
                    SelfConflictRule {
                        elements: vec!["ElectricWeapon".to_string(), "WaterMine".to_string()],
                        effect: "self_damage_risk".to_string(),
                        value: 0.25,
                        description: "电枪+水雷：潮湿环境下电枪自伤概率提升25%".to_string(),
                    },
                ],
                teammate_conflicts: Some(TeammateConflictRules {
                    radius: 5.0,
                    falloff: "linear".to_string(),
                    combinations: vec![
                        TeammateConflictCombination {
                            elements: vec!["IceArmor".to_string(), "FireWeapon".to_string()],
                            effect: "fire_output_reduction".to_string(),
                            max_value: 0.15,
                            description: "冰甲队友降低火枪输出，贴脸15%，5米边缘0%".to_string(),
                        },
                        TeammateConflictCombination {
                            elements: vec!["PoisonArmor".to_string(), "HealAbility".to_string()],
                            effect: "heal_reduction".to_string(),
                            max_value: 0.20,
                            description: "毒甲队友降低生命恢复效率，贴脸20%，5米边缘0%".to_string(),
                        },
                        TeammateConflictCombination {
                            elements: vec!["ElectricArmor".to_string(), "ElectronicDevice".to_string()],
                            effect: "device_interference".to_string(),
                            max_value: 0.10,
                            description: "电甲队友干扰电子设备，贴脸10%，5米边缘0%".to_string(),
                        },
                    ],
                }),
            },
            synergies: vec![
                SynergyRule {
                    name: "烈焰共鸣".to_string(),
                    condition: "three_fire_players_spaced".to_string(),
                    trigger: "three_fire_players_distance_gt_5m".to_string(),
                    effect: serde_yaml::from_str("{fire_damage_bonus: 0.25, burn_stackable: true}").unwrap(),
                    description: "三名火系玩家间距>5米，各自火伤+25%，燃烧叠加".to_string(),
                },
                SynergyRule {
                    name: "电解".to_string(),
                    condition: "electric_on_wet".to_string(),
                    trigger: "electric_attack_on_wet_target".to_string(),
                    effect: serde_yaml::from_str("{ignore_shield: true, direct_hp_damage: true}").unwrap(),
                    description: "电系攻击潮湿目标，无视常规护盾直接伤害生命值".to_string(),
                },
                SynergyRule {
                    name: "毒雾爆炸".to_string(),
                    condition: "poison_then_fire".to_string(),
                    trigger: "poison_corrosion_followed_by_fire_ignite".to_string(),
                    effect: serde_yaml::from_str("{create_poison_fire_cloud: true, cloud_radius: 5.0, cloud_duration: 8.0}").unwrap(),
                    description: "毒系腐蚀后火系点燃，产生范围毒火混合云".to_string(),
                },
                SynergyRule {
                    name: "爆裂冻结".to_string(),
                    condition: "fire_then_ice".to_string(),
                    trigger: "fire_burning_followed_by_ice_freeze".to_string(),
                    effect: serde_yaml::from_str("{armor_shatter: true, physical_vulnerability: 2.0}").unwrap(),
                    description: "火系先手挂燃烧，冰系后手冻结，目标护甲碎裂，受到200%物理易伤".to_string(),
                },
            ],
        }
    }
}

/// 元素系统运行时
pub struct ElementSystem {
    config: ElementConfig,
    /// 反应查找表: (EntityElementState, ElementType) -> ElementReaction
    reaction_lookup: HashMap<(EntityElementState, ElementType), ElementReaction>,
}

impl ElementSystem {
    /// 创建新的元素系统
    pub fn new(config: ElementConfig) -> Self {
        let mut reaction_lookup = HashMap::new();
        
        for reaction in &config.reactions {
            let key = (reaction.target_state.clone(), reaction.incoming_element);
            reaction_lookup.insert(key, reaction.clone());
        }

        info!("元素系统初始化完成: {} 种反应规则", reaction_lookup.len());
        
        Self {
            config,
            reaction_lookup,
        }
    }

    /// 已加载的反应规则数量（供展示层日志/界面使用）
    pub fn reaction_count(&self) -> usize {
        self.config.reactions.len()
    }

    /// 查询元素反应
    /// 
    /// # 参数
    /// - `target_state`: 目标实体的当前元素状态
    /// - `incoming_element`:  incoming的攻击元素
    ///
    /// # 返回
    /// - `Some(ElementReaction)`: 匹配到的反应规则
    /// - `None`: 无反应（物理攻击通常无特殊反应）
    pub fn query_reaction(
        &self,
        target_state: &EntityElementState,
        incoming_element: &ElementType,
    ) -> Option<&ElementReaction> {
        let key = (target_state.clone(), *incoming_element);
        self.reaction_lookup.get(&key)
    }

    /// 计算最终伤害倍率
    /// 
    /// 流程:
    /// 1. 查询元素反应矩阵
    /// 2. 应用伤害倍率
    /// 3. 返回最终倍率（无反应则返回1.0）
    pub fn calculate_damage_multiplier(
        &self,
        target_state: &EntityElementState,
        incoming_element: &ElementType,
    ) -> f32 {
        self.query_reaction(target_state, incoming_element)
            .map(|r| r.damage_multiplier)
            .unwrap_or(1.0)
    }

    /// 计算队友互斥惩罚（距离衰减，全部参数来自配置表）
    ///
    /// 半径、衰减曲线取 `mutual_exclusion.teammate_conflicts`，
    /// 上限取与元素对匹配的 combination 的 `max_value`。
    /// 元素按英文键名前缀匹配 `elements` 字符串（"IceArmor" 以 "Ice" 开头）。
    ///
    /// # 参数
    /// - `element_a`: 玩家A的元素/装备类型
    /// - `element_b`: 玩家B的元素/装备类型
    /// - `distance`: 两人之间的距离（米）
    ///
    /// # 返回
    /// 惩罚系数（0.0 ~ max_value），超出互斥半径返回0.0
    pub fn calculate_teammate_penalty(
        &self,
        element_a: &ElementType,
        element_b: &ElementType,
        distance: f32,
    ) -> f32 {
        let Some(rules) = &self.config.mutual_exclusion.teammate_conflicts else {
            return 0.0;
        };
        let key_a = element_a.key();
        let key_b = element_b.key();
        let Some(combo) = rules
            .combinations
            .iter()
            .find(|c| pair_matches(&c.elements, key_a, key_b))
        else {
            return 0.0;
        };

        if distance >= rules.radius {
            return 0.0;
        }

        match rules.falloff.as_str() {
            // 线性衰减: 贴脸100%生效，半径边缘0%生效
            "linear" => combo.max_value * (1.0 - distance / rules.radius),
            // 未知曲线：半径内全惩罚
            _ => combo.max_value,
        }
    }

    /// 计算自身装备互斥惩罚（数值来自配置表）
    ///
    /// 匹配 `mutual_exclusion.self_conflicts`，`elements` 顺序约定为
    /// `[护甲元素前缀, 武器元素前缀]`，如 [IceArmor, FireWeapon] 命中
    /// (护甲=Ice, 武器=Fire)。
    pub fn calculate_self_penalty(
        &self,
        armor_element: &ElementType,
        weapon_element: &ElementType,
    ) -> f32 {
        let key_armor = armor_element.key();
        let key_weapon = weapon_element.key();
        for rule in &self.config.mutual_exclusion.self_conflicts {
            if rule.elements.len() >= 2
                && rule.elements[0].starts_with(key_armor)
                && rule.elements[1].starts_with(key_weapon)
            {
                return rule.value;
            }
        }
        0.0
    }

    /// 获取环境互动修正（配置表驱动：环境行与元素反应共用同一张表）
    ///
    /// 以 (环境状态, 元素) 为键查询反应表，取 `damage_multiplier`；
    /// 表中没有的组合返回 1.0。环境行示例见 YAML 的
    /// `target_state: RainEnvironment` 等条目。
    pub fn get_environment_modifier(
        &self,
        environment: &EntityElementState,
        element: &ElementType,
    ) -> f32 {
        self.query_reaction(environment, element)
            .map(|r| r.damage_multiplier)
            .unwrap_or(1.0)
    }

    /// 检查协同增益是否触发
    pub fn check_synergy_trigger(
        &self,
        synergy_name: &str,
        condition_params: &HashMap<String, f32>,
    ) -> bool {
        for synergy in &self.config.synergies {
            if synergy.name == synergy_name {
                return match synergy.condition.as_str() {
                    "three_fire_players_spaced" => {
                        condition_params.get("fire_player_count").copied().unwrap_or(0.0) >= 3.0
                            && condition_params.get("min_distance").copied().unwrap_or(0.0) > 5.0
                    }
                    _ => false,
                };
            }
        }
        false
    }

    /// 按名称取协同增益的效果负载（如 fire_damage_bonus），
    /// 供调用方读取数值——效果数值必须来自配置表，不允许散落硬编码。
    pub fn synergy_effect(&self, synergy_name: &str) -> Option<&serde_yaml::Value> {
        self.config
            .synergies
            .iter()
            .find(|s| s.name == synergy_name)
            .map(|s| &s.effect)
    }

    /// 打印元素互斥速查表
    pub fn print_cheat_sheet(&self) {
        info!("========== 元素互斥速查表 ==========");
        info!("冰吸热 → 火输出降低");
        info!("火升温 → 冰效率降低");
        info!("电导电 → 水/潮湿环境自伤");
        info!("毒腐蚀 → 队友生命恢复降低");
        info!("距离 > 5米 = 无互斥");
        info!("距离 < 5米 = 线性衰减互斥");
        info!("====================================");
    }
}

use tracing::info;

/// 元素对是否命中互斥规则的 elements 列表（英文键名前缀匹配）：
/// 列表中需存在两个不同条目分别以 key_a、key_b 开头。
/// 同名元素不互斥（a == b 时恒为 false）。
fn pair_matches(elements: &[String], key_a: &str, key_b: &str) -> bool {
    if key_a == key_b {
        return false;
    }
    elements.iter().enumerate().any(|(i, item)| {
        item.starts_with(key_a)
            && elements
                .iter()
                .enumerate()
                .any(|(j, other)| i != j && other.starts_with(key_b))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_reaction_lookup() {
        let config = ElementConfig::default();
        let system = ElementSystem::new(config);

        let reaction = system.query_reaction(
            &EntityElementState::Wet,
            &ElementType::Fire,
        );
        assert!(reaction.is_some());
        assert_eq!(reaction.unwrap().damage_multiplier, 1.5);
    }

    #[test]
    fn test_teammate_penalty_linear_falloff() {
        let config = ElementConfig::default();
        let system = ElementSystem::new(config);

        // 距离0米: 15%惩罚
        let penalty = system.calculate_teammate_penalty(&ElementType::Ice, &ElementType::Fire, 0.0);
        assert!((penalty - 0.15).abs() < 0.001);

        // 距离2.5米: 7.5%惩罚
        let penalty = system.calculate_teammate_penalty(&ElementType::Ice, &ElementType::Fire, 2.5);
        assert!((penalty - 0.075).abs() < 0.001);

        // 距离5米: 0%惩罚
        let penalty = system.calculate_teammate_penalty(&ElementType::Ice, &ElementType::Fire, 5.0);
        assert_eq!(penalty, 0.0);

        // 距离6米: 0%惩罚（超出范围）
        let penalty = system.calculate_teammate_penalty(&ElementType::Ice, &ElementType::Fire, 6.0);
        assert_eq!(penalty, 0.0);
    }

    #[test]
    fn test_environment_modifiers() {
        let config = ElementConfig::default();
        let system = ElementSystem::new(config);

        // 雨天火系-30%
        let modifier = system.get_environment_modifier(
            &EntityElementState::RainEnvironment,
            &ElementType::Fire,
        );
        assert_eq!(modifier, 0.7);

        // 雪地冰系+30%
        let modifier = system.get_environment_modifier(
            &EntityElementState::SnowEnvironment,
            &ElementType::Ice,
        );
        assert_eq!(modifier, 1.3);

        // 无环境修正
        let modifier = system.get_environment_modifier(
            &EntityElementState::Normal,
            &ElementType::Fire,
        );
        assert_eq!(modifier, 1.0);
    }

    #[test]
    fn test_self_penalty_from_config() {
        let system = ElementSystem::new(ElementConfig::default());

        // 冰甲+火枪 / 火甲+冰雷 / 电枪+水雷：数值来自默认配置表
        assert!((system.calculate_self_penalty(&ElementType::Ice, &ElementType::Fire) - 0.15).abs() < 1e-6);
        assert!((system.calculate_self_penalty(&ElementType::Fire, &ElementType::Ice) - 0.15).abs() < 1e-6);
        assert!((system.calculate_self_penalty(&ElementType::Electric, &ElementType::Water) - 0.25).abs() < 1e-6);
        // 反向搭配不在表中（顺序约定 [护甲, 武器]）
        assert_eq!(system.calculate_self_penalty(&ElementType::Water, &ElementType::Electric), 0.0);
        assert_eq!(system.calculate_self_penalty(&ElementType::Ice, &ElementType::Ice), 0.0);
    }

    #[test]
    fn test_synergy_effect_payload() {
        let system = ElementSystem::new(ElementConfig::default());

        let effect = system.synergy_effect("烈焰共鸣").expect("默认配置应含烈焰共鸣");
        let bonus = effect
            .get("fire_damage_bonus")
            .and_then(|v| v.as_f64())
            .expect("应含 fire_damage_bonus");
        assert!((bonus - 0.25).abs() < 1e-6);

        assert!(system.synergy_effect("不存在的协同").is_none());
    }
}
