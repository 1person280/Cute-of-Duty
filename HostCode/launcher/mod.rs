//! launcher 模块 —— 客户端表现层装配入口（服务器权威架构的表现半边）
//!
//! 设计动机：表现层各子域（`flow` 状态机、`net` 网络、`menu` 菜单、`hud` 战斗内 UI、
//! `world` 3D 场景、`shared` 共享资源）是与本模块平级的六个顶层模块（见 main.rs 的
//! 模块声明）。本模块只做装配：把它们的系统/资源挂进 Bevy App，绝不本地模拟实体状态。

use std::sync::mpsc;

use bevy::prelude::*;
use cute_of_duty_server::net::protocol::{ClientMessage, EntitySnapshot};

use crate::flow::AppState;

/// 入口：建快照/控制/上行三路通道，起后台网络拉取线程，跑 Bevy 游玩主循环。
///
/// `addr` 为服务端监听地址（如 `127.0.0.1:8888`）。
pub fn run(addr: &str) {
    // 三路通道：快照（渲染对账）、下行控制（握手/事件/延迟/撤离回程）、上行意图。
    let (snapshot_tx, snapshot_rx) = mpsc::channel::<Vec<EntitySnapshot>>();
    let (control_tx, control_rx) = mpsc::channel::<crate::net::ClientInbound>();
    let (up_tx, up_rx) = mpsc::channel::<ClientMessage>();
    let net_addr = addr.to_string();
    std::thread::spawn(move || {
        crate::net::run_pull_loop(&net_addr, snapshot_tx, control_tx, up_rx);
    });

    App::new()
        // 贴图/资源根目录固定在 HostCode/assets（编译期绝对路径），
        // 摆脱「exe 所在目录/cwd」依赖 —— 否则 bevy 会去 target\debug\assets 找不到旧 UI 贴图。
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/assets").to_string(),
            ..default()
        }))
        .insert_resource(crate::net::SnapshotBuffer::new(snapshot_rx))
        .insert_resource(crate::net::ControlBuffer(std::sync::Mutex::new(control_rx)))
        .insert_resource(crate::net::NetOut(up_tx))
        // 中文字体句柄默认缺失（Default=None）但资源恒存在，避免任何 UI 系统
        // 在字体注入前的首帧对 `Res<CjkFont>` 取值 panic。
        .init_resource::<crate::flow::CjkFont>()
        .init_resource::<crate::menu::GameSettings>()
        .init_resource::<crate::menu::SelectedMode>()
        .init_resource::<crate::menu::SelectedCategory>()
        .init_state::<AppState>()
        // bevy 0.14 必须显式启用 state-scoped 清理：`init_state` 只注册状态机与 OnEnter/OnExit，
        // 不会自动把 `clear_state_scoped_entities` 挂到 StateTransition。缺这一步则所有
        // `StateScoped(AppState::*)` 实体（主菜单/加载屏/HUD）永不销毁——进训练场时主菜单
        // 整屏覆盖在 3D 上、UI 闪烁叠加、无法操作。显式开启后离开状态自动递归销毁对应 UI 树。
        .enable_state_scoped_entities::<AppState>()
        // 全局资源 + 场景体积网格，仅在启动时准备一次。
        .add_systems(
            Startup,
            (
                crate::flow::setup_global,
                crate::shared::init_ui_assets,
                // world_assets 必须先于 spawn_scene 建好，world 才读得到贴图句柄。
                (crate::world::init_world_assets, spawn_scene).chain(),
            ),
        )
        // 全局常驻：快照对账、相机跟随、控制路由、延迟面板、设置应用，以及 HUD 贴图就绪。
        .add_systems(
            Update,
            (
                crate::net::receive_snapshots,
                crate::net::apply_entities,
                crate::world::follow_system,
                crate::flow::route_control_messages,
                crate::shared::refresh_ui_ready,
                crate::world::refresh_world_ready,
                crate::net::caps_toggle,
                crate::net::spawn_panel,
                crate::net::panel_update,
                crate::menu::settings_apply_fov,
                crate::menu::settings_apply_ambient,
            ),
        )
        // Loading 屏惰性生成 + 进度刷新（仅 Loading 态）。
        .add_systems(
            Update,
            (crate::flow::spawn_loading, crate::flow::loading_tick).run_if(in_state(AppState::Loading)),
        )
        // 状态驱动：Loading → MainMenu → InGame → MainMenu。
        .add_systems(OnEnter(AppState::MainMenu), crate::menu::spawn_menu)
        .add_systems(
            Update,
            (
                crate::menu::tick_grace,
                crate::menu::main_menu_interaction,
                crate::menu::main_menu_style,
                crate::menu::main_menu_loadout,
                crate::menu::ensure_overlay,
                crate::menu::arsenal_interaction,
            )
                .chain()
                .run_if(in_state(AppState::MainMenu)),
        )
        .add_systems(OnEnter(AppState::InGame), crate::hud::spawn_hud)
        .add_systems(
            Update,
            (
                crate::hud::update_extract,
                crate::hud::extract_interaction,
                crate::hud::update_vitals,
                crate::hud::update_minimap,
                crate::hud::update_skills,
                crate::hud::update_feed,
                crate::hud::update_kill,
                crate::hud::operator_highlight,
                crate::hud::operator_input,
                crate::net::input_system,
            )
                .chain()
                .run_if(in_state(AppState::InGame)),
        )
        .run();
}

/// 场景初始化：轨道相机 + 静态环境 + 共享体素网格。
fn spawn_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world_assets: Res<crate::world::WorldAssets>,
) {
    crate::world::spawn_camera(&mut commands);
    crate::world::spawn_world(&mut commands, &mut meshes, &mut materials, &world_assets);
    commands.insert_resource(crate::net::CubeMesh {
        handle: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
    });
    // 全局环境光：供「设置浮层 · 环境亮度」调节写入（世界仅配了 DirectionalLight）。
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.55,
    });
}
