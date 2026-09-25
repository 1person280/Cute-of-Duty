//! 干员展示元数据（仅表现层）：把服务端 `operator_id` 映射为干员名 / 元素配色 / 制式武器名。
//!
//! 设计动机：这些是稳定展示母板（与 `ServerCode/operator::roster()` 视觉一致），不含任何
//! 战力/判定数据——弹道、技能、元素反应一律由服务端按 `operator_id` 裁决，客户端只画色与名。

use bevy::prelude::Color;

/// 单项干员展示元数据。
pub struct OperatorMeta {
    pub name: &'static str,
    pub color: Color,
    pub weapon: &'static str,
    pub skill_q: &'static str,
    pub skill_e: &'static str,
}

/// 4 元素干员展示母板（与旧版 焰狐/霜刃/雷豹/毒蛛 及服务端 roster 顺序一致）。
pub const OPERATORS: [OperatorMeta; 4] = [
    OperatorMeta {
        name: "焰狐",
        color: Color::srgb(0.95, 0.55, 0.20),
        weapon: "烈焰步枪",
        skill_q: "爆燃弹",
        skill_e: "焦土爆发",
    },
    OperatorMeta {
        name: "霜刃",
        color: Color::srgb(0.25, 0.60, 0.95),
        weapon: "冰霜步枪",
        skill_q: "冰锥弹",
        skill_e: "冰封领域",
    },
    OperatorMeta {
        name: "雷豹",
        color: Color::srgb(0.90, 0.75, 0.20),
        weapon: "雷电步枪",
        skill_q: "电磁突进",
        skill_e: "过载脉冲",
    },
    OperatorMeta {
        name: "毒蛛",
        color: Color::srgb(0.40, 0.75, 0.35),
        weapon: "毒液步枪",
        skill_q: "毒雾弹",
        skill_e: "剧毒潮涌",
    },
];

/// 越界安全取干员元数据（会话固定一号干员时也能正常渲染）。
pub fn meta(id: u32) -> &'static OperatorMeta {
    OPERATORS.get(id as usize).unwrap_or(&OPERATORS[0])
}