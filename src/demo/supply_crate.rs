//! 对局内物资箱：场景散布的木箱，开启后弹出 3×4（12格）随机战利品面板。
//!
//! 设计取舍：
//! - 战利品**实体化**为 [`SupplyCrate`] 组件（含 12 格 `Option<PickupItem>`，
//!   取走的格子为 None），而非临时资源——多个箱子的战利品互不干扰，
//!   且箱体与战利品同生命周期，随关卡重建一起销毁。
//! - 面板采用「3×4 战利品格 + 下方背包落点格」两区布局，支持**拖拽**（按住
//!   左键拖动 + 悬停背包格松开）与 **Shift+左键快移**，与赛前仓库选装手感一致。
//! - 打开走**既有交互物品通路**：箱子登记为 [`Station`]（`StationKind::SupplyCrate`），
//!   靠近后出现在统一交互菜单（站点条目优先于散落拾取物，F 必开箱不误拾），
//!   确认后由 `station_system` 打开本模块的面板并放光标，Esc / F 关闭并锁回光标。
//! - 拖拽半透明幻影跟随光标，松开在背包格上才真正结算（否则不消耗战利品）。

use bevy::prelude::*;
use bevy::pbr::NotShadowCaster;
use bevy::window::PrimaryWindow;
use rand::Rng;
use crate::element::ElementType;
use crate::map::StationKind;
use crate::model::{mat_voxel, mat_emissive, Player};
use crate::demo::components::*;
use crate::demo::inventory::pickup_text_color;

/// 战利品格子数：3 列 × 4 行
pub(crate) const CRATE_CELLS: usize = 12;
/// 面板下方背包落点格数
pub(crate) const CRATE_SLOTS: usize = 8;
/// 箱体半尺寸
pub(crate) const CRATE_HALF: [f32; 3] = [0.5, 0.35, 0.5];

/// 场景木箱实体：战利品（12 格，None = 已取走）+ 显示名
#[derive(Component)]
pub(crate) struct SupplyCrate {
    pub(crate) loot: Vec<Option<PickupItem>>,
    pub(crate) label: &'static str,
}

/// 当前打开的物资箱窗口：None = 关闭
#[derive(Resource, Default)]
pub(crate) struct CrateWindow {
    pub(crate) crate_entity: Option<Entity>,
}

/// 战利品拖拽中间态：Some(格序) = 正在拖动该格
#[derive(Resource, Default)]
pub(crate) struct CrateDrag {
    pub(crate) source: Option<usize>,
}

pub(crate) const CRATE_GHOST_SNAP: f32 = 90.0;

#[derive(Component)]
pub(crate) struct CrateUIRoot;

#[derive(Component)]
pub(crate) struct CrateCellUI(pub(crate) usize);

#[derive(Component)]
pub(crate) struct CrateCellText(pub(crate) usize);

/// 面板下方背包落点格（独立组件，避免与全局背包查询串扰）
#[derive(Component)]
pub(crate) struct CrateSlotUI(pub(crate) usize);

#[derive(Component)]
pub(crate) struct CrateSlotText(pub(crate) usize);

#[derive(Component)]
pub(crate) struct CrateGhost;

#[derive(Component)]
pub(crate) struct CrateGhostText;

#[derive(Component)]
pub(crate) struct CrateHintText;

/// 随机战利品工厂数组：12 种候选，生成时等权随机取满。
/// 覆盖补给台物资（弹药/医疗/护甲/四系手雷）+ 武器。
fn roll_crate_item() -> PickupItem {
    const POOL: [fn() -> PickupItem; 12] = [
        || PickupItem { name: "步枪弹药 ×60".into(), item_type: PickupType::Ammo { amount: 60 } },
        || PickupItem { name: "步枪弹药 ×60".into(), item_type: PickupType::Ammo { amount: 60 } },
        || PickupItem { name: "医疗包".into(), item_type: PickupType::Health { amount: 25.0 } },
        || PickupItem { name: "医疗包".into(), item_type: PickupType::Health { amount: 25.0 } },
        || PickupItem { name: "大型医疗包".into(), item_type: PickupType::Health { amount: 50.0 } },
        || PickupItem { name: "护甲片".into(), item_type: PickupType::Armor { amount: 20.0 } },
        || PickupItem { name: "烈焰手雷".into(), item_type: PickupType::Grenade { element: ElementType::Fire } },
        || PickupItem { name: "冰霜手雷".into(), item_type: PickupType::Grenade { element: ElementType::Ice } },
        || PickupItem { name: "雷电手雷".into(), item_type: PickupType::Grenade { element: ElementType::Electric } },
        || PickupItem { name: "毒素手雷".into(), item_type: PickupType::Grenade { element: ElementType::Poison } },
        || PickupItem { name: "烈焰步枪".into(), item_type: PickupType::Weapon { element: ElementType::Fire } },
        || PickupItem { name: "冰霜步枪".into(), item_type: PickupType::Weapon { element: ElementType::Ice } },
    ];
    let idx = rand::thread_rng().gen_range(0..POOL.len());
    POOL[idx]()
}

