//! HUD 左下 vitals：血条 / 护甲条 / 弹药 / 武器槽 / 干员名
//!
//! 设计动机：Vitals 只读权威快照中本人条目。血量条那栏底图可用 `assets/ui/health_bar.jpg`
//! 做铺垫（贴图就绪才挂），其上叠加**过程化**红色填充条（宽度随 `hp%`），保证即便贴图
//! 路径失效也照样显示血条数值质感——图片锦上添花，数值永远可靠。

use bevy::prelude::*;

use crate::flow::flow_state::{self as flow, CjkFont, LocalPlayer};
use super::hud_root::optional_image;
use crate::shared::operator_meta::meta;
use crate::net::snapshot::SnapshotBuffer;
use crate::shared::theme;
use crate::shared::ui_assets::UiAssets;

/// 血条填充（宽度按 hp% 更新）。
#[derive(Component)]
pub struct HealthFill;

/// 护甲条填充（宽度按 armor 相对上限更新）。
#[derive(Component)]
pub struct ArmorFill;

/// vitals 面板顶行：干员名（逐帧按 operator_id 上色）。
#[derive(Component)]
pub struct OperatorName;

/// 物品/弹药槽行文本（逐帧刷新）。
#[derive(Component)]
pub struct ItemSlotText;

/// 装配 vitals 面板（左下角，旧版样式：HP 180×22、护盾 140×10、深色半透明底）。
pub fn spawn_vitals(p: &mut ChildBuilder<'_>, fonts: &CjkFont, ui: &UiAssets) {
    // 左下 vitals 容器（绝对定位，深蓝半透明底）
    p.spawn(NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            bottom: Val::Px(16.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        background_color: theme::PANEL_BG.into(),
        ..default()
    })
    .with_children(|v| {
        // 干员行：头像（可无）+ 干员名（元素色）
        let mut avatar_row = v.spawn(NodeBundle {
            style: Style {
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                ..default()
            },
            ..default()
        });
        optional_image(&mut avatar_row, ui, ui.avatar.clone(), Val::Px(44.0), Val::Px(44.0));
        avatar_row.with_children(|a| {
            a.spawn((
                OperatorName,
                TextBundle::from_section(
                    "干员 #0",
                    flow::style(fonts, 20.0, theme::TEXT_WHITE),
                ),
            ));
        });

        // 血条：底图（可无）+ 填充；旧版 180×22
        let mut hp = v.spawn(NodeBundle {
            style: Style {
                width: Val::Px(180.0),
                height: Val::Px(22.0),
                justify_content: JustifyContent::FlexStart,
                ..default()
            },
            background_color: Color::srgb(0.15, 0.13, 0.13).into(),
            ..default()
        });
        optional_image(&mut hp, ui, ui.health_bar.clone(), Val::Px(180.0), Val::Px(22.0));
        hp.with_children(|h| {
            h.spawn((
                HealthFill,
                NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    background_color: HP_RED.into(),
                    ..default()
                },
            ));
        });

        // 护甲条：蓝色过程化条；旧版 140×10
        v.spawn(NodeBundle {
            style: Style {
                width: Val::Px(140.0),
                height: Val::Px(10.0),
                justify_content: JustifyContent::FlexStart,
                ..default()
            },
            background_color: Color::srgb(0.10, 0.13, 0.17).into(),
            ..default()
        })
        .with_children(|a| {
            a.spawn((
                ArmorFill,
                NodeBundle {
                    style: Style {
                        width: Val::Percent(0.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    background_color: ARMOR_BLUE.into(),
                    ..default()
                },
            ));
        });

        // 物品/弹药槽行（展示性，弹药来自快照）
        v.spawn((
            ItemSlotText,
            TextBundle::from_section(
                "弹药 --",
                flow::style(fonts, 15.0, theme::TEXT_DIM),
            ),
        ));
    });
}

/// 每帧刷新 vitals：血条宽度、护甲条宽度、干员名与物品槽文本。
///
/// 各处文本/条以 `Without` 交叉标记证明不相交，避免 B0001 可变访问冲突。
#[allow(clippy::type_complexity)]
pub fn update_vitals(
    snap: Res<SnapshotBuffer>,
    player: Res<LocalPlayer>,
    mut hp: Query<&mut Style, (With<HealthFill>, Without<ArmorFill>, Without<OperatorName>, Without<ItemSlotText>)>,
    mut armor: Query<&mut Style, (With<ArmorFill>, Without<HealthFill>, Without<OperatorName>, Without<ItemSlotText>)>,
    mut name: Query<&mut Text, (With<OperatorName>, Without<HealthFill>, Without<ArmorFill>, Without<ItemSlotText>)>,
    mut slots: Query<&mut Text, (With<ItemSlotText>, Without<HealthFill>, Without<ArmorFill>, Without<OperatorName>)>,
) {
    let snapshot = snap
        .current
        .iter()
        .find(|e| e.entity_id == player.entity_id);

    // 缺本人实体：全默认（100%、0%、一号干员、占位文本）
    let (hp_f, armor_f, ammo, op) = match snapshot {
        Some(e) => (
            (e.hp / 100.0).clamp(0.0, 1.0),
            (e.armor / 100.0).clamp(0.0, 1.0),
            e.ammo,
            e.operator_id,
        ),
        None => (1.0, 0.0, -1, 0),
    };

    if let Ok(mut s) = hp.get_single_mut() {
        s.width = Val::Percent(hp_f * 100.0);
    }
    if let Ok(mut s) = armor.get_single_mut() {
        s.width = Val::Percent(armor_f * 100.0);
    }
    if let Ok(mut t) = name.get_single_mut() {
        t.sections[0].value = meta(op).name.to_string();
        t.sections[0].style.color = meta(op).color;
    }
    if let Ok(mut t) = slots.get_single_mut() {
        let ammo_str = if ammo < 0 { "--".to_string() } else { format!("{ammo}") };
        t.sections[0].value = format!("弹药 {ammo_str}");
    }
}

/// 血条红。
const HP_RED: Color = Color::srgb(0.90, 0.18, 0.18);
/// 护甲蓝。
const ARMOR_BLUE: Color = Color::srgb(0.25, 0.55, 0.95);