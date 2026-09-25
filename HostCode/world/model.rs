//! 体素造型（客户端稳定冷资源映射）
//!
//! 设计动机（对齐全局约束「客户端只承载不易变的东西」）：每个 `ModelPreset`
//! 对应的几何尺寸与配色在这里一次性写死，属冷资源；服务端只发 preset 身份，
//! 客户端据此挑选这套稳定造型，绝不本地改模型 —— 造型决定权始终在服务端。

use bevy::color::Color;
use bevy::prelude::*;
use cute_of_duty_server::model::ModelPreset;

/// 单个体素角色/物体由「躯干 + 头」两个缩放方块拼出的稳定造型。
///
/// `anchor` 为角色脚底基准高度（服务端坐标的 y 轴原点），据此摆正整体。
#[derive(Clone, Copy)]
pub struct VoxelBody {
    /// 躯干主色
    pub primary: Color,
    /// 头部/饰条强调色
    pub accent: Color,
    /// 躯干半高（决定身体块缩放）
    pub torso_scale: Vec3,
    /// 头部块缩放
    pub head_scale: Vec3,
    /// 脚底基准→头部中心的相对高度（放头用）
    pub head_y: f32,
}

/// 由服务端权威的模型身份，映射到一套稳定的体素造型（冷数据，一经定型即冻结）。
///
/// `AimTarget`（训练靶机）复用物体类单方块造型，配色取靶面红白，便于识别被击打目标。
pub fn voxel_for(preset: ModelPreset) -> VoxelBody {
    let (primary, accent) = match preset {
        ModelPreset::OperativeFire => (
            Color::srgb(0.75, 0.20, 0.16), // 焦红
            Color::srgb(1.00, 0.55, 0.00),
        ),
        ModelPreset::OperativeIce => (
            Color::srgb(0.35, 0.55, 0.90), // 霜蓝
            Color::srgb(0.75, 0.90, 1.00),
        ),
        ModelPreset::OperativeElectric => (
            Color::srgb(0.90, 0.75, 0.15), // 电黄
            Color::srgb(0.55, 0.95, 0.85),
        ),
        ModelPreset::OperativePoison => (
            Color::srgb(0.55, 0.82, 0.25), // 毒绿
            Color::srgb(0.20, 0.40, 0.20),
        ),
        ModelPreset::EnemyThug => (
            Color::srgb(0.25, 0.28, 0.32), // 战术深灰
            Color::srgb(0.85, 0.18, 0.18),
        ),
        ModelPreset::Grenade => (
            Color::srgb(0.15, 0.55, 0.30),
            Color::srgb(0.30, 0.30, 0.30),
        ),
        ModelPreset::SupplyCrate => (
            Color::srgb(0.45, 0.34, 0.22), // 补给木箱
            Color::srgb(0.95, 0.80, 0.30),
        ),
        ModelPreset::Obstacle => (
            Color::srgb(0.35, 0.35, 0.38),
            Color::srgb(0.20, 0.20, 0.22),
        ),
        ModelPreset::AimTarget => (
            Color::srgb(0.75, 0.18, 0.16), // 靶面红
            Color::srgb(0.95, 0.95, 0.85), // 靶心白
        ),
    };

    // 角色类用"立人"造型，物体类退化为单方块
    let humanoid = matches!(
        preset,
        ModelPreset::OperativeFire
            | ModelPreset::OperativeIce
            | ModelPreset::OperativeElectric
            | ModelPreset::OperativePoison
            | ModelPreset::EnemyThug
    );
    if humanoid {
        VoxelBody {
            primary,
            accent,
            torso_scale: Vec3::new(1.1, 1.5, 0.7),
            head_scale: Vec3::splat(0.85),
            head_y: 2.25,
        }
    } else {
        VoxelBody {
            primary,
            accent,
            torso_scale: Vec3::splat(0.9),
            head_scale: Vec3::ZERO,
            head_y: 0.0,
        }
    }
}