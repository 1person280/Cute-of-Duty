//! 世界生成：地图布局落地、材质缓存、资产池、靶/拾取物/站点生成

use std::collections::HashMap;
use bevy::prelude::*;
use bevy::pbr::NotShadowCaster;
use crate::map::{GlowKind, GlowSpec, MapLayout, MaterialKind, PickupKind, PickupSpec, Prop, Shape, StationSpec, TargetSpec};
use crate::model::{
    mat_voxel, palette, PlayerCamera,
};
use super::character::{spawn_player, spawn_enemy, CharacterPreset};
use super::camera::{CamPivot, ShoulderPivot, PitchPivot, SpringArm, SpringArmState, PIVOT_HEIGHT, ARM_SHOULDER_X_NORMAL, ARM_EYE_Y_NORMAL, ARM_LEN_NORMAL};
use super::hud::EffectAssets;
use crate::map::training;
use super::common::*;
use super::components::*;

pub(crate) fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Main directional light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 8000.0,
            shadows_enabled: true,
            shadow_depth_bias: 0.02,
            shadow_normal_bias: 0.6,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ, -0.8, 0.5, 0.0,
        )),
        ..default()
    });

    // Fill light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 1500.0,
            shadows_enabled: false,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ, -0.3, -1.5, 0.0,
        )),
        ..default()
    });

    // Corner lights
    for pos in [(15.0, 2.5, 15.0), (-15.0, 2.5, 15.0), (15.0, 2.5, -15.0), (-15.0, 2.5, -15.0)] {
        commands.spawn(PointLightBundle {
            point_light: PointLight {
                intensity: 40000.0,
                color: Color::srgb(0.9, 0.85, 0.75),
                range: 22.0,
                shadows_enabled: false,
                ..default()
            },
            transform: Transform::from_xyz(pos.0, pos.1, pos.2),
            ..default()
        });
    }

    // Player
    let player_pos = Vec3::new(0.0, 0.0, 0.0);
    spawn_player(&mut commands, &mut meshes, &mut materials, player_pos);

    // 越肩相机装配（SpringArm 架构）：
    // CamPivot(脚底, TopLevel 不随模型旋转) → ShoulderPivot(Yaw) → PitchPivot(Pitch)
    //   → SpringArm(右肩偏移 + 后方距离，带碰撞缩回) → Camera
    // 相机本地旋转保持单位，视线始终 = 枢轴前方（与瞄准方向平行越过右肩）
    commands.spawn((
        SpatialBundle { transform: Transform::from_translation(player_pos), ..default() },
        CamPivot,
    )).with_children(|pivot| {
        pivot.spawn((SpatialBundle::default(), ShoulderPivot)).with_children(|yaw| {
            yaw.spawn((
                SpatialBundle { transform: Transform::from_xyz(0.0, PIVOT_HEIGHT, 0.0), ..default() },
                PitchPivot,
            )).with_children(|pitch| {
                pitch.spawn((
                    SpatialBundle {
                        transform: Transform::from_xyz(
                            ARM_SHOULDER_X_NORMAL, ARM_EYE_Y_NORMAL, ARM_LEN_NORMAL),
                        ..default()
                    },
                    SpringArm,
                    SpringArmState { len: ARM_LEN_NORMAL },
                )).with_children(|arm| {
                    arm.spawn((Camera3dBundle::default(), PlayerCamera::default()));
                });
            });
        });
    });

    // AI enemies
    spawn_enemy(&mut commands, &mut meshes, &mut materials, Vec3::new(8.0, 0.0, -8.0), CharacterPreset::EnemyIce);
    spawn_enemy(&mut commands, &mut meshes, &mut materials, Vec3::new(-8.0, 0.0, -6.0), CharacterPreset::TeammateElectric);

    // Training ground
    spawn_training_ground(&mut commands, &mut meshes, &mut materials);

    // 共享特效资产：所有运行时特效（曳光/火花/粒子/爆炸/投掷物）复用
    commands.insert_resource(EffectAssets::new(&mut meshes, &mut materials));
}

// =============================================================================
// Training Ground —— 数据驱动渲染
// =============================================================================

