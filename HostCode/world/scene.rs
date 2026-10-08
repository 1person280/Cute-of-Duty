//! 静态世界生成：完整草坪训练场（1×1km 露天搜打撤大场）+ 光照
//!
//! 设计动机（服务器权威边界）：环境是「不易变」的稳定内容，正应由客户端承载。
//! 这里消费服务端经 `WorldCatalog` 下发的**纯数据布局**（不触碰任何服务端模拟逻辑），
//! 把它翻译成本地 bevy 静态 mesh —— 只渲染**不变**的部分（地板 / 静态 props / 发光件）。
//! 一切**会动/会变**的东西（靶、拾取物、玩家、敌人）仍由服务端经快照下发、
//! `snapshot.rs` 负责绘制，本模块绝不重复生成，避免双份实体。
//!
//! 活动地图选择：0.3.2 稳定版运行时渲染的即 `map::lawn`（南端出生、北端撤离信标），
//! 本模块按其原版光照参数（主光 8000 + 补光 1500 + 四角点光 80000/60m）复刻观感。

use bevy::prelude::*;
use bevy::pbr::NotShadowCaster;
use cute_of_duty_contract::map::{GlowKind, GlowSpec, MapLayout, MaterialKind, Prop, Shape};

use crate::flow::WorldCatalog;

/// 场景**基础装配**（`Startup` 一次性）：相机 + 共享网格资源 + 全局环境光。
///
/// 设计动机（Why）：这三样**不依赖地图布局**，且是 Update 早期系统的硬依赖——
/// `net::apply_entities` 取 `Res<CubeMesh>`、`menu::settings_apply_ambient` 取 `ResMut<AmbientLight>`，
/// 缺任一资源即 panic。故它们必须留在 `Startup`（目录尚未到达也先备好），
/// 不能随地图几何一起延后。地图几何与光照见 [`spawn_world_when_ready`]。
pub fn spawn_scene_baseline(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    crate::world::spawn_camera(&mut commands);
    commands.insert_resource(crate::net::CubeMesh {
        handle: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
    });
    // 全局环境光：供「设置浮层 · 环境亮度」调节写入（世界仅配了 DirectionalLight）。
    // bevy 0.16 起 `AmbientLight.brightness` 与光照强度同量纲（lux，典型值数百到数千），
    // 0.15 时代的 0.55 倍率在新量纲下≈全黑（背光面纯黑、盒子底面纯黑，即 0.14.1 起的
    // 「往南走渲染错乱」根因）。此处取逻辑默认 0.55 × 4000 = 2200 lux 作首帧兜底，
    // 正式值由 `settings_apply_ambient` 在启动帧覆盖（见 flow/settings.rs 的量纲映射）。
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 2_200.0,
        // 0.16 新增字段：本世界无光照贴图网格，取默认 true 保持既有全局环境光语义。
        affects_lightmapped_meshes: true,
    });
}

/// 场景**几何装配**（`Update`，就绪首帧恰好一次）：布局到达后生成光照 + 静态地图。
///
/// 设计动机（Why）：地图布局改为服务端下发后不再有编译期静态表可用，`Startup` 又早于连接
/// （只跑一次，门控永远不通过）；故改挂 `Update` 并用 `Local` 守卫实现「就绪首帧恰好一次」。
/// 未就绪时直接跳过——光照/几何稍后补齐，期间主菜单为整屏不透明层，无观感影响。
pub fn spawn_world_when_ready(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    catalog: Res<WorldCatalog>,
    mut spawned: Local<bool>,
) {
    if *spawned {
        return;
    }
    let Some(layout) = catalog.layout.as_ref() else {
        return; // 目录未到达：等下一帧
    };
    *spawned = true;
    spawn_world(&mut commands, &mut meshes, &mut materials, layout);
}

/// 场景主光照（主平行光 + 补光 + 四角点光，参数对齐 0.3.2 `demo/world.rs`）。
///
/// bevy 0.15：光源直接 spawn `DirectionalLight`/`PointLight` + `Transform`
/// （required components 自动补齐 `Cascades`/`Visibility` 等），阴影开关是 `shadows_enabled`。
pub fn spawn_world(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    layout: &MapLayout,
) {
    // 主平行光（0.3.2 数值：8000 lux，硬阴影）
    commands.spawn((
        DirectionalLight {
            illuminance: 8_000.0,
            shadows_enabled: true,
            shadow_depth_bias: 0.02,
            shadow_normal_bias: 0.6,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -0.8,
            0.5,
            0.0,
        )),
    ));

    // 补光（无阴影，压低阴影死黑，保持旧版露天观感）
    commands.spawn((
        DirectionalLight {
            illuminance: 1_500.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -0.3,
            -1.5,
            0.0,
        )),
    ));

    // 角部点光：随活动地图（1×1km 草坪场）的四角布置（0.3.2 数值 80000/60m）。
    let corner = layout.half_extent;
    for pos in [(corner, 2.5, corner), (-corner, 2.5, corner), (corner, 2.5, -corner), (-corner, 2.5, -corner)] {
        commands.spawn((
            PointLight {
                intensity: 80_000.0,
                color: Color::srgb(0.9, 0.85, 0.75),
                range: 60.0,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_xyz(pos.0, pos.1, pos.2),
        ));
    }

    // 完整草坪训练场（静态部分：地板 + props + glows）
    spawn_map_layout(layout, commands, meshes, materials);
}

