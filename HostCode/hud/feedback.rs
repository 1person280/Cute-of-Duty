//! 准星左侧命中反馈飘字（伤害数值 / 元素反应名）
//!
//! 设计动机（Why）：开火命中后玩家需要即时的「打掉多少血 / 触发了什么反应」确认。
//! 数值与反应名全部来自服务端权威结算（`EventKind::Hit` 扩字段，由 `flow::state`
//! 写入 [`crate::flow::HitFeedback`] 资源），本模块只做展示：单条飘字、新命中覆盖
//! 旧条目、0.8s 淡出；有反应时两行（伤害行 + 反应名行）；爆头伤害数字变黄。
//! 纯表现层，不做任何伤害推算。

use bevy::prelude::*;

use crate::flow::{CjkFont, HitFeedback};

/// 飘字淡出总时长（秒）：新命中覆盖旧条目，停火后自此时长内淡出。
const FADE_SECS: f32 = 0.8;
/// 爆头伤害数字颜色（黄）。
const HEADSHOT_COLOR: Color = Color::srgb(0.95, 0.80, 0.15);
/// 普通伤害数字颜色（白）。
const NORMAL_COLOR: Color = Color::srgb(0.95, 0.95, 0.95);
/// 反应名行颜色（青，示意元素反应）。
const REACTION_COLOR: Color = Color::srgb(0.20, 0.90, 0.95);
/// 飘字块右缘距准星中心锚点的水平偏移（px，准星左侧留白）。
const LEFT_OFFSET: f32 = 28.0;

/// 飘字块根标记（准星左侧锚点子节点，供整组显隐）。
#[derive(Component)]
pub(crate) struct FeedbackBlock;

/// 伤害行文本标记（供刷新系统定位）。
#[derive(Component)]
pub(crate) struct DamageLine;

/// 反应名行文本标记（无反应时文本为空串、行高塌缩不可见）。
#[derive(Component)]
pub(crate) struct ReactionLine;

/// 装配准星左侧飘字（全屏居中零尺寸锚点 + 左侧偏移的文本块，默认隐藏）。
pub fn spawn_hit_feedback(p: &mut ChildSpawnerCommands<'_>, fonts: &CjkFont) {
    p.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            width: Val::Px(0.0),
            height: Val::Px(0.0),
            ..default()
        },
    ))
    .with_children(|c| {
        c.spawn((
            FeedbackBlock,
            Node {
                position_type: PositionType::Absolute,
                // 右缘贴在准星左侧：文本随内容向左生长，不遮挡准星。
                right: Val::Px(LEFT_OFFSET),
                top: Val::Px(-16.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                ..default()
            },
            Visibility::Hidden,
        ))
        .with_children(|c| {
            c.spawn((
                DamageLine,
                crate::flow::text("", crate::flow::style(fonts, 18.0, NORMAL_COLOR)),
            ));
            c.spawn((
                ReactionLine,
                crate::flow::text("", crate::flow::style(fonts, 14.0, REACTION_COLOR)),
            ));
        });
    });
}

/// 每帧刷新飘字：读 [`HitFeedback`] 计时淡出，按爆头/反应改写文本与颜色。
///
/// 覆盖语义：`remaining` 由 flow 侧 `record` 重置，连发时数字不断被最新命中替换。
pub fn update_hit_feedback(
    mut feedback: ResMut<HitFeedback>,
    time: Res<Time>,
    mut blocks: Query<&mut Visibility, With<FeedbackBlock>>,
    mut lines: Query<(&DamageLine, &mut Text, &mut TextColor)>,
    mut reactions: Query<(&ReactionLine, &mut Text, &mut TextColor), Without<DamageLine>>,
) {
    let Ok(mut vis) = blocks.single_mut() else { return };
    if feedback.remaining <= 0.0 {
        *vis = Visibility::Hidden;
        return;
    }
    *vis = Visibility::Visible;
    let remaining = feedback.tick(time.delta_secs());

    let alpha = (remaining / FADE_SECS).clamp(0.0, 1.0);
    let damage_color = with_alpha(
        if feedback.is_headshot { HEADSHOT_COLOR } else { NORMAL_COLOR },
        alpha,
    );
    for (_, mut text, mut color) in &mut lines {
        let want = feedback.damage_text();
        if **text != want {
            **text = want;
        }
        color.0 = damage_color;
    }
    let reaction_color = with_alpha(REACTION_COLOR, alpha);
    for (_, mut text, mut color) in &mut reactions {
        let want = feedback.reaction.clone().unwrap_or_default();
        if **text != want {
            **text = want;
        }
        color.0 = reaction_color;
    }
}

/// 伤害行文本（四舍五入取整展示；服务端权威数值，客户端不做任何换算）。
impl HitFeedback {
    fn damage_text(&self) -> String {
        format!("{}", self.damage.round() as i64)
    }
}

/// 颜色带透明度（淡出用；保持 RGB 不变只压 alpha）。
fn with_alpha(c: Color, alpha: f32) -> Color {
    let mut srgba = c.to_srgba();
    srgba.set_alpha(alpha);
    Color::Srgba(srgba)
}
