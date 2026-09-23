//! 四名干员的专属体素模型构建（来自 src/model/mod.rs 原"干员模型"段）
//! 共用同一方块骨架与枢轴约定（YanhuLimb 四肢 + TailA/B/C 尾/发链 + 头/枪枢轴），
//! 元素件（刘海挑染/胸徽/臂章/照门等）共享 accent 发光材质，切干员时随 swap 重着色。

use bevy::prelude::*;

use super::palette;
use super::rig::{LimbKind, PlayerAimGun, PlayerHeadPivot, YanhuLimb};

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
    parent: &mut ChildSpawnerCommands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mat: &Handle<StandardMaterial>,
    origin: Vec3,
    size: Vec3,
    rot: Quat,
) {
    parent.spawn((
        Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
        MeshMaterial3d(mat.clone()),
        Transform::from_translation(origin).with_rotation(rot),
    ));
}

/// 焰狐干员模型（[YSM] 是，史蒂夫模型 mod 美学，35 盒）：
/// 原版 Steve 方块骨架 + 特征方块（狐耳/三节尾/发冠刘海挑染）+ 二次元皮肤级配色。
/// 规格与 assets/characters/model/yanhu.geometry.json、yanhu_model_preview.png 一致。
/// 元素件（刘海挑染/胸徽/臂章/照门）共享 accent 发光材质——切干员时随 switch_operator 重着色。
pub fn build_yanhu(
    parent: &mut ChildSpawnerCommands,
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
    let mut arm_l = parent.spawn((Transform::from_translation(arm_l_pivot), YanhuLimb::new(LimbKind::ArmL)));
    arm_l.with_children(|p| {
        let (c, s) = yanhu_px(-8.0, 19.0, -2.0, 4.0, 5.0, 4.0);    voxel_box(p, meshes, &green, c - arm_l_pivot, s, q);   // armLup
        let (c, s) = yanhu_px(-8.0, 12.0, -2.0, 4.0, 7.0, 4.0);    voxel_box(p, meshes, &skin, c - arm_l_pivot, s, q);    // armLlo
    });
    let arm_r_pivot = yanhu_point(6.0, 24.0, -0.5);
    let mut arm_r = parent.spawn((Transform::from_translation(arm_r_pivot), YanhuLimb::new(LimbKind::ArmR)));
    arm_r.with_children(|p| {
        let (c, s) = yanhu_px(4.0, 19.0, -2.0, 4.0, 5.0, 4.0);     voxel_box(p, meshes, &green, c - arm_r_pivot, s, q);   // armRup
        let (c, s) = yanhu_px(3.9, 20.0, -2.1, 4.2, 2.0, 4.2);     voxel_box(p, meshes, accent, c - arm_r_pivot, s, q);   // armband
        let (c, s) = yanhu_px(4.0, 12.0, -2.0, 4.0, 7.0, 4.0);     voxel_box(p, meshes, &skin, c - arm_r_pivot, s, q);    // armRlo
    });
    let leg_l_pivot = yanhu_point(-2.0, 12.0, 0.0);
    let mut leg_l = parent.spawn((Transform::from_translation(leg_l_pivot), YanhuLimb::new(LimbKind::LegL)));
    leg_l.with_children(|p| {
        let (c, s) = yanhu_px(-4.0, 3.0, -2.0, 4.0, 9.0, 4.0);     voxel_box(p, meshes, &green, c - leg_l_pivot, s, q);   // legL
        let (c, s) = yanhu_px(-4.0, 0.0, -3.0, 4.0, 3.0, 5.0);     voxel_box(p, meshes, &boot, c - leg_l_pivot, s, q);    // bootL
    });
    let leg_r_pivot = yanhu_point(2.0, 12.0, 0.0);
    let mut leg_r = parent.spawn((Transform::from_translation(leg_r_pivot), YanhuLimb::new(LimbKind::LegR)));
    leg_r.with_children(|p| {
        let (c, s) = yanhu_px(0.0, 3.0, -2.0, 4.0, 9.0, 4.0);      voxel_box(p, meshes, &green, c - leg_r_pivot, s, q);   // legR
        let (c, s) = yanhu_px(0.0, 0.0, -3.0, 4.0, 3.0, 5.0);      voxel_box(p, meshes, &boot, c - leg_r_pivot, s, q);    // bootR
    });
    // 三节尾巴链式枢轴：A 挂躯干，B 挂 A 末端，C 挂 B 末端（摇摆时逐节跟随）
    let tail_a_pivot = yanhu_point(0.0, 14.5, 2.0);
    let tail_b_pivot = yanhu_point(0.0, 16.0, 8.0);
    let tail_c_pivot = yanhu_point(0.0, 17.0, 13.0);
    let mut tail_a = parent.spawn((Transform::from_translation(tail_a_pivot), YanhuLimb::new(LimbKind::TailA)));
    tail_a.with_children(|ta| {
        let (c, s) = yanhu_px(-1.5, 13.0, 2.0, 3.0, 3.0, 7.0);     voxel_box(ta, meshes, &hair, c - tail_a_pivot, s, q);   // tailA
        let mut tail_b = ta.spawn((Transform::from_translation(tail_b_pivot - tail_a_pivot), YanhuLimb::new(LimbKind::TailB)));
        tail_b.with_children(|tb| {
            let (c, s) = yanhu_px(-1.0, 15.0, 8.0, 2.0, 2.0, 6.0);     voxel_box(tb, meshes, &hair, c - tail_b_pivot, s, q);   // tailB
            let mut tail_c = tb.spawn((Transform::from_translation(tail_c_pivot - tail_b_pivot), YanhuLimb::new(LimbKind::TailC)));
            tail_c.with_children(|tc| {
                let (c, s) = yanhu_px(-1.0, 16.0, 13.0, 2.0, 2.0, 4.0);    voxel_box(tc, meshes, &cream, c - tail_c_pivot, s, q);  // tailC
            });
        });
    });

    // ---- 头（枢轴随瞄准俯仰；发/耳随头动，耳微外倾） ----
    let head_base = Vec3::new(0.0, 24.0 * YANHU_S, 0.0);
    let mut head_pivot = parent.spawn(Transform::from_translation(head_base));
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
    let mut gun_pivot = parent.spawn(Transform::from_translation(Vec3::new(0.517, 1.55, 0.32)));
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
pub(crate) fn build_shuangren(
    parent: &mut ChildSpawnerCommands,
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
    let mut arm_l = parent.spawn((Transform::from_translation(arm_l_pivot), YanhuLimb::new(LimbKind::ArmL)));
    arm_l.with_children(|p| {
        let (c, s) = yanhu_px(-8.0, 19.0, -2.0, 4.0, 5.0, 4.0);    voxel_box(p, meshes, &frost, c - arm_l_pivot, s, q);   // armLup
        let (c, s) = yanhu_px(-8.0, 12.0, -2.0, 4.0, 7.0, 4.0);    voxel_box(p, meshes, &skin, c - arm_l_pivot, s, q);    // armLlo
        let (c, s) = yanhu_px(-8.1, 17.0, -2.1, 4.2, 1.6, 4.2);    voxel_box(p, meshes, accent, c - arm_l_pivot, s, q);   // armband
    });
    let arm_r_pivot = yanhu_point(6.0, 24.0, -0.5);
    let mut arm_r = parent.spawn((Transform::from_translation(arm_r_pivot), YanhuLimb::new(LimbKind::ArmR)));
    arm_r.with_children(|p| {
        let (c, s) = yanhu_px(4.0, 19.0, -2.0, 4.0, 5.0, 4.0);     voxel_box(p, meshes, &frost, c - arm_r_pivot, s, q);   // armRup
        let (c, s) = yanhu_px(4.0, 12.0, -2.0, 4.0, 7.0, 4.0);     voxel_box(p, meshes, &skin, c - arm_r_pivot, s, q);    // armRlo
    });
    let leg_l_pivot = yanhu_point(-2.0, 12.0, 0.0);
    let mut leg_l = parent.spawn((Transform::from_translation(leg_l_pivot), YanhuLimb::new(LimbKind::LegL)));
    leg_l.with_children(|p| {
        let (c, s) = yanhu_px(-4.0, 3.0, -2.0, 4.0, 9.0, 4.0);     voxel_box(p, meshes, &frost, c - leg_l_pivot, s, q);   // legL
        let (c, s) = yanhu_px(-4.0, 0.0, -3.0, 4.0, 3.0, 5.0);     voxel_box(p, meshes, &boot, c - leg_l_pivot, s, q);    // bootL
    });
    let leg_r_pivot = yanhu_point(2.0, 12.0, 0.0);
    let mut leg_r = parent.spawn((Transform::from_translation(leg_r_pivot), YanhuLimb::new(LimbKind::LegR)));
    leg_r.with_children(|p| {
        let (c, s) = yanhu_px(0.0, 3.0, -2.0, 4.0, 9.0, 4.0);      voxel_box(p, meshes, &frost, c - leg_r_pivot, s, q);   // legR
        let (c, s) = yanhu_px(0.0, 0.0, -3.0, 4.0, 3.0, 5.0);      voxel_box(p, meshes, &boot, c - leg_r_pivot, s, q);    // bootR
    });

    // ---- 长马尾链式枢轴（复用 TailA/B/C，挂在后脑高度垂到腰后） ----
    let tail_a_pivot = yanhu_point(0.0, 24.0, 3.0);
    let tail_b_pivot = yanhu_point(0.0, 18.0, 3.0);
    let tail_c_pivot = yanhu_point(0.0, 10.0, 3.0);
    let mut tail_a = parent.spawn((Transform::from_translation(tail_a_pivot), YanhuLimb::new(LimbKind::TailA)));
    tail_a.with_children(|ta| {
        let (c, s) = yanhu_px(-1.5, 22.5, 2.4, 3.0, 1.5, 4.0);     voxel_box(ta, meshes, accent, c - tail_a_pivot, s, q); // 发带
        let (c, s) = yanhu_px(-1.5, 18.0, 2.5, 3.0, 8.0, 4.0);     voxel_box(ta, meshes, &hair, c - tail_a_pivot, s, q);  // ponyA
        let mut tail_b = ta.spawn((Transform::from_translation(tail_b_pivot - tail_a_pivot), YanhuLimb::new(LimbKind::TailB)));
        tail_b.with_children(|tb| {
            let (c, s) = yanhu_px(-1.25, 10.0, 2.8, 2.5, 8.0, 3.4);    voxel_box(tb, meshes, &hair_dark, c - tail_b_pivot, s, q); // ponyB
            let mut tail_c = tb.spawn((Transform::from_translation(tail_c_pivot - tail_b_pivot), YanhuLimb::new(LimbKind::TailC)));
            tail_c.with_children(|tc| {
                let (c, s) = yanhu_px(-1.0, 4.0, 3.0, 2.0, 6.0, 3.0);      voxel_box(tc, meshes, &hair, c - tail_c_pivot, s, q);  // ponyC
            });
        });
    });

    // ---- 头（枢轴随瞄准俯仰；冰晶头冠 + 双冰角） ----
    let head_base = Vec3::new(0.0, 24.0 * YANHU_S, 0.0);
    let mut head_pivot = parent.spawn(Transform::from_translation(head_base));
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
    let mut gun_pivot = parent.spawn(Transform::from_translation(Vec3::new(0.517, 1.55, 0.32)));
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
pub(crate) fn build_leibao(
    parent: &mut ChildSpawnerCommands,
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
    let mut arm_l = parent.spawn((Transform::from_translation(arm_l_pivot), YanhuLimb::new(LimbKind::ArmL)));
    arm_l.with_children(|p| {
        let (c, s) = yanhu_px(-8.0, 19.0, -2.0, 4.0, 5.0, 4.0);    voxel_box(p, meshes, &charcoal, c - arm_l_pivot, s, q); // armLup
        let (c, s) = yanhu_px(-8.0, 12.0, -2.0, 4.0, 7.0, 4.0);    voxel_box(p, meshes, &skin, c - arm_l_pivot, s, q);     // armLlo
        let (c, s) = yanhu_px(-8.1, 14.5, -2.1, 4.2, 0.6, 4.2);    voxel_box(p, meshes, accent, c - arm_l_pivot, s, q);    // 雷纹臂
    });
    let arm_r_pivot = yanhu_point(6.0, 24.0, -0.5);
    let mut arm_r = parent.spawn((Transform::from_translation(arm_r_pivot), YanhuLimb::new(LimbKind::ArmR)));
    arm_r.with_children(|p| {
        let (c, s) = yanhu_px(4.0, 19.0, -2.0, 4.0, 5.0, 4.0);     voxel_box(p, meshes, &charcoal, c - arm_r_pivot, s, q); // armRup
        let (c, s) = yanhu_px(4.0, 12.0, -2.0, 4.0, 7.0, 4.0);     voxel_box(p, meshes, &skin, c - arm_r_pivot, s, q);     // armRlo
        let (c, s) = yanhu_px(3.9, 14.5, -2.1, 4.2, 0.6, 4.2);     voxel_box(p, meshes, accent, c - arm_r_pivot, s, q);    // 雷纹臂
    });
    let leg_l_pivot = yanhu_point(-2.0, 12.0, 0.0);
    let mut leg_l = parent.spawn((Transform::from_translation(leg_l_pivot), YanhuLimb::new(LimbKind::LegL)));
    leg_l.with_children(|p| {
        let (c, s) = yanhu_px(-4.0, 3.0, -2.0, 4.0, 9.0, 4.0);     voxel_box(p, meshes, &charcoal, c - leg_l_pivot, s, q); // legL
        let (c, s) = yanhu_px(-4.0, 6.5, -2.1, 4.2, 0.6, 4.2);     voxel_box(p, meshes, accent, c - leg_l_pivot, s, q);    // 雷纹腿
        let (c, s) = yanhu_px(-4.0, 0.0, -3.0, 4.0, 3.0, 5.0);     voxel_box(p, meshes, &boot, c - leg_l_pivot, s, q);     // bootL
    });
    let leg_r_pivot = yanhu_point(2.0, 12.0, 0.0);
    let mut leg_r = parent.spawn((Transform::from_translation(leg_r_pivot), YanhuLimb::new(LimbKind::LegR)));
    leg_r.with_children(|p| {
        let (c, s) = yanhu_px(0.0, 3.0, -2.0, 4.0, 9.0, 4.0);      voxel_box(p, meshes, &charcoal, c - leg_r_pivot, s, q); // legR
        let (c, s) = yanhu_px(-0.1, 6.5, -2.1, 4.2, 0.6, 4.2);     voxel_box(p, meshes, accent, c - leg_r_pivot, s, q);    // 雷纹腿
        let (c, s) = yanhu_px(0.0, 0.0, -3.0, 4.0, 3.0, 5.0);      voxel_box(p, meshes, &boot, c - leg_r_pivot, s, q);     // bootR
    });

    // ---- 豹尾链式枢轴（细长水平豹尾，尾尖带电环） ----
    let tail_a_pivot = yanhu_point(0.0, 14.5, 2.0);
    let tail_b_pivot = yanhu_point(0.0, 15.5, 8.0);
    let tail_c_pivot = yanhu_point(0.0, 16.0, 13.0);
    let mut tail_a = parent.spawn((Transform::from_translation(tail_a_pivot), YanhuLimb::new(LimbKind::TailA)));
    tail_a.with_children(|ta| {
        let (c, s) = yanhu_px(-1.0, 13.5, 2.0, 2.0, 3.0, 7.0);     voxel_box(ta, meshes, &hair_dark, c - tail_a_pivot, s, q); // tailA
        let (c, s) = yanhu_px(-1.1, 14.0, 6.6, 2.2, 2.2, 0.7);     voxel_box(ta, meshes, accent, c - tail_a_pivot, s, q);     // 电环A
        let mut tail_b = ta.spawn((Transform::from_translation(tail_b_pivot - tail_a_pivot), YanhuLimb::new(LimbKind::TailB)));
        tail_b.with_children(|tb| {
            let (c, s) = yanhu_px(-0.75, 14.5, 8.0, 1.5, 2.0, 6.0);    voxel_box(tb, meshes, &hair_dark, c - tail_b_pivot, s, q); // tailB
            let (c, s) = yanhu_px(-0.85, 15.0, 12.2, 1.7, 1.7, 0.7);   voxel_box(tb, meshes, accent, c - tail_b_pivot, s, q);     // 电环B
            let mut tail_c = tb.spawn((Transform::from_translation(tail_c_pivot - tail_b_pivot), YanhuLimb::new(LimbKind::TailC)));
            tail_c.with_children(|tc| {
                let (c, s) = yanhu_px(-0.5, 15.0, 13.0, 1.0, 1.5, 4.0);    voxel_box(tc, meshes, &hair, c - tail_c_pivot, s, q);  // tailC（银白尾尖）
            });
        });
    });

    // ---- 头（枢轴随瞄准俯仰；豹耳 + 额前战术目镜） ----
    let head_base = Vec3::new(0.0, 24.0 * YANHU_S, 0.0);
    let mut head_pivot = parent.spawn(Transform::from_translation(head_base));
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
    let mut gun_pivot = parent.spawn(Transform::from_translation(Vec3::new(0.517, 1.55, 0.32)));
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
pub(crate) fn build_duzhu(
    parent: &mut ChildSpawnerCommands,
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
    let mut arm_l = parent.spawn((Transform::from_translation(arm_l_pivot), YanhuLimb::new(LimbKind::ArmL)));
    arm_l.with_children(|p| {
        let (c, s) = yanhu_px(-8.0, 19.0, -2.0, 4.0, 5.0, 4.0);    voxel_box(p, meshes, &robe, c - arm_l_pivot, s, q);    // armLup
        let (c, s) = yanhu_px(-8.0, 12.0, -2.0, 4.0, 7.0, 4.0);    voxel_box(p, meshes, &robe_dark, c - arm_l_pivot, s, q); // armLlo（袖手套）
    });
    let arm_r_pivot = yanhu_point(6.0, 24.0, -0.5);
    let mut arm_r = parent.spawn((Transform::from_translation(arm_r_pivot), YanhuLimb::new(LimbKind::ArmR)));
    arm_r.with_children(|p| {
        let (c, s) = yanhu_px(4.0, 19.0, -2.0, 4.0, 5.0, 4.0);     voxel_box(p, meshes, &robe, c - arm_r_pivot, s, q);    // armRup
        let (c, s) = yanhu_px(4.0, 12.0, -2.0, 4.0, 7.0, 4.0);     voxel_box(p, meshes, &robe_dark, c - arm_r_pivot, s, q); // armRlo
    });
    let leg_l_pivot = yanhu_point(-2.0, 12.0, 0.0);
    let mut leg_l = parent.spawn((Transform::from_translation(leg_l_pivot), YanhuLimb::new(LimbKind::LegL)));
    leg_l.with_children(|p| {
        let (c, s) = yanhu_px(-4.0, 3.0, -2.0, 4.0, 9.0, 4.0);     voxel_box(p, meshes, &robe_dark, c - leg_l_pivot, s, q); // legL
        let (c, s) = yanhu_px(-4.0, 0.0, -3.0, 4.0, 3.0, 5.0);     voxel_box(p, meshes, &boot, c - leg_l_pivot, s, q);      // bootL
    });
    let leg_r_pivot = yanhu_point(2.0, 12.0, 0.0);
    let mut leg_r = parent.spawn((Transform::from_translation(leg_r_pivot), YanhuLimb::new(LimbKind::LegR)));
    leg_r.with_children(|p| {
        let (c, s) = yanhu_px(0.0, 3.0, -2.0, 4.0, 9.0, 4.0);      voxel_box(p, meshes, &robe_dark, c - leg_r_pivot, s, q); // legR
        let (c, s) = yanhu_px(0.0, 0.0, -3.0, 4.0, 3.0, 5.0);      voxel_box(p, meshes, &boot, c - leg_r_pivot, s, q);      // bootR
    });

    // ---- 后发链式枢轴（复用 TailA/B/C，齐腰长直发） ----
    let tail_a_pivot = yanhu_point(0.0, 24.0, 3.0);
    let tail_b_pivot = yanhu_point(0.0, 18.0, 3.0);
    let tail_c_pivot = yanhu_point(0.0, 10.0, 3.0);
    let mut tail_a = parent.spawn((Transform::from_translation(tail_a_pivot), YanhuLimb::new(LimbKind::TailA)));
    tail_a.with_children(|ta| {
        let (c, s) = yanhu_px(-2.0, 18.0, 2.6, 4.0, 8.0, 3.0);     voxel_box(ta, meshes, &hair, c - tail_a_pivot, s, q);  // hairA
        let mut tail_b = ta.spawn((Transform::from_translation(tail_b_pivot - tail_a_pivot), YanhuLimb::new(LimbKind::TailB)));
        tail_b.with_children(|tb| {
            let (c, s) = yanhu_px(-1.75, 10.0, 2.8, 3.5, 8.0, 2.6);    voxel_box(tb, meshes, &hair, c - tail_b_pivot, s, q);  // hairB
            let mut tail_c = tb.spawn((Transform::from_translation(tail_c_pivot - tail_b_pivot), YanhuLimb::new(LimbKind::TailC)));
            tail_c.with_children(|tc| {
                let (c, s) = yanhu_px(-1.5, 4.0, 3.0, 3.0, 6.0, 2.2);      voxel_box(tc, meshes, accent, c - tail_c_pivot, s, q); // 发梢（毒绿渐变）
            });
        });
    });

    // ---- 头（枢轴随瞄准俯仰；兜帽 + 半脸面罩） ----
    let head_base = Vec3::new(0.0, 24.0 * YANHU_S, 0.0);
    let mut head_pivot = parent.spawn(Transform::from_translation(head_base));
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
    let mut gun_pivot = parent.spawn(Transform::from_translation(Vec3::new(0.517, 1.55, 0.32)));
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