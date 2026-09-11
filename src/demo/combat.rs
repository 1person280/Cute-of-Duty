//! 战斗：射击、换弹、元素反应、技能与手雷、爆炸、伤害数字

use bevy::prelude::*;
use bevy::pbr::NotShadowCaster;
use bevy::window::PrimaryWindow;
use crate::element::{ElementSystem, ElementType, EntityElementState, ReactionResult};
use crate::operator::{roster, SkillKind};
use crate::model::{
    Player, OperatorState, PlayerCamera, PlayerMovement,
};
use super::inventory::HeldGrenade;
use super::common::*;
use super::camera::{lerp, ray_sphere_hit, WEAKPOINT_CORE_R, WEAKPOINT_MULT};
use super::controller::collides;
use super::hud::{EffectAssets, EffectMatKind};
use super::components::*;

pub(crate) fn reload_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<(&mut PlayerMovement, &mut WeaponSlot, &mut Inventory), With<Player>>,
    time: Res<Time>,
    input_state: Res<InputState>,
) {
    let Ok((mut movement, weapon_slot, mut inventory)) = player_query.get_single_mut() else { return };

    let current_idx = weapon_slot.current;

    // Tick reload timer if active
    if let Some(ref mut timer) = movement.reload_timer {
        timer.tick(time.delta());
        if timer.finished() {
            // 换弹从背包弹药池取弹
            let widx = current_idx;
            let needed = inventory.weapons[widx].max_ammo - inventory.weapons[widx].ammo;
            if needed > 0 {
                let take = needed.min(inventory.ammo_pool);
                inventory.weapons[widx].ammo += take;
                inventory.ammo_pool -= take;
            }
            movement.reload_timer = None;
        }
    }

    // UI 打开（光标解锁）时 R 留给背包道具使用，不触发换弹
    if !input_state.cursor_locked { return; }

    // Start reload on R key press
    if keyboard.just_pressed(KeyCode::KeyR) && movement.reload_timer.is_none() {
        if inventory.weapons[weapon_slot.current].ammo < inventory.weapons[weapon_slot.current].max_ammo
            && inventory.ammo_pool > 0
        {
            let reload_time = 1.8; // seconds
            movement.reload_timer = Some(Timer::from_seconds(reload_time, TimerMode::Once));
        }
    }
}