// 地图几何数据由 `crate::map::training` 提供（纯数据，无 bevy 依赖）。
// 本文件只负责把数据渲染成实体，不在这里摆放任何掩体/靶位；
// 调整训练场布局请改 src/map/training/mod.rs。

pub(crate) fn spawn_training_ground(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    spawn_map_layout(&training::layout(), commands, meshes, materials);
}

/// 通用地图渲染器：渲染任意 [`MapLayout`] 为场景实体
pub(crate) fn spawn_map_layout(
    layout: &MapLayout,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    spawn_floor(layout, commands, meshes, materials);

    let mats = MapMaterials::new(materials);
    // 网格/材质全场查池复用：每个独立网格或材质都是一份 GPU 缓冲，
    // 核显上累积过多会 OOM（与棋盘格地面共用网格同一原因）
    let mut pool = AssetPool::default();
    for prop in &layout.props {
        spawn_prop(prop, commands, meshes, &mats, &mut pool);
    }
    for target in &layout.targets {
        spawn_map_target(target, commands, meshes, materials, &mut pool);
    }
    for pickup in &layout.pickups {
        spawn_map_pickup(pickup, commands, meshes, materials, &mut pool);
    }
    for glow in &layout.glows {
        spawn_glow(glow, commands, meshes, materials, &mut pool);
    }
    for station in &layout.stations {
        spawn_map_station(station, commands);
    }
}

/// 棋盘格地面（不含碰撞：地面行走由玩家系统自行处理）
pub(crate) fn spawn_floor(
    layout: &MapLayout,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let ground_a = mat_voxel(materials, palette::GROUND_A);
    let ground_b = mat_voxel(materials, palette::GROUND_B);
    let tile = layout.floor_tile;
    let n = (layout.half_extent / tile).round() as i32;
    // 全部棋盘格共用一个网格：961 块地砖各建一个网格会在核显上
    // 累积上千个 GPU 缓冲，是 OOM 崩溃的主要底噪
    let tile_mesh = meshes.add(Cuboid::new(tile, 0.2, tile));
    for x in -n..=n {
        for z in -n..=n {
            let is_dark = (x + z) % 2 == 0;
            commands.spawn(PbrBundle {
                mesh: tile_mesh.clone(),
                material: if is_dark { ground_a.clone() } else { ground_b.clone() },
                transform: Transform::from_xyz(x as f32 * tile, -0.1, z as f32 * tile),
                ..default()
            }).insert(NotShadowCaster);
        }
    }
}

/// 语义材质映射表：一次构造，全场复用
pub(crate) struct MapMaterials {
    pub(crate) concrete: Handle<StandardMaterial>,
    pub(crate) rust: Handle<StandardMaterial>,
    pub(crate) steel: Handle<StandardMaterial>,
    pub(crate) target_red: Handle<StandardMaterial>,
    pub(crate) target_white: Handle<StandardMaterial>,
    pub(crate) paint_white: Handle<StandardMaterial>,
    pub(crate) dark: Handle<StandardMaterial>,
    pub(crate) pipe: Handle<StandardMaterial>,
    pub(crate) warn_yellow: Handle<StandardMaterial>,
    pub(crate) warn_orange: Handle<StandardMaterial>,
    pub(crate) warn_red: Handle<StandardMaterial>,
}

impl MapMaterials {
    pub(crate) fn new(materials: &mut ResMut<Assets<StandardMaterial>>) -> Self {
        Self {
            concrete: mat_voxel(materials, palette::CONCRETE),
            rust: mat_voxel(materials, palette::RUST),
            steel: mat_voxel(materials, Color::srgb(0.35, 0.37, 0.38)),
            target_red: mat_voxel(materials, palette::TARGET_RED),
            target_white: mat_voxel(materials, palette::TARGET_WHITE),
            paint_white: mat_voxel(materials, Color::srgb(0.9, 0.9, 0.9)),
            dark: mat_voxel(materials, Color::srgb(0.2, 0.2, 0.22)),
            pipe: mat_voxel(materials, Color::srgb(0.5, 0.45, 0.4)),
            warn_yellow: mat_voxel(materials, Color::srgb(0.9, 0.7, 0.15)),
            warn_orange: mat_voxel(materials, Color::srgb(0.85, 0.45, 0.15)),
            warn_red: mat_voxel(materials, Color::srgb(0.85, 0.15, 0.15)),
        }
    }

