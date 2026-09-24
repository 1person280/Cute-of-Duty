//! Cute Of Duty 服务端（cod_server）
//!
//! 服务器权威架构落地：本进程持有**所有热数据**（世界模拟、元素状态、战局），
//! 以固定 Tick（60Hz，复用 `engine::GameLoop` 的确定性模拟）推进；每个 Tick
//! 消费客户端输入意图 → 计算 → 构建已按 AOI 过滤的权威快照 → 广播给各客户端。
//!
//! 客户端只发输入、只收快照；本进程是游戏世界的唯一真理源。

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::{info, warn};

use cute_of_duty_server::combat::{self, CombatIntent};
use cute_of_duty_server::config;
use cute_of_duty_server::element::{ElementSystem, EntityElementState};
use cute_of_duty_server::engine::{GameLoop, TickConfig};
use cute_of_duty_server::entity::EntityId;
use cute_of_duty_server::equipment::EquipmentSystem;
use cute_of_duty_server::net::protocol::{EventKind, PlayerInput, ServerMessage};
use cute_of_duty_server::net::{self, NetCommand, NetRuntime};

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

    let tick_config = TickConfig {
        tick_rate_hz: 60,
        max_frame_time_ms: 50.0,
        enable_determinism_check: true,
    };
    let mut sim = GameLoop::new(tick_config, element_system, equipment_system);
    sim.set_environment(EntityElementState::Normal);

    // 进场即生成训练场实弹靶（服务端权威：靶位来自地图数据，命中计分由 combat 结算）
    let training = cute_of_duty_server::map::training::layout();
    cute_of_duty_server::combat::range::spawn_range_targets(sim.world_mut(), &training.targets);
    info!("训练场已就绪：{} 个靶机进场", training.targets.len());

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

    // 服务端权威主循环
    serve_authoritative(&mut sim, &rt, &mut cmd_rx, tick_config.tick_rate_hz).await;

    Ok(())
}

/// 服务器权威主循环：60Hz 固定 Tick + 输入消费 + 快照广播。
///
/// 设计动机：长 TTK 游戏不必支持毫秒级瞬时同步，固定 Tick 的确定性权威循环
/// 足以承载 TCP 可靠同步；每 Tick 一次"算→广播"即实现「服务端算、客户端看」。
async fn serve_authoritative(
    sim: &mut GameLoop,
    rt: &Arc<NetRuntime>,
    cmd_rx: &mut mpsc::UnboundedReceiver<NetCommand>,
    tick_rate_hz: u32,
) {
    let tick_interval = std::time::Duration::from_micros(1_000_000 / u64::from(tick_rate_hz));
    let dt = 1.0 / tick_rate_hz as f32;

    // conn_id → 该客户端在权威世界的实体
    let mut conn_entity: HashMap<u64, EntityId> = HashMap::new();

    loop {
        // 阶段1：消费本帧所有客户端输入意图（非阻塞清空队列）
        drain_commands(sim, &rt, &mut conn_entity, cmd_rx);

        // 阶段2：推进确定性模拟一个 Tick
        sim.tick(dt);

        // 阶段2.5：转发战斗事件（击杀/命中）到对应连接，独立于快照各自按序下发
        forward_combat_events(rt, &conn_entity, &sim.drain_combat_events());

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

        // 阶段4：按固定 Tick 节流
        tokio::time::sleep(tick_interval).await;
    }
}

/// 消费本帧命令队列中的全部事件并应用到权威模拟。
fn drain_commands(
    sim: &mut GameLoop,
    rt: &Arc<NetRuntime>,
    conn_entity: &mut HashMap<u64, EntityId>,
    cmd_rx: &mut mpsc::UnboundedReceiver<NetCommand>,
) {
    while let Ok(cmd) = cmd_rx.try_recv() {
        match cmd {
            NetCommand::Connect { conn_id, name } => {
                // 为新连接在权威世界创建玩家实体（挂战斗状态），并回握手帧。
                // 出生点取训练场地图的 `player_spawn`，保证朝向射击馆（-Z）。
                let spawn = cute_of_duty_server::map::training::layout().player_spawn;
                let eid = combat::spawn_player(
                    sim.world_mut(),
                    cute_of_duty_server::damage::Vec3::new(spawn[0], spawn[1], spawn[2]),
                    0,
                );
                conn_entity.insert(conn_id, eid);
                info!("玩家「{name}」(#conn {conn_id}) 加入，权威实体 {eid:?}");
                rt.send_to(
                    conn_id,
                    ServerMessage::Handshake {
                        assigned_id: eid.as_u64(),
                        tick_rate_hz: 60,
                    },
                );
            }
            NetCommand::Input { conn_id, player } => {
                if let Some(&eid) = conn_entity.get(&conn_id) {
                    apply_input(sim, eid, &player);
                }
            }
            NetCommand::Disconnect { conn_id } => {
                if let Some(eid) = conn_entity.remove(&conn_id) {
                    sim.world_mut().despawn(eid);
                    info!("连接 #conn {conn_id} 断开，已回收权威实体 {eid:?}");
                }
            }
        }
    }
}