pub(crate) fn shooting_system(
    mut commands: Commands,
    mut effects: ResMut<EffectAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mouse: Res<ButtonInput<MouseButton>>,
    element_system: Res<ElementalSystem>,
    cam_query: Query<&PlayerCamera>,
    cam_gtransform: Query<&GlobalTransform, With<PlayerCamera>>,
    mut player_query: Query<(&Transform, &mut PlayerMovement, &mut WeaponSlot, &mut Inventory), With<Player>>,
    mut target_query: Query<(Entity, &mut TargetDummy, &Transform, &Children)>,
    time: Res<Time>,
    input_state: Res<InputState>,
    held: Res<HeldGrenade>,
) {
    let Ok((player_transform, mut movement, weapon_slot, mut inventory)) = player_query.get_single_mut() else { return };
    movement.shoot_cooldown.tick(time.delta());

    // 背包/轮盘/站点等 UI 打开（光标解锁）时不射击
    if !input_state.cursor_locked { return; }

    // 手雷持握（瞄准投掷姿态）时左键交给投掷，步枪不射击
    if held.item.is_some() { return; }

    // Cannot shoot while reloading
    if movement.reload_timer.is_some() { return; }

    let shooting = mouse.pressed(MouseButton::Left);
    if !shooting || !movement.shoot_cooldown.finished() { return; }

    let idx = weapon_slot.current;
    let empty = inventory.weapons[idx].ammo <= 0;
    if empty {
        // Auto-reload when empty（备弹来自背包弹药池）
        if inventory.ammo_pool > 0 && movement.reload_timer.is_none() {
            let reload_time = 1.8;
            movement.reload_timer = Some(Timer::from_seconds(reload_time, TimerMode::Once));
        }
        return;
    }
    inventory.weapons[idx].ammo -= 1;
    let element = inventory.weapons[idx].element;
    let damage = inventory.weapons[idx].damage;
    let fire_interval = inventory.weapons[idx].fire_interval;
    movement.shoot_cooldown = Timer::from_seconds(fire_interval, TimerMode::Once);

    let Ok(cam) = cam_query.get_single() else { return };

    // 命中判定以准星为准：射线从相机出发、沿视线方向（与屏幕中心一致）。
    // 角色与相机之间的物体不参与检测（SpringArm 已做相机避障，射线只查靶子）。
    let cam_gtf = cam_gtransform.get_single().copied()
        .map(|tf| tf.compute_transform())
        .unwrap_or_default();
    let origin = cam_gtf.translation;
    let dir = cam_gtf.forward().normalize();

    // Raycast：躯干大球（宽松判定）+ 靶心核心小球（弱点）
    let mut closest: Option<(Entity, f32, Vec3, bool)> = None;
    for (entity, dummy, transform, _children) in target_query.iter() {
        if dummy.down_timer.is_some() { continue; } // 已倒地的靶子不再受击
        let base = transform.translation;
        let mut hit_t = ray_sphere_hit(origin, dir, base + Vec3::Y * 1.5, 1.5);
        let mut weak = false;
        // 弱点：靶板中心红心（人形敌人则对应头部区域）
        if let Some(t_core) = ray_sphere_hit(origin, dir, base, WEAKPOINT_CORE_R) {
            if hit_t.map_or(true, |t| t_core < t) {
                hit_t = Some(t_core);
                weak = true;
            }
        }
        if let Some(t) = hit_t {
            if t <= 60.0 && closest.map_or(true, |(_, d, _, _)| t < d) {
                closest = Some((entity, t, origin + dir * t, weak));
            }
        }
    }

    let end = if let Some((_, d, _, _)) = closest { origin + dir * d } else { origin + dir * 60.0 };
    // 曳光从枪口出发、收敛到命中点（TPS 标准做法：判定跟准星，视觉跟枪口）
    let aim_t = cam.aim_lerp * cam.aim_lerp * (3.0 - 2.0 * cam.aim_lerp);
    let muzzle = player_transform.translation
        + Quat::from_rotation_y(movement.model_yaw)
            * Vec3::new(0.4, lerp(1.3, 2.35, aim_t) + 0.08, lerp(1.15, 1.05, aim_t));
    let mid = (muzzle + end) / 2.0;
    let len = (end - muzzle).length();

    // Laser trail（共享网格按弹道长度缩放 Z）
    let color = element.color();
    commands.spawn((
        PbrBundle {
            mesh: effects.tracer.clone(),
            material: effects.material(&mut materials, element, EffectMatKind::Plain),
            transform: Transform::from_translation(mid)
                .looking_at(end, Vec3::Y)
                .with_scale(Vec3::new(1.0, 1.0, len)),
            ..default()
        },
        BulletHit { timer: Timer::from_seconds(0.06, TimerMode::Once) },
    ));

    // Muzzle flash
    commands.spawn((
        PbrBundle {
            mesh: effects.spark.clone(),
            material: effects.muzzle_material.clone(),
            transform: Transform::from_translation(muzzle),
            ..default()
        },
        BulletHit { timer: Timer::from_seconds(0.04, TimerMode::Once) },
    ));

    // Hit processing
    if let Some((entity, _, hit_point, weak)) = closest {
        if let Ok((_, mut dummy, _, _)) = target_query.get_mut(entity) {
            // 元素反应：查询核心配置表（与cod1共用同一套规则）
            // 弱点打击：命中靶心核心，基础伤害 ×1.8
            let (dmg, reaction_name) = element_reaction(
                &element_system.0,
                dummy.element_state,
                element,
                damage * if weak { WEAKPOINT_MULT } else { 1.0 },
            );

            dummy.current_health -= dmg;
            dummy.hit_flash = Some(Timer::from_seconds(0.2, TimerMode::Once));
            // 若这一击致命，靶子朝来弹方向倒下（取弹道水平分量）
            dummy.fall_dir = Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();
            if element != ElementType::Physical {
                dummy.element_state = Some(element);
                dummy.state_timer = Some(Timer::from_seconds(3.5, TimerMode::Once));
            }

            // Damage popup (element damage / actual damage / weak point)
            let popup_text = if weak {
                if let Some(ref reaction) = reaction_name {
                    format!("弱点 {} ({})", dmg as i32, reaction)
                } else {
                    format!("弱点 {}", dmg as i32)
                }
            } else if let Some(ref reaction) = reaction_name {
                format!("{} ({})", dmg as i32, reaction)
            } else {
                format!("{}", dmg as i32)
            };
            let popup_color = if weak {
                Color::srgb(1.0, 0.45, 0.1) // hot orange for weak points
            } else if reaction_name.is_some() {
                Color::srgb(1.0, 0.85, 0.2) // gold for reactions
            } else {
                color
            };
            spawn_damage_popup(&mut commands, hit_point + Vec3::Y * 0.5, popup_text, popup_color);

            // Damage particles
            for _ in 0..5 {
                let dir = Vec3::new(
                    (rand::random::<f32>() - 0.5) * 2.0,
                    rand::random::<f32>(),
                    (rand::random::<f32>() - 0.5) * 2.0,
                ).normalize();
                commands.spawn((
                    PbrBundle {
                        mesh: effects.particle.clone(),
                        material: effects.material(&mut materials, element, EffectMatKind::Particle),
                        transform: Transform::from_translation(hit_point),
                        ..default()
                    },
                    DamageParticle {
                        velocity: dir * 3.0,
                        timer: Timer::from_seconds(0.5, TimerMode::Once),
                    },
                ));
            }

            // Hit explosion
            commands.spawn((
                PbrBundle {
                    mesh: effects.spark.clone(),
                    material: effects.material(&mut materials, element, EffectMatKind::HitFlash),
                    transform: Transform::from_translation(hit_point).with_scale(Vec3::splat(2.5)),
                    ..default()
                },
                BulletHit { timer: Timer::from_seconds(0.15, TimerMode::Once) },
            ));

            // Reaction text
            if let Some(name) = reaction_name {
                spawn_reaction_text(&mut commands, hit_point + Vec3::Y * 0.8, name, color);
            }
        }
    }
}

