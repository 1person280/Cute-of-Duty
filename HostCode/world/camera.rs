//! 第三人称越肩镜头 + 鼠标自由视角
///
/// 设计动机（Why）：0.6 早期版本朝向由 WASD 位移方向推导，既没有鼠标自由视角、
/// 也无法把瞄准朝向报给服务端（弹道与移动轴系都因此失真）。本模块改为**鼠标驱动**：
/// 偏航/俯仰写入共享资源 [`AimRig`]（`flow_state`），镜头据此摆放；`net::input_system`
/// 再把同一份 `AimRig` 作为 `aim_yaw/aim_pitch` 上报 —— 视角与移动/弹道轴系从此同源。
///
/// 取景方式（Why，对齐 0.3.2 `demo/camera.rs` 的越肩 SpringArm）：镜头挂在角色**右肩**后方，
/// 并**沿视线方向平行注视**（而非回看角色）——角色因此稳定落在画面**偏左**，正是"第三人称
/// 越肩"的标志观感；此前"纯环绕 + 始终看向角色胸口"的取景会让角色正中、并在贴墙时糊脸，
/// 观感偏近于第一人称。轴系与旧版一致：右肩水平偏移 `SHOULDER_OFFSET`，臂长 `CAMERA_DIST`。
///
/// 轴系约定（与 `flow_state::AimRig` 及服务端 `shooter` 完全一致）：
/// - 方向 = `(sinY·cosP, sinP, cosY·cosP)`，pitch 为正 = 抬头；
/// - 默认 yaw = π 面向 -Z（北/撤离区），镜头落在玩家 +Z 侧后方，与服务端出生朝向一致。
///
/// 视角/距离/FOV 均为纯表现层设定，不属于任何服务端权威数据（`game_settings` 据此改 FOV）。
/// 镜头位置取自权威快照（服务端唯一真相源），客户端不做本地位置校订。

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

use cute_of_duty_contract::map::MapLayout;

use crate::flow::flow_state::{AimRig, LocalPlayer};
use crate::flow::{GameSettings, WorldCatalog};
use crate::net::snapshot::SnapshotBuffer;

/// 第三人称越肩镜头标记（每帧由 `follow_system` 重写世界变换；`game_settings` 据此改 FOV）。
///
/// `aim_blend` 为越肩瞄准的过渡进度（0 = 常态取景，1 = 瞄准取景），由 `follow_system`
/// 按时间朝目标值收敛后写入；投影侧（`settings_apply_fov`）读它同步收窄 FOV，
/// 从而**全工程只有一处写 `Projection`**，避免两系统同帧争用同一组件的调度冲突。
#[derive(Component)]
pub struct ChaseCamera {
    pub aim_blend: f32,
    /// 当前实际臂长（米，沿"锚点 → 理想机位"方向度量）。
    ///
    /// 存在动机（Why）：SpringArm 避障需要"记忆"上一帧臂长以区分收缩/回伸——
    /// 撞墙时瞬间收缩（防穿模），离墙后按速度缓伸（防镜头弹跳），
    /// 恒由 `follow_system` 维护，不参与快照/协议。
    pub arm_dist: f32,
}

