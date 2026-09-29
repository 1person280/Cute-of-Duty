//! 模型身份权威（服务器决定"这个实体长什么样"）
//!
//! 设计动机（对齐全局约束「易变逻辑放服务端、客户端只承载稳定冷资源」）：
//! 一个实体用哪种体素模型、哪套配色，属于**会随战局/内容迭代而变**的易变信息，
//! 因此由服务端权威决定，随快照下发；客户端只拿本地稳定几何/贴图按 preset 绘制。
//!
//! 本模块不含任何渲染代码，只产出被称为 `ModelPreset` 的身份标记——它是客户端
//! 渲染器与服务端世界之间的一份"冷映射"契约：preset 稳定、客户端据此选网格。
//!
//! 体素几何/动画的权威解析见子模块 [`loader`]：造型内容（骨/盒/表达式）由服务端
//! 从编译期嵌入的 `FireFox.json` 解析，并经协议下发到客户端内存供渲染。

pub mod loader;

// 体素模型的**形状契约**（身份标记 + 几何/动画结构 + 像素比例）是双端共享的线格式类型，
// 归契约 crate 所有；解析（`loader`）与服务端专属的实体→模型映射留在服务端。
pub use cute_of_duty_contract::model::{
    ModelPreset, RotationExpr, VoxelAnimationSpec, VoxelBone, VoxelBoneTrack, VoxelCube,
    VoxelModelSpec, YANHU_SCALE,
};
pub use loader::{animations, catalog};

use crate::entity::EntityType;

/// 由实体类型派生默认模型身份。
///
/// 为何放在服务端：一个实体最终以哪种模型呈现，随策划内容迭代而变，属"易变"信息，
/// 故作为服务端权威在此裁决，而非写死在客户端。映射依赖服务端的 [`EntityType`]，
/// 按 Rust 孤儿规则不能作为契约类型上的 inherent 方法，故落为服务端自由函数。
pub fn preset_for_entity_type(t: EntityType) -> ModelPreset {
    match t {
        EntityType::Player => ModelPreset::OperativeFire,
        EntityType::AI => ModelPreset::EnemyThug,
        EntityType::Grenade => ModelPreset::Grenade,
        EntityType::Loot => ModelPreset::SupplyCrate,
        EntityType::Obstacle => ModelPreset::Obstacle,
        EntityType::Target => ModelPreset::AimTarget,
        EntityType::Station => ModelPreset::Station,
    }
}