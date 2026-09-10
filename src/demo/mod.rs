//! Cute Of Duty 1: Simple - 3D Pixel FPS Demo
//!
//! Tech: Bevy 0.14 + Voxel Rendering
//! Style: Minecraft Steve model inspired
//!
//! Controls (FPS Standard):
//! - Mouse: Free-look (horizontal + vertical)
//! - WASD: Move
//! - Space: Jump
//! - Shift: Sprint
//! - 1 / 2: Switch between the two primary weapons
//! - Q / E: Operator skills (each operator has a unique mechanic: burn DoT / freeze / dash / poison zone)
//! - LMB: Shoot
//! - F: Open unified interact menu (pickups + nearby stations); scroll to select, F to confirm
//! - Tab: Open/close backpack (two primary weapons, ammo pool, supplies)
//! - R (hover item in backpack): Use item
//! - 3: Quick-use recovery item / hold to open radial wheel
//! - 4: Quick-use tactical item / hold to open radial wheel
//! - Esc: Close backpack / station panel, release mouse otherwise

use std::collections::HashMap;
use bevy::prelude::*;
use bevy::asset::AssetPlugin;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::pbr::NotShadowCaster;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use crate::element::{ElementConfig, ElementSystem, ElementType, EntityElementState, ReactionResult};
use crate::map::training;
use crate::map::{GlowKind, GlowSpec, MapLayout, MaterialKind, PickupKind, PickupSpec, Prop, Shape, StationKind, StationSpec, TargetSpec};
use crate::operator::{rifle_profile, roster, SkillKind};
use crate::model::{
    build_yanhu, mat_emissive, mat_voxel, operator_model_swap_system, palette, ready_timer,
    yanhu_action_system, OperatorAccent, Player, OperatorState, PlayerAimGun, PlayerCamera,
    PlayerHeadPivot, PlayerModelRoot, PlayerMovement,
};

/// 核心元素系统的Bevy资源包装
///
/// 核心库不依赖bevy，Resource trait由Demo侧的newtype提供
#[derive(Resource)]
struct ElementalSystem(ElementSystem);

pub fn run() {
    // 加载核心元素配置表（与cod1共用核心库统一加载器，自动定位项目根目录）
    let element_config = load_element_config();
    let element_system = ElementSystem::new(element_config);

    // assets 按项目根目录解析（而非 CWD），从任意目录启动都不丢资源
    let asset_file_path = crate::config::project_root()
        .map(|root| root.join("assets").to_string_lossy().into_owned());

    let mut plugins = DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Cute Of Duty 1 - 3D Pixel FPS".to_string(),
            resolution: (1280.0_f32, 720.0_f32).into(),
            position: WindowPosition::Centered(MonitorSelection::Primary),
            resizable: false,
            ..default()
        }),
        ..default()
    });
    if let Some(path) = asset_file_path {
        plugins = plugins.set(AssetPlugin {
            file_path: path,
            ..default()
        });
    }

    App::new()
        .add_plugins(plugins)
        .insert_resource(ElementalSystem(element_system))
        .init_state::<AppState>()
        .init_resource::<InputState>()
        .init_resource::<NearbyInteract>()
        .init_resource::<WheelState>()
        .init_resource::<KillStats>()
        .init_resource::<OpenStation>()
        .init_resource::<GameSettings>()
        .init_resource::<SelectedMode>()
        .init_resource::<SelectedCategory>()
        .init_resource::<PauseMenu>()
        .init_resource::<HeldGrenade>()
        .add_event::<KillEvent>()
        .insert_resource(ClearColor(Color::srgb(0.12, 0.14, 0.18)))
        .insert_resource(AmbientLight {
            color: Color::srgb(0.9, 0.92, 1.0),
            brightness: 0.55,
        })
        // 前端流程：加载页 → 主菜单 → 选择训练场进入游戏。
        // 世界与 HUD 的构建全部延迟到 OnEnter(InGame)，
        // 字体必须在 Startup 最先加载（前端界面文本同样依赖 CJK 字形）。
        .add_systems(Startup, (load_cjk_font, log_element_config, setup_menu_camera, setup_loading_screen).chain())
        .add_systems(Update, loading_tick.run_if(in_state(AppState::Loading)))
        .add_systems(OnExit(AppState::Loading), despawn_loading_screen)
        .add_systems(OnEnter(AppState::MainMenu), (setup_menu_camera, setup_main_menu, release_cursor))
        .add_systems(Update, (main_menu_interaction, main_menu_style).chain().run_if(in_state(AppState::MainMenu)))
        .add_systems(OnExit(AppState::MainMenu), despawn_main_menu)
        .add_systems(OnEnter(AppState::InGame), (
            despawn_menu_camera,
            setup_world,
            setup_hud,
            setup_minimap,
            setup_inventory_hud,
            setup_station_ui,
            setup_item_wheel,
            grab_cursor,
        ))
        // 返回主界面：清空全部游戏实体与游戏态资源，下次进入时全量重建
        .add_systems(OnExit(AppState::InGame), teardown_game)
        // /~ 暂停菜单（资源门控，不占状态机）
        .add_systems(Update, pause_toggle.run_if(in_state(AppState::InGame)))
        .add_systems(Update, pause_menu_interaction.run_if(in_state(AppState::InGame)).run_if(pause_open))
        .add_systems(Update, settings_apply_fov.run_if(in_state(AppState::InGame)))
        .add_systems(Update, settings_apply_ambient)
        // 越肩 SpringArm：输入/瞄准态 → aim_lerp → 相机装配 → 投掷读视线，按此顺序
        .add_systems(Update, (
            aim_system.before(aim_lerp_system),
            aim_lerp_system.before(aim_rig_system),
            fps_controller.before(aim_rig_system),
            grenade_throw_system.after(aim_rig_system),
        ).run_if(in_state(AppState::InGame)).run_if(not(pause_open)))
        .add_systems(Update, (
            // 自由光标切换须先于背包开关读 Esc：背包关闭路径会消费 Esc（见 inventory_toggle）
            cursor_grab_toggle.before(inventory_toggle),
            fps_controller,
            operator_model_swap_system.before(aim_rig_system),
            aim_rig_system,
            yanhu_action_system.after(aim_rig_system),
            weapon_switch,
            reload_system,
            shooting_system,
            skill_system,
            operator_cooldown_tick,
            bullet_cleanup,
            grenade_physics,
            zone_tick_system,
            explosion_expand,
            damage_particle_system,
            damage_popup_system,
            hit_flash_system,
            minimap_update_system,
        ).run_if(in_state(AppState::InGame)).run_if(not(pause_open)))
        .add_systems(Update, (
            target_dummy_logic,
            moving_target_logic,
            crosshair_hit_feedback,
            hud_update_system,
            hud_vitals_text_system,
            low_ammo_blink,
            screen_edge_glow,
            floating_reaction_text,
            respawn_flash_system,
            inventory_toggle,
            interact_detection_system,
            interact_scroll_system,
            interact_menu_update,
            interact_execute_system,
            inventory_ui_update,
            inventory_item_use_system,
            item_wheel_system,
            hud_item_slots_system,
            kill_feed_system,
        ).run_if(in_state(AppState::InGame)).run_if(not(pause_open)))
        .add_systems(Update, (
            // 站点面板读取交互菜单的选中条目：必须在拾取执行（消费 F）之后
            station_system.after(cursor_grab_toggle).after(interact_execute_system),
            supply_station_click_system,
            operator_station_click_system,
            supply_ui_update_system,
            operator_ui_update_system,
            hud_operator_name_system,
        ).run_if(in_state(AppState::InGame)).run_if(not(pause_open)))
        .run();
}

// =============================================================================
// Game Flow —— 加载页 → 主菜单 → 选择训练场 → 进入游戏
// =============================================================================

/// 前端流程状态机：启动先过加载页，主菜单选择训练场后才搭建游戏世界
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
enum AppState {
    #[default]
    Loading,
    MainMenu,
    InGame,
}

/// 加载页/主菜单专用 2D 相机（游戏世界就绪前 bevy_ui 没有相机无法渲染）
#[derive(Resource)]
struct MenuCamera(Entity);

/// 加载页可更新节点的句柄
#[derive(Resource)]
struct LoadingScreen {
    root: Entity,
    step: Entity,
    fill: Entity,
    percent: Entity,
}

/// 加载计时器
#[derive(Resource)]
struct LoadingTimer(Timer);

/// 主菜单各可更新节点句柄（进入游戏时随根节点统一销毁）
#[derive(Resource)]
struct MainMenuUi {
    root: Entity,
    /// 右侧 60% 模式面板（"切换模式"呼出/收起）
    mode_panel: Entity,
    /// 齿轮呼出的设置浮层
    settings_overlay: Entity,
    /// 设置浮层的全屏压暗层（与浮层同步显隐）
    settings_backdrop: Entity,
    /// 右下角"当前模式"状态行
    status_text: Entity,
}

/// 菜单刚打开时的输入保护期：拦截启动瞬间残留的按键/按下状态误触按钮
#[derive(Resource)]
struct MenuGrace(Timer);

/// 可交互菜单按钮（无此标记的按钮为占位项，不参与交互）
#[derive(Component)]
struct MenuButton;

/// 退出按钮
#[derive(Component)]
struct QuitButton;

/// 游戏模式 id：面板选中项 + "开始游戏"的入口分发
#[derive(Clone, Copy, PartialEq, Debug, Default)]
enum GameModeId {
    #[default]
    Training,
    Campaign,
    Evacuation,
}

/// 模式元数据：名称 / 描述 / 分类下标 / 是否已开放
struct ModeSpec {
    id: GameModeId,
    name: &'static str,
    desc: &'static str,
    category: usize,
    available: bool,
}

/// 全部游戏模式（分类下标对应 MODE_CATEGORIES，0 为"全部"）
const GAME_MODES: [ModeSpec; 3] = [
    ModeSpec { id: GameModeId::Training, name: "训练场", desc: "单人 · 射击与元素反应演练", category: 1, available: true },
    ModeSpec { id: GameModeId::Campaign, name: "战役模式", desc: "章节化 PVE 战役", category: 2, available: false },
    ModeSpec { id: GameModeId::Evacuation, name: "多人撤离", desc: "组队搜刮 · 带装撤离", category: 3, available: false },
];

/// 模式分类标签，0 号为"全部"（不过滤）
const MODE_CATEGORIES: [&str; 4] = ["全部", "演练", "战役", "撤离"];

/// 当前选中的游戏模式（会话内保留，返回主界面后记住上次选择）
#[derive(Resource, Default)]
struct SelectedMode(GameModeId);

/// 当前选中的分类下标
#[derive(Resource, Default)]
struct SelectedCategory(usize);

/// "切换模式"按钮：呼出/收起右侧模式面板
#[derive(Component)]
struct SwitchModeButton;

/// "开始游戏"按钮：进入当前选中的模式
#[derive(Component)]
struct StartGameButton;

/// 右上角齿轮按钮：打开设置浮层
#[derive(Component)]
struct GearButton;

/// 齿轮图标图片节点（悬停变色用）
#[derive(Component)]
struct GearIcon;

/// 设置浮层的"返回"按钮
#[derive(Component)]
struct SettingsCloseButton;

/// 模式分类按钮：按下标过滤模式列表
#[derive(Component)]
struct CategoryButton(usize);

/// 模式面板根节点标记（样式系统读取其显隐，联动模式行可见性）
#[derive(Component)]
struct ModePanelRoot;

/// 模式列表中的一行：点击选中该模式
#[derive(Component)]
struct ModeRow(GameModeId);

/// 右下角"当前模式"状态行（选中变化/非法进入时更新）
#[derive(Component)]
struct StatusText;

/// 加载分步文案：真实初始化很轻，按统一节奏展示各子系统就位
const LOADING_STEPS: [&str; 5] = [
    "初始化引擎核心…",
    "加载元素反应配置…",
    "构建训练场地图…",
    "准备干员与武器档案…",
    "校准 HUD 与小地图…",
];

const LOADING_DURATION_SECS: f32 = 2.8;

fn menu_accent() -> Color {
    Color::srgb(0.30, 0.78, 1.0)
}

/// 可交互按钮的两态配色（构建与交互系统共用，保证视觉一致）
fn menu_button_palette(hovered: bool) -> (BackgroundColor, BorderColor) {
    if hovered {
        (
            BackgroundColor(Color::srgba(0.14, 0.20, 0.28, 0.98)),
            BorderColor(menu_accent()),
        )
    } else {
        (
            BackgroundColor(Color::srgba(0.10, 0.14, 0.20, 0.95)),
            BorderColor(Color::srgb(0.22, 0.28, 0.36)),
        )
    }
}

fn setup_menu_camera(menu: Option<Res<MenuCamera>>, mut commands: Commands) {
    // 幂等：首次启动由 Startup 链创建；从游戏返回主菜单时这里重建
    if menu.is_some() {
        return;
    }
    let id = commands.spawn(Camera2dBundle::default()).id();
    commands.insert_resource(MenuCamera(id));
}

fn setup_loading_screen(mut commands: Commands) {
    let mut step_id = Entity::PLACEHOLDER;
    let mut pct_id = Entity::PLACEHOLDER;
    let mut fill_id = Entity::PLACEHOLDER;

    let root = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgb(0.04, 0.06, 0.09)),
            ..default()
        })
        .with_children(|root| {
            root.spawn(TextBundle::from_section(
                "CUTE OF DUTY",
                TextStyle { font_size: 84.0, color: Color::srgb(0.92, 0.95, 1.0), ..default() },
            ));
            root.spawn(TextBundle::from_section(
                "SIMPLE · 像素战术撤离 · PRE-ALPHA",
                TextStyle { font_size: 20.0, color: Color::srgb(0.55, 0.62, 0.72), ..default() },
            ));
            root.spawn(NodeBundle {
                style: Style { height: Val::Px(56.0), ..default() },
                ..default()
            });
            step_id = root.spawn(TextBundle::from_section(
                LOADING_STEPS[0],
                TextStyle { font_size: 18.0, color: Color::srgb(0.70, 0.78, 0.88), ..default() },
            )).id();
        })
        .id();

    commands.entity(root).with_children(|root| {
        // 进度条：容器 + 百分比宽度的填充条
        root.spawn(NodeBundle {
            style: Style {
                width: Val::Px(520.0),
                height: Val::Px(16.0),
                padding: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::srgb(0.10, 0.13, 0.18)),
            ..default()
        }).with_children(|bar| {
            fill_id = bar.spawn(NodeBundle {
                style: Style { width: Val::Percent(0.0), height: Val::Percent(100.0), ..default() },
                background_color: BackgroundColor(menu_accent()),
                ..default()
            }).id();
        });
        pct_id = root.spawn(TextBundle::from_section(
            "0%",
            TextStyle { font_size: 15.0, color: menu_accent(), ..default() },
        )).id();
        root.spawn(TextBundle::from_section(
            "首次启动需要编译渲染管线，请稍候 · 按任意键跳过",
            TextStyle { font_size: 13.0, color: Color::srgb(0.40, 0.46, 0.55), ..default() },
        ));
    });

    commands.insert_resource(LoadingScreen { root, step: step_id, fill: fill_id, percent: pct_id });
    commands.insert_resource(LoadingTimer(Timer::from_seconds(LOADING_DURATION_SECS, TimerMode::Once)));
}

fn loading_tick(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut timer: ResMut<LoadingTimer>,
    screen: Res<LoadingScreen>,
    mut texts: Query<&mut Text>,
    mut styles: Query<&mut Style>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    timer.0.tick(time.delta());
    // 任意键/点击跳过加载动画
    if keys.get_just_pressed().next().is_some() || mouse.get_just_pressed().next().is_some() {
        let total = timer.0.duration();
        timer.0.set_elapsed(total);
    }

    let t = (timer.0.elapsed_secs() / timer.0.duration().as_secs_f32()).clamp(0.0, 1.0);
    let step_index = ((t * LOADING_STEPS.len() as f32) as usize).min(LOADING_STEPS.len() - 1);

    if let Ok(mut text) = texts.get_mut(screen.step) {
        text.sections[0].value = LOADING_STEPS[step_index].to_string();
    }
    if let Ok(mut style) = styles.get_mut(screen.fill) {
        style.width = Val::Percent(t * 100.0);
    }
    if let Ok(mut text) = texts.get_mut(screen.percent) {
        text.sections[0].value = format!("{:.0}%", t * 100.0);
    }

    if timer.0.finished() {
        next_state.set(AppState::MainMenu);
    }
}

fn despawn_loading_screen(mut commands: Commands, screen: Res<LoadingScreen>) {
    // bevy 0.14 的 despawn() 不递归销毁子节点，UI 树必须用 despawn_recursive
    commands.entity(screen.root).despawn_recursive();
    commands.remove_resource::<LoadingScreen>();
    commands.remove_resource::<LoadingTimer>();
}

fn setup_main_menu(
    mut commands: Commands,
    assets: Res<AssetServer>,
    settings: Res<GameSettings>,
    selected: Res<SelectedMode>,
) {
    let mut quit_btn = Entity::PLACEHOLDER;
    let mut switch_btn = Entity::PLACEHOLDER;
    let mut start_btn = Entity::PLACEHOLDER;
    let mut close_btn = Entity::PLACEHOLDER;
    let mut status_text = Entity::PLACEHOLDER;
    let mut mode_panel = Entity::PLACEHOLDER;
    let mut settings_overlay = Entity::PLACEHOLDER;
    let mut settings_backdrop = Entity::PLACEHOLDER;

    let gear_texture: Handle<Image> = assets.load("ui/gear_icon.png");
    let (base_bg, base_border) = menu_button_palette(false);

    let root = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgb(0.04, 0.06, 0.09)),
            ..default()
        })
        .id();

    commands.entity(root).with_children(|root| {
        // 左上角角标
        root.spawn(TextBundle {
            text: Text::from_section(
                "PRE-ALPHA v0.2.1",
                TextStyle { font_size: 14.0, color: Color::srgb(0.42, 0.48, 0.56), ..default() },
            ),
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(18.0),
                left: Val::Px(24.0),
                ..default()
            },
            ..default()
        });
        root.spawn(TextBundle {
            text: Text::from_section(
                "CUTE OF DUTY 1: SIMPLE",
                TextStyle { font_size: 14.0, color: Color::srgb(0.42, 0.48, 0.56), ..default() },
            ),
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(40.0),
                left: Val::Px(24.0),
                ..default()
            },
            ..default()
        });

        // 左侧标题区（40% 宽：模式面板展开时标题仍完整可见）
        root.spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(40.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            ..default()
        }).with_children(|left| {
            left.spawn(TextBundle::from_section(
                "CUTE OF DUTY",
                TextStyle { font_size: 64.0, color: Color::srgb(0.92, 0.95, 1.0), ..default() },
            ));
            left.spawn(TextBundle::from_section(
                "SIMPLE · 像素战术撤离",
                TextStyle { font_size: 18.0, color: menu_accent(), ..default() },
            ));
            left.spawn(TextBundle::from_section(
                "点右下角「切换模式」选择作战模式",
                TextStyle { font_size: 14.0, color: Color::srgb(0.42, 0.48, 0.56), ..default() },
            ));
        });

        // 底部左侧：操作提示 + 退出
        root.spawn(TextBundle {
            text: Text::from_section(
                "WASD 移动 · 左键射击 · 右键越肩瞄准 · R 换弹\nTab/Esc 背包 · Q/E 干员技能 · F 互动（拾取/站点）",
                TextStyle { font_size: 13.0, color: Color::srgb(0.42, 0.48, 0.56), ..default() },
            ),
            style: Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(80.0),
                left: Val::Px(24.0),
                ..default()
            },
            ..default()
        });
        root.spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(24.0),
                bottom: Val::Px(24.0),
                ..default()
            },
            ..default()
        }).with_children(|wrap| {
            quit_btn = spawn_action_button(wrap, "退 出 游 戏", 170.0, 44.0, 18.0);
        });

        // 右下角：当前模式状态行 + 切换模式/开始游戏（ZIndex 压在模式面板之上）
        root.spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                right: Val::Px(24.0),
                bottom: Val::Px(24.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexEnd,
                row_gap: Val::Px(10.0),
                ..default()
            },
            z_index: ZIndex::Global(10),
            ..default()
        }).with_children(|col| {
            status_text = col.spawn((
                TextBundle::from_section(
                    format!("当前模式：{}", game_mode_spec(selected.0).name),
                    TextStyle { font_size: 15.0, color: menu_accent(), ..default() },
                ),
                StatusText,
            )).id();
            col.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(12.0),
                    ..default()
                },
                ..default()
            }).with_children(|row| {
                switch_btn = spawn_action_button(row, "切 换 模 式", 176.0, 54.0, 20.0);
                start_btn = spawn_action_button(row, "开 始 游 戏", 216.0, 54.0, 20.0);
            });
        });

        // 右上角齿轮：打开设置浮层（ZIndex 压在模式面板之上）
        root.spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(16.0),
                    right: Val::Px(24.0),
                    width: Val::Px(46.0),
                    height: Val::Px(46.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                background_color: base_bg,
                border_color: base_border,
                border_radius: BorderRadius::all(Val::Px(6.0)),
                z_index: ZIndex::Global(10),
                ..default()
            },
            Interaction::default(),
            MenuButton,
            GearButton,
        )).with_children(|btn| {
            btn.spawn((
                NodeBundle {
                    style: Style { width: Val::Px(26.0), height: Val::Px(26.0), ..default() },
                    ..default()
                },
                UiImage::new(gear_texture.clone()),
                GearIcon,
            ));
        });

        // 右侧 60% 模式面板："切换模式"呼出/收起
        mode_panel = root.spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(60.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect {
                        left: Val::Px(44.0),
                        right: Val::Px(44.0),
                        top: Val::Px(32.0),
                        bottom: Val::Px(110.0),
                    },
                    border: UiRect { left: Val::Px(2.0), ..default() },
                    row_gap: Val::Px(14.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.05, 0.07, 0.11, 0.985)),
                border_color: BorderColor(menu_accent()),
                visibility: Visibility::Hidden,
                ..default()
            },
            ModePanelRoot,
        )).with_children(|panel| {
            panel.spawn(TextBundle::from_section(
                "选 择 作 战 模 式",
                TextStyle { font_size: 30.0, color: Color::srgb(0.92, 0.95, 1.0), ..default() },
            ));
            panel.spawn(TextBundle::from_section(
                "按分类筛选 · 选中后点右下角「开始游戏」",
                TextStyle { font_size: 14.0, color: Color::srgb(0.55, 0.62, 0.72), ..default() },
            ));
            panel.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(28.0),
                    flex_grow: 1.0,
                    ..default()
                },
                ..default()
            }).with_children(|body| {
                // 分类列：0 号"全部"不过滤，其余按下标过滤模式行
                body.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        width: Val::Px(150.0),
                        ..default()
                    },
                    ..default()
                }).with_children(|cats| {
                    cats.spawn(TextBundle::from_section(
                        "分 类",
                        TextStyle { font_size: 14.0, color: Color::srgb(0.55, 0.62, 0.72), ..default() },
                    ));
                    for (index, name) in MODE_CATEGORIES.iter().enumerate() {
                        cats.spawn((
                            NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(44.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                background_color: base_bg,
                                border_color: base_border,
                                border_radius: BorderRadius::all(Val::Px(4.0)),
                                ..default()
                            },
                            Interaction::default(),
                            CategoryButton(index),
                        )).with_children(|b| {
                            b.spawn(TextBundle::from_section(
                                *name,
                                TextStyle { font_size: 17.0, color: Color::srgb(0.85, 0.89, 0.95), ..default() },
                            ));
                        });
                    }
                });
                // 模式列：一行一个模式（名称+描述 | 状态标签）
                body.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(12.0),
                        flex_grow: 1.0,
                        ..default()
                    },
                    ..default()
                }).with_children(|modes| {
                    modes.spawn(TextBundle::from_section(
                        "游 戏 模 式",
                        TextStyle { font_size: 14.0, color: Color::srgb(0.55, 0.62, 0.72), ..default() },
                    ));
                    for spec in GAME_MODES.iter() {
                        let (name_color, desc_color, tag, tag_color) = if spec.available {
                            (Color::srgb(0.92, 0.95, 1.0), Color::srgb(0.55, 0.62, 0.72), "可 用", menu_accent())
                        } else {
                            (Color::srgb(0.45, 0.50, 0.58), Color::srgb(0.30, 0.35, 0.42), "敬请期待", Color::srgb(0.34, 0.39, 0.46))
                        };
                        modes.spawn((
                            NodeBundle {
                                style: Style {
                                    height: Val::Px(68.0),
                                    justify_content: JustifyContent::SpaceBetween,
                                    align_items: AlignItems::Center,
                                    padding: UiRect {
                                        left: Val::Px(22.0),
                                        right: Val::Px(22.0),
                                        top: Val::Px(0.0),
                                        bottom: Val::Px(0.0),
                                    },
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                background_color: base_bg,
                                border_color: base_border,
                                border_radius: BorderRadius::all(Val::Px(4.0)),
                                ..default()
                            },
                            Interaction::default(),
                            ModeRow(spec.id),
                        )).with_children(|row| {
                            row.spawn(NodeBundle {
                                style: Style {
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(3.0),
                                    ..default()
                                },
                                ..default()
                            }).with_children(|left| {
                                left.spawn(TextBundle::from_section(
                                    spec.name,
                                    TextStyle { font_size: 21.0, color: name_color, ..default() },
                                ));
                                left.spawn(TextBundle::from_section(
                                    spec.desc,
                                    TextStyle { font_size: 13.0, color: desc_color, ..default() },
                                ));
                            });
                            row.spawn(TextBundle::from_section(
                                tag,
                                TextStyle { font_size: 15.0, color: tag_color, ..default() },
                            ));
                        });
                    }
                });
            });
        }).id();

        // 设置浮层（齿轮呼出；ZIndex 盖在模式面板与右下角按钮之上）
        // 压暗层用独立兄弟节点 + 显式 ZIndex：0.14 里带 Global Z 的父节点下，
        // absolute 背景子节点与兄弟子树的遮挡关系不可靠，压暗层必须自己占一层
        settings_backdrop = root.spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.02, 0.60)),
            visibility: Visibility::Hidden,
            z_index: ZIndex::Global(15),
            ..default()
        }).id();
        settings_overlay = root.spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            visibility: Visibility::Hidden,
            z_index: ZIndex::Global(20),
            ..default()
        }).with_children(|overlay| {
            overlay.spawn(TextBundle::from_section(
                "游 戏 设 置",
                TextStyle { font_size: 36.0, color: Color::srgb(0.92, 0.95, 1.0), ..default() },
            ));
            overlay.spawn(NodeBundle {
                style: Style { height: Val::Px(8.0), ..default() },
                ..default()
            });
            spawn_setting_row(overlay, &settings, SettingKind::Sensitivity, "鼠标灵敏度");
            spawn_setting_row(overlay, &settings, SettingKind::Fov, "视野 (FOV)");
            spawn_setting_row(overlay, &settings, SettingKind::Ambient, "环境亮度");
            overlay.spawn(NodeBundle {
                style: Style { height: Val::Px(10.0), ..default() },
                ..default()
            });
            close_btn = spawn_action_button(overlay, "返 回", 200.0, 48.0, 20.0);
        }).id();
    });

    commands.entity(close_btn).insert(SettingsCloseButton);
    commands.entity(quit_btn).insert(QuitButton);
    commands.entity(switch_btn).insert(SwitchModeButton);
    commands.entity(start_btn).insert(StartGameButton);
    commands.insert_resource(MainMenuUi {
        root,
        mode_panel,
        settings_overlay,
        settings_backdrop,
        status_text,
    });
    commands.insert_resource(MenuGrace(Timer::from_seconds(0.4, TimerMode::Once)));
}

