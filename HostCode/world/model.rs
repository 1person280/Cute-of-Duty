//! 体素造型（客户端稳定冷资源映射）
//!
//! 设计动机（对齐全局约束「客户端只承载不易变的东西」）：每个 `ModelPreset`
//! 对应的几何尺寸与配色在这里一次性写死，属冷资源；服务端只发 preset 身份，
//! 客户端据此挑选这套稳定造型，绝不本地改模型 —— 造型决定权始终在服务端。

use bevy::color::Color;
use bevy::prelude::*;
use cute_of_duty_server::element::ElementType;
use cute_of_duty_server::interact::InteractKind;
use cute_of_duty_server::map::{PickupKind, StationKind};
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
        ModelPreset::Station => (
            Color::srgb(0.30, 0.33, 0.38), // 台体钢灰
            Color::srgb(1.00, 0.80, 0.25), // 台面琥珀描边
        ),
    };

    // 功能站点：矮而宽的单方块台体（区别于 0.9 立方的小物件）。
    if preset == ModelPreset::Station {
        return VoxelBody {
            primary,
            accent,
            torso_scale: Vec3::new(1.6, 0.5, 1.0),
            head_scale: Vec3::ZERO,
            head_y: 0.0,
        };
    }

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

/// 元素展示配色（客户端冷映射，与服务端 element 语义无关）。
fn element_rgb(e: ElementType) -> [f32; 3] {
    match e {
        ElementType::Fire => [0.95, 0.45, 0.15],
        ElementType::Ice => [0.35, 0.75, 0.95],
        ElementType::Electric => [0.90, 0.80, 0.25],
        ElementType::Poison => [0.45, 0.80, 0.30],
        ElementType::Physical => [0.75, 0.75, 0.78],
        ElementType::Water => [0.30, 0.55, 0.90],
    }
}

/// 可交互物 → 材质配色变体编号（供 [`crate::net::snapshot::EntityMaterials`] 作缓存 key）。
///
/// 设计动机（Why）：所有拾取物共用 `SupplyCrate` 造型以省资源，但战场上必须一眼分辨
/// "这是什么"。故服务端只发语义（`InteractKind`），客户端把它折算成一个小的配色变体号，
/// 与 `ModelPreset` 一起作为材质缓存键——既不复用错色，也不为每种道具新增一套网格。
/// 返回 `0` 表示"用 preset 自带配色"（普通实体）。
pub fn tint_code(kind: Option<&InteractKind>) -> u8 {
    match kind {
        None => 0,
        Some(InteractKind::Pickup(p)) => match p {
            PickupKind::Ammo { .. } => 1,
            PickupKind::Health { .. } => 2,
            PickupKind::Armor { .. } => 3,
            PickupKind::Grenade { element } => 10 + element_index(*element),
            PickupKind::Weapon { element } => 20 + element_index(*element),
        },
        Some(InteractKind::Station(s)) => match s {
            StationKind::SupplyTable => 30,
            StationKind::OperatorDesk => 31,
            StationKind::SupplyCrate => 32,
        },
    }
}

/// 变体编号 → （主色, 强调色）。仅对 [`tint_code`] 的非 0 结果有效。
pub fn tint_colors(code: u8) -> (Color, Color) {
    let rgb = |c: [f32; 3]| Color::srgb(c[0], c[1], c[2]);
    match code {
        1 => (rgb([0.55, 0.42, 0.18]), rgb([0.95, 0.80, 0.35])), // 弹药：黄铜
        2 => (rgb([0.72, 0.18, 0.18]), rgb([0.95, 0.95, 0.95])), // 医疗：红白
        3 => (rgb([0.22, 0.35, 0.65]), rgb([0.55, 0.75, 1.00])), // 护甲：钢蓝
        30 => (rgb([0.38, 0.30, 0.14]), rgb([1.00, 0.65, 0.15])), // 补给台：琥珀
        31 => (rgb([0.14, 0.32, 0.34]), rgb([0.20, 0.90, 0.95])), // 干员切换台：青
        32 => (rgb([0.40, 0.28, 0.12]), rgb([1.00, 0.55, 0.12])), // 物资箱：橙
        c if (10..16).contains(&c) => {
            // 元素手雷：按元素上色（主色压暗、强调色取亮）
            let e = &[
                ElementType::Fire,
                ElementType::Ice,
                ElementType::Electric,
                ElementType::Poison,
                ElementType::Physical,
                ElementType::Water,
            ][(c - 10) as usize];
            let base = element_rgb(*e);
            (
                rgb([base[0] * 0.55, base[1] * 0.55, base[2] * 0.55]),
                rgb(base),
            )
        }
        c if (20..26).contains(&c) => {
            // 元素武器：同元素色系，但主色更亮以示"武器"身份
            let e = &[
                ElementType::Fire,
                ElementType::Ice,
                ElementType::Electric,
                ElementType::Poison,
                ElementType::Physical,
                ElementType::Water,
            ][(c - 20) as usize];
            let base = element_rgb(*e);
            (rgb(base), rgb([0.95, 0.95, 0.95]))
        }
        // 兜底：理论上不可达，返回中性灰避免 panic。
        _ => (Color::srgb(0.5, 0.5, 0.5), Color::srgb(0.8, 0.8, 0.8)),
    }
}

/// 元素 → 0..6 的稳定序号（供变体编号编码）。
fn element_index(e: ElementType) -> u8 {
    match e {
        ElementType::Fire => 0,
        ElementType::Ice => 1,
        ElementType::Electric => 2,
        ElementType::Poison => 3,
        ElementType::Physical => 4,
        ElementType::Water => 5,
    }
}