//! HUD：准星/血条/弹药/技能栏构建、每帧更新、底部物品栏、击杀播报

use bevy::prelude::*;
use crate::element::ElementType;
use crate::operator::roster;
use crate::model::{
    palette, Player, OperatorState, PlayerMovement,
};
use super::inventory::{HeldHintRoot, HELD_HINT_TEXT};
use super::common::*;
use super::components::*;

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

// =============================================================================
// 共享特效资产
// =============================================================================

/// 特效材质变体（按元素缓存，避免重复创建）
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum EffectMatKind {
    /// 基础自发光：曳光/碎块/投掷物
    Plain,
    /// 伤害粒子：自发光 × 0.5
    Particle,
    /// 命中爆闪：自发光 × 1.5
    HitFlash,
    /// 爆炸主体：半透明，自发光 × 2
    Explosion,
    /// 持续区域（毒雾等）：半透明，自发光 × 1.2
    Zone,
}

/// 所有一次性特效共享的网格与材质。
/// Bevy 0.14 的 `Assets` 不会自动回收：此前每颗子弹/每次爆炸都现场
/// `meshes.add` 新网格，实体销毁后 GPU 缓冲仍然累积，在核显上几十秒
/// 就会撑爆到 wgpu OutOfMemory 崩溃。
#[derive(Resource)]
pub(crate) struct EffectAssets {
    /// 曳光：单位深度细长盒，使用时按弹道长度缩放 Z
    pub(crate) tracer: Handle<Mesh>,
    /// 火花小球：枪口焰与命中爆闪共用（尺寸靠缩放区分）
    pub(crate) spark: Handle<Mesh>,
    /// 伤害粒子：小立方体
    pub(crate) particle: Handle<Mesh>,
    /// 爆炸主体：半透明大球（缩放由 ExplosionEffect 驱动）
    pub(crate) explosion_sphere: Handle<Mesh>,
    /// 爆炸碎块：小立方体
    pub(crate) explosion_debris: Handle<Mesh>,
    /// 手雷/爆裂投掷物：小球
    pub(crate) projectile: Handle<Mesh>,
    /// 冰冻冰块：罩住目标半透明立方体
    pub(crate) frost_cube: Handle<Mesh>,
    /// 毒雾/持续区域：单位圆柱，按区域半径缩放 XZ
    pub(crate) zone_cylinder: Handle<Mesh>,
    /// 冰冻冰块材质（半透明淡蓝，全目标共用）
    pub(crate) frost_material: Handle<StandardMaterial>,
    /// 枪口焰材质（固定暖白）
    pub(crate) muzzle_material: Handle<StandardMaterial>,
    /// (元素, 变体) → 材质 缓存
    pub(crate) element_materials: Vec<(ElementType, EffectMatKind, Handle<StandardMaterial>)>,
}

impl EffectAssets {
    pub(crate) fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            tracer: meshes.add(Cuboid::new(0.04, 0.04, 1.0)),
            spark: meshes.add(Sphere::new(0.08).mesh().ico(2).unwrap()),
            particle: meshes.add(Cuboid::new(0.08, 0.08, 0.08)),
            explosion_sphere: meshes.add(Sphere::new(0.5).mesh().ico(2).unwrap()),
            explosion_debris: meshes.add(Cuboid::new(0.1, 0.1, 0.1)),
            projectile: meshes.add(Sphere::new(0.15).mesh().ico(2).unwrap()),
            frost_cube: meshes.add(Cuboid::new(2.0, 2.0, 2.0)),
            zone_cylinder: meshes.add(Cylinder::new(1.0, 1.4)),
            frost_material: materials.add(StandardMaterial {
                base_color: Color::srgba(0.55, 0.85, 1.0, 0.45),
                emissive: LinearRgba::rgb(0.35, 0.7, 1.0) * 1.2,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            muzzle_material: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.9, 0.6),
                emissive: LinearRgba::rgb(1.0, 0.9, 0.6) * 8.0,
                ..default()
            }),
            element_materials: Vec::new(),
        }
    }

    /// 取指定元素与变体的材质，没有则创建并缓存
    pub(crate) fn material(
        &mut self,
        materials: &mut Assets<StandardMaterial>,
        element: ElementType,
        kind: EffectMatKind,
    ) -> Handle<StandardMaterial> {
        if let Some((_, _, handle)) = self
            .element_materials
            .iter()
            .find(|(e, k, _)| e == &element && k == &kind)
        {
            return handle.clone();
        }
        let color = element.color();
        let emissive = element.emissive();
        let mut mat = StandardMaterial { base_color: color, emissive, ..default() };
        match kind {
            EffectMatKind::Plain => {}
            EffectMatKind::Particle => mat.emissive = emissive * 0.5,
            EffectMatKind::HitFlash => mat.emissive = emissive * 1.5,
            EffectMatKind::Explosion => {
                mat.emissive = emissive * 2.0;
                mat.alpha_mode = AlphaMode::Blend;
            }
            EffectMatKind::Zone => {
                mat.base_color = color.with_alpha(0.35);
                mat.emissive = emissive * 1.2;
                mat.alpha_mode = AlphaMode::Blend;
            }
        }
        let handle = materials.add(mat);
        self.element_materials.push((element, kind, handle.clone()));
        handle
    }
}

