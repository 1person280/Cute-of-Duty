//! 干员模型与动作模块 —— 玩家角色体素建模、程序化动作与换装系统
//!
//! 从 3D Demo（src/demo）解耦而来，feature "demo" 门控（仅 Demo 需要 bevy）。
//! 职责：
//! - 四名干员的专属体素模型（焰狐/霜刃/雷豹/毒蛛），共用同一方块骨架与枢轴约定
//! - yanhu_action_system：行走/疾跑/跳跃/瞄准持枪/开火后坐/尾巴摇摆（全干员通用）
//! - operator_model_swap_system：切换干员时整体换模型、按干员重着色
//!
//! 与 demo 的接口：demo 玩法层通过本模块的组件（PlayerMovement/PlayerCamera/
//! OperatorState 等）驱动模型，本模块不包含任何玩法逻辑。

use bevy::prelude::*;

use crate::operator::roster;
use crate::demo::camera::{CamPivot, PitchPivot, ShoulderPivot, SpringArm, lerp};


// ===== 以下代码自 src/demo/mod.rs 原样迁入（干员模型/动作域） =====

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

// ===== 以下代码自 src/demo/mod.rs 原样迁入（干员模型/动作域） =====

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

// ===== 以下代码自 src/demo/mod.rs 原样迁入（干员模型/动作域） =====

/// 玩家模型发光饰条的材质句柄（头冠/胸口/枪身共用），切换干员时重新着色
#[derive(Component)]
pub struct OperatorAccent(pub Handle<StandardMaterial>);

// ===== 以下代码自 src/demo/mod.rs 原样迁入（干员模型/动作域） =====

// =============================================================================
// Palette
// =============================================================================

/// 玩家实体标记（demo 玩法层与 model 模块共用的实体锚点）
#[derive(Component)]
pub struct Player;

pub mod palette {
    use bevy::prelude::Color;
    pub const SKIN: Color = Color::srgb(0.96, 0.80, 0.69);
    pub const TACTICAL_GREEN: Color = Color::srgb(0.22, 0.32, 0.22);
    pub const TACTICAL_DARK: Color = Color::srgb(0.15, 0.17, 0.15);
    pub const ARMOR_GREY: Color = Color::srgb(0.45, 0.48, 0.50);
    pub const ARMOR_DARK: Color = Color::srgb(0.28, 0.30, 0.32);
    pub const EYE_BLUE: Color = Color::srgb(0.3, 0.9, 1.0);
    pub const EYE_RED: Color = Color::srgb(1.0, 0.25, 0.15);
    pub const BOOTS: Color = Color::srgb(0.18, 0.14, 0.12);
    pub const GROUND_A: Color = Color::srgb(0.32, 0.36, 0.28);
    pub const GROUND_B: Color = Color::srgb(0.24, 0.27, 0.22);
    pub const CONCRETE: Color = Color::srgb(0.50, 0.50, 0.54);
    pub const RUST: Color = Color::srgb(0.60, 0.35, 0.22);
    pub const TARGET_RED: Color = Color::srgb(0.85, 0.15, 0.15);
    pub const TARGET_WHITE: Color = Color::srgb(0.92, 0.92, 0.92);
    pub const HP_RED: Color = Color::srgb(0.90, 0.18, 0.18);
    pub const ARMOR_BLUE: Color = Color::srgb(0.25, 0.55, 0.95);
}

// ===== 以下代码自 src/demo/mod.rs 原样迁入（干员模型/动作域） =====

/// 焰狐模型设计像素 → 世界单位（32px 身高 ≈ 3.52，与通用 steve 的 3.5 相当）
const YANHU_S: f32 = 0.11;

/// 设计像素（角点原点，脸朝 -Z，同 assets/characters/model/yanhu.geometry.json）
/// → 本地空间（盒子中心，脸朝 +Z，与 build_steve 朝向约定一致）
fn yanhu_px(x: f32, y: f32, z: f32, w: f32, h: f32, d: f32) -> (Vec3, Vec3) {
    (
        Vec3::new((x + w * 0.5) * YANHU_S, (y + h * 0.5) * YANHU_S, -(z + d * 0.5) * YANHU_S),
        Vec3::new(w * YANHU_S, h * YANHU_S, d * YANHU_S),
    )
}

/// 模型像素坐标点 → 世界米坐标（与 yanhu_px 同一换算，供枢轴关节定位）
fn yanhu_point(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x * YANHU_S, y * YANHU_S, -z * YANHU_S)
}

fn voxel_box(
    parent: &mut ChildBuilder,
    meshes: &mut ResMut<Assets<Mesh>>,
    mat: &Handle<StandardMaterial>,
    origin: Vec3,
    size: Vec3,
    rot: Quat,
) {
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(size.x, size.y, size.z)),
        material: mat.clone(),
        transform: Transform::from_translation(origin).with_rotation(rot),
        ..default()
    });
}