    pub(crate) fn get(&self, kind: MaterialKind) -> Handle<StandardMaterial> {
        match kind {
            MaterialKind::Concrete => self.concrete.clone(),
            MaterialKind::Rust => self.rust.clone(),
            MaterialKind::Steel => self.steel.clone(),
            MaterialKind::TargetRed => self.target_red.clone(),
            MaterialKind::TargetWhite => self.target_white.clone(),
            MaterialKind::PaintWhite => self.paint_white.clone(),
            MaterialKind::Dark => self.dark.clone(),
            MaterialKind::Pipe => self.pipe.clone(),
            MaterialKind::WarningYellow => self.warn_yellow.clone(),
            MaterialKind::WarningOrange => self.warn_orange.clone(),
            MaterialKind::WarningRed => self.warn_red.clone(),
        }
    }
}

/// 全场共享的网格与材质池：同尺寸/同颜色的重复件只建一份 GPU 缓冲
/// （标线虚线、矮墙段、靶机部件、拾取物外壳等），核显上缓冲过多会 OOM
#[derive(Default)]
pub(crate) struct AssetPool {
    pub(crate) boxes: HashMap<[u32; 3], Handle<Mesh>>,
    pub(crate) cylinders: HashMap<[u32; 2], Handle<Mesh>>,
    pub(crate) spheres: HashMap<u32, Handle<Mesh>>,
    pub(crate) voxel_mats: HashMap<[u32; 4], Handle<StandardMaterial>>,
    pub(crate) emissive_mats: HashMap<[u32; 5], Handle<StandardMaterial>>,
}

impl AssetPool {
    pub(crate) fn box_mesh(&mut self, meshes: &mut ResMut<Assets<Mesh>>, w: f32, h: f32, d: f32) -> Handle<Mesh> {
        self.boxes
            .entry([w.to_bits(), h.to_bits(), d.to_bits()])
            .or_insert_with(|| meshes.add(Cuboid::new(w, h, d)))
            .clone()
    }

    pub(crate) fn cylinder_mesh(&mut self, meshes: &mut ResMut<Assets<Mesh>>, radius: f32, height: f32) -> Handle<Mesh> {
        self.cylinders
            .entry([radius.to_bits(), height.to_bits()])
            .or_insert_with(|| meshes.add(Cylinder::new(radius, height)))
            .clone()
    }

    pub(crate) fn sphere_mesh(&mut self, meshes: &mut ResMut<Assets<Mesh>>, radius: f32) -> Handle<Mesh> {
        self.spheres
            .entry(radius.to_bits())
            .or_insert_with(|| meshes.add(Sphere::new(radius).mesh().ico(2).unwrap()))
            .clone()
    }

    pub(crate) fn voxel_mat(&mut self, materials: &mut ResMut<Assets<StandardMaterial>>, color: Color) -> Handle<StandardMaterial> {
        let key = mat_key(color);
        self.voxel_mats
            .entry(key)
            .or_insert_with(|| mat_voxel(materials, color))
            .clone()
    }

    /// mult：自发光强度倍数（2.0=灯带/拾取物，3.0=靶心，4.0=浮动指示球）
    pub(crate) fn emissive_mat(
        &mut self,
        materials: &mut ResMut<Assets<StandardMaterial>>,
        color: Color,
        mult: f32,
    ) -> Handle<StandardMaterial> {
        let l = color.to_linear();
        let key = [
            l.red.to_bits(),
            l.green.to_bits(),
            l.blue.to_bits(),
            l.alpha.to_bits(),
            mult.to_bits(),
        ];
        self.emissive_mats
            .entry(key)
            .or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: color,
                    emissive: color.to_linear() * mult,
                    metallic: 0.0,
                    perceptual_roughness: 1.0,
                    ..default()
                })
            })
            .clone()
    }
}

/// 材质缓存键：线性 RGBA 的 bit 量化
pub(crate) fn mat_key(color: Color) -> [u32; 4] {
    let l = color.to_linear();
    [l.red.to_bits(), l.green.to_bits(), l.blue.to_bits(), l.alpha.to_bits()]
}