// =============================================================================
// FPS Controller
// =============================================================================

#[allow(clippy::type_complexity)]
pub(crate) fn hud_update_system(
    player_query: Query<(&Health, &Armor, &WeaponSlot, &OperatorState, &PlayerMovement, &Inventory), With<Player>>,
    mut styles: ParamSet<(
        Query<&mut Style, With<HudHealthBarFill>>,
        Query<&mut Style, With<HudArmorBarFill>>,
        Query<&mut Style, With<HudSkillQFill>>,
        Query<&mut Style, With<HudSkillEFill>>,
    )>,
    mut texts: ParamSet<(
        Query<&mut Text, With<HudAmmoMain>>,
        Query<&mut Text, With<HudWeaponSlot1>>,
        Query<&mut Text, With<HudWeaponSlot2>>,
        Query<&mut Text, With<HudSkillQText>>,
        Query<&mut Text, With<HudSkillEText>>,
        Query<&mut Text, With<HudReloadText>>,
        Query<&mut Text, With<HudSkillQLabel>>,
        Query<&mut Text, With<HudSkillELabel>>,
    )>,
    mut q_fill_bg: Query<&mut BackgroundColor, (With<HudSkillQFill>, Without<HudSkillEFill>)>,
    mut e_fill_bg: Query<&mut BackgroundColor, (With<HudSkillEFill>, Without<HudSkillQFill>)>,
) {
    let Ok((health, armor, weapon_slot, op, movement, inventory)) = player_query.get_single() else { return };
    let op_def = &roster()[op.active];

    // 固定双主武器：slot 0/1 即 1/2 号位
    let cur_w = &inventory.weapons[weapon_slot.current];
    let w0 = &inventory.weapons[0];
    let w1 = &inventory.weapons[1];

    {
        let mut health_bar = styles.p0();
        if let Ok(mut style) = health_bar.get_single_mut() {
            style.width = Val::Percent((health.current / health.max * 100.0).clamp(0.0, 100.0));
        }
    }
    {
        let mut armor_bar = styles.p1();
        if let Ok(mut style) = armor_bar.get_single_mut() {
            style.width = Val::Percent((armor.current / armor.max * 100.0).clamp(0.0, 100.0));
        }
    }

    // 弹药主显示：弹匣 / 背包弹药池
    {
        let mut ammo_main = texts.p0();
        if let Ok(mut text) = ammo_main.get_single_mut() {
            text.sections[0].value = format!("{} / {}", cur_w.ammo, inventory.ammo_pool);
        }
    }

    {
        let mut weapon_slot1 = texts.p1();
        if let Ok(mut text) = weapon_slot1.get_single_mut() {
            let color = if weapon_slot.current == 0 { w0.element.color() } else { Color::srgb(0.4, 0.4, 0.4) };
            text.sections[0].value = format!("[1] {}", w0.name);
            text.sections[0].style.color = color;
        }
    }
    {
        let mut weapon_slot2 = texts.p2();
        if let Ok(mut text) = weapon_slot2.get_single_mut() {
            let color = if weapon_slot.current == 1 { w1.element.color() } else { Color::srgb(0.4, 0.4, 0.4) };
            text.sections[0].value = format!("[2] {}", w1.name);
            text.sections[0].style.color = color;
        }
    }

    // Skill Q（技能元素 = 干员亲和元素）
    let q_pct = (op.q.elapsed_secs() / op_def.q.cooldown_secs).clamp(0.0, 1.0);
    {
        let mut skill_q_fill = styles.p2();
        if let Ok(mut style) = skill_q_fill.get_single_mut() {
            style.height = Val::Percent(q_pct * 100.0);
        }
    }
    if let Ok(mut bg) = q_fill_bg.get_single_mut() {
        bg.0 = op_def.element.color();
    }
    {
        let mut skill_q_text = texts.p3();
        if let Ok(mut text) = skill_q_text.get_single_mut() {
            if q_pct >= 1.0 {
                text.sections[0].value = "Q".to_string();
                text.sections[0].style.color = op_def.element.color();
                text.sections[1].value = String::new();
            } else {
                // 字母保持可见（变灰），冷却秒数作为旁注追加
                text.sections[0].value = "Q".to_string();
                text.sections[0].style.color = Color::srgb(0.45, 0.45, 0.45);
                let remaining = op_def.q.cooldown_secs - op.q.elapsed_secs();
                text.sections[1].value = format!(" {:.0}", remaining.ceil());
            }
        }
    }

    // Skill E
    let e_pct = (op.e.elapsed_secs() / op_def.e.cooldown_secs).clamp(0.0, 1.0);
    {
        let mut skill_e_fill = styles.p3();
        if let Ok(mut style) = skill_e_fill.get_single_mut() {
            style.height = Val::Percent(e_pct * 100.0);
        }
    }
    if let Ok(mut bg) = e_fill_bg.get_single_mut() {
        bg.0 = op_def.element.color();
    }
    {
        let mut skill_e_text = texts.p4();
        if let Ok(mut text) = skill_e_text.get_single_mut() {
            if e_pct >= 1.0 {
                text.sections[0].value = "E".to_string();
                text.sections[0].style.color = op_def.element.color();
                text.sections[1].value = String::new();
            } else {
                text.sections[0].value = "E".to_string();
                text.sections[0].style.color = Color::srgb(0.45, 0.45, 0.45);
                let remaining = op_def.e.cooldown_secs - op.e.elapsed_secs();
                text.sections[1].value = format!(" {:.0}", remaining.ceil());
            }
        }
    }

    // Reload status: show prominently when reloading
    {
        let mut reload_text = texts.p5();
        if let Ok(mut text) = reload_text.get_single_mut() {
            if let Some(ref timer) = movement.reload_timer {
                let remaining = timer.duration().as_secs_f32() - timer.elapsed_secs();
                text.sections[0].value = format!("RELOADING {:.1}s", remaining.max(0.0));
                text.sections[0].style.color = Color::srgb(1.0, 0.6, 0.1);
                text.sections[0].style.font_size = 18.0;
            } else {
                text.sections[0].value = "".to_string();
                text.sections[0].style.font_size = 14.0;
            }
        }
    }

    // Skill bottom labels: 技能名随干员变化
    {
        let mut skill_q_label = texts.p6();
        if let Ok(mut text) = skill_q_label.get_single_mut() {
            text.sections[0].value = op_def.q.name.to_string();
            text.sections[0].style.color = op_def.element.color();
        }
    }
    {
        let mut skill_e_label = texts.p7();
        if let Ok(mut text) = skill_e_label.get_single_mut() {
            text.sections[0].value = op_def.e.name.to_string();
            text.sections[0].style.color = op_def.element.color();
        }
    }
}