/// 焰狐干员模型（[YSM] 是，史蒂夫模型 mod 美学，35 盒）：
/// 原版 Steve 方块骨架 + 特征方块（狐耳/三节尾/发冠刘海挑染）+ 二次元皮肤级配色。
/// 规格与 assets/characters/model/yanhu.geometry.json、yanhu_model_preview.png 一致。
/// 元素件（刘海挑染/胸徽/臂章/照门）共享 accent 发光材质——切干员时随 switch_operator 重着色。
pub fn build_yanhu(
    parent: &mut ChildBuilder,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    accent: &Handle<StandardMaterial>,
    eye: &Handle<StandardMaterial>,
) {
    let skin = mat_voxel(materials, palette::SKIN);
    let green = mat_voxel(materials, palette::TACTICAL_GREEN);
    let armor = mat_voxel(materials, palette::ARMOR_GREY);
    let boot = mat_voxel(materials, palette::BOOTS);
    let dark = mat_voxel(materials, palette::TACTICAL_DARK);
    let gun = mat_voxel(materials, Color::srgb(0.3, 0.3, 0.35));
    let hair = mat_voxel(materials, Color::srgb(0.76, 0.36, 0.12));
    let hair_dark = mat_voxel(materials, Color::srgb(0.55, 0.24, 0.09));
    let cream = mat_voxel(materials, Color::srgb(0.96, 0.88, 0.78));
    let q = Quat::IDENTITY;

    // ---- 躯干/四肢/尾巴（挂根） ----
    let (c, s) = yanhu_px(-4.0, 12.0, -2.0, 8.0, 12.0, 4.0);   voxel_box(parent, meshes, &green, c, s, q);   // torso
    let (c, s) = yanhu_px(-4.0, 12.0, -2.5, 8.0, 2.0, 5.0);    voxel_box(parent, meshes, &dark, c, s, q);    // belt
    let (c, s) = yanhu_px(-4.0, 15.0, -3.0, 8.0, 5.0, 1.0);    voxel_box(parent, meshes, &dark, c, s, q);    // vest
    let (c, s) = yanhu_px(-1.0, 16.0, -3.3, 2.0, 2.0, 0.3);    voxel_box(parent, meshes, accent, c, s, q);   // vestPin
    let (c, s) = yanhu_px(-6.0, 22.0, -3.0, 3.0, 2.0, 3.0);    voxel_box(parent, meshes, &armor, c, s, q);   // shdL
    let (c, s) = yanhu_px(3.0, 22.0, -3.0, 3.0, 2.0, 3.0);     voxel_box(parent, meshes, &armor, c, s, q);   // shdR
    // 四肢包进枢轴（关节点在肩/髋），由 yanhu_action_system 驱动行走/跳跃/瞄准动作
    let arm_l_pivot = yanhu_point(-6.0, 24.0, -0.5);
    let mut arm_l = parent.spawn((SpatialBundle { transform: Transform::from_translation(arm_l_pivot), ..default() }, YanhuLimb::new(LimbKind::ArmL)));
    arm_l.with_children(|p| {
        let (c, s) = yanhu_px(-8.0, 19.0, -2.0, 4.0, 5.0, 4.0);    voxel_box(p, meshes, &green, c - arm_l_pivot, s, q);   // armLup
        let (c, s) = yanhu_px(-8.0, 12.0, -2.0, 4.0, 7.0, 4.0);    voxel_box(p, meshes, &skin, c - arm_l_pivot, s, q);    // armLlo
    });
    let arm_r_pivot = yanhu_point(6.0, 24.0, -0.5);
    let mut arm_r = parent.spawn((SpatialBundle { transform: Transform::from_translation(arm_r_pivot), ..default() }, YanhuLimb::new(LimbKind::ArmR)));
    arm_r.with_children(|p| {
        let (c, s) = yanhu_px(4.0, 19.0, -2.0, 4.0, 5.0, 4.0);     voxel_box(p, meshes, &green, c - arm_r_pivot, s, q);   // armRup
        let (c, s) = yanhu_px(3.9, 20.0, -2.1, 4.2, 2.0, 4.2);     voxel_box(p, meshes, accent, c - arm_r_pivot, s, q);   // armband
        let (c, s) = yanhu_px(4.0, 12.0, -2.0, 4.0, 7.0, 4.0);     voxel_box(p, meshes, &skin, c - arm_r_pivot, s, q);    // armRlo
    });
    let leg_l_pivot = yanhu_point(-2.0, 12.0, 0.0);
    let mut leg_l = parent.spawn((SpatialBundle { transform: Transform::from_translation(leg_l_pivot), ..default() }, YanhuLimb::new(LimbKind::LegL)));
    leg_l.with_children(|p| {
        let (c, s) = yanhu_px(-4.0, 3.0, -2.0, 4.0, 9.0, 4.0);     voxel_box(p, meshes, &green, c - leg_l_pivot, s, q);   // legL
        let (c, s) = yanhu_px(-4.0, 0.0, -3.0, 4.0, 3.0, 5.0);     voxel_box(p, meshes, &boot, c - leg_l_pivot, s, q);    // bootL
    });
    let leg_r_pivot = yanhu_point(2.0, 12.0, 0.0);
    let mut leg_r = parent.spawn((SpatialBundle { transform: Transform::from_translation(leg_r_pivot), ..default() }, YanhuLimb::new(LimbKind::LegR)));
    leg_r.with_children(|p| {
        let (c, s) = yanhu_px(0.0, 3.0, -2.0, 4.0, 9.0, 4.0);      voxel_box(p, meshes, &green, c - leg_r_pivot, s, q);   // legR
        let (c, s) = yanhu_px(0.0, 0.0, -3.0, 4.0, 3.0, 5.0);      voxel_box(p, meshes, &boot, c - leg_r_pivot, s, q);    // bootR
    });
    // 三节尾巴链式枢轴：A 挂躯干，B 挂 A 末端，C 挂 B 末端（摇摆时逐节跟随）
    let tail_a_pivot = yanhu_point(0.0, 14.5, 2.0);
    let tail_b_pivot = yanhu_point(0.0, 16.0, 8.0);
    let tail_c_pivot = yanhu_point(0.0, 17.0, 13.0);
    let mut tail_a = parent.spawn((SpatialBundle { transform: Transform::from_translation(tail_a_pivot), ..default() }, YanhuLimb::new(LimbKind::TailA)));
    tail_a.with_children(|ta| {
        let (c, s) = yanhu_px(-1.5, 13.0, 2.0, 3.0, 3.0, 7.0);     voxel_box(ta, meshes, &hair, c - tail_a_pivot, s, q);   // tailA
        let mut tail_b = ta.spawn((SpatialBundle { transform: Transform::from_translation(tail_b_pivot - tail_a_pivot), ..default() }, YanhuLimb::new(LimbKind::TailB)));
        tail_b.with_children(|tb| {
            let (c, s) = yanhu_px(-1.0, 15.0, 8.0, 2.0, 2.0, 6.0);     voxel_box(tb, meshes, &hair, c - tail_b_pivot, s, q);   // tailB
            let mut tail_c = tb.spawn((SpatialBundle { transform: Transform::from_translation(tail_c_pivot - tail_b_pivot), ..default() }, YanhuLimb::new(LimbKind::TailC)));
            tail_c.with_children(|tc| {
                let (c, s) = yanhu_px(-1.0, 16.0, 13.0, 2.0, 2.0, 4.0);    voxel_box(tc, meshes, &cream, c - tail_c_pivot, s, q);  // tailC
            });
        });
    });

    // ---- 头（枢轴随瞄准俯仰；发/耳随头动，耳微外倾） ----
    let head_base = Vec3::new(0.0, 24.0 * YANHU_S, 0.0);
    let mut head_pivot = parent.spawn(SpatialBundle {
        transform: Transform::from_translation(head_base),
        ..default()
    });
    head_pivot.insert(PlayerHeadPivot);
    head_pivot.with_children(|hd| {
        let tilt_l = Quat::from_rotation_z(0.15);
        let tilt_r = Quat::from_rotation_z(-0.15);
        let (c, s) = yanhu_px(-4.0, 24.0, -4.0, 8.0, 8.0, 8.0);    voxel_box(hd, meshes, &skin, c - head_base, s, q);      // head
        let (c, s) = yanhu_px(-3.0, 27.0, -4.3, 2.0, 2.0, 0.3);    voxel_box(hd, meshes, eye, c - head_base, s, q);        // eyeL
        let (c, s) = yanhu_px(1.0, 27.0, -4.3, 2.0, 2.0, 0.3);     voxel_box(hd, meshes, eye, c - head_base, s, q);        // eyeR
        let (c, s) = yanhu_px(-4.5, 29.0, -4.5, 9.0, 3.0, 9.0);    voxel_box(hd, meshes, &hair, c - head_base, s, q);      // hairCap
        let (c, s) = yanhu_px(-4.5, 24.0, 3.5, 9.0, 6.0, 1.0);     voxel_box(hd, meshes, &hair_dark, c - head_base, s, q); // hairBack
        let (c, s) = yanhu_px(-5.5, 24.0, -4.0, 1.0, 5.0, 8.0);    voxel_box(hd, meshes, &hair_dark, c - head_base, s, q); // hairSideL
        let (c, s) = yanhu_px(4.5, 24.0, -4.0, 1.0, 5.0, 8.0);     voxel_box(hd, meshes, &hair_dark, c - head_base, s, q); // hairSideR
        let (c, s) = yanhu_px(-4.0, 27.0, -5.0, 8.0, 3.0, 0.5);    voxel_box(hd, meshes, &hair, c - head_base, s, q);      // bangs
        let (c, s) = yanhu_px(-2.0, 29.0, -5.3, 2.0, 2.0, 0.3);    voxel_box(hd, meshes, accent, c - head_base, s, q);     // streak
        let (c, s) = yanhu_px(-4.0, 32.0, -2.0, 3.0, 3.0, 2.0);    voxel_box(hd, meshes, &hair, c - head_base, s, tilt_l); // earL
        let (c, s) = yanhu_px(-3.4, 32.6, -2.3, 1.8, 1.8, 0.3);    voxel_box(hd, meshes, &cream, c - head_base, s, tilt_l);// earLin
        let (c, s) = yanhu_px(1.0, 32.0, -2.0, 3.0, 3.0, 2.0);     voxel_box(hd, meshes, &hair, c - head_base, s, tilt_r); // earR
        let (c, s) = yanhu_px(1.6, 32.6, -2.3, 1.8, 1.8, 0.3);     voxel_box(hd, meshes, &cream, c - head_base, s, tilt_r);// earRin
    });

    // ---- 枪（枢轴瞄准时从腰际举到肩上，同 build_steve 程序化持枪） ----
    let (gun_center, _) = yanhu_px(4.2, 16.0, -6.0, 1.0, 1.0, 7.0);
    let mut gun_pivot = parent.spawn(SpatialBundle {
        transform: Transform::from_translation(Vec3::new(0.517, 1.55, 0.32)),
        ..default()
    });
    gun_pivot.insert(PlayerAimGun {
        base: Vec3::new(0.517, 1.55, 0.32),
        raised: Vec3::new(0.517, 2.5, 0.38),
    });
    gun_pivot.with_children(|gp| {
        let (c, s) = yanhu_px(4.2, 16.0, -6.0, 1.0, 1.0, 7.0);    voxel_box(gp, meshes, &gun, c - gun_center, s, q);   // gunBody
        let (c, s) = yanhu_px(4.3, 14.5, -3.0, 0.7, 1.5, 2.0);     voxel_box(gp, meshes, &dark, c - gun_center, s, q);  // gunMag
        let (c, s) = yanhu_px(4.3, 15.8, 0.0, 0.7, 0.8, 2.0);      voxel_box(gp, meshes, &dark, c - gun_center, s, q);  // gunStock
        let (c, s) = yanhu_px(4.3, 17.0, -3.0, 0.4, 1.0, 1.0);     voxel_box(gp, meshes, accent, c - gun_center, s, q); // gunSight
    });
}