fn random_crate_loot() -> Vec<Option<PickupItem>> {
    (0..CRATE_CELLS).map(|_| Some(roll_crate_item())).collect()
}

/// 上一局遗留的打开态/拖拽态在重建关卡前清空（实体已随关卡销毁）。
/// 与 [`spawn_crates`] 拆开：本系统仅重置资源，不触碰场景。OnEnter InGame 调用。
pub(crate) fn reset_crate_state(
    mut window: ResMut<CrateWindow>,
    mut drag: ResMut<CrateDrag>,
) {
    window.crate_entity = None;
    drag.source = None;
}

/// 生成一批物资箱到场景，**由 `setup_world` 在生成玩家之后调用**（同一命令批次，
/// 保证只要开局玩家出生，木箱必然在场，杜绝调度遗漏）。木箱几何与材质全场共享
/// 一份 GPU 缓冲（核显友好）；选点贴近出生区北侧，玩家开局即见、必然经过。
pub(crate) fn spawn_crates(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // 体素木箱：四角立柱 + 各面横向木板条（留板缝成栅格，像素可爱风）+ 平顶箱盖。
    // 全部用共享 mesh/材质，一次入 GPU 缓冲，核显友好。
    // 货箱外包络仍是 [±CRATE_HALF]，立柱/板条都在其内，碰撞体/交互距离不变。
    // --- 四角立柱（贯穿箱体高度的竖向细柱）---
    let post = meshes.add(Cuboid::new(0.12, CRATE_HALF[1] * 2.0, 0.12));
    // --- 横向木板条（长边跨箱体宽度，薄片，供四面外沿排列）---
    let slat = meshes.add(Cuboid::new(CRATE_HALF[0] * 2.0 - 0.10, 0.13, 0.045));
    // --- 平顶箱盖（略大于箱口）---
    let lid = meshes.add(Cuboid::new(CRATE_HALF[0] * 2.0 + 0.10, 0.07, CRATE_HALF[2] * 2.0 + 0.10));
    // --- 箱顶发光信标：橙色细柱，让玩家很远就能定位（核显无大光源开销）---
    let beacon_mesh = meshes.add(Cuboid::new(0.22, 0.5, 0.22));

    let frame_mat = mat_voxel(materials, Color::srgb(0.38, 0.26, 0.15));   // 深色立柱/边框
    let plank_mat = mat_voxel(materials, Color::srgb(0.66, 0.46, 0.23));   // 浅色木板条
    let lid_mat = mat_voxel(materials, Color::srgb(0.74, 0.53, 0.29));     // 箱盖
    let beacon_mat = mat_emissive(materials, Color::srgb(1.0, 0.55, 0.12));

    // 各面木板条的垂直位置（沿 y 均布，间留板缝，让箱体呈栅格透光感）
    let slat_ys = [-0.26, -0.11, 0.04, 0.19, 0.32];

    // 出生点 (0,470) 面向 -z：首箱就摆在出生点正前方 8m，转弯处也补两箱，
    // 确保玩家开局第一眼看到的"发光柱"下就是可开箱木箱，而不是被更远的
    // 散落橙色补给（普通拾取）抢走注意力。
    let spots = [
        Vec3::new(0.0, 0.0, 466.0),
        Vec3::new(90.0, 0.0, 455.0),
        Vec3::new(-90.0, 0.0, 455.0),
        Vec3::new(0.0, 0.0, 350.0),
    ];
    for (i, pos) in spots.iter().enumerate() {
        commands
            .spawn((
                SpatialBundle {
                    transform: Transform::from_translation(*pos),
                    ..default()
                },
                SupplyCrate {
                    loot: random_crate_loot(),
                    label: "物资箱",
                },
                // 登记为交互站点：走近出现在统一交互菜单（站点优先于拾取），
                // F 由 station_system 打开本模块面板。
                Station { kind: StationKind::SupplyCrate, label: "物资箱" },
                Collider { half_size: Vec3::from(CRATE_HALF) },
                NotShadowCaster,
            ))
            .with_children(|c| {
                // 四角立柱（略内收，形成仓体骨架）
                for (dx, dz) in [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
                    let inset = CRATE_HALF[0] - 0.06;
                    c.spawn(PbrBundle {
                        mesh: post.clone(),
                        material: frame_mat.clone(),
                        transform: Transform::from_xyz(dx * inset, 0.0, dz * inset),
                        ..default()
                    });
                }
                // 四面木板条：+z 面 / -z 面（沿 x 铺开），+x 面 / -x 面需绕 y 轴转 90°
                let side_off = CRATE_HALF[2] - 0.03;
                let side_off_x = CRATE_HALF[0] - 0.03;
                for y in slat_ys {
                    // 前后（z 面）
                    for z in [side_off, -side_off] {
                        c.spawn(PbrBundle {
                            mesh: slat.clone(),
                            material: plank_mat.clone(),
                            transform: Transform::from_xyz(0.0, y, z),
                            ..default()
                        });
                    }
                    // 左右（x 面，绕 y 转 90°）
                    for x in [side_off_x, -side_off_x] {
                        c.spawn(PbrBundle {
                            mesh: slat.clone(),
                            material: plank_mat.clone(),
                            transform: Transform::from_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2))
                                .with_translation(Vec3::new(x, y, 0.0)),
                            ..default()
                        });
                    }
                }
                // 平顶箱盖
                c.spawn(PbrBundle {
                    mesh: lid.clone(),
                    material: lid_mat.clone(),
                    transform: Transform::from_xyz(0.0, CRATE_HALF[1], 0.0),
                    ..default()
                });
                c.spawn(PbrBundle {
                    mesh: beacon_mesh.clone(),
                    material: beacon_mat.clone(),
                    transform: Transform::from_xyz(0.0, CRATE_HALF[1] + 0.25, 0.0),
                    ..default()
                });
            });
        bevy::log::info!("[crate {}] spawned at ({}, {})", i + 1, pos.x, pos.z);
    }
}