/// 血条/护甲条数值文本（独立系统：hud_update_system 的 Text ParamSet 已满 8 席，
/// 且同一系统内两个可变 Text 查询必须装进 ParamSet，否则 B0001 冲突）
pub(crate) fn hud_vitals_text_system(
    player_query: Query<(&Health, &Armor), With<Player>>,
    mut vitals: ParamSet<(
        Query<&mut Text, With<HudHealthText>>,
        Query<&mut Text, With<HudArmorText>>,
    )>,
) {
    let Ok((health, armor)) = player_query.get_single() else { return };
    {
        let mut hp = vitals.p0();
        if let Ok(mut text) = hp.get_single_mut() {
            text.sections[0].value = format!("HP {}/{}", health.current.round() as i32, health.max as i32);
        }
    }
    {
        let mut armor_text = vitals.p1();
        if let Ok(mut text) = armor_text.get_single_mut() {
            text.sections[0].value = format!("ARMOR {}/{}", armor.current.round() as i32, armor.max as i32);
        }
    }
}

/// 当前干员名 HUD（独立系统：hud_update_system 的 Text ParamSet 已满 8 席）
pub(crate) fn hud_operator_name_system(
    player_query: Query<&OperatorState, With<Player>>,
    mut operator_name: Query<&mut Text, With<HudOperatorName>>,
) {
    let Ok(op) = player_query.get_single() else { return };
    let Ok(mut text) = operator_name.get_single_mut() else { return };
    let op_def = &roster()[op.active];
    text.sections[0].value = format!("干员 · {}", op_def.name);
    text.sections[0].style.color = op_def.element.color();
}