/// 霜刃干员模型（冰系，[YSM] 是，史蒂夫模型 mod 美学）：
/// 与焰狐同一方块骨架 + 特征方块（冰晶头冠/双冰角/长马尾/背负战刃/霜甲裙摆）。
/// 枢轴约定与焰狐一致（YanhuLimb 四肢 + TailA/B/C 长马尾链），
/// 动作由 yanhu_action_system 统一驱动；冰晶件共享 accent 发光材质（切干员随元素重着色）。
fn build_shuangren(
    parent: &mut ChildBuilder,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    accent: &Handle<StandardMaterial>,
    eye: &Handle<StandardMaterial>,
) {
    let skin = mat_voxel(materials, palette::SKIN);
    let frost = mat_voxel(materials, Color::srgb(0.86, 0.90, 0.93)); // 霜白甲
    let dark = mat_voxel(materials, palette::TACTICAL_DARK);
    let boot = mat_voxel(materials, palette::BOOTS);
    let gun = mat_voxel(materials, Color::srgb(0.3, 0.3, 0.35));
    let hair = mat_voxel(materials, Color::srgb(0.70, 0.86, 0.96));  // 霜蓝长发
    let hair_dark = mat_voxel(materials, Color::srgb(0.44, 0.62, 0.75));
    let q = Quat::IDENTITY;

    // ---- 躯干/四肢（挂根） ----
    let (c, s) = yanhu_px(-4.0, 12.0, -2.0, 8.0, 12.0, 4.0);   voxel_box(parent, meshes, &frost, c, s, q);   // torso
    let (c, s) = yanhu_px(-4.0, 12.0, -2.5, 8.0, 2.0, 5.0);    voxel_box(parent, meshes, &dark, c, s, q);     // belt
    let (c, s) = yanhu_px(-4.0, 16.0, -3.0, 8.0, 6.0, 1.0);    voxel_box(parent, meshes, &dark, c, s, q);     // vest
    let (c, s) = yanhu_px(-1.0, 18.0, -3.3, 2.0, 2.0, 0.3);    voxel_box(parent, meshes, accent, c, s, q);    // vestPin
    // 背负战刃（竖持，刃尖朝上，挂背部不动）
    let (c, s) = yanhu_px(-0.5, 16.0, 2.6, 1.0, 12.0, 0.8);    voxel_box(parent, meshes, &gun, c, s, q);      // blade
    let (c, s) = yanhu_px(-1.5, 20.0, 2.4, 3.0, 1.0, 1.2);     voxel_box(parent, meshes, &dark, c, s, q);     // guard
    let (c, s) = yanhu_px(-1.0, 13.0, 2.5, 2.0, 4.0, 1.0);     voxel_box(parent, meshes, &boot, c, s, q);     // hilt
    // 裙摆（前后各两片，随腿侧静置）
    let (c, s) = yanhu_px(-4.0, 12.0, 1.6, 3.5, 8.0, 0.6);     voxel_box(parent, meshes, &dark, c, s, q);     // coatL
    let (c, s) = yanhu_px(0.5, 12.0, 1.6, 3.5, 8.0, 0.6);      voxel_box(parent, meshes, &dark, c, s, q);     // coatR
    let (c, s) = yanhu_px(-6.5, 22.0, -3.0, 3.5, 3.0, 4.0);    voxel_box(parent, meshes, &frost, c, s, q);    // shdL
    let (c, s) = yanhu_px(-6.5, 25.0, -3.0, 3.5, 0.8, 4.0);    voxel_box(parent, meshes, accent, c, s, q);    // shdLTrim
    let (c, s) = yanhu_px(3.0, 22.0, -3.0, 3.5, 3.0, 4.0);     voxel_box(parent, meshes, &frost, c, s, q);    // shdR
    let (c, s) = yanhu_px(3.0, 25.0, -3.0, 3.5, 0.8, 4.0);     voxel_box(parent, meshes, accent, c, s, q);    // shdRTrim

    // ---- 四肢枢轴（与焰狐同一关节点，动作系统直接复用） ----
    let arm_l_pivot = yanhu_point(-6.0, 24.0, -0.5);
    let mut arm_l = parent.spawn((SpatialBundle { transform: Transform::from_translation(arm_l_pivot), ..default() }, YanhuLimb::new(LimbKind::ArmL)));
    arm_l.with_children(|p| {
        let (c, s) = yanhu_px(-8.0, 19.0, -2.0, 4.0, 5.0, 4.0);    voxel_box(p, meshes, &frost, c - arm_l_pivot, s, q);   // armLup
        let (c, s) = yanhu_px(-8.0, 12.0, -2.0, 4.0, 7.0, 4.0);    voxel_box(p, meshes, &skin, c - arm_l_pivot, s, q);    // armLlo
        let (c, s) = yanhu_px(-8.1, 17.0, -2.1, 4.2, 1.6, 4.2);    voxel_box(p, meshes, accent, c - arm_l_pivot, s, q);   // armband
    });
    let arm_r_pivot = yanhu_point(6.0, 24.0, -0.5);
    let mut arm_r = parent.spawn((SpatialBundle { transform: Transform::from_translation(arm_r_pivot), ..default() }, YanhuLimb::new(LimbKind::ArmR)));
    arm_r.with_children(|p| {
        let (c, s) = yanhu_px(4.0, 19.0, -2.0, 4.0, 5.0, 4.0);     voxel_box(p, meshes, &frost, c - arm_r_pivot, s, q);   // armRup
        let (c, s) = yanhu_px(4.0, 12.0, -2.0, 4.0, 7.0, 4.0);     voxel_box(p, meshes, &skin, c - arm_r_pivot, s, q);    // armRlo
    });
    let leg_l_pivot = yanhu_point(-2.0, 12.0, 0.0);
    let mut leg_l = parent.spawn((SpatialBundle { transform: Transform::from_translation(leg_l_pivot), ..default() }, YanhuLimb::new(LimbKind::LegL)));
    leg_l.with_children(|p| {
        let (c, s) = yanhu_px(-4.0, 3.0, -2.0, 4.0, 9.0, 4.0);     voxel_box(p, meshes, &frost, c - leg_l_pivot, s, q);   // legL
        let (c, s) = yanhu_px(-4.0, 0.0, -3.0, 4.0, 3.0, 5.0);     voxel_box(p, meshes, &boot, c - leg_l_pivot, s, q);    // bootL
    });
    let leg_r_pivot = yanhu_point(2.0, 12.0, 0.0);
    let mut leg_r = parent.spawn((SpatialBundle { transform: Transform::from_translation(leg_r_pivot), ..default() }, YanhuLimb::new(LimbKind::LegR)));
    leg_r.with_children(|p| {
        let (c, s) = yanhu_px(0.0, 3.0, -2.0, 4.0, 9.0, 4.0);      voxel_box(p, meshes, &frost, c - leg_r_pivot, s, q);   // legR
        let (c, s) = yanhu_px(0.0, 0.0, -3.0, 4.0, 3.0, 5.0);      voxel_box(p, meshes, &boot, c - leg_r_pivot, s, q);    // bootR
    });

    // ---- 长马尾链式枢轴（复用 TailA/B/C，挂在后脑高度垂到腰后） ----
    let tail_a_pivot = yanhu_point(0.0, 24.0, 3.0);
    let tail_b_pivot = yanhu_point(0.0, 18.0, 3.0);
    let tail_c_pivot = yanhu_point(0.0, 10.0, 3.0);
    let mut tail_a = parent.spawn((SpatialBundle { transform: Transform::from_translation(tail_a_pivot), ..default() }, YanhuLimb::new(LimbKind::TailA)));
    tail_a.with_children(|ta| {
        let (c, s) = yanhu_px(-1.5, 22.5, 2.4, 3.0, 1.5, 4.0);     voxel_box(ta, meshes, accent, c - tail_a_pivot, s, q); // 发带
        let (c, s) = yanhu_px(-1.5, 18.0, 2.5, 3.0, 8.0, 4.0);     voxel_box(ta, meshes, &hair, c - tail_a_pivot, s, q);  // ponyA
        let mut tail_b = ta.spawn((SpatialBundle { transform: Transform::from_translation(tail_b_pivot - tail_a_pivot), ..default() }, YanhuLimb::new(LimbKind::TailB)));
        tail_b.with_children(|tb| {
            let (c, s) = yanhu_px(-1.25, 10.0, 2.8, 2.5, 8.0, 3.4);    voxel_box(tb, meshes, &hair_dark, c - tail_b_pivot, s, q); // ponyB
            let mut tail_c = tb.spawn((SpatialBundle { transform: Transform::from_translation(tail_c_pivot - tail_b_pivot), ..default() }, YanhuLimb::new(LimbKind::TailC)));
            tail_c.with_children(|tc| {
                let (c, s) = yanhu_px(-1.0, 4.0, 3.0, 2.0, 6.0, 3.0);      voxel_box(tc, meshes, &hair, c - tail_c_pivot, s, q);  // ponyC
            });
        });
    });

    // ---- 头（枢轴随瞄准俯仰；冰晶头冠 + 双冰角） ----
    let head_base = Vec3::new(0.0, 24.0 * YANHU_S, 0.0);
    let mut head_pivot = parent.spawn(SpatialBundle {
        transform: Transform::from_translation(head_base),
        ..default()
    });
    head_pivot.insert(PlayerHeadPivot);
    head_pivot.with_children(|hd| {
        let tilt_l = Quat::from_rotation_z(0.30);
        let tilt_r = Quat::from_rotation_z(-0.30);
        let (c, s) = yanhu_px(-4.0, 24.0, -4.0, 8.0, 8.0, 8.0);    voxel_box(hd, meshes, &skin, c - head_base, s, q);      // head
        let (c, s) = yanhu_px(-3.0, 27.0, -4.3, 2.0, 2.0, 0.3);    voxel_box(hd, meshes, eye, c - head_base, s, q);        // eyeL
        let (c, s) = yanhu_px(1.0, 27.0, -4.3, 2.0, 2.0, 0.3);     voxel_box(hd, meshes, eye, c - head_base, s, q);        // eyeR
        let (c, s) = yanhu_px(-4.5, 29.0, -4.5, 9.0, 3.0, 9.0);    voxel_box(hd, meshes, &hair, c - head_base, s, q);      // hairCap
        let (c, s) = yanhu_px(-4.5, 24.0, 3.5, 9.0, 6.0, 1.0);     voxel_box(hd, meshes, &hair_dark, c - head_base, s, q); // hairBack
        let (c, s) = yanhu_px(-5.0, 24.0, -4.0, 1.0, 6.0, 8.0);    voxel_box(hd, meshes, &hair, c - head_base, s, q);      // hairSideL
        let (c, s) = yanhu_px(4.0, 24.0, -4.0, 1.0, 6.0, 8.0);     voxel_box(hd, meshes, &hair, c - head_base, s, q);      // hairSideR
        let (c, s) = yanhu_px(-4.0, 27.0, -5.0, 8.0, 3.0, 0.5);    voxel_box(hd, meshes, &hair, c - head_base, s, q);      // bangs
        let (c, s) = yanhu_px(-2.5, 31.2, -4.4, 5.0, 1.0, 0.4);    voxel_box(hd, meshes, accent, c - head_base, s, q);     // 冰晶头冠
        let (c, s) = yanhu_px(-6.2, 30.0, -1.0, 1.5, 4.0, 1.5);    voxel_box(hd, meshes, accent, c - head_base, s, tilt_l);// 冰角L
        let (c, s) = yanhu_px(4.7, 30.0, -1.0, 1.5, 4.0, 1.5);     voxel_box(hd, meshes, accent, c - head_base, s, tilt_r);// 冰角R
    });

    // ---- 枪（与焰狐同一持枪枢轴，瞄准从腰际举到肩上） ----
    let (gun_center, _) = yanhu_px(4.2, 16.0, -6.0, 1.0, 1.0, 7.0);
    let mut gun_pivot = parent.spawn(SpatialBundle {
        transform: Transform::from_translation(Vec3::new(0.517, 1.55, 0.32)),
        ..default()
    });
    gun_pivot.insert(PlayerAimGun {
        base: Vec3::new(0.517, 1.55, 0.32),
        raised: Vec3::new(0.517, 2.5, 0.38),
    });
    gun_pivot.with_children(|gp| {
        let (c, s) = yanhu_px(4.2, 16.0, -6.0, 1.0, 1.0, 7.0);    voxel_box(gp, meshes, &gun, c - gun_center, s, q);   // gunBody
        let (c, s) = yanhu_px(4.3, 14.5, -3.0, 0.7, 1.5, 2.0);     voxel_box(gp, meshes, &dark, c - gun_center, s, q);  // gunMag
        let (c, s) = yanhu_px(4.3, 15.8, 0.0, 0.7, 0.8, 2.0);      voxel_box(gp, meshes, &dark, c - gun_center, s, q);  // gunStock
        let (c, s) = yanhu_px(4.3, 17.0, -3.0, 0.4, 1.0, 1.0);     voxel_box(gp, meshes, accent, c - gun_center, s, q); // gunSight
    });
}