/// 通用地图渲染器：把任意 `MapLayout` 的静态层落地为 bevy 实体。
///
/// 为什么只渲染这三层：`targets`/`pickups` 与玩家一样是**服务端权威的动态实体**，
/// 会经快照进入 `snapshot.rs` 对账；若这里再画一遍会出现双份。`stations` 仅登记
/// 交互位，其桌面几何已在 `props` 里，改坐标只动 `map/lawn/*`。
fn spawn_map_layout(
    layout: &MapLayout,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    spawn_floor(layout, commands, meshes, materials);

    // 语义材质回收：一种料只建一份 GPU 缓冲，核显上避免累积 OOM（0.3.2 教训）。
    let mats = MapMaterials::new(materials);
    // 共享网格池：同尺寸的方块/圆柱只建一次 mesh。
    let mut pool = MeshPool::default();
    for prop in &layout.props {
        spawn_prop(prop, commands, meshes, &mats, &mut pool);
    }
    for glow in &layout.glows {
        spawn_glow(glow, commands, meshes, materials, &mut pool);
    }
}

/// 棋盘格地面：全部共用一个 `Cuboid` 网格，铺满 `[-half, half]`。
fn spawn_floor(
    layout: &MapLayout,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let ground_a = mat_voxel(materials, GROUND_A);
    let ground_b = mat_voxel(materials, GROUND_B);
    let tile = layout.floor_tile;
    let n = (layout.half_extent / tile).round() as i32;
    let tile_mesh = meshes.add(Cuboid::new(tile, 0.2, tile));
    for x in -n..=n {
        for z in -n..=n {
            let is_dark = (x + z) % 2 == 0;
            commands
                .spawn((
                    Mesh3d(tile_mesh.clone()),
                    MeshMaterial3d(if is_dark { ground_a.clone() } else { ground_b.clone() }),
                    Transform::from_xyz(
                        x as f32 * tile,
                        -0.1,
                        z as f32 * tile,
                    ),
                ))
                .insert(NotShadowCaster);
        }
    }
}

/// 语义材质表：一次性构造，全场复用（颜色取自 0.3.2 `model/palette.rs`）。
struct MapMaterials {
    concrete: Handle<StandardMaterial>,
    rust: Handle<StandardMaterial>,
    steel: Handle<StandardMaterial>,
    target_red: Handle<StandardMaterial>,
    target_white: Handle<StandardMaterial>,
    paint_white: Handle<StandardMaterial>,
    dark: Handle<StandardMaterial>,
    pipe: Handle<StandardMaterial>,
    warn_yellow: Handle<StandardMaterial>,
    warn_orange: Handle<StandardMaterial>,
    warn_red: Handle<StandardMaterial>,
}

impl MapMaterials {
    fn new(materials: &mut ResMut<Assets<StandardMaterial>>) -> Self {
        Self {
            // 全部走稳定色板：客户端不再挂概念美术原图（1920²~2560×1440 贴图既撑爆显存，
            // 又与本项目"体素低模"美术方向相悖），仅以底色出画。
            concrete: mat_voxel(materials, CONCRETE),
            rust: mat_voxel(materials, RUST),
            steel: mat_voxel(materials, STEEL),
            target_red: mat_voxel(materials, TARGET_RED),
            target_white: mat_voxel(materials, TARGET_WHITE),
            paint_white: mat_voxel(materials, PAINT_WHITE),
            dark: mat_voxel(materials, DARK),
            pipe: mat_voxel(materials, PIPE),
            warn_yellow: mat_voxel(materials, WARN_YELLOW),
            warn_orange: mat_voxel(materials, WARN_ORANGE),
            warn_red: mat_voxel(materials, WARN_RED),
        }
    }

