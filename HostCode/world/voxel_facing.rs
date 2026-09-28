//! 本人模型朝向同步（表现层跟随视线偏航）
//!
//! 设计动机（Why，"永远向北"的根因）：模型根实体在 `snapshot::spawn_body` 落成时只写了一次
//! 固定旋转，此后无人更新——镜头转动时角色纹丝不动。老版本（0.3.2 `controller::fps_controller`）
//! 每帧把 `model_yaw`（= 相机偏航）写进模型根旋转，角色随之转身。本系统复刻该行为：
//! 读共享的 [`AimRig::yaw`]（`mouse_look_system` 每帧写入的权威瞄准偏航）同步本人模型根朝向。
//!
//! 轴系约定：几何换算把设计坐标的 z 取负（见 `voxel_model` 模块头），模型**本地正面为 +Z**，
//! 故根旋转直接取 `from_rotation_y(yaw)` 即与相机视线同向（默认 `yaw=π` 时面向 -Z 北方）。

use bevy::prelude::*;

use crate::flow::flow_state::AimRig;
use crate::net::snapshot::RenderedEntity;
use crate::world::voxel_model::VoxelRendered;

/// 每帧把本人体素模型根的旋转对齐到当前视线偏航（第三人称「人随视线转」）。
///
/// 只作用于已升级为体素模型的本人实体（`VoxelRendered`）——敌人/靶机的方块造型朝向
/// 不由客户端裁决，保持现状。
pub fn face_aim_direction(
    rig: Res<AimRig>,
    mut query: Query<&mut Transform, (With<RenderedEntity>, With<VoxelRendered>)>,
) {
    let rotation = Quat::from_rotation_y(rig.yaw);
    for mut tf in &mut query {
        tf.rotation = rotation;
    }
}