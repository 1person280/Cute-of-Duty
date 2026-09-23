//! HUD 构建：准星、干员名/血条/护甲条/技能栏、底部物品栏、弹药槽、击杀播报根、边缘光晕

use bevy::prelude::*;
use crate::element::ElementType;
use crate::model::palette;
use crate::demo::inventory::{HeldHintRoot, HELD_HINT_TEXT};
use crate::demo::extraction::{ExtractionHintRoot, ExtractionHintText};
use crate::demo::frontend::*;
use crate::demo::components::*;

pub(crate) fn setup_hud(mut commands: Commands) {
    // Crosshair lines：四线 + 中心点，围绕锚点以像素偏移布置

    // Root：屏幕居中容器 → 0×0 锚点 → 准星部件（任何窗口尺寸都在正中央）
    commands.spawn((
        Node {
            width: Val::Percent(100.0), height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        CrosshairRoot,
    )).with_children(|root| {
        root.spawn((
            Node {
                width: Val::Px(0.0), height: Val::Px(0.0),
                ..default()
            },
            CrosshairAnchor,
        )).with_children(|anchor| {
            anchor.spawn((Node {
                position_type: PositionType::Absolute,
                width: Val::Px(2.0), height: Val::Px(12.0),
                top: Val::Px(-20.0),
                left: Val::Px(-1.0),
                ..default()
            }, BackgroundColor(Color::srgba(0.95, 0.95, 0.95, 0.9)), CrosshairLine)); // top
            anchor.spawn((Node {
                position_type: PositionType::Absolute,
                width: Val::Px(2.0), height: Val::Px(10.0),
                top: Val::Px(-6.0),
                left: Val::Px(-1.0),
                ..default()
            }, BackgroundColor(Color::srgba(0.95, 0.95, 0.95, 0.9)), CrosshairCenter)); // upper center
            anchor.spawn((Node {
                position_type: PositionType::Absolute,
                width: Val::Px(2.0), height: Val::Px(12.0),
                top: Val::Px(8.0),
                left: Val::Px(-1.0),
                ..default()
            }, BackgroundColor(Color::srgba(0.95, 0.95, 0.95, 0.9)), CrosshairLine)); // bottom
            anchor.spawn((Node {
                position_type: PositionType::Absolute,
                width: Val::Px(12.0), height: Val::Px(2.0),
                top: Val::Px(-1.0),
                left: Val::Px(-20.0),
                ..default()
            }, BackgroundColor(Color::srgba(0.95, 0.95, 0.95, 0.9)), CrosshairLine)); // left
            anchor.spawn((Node {
                position_type: PositionType::Absolute,
                width: Val::Px(2.0), height: Val::Px(2.0),
                top: Val::Px(-1.0),
                left: Val::Px(-1.0),
                ..default()
            }, BackgroundColor(Color::srgba(0.95, 0.95, 0.95, 0.9)), CrosshairCenter)); // center dot
            anchor.spawn((Node {
                position_type: PositionType::Absolute,
                width: Val::Px(12.0), height: Val::Px(2.0),
                top: Val::Px(-1.0),
                left: Val::Px(8.0),
                ..default()
            }, BackgroundColor(Color::srgba(0.95, 0.95, 0.95, 0.9)), CrosshairLine)); // right
        });
    });

    // 手雷持握提示（准星下方，grenade_throw_system 控制显隐）
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            top: Val::Percent(58.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        Visibility::Hidden,
        HeldHintRoot,
    )).with_children(|hint| {
        hint.spawn((
            Text::new(HELD_HINT_TEXT),
            TextFont { font_size: FontSize::Px(16.0), ..default() },
            TextColor(Color::srgb(1.0, 0.8, 0.25)),
        ));
    });

    // 撤离区提示：接近北端红色信标时显示，extraction_zone_system 控制显隐与回车确认
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            top: Val::Percent(32.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        Visibility::Hidden,
        ExtractionHintRoot,
    )).with_children(|hint| {
        hint.spawn((
            Text::new("已抵达撤离区 · 按 Enter 确认撤离"),
            TextFont { font_size: FontSize::Px(18.0), ..default() },
            TextColor(Color::srgb(0.9, 0.2, 0.2)),
            ExtractionHintText,
        ));
    });

    // Bottom HUD bar
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0), height: Val::Px(160.0),
            bottom: Val::Px(0.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::End,
            padding: UiRect::all(Val::Px(20.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
    )).with_children(|bottom| {
        // LEFT: HP/Armor + Skills
        bottom.spawn((
            Node {
                flex_direction: FlexDirection::Column, align_items: AlignItems::Start, row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        )).with_children(|left| {
            // 当前干员名（元素色，随切换台变更）
            left.spawn((
                Text::new("干员 · 焰狐"),
                TextFont { font_size: FontSize::Px(15.0), ..default() },
                TextColor(ElementType::Fire.color()),
                HudOperatorName,
            ));
            // HP bar bg
            left.spawn((
                Node {
                    width: Val::Px(180.0), height: Val::Px(22.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.08, 0.08, 0.08)),
                HudHealthBarBg,
            )).with_children(|bg| {
                bg.spawn((
                    Node {
                        width: Val::Percent(100.0), height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(palette::HP_RED),
                    HudHealthBarFill,
                ));
            });
            left.spawn((
                Text::new("HP 100/100"),
                TextFont { font_size: FontSize::Px(13.0), ..default() },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                HudHealthText,
            ));

            // Armor bar bg
            left.spawn((
                Node {
                    width: Val::Px(140.0), height: Val::Px(10.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.08, 0.08, 0.08)),
                HudArmorBarBg,
            )).with_children(|bg| {
                bg.spawn((
                    Node {
                        width: Val::Percent(60.0), height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(palette::ARMOR_BLUE),
                    HudArmorBarFill,
                ));
            });
            left.spawn((
                Text::new("ARMOR 60/100"),
                TextFont { font_size: FontSize::Px(11.0), ..default() },
                TextColor(Color::srgb(0.7, 0.8, 1.0)),
                HudArmorText,
            ));

            // Skills row
            left.spawn((
                Node {
                    flex_direction: FlexDirection::Row, column_gap: Val::Px(10.0), margin: UiRect::top(Val::Px(10.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            )).with_children(|skills| {
                // Q skill
                skills.spawn((
                    Node {
                        flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(3.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                )).with_children(|q_col| {
                    q_col.spawn((
                        Node {
                            width: Val::Px(52.0), height: Val::Px(52.0),
                            justify_content: JustifyContent::Center, align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.08, 0.08, 0.08, 0.9)),
                    )).with_children(|q| {
                        q.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                width: Val::Percent(100.0), height: Val::Percent(0.0),
                                bottom: Val::Px(0.0),
                                ..default()
                            },
                            BackgroundColor(ElementType::Fire.color()),
                            HudSkillQFill,
                        ));
                        q.spawn((
                            Text::new("Q"),
                            TextFont { font_size: FontSize::Px(22.0), ..default() },
                            TextColor(ElementType::Fire.color()),
                            HudSkillQText,
                        )).with_children(|sec| {
                            // 字母常驻显示，冷却时右侧由子 TextSpan 追加倒计时秒数
                            sec.spawn((
                                TextSpan::new(""),
                                TextFont { font_size: FontSize::Px(12.0), ..default() },
                                TextColor(Color::srgb(0.95, 0.95, 0.95)),
                                HudSkillQText,
                            ));
                        });
                    });
                    // Bottom label bar
                    q_col.spawn((
                        Node {
                            width: Val::Px(54.0), height: Val::Px(16.0),
                            justify_content: JustifyContent::Center, align_items: AlignItems::Center,
                            margin: UiRect::top(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.95, 0.95, 0.95, 0.35)),
                    )).with_children(|label| {
                        label.spawn((
                            Text::new("Gren·火"),
                            TextFont { font_size: FontSize::Px(11.0), ..default() },
                            TextColor(Color::srgb(0.15, 0.15, 0.15)),
                            HudSkillQLabel,
                        ));
                    });
                });
                // E skill
                skills.spawn((
                    Node {
                        flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(3.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                )).with_children(|e_col| {
                    e_col.spawn((
                        Node {
                            width: Val::Px(52.0), height: Val::Px(52.0),
                            justify_content: JustifyContent::Center, align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.08, 0.08, 0.08, 0.9)),
                    )).with_children(|e| {
                        e.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                width: Val::Percent(100.0), height: Val::Percent(0.0),
                                bottom: Val::Px(0.0),
                                ..default()
                            },
                            BackgroundColor(ElementType::Fire.color()),
                            HudSkillEFill,
                        ));
                        e.spawn((
                            Text::new("E"),
                            TextFont { font_size: FontSize::Px(22.0), ..default() },
                            TextColor(ElementType::Fire.color()),
                            HudSkillEText,
                        )).with_children(|sec| {
                            // 字母常驻显示，冷却时右侧由子 TextSpan 追加倒计时秒数
                            sec.spawn((
                                TextSpan::new(""),
                                TextFont { font_size: FontSize::Px(12.0), ..default() },
                                TextColor(Color::srgb(0.95, 0.95, 0.95)),
                                HudSkillEText,
                            ));
                        });
                    });
                    // Bottom label bar
                    e_col.spawn((
                        Node {
                            width: Val::Px(54.0), height: Val::Px(16.0),
                            justify_content: JustifyContent::Center, align_items: AlignItems::Center,
                            margin: UiRect::top(Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.95, 0.95, 0.95, 0.35)),
                    )).with_children(|label| {
                        label.spawn((
                            Text::new("Burst·火"),
                            TextFont { font_size: FontSize::Px(11.0), ..default() },
                            TextColor(Color::srgb(0.15, 0.15, 0.15)),
                            HudSkillELabel,
                        ));
                    });
                });
                // 快捷道具图标（3 恢复 / 4 战术）：数量随背包实时刷新，无货变灰
                spawn_item_icon(skills, 0, "3", "恢复", ITEM_RECOVERY_COLOR);
                spawn_item_icon(skills, 1, "4", "战术", ITEM_TACTICAL_COLOR);
            });
        });

        // RIGHT: Weapons + Ammo
        bottom.spawn((
            Node {
                flex_direction: FlexDirection::Column, align_items: AlignItems::End, row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        )).with_children(|right| {
            // Weapon slots
            right.spawn((
                Node {
                    flex_direction: FlexDirection::Row, column_gap: Val::Px(8.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            )).with_children(|weapons| {
                weapons.spawn((
                    Text::new("[1] 烈焰步枪"),
                    TextFont { font_size: FontSize::Px(14.0), ..default() },
                    TextColor(ElementType::Fire.color()),
                    HudWeaponSlot1,
                ));
                weapons.spawn((
                    Text::new("[2] 冰霜步枪"),
                    TextFont { font_size: FontSize::Px(14.0), ..default() },
                    TextColor(Color::srgb(0.5, 0.5, 0.5)),
                    HudWeaponSlot2,
                ));
            });
            right.spawn((
                Text::new("30 / 90"),
                TextFont { font_size: FontSize::Px(32.0), ..default() },
                TextColor(Color::srgb(0.95, 0.95, 0.95)),
                HudAmmoMain,
            ));
            right.spawn((
                Text::new(""),
                TextFont { font_size: FontSize::Px(12.0), ..default() },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
                HudAmmoReserve,
            ));
            right.spawn((
                Text::new(""),
                TextFont { font_size: FontSize::Px(14.0), ..default() },
                TextColor(Color::srgb(0.9, 0.7, 0.2)),
                HudReloadText,
            ));
            right.spawn((
                Text::new(""),
                TextFont { font_size: FontSize::Px(12.0), ..default() },
                TextColor(Color::srgb(0.9, 0.5, 0.1)),
                HudWeaponName,
            ));
        });
    });

    // Edge glow for element status
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0), height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 0.0, 0.0, 0.0)),
        HudEdgeGlow,
    ));

    // Kill feed (top-right): total counter + fading kill notifications
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(14.0),
            right: Val::Px(16.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::End,
            row_gap: Val::Px(4.0),
            ..default()
        },
        KillFeedRoot,
    )).with_children(|feed| {
        feed.spawn((
            Text::new("击杀 0"),
            TextFont { font_size: FontSize::Px(18.0), ..default() },
            TextColor(Color::srgb(1.0, 0.8, 0.25)),
            KillFeedTotal,
        ));
    });
}

