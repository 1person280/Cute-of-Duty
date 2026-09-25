//! HUD 居中白色准星（旧版画龙点睛元素）。
//!
//! 设计动机：纯表现层静态 UI，用中心锚定 + 四线像素偏移复刻旧版准星（上/下/左/右四线
//! 留中心缺口、中心一带点）。不读任何快照，仅在 `InGame` 态由 `hud.rs` 装配，
//! `StateScoped` 随离场销毁。

use bevy::prelude::*;

/// 线长（单臂）。
const LINE_LEN: f32 = 10.0;
/// 线厚。
const LINE_W: f32 = 2.0;
/// 中心缺口（线离中心距离）。
const GAP: f32 = 3.0;
/// 中心点直径。
const DOT: f32 = 2.0;

/// 准星根标记。
#[derive(Component)]
pub struct CrosshairRoot;

/// 装配白色十字准星（全屏居中零尺寸锚点 + 绝对定位四线/点）。
pub fn spawn_crosshair(p: &mut ChildBuilder<'_>) {
    p.spawn((
        CrosshairRoot,
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                width: Val::Px(0.0),
                height: Val::Px(0.0),
                ..default()
            },
            ..default()
        },
    ))
    .with_children(|c| {
        // 上壁
        c.spawn(bar(Val::Px(-LINE_W / 2.0), Val::Px(-(GAP + LINE_LEN)), LINE_W, LINE_LEN));
        // 下壁
        c.spawn(bar(Val::Px(-LINE_W / 2.0), Val::Px(GAP), LINE_W, LINE_LEN));
        // 左臂
        c.spawn(bar(Val::Px(-(GAP + LINE_LEN)), Val::Px(-LINE_W / 2.0), LINE_LEN, LINE_W));
        // 右臂
        c.spawn(bar(Val::Px(GAP), Val::Px(-LINE_W / 2.0), LINE_LEN, LINE_W));
        // 中心点
        c.spawn(bar(Val::Px(-DOT / 2.0), Val::Px(-DOT / 2.0), DOT, DOT));
    });
}

/// 一条白色直壁（绝对定位，left/top 为相对中心锚点的像素偏移）。
fn bar(left: Val, top: Val, w: f32, h: f32) -> NodeBundle {
    NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            left,
            top,
            width: Val::Px(w),
            height: Val::Px(h),
            ..default()
        },
        background_color: Color::WHITE.into(),
        ..default()
    }
}