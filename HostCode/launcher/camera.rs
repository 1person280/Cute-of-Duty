//! 观察相机：绕原点自动缓转的轨道机位
//!
//! 设计动机：渲染冒烟无需玩家操控，固定轨道便于肉眼观察服务端快照驱动的实体
//! 在场景中移动。视角、距离为本地表现层设定，不属于任何服务端权威数据。

use bevy::prelude::*;

/// 轨道相机标记（每帧由 `orbit_system` 重写世界变换）。
#[derive(Component)]
pub struct OrbitCamera;

/// 轨道半径、轨道高度与角速度（弧度/秒）。
const ORBIT_RADIUS: f32 = 22.0;
const ORBIT_HEIGHT: f32 = 14.0;
const ORBIT_SPEED: f32 = 0.18;

/// 生成轨道相机（默认看向坐标原点，即训练场中心）。
///
/// bevy 0.14 用 `Camera3dBundle` 承载相机（相机渲染图/投影/可见性等一并装配），
/// 不能像 0.19 那样只挂 `Camera3d` 组件 + 自定义 `Transform`。
pub fn spawn_camera(commands: &mut Commands) {
    commands.spawn((
        OrbitCamera,
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, ORBIT_HEIGHT, ORBIT_RADIUS)
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
    ));
}

/// 每帧沿水平面驱动相机绕原点缓转，保持俯视角度不变。
pub fn orbit_system(mut query: Query<(&mut Transform, &OrbitCamera)>, time: Res<Time>) {
    for (mut tf, _) in &mut query {
        let angle = time.elapsed_seconds() * ORBIT_SPEED;
        tf.translation.x = ORBIT_RADIUS * angle.cos();
        tf.translation.z = ORBIT_RADIUS * angle.sin();
        tf.translation.y = ORBIT_HEIGHT;
        tf.look_at(Vec3::ZERO, Vec3::Y);
    }
}