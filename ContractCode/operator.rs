//! 干员与武器档案系统 —— 纯数据定义（配置表驱动）
//!
//! 架构约定：本模块**不依赖 bevy**，`cod1` 无头模拟与 `cod1-demo` 共用。
//!
//! Q/E 技能归属干员而非武器：
//! - 技能组 = 形态（[`SkillKind`]）+ 独特机制（[`SkillEffect`]），
//!   每名干员一套，切换干员即切换技能组；武器只决定射击本身。
//! - 机制差异化（而不是只改数值）：
//!   * 焰狐：命中**点燃**，目标持续灼烧（DoT）
//!   * 霜刃：**冰冻控制**，目标停止行动数秒
//!   * 雷豹：**位移冲刺**（Q）+ 电麻眩晕（E）
//!   * 毒蛛：留下**持续毒雾区域**，圈内目标周期掉血
//!
//! 步枪档案（[`rifle_profile`]）同样集中在此，武器数值不散落在渲染层。

use crate::element::ElementType;

/// 技能形态：技能如何作用于世界
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SkillKind {
    /// 投掷物：沿视线抛出，落地或引信耗尽时在落点结算伤害与机制
    Grenade { damage: f32, radius: f32 },
    /// 以自身为中心立即结算伤害与机制
    Burst { damage: f32, radius: f32 },
    /// 玩家位移：朝视线方向疾冲（撞到障碍即停）
    Dash { distance: f32 },
}

/// 技能附加机制：除直接伤害外的独特效果（0 值 = 不启用该机制）
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct SkillEffect {
    /// 点燃：命中目标持续灼烧 burn_dps 点/秒，共 burn_secs 秒
    pub burn_dps: f32,
    pub burn_secs: f32,
    /// 冰冻/电麻：目标停止移动 freeze_secs 秒
    pub freeze_secs: f32,
    /// 毒雾区：在作用点留下 zone_secs 的持续区域，圈内目标受 zone_dps 点/秒
    pub zone_secs: f32,
    pub zone_dps: f32,
}

impl SkillEffect {
    /// 无附加机制
    pub const NONE: SkillEffect = SkillEffect {
        burn_dps: 0.0,
        burn_secs: 0.0,
        freeze_secs: 0.0,
        zone_secs: 0.0,
        zone_dps: 0.0,
    };

    /// 点燃（对命中目标的持续伤害）
    pub const fn burn(dps: f32, secs: f32) -> Self {
        Self { burn_dps: dps, burn_secs: secs, ..Self::NONE }
    }

    /// 冰冻/电麻（硬控）
    pub const fn freeze(secs: f32) -> Self {
        Self { freeze_secs: secs, ..Self::NONE }
    }

    /// 毒雾区域（地面持续伤害区）
    pub const fn zone(secs: f32, dps: f32) -> Self {
        Self { zone_secs: secs, zone_dps: dps, ..Self::NONE }
    }
}

/// 单个技能（Q 或 E）的定义
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkillDef {
    /// 技能名（HUD 图标下方标签，控制在 4 字以内）
    pub name: &'static str,
    /// 一句话机制描述（切换台展示）
    pub desc: &'static str,
    pub cooldown_secs: f32,
    pub kind: SkillKind,
    pub effect: SkillEffect,
}

/// 干员定义
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OperatorDef {
    pub id: &'static str,
    /// 干员名（HUD 与切换台展示）
    pub name: &'static str,
    /// 定位（突击手/控场手…）
    pub title: &'static str,
    /// 元素亲和：决定技能元素与角色表现色
    pub element: ElementType,
    /// 被动描述（展示用）
    pub passive: &'static str,
    pub q: SkillDef,
    pub e: SkillDef,
}

/// 干员名册（顺序即切换台展示顺序与默认干员位次）
const ROSTER: [OperatorDef; 4] = [
    // 火系输出：直接伤害 + 点燃 DoT
    OperatorDef {
        id: "ember_fox",
        name: "焰狐",
        title: "突击手",
        element: ElementType::Fire,
        passive: "火系压制：技能命中会点燃目标，持续灼烧。",
        q: SkillDef {
            name: "爆燃弹",
            desc: "命中点燃：灼烧目标 3 秒",
            cooldown_secs: 5.0,
            kind: SkillKind::Grenade { damage: 40.0, radius: 3.5 },
            effect: SkillEffect::burn(10.0, 3.0),
        },
        e: SkillDef {
            name: "焦土爆发",
            desc: "点燃周围目标 3 秒",
            cooldown_secs: 10.0,
            kind: SkillKind::Burst { damage: 30.0, radius: 4.0 },
            effect: SkillEffect::burn(8.0, 3.0),
        },
    },
    // 冰系控场：冰冻硬控
    OperatorDef {
        id: "frost_blade",
        name: "霜刃",
        title: "控场手",
        element: ElementType::Ice,
        passive: "冰封控场：被冰冻的目标完全停止行动。",
        q: SkillDef {
            name: "冰锥弹",
            desc: "冰冻命中目标 3 秒",
            cooldown_secs: 4.0,
            kind: SkillKind::Grenade { damage: 24.0, radius: 3.0 },
            effect: SkillEffect::freeze(3.0),
        },
        e: SkillDef {
            name: "冰封领域",
            desc: "冰冻周围目标 2.5 秒",
            cooldown_secs: 12.0,
            kind: SkillKind::Burst { damage: 18.0, radius: 5.5 },
            effect: SkillEffect::freeze(2.5),
        },
    },
    // 电系机动：位移 + 短眩晕
    OperatorDef {
        id: "volt_panther",
        name: "雷豹",
        title: "游击手",
        element: ElementType::Electric,
        passive: "高机动作战：突进拉近距离，脉冲电麻敌人。",
        q: SkillDef {
            name: "电磁突进",
            desc: "朝视线方向疾冲 7 米",
            cooldown_secs: 3.5,
            kind: SkillKind::Dash { distance: 7.0 },
            effect: SkillEffect::NONE,
        },
        e: SkillDef {
            name: "过载脉冲",
            desc: "电麻周围目标 1.2 秒",
            cooldown_secs: 8.0,
            kind: SkillKind::Burst { damage: 26.0, radius: 4.2 },
            effect: SkillEffect::freeze(1.2),
        },
    },
    // 毒系区域压制：持续毒雾区
    OperatorDef {
        id: "venom_spider",
        name: "毒蛛",
        title: "压制手",
        element: ElementType::Poison,
        passive: "区域封锁：毒雾持续侵蚀圈内的一切目标。",
        q: SkillDef {
            name: "毒雾弹",
            desc: "留下毒雾：圈内持续掉血 6 秒",
            cooldown_secs: 6.0,
            kind: SkillKind::Grenade { damage: 20.0, radius: 3.5 },
            effect: SkillEffect::zone(6.0, 12.0),
        },
        e: SkillDef {
            name: "剧毒潮涌",
            desc: "原地展开毒雾 5 秒",
            cooldown_secs: 11.0,
            kind: SkillKind::Burst { damage: 16.0, radius: 4.5 },
            effect: SkillEffect::zone(5.0, 10.0),
        },
    },
];

/// 干员名册
pub fn roster() -> &'static [OperatorDef] {
    &ROSTER
}

/// 步枪武器档案：按元素区分的步枪数值
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RifleProfile {
    pub element: ElementType,
    pub name: &'static str,
    pub max_ammo: i32,
    pub fire_interval: f32,
    pub damage: f32,
}

/// 元素 → 步枪档案。未定义元素的元素回退到制式步枪。
pub fn rifle_profile(element: ElementType) -> RifleProfile {
    match element {
        ElementType::Fire => RifleProfile {
            element,
            name: "烈焰步枪",
            max_ammo: 30,
            fire_interval: 0.12,
            damage: 12.0,
        },
        ElementType::Ice => RifleProfile {
            element,
            name: "冰霜步枪",
            max_ammo: 25,
            fire_interval: 0.15,
            damage: 15.0,
        },
        ElementType::Electric => RifleProfile {
            element,
            name: "雷电步枪",
            max_ammo: 28,
            fire_interval: 0.13,
            damage: 13.0,
        },
        ElementType::Poison => RifleProfile {
            element,
            name: "毒液步枪",
            max_ammo: 20,
            fire_interval: 0.18,
            damage: 18.0,
        },
        _ => RifleProfile {
            element,
            name: "制式步枪",
            max_ammo: 30,
            fire_interval: 0.14,
            damage: 12.0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roster_has_distinct_operators() {
        let roster = roster();
        assert!(roster.len() >= 2, "名册至少要能切换出两名干员");
        let mut elements = Vec::new();
        for op in roster {
            assert!(!op.id.is_empty() && !op.name.is_empty());
            elements.push(op.element);
        }
        let n = elements.len();
        elements.dedup();
        assert_eq!(elements.len(), n, "干员元素亲和不能重复");
    }

    #[test]
    fn all_skills_are_valid() {
        for op in roster() {
            for (slot, skill) in [("Q", &op.q), ("E", &op.e)] {
                assert!(skill.cooldown_secs > 0.0, "{} {} 冷却非法", op.name, slot);
                assert!(!skill.name.is_empty(), "{} {} 缺少技能名", op.name, slot);
                assert!(!skill.desc.is_empty(), "{} {} 缺少机制描述", op.name, slot);
                let e = &skill.effect;
                assert!(e.burn_dps >= 0.0 && e.burn_secs >= 0.0, "{} {} 点燃数值非法", op.name, slot);
                assert!(e.freeze_secs >= 0.0, "{} {} 冰冻时长非法", op.name, slot);
                assert!(e.zone_secs >= 0.0 && e.zone_dps >= 0.0, "{} {} 毒区数值非法", op.name, slot);
                // 启用机制时数值必须成对有效
                assert!(
                    (e.burn_secs > 0.0) == (e.burn_dps > 0.0),
                    "{} {} 点燃数值不完整",
                    op.name,
                    slot
                );
                assert!(
                    (e.zone_secs > 0.0) == (e.zone_dps > 0.0),
                    "{} {} 毒区数值不完整",
                    op.name,
                    slot
                );
                match skill.kind {
                    SkillKind::Grenade { damage, radius } | SkillKind::Burst { damage, radius } => {
                        assert!(damage > 0.0 && radius > 0.0, "{} {} 范围数值非法", op.name, slot);
                    }
                    SkillKind::Dash { distance } => {
                        assert!(distance > 0.0, "{} {} 冲刺距离非法", op.name, slot);
                    }
                }
            }
        }
    }

    /// 干员技能必须机制差异化：不能全是同一种"范围伤害"
    #[test]
    fn roster_covers_distinct_mechanics() {
        let roster = roster();
        let any_skill = |pred: fn(&SkillDef) -> bool| {
            roster.iter().any(|op| pred(&op.q) || pred(&op.e))
        };
        // 冰系硬控：至少一个 ≥3 秒的冰冻（用户示例：冰手雷冰冻控制 3 秒）
        assert!(
            any_skill(|s| s.effect.freeze_secs >= 3.0),
            "缺少 ≥3s 的冰冻控制技能"
        );
        // 火系持续伤害
        assert!(any_skill(|s| s.effect.burn_secs > 0.0), "缺少点燃 DoT 技能");
        // 毒系区域
        assert!(any_skill(|s| s.effect.zone_secs > 0.0), "缺少毒雾区域技能");
        // 位移技能
        assert!(
            roster.iter().any(|op| matches!(op.q.kind, SkillKind::Dash { .. })),
            "缺少位移冲刺技能"
        );
        // 每名干员的 Q 与 E 描述不得雷同（机制要有区分度）
        for op in roster.iter() {
            assert_ne!(op.q.desc, op.e.desc, "{} 的 Q/E 机制描述雷同", op.name);
        }
    }

    #[test]
    fn rifle_profiles_are_valid() {
        let mut names = Vec::new();
        for element in [
            ElementType::Fire,
            ElementType::Ice,
            ElementType::Electric,
            ElementType::Poison,
        ] {
            let p = rifle_profile(element);
            assert!(p.max_ammo > 0 && p.fire_interval > 0.0 && p.damage > 0.0);
            assert_eq!(p.element, element, "档案元素与请求元素不一致");
            names.push(p.name);
        }
        let n = names.len();
        names.dedup();
        assert_eq!(names.len(), n, "步枪档案名不能重复");
    }

    #[test]
    fn legacy_balance_is_pinned() {
        // 旧版默认双枪数值：防止改档时无意破坏既有手感
        let fire = rifle_profile(ElementType::Fire);
        assert_eq!(fire.name, "烈焰步枪");
        assert_eq!(fire.max_ammo, 30);
        assert!((fire.fire_interval - 0.12).abs() < 1e-6);
        assert!((fire.damage - 12.0).abs() < 1e-6);

        let ice = rifle_profile(ElementType::Ice);
        assert_eq!(ice.name, "冰霜步枪");
        assert_eq!(ice.max_ammo, 25);
        assert!((ice.damage - 15.0).abs() < 1e-6);
    }
}
