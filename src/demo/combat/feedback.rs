//! 命中反馈：伤害跳字 / 反应文字 / 伤害粒子 / 命中闪光 / 弹道清理

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use crate::model::PlayerCamera;
use crate::demo::components::*;

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
    let camera_opt = camera_query.get_single().ok();
    let window_height = windows.get_single().map(|w| w.height()).unwrap_or(1080.0);

    for (entity, mut style, mut text, mut bg, mut popup) in popup_query.iter_mut() {
        popup.timer.tick(time.delta());
        let t = popup.timer.elapsed_secs() / popup.timer.duration().as_secs_f32();

        // 世界空间上移（与相机无关）
        popup.world_pos.y += 2.5 * time.delta_seconds();

        // 屏幕坐标换算才依赖相机：view 缺失（多相机/相机暂不可用）只是本帧不刷新位置，
        // 绝不因此跳过下方的 tick→finished→despawn——否则弹字会永久存活、缓慢堆积。
        if let Some((camera, camera_transform)) = &camera_opt {
            if let Some(viewport_pos) = camera.world_to_viewport(camera_transform, popup.world_pos) {
                style.left = Val::Px(viewport_pos.x);
                style.top = Val::Px(window_height - viewport_pos.y);
            }
        }

        // Fade out near end
        if t > 0.6 {
            let alpha = (1.0 - (t - 0.6) / 0.4).clamp(0.0, 1.0);
            text.sections[0].style.color = text.sections[0].style.color.with_alpha(alpha);
            bg.0 = bg.0.with_alpha(alpha * 0.85);
        }

        if popup.timer.finished() {
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

pub(crate) fn bullet_cleanup(mut commands: Commands, mut query: Query<(Entity, &mut BulletHit)>, time: Res<Time>) {
    for (entity, mut hit) in query.iter_mut() {
        hit.timer.tick(time.delta());
        if hit.timer.finished() { commands.entity(entity).despawn(); }
    }
}

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