/// 镜头到角色的臂长（米，沿 0.3.2 `ARM_LEN_NORMAL` 标定值）。
///
/// 设计动机（Why）：该值按角色实际身高标定。焰狐体素模型 32 设计像素 × `YANHU_S=0.11`
/// ≈ **3.52m**，与 0.3.2 的 3.5m 体型一致，故直接沿用旧版 6.5m 臂长；此前 4.2m 是给
/// 旧方块回退角色（约 2.67m）调的，套到焰狐身上会让模型怼满屏幕（"外观怪异"）。
const CAMERA_DIST: f32 = 6.5;
/// 越肩机位的右肩水平偏移（米，0.3.2 `ARM_SHOULDER_X_NORMAL`）：镜头右移后角色落于画面偏左。
const SHOULDER_OFFSET: f32 = 0.55;
/// 瞄准时的臂长（米，镜头贴近右肩，旧版 `ARM_LEN_AIM`）。
const AIM_DIST: f32 = 2.4;
/// 瞄准时的右肩偏移（米）：比常态更外扩，避免贴脸时角色糊住画面。
const AIM_SHOULDER: f32 = 1.0;
/// 常态 ↔ 瞄准取景的过渡时长（秒，smoothstep），沿 0.3.2 手感。
const AIM_BLEND_SECS: f32 = 0.22;
/// 瞄准时 FOV 收窄比例（28%），由投影侧读取（见 [`ChaseCamera::aim_blend`]）。
pub const AIM_FOV_NARROW: f32 = 0.28;
/// 视线锚点高度（米，沿 0.3.2 `PIVOT_HEIGHT` 标定值）：落在角色肩颈处（焰狐头中心约 3.0m），
/// 保证 3.52m 全身在框。此前 1.55 是按旧方块回退角色身高取的，套到焰狐身上会仰视、只见下半身。
const PIVOT_Y: f32 = 2.6;
/// 俯仰限位：抬头不高于 +0.55（约 31°）、低头不低于 -0.75（约 -43°），
/// 避免镜头钻到角色脚下/贴地。瞄准用 `AimRig.pitch` 另有更宽的限位。
const ORBIT_PITCH_MIN: f32 = -0.75;
const ORBIT_PITCH_MAX: f32 = 0.55;
/// 镜头最低高度（米），防止大角度俯视时穿到地面以下。
const MIN_CAM_Y: f32 = 0.35;
/// 撞墙时臂长的下限（米）：镜头最多缩到锚点背后 0.7m，避免穿进角色体内或贴脸糊屏。
const MIN_ARM_DIST: f32 = 0.7;
/// 镜头与墙面的安全间隙（米）：命中点再往回收一点，并吸收收缩平滑的滞后
/// （25/s 收缩在 60fps 下约一帧 0.1m、玩家 6m/s 后退约 0.24m），杜绝近裁面切进墙里。
const WALL_MARGIN: f32 = 0.30;
/// 贴墙收缩速率（1/秒，指数平滑）。取高值：威胁侧要快，但仍非瞬跳——
/// 时间常数 τ = 1/25 ≈ 40ms，视觉上是"顺"而不是"啪"。
const CAM_ARM_SHRINK_RATE: f32 = 25.0;
/// 离墙回伸速率（1/秒，指数平滑）。取低值：从容归位，避免镜头"弹开"造成的眩晕。
const CAM_ARM_GROW_RATE: f32 = 10.0;

/// 鼠标灵敏度（弧度/像素），再乘设置里的灵敏度；数值沿 0.3.2 手感。
const MOUSE_SENS_X: f32 = 0.0024;
const MOUSE_SENS_Y: f32 = 0.0021;
/// 俯仰限制：抬头 50° / 低头 70°（与服务端视线方向同号：pitch 正 = 抬头）。
const PITCH_UP_MAX: f32 = 0.873;
const PITCH_DOWN_MAX: f32 = 1.217;
/// 单帧鼠标位移上限（像素）：光标刚锁定时系统可能上报一次“从旧位置跳到中心”的巨大位移，
/// 限幅可防止视角被该跳变甩飞（正常鼠标移动远低于此值）。
const MAX_FRAME_DELTA: f32 = 200.0;

/// 生成越肩摄像机（初始面向 -Z 北侧；握手前停在原点后方，仍在场内）。
///
/// bevy 0.15 直接 spawn `Camera3d` + `Transform`（渲染图/投影/可见性由
/// required components 一并补齐）。
pub fn spawn_camera(commands: &mut Commands) {
    let look = Vec3::new(0.0, PIVOT_Y, 0.0);
    commands.spawn((
        ChaseCamera { aim_blend: 0.0, arm_dist: CAMERA_DIST },
        Camera3d::default(),
        Transform::from_translation(look + Vec3::new(0.0, 0.0, CAMERA_DIST))
            .looking_at(look, Vec3::Y),
    ));
}