/// 雷豹干员模型（电系，[YSM] 方块人美学）：
/// 炭黑突击甲 + 狂野短发 + 豹耳 + 额前战术目镜 + 四肢雷纹（发光）+ 豹尾电环。
/// 枢轴约定与焰狐一致（YanhuLimb 四肢 + TailA/B/C 豹尾链），动作由 yanhu_action_system 统一驱动。
fn build_leibao(
    parent: &mut ChildBuilder,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    accent: &Handle<StandardMaterial>,
    eye: &Handle<StandardMaterial>,
) {
    let skin = mat_voxel(materials, palette::SKIN);
    let charcoal = mat_voxel(materials, Color::srgb(0.22, 0.24, 0.27)); // 炭黑甲
    let dark = mat_voxel(materials, palette::TACTICAL_DARK);
    let boot = mat_voxel(materials, palette::BOOTS);
    let gun = mat_voxel(materials, Color::srgb(0.3, 0.3, 0.35));
    let hair = mat_voxel(materials, Color::srgb(0.88, 0.88, 0.90));     // 银白乱发
    let hair_dark = mat_voxel(materials, Color::srgb(0.55, 0.56, 0.60));
    let q = Quat::IDENTITY;

    // ---- 躯干/四肢（挂根） ----
    let (c, s) = yanhu_px(-4.0, 12.0, -2.0, 8.0, 12.0, 4.0);   voxel_box(parent, meshes, &charcoal, c, s, q); // torso
    let (c, s) = yanhu_px(-4.0, 12.0, -2.5, 8.0, 2.0, 5.0);    voxel_box(parent, meshes, &dark, c, s, q);     // belt
    let (c, s) = yanhu_px(-4.0, 15.0, -3.0, 8.0, 5.0, 1.0);    voxel_box(parent, meshes, &dark, c, s, q);     // vest
    let (c, s) = yanhu_px(-1.0, 16.0, -3.3, 2.0, 2.0, 0.3);    voxel_box(parent, meshes, accent, c, s, q);    // vestPin
    let (c, s) = yanhu_px(-6.5, 22.0, -3.0, 3.5, 3.0, 4.0);    voxel_box(parent, meshes, &charcoal, c, s, q); // shdL
    let (c, s) = yanhu_px(3.0, 22.0, -3.0, 3.5, 3.0, 4.0);     voxel_box(parent, meshes, &charcoal, c, s, q); // shdR
    let (c, s) = yanhu_px(-6.5, 24.5, -3.0, 3.5, 0.6, 4.0);    voxel_box(parent, meshes, accent, c, s, q);    // 雷纹L
    let (c, s) = yanhu_px(3.0, 24.5, -3.0, 3.5, 0.6, 4.0);     voxel_box(parent, meshes, accent, c, s, q);    // 雷纹R

    // ---- 四肢枢轴（与焰狐同一关节点，动作系统直接复用） ----
    let arm_l_pivot = yanhu_point(-6.0, 24.0, -0.5);
    let mut arm_l = parent.spawn((SpatialBundle { transform: Transform::from_translation(arm_l_pivot), ..default() }, YanhuLimb::new(LimbKind::ArmL)));
    arm_l.with_children(|p| {
        let (c, s) = yanhu_px(-8.0, 19.0, -2.0, 4.0, 5.0, 4.0);    voxel_box(p, meshes, &charcoal, c - arm_l_pivot, s, q); // armLup
        let (c, s) = yanhu_px(-8.0, 12.0, -2.0, 4.0, 7.0, 4.0);    voxel_box(p, meshes, &skin, c - arm_l_pivot, s, q);     // armLlo
        let (c, s) = yanhu_px(-8.1, 14.5, -2.1, 4.2, 0.6, 4.2);    voxel_box(p, meshes, accent, c - arm_l_pivot, s, q);    // 雷纹臂
    });
    let arm_r_pivot = yanhu_point(6.0, 24.0, -0.5);
    let mut arm_r = parent.spawn((SpatialBundle { transform: Transform::from_translation(arm_r_pivot), ..default() }, YanhuLimb::new(LimbKind::ArmR)));
    arm_r.with_children(|p| {
        let (c, s) = yanhu_px(4.0, 19.0, -2.0, 4.0, 5.0, 4.0);     voxel_box(p, meshes, &charcoal, c - arm_r_pivot, s, q); // armRup
        let (c, s) = yanhu_px(4.0, 12.0, -2.0, 4.0, 7.0, 4.0);     voxel_box(p, meshes, &skin, c - arm_r_pivot, s, q);     // armRlo
        let (c, s) = yanhu_px(3.9, 14.5, -2.1, 4.2, 0.6, 4.2);     voxel_box(p, meshes, accent, c - arm_r_pivot, s, q);    // 雷纹臂
    });
    let leg_l_pivot = yanhu_point(-2.0, 12.0, 0.0);
    let mut leg_l = parent.spawn((SpatialBundle { transform: Transform::from_translation(leg_l_pivot), ..default() }, YanhuLimb::new(LimbKind::LegL)));
    leg_l.with_children(|p| {
        let (c, s) = yanhu_px(-4.0, 3.0, -2.0, 4.0, 9.0, 4.0);     voxel_box(p, meshes, &charcoal, c - leg_l_pivot, s, q); // legL
        let (c, s) = yanhu_px(-4.0, 6.5, -2.1, 4.2, 0.6, 4.2);     voxel_box(p, meshes, accent, c - leg_l_pivot, s, q);    // 雷纹腿
        let (c, s) = yanhu_px(-4.0, 0.0, -3.0, 4.0, 3.0, 5.0);     voxel_box(p, meshes, &boot, c - leg_l_pivot, s, q);     // bootL
    });
    let leg_r_pivot = yanhu_point(2.0, 12.0, 0.0);
    let mut leg_r = parent.spawn((SpatialBundle { transform: Transform::from_translation(leg_r_pivot), ..default() }, YanhuLimb::new(LimbKind::LegR)));
    leg_r.with_children(|p| {
        let (c, s) = yanhu_px(0.0, 3.0, -2.0, 4.0, 9.0, 4.0);      voxel_box(p, meshes, &charcoal, c - leg_r_pivot, s, q); // legR
        let (c, s) = yanhu_px(-0.1, 6.5, -2.1, 4.2, 0.6, 4.2);     voxel_box(p, meshes, accent, c - leg_r_pivot, s, q);    // 雷纹腿
        let (c, s) = yanhu_px(0.0, 0.0, -3.0, 4.0, 3.0, 5.0);      voxel_box(p, meshes, &boot, c - leg_r_pivot, s, q);     // bootR
    });

    // ---- 豹尾链式枢轴（细长水平豹尾，尾尖带电环） ----
    let tail_a_pivot = yanhu_point(0.0, 14.5, 2.0);
    let tail_b_pivot = yanhu_point(0.0, 15.5, 8.0);
    let tail_c_pivot = yanhu_point(0.0, 16.0, 13.0);
    let mut tail_a = parent.spawn((SpatialBundle { transform: Transform::from_translation(tail_a_pivot), ..default() }, YanhuLimb::new(LimbKind::TailA)));
    tail_a.with_children(|ta| {
        let (c, s) = yanhu_px(-1.0, 13.5, 2.0, 2.0, 3.0, 7.0);     voxel_box(ta, meshes, &hair_dark, c - tail_a_pivot, s, q); // tailA
        let (c, s) = yanhu_px(-1.1, 14.0, 6.6, 2.2, 2.2, 0.7);     voxel_box(ta, meshes, accent, c - tail_a_pivot, s, q);     // 电环A
        let mut tail_b = ta.spawn((SpatialBundle { transform: Transform::from_translation(tail_b_pivot - tail_a_pivot), ..default() }, YanhuLimb::new(LimbKind::TailB)));
        tail_b.with_children(|tb| {
            let (c, s) = yanhu_px(-0.75, 14.5, 8.0, 1.5, 2.0, 6.0);    voxel_box(tb, meshes, &hair_dark, c - tail_b_pivot, s, q); // tailB
            let (c, s) = yanhu_px(-0.85, 15.0, 12.2, 1.7, 1.7, 0.7);   voxel_box(tb, meshes, accent, c - tail_b_pivot, s, q);     // 电环B
            let mut tail_c = tb.spawn((SpatialBundle { transform: Transform::from_translation(tail_c_pivot - tail_b_pivot), ..default() }, YanhuLimb::new(LimbKind::TailC)));
            tail_c.with_children(|tc| {
                let (c, s) = yanhu_px(-0.5, 15.0, 13.0, 1.0, 1.5, 4.0);    voxel_box(tc, meshes, &hair, c - tail_c_pivot, s, q);  // tailC（银白尾尖）
            });
        });
    });

    // ---- 头（枢轴随瞄准俯仰；豹耳 + 额前战术目镜） ----
    let head_base = Vec3::new(0.0, 24.0 * YANHU_S, 0.0);
    let mut head_pivot = parent.spawn(SpatialBundle {
        transform: Transform::from_translation(head_base),
        ..default()
    });
    head_pivot.insert(PlayerHeadPivot);
    head_pivot.with_children(|hd| {
        let tilt_l = Quat::from_rotation_z(0.25);
        let tilt_r = Quat::from_rotation_z(-0.25);
        let (c, s) = yanhu_px(-4.0, 24.0, -4.0, 8.0, 8.0, 8.0);    voxel_box(hd, meshes, &skin, c - head_base, s, q);      // head
        let (c, s) = yanhu_px(-3.0, 27.0, -4.3, 2.0, 2.0, 0.3);    voxel_box(hd, meshes, eye, c - head_base, s, q);        // eyeL
        let (c, s) = yanhu_px(1.0, 27.0, -4.3, 2.0, 2.0, 0.3);     voxel_box(hd, meshes, eye, c - head_base, s, q);        // eyeR
        let (c, s) = yanhu_px(-4.5, 29.0, -4.5, 9.0, 3.0, 9.0);    voxel_box(hd, meshes, &hair, c - head_base, s, q);      // hairCap
        let (c, s) = yanhu_px(-4.5, 24.0, 3.5, 9.0, 5.0, 1.0);     voxel_box(hd, meshes, &hair_dark, c - head_base, s, q); // hairBack
        let (c, s) = yanhu_px(-4.0, 27.0, -5.0, 8.0, 3.0, 0.5);    voxel_box(hd, meshes, &hair, c - head_base, s, q);      // bangs（碎刘海）
        let (c, s) = yanhu_px(-4.5, 30.0, -5.2, 9.0, 1.2, 0.6);    voxel_box(hd, meshes, &dark, c - head_base, s, q);      // 目镜框
        let (c, s) = yanhu_px(-3.0, 30.1, -5.4, 6.0, 1.0, 0.3);    voxel_box(hd, meshes, accent, c - head_base, s, q);     // 目镜亮带
        let (c, s) = yanhu_px(-6.0, 32.0, -2.0, 3.0, 3.0, 2.0);    voxel_box(hd, meshes, &hair_dark, c - head_base, s, tilt_l); // 豹耳L
        let (c, s) = yanhu_px(-5.4, 32.6, -2.3, 1.8, 1.8, 0.3);    voxel_box(hd, meshes, accent, c - head_base, s, tilt_l);     // 耳内L
        let (c, s) = yanhu_px(3.0, 32.0, -2.0, 3.0, 3.0, 2.0);     voxel_box(hd, meshes, &hair_dark, c - head_base, s, tilt_r); // 豹耳R
        let (c, s) = yanhu_px(3.6, 32.6, -2.3, 1.8, 1.8, 0.3);     voxel_box(hd, meshes, accent, c - head_base, s, tilt_r);     // 耳内R
    });

    // ---- 枪（与焰狐同一持枪枢轴） ----
    let (gun_center, _) = yanhu_px(4.2, 16.0, -6.0, 1.0, 1.0, 7.0);
    let mut gun_pivot = parent.spawn(SpatialBundle {
        transform: Transform::from_translation(Vec3::new(0.517, 1.55, 0.32)),
        ..default()
    });
    gun_pivot.insert(PlayerAimGun {
        base: Vec3::new(0.517, 1.55, 0.32),
        raised: Vec3::new(0.517, 2.5, 0.38),
    });
    gun_pivot.with_children(|gp| {
        let (c, s) = yanhu_px(4.2, 16.0, -6.0, 1.0, 1.0, 7.0);    voxel_box(gp, meshes, &gun, c - gun_center, s, q);   // gunBody
        let (c, s) = yanhu_px(4.3, 14.5, -3.0, 0.7, 1.5, 2.0);     voxel_box(gp, meshes, &dark, c - gun_center, s, q);  // gunMag
        let (c, s) = yanhu_px(4.3, 15.8, 0.0, 0.7, 0.8, 2.0);      voxel_box(gp, meshes, &dark, c - gun_center, s, q);  // gunStock
        let (c, s) = yanhu_px(4.3, 17.0, -3.0, 0.4, 1.0, 1.0);     voxel_box(gp, meshes, accent, c - gun_center, s, q); // gunSight
    });
}

