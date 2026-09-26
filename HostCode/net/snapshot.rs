//! 实体渲染跟随：把服务端权威快照映射为场景中的体素造型
//!
//! 设计动机（服务器权威）：本系统**只消费** `EntitySnapshot`（位置/存活/模型身份均为
//! 服务端裁决值），为已存在的实体更新坐标系、为新实体生成造型、为消失实体销毁，
//! 绝不做任何本地校订 —— 客户端只是服务端世界的一块"绘画镜面"。

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use cute_of_duty_server::model::ModelPreset;
use cute_of_duty_server::net::protocol::EntitySnapshot;

use crate::world::model::voxel_for;

/// 已渲染实体的根标记：记住服务端实体 ID，供跨帧对账。
#[derive(Component)]
pub struct RenderedEntity {
    pub id: u64,
}

/// 共享体素网格（所有造型复用同一单位立方体，仅以缩放+材质区分）。
#[derive(Resource)]
pub struct CubeMesh {
    pub handle: Handle<Mesh>,
}

/// 造型材质缓存：每个 `ModelPreset` 只建一份主色/强调色材质。
///
/// 设计动机（Why）：实体随 AOI 进出视野会被反复 `spawn_body`；若每次都对
/// `Assets<StandardMaterial>` 调 `add`，材质资源会随实体增删**单调累积**（GPU 缓冲
/// 永不释放），长时间游玩即内存涨到 GB、帧时间崩坏（表现为"延迟"飙升）。
/// 造型身份是服务端权威且取值有限（`ModelPreset` 枚举），故按 preset 缓存句柄一次成型。
#[derive(Resource, Default)]
pub struct EntityMaterials {
    cache: HashMap<ModelPreset, (Handle<StandardMaterial>, Handle<StandardMaterial>)>,
}

impl EntityMaterials {
    /// 取某造型的（主色, 强调色）材质句柄；首次使用才创建，之后全场复用。
    fn handles_for(
        &mut self,
        materials: &mut Assets<StandardMaterial>,
        preset: ModelPreset,
    ) -> (Handle<StandardMaterial>, Handle<StandardMaterial>) {
        if let Some(pair) = self.cache.get(&preset) {
            return pair.clone();
        }
        let body = voxel_for(preset);
        let primary = materials.add(StandardMaterial {
            base_color: body.primary,
            perceptual_roughness: 0.7,
            ..default()
        });
        let accent = materials.add(StandardMaterial {
            base_color: body.accent,
            emissive: body.accent.into(),
            perceptual_roughness: 0.6,
            ..default()
        });
        let pair = (primary, accent);
        self.cache.insert(preset, pair.clone());
        pair
    }
}

/// 服务端快照来源（背景网络线程持续灌入最新一帧实体列表）。
///
/// `std::sync::mpsc::Receiver` 非 `Sync`，而 bevy 资源要求 `Send + Sync`，
/// 故以 `Mutex` 包裹；渲染单线程读，锁竞争可忽略。
#[derive(Resource)]
pub struct SnapshotBuffer {
    pub rx: std::sync::Mutex<std::sync::mpsc::Receiver<Vec<EntitySnapshot>>>,
    pub current: Vec<EntitySnapshot>,
}

impl SnapshotBuffer {
    pub fn new(rx: std::sync::mpsc::Receiver<Vec<EntitySnapshot>>) -> Self {
        Self {
            rx: std::sync::Mutex::new(rx),
            current: Vec::new(),
        }
    }
}

/// 每帧排空通道，仅保留最新一帧快照。
pub fn receive_snapshots(mut buffer: ResMut<SnapshotBuffer>) {
    // 先取最新帧（持锁只做通道读取），释放借用后再写入 —— 避免与 `current` 可变借用冲突
    let mut latest: Option<Vec<EntitySnapshot>> = None;
    {
        let rx = buffer.rx.lock().unwrap();
        while let Ok(frame) = rx.try_recv() {
            latest = Some(frame);
        }
    }
    if let Some(frame) = latest {
        buffer.current = frame;
    }
}

/// 对账：更新坐标（服务端权威）→ 生成缺失 → 销毁消失。
#[allow(clippy::type_complexity)]
pub fn apply_entities(
    mut commands: Commands,
    buffer: Res<SnapshotBuffer>,
    cube: Res<CubeMesh>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut entity_mats: ResMut<EntityMaterials>,
    mut query: Query<(Entity, &mut Transform, &RenderedEntity)>,
) {
    // 快照里仍然存活/可见的 ID（AOI 过滤后仅服务端下发的就是应渲染的）
    let present: HashSet<u64> = buffer.current.iter().map(|e| e.entity_id).collect();

    // 1) 同步既有实体的坐标；收集本地 ID 以便随后判定缺失
    let mut local_ids: HashSet<u64> = HashSet::new();
    for (_, mut tf, rendered) in query.iter_mut() {
        local_ids.insert(rendered.id);
        if let Some(entry) = buffer.current.iter().find(|e| e.entity_id == rendered.id) {
            // 位置以服务端权威为准，直接套用
            tf.translation = Vec3::new(entry.x, entry.y, entry.z);
        }
    }

    // 2) 生成快照里有、本地还没有的实体
    for entry in &buffer.current {
        if !local_ids.contains(&entry.entity_id) {
            spawn_body(&mut commands, &cube, &mut materials, &mut entity_mats, entry);
        }
    }

    // 3) 销毁本地已有、但快照里已消失的实体（不留幽灵）
    for (entity, _, rendered) in query.iter() {
        if !present.contains(&rendered.id) {
            commands.entity(entity).despawn();
        }
    }
}

/// 依据模型身份生成一个体素造型（脚底对齐服务端坐标 y）。
///
/// 注意（B0004 教训）：根实体会以 `with_children` 挂可渲染的 `Mesh3d` 子级，
/// 父实体必须显式补 `Visibility::default()`，否则子级继承层级损坏（B0004 告警）。
fn spawn_body(
    commands: &mut Commands,
    cube: &CubeMesh,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    entity_mats: &mut ResMut<EntityMaterials>,
    entry: &EntitySnapshot,
) {
    let body = voxel_for(entry.model_preset);
    // 材质按造型身份复用（见 `EntityMaterials`）：实体反复增删不再累积材质资源。
    let (primary, accent) = entity_mats.handles_for(materials, entry.model_preset);

    let mut root = commands.spawn((
        RenderedEntity { id: entry.entity_id },
        Visibility::default(), // 父补可见性，预防 B0004 层级损坏
        Transform::from_translation(Vec3::new(entry.x, entry.y, entry.z)),
    ));

    root.with_children(|p| {
        if body.head_scale == Vec3::ZERO {
            // 物体类：单方块
            p.spawn((
                PbrBundle {
                    mesh: cube.handle.clone(),
                    material: primary,
                    transform: Transform::from_scale(body.torso_scale),
                    ..default()
                },
            ));
        } else {
            // 角色类：躯干 + 头（脚底对齐 y=0，躯干块中心抬至 y=0.75）
            p.spawn((
                PbrBundle {
                    mesh: cube.handle.clone(),
                    material: primary,
                    transform: Transform::from_translation(Vec3::new(0.0, 0.75, 0.0))
                        .with_scale(body.torso_scale),
                    ..default()
                },
            ));
            p.spawn((
                PbrBundle {
                    mesh: cube.handle.clone(),
                    material: accent,
                    transform: Transform::from_translation(Vec3::new(0.0, body.head_y, 0.0))
                        .with_scale(body.head_scale),
                    ..default()
                },
            ));
        }
    });
}