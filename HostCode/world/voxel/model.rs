//! 焰狐体素模型渲染（把服务端下发的 [`VoxelModelSpec`] 落成实体树）
//!
//! 设计动机（对齐全局约束「易变内容放服务端、客户端只画」）：几何骨/盒不由客户端硬编码，
//! 而是消费服务端经协议下发的内存副本。本模块只做"把规格摆成 3D 实体"这一件事——
//! 复用场景共享的单位立方网格，盒仅做**缩放着色**，不新建 35 个 mesh。
//!
//! 坐标换算沿用 0.3.2 参考实现（`_ref/.../operator_models.rs`）的设计像素约定：
//! 点 `(x, y, z) → (x·S, y·S, −z·S)`（S 随 spec 下发），盒中心 `origin + size/2`，
//! 盒尺寸 `size·S`；因 z 取负是一次镜射（行列式 −1，会反转 X/Y 轴旋转手性），
//! 故骨旋转欧拉角按 `(−rx, −ry, rz)` 折算到世界空间。

use std::collections::HashMap;

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use cute_of_duty_contract::model::{VoxelBone, VoxelCube, VoxelModelSpec};

use crate::net::snapshot::CubeMesh;

/// 骨枢轴节点标记：携带骨名，供 `voxel::idle` 按名寻址写入动画旋转。
#[derive(Component)]
pub struct VoxelBoneNode(pub String);

/// 本人模型根标记：表明该实体已按服务端下发的体素模型落成。
///
/// 设计动机（Why）：握手/模型目录与首帧快照分属两路通道，抵达次序不定——本人实体
/// 可能先以方块回退落成、随后目录才到。对账系统据本标记判定"是否已升级为体素模型"，
/// 未升级者拆掉重建，避免本人永久停在方块造型上。
#[derive(Component)]
pub struct VoxelRendered;

/// 体素材质缓存：按**颜色**只建一份，避免反复 spawn 时材质资源单调累积
/// （与 `EntityMaterials` 同思路：造型/配色取值有限，句柄一次成型全场复用）。
#[derive(Resource, Default)]
pub struct VoxelMaterials {
    cache: HashMap<[u8; 4], Handle<StandardMaterial>>,
}

impl VoxelMaterials {
    /// 取某颜色的材质句柄；首次使用才创建。
    fn handle(
        &mut self,
        materials: &mut Assets<StandardMaterial>,
        color: Color,
    ) -> Handle<StandardMaterial> {
        let key = color_key(color);
        if let Some(h) = self.cache.get(&key) {
            return h.clone();
        }
        let h = materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 0.85,
            ..default()
        });
        self.cache.insert(key, h.clone());
        h
    }
}

/// 颜色 → 可哈希键（量化到 8bit 通道，避免浮点相等陷阱）。
fn color_key(c: Color) -> [u8; 4] {
    let s = c.to_srgba();
    [
        (s.red * 255.0) as u8,
        (s.green * 255.0) as u8,
        (s.blue * 255.0) as u8,
        (s.alpha * 255.0) as u8,
    ]
}

/// 设计像素点 → 世界坐标（脚底原点、脸朝 −Z）。
fn px_point(p: [f32; 3], s: f32) -> Vec3 {
    Vec3::new(p[0] * s, p[1] * s, -p[2] * s)
}

/// 设计像素盒 → 世界盒中心（`origin + size/2`）。
fn px_center(origin: [f32; 3], size: [f32; 3], s: f32) -> Vec3 {
    Vec3::new(
        (origin[0] + size[0] * 0.5) * s,
        (origin[1] + size[1] * 0.5) * s,
        -(origin[2] + size[2] * 0.5) * s,
    )
}

/// 设计像素盒 → 世界盒尺寸。
fn px_size(size: [f32; 3], s: f32) -> Vec3 {
    Vec3::new(size[0] * s, size[1] * s, size[2] * s)
}

/// 骨静置旋转（度，YSM XYZ）→ 世界四元数（镜射补偿 `(−rx, −ry, rz)`）。
fn bone_quat(rotation: [f32; 3]) -> Quat {
    Quat::from_euler(
        EulerRot::XYZ,
        (-rotation[0]).to_radians(),
        (-rotation[1]).to_radians(),
        rotation[2].to_radians(),
    )
}