/// 毒蛛干员模型（毒系，[YSM] 方块人美学）：
/// 紫袍兜帽 + 半脸面罩 + 四根蛛足（背部展开，静置装饰）+ 蛛徽 + 长直发。
/// 枢轴约定与焰狐一致（YanhuLimb 四肢 + TailA/B/C 后发链），动作由 yanhu_action_system 统一驱动。
fn build_duzhu(
    parent: &mut ChildBuilder,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    accent: &Handle<StandardMaterial>,
    eye: &Handle<StandardMaterial>,
) {
    let skin = mat_voxel(materials, palette::SKIN);
    let robe = mat_voxel(materials, Color::srgb(0.30, 0.24, 0.38));     // 暗紫袍
    let robe_dark = mat_voxel(materials, Color::srgb(0.20, 0.16, 0.27));
    let boot = mat_voxel(materials, palette::BOOTS);
    let gun = mat_voxel(materials, Color::srgb(0.3, 0.3, 0.35));
    let hair = mat_voxel(materials, Color::srgb(0.48, 0.38, 0.62));     // 紫发
    let q = Quat::IDENTITY;

    // ---- 躯干/四肢（挂根） ----
    let (c, s) = yanhu_px(-4.0, 12.0, -2.0, 8.0, 12.0, 4.0);   voxel_box(parent, meshes, &robe, c, s, q);     // torso
    let (c, s) = yanhu_px(-4.0, 12.0, -2.5, 8.0, 2.0, 5.0);    voxel_box(parent, meshes, &robe_dark, c, s, q);// belt
    let (c, s) = yanhu_px(-4.0, 15.0, -3.0, 8.0, 5.0, 1.0);    voxel_box(parent, meshes, &robe_dark, c, s, q);// vest
    let (c, s) = yanhu_px(-1.0, 16.0, -3.3, 2.0, 2.0, 0.3);    voxel_box(parent, meshes, accent, c, s, q);    // 蛛徽（发光）
    // 四根蛛足：从背部向两侧展开（静置装饰，左右各二）
    let (c, s) = yanhu_px(-9.0, 20.0, 2.0, 6.0, 0.9, 0.9);     voxel_box(parent, meshes, &robe_dark, c, s, q);// 蛛足L1
    let (c, s) = yanhu_px(-10.0, 15.5, 2.2, 7.0, 0.9, 0.9);    voxel_box(parent, meshes, &robe_dark, c, s, q);// 蛛足L2
    let (c, s) = yanhu_px(3.0, 20.0, 2.0, 6.0, 0.9, 0.9);      voxel_box(parent, meshes, &robe_dark, c, s, q);// 蛛足R1
    let (c, s) = yanhu_px(3.0, 15.5, 2.2, 7.0, 0.9, 0.9);      voxel_box(parent, meshes, &robe_dark, c, s, q);// 蛛足R2
    // 裙摆（前后各两片）
    let (c, s) = yanhu_px(-4.0, 12.0, 1.6, 3.5, 9.0, 0.6);     voxel_box(parent, meshes, &robe_dark, c, s, q);// coatL
    let (c, s) = yanhu_px(0.5, 12.0, 1.6, 3.5, 9.0, 0.6);      voxel_box(parent, meshes, &robe_dark, c, s, q);// coatR
    let (c, s) = yanhu_px(-4.0, 21.0, 2.4, 8.0, 9.0, 0.6);     voxel_box(parent, meshes, &robe, c, s, q);     // 披风（后背整片）

    // ---- 四肢枢轴（与焰狐同一关节点，动作系统直接复用） ----
    let arm_l_pivot = yanhu_point(-6.0, 24.0, -0.5);
    let mut arm_l = parent.spawn((SpatialBundle { transform: Transform::from_translation(arm_l_pivot), ..default() }, YanhuLimb::new(LimbKind::ArmL)));
    arm_l.with_children(|p| {
        let (c, s) = yanhu_px(-8.0, 19.0, -2.0, 4.0, 5.0, 4.0);    voxel_box(p, meshes, &robe, c - arm_l_pivot, s, q);    // armLup
        let (c, s) = yanhu_px(-8.0, 12.0, -2.0, 4.0, 7.0, 4.0);    voxel_box(p, meshes, &robe_dark, c - arm_l_pivot, s, q); // armLlo（袖手套）
    });
    let arm_r_pivot = yanhu_point(6.0, 24.0, -0.5);
    let mut arm_r = parent.spawn((SpatialBundle { transform: Transform::from_translation(arm_r_pivot), ..default() }, YanhuLimb::new(LimbKind::ArmR)));
    arm_r.with_children(|p| {
        let (c, s) = yanhu_px(4.0, 19.0, -2.0, 4.0, 5.0, 4.0);     voxel_box(p, meshes, &robe, c - arm_r_pivot, s, q);    // armRup
        let (c, s) = yanhu_px(4.0, 12.0, -2.0, 4.0, 7.0, 4.0);     voxel_box(p, meshes, &robe_dark, c - arm_r_pivot, s, q); // armRlo
    });
    let leg_l_pivot = yanhu_point(-2.0, 12.0, 0.0);
    let mut leg_l = parent.spawn((SpatialBundle { transform: Transform::from_translation(leg_l_pivot), ..default() }, YanhuLimb::new(LimbKind::LegL)));
    leg_l.with_children(|p| {
        let (c, s) = yanhu_px(-4.0, 3.0, -2.0, 4.0, 9.0, 4.0);     voxel_box(p, meshes, &robe_dark, c - leg_l_pivot, s, q); // legL
        let (c, s) = yanhu_px(-4.0, 0.0, -3.0, 4.0, 3.0, 5.0);     voxel_box(p, meshes, &boot, c - leg_l_pivot, s, q);      // bootL
    });
    let leg_r_pivot = yanhu_point(2.0, 12.0, 0.0);
    let mut leg_r = parent.spawn((SpatialBundle { transform: Transform::from_translation(leg_r_pivot), ..default() }, YanhuLimb::new(LimbKind::LegR)));
    leg_r.with_children(|p| {
        let (c, s) = yanhu_px(0.0, 3.0, -2.0, 4.0, 9.0, 4.0);      voxel_box(p, meshes, &robe_dark, c - leg_r_pivot, s, q); // legR
        let (c, s) = yanhu_px(0.0, 0.0, -3.0, 4.0, 3.0, 5.0);      voxel_box(p, meshes, &boot, c - leg_r_pivot, s, q);      // bootR
    });

    // ---- 后发链式枢轴（复用 TailA/B/C，齐腰长直发） ----
    let tail_a_pivot = yanhu_point(0.0, 24.0, 3.0);
    let tail_b_pivot = yanhu_point(0.0, 18.0, 3.0);
    let tail_c_pivot = yanhu_point(0.0, 10.0, 3.0);
    let mut tail_a = parent.spawn((SpatialBundle { transform: Transform::from_translation(tail_a_pivot), ..default() }, YanhuLimb::new(LimbKind::TailA)));
    tail_a.with_children(|ta| {
        let (c, s) = yanhu_px(-2.0, 18.0, 2.6, 4.0, 8.0, 3.0);     voxel_box(ta, meshes, &hair, c - tail_a_pivot, s, q);  // hairA
        let mut tail_b = ta.spawn((SpatialBundle { transform: Transform::from_translation(tail_b_pivot - tail_a_pivot), ..default() }, YanhuLimb::new(LimbKind::TailB)));
        tail_b.with_children(|tb| {
            let (c, s) = yanhu_px(-1.75, 10.0, 2.8, 3.5, 8.0, 2.6);    voxel_box(tb, meshes, &hair, c - tail_b_pivot, s, q);  // hairB
            let mut tail_c = tb.spawn((SpatialBundle { transform: Transform::from_translation(tail_c_pivot - tail_b_pivot), ..default() }, YanhuLimb::new(LimbKind::TailC)));
            tail_c.with_children(|tc| {
                let (c, s) = yanhu_px(-1.5, 4.0, 3.0, 3.0, 6.0, 2.2);      voxel_box(tc, meshes, accent, c - tail_c_pivot, s, q); // 发梢（毒绿渐变）
            });
        });
    });

    // ---- 头（枢轴随瞄准俯仰；兜帽 + 半脸面罩） ----
    let head_base = Vec3::new(0.0, 24.0 * YANHU_S, 0.0);
    let mut head_pivot = parent.spawn(SpatialBundle {
        transform: Transform::from_translation(head_base),
        ..default()
    });
    head_pivot.insert(PlayerHeadPivot);
    head_pivot.with_children(|hd| {
        let (c, s) = yanhu_px(-4.0, 24.0, -4.0, 8.0, 8.0, 8.0);    voxel_box(hd, meshes, &skin, c - head_base, s, q);      // head
        let (c, s) = yanhu_px(-3.0, 27.0, -4.3, 2.0, 2.0, 0.3);    voxel_box(hd, meshes, eye, c - head_base, s, q);        // eyeL
        let (c, s) = yanhu_px(1.0, 27.0, -4.3, 2.0, 2.0, 0.3);     voxel_box(hd, meshes, eye, c - head_base, s, q);        // eyeR
        let (c, s) = yanhu_px(-4.5, 31.0, -4.5, 9.0, 2.0, 9.0);    voxel_box(hd, meshes, &robe, c - head_base, s, q);      // 兜帽顶
        let (c, s) = yanhu_px(-5.0, 24.0, -4.2, 1.2, 8.0, 8.4);    voxel_box(hd, meshes, &robe, c - head_base, s, q);      // 兜帽侧L
        let (c, s) = yanhu_px(3.8, 24.0, -4.2, 1.2, 8.0, 8.4);     voxel_box(hd, meshes, &robe, c - head_base, s, q);      // 兜帽侧R
        let (c, s) = yanhu_px(-4.5, 31.0, -4.5, 9.0, 1.2, 9.0);    voxel_box(hd, meshes, &robe, c - head_base, s, q);      // 兜帽沿
        let (c, s) = yanhu_px(-4.0, 28.5, -4.4, 8.0, 1.0, 0.4);    voxel_box(hd, meshes, &hair, c - head_base, s, q);      // 刘海
        let (c, s) = yanhu_px(-4.0, 24.0, 3.5, 8.0, 7.0, 1.0);     voxel_box(hd, meshes, &hair, c - head_base, s, q);      // 后发
        let (c, s) = yanhu_px(-3.0, 24.2, -4.4, 6.0, 2.2, 0.4);    voxel_box(hd, meshes, &robe_dark, c - head_base, s, q); // 半脸面罩
    });

    // ---- 枪（与焰狐同一持枪枢轴） ----
    let (gun_center, _) = yanhu_px(4.2, 16.0, -6.0, 1.0, 1.0, 7.0);
    let mut gun_pivot = parent.spawn(SpatialBundle {
        transform: Transform::from_translation(Vec3::new(0.517, 1.55, 0.32)),
        ..default()
    });
    gun_pivot.insert(PlayerAimGun {
        base: Vec3::new(0.517, 1.55, 0.32),
        raised: Vec3::new(0.517, 2.5, 0.38),
    });
    gun_pivot.with_children(|gp| {
        let (c, s) = yanhu_px(4.2, 16.0, -6.0, 1.0, 1.0, 7.0);    voxel_box(gp, meshes, &gun, c - gun_center, s, q);   // gunBody
        let (c, s) = yanhu_px(4.3, 14.5, -3.0, 0.7, 1.5, 2.0);     voxel_box(gp, meshes, &robe_dark, c - gun_center, s, q); // gunMag
        let (c, s) = yanhu_px(4.3, 15.8, 0.0, 0.7, 0.8, 2.0);      voxel_box(gp, meshes, &robe_dark, c - gun_center, s, q); // gunStock
        let (c, s) = yanhu_px(4.3, 17.0, -3.0, 0.4, 1.0, 1.0);     voxel_box(gp, meshes, accent, c - gun_center, s, q); // gunSight
    });
}

