//! Cute Of Duty 服务端（cod_server）
//!
//! 服务器权威架构落地：本进程持有**所有热数据**（世界模拟、元素状态、战局），
//! 以固定 Tick（60Hz，复用 `engine::GameLoop` 的确定性模拟）推进；每个 Tick
//! 消费客户端输入意图 → 计算 → 构建已按 AOI 过滤的权威快照 → 广播给各客户端。
//!
//! 客户端只发输入、只收快照；本进程是游戏世界的唯一真理源。
//!
//! 编排边界：本文件是**唯一编排者**——只把各模块句柄接起来（启动装配 + 固定 Tick 主循环）。
//! 每 Tick 各阶段的实现已抽到 [`cute_of_duty_server::net::stages`]，web 装配抽到
//! [`cute_of_duty_server::net::web::spawn`]，本文件因而保持精简、仅剩装配与循环骨架。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::{info, warn};

use cute_of_duty_server::config;
use cute_of_duty_server::element::{ElementSystem, EntityElementState};
use cute_of_duty_server::engine::{GameLoop, TickConfig};
use cute_of_duty_server::equipment::EquipmentSystem;
use cute_of_duty_server::net::protocol::PlayerInput;
use cute_of_duty_server::net::{self, NetCommand, NetRuntime};
use cute_of_duty_server::entity::EntityId;
use cute_of_duty_server::player::PlayerProfile;
use cute_of_duty_server::storage::JsonLogRepo;

/// 服务端默认监听地址（本机回环；正式环境改为对外网卡并置于反向代理后）。
const DEFAULT_ADDR: &str = "127.0.0.1:8888";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("========================================");
    info!("Cute Of Duty 服务器权威（cod_server）");
    info!("========================================");

    // 配置：单一事实来源（服务器权威持有并加载）
    let (element_config, config_path) = config::load_element_config()?;
    match &config_path {
        Some(path) => info!("元素配置加载完成: {}", path.display()),
        None => warn!("未找到配置文件，使用内置默认配置"),
    }

    let element_system = Arc::new(ElementSystem::new(element_config));
    let equipment_system = Arc::new(EquipmentSystem::new());
    // 预留一份给背包 CRUD 服务（装备热实例注册表；主循环经它铸造/丢弃装备）
    let equipment_for_loop = Arc::clone(&equipment_system);

    let tick_config = TickConfig {
        tick_rate_hz: 60,
        max_frame_time_ms: 50.0,
        enable_determinism_check: true,
    };
    let mut sim = GameLoop::new(tick_config, element_system, equipment_system);
    sim.set_environment(EntityElementState::Normal);

    // 进场即生成训练场实弹靶（服务端权威：靶位来自地图数据，命中计分由 combat 结算）。
    // 活动地图 = 0.3.2 运行时使用的 `map::lawn`（1×1km 露天搜打撤大场）。
    let training = cute_of_duty_server::map::lawn::layout();
    cute_of_duty_server::combat::range::spawn_range_targets(sim.world_mut(), &training.targets);
    let (pickups, stations) =
        cute_of_duty_server::interact::spawn_from_layout(sim.world_mut(), &training);
    info!(
        "训练场已就绪：{} 个靶机 / {} 个拾取物 / {} 个功能站点进场",
        training.targets.len(),
        pickups,
        stations
    );

    // 网络会话运行时
    let mut runtime = NetRuntime::new();
    let mut cmd_rx = runtime.take_command_receiver();
    let rt = Arc::new(runtime);

    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_ADDR.to_string());

    // 接受连接（独立任务）
    let rt_for_accept = Arc::clone(&rt);
    tokio::spawn(async move {
        if let Err(e) = net::accept_loop(addr, rt_for_accept).await {
            warn!("accept_loop 退出: {e}");
        }
    });

    // Web 服务（服务端内置）：门户/状态页 + 运维 API。装配细节见 `net::web::spawn`。
    let player_slot = net::web::spawn::spawn_web(Arc::clone(&rt))?;

    // 冷数据仓库（玩家档案/背包/货币/战绩——跨局需保存、重连要续回的数据）
    // 配置驱动目录：环境变量 COD_DATA_DIR > workspace 根 ServerCode/data（档案落其 profiles/ 子目录）。
    let mut repo = cute_of_duty_server::storage::open_repo()?;
    info!("冷数据仓库就绪: {}", repo_dir_display());

    // 服务端权威主循环
    serve_authoritative(
        &mut sim,
        &rt,
        &mut cmd_rx,
        tick_config.tick_rate_hz,
        &mut repo,
        &equipment_for_loop,
        &player_slot,
    )
    .await;

    Ok(())
}

/// 打印仓库根目录（供启动日志；仅用于展示）。
fn repo_dir_display() -> String {
    cute_of_duty_server::storage::resolve_data_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "<未解析，回退 data>".to_string())
}