/// 构建物资箱面板（隐藏，由 station_system 在打开时控制显隐）
pub(crate) fn setup_crate_ui(mut commands: Commands) {
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
            visibility: Visibility::Hidden,
            ..default()
        },
        CrateUIRoot,
    )).with_children(|root| {
        root.spawn(NodeBundle {
            style: Style {
                width: Val::Px(430.0),
                height: Val::Auto,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(10.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.06, 0.06, 0.08, 0.95)),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            ..default()
        }).with_children(|panel| {
            panel.spawn(TextBundle {
                text: Text::from_section(
                    "物资箱 · 3×4 随机战利品",
                    TextStyle { font_size: 20.0, color: Color::srgb(0.9, 0.9, 0.9), ..default() }
                ),
                ..default()
            });
            panel.spawn(NodeBundle {
                style: Style { width: Val::Percent(100.0), height: Val::Px(2.0), ..default() },
                background_color: BackgroundColor(Color::srgba(0.4, 0.4, 0.5, 0.3)),
                ..default()
            });
            // 3×4 战利品格
            panel.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(10.0),
                    row_gap: Val::Px(10.0),
                    margin: UiRect::top(Val::Px(6.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                ..default()
            }).with_children(|grid| {
                for i in 0..CRATE_CELLS {
                    grid.spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Px((390.0 - 20.0) / 3.0),
                                height: Val::Px(48.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::srgba(0.14, 0.14, 0.16, 0.95)),
                            border_color: BorderColor(Color::srgba(0.3, 0.3, 0.35, 0.6)),
                            border_radius: BorderRadius::all(Val::Px(4.0)),
                            ..default()
                        },
                        CrateCellUI(i),
                        Interaction::default(),
                    )).with_children(|cell| {
                        cell.spawn((
                            TextBundle {
                                text: Text::from_section(
                                    " ",
                                    TextStyle { font_size: 12.0, color: Color::srgb(0.85, 0.85, 0.85), ..default() }
                                ),
                                style: Style { width: Val::Percent(96.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                                ..default()
                            },
                            CrateCellText(i),
                        ));
                    });
                }
            });
            // 分隔线
            panel.spawn(NodeBundle {
                style: Style { width: Val::Percent(100.0), height: Val::Px(2.0), margin: UiRect::top(Val::Px(2.0)), ..default() },
                background_color: BackgroundColor(Color::srgba(0.4, 0.4, 0.5, 0.3)),
                ..default()
            });
            // 背包落点
            panel.spawn(TextBundle {
                text: Text::from_section(
                    "背包（拖到此处领取 · 满格需先在 Tab 背包用掉）",
                    TextStyle { font_size: 12.0, color: Color::srgb(0.5, 0.5, 0.55), ..default() }
                ),
                style: Style { margin: UiRect::top(Val::Px(4.0)), ..default() },
                ..default()
            });
            panel.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(10.0),
                    row_gap: Val::Px(10.0),
                    margin: UiRect::top(Val::Px(4.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                ..default()
            }).with_children(|slots| {
                for i in 0..CRATE_SLOTS {
                    slots.spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Px(86.0),
                                height: Val::Px(52.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::srgba(0.14, 0.14, 0.16, 0.95)),
                            border_color: BorderColor(Color::srgba(0.3, 0.3, 0.35, 0.6)),
                            border_radius: BorderRadius::all(Val::Px(4.0)),
                            ..default()
                        },
                        CrateSlotUI(i),
                        Interaction::default(),
                    )).with_children(|slot| {
                        slot.spawn((
                            TextBundle {
                                text: Text::from_section(
                                    " ",
                                    TextStyle { font_size: 11.0, color: Color::srgb(0.85, 0.85, 0.85), ..default() }
                                ),
                                style: Style { width: Val::Percent(96.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                                ..default()
                            },
                            CrateSlotText(i),
                        ));
                    });
                }
            });
            panel.spawn((
                TextBundle {
                    text: Text::from_section(
                        "",
                        TextStyle { font_size: 13.0, color: Color::srgb(0.6, 0.9, 0.6), ..default() }
                    ),
                    ..default()
                },
                CrateHintText,
            ));
            panel.spawn(TextBundle {
                text: Text::from_section(
                    "拖拽或 Shift+左键 领取战利品 · F / Esc 关闭",
                    TextStyle { font_size: 12.0, color: Color::srgb(0.5, 0.5, 0.55), ..default() }
                ),
                ..default()
            });
        });
        // 拖拽幻影（相对屏幕全屏定位，默认隐藏）
        root.spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.2, 0.2, 0.25, 0.9)),
                border_radius: BorderRadius::all(Val::Px(6.0)),
                visibility: Visibility::Hidden,
                ..default()
            },
            CrateGhost,
        )).with_children(|g| {
            g.spawn((
                TextBundle {
                    text: Text::from_section("", TextStyle { font_size: 13.0, color: Color::WHITE, ..default() }),
                    ..default()
                },
                CrateGhostText,
            ));
        });
    });
}

/// 把战利品送入背包：弹药入池、武器替换当前手持、其余占一格。
/// 结算前先校验容量，失败则把物品放回格子（不消耗）。
fn try_transfer(
    crate_loot: &mut Vec<Option<PickupItem>>,
    cell: usize,
    inventory: &mut Inventory,
    weapon_slot: &WeaponSlot,
) -> Result<String, String> {
    let Some(item) = crate_loot.get_mut(cell).and_then(|o| o.take()) else {
        return Err("已取走".to_string());
    };
    let full = match &item.item_type {
        PickupType::Ammo { .. } | PickupType::Weapon { .. } => false,
        _ => inventory.items.len() >= inventory.max_slots,
    };
    if full {
        let label = item.name.clone();
        crate_loot[cell] = Some(item);
        return Err(format!("背包已满（{}）", label));
    }
    let msg = match &item.item_type {
        PickupType::Ammo { amount } => {
            inventory.ammo_pool += *amount;
            format!("已领取：{}（入弹药池）", item.name)
        }
        PickupType::Weapon { element } => {
            inventory.weapons[weapon_slot.current] = WeaponData::from_profile(&crate::operator::rifle_profile(*element));
            format!("已领取武器：{}（替换当前手持）", item.name)
        }
        _ => {
            inventory.items.push(item.clone());
            format!("已领取：{}", item.name)
        }
    };
    Ok(msg)
}

/// 刷新战利品格/背包落点文本与悬停高亮、提示文案。
pub(crate) fn crate_ui_system(
    win: Res<CrateWindow>,
    mut cell_texts: Query<(&CrateCellUI, &mut Text), (Without<CrateSlotUI>, Without<CrateHintText>)>,
    mut cell_bgs: Query<(&CrateCellUI, &Interaction, &mut BorderColor, &mut BackgroundColor), Without<CrateSlotUI>>,
    mut slot_texts: Query<(&CrateSlotUI, &mut Text), (Without<CrateCellUI>, Without<CrateHintText>)>,
    mut slot_bgs: Query<(&CrateSlotUI, &Interaction, &mut BorderColor, &mut BackgroundColor), Without<CrateCellUI>>,
    mut hint: Query<&mut Text, (With<CrateHintText>, Without<CrateCellUI>, Without<CrateSlotUI>)>,
    crates: Query<&SupplyCrate>,
    player: Query<&Inventory, With<Player>>,
) {
    let Some(entity) = win.crate_entity else { return };
    let Ok(crate_comp) = crates.get(entity) else { return };
    let Ok(inventory) = player.get_single() else { return };

    for (cell, mut text) in cell_texts.iter_mut() {
        match crate_comp.loot.get(cell.0).cloned().flatten() {
            Some(item) => {
                text.sections[0].value = item.name;
                text.sections[0].style.color = pickup_text_color(&item.item_type);
            }
            None => {
                text.sections[0].value = String::new();
                text.sections[0].style.color = Color::srgb(0.4, 0.4, 0.4);
            }
        }
    }
    for (_cell, interaction, mut border, mut bg) in cell_bgs.iter_mut() {
        if *interaction == Interaction::Hovered {
            border.0 = Color::srgba(1.0, 0.8, 0.25, 0.95);
            bg.0 = Color::srgba(0.26, 0.27, 0.32, 0.95);
        } else {
            border.0 = Color::srgba(0.3, 0.3, 0.35, 0.6);
            bg.0 = Color::srgba(0.14, 0.14, 0.16, 0.95);
        }
    }
    for (slot, mut text) in slot_texts.iter_mut() {
        if let Some(item) = inventory.items.get(slot.0) {
            text.sections[0].value = item.name.clone();
            text.sections[0].style.color = pickup_text_color(&item.item_type);
        } else {
            text.sections[0].value = " ".to_string();
            text.sections[0].style.color = Color::srgb(0.4, 0.4, 0.4);
        }
    }
    for (_slot, interaction, mut border, mut bg) in slot_bgs.iter_mut() {
        if *interaction == Interaction::Hovered {
            border.0 = Color::srgba(1.0, 0.8, 0.25, 0.95);
            bg.0 = Color::srgba(0.26, 0.27, 0.32, 0.95);
        } else {
            border.0 = Color::srgba(0.3, 0.3, 0.35, 0.6);
            bg.0 = Color::srgba(0.14, 0.14, 0.16, 0.95);
        }
    }
    // 提示：若背包已满则告警
    if let Ok(mut text) = hint.get_single_mut() {
        text.sections[0].value = if inventory.items.len() >= inventory.max_slots {
            "背包已满，先在 Tab 背包用掉物资".to_string()
        } else {
            String::new()
        };
    }
}

/// 拖拽 + Shift 快移领取战利品，并驱动幻影跟随光标。
pub(crate) fn crate_drag_system(
    win: Res<CrateWindow>,
    mut drag: ResMut<CrateDrag>,
    mouse: Res<ButtonInput<MouseButton>>,
    shift: Res<ButtonInput<KeyCode>>,
    mut crates: Query<&mut SupplyCrate>,
    mut player: Query<(&mut Inventory, &WeaponSlot), With<Player>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut ghost: Query<(&mut Visibility, &mut Style), With<CrateGhost>>,
    mut ghost_text: Query<&mut Text, (With<CrateGhostText>, Without<CrateHintText>)>,
    mut hint: Query<&mut Text, (With<CrateHintText>, Without<CrateGhostText>)>,
    cell_interactions: Query<(&CrateCellUI, &Interaction), Without<CrateSlotUI>>,
    slot_interactions: Query<(&CrateSlotUI, &Interaction), Without<CrateCellUI>>,
) {
    let Some(entity) = win.crate_entity else { return };
    let Ok(mut crate_comp) = crates.get_mut(entity) else { return };
    let Ok((mut inventory, weapon_slot)) = player.get_single_mut() else { return };
    let loot = &mut crate_comp.loot;

    let mut report: Option<String> = None;
    let cursor = window_query.get_single().ok().and_then(|w| w.cursor_position());

    // ---- 幻影跟随光标 ----
    if drag.source.is_some() {
        if let Ok((mut vis, mut style)) = ghost.get_single_mut() {
            if let Some(c) = cursor {
                style.left = Val::Px(c.x - CRATE_GHOST_SNAP);
                style.top = Val::Px(c.y - 12.0);
            }
            *vis = Visibility::Visible;
        }
    } else if let Ok((mut vis, _)) = ghost.get_single_mut() {
        *vis = Visibility::Hidden;
    }

    // Shift+左键快移：直接从格子入包
    if drag.source.is_none() && shift.pressed(KeyCode::ShiftLeft) && mouse.just_pressed(MouseButton::Left) {
        if let Some((cell, _)) = pressed_or_hovered_cell(&cell_interactions) {
            report = Some(match try_transfer(loot, cell, &mut inventory, weapon_slot) {
                Ok(m) => m,
                Err(e) => e,
            });
        }
        if let Ok(mut text) = hint.get_single_mut() {
            if let Some(message) = &report {
                text.sections[0].value = message.clone();
            }
        }
        return;
    }

    // ---- 拖拽开始：左键按下（非 Shift）且有物资的格子 ----
    if drag.source.is_none() && mouse.just_pressed(MouseButton::Left) && !shift.pressed(KeyCode::ShiftLeft) {
        if let Some((cell, _)) = cell_interactions.iter()
            .find(|(_c, interaction)| **interaction == Interaction::Pressed)
            .filter(|(cell, _)| matches!(loot.get(cell.0), Some(Some(_))))
        {
            drag.source = Some(cell.0);
            // 初始化幻影文本
            if let Ok(mut text) = ghost_text.get_single_mut() {
                text.sections[0].value = loot[cell.0].as_ref().map(|i| i.name.clone()).unwrap_or_default();
            }
        }
    }

    // ---- 拖拽中：松开结算 ----
    if let Some(source) = drag.source {
        if mouse.just_released(MouseButton::Left) {
            // 悬停任意背包落点格才结算
            let dropped = slot_interactions.iter()
                .find(|(_, interaction)| **interaction == Interaction::Hovered)
                .map(|(slot, _)| slot.0);
            if dropped.is_some() {
                report = Some(match try_transfer(loot, source, &mut inventory, weapon_slot) {
                    Ok(m) => m,
                    Err(e) => e,
                });
            }
            drag.source = None;
            if let Ok((mut vis, _)) = ghost.get_single_mut() {
                *vis = Visibility::Hidden;
            }
            if let Ok(mut text) = hint.get_single_mut() {
                if let Some(message) = &report {
                    text.sections[0].value = message.clone();
                }
            }
        }
    }
}

fn pressed_or_hovered_cell(cells: &Query<(&CrateCellUI, &Interaction), Without<CrateSlotUI>>) -> Option<(usize, usize)> {
    cells.iter()
        .find(|(_c, interaction)| **interaction == Interaction::Hovered || **interaction == Interaction::Pressed)
        .map(|(cell, _)| (cell.0, cell.0))
}