/// 将"已附着的元素"映射为核心系统的实体元素状态
pub(crate) fn applied_element_state(element: ElementType) -> Option<EntityElementState> {
    match element {
        ElementType::Fire => Some(EntityElementState::Burning),
        ElementType::Ice => Some(EntityElementState::Frozen),
        ElementType::Electric => Some(EntityElementState::Electrified),
        ElementType::Poison => Some(EntityElementState::Poisoned),
        ElementType::Water => Some(EntityElementState::Wet),
        ElementType::Physical => None,
    }
}

/// 反应结果的中文名（弹字显示）
pub(crate) fn reaction_result_label(result: &ReactionResult) -> &'static str {
    match result {
        ReactionResult::Vaporize => "蒸发！",
        ReactionResult::Melt => "融化！",
        ReactionResult::Burning => "燃烧！",
        ReactionResult::Electrolysis => "电解！",
        ReactionResult::Superconduct => "超导！",
        ReactionResult::PoisonExplosion => "毒爆！",
        ReactionResult::PoisonCloud => "毒云！",
        ReactionResult::ShatterFreeze => "爆裂冻结！",
        ReactionResult::PhysicalVulnerability => "物理易伤！",
        ReactionResult::RainSuppressed => "雨天压制",
        ReactionResult::RainAmplified => "雨天增强",
        ReactionResult::Overheat => "过热",
        ReactionResult::OverheatRisk => "过热风险",
        ReactionResult::SnowAmplified => "雪地增强",
        ReactionResult::SnowSuppressed => "雪地压制",
        ReactionResult::ConductiveRisk => "导电风险",
    }
}

