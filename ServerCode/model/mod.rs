//! 模型身份权威（服务器决定"这个实体长什么样"）
//!
//! 设计动机（对齐全局约束「易变逻辑放服务端、客户端只承载稳定冷资源」）：
//! 一个实体用哪种体素模型、哪套配色，属于**会随战局/内容迭代而变**的易变信息，
//! 因此由服务端权威决定，随快照下发；客户端只拿本地稳定几何/贴图按 preset 绘制。
//!
//! 本模块不含任何渲染代码，只产出被称为 `ModelPreset` 的身份标记——它是客户端
//! 渲染器与服务端世界之间的一份"冷映射"契约：preset 稳定、客户端据此选网格。

use crate::entity::EntityType;

/// 体素模型身份（服务端权威标记，经快照下行给客户端）
///
/// 命名即"这个实体的胸口形象"，客户端依此从本地资产挑选体素网格与配色。
/// 新增造型只需在此枚举加一档，并在 `from_entity_type` 里建立映射。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ModelPreset {
    /// 我方火系干员（默认玩家形象）
    OperativeFire,
    /// 我方冰系干员
    OperativeIce,
    /// 我方电系干员
    OperativeElectric,
    /// 我方毒系干员
    OperativePoison,
    /// 敌方通用暴徒
    EnemyThug,
    /// 手雷投射物
    Grenade,
    /// 补给箱（搜刮点）
    SupplyCrate,
    /// 场景障碍/掩体
    Obstacle,
    /// 训练靶（实弹靶机造型）
    AimTarget,
    /// 功能站点（补给台/干员切换台/物资箱的矮台造型）
    Station,
}

impl ModelPreset {
    /// 由实体类型派生默认模型身份。
    ///
    /// 为何放在服务端：一个实体最终以哪种模型呈现，随策划内容迭代而变，
    /// 属"易变"信息，故作为服务端权威在此裁决，而非写死在客户端。
    pub fn from_entity_type(t: EntityType) -> Self {
        match t {
            EntityType::Player => Self::OperativeFire,
            EntityType::AI => Self::EnemyThug,
            EntityType::Grenade => Self::Grenade,
            EntityType::Loot => Self::SupplyCrate,
            EntityType::Obstacle => Self::Obstacle,
            EntityType::Target => Self::AimTarget,
            EntityType::Station => Self::Station,
        }
    }
}