/// 材质键 → 颜色（客户端稳定冷资源，取自 0.3.2 `palette` 的逐盒材质口径）。
///
/// 设计动机（Why）：造型的层次靠**逐盒材质**维系——同一根骨上并存皮肤/毛发/发暗部/
/// 奶油白/护甲/战术暗部等多种材质，只按骨名上色会把这些细节全部抹平成一坨纯色。
/// 色值沿用 0.3.2 色板（`_ref/.../palette.rs`）保证与老版本观感一致；`accent` 取元素
/// 火色，随干员切换重着色时仍是对应元素的强调色。
fn material_color(key: &str) -> Option<Color> {
    Some(match key {
        "skin" => Color::srgb(0.96, 0.80, 0.69),
        "green" => Color::srgb(0.22, 0.32, 0.22),
        "dark" => Color::srgb(0.15, 0.17, 0.15),
        "armor" => Color::srgb(0.45, 0.48, 0.50),
        "boot" => Color::srgb(0.18, 0.14, 0.12),
        "hair" => Color::srgb(0.76, 0.36, 0.12),
        "hair_dark" => Color::srgb(0.55, 0.24, 0.09),
        "cream" => Color::srgb(0.96, 0.88, 0.78),
        "eye" => Color::srgb(0.30, 0.90, 1.00),
        "gun" => Color::srgb(0.30, 0.30, 0.35),
        "accent" => Color::srgb(1.00, 0.45, 0.12),
        _ => return None,
    })
}

/// 兜底着色（几何缺 `mat` 时）：按骨名取基色。
fn bone_color(name: &str) -> Color {
    match name {
        "body" => Color::srgb(0.22, 0.32, 0.22),        // 战术绿
        "head" | "arm_left" | "arm_right" => Color::srgb(0.96, 0.80, 0.69), // 皮肤
        "ear_left" | "ear_right" | "tail_a" | "tail_b" => Color::srgb(0.76, 0.36, 0.12), // 狐橙
        "tail_c" => Color::srgb(0.96, 0.88, 0.78),      // 尾尖奶油白
        "leg_left" | "leg_right" => Color::srgb(0.22, 0.32, 0.22),
        "gun" => Color::srgb(0.30, 0.30, 0.35),         // 枪灰
        _ => Color::srgb(0.55, 0.55, 0.58),
    }
}

/// 单盒配色：优先取服务端下发的 `mat` 材质键；未标注时退回按骨名着色（细节薄片取火色强调）。
fn cube_color(cube: &VoxelCube, bone_name: &str) -> Color {
    if let Some(key) = cube.mat.as_deref() {
        if let Some(color) = material_color(key) {
            return color;
        }
    }
    let s = cube.size;
    let thin = s[0] < 0.6 || s[1] < 0.6 || s[2] < 0.6;
    if thin && bone_name != "gun" {
        Color::srgb(1.00, 0.45, 0.12)
    } else {
        bone_color(bone_name)
    }
}

/// 把一套体素模型挂到已建好的实体根下（根实体承载世界坐标，本函数只加子层级）。
///
/// 根实体由 `snapshot::spawn_body` 生成（已含位置/朝向）；此处遍历骨骼树，
/// 每骨一个枢轴实体、盒作其子实体，子骨相对父骨枢轴定位（旋转天然逐级传递）。
pub fn spawn_voxel_body(
    root: &mut EntityCommands,
    cube: &CubeMesh,
    materials: &mut Assets<StandardMaterial>,
    cache: &mut VoxelMaterials,
    spec: &VoxelModelSpec,
) {
    let s = spec.scale;
    root.with_children(|p| {
        for bone in spec.bones.iter().filter(|b| b.parent.is_none()) {
            build_bone(p, bone, &spec.bones, s, &cube.handle, materials, cache);
        }
    });
}

/// 递归构建一根骨（枢轴实体 + 所属盒 + 子骨）。
fn build_bone(
    parent: &mut ChildBuilder,
    bone: &VoxelBone,
    bones: &[VoxelBone],
    s: f32,
    mesh: &Handle<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    cache: &mut VoxelMaterials,
) {
    let pivot = px_point(bone.pivot, s);
    // 骨实体本地位置 = 本骨枢轴 − 父骨枢轴（无父则相对根原点）。
    let parent_pivot = bone
        .parent
        .as_ref()
        .and_then(|n| bones.iter().find(|b| &b.name == n))
        .map(|b| px_point(b.pivot, s))
        .unwrap_or(Vec3::ZERO);
    let local = pivot - parent_pivot;

    parent
        .spawn((
            Transform::from_translation(local).with_rotation(bone_quat(bone.rotation)),
            Visibility::default(),
            VoxelBoneNode(bone.name.clone()),
        ))
        .with_children(|p| {
            for c in &bone.cubes {
                let center = px_center(c.origin, c.size, s) - pivot;
                let size = px_size(c.size, s);
                let material = cache.handle(materials, cube_color(c, &bone.name));
                p.spawn(PbrBundle {
                    mesh: Mesh3d(mesh.clone()),
                    material: MeshMaterial3d(material),
                    transform: Transform::from_translation(center).with_scale(size),
                    ..default()
                });
            }
            for child in bones
                .iter()
                .filter(|b| b.parent.as_deref() == Some(bone.name.as_str()))
            {
                build_bone(p, child, bones, s, mesh, materials, cache);
            }
        });
}