/// 右下角/底部小号操作按钮：居中单行标签，构建即挂 Interaction（悬停样式与主菜单统一）
fn spawn_action_button(
    parent: &mut ChildBuilder,
    label: &str,
    width: f32,
    height: f32,
    font_size: f32,
) -> Entity {
    let (bg, border) = menu_button_palette(false);
    parent.spawn((
        NodeBundle {
            style: Style {
                width: Val::Px(width),
                height: Val::Px(height),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            background_color: bg,
            border_color: border,
            border_radius: BorderRadius::all(Val::Px(4.0)),
            ..default()
        },
        Interaction::default(),
        MenuButton,
    )).with_children(|btn| {
        btn.spawn(TextBundle::from_section(
            label,
            TextStyle { font_size, color: Color::srgb(0.92, 0.95, 1.0), ..default() },
        ));
    }).id()
}

/// 构建单个菜单按钮；interactive=false 时为灰态占位（不挂 Interaction，不响应悬停/点击）
fn spawn_menu_button(
    parent: &mut ChildBuilder,
    label: &str,
    desc: &str,
    interactive: bool,
) -> Entity {
    let (bg, border) = if interactive {
        menu_button_palette(false)
    } else {
        (
            BackgroundColor(Color::srgba(0.06, 0.08, 0.11, 0.85)),
            BorderColor(Color::srgb(0.13, 0.16, 0.21)),
        )
    };
    let label_color = if interactive { Color::srgb(0.92, 0.95, 1.0) } else { Color::srgb(0.36, 0.40, 0.46) };
    let desc_color = if interactive { Color::srgb(0.55, 0.62, 0.72) } else { Color::srgb(0.26, 0.30, 0.36) };

    let mut button = parent.spawn(NodeBundle {
        style: Style {
            width: Val::Px(460.0),
            height: Val::Px(64.0),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            padding: UiRect::axes(Val::Px(24.0), Val::Px(0.0)),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        background_color: bg,
        border_color: border,
        border_radius: BorderRadius::all(Val::Px(4.0)),
        ..default()
    });
    button.with_children(|row| {
        row.spawn(TextBundle::from_section(
            label,
            TextStyle { font_size: 26.0, color: label_color, ..default() },
        ));
        row.spawn(TextBundle::from_section(
            desc,
            TextStyle { font_size: 14.0, color: desc_color, ..default() },
        ));
    });

    if interactive {
        button.insert(Interaction::default());
        button.insert(MenuButton);
    }
    button.id()
}

/// 按模式 id 查元数据表
fn game_mode_spec(id: GameModeId) -> &'static ModeSpec {
    GAME_MODES.iter().find(|spec| spec.id == id).expect("未知的游戏模式 id")
}

#[allow(clippy::type_complexity)]
fn main_menu_interaction(
    mut next_state: ResMut<NextState<AppState>>,
    mut app_exit: EventWriter<AppExit>,
    time: Res<Time>,
    mut grace: ResMut<MenuGrace>,
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<GameSettings>,
    mut selected: ResMut<SelectedMode>,
    mut category: ResMut<SelectedCategory>,
    ui: Res<MainMenuUi>,
    mut visibility: Query<&mut Visibility>,
    mut value_texts: Query<(&SettingValueText, &mut Text), Without<StatusText>>,
    mut status_texts: Query<&mut Text, (With<StatusText>, Without<SettingValueText>)>,
    buttons: Query<
        (
            &Interaction,
            Option<&SwitchModeButton>,
            Option<&StartGameButton>,
            Option<&GearButton>,
            Option<&SettingsCloseButton>,
            Option<&QuitButton>,
        ),
        Changed<Interaction>,
    >,
    adjust_buttons: Query<(&SettingAdjust, &Interaction), Changed<Interaction>>,
    category_buttons: Query<(&CategoryButton, &Interaction), Changed<Interaction>>,
    mode_rows: Query<(&ModeRow, &Interaction), Changed<Interaction>>,
) {
    grace.0.tick(time.delta());
    if !grace.0.finished() {
        return;
    }

    // Esc 关闭设置浮层（连同压暗层）
    if keys.just_pressed(KeyCode::Escape) {
        if let Ok(mut vis) = visibility.get_mut(ui.settings_overlay) {
            if matches!(*vis, Visibility::Visible) {
                *vis = Visibility::Hidden;
                if let Ok(mut backdrop_vis) = visibility.get_mut(ui.settings_backdrop) {
                    *backdrop_vis = Visibility::Hidden;
                }
            }
        }
    }
    let settings_open = visibility
        .get(ui.settings_overlay)
        .map_or(false, |vis| matches!(*vis, Visibility::Visible));

    // 按钮动作分发（设置浮层打开时，除"返回"外全部拦截）
    for (interaction, switch, start, gear, close, quit) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if settings_open && close.is_none() {
            continue;
        }
        if close.is_some() {
            if let Ok(mut vis) = visibility.get_mut(ui.settings_overlay) {
                *vis = Visibility::Hidden;
            }
            if let Ok(mut backdrop_vis) = visibility.get_mut(ui.settings_backdrop) {
                *backdrop_vis = Visibility::Hidden;
            }
        } else if switch.is_some() {
            if let Ok(mut vis) = visibility.get_mut(ui.mode_panel) {
                let open = matches!(*vis, Visibility::Visible);
                *vis = if open { Visibility::Hidden } else { Visibility::Visible };
            }
        } else if start.is_some() {
            let spec = game_mode_spec(selected.0);
            if spec.available {
                next_state.set(AppState::InGame);
                return;
            }
            if let Ok(mut text) = status_texts.get_mut(ui.status_text) {
                text.sections[0].value = format!("「{}」尚未开放，敬请期待", spec.name);
            }
        } else if gear.is_some() {
            if let Ok(mut vis) = visibility.get_mut(ui.settings_overlay) {
                *vis = Visibility::Visible;
            }
            if let Ok(mut backdrop_vis) = visibility.get_mut(ui.settings_backdrop) {
                *backdrop_vis = Visibility::Visible;
            }
        } else if quit.is_some() {
            app_exit.send(AppExit::Success);
            return;
        }
    }

    // 分类 / 模式选择（面板内部互斥：设置浮层打开时不响应）
    if !settings_open {
        for (cat, interaction) in &category_buttons {
            if *interaction == Interaction::Pressed {
                category.0 = cat.0;
            }
        }
        for (row, interaction) in &mode_rows {
            if *interaction == Interaction::Pressed {
                selected.0 = row.0;
            }
        }
    }

    // 设置浮层里的步进调节（与暂停菜单共用一套设置行控件）
    let mut adjusted = false;
    for (adjust, interaction) in &adjust_buttons {
        if *interaction == Interaction::Pressed {
            apply_setting_step(&mut settings, adjust.kind, adjust.delta);
            adjusted = true;
        }
    }
    if adjusted {
        for (value, mut text) in value_texts.iter_mut() {
            text.sections[0].value = setting_label(&settings, value.0);
        }
    }
}

/// 主菜单样式层：普通按钮悬停高亮、齿轮图标变色、模式行选中/置灰/分类过滤、状态行同步。
/// 只在有输入或选中态变化时重刷，避免每帧覆写样式导致变更检测空转。
fn main_menu_style(
    ui: Res<MainMenuUi>,
    selected: Res<SelectedMode>,
    category: Res<SelectedCategory>,
    panel_vis: Query<&Visibility, (With<ModePanelRoot>, Without<ModeRow>)>,
    gear_hover: Query<&Interaction, (With<GearButton>, Changed<Interaction>)>,
    mut gear_icon: Query<&mut UiImage, With<GearIcon>>,
    mut hover_buttons: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<MenuButton>),
    >,
    row_changed: Query<&Interaction, (With<ModeRow>, Changed<Interaction>)>,
    category_changed: Query<&Interaction, (With<CategoryButton>, Changed<Interaction>)>,
    mut mode_rows: Query<
        (&ModeRow, &Interaction, &mut BackgroundColor, &mut BorderColor, &mut Visibility),
        (Without<MenuButton>, Without<CategoryButton>, Without<ModePanelRoot>),
    >,
    mut categories: Query<
        (&CategoryButton, &Interaction, &mut BackgroundColor, &mut BorderColor),
        (Without<MenuButton>, Without<ModeRow>),
    >,
    mut status_texts: Query<&mut Text, (With<StatusText>, Without<SettingValueText>)>,
) {
    let touched = ui.is_changed()
        || selected.is_changed()
        || category.is_changed()
        || !gear_hover.is_empty()
        || !hover_buttons.is_empty()
        || !row_changed.is_empty()
        || !category_changed.is_empty();
    if !touched {
        return;
    }

    let (base_bg, base_border) = menu_button_palette(false);
    let (hover_bg, hover_border) = menu_button_palette(true);

    // 普通按钮（切换/开始/退出/返回/齿轮底板）悬停高亮
    for (interaction, mut bg, mut border) in &mut hover_buttons {
        match *interaction {
            Interaction::Hovered => { *bg = hover_bg; *border = hover_border; }
            Interaction::None => { *bg = base_bg; *border = base_border; }
            Interaction::Pressed => {}
        }
    }

    // 齿轮图标随悬停着色
    if let Ok(interaction) = gear_hover.get_single() {
        if let Ok(mut image) = gear_icon.get_single_mut() {
            image.color = if *interaction == Interaction::Hovered {
                menu_accent()
            } else {
                Color::WHITE
            };
        }
    }

    // 模式行：面板打开时按分类过滤可见；未开放置灰、选中高亮。
    // 注意 bevy 0.14 中被显式写成 Visible 的子节点会在隐藏父节点下漏渲染，
    // 因此行可见性必须与面板显隐联动，不能只看分类。
    let panel_open = panel_vis
        .get(ui.mode_panel)
        .map_or(false, |vis| matches!(*vis, Visibility::Visible));
    for (row, interaction, mut bg, mut border, mut vis) in &mut mode_rows {
        let spec = game_mode_spec(row.0);
        let hovered = *interaction == Interaction::Hovered;
        *vis = if panel_open && (category.0 == 0 || spec.category == category.0) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if row.0 == selected.0 {
            *bg = BackgroundColor(Color::srgba(0.12, 0.22, 0.30, 0.98));
            *border = BorderColor(menu_accent());
        } else if !spec.available {
            *bg = BackgroundColor(Color::srgba(0.07, 0.09, 0.12, 0.90));
            *border = BorderColor(Color::srgb(0.15, 0.18, 0.24));
        } else if hovered {
            *bg = hover_bg;
            *border = hover_border;
        } else {
            *bg = base_bg;
            *border = base_border;
        }
    }

    // 分类按钮：选中高亮
    for (cat, interaction, mut bg, mut border) in &mut categories {
        let hovered = *interaction == Interaction::Hovered;
        if cat.0 == category.0 {
            *bg = BackgroundColor(Color::srgba(0.12, 0.22, 0.30, 0.98));
            *border = BorderColor(menu_accent());
        } else if hovered {
            *bg = hover_bg;
            *border = hover_border;
        } else {
            *bg = base_bg;
            *border = base_border;
        }
    }

    // 选中模式变化 → 右下角状态行同步
    if selected.is_changed() {
        if let Ok(mut text) = status_texts.get_mut(ui.status_text) {
            text.sections[0].value = format!("当前模式：{}", game_mode_spec(selected.0).name);
        }
    }
}

fn despawn_main_menu(mut commands: Commands, ui: Res<MainMenuUi>) {
    commands.entity(ui.root).despawn_recursive();
    commands.remove_resource::<MainMenuUi>();
    commands.remove_resource::<MenuGrace>();
}

fn despawn_menu_camera(mut commands: Commands, cam: Res<MenuCamera>) {
    commands.entity(cam.0).despawn();
    commands.remove_resource::<MenuCamera>();
}

/// 回到前端界面时释放鼠标（返回主菜单时复用）
fn release_cursor(
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    mut input_state: ResMut<InputState>,
) {
    unlock_cursor(&mut window_query, &mut input_state);
}

// =============================================================================
// Pause Menu —— /~ 呼出：暂停 + 游戏设置 + 返回主界面
// =============================================================================

/// 会话内游戏设置（"游戏设置"页调整；返回主界面后保留，下次进训练场继续生效）
#[derive(Resource)]
struct GameSettings {
    mouse_sensitivity: f32,
    fov_deg: f32,
    ambient_brightness: f32,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self { mouse_sensitivity: 1.0, fov_deg: 45.0, ambient_brightness: 0.55 }
    }
}

/// 暂停菜单状态：关闭 / 主面板 / 设置面板。
/// 刻意不做成 States：从 Paused 回 InGame 会再次触发 OnEnter(InGame) 重建整个世界，
/// 因此暂停只是 InGame 内的资源门控，只有"返回主界面"才真正切状态。
#[derive(Resource, Default, Clone, Copy, PartialEq)]
enum PauseMenu {
    #[default]
    Closed,
    Main,
    Settings,
}

#[derive(Resource)]
struct PauseMenuUi {
    root: Entity,
    main_panel: Entity,
    settings_panel: Entity,
}

/// 暂停菜单刚打开时的输入保护期：拦截开菜单瞬间按住的射击键误触按钮
#[derive(Resource)]
struct PauseGrace(Timer);

/// 暂停面板按钮动作（挂在按钮实体上，交互系统统一分发）
#[derive(Component, Clone, Copy, PartialEq)]
enum PauseAction {
    Resume,
    OpenSettings,
    BackToPause,
    ReturnMainMenu,
}

/// 可调设置项类别
#[derive(Component, Clone, Copy, PartialEq)]
enum SettingKind {
    Sensitivity,
    Fov,
    Ambient,
}

/// ◀/▶ 步进按钮：kind 对应设置项，delta 为步进量（负为减小）
#[derive(Component)]
struct SettingAdjust {
    kind: SettingKind,
    delta: f32,
}

/// 设置项当前值文本
#[derive(Component)]
struct SettingValueText(SettingKind);

/// 游戏逻辑门控条件：暂停菜单打开时不跑（挂在三个游戏系统元组上）
fn pause_open(pause: Res<PauseMenu>) -> bool {
    *pause != PauseMenu::Closed
}

fn pause_toggle(
    keyboard: Res<ButtonInput<KeyCode>>,
    wheel: Res<WheelState>,
    open_station: Res<OpenStation>,
    time: Res<Time>,
    mut grace: Option<ResMut<PauseGrace>>,
    mut pause: ResMut<PauseMenu>,
    ui: Option<Res<PauseMenuUi>>,
    settings: Res<GameSettings>,
    mut commands: Commands,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut input_state: ResMut<InputState>,
) {
    if let Some(grace) = grace.as_mut() {
        grace.0.tick(time.delta());
    }
    // 轮盘/站点面板打开时 / 键由它们负责（与 Esc 的跳过规则一致）
    if wheel.open || wheel.pending_key.is_some() || *open_station != OpenStation::None {
        return;
    }
    if !keyboard.just_pressed(KeyCode::Backquote) {
        return;
    }
    match *pause {
        PauseMenu::Closed => {
            *pause = PauseMenu::Main;
            spawn_pause_ui(&mut commands, &settings);
            unlock_cursor(&mut windows, &mut input_state);
        }
        _ => {
            *pause = PauseMenu::Closed;
            if let Some(ui) = ui {
                // bevy 0.14 despawn() 不递归，UI 树必须 despawn_recursive
                commands.entity(ui.root).despawn_recursive();
                commands.remove_resource::<PauseMenuUi>();
                commands.remove_resource::<PauseGrace>();
            }
            lock_cursor(&mut windows, &mut input_state);
        }
    }
}

fn lock_cursor(windows: &mut Query<&mut Window, With<PrimaryWindow>>, input_state: &mut InputState) {
    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor.visible = false;
        window.cursor.grab_mode = CursorGrabMode::Locked;
    }
    input_state.cursor_locked = true;
}

fn unlock_cursor(windows: &mut Query<&mut Window, With<PrimaryWindow>>, input_state: &mut InputState) {
    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor.visible = true;
        window.cursor.grab_mode = CursorGrabMode::None;
    }
    input_state.cursor_locked = false;
}

fn spawn_pause_ui(commands: &mut Commands, settings: &GameSettings) {
    let mut resume_btn = Entity::PLACEHOLDER;
    let mut settings_btn = Entity::PLACEHOLDER;
    let mut return_btn = Entity::PLACEHOLDER;
    let mut back_btn = Entity::PLACEHOLDER;

    let root = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        })
        .id();

    let mut main_panel = Entity::PLACEHOLDER;
    let mut settings_panel = Entity::PLACEHOLDER;

    commands.entity(root).with_children(|root| {
        // 全屏压暗层：独立绝对定位节点（与 HUD 边缘光同款写法，可靠渲染）
        root.spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.02, 0.55)),
            ..default()
        });
        main_panel = root.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            ..default()
        }).with_children(|panel| {
            panel.spawn(TextBundle::from_section(
                "游 戏 暂 停",
                TextStyle { font_size: 46.0, color: Color::srgb(0.92, 0.95, 1.0), ..default() },
            ));
            panel.spawn(TextBundle::from_section(
                "按 / 或 ~ 键继续游戏",
                TextStyle { font_size: 14.0, color: Color::srgb(0.55, 0.62, 0.72), ..default() },
            ));
            panel.spawn(NodeBundle {
                style: Style { height: Val::Px(18.0), ..default() },
                ..default()
            });
            resume_btn = spawn_menu_button(panel, "返 回 游 戏", "继续当前训练", true);
            settings_btn = spawn_menu_button(panel, "游 戏 设 置", "灵敏度 · 视野 · 亮度", true);
            return_btn = spawn_menu_button(panel, "返 回 主 界 面", "结束本次训练", true);
        }).id();

        settings_panel = root.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(14.0),
                ..default()
            },
            visibility: Visibility::Hidden,
            ..default()
        }).with_children(|panel| {
            panel.spawn(TextBundle::from_section(
                "游 戏 设 置",
                TextStyle { font_size: 40.0, color: Color::srgb(0.92, 0.95, 1.0), ..default() },
            ));
            panel.spawn(NodeBundle {
                style: Style { height: Val::Px(10.0), ..default() },
                ..default()
            });
            spawn_setting_row(panel, settings, SettingKind::Sensitivity, "鼠标灵敏度");
            spawn_setting_row(panel, settings, SettingKind::Fov, "视野 (FOV)");
            spawn_setting_row(panel, settings, SettingKind::Ambient, "环境亮度");
            panel.spawn(NodeBundle {
                style: Style { height: Val::Px(12.0), ..default() },
                ..default()
            });
            back_btn = spawn_menu_button(panel, "返 回", "回到暂停菜单", true);
        }).id();
    });

    commands.entity(resume_btn).insert(PauseAction::Resume);
    commands.entity(settings_btn).insert(PauseAction::OpenSettings);
    commands.entity(return_btn).insert(PauseAction::ReturnMainMenu);
    commands.entity(back_btn).insert(PauseAction::BackToPause);

    commands.insert_resource(PauseMenuUi { root, main_panel, settings_panel });
    commands.insert_resource(PauseGrace(Timer::from_seconds(0.25, TimerMode::Once)));
}

/// 设置行：标签 + ◀ 值 ▶
fn spawn_setting_row(parent: &mut ChildBuilder, settings: &GameSettings, kind: SettingKind, label: &str) {
    parent.spawn(NodeBundle {
        style: Style {
            width: Val::Px(460.0),
            height: Val::Px(46.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::px(18.0, 12.0, 0.0, 0.0),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        background_color: BackgroundColor(Color::srgba(0.10, 0.14, 0.20, 0.95)),
        border_color: BorderColor(Color::srgb(0.22, 0.28, 0.36)),
        border_radius: BorderRadius::all(Val::Px(4.0)),
        ..default()
    }).with_children(|row| {
        row.spawn(TextBundle::from_section(
            label,
            TextStyle { font_size: 19.0, color: Color::srgb(0.85, 0.89, 0.95), ..default() },
        ));
        row.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                ..default()
            },
            ..default()
        }).with_children(|ctrl| {
            spawn_step_button(ctrl, "<", kind, -setting_step(kind));
            ctrl.spawn((
                TextBundle::from_section(
                    setting_label(settings, kind),
                    TextStyle { font_size: 18.0, color: menu_accent(), ..default() },
                ),
                SettingValueText(kind),
            ));
            spawn_step_button(ctrl, ">", kind, setting_step(kind));
        });
    });
}

fn spawn_step_button(parent: &mut ChildBuilder, glyph: &str, kind: SettingKind, delta: f32) {
    parent.spawn((
        NodeBundle {
            style: Style {
                width: Val::Px(40.0),
                height: Val::Px(34.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.14, 0.20, 0.28, 0.98)),
            border_color: BorderColor(menu_accent()),
            border_radius: BorderRadius::all(Val::Px(4.0)),
            ..default()
        },
        Interaction::default(),
        MenuButton,
        SettingAdjust { kind, delta },
    )).with_children(|btn| {
        btn.spawn(TextBundle::from_section(
            glyph,
            TextStyle { font_size: 18.0, color: Color::srgb(0.92, 0.95, 1.0), ..default() },
        ));
    });
}

fn setting_step(kind: SettingKind) -> f32 {
    match kind {
        SettingKind::Sensitivity => 0.1,
        SettingKind::Fov => 5.0,
        SettingKind::Ambient => 0.05,
    }
}

fn setting_label(settings: &GameSettings, kind: SettingKind) -> String {
    match kind {
        SettingKind::Sensitivity => format!("x{:.1}", settings.mouse_sensitivity),
        SettingKind::Fov => format!("{:.0}", settings.fov_deg),
        SettingKind::Ambient => format!("{:.2}", settings.ambient_brightness),
    }
}

fn apply_setting_step(settings: &mut GameSettings, kind: SettingKind, delta: f32) {
    match kind {
        SettingKind::Sensitivity => settings.mouse_sensitivity = (settings.mouse_sensitivity + delta).clamp(0.2, 3.0),
        SettingKind::Fov => settings.fov_deg = (settings.fov_deg + delta).clamp(40.0, 110.0),
        SettingKind::Ambient => settings.ambient_brightness = (settings.ambient_brightness + delta).clamp(0.10, 1.20),
    }
}

fn pause_menu_interaction(
    mut commands: Commands,
    mut next_state: ResMut<NextState<AppState>>,
    mut pause: ResMut<PauseMenu>,
    mut settings: ResMut<GameSettings>,
    ui: Option<Res<PauseMenuUi>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut input_state: ResMut<InputState>,
    mut visibility: Query<&mut Visibility>,
    mut hover_buttons: Query<
        (Entity, &Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<MenuButton>),
    >,
    action_buttons: Query<(&PauseAction, &Interaction), Changed<Interaction>>,
    adjust_buttons: Query<(&SettingAdjust, &Interaction), Changed<Interaction>>,
    mut value_texts: Query<(&SettingValueText, &mut Text)>,
    grace: Option<Res<PauseGrace>>,
) {
    if let Some(grace) = grace {
        if !grace.0.finished() {
            return;
        }
    }

    // 悬停高亮（与主菜单同一套配色）
    let (base_bg, base_border) = menu_button_palette(false);
    let (hover_bg, hover_border) = menu_button_palette(true);
    for (_, interaction, mut bg, mut border) in &mut hover_buttons {
        match *interaction {
            Interaction::Hovered => { *bg = hover_bg; *border = hover_border; }
            Interaction::None => { *bg = base_bg; *border = base_border; }
            Interaction::Pressed => {}
        }
    }

    for (action, interaction) in &action_buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match action {
            PauseAction::Resume => {
                *pause = PauseMenu::Closed;
                if let Some(ui) = ui.as_ref() {
                    commands.entity(ui.root).despawn_recursive();
                    commands.remove_resource::<PauseMenuUi>();
                    commands.remove_resource::<PauseGrace>();
                }
                lock_cursor(&mut windows, &mut input_state);
                return;
            }
            PauseAction::OpenSettings | PauseAction::BackToPause => {
                let target = if *action == PauseAction::OpenSettings {
                    PauseMenu::Settings
                } else {
                    PauseMenu::Main
                };
                *pause = target;
                if let Some(ui) = ui.as_ref() {
                    if let Ok(mut vis) = visibility.get_mut(ui.main_panel) {
                        *vis = if target == PauseMenu::Main { Visibility::Visible } else { Visibility::Hidden };
                    }
                    if let Ok(mut vis) = visibility.get_mut(ui.settings_panel) {
                        *vis = if target == PauseMenu::Settings { Visibility::Visible } else { Visibility::Hidden };
                    }
                }
            }
            PauseAction::ReturnMainMenu => {
                // 直接切状态；清场交给 OnExit(InGame) 的 teardown_game
                next_state.set(AppState::MainMenu);
                return;
            }
        }
    }

    let mut adjusted = false;
    for (adjust, interaction) in &adjust_buttons {
        if *interaction == Interaction::Pressed {
            apply_setting_step(&mut settings, adjust.kind, adjust.delta);
            adjusted = true;
        }
    }
    if adjusted {
        for (value, mut text) in value_texts.iter_mut() {
            text.sections[0].value = setting_label(&settings, value.0);
        }
    }
}

/// 返回主界面时的清场：销毁除窗口外的所有实体并重置游戏态资源，
/// 下一次进入训练场由 OnEnter(InGame) 全量重建。
fn teardown_game(world: &mut World) {
    let window_entities: Vec<Entity> = world
        .query_filtered::<Entity, With<Window>>()
        .iter(world)
        .collect();
    let doomed: Vec<Entity> = world
        .iter_entities()
        .map(|entity| entity.id())
        .filter(|entity| !window_entities.contains(entity))
        .collect();
    for entity in doomed {
        world.despawn(entity);
    }

    world.insert_resource(NearbyInteract::default());
    world.insert_resource(WheelState::default());
    world.insert_resource(HeldGrenade::default());
    world.insert_resource(KillStats::default());
    world.insert_resource(OpenStation::None);
    world.insert_resource(InputState::default());
    world.insert_resource(PauseMenu::default());
    world.remove_resource::<PauseMenuUi>();
    world.remove_resource::<PauseGrace>();
    // setup_world 在下次进入时重新生成
    world.remove_resource::<EffectAssets>();
}

fn settings_apply_fov(
    settings: Res<GameSettings>,
    mut cameras: Query<(&mut Projection, &PlayerCamera)>,
) {
    // 不做 is_changed 短路：返回主界面再进游戏会生成新相机（默认 FOV），
    // 每帧幂等应用才能让设置在重建后保持一致
    for (mut projection, cam) in &mut cameras {
        if let Projection::Perspective(perspective) = &mut *projection {
            // 越肩瞄准时视野收窄（最多 -28%），与 aim_lerp 同步平滑过渡
            let target = settings.fov_deg.to_radians() * (1.0 - 0.28 * cam.aim_lerp);
            if perspective.fov != target {
                perspective.fov = target;
            }
        }
    }
}

fn settings_apply_ambient(settings: Res<GameSettings>, mut ambient: ResMut<AmbientLight>) {
    if !settings.is_changed() {
        return;
    }
    ambient.brightness = settings.ambient_brightness;
}

// =============================================================================
// Font
// =============================================================================

/// 内置中文字体（黑体 SimHei）。
///
/// Bevy 默认字体 FiraMono 不含 CJK 字形，中文会渲染成方块；
/// 必须在 Startup 阶段（任何文本布局发生之前）替换默认字体资产，
/// 因为 bevy_ui 的文本系统不监听字体资产的变更事件，事后替换不会触发布局重算。
const CJK_FONT_BYTES: &[u8] = include_bytes!("../../assets/fonts/simhei.ttf");

fn load_cjk_font(mut fonts: ResMut<Assets<Font>>) {
    match Font::try_from_bytes(CJK_FONT_BYTES.to_vec()) {
        Ok(font) => {
            // Bevy 在 TextPlugin::build 时把 FiraMono 注册在
            // Handle<Font>::default() 这个资产 id 下，所有未显式指定
            // font 的 TextStyle 都走它；原地覆盖即可让全部文本获得中文支持。
            fonts.insert(Handle::<Font>::default().id(), font);
        }
        Err(e) => {
            bevy::log::warn!("内置中文字体解析失败，中文将显示为方块: {:?}", e);
        }
    }
}

/// bevy_log 的订阅者在 DefaultPlugins 构建时才安装，
/// App::new() 之前用 bevy::log 打印的内容会被静默丢弃；
/// 因此启动期中文日志统一放到 Startup 系统里输出。
fn log_element_config(system: Res<ElementalSystem>) {
    bevy::log::info!("元素配置加载完成: {} 种反应规则", system.0.reaction_count());
}

// =============================================================================
// Element Definitions
// =============================================================================

// 元素类型直接复用核心库（crate::element::ElementType），
// 与cod1共享同一套配置表驱动的反应逻辑；
// 颜色/发光等纯表现信息由Demo通过扩展trait提供。

/// 元素表现层扩展（Demo专用）：核心库不含渲染信息
trait ElementVisual {
    fn color(&self) -> Color;
    fn emissive(&self) -> LinearRgba;
}

impl ElementVisual for ElementType {
    fn color(&self) -> Color {
        match self {
            ElementType::Fire => Color::srgb(1.0, 0.45, 0.12),
            ElementType::Ice => Color::srgb(0.35, 0.80, 1.0),
            ElementType::Electric => Color::srgb(0.95, 1.0, 0.15),
            ElementType::Poison => Color::srgb(0.50, 0.95, 0.20),
            ElementType::Physical => Color::srgb(0.75, 0.75, 0.75),
            ElementType::Water => Color::srgb(0.30, 0.55, 0.95),
        }
    }

    fn emissive(&self) -> LinearRgba {
        self.color().to_linear() * 6.0
    }
}

/// 加载元素配置文件（核心库统一加载器：自动定位项目根目录）。
/// 此函数在 App::new() 之前执行，bevy_log 订阅者尚未安装，
/// 必须用 eprintln! 输出，否则日志会被静默丢弃。
/// 文件缺失回退内置默认配置；解析失败直接退出——
/// 改错配置表应当大声失败，而不是静默用默认值让设计师误以为改动生效。
fn load_element_config() -> ElementConfig {
    match crate::config::load_element_config() {
        Ok((config, Some(path))) => {
            eprintln!(
                "元素配置加载完成: {} ({} 种反应规则)",
                path.display(),
                config.reactions.len()
            );
            config
        }
        Ok((config, None)) => {
            eprintln!("配置文件未找到(config/element_reactions.yaml)，使用内置默认配置");
            config
        }
        Err(e) => {
            eprintln!("{e}");
            eprintln!("请修正 config/element_reactions.yaml 后重新启动");
            std::process::exit(1);
        }
    }
}

// =============================================================================
// Components
// =============================================================================

#[derive(Resource, Default)]
struct InputState {
    pub cursor_locked: bool,
}

#[derive(Resource, Default)]
struct NearbyInteract {
    /// 统一交互条目：站点在最前，拾取物按距离升序排在其后
    entries: Vec<InteractEntry>,
    /// 当前选中的 entries 下标
    selected: usize,
    /// 滚动窗口起始下标：固定高度只展示 [scroll_start, scroll_start+可见行数) 的条目
    scroll_start: usize,
}

