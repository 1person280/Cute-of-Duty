//! 命中反馈：伤害跳字 / 反应文字 / 伤害粒子 / 命中闪光 / 弹道清理

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::model::PlayerCamera;
use crate::demo::components::*;

pub(crate) fn spawn_reaction_text(commands: &mut Commands, pos: Vec3, text: String, color: Color) {
    commands.spawn((
        Text2d::new(text),
        TextFont { font_size: FontSize::Px(28.0), ..default() },
        TextColor(color),
        Transform::from_translation(pos).looking_at(pos + Vec3::X, Vec3::Y),
        FloatingReaction { timer: Timer::from_seconds(1.0, TimerMode::Once) },
    ));
}

pub(crate) fn spawn_damage_popup(commands: &mut Commands, pos: Vec3, text: String, color: Color) {
    commands.spawn((
        Text::new(text),
        TextFont { font_size: FontSize::Px(28.0), ..default() },
        TextColor(Color::srgb(1.0, 1.0, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            padding: UiRect::all(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(color.to_linear().with_alpha(0.85).into()),
        GlobalZIndex(100),
        DamagePopup {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
            world_pos: pos,
        },
    ));
}

pub(crate) fn damage_popup_system(
    mut commands: Commands,
    mut popup_query: Query<(Entity, &mut Node, &mut Text, &mut BackgroundColor, &mut TextColor, &mut DamagePopup)>,
    camera_query: Query<(&Camera, &GlobalTransform), With<PlayerCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
) {
    let camera_opt = camera_query.single().ok();
    let window_height = windows.single().map(|w| w.height()).unwrap_or(1080.0);

    for (entity, mut node, _, mut bg, mut text_color, mut popup) in popup_query.iter_mut() {
        popup.timer.tick(time.delta());
        let t = popup.timer.elapsed_secs() / popup.timer.duration().as_secs_f32();

        // 世界空间上移（与相机无关）
        popup.world_pos.y += 2.5 * time.delta_secs();

        // 屏幕坐标换算才依赖相机：view 缺失（多相机/相机暂不可用）只是本帧不刷新位置，
        // 绝不因此跳过下方的 tick→finished→despawn——否则弹字会永久存活、缓慢堆积。
        if let Some((camera, camera_transform)) = &camera_opt {
            if let Ok(viewport_pos) = camera.world_to_viewport(camera_transform, popup.world_pos) {
                node.left = Val::Px(viewport_pos.x);
                node.top = Val::Px(window_height - viewport_pos.y);
            }
        }

        // Fade out near end
        if t > 0.6 {
            let alpha = (1.0 - (t - 0.6) / 0.4).clamp(0.0, 1.0);
            text_color.0 = text_color.0.with_alpha(alpha);
            bg.0 = bg.0.with_alpha(alpha * 0.85);
        }

        if popup.timer.is_finished() {
            commands.entity(entity).despawn();
            continue;
        }
    }
}

pub(crate) fn damage_particle_system(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut DamageParticle)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut part) in query.iter_mut() {
        part.timer.tick(time.delta());
        transform.translation += part.velocity * time.delta_secs();
        part.velocity.y -= 8.0 * time.delta_secs();
        if part.timer.is_finished() { commands.entity(entity).despawn(); }
    }
}

pub(crate) fn floating_reaction_text(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut FloatingReaction)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut text) in query.iter_mut() {
        text.timer.tick(time.delta());
        transform.translation.y += 1.5 * time.delta_secs();
        if text.timer.is_finished() { commands.entity(entity).despawn(); }
    }
}

pub(crate) fn bullet_cleanup(mut commands: Commands, mut query: Query<(Entity, &mut BulletHit)>, time: Res<Time>) {
    for (entity, mut hit) in query.iter_mut() {
        hit.timer.tick(time.delta());
        if hit.timer.is_finished() { commands.entity(entity).despawn(); }
    }
}

pub(crate) fn hit_flash_system(
    mut query: Query<&mut TargetDummy>,
    time: Res<Time>,
) {
    for mut dummy in query.iter_mut() {
        if let Some(ref mut timer) = dummy.hit_flash {
            timer.tick(time.delta());
            if timer.is_finished() { dummy.hit_flash = None; }
        }
        if let Some(ref mut timer) = dummy.state_timer {
            timer.tick(time.delta());
            if timer.is_finished() { dummy.element_state = None; dummy.state_timer = None; }
        }
    }

}