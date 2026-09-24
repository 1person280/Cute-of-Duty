//! 静态环境：地面、光照与若干掩体
//!
//! 设计动机：环境是「不易变」的稳定内容，正应由客户端承载。这里只摆放无需网络
//! 同步的静态几何与灯光；所有**会动/会变**的东西一律走服务端快照（见 `snapshot.rs`）。

use bevy::prelude::*;

/// 生成地面（大号扁立方）、环境光 + 平行光，以及一排静态掩体。
pub fn spawn_world(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // 地面
    let ground_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.16, 0.18, 0.16),
        perceptual_roughness: 0.9,
        ..default()
    });
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(60.0, 0.3, 60.0)),
            material: ground_mat,
            transform: Transform::from_xyz(0.0, -0.15, 0.0),
            ..default()
        },
    ));

    // 平行光（主光）做主要照明（bevy 0.14 为 DirectionalLightBundle，阴影开关是 shadows_enabled）
    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                illuminance: 18_000.0,
                shadows_enabled: true,
                ..default()
            },
            transform: Transform::from_xyz(12.0, 20.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
    ));

    // 静态掩体：绕原点摆一圈矮墙，给出场景纵深
    let obstacle_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.30, 0.30, 0.33),
        ..default()
    });
    let wall = meshes.add(Cuboid::new(2.0, 1.4, 0.5));
    for i in 0..6u32 {
        let angle = std::f32::consts::TAU * i as f32 / 6.0;
        let r = 8.0;
        commands.spawn((
            PbrBundle {
                mesh: wall.clone(),
                material: obstacle_mat.clone(),
                transform: Transform::from_xyz(r * angle.cos(), 0.7, r * angle.sin()),
                ..default()
            },
        ));
    }
}