/// 交互菜单条目：F 执行选中项 —— 拾取物直接拾取，站点打开面板
#[derive(Clone, Copy)]
enum InteractEntry {
    Pickup(Entity),
    Station { kind: StationKind, label: &'static str },
}




#[derive(Component)]
struct Health { current: f32, max: f32 }
impl Default for Health { fn default() -> Self { Self { current: 100.0, max: 100.0 } } }

#[derive(Component)]
struct Armor { current: f32, max: f32 }
impl Default for Armor { fn default() -> Self { Self { current: 60.0, max: 100.0 } } }

#[derive(Component, Clone)]
struct WeaponData {
    element: ElementType,
    ammo: i32,
    max_ammo: i32,
    fire_interval: f32,
    damage: f32,
    name: String,
}

impl WeaponData {
    /// 从核心武器档案构建一把满弹步枪（备弹统一放背包弹药池）
    fn from_profile(profile: &crate::operator::RifleProfile) -> Self {
        Self {
            element: profile.element,
            ammo: profile.max_ammo,
            max_ammo: profile.max_ammo,
            fire_interval: profile.fire_interval,
            damage: profile.damage,
            name: profile.name.to_string(),
        }
    }
}

/// 主武器固定双槽：背包里永远只有两把枪，拾取新枪直接替换当前手持的那把
const MAX_WEAPONS: usize = 2;

/// 武器槽：只有"当前手持哪把"一个状态（weapons[0]/[1] 即 1/2 号主武器位）
#[derive(Component)]
struct WeaponSlot { current: usize }
impl Default for WeaponSlot {
    fn default() -> Self {
        Self { current: 0 }
    }
}


/// 切换干员：重置 Q/E 冷却为该干员的配置值，并把角色发光饰条染成新干员元素色
fn switch_operator(
    state: &mut OperatorState,
    accent: &OperatorAccent,
    materials: &mut Assets<StandardMaterial>,
    idx: usize,
) {
    let Some(op) = roster().get(idx) else { return };
    state.active = idx;
    state.q = ready_timer(op.q.cooldown_secs);
    state.e = ready_timer(op.e.cooldown_secs);
    if let Some(mat) = materials.get_mut(&accent.0) {
        mat.base_color = op.element.color();
        mat.emissive = op.element.emissive();
    }
}


#[derive(Component)]
struct BulletHit { timer: Timer }

#[derive(Component)]
struct DamageParticle { velocity: Vec3, timer: Timer }

#[derive(Component)]
struct GrenadeProjectile {
    velocity: Vec3,
    element: ElementType,
    /// 落点爆炸伤害与半径（干员技能/背包道具各自配置）
    damage: f32,
    radius: f32,
    /// 落点附加机制（点燃/冰冻/毒区，来自干员档案或背包道具默认值）
    effect: crate::operator::SkillEffect,
    timer: Timer,
}

/// 挂在目标身上的持续伤害（点燃等）：每 DOT_TICK 秒结算一次
#[derive(Clone, Debug)]
struct DamageOverTime {
    element: ElementType,
    dps: f32,
    tick: Timer,
    remaining: Timer,
}

/// 持续伤害的结算间隔（秒）
const DOT_TICK: f32 = 0.5;

#[derive(Component)]
struct ExplosionEffect { timer: Timer, max_scale: f32 }

#[derive(Component)]
struct TargetDummy {
    max_health: f32,
    current_health: f32,
    hit_flash: Option<Timer>,
    element_state: Option<ElementType>,
    state_timer: Option<Timer>,
    /// 倒地倒计时：Some = 已被击倒（倒地期间不可再被命中/移动）
    down_timer: Option<Timer>,
    /// 倒下方向（水平单位向量，取自致命一击的来弹方向）
    fall_dir: Vec3,
    /// 冰冻/电麻硬控：Some = 停止行动计时中（移动靶停止巡逻）
    frozen: Option<Timer>,
    /// 冰冻视觉（冰块）子实体，解冻/被击倒时移除
    frozen_visual: Option<Entity>,
    /// 持续伤害（点燃等），可叠加
    dots: Vec<DamageOverTime>,
    /// 击杀播报中显示的名称
    label: &'static str,
}
impl Default for TargetDummy {
    fn default() -> Self {
        Self {
            max_health: 200.0, current_health: 200.0,
            hit_flash: None, element_state: None, state_timer: None,
            down_timer: None, fall_dir: Vec3::X, label: "训练靶",
            frozen: None, frozen_visual: None, dots: Vec::new(),
        }
    }
}
/// 击倒后经过 fall_time 秒完全趴下，down_secs 秒后原地复活
const TARGET_FALL_TIME: f32 = 0.55;
const TARGET_DOWN_SECS: f32 = 4.0;

#[derive(Component)]
struct MovingTarget { speed: f32, range: f32, origin: Vec3, direction: f32 }

#[derive(Component)]
struct CrosshairRoot;

#[derive(Component)]
/// 准星锚点：flex 居中的 0×0 节点，准星部件相对它绝对定位（真·屏幕正中）
struct CrosshairAnchor;

#[derive(Component)]
struct CrosshairLine;

#[derive(Component)]
struct CrosshairCenter;

#[derive(Component)]
struct HudHealthBarBg;

#[derive(Component)]
struct HudHealthBarFill;

/// 血条下方的 "HP 100/100" 数值文本
#[derive(Component)]
struct HudHealthText;

#[derive(Component)]
struct HudArmorBarBg;

#[derive(Component)]
struct HudArmorBarFill;

/// 护甲条下方的 "ARMOR 60/100" 数值文本
#[derive(Component)]
struct HudArmorText;

#[derive(Component)]
struct HudAmmoMain;

#[derive(Component)]
struct HudAmmoReserve;

#[derive(Component)]
struct HudWeaponName;

#[derive(Component)]
struct HudWeaponSlot1;

#[derive(Component)]
struct HudWeaponSlot2;

#[derive(Component)]
struct HudSkillQBg;

#[derive(Component)]
struct HudSkillQFill;

#[derive(Component)]
struct HudSkillQText;

#[derive(Component)]
struct HudSkillEBg;

#[derive(Component)]
struct HudSkillEFill;

#[derive(Component)]
struct HudSkillEText;

#[derive(Component)]
struct HudEdgeGlow;

#[derive(Component)]
struct HudReloadText;

#[derive(Component)]
struct HudSkillQLabel;

#[derive(Component)]
struct HudSkillELabel;

#[derive(Component)]
struct FloatingReaction { timer: Timer }

#[derive(Component)]
struct DamagePopup {
    timer: Timer,
    world_pos: Vec3,
}

#[derive(Component)]
struct Collider { half_size: Vec3 }

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum ItemCategory {
    /// 恢复道具（3号键 / 轮盘）：医疗、护甲、弹药等补给
    #[default]
    Consumable,
    /// 战术道具（4号键 / 轮盘）：可投掷的元素手雷
    Tactical,
}

#[derive(Clone)]
enum PickupType {
    /// 步枪弹药：拾取时直接补充背包弹药池（不占物资槽位）
    Ammo { amount: i32 },
    Health { amount: f32 },
    Armor { amount: f32 },
    Grenade { element: ElementType },
    /// 步枪武器：拾取后进入背包武器架
    Weapon { element: ElementType },
}
impl PickupType {
    fn category(&self) -> ItemCategory {
        match self {
            PickupType::Grenade { .. } => ItemCategory::Tactical,
            _ => ItemCategory::Consumable,
        }
    }
}

#[derive(Clone, Component)]
struct PickupItem {
    name: String,
    item_type: PickupType,
}

/// 玩家背包：物资槽 + 武器架 + 弹药池。
/// 武器本体永远存放在 weapons（WeaponSlot 只存装备下标），
/// 弹药是池化资源（换弹从这里取弹，弹药拾取直接入池）。
#[derive(Component)]
struct Inventory {
    items: Vec<PickupItem>,
    max_slots: usize,
    weapons: Vec<WeaponData>,
    max_weapons: usize,
    ammo_pool: i32,
}
impl Default for Inventory {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            max_slots: 8,
            weapons: vec![
                WeaponData::from_profile(&rifle_profile(ElementType::Fire)),
                WeaponData::from_profile(&rifle_profile(ElementType::Ice)),
            ],
            max_weapons: MAX_WEAPONS,
            ammo_pool: 150,
        }
    }
}

#[derive(Component)]
struct InventoryUI;

/// 统一交互菜单根节点：站点 + 附近拾取物，滚轮选择 / F 确认
#[derive(Component)]
struct InteractMenuUI;

/// 交互菜单的面板容器（深色底）：bevy 0.14 UI 不按祖先可见性剔除，
/// 收起时需与标题/行/页脚一起显式隐藏
#[derive(Component)]
struct InteractMenuPanel;

/// 交互菜单第 i 个可见行槽（滚动窗口内第 i 行，选中高亮背景/边框挂在行节点上）
#[derive(Component)]
struct InteractRow(usize);

/// 交互菜单第 i 个可见行槽的文本（行节点的子实体）
#[derive(Component)]
struct InteractRowText(usize);

/// 交互菜单滚动条部件：凹槽常驻占位（保持面板宽度稳定），滑块随滚动窗口移动
#[derive(Component)]
struct InteractScrollBar(ScrollbarPart);

/// 滚动条部件角色
#[derive(Clone, Copy, PartialEq)]
enum ScrollbarPart {
    Track,
    Thumb,
}

/// 交互菜单标题行（F 徽标 + 标题）：bevy 0.14 UI 不按祖先可见性剔除子节点，
/// 收起菜单时必须连这一层一起显式隐藏
#[derive(Component)]
struct InteractMenuHeader;

/// 交互菜单底部提示行：固定操作提示 + 选中条目的满载/替换警告
#[derive(Component)]
struct InteractMenuHintText;

#[derive(Component)]
struct InventorySlotUI(usize);

#[derive(Component)]
struct InventorySlotText(usize);

// --- 背包扩展区（武器架 / 弹药池） ---

#[derive(Component)]
struct BackpackWeaponSlot(usize);

#[derive(Component)]
struct BackpackWeaponText(usize);

#[derive(Component)]
struct HudBackpackAmmo;

// --- HUD：当前干员名 ---

#[derive(Component)]
struct HudOperatorName;

// --- 功能站点（补给台 / 干员切换台） ---

/// 场景交互站点（几何由地图 props 提供，这里只登记语义与位置）
#[derive(Component)]
struct Station {
    kind: StationKind,
    label: &'static str,
}

/// 距离站点多远可交互（米）
const STATION_USE_RANGE: f32 = 3.0;

/// 当前打开的站点面板
#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
enum OpenStation {
    #[default]
    None,
    Supply,
    Operator,
}

#[derive(Component)]
struct SupplyUIRoot;

#[derive(Component)]
struct SupplyRow(usize);

#[derive(Component)]
struct SupplyStatusText;

#[derive(Component)]
struct OperatorUIRoot;

#[derive(Component)]
struct OperatorCard(usize);

#[derive(Component)]
struct OperatorCardText(usize);

#[derive(Component)]
struct OperatorStatusText;

/// 补给台可领取的物资行：(名称, 效果说明)
const SUPPLY_ROWS: [(&str, &str); 7] = [
    ("步枪弹药 ×60", "直接补充弹药池"),
    ("医疗包", "恢复 25 HP"),
    ("护甲片", "恢复 20 护甲"),
    ("烈焰手雷", "火系战术道具"),
    ("冰霜手雷", "冰系战术道具"),
    ("雷电手雷", "电系战术道具"),
    ("毒素手雷", "毒系战术道具"),
];

// --- 统一交互菜单（站点 + 拾取物） ---

/// 交互菜单固定高度内可见的行槽数（超出部分折叠，靠滚动窗口查看）
const INTERACT_MENU_VISIBLE_ROWS: usize = 8;
/// 交互菜单最多缓存的条目数（超过可见行数的条目靠滚动条查看）
const INTERACT_MENU_MAX_ENTRIES: usize = 16;
/// 交互菜单单行固定高度 / 行间距：行槽常驻占位，保证面板高度不随条目数变化
const INTERACT_ROW_H: f32 = 28.0;
const INTERACT_ROW_GAP: f32 = 4.0;
/// 滚动条凹槽高度 = 可见行总高（与条目列表列等高）
const INTERACT_SCROLL_TRACK_H: f32 =
    INTERACT_MENU_VISIBLE_ROWS as f32 * INTERACT_ROW_H + (INTERACT_MENU_VISIBLE_ROWS as f32 - 1.0) * INTERACT_ROW_GAP;

// --- 道具轮盘（长按 3/4 呼出） ---

#[derive(Component)]
struct WheelRoot;

#[derive(Component)]
struct WheelHubText;

/// 轮盘中心的"取消"按钮：点击撤销本次使用（不消耗道具）
#[derive(Component)]
struct WheelCancelButton;

#[derive(Component)]
struct WheelCard(usize);

#[derive(Component)]
struct WheelCardText(usize);

/// 轮盘状态机：按下3/4 → 短按快速使用首个对应道具，长按呼出轮盘，松开键确认使用。
/// 光标回到中心或点击中心"取消"键 → 撤销使用（不消耗道具）。
#[derive(Resource, Default)]
struct WheelState {
    open: bool,
    /// 已按下但尚未决定（短按/长按）的按键
    pending_key: Option<KeyCode>,
    pending_hold: f32,
    /// 轮盘对应的道具类别
    category: ItemCategory,
    /// 轮盘展示的背包条目索引（已按类别过滤）
    filtered: Vec<usize>,
    /// 当前选中的 filtered 下标
    selected: Option<usize>,
    /// 轮盘已打开时长（秒），用于超时兜底
    open_secs: f32,
}

/// 轮盘按键按住多久后判定为“长按”并呼出轮盘
const WHEEL_OPEN_DELAY: f32 = 0.28;
/// 轮盘打开多久后强制收起（兜底防卡屏）
const WHEEL_MAX_OPEN_SECS: f32 = 10.0;
/// 轮盘卡片环绕半径（逻辑像素）
const WHEEL_RADIUS: f32 = 150.0;
const WHEEL_CARD_W: f32 = 132.0;
const WHEEL_CARD_H: f32 = 44.0;

// --- 底部快捷道具图标（3 恢复 / 4 战术） ---

/// 恢复道具（3号）图标色
const ITEM_RECOVERY_COLOR: Color = Color::srgb(0.35, 0.85, 0.45);
/// 战术道具（4号）图标色
const ITEM_TACTICAL_COLOR: Color = Color::srgb(1.0, 0.62, 0.18);

#[derive(Component)]
struct HudItemSlotText(usize);

#[derive(Component)]
struct HudItemSlotLabel(usize);

// --- 击杀播报（右上角） ---

#[derive(Event)]
struct KillEvent {
    name: String,
}

#[derive(Resource, Default)]
struct KillStats {
    total: u32,
}

#[derive(Component)]
struct KillFeedRoot;

#[derive(Component)]
struct KillFeedTotal;

#[derive(Component)]
struct KillFeedEntry {
    timer: Timer,
    base_color: Color,
}


// =============================================================================
// World Setup
// =============================================================================

fn setup_world(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Main directional light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 8000.0,
            shadows_enabled: true,
            shadow_depth_bias: 0.02,
            shadow_normal_bias: 0.6,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ, -0.8, 0.5, 0.0,
        )),
        ..default()
    });

    // Fill light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 1500.0,
            shadows_enabled: false,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ, -0.3, -1.5, 0.0,
        )),
        ..default()
    });

    // Corner lights
    for pos in [(15.0, 2.5, 15.0), (-15.0, 2.5, 15.0), (15.0, 2.5, -15.0), (-15.0, 2.5, -15.0)] {
        commands.spawn(PointLightBundle {
            point_light: PointLight {
                intensity: 40000.0,
                color: Color::srgb(0.9, 0.85, 0.75),
                range: 22.0,
                shadows_enabled: false,
                ..default()
            },
            transform: Transform::from_xyz(pos.0, pos.1, pos.2),
            ..default()
        });
    }

    // Player
    let player_pos = Vec3::new(0.0, 0.0, 0.0);
    spawn_player(&mut commands, &mut meshes, &mut materials, player_pos);

    // 越肩相机装配（SpringArm 架构）：
    // CamPivot(脚底, TopLevel 不随模型旋转) → ShoulderPivot(Yaw) → PitchPivot(Pitch)
    //   → SpringArm(右肩偏移 + 后方距离，带碰撞缩回) → Camera
    // 相机本地旋转保持单位，视线始终 = 枢轴前方（与瞄准方向平行越过右肩）
    commands.spawn((
        SpatialBundle { transform: Transform::from_translation(player_pos), ..default() },
        CamPivot,
    )).with_children(|pivot| {
        pivot.spawn((SpatialBundle::default(), ShoulderPivot)).with_children(|yaw| {
            yaw.spawn((
                SpatialBundle { transform: Transform::from_xyz(0.0, PIVOT_HEIGHT, 0.0), ..default() },
                PitchPivot,
            )).with_children(|pitch| {
                pitch.spawn((
                    SpatialBundle {
                        transform: Transform::from_xyz(
                            ARM_SHOULDER_X_NORMAL, ARM_EYE_Y_NORMAL, ARM_LEN_NORMAL),
                        ..default()
                    },
                    SpringArm,
                    SpringArmState { len: ARM_LEN_NORMAL },
                )).with_children(|arm| {
                    arm.spawn((Camera3dBundle::default(), PlayerCamera::default()));
                });
            });
        });
    });

    // AI enemies
    spawn_enemy(&mut commands, &mut meshes, &mut materials, Vec3::new(8.0, 0.0, -8.0), CharacterPreset::EnemyIce);
    spawn_enemy(&mut commands, &mut meshes, &mut materials, Vec3::new(-8.0, 0.0, -6.0), CharacterPreset::TeammateElectric);

    // Training ground
    spawn_training_ground(&mut commands, &mut meshes, &mut materials);

    // 共享特效资产：所有运行时特效（曳光/火花/粒子/爆炸/投掷物）复用
    commands.insert_resource(EffectAssets::new(&mut meshes, &mut materials));
}

// =============================================================================
// Training Ground —— 数据驱动渲染
// =============================================================================

// 地图几何数据由 `crate::map::training` 提供（纯数据，无 bevy 依赖）。
// 本文件只负责把数据渲染成实体，不在这里摆放任何掩体/靶位；
// 调整训练场布局请改 src/map/training/mod.rs。

fn spawn_training_ground(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    spawn_map_layout(&training::layout(), commands, meshes, materials);
}

/// 通用地图渲染器：渲染任意 [`MapLayout`] 为场景实体
fn spawn_map_layout(
    layout: &MapLayout,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    spawn_floor(layout, commands, meshes, materials);

    let mats = MapMaterials::new(materials);
    // 网格/材质全场查池复用：每个独立网格或材质都是一份 GPU 缓冲，
    // 核显上累积过多会 OOM（与棋盘格地面共用网格同一原因）
    let mut pool = AssetPool::default();
    for prop in &layout.props {
        spawn_prop(prop, commands, meshes, &mats, &mut pool);
    }
    for target in &layout.targets {
        spawn_map_target(target, commands, meshes, materials, &mut pool);
    }
    for pickup in &layout.pickups {
        spawn_map_pickup(pickup, commands, meshes, materials, &mut pool);
    }
    for glow in &layout.glows {
        spawn_glow(glow, commands, meshes, materials, &mut pool);
    }
    for station in &layout.stations {
        spawn_map_station(station, commands);
    }
}

/// 棋盘格地面（不含碰撞：地面行走由玩家系统自行处理）
fn spawn_floor(
    layout: &MapLayout,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let ground_a = mat_voxel(materials, palette::GROUND_A);
    let ground_b = mat_voxel(materials, palette::GROUND_B);
    let tile = layout.floor_tile;
    let n = (layout.half_extent / tile).round() as i32;
    // 全部棋盘格共用一个网格：961 块地砖各建一个网格会在核显上
    // 累积上千个 GPU 缓冲，是 OOM 崩溃的主要底噪
    let tile_mesh = meshes.add(Cuboid::new(tile, 0.2, tile));
    for x in -n..=n {
        for z in -n..=n {
            let is_dark = (x + z) % 2 == 0;
            commands.spawn(PbrBundle {
                mesh: tile_mesh.clone(),
                material: if is_dark { ground_a.clone() } else { ground_b.clone() },
                transform: Transform::from_xyz(x as f32 * tile, -0.1, z as f32 * tile),
                ..default()
            }).insert(NotShadowCaster);
        }
    }
}

/// 语义材质映射表：一次构造，全场复用
struct MapMaterials {
    concrete: Handle<StandardMaterial>,
    rust: Handle<StandardMaterial>,
    steel: Handle<StandardMaterial>,
    target_red: Handle<StandardMaterial>,
    target_white: Handle<StandardMaterial>,
    paint_white: Handle<StandardMaterial>,
    dark: Handle<StandardMaterial>,
    pipe: Handle<StandardMaterial>,
    warn_yellow: Handle<StandardMaterial>,
    warn_orange: Handle<StandardMaterial>,
    warn_red: Handle<StandardMaterial>,
}

impl MapMaterials {
    fn new(materials: &mut ResMut<Assets<StandardMaterial>>) -> Self {
        Self {
            concrete: mat_voxel(materials, palette::CONCRETE),
            rust: mat_voxel(materials, palette::RUST),
            steel: mat_voxel(materials, Color::srgb(0.35, 0.37, 0.38)),
            target_red: mat_voxel(materials, palette::TARGET_RED),
            target_white: mat_voxel(materials, palette::TARGET_WHITE),
            paint_white: mat_voxel(materials, Color::srgb(0.9, 0.9, 0.9)),
            dark: mat_voxel(materials, Color::srgb(0.2, 0.2, 0.22)),
            pipe: mat_voxel(materials, Color::srgb(0.5, 0.45, 0.4)),
            warn_yellow: mat_voxel(materials, Color::srgb(0.9, 0.7, 0.15)),
            warn_orange: mat_voxel(materials, Color::srgb(0.85, 0.45, 0.15)),
            warn_red: mat_voxel(materials, Color::srgb(0.85, 0.15, 0.15)),
        }
    }

    fn get(&self, kind: MaterialKind) -> Handle<StandardMaterial> {
        match kind {
            MaterialKind::Concrete => self.concrete.clone(),
            MaterialKind::Rust => self.rust.clone(),
            MaterialKind::Steel => self.steel.clone(),
            MaterialKind::TargetRed => self.target_red.clone(),
            MaterialKind::TargetWhite => self.target_white.clone(),
            MaterialKind::PaintWhite => self.paint_white.clone(),
            MaterialKind::Dark => self.dark.clone(),
            MaterialKind::Pipe => self.pipe.clone(),
            MaterialKind::WarningYellow => self.warn_yellow.clone(),
            MaterialKind::WarningOrange => self.warn_orange.clone(),
            MaterialKind::WarningRed => self.warn_red.clone(),
        }
    }
}

/// 全场共享的网格与材质池：同尺寸/同颜色的重复件只建一份 GPU 缓冲
/// （标线虚线、矮墙段、靶机部件、拾取物外壳等），核显上缓冲过多会 OOM
#[derive(Default)]
struct AssetPool {
    boxes: HashMap<[u32; 3], Handle<Mesh>>,
    cylinders: HashMap<[u32; 2], Handle<Mesh>>,
    spheres: HashMap<u32, Handle<Mesh>>,
    voxel_mats: HashMap<[u32; 4], Handle<StandardMaterial>>,
    emissive_mats: HashMap<[u32; 5], Handle<StandardMaterial>>,
}

impl AssetPool {
    fn box_mesh(&mut self, meshes: &mut ResMut<Assets<Mesh>>, w: f32, h: f32, d: f32) -> Handle<Mesh> {
        self.boxes
            .entry([w.to_bits(), h.to_bits(), d.to_bits()])
            .or_insert_with(|| meshes.add(Cuboid::new(w, h, d)))
            .clone()
    }

    fn cylinder_mesh(&mut self, meshes: &mut ResMut<Assets<Mesh>>, radius: f32, height: f32) -> Handle<Mesh> {
        self.cylinders
            .entry([radius.to_bits(), height.to_bits()])
            .or_insert_with(|| meshes.add(Cylinder::new(radius, height)))
            .clone()
    }

    fn sphere_mesh(&mut self, meshes: &mut ResMut<Assets<Mesh>>, radius: f32) -> Handle<Mesh> {
        self.spheres
            .entry(radius.to_bits())
            .or_insert_with(|| meshes.add(Sphere::new(radius).mesh().ico(2).unwrap()))
            .clone()
    }

    fn voxel_mat(&mut self, materials: &mut ResMut<Assets<StandardMaterial>>, color: Color) -> Handle<StandardMaterial> {
        let key = mat_key(color);
        self.voxel_mats
            .entry(key)
            .or_insert_with(|| mat_voxel(materials, color))
            .clone()
    }

    /// mult：自发光强度倍数（2.0=灯带/拾取物，3.0=靶心，4.0=浮动指示球）
    fn emissive_mat(
        &mut self,
        materials: &mut ResMut<Assets<StandardMaterial>>,
        color: Color,
        mult: f32,
    ) -> Handle<StandardMaterial> {
        let l = color.to_linear();
        let key = [
            l.red.to_bits(),
            l.green.to_bits(),
            l.blue.to_bits(),
            l.alpha.to_bits(),
            mult.to_bits(),
        ];
        self.emissive_mats
            .entry(key)
            .or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: color,
                    emissive: color.to_linear() * mult,
                    metallic: 0.0,
                    perceptual_roughness: 1.0,
                    ..default()
                })
            })
            .clone()
    }
}

/// 材质缓存键：线性 RGBA 的 bit 量化
fn mat_key(color: Color) -> [u32; 4] {
    let l = color.to_linear();
    [l.red.to_bits(), l.green.to_bits(), l.blue.to_bits(), l.alpha.to_bits()]
}

/// 渲染单个静态物体（掩体/标线/装饰/管道），同尺寸网格走池复用
fn spawn_prop(
    prop: &Prop,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mats: &MapMaterials,
    pool: &mut AssetPool,
) {
    let mesh = match prop.shape {
        Shape::Box => pool.box_mesh(meshes, prop.half[0] * 2.0, prop.half[1] * 2.0, prop.half[2] * 2.0),
        Shape::Cylinder { radius, height } => pool.cylinder_mesh(meshes, radius, height),
    };
    let mut transform = Transform::from_translation(Vec3::from(prop.pos));
    if let Some((axis, angle)) = prop.rot {
        transform.rotation = Quat::from_axis_angle(Vec3::from(axis), angle);
    }
    let mut entity = commands.spawn(PbrBundle {
        mesh,
        material: mats.get(prop.material),
        transform,
        ..default()
    });
    entity.insert(NotShadowCaster);
    if prop.solid {
        entity.insert(Collider { half_size: Vec3::from(prop.aabb_half()) });
    }
}

/// 渲染单个靶（静态靶走 dummy，移动靶挂 MovingTarget 组件）
fn spawn_map_target(
    spec: &TargetSpec,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pool: &mut AssetPool,
) {
    let pos = Vec3::from(spec.pos);
    let red = pool.voxel_mat(materials, palette::TARGET_RED);
    let white = pool.voxel_mat(materials, palette::TARGET_WHITE);
    match spec.motion {
        None => {
            spawn_dummy(commands, meshes, materials, pool, pos, red, white, spec.label);
        }
        Some(m) => {
            let board = pool.box_mesh(meshes, 0.8, 0.8, 0.2);
            let inner = pool.box_mesh(meshes, 0.4, 0.4, 0.25);
            commands.spawn((
                SpatialBundle { transform: Transform::from_translation(pos), ..default() },
                MovingTarget { speed: m.speed, range: m.range, origin: pos, direction: m.start_dir },
                TargetDummy { label: spec.label, ..default() },
            )).with_children(|p| {
                p.spawn(PbrBundle { mesh: board, material: red, ..default() });
                p.spawn(PbrBundle { mesh: inner, material: white, ..default() });
            });
        }
    }
}

/// 渲染单个拾取物：数据层道具枚举 → demo 道具与配色
fn spawn_map_pickup(
    spec: &PickupSpec,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pool: &mut AssetPool,
) {
    let (item_type, color) = match spec.kind {
        PickupKind::Ammo { amount } => (PickupType::Ammo { amount }, Color::srgb(0.9, 0.7, 0.2)),
        PickupKind::Health { amount } => (PickupType::Health { amount }, Color::srgb(0.9, 0.2, 0.2)),
        PickupKind::Armor { amount } => (PickupType::Armor { amount }, Color::srgb(0.2, 0.5, 0.9)),
        PickupKind::Grenade { element } => (PickupType::Grenade { element }, element.color()),
        PickupKind::Weapon { element } => (PickupType::Weapon { element }, element.color()),
    };
    spawn_pickup_item(commands, meshes, materials, pool, Vec3::from(spec.pos), item_type, spec.label, color);
}

/// 生成场景功能站点（补给台/干员切换台的交互登记点，桌面几何由 props 提供）
fn spawn_map_station(spec: &StationSpec, commands: &mut Commands) {
    commands.spawn((
        SpatialBundle {
            transform: Transform::from_translation(Vec3::from(spec.pos)),
            ..default()
        },
        Station { kind: spec.kind, label: spec.label },
    ));
}