/// 鼠标自由视角：把本帧鼠标位移折进共享的 [`AimRig`]（偏航/俯仰）。
///
/// 只在游戏内且未暂停时运行（`launcher` 以 `pause_closed` 门控），且仅在光标锁定
/// （`menu::pause::cursor_lock_system` 保证）时才有意义——菜单态鼠标用于点按 UI。
pub fn mouse_look_system(
    mut motion: EventReader<MouseMotion>,
    mouse: Res<ButtonInput<MouseButton>>,
    settings: Res<GameSettings>,
    mut rig: ResMut<AimRig>,
) {
    // 瞄准态由右键按住决定（相机取景过渡 + 上行 `aim` 意图同源，见 `net::input_system`）。
    rig.aiming = mouse.pressed(MouseButton::Right);

    let mut delta = Vec2::ZERO;
    for ev in motion.read() {
        delta += ev.delta;
    }
    if delta == Vec2::ZERO {
        return;
    }
    // 限幅：抵消光标锁定瞬间的巨大跳变（见 `MAX_FRAME_DELTA`）。
    delta = delta.clamp(Vec2::splat(-MAX_FRAME_DELTA), Vec2::splat(MAX_FRAME_DELTA));
    // 鼠标右移 → 视线右转（yaw 减小，见模块头轴系推导）；鼠标下移 → 低头（pitch 减小）。
    rig.yaw -= delta.x * MOUSE_SENS_X * settings.mouse_sensitivity;
    rig.pitch -= delta.y * MOUSE_SENS_Y * settings.mouse_sensitivity;
    rig.pitch = rig.pitch.clamp(-PITCH_DOWN_MAX, PITCH_UP_MAX);
}

/// 手雷持握期间强制越肩瞄准（表现层）：把持雷态叠加进 [`AimRig::aiming`]。
///
/// 设计动机（Why）：持雷时服务端已按越肩速度（`AIM_MULT`）结算位移；客户端相机必须同步
/// 收臂收 FOV，否则"看着全速站立、实际走得很慢"，表现与权威不一致。本系统**排在
/// [`mouse_look_system`] 之后**：先由鼠标写入右键基准，再叠加持雷态——`AimRig::aiming`
/// 全程单点写入，杜绝两系统同帧互相覆盖。
pub fn sync_grenade_aim(held: Res<crate::hud::HeldGrenadeState>, mut rig: ResMut<AimRig>) {
    if held.element.is_some() {
        rig.aiming = true;
    }
}

