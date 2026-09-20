//! 特效网格/材质资产池（按元素缓存，避免重复创建与 GPU 缓冲累积）

use bevy::prelude::*;
use crate::element::ElementType;
use crate::demo::frontend::*;

/// 特效材质变体（按元素缓存，避免重复创建）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum EffectMatKind {
    /// 基础自发光：曳光/碎块/投掷物
    Plain,
    /// 伤害粒子：自发光 × 0.5
    Particle,
    /// 命中爆闪：自发光 × 1.5
    HitFlash,
    /// 爆炸主体：半透明，自发光 × 2
    Explosion,
    /// 持续区域（毒雾等）：半透明，自发光 × 1.2
    Zone,
}

/// 所有一次性特效共享的网格与材质。
/// Bevy 0.14 的 `Assets` 不会自动回收：此前每颗子弹/每次爆炸都现场
/// `meshes.add` 新网格，实体销毁后 GPU 缓冲仍然累积，在核显上几十秒
/// 就会撑爆到 wgpu OutOfMemory 崩溃。
#[derive(Resource)]
pub(crate) struct EffectAssets {
    /// 曳光：单位深度细长盒，使用时按弹道长度缩放 Z
    pub(crate) tracer: Handle<Mesh>,
    /// 火花小球：枪口焰与命中爆闪共用（尺寸靠缩放区分）
    pub(crate) spark: Handle<Mesh>,
    /// 伤害粒子：小立方体
    pub(crate) particle: Handle<Mesh>,
    /// 爆炸主体：半透明大球（缩放由 ExplosionEffect 驱动）
    pub(crate) explosion_sphere: Handle<Mesh>,
    /// 爆炸碎块：小立方体
    pub(crate) explosion_debris: Handle<Mesh>,
    /// 手雷/爆裂投掷物：小球
    pub(crate) projectile: Handle<Mesh>,
    /// 冰冻冰块：罩住目标半透明立方体
    pub(crate) frost_cube: Handle<Mesh>,
    /// 毒雾/持续区域：单位圆柱，按区域半径缩放 XZ
    pub(crate) zone_cylinder: Handle<Mesh>,
    /// 冰冻冰块材质（半透明淡蓝，全目标共用）
    pub(crate) frost_material: Handle<StandardMaterial>,
    /// 枪口焰材质（固定暖白）
    pub(crate) muzzle_material: Handle<StandardMaterial>,
    /// (元素, 变体) → 材质 缓存
    pub(crate) element_materials: Vec<(ElementType, EffectMatKind, Handle<StandardMaterial>)>,
}

impl EffectAssets {
    pub(crate) fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            tracer: meshes.add(Cuboid::new(0.04, 0.04, 1.0)),
            spark: meshes.add(Sphere::new(0.08).mesh().ico(2).unwrap()),
            particle: meshes.add(Cuboid::new(0.08, 0.08, 0.08)),
            explosion_sphere: meshes.add(Sphere::new(0.5).mesh().ico(2).unwrap()),
            explosion_debris: meshes.add(Cuboid::new(0.1, 0.1, 0.1)),
            projectile: meshes.add(Sphere::new(0.15).mesh().ico(2).unwrap()),
            frost_cube: meshes.add(Cuboid::new(2.0, 2.0, 2.0)),
            zone_cylinder: meshes.add(Cylinder::new(1.0, 1.4)),
            frost_material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.55, 0.85, 1.0, 0.45),
                emissive: LinearRgba::rgb(0.35, 0.7, 1.0) * 1.2,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            muzzle_material: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.9, 0.6),
                emissive: LinearRgba::rgb(1.0, 0.9, 0.6) * 8.0,
                ..default()
            }),
            element_materials: Vec::new(),
        }
    }

    /// 取指定元素与变体的材质，没有则创建并缓存
    pub(crate) fn material(
        &mut self,
        materials: &mut Assets<StandardMaterial>,
        element: ElementType,
        kind: EffectMatKind,
    ) -> Handle<StandardMaterial> {
        if let Some((_, _, handle)) = self
            .element_materials
            .iter()
            .find(|(e, k, _)| e == &element && k == &kind)
        {
            return handle.clone();
        }
        let color = element.color();
        let emissive = element.emissive();
        let mut mat = StandardMaterial { base_color: color, emissive, ..default() };
        match kind {
            EffectMatKind::Plain => {}
            EffectMatKind::Particle => mat.emissive = emissive * 0.5,
            EffectMatKind::HitFlash => mat.emissive = emissive * 1.5,
            EffectMatKind::Explosion => {
                mat.emissive = emissive * 2.0;
                mat.alpha_mode = AlphaMode::Blend;
            }
            EffectMatKind::Zone => {
                mat.base_color = color.with_alpha(0.35);
                mat.emissive = emissive * 1.2;
                mat.alpha_mode = AlphaMode::Blend;
            }
        }
        let handle = materials.add(mat);
        self.element_materials.push((element, kind, handle.clone()));
        handle
    }
}