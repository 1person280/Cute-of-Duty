//! Cute Of Duty 1: Simple - 3D Pixel FPS Demo（组装层）
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
//!
//! 模块划分（本文件只负责组装：插件/资源/系统注册，不含任何玩法逻辑）：
//! - `common`     光标锁定、CJK 字体、元素系统胶水、AppState
//! - `components` 全部组件/标记定义（纯数据）
//! - `menu`       主菜单与加载屏
//! - `pause`      暂停菜单与设置
//! - `world`      地图布局落地与材质/资产池
//! - `character`  角色（玩家/敌人）生成
//! - `controller` FPS 移动与碰撞
//! - `camera`     相机 Rig 与瞄准
//! - `combat`     射击/技能/手雷/爆炸
//! - `targets`    靶标逻辑
//! - `minimap`    小地图与罗盘
//! - `hud`        HUD 与击杀播报
//! - `inventory`  背包/交互菜单/物品轮盘
//! - `stations`   补给台/干员切换台

use bevy::prelude::*;
use bevy::asset::AssetPlugin;
use crate::element::ElementSystem;
use crate::model::{operator_model_swap_system, yanhu_action_system};
use bevy::window::WindowPlugin;

pub mod camera; // model 直接依赖相机组件（避免 demo⇄model 整体互相 use）
mod character;
mod combat;
mod common;
mod components;
mod controller;
mod hud;
mod inventory;
mod menu;
mod minimap;
mod pause;
mod stations;
mod targets;
mod world;

// model 依赖相机组件：保持原 `crate::demo::X` 路径可用
pub use camera::{CamPivot, PitchPivot, ShoulderPivot, SpringArm, lerp};
use camera::*;

use combat::*;
use common::*;
use components::*;
use controller::*;
use hud::*;
use inventory::*;
use menu::*;
use minimap::*;
use pause::*;
use stations::*;
use targets::*;
use world::*;

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

