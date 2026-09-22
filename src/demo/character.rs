//! 角色生成：玩家/敌人实体与体素模型构建

use bevy::prelude::*;
use crate::element::ElementType;
use crate::operator::roster;
use crate::model::{
    build_yanhu, mat_emissive, mat_voxel, palette, OperatorAccent, Player, OperatorState, PlayerAimGun,
    PlayerHeadPivot, PlayerModelRoot, PlayerMovement,
};
use super::minimap::Faction;
use super::frontend::*;
use super::components::*;
use super::loadout::{Loadout, apply_loadout};

pub(crate) enum CharacterPreset { PlayerFire, EnemyIce, TeammateElectric }

pub(crate) fn spawn_player(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    loadout: &Loadout,
) {
    // 发光饰条 = 当前干员的元素色（切换干员时由 switch_operator 重新着色）
    let accent = mat_emissive(materials, roster()[0].element.color());
    let eye = mat_emissive(materials, palette::EYE_BLUE);

    // 起步背包按"仓库"选装落地（双主武器固定；物资/护甲/弹药池随勾选）
    let mut inventory = Inventory::default();
    let mut armor = Armor::default();
    apply_loadout(&mut inventory, &mut armor, loadout);

    let mut root = commands.spawn((
        SpatialBundle { transform: Transform::from_translation(pos), ..default() },
        Player,
        PlayerMovement::default(),
        Health::default(),
        armor,
        WeaponSlot::default(),
        OperatorState::default(),
        OperatorAccent(accent.clone()),
        inventory,
    ));
    root.with_children(|p| {
        // 玩家模型包进带标记的根：切干员时由 operator_model_swap_system 整体换模型
        // 默认焰狐（焦狐）；切到霜刃时换成冰系专属模型。敌人仍用通用 steve
        let mut model_root = p.spawn((
            SpatialBundle::default(),
            PlayerModelRoot { op_idx: 0 },
        ));
        model_root.with_children(|m| {
            build_yanhu(m, meshes, materials, &accent, &eye);
        });
    });
}

pub(crate) fn spawn_enemy(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    preset: CharacterPreset,
) {
    let (primary, accent_color, eye_color, helmet) = match preset {
        CharacterPreset::EnemyIce => (palette::ARMOR_DARK, ElementType::Ice.color(), palette::EYE_RED, true),
        CharacterPreset::TeammateElectric => (palette::TACTICAL_DARK, ElementType::Electric.color(), palette::EYE_BLUE, false),
        _ => (palette::TACTICAL_GREEN, ElementType::Fire.color(), palette::EYE_BLUE, true),
    };

    let body = mat_voxel(materials, primary);
    let skin = mat_voxel(materials, palette::SKIN);
    let accent = mat_emissive(materials, accent_color);
    let eye = mat_emissive(materials, eye_color);
    let armor = mat_voxel(materials, palette::ARMOR_GREY);
    let boot = mat_voxel(materials, palette::BOOTS);

    let mut root = commands.spawn((
        SpatialBundle { transform: Transform::from_translation(pos), ..default() },
        VoxelCharacter,
        // 阵营标记：小地图上敌我异色（PlayerFire 只用于玩家本体，不会走到这里）
        match preset {
            CharacterPreset::EnemyIce => Faction::Enemy,
            CharacterPreset::TeammateElectric | CharacterPreset::PlayerFire => Faction::Teammate,
        },
    ));
    root.with_children(|p| {
        build_steve(p, meshes, materials, helmet, false, &body, &skin, &accent, &eye, &armor, &boot);
    });
}

#[derive(Component)]
pub(crate) struct VoxelCharacter;

