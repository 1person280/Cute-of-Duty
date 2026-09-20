//! HUD 构建：准星、干员名/血条/护甲条/技能栏、底部物品栏、弹药槽、击杀播报根、边缘光晕

use bevy::prelude::*;
use crate::element::ElementType;
use crate::model::palette;
use crate::demo::inventory::{HeldHintRoot, HELD_HINT_TEXT};
use crate::demo::frontend::*;
use crate::demo::components::*;

pub(crate) fn setup_hud(mut commands: Commands) {
    // Crosshair lines：四线 + 中心点，围绕锚点以像素偏移布置
    let crosshair_style = |left: f32, top: f32, w: f32, h: f32| NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            width: Val::Px(w), height: Val::Px(h),
            top: Val::Px(top),
            left: Val::Px(left),
            ..default()
        },
        background_color: BackgroundColor(Color::srgba(0.95, 0.95, 0.95, 0.9)),
        ..default()
    };

    // Root：屏幕居中容器 → 0×0 锚点 → 准星部件（任何窗口尺寸都在正中央）
    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0), height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            ..default()
        },
        CrosshairRoot,
    )).with_children(|root| {
        root.spawn((
            NodeBundle { style: Style { width: Val::Px(0.0), height: Val::Px(0.0), ..default() }, ..default() },
            CrosshairAnchor,
        )).with_children(|anchor| {
            anchor.spawn((crosshair_style(-1.0, -20.0, 2.0, 12.0), CrosshairLine)); // top
            anchor.spawn((crosshair_style(-1.0, -6.0, 2.0, 10.0), CrosshairCenter)); // upper center
            anchor.spawn((crosshair_style(-1.0, 8.0, 2.0, 12.0), CrosshairLine)); // bottom
            anchor.spawn((crosshair_style(-20.0, -1.0, 12.0, 2.0), CrosshairLine)); // left
            anchor.spawn((crosshair_style(-1.0, -1.0, 2.0, 2.0), CrosshairCenter)); // center dot
            anchor.spawn((crosshair_style(8.0, -1.0, 12.0, 2.0), CrosshairLine)); // right
        });
    });

    // 手雷持握提示（准星下方，grenade_throw_system 控制显隐）
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                top: Val::Percent(58.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            visibility: Visibility::Hidden,
            ..default()
        },
        HeldHintRoot,
    )).with_children(|hint| {
        hint.spawn(TextBundle {
            text: Text::from_section(
                HELD_HINT_TEXT,
                TextStyle { font_size: 16.0, color: Color::srgb(1.0, 0.8, 0.25), ..default() },
            ),
            ..default()
        });
    });

    // Bottom HUD bar
    commands.spawn(NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0), height: Val::Px(160.0),
            bottom: Val::Px(0.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::End,
            padding: UiRect::all(Val::Px(20.0)),
            ..default()
        },
        background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        ..default()
    }).with_children(|bottom| {
        // LEFT: HP/Armor + Skills
        bottom.spawn(NodeBundle {
            style: Style { flex_direction: FlexDirection::Column, align_items: AlignItems::Start, row_gap: Val::Px(6.0), ..default() },
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            ..default()
        }).with_children(|left| {
            // 当前干员名（元素色，随切换台变更）
            left.spawn((
                TextBundle {
                    text: Text::from_section(
                        "干员 · 焰狐",
                        TextStyle { font_size: 15.0, color: ElementType::Fire.color(), ..default() },
                    ),
                    ..default()
                },
                HudOperatorName,
            ));
            // HP bar bg
            left.spawn((
                NodeBundle {
                    style: Style { width: Val::Px(180.0), height: Val::Px(22.0), ..default() },
                    background_color: BackgroundColor(Color::srgb(0.08, 0.08, 0.08)),
                    ..default()
                },
                HudHealthBarBg,
            )).with_children(|bg| {
                bg.spawn((
                    NodeBundle {
                        style: Style { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                        background_color: BackgroundColor(palette::HP_RED),
                        ..default()
                    },
                    HudHealthBarFill,
                ));
            });
            left.spawn((
                TextBundle {
                    text: Text::from_section("HP 100/100", TextStyle { font_size: 13.0, color: Color::srgb(0.9, 0.9, 0.9), ..default() }),
                    ..default()
                },
                HudHealthText,
            ));

            // Armor bar bg
            left.spawn((
                NodeBundle {
                    style: Style { width: Val::Px(140.0), height: Val::Px(10.0), ..default() },
                    background_color: BackgroundColor(Color::srgb(0.08, 0.08, 0.08)),
                    ..default()
                },
                HudArmorBarBg,
            )).with_children(|bg| {
                bg.spawn((
                    NodeBundle {
                        style: Style { width: Val::Percent(60.0), height: Val::Percent(100.0), ..default() },
                        background_color: BackgroundColor(palette::ARMOR_BLUE),
                        ..default()
                    },
                    HudArmorBarFill,
                ));
            });
            left.spawn((
                TextBundle {
                    text: Text::from_section("ARMOR 60/100", TextStyle { font_size: 11.0, color: Color::srgb(0.7, 0.8, 1.0), ..default() }),
                    ..default()
                },
                HudArmorText,
            ));

            // Skills row
            left.spawn(NodeBundle {
                style: Style { flex_direction: FlexDirection::Row, column_gap: Val::Px(10.0), margin: UiRect::top(Val::Px(10.0)), ..default() },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                ..default()
            }).with_children(|skills| {
                // Q skill
                skills.spawn(NodeBundle {
                    style: Style { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(3.0), ..default() },
                    background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                    ..default()
                }).with_children(|q_col| {
                    q_col.spawn(NodeBundle {
                        style: Style { width: Val::Px(52.0), height: Val::Px(52.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                        background_color: BackgroundColor(Color::srgba(0.08, 0.08, 0.08, 0.9)),
                        ..default()
                    }).with_children(|q| {
                        q.spawn((
                            NodeBundle {
                                style: Style { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(0.0), bottom: Val::Px(0.0), ..default() },
                                background_color: BackgroundColor(ElementType::Fire.color()),
                                ..default()
                            },
                            HudSkillQFill,
                        ));
                        q.spawn((
                            TextBundle {
                                // 字母常驻显示，冷却时右侧追加倒计时秒数
                                text: Text::from_sections([
                                    TextSection::new("Q", TextStyle { font_size: 22.0, color: ElementType::Fire.color(), ..default() }),
                                    TextSection::new("", TextStyle { font_size: 12.0, color: Color::srgb(0.95, 0.95, 0.95), ..default() }),
                                ]),
                                ..default()
                            },
                            HudSkillQText,
                        ));
                    });
                    // Bottom label bar
                    q_col.spawn(NodeBundle {
                        style: Style { width: Val::Px(54.0), height: Val::Px(16.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, margin: UiRect::top(Val::Px(2.0)), ..default() },
                        background_color: BackgroundColor(Color::srgba(0.95, 0.95, 0.95, 0.35)),
                        ..default()
                    }).with_children(|label| {
                        label.spawn((
                            TextBundle {
                                text: Text::from_section("Gren·火", TextStyle { font_size: 11.0, color: Color::srgb(0.15, 0.15, 0.15), ..default() }),
                                ..default()
                            },
                            HudSkillQLabel,
                        ));
                    });
                });
                // E skill
                skills.spawn(NodeBundle {
                    style: Style { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(3.0), ..default() },
                    background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                    ..default()
                }).with_children(|e_col| {
                    e_col.spawn(NodeBundle {
                        style: Style { width: Val::Px(52.0), height: Val::Px(52.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                        background_color: BackgroundColor(Color::srgba(0.08, 0.08, 0.08, 0.9)),
                        ..default()
                    }).with_children(|e| {
                        e.spawn((
                            NodeBundle {
                                style: Style { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(0.0), bottom: Val::Px(0.0), ..default() },
                                background_color: BackgroundColor(ElementType::Fire.color()),
                                ..default()
                            },
                            HudSkillEFill,
                        ));
                        e.spawn((
                            TextBundle {
                                // 字母常驻显示，冷却时右侧追加倒计时秒数
                                text: Text::from_sections([
                                    TextSection::new("E", TextStyle { font_size: 22.0, color: ElementType::Fire.color(), ..default() }),
                                    TextSection::new("", TextStyle { font_size: 12.0, color: Color::srgb(0.95, 0.95, 0.95), ..default() }),
                                ]),
                                ..default()
                            },
                            HudSkillEText,
                        ));
                    });
                    // Bottom label bar
                    e_col.spawn(NodeBundle {
                        style: Style { width: Val::Px(54.0), height: Val::Px(16.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, margin: UiRect::top(Val::Px(2.0)), ..default() },
                        background_color: BackgroundColor(Color::srgba(0.95, 0.95, 0.95, 0.35)),
                        ..default()
                    }).with_children(|label| {
                        label.spawn((
                            TextBundle {
                                text: Text::from_section("Burst·火", TextStyle { font_size: 11.0, color: Color::srgb(0.15, 0.15, 0.15), ..default() }),
                                ..default()
                            },
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
        bottom.spawn(NodeBundle {
            style: Style { flex_direction: FlexDirection::Column, align_items: AlignItems::End, row_gap: Val::Px(4.0), ..default() },
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            ..default()
        }).with_children(|right| {
            // Weapon slots
            right.spawn(NodeBundle {
                style: Style { flex_direction: FlexDirection::Row, column_gap: Val::Px(8.0), ..default() },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                ..default()
            }).with_children(|weapons| {
                weapons.spawn((
                    TextBundle {
                        text: Text::from_section("[1] 烈焰步枪", TextStyle { font_size: 14.0, color: ElementType::Fire.color(), ..default() }),
                        ..default()
                    },
                    HudWeaponSlot1,
                ));
                weapons.spawn((
                    TextBundle {
                        text: Text::from_section("[2] 冰霜步枪", TextStyle { font_size: 14.0, color: Color::srgb(0.5, 0.5, 0.5), ..default() }),
                        ..default()
                    },
                    HudWeaponSlot2,
                ));
            });
            right.spawn((
                TextBundle {
                    text: Text::from_section("30 / 90", TextStyle { font_size: 32.0, color: Color::srgb(0.95, 0.95, 0.95), ..default() }),
                    ..default()
                },
                HudAmmoMain,
            ));
            right.spawn((
                TextBundle {
                    text: Text::from_section("", TextStyle { font_size: 12.0, color: Color::srgb(0.7, 0.7, 0.7), ..default() }),
                    ..default()
                },
                HudAmmoReserve,
            ));
            right.spawn((
                TextBundle {
                    text: Text::from_section("", TextStyle { font_size: 14.0, color: Color::srgb(0.9, 0.7, 0.2), ..default() }),
                    ..default()
                },
                HudReloadText,
            ));
            right.spawn((
                TextBundle {
                    text: Text::from_section("", TextStyle { font_size: 12.0, color: Color::srgb(0.9, 0.5, 0.1), ..default() }),
                    ..default()
                },
                HudWeaponName,
            ));
        });
    });

    // Edge glow for element status
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0), height: Val::Percent(100.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(1.0, 0.0, 0.0, 0.0)),
            ..default()
        },
        HudEdgeGlow,
    ));

    // Kill feed (top-right): total counter + fading kill notifications
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(14.0),
                right: Val::Px(16.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::End,
                row_gap: Val::Px(4.0),
                ..default()
            },
            background_color: BackgroundColor(Color::NONE),
            ..default()
        },
        KillFeedRoot,
    )).with_children(|feed| {
        feed.spawn((
            TextBundle {
                text: Text::from_section(
                    "击杀 0",
                    TextStyle { font_size: 18.0, color: Color::srgb(1.0, 0.8, 0.25), ..default() }
                ),
                ..default()
            },
            KillFeedTotal,
        ));
    });
}

/// 在技能栏生成一个快捷道具图标（样式与 Q/E 技能图标一致：52x52 图标 + 底部标签条）
pub(crate) fn spawn_item_icon(skills: &mut ChildBuilder, slot: usize, key: &str, label: &str, color: Color) {
    skills.spawn(NodeBundle {
        style: Style { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(3.0), ..default() },
        background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        ..default()
    }).with_children(|col| {
        col.spawn(NodeBundle {
            style: Style { width: Val::Px(52.0), height: Val::Px(52.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
            background_color: BackgroundColor(Color::srgba(0.08, 0.08, 0.08, 0.9)),
            ..default()
        }).with_children(|icon| {
            icon.spawn((
                TextBundle {
                    // 按键常驻显示，背包有货时右侧追加数量
                    text: Text::from_sections([
                        TextSection::new(key, TextStyle { font_size: 22.0, color, ..default() }),
                        TextSection::new("", TextStyle { font_size: 12.0, color: Color::srgb(0.95, 0.95, 0.95), ..default() }),
                    ]),
                    ..default()
                },
                HudItemSlotText(slot),
            ));
        });
        // Bottom label bar
        col.spawn(NodeBundle {
            style: Style { width: Val::Px(54.0), height: Val::Px(16.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, margin: UiRect::top(Val::Px(2.0)), ..default() },
            background_color: BackgroundColor(Color::srgba(0.95, 0.95, 0.95, 0.35)),
            ..default()
        }).with_children(|bar| {
            bar.spawn((
                TextBundle {
                    text: Text::from_section(label, TextStyle { font_size: 11.0, color: Color::srgb(0.15, 0.15, 0.15), ..default() }),
                    ..default()
                },
                HudItemSlotLabel(slot),
            ));
        });
    });
}