//! 3D 世界子模块 —— 场景几何、相机跟随、干员体素模型
//!
//! 设计动机（Why）：训练场几何与相机枢轴是表现层装配；实体身份（干员型号）与血量等
//! 判定由服务端权威，本模块只按快照 model_preset 选体素网格、按稳定色板上材质。
//!
//! 命名约定：单词语义概念直接作文件名；两词概念单开子目录（如 `voxel/model.rs`），
//! 不再用下划线拼接文件名。

pub(crate) mod camera;
pub(crate) mod grenade;
pub(crate) mod model;
pub(crate) mod scene;
pub(crate) mod voxel;

pub(crate) use camera::{follow_system, mouse_look_system, spawn_camera, sync_grenade_aim};
pub(crate) use grenade::preview::draw_grenade_preview;
pub(crate) use scene::{spawn_scene_baseline, spawn_world_when_ready};
pub(crate) use voxel::facing::face_aim_direction;
pub(crate) use voxel::idle::drive_idle;
pub(crate) use voxel::model::VoxelMaterials;