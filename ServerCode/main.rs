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
use cute_of_duty_server::inventory;
use cute_of_duty_server::net::protocol::{EventKind, InventoryAction, PlayerInput, ServerMessage};
use cute_of_duty_server::net::{self, NetCommand, NetRuntime};
use cute_of_duty_server::player::PlayerProfile;
use cute_of_duty_server::storage::{ColdRepo, JsonLogRepo};

/// 服务端默认监听地址（本机回环；正式环境改为对外网卡并置于反向代理后）。
const DEFAULT_ADDR: &str = "127.0.0.1:8888";

/// 撤离点世界坐标（草坪训练场北端撤离光垫中心，与 `map::lawn::extract_zone` 一致）。
const EXTRACTION_POINT: (f32, f32) = (0.0, -440.0);
/// 判定"进入撤离区"的触发半径（米，平面距离，忽略 Y）。撤离光垫半边长 3m，取 12m 留余量。
const EXTRACTION_RANGE: f32 = 12.0;

/// 疾跑相对基础移速的倍率（`Entity.move_speed × 此值`）。1km 大场南北纵深 910m，
/// 无疾跑靠基础步速横穿耗时过久，故给一个明确的冲刺档（沿 0.3.2 手感量级）。
const SPRINT_MULT: f32 = 1.6;

/// 越肩瞄准时的移速倍率（`速度 × 此值`）。瞄准是"用机动性换精度"的博弈位：
/// 按住右键即从常态 5 m/s 降到 2.75 m/s，与旧版手感一致。
const AIM_MULT: f32 = 0.55;

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

    // 冷数据仓库（玩家档案/背包/货币/战绩——跨局需保存、重连要续回的数据）
    // 配置驱动目录：环境变量 COD_DATA_DIR > workspace 根 ServerCode/data/profiles。
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
    )
    .await;

    Ok(())
}