/// 元素反应结算：查询核心配置表（config/element_reactions.yaml）
///
/// 返回 (最终伤害, 反应名称)。伤害 = 基础伤害 × 反应倍率，
/// 与cod1的伤害结算走同一份YAML规则，新增反应无需改代码。
pub(crate) fn element_reaction(
    system: &ElementSystem,
    existing: Option<ElementType>,
    incoming: ElementType,
    base_damage: f32,
) -> (f32, Option<String>) {
    let Some(existing) = existing else { return (base_damage, None); };
    let Some(state) = applied_element_state(existing) else { return (base_damage, None); };
    let Some(reaction) = system.query_reaction(&state, &incoming) else {
        return (base_damage, None);
    };

    let damage = base_damage * reaction.damage_multiplier;
    (damage, Some(reaction_result_label(&reaction.result).to_string()))
}

pub(crate) fn spawn_reaction_text(commands: &mut Commands, pos: Vec3, text: String, color: Color) {
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(text, TextStyle { font_size: 28.0, color, ..default() }),
            transform: Transform::from_translation(pos).looking_at(pos + Vec3::X, Vec3::Y),
            ..default()
        },
        FloatingReaction { timer: Timer::from_seconds(1.0, TimerMode::Once) },
    ));
}

pub(crate) fn spawn_damage_popup(commands: &mut Commands, pos: Vec3, text: String, color: Color) {
    commands.spawn((
        TextBundle {
            text: Text::from_section(
                text,
                TextStyle {
                    font_size: 28.0,
                    color: Color::srgb(1.0, 1.0, 1.0),
                    ..default()
                },
            ),
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            background_color: BackgroundColor(color.to_linear().with_alpha(0.85).into()),
            z_index: ZIndex::Global(100),
            ..default()
        },
        DamagePopup {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
            world_pos: pos,
        },
    ));
}

pub(crate) fn damage_popup_system(
    mut commands: Commands,
    mut popup_query: Query<(Entity, &mut Style, &mut Text, &mut BackgroundColor, &mut DamagePopup)>,
    camera_query: Query<(&Camera, &GlobalTransform), With<PlayerCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
) {
    let Ok((camera, camera_transform)) = camera_query.get_single() else { return };
    let window_height = windows.get_single().map(|w| w.height()).unwrap_or(1080.0);

    for (entity, mut style, mut text, mut bg, mut popup) in popup_query.iter_mut() {
        popup.timer.tick(time.delta());
        let t = popup.timer.elapsed_secs() / popup.timer.duration().as_secs_f32();

        // Move upward in world space
        popup.world_pos.y += 2.5 * time.delta_seconds();

        // Convert world position to screen position
        if let Some(viewport_pos) = camera.world_to_viewport(camera_transform, popup.world_pos) {
            style.left = Val::Px(viewport_pos.x);
            style.top = Val::Px(window_height - viewport_pos.y);
        }

        // Fade out near end
        if t > 0.6 {
            let alpha = (1.0 - (t - 0.6) / 0.4).clamp(0.0, 1.0);
            text.sections[0].style.color = text.sections[0].style.color.with_alpha(alpha);
            bg.0 = bg.0.with_alpha(alpha * 0.85);
        }

        if popup.timer.finished() {
            commands.entity(entity).despawn();
        }
    }
}

// =============================================================================
// Operator Skills (Q/E)
// =============================================================================

/// 背包战术手雷的伤害与半径（干员 Q 手雷的数值由核心库干员档案提供）
pub(crate) const ITEM_GRENADE_DAMAGE: f32 = 40.0;
pub(crate) const ITEM_GRENADE_RADIUS: f32 = 3.5;

/// 持续区域（毒雾）：周期性对圈内目标结算机制伤害
#[derive(Component)]
pub(crate) struct SkillZone {
    pub(crate) element: ElementType,
    pub(crate) dps: f32,
    pub(crate) radius: f32,
    pub(crate) tick: Timer,
    pub(crate) lifetime: Timer,
}

