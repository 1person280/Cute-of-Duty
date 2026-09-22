//! 持握手雷：背包取出→进入越肩瞄准姿态→左键沿相机视线投掷 / Esc 取消放回；
//! 以及未被落到的道具使用入口 use_item_at（恢复类立即生效、手雷转持握）。

use bevy::prelude::*;
use bevy::ecs::system::SystemParam;
use crate::demo::components::*;
use crate::demo::hud::{EffectAssets, EffectMatKind};
use crate::demo::combat::{ITEM_GRENADE_DAMAGE, ITEM_GRENADE_RADIUS};
use crate::model::{Player, PlayerCamera};

// =============================================================================
// Held Grenade (hover+R / quick keys / radial wheel 共用)
// =============================================================================

/// 已从背包取出、正在瞄准持握的手雷：进入越肩瞄准姿态，左键投出 / Esc 取消放回。
/// 投掷物必须"先瞄准后释放"，因此手雷不再有任何即时投掷路径。
#[derive(Resource, Default)]
pub(crate) struct HeldGrenade {
    pub(crate) item: Option<PickupItem>,
}

#[derive(Component)]
pub(crate) struct HeldHintRoot;

/// 持握手雷时的屏幕提示文案（固定，只切显隐）
pub(crate) const HELD_HINT_TEXT: &str = "手持手雷 — 左键投掷 · Esc 取消";

/// 使用背包第 index 个道具：恢复类立即生效；手雷取出持握（不立即消耗弹道），
/// 进入越肩瞄准后由 grenade_throw_system 投出，Esc 取消放回背包。
pub(crate) fn use_item_at(
    index: usize,
    items: &mut Vec<PickupItem>,
    health: &mut Health,
    armor: &mut Armor,
    held: &mut HeldGrenade,
) -> Option<String> {
    let item = items.get(index)?.clone();
    match &item.item_type {
        // 弹药拾取时已直接入弹药池，不会作为背包物品出现在这里
        PickupType::Ammo { .. } => {}
        PickupType::Health { amount } => {
            health.current = (health.current + *amount).min(health.max);
        }
        PickupType::Armor { amount } => {
            armor.current = (armor.current + *amount).min(armor.max);
        }
        PickupType::Grenade { .. } => {
            // 投掷物必须先瞄准再释放：取出持握并进入瞄准姿态。
            // 已持握时本次使用作废，道具留在背包（items.remove 不会执行）。
            if held.item.is_some() { return None; }
            held.item = Some(item.clone());
        }
        // 武器通过背包"悬停+R 装备"使用，不走道具消耗
        PickupType::Weapon { .. } => return None,
    }
    items.remove(index);
    Some(item.name)
}

/// 手雷投掷的输入上下文：资源态（资产/输入/焦点）+ 只读相机查询，
/// 可变查询（玩家背包、提示显隐）在系统参数中直连。
#[derive(SystemParam)]
pub(crate) struct GrenadeThrowInput<'w, 's> {
    effects: ResMut<'w, EffectAssets>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
    keyboard: Res<'w, ButtonInput<KeyCode>>,
    input_state: Res<'w, InputState>,
    held: ResMut<'w, HeldGrenade>,
    cam_query: Query<'w, 's, &'static GlobalTransform, With<PlayerCamera>>,
}

/// 手雷投掷：持握状态下左键沿相机视线（含俯仰）投出，Esc 取消放回背包。
/// 投掷方向取自相机真实朝向，与越肩准星对齐；持握期间 aim_system 强制瞄准。
pub(crate) fn grenade_throw_system(
    mut commands: Commands,
    mut r: GrenadeThrowInput,
    mut player_query: Query<(&Transform, &mut Inventory), With<Player>>,
    mut hint_vis: Query<&mut Visibility, With<HeldHintRoot>>,
) {
    // 持握提示：只在持握时显示（文案固定，创建时已写好）
    if let Ok(mut vis) = hint_vis.get_single_mut() {
        *vis = if r.held.item.is_some() { Visibility::Visible } else { Visibility::Hidden };
    }
    if r.held.item.is_none() { return; }
    // UI 打开（光标解锁）时不投掷也不取消：左键属于界面
    if !r.input_state.cursor_locked { return; }
    let Ok((player_transform, mut inventory)) = player_query.get_single_mut() else { return };

    // Esc 取消：手雷放回背包（cursor_grab_toggle 在持握期间跳过 Esc，由这里接管）
    if r.keyboard.just_pressed(KeyCode::Escape) {
        if let Some(item) = r.held.item.take() {
            inventory.items.push(item);
        }
        return;
    }

    // 左键释放投掷：方向 = 相机视线（含俯仰），与准星一致
    if r.mouse.just_pressed(MouseButton::Left) {
        let Some(item) = r.held.item.take() else { return };
        let PickupType::Grenade { element } = item.item_type else {
            // 理论不可达（只有手雷会进入持握）：异常物品放回背包
            inventory.items.push(item);
            return;
        };
        let Ok(cam_tf) = r.cam_query.get_single() else { return };
        let (_, rotation, _) = cam_tf.to_scale_rotation_translation();
        let dir = (rotation * Vec3::NEG_Z).normalize();
        let origin = player_transform.translation + Vec3::Y * 1.7 + dir * 0.4;
        commands.spawn((
            PbrBundle {
                mesh: r.effects.projectile.clone(),
                material: r.effects.material(&mut r.materials, element, EffectMatKind::Plain),
                transform: Transform::from_translation(origin),
                ..default()
            },
            GrenadeProjectile {
                velocity: dir * 13.0 + Vec3::Y * 3.0,
                element,
                damage: ITEM_GRENADE_DAMAGE,
                radius: ITEM_GRENADE_RADIUS,
                effect: crate::operator::SkillEffect::NONE,
                timer: Timer::from_seconds(1.5, TimerMode::Once),
            },
        ));
    }
}