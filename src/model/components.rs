//! 玩家/干员玩法驱动组件（来自 src/model/mod.rs 原组件段）
//! demo 玩法层通过这些组件驱动模型与相机，不包含任何玩法逻辑。

use bevy::prelude::*;

use crate::operator::roster;

#[derive(Component)]
pub struct PlayerCamera {
    pub yaw: f32,
    pub pitch: f32,
    /// 越肩瞄准姿态开关：右键按住或手持手雷时为真（aim_system 每帧写入）
    pub aiming: bool,
    /// 瞄准过渡系数 0..1：0=普通肩后环绕，1=贴肩瞄准，update_camera 按此插值机位
    pub aim_lerp: f32,
}
impl Default for PlayerCamera {
    /// yaw = π：出生面向 -Z（训练场射击道），相机轨道在身后 +Z。
    /// 之前是 0，出生面向场外开阔侧、背对靶场。
    fn default() -> Self { Self { yaw: std::f32::consts::PI, pitch: 0.35, aiming: false, aim_lerp: 0.0 } }
}

#[derive(Component)]
pub struct PlayerMovement {
    pub velocity: Vec3,
    pub is_grounded: bool,
    pub shoot_cooldown: Timer,
    pub reload_timer: Option<Timer>,
    /// 模型当前朝向（弧度）：瞄准时平滑追相机 Yaw，普通视角即时同向
    pub model_yaw: f32,
    /// 本帧水平移动速度（m/s，0=静止）：动作系统据此决定步频与摆幅
    pub planar_speed: f32,
}
impl Default for PlayerMovement {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            is_grounded: true,
            shoot_cooldown: Timer::from_seconds(0.12, TimerMode::Once),
            reload_timer: None,
            model_yaw: std::f32::consts::PI,
            planar_speed: 0.0,
        }
    }
}

/// 玩家当前干员状态：名册下标 + Q/E 冷却。
/// 技能定义（元素/伤害/半径/冷却）来自核心库 operator 名册，切换干员即换技能组。
#[derive(Component)]
pub struct OperatorState { pub active: usize, pub q: Timer, pub e: Timer }
impl Default for OperatorState {
    fn default() -> Self {
        let op = &roster()[0];
        Self { active: 0, q: ready_timer(op.q.cooldown_secs), e: ready_timer(op.e.cooldown_secs) }
    }
}

/// 已就绪的冷却计时器（预滴满，保证开局技能立即可用且图标不满灰）
pub fn ready_timer(cooldown_secs: f32) -> Timer {
    let mut t = Timer::from_seconds(cooldown_secs, TimerMode::Once);
    t.tick(t.duration());
    t
}

/// 玩家模型发光饰条的材质句柄（头冠/胸口/枪身共用），切换干员时重新着色
#[derive(Component)]
pub struct OperatorAccent(pub Handle<StandardMaterial>);

/// 玩家实体标记（demo 玩法层与 model 模块共用的实体锚点）
#[derive(Component)]
pub struct Player;