/// 渲染单个发光件（霓虹灯带/信标/出生光垫）
fn spawn_glow(
    glow: &GlowSpec,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pool: &mut AssetPool,
) {
    let color = match glow.glow {
        GlowKind::Green => Color::srgb(0.1, 0.9, 0.4),
        GlowKind::Orange => Color::srgb(0.9, 0.5, 0.1),
        GlowKind::Red => palette::TARGET_RED,
        GlowKind::SpawnPad => Color::srgb(0.15, 0.85, 0.25),
        GlowKind::Supply => Color::srgb(1.0, 0.65, 0.15),
        GlowKind::Operator => Color::srgb(0.2, 0.9, 0.95),
    };
    let mesh = match glow.shape {
        Shape::Box => pool.box_mesh(meshes, glow.half[0] * 2.0, glow.half[1] * 2.0, glow.half[2] * 2.0),
        Shape::Cylinder { radius, height } => pool.cylinder_mesh(meshes, radius, height),
    };
    commands.spawn(PbrBundle {
        mesh,
        material: pool.emissive_mat(materials, color, 2.0),
        transform: Transform::from_translation(Vec3::from(glow.pos)),
        ..default()
    }).insert(NotShadowCaster);
}

#[allow(clippy::too_many_arguments)]
fn spawn_dummy(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pool: &mut AssetPool,
    position: Vec3,
    red: Handle<StandardMaterial>,
    white: Handle<StandardMaterial>,
    label: &'static str,
) {
    // 支柱底端贴地：高台等架空靶位自动获得更长支撑
    let pole_y = 0.75 - position.y;
    let board = pool.box_mesh(meshes, 1.2, 1.2, 0.3);
    let inner = pool.box_mesh(meshes, 0.6, 0.6, 0.35);
    let core = pool.box_mesh(meshes, 0.2, 0.2, 0.4);
    let pole = pool.box_mesh(meshes, 0.2, 1.5, 0.2);
    let core_mat = pool.emissive_mat(materials, Color::srgb(1.0, 0.0, 0.0), 3.0);
    let pole_mat = pool.voxel_mat(materials, Color::srgb(0.4, 0.4, 0.4));

    commands.spawn((
        SpatialBundle {
            transform: Transform::from_translation(position),
            ..default()
        },
        TargetDummy { label, ..default() },
    )).with_children(|p| {
        p.spawn(PbrBundle {
            mesh: board,
            material: red,
            ..default()
        });
        p.spawn(PbrBundle {
            mesh: inner,
            material: white,
            ..default()
        });
        p.spawn(PbrBundle {
            mesh: core,
            material: core_mat,
            ..default()
        });
        p.spawn(PbrBundle {
            mesh: pole,
            material: pole_mat,
            transform: Transform::from_xyz(0.0, pole_y, 0.0),
            ..default()
        });
    });
}

fn spawn_pickup_item(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pool: &mut AssetPool,
    position: Vec3,
    item_type: PickupType,
    name: &str,
    color: Color,
) {
    let (mesh_size, glow_color) = match &item_type {
        PickupType::Ammo { .. } => (Vec3::new(0.5, 0.35, 0.35), Color::srgb(1.0, 0.85, 0.3)),
        PickupType::Health { .. } => (Vec3::new(0.4, 0.25, 0.4), Color::srgb(1.0, 0.3, 0.3)),
        PickupType::Armor { .. } => (Vec3::new(0.4, 0.3, 0.5), Color::srgb(0.3, 0.6, 1.0)),
        PickupType::Grenade { element } => (Vec3::new(0.32, 0.32, 0.32), element.color()),
        PickupType::Weapon { element } => (Vec3::new(0.18, 0.18, 0.9), element.color()),
    };
    let body = pool.box_mesh(meshes, mesh_size.x, mesh_size.y, mesh_size.z);
    let body_mat = pool.emissive_mat(materials, color, 2.0);
    let orb = pool.sphere_mesh(meshes, 0.08);
    let orb_mat = pool.emissive_mat(materials, glow_color, 4.0);
    commands.spawn((
        PbrBundle {
            mesh: body,
            material: body_mat,
            transform: Transform::from_translation(position),
            ..default()
        },
        PickupItem { name: name.to_string(), item_type: item_type.clone() },
        Collider { half_size: Vec3::new(mesh_size.x * 0.5, mesh_size.y * 0.5, mesh_size.z * 0.5) },
    )).with_children(|p| {
        // Floating indicator
        p.spawn((
            PbrBundle {
                mesh: orb,
                material: orb_mat,
                transform: Transform::from_xyz(0.0, mesh_size.y * 0.5 + 0.2, 0.0),
                ..default()
            },
        ));
    });
}

// =============================================================================
// Character Spawners
// =============================================================================

enum CharacterPreset { PlayerFire, EnemyIce, TeammateElectric }

fn spawn_player(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
) {
    // 发光饰条 = 当前干员的元素色（切换干员时由 switch_operator 重新着色）
    let accent = mat_emissive(materials, roster()[0].element.color());
    let eye = mat_emissive(materials, palette::EYE_BLUE);

    let mut root = commands.spawn((
        SpatialBundle { transform: Transform::from_translation(pos), ..default() },
        Player,
        PlayerMovement::default(),
        Health::default(),
        Armor::default(),
        WeaponSlot::default(),
        OperatorState::default(),
        OperatorAccent(accent.clone()),
        // 背包默认自带两把起步步枪与弹药池；武器架可在场上拾取扩充
        Inventory {
            items: vec![
                PickupItem { name: "医疗包".to_string(), item_type: PickupType::Health { amount: 30.0 } },
                PickupItem { name: "烈焰手雷".to_string(), item_type: PickupType::Grenade { element: ElementType::Fire } },
                PickupItem { name: "冰霜手雷".to_string(), item_type: PickupType::Grenade { element: ElementType::Ice } },
            ],
            ..Inventory::default()
        },
    ));
    root.with_children(|p| {
        // 玩家模型包进带标记的根：切干员时由 operator_model_swap_system 整体换模型
        // 默认焰狐（焦狐）；切到霜刃时换成冰系专属模型。敌人仍用通用 steve
        let mut model_root = p.spawn((
            SpatialBundle::default(),
            PlayerModelRoot { op_idx: 0 },
        ));
        model_root.with_children(|m| {
            build_yanhu(m, meshes, materials, &accent, &eye);
        });
    });
}

fn spawn_enemy(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    preset: CharacterPreset,
) {
    let (primary, accent_color, eye_color, helmet) = match preset {
        CharacterPreset::EnemyIce => (palette::ARMOR_DARK, ElementType::Ice.color(), palette::EYE_RED, true),
        CharacterPreset::TeammateElectric => (palette::TACTICAL_DARK, ElementType::Electric.color(), palette::EYE_BLUE, false),
        _ => (palette::TACTICAL_GREEN, ElementType::Fire.color(), palette::EYE_BLUE, true),
    };

    let body = mat_voxel(materials, primary);
    let skin = mat_voxel(materials, palette::SKIN);
    let accent = mat_emissive(materials, accent_color);
    let eye = mat_emissive(materials, eye_color);
    let armor = mat_voxel(materials, palette::ARMOR_GREY);
    let boot = mat_voxel(materials, palette::BOOTS);

    let mut root = commands.spawn((
        SpatialBundle { transform: Transform::from_translation(pos), ..default() },
        VoxelCharacter,
        // 阵营标记：小地图上敌我异色（PlayerFire 只用于玩家本体，不会走到这里）
        match preset {
            CharacterPreset::EnemyIce => Faction::Enemy,
            CharacterPreset::TeammateElectric | CharacterPreset::PlayerFire => Faction::Teammate,
        },
    ));
    root.with_children(|p| {
        build_steve(p, meshes, materials, helmet, false, &body, &skin, &accent, &eye, &armor, &boot);
    });
}

#[derive(Component)]
struct VoxelCharacter;

fn build_steve(
    parent: &mut ChildBuilder,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    helmet: bool,
    for_player: bool,
    body: &Handle<StandardMaterial>,
    skin: &Handle<StandardMaterial>,
    accent: &Handle<StandardMaterial>,
    eye: &Handle<StandardMaterial>,
    armor: &Handle<StandardMaterial>,
    boot: &Handle<StandardMaterial>,
) {
    // Head：包一层枢轴，玩家瞄准时做 Aim Offset 头部俯仰（敌人不需要）
    let mut head_pivot = parent.spawn(SpatialBundle {
        transform: Transform::from_xyz(0.0, 3.0, 0.0),
        ..default()
    });
    if for_player { head_pivot.insert(PlayerHeadPivot); }
    head_pivot.with_children(|h| {
        h.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            material: if helmet { armor.clone() } else { skin.clone() },
            ..default()
        });
        h.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(0.2, 0.15, 0.05)),
            material: eye.clone(),
            transform: Transform::from_xyz(-0.2, 0.05, 0.51),
            ..default()
        });
        h.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(0.2, 0.15, 0.05)),
            material: eye.clone(),
            transform: Transform::from_xyz(0.2, 0.05, 0.51),
            ..default()
        });
        h.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(0.3, 0.15, 0.3)),
            material: accent.clone(),
            transform: Transform::from_xyz(0.0, 0.55, 0.0),
            ..default()
        });
    });

    // Torso
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(1.0, 1.5, 0.5)),
        material: body.clone(),
        transform: Transform::from_xyz(0.0, 1.75, 0.0),
        ..default()
    });
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(0.3, 0.3, 0.05)),
        material: accent.clone(),
        transform: Transform::from_xyz(0.0, 2.0, 0.26),
        ..default()
    });
    // Shoulders
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(0.4, 0.4, 0.4)),
        material: armor.clone(),
        transform: Transform::from_xyz(-0.7, 2.3, 0.0),
        ..default()
    });
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(0.4, 0.4, 0.4)),
        material: armor.clone(),
        transform: Transform::from_xyz(0.7, 2.3, 0.0),
        ..default()
    });

    // Arms
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(0.5, 1.5, 0.5)),
        material: skin.clone(),
        transform: Transform::from_xyz(-0.75, 1.75, 0.0),
        ..default()
    });
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(0.55, 0.6, 0.55)),
        material: armor.clone(),
        transform: Transform::from_xyz(-0.75, 2.2, 0.0),
        ..default()
    });
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(0.5, 1.5, 0.5)),
        material: skin.clone(),
        transform: Transform::from_xyz(0.75, 1.75, 0.0),
        ..default()
    });
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(0.55, 0.6, 0.55)),
        material: armor.clone(),
        transform: Transform::from_xyz(0.75, 2.2, 0.0),
        ..default()
    });

    // Gun：整体包一层枢轴；玩家瞄准时从腰际举到肩上（程序化持枪姿态）
    let gun = mat_voxel(materials, Color::srgb(0.3, 0.3, 0.35));
    let mut gun_pivot = parent.spawn(SpatialBundle {
        transform: Transform::from_xyz(0.4, 1.3, 0.6),
        ..default()
    });
    if for_player {
        gun_pivot.insert(PlayerAimGun {
            base: Vec3::new(0.4, 1.3, 0.6),
            raised: Vec3::new(0.4, 2.35, 0.5),
        });
    }
    gun_pivot.with_children(|g| {
        g.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(0.15, 0.15, 1.2)),
            material: gun.clone(),
            ..default()
        });
        g.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(0.08, 0.08, 0.3)),
            material: accent.clone(),
            transform: Transform::from_xyz(0.0, 0.08, -0.1),
            ..default()
        });
        g.spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(0.12, 0.25, 0.4)),
            material: gun,
            transform: Transform::from_xyz(0.0, -0.1, -0.7),
            ..default()
        });
    });

    // Legs
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(0.5, 1.5, 0.5)),
        material: body.clone(),
        transform: Transform::from_xyz(-0.25, 0.75, 0.0),
        ..default()
    });
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(0.55, 0.4, 0.6)),
        material: boot.clone(),
        transform: Transform::from_xyz(-0.25, 0.2, 0.05),
        ..default()
    });
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(0.5, 1.5, 0.5)),
        material: body.clone(),
        transform: Transform::from_xyz(0.25, 0.75, 0.0),
        ..default()
    });
    parent.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(0.55, 0.4, 0.6)),
        material: boot.clone(),
        transform: Transform::from_xyz(0.25, 0.2, 0.05),
        ..default()
    });
}


// =============================================================================
// Minimap —— 左上角正方形小地图 + 相机朝向罗盘条
// =============================================================================
//
// 结构与数据来源：
// - 掩体/站点来自核心库 `crate::map` 的 MapLayout（一次性静态摆位，障碍不动）；
// - 玩家/敌人/队友/训练靶/拾取物为动态实体，每帧映射到地图像素坐标；
// - 地图朝向固定为"正北朝上"（-Z 方向），玩家点随位置移动；
// - 地图上方的罗盘条：刻度随相机 yaw 滚动，中央指针 + 方位读数固定，
//   方位约定 正北=0°、正东=90°、正南=180°、正西=270°。

/// 小地图边长（像素）
const MINIMAP_PX: f32 = 180.0;
/// 罗盘条高度（像素）
const COMPASS_H: f32 = 26.0;
/// 地图框边宽：绘制区内缩，避免掩体贴到边框上
const MAP_BORDER: f32 = 2.0;
/// 小地图绘制区边长（扣除边框）
const MAP_INNER: f32 = MINIMAP_PX - 2.0 * MAP_BORDER;

#[derive(Component)]
struct MinimapRoot;

/// 角色阵营：小地图上敌我异色
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Faction { Enemy, Teammate }

/// 小地图需要的地图数据快照。
/// 核心库不依赖 bevy，MapLayout 本身不是 Resource，由 demo 侧包一层。
#[derive(Resource)]
struct MinimapMap { half_extent: f32 }

/// 玩家定位点（白色小点）
#[derive(Component)]
struct MinimapPlayerDot;

/// 玩家朝向菱形（绕自身中心旋转，UI 节点 Transform 平移基准为节点中心）
#[derive(Component)]
struct MinimapPlayerArrow;

/// 小地图动态点池的一格：靶/拾取物/敌人/队友共用，按 kind 分组编号
#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct MinimapDot { kind: DotKind, slot: usize }

#[derive(Clone, Copy, PartialEq, Eq)]
enum DotKind { Target, Pickup, Enemy, Teammate }

/// 罗盘刻度：angle 为世界方位角（度，正北=0，顺时针增加）
#[derive(Component)]
struct CompassTick { angle: f32 }

/// 罗盘方位字（北/东/南/西），随刻度一起滚动
#[derive(Component)]
struct CompassLabel { angle: f32 }

/// 罗盘中央方位读数（如"正北 0°"）
#[derive(Component)]
struct CompassHeadingText;

/// 世界坐标（x 向东 / z 向南）→ 小地图像素 x/y（左上角为西北角，正北朝上）
fn world_to_map(v: f32, half_extent: f32) -> f32 {
    MAP_BORDER + (v + half_extent) / (2.0 * half_extent) * MAP_INNER
}

/// 归一化角度差到 [-180, 180)
fn wrap_deg(a: f32) -> f32 {
    a - 360.0 * ((a + 180.0) / 360.0).floor()
}

/// 相机 yaw 弧度 → 方位角弧度（正北=0，顺时针）。
/// yaw=π 时面向 -Z（正北）；yaw=π/2 面向 +X（正东）。
fn heading_rad(yaw: f32) -> f32 {
    (std::f32::consts::PI - yaw).rem_euclid(std::f32::consts::TAU)
}

fn setup_minimap(mut commands: Commands) {
    let layout = training::layout();
    let half = layout.half_extent;

    // 根节点：左上角，纵向排列 罗盘条 + 方形地图（边距与右上角击杀播报一致）
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(14.0),
                left: Val::Px(16.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            background_color: BackgroundColor(Color::NONE),
            ..default()
        },
        MinimapRoot,
    )).with_children(|root| {
        // ---- 罗盘条：刻度滚动，中央指针 + 读数固定 ----
        root.spawn(NodeBundle {
            style: Style {
                width: Val::Px(MINIMAP_PX),
                height: Val::Px(COMPASS_H),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.7)),
            ..default()
        }).with_children(|strip| {
            // 每 15° 一根刻度；45° 倍数加高，正方位最亮（方位字单列）
            for deg in (0..360).step_by(15) {
                let major = deg % 45 == 0;
                let cardinal = deg % 90 == 0;
                strip.spawn((
                    NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            top: Val::Px(0.0),
                            width: Val::Px(if cardinal { 3.0 } else { 2.0 }),
                            height: Val::Px(if major { 9.0 } else { 6.0 }),
                            ..default()
                        },
                        background_color: BackgroundColor(if cardinal {
                            Color::srgba(0.95, 0.95, 0.95, 0.9)
                        } else if major {
                            Color::srgba(0.85, 0.85, 0.85, 0.55)
                        } else {
                            Color::srgba(0.7, 0.7, 0.7, 0.3)
                        }),
                        ..default()
                    },
                    CompassTick { angle: deg as f32 },
                ));
            }
            // 四个方位字随刻度滚动
            for (deg, name) in [(0.0, "北"), (90.0, "东"), (180.0, "南"), (270.0, "西")] {
                strip.spawn((
                    NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            top: Val::Px(9.0),
                            width: Val::Px(14.0),
                            height: Val::Px(12.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: BackgroundColor(Color::NONE),
                        ..default()
                    },
                    CompassLabel { angle: deg },
                )).with_children(|label| {
                    label.spawn(TextBundle {
                        text: Text::from_section(
                            name,
                            TextStyle { font_size: 10.0, color: Color::srgb(0.95, 0.95, 0.95), ..default() },
                        ),
                        ..default()
                    });
                });
            }
            // 中央指针（固定）
            strip.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(0.0),
                    left: Val::Px(MINIMAP_PX * 0.5 - 1.0),
                    width: Val::Px(2.0),
                    height: Val::Px(8.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgb(1.0, 0.8, 0.25)),
                ..default()
            });
            // 中央方位读数（in-flow 子节点，由 justify_content 居中）
            strip.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(64.0),
                    height: Val::Px(14.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)),
                ..default()
            }).with_children(|readout| {
                readout.spawn((
                    TextBundle {
                        text: Text::from_section(
                            "正北 0°",
                            TextStyle { font_size: 10.0, color: Color::srgb(0.95, 0.95, 0.9), ..default() },
                        ),
                        ..default()
                    },
                    CompassHeadingText,
                ));
            });
        });

        // ---- 方形小地图 ----
        root.spawn(NodeBundle {
            style: Style {
                width: Val::Px(MINIMAP_PX),
                height: Val::Px(MINIMAP_PX),
                border: UiRect::all(Val::Px(MAP_BORDER)),
                overflow: Overflow::clip(),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.04, 0.05, 0.07, 0.78)),
            border_color: BorderColor(Color::srgba(0.6, 0.63, 0.67, 0.9)),
            ..default()
        }).with_children(|map| {
            // 静态掩体：一次性摆位（障碍不动）；只画有碰撞且高过膝的，
            // 标线/管道等无碰撞装饰不上图
            for prop in &layout.props {
                if !prop.solid { continue; }
                let aabb = prop.aabb_half();
                if prop.pos[1] + aabb[1] <= 0.5 { continue; }
                let scale = MAP_INNER / (2.0 * half);
                let w = (aabb[0] * 2.0 * scale).max(2.0);
                let h = (aabb[2] * 2.0 * scale).max(2.0);
                map.spawn(NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(world_to_map(prop.pos[0], half) - w * 0.5),
                        top: Val::Px(world_to_map(prop.pos[2], half) - h * 0.5),
                        width: Val::Px(w),
                        height: Val::Px(h),
                        ..default()
                    },
                    background_color: BackgroundColor(minimap_material_color(prop.material)),
                    ..default()
                });
            }
            // 功能站点：补给台（琥珀）/ 干员切换台（青）
            for station in &layout.stations {
                let color = match station.kind {
                    StationKind::SupplyTable => Color::srgb(1.0, 0.65, 0.15),
                    StationKind::OperatorDesk => Color::srgb(0.2, 0.9, 0.95),
                };
                map.spawn(NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(world_to_map(station.pos[0], half) - 3.0),
                        top: Val::Px(world_to_map(station.pos[2], half) - 3.0),
                        width: Val::Px(6.0),
                        height: Val::Px(6.0),
                        ..default()
                    },
                    background_color: BackgroundColor(color),
                    ..default()
                });
            }
            // 动态点池：初始隐藏，minimap_update_system 每帧填充
            for (kind, count) in [
                (DotKind::Target, 12),
                (DotKind::Pickup, 16),
                (DotKind::Enemy, 4),
                (DotKind::Teammate, 4),
            ] {
                for slot in 0..count {
                    let size = dot_size(kind);
                    map.spawn((
                        NodeBundle {
                            style: Style {
                                position_type: PositionType::Absolute,
                                width: Val::Px(size),
                                height: Val::Px(size),
                                display: Display::None,
                                ..default()
                            },
                            background_color: BackgroundColor(Color::NONE),
                            ..default()
                        },
                        MinimapDot { kind, slot },
                    ));
                }
            }
            // 玩家：白色定位点 + 朝向菱形（后生成者在上层）
            map.spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(world_to_map(layout.player_spawn[0], half) - 2.0),
                        top: Val::Px(world_to_map(layout.player_spawn[2], half) - 2.0),
                        width: Val::Px(4.0),
                        height: Val::Px(4.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgb(0.95, 0.95, 0.95)),
                    ..default()
                },
                MinimapPlayerDot,
            ));
            map.spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(world_to_map(layout.player_spawn[0], half) - 5.5),
                        top: Val::Px(world_to_map(layout.player_spawn[2], half) - 5.5),
                        width: Val::Px(11.0),
                        height: Val::Px(11.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.35, 0.95, 0.6, 0.5)),
                    transform: Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
                    ..default()
                },
                MinimapPlayerArrow,
            ));
        });
    });

    commands.insert_resource(MinimapMap { half_extent: layout.half_extent });
}

/// 掩体材质 → 小地图配色（数据层只声明材质语义，颜色由表现层映射）
fn minimap_material_color(m: MaterialKind) -> Color {
    match m {
        MaterialKind::Concrete => Color::srgba(0.58, 0.60, 0.64, 0.85),
        MaterialKind::Rust => Color::srgba(0.56, 0.42, 0.30, 0.85),
        MaterialKind::Steel => Color::srgba(0.40, 0.43, 0.47, 0.85),
        MaterialKind::TargetRed => Color::srgba(0.85, 0.28, 0.22, 0.85),
        MaterialKind::TargetWhite | MaterialKind::PaintWhite => Color::srgba(0.80, 0.80, 0.82, 0.85),
        MaterialKind::Dark => Color::srgba(0.25, 0.26, 0.30, 0.85),
        MaterialKind::Pipe => Color::srgba(0.52, 0.47, 0.42, 0.85),
        MaterialKind::WarningYellow => Color::srgba(0.85, 0.75, 0.20, 0.85),
        MaterialKind::WarningOrange => Color::srgba(0.85, 0.50, 0.15, 0.85),
        MaterialKind::WarningRed => Color::srgba(0.85, 0.25, 0.15, 0.85),
    }
}

/// 拾取物 → 小地图点色（与场上发光色一致）
fn minimap_pickup_color(item: &PickupType) -> Color {
    match item {
        PickupType::Ammo { .. } => Color::srgba(0.9, 0.7, 0.2, 0.95),
        PickupType::Health { .. } => Color::srgba(0.9, 0.25, 0.25, 0.95),
        PickupType::Armor { .. } => Color::srgba(0.3, 0.55, 0.95, 0.95),
        PickupType::Grenade { element } | PickupType::Weapon { element } => element.color().with_alpha(0.95),
    }
}

fn dot_size(kind: DotKind) -> f32 {
    match kind {
        DotKind::Target => 5.0,
        DotKind::Pickup => 4.0,
        DotKind::Enemy | DotKind::Teammate => 6.0,
    }
}

/// 每帧更新：罗盘滚动 + 方位读数 + 玩家/动态实体落图。
/// 各 Style 可变查询用互斥的 With 标记隔离，避免 B0001 运行时冲突。
#[allow(clippy::type_complexity)]
fn minimap_update_system(
    cam_query: Query<&PlayerCamera>,
    player_query: Query<&Transform, With<Player>>,
    map: Res<MinimapMap>,
    characters: Query<(&Transform, &Faction), Without<Player>>,
    targets: Query<&Transform, (With<TargetDummy>, Without<Player>, Without<Faction>)>,
    pickups: Query<(&Transform, &PickupItem), (Without<Player>, Without<Faction>, Without<TargetDummy>)>,
    mut dots: Query<
        (&MinimapDot, &mut Style, &mut BackgroundColor),
        (Without<MinimapPlayerDot>, Without<MinimapPlayerArrow>, Without<CompassTick>, Without<CompassLabel>),
    >,
    mut ticks: Query<
        (&CompassTick, &mut Style),
        (Without<MinimapDot>, Without<MinimapPlayerDot>, Without<MinimapPlayerArrow>, Without<CompassLabel>),
    >,
    mut labels: Query<
        (&CompassLabel, &mut Style),
        (Without<MinimapDot>, Without<MinimapPlayerDot>, Without<MinimapPlayerArrow>, Without<CompassTick>),
    >,
    mut player_dot: Query<
        &mut Style,
        (With<MinimapPlayerDot>, Without<MinimapPlayerArrow>, Without<MinimapDot>, Without<CompassTick>, Without<CompassLabel>),
    >,
    mut player_arrow: Query<
        (&mut Style, &mut Transform),
        // &mut Transform 须与上方四处 &Transform 读访问逐一对立，否则 B0001
        (With<MinimapPlayerArrow>, Without<MinimapPlayerDot>, Without<MinimapDot>, Without<CompassTick>, Without<CompassLabel>,
         Without<Player>, Without<Faction>, Without<TargetDummy>, Without<PickupItem>),
    >,
    mut heading_text: Query<&mut Text, With<CompassHeadingText>>,
) {
    let Ok(cam) = cam_query.get_single() else { return };
    let Ok(player) = player_query.get_single() else { return };
    let half = map.half_extent;

    // ---- 罗盘：刻度/方位字按 1px=1° 滚动，超出可视范围隐藏 ----
    let heading = heading_rad(cam.yaw);
    let heading_deg = heading.to_degrees();
    for (tick, mut style) in ticks.iter_mut() {
        let rel = wrap_deg(tick.angle - heading_deg);
        style.left = Val::Px(MINIMAP_PX * 0.5 + rel - 1.0);
        style.display = if rel.abs() <= 95.0 { Display::Flex } else { Display::None };
    }
    for (label, mut style) in labels.iter_mut() {
        let rel = wrap_deg(label.angle - heading_deg);
        style.left = Val::Px(MINIMAP_PX * 0.5 + rel - 7.0);
        style.display = if rel.abs() <= 95.0 { Display::Flex } else { Display::None };
    }

    // 中央读数：八方位名 + 角度
    const DIR_NAMES: [&str; 8] = ["正北", "东北", "正东", "东南", "正南", "西南", "正西", "西北"];
    if let Ok(mut text) = heading_text.get_single_mut() {
        let dir = DIR_NAMES[((heading_deg + 22.5) / 45.0) as usize % 8];
        text.sections[0].value = format!("{} {:.0}°", dir, heading_deg);
    }

    // ---- 玩家标记：定位点居中，菱形指向相机朝向 ----
    let px = world_to_map(player.translation.x, half);
    let py = world_to_map(player.translation.z, half);
    if let Ok(mut style) = player_dot.get_single_mut() {
        style.left = Val::Px(px - 2.0);
        style.top = Val::Px(py - 2.0);
    }
    if let Ok((mut style, mut transform)) = player_arrow.get_single_mut() {
        style.left = Val::Px(px - 5.5);
        style.top = Val::Px(py - 5.5);
        // UI 屏幕 y 向下，rotation.z 正值在屏上表现为顺时针，恰与罗盘方位一致
        transform.rotation = Quat::from_rotation_z(heading + std::f32::consts::FRAC_PI_4);
    }

    // ---- 动态实体 → 地图像素 ----
    let to_px = |t: &Transform| [world_to_map(t.translation.x, half), world_to_map(t.translation.z, half)];
    let target_px: Vec<[f32; 2]> = targets.iter().map(to_px).collect();
    let pickup_px: Vec<([f32; 2], Color)> = pickups
        .iter()
        .map(|(t, item)| (to_px(t), minimap_pickup_color(&item.item_type)))
        .collect();
    let mut enemy_px: Vec<[f32; 2]> = Vec::new();
    let mut teammate_px: Vec<[f32; 2]> = Vec::new();
    for (t, faction) in characters.iter() {
        match faction {
            Faction::Enemy => enemy_px.push(to_px(t)),
            Faction::Teammate => teammate_px.push(to_px(t)),
        }
    }

    for (dot, mut style, mut bg) in dots.iter_mut() {
        let entry = match dot.kind {
            DotKind::Target => target_px.get(dot.slot).map(|p| (*p, Color::srgba(0.95, 0.35, 0.3, 0.9))),
            DotKind::Pickup => pickup_px.get(dot.slot).map(|(p, c)| (*p, *c)),
            DotKind::Enemy => enemy_px.get(dot.slot).map(|p| (*p, Color::srgba(1.0, 0.3, 0.25, 0.95))),
            DotKind::Teammate => teammate_px.get(dot.slot).map(|p| (*p, Color::srgba(0.25, 0.9, 0.45, 0.95))),
        };
        if let Some(([x, y], color)) = entry {
            let s = dot_size(dot.kind) * 0.5;
            style.display = Display::Flex;
            style.left = Val::Px(x - s);
            style.top = Val::Px(y - s);
            bg.0 = color;
        } else {
            style.display = Display::None;
        }
    }
}