/// 打印仓库根目录（供启动日志；仅用于展示）。
fn repo_dir_display() -> String {
    cute_of_duty_server::storage::resolve_data_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "<未解析，回退 data/profiles>".to_string())
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
    repo: &mut JsonLogRepo,
    equipment: &Arc<EquipmentSystem>,
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
    // 全局装备 ID 分配器（跨连接唯一，供背包 Craft 裁决）
    let mut next_equipment_id: u64 = 1000;

    loop {
        // 阶段1：消费本帧所有客户端输入意图（非阻塞清空队列；Input 只存最新不立即结算）
        drain_commands(
            sim,
            rt,
            &mut conn_entity,
            &mut conn_profiles,
            &mut conn_loadout,
            &mut conn_input,
            repo,
            equipment,
            &mut next_equipment_id,
            cmd_rx,
        );

        // 阶段1.5：按最新意图推进所有玩家（位移/朝向/战斗），每 Tick 恰好一次。
        // 与 dt 配套构成确定性结算：速度单位 m/s，位移 = 速度 × dt。
        for (&conn_id, &eid) in conn_entity.iter() {
            if let Some(input) = conn_input.get(&conn_id) {
                apply_input(sim, eid, input, dt);
            }
        }

        // 阶段1.6：清空已消费的边沿量（换弹/技能），避免同一次按键在后续 Tick 被重复触发。
        // 扳机（shoot）是持续量，按住即持续开火，故不在此清空。
        for input in conn_input.values_mut() {
            input.reload = false;
            input.skill_q = false;
            input.skill_e = false;
        }

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
///
/// 冷数据接入点（Why）：玩家档案属**冷数据**——连接期在 `conn_profiles` 持有热副本，
/// 断开时经 `repo` 落盘（append-only，至少一次持久）。热数据（实体/血量/CD）仍走
/// `sim.world`，绝不进仓库。
#[allow(clippy::too_many_arguments)]
fn drain_commands(
    sim: &mut GameLoop,
    rt: &Arc<NetRuntime>,
    conn_entity: &mut HashMap<u64, EntityId>,
    conn_profiles: &mut HashMap<u64, PlayerProfile>,
    conn_loadout: &mut HashMap<u64, Vec<String>>,
    conn_input: &mut HashMap<u64, PlayerInput>,
    repo: &mut JsonLogRepo,
    equipment: &Arc<EquipmentSystem>,
    next_equipment_id: &mut u64,
    cmd_rx: &mut mpsc::UnboundedReceiver<NetCommand>,
) {
    while let Ok(cmd) = cmd_rx.try_recv() {
        match cmd {
            NetCommand::Connect { conn_id, name } => {
                // 冷数据：按玩家名解析档案 ID 并加载（无档则新建默认），
                // 作为本连接的热副本驻内存；失败仅告警不阻断进场。
                let player_id = player_id_of(&name);
                match repo.load(player_id) {
                    Ok(Some(profile)) => {
                        info!(
                            "玩家「{name}」#{} 已加载存档：等级{} 背包{}/{}{}",
                            player_id,
                            profile.level,
                            profile.inventory.used_slots,
                            profile.inventory.max_slots,
                            currency_summary(&profile),
                        );
                        conn_profiles.insert(conn_id, profile);
                    }
                    Ok(None) => {
                        let fresh = PlayerProfile::new(player_id, &name);
                        info!("玩家「{name}」#{} 首登，新建默认档案", player_id);
                        conn_profiles.insert(conn_id, fresh);
                    }
                    Err(e) => {
                        warn!("玩家「{name}」#{} 存档读取失败（续用默认）：{e}", player_id);
                        conn_profiles.insert(conn_id, PlayerProfile::new(player_id, &name));
                    }
                }

                // 热数据：为连接在权威世界创建玩家实体（挂战斗状态），并回握手帧。
                // 出生点取活动地图（草坪训练场）的 `player_spawn`（南端 z=470，面向 -Z 北）。
                let spawn = cute_of_duty_server::map::lawn::layout().player_spawn;
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
                // 连续量（移动/朝向）只记最新；边沿量（换弹/技能）在一个 Tick 内可能被更晚
                // 的同 Tick 输入覆盖，故按位「或」锁存，直到被某次 Tick 消费后清空（见阶段1.5），
                // 保证单次按键既不因乱序丢失、也不被重复触发。
                let slot = conn_input.entry(conn_id).or_default();
                let (pending_reload, pending_q, pending_e) =
                    (slot.reload, slot.skill_q, slot.skill_e);
                *slot = player;
                slot.reload |= pending_reload;
                slot.skill_q |= pending_q;
                slot.skill_e |= pending_e;
            }
            NetCommand::Inventory { conn_id, action } => {
                apply_inventory_action(
                    rt,
                    conn_profiles,
                    equipment,
                    next_equipment_id,
                    conn_id,
                    action,
                );
            }
            NetCommand::Loadout { conn_id, carried } => {
                // 选装为本局会话热副本：只存不落冷库（冷数据防腐只收档案/背包/货币）。
                conn_loadout.insert(conn_id, carried.clone());
                info!("玩家 #conn {conn_id} 选装确认：{} 件", carried.len());
                rt.send_to(
                    conn_id,
                    ServerMessage::Event {
                        kind: EventKind::Announce {
                            text: format!("携带已确认：{} 件", carried.len()),
                        },
                    },
                );
            }
            NetCommand::StartTraining { conn_id } => {
                // 玩家实体在 Connect 已出生；这里仅需权威确认与状态播报。
                info!("玩家 #conn {conn_id} 进入训练场");
                rt.send_to(
                    conn_id,
                    ServerMessage::Event {
                        kind: EventKind::Announce {
                            text: "已进入训练场 · 歼灭目标后到北端撤离".to_string(),
                        },
                    },
                );
            }
            NetCommand::ExtractRequest { conn_id } => {
                handle_extract(sim, rt, &conn_entity, conn_id);
            }
            NetCommand::SwitchOperator { conn_id, operator_id } => {
                if let Some(&eid) = conn_entity.get(&conn_id) {
                    combat::switch_operator(sim.world_mut(), eid, operator_id);
                }
            }
            NetCommand::Ping { conn_id, seq } => {
                // 延迟探测：原样回显，客户端据此算往返延迟
                rt.send_to(conn_id, ServerMessage::Pong { seq });
            }
            NetCommand::Disconnect { conn_id } => {
                conn_loadout.remove(&conn_id);
                conn_input.remove(&conn_id);
                // 冷数据：先落盘档案（至少一次持久），再回收权威实体。
                if let Some(profile) = conn_profiles.remove(&conn_id) {
                    if let Err(e) = repo.save(profile.player_id, &profile) {
                        warn!("玩家{}档案件落盘失败：{e}", profile.player_id);
                    } else {
                        info!("玩家{}档案已落盘", profile.player_id);
                    }
                }
                if let Some(eid) = conn_entity.remove(&conn_id) {
                    sim.world_mut().despawn(eid);
                    info!("连接 #conn {conn_id} 断开，已回收权威实体 {eid:?}");
                }
            }
        }
    }
}

/// 结算背包 CRUD 意图并回执播报。
///
/// 服务端权威（Why）：元素/上限/余额全部在域服务内裁决；同一份 `conn_profiles`
/// 热副本被修改，正是随后 Disconnect 落盘到冷仓库的数据，保证"读到的就是会存下的"。
/// 结果统一以 `Event::Announce` 文本回执（不改线格式，客户端契约稳定）。
fn apply_inventory_action(
    rt: &Arc<NetRuntime>,
    conn_profiles: &mut HashMap<u64, PlayerProfile>,
    equipment: &Arc<EquipmentSystem>,
    next_equipment_id: &mut u64,
    conn_id: u64,
    action: InventoryAction,
) {
    let Some(profile) = conn_profiles.get_mut(&conn_id) else {
        return; // 尚未完成握手（无热副本档案）时忽略该意图
    };

    let reply = match action {
        InventoryAction::Craft {
            name,
            eq_type,
            tier,
            base_value,
            specified_element,
        } => {
            let req = inventory::CraftRequest {
                name,
                eq_type,
                tier,
                base_value,
                specified_element,
            };
            match inventory::craft(equipment, profile, next_equipment_id, req) {
                Ok(eq) => format!(
                    "锻造成功：{}(t{}/{:?})元素{:?}",
                    eq.name,
                    eq.tier.as_u8(),
                    eq.eq_type,
                    eq.element,
                ),
                Err(e) => format!("锻造失败：{e}"),
            }
        }
        InventoryAction::Discard { index } => {
            match inventory::discard_by_index(equipment, profile, index) {
                Ok(()) => format!("已丢弃背包第 {index} 件"),
                Err(e) => format!("丢弃失败：{e}"),
            }
        }
        InventoryAction::AdjustCurrency {
            soft,
            hard,
            season,
        } => match inventory::adjust_currency(profile, soft, hard, season) {
            Ok(()) => format!("货币已更新：软{soft:+} 硬{hard:+} 季{season:+}"),
            Err(e) => format!("货币操作失败：{e}"),
        },
    };

    rt.send_to(
        conn_id,
        ServerMessage::Event {
            kind: EventKind::Announce { text: reply },
        },
    );
}

/// 撤离请求的权威判定：读取玩家在权威世界的坐标，与撤离点做平面距离校验。
///
/// 服务端权威（Why）：是否"站上撤离区"必须在服务端算，客户端只上报意图；
/// 距离校验通过才发 `ReturnToMenu`（客户端据此回主界面），否则仅 Announce 提示。
fn handle_extract(
    sim: &mut GameLoop,
    rt: &Arc<NetRuntime>,
    conn_entity: &HashMap<u64, EntityId>,
    conn_id: u64,
) {
    let Some(&eid) = conn_entity.get(&conn_id) else {
        return;
    };
    let Some(entity) = sim.world().get_entity(eid) else {
        return;
    };
    // 平面距离（忽略 Y），与撤退判定口径一致
    let dp = entity.position;
    let dx = dp.x - EXTRACTION_POINT.0;
    let dz = dp.z - EXTRACTION_POINT.1;
    let in_zone = (dx * dx + dz * dz).sqrt() <= EXTRACTION_RANGE;

    if in_zone {
        info!("玩家 #conn {conn_id} 撤离成功");
        rt.send_to(conn_id, ServerMessage::ReturnToMenu);
        rt.send_to(
            conn_id,
            ServerMessage::Event {
                kind: EventKind::Announce {
                    text: "撤离成功 · 已返回主界面".to_string(),
                },
            },
        );
    } else {
        rt.send_to(
            conn_id,
            ServerMessage::Event {
                kind: EventKind::Announce {
                    text: "未到达撤离区".to_string(),
                },
            },
        );
    }
}

/// 按玩家名得到稳定的档案 ID（Why：连接协议只带 `profile` 名，档名作为身份；
/// 用 blake3 哈希出稳定 u64 作为 `PlayerProfile.player_id` 与冷数据仓库键）。
fn player_id_of(name: &str) -> u64 {
    let digest = blake3::hash(name.as_bytes());
    u64::from_le_bytes(digest.as_bytes()[0..8].try_into().unwrap())
}

/// 供启动/加载日志展示货币钱包（软/硬通货、赛季代币）。
fn currency_summary(profile: &PlayerProfile) -> String {
    let w = &profile.inventory.currency;
    format!("货币(软{}硬{}季{})", w.soft_currency, w.hard_currency, w.season_tokens)
}

/// 把客户端输入意图应用到权威实体（位移 + 朝向 + 战斗意图），每 Tick 调用一次。
///
/// 服务器权威原则：最终位移由服务端按本节转速写回 `world`，再经快照回传，
/// 客户端不做任何本地校订 → 天生免疫坐标篡改。战斗（开火/换弹/技能）也在此
/// 经 `sim.apply_combat_input` 结算，命中/击杀由服务端定夺。
///
/// 轴系（与客户端 `AimRig` 同源）：视线水平分量 `(sinY, cosY)`，右向量
/// `(-cosY, sinY)`；`W/S` 沿视线前后、`D/A` 沿右手左右。位移量 = `速度 × dt`
/// （`dt` 为固定 Tick 步长），因此速度单位是 m/s、与消息频率无关。
fn apply_input(sim: &mut GameLoop, eid: EntityId, input: &PlayerInput, dt: f32) {
    let yaw = input.aim_yaw;
    let (fx, fz) = (yaw.sin(), yaw.cos()); // 视线水平方向
    let (rx, rz) = (-yaw.cos(), yaw.sin()); // 右手方向 = forward × Y

    let mut mx = 0f32;
    let mut mz = 0f32;
    if input.move_forward { mx += fx; mz += fz; }
    if input.move_backward { mx -= fx; mz -= fz; }
    if input.move_right { mx += rx; mz += rz; }
    if input.move_left { mx -= rx; mz -= rz; }

    // 位移 + 朝向：先写回权威实体
    {
        let Some(entity) = sim.world_mut().get_entity_mut(eid) else { return };
        let len = (mx * mx + mz * mz).sqrt();
        if len > 1e-6 {
            let mut speed = if input.sprint { entity.move_speed * SPRINT_MULT } else { entity.move_speed };
            // 瞄准优先于疾跑压制移速：按住右键即进入"慢走精度档"（见 `AIM_MULT`）。
            if input.aim {
                speed *= AIM_MULT;
            }
            let step = speed * dt;
            entity.position.x += mx / len * step;
            entity.position.z += mz / len * step;
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