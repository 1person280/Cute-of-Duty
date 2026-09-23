//! 干员模型置换系统（来自 src/model/mod.rs 原"模型切换"段）
//! spawn_player 建的模型包在 PlayerModelRoot 下并记录 op_idx；切换干员时本系统
//! 对比 OperatorState.active 与记录值，不一致则 despawn 旧模型整体、按新干员重建。
//! 必须排在 aim_rig_system 之前：重建后场景里保证只有一套头/枪枢轴可供 get_single。

use bevy::prelude::*;

use crate::operator::roster;

use super::components::{OperatorAccent, OperatorState, Player};
use super::operator_models::{build_duzhu, build_leibao, build_shuangren, build_yanhu, mat_emissive};
use super::palette;
use super::rig::PlayerModelRoot;

pub fn operator_model_swap_system(
    mut commands: Commands,
    player_query: Query<(Entity, &OperatorState, &OperatorAccent), With<Player>>,
    model_query: Query<(Entity, &PlayerModelRoot), Without<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok((player, state, accent)) = player_query.single() else { return };
    let Ok((model_e, model)) = model_query.single() else { return };
    if model.op_idx == state.active { return; }

    commands.entity(model_e).despawn();
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
        let mut model_root = p.spawn((Transform::default(), PlayerModelRoot { op_idx: idx }));
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