// =============================================================================
// HUD Setup - Polished FPS Interface
// =============================================================================

fn setup_hud(mut commands: Commands) {
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
fn spawn_item_icon(skills: &mut ChildBuilder, slot: usize, key: &str, label: &str, color: Color) {
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
enum EffectMatKind {
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
struct EffectAssets {
    /// 曳光：单位深度细长盒，使用时按弹道长度缩放 Z
    tracer: Handle<Mesh>,
    /// 火花小球：枪口焰与命中爆闪共用（尺寸靠缩放区分）
    spark: Handle<Mesh>,
    /// 伤害粒子：小立方体
    particle: Handle<Mesh>,
    /// 爆炸主体：半透明大球（缩放由 ExplosionEffect 驱动）
    explosion_sphere: Handle<Mesh>,
    /// 爆炸碎块：小立方体
    explosion_debris: Handle<Mesh>,
    /// 手雷/爆裂投掷物：小球
    projectile: Handle<Mesh>,
    /// 冰冻冰块：罩住目标半透明立方体
    frost_cube: Handle<Mesh>,
    /// 毒雾/持续区域：单位圆柱，按区域半径缩放 XZ
    zone_cylinder: Handle<Mesh>,
    /// 冰冻冰块材质（半透明淡蓝，全目标共用）
    frost_material: Handle<StandardMaterial>,
    /// 枪口焰材质（固定暖白）
    muzzle_material: Handle<StandardMaterial>,
    /// (元素, 变体) → 材质 缓存
    element_materials: Vec<(ElementType, EffectMatKind, Handle<StandardMaterial>)>,
}

impl EffectAssets {
    fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
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
    fn material(
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

fn fps_controller(
    mut player_query: Query<(&mut Transform, &mut PlayerMovement), With<Player>>,
    mut cam_query: Query<&mut PlayerCamera>,
    mut mouse_events: EventReader<MouseMotion>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    input_state: Res<InputState>,
    settings: Res<GameSettings>,
    colliders: Query<(&Transform, &Collider), Without<Player>>,
) {
    if !input_state.cursor_locked {
        // Clear mouse events so they don't accumulate
        mouse_events.clear();
        return;
    }

    let dt = time.delta_seconds();
    let mut delta = Vec2::ZERO;
    for ev in mouse_events.read() { delta += ev.delta; }

    let Ok(mut cam) = cam_query.get_single_mut() else { return };
    if delta != Vec2::ZERO {
        // 鼠标 X → 肩轴 Yaw，Y → 俯仰 Pitch；两轴灵敏度独立
        cam.yaw -= delta.x * MOUSE_SENS_X * settings.mouse_sensitivity;
        cam.pitch += delta.y * MOUSE_SENS_Y * settings.mouse_sensitivity;
        // 俯仰限制：仰视 50° / 俯视 70°（pitch 正值 = 俯视）
        cam.pitch = cam.pitch.clamp(-PITCH_UP_MAX, PITCH_DOWN_MAX);
    }

    let yaw = cam.yaw;
    for (mut transform, mut movement) in player_query.iter_mut() {
        // 角色朝向：瞄准时以最短角差 + 指数平滑追相机朝向（等效球面插值，
        // 快速转身不抖动）；普通视角保持原有的即时同向
        if cam.aiming {
            let diff = shortest_angle(movement.model_yaw, yaw);
            movement.model_yaw += diff * (1.0 - (-AIM_TURN_RATE * dt).exp());
        } else {
            movement.model_yaw = yaw;
        }
        transform.rotation = Quat::from_rotation_y(movement.model_yaw);
        let mut input = Vec3::ZERO;
        if keyboard.pressed(KeyCode::KeyW) { input.z += 1.0; }
        if keyboard.pressed(KeyCode::KeyS) { input.z -= 1.0; }
        if keyboard.pressed(KeyCode::KeyA) { input.x -= 1.0; }
        if keyboard.pressed(KeyCode::KeyD) { input.x += 1.0; }

        // 移动输入始终相对相机方向（W = 相机前方）；瞄准时移速降至 55%
        let mut speed = if keyboard.pressed(KeyCode::ShiftLeft) { 7.0 } else { 4.0 };
        if cam.aiming { speed *= AIM_MOVE_FACTOR; }
        // 动作系统读这个值决定步频/摆幅（撞墙时仍保持走姿，输入在即视为移动）
        movement.planar_speed = if input != Vec3::ZERO { speed } else { 0.0 };
        if input != Vec3::ZERO {
            input = input.normalize();
            let forward = Vec3::new(yaw.sin(), 0.0, yaw.cos());
            let right = forward.cross(Vec3::Y);
            let desired_move = (forward * input.z + right * input.x) * speed * dt;
            let new_pos = transform.translation + desired_move;
            // Simple AABB collision: resolve X and Z separately
            let player_half = Vec3::new(0.4, 1.0, 0.4);
            let mut resolved = transform.translation;
            // Try X move
            let try_x = Vec3::new(new_pos.x, resolved.y, resolved.z);
            if !collides(try_x, player_half, &colliders) {
                resolved.x = try_x.x;
            }
            // Try Z move
            let try_z = Vec3::new(resolved.x, resolved.y, new_pos.z);
            if !collides(try_z, player_half, &colliders) {
                resolved.z = try_z.z;
            }
            transform.translation = resolved;
        }

        if keyboard.just_pressed(KeyCode::Space) && movement.is_grounded {
            movement.velocity.y = 6.5;
            movement.is_grounded = false;
        }
        if !movement.is_grounded {
            movement.velocity.y -= 18.0 * dt;
            let mut new_y = transform.translation + movement.velocity * dt;
            // Collision for vertical falling
            let player_half = Vec3::new(0.4, 1.0, 0.4);
            if collides(new_y, player_half, &colliders) && movement.velocity.y < 0.0 {
                movement.velocity.y = 0.0;
                movement.is_grounded = true;
                // Snap to nearest safe Y
                let step = 0.1;
                for _ in 0..20 {
                    new_y.y += step;
                    if !collides(new_y, player_half, &colliders) {
                        transform.translation = new_y;
                        break;
                    }
                }
            } else {
                transform.translation = new_y;
                if transform.translation.y <= 0.0 {
                    transform.translation.y = 0.0;
                    movement.velocity.y = 0.0;
                    movement.is_grounded = true;
                }
            }
        }
    }
}

fn collides(
    player_pos: Vec3,
    player_half: Vec3,
    colliders: &Query<(&Transform, &Collider), Without<Player>>,
) -> bool {
    for (t, c) in colliders.iter() {
        let min_a = player_pos - player_half;
        let max_a = player_pos + player_half;
        let min_b = t.translation - c.half_size;
        let max_b = t.translation + c.half_size;
        if min_a.x < max_b.x && max_a.x > min_b.x
            && min_a.y < max_b.y && max_a.y > min_b.y
            && min_a.z < max_b.z && max_a.z > min_b.z
        {
            return true;
        }
    }
    false
}

// =============================================================================
// 越肩第三人称瞄准（SpringArm 相机架构，参考原神弓手瞄准模式）
// =============================================================================

// —— 相机装配 ——
#[derive(Component)]
/// 顶级枢轴：位于角色脚底中心，不随角色模型旋转（TopLevel），每帧跟随角色位置
pub struct CamPivot;

#[derive(Component)]
/// 偏航控制节点（鼠标 X 轴）
pub struct ShoulderPivot;

#[derive(Component)]
/// 俯仰控制节点（鼠标 Y 轴）
pub struct PitchPivot;

#[derive(Component)]
/// 弹簧臂：本地 Z = 后方距离，相机挂在其末端
pub struct SpringArm;

#[derive(Component)]
struct SpringArmState { len: f32 }


// —— SpringArm 参数（占位值已按 3.7m 体型的体素模型校准） ——
/// 俯仰枢轴高度（相对脚底 Pivot，模型颈肩处；头中心在 3.0）
const PIVOT_HEIGHT: f32 = 2.6;
/// 默认机位：右肩水平偏移 / 垂直偏移（附加于枢轴高度）/ 后方距离
const ARM_SHOULDER_X_NORMAL: f32 = 0.55;
const ARM_EYE_Y_NORMAL: f32 = 0.35;
const ARM_LEN_NORMAL: f32 = 6.5;
/// 越肩机位（瞄准）
const ARM_SHOULDER_X_AIM: f32 = 1.0;
const ARM_EYE_Y_AIM: f32 = 0.3;
const ARM_LEN_AIM: f32 = 2.4;
/// 正常 ↔ 瞄准过渡耗时（秒），smoothstep
const ARM_TRANSITION_SECS: f32 = 0.22;
/// 弹簧臂碰撞：贴墙最小距离 / 相机最低高度
const ARM_MIN_LEN: f32 = 0.7;
const ARM_GROUND_MIN_Y: f32 = 0.35;
/// 弹簧臂伸缩平滑：缩回快（避障跟手）、伸出慢（防抖）
const ARM_RETRACT_RATE: f32 = 30.0;
const ARM_EXTEND_RATE: f32 = 5.0;

// —— 瞄准行为 ——
/// 瞄准时移动速度降至 55%
const AIM_MOVE_FACTOR: f32 = 0.55;
/// 瞄准时角色转身插值速率（指数平滑）
const AIM_TURN_RATE: f32 = 12.0;
/// 俯仰限制：仰视 50°（pitch 取负）/ 俯视 70°（pitch 正值 = 俯视）
const PITCH_UP_MAX: f32 = 0.873;   // 50°
const PITCH_DOWN_MAX: f32 = 1.217; // 70°
/// 鼠标灵敏度：X 轴（Yaw）/ Y 轴（Pitch），弧度/像素，再乘设置里的灵敏度
const MOUSE_SENS_X: f32 = 0.0024;
const MOUSE_SENS_Y: f32 = 0.0021;
/// 头部 Aim Offset 最大俯仰（约为视线俯仰的 45%，上限 25°）
const HEAD_AIM_RATIO: f32 = 0.45;
const HEAD_AIM_MAX: f32 = 0.44;

// —— 弱点打击 ——
/// 命中靶心核心的额外伤害倍率
const WEAKPOINT_MULT: f32 = 1.8;
/// 靶心核心判定球半径（对应靶板内环白圈；人形敌人的头部弱点半径同为 0.3~0.55）
const WEAKPOINT_CORE_R: f32 = 0.3;

pub fn lerp(a: f32, b: f32, t: f32) -> f32 { a + (b - a) * t }

/// 最短角差：把 to - from 收拢到 [-π, π]，供转身平滑使用
fn shortest_angle(from: f32, to: f32) -> f32 {
    let d = (to - from).rem_euclid(std::f32::consts::TAU);
    if d > std::f32::consts::PI { d - std::f32::consts::TAU } else { d }
}

/// 射线 vs AABB（slab 法），返回最近正向命中距离
fn ray_aabb_hit(origin: Vec3, dir: Vec3, half: Vec3, center: Vec3) -> Option<f32> {
    let mut tmin = 0.0f32;
    let mut tmax = f32::MAX;
    for axis in 0..3 {
        let o = origin[axis]; let d = dir[axis]; let h = half[axis]; let c = center[axis];
        if d.abs() < 1e-6 {
            if (o - c).abs() > h { return None; }
            continue;
        }
        let mut t1 = (c - h - o) / d;
        let mut t2 = (c + h - o) / d;
        if t1 > t2 { std::mem::swap(&mut t1, &mut t2); }
        tmin = tmin.max(t1);
        tmax = tmax.min(t2);
        if tmin > tmax { return None; }
    }
    if tmin > 0.0 { Some(tmin) } else { None }
}

/// 射线 vs 球，返回最近正向命中距离
fn ray_sphere_hit(origin: Vec3, dir: Vec3, center: Vec3, radius: f32) -> Option<f32> {
    let oc = center - origin;
    let proj = oc.dot(dir);
    if proj < 0.0 { return None; }
    let d2 = oc.length_squared() - proj * proj;
    let r2 = radius * radius;
    if d2 > r2 { return None; }
    Some(proj - (r2 - d2).sqrt())
}

/// 越肩相机每帧驱动：
/// 1) Pivot 跟随角色脚底；ShoulderPivot/Yaw、PitchPivot/Pitch 写入鼠标视角
/// 2) SpringArm 在默认右肩机位与越肩机位间按 aim_lerp 过渡（smoothstep）
/// 3) 臂长做射线避障（撞墙缩回、离墙缓伸，最小距离钳制 + 地面钳制）
/// 4) 头部俯仰跟随视线、枪械从腰际举到肩上（程序化 Aim Offset）
fn aim_rig_system(
    player_query: Query<&Transform, With<Player>>,
    cam_query: Query<&PlayerCamera>,
    mut pivot_query: Query<&mut Transform, (With<CamPivot>, Without<Player>)>,
    mut yaw_query: Query<&mut Transform, (With<ShoulderPivot>, Without<CamPivot>, Without<Player>)>,
    mut pitch_query: Query<&mut Transform, (With<PitchPivot>, Without<CamPivot>, Without<ShoulderPivot>, Without<Player>)>,
    mut arm_query: Query<(&mut Transform, &mut SpringArmState), (With<SpringArm>, Without<CamPivot>, Without<ShoulderPivot>, Without<PitchPivot>, Without<Player>)>,
    mut head_query: Query<&mut Transform, (With<PlayerHeadPivot>, Without<CamPivot>, Without<ShoulderPivot>, Without<PitchPivot>, Without<SpringArm>, Without<PlayerAimGun>, Without<Player>)>,
    mut gun_query: Query<(&PlayerAimGun, &mut Transform), (Without<CamPivot>, Without<ShoulderPivot>, Without<PitchPivot>, Without<SpringArm>, Without<PlayerHeadPivot>, Without<Player>)>,
    colliders: Query<(&Transform, &Collider), (Without<CamPivot>, Without<ShoulderPivot>, Without<PitchPivot>, Without<SpringArm>, Without<PlayerHeadPivot>, Without<PlayerAimGun>, Without<Player>)>,
    time: Res<Time>,
) {
    let Ok(player_transform) = player_query.get_single() else { return };
    let Ok(cam) = cam_query.get_single() else { return };

    // 瞄准过渡系数（smoothstep）
    let step = time.delta_seconds() / ARM_TRANSITION_SECS;
    let aim_lerp = if cam.aiming { (cam.aim_lerp + step).min(1.0) } else { (cam.aim_lerp - step).max(0.0) };
    let t = aim_lerp * aim_lerp * (3.0 - 2.0 * aim_lerp);

    let pitch = cam.pitch.clamp(-PITCH_UP_MAX, PITCH_DOWN_MAX);
    // 俯仰旋转量（rotation_x 正值 = 抬头，取负与"pitch 正 = 俯视"约定对齐）
    let rot = Quat::from_rotation_y(cam.yaw + std::f32::consts::PI) * Quat::from_rotation_x(-pitch);

    if let Ok(mut pivot) = pivot_query.get_single_mut() {
        pivot.translation = player_transform.translation;
    }
    if let Ok(mut yaw_p) = yaw_query.get_single_mut() {
        yaw_p.rotation = Quat::from_rotation_y(cam.yaw + std::f32::consts::PI);
    }
    if let Ok(mut pitch_p) = pitch_query.get_single_mut() {
        pitch_p.rotation = Quat::from_rotation_x(-pitch);
    }

    // 机位参数插值
    let shoulder_x = lerp(ARM_SHOULDER_X_NORMAL, ARM_SHOULDER_X_AIM, t);
    let eye_y = lerp(ARM_EYE_Y_NORMAL, ARM_EYE_Y_AIM, t);
    let want_len = lerp(ARM_LEN_NORMAL, ARM_LEN_AIM, t);

    // 弹簧臂避障：从臂根（肩位）沿臂方向打射线，撞墙缩回，地面钳制
    let base = player_transform.translation + PIVOT_HEIGHT * Vec3::Y + rot * Vec3::new(shoulder_x, eye_y, 0.0);
    let back = rot * Vec3::Z;
    let mut blocked = want_len;
    for (col_t, col) in colliders.iter() {
        if let Some(d) = ray_aabb_hit(base, back, col.half_size, col_t.translation) {
            if d < blocked { blocked = d; }
        }
    }
    if back.y < -1e-4 {
        let t_ground = (ARM_GROUND_MIN_Y - base.y) / back.y;
        if t_ground > 0.0 && t_ground < blocked { blocked = t_ground; }
    }
    let target_len = blocked.max(ARM_MIN_LEN).min(want_len);

    if let Ok((mut arm, mut state)) = arm_query.get_single_mut() {
        let rate = if target_len < state.len { ARM_RETRACT_RATE } else { ARM_EXTEND_RATE };
        state.len = lerp(state.len, target_len, 1.0 - (-rate * time.delta_seconds()).exp());
        arm.translation = Vec3::new(shoulder_x, eye_y, state.len);
    }

    // 程序化 Aim Offset：头部随视线俯仰（约 45%，上限 25°），枪从腰际举到肩上
    let head_pitch = (-pitch * HEAD_AIM_RATIO).clamp(-HEAD_AIM_MAX, HEAD_AIM_MAX) * aim_lerp;
    if let Ok(mut head) = head_query.get_single_mut() {
        head.rotation = Quat::from_rotation_x(head_pitch);
    }
    if let Ok((gun, mut gun_t)) = gun_query.get_single_mut() {
        gun_t.translation = gun.base.lerp(gun.raised, t);
        gun_t.rotation = Quat::from_rotation_x(-pitch * 0.6 * aim_lerp);
    }
}

/// 推进 aim_lerp（与 aim_rig_system 拆开，避免只读/可变借用交织）
fn aim_lerp_system(
    mut cam_query: Query<&mut PlayerCamera>,
    time: Res<Time>,
) {
    let step = time.delta_seconds() / ARM_TRANSITION_SECS;
    for mut cam in &mut cam_query {
        cam.aim_lerp = if cam.aiming { (cam.aim_lerp + step).min(1.0) } else { (cam.aim_lerp - step).max(0.0) };
    }
}


/// 瞄准输入：按住右键进入越肩瞄准；手持手雷时强制瞄准（投掷姿态）。
/// UI 打开（光标解锁）且未持握时自动退出瞄准。
fn aim_system(
    mouse: Res<ButtonInput<MouseButton>>,
    input_state: Res<InputState>,
    held: Res<HeldGrenade>,
    mut cam_query: Query<&mut PlayerCamera>,
) {
    let Ok(mut cam) = cam_query.get_single_mut() else { return };
    cam.aiming = held.item.is_some()
        || (input_state.cursor_locked && mouse.pressed(MouseButton::Right));
}

fn grab_cursor(
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    mut input_state: ResMut<InputState>,
) {
    let mut window = window_query.single_mut();
    window.cursor.visible = false;
    window.cursor.grab_mode = CursorGrabMode::Locked;
    input_state.cursor_locked = true;
}

fn cursor_grab_toggle(
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut input_state: ResMut<InputState>,
    wheel: Res<WheelState>,
    held: Res<HeldGrenade>,
    open_station: Res<OpenStation>,
    backpack_ui: Query<&Visibility, With<InventoryUI>>,
) {
    // 轮盘打开时 Esc 由轮盘负责（取消并收起），这里跳过避免抢占光标状态
    if wheel.open || wheel.pending_key.is_some() { return; }
    // 手雷持握时 Esc 由 grenade_throw_system 负责（取消持握放回背包），光标保持锁定
    if held.item.is_some() { return; }
    // 站点面板打开时 Esc 由 station_system 负责（本系统在它之前运行，看到打开态直接跳过）
    if *open_station != OpenStation::None { return; }
    // 背包打开时 Esc 由 inventory_toggle 负责关闭背包；
    // 若在这里把光标锁回，随后 station_system 会看到"光标已锁 + Esc"而误开补给台
    if backpack_ui.get_single().map_or(false, |vis| *vis == Visibility::Visible) { return; }
    if keyboard.just_pressed(KeyCode::Escape) {
        let mut window = window_query.single_mut();
        if window.cursor.grab_mode == CursorGrabMode::Locked {
            window.cursor.visible = true;
            window.cursor.grab_mode = CursorGrabMode::None;
            input_state.cursor_locked = false;
        } else {
            window.cursor.visible = false;
            window.cursor.grab_mode = CursorGrabMode::Locked;
            input_state.cursor_locked = true;
        }
    }
}

// =============================================================================
// Weapon & Shooting
// =============================================================================

fn weapon_switch(keyboard: Res<ButtonInput<KeyCode>>, mut query: Query<&mut WeaponSlot, With<Player>>) {
    let Ok(mut slot) = query.get_single_mut() else { return };
    if keyboard.just_pressed(KeyCode::Digit1) && slot.current != 0 { slot.current = 0; }
    if keyboard.just_pressed(KeyCode::Digit2) && slot.current != 1 { slot.current = 1; }
}

fn reload_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<(&mut PlayerMovement, &mut WeaponSlot, &mut Inventory), With<Player>>,
    time: Res<Time>,
    input_state: Res<InputState>,
) {
    let Ok((mut movement, weapon_slot, mut inventory)) = player_query.get_single_mut() else { return };

    let current_idx = weapon_slot.current;

    // Tick reload timer if active
    if let Some(ref mut timer) = movement.reload_timer {
        timer.tick(time.delta());
        if timer.finished() {
            // 换弹从背包弹药池取弹
            let widx = current_idx;
            let needed = inventory.weapons[widx].max_ammo - inventory.weapons[widx].ammo;
            if needed > 0 {
                let take = needed.min(inventory.ammo_pool);
                inventory.weapons[widx].ammo += take;
                inventory.ammo_pool -= take;
            }
            movement.reload_timer = None;
        }
    }

    // UI 打开（光标解锁）时 R 留给背包道具使用，不触发换弹
    if !input_state.cursor_locked { return; }

    // Start reload on R key press
    if keyboard.just_pressed(KeyCode::KeyR) && movement.reload_timer.is_none() {
        if inventory.weapons[weapon_slot.current].ammo < inventory.weapons[weapon_slot.current].max_ammo
            && inventory.ammo_pool > 0
        {
            let reload_time = 1.8; // seconds
            movement.reload_timer = Some(Timer::from_seconds(reload_time, TimerMode::Once));
        }
    }
}

fn shooting_system(
    mut commands: Commands,
    mut effects: ResMut<EffectAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mouse: Res<ButtonInput<MouseButton>>,
    element_system: Res<ElementalSystem>,
    cam_query: Query<&PlayerCamera>,
    cam_gtransform: Query<&GlobalTransform, With<PlayerCamera>>,
    mut player_query: Query<(&Transform, &mut PlayerMovement, &mut WeaponSlot, &mut Inventory), With<Player>>,
    mut target_query: Query<(Entity, &mut TargetDummy, &Transform, &Children)>,
    time: Res<Time>,
    input_state: Res<InputState>,
    held: Res<HeldGrenade>,
) {
    let Ok((player_transform, mut movement, weapon_slot, mut inventory)) = player_query.get_single_mut() else { return };
    movement.shoot_cooldown.tick(time.delta());

    // 背包/轮盘/站点等 UI 打开（光标解锁）时不射击
    if !input_state.cursor_locked { return; }

    // 手雷持握（瞄准投掷姿态）时左键交给投掷，步枪不射击
    if held.item.is_some() { return; }

    // Cannot shoot while reloading
    if movement.reload_timer.is_some() { return; }

    let shooting = mouse.pressed(MouseButton::Left);
    if !shooting || !movement.shoot_cooldown.finished() { return; }

    let idx = weapon_slot.current;
    let empty = inventory.weapons[idx].ammo <= 0;
    if empty {
        // Auto-reload when empty（备弹来自背包弹药池）
        if inventory.ammo_pool > 0 && movement.reload_timer.is_none() {
            let reload_time = 1.8;
            movement.reload_timer = Some(Timer::from_seconds(reload_time, TimerMode::Once));
        }
        return;
    }
    inventory.weapons[idx].ammo -= 1;
    let element = inventory.weapons[idx].element;
    let damage = inventory.weapons[idx].damage;
    let fire_interval = inventory.weapons[idx].fire_interval;
    movement.shoot_cooldown = Timer::from_seconds(fire_interval, TimerMode::Once);

    let Ok(cam) = cam_query.get_single() else { return };

    // 命中判定以准星为准：射线从相机出发、沿视线方向（与屏幕中心一致）。
    // 角色与相机之间的物体不参与检测（SpringArm 已做相机避障，射线只查靶子）。
    let cam_gtf = cam_gtransform.get_single().copied()
        .map(|tf| tf.compute_transform())
        .unwrap_or_default();
    let origin = cam_gtf.translation;
    let dir = cam_gtf.forward().normalize();

    // Raycast：躯干大球（宽松判定）+ 靶心核心小球（弱点）
    let mut closest: Option<(Entity, f32, Vec3, bool)> = None;
    for (entity, dummy, transform, _children) in target_query.iter() {
        if dummy.down_timer.is_some() { continue; } // 已倒地的靶子不再受击
        let base = transform.translation;
        let mut hit_t = ray_sphere_hit(origin, dir, base + Vec3::Y * 1.5, 1.5);
        let mut weak = false;
        // 弱点：靶板中心红心（人形敌人则对应头部区域）
        if let Some(t_core) = ray_sphere_hit(origin, dir, base, WEAKPOINT_CORE_R) {
            if hit_t.map_or(true, |t| t_core < t) {
                hit_t = Some(t_core);
                weak = true;
            }
        }
        if let Some(t) = hit_t {
            if t <= 60.0 && closest.map_or(true, |(_, d, _, _)| t < d) {
                closest = Some((entity, t, origin + dir * t, weak));
            }
        }
    }

    let end = if let Some((_, d, _, _)) = closest { origin + dir * d } else { origin + dir * 60.0 };
    // 曳光从枪口出发、收敛到命中点（TPS 标准做法：判定跟准星，视觉跟枪口）
    let aim_t = cam.aim_lerp * cam.aim_lerp * (3.0 - 2.0 * cam.aim_lerp);
    let muzzle = player_transform.translation
        + Quat::from_rotation_y(movement.model_yaw)
            * Vec3::new(0.4, lerp(1.3, 2.35, aim_t) + 0.08, lerp(1.15, 1.05, aim_t));
    let mid = (muzzle + end) / 2.0;
    let len = (end - muzzle).length();

    // Laser trail（共享网格按弹道长度缩放 Z）
    let color = element.color();
    commands.spawn((
        PbrBundle {
            mesh: effects.tracer.clone(),
            material: effects.material(&mut materials, element, EffectMatKind::Plain),
            transform: Transform::from_translation(mid)
                .looking_at(end, Vec3::Y)
                .with_scale(Vec3::new(1.0, 1.0, len)),
            ..default()
        },
        BulletHit { timer: Timer::from_seconds(0.06, TimerMode::Once) },
    ));

    // Muzzle flash
    commands.spawn((
        PbrBundle {
            mesh: effects.spark.clone(),
            material: effects.muzzle_material.clone(),
            transform: Transform::from_translation(muzzle),
            ..default()
        },
        BulletHit { timer: Timer::from_seconds(0.04, TimerMode::Once) },
    ));

    // Hit processing
    if let Some((entity, _, hit_point, weak)) = closest {
        if let Ok((_, mut dummy, _, _)) = target_query.get_mut(entity) {
            // 元素反应：查询核心配置表（与cod1共用同一套规则）
            // 弱点打击：命中靶心核心，基础伤害 ×1.8
            let (dmg, reaction_name) = element_reaction(
                &element_system.0,
                dummy.element_state,
                element,
                damage * if weak { WEAKPOINT_MULT } else { 1.0 },
            );

            dummy.current_health -= dmg;
            dummy.hit_flash = Some(Timer::from_seconds(0.2, TimerMode::Once));
            // 若这一击致命，靶子朝来弹方向倒下（取弹道水平分量）
            dummy.fall_dir = Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();
            if element != ElementType::Physical {
                dummy.element_state = Some(element);
                dummy.state_timer = Some(Timer::from_seconds(3.5, TimerMode::Once));
            }

            // Damage popup (element damage / actual damage / weak point)
            let popup_text = if weak {
                if let Some(ref reaction) = reaction_name {
                    format!("弱点 {} ({})", dmg as i32, reaction)
                } else {
                    format!("弱点 {}", dmg as i32)
                }
            } else if let Some(ref reaction) = reaction_name {
                format!("{} ({})", dmg as i32, reaction)
            } else {
                format!("{}", dmg as i32)
            };
            let popup_color = if weak {
                Color::srgb(1.0, 0.45, 0.1) // hot orange for weak points
            } else if reaction_name.is_some() {
                Color::srgb(1.0, 0.85, 0.2) // gold for reactions
            } else {
                color
            };
            spawn_damage_popup(&mut commands, hit_point + Vec3::Y * 0.5, popup_text, popup_color);

            // Damage particles
            for _ in 0..5 {
                let dir = Vec3::new(
                    (rand::random::<f32>() - 0.5) * 2.0,
                    rand::random::<f32>(),
                    (rand::random::<f32>() - 0.5) * 2.0,
                ).normalize();
                commands.spawn((
                    PbrBundle {
                        mesh: effects.particle.clone(),
                        material: effects.material(&mut materials, element, EffectMatKind::Particle),
                        transform: Transform::from_translation(hit_point),
                        ..default()
                    },
                    DamageParticle {
                        velocity: dir * 3.0,
                        timer: Timer::from_seconds(0.5, TimerMode::Once),
                    },
                ));
            }

            // Hit explosion
            commands.spawn((
                PbrBundle {
                    mesh: effects.spark.clone(),
                    material: effects.material(&mut materials, element, EffectMatKind::HitFlash),
                    transform: Transform::from_translation(hit_point).with_scale(Vec3::splat(2.5)),
                    ..default()
                },
                BulletHit { timer: Timer::from_seconds(0.15, TimerMode::Once) },
            ));

            // Reaction text
            if let Some(name) = reaction_name {
                spawn_reaction_text(&mut commands, hit_point + Vec3::Y * 0.8, name, color);
            }
        }
    }
}

/// 将"已附着的元素"映射为核心系统的实体元素状态
fn applied_element_state(element: ElementType) -> Option<EntityElementState> {
    match element {
        ElementType::Fire => Some(EntityElementState::Burning),
        ElementType::Ice => Some(EntityElementState::Frozen),
        ElementType::Electric => Some(EntityElementState::Electrified),
        ElementType::Poison => Some(EntityElementState::Poisoned),
        ElementType::Water => Some(EntityElementState::Wet),
        ElementType::Physical => None,
    }
}

/// 反应结果的中文名（弹字显示）
fn reaction_result_label(result: &ReactionResult) -> &'static str {
    match result {
        ReactionResult::Vaporize => "蒸发！",
        ReactionResult::Melt => "融化！",
        ReactionResult::Burning => "燃烧！",
        ReactionResult::Electrolysis => "电解！",
        ReactionResult::Superconduct => "超导！",
        ReactionResult::PoisonExplosion => "毒爆！",
        ReactionResult::PoisonCloud => "毒云！",
        ReactionResult::ShatterFreeze => "爆裂冻结！",
        ReactionResult::PhysicalVulnerability => "物理易伤！",
        ReactionResult::RainSuppressed => "雨天压制",
        ReactionResult::RainAmplified => "雨天增强",
        ReactionResult::Overheat => "过热",
        ReactionResult::OverheatRisk => "过热风险",
        ReactionResult::SnowAmplified => "雪地增强",
        ReactionResult::SnowSuppressed => "雪地压制",
        ReactionResult::ConductiveRisk => "导电风险",
    }
}

/// 元素反应结算：查询核心配置表（config/element_reactions.yaml）
///
/// 返回 (最终伤害, 反应名称)。伤害 = 基础伤害 × 反应倍率，
/// 与cod1的伤害结算走同一份YAML规则，新增反应无需改代码。
fn element_reaction(
    system: &ElementSystem,
    existing: Option<ElementType>,
    incoming: ElementType,
    base_damage: f32,
) -> (f32, Option<String>) {
    let Some(existing) = existing else { return (base_damage, None); };
    let Some(state) = applied_element_state(existing) else { return (base_damage, None); };
    let Some(reaction) = system.query_reaction(&state, &incoming) else {
        return (base_damage, None);
    };

    let damage = base_damage * reaction.damage_multiplier;
    (damage, Some(reaction_result_label(&reaction.result).to_string()))
}

fn spawn_reaction_text(commands: &mut Commands, pos: Vec3, text: String, color: Color) {
    commands.spawn((
        Text2dBundle {
            text: Text::from_section(text, TextStyle { font_size: 28.0, color, ..default() }),
            transform: Transform::from_translation(pos).looking_at(pos + Vec3::X, Vec3::Y),
            ..default()
        },
        FloatingReaction { timer: Timer::from_seconds(1.0, TimerMode::Once) },
    ));
}

fn spawn_damage_popup(commands: &mut Commands, pos: Vec3, text: String, color: Color) {
    commands.spawn((
        TextBundle {
            text: Text::from_section(
                text,
                TextStyle {
                    font_size: 28.0,
                    color: Color::srgb(1.0, 1.0, 1.0),
                    ..default()
                },
            ),
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            background_color: BackgroundColor(color.to_linear().with_alpha(0.85).into()),
            z_index: ZIndex::Global(100),
            ..default()
        },
        DamagePopup {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
            world_pos: pos,
        },
    ));
}

fn damage_popup_system(
    mut commands: Commands,
    mut popup_query: Query<(Entity, &mut Style, &mut Text, &mut BackgroundColor, &mut DamagePopup)>,
    camera_query: Query<(&Camera, &GlobalTransform), With<PlayerCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
) {
    let Ok((camera, camera_transform)) = camera_query.get_single() else { return };
    let window_height = windows.get_single().map(|w| w.height()).unwrap_or(1080.0);

    for (entity, mut style, mut text, mut bg, mut popup) in popup_query.iter_mut() {
        popup.timer.tick(time.delta());
        let t = popup.timer.elapsed_secs() / popup.timer.duration().as_secs_f32();

        // Move upward in world space
        popup.world_pos.y += 2.5 * time.delta_seconds();

        // Convert world position to screen position
        if let Some(viewport_pos) = camera.world_to_viewport(camera_transform, popup.world_pos) {
            style.left = Val::Px(viewport_pos.x);
            style.top = Val::Px(window_height - viewport_pos.y);
        }

        // Fade out near end
        if t > 0.6 {
            let alpha = (1.0 - (t - 0.6) / 0.4).clamp(0.0, 1.0);
            text.sections[0].style.color = text.sections[0].style.color.with_alpha(alpha);
            bg.0 = bg.0.with_alpha(alpha * 0.85);
        }

        if popup.timer.finished() {
            commands.entity(entity).despawn();
        }
    }
}

// =============================================================================
// Operator Skills (Q/E)
// =============================================================================

/// 背包战术手雷的伤害与半径（干员 Q 手雷的数值由核心库干员档案提供）
const ITEM_GRENADE_DAMAGE: f32 = 40.0;
const ITEM_GRENADE_RADIUS: f32 = 3.5;

/// 持续区域（毒雾）：周期性对圈内目标结算机制伤害
#[derive(Component)]
struct SkillZone {
    element: ElementType,
    dps: f32,
    radius: f32,
    tick: Timer,
    lifetime: Timer,
}

/// 对爆炸点周围的目标结算范围伤害 + 干员机制（点燃/冰冻）；
/// 毒雾区域由调用方在作用点另行生成（spawn_skill_zone）。
/// 线性距离衰减（边缘保底30%）+ 元素反应，规则查核心配置表。
#[allow(clippy::too_many_arguments)]
fn apply_explosion_damage(
    commands: &mut Commands,
    effects: &EffectAssets,
    element_system: &ElementSystem,
    targets: &mut Query<(Entity, &mut TargetDummy, &Transform, &Children), (Without<GrenadeProjectile>, Without<Player>)>,
    pos: Vec3,
    element: ElementType,
    base_damage: f32,
    radius: f32,
    effect: crate::operator::SkillEffect,
) {
    for (entity, mut dummy, transform, _) in targets.iter_mut() {
        if dummy.down_timer.is_some() { continue; } // 已倒地的靶子不再受击
        let dist = transform.translation.distance(pos);
        if dist > radius { continue; }
        let falloff = (1.0 - dist / radius).max(0.3);
        let (dmg, reaction_name) = element_reaction(
            element_system,
            dummy.element_state,
            element,
            base_damage * falloff,
        );

        dummy.current_health -= dmg;
        dummy.hit_flash = Some(Timer::from_seconds(0.2, TimerMode::Once));
        // 若这一击致命，靶子被冲击波掀翻：沿爆炸中心指向靶子的方向倒下
        let blast_dir = (transform.translation - pos).with_y(0.0);
        if blast_dir.length_squared() > 1e-6 {
            dummy.fall_dir = blast_dir.normalize();
        }
        if element != ElementType::Physical {
            dummy.element_state = Some(element);
            dummy.state_timer = Some(Timer::from_seconds(3.5, TimerMode::Once));
        }

        // 机制：点燃（DoT 挂到目标身上）
        if effect.burn_secs > 0.0 && effect.burn_dps > 0.0 {
            dummy.dots.push(DamageOverTime {
                element,
                dps: effect.burn_dps,
                tick: Timer::from_seconds(DOT_TICK, TimerMode::Repeating),
                remaining: Timer::from_seconds(effect.burn_secs, TimerMode::Once),
            });
        }
        // 机制：冰冻/电麻（目标停止行动 + 冰块视觉；已冻结则只刷新时长）
        if effect.freeze_secs > 0.0 {
            if dummy.frozen.is_none() {
                dummy.frozen = Some(Timer::from_seconds(effect.freeze_secs, TimerMode::Once));
                let ice = commands.spawn((
                    PbrBundle {
                        mesh: effects.frost_cube.clone(),
                        material: effects.frost_material.clone(),
                        transform: Transform::from_translation(Vec3::new(0.0, 0.4, 0.0)),
                        ..default()
                    },
                    NotShadowCaster,
                )).id();
                commands.entity(entity).add_child(ice);
                dummy.frozen_visual = Some(ice);
            } else if let Some(t) = dummy.frozen.as_mut() {
                t.reset();
            }
        }

        let popup_text = if let Some(ref reaction) = reaction_name {
            format!("{} ({})", dmg as i32, reaction)
        } else {
            format!("{}", dmg as i32)
        };
        let popup_color = if reaction_name.is_some() {
            Color::srgb(1.0, 0.85, 0.2)
        } else {
            element.color()
        };
        spawn_damage_popup(commands, transform.translation + Vec3::Y * 0.8, popup_text, popup_color);
    }
}

/// 在作用点展开持续毒雾区（毒蛛机制）：圈内目标每 DOT_TICK 秒掉血
fn spawn_skill_zone(
    commands: &mut Commands,
    effects: &mut EffectAssets,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
    effect: crate::operator::SkillEffect,
) {
    if effect.zone_secs <= 0.0 || effect.zone_dps <= 0.0 { return; }
    // 元素取毒系固定（当前仅毒蛛配置毒区；半径沿用档案缺省 3.5）
    let element = ElementType::Poison;
    let radius = 3.5;
    commands.spawn((
        PbrBundle {
            mesh: effects.zone_cylinder.clone(),
            material: effects.material(materials, element, EffectMatKind::Zone),
            transform: Transform::from_translation(pos.with_y(0.0) + Vec3::Y * 0.7)
                .with_scale(Vec3::new(radius, 1.0, radius)),
            ..default()
        },
        NotShadowCaster,
        SkillZone {
            element,
            dps: effect.zone_dps,
            radius,
            tick: Timer::from_seconds(DOT_TICK, TimerMode::Repeating),
            lifetime: Timer::from_seconds(effect.zone_secs, TimerMode::Once),
        },
    ));
}

/// 毒雾区每帧驱动：周期掉血结算 + 到期消散
fn zone_tick_system(
    mut commands: Commands,
    mut zones: Query<(Entity, &mut Transform, &mut SkillZone)>,
    mut targets: Query<(&Transform, &mut TargetDummy), (Without<SkillZone>, Without<GrenadeProjectile>)>,
    element_system: Res<ElementalSystem>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut zone) in zones.iter_mut() {
        zone.lifetime.tick(time.delta());
        zone.tick.tick(time.delta());
        // 消散前塌缩视觉
        let remain = zone.lifetime.remaining_secs();
        if remain < 0.4 {
            let s = (remain / 0.4).max(0.05);
            transform.scale = Vec3::new(transform.scale.x, s, transform.scale.z);
        }
        if zone.tick.just_finished() {
            for (t, mut dummy) in targets.iter_mut() {
                if dummy.down_timer.is_some() { continue; }
                let offset = t.translation - transform.translation;
                let flat_dist = Vec3::new(offset.x, 0.0, offset.z).length();
                if flat_dist > zone.radius { continue; }
                let (dmg, _) = element_reaction(
                    &element_system.0,
                    dummy.element_state,
                    zone.element,
                    zone.dps * DOT_TICK,
                );
                dummy.current_health -= dmg;
                dummy.element_state = Some(zone.element);
                dummy.state_timer = Some(Timer::from_seconds(3.5, TimerMode::Once));
                spawn_damage_popup(
                    &mut commands,
                    t.translation + Vec3::Y * 1.2,
                    format!("{}", dmg as i32),
                    zone.element.color(),
                );
            }
        }
        if zone.lifetime.finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn skill_system(
    mut commands: Commands,
    mut effects: ResMut<EffectAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    element_system: Res<ElementalSystem>,
    mut player_query: Query<(&mut Transform, &mut OperatorState), With<Player>>,
    mut target_query: Query<(Entity, &mut TargetDummy, &Transform, &Children), (Without<GrenadeProjectile>, Without<Player>)>,
    colliders: Query<(&Transform, &Collider), Without<Player>>,
    cam_query: Query<&PlayerCamera>,
    input_state: Res<InputState>,
) {
    // 背包/站点等 UI 打开（光标解锁）时不触发技能
    if !input_state.cursor_locked { return; }
    let Ok((mut player_transform, mut op)) = player_query.get_single_mut() else { return };
    let Ok(cam) = cam_query.get_single() else { return };

    // 技能定义来自核心库干员名册：形态/机制/冷却全部随干员切换
    let op_def = &roster()[op.active];
    let element = op_def.element;
    if keyboard.just_pressed(KeyCode::KeyQ) && op.q.finished() {
        op.q.reset();
        cast_skill(&mut commands, &mut effects, &mut materials, &element_system.0,
            &mut target_query, &colliders, &mut player_transform, cam.yaw, element, op_def.q);
    }
    if keyboard.just_pressed(KeyCode::KeyE) && op.e.finished() {
        op.e.reset();
        cast_skill(&mut commands, &mut effects, &mut materials, &element_system.0,
            &mut target_query, &colliders, &mut player_transform, cam.yaw, element, op_def.e);
    }
}

/// 施放一个技能定义：形态由核心配置（SkillKind）决定，
/// 独特机制（点燃/冰冻/毒区/位移）由 SkillEffect/Kind 驱动，元素取干员亲和元素。
#[allow(clippy::too_many_arguments)]
fn cast_skill(
    commands: &mut Commands,
    effects: &mut EffectAssets,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    element_system: &ElementSystem,
    target_query: &mut Query<(Entity, &mut TargetDummy, &Transform, &Children), (Without<GrenadeProjectile>, Without<Player>)>,
    colliders: &Query<(&Transform, &Collider), Without<Player>>,
    player_transform: &mut Transform,
    yaw: f32,
    element: ElementType,
    skill: crate::operator::SkillDef,
) {
    let fwd = Vec3::new(yaw.sin(), 0.0, yaw.cos());
    match skill.kind {
        SkillKind::Grenade { damage, radius } => {
            commands.spawn((
                PbrBundle {
                    mesh: effects.projectile.clone(),
                    material: effects.material(materials, element, EffectMatKind::Plain),
                    transform: Transform::from_translation(player_transform.translation + Vec3::new(0.0, 2.5, 0.0) + fwd * 0.5),
                    ..default()
                },
                GrenadeProjectile {
                    velocity: fwd * 12.0 + Vec3::Y * 6.0,
                    element,
                    damage,
                    radius,
                    effect: skill.effect,
                    timer: Timer::from_seconds(1.5, TimerMode::Once),
                },
            ));
        }
        SkillKind::Burst { damage, radius } => {
            let burst_pos = player_transform.translation + Vec3::new(0.0, 1.0, 0.0);
            // 范围伤害 + 机制（点燃/冰冻）+ 视觉特效
            apply_explosion_damage(commands, effects, element_system, target_query,
                burst_pos, element, damage, radius, skill.effect);
            spawn_explosion(commands, effects, materials, burst_pos, element, radius + 0.5);
            // 机制：毒雾区在脚下展开
            spawn_skill_zone(commands, effects, materials, player_transform.translation, skill.effect);
        }
        SkillKind::Dash { distance } => {
            // 机制：沿视线水平疾冲，逐段碰撞检测，撞障碍即停
            let start = player_transform.translation;
            let half = Vec3::new(0.4, 1.0, 0.4);
            let step = 0.25;
            let mut moved = 0.0f32;
            while moved < distance {
                let next = (moved + step).min(distance);
                if collides(start + fwd * next, half, colliders) { break; }
                moved = next;
            }
            player_transform.translation = start + fwd * moved;
            // 冲刺残影：起点与终点各撒一把元素火花
            for pos in [start + Vec3::Y, start + fwd * moved + Vec3::Y] {
                for _ in 0..6 {
                    let dir = Vec3::new(
                        (rand::random::<f32>() - 0.5) * 2.0,
                        rand::random::<f32>(),
                        (rand::random::<f32>() - 0.5) * 2.0,
                    ).normalize();
                    commands.spawn((
                        PbrBundle {
                            mesh: effects.particle.clone(),
                            material: effects.material(materials, element, EffectMatKind::Particle),
                            transform: Transform::from_translation(pos),
                            ..default()
                        },
                        DamageParticle {
                            velocity: dir * 2.5,
                            timer: Timer::from_seconds(0.35, TimerMode::Once),
                        },
                    ));
                }
            }
        }
    }
}

fn operator_cooldown_tick(mut query: Query<&mut OperatorState, With<Player>>, time: Res<Time>) {
    let Ok(mut op) = query.get_single_mut() else { return };
    op.q.tick(time.delta());
    op.e.tick(time.delta());
}

fn spawn_explosion(
    commands: &mut Commands,
    effects: &mut EffectAssets,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
    element: ElementType,
    max_scale: f32,
) {
    commands.spawn((
        PbrBundle {
            mesh: effects.explosion_sphere.clone(),
            material: effects.material(materials, element, EffectMatKind::Explosion),
            transform: Transform::from_translation(pos).with_scale(Vec3::splat(0.1)),
            ..default()
        },
        ExplosionEffect { timer: Timer::from_seconds(0.6, TimerMode::Once), max_scale },
    ));
    for _ in 0..10 {
        let dir = Vec3::new(
            (rand::random::<f32>() - 0.5) * 2.0,
            rand::random::<f32>() * 0.8 + 0.2,
            (rand::random::<f32>() - 0.5) * 2.0,
        ).normalize();
        commands.spawn((
            PbrBundle {
                mesh: effects.explosion_debris.clone(),
                material: effects.material(materials, element, EffectMatKind::Plain),
                transform: Transform::from_translation(pos + dir * 0.5),
                ..default()
            },
            BulletHit { timer: Timer::from_seconds(0.5, TimerMode::Once) },
        ));
    }
}

// =============================================================================
// Projectile & Effect Systems
// =============================================================================

fn grenade_physics(
    mut commands: Commands,
    mut effects: ResMut<EffectAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    element_system: Res<ElementalSystem>,
    mut query: Query<(Entity, &mut Transform, &mut GrenadeProjectile), Without<TargetDummy>>,
    mut target_query: Query<(Entity, &mut TargetDummy, &Transform, &Children), (Without<GrenadeProjectile>, Without<Player>)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut grenade) in query.iter_mut() {
        grenade.timer.tick(time.delta());
        grenade.velocity.y -= 12.0 * time.delta_seconds();
        transform.translation += grenade.velocity * time.delta_seconds();

        // 落地或引信耗尽时引爆：范围伤害 + 机制 + 视觉特效（数值由投掷物自带）
        if transform.translation.y <= 0.15 {
            transform.translation.y = 0.15;
            let impact = transform.translation;
            apply_explosion_damage(&mut commands, &effects, &element_system.0, &mut target_query,
                impact, grenade.element, grenade.damage, grenade.radius, grenade.effect);
            spawn_skill_zone(&mut commands, &mut effects, &mut materials, impact, grenade.effect);
            spawn_explosion(&mut commands, &mut effects, &mut materials, impact, grenade.element, grenade.radius + 1.5);
            commands.entity(entity).despawn();
            continue;
        }
        if grenade.timer.finished() {
            let impact = transform.translation;
            apply_explosion_damage(&mut commands, &effects, &element_system.0, &mut target_query,
                impact, grenade.element, grenade.damage, grenade.radius, grenade.effect);
            spawn_skill_zone(&mut commands, &mut effects, &mut materials, impact, grenade.effect);
            spawn_explosion(&mut commands, &mut effects, &mut materials, impact, grenade.element, grenade.radius + 1.5);
            commands.entity(entity).despawn();
        }
    }
}

fn explosion_expand(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut ExplosionEffect)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut effect) in query.iter_mut() {
        effect.timer.tick(time.delta());
        let t = effect.timer.elapsed_secs() / effect.timer.duration().as_secs_f32();
        let scale = effect.max_scale * t.min(1.0);
        transform.scale = Vec3::splat(scale);
        if effect.timer.finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn bullet_cleanup(mut commands: Commands, mut query: Query<(Entity, &mut BulletHit)>, time: Res<Time>) {
    for (entity, mut hit) in query.iter_mut() {
        hit.timer.tick(time.delta());
        if hit.timer.finished() { commands.entity(entity).despawn(); }
    }
}

fn damage_particle_system(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut DamageParticle)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut part) in query.iter_mut() {
        part.timer.tick(time.delta());
        transform.translation += part.velocity * time.delta_seconds();
        part.velocity.y -= 8.0 * time.delta_seconds();
        if part.timer.finished() { commands.entity(entity).despawn(); }
    }
}

fn floating_reaction_text(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut FloatingReaction)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut text) in query.iter_mut() {
        text.timer.tick(time.delta());
        transform.translation.y += 1.5 * time.delta_seconds();
        if text.timer.finished() { commands.entity(entity).despawn(); }
    }
}