/// 每帧把镜头摆到角色右肩后方并沿视线平行注视（越肩第三人称）；握手前无本人实体则停在原点。
///
/// 目标位置取自权威快照（服务端唯一真相源），客户端不做本地校订；镜头姿态纯属表现层。
/// 注意：本人实体若**暂时**不在本帧快照（AOI/对账间隙），直接保持上一帧机位而非退回世界原点——
/// 否则镜头会瞬移到 (0,0,0)，表现为"看不见自己、场景乱飘"。
pub fn follow_system(
    mut query: Query<(&mut Transform, &mut ChaseCamera)>,
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    rig: Res<AimRig>,
    time: Res<Time>,
    catalog: Res<WorldCatalog>,
    mut colliders: Local<Option<Vec<(Vec3, Vec3)>>>,
) {
    let center = if player.entity_id != 0 {
        match snap
            .current
            .iter()
            .find(|e| e.entity_id == player.entity_id)
        {
            Some(e) => Vec3::new(e.x, e.y, e.z),
            // 已握手但本帧快照缺本人条目：保持上一帧机位，避免跳回原点。
            None => return,
        }
    } else {
        Vec3::ZERO
    };

    // 视线锚点固定在角色肩部；镜头绕其沿视线反方向退到身后（含俯仰的环轨道）。
    let pitch = rig.pitch.clamp(ORBIT_PITCH_MIN, ORBIT_PITCH_MAX);
    let cos_p = pitch.cos();
    // 视线方向（与服务端弹道同号：pitch 正 = 抬头）。
    let dir = Vec3::new(rig.yaw.sin() * cos_p, pitch.sin(), rig.yaw.cos() * cos_p);
    // 水平右向量 = normalize(cross(forward, Y))，用于右肩平移（越肩机位）。
    let right = Vec3::new(-rig.yaw.cos(), 0.0, rig.yaw.sin());

    let anchor = center + Vec3::Y * PIVOT_Y;

    // 越肩瞄准过渡：`aim_blend` 按「dt / 过渡时长」朝目标推进（帧率无关），
    // 再取 smoothstep 缓动，使收臂 / 收 FOV 起步与收尾都不生硬。
    let target = if rig.aiming { 1.0 } else { 0.0 };
    let step = time.delta_secs() / AIM_BLEND_SECS;
    for (mut tf, mut cam) in &mut query {
        cam.aim_blend = (cam.aim_blend + (target - cam.aim_blend).clamp(-step, step)).clamp(0.0, 1.0);
        let b = cam.aim_blend;
        let eased = b * b * (3.0 - 2.0 * b);

        let dist = CAMERA_DIST + (AIM_DIST - CAMERA_DIST) * eased;
        let shoulder = SHOULDER_OFFSET + (AIM_SHOULDER - SHOULDER_OFFSET) * eased;
        // 理想机位（未避障）：锚点 + 右肩偏移 - 视线臂长。
        let ideal = anchor + right * shoulder - dir * dist;

        // SpringArm 避障：沿"锚点 → 理想机位"做射线-AABB 扫掠，命中实体墙（solid prop）则收缩臂长。
        // 收缩/回伸均走非对称指数平滑（见 `approach_arm`），不再瞬时跳变。
        let to = ideal - anchor;
        let full = to.length();
        if full > 1e-4 {
            // 目录未到达时无法建碰撞盒（相机此时也尚未生成，正常不会走到），跳过避障。
            if let Some(layout) = catalog.layout.as_ref() {
                let colliders = colliders.get_or_insert_with(|| build_colliders(layout));
                // 安全臂长：命中实体墙则收至「命中点 − 安全间隙」，否则保持全臂长。
                let safe = match sweep_nearest(anchor, to / full, full, colliders) {
                    Some(t) => (t - WALL_MARGIN).clamp(MIN_ARM_DIST, full),
                    None => full,
                };
                // 非对称指数平滑（收缩快 / 回伸慢、帧率无关），取代原先
                // "收缩瞬时跳变 + 回伸线性匀速"的硬切换，消除扫过掩体边缘时的瞬跳。
                cam.arm_dist = approach_arm(cam.arm_dist, safe, time.delta_secs());
            }
        }
        let mut cam_pos = anchor + (to / full) * cam.arm_dist;
        if cam_pos.y < MIN_CAM_Y {
            cam_pos.y = MIN_CAM_Y;
        }
        tf.translation = cam_pos;
        // 沿视线**平行**注视（不回看角色）：角色因此稳定落在画面偏左，形成越肩第三人称观感。
        tf.look_at(cam_pos + dir, Vec3::Y);
    }
}

/// 收集地图静态 solid 物体的 AABB（世界坐标 min/max），供 SpringArm 扫掠使用。
///
/// 设计动机（Why）：镜头避障属**表现层**，不需要进服务端模拟；直接读服务端下发的
/// 地图布局纯数据（与服务端同一事实来源，不触碰任何模拟逻辑），缓存一次即可。
fn build_colliders(layout: &MapLayout) -> Vec<(Vec3, Vec3)> {
    layout
        .props
        .iter()
        .filter(|p| p.solid)
        .map(|p| {
            let h = p.aabb_half();
            let c = Vec3::new(p.pos[0], p.pos[1], p.pos[2]);
            let half = Vec3::new(h[0], h[1], h[2]);
            (c - half, c + half)
        })
        .collect()
}