/// 对爆炸点周围的目标结算范围伤害 + 干员机制（点燃/冰冻）；
/// 毒雾区域由调用方在作用点另行生成（spawn_skill_zone）。
/// 线性距离衰减（边缘保底30%）+ 元素反应，规则查核心配置表。
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_explosion_damage(
    commands: &mut Commands,
    effects: &EffectAssets,
    element_system: &ElementSystem,
    targets: &mut Query<(Entity, &mut TargetDummy, &Transform, &Children), (Without<GrenadeProjectile>, Without<Player>)>,
    pos: Vec3,
    element: ElementType,
    base_damage: f32,
    radius: f32,
    effect: crate::operator::SkillEffect,
) {
    for (entity, mut dummy, transform, _) in targets.iter_mut() {
        if dummy.down_timer.is_some() { continue; } // 已倒地的靶子不再受击
        let dist = transform.translation.distance(pos);
        if dist > radius { continue; }
        let falloff = (1.0 - dist / radius).max(0.3);
        let (dmg, reaction_name) = element_reaction(
            element_system,
            dummy.element_state,
            element,
            base_damage * falloff,
        );

        dummy.current_health -= dmg;
        dummy.hit_flash = Some(Timer::from_seconds(0.2, TimerMode::Once));
        // 若这一击致命，靶子被冲击波掀翻：沿爆炸中心指向靶子的方向倒下
        let blast_dir = (transform.translation - pos).with_y(0.0);
        if blast_dir.length_squared() > 1e-6 {
            dummy.fall_dir = blast_dir.normalize();
        }
        if element != ElementType::Physical {
            dummy.element_state = Some(element);
            dummy.state_timer = Some(Timer::from_seconds(3.5, TimerMode::Once));
        }

        // 机制：点燃（DoT 挂到目标身上）
        if effect.burn_secs > 0.0 && effect.burn_dps > 0.0 {
            dummy.dots.push(DamageOverTime {
                element,
                dps: effect.burn_dps,
                tick: Timer::from_seconds(DOT_TICK, TimerMode::Repeating),
                remaining: Timer::from_seconds(effect.burn_secs, TimerMode::Once),
            });
        }
        // 机制：冰冻/电麻（目标停止行动 + 冰块视觉；已冻结则只刷新时长）
        if effect.freeze_secs > 0.0 {
            if dummy.frozen.is_none() {
                dummy.frozen = Some(Timer::from_seconds(effect.freeze_secs, TimerMode::Once));
                let ice = commands.spawn((
                    PbrBundle {
                        mesh: effects.frost_cube.clone(),
                        material: effects.frost_material.clone(),
                        transform: Transform::from_translation(Vec3::new(0.0, 0.4, 0.0)),
                        ..default()
                    },
                    NotShadowCaster,
                )).id();
                commands.entity(entity).add_child(ice);
                dummy.frozen_visual = Some(ice);
            } else if let Some(t) = dummy.frozen.as_mut() {
                t.reset();
            }
        }

        let popup_text = if let Some(ref reaction) = reaction_name {
            format!("{} ({})", dmg as i32, reaction)
        } else {
            format!("{}", dmg as i32)
        };
        let popup_color = if reaction_name.is_some() {
            Color::srgb(1.0, 0.85, 0.2)
        } else {
            element.color()
        };
        spawn_damage_popup(commands, transform.translation + Vec3::Y * 0.8, popup_text, popup_color);
    }
}

/// 在作用点展开持续毒雾区（毒蛛机制）：圈内目标每 DOT_TICK 秒掉血
pub(crate) fn spawn_skill_zone(
    commands: &mut Commands,
    effects: &mut EffectAssets,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
    effect: crate::operator::SkillEffect,
) {
    if effect.zone_secs <= 0.0 || effect.zone_dps <= 0.0 { return; }
    // 元素取毒系固定（当前仅毒蛛配置毒区；半径沿用档案缺省 3.5）
    let element = ElementType::Poison;
    let radius = 3.5;
    commands.spawn((
        PbrBundle {
            mesh: effects.zone_cylinder.clone(),
            material: effects.material(materials, element, EffectMatKind::Zone),
            transform: Transform::from_translation(pos.with_y(0.0) + Vec3::Y * 0.7)
                .with_scale(Vec3::new(radius, 1.0, radius)),
            ..default()
        },
        NotShadowCaster,
        SkillZone {
            element,
            dps: effect.zone_dps,
            radius,
            tick: Timer::from_seconds(DOT_TICK, TimerMode::Repeating),
            lifetime: Timer::from_seconds(effect.zone_secs, TimerMode::Once),
        },
    ));
}