pub fn mat_voxel(materials: &mut ResMut<Assets<StandardMaterial>>, color: Color) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color, metallic: 0.0, perceptual_roughness: 1.0, reflectance: 0.1, ..default()
    })
}

pub fn mat_emissive(materials: &mut ResMut<Assets<StandardMaterial>>, color: Color) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: color, emissive: color.to_linear() * 2.0, metallic: 0.0, perceptual_roughness: 1.0, ..default()
    })
}

// ===== 以下代码自 src/demo/mod.rs 原样迁入（干员模型/动作域） =====

#[derive(Component)]
/// 玩家头部枢轴（Aim Offset：瞄准时头部俯仰跟随视线）
pub struct PlayerHeadPivot;

#[derive(Component)]
/// 玩家持枪位：瞄准时从腰际举到肩上（无动画系统，用程序化过渡代替 Upper Body Layer）
pub struct PlayerAimGun { pub base: Vec3, pub raised: Vec3 }

// —— 程序化动作（yanhu_action_system 驱动） ——
/// 四肢/尾巴枢轴类别：行走摆动相位、瞄准姿态、尾巴摇摆均按此分支
#[derive(Component, Clone, Copy)]
pub struct YanhuLimb(LimbKind);

#[derive(Clone, Copy)]
enum LimbKind { ArmL, ArmR, LegL, LegR, TailA, TailB, TailC }