pub(crate) fn build_steve(
    parent: &mut ChildBuilder,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    helmet: bool,
    for_player: bool,
    body: &Handle<StandardMaterial>,
    skin: &Handle<StandardMaterial>,
    accent: &Handle<StandardMaterial>,
    eye: &Handle<StandardMaterial>,
    armor: &Handle<StandardMaterial>,
    boot: &Handle<StandardMaterial>,
) {
    // Head：包一层枢轴，玩家瞄准时做 Aim Offset 头部俯仰（敌人不需要）
    let mut head_pivot = parent.spawn(SpatialBundle {
        transform: Transform::from_xyz(0.0, 3.0, 0.0),
        ..default()
    });
    if for_player { head_pivot.insert(PlayerHeadPivot); }
    head_pivot.with_children(|h| {
        h.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            material: if helmet { armor.clone() } else { skin.clone() },
            ..default()
        });
        h.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(0.2, 0.15, 0.05)),
            material: eye.clone(),
            transform: Transform::from_xyz(-0.2, 0.05, 0.51),
            ..default()
        });
        h.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(0.2, 0.15, 0.05)),
            material: eye.clone(),
            transform: Transform::from_xyz(0.2, 0.05, 0.51),
            ..default()
        });
        h.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(0.3, 0.15, 0.3)),
            material: accent.clone(),
            transform: Transform::from_xyz(0.0, 0.55, 0.0),
            ..default()
        });
    });

    // Torso
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(1.0, 1.5, 0.5)),
        material: body.clone(),
        transform: Transform::from_xyz(0.0, 1.75, 0.0),
        ..default()
    });
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(0.3, 0.3, 0.05)),
        material: accent.clone(),
        transform: Transform::from_xyz(0.0, 2.0, 0.26),
        ..default()
    });
    // Shoulders
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(0.4, 0.4, 0.4)),
        material: armor.clone(),
        transform: Transform::from_xyz(-0.7, 2.3, 0.0),
        ..default()
    });
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(0.4, 0.4, 0.4)),
        material: armor.clone(),
        transform: Transform::from_xyz(0.7, 2.3, 0.0),
        ..default()
    });

    // Arms
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(0.5, 1.5, 0.5)),
        material: skin.clone(),
        transform: Transform::from_xyz(-0.75, 1.75, 0.0),
        ..default()
    });
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(0.55, 0.6, 0.55)),
        material: armor.clone(),
        transform: Transform::from_xyz(-0.75, 2.2, 0.0),
        ..default()
    });
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(0.5, 1.5, 0.5)),
        material: skin.clone(),
        transform: Transform::from_xyz(0.75, 1.75, 0.0),
        ..default()
    });
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(0.55, 0.6, 0.55)),
        material: armor.clone(),
        transform: Transform::from_xyz(0.75, 2.2, 0.0),
        ..default()
    });

    // Gun：整体包一层枢轴；玩家瞄准时从腰际举到肩上（程序化持枪姿态）
    let gun = mat_voxel(materials, Color::srgb(0.3, 0.3, 0.35));
    let mut gun_pivot = parent.spawn(SpatialBundle {
        transform: Transform::from_xyz(0.4, 1.3, 0.6),
        ..default()
    });
    if for_player {
        gun_pivot.insert(PlayerAimGun {
            base: Vec3::new(0.4, 1.3, 0.6),
            raised: Vec3::new(0.4, 2.35, 0.5),
        });
    }
    gun_pivot.with_children(|g| {
        g.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(0.15, 0.15, 1.2)),
            material: gun.clone(),
            ..default()
        });
        g.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(0.08, 0.08, 0.3)),
            material: accent.clone(),
            transform: Transform::from_xyz(0.0, 0.08, -0.1),
            ..default()
        });
        g.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(0.12, 0.25, 0.4)),
            material: gun,
            transform: Transform::from_xyz(0.0, -0.1, -0.7),
            ..default()
        });
    });

    // Legs
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(0.5, 1.5, 0.5)),
        material: body.clone(),
        transform: Transform::from_xyz(-0.25, 0.75, 0.0),
        ..default()
    });
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(0.55, 0.4, 0.6)),
        material: boot.clone(),
        transform: Transform::from_xyz(-0.25, 0.2, 0.05),
        ..default()
    });
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(0.5, 1.5, 0.5)),
        material: body.clone(),
        transform: Transform::from_xyz(0.25, 0.75, 0.0),
        ..default()
    });
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(0.55, 0.4, 0.6)),
        material: boot.clone(),
        transform: Transform::from_xyz(0.25, 0.2, 0.05),
        ..default()
    });
}


// =============================================================================
// Minimap —— 左上角正方形小地图 + 相机朝向罗盘条
// =============================================================================
//
// 结构与数据来源：
// - 掩体/站点来自核心库 `crate::map` 的 MapLayout（一次性静态摆位，障碍不动）；
// - 玩家/敌人/队友/训练靶/拾取物为动态实体，每帧映射到地图像素坐标；
// - 地图朝向固定为"正北朝上"（-Z 方向），玩家点随位置移动；
// - 地图上方的罗盘条：刻度随相机 yaw 滚动，中央指针 + 方位读数固定，
//   方位约定 正北=0°、正东=90°、正南=180°、正西=270°。