/// 毒雾区每帧驱动：周期掉血结算 + 到期消散
pub(crate) fn zone_tick_system(
    mut commands: Commands,
    mut zones: Query<(Entity, &mut Transform, &mut SkillZone)>,
    mut targets: Query<(&Transform, &mut TargetDummy), (Without<SkillZone>, Without<GrenadeProjectile>)>,
    element_system: Res<ElementalSystem>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut zone) in zones.iter_mut() {
        zone.lifetime.tick(time.delta());
        zone.tick.tick(time.delta());
        // 消散前塌缩视觉
        let remain = zone.lifetime.remaining_secs();
        if remain < 0.4 {
            let s = (remain / 0.4).max(0.05);
            transform.scale = Vec3::new(transform.scale.x, s, transform.scale.z);
        }
        if zone.tick.just_finished() {
            for (t, mut dummy) in targets.iter_mut() {
                if dummy.down_timer.is_some() { continue; }
                let offset = t.translation - transform.translation;
                let flat_dist = Vec3::new(offset.x, 0.0, offset.z).length();
                if flat_dist > zone.radius { continue; }
                let (dmg, _) = element_reaction(
                    &element_system.0,
                    dummy.element_state,
                    zone.element,
                    zone.dps * DOT_TICK,
                );
                dummy.current_health -= dmg;
                dummy.element_state = Some(zone.element);
                dummy.state_timer = Some(Timer::from_seconds(3.5, TimerMode::Once));
                spawn_damage_popup(
                    &mut commands,
                    t.translation + Vec3::Y * 1.2,
                    format!("{}", dmg as i32),
                    zone.element.color(),
                );
            }
        }
        if zone.lifetime.finished() {
            commands.entity(entity).despawn();
        }
    }
}

pub(crate) fn skill_system(
    mut commands: Commands,
    mut effects: ResMut<EffectAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    element_system: Res<ElementalSystem>,
    mut player_query: Query<(&mut Transform, &mut OperatorState), With<Player>>,
    mut target_query: Query<(Entity, &mut TargetDummy, &Transform, &Children), (Without<GrenadeProjectile>, Without<Player>)>,
    colliders: Query<(&Transform, &Collider), Without<Player>>,
    cam_query: Query<&PlayerCamera>,
    input_state: Res<InputState>,
) {
    // 背包/站点等 UI 打开（光标解锁）时不触发技能
    if !input_state.cursor_locked { return; }
    let Ok((mut player_transform, mut op)) = player_query.get_single_mut() else { return };
    let Ok(cam) = cam_query.get_single() else { return };

    // 技能定义来自核心库干员名册：形态/机制/冷却全部随干员切换
    let op_def = &roster()[op.active];
    let element = op_def.element;
    if keyboard.just_pressed(KeyCode::KeyQ) && op.q.finished() {
        op.q.reset();
        cast_skill(&mut commands, &mut effects, &mut materials, &element_system.0,
            &mut target_query, &colliders, &mut player_transform, cam.yaw, element, op_def.q);
    }
    if keyboard.just_pressed(KeyCode::KeyE) && op.e.finished() {
        op.e.reset();
        cast_skill(&mut commands, &mut effects, &mut materials, &element_system.0,
            &mut target_query, &colliders, &mut player_transform, cam.yaw, element, op_def.e);
    }
}