impl YanhuLimb {
    fn new(kind: LimbKind) -> Self { Self(kind) }
}

/// 玩家模型根：记录当前模型对应的干员序号，
/// 切干员时 operator_model_swap_system 据此 despawn 旧模型、按新干员重建
#[derive(Component)]
pub struct PlayerModelRoot { pub op_idx: usize }

// ===== 以下代码自 src/demo/mod.rs 原样迁入（干员模型/动作域） =====

// =============================================================================
// 干员模型切换 —— 切到霜刃换冰系专属模型，其余干员用焰狐
// =============================================================================
//
// spawn_player 建的模型包在 PlayerModelRoot 下并记录 op_idx；切换干员时本系统
// 对比 OperatorState.active 与记录值，不一致则 despawn 旧模型整体、按新干员重建。
// 必须排在 aim_rig_system 之前：重建后场景里保证只有一套头/枪枢轴可供 get_single。
pub fn operator_model_swap_system(
    mut commands: Commands,
    player_query: Query<(Entity, &OperatorState, &OperatorAccent), With<Player>>,
    model_query: Query<(Entity, &PlayerModelRoot), Without<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok((player, state, accent)) = player_query.get_single() else { return };
    let Ok((model_e, model)) = model_query.get_single() else { return };
    if model.op_idx == state.active { return; }

    commands.entity(model_e).despawn_recursive();
    let accent = accent.0.clone();
    let idx = state.active;
    // 眼睛颜色按干员区分：雷豹金黄 / 毒蛛毒绿 / 其余冰蓝；accent 沿用共享材质句柄
    // （switch_operator 已重着色为新干员元素色）
    let name = roster()[idx].name;
    let eye_color = match name {
        "雷豹" => Color::srgb(1.00, 0.82, 0.25),
        "毒蛛" => Color::srgb(0.55, 0.90, 0.35),
        _ => palette::EYE_BLUE,
    };
    let eye = mat_emissive(&mut materials, eye_color);
    commands.entity(player).with_children(move |p| {
        let mut model_root = p.spawn((SpatialBundle::default(), PlayerModelRoot { op_idx: idx }));
        model_root.with_children(|m| {
            // 每名干员一套专属外观；未建模的干员暂用焰狐体型（accent 已随元素变色）
            match name {
                "霜刃" => build_shuangren(m, &mut meshes, &mut materials, &accent, &eye),
                "雷豹" => build_leibao(m, &mut meshes, &mut materials, &accent, &eye),
                "毒蛛" => build_duzhu(m, &mut meshes, &mut materials, &accent, &eye),
                _ => build_yanhu(m, &mut meshes, &mut materials, &accent, &eye),
            }
        });
    });
}