/// 把当前臂长朝安全臂长做**非对称、帧率无关的指数平滑**：收缩用高率、回伸用低率。
///
/// 设计动机（Why）：原实现收缩侧为瞬时赋值、回伸侧为线性匀速——视角扫过掩体边缘时
/// 臂长会"啪"地跳变。指数平滑 `α = 1 - exp(-k·dt)` 使**相同总时长在不同帧率下收敛一致**，
/// 且收缩率高于回伸率：威胁侧仍足够快，却不产生瞬跳。防穿模由调用处的 `WALL_MARGIN` 兜底。
fn approach_arm(current: f32, target: f32, dt: f32) -> f32 {
    let rate = if target < current {
        CAM_ARM_SHRINK_RATE
    } else {
        CAM_ARM_GROW_RATE
    };
    let alpha = 1.0 - (-rate * dt).exp();
    current + (target - current) * alpha
}

/// 射线-AABB 最近命中距离（slab 法）；`dir` 须为单位向量，返回 `[0, max_t]` 内的最小 `t`。
fn sweep_nearest(origin: Vec3, dir: Vec3, max_t: f32, colliders: &[(Vec3, Vec3)]) -> Option<f32> {
    let o = origin.to_array();
    let d = dir.to_array();
    let mut nearest: Option<f32> = None;
    for &(min, max) in colliders {
        let lo = min.to_array();
        let hi = max.to_array();
        let (mut t_min, mut t_max) = (0.0f32, max_t);
        let mut hit = true;
        for axis in 0..3 {
            if d[axis].abs() < 1e-6 {
                if o[axis] < lo[axis] || o[axis] > hi[axis] {
                    hit = false;
                    break;
                }
            } else {
                let inv = 1.0 / d[axis];
                let (mut t1, mut t2) = ((lo[axis] - o[axis]) * inv, (hi[axis] - o[axis]) * inv);
                if t1 > t2 {
                    std::mem::swap(&mut t1, &mut t2);
                }
                t_min = t_min.max(t1);
                t_max = t_max.min(t2);
                if t_min > t_max {
                    hit = false;
                    break;
                }
            }
        }
        if hit && t_min <= max_t {
            nearest = Some(match nearest {
                Some(n) => n.min(t_min),
                None => t_min,
            });
        }
    }
    nearest
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 帧率无关：相同总时长、不同步长，臂长收敛结果应一致（指数平滑的核心性质，
    /// 也是本实现相对旧线性匀速回伸的关键收益）。
    #[test]
    fn approach_arm_is_frame_rate_independent() {
        // 同一总时长 0.05s：60fps 走 3 步 × 1/60s，120fps 走 6 步 × 1/120s。
        let mut a = CAMERA_DIST;
        for _ in 0..3 {
            a = approach_arm(a, MIN_ARM_DIST, 1.0 / 60.0);
        }
        let mut b = CAMERA_DIST;
        for _ in 0..6 {
            b = approach_arm(b, MIN_ARM_DIST, 1.0 / 120.0);
        }
        assert!((a - b).abs() < 1e-4, "不同帧率收敛应一致: {a} vs {b}");
    }

    /// 不超调：任何步长下臂长都落在 [target, current] 区间内（收缩不越过下限、回伸不越过上限）。
    #[test]
    fn approach_arm_never_overshoots() {
        // 夸张 dt：α 近似 1，结果应贴住目标但不越界。
        let shrink = approach_arm(CAMERA_DIST, MIN_ARM_DIST, 10.0);
        assert!(
            (MIN_ARM_DIST..=CAMERA_DIST).contains(&shrink),
            "收缩越过目标区间: {shrink}"
        );
        let grow = approach_arm(MIN_ARM_DIST, CAMERA_DIST, 10.0);
        assert!(
            (MIN_ARM_DIST..=CAMERA_DIST).contains(&grow),
            "回伸越过目标区间: {grow}"
        );
        // dt = 0 不改变现状。
        assert_eq!(approach_arm(3.3, MIN_ARM_DIST, 0.0), 3.3);
    }
}