/// 施放一个技能定义：形态由核心配置（SkillKind）决定，
/// 独特机制（点燃/冰冻/毒区/位移）由 SkillEffect/Kind 驱动，元素取干员亲和元素。
#[allow(clippy::too_many_arguments)]
pub(crate) fn cast_skill(
    commands: &mut Commands,
    effects: &mut EffectAssets,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    element_system: &ElementSystem,
    target_query: &mut Query<(Entity, &mut TargetDummy, &Transform, &Children), (Without<GrenadeProjectile>, Without<Player>)>,
    colliders: &Query<(&Transform, &Collider), Without<Player>>,
    player_transform: &mut Transform,
    yaw: f32,
    element: ElementType,
    skill: crate::operator::SkillDef,
) {
    let fwd = Vec3::new(yaw.sin(), 0.0, yaw.cos());
    match skill.kind {
        SkillKind::Grenade { damage, radius } => {
            commands.spawn((
                PbrBundle {
                    mesh: effects.projectile.clone(),
                    material: effects.material(materials, element, EffectMatKind::Plain),
                    transform: Transform::from_translation(player_transform.translation + Vec3::new(0.0, 2.5, 0.0) + fwd * 0.5),
                    ..default()
                },
                GrenadeProjectile {
                    velocity: fwd * 12.0 + Vec3::Y * 6.0,
                    element,
                    damage,
                    radius,
                    effect: skill.effect,
                    timer: Timer::from_seconds(1.5, TimerMode::Once),
                },
            ));
        }
        SkillKind::Burst { damage, radius } => {
            let burst_pos = player_transform.translation + Vec3::new(0.0, 1.0, 0.0);
            // 范围伤害 + 机制（点燃/冰冻）+ 视觉特效
            apply_explosion_damage(commands, effects, element_system, target_query,
                burst_pos, element, damage, radius, skill.effect);
            spawn_explosion(commands, effects, materials, burst_pos, element, radius + 0.5);
            // 机制：毒雾区在脚下展开
            spawn_skill_zone(commands, effects, materials, player_transform.translation, skill.effect);
        }
        SkillKind::Dash { distance } => {
            // 机制：沿视线水平疾冲，逐段碰撞检测，撞障碍即停
            let start = player_transform.translation;
            let half = Vec3::new(0.4, 1.0, 0.4);
            let step = 0.25;
            let mut moved = 0.0f32;
            while moved < distance {
                let next = (moved + step).min(distance);
                if collides(start + fwd * next, half, colliders) { break; }
                moved = next;
            }
            player_transform.translation = start + fwd * moved;
            // 冲刺残影：起点与终点各撒一把元素火花
            for pos in [start + Vec3::Y, start + fwd * moved + Vec3::Y] {
                for _ in 0..6 {
                    let dir = Vec3::new(
                        (rand::random::<f32>() - 0.5) * 2.0,
                        rand::random::<f32>(),
                        (rand::random::<f32>() - 0.5) * 2.0,
                    ).normalize();
                    commands.spawn((
                        PbrBundle {
                            mesh: effects.particle.clone(),
                            material: effects.material(materials, element, EffectMatKind::Particle),
                            transform: Transform::from_translation(pos),
                            ..default()
                        },
                        DamageParticle {
                            velocity: dir * 2.5,
                            timer: Timer::from_seconds(0.35, TimerMode::Once),
                        },
                    ));
                }
            }
        }
    }
}

pub(crate) fn operator_cooldown_tick(mut query: Query<&mut OperatorState, With<Player>>, time: Res<Time>) {
    let Ok(mut op) = query.get_single_mut() else { return };
    op.q.tick(time.delta());
    op.e.tick(time.delta());
}

pub(crate) fn spawn_explosion(
    commands: &mut Commands,
    effects: &mut EffectAssets,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
    element: ElementType,
    max_scale: f32,
) {
    commands.spawn((
        PbrBundle {
            mesh: effects.explosion_sphere.clone(),
            material: effects.material(materials, element, EffectMatKind::Explosion),
            transform: Transform::from_translation(pos).with_scale(Vec3::splat(0.1)),
            ..default()
        },
        ExplosionEffect { timer: Timer::from_seconds(0.6, TimerMode::Once), max_scale },
    ));
    for _ in 0..10 {
        let dir = Vec3::new(
            (rand::random::<f32>() - 0.5) * 2.0,
            rand::random::<f32>() * 0.8 + 0.2,
            (rand::random::<f32>() - 0.5) * 2.0,
        ).normalize();
        commands.spawn((
            PbrBundle {
                mesh: effects.explosion_debris.clone(),
                material: effects.material(materials, element, EffectMatKind::Plain),
                transform: Transform::from_translation(pos + dir * 0.5),
                ..default()
            },
            BulletHit { timer: Timer::from_seconds(0.5, TimerMode::Once) },
        ));
    }
}