/// 把客户端输入意图应用到权威实体（位移 + 朝向 + 战斗意图）。
///
/// 服务器权威原则：最终位移由服务端按本节转速写回 `world`，再经快照回传，
/// 客户端不做任何本地校订 → 天生免疫坐标篡改。战斗（开火/换弹/技能）也在此
/// 经 `sim.apply_combat_input` 结算，命中/击杀由服务端定夺。
fn apply_input(sim: &mut GameLoop, eid: EntityId, input: &PlayerInput) {
    // 位移 + 朝向：先写回权威实体
    {
        let Some(entity) = sim.world_mut().get_entity_mut(eid) else { return };
        let mut dx = 0f32;
        let mut dz = 0f32;
        if input.move_forward { dz -= 1.0; }
        if input.move_backward { dz += 1.0; }
        if input.move_left { dx -= 1.0; }
        if input.move_right { dx += 1.0; }
        let step = entity.move_speed * 0.1f32; // 单个 Tick 的位移（60Hz）
        if dx != 0.0 || dz != 0.0 {
            let len = (dx * dx + dz * dz).sqrt();
            entity.position.x += dx / len * step;
            entity.position.z += dz / len * step;
        }
        // 刷新战斗朝向（供本帧射线 / 技能方向）
        if let Some(cb) = entity.get_component_mut::<cute_of_duty_server::combat::Combatant>() {
            cb.update_aim(input.aim_yaw, input.aim_pitch);
        }
    }

    // 战斗意图独立结算（与位移解耦）
    sim.apply_combat_input(
        eid,
        CombatIntent {
            shoot: input.shoot,
            reload: input.reload,
            skill_q: input.skill_q,
            skill_e: input.skill_e,
            yaw: input.aim_yaw,
            pitch: input.aim_pitch,
        },
    );
}

/// 把本帧战斗突发事件（击杀/命中）转发给受影响玩家的连接。
///
/// `conn_entity` 提供“连接 ↦ 实体”映射，由此反查“实体 ↦ 连接”。
/// AI 目标无连接时不下推（HUD 只关心玩家相关事件）。
fn forward_combat_events(
    rt: &Arc<NetRuntime>,
    conn_entity: &HashMap<u64, EntityId>,
    events: &[cute_of_duty_server::combat::CombatEvent],
) {
    if events.is_empty() {
        return;
    }
    // 反查：实体 ID → 连接
    let mut eid_to_conn = HashMap::new();
    for (&conn_id, &eid) in conn_entity.iter() {
        eid_to_conn.insert(eid, conn_id);
    }
    for ev in events {
        match ev {
            cute_of_duty_server::combat::CombatEvent::Kill { killer, victim } => {
                if let Some(&conn_id) = eid_to_conn.get(&EntityId::new(*killer)) {
                    rt.send_to(conn_id, ServerMessage::Event { kind: EventKind::Kill { killer_id: *killer, victim_id: *victim } });
                }
            }
            cute_of_duty_server::combat::CombatEvent::Hit { source, target, is_headshot } => {
                // 命中反馈同时给射手（为自己命中出彩、训练靶计分 HUD）
                // 与受击者（被击中掉血警示）。各查一次连接，未上线的一侧自动忽略。
                if let Some(&conn_id) = eid_to_conn.get(&EntityId::new(*source)) {
                    rt.send_to(conn_id, ServerMessage::Event { kind: EventKind::Hit { source_id: *source, target_id: *target, is_headshot: *is_headshot } });
                }
                if let Some(&conn_id) = eid_to_conn.get(&EntityId::new(*target)) {
                    rt.send_to(conn_id, ServerMessage::Event { kind: EventKind::Hit { source_id: *source, target_id: *target, is_headshot: *is_headshot } });
                }
            }
        }
    }
}