pub(crate) fn low_ammo_blink(
    mut ammo_main: Query<&mut Text, With<HudAmmoMain>>,
    player_query: Query<(&WeaponSlot, &Inventory), With<Player>>,
    time: Res<Time>,
) {
    let Ok((weapon_slot, inventory)) = player_query.get_single() else { return };
    let weapon = &inventory.weapons[weapon_slot.current];
    let Ok(mut text) = ammo_main.get_single_mut() else { return };

    if weapon.ammo <= 5 && weapon.ammo > 0 {
        let flash = (time.elapsed_seconds() * 6.0).sin() > 0.0;
        text.sections[0].style.color = if flash { Color::srgb(1.0, 0.2, 0.2) } else { Color::srgb(0.95, 0.95, 0.95) };
    } else {
        text.sections[0].style.color = Color::srgb(0.95, 0.95, 0.95);
    }
}

pub(crate) fn screen_edge_glow(
    mut edge_glow: Query<&mut BackgroundColor, With<HudEdgeGlow>>,
    player_query: Query<&Health, With<Player>>,
) {
    let Ok(health) = player_query.get_single() else { return };
    let Ok(mut bg) = edge_glow.get_single_mut() else { return };

    let hp_ratio = health.current / health.max;
    if hp_ratio < 0.3 {
        bg.0 = Color::srgba(1.0, 0.1, 0.1, 0.15 * (1.0 - hp_ratio / 0.3));
    } else {
        bg.0 = Color::srgba(1.0, 0.0, 0.0, 0.0);
    }
}

pub(crate) fn respawn_flash_system(
    _query: Query<&mut BackgroundColor, With<HudEdgeGlow>>,
) {
}

// =============================================================================
// Inventory & Pickup Systems
// =============================================================================

/// 底部 3/4 道具图标：显示背包中对应类别的数量，无货时按键变灰
pub(crate) fn hud_item_slots_system(
    player_query: Query<&Inventory, With<Player>>,
    mut slot_texts: Query<(&HudItemSlotText, &mut Text), Without<HudItemSlotLabel>>,
    mut slot_labels: Query<(&HudItemSlotLabel, &mut Text), Without<HudItemSlotText>>,
) {
    let mut counts = [0usize; 2];
    if let Ok(inventory) = player_query.get_single() {
        for item in &inventory.items {
            match item.item_type.category() {
                ItemCategory::Consumable => counts[0] += 1,
                ItemCategory::Tactical => counts[1] += 1,
            }
        }
    }

    let colors = [ITEM_RECOVERY_COLOR, ITEM_TACTICAL_COLOR];
    for (slot, mut text) in slot_texts.iter_mut() {
        let n = counts[slot.0];
        text.sections[0].style.color = if n > 0 {
            colors[slot.0]
        } else {
            Color::srgb(0.35, 0.35, 0.35)
        };
        text.sections[1].value = if n > 0 { format!(" ×{}", n) } else { String::new() };
    }
    for (slot, mut text) in slot_labels.iter_mut() {
        text.sections[0].style.color = if counts[slot.0] > 0 {
            Color::srgb(0.15, 0.15, 0.15)
        } else {
            Color::srgba(0.15, 0.15, 0.15, 0.45)
        };
    }
}

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