// =============================================================================
// Target Logic
// =============================================================================

fn hit_flash_system(
    mut query: Query<&mut TargetDummy>,
    time: Res<Time>,
) {
    for mut dummy in query.iter_mut() {
        if let Some(ref mut timer) = dummy.hit_flash {
            timer.tick(time.delta());
            if timer.finished() { dummy.hit_flash = None; }
        }
        if let Some(ref mut timer) = dummy.state_timer {
            timer.tick(time.delta());
            if timer.finished() { dummy.element_state = None; dummy.state_timer = None; }
        }
    }
}

fn target_dummy_logic(
    mut commands: Commands,
    mut effects: ResMut<EffectAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut query: Query<(Entity, &mut TargetDummy, &mut Transform)>,
    mut kill_events: EventWriter<KillEvent>,
    time: Res<Time>,
) {
    for (_entity, mut dummy, mut transform) in query.iter_mut() {
        let dt = time.delta();

        // 冰冻/电麻计时：到时解冻并移除冰块视觉
        if let Some(ref mut frozen) = dummy.frozen {
            frozen.tick(dt);
            if frozen.finished() {
                dummy.frozen = None;
                if let Some(ice) = dummy.frozen_visual.take() {
                    commands.entity(ice).despawn();
                }
            }
        }

        // 持续伤害（点燃等）：按 tick 结算并弹出伤害数字
        let mut pending_dmg = 0.0f32;
        let mut pending_popups: Vec<(f32, ElementType)> = Vec::new();
        for dot in dummy.dots.iter_mut() {
            dot.remaining.tick(dt);
            dot.tick.tick(dt);
            if dot.tick.just_finished() {
                let dmg = dot.dps * DOT_TICK;
                pending_dmg += dmg;
                pending_popups.push((dmg, dot.element));
            }
        }
        dummy.current_health -= pending_dmg;
        for (dmg, element) in pending_popups {
            spawn_damage_popup(&mut commands,
                transform.translation + Vec3::Y * 1.2,
                format!("{}", dmg as i32),
                element.color());
        }
        dummy.dots.retain(|dot| !dot.remaining.finished());

        let fall_dir = dummy.fall_dir;
        if let Some(ref mut down) = dummy.down_timer {
            // 倒地阶段：前 fall_time 秒播放翻倒动画，之后躺到计时结束原地复活
            down.tick(time.delta());
            let fall_progress = (down.elapsed_secs() / TARGET_FALL_TIME).min(1.0);
            let eased = fall_progress * fall_progress; // 加速下坠感
            let axis = Vec3::Y.cross(fall_dir);
            if axis.length_squared() > 1e-6 {
                transform.rotation = Quat::from_axis_angle(axis.normalize(), eased * std::f32::consts::FRAC_PI_2);
            }
            if down.finished() {
                dummy.current_health = dummy.max_health;
                dummy.element_state = None;
                dummy.state_timer = None;
                dummy.hit_flash = None;
                dummy.down_timer = None;
                dummy.dots.clear();
                dummy.frozen = None;
                if let Some(ice) = dummy.frozen_visual.take() {
                    commands.entity(ice).despawn();
                }
                transform.rotation = Quat::IDENTITY;
            }
        } else if dummy.current_health <= 0.0 {
            // 击倒瞬间：爆炸特效 + 击杀播报，进入倒地状态（不再满血瞬间重置）
            // 同时清空持续效果（点燃熄灭、冰块碎裂）
            kill_events.send(KillEvent { name: dummy.label.to_string() });
            dummy.current_health = 0.0;
            dummy.down_timer = Some(Timer::from_seconds(TARGET_DOWN_SECS, TimerMode::Once));
            dummy.element_state = None;
            dummy.state_timer = None;
            dummy.hit_flash = None;
            dummy.dots.clear();
            dummy.frozen = None;
            if let Some(ice) = dummy.frozen_visual.take() {
                commands.entity(ice).despawn();
            }
            spawn_explosion(&mut commands, &mut effects, &mut materials,
                transform.translation + Vec3::Y * 1.0, ElementType::Physical, 2.5);
        }
    }
}

fn moving_target_logic(
    mut query: Query<(&mut Transform, &mut MovingTarget, Option<&TargetDummy>)>,
    time: Res<Time>,
) {
    for (mut transform, mut target, dummy) in query.iter_mut() {
        // 倒地或被冰冻期间停止巡逻
        if dummy.map(|d| d.down_timer.is_some() || d.frozen.is_some()).unwrap_or(false) { continue; }
        let offset = transform.translation - target.origin;
        if offset.x.abs() > target.range { target.direction *= -1.0; }
        transform.translation.x += target.direction * target.speed * time.delta_seconds();
    }
}

// =============================================================================
// HUD Dynamic Systems
// =============================================================================

fn crosshair_hit_feedback(
    mut crosshair_lines: Query<&mut BackgroundColor, With<CrosshairLine>>,
    mut crosshair_center: Query<&mut BackgroundColor, (With<CrosshairCenter>, Without<CrosshairLine>)>,
    mut player_query: Query<&mut PlayerMovement, With<Player>>,
    cam_query: Query<&PlayerCamera>,
) {
    let Ok(movement) = player_query.get_single_mut() else { return };
    let t = movement.shoot_cooldown.elapsed_secs() / movement.shoot_cooldown.duration().as_secs_f32();
    // 命中反馈（红） > 瞄准态（琥珀） > 常态（白）
    let aiming = cam_query.get_single().map(|c| c.aim_lerp > 0.35).unwrap_or(false);
    let color = if t < 0.3 {
        Color::srgba(1.0, 0.3, 0.3, 0.9)
    } else if aiming {
        Color::srgba(1.0, 0.8, 0.25, 0.9)
    } else {
        Color::srgba(0.95, 0.95, 0.95, 0.9)
    };
    for mut bg in crosshair_lines.iter_mut() { bg.0 = color; }
    for mut bg in crosshair_center.iter_mut() { bg.0 = color; }
}

