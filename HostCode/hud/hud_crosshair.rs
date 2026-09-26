//! HUD 居中白色准星（旧版画龙点睛元素）。
//!
//! 设计动机：纯表现层静态 UI，用中心锚定 + 四线像素偏移复刻旧版准星（上/下/左/右四线
//! 留中心缺口、中心一带点）。不读任何快照，仅在 `InGame` 态由 `hud.rs` 装配，
//! `StateScoped` 随离场销毁。
//!
//! 唯一的状态读取是 [`AimRig::aiming`]：越肩瞄准时准星转为**琥珀**（旧版 ADS 提示），
//! 常态为白——用颜色让玩家确认"现在处于瞄准档"。

use bevy::prelude::*;

use crate::flow::flow_state::AimRig;
use crate::shared::theme;

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

/// 准星单条画壁（供 [`update_crosshair`] 整组换色）。
#[derive(Component)]
pub struct CrosshairBar;

/// 越肩瞄准态准星换色：常态白 / 瞄准琥珀（值未变则不写，避免无谓的变更检测）。
pub fn update_crosshair(rig: Res<AimRig>, mut bars: Query<&mut BackgroundColor, With<CrosshairBar>>) {
    let want = if rig.aiming { theme::ACCENT_AMBER } else { Color::WHITE };
    for mut bg in &mut bars {
        if bg.0 != want {
            bg.0 = want;
        }
    }
}

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
        // 四臂 + 中心点：每条都带 `CrosshairBar`，供瞄准态整组换成琥珀。
        // 上壁
        c.spawn(bar(Val::Px(-LINE_W / 2.0), Val::Px(-(GAP + LINE_LEN)), LINE_W, LINE_LEN))
            .insert(CrosshairBar);
        // 下壁
        c.spawn(bar(Val::Px(-LINE_W / 2.0), Val::Px(GAP), LINE_W, LINE_LEN))
            .insert(CrosshairBar);
        // 左臂
        c.spawn(bar(Val::Px(-(GAP + LINE_LEN)), Val::Px(-LINE_W / 2.0), LINE_LEN, LINE_W))
            .insert(CrosshairBar);
        // 右臂
        c.spawn(bar(Val::Px(GAP), Val::Px(-LINE_W / 2.0), LINE_LEN, LINE_W))
            .insert(CrosshairBar);
        // 中心点
        c.spawn(bar(Val::Px(-DOT / 2.0), Val::Px(-DOT / 2.0), DOT, DOT))
            .insert(CrosshairBar);
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