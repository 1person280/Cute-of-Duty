//! launcher 模块 —— 客户端唯一承载模块（服务器权威架构的表现半边）
//!
//! 设计动机：全客户端逻辑收敛于此 —— 既有后台网络拉快照（`network`），
//! 也有 Bevy 表现主循环出画（`snapshot`/`model`/`camera`/`world`）。本层只消费
//! 服务端权威快照，绝不本地模拟；bevy 以动态库（bevy_dylib）装载，与本 exe 同发。

use std::sync::mpsc;

use bevy::prelude::*;
use cute_of_duty_server::net::protocol::EntitySnapshot;

pub(crate) mod camera;
pub(crate) mod model;
pub(crate) mod network;
pub(crate) mod snapshot;
pub(crate) mod world;

/// 入口：建立快照通道、起后台网络拉取线程、跑 Bevy 表现主循环。
///
/// `addr` 为服务端监听地址（如 `127.0.0.1:8888`）。
pub fn run(addr: &str) {
    let (snapshot_tx, snapshot_rx) = mpsc::channel::<Vec<EntitySnapshot>>();
    let net_addr = addr.to_string();
    std::thread::spawn(move || network::run_pull_loop(&net_addr, snapshot_tx));

    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(snapshot::SnapshotBuffer::new(snapshot_rx))
        .add_systems(Startup, spawn_scene)
        .add_systems(
            Update,
            (
                snapshot::receive_snapshots,
                snapshot::apply_entities,
                camera::orbit_system,
            ),
        )
        .run();
}

/// 场景初始化：轨道相机 + 静态环境 + 共享体素网格。
fn spawn_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    camera::spawn_camera(&mut commands);
    world::spawn_world(&mut commands, &mut meshes, &mut materials);
    commands.insert_resource(snapshot::CubeMesh {
        handle: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
    });
}