#[allow(clippy::type_complexity)]
fn hud_update_system(
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
fn hud_vitals_text_system(
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
fn hud_operator_name_system(
    player_query: Query<&OperatorState, With<Player>>,
    mut operator_name: Query<&mut Text, With<HudOperatorName>>,
) {
    let Ok(op) = player_query.get_single() else { return };
    let Ok(mut text) = operator_name.get_single_mut() else { return };
    let op_def = &roster()[op.active];
    text.sections[0].value = format!("干员 · {}", op_def.name);
    text.sections[0].style.color = op_def.element.color();
}

fn low_ammo_blink(
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

fn screen_edge_glow(
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

fn respawn_flash_system(
    _query: Query<&mut BackgroundColor, With<HudEdgeGlow>>,
) {
}

// =============================================================================
// Inventory & Pickup Systems
// =============================================================================

fn setup_inventory_hud(mut commands: Commands) {
    // Centered inventory overlay (hidden by default)
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
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            visibility: Visibility::Hidden,
            ..default()
        },
        InventoryUI,
    )).with_children(|overlay| {
        // Panel background
        overlay.spawn(NodeBundle {
            style: Style {
                width: Val::Px(540.0),
                height: Val::Auto,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(8.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.06, 0.06, 0.08, 0.95)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        }).with_children(|panel| {
            // Title row: 背包 + 弹药池
            panel.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    margin: UiRect::bottom(Val::Px(4.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                ..default()
            }).with_children(|title_row| {
                title_row.spawn(TextBundle {
                    text: Text::from_section(
                        "背包",
                        TextStyle { font_size: 20.0, color: Color::srgb(0.9, 0.9, 0.9), ..default() }
                    ),
                    ..default()
                });
                title_row.spawn((
                    TextBundle {
                        text: Text::from_section(
                            "弹药池 150",
                            TextStyle { font_size: 14.0, color: Color::srgb(0.95, 0.78, 0.25), ..default() }
                        ),
                        ..default()
                    },
                    HudBackpackAmmo,
                ));
            });
            // Divider line
            panel.spawn(NodeBundle {
                style: Style { width: Val::Percent(100.0), height: Val::Px(2.0), ..default() },
                background_color: BackgroundColor(Color::srgba(0.4, 0.4, 0.5, 0.3)),
                ..default()
            });
            // 武器架：固定双主武器，金色边框
            panel.spawn(TextBundle {
                text: Text::from_section(
                    "主武器 ×2（场上拾取新武器将替换当前手持）",
                    TextStyle { font_size: 12.0, color: Color::srgb(0.5, 0.5, 0.55), ..default() }
                ),
                ..default()
            });
            panel.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(8.0),
                    margin: UiRect::top(Val::Px(4.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                ..default()
            }).with_children(|weapons| {
                for i in 0..MAX_WEAPONS {
                    weapons.spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Px(246.0),
                                height: Val::Px(64.0),
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
                        Interaction::default(),
                        BackpackWeaponSlot(i),
                    )).with_children(|box_| {
                        box_.spawn((
                            TextBundle {
                                text: Text::from_sections([
                                    TextSection::new("空", TextStyle { font_size: 12.0, color: Color::srgb(0.85, 0.85, 0.85), ..default() }),
                                    TextSection::new("", TextStyle { font_size: 11.0, color: Color::srgb(0.8, 0.8, 0.82), ..default() }),
                                ]),
                                ..default()
                            },
                            BackpackWeaponText(i),
                        ));
                    });
                }
            });
            // Divider line
            panel.spawn(NodeBundle {
                style: Style { width: Val::Percent(100.0), height: Val::Px(2.0), margin: UiRect::top(Val::Px(2.0)), ..default() },
                background_color: BackgroundColor(Color::srgba(0.4, 0.4, 0.5, 0.3)),
                ..default()
            });
            // 物资区标题
            panel.spawn(TextBundle {
                text: Text::from_section(
                    "物资（悬停 + R 使用）",
                    TextStyle { font_size: 12.0, color: Color::srgb(0.5, 0.5, 0.55), ..default() }
                ),
                style: Style { margin: UiRect::top(Val::Px(4.0)), ..default() },
                ..default()
            });
            // Grid of slots
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
                for i in 0..8 {
                    grid.spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Px(86.0),
                                height: Val::Px(86.0),
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
                        InventorySlotUI(i),
                        Interaction::default(),
                    )).with_children(|slot| {
                        slot.spawn((
                            TextBundle {
                                text: Text::from_section(
                                    " ",
                                    TextStyle { font_size: 13.0, color: Color::srgb(0.85, 0.85, 0.85), ..default() }
                                ),
                                style: Style { width: Val::Percent(90.0), height: Val::Auto, justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                                ..default()
                            },
                            InventorySlotText(i),
                        ));
                    });
                }
            });
            // Instructions
            panel.spawn(TextBundle {
                text: Text::from_section(
                    "Tab/Esc 关闭 · 悬停物资+R 使用 · 3/4 快捷 / 长按呼出轮盘（点击中心撤销）\n手雷使用后持握瞄准：左键投掷 / Esc 取消 · 右键越肩瞄准 · 拾取武器替换当前手持 · 弹药拾取直接入弹药池",
                    TextStyle { font_size: 12.0, color: Color::srgb(0.5, 0.5, 0.55), ..default() }
                ),
                style: Style { margin: UiRect::top(Val::Px(6.0)), ..default() },
                ..default()
            });
        });
    });

    // 统一交互菜单（默认隐藏）——底部居中：站点条目在前 + 附近拾取物，滚轮选择 / F 确认
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::End,
                padding: UiRect::bottom(Val::Px(160.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
            visibility: Visibility::Hidden,
            ..default()
        },
        InteractMenuUI,
    )).with_children(|parent| {
        parent.spawn((
            NodeBundle {
            style: Style {
                width: Val::Px(240.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                row_gap: Val::Px(4.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.78)),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            ..default()
        },
        InteractMenuPanel,
        )).with_children(|panel| {
            // 标题行：F 徽标 + 标题（收起时需随菜单一起显式隐藏）
            panel.spawn((
                NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        margin: UiRect::bottom(Val::Px(2.0)),
                        ..default()
                    },
                    ..default()
                },
                InteractMenuHeader,
            )).with_children(|header| {
                header.spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(22.0),
                        height: Val::Px(22.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.9, 0.85, 0.25, 0.9)),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                }).with_children(|badge| {
                    badge.spawn(TextBundle {
                        text: Text::from_section("F", TextStyle { font_size: 13.0, color: Color::srgb(0.1, 0.1, 0.1), ..default() }),
                        ..default()
                    });
                });
                header.spawn(TextBundle {
                    text: Text::from_section("互动菜单", TextStyle { font_size: 15.0, color: Color::srgb(0.9, 0.9, 0.9), ..default() }),
                    ..default()
                });
            });
            // 正文行：左侧固定高度的条目列表 + 右侧滚动条（凹槽常驻占位，面板宽度稳定）
            panel.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(6.0),
                    align_items: AlignItems::FlexStart,
                    ..default()
                },
                ..default()
            }).with_children(|body| {
                // 条目列表：8 个行槽常驻占位（隐藏也保留空间），面板高度不随条目数变化；
                // 超出可见行数的条目折叠在滚动窗口外，由 interact_menu_update 映射显示
                body.spawn(NodeBundle {
                    style: Style {
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(INTERACT_ROW_GAP),
                        ..default()
                    },
                    ..default()
                }).with_children(|list| {
                    // 条目行：每行独立节点、固定行高 + 统一左内边距，保证各行文本严格左对齐；
                    // 选中高亮（背景/边框）由 interact_menu_update 每帧刷新
                    for i in 0..INTERACT_MENU_VISIBLE_ROWS {
                        list.spawn((
                            NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(INTERACT_ROW_H),
                                    align_items: AlignItems::Center,
                                    padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::srgba(0.10, 0.10, 0.13, 0.85)),
                                border_color: BorderColor(Color::srgba(0.3, 0.3, 0.35, 0.4)),
                                border_radius: BorderRadius::all(Val::Px(4.0)),
                                visibility: Visibility::Hidden,
                                ..default()
                            },
                            InteractRow(i),
                        )).with_children(|row| {
                            row.spawn((
                                TextBundle {
                                    text: Text::from_section(
                                        "",
                                        TextStyle { font_size: 14.0, color: Color::srgb(0.9, 0.9, 0.9), ..default() }
                                    ),
                                    ..default()
                                },
                                InteractRowText(i),
                            ));
                        });
                    }
                });
                // 滚动条凹槽：高度与条目列表等高；条目未超出可见行数时整体隐藏（占位不塌）
                body.spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Px(6.0),
                            height: Val::Px(INTERACT_SCROLL_TRACK_H),
                            margin: UiRect::top(Val::Px(1.0)),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.08)),
                        border_radius: BorderRadius::all(Val::Px(3.0)),
                        visibility: Visibility::Hidden,
                        ..default()
                    },
                    InteractScrollBar(ScrollbarPart::Track),
                )).with_children(|track| {
                    // 滑块：绝对定位在凹槽内，top/height 由 interact_menu_update 按窗口位置刷新
                    track.spawn((
                        NodeBundle {
                            style: Style {
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                width: Val::Percent(100.0),
                                top: Val::Px(0.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::srgba(0.6, 0.6, 0.65, 0.7)),
                            border_radius: BorderRadius::all(Val::Px(3.0)),
                            visibility: Visibility::Hidden,
                            ..default()
                        },
                        InteractScrollBar(ScrollbarPart::Thumb),
                    ));
                });
            });
            // 底部提示行：操作说明，选中条目有满载/替换警告时由 interact_menu_update 覆写
            panel.spawn((
                TextBundle {
                    text: Text::from_section(
                        "滚轮选择 · F 确认",
                        TextStyle { font_size: 11.0, color: Color::srgb(0.5, 0.5, 0.55), ..default() }
                    ),
                    ..default()
                },
                InteractMenuHintText,
            ));
        });
    });
}

fn inventory_toggle(
    mut keyboard: ResMut<ButtonInput<KeyCode>>,
    mut inventory_ui: Query<&mut Visibility, With<InventoryUI>>,
    mut input_state: ResMut<InputState>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    wheel: Res<WheelState>,
    open_station: Res<OpenStation>,
) {
    // 轮盘打开/按住期间不响应 Tab/Esc，避免两层 UI 叠加
    if wheel.open || wheel.pending_key.is_some() { return; }
    // 站点面板打开时不响应（站点面板自带开关）
    if *open_station != OpenStation::None { return; }
    let tab = keyboard.just_pressed(KeyCode::Tab);
    let esc = keyboard.just_pressed(KeyCode::Escape);
    if !tab && !esc { return; }
    let Ok(mut vis) = inventory_ui.get_single_mut() else { return };
    let mut window = window_query.single_mut();
    if *vis == Visibility::Visible {
        // 背包打开时 Tab / Esc 均关闭；消费掉 Esc 按下态，
        // 防止同帧更晚运行的 cursor_grab_toggle 再把光标解锁
        *vis = Visibility::Hidden;
        window.cursor.visible = false;
        window.cursor.grab_mode = CursorGrabMode::Locked;
        input_state.cursor_locked = true;
        keyboard.clear_just_pressed(KeyCode::Escape);
    } else if tab {
        // 仅 Tab 打开；Esc 留给自由光标切换
        *vis = Visibility::Visible;
        window.cursor.visible = true;
        window.cursor.grab_mode = CursorGrabMode::None;
        input_state.cursor_locked = false;
    }
}

fn interact_detection_system(
    player_query: Query<&Transform, With<Player>>,
    pickup_query: Query<(Entity, &Transform, &PickupItem)>,
    station_query: Query<(&Transform, &Station)>,
    mut nearby: ResMut<NearbyInteract>,
) {
    let Ok(player_transform) = player_query.get_single() else { return };
    let player_pos = player_transform.translation;

    // 站点条目在最前：靠近桌子时优先开台，不会被地上的散落物抢走 F
    let mut entries: Vec<InteractEntry> = Vec::new();
    let mut best = STATION_USE_RANGE;
    let mut near_station: Option<(&StationKind, &'static str)> = None;
    for (transform, station) in station_query.iter() {
        let dist = transform.translation.distance(player_pos);
        if dist < best {
            best = dist;
            near_station = Some((&station.kind, station.label));
        }
    }
    if let Some((kind, label)) = near_station {
        entries.push(InteractEntry::Station { kind: *kind, label });
    }

    // 拾取物按距离升序排在站点之后
    let mut items: Vec<(Entity, f32)> = Vec::new();
    for (entity, transform, _item) in pickup_query.iter() {
        let dist = (transform.translation - player_pos).length();
        if dist < 3.5 {
            items.push((entity, dist));
        }
    }
    items.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    entries.extend(items.into_iter().map(|(e, _)| InteractEntry::Pickup(e)));
    entries.truncate(INTERACT_MENU_MAX_ENTRIES);

    nearby.entries = entries;
    if nearby.selected >= nearby.entries.len() && !nearby.entries.is_empty() {
        nearby.selected = 0;
        // 选中项跳回开头时滚动窗口一并归位，保证选中条目可见
        nearby.scroll_start = 0;
    }
    // 条目缩水后钳制窗口起点：超出固定高度的条目折叠在窗口外
    nearby.scroll_start = nearby.scroll_start.min(nearby.entries.len().saturating_sub(INTERACT_MENU_VISIBLE_ROWS));
}

fn interact_scroll_system(
    mut scroll_events: EventReader<MouseWheel>,
    mut nearby: ResMut<NearbyInteract>,
    input_state: Res<InputState>,
    wheel: Res<WheelState>,
) {
    // 交互菜单未显示（光标解锁/轮盘占用）时滚轮不改变选中项
    if !input_state.cursor_locked || wheel.open || wheel.pending_key.is_some() { return; }
    // 先钳制窗口起点，防止上一帧条目缩水后窗口越界
    nearby.scroll_start = nearby.scroll_start.min(nearby.entries.len().saturating_sub(INTERACT_MENU_VISIBLE_ROWS));
    for ev in scroll_events.read() {
        if nearby.entries.len() <= 1 { continue; }
        if ev.y > 0.0 {
            nearby.selected = (nearby.selected + nearby.entries.len() - 1) % nearby.entries.len();
        } else if ev.y < 0.0 {
            nearby.selected = (nearby.selected + 1) % nearby.entries.len();
        }
        // 滚动窗口跟随选中项：仅在选中项离开固定可见范围时最小幅度移动
        if nearby.selected < nearby.scroll_start {
            nearby.scroll_start = nearby.selected;
        } else if nearby.selected >= nearby.scroll_start + INTERACT_MENU_VISIBLE_ROWS {
            nearby.scroll_start = nearby.selected + 1 - INTERACT_MENU_VISIBLE_ROWS;
        }
    }
}

/// 统一交互菜单刷新：显隐、滚动窗口逐行文本与选中高亮、滚动条位置、底部警告提示。
/// 面板高度固定（行槽常驻占位）：条目超出可见行数时折叠在窗口外，
/// 行槽 j 显示 entries[scroll_start + j]，滚动条滑块同步窗口位置。
#[allow(clippy::too_many_arguments)]
fn interact_menu_update(
    nearby: Res<NearbyInteract>,
    pickup_query: Query<&PickupItem>,
    player_query: Query<(&Inventory, &WeaponSlot), With<Player>>,
    input_state: Res<InputState>,
    wheel: Res<WheelState>,
    held: Res<HeldGrenade>,
    mut root_vis: Query<&mut Visibility, With<InteractMenuUI>>,
    // bevy 0.14 UI 不会因祖先 Hidden 剔除子节点：收起时面板/标题/行/滚动条/文本都要各自隐藏
    mut panel_vis: Query<&mut Visibility, (With<InteractMenuPanel>, Without<InteractMenuUI>, Without<InteractRow>, Without<InteractMenuHeader>)>,
    mut header_vis: Query<&mut Visibility, (With<InteractMenuHeader>, Without<InteractMenuUI>, Without<InteractRow>, Without<InteractMenuPanel>)>,
    mut rows: Query<(&InteractRow, &mut Visibility, &mut BackgroundColor, &mut BorderColor), (Without<InteractRowText>, Without<InteractMenuUI>, Without<InteractMenuPanel>, Without<InteractMenuHeader>)>,
    mut texts: Query<(&InteractRowText, &mut Text), Without<InteractMenuHintText>>,
    mut hint: Query<&mut Text, (With<InteractMenuHintText>, Without<InteractRowText>)>,
    mut scrollbar: Query<(&InteractScrollBar, &mut Style, &mut Visibility), (Without<InteractMenuUI>, Without<InteractMenuPanel>, Without<InteractMenuHeader>, Without<InteractRow>)>,
) {
    // 背包/站点/轮盘等任一 UI 占用（光标解锁）或持雷瞄准时收起菜单
    let show = !nearby.entries.is_empty()
        && input_state.cursor_locked
        && !wheel.open
        && wheel.pending_key.is_none()
        && held.item.is_none();
    if let Ok(mut vis) = root_vis.get_single_mut() {
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
    }
    if let Ok(mut vis) = panel_vis.get_single_mut() {
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
    }
    if let Ok(mut vis) = header_vis.get_single_mut() {
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
    }

    // 滚动窗口：行槽 j 显示 entries[scroll_start + j]；scroll_start 由滚轮/检测系统维护，
    // 这里做显示层钳制，避免条目缩水后窗口越界一帧
    let total = nearby.entries.len();
    let max_start = total.saturating_sub(INTERACT_MENU_VISIBLE_ROWS);
    let start = nearby.scroll_start.min(max_start);

    for (row, mut vis, mut bg, mut border) in rows.iter_mut() {
        let entry_idx = start + row.0;
        if !show || nearby.entries.get(entry_idx).is_none() {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Visible;
        let selected = entry_idx == nearby.selected;
        *bg = BackgroundColor(if selected {
            Color::srgba(0.26, 0.27, 0.32, 0.95)
        } else {
            Color::srgba(0.10, 0.10, 0.13, 0.85)
        });
        *border = BorderColor(if selected {
            Color::srgba(1.0, 0.8, 0.25, 0.95)
        } else {
            Color::srgba(0.3, 0.3, 0.35, 0.4)
        });
    }
    for (row, mut text) in texts.iter_mut() {
        let entry_idx = start + row.0;
        if !show {
            text.sections[0].value = String::new();
            continue;
        }
        let Some(entry) = nearby.entries.get(entry_idx) else {
            text.sections[0].value = String::new();
            continue;
        };
        match *entry {
            InteractEntry::Station { kind, label } => {
                text.sections[0].value = label.to_string();
                text.sections[0].style.color = station_accent(kind);
            }
            InteractEntry::Pickup(entity) => {
                if let Ok(item) = pickup_query.get(entity) {
                    text.sections[0].value = item.name.clone();
                    text.sections[0].style.color = pickup_text_color(&item.item_type);
                }
            }
        }
    }
    // 滚动条：条目超出可见行数才显示；滑块高度 = 可见占比，位置对应窗口起点
    let scrolling = show && total > INTERACT_MENU_VISIBLE_ROWS;
    for (part, mut style, mut vis) in scrollbar.iter_mut() {
        *vis = if scrolling { Visibility::Visible } else { Visibility::Hidden };
        if part.0 == ScrollbarPart::Thumb && scrolling {
            let thumb_h = INTERACT_SCROLL_TRACK_H * (INTERACT_MENU_VISIBLE_ROWS as f32 / total as f32);
            let travel = INTERACT_SCROLL_TRACK_H - thumb_h;
            style.height = Val::Px(thumb_h);
            style.top = Val::Px(if max_start > 0 { travel * (start as f32 / max_start as f32) } else { 0.0 });
        }
    }
    // 底部提示：操作说明 + 选中拾取物的满载/替换警告（弹药直接入池永不占槽）
    if let Ok(mut text) = hint.get_single_mut() {
        let mut value = if show { "滚轮选择 · F 确认".to_string() } else { String::new() };
        if show {
            if let Some(InteractEntry::Pickup(entity)) = nearby.entries.get(nearby.selected) {
                if let Ok(item) = pickup_query.get(*entity) {
                    if let Ok((inventory, weapon_slot)) = player_query.get_single() {
                        match item.item_type {
                            PickupType::Weapon { .. } => {
                                if inventory.weapons.len() >= inventory.max_weapons {
                                    let cur = &inventory.weapons[weapon_slot.current];
                                    value = format!("将替换当前武器 {}", cur.name);
                                }
                            }
                            PickupType::Ammo { .. } => {}
                            _ => {
                                if inventory.items.len() >= inventory.max_slots {
                                    value = "背包已满".to_string();
                                }
                            }
                        }
                    }
                }
            }
        }
        text.sections[0].value = value;
    }
}

/// 站点条目配色（与小地图一致：补给台琥珀 / 干员切换台青）
fn station_accent(kind: StationKind) -> Color {
    match kind {
        StationKind::SupplyTable => Color::srgb(1.0, 0.65, 0.15),
        StationKind::OperatorDesk => Color::srgb(0.2, 0.9, 0.95),
    }
}

/// 拾取物条目配色（与场上发光色一致）
fn pickup_text_color(item_type: &PickupType) -> Color {
    match item_type {
        PickupType::Ammo { .. } => Color::srgb(0.9, 0.7, 0.2),
        PickupType::Health { .. } => Color::srgb(0.9, 0.2, 0.2),
        PickupType::Armor { .. } => Color::srgb(0.2, 0.5, 0.9),
        PickupType::Grenade { element } | PickupType::Weapon { element } => element.color(),
    }
}

fn interact_execute_system(
    mut keyboard: ResMut<ButtonInput<KeyCode>>,
    mut nearby: ResMut<NearbyInteract>,
    mut player_query: Query<(&mut Inventory, &WeaponSlot), With<Player>>,
    pickup_query: Query<&PickupItem>,
    mut commands: Commands,
    input_state: Res<InputState>,
    wheel: Res<WheelState>,
    held: Res<HeldGrenade>,
) {
    // UI 打开（光标解锁）/轮盘占用/持雷瞄准时不拾取
    if !input_state.cursor_locked || wheel.open || wheel.pending_key.is_some() || held.item.is_some() { return; }
    if !keyboard.just_pressed(KeyCode::KeyF) || nearby.entries.is_empty() { return; }
    // 只处理拾取物条目；站点条目由 station_system 开面板
    let Some(InteractEntry::Pickup(selected_entity)) = nearby.entries.get(nearby.selected).copied() else { return; };
    // 本次 F 已被拾取消费：清掉按下态，避免同帧 station_system 再把面板打开
    keyboard.clear_just_pressed(KeyCode::KeyF);
    let Ok((mut inventory, weapon_slot)) = player_query.get_single_mut() else { return };

    if let Ok(item) = pickup_query.get(selected_entity) {
        // 弹药直接补充弹药池，武器替换当前手持，其余物资入背包槽
        let mut taken = false;
        match &item.item_type {
            PickupType::Ammo { amount } => {
                inventory.ammo_pool += *amount;
                taken = true;
            }
            PickupType::Weapon { element } => {
                // 双主武器固定：新枪替换当前手持的那一把（旧枪弃置）
                let slot = weapon_slot.current;
                inventory.weapons[slot] = WeaponData::from_profile(&rifle_profile(*element));
                taken = true;
            }
            _ => {
                if inventory.items.len() < inventory.max_slots {
                    inventory.items.push(item.clone());
                    taken = true;
                }
            }
        }

        if taken {
            // Remove pickup entity from world
            commands.entity(selected_entity).despawn_recursive();
            // Remove from nearby list
            nearby.entries.retain(|e| !matches!(e, InteractEntry::Pickup(e2) if *e2 == selected_entity));
            if nearby.selected >= nearby.entries.len() && !nearby.entries.is_empty() {
                nearby.selected = nearby.entries.len() - 1;
            }
            // 窗口钳制：条目缩水后不越界，且保证选中项仍留在可见窗口内
            nearby.scroll_start = nearby.scroll_start
                .min(nearby.entries.len().saturating_sub(INTERACT_MENU_VISIBLE_ROWS))
                .min(nearby.selected);
        }
    }
}

/// 背包 UI 刷新：物资槽文本、武器架文本（槽位标记/弹匣数）、弹药池、悬停高亮
#[allow(clippy::type_complexity)]
fn inventory_ui_update(
    player_query: Query<&Inventory, With<Player>>,
    mut slot_texts: Query<(&InventorySlotText, &mut Text), (Without<BackpackWeaponText>, Without<HudBackpackAmmo>)>,
    mut weapon_texts: Query<(&BackpackWeaponText, &mut Text), (Without<InventorySlotText>, Without<HudBackpackAmmo>)>,
    mut ammo_text: Query<&mut Text, (With<HudBackpackAmmo>, Without<InventorySlotText>, Without<BackpackWeaponText>)>,
    #[allow(clippy::type_complexity)] mut slot_bgs: Query<
        (&InventorySlotUI, &Interaction, &mut BorderColor, &mut BackgroundColor),
        Without<BackpackWeaponSlot>,
    >,
    #[allow(clippy::type_complexity)] mut weapon_bgs: Query<
        (&BackpackWeaponSlot, &Interaction, &mut BorderColor, &mut BackgroundColor),
        Without<InventorySlotUI>,
    >,
) {
    let Ok(inventory) = player_query.get_single() else { return };

    // 物资槽
    for (slot, mut text) in slot_texts.iter_mut() {
        if let Some(item) = inventory.items.get(slot.0) {
            text.sections[0].value = item.name.clone();
        } else {
            text.sections[0].value = "".to_string();
        }
    }
    // 武器架：固定双主武器，槽位标记 + 名称 + 弹匣
    for (bw, mut text) in weapon_texts.iter_mut() {
        if let Some(w) = inventory.weapons.get(bw.0) {
            text.sections[0].value = format!("[{}] {}", bw.0 + 1, w.name);
            text.sections[0].style.color = w.element.color();
            text.sections[1].value = format!("\n{}/{} 弹", w.ammo, w.max_ammo);
            text.sections[1].style.color = Color::srgb(0.8, 0.8, 0.82);
        } else {
            text.sections[0].value = "空".to_string();
            text.sections[0].style.color = Color::srgb(0.35, 0.35, 0.35);
            text.sections[1].value = String::new();
        }
    }
    // 弹药池
    if let Ok(mut text) = ammo_text.get_single_mut() {
        text.sections[0].value = format!("弹药池 {}", inventory.ammo_pool);
    }
    // 物资槽悬停高亮：亮色边框 + 提亮底色，提示可按 R 使用
    for (_slot_ui, interaction, mut border, mut bg) in slot_bgs.iter_mut() {
        if *interaction == Interaction::Hovered {
            border.0 = Color::srgba(1.0, 0.8, 0.25, 0.95);
            bg.0 = Color::srgba(0.26, 0.27, 0.32, 0.95);
        } else {
            border.0 = Color::srgba(0.3, 0.3, 0.35, 0.6);
            bg.0 = Color::srgba(0.14, 0.14, 0.16, 0.95);
        }
    }
    // 武器架高亮：双主武器恒在架，金色边框；悬停白色
    for (bw, interaction, mut border, mut bg) in weapon_bgs.iter_mut() {
        let equipped = bw.0 < inventory.weapons.len();
        let hovered = *interaction == Interaction::Hovered;
        border.0 = if hovered {
            Color::srgba(0.95, 0.95, 0.95, 0.95)
        } else if equipped {
            Color::srgba(1.0, 0.8, 0.25, 0.95)
        } else {
            Color::srgba(0.3, 0.3, 0.35, 0.6)
        };
        bg.0 = if hovered {
            Color::srgba(0.26, 0.27, 0.32, 0.95)
        } else if equipped {
            Color::srgba(0.2, 0.18, 0.1, 0.95)
        } else {
            Color::srgba(0.14, 0.14, 0.16, 0.95)
        };
    }
}

// =============================================================================
// Inventory Item Usage (hover+R / quick keys / radial wheel)
// =============================================================================

/// 已从背包取出、正在瞄准持握的手雷：进入越肩瞄准姿态，左键投出 / Esc 取消放回。
/// 投掷物必须"先瞄准后释放"，因此手雷不再有任何即时投掷路径。
#[derive(Resource, Default)]
struct HeldGrenade {
    item: Option<PickupItem>,
}

#[derive(Component)]
struct HeldHintRoot;

/// 持握手雷时的屏幕提示文案（固定，只切显隐）
const HELD_HINT_TEXT: &str = "手持手雷 — 左键投掷 · Esc 取消";

/// 使用背包第 index 个道具：恢复类立即生效；手雷取出持握（不立即消耗弹道），
/// 进入越肩瞄准后由 grenade_throw_system 投出，Esc 取消放回背包。
fn use_item_at(
    index: usize,
    items: &mut Vec<PickupItem>,
    health: &mut Health,
    armor: &mut Armor,
    held: &mut HeldGrenade,
) -> Option<String> {
    let item = items.get(index)?.clone();
    match &item.item_type {
        // 弹药拾取时已直接入弹药池，不会作为背包物品出现在这里
        PickupType::Ammo { .. } => {}
        PickupType::Health { amount } => {
            health.current = (health.current + *amount).min(health.max);
        }
        PickupType::Armor { amount } => {
            armor.current = (armor.current + *amount).min(armor.max);
        }
        PickupType::Grenade { .. } => {
            // 投掷物必须先瞄准再释放：取出持握并进入瞄准姿态。
            // 已持握时本次使用作废，道具留在背包（items.remove 不会执行）。
            if held.item.is_some() { return None; }
            held.item = Some(item.clone());
        }
        // 武器通过背包"悬停+R 装备"使用，不走道具消耗
        PickupType::Weapon { .. } => return None,
    }
    items.remove(index);
    Some(item.name)
}

/// 背包内鼠标悬停物资槽 + 按 R：直接使用（武器固定双槽，无需背包内装备）
fn inventory_item_use_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    inventory_ui: Query<&Visibility, With<InventoryUI>>,
    item_slots: Query<(&InventorySlotUI, &Interaction)>,
    mut player_query: Query<(&mut Inventory, &mut Health, &mut Armor), With<Player>>,
    mut held: ResMut<HeldGrenade>,
) {
    let Ok(vis) = inventory_ui.get_single() else { return };
    if *vis != Visibility::Visible { return; }
    if !keyboard.just_pressed(KeyCode::KeyR) { return; }

    let Ok((mut inventory, mut health, mut armor)) = player_query.get_single_mut() else { return };

    // 悬停物资：使用
    let hovered_item = item_slots.iter()
        .find(|(_, interaction)| **interaction == Interaction::Hovered)
        .map(|(slot, _)| slot.0);
    let Some(slot_index) = hovered_item else { return };
    if slot_index >= inventory.items.len() { return; }
    use_item_at(slot_index, &mut inventory.items, &mut health, &mut armor, &mut held);
}

/// 手雷投掷：持握状态下左键沿相机视线（含俯仰）投出，Esc 取消放回背包。
/// 投掷方向取自相机真实朝向，与越肩准星对齐；持握期间 aim_system 强制瞄准。
#[allow(clippy::too_many_arguments)]
fn grenade_throw_system(
    mut commands: Commands,
    mut effects: ResMut<EffectAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    input_state: Res<InputState>,
    mut held: ResMut<HeldGrenade>,
    cam_query: Query<&GlobalTransform, With<PlayerCamera>>,
    mut player_query: Query<(&Transform, &mut Inventory), With<Player>>,
    mut hint_vis: Query<&mut Visibility, With<HeldHintRoot>>,
) {
    // 持握提示：只在持握时显示（文案固定，创建时已写好）
    if let Ok(mut vis) = hint_vis.get_single_mut() {
        *vis = if held.item.is_some() { Visibility::Visible } else { Visibility::Hidden };
    }
    if held.item.is_none() { return; }
    // UI 打开（光标解锁）时不投掷也不取消：左键属于界面
    if !input_state.cursor_locked { return; }
    let Ok((player_transform, mut inventory)) = player_query.get_single_mut() else { return };

    // Esc 取消：手雷放回背包（cursor_grab_toggle 在持握期间跳过 Esc，由这里接管）
    if keyboard.just_pressed(KeyCode::Escape) {
        if let Some(item) = held.item.take() {
            inventory.items.push(item);
        }
        return;
    }

    // 左键释放投掷：方向 = 相机视线（含俯仰），与准星一致
    if mouse.just_pressed(MouseButton::Left) {
        let Some(item) = held.item.take() else { return };
        let PickupType::Grenade { element } = item.item_type else {
            // 理论不可达（只有手雷会进入持握）：异常物品放回背包
            inventory.items.push(item);
            return;
        };
        let Ok(cam_tf) = cam_query.get_single() else { return };
        let (_, rotation, _) = cam_tf.to_scale_rotation_translation();
        let dir = (rotation * Vec3::NEG_Z).normalize();
        let origin = player_transform.translation + Vec3::Y * 1.7 + dir * 0.4;
        commands.spawn((
            PbrBundle {
                mesh: effects.projectile.clone(),
                material: effects.material(&mut materials, element, EffectMatKind::Plain),
                transform: Transform::from_translation(origin),
                ..default()
            },
            GrenadeProjectile {
                velocity: dir * 13.0 + Vec3::Y * 3.0,
                element,
                damage: ITEM_GRENADE_DAMAGE,
                radius: ITEM_GRENADE_RADIUS,
                effect: crate::operator::SkillEffect::NONE,
                timer: Timer::from_seconds(1.5, TimerMode::Once),
            },
        ));
    }
}

/// 长按 3/4 呼出的道具轮盘 UI（隐藏，由 item_wheel_system 控制显隐与布局）
fn setup_item_wheel(mut commands: Commands) {
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
        WheelRoot,
    )).with_children(|root| {
        // 中心区：提示条 + 取消按钮（点击撤销使用）
        root.spawn((
            NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(10.0),
                    ..default()
                },
                ..default()
            },
        )).with_children(|hub| {
            // 选中信息条
            hub.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Px(300.0),
                        height: Val::Px(46.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.06, 0.06, 0.09, 0.92)),
                    border_color: BorderColor(Color::srgba(0.45, 0.45, 0.5, 0.8)),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    ..default()
                },
            )).with_children(|bar| {
                bar.spawn((
                    TextBundle {
                        text: Text::from_section(
                            "",
                            TextStyle { font_size: 15.0, color: Color::srgb(0.92, 0.92, 0.92), ..default() }
                        ),
                        ..default()
                    },
                    WheelHubText,
                ));
            });
            // 取消按钮：点击撤销使用，光标悬停时清空选卡
            hub.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Px(120.0),
                        height: Val::Px(34.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.12, 0.07, 0.07, 0.92)),
                    border_color: BorderColor(Color::srgba(0.85, 0.35, 0.3, 0.8)),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    ..default()
                },
                Interaction::default(),
                WheelCancelButton,
            )).with_children(|btn| {
                btn.spawn((
                    TextBundle {
                        text: Text::from_section(
                            "✕ 取消使用",
                            TextStyle { font_size: 14.0, color: Color::srgb(0.95, 0.6, 0.55), ..default() }
                        ),
                        ..default()
                    },
                ));
            });
        });
        // 轮盘卡片：绝对定位，最多展示 8 格（与背包槽位上限一致）
        for i in 0..8 {
            root.spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        width: Val::Px(WHEEL_CARD_W),
                        height: Val::Px(WHEEL_CARD_H),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.10, 0.10, 0.13, 0.92)),
                    border_color: BorderColor(Color::srgba(0.35, 0.35, 0.4, 0.6)),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                },
                WheelCard(i),
            )).with_children(|card| {
                card.spawn((
                    TextBundle {
                        text: Text::from_section(
                            "",
                            TextStyle { font_size: 14.0, color: Color::srgb(0.9, 0.9, 0.9), ..default() }
                        ),
                        ..default()
                    },
                    WheelCardText(i),
                ));
            });
        }
    });
}