    fn get(&self, kind: MaterialKind) -> Handle<StandardMaterial> {
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

/// 共享网格池：同尺寸立方/圆柱只建一份，返回句柄（键用 f32 bit 量化去重）。
#[derive(Default)]
struct MeshPool {
    boxes: std::collections::HashMap<[u32; 3], Handle<Mesh>>,
    cylinders: std::collections::HashMap<[u32; 2], Handle<Mesh>>,
}

impl MeshPool {
    fn box_mesh(&mut self, meshes: &mut ResMut<Assets<Mesh>>, w: f32, h: f32, d: f32) -> Handle<Mesh> {
        self.boxes
            .entry([w.to_bits(), h.to_bits(), d.to_bits()])
            .or_insert_with(|| meshes.add(Cuboid::new(w, h, d)))
            .clone()
    }

    fn cylinder_mesh(&mut self, meshes: &mut ResMut<Assets<Mesh>>, radius: f32, height: f32) -> Handle<Mesh> {
        self.cylinders
            .entry([radius.to_bits(), height.to_bits()])
            .or_insert_with(|| meshes.add(Cylinder::new(radius, height)))
            .clone()
    }
}

/// 渲染单个静态物体：Box → 立方体，Cylinder → 直立圆柱；支持轴角旋转。
fn spawn_prop(
    prop: &Prop,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mats: &MapMaterials,
    pool: &mut MeshPool,
) {
    let mesh = match prop.shape {
        Shape::Box => pool.box_mesh(meshes, prop.half[0] * 2.0, prop.half[1] * 2.0, prop.half[2] * 2.0),
        Shape::Cylinder { radius, height } => pool.cylinder_mesh(meshes, radius, height),
    };
    let mut transform = Transform::from_translation(Vec3::from(prop.pos));
    if let Some((axis, angle)) = prop.rot {
        transform.rotation = Quat::from_axis_angle(Vec3::from(axis), angle);
    }
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(mats.get(prop.material)),
        transform,
    ));
}

/// 渲染单个发光件：霓虹灯带 / 信标 / 出生光垫（自发光材质）。
fn spawn_glow(
    glow: &GlowSpec,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pool: &mut MeshPool,
) {
    let color = glow_color(glow.glow);
    let mesh = match glow.shape {
        Shape::Box => pool.box_mesh(meshes, glow.half[0] * 2.0, glow.half[1] * 2.0, glow.half[2] * 2.0),
        Shape::Cylinder { radius, height } => pool.cylinder_mesh(meshes, radius, height),
    };
    // 自发光材质：基色置暗、emissive 提亮 2 倍，营造"灯/光垫"氛围。
    let mat = materials.add(StandardMaterial {
        base_color: color,
        emissive: color.to_linear() * 2.0,
        metallic: 0.0,
        perceptual_roughness: 1.0,
        ..default()
    });
    commands
        .spawn((
            Mesh3d(mesh),
            MeshMaterial3d(mat),
            Transform::from_translation(Vec3::from(glow.pos)),
        ))
        .insert(NotShadowCaster);
}

/// `GlowKind` → 具体发光色（沿 0.3.2 数值）。
fn glow_color(kind: GlowKind) -> Color {
    match kind {
        GlowKind::Green => Color::srgb(0.10, 0.90, 0.40),
        GlowKind::Orange => Color::srgb(0.90, 0.50, 0.10),
        GlowKind::Red => TARGET_RED,
        GlowKind::SpawnPad => Color::srgb(0.15, 0.85, 0.25),
        GlowKind::Supply => Color::srgb(1.00, 0.65, 0.15),
        GlowKind::Operator => Color::srgb(0.20, 0.90, 0.95),
    }
}

/// 便捷：以某色建一块无金属粗粒材质（供地面/语义材复用）。
fn mat_voxel(materials: &mut ResMut<Assets<StandardMaterial>>, color: Color) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color,
        metallic: 0.0,
        perceptual_roughness: 0.9,
        ..default()
    })
}

// —— 地图色板（取自 0.3.2 `model/palette.rs` + 语义材质常量，稳定冷资源）——
const GROUND_A: Color = Color::srgb(0.32, 0.36, 0.28);
const GROUND_B: Color = Color::srgb(0.24, 0.27, 0.22);
const CONCRETE: Color = Color::srgb(0.50, 0.50, 0.54);
const RUST: Color = Color::srgb(0.60, 0.35, 0.22);
const STEEL: Color = Color::srgb(0.35, 0.37, 0.38);
const TARGET_RED: Color = Color::srgb(0.85, 0.15, 0.15);
const TARGET_WHITE: Color = Color::srgb(0.92, 0.92, 0.92);
const PAINT_WHITE: Color = Color::srgb(0.90, 0.90, 0.90);
const DARK: Color = Color::srgb(0.20, 0.20, 0.22);
const PIPE: Color = Color::srgb(0.50, 0.45, 0.40);
const WARN_YELLOW: Color = Color::srgb(0.90, 0.70, 0.15);
const WARN_ORANGE: Color = Color::srgb(0.85, 0.45, 0.15);
const WARN_RED: Color = Color::srgb(0.85, 0.15, 0.15);