/// 渲染单个静态物体（掩体/标线/装饰/管道），同尺寸网格走池复用
pub(crate) fn spawn_prop(
    prop: &Prop,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mats: &MapMaterials,
    pool: &mut AssetPool,
) {
    let mesh = match prop.shape {
        Shape::Box => pool.box_mesh(meshes, prop.half[0] * 2.0, prop.half[1] * 2.0, prop.half[2] * 2.0),
        Shape::Cylinder { radius, height } => pool.cylinder_mesh(meshes, radius, height),
    };
    let mut transform = Transform::from_translation(Vec3::from(prop.pos));
    if let Some((axis, angle)) = prop.rot {
        transform.rotation = Quat::from_axis_angle(Vec3::from(axis), angle);
    }
    let mut entity = commands.spawn(PbrBundle {
        mesh,
        material: mats.get(prop.material),
        transform,
        ..default()
    });
    entity.insert(NotShadowCaster);
    if prop.solid {
        entity.insert(Collider { half_size: Vec3::from(prop.aabb_half()) });
    }
}

/// 渲染单个靶（静态靶走 dummy，移动靶挂 MovingTarget 组件）
pub(crate) fn spawn_map_target(
    spec: &TargetSpec,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pool: &mut AssetPool,
) {
    let pos = Vec3::from(spec.pos);
    let red = pool.voxel_mat(materials, palette::TARGET_RED);
    let white = pool.voxel_mat(materials, palette::TARGET_WHITE);
    match spec.motion {
        None => {
            spawn_dummy(commands, meshes, materials, pool, pos, red, white, spec.label);
        }
        Some(m) => {
            let board = pool.box_mesh(meshes, 0.8, 0.8, 0.2);
            let inner = pool.box_mesh(meshes, 0.4, 0.4, 0.25);
            commands.spawn((
                SpatialBundle { transform: Transform::from_translation(pos), ..default() },
                MovingTarget { speed: m.speed, range: m.range, origin: pos, direction: m.start_dir },
                TargetDummy { label: spec.label, ..default() },
            )).with_children(|p| {
                p.spawn(PbrBundle { mesh: board, material: red, ..default() });
                p.spawn(PbrBundle { mesh: inner, material: white, ..default() });
            });
        }
    }
}

/// 渲染单个拾取物：数据层道具枚举 → demo 道具与配色
pub(crate) fn spawn_map_pickup(
    spec: &PickupSpec,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pool: &mut AssetPool,
) {
    let (item_type, color) = match spec.kind {
        PickupKind::Ammo { amount } => (PickupType::Ammo { amount }, Color::srgb(0.9, 0.7, 0.2)),
        PickupKind::Health { amount } => (PickupType::Health { amount }, Color::srgb(0.9, 0.2, 0.2)),
        PickupKind::Armor { amount } => (PickupType::Armor { amount }, Color::srgb(0.2, 0.5, 0.9)),
        PickupKind::Grenade { element } => (PickupType::Grenade { element }, element.color()),
        PickupKind::Weapon { element } => (PickupType::Weapon { element }, element.color()),
    };
    spawn_pickup_item(commands, meshes, materials, pool, Vec3::from(spec.pos), item_type, spec.label, color);
}

/// 生成场景功能站点（补给台/干员切换台的交互登记点，桌面几何由 props 提供）
pub(crate) fn spawn_map_station(spec: &StationSpec, commands: &mut Commands) {
    commands.spawn((
        SpatialBundle {
            transform: Transform::from_translation(Vec3::from(spec.pos)),
            ..default()
        },
        Station { kind: spec.kind, label: spec.label },
    ));
}

