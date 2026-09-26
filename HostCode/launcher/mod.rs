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
        // 造型材质缓存：避免实体随 AOI 进出视野反复 spawn 时材质资源单调累积。
        .init_resource::<crate::net::EntityMaterials>()
        // 中文字体句柄默认缺失（Default=None）但资源恒存在，避免任何 UI 系统
        // 在字体注入前的首帧对 `Res<CjkFont>` 取值 panic。
        .init_resource::<crate::flow::CjkFont>()
        .init_resource::<crate::menu::GameSettings>()
        .init_resource::<crate::menu::SelectedMode>()
        .init_resource::<crate::menu::SelectedCategory>()
        .init_resource::<crate::menu::ArsenalVisible>()
        .init_resource::<crate::menu::ArsenalSelection>()
        .init_resource::<crate::menu::ArsenalDrag>()
        // 暂停门控资源常驻（`pause_closed` 运行条件与暂停系统都读它；默认 Closed）。
        .init_resource::<crate::menu::PauseMenu>()
        // 战术全景图门控资源常驻（`bigmap_closed` 运行条件读它；默认关闭）。
        .init_resource::<crate::hud::BigMapOpen>()
        // 交互状态资源常驻（`gameplay_input_active` 读它的 open 字段冻结输入；默认关）。
        .init_resource::<crate::hud::InteractState>()
        // 消耗品径向轮盘 / 物资箱格位面板门控资源常驻（默认关；`gameplay_input_active` 读其 open）。
        .init_resource::<crate::hud::ItemWheelState>()
        .init_resource::<crate::hud::LootPanelState>()
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
                spawn_scene,
            ),
        )
        // 全局常驻：快照对账、鼠标视角、相机跟随、控制路由、延迟面板、设置应用，以及
        // HUD 贴图就绪。整体 `.chain()`：保证「收快照 → 对账」「鼠标写入 AimRig → 相机读」
        // 的先后关系（Res/ResMut 冲突下 bevy 默认不定序，显式串联才确定）。
        // 鼠标视角与相机跟随在暂停时冻结（`pause_closed`）：暂停期间视角/朝向不再变化。
        .add_systems(
            Update,
            (
                crate::net::receive_snapshots,
                crate::net::apply_entities,
                crate::world::mouse_look_system.run_if(crate::hud::gameplay_input_active),
                crate::world::follow_system.run_if(crate::menu::pause_closed),
                crate::flow::route_control_messages,
                crate::shared::refresh_ui_ready,
                crate::net::caps_toggle,
                crate::net::spawn_panel,
                crate::net::panel_update,
                crate::menu::settings_apply_fov,
                crate::menu::settings_apply_ambient,
                // 光标锁定随状态/暂停翻转，需在各状态下都跑（自身判态，无 run_if）。
                crate::menu::cursor_lock_system,
            )
                .chain(),
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
                crate::menu::arsenal_drag_system,
                crate::menu::arsenal_interaction,
            )
                .chain()
                .run_if(in_state(AppState::MainMenu)),
        )
        .add_systems(OnEnter(AppState::InGame), crate::hud::spawn_hud)
        // 离开训练场：兜底销毁暂停浮层与复位全景图门控（返回主界面 / 状态切换通用）。
        .add_systems(
            OnExit(AppState::InGame),
            (
                crate::menu::teardown_pause,
                crate::hud::reset_bigmap,
                crate::hud::reset_interact,
                crate::hud::reset_item_wheel,
                crate::hud::reset_loot_panel,
            ),
        )
        .add_systems(
            Update,
            (
                // 组一：开关/引导/交互链（顺序敏感）。bevy 0.14 的 `.chain()` 只对 ≤20 元组
                // 提供实现，故拆成两个内部链后再外层链，保持总的先后顺序不变。
                (
                    // 暂停开关与面板交互最先跑：同帧生效的暂停门控可立即冻结下方输入。
                    crate::menu::pause_toggle,
                    crate::menu::pause_menu_interaction,
                    // 全景图开关紧随暂停之后：同帧生效的地图门控可立即冻结下方输入/视角。
                    crate::hud::bigmap_toggle,
                    crate::hud::update_bigmap,
                    crate::hud::update_extract,
                    crate::hud::extract_interaction,
                    // 交互链：刷附近目标 → 轮盘/物资箱输入 → F/滚轮/点击输入 → 点击选项 → 上报 → 重建。
                    // 注意：交互输入本身**不**受 `gameplay_input_active` 门控（面板打开时
                    // 正是它负责响应选择/关闭），仅下游玩法输入（移动/开火/切枪）被面板状态冻结。
                    // 轮盘与物资箱面板排在交互输入**之前**：同帧按下的 F/滚轮由它们优先接管，
                    // 避免"开面板当帧就被交互链重复消费"。
                    crate::hud::update_interact_entries,
                    crate::hud::item_wheel_input,
                    crate::hud::loot_panel_input,
                    // 格位面板鼠标搬运（拖拽 / Shift+左键）：与键盘后备同帧，先于交互输入。
                    crate::hud::loot_panel_drag,
                    crate::hud::interact_input,
                    crate::hud::interact_menu_click,
                    crate::hud::interact_commit,
                    crate::hud::sync_interact_panel,
                    crate::hud::sync_interact_menu,
                )
                    .chain(),
                // 组二：HUD 刷新 + 玩法输入。
                (
                    crate::hud::update_vitals_bars,
                    crate::hud::update_vitals_text,
                    crate::hud::update_crosshair,
                    crate::hud::update_minimap,
                    crate::hud::update_skills,
                    crate::hud::update_feed,
                    crate::hud::update_alert,
                    crate::hud::update_kill,
                    // 模态覆盖层绘制：径向轮盘扇区 / 物资箱两个 4×3 网格。
                    crate::hud::update_item_wheel,
                    crate::hud::sync_loot_panel,
                    // 玩法意图输入在暂停/全景图/交互二级面板/物资箱面板/径向轮盘打开时冻结。
                    crate::net::input_system.run_if(crate::hud::gameplay_input_active),
                )
                    .chain(),
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
) {
    crate::world::spawn_camera(&mut commands);
    crate::world::spawn_world(&mut commands, &mut meshes, &mut materials);
    commands.insert_resource(crate::net::CubeMesh {
        handle: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
    });
    // 全局环境光：供「设置浮层 · 环境亮度」调节写入（世界仅配了 DirectionalLight）。
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.55,
    });
}
