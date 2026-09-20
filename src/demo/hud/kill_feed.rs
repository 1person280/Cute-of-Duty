//! 击杀播报：右上角累计击杀数与渐隐的击杀通知

use bevy::prelude::*;
use crate::demo::components::*;

/// 击杀计数 + 每帧播放新击杀通知，结束后自动淡出并销毁
pub(crate) fn kill_feed_system(
    mut commands: Commands,
    mut kill_events: EventReader<KillEvent>,
    mut stats: ResMut<KillStats>,
    feed_root: Query<Entity, With<KillFeedRoot>>,
    mut total_text: Query<&mut Text, (With<KillFeedTotal>, Without<KillFeedEntry>)>,
    mut entries: Query<(Entity, &mut KillFeedEntry, &mut Text), Without<KillFeedTotal>>,
    time: Res<Time>,
) {
    let mut new_kill = false;
    for ev in kill_events.read() {
        new_kill = true;
        stats.total += 1;
        let Ok(root) = feed_root.get_single() else { continue };
        let base = Color::srgb(1.0, 0.87, 0.45);
        commands.entity(root).with_children(|feed| {
            feed.spawn((
                TextBundle {
                    text: Text::from_section(
                        format!("击杀 · {}", ev.name),
                        TextStyle { font_size: 15.0, color: base, ..default() }
                    ),
                    ..default()
                },
                KillFeedEntry { timer: Timer::from_seconds(4.0, TimerMode::Once), base_color: base },
            ));
        });
    }
    if new_kill {
        if let Ok(mut text) = total_text.get_single_mut() {
            text.sections[0].value = format!("击杀 {}", stats.total);
        }
    }
    for (entity, mut entry, mut text) in entries.iter_mut() {
        entry.timer.tick(time.delta());
        let alpha = entry.timer.remaining_secs().clamp(0.0, 1.0);
        text.sections[0].style.color = entry.base_color.with_alpha(alpha);
        if entry.timer.finished() {
            commands.entity(entity).despawn();
        }
    }
}