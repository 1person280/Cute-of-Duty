//! 3D 世界子模块 —— 场景几何、相机跟随、干员体素模型
//!
//! 设计动机（Why）：训练场几何与相机枢轴是表现层装配；实体身份（干员型号）与血量等
//! 判定由服务端权威，本模块只按快照 model_preset 选体素网格、按稳定色板上材质。

pub(crate) mod camera;
pub(crate) mod model;
pub(crate) mod world_scene;

pub(crate) use camera::{follow_system, mouse_look_system, spawn_camera};
pub(crate) use world_scene::spawn_world;