/// 渲染单个发光件（霓虹灯带/信标/出生光垫）
pub(crate) fn spawn_glow(
    glow: &GlowSpec,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pool: &mut AssetPool,
) {
    let color = match glow.glow {
        GlowKind::Green => Color::srgb(0.1, 0.9, 0.4),
        GlowKind::Orange => Color::srgb(0.9, 0.5, 0.1),
        GlowKind::Red => palette::TARGET_RED,
        GlowKind::SpawnPad => Color::srgb(0.15, 0.85, 0.25),
        GlowKind::Supply => Color::srgb(1.0, 0.65, 0.15),
        GlowKind::Operator => Color::srgb(0.2, 0.9, 0.95),
    };
    let mesh = match glow.shape {
        Shape::Box => pool.box_mesh(meshes, glow.half[0] * 2.0, glow.half[1] * 2.0, glow.half[2] * 2.0),
        Shape::Cylinder { radius, height } => pool.cylinder_mesh(meshes, radius, height),
    };
    commands.spawn(PbrBundle {
        mesh,
        material: pool.emissive_mat(materials, color, 2.0),
        transform: Transform::from_translation(Vec3::from(glow.pos)),
        ..default()
    }).insert(NotShadowCaster);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_dummy(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pool: &mut AssetPool,
    position: Vec3,
    red: Handle<StandardMaterial>,
    white: Handle<StandardMaterial>,
    label: &'static str,
) {
    // 支柱底端贴地：高台等架空靶位自动获得更长支撑
    let pole_y = 0.75 - position.y;
    let board = pool.box_mesh(meshes, 1.2, 1.2, 0.3);
    let inner = pool.box_mesh(meshes, 0.6, 0.6, 0.35);
    let core = pool.box_mesh(meshes, 0.2, 0.2, 0.4);
    let pole = pool.box_mesh(meshes, 0.2, 1.5, 0.2);
    let core_mat = pool.emissive_mat(materials, Color::srgb(1.0, 0.0, 0.0), 3.0);
    let pole_mat = pool.voxel_mat(materials, Color::srgb(0.4, 0.4, 0.4));

    commands.spawn((
        SpatialBundle {
            transform: Transform::from_translation(position),
            ..default()
        },
        TargetDummy { label, ..default() },
    )).with_children(|p| {
        p.spawn(PbrBundle {
            mesh: board,
            material: red,
            ..default()
        });
        p.spawn(PbrBundle {
            mesh: inner,
            material: white,
            ..default()
        });
        p.spawn(PbrBundle {
            mesh: core,
            material: core_mat,
            ..default()
        });
        p.spawn(PbrBundle {
            mesh: pole,
            material: pole_mat,
            transform: Transform::from_xyz(0.0, pole_y, 0.0),
            ..default()
        });
    });
}

pub(crate) fn spawn_pickup_item(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pool: &mut AssetPool,
    position: Vec3,
    item_type: PickupType,
    name: &str,
    color: Color,
) {
    let (mesh_size, glow_color) = match &item_type {
        PickupType::Ammo { .. } => (Vec3::new(0.5, 0.35, 0.35), Color::srgb(1.0, 0.85, 0.3)),
        PickupType::Health { .. } => (Vec3::new(0.4, 0.25, 0.4), Color::srgb(1.0, 0.3, 0.3)),
        PickupType::Armor { .. } => (Vec3::new(0.4, 0.3, 0.5), Color::srgb(0.3, 0.6, 1.0)),
        PickupType::Grenade { element } => (Vec3::new(0.32, 0.32, 0.32), element.color()),
        PickupType::Weapon { element } => (Vec3::new(0.18, 0.18, 0.9), element.color()),
    };
    let body = pool.box_mesh(meshes, mesh_size.x, mesh_size.y, mesh_size.z);
    let body_mat = pool.emissive_mat(materials, color, 2.0);
    let orb = pool.sphere_mesh(meshes, 0.08);
    let orb_mat = pool.emissive_mat(materials, glow_color, 4.0);
    commands.spawn((
        PbrBundle {
            mesh: body,
            material: body_mat,
            transform: Transform::from_translation(position),
            ..default()
        },
        PickupItem { name: name.to_string(), item_type: item_type.clone() },
        Collider { half_size: Vec3::new(mesh_size.x * 0.5, mesh_size.y * 0.5, mesh_size.z * 0.5) },
    )).with_children(|p| {
        // Floating indicator
        p.spawn((
            PbrBundle {
                mesh: orb,
                material: orb_mat,
                transform: Transform::from_xyz(0.0, mesh_size.y * 0.5 + 0.2, 0.0),
                ..default()
            },
        ));
    });
}

// =============================================================================
// Character Spawners
// =============================================================================