// =============================================================================
// Projectile & Effect Systems
// =============================================================================

pub(crate) fn grenade_physics(
    mut commands: Commands,
    mut effects: ResMut<EffectAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    element_system: Res<ElementalSystem>,
    mut query: Query<(Entity, &mut Transform, &mut GrenadeProjectile), Without<TargetDummy>>,
    mut target_query: Query<(Entity, &mut TargetDummy, &Transform, &Children), (Without<GrenadeProjectile>, Without<Player>)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut grenade) in query.iter_mut() {
        grenade.timer.tick(time.delta());
        grenade.velocity.y -= 12.0 * time.delta_seconds();
        transform.translation += grenade.velocity * time.delta_seconds();

        // 落地或引信耗尽时引爆：范围伤害 + 机制 + 视觉特效（数值由投掷物自带）
        if transform.translation.y <= 0.15 {
            transform.translation.y = 0.15;
            let impact = transform.translation;
            apply_explosion_damage(&mut commands, &effects, &element_system.0, &mut target_query,
                impact, grenade.element, grenade.damage, grenade.radius, grenade.effect);
            spawn_skill_zone(&mut commands, &mut effects, &mut materials, impact, grenade.effect);
            spawn_explosion(&mut commands, &mut effects, &mut materials, impact, grenade.element, grenade.radius + 1.5);
            commands.entity(entity).despawn();
            continue;
        }
        if grenade.timer.finished() {
            let impact = transform.translation;
            apply_explosion_damage(&mut commands, &effects, &element_system.0, &mut target_query,
                impact, grenade.element, grenade.damage, grenade.radius, grenade.effect);
            spawn_skill_zone(&mut commands, &mut effects, &mut materials, impact, grenade.effect);
            spawn_explosion(&mut commands, &mut effects, &mut materials, impact, grenade.element, grenade.radius + 1.5);
            commands.entity(entity).despawn();
        }
    }
}

pub(crate) fn explosion_expand(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut ExplosionEffect)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut effect) in query.iter_mut() {
        effect.timer.tick(time.delta());
        let t = effect.timer.elapsed_secs() / effect.timer.duration().as_secs_f32();
        let scale = effect.max_scale * t.min(1.0);
        transform.scale = Vec3::splat(scale);
        if effect.timer.finished() {
            commands.entity(entity).despawn();
        }
    }
}

pub(crate) fn bullet_cleanup(mut commands: Commands, mut query: Query<(Entity, &mut BulletHit)>, time: Res<Time>) {
    for (entity, mut hit) in query.iter_mut() {
        hit.timer.tick(time.delta());
        if hit.timer.finished() { commands.entity(entity).despawn(); }
    }
}

pub(crate) fn damage_particle_system(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut DamageParticle)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut part) in query.iter_mut() {
        part.timer.tick(time.delta());
        transform.translation += part.velocity * time.delta_seconds();
        part.velocity.y -= 8.0 * time.delta_seconds();
        if part.timer.finished() { commands.entity(entity).despawn(); }
    }
}

pub(crate) fn floating_reaction_text(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut FloatingReaction)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut text) in query.iter_mut() {
        text.timer.tick(time.delta());
        transform.translation.y += 1.5 * time.delta_seconds();
        if text.timer.finished() { commands.entity(entity).despawn(); }
    }
}

// =============================================================================
// Target Logic
// =============================================================================

pub(crate) fn hit_flash_system(
    mut query: Query<&mut TargetDummy>,
    time: Res<Time>,
) {
    for mut dummy in query.iter_mut() {
        if let Some(ref mut timer) = dummy.hit_flash {
            timer.tick(time.delta());
            if timer.finished() { dummy.hit_flash = None; }
        }
        if let Some(ref mut timer) = dummy.state_timer {
            timer.tick(time.delta());
            if timer.finished() { dummy.element_state = None; dummy.state_timer = None; }
        }
    }

}