/// 服务器权威主循环：60Hz 固定 Tick + 输入消费 + 快照广播。
///
/// 设计动机：长 TTK 游戏不必支持毫秒级瞬时同步，固定 Tick 的确定性权威循环
/// 足以承载 TCP 可靠同步；每 Tick 一次"算→广播"即实现「服务端算、客户端看」。
/// 各阶段实现见 [`net::stages`]，本函数只负责按序驱动它们。
async fn serve_authoritative(
    sim: &mut GameLoop,
    rt: &Arc<NetRuntime>,
    cmd_rx: &mut mpsc::UnboundedReceiver<NetCommand>,
    tick_rate_hz: u32,
    repo: &mut JsonLogRepo,
    equipment: &Arc<EquipmentSystem>,
    player_slot: &net::web::PlayerSlot,
) {
    let tick_interval = std::time::Duration::from_micros(1_000_000 / u64::from(tick_rate_hz));
    let dt = 1.0 / tick_rate_hz as f32;

    // conn_id → 该客户端在权威世界的实体
    let mut conn_entity: HashMap<u64, EntityId> = HashMap::new();
    // conn_id → 该连接正在经手的玩家冷档案（热副本：连接期内驻内存，断开落盘）
    let mut conn_profiles: HashMap<u64, PlayerProfile> = HashMap::new();
    // conn_id → 仓库选装携带清单（本局会话热副本：断开即弃，跨局冷数据走 conn_profiles）
    let mut conn_loadout: HashMap<u64, Vec<String>> = HashMap::new();
    // conn_id → 该连接最近一次的输入意图。固定 Tick 玩法的关键（Why）：客户端按渲染帧
    // 上报意图，频率与网络批处理都不可控；服务端只保留"最新意图"，每 Tick 结算一次
    // （位移 = 速度 × dt），移速因而与客户端帧率/消息条数彻底解耦，天然确定性。
    let mut conn_input: HashMap<u64, PlayerInput> = HashMap::new();
    // conn_id → 该连接**常驻资源集合**（已下发且客户端仍持有的资源键）。0.12：客户端对象池
    // 淘汰时经主通道上报 `PoolSync`，服务端据此移除；未来该键需要时再经资源通道重发。
    let mut conn_resident: HashMap<u64, HashSet<u64>> = HashMap::new();
    // 全局装备 ID 分配器（跨连接唯一，供背包 Craft 裁决）
    let mut next_equipment_id: u64 = 1000;

    loop {
        // 阶段1：消费本帧所有客户端输入意图（非阻塞清空队列；Input 只存最新不立即结算）
        net::stages::drain_commands(
            sim,
            rt,
            &mut conn_entity,
            &mut conn_profiles,
            &mut conn_loadout,
            &mut conn_input,
            &mut conn_resident,
            repo,
            equipment,
            &mut next_equipment_id,
            cmd_rx,
        );

        // 阶段1.5：按最新意图推进所有玩家（位移/朝向/战斗），每 Tick 恰好一次。
        // 与 dt 配套构成确定性结算：速度单位 m/s，位移 = 速度 × dt。
        for (&conn_id, &eid) in conn_entity.iter() {
            if let Some(input) = conn_input.get(&conn_id) {
                net::stages::apply_input(sim, eid, input, dt);
            }
        }

        // 阶段1.6：清空已消费的边沿量（换弹/技能/取消持雷），避免同一次按键在后续 Tick
        // 被重复触发。扳机（shoot）是持续量，按住即持续开火，故不在此清空。
        for input in conn_input.values_mut() {
            input.reload = false;
            input.skill_q = false;
            input.skill_e = false;
            input.weapon_slot = None;
            input.use_slot = None;
            input.grenade_cancel = false;
        }

        // 阶段2：推进确定性模拟一个 Tick
        sim.tick(dt);

        // 阶段2.5：转发战斗事件（击杀/命中）到对应连接，独立于快照各自按序下发
        net::stages::forward_combat_events(rt, &conn_entity, &sim.drain_combat_events());

        // 阶段3：为每个在线客户端构建 AOI 过滤后的权威快照并扇出
        let mut disconnected = Vec::new();
        for (&conn_id, &eid) in conn_entity.iter() {
            let observer = match sim.world().get_entity(eid) {
                Some(entity) => (entity.position.x, entity.position.y, entity.position.z),
                None => {
                    // 实体已被战局清理：注销该连接
                    disconnected.push(conn_id);
                    continue;
                }
            };
            let snapshot = net::build_snapshot(sim.world(), sim.current_tick(), observer);
            rt.send_to(conn_id, snapshot);
        }
        for conn_id in disconnected {
            conn_entity.remove(&conn_id);
        }

        // 阶段3.5：刷新 web 侧玩家展示槽（姓名/等级来自本连接热档案；web 只读快照）。
        net::stages::refresh_web_players(player_slot, &conn_entity, &conn_profiles);

        // 阶段4：按固定 Tick 节流
        tokio::time::sleep(tick_interval).await;
    }
}