/// 在技能栏生成一个快捷道具图标（样式与 Q/E 技能图标一致：52x52 图标 + 底部标签条）。
/// 图标为父 Text（按键，读 Text+TextColor）+ 子 TextSpan（数量），共用 HudItemSlotText 标记。
pub(crate) fn spawn_item_icon(skills: &mut ChildSpawnerCommands, slot: usize, key: &str, label: &str, color: Color) {
    skills.spawn((
        Node {
            flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(3.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
    )).with_children(|col| {
        col.spawn((
            Node {
                width: Val::Px(52.0), height: Val::Px(52.0),
                justify_content: JustifyContent::Center, align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.08, 0.08, 0.9)),
        )).with_children(|icon| {
            icon.spawn((
                Text::new(key),
                TextFont { font_size: FontSize::Px(22.0), ..default() },
                TextColor(color),
                HudItemSlotText(slot),
            )).with_children(|sec| {
                // 按键常驻显示，背包有货时右侧由子 TextSpan 追加数量
                sec.spawn((
                    TextSpan::new(""),
                    TextFont { font_size: FontSize::Px(12.0), ..default() },
                    TextColor(Color::srgb(0.95, 0.95, 0.95)),
                    HudItemSlotText(slot),
                ));
            });
        });
        // Bottom label bar
        col.spawn((
            Node {
                width: Val::Px(54.0), height: Val::Px(16.0),
                justify_content: JustifyContent::Center, align_items: AlignItems::Center,
                margin: UiRect::top(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.95, 0.95, 0.95, 0.35)),
        )).with_children(|bar| {
            bar.spawn((
                Text::new(label),
                TextFont { font_size: FontSize::Px(11.0), ..default() },
                TextColor(Color::srgb(0.15, 0.15, 0.15)),
                HudItemSlotLabel(slot),
            ));
        });
    });
}