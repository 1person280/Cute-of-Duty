//! 击杀播报：右上角累计击杀数与渐隐的击杀通知

use bevy::prelude::*;
use crate::demo::components::*;

/// 击杀计数 + 每帧播放新击杀通知，结束后自动淡出并销毁
pub(crate) fn kill_feed_system(
    mut commands: Commands,
    mut kill_events: MessageReader<KillEvent>,
    mut stats: ResMut<KillStats>,
    feed_root: Query<Entity, With<KillFeedRoot>>,
    mut total_text: Query<&mut Text, (With<KillFeedTotal>, Without<KillFeedEntry>)>,
    mut entries: Query<(Entity, &mut KillFeedEntry, &mut Text, &mut TextColor), Without<KillFeedTotal>>,
    time: Res<Time>,
) {
    let mut new_kill = false;
    for ev in kill_events.read() {
        new_kill = true;
        stats.total += 1;
        let Ok(root) = feed_root.single() else { continue };
        let base = Color::srgb(1.0, 0.87, 0.45);
        commands.entity(root).with_children(|feed| {
            feed.spawn((
                Text::new(format!("击杀 · {}", ev.name)),
                TextFont { font_size: FontSize::Px(15.0), ..default() },
                TextColor(base),
                KillFeedEntry { timer: Timer::from_seconds(4.0, TimerMode::Once), base_color: base },
            ));
        });
    }
    if new_kill {
        if let Ok(mut text) = total_text.single_mut() {
            text.0 = format!("击杀 {}", stats.total);
        }
    }
    for (entity, mut entry, _text, mut color) in entries.iter_mut() {
        entry.timer.tick(time.delta());
        let alpha = entry.timer.remaining_secs().clamp(0.0, 1.0);
        color.0 = entry.base_color.with_alpha(alpha);
        if entry.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}