// =============================================================================
// 程序化动作系统 —— 焰狐模型的行走 / 疾跑 / 跳跃 / 瞄准持枪 / 开火后坐 / 尾巴摇摆
// =============================================================================
//
// 全部在 YanhuLimb 枢轴（肩/髋/尾根）上做旋转合成，不触碰四肢盒子本身：
// - 行走摆臂摆腿：正弦步态，步频与摆幅随 planar_speed 增长（疾跑明显加快加大）；
// - 待机呼吸：手臂极小幅摆动 + 尾巴慢速摇摆（三节尾巴相位递延，链式跟随）；
// - 跳跃滞空：双腿前后分张、左臂后摆的空中姿态（瞄准态保留持枪臂）；
// - 瞄准：按 aim_lerp 混入持枪姿态（双臂前举托枪、微微内收），与步态平滑过渡；
// - 开火后坐：射击冷却刚重启时给持枪臂与枪身一个衰减上抬（约 0.25s 归零）。
// 必须排在 aim_rig_system 之后：枪枢轴的旋转由瞄准系统写入，本系统在其上叠加后坐。
pub fn yanhu_action_system(
    mut player_query: Query<&mut PlayerMovement, With<Player>>,
    cam_query: Query<&PlayerCamera>,
    mut limb_query: Query<(&YanhuLimb, &mut Transform), (
        Without<Player>, Without<PlayerHeadPivot>, Without<PlayerAimGun>,
        Without<CamPivot>, Without<ShoulderPivot>, Without<PitchPivot>, Without<SpringArm>,
    )>,
    mut gun_query: Query<&mut Transform, (
        With<PlayerAimGun>, Without<Player>, Without<PlayerHeadPivot>, Without<YanhuLimb>,
        Without<CamPivot>, Without<ShoulderPivot>, Without<PitchPivot>, Without<SpringArm>,
    )>,
    time: Res<Time>,
) {
    let Ok(cam) = cam_query.get_single() else { return };
    // 与 aim_rig_system 相同的 smoothstep 过渡系数
    let a = cam.aim_lerp;
    let aim_t = a * a * (3.0 - 2.0 * a);
    let t = time.elapsed_seconds();
    // 尾巴慢摆相位（待机也在摇）
    let tail_idle = t * 1.7;

    for movement in player_query.iter_mut() {
        let speed_ratio = (movement.planar_speed / 4.0).min(1.4);
        let stride = t * (5.0 + 4.0 * speed_ratio); // 步频：走 5，疾跑约 10.8
        let amp = speed_ratio.min(1.15);            // 摆幅随速度增长
        let s = stride.sin();
        // 开火后坐强度：射击冷却刚重启≈1，随冷却流逝平方衰减归零
        let kick: f32 = (1.0 - movement.shoot_cooldown.fraction()).powi(2);
        let airborne = !movement.is_grounded;

        for (limb, mut tf) in limb_query.iter_mut() {
            let (mut rx, mut ry, mut rz) = (0.0f32, 0.0f32, 0.0f32);
            match limb.0 {
                LimbKind::ArmL => {
                    rx = 0.65 * amp * s + 0.04 * (t * 2.0).sin(); // 与右臂反相摆动 + 呼吸
                    rz = 0.10 * aim_t;                            // 内收扶枪
                    if airborne { rx = -0.85; }
                    rx = lerp(rx, 1.05, aim_t) + 0.08 * kick;
                }
                LimbKind::ArmR => {
                    rx = -0.65 * amp * s + 0.04 * (t * 2.0).sin();
                    rz = -0.10 * aim_t;
                    if airborne { rx = -0.85; }
                    rx = lerp(rx, 1.25, aim_t) + 0.30 * kick;     // 持枪臂吃主要后坐
                }
                LimbKind::LegL => {
                    rx = -0.55 * amp * s;
                    if airborne { rx = -0.55; }                   // 滞空前腿抬起
                }
                LimbKind::LegR => {
                    rx = 0.55 * amp * s;
                    if airborne { rx = 0.40; }                    // 滞空后腿后蹬
                }
                LimbKind::TailA => {
                    ry = 0.26 * tail_idle.sin() + 0.22 * amp * s; // 待机慢摆 + 跑动甩尾
                    rx = -0.22 * amp;                             // 疾跑时尾巴微微上扬
                }
                LimbKind::TailB => { ry = 0.32 * (tail_idle - 0.6).sin() + 0.22 * amp * s; }
                LimbKind::TailC => { ry = 0.36 * (tail_idle - 1.2).sin(); }
            }
            tf.rotation = Quat::from_euler(EulerRot::XYZ, rx, ry, rz);
        }

        // 枪身后坐：在 aim_rig_system 写入的瞄准旋转上叠加一个上抬增量
        if let Ok(mut gun_t) = gun_query.get_single_mut() {
            gun_t.rotation = gun_t.rotation * Quat::from_rotation_x(0.10 * kick);
        }
    }
}