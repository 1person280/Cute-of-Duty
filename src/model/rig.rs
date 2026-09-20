//! 玩家模型骨骼枢轴组件（来自 src/model/mod.rs 原"动作相关组件"段）
//! - PlayerHeadPivot / PlayerAimGun：头/枪枢轴，供瞄准与后坐系统叠加旋转
//! - YanhuLimb + LimbKind：四肢/尾链枢轴类别，动作系统与各干员构建共用同一约定
//! - PlayerModelRoot：模型根，记录当前干员序号，供模型置换系统 despawn/重建

use bevy::prelude::*;

#[derive(Component)]
/// 玩家头部枢轴（Aim Offset：瞄准时头部俯仰跟随视线）
pub struct PlayerHeadPivot;

#[derive(Component)]
/// 玩家持枪位：瞄准时从腰际举到肩上（无动画系统，用程序化过渡代替 Upper Body Layer）
pub struct PlayerAimGun { pub base: Vec3, pub raised: Vec3 }

// —— 程序化动作（yanhu_action_system 驱动） ——
/// 四肢/尾巴枢轴类别：行走摆动相位、瞄准姿态、尾巴摇摆均按此分支
#[derive(Component, Clone, Copy)]
pub struct YanhuLimb(pub(crate) LimbKind);

#[derive(Clone, Copy)]
pub(crate) enum LimbKind { ArmL, ArmR, LegL, LegR, TailA, TailB, TailC }

impl YanhuLimb {
    pub(crate) fn new(kind: LimbKind) -> Self { Self(kind) }
}

/// 玩家模型根：记录当前模型对应的干员序号，
/// 切干员时 operator_model_swap_system 据此 despawn 旧模型、按新干员重建
#[derive(Component)]
pub struct PlayerModelRoot { pub op_idx: usize }