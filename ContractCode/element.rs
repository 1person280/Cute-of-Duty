//! 元素类型契约（两端共享的枚举与纯函数）
//!
//! 设计动机（Why）：[`ElementType`] 与 [`EntityElementState`] 出现在快照/事件/物品载荷里，
//! 是**契约的一部分**；而"反应表怎么算"（`ElementSystem` / `ElementConfig`）属服务端模拟逻辑，
//! 留在 `ServerCode::element`。此处只放枚举与其无副作用的展示/判据方法。

use serde::{Deserialize, Serialize};

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