/// 3/4 键状态机：
/// 短按 → 快速使用背包中第一个对应类别道具；
/// 长按 WHEEL_OPEN_DELAY 秒 → 呼出轮盘（解锁光标），光标指向选卡，松开按键确认使用；
/// 光标停在中心或点击中心"取消"键 → 撤销使用（不消耗道具）。
#[allow(clippy::too_many_arguments)]
fn item_wheel_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    mut input_state: ResMut<InputState>,
    mut wheel: ResMut<WheelState>,
    mut player_query: Query<(&mut Inventory, &mut Health, &mut Armor), With<Player>>,
    mut held: ResMut<HeldGrenade>,
    mut root_vis: Query<&mut Visibility, (With<WheelRoot>, Without<WheelCard>)>,
    mut hub_text: Query<&mut Text, (With<WheelHubText>, Without<WheelCardText>)>,
    cancel_btn: Query<&Interaction, With<WheelCancelButton>>,
    mut cards: Query<(&WheelCard, &mut Style, &mut BorderColor, &mut Visibility, &mut BackgroundColor), Without<WheelRoot>>,
    mut card_texts: Query<(&WheelCardText, &mut Text), Without<WheelHubText>>,
) {
    // 0) 收起判定最先执行：松开 3/4 / Esc / 失焦 / 超时 → 本帧立刻收起并使用选中道具。
    //    必须放在显隐刷新与卡片布局之前——低端机上单帧渲染可达数秒，若先布局后判定，
    //    松手那一帧会带着整圈卡片多渲染数秒，看起来就是“松手后轮盘滞留”。
    if wheel.open {
        wheel.open_secs += time.delta_seconds();
        let wheel_key = if wheel.category == ItemCategory::Consumable { KeyCode::Digit3 } else { KeyCode::Digit4 };
        let window_focused = window_query.get_single().map(|w| w.focused).unwrap_or(true);
        let force_cancel = keyboard.just_pressed(KeyCode::Escape)
            || !window_focused
            || wheel.open_secs >= WHEEL_MAX_OPEN_SECS;
        if force_cancel || !keyboard.pressed(wheel_key) {
            let confirmed = !force_cancel && keyboard.just_released(wheel_key);
            if confirmed {
                // 中心撤销：无选中（光标停在中心/取消键上）→ 不消耗道具直接收起
                if let Some(sel) = wheel.selected {
                    if let Some(&inv_idx) = wheel.filtered.get(sel) {
                        if let Ok((mut inventory, mut health, mut armor)) = player_query.get_single_mut() {
                            use_item_at(inv_idx, &mut inventory.items, &mut health, &mut armor, &mut held);
                        }
                    }
                }
            }
            wheel.open = false;
            wheel.selected = None;
            if let Ok(mut window) = window_query.get_single_mut() {
                window.cursor.visible = false;
                window.cursor.grab_mode = CursorGrabMode::Locked;
            }
            input_state.cursor_locked = true;
        }
    }

    // 1) 根节点与卡片显隐兜底：按【收起判定之后】的状态刷新，任何提前 return
    //    （快速使用/判定中）都不会把轮盘留在屏幕上；卡片显式隐藏，不依赖父继承
    let Ok(mut root_vis) = root_vis.get_single_mut() else { return };
    *root_vis = if wheel.open { Visibility::Visible } else { Visibility::Hidden };
    if !wheel.open {
        for (_, _, _, mut visibility, _) in cards.iter_mut() {
            *visibility = Visibility::Hidden;
        }
    }

    // 1) 开始按住 3/4（仅游戏进行中、无 UI 占用时）
    if !wheel.open && wheel.pending_key.is_none() && input_state.cursor_locked {
        if keyboard.just_pressed(KeyCode::Digit3) {
            wheel.pending_key = Some(KeyCode::Digit3);
            wheel.pending_hold = 0.0;
        } else if keyboard.just_pressed(KeyCode::Digit4) {
            wheel.pending_key = Some(KeyCode::Digit4);
            wheel.pending_hold = 0.0;
        }
    }

    // 2) 待判定：短按快速使用 / 长按呼出轮盘
    if let Some(key) = wheel.pending_key {
        // 按住期间其他 UI（站点/背包）抢走了光标 → 放弃本次判定：
        // 既不快速使用也不呼出轮盘，避免轮盘叠在面板上、松键后滞留
        if !input_state.cursor_locked {
            wheel.pending_key = None;
            return;
        }
        wheel.pending_hold += time.delta_seconds();
        if keyboard.just_released(key) {
            wheel.pending_key = None;
            let category = if key == KeyCode::Digit3 { ItemCategory::Consumable } else { ItemCategory::Tactical };
            if let Ok((mut inventory, mut health, mut armor)) = player_query.get_single_mut() {
                if let Some(idx) = inventory.items.iter().position(|it| it.item_type.category() == category) {
                    use_item_at(idx, &mut inventory.items, &mut health, &mut armor, &mut held);
                }
            }
            return;
        } else if wheel.pending_hold >= WHEEL_OPEN_DELAY {
            wheel.pending_key = None;
            let category = if key == KeyCode::Digit3 { ItemCategory::Consumable } else { ItemCategory::Tactical };
            // 该类别没有道具时不呼出轮盘（快速轻点同样是无操作）
            let has_items = player_query.get_single()
                .map(|(inv, _, _)| inv.items.iter().any(|it| it.item_type.category() == category))
                .unwrap_or(false);
            if has_items {
                wheel.open = true;
                wheel.category = category;
                wheel.selected = None;
                wheel.open_secs = 0.0;
                if let Ok(mut window) = window_query.get_single_mut() {
                    window.cursor.visible = true;
                    window.cursor.grab_mode = CursorGrabMode::None;
                }
                input_state.cursor_locked = false;
            }
        } else {
            return; // 仍在判定中，本轮不动轮盘 UI
        }
    }

    // 轮盘未打开时到此为止（显隐已在顶部按状态刷新）。
    // 必须有此守卫：否则下方“打开态”逻辑会在正常游戏时每帧执行，
    // 把轮盘点亮又立刻取消，表现为“没长按 3/4 也常驻屏幕”。
    if !wheel.open { return; }

    // 3) 轮盘打开：收集同类道具、光标选卡（收起判定已前移到系统开头）
    let Ok((inventory, ..)) = player_query.get_single_mut() else { return };
    let filtered: Vec<usize> = inventory.items.iter().enumerate()
        .filter(|(_, it)| it.item_type.category() == wheel.category)
        .map(|(i, _)| i)
        .collect();
    wheel.filtered = filtered;
    if wheel.filtered.is_empty() {
        wheel.open = false;
        *root_vis = Visibility::Hidden;
        for (_, _, _, mut visibility, _) in cards.iter_mut() {
            *visibility = Visibility::Hidden;
        }
        if let Ok(mut window) = window_query.get_single_mut() {
            window.cursor.visible = false;
            window.cursor.grab_mode = CursorGrabMode::Locked;
        }
        input_state.cursor_locked = true;
        return;
    }
    *root_vis = Visibility::Visible;

    let Ok(mut window) = window_query.get_single_mut() else { return };
    let center = Vec2::new(window.width() * 0.5, window.height() * 0.5);

    // 点击轮盘中心"取消"键：撤销使用（不消耗道具），收起轮盘并锁定光标
    let hover_cancel = cancel_btn.iter().any(|i| *i != Interaction::None);
    if hover_cancel && mouse.just_pressed(MouseButton::Left) {
        wheel.open = false;
        wheel.selected = None;
        window.cursor.visible = false;
        window.cursor.grab_mode = CursorGrabMode::Locked;
        input_state.cursor_locked = true;
        return;
    }

    // 光标选卡：移出中心死区才选中；回到中心或悬停取消键即清空选择（此时松开 3/4 = 撤销）
    if let Some(cursor) = window.cursor_position() {
        let offset = cursor - center;
        if offset.length() > 40.0 && !hover_cancel {
            let angle = offset.y.atan2(offset.x);
            let n = wheel.filtered.len();
            let sector = std::f32::consts::TAU / n as f32;
            let mut best = 0usize;
            let mut best_dist = f32::MAX;
            for i in 0..n {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 * sector;
                let mut d = (a - angle).rem_euclid(std::f32::consts::TAU);
                if d > std::f32::consts::PI { d = std::f32::consts::TAU - d; }
                if d < best_dist { best_dist = d; best = i; }
            }
            wheel.selected = Some(best);
        } else {
            // 中心区 = 撤销位：不选中任何卡片
            wheel.selected = None;
        }
    }

    // 卡片环形布局 + 选中高亮 + 显隐 + 文本；超出道具数量的卡片隐藏
    let n = wheel.filtered.len();
    let sector = std::f32::consts::TAU / n as f32;
    for (card, mut style, mut border, mut visibility, mut bg) in cards.iter_mut() {
        let i = card.0;
        if i >= n {
            *visibility = Visibility::Hidden;
            continue;
        }
        *visibility = Visibility::Visible;
        let a = -std::f32::consts::FRAC_PI_2 + i as f32 * sector;
        let cx = center.x + a.cos() * WHEEL_RADIUS - WHEEL_CARD_W * 0.5;
        let cy = center.y + a.sin() * WHEEL_RADIUS - WHEEL_CARD_H * 0.5;
        style.position_type = PositionType::Absolute;
        style.left = Val::Px(cx);
        style.top = Val::Px(cy);
        let selected = Some(i) == wheel.selected;
        border.0 = if selected {
            Color::srgba(1.0, 0.8, 0.25, 0.95)
        } else {
            Color::srgba(0.35, 0.35, 0.4, 0.6)
        };
        bg.0 = if selected {
            Color::srgba(0.30, 0.28, 0.14, 0.95)
        } else {
            Color::srgba(0.10, 0.10, 0.13, 0.92)
        };
    }
    for (card_text, mut text) in card_texts.iter_mut() {
        if let Some(&inv_idx) = wheel.filtered.get(card_text.0) {
            if let Some(item) = inventory.items.get(inv_idx) {
                text.sections[0].value = item.name.clone();
                continue;
            }
        }
        text.sections[0].value = "".to_string();
    }
    if let Ok(mut hub) = hub_text.get_single_mut() {
        let sel_name = wheel.selected
            .and_then(|s| wheel.filtered.get(s))
            .and_then(|&idx| inventory.items.get(idx))
            .map(|item| item.name.as_str())
            .unwrap_or("移出中心选择道具");
        let cat_label = match wheel.category {
            ItemCategory::Consumable => "恢复",
            ItemCategory::Tactical => "战术",
        };
        hub.sections[0].value = format!("[{}] {} — 松开使用", cat_label, sel_name);
    }
}

// =============================================================================
// 功能站点：无限物资补给台 / 干员切换台
// =============================================================================

/// 构建两张功能台的交互面板（隐藏，由 station_system 控制显隐）
fn setup_station_ui(mut commands: Commands) {
    // --- 干员切换台面板 ---
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
        OperatorUIRoot,
    )).with_children(|root| {
        root.spawn(NodeBundle {
            style: Style {
                width: Val::Px(520.0),
                height: Val::Auto,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(8.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.06, 0.06, 0.08, 0.95)),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            ..default()
        }).with_children(|panel| {
            panel.spawn(TextBundle {
                text: Text::from_section(
                    "干员切换台",
                    TextStyle { font_size: 20.0, color: Color::srgb(0.9, 0.9, 0.9), ..default() }
                ),
                ..default()
            });
            panel.spawn(NodeBundle {
                style: Style { width: Val::Percent(100.0), height: Val::Px(2.0), ..default() },
                background_color: BackgroundColor(Color::srgba(0.4, 0.4, 0.5, 0.3)),
                ..default()
            });
            for i in 0..roster().len() {
                panel.spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            height: Val::Auto,
                            padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::srgba(0.10, 0.10, 0.13, 0.92)),
                        border_color: BorderColor(Color::srgba(0.35, 0.35, 0.4, 0.6)),
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        ..default()
                    },
                    Interaction::default(),
                    OperatorCard(i),
                )).with_children(|card| {
                    card.spawn((
                        TextBundle {
                            text: Text::from_sections([
                                TextSection::new("", TextStyle { font_size: 15.0, color: Color::WHITE, ..default() }),
                                TextSection::new("", TextStyle { font_size: 12.0, color: Color::srgb(0.75, 0.75, 0.78), ..default() }),
                                TextSection::new("", TextStyle { font_size: 12.0, color: Color::srgb(0.75, 0.75, 0.78), ..default() }),
                                TextSection::new("", TextStyle { font_size: 11.0, color: Color::srgb(0.5, 0.5, 0.55), ..default() }),
                            ]),
                            ..default()
                        },
                        OperatorCardText(i),
                    ));
                });
            }
            panel.spawn((
                TextBundle {
                    text: Text::from_section(
                        "",
                        TextStyle { font_size: 13.0, color: Color::srgb(0.6, 0.9, 0.6), ..default() }
                    ),
                    ..default()
                },
                OperatorStatusText,
            ));
            panel.spawn(TextBundle {
                text: Text::from_section(
                    "点击卡片切换干员（Q/E 技能组与元素随之更换） · F / Esc 关闭",
                    TextStyle { font_size: 12.0, color: Color::srgb(0.5, 0.5, 0.55), ..default() }
                ),
                ..default()
            });
        });
    });

    // --- 无限物资补给台面板 ---
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
        SupplyUIRoot,
    )).with_children(|root| {
        root.spawn(NodeBundle {
            style: Style {
                width: Val::Px(420.0),
                height: Val::Auto,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(8.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.06, 0.06, 0.08, 0.95)),
            border_radius: BorderRadius::all(Val::Px(8.0)),
            ..default()
        }).with_children(|panel| {
            panel.spawn(TextBundle {
                text: Text::from_section(
                    "补给台 · 无限物资",
                    TextStyle { font_size: 20.0, color: Color::srgb(0.9, 0.9, 0.9), ..default() }
                ),
                ..default()
            });
            panel.spawn(NodeBundle {
                style: Style { width: Val::Percent(100.0), height: Val::Px(2.0), ..default() },
                background_color: BackgroundColor(Color::srgba(0.4, 0.4, 0.5, 0.3)),
                ..default()
            });
            for (i, (name, hint)) in SUPPLY_ROWS.iter().enumerate() {
                panel.spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            height: Val::Auto,
                            padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::srgba(0.10, 0.10, 0.13, 0.92)),
                        border_color: BorderColor(Color::srgba(0.35, 0.35, 0.4, 0.6)),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                    Interaction::default(),
                    SupplyRow(i),
                )).with_children(|row| {
                    row.spawn(TextBundle {
                        text: Text::from_sections([
                            TextSection::new(*name, TextStyle { font_size: 14.0, color: Color::srgb(0.9, 0.9, 0.9), ..default() }),
                            TextSection::new(format!("  —  {}", hint), TextStyle { font_size: 11.0, color: Color::srgb(0.5, 0.5, 0.55), ..default() }),
                        ]),
                        ..default()
                    });
                });
            }
            panel.spawn((
                TextBundle {
                    text: Text::from_section(
                        "",
                        TextStyle { font_size: 13.0, color: Color::srgb(0.6, 0.9, 0.6), ..default() }
                    ),
                    ..default()
                },
                SupplyStatusText,
            ));
            panel.spawn(TextBundle {
                text: Text::from_section(
                    "点击行领取（弹药入池、其余入背包） · F / Esc 关闭",
                    TextStyle { font_size: 12.0, color: Color::srgb(0.5, 0.5, 0.55), ..default() }
                ),
                ..default()
            });
        });
    });
    // 旧的"站点提示条"已并入底部统一交互菜单（interact_menu_update），不再单独生成
}

/// 站点面板开关：F 打开统一交互菜单（NearbyInteract）中选中的站点条目，
/// F / Esc 关闭已开面板并锁回光标。Esc 不再直接开站点（开面板统一走 F + 交互菜单），
/// 避免背包关闭同帧 Esc 被本系统抢走而误开面板。
/// 运行顺序在 cursor_grab_toggle 之后；靠近检测由 interact_detection_system 负责。
#[allow(clippy::too_many_arguments)]
fn station_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    mut input_state: ResMut<InputState>,
    mut open: ResMut<OpenStation>,
    wheel: Res<WheelState>,
    held: Res<HeldGrenade>,
    nearby: Res<NearbyInteract>,
    mut supply_vis: Query<&mut Visibility, (With<SupplyUIRoot>, Without<OperatorUIRoot>)>,
    mut operator_vis: Query<&mut Visibility, (With<OperatorUIRoot>, Without<SupplyUIRoot>)>,
) {
    // 轮盘打开/按住期间、手雷持握时不处理：开会在轮盘/瞄准上叠面板；关会把光标锁回
    let wheel_busy = wheel.open || wheel.pending_key.is_some();
    if wheel_busy || held.item.is_some() { return; }

    if keyboard.just_pressed(KeyCode::KeyF) {
        if *open == OpenStation::None {
            // 无 UI 占用（光标锁定）且选中条目是站点时才开面板；拾取物条目交给 interact_execute_system
            if input_state.cursor_locked {
                if let Some(InteractEntry::Station { kind, .. }) = nearby.entries.get(nearby.selected) {
                    *open = match kind {
                        StationKind::SupplyTable => OpenStation::Supply,
                        StationKind::OperatorDesk => OpenStation::Operator,
                    };
                    if let Ok(mut window) = window_query.get_single_mut() {
                        window.cursor.visible = true;
                        window.cursor.grab_mode = CursorGrabMode::None;
                    }
                    input_state.cursor_locked = false;
                }
            }
        } else {
            close_station_panel(&mut window_query, &mut input_state, &mut open);
        }
    } else if keyboard.just_pressed(KeyCode::Escape) {
        // Esc 只负责关闭已打开的面板
        if *open != OpenStation::None {
            close_station_panel(&mut window_query, &mut input_state, &mut open);
        }
    }

    // 面板显隐兜底：每帧按状态刷新，任何提前 return 都不会把面板留在屏幕上
    if let Ok(mut vis) = supply_vis.get_single_mut() {
        *vis = if *open == OpenStation::Supply { Visibility::Visible } else { Visibility::Hidden };
    }
    if let Ok(mut vis) = operator_vis.get_single_mut() {
        *vis = if *open == OpenStation::Operator { Visibility::Visible } else { Visibility::Hidden };
    }
}

/// 关闭站点面板：锁回光标并清空打开态
fn close_station_panel(
    window_query: &mut Query<&mut Window, With<PrimaryWindow>>,
    input_state: &mut InputState,
    open: &mut OpenStation,
) {
    *open = OpenStation::None;
    if let Ok(mut window) = window_query.get_single_mut() {
        window.cursor.visible = false;
        window.cursor.grab_mode = CursorGrabMode::Locked;
    }
    input_state.cursor_locked = true;
}

/// 无限补给台：按行发放物资（弹药直接入弹药池，其余入背包物资槽）
fn grant_supply(row: usize, inventory: &mut Inventory) -> String {
    match row {
        0 => {
            inventory.ammo_pool += 60;
            "已领取：步枪弹药 ×60（入弹药池）".to_string()
        }
        1 => push_supply(inventory, "医疗包", PickupType::Health { amount: 25.0 }),
        2 => push_supply(inventory, "护甲片", PickupType::Armor { amount: 20.0 }),
        3 => push_supply(inventory, "烈焰手雷", PickupType::Grenade { element: ElementType::Fire }),
        4 => push_supply(inventory, "冰霜手雷", PickupType::Grenade { element: ElementType::Ice }),
        5 => push_supply(inventory, "雷电手雷", PickupType::Grenade { element: ElementType::Electric }),
        6 => push_supply(inventory, "毒素手雷", PickupType::Grenade { element: ElementType::Poison }),
        _ => String::new(),
    }
}

fn push_supply(inventory: &mut Inventory, name: &str, item_type: PickupType) -> String {
    if inventory.items.len() >= inventory.max_slots {
        return "背包已满（先在背包中使用道具）".to_string();
    }
    inventory.items.push(PickupItem { name: name.to_string(), item_type });
    format!("已领取：{}", name)
}

/// 补给台点击：悬停行 + 左键领取一行物资（无限次）
fn supply_station_click_system(
    mouse: Res<ButtonInput<MouseButton>>,
    open: Res<OpenStation>,
    rows: Query<(&SupplyRow, &Interaction)>,
    mut player_query: Query<&mut Inventory, With<Player>>,
    mut status: Query<&mut Text, (With<SupplyStatusText>, Without<OperatorStatusText>)>,
) {
    if *open != OpenStation::Supply { return; }
    if !mouse.just_pressed(MouseButton::Left) { return; }
    let Some((row, _)) = rows.iter().find(|(_, interaction)| **interaction == Interaction::Pressed) else { return };
    let Ok(mut inventory) = player_query.get_single_mut() else { return };
    let msg = grant_supply(row.0, &mut inventory);
    if let Ok(mut text) = status.get_single_mut() {
        text.sections[0].value = msg;
    }
}

/// 干员切换台点击：点击卡片切换当前干员（技能组/冷却/角色饰条色一并更新）
fn operator_station_click_system(
    mouse: Res<ButtonInput<MouseButton>>,
    open: Res<OpenStation>,
    cards: Query<(&OperatorCard, &Interaction)>,
    mut player_query: Query<(&mut OperatorState, &OperatorAccent), With<Player>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut status: Query<&mut Text, (With<OperatorStatusText>, Without<SupplyStatusText>)>,
) {
    if *open != OpenStation::Operator { return; }
    if !mouse.just_pressed(MouseButton::Left) { return; }
    let Some((card, _)) = cards.iter().find(|(_, interaction)| **interaction == Interaction::Pressed) else { return };
    let Ok((mut op, accent)) = player_query.get_single_mut() else { return };
    if card.0 == op.active {
        if let Ok(mut text) = status.get_single_mut() {
            text.sections[0].value = format!("当前已是 {}", roster()[op.active].name);
        }
        return;
    }
    switch_operator(&mut op, accent, &mut materials, card.0);
    if let Ok(mut text) = status.get_single_mut() {
        text.sections[0].value = format!("已切换至 {}", roster()[card.0].name);
    }
}

/// 补给台面板：悬停行高亮
#[allow(clippy::type_complexity)]
fn supply_ui_update_system(
    mut rows: Query<(&SupplyRow, &Interaction, &mut BorderColor, &mut BackgroundColor), Without<OperatorCard>>,
) {
    for (_row, interaction, mut border, mut bg) in rows.iter_mut() {
        if *interaction == Interaction::Hovered {
            border.0 = Color::srgba(1.0, 0.8, 0.25, 0.95);
            bg.0 = Color::srgba(0.26, 0.27, 0.32, 0.95);
        } else {
            border.0 = Color::srgba(0.35, 0.35, 0.4, 0.6);
            bg.0 = Color::srgba(0.10, 0.10, 0.13, 0.92);
        }
    }
}

/// 干员切换台面板：卡片高亮（当前干员金色 + 元素底色，悬停白框）与文本刷新
#[allow(clippy::type_complexity)]
fn operator_ui_update_system(
    mut cards: Query<(&OperatorCard, &Interaction, &mut BorderColor, &mut BackgroundColor), Without<SupplyRow>>,
    mut texts: Query<(&OperatorCardText, &mut Text)>,
    player_query: Query<&OperatorState, With<Player>>,
) {
    let active = player_query.get_single().map(|op| op.active).unwrap_or(usize::MAX);
    for (card, interaction, mut border, mut bg) in cards.iter_mut() {
        let selected = card.0 == active;
        let hovered = *interaction == Interaction::Hovered;
        border.0 = if hovered {
            Color::srgba(0.95, 0.95, 0.95, 0.95)
        } else if selected {
            Color::srgba(1.0, 0.8, 0.25, 0.95)
        } else {
            Color::srgba(0.35, 0.35, 0.4, 0.6)
        };
        bg.0 = if selected {
            roster().get(card.0).map(|op| op.element.color().with_alpha(0.28)).unwrap_or(Color::srgba(0.10, 0.10, 0.13, 0.92))
        } else if hovered {
            Color::srgba(0.3, 0.3, 0.35, 0.92)
        } else {
            Color::srgba(0.10, 0.10, 0.13, 0.92)
        };
    }
    for (ct, mut text) in texts.iter_mut() {
        let Some(op) = roster().get(ct.0) else { continue };
        let mark = if ct.0 == active { "   ✓ 当前" } else { "" };
        text.sections[0].value = format!("{} · {}{}", op.name, op.title, mark);
        text.sections[0].style.color = op.element.color();
        text.sections[1].value = format!(
            "\nQ {}（{}s 冷却）：{}",
            op.q.name, op.q.cooldown_secs, op.q.desc
        );
        text.sections[1].style.color = Color::srgb(0.75, 0.75, 0.78);
        text.sections[2].value = format!(
            "\nE {}（{}s 冷却）：{}",
            op.e.name, op.e.cooldown_secs, op.e.desc
        );
        text.sections[2].style.color = Color::srgb(0.75, 0.75, 0.78);
        text.sections[3].value = format!("\n{}", op.passive);
        text.sections[3].style.color = Color::srgb(0.5, 0.5, 0.55);
    }
}

// =============================================================================
// Kill Feed (top-right)
// =============================================================================

/// 底部 3/4 道具图标：显示背包中对应类别的数量，无货时按键变灰
fn hud_item_slots_system(
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

fn kill_feed_system(
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




