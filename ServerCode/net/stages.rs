//! 服务端权威主循环的阶段函数（从 `main.rs` 抽出的编排段）。
//!
//! 设计动机（Why）：`ServerCode/main.rs` 是唯一编排者，负责把各模块句柄接起来；但
//! 「消费网络命令 → 应用到权威世界 → 回执播报」这段阶段实现篇幅很大，堆在二进制入口里
//! 会让 `main.rs` 突破 600 行红线。故把**可整体外移的阶段逻辑**集中到 `net` 模块下
//! （`net` 已承担「服务端专属编排」，见 `net/mod.rs`）。本文件的函数只接受调用方持有的
//! 句柄与数据、不新增任何模块级状态，故整体行为与原先逐字一致。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::combat::{self, CombatIntent};
use crate::engine::GameLoop;
use crate::entity::EntityId;
use crate::equipment::EquipmentSystem;
use crate::inventory;
use crate::items::{self, Backpack, Container, TransferDir, TransferResult};
use crate::map::PickupKind;
use crate::motion;
use crate::net::protocol::{EventKind, InventoryAction, PlayerInput, ServerMessage};
use crate::net::runtime::{NetCommand, NetRuntime};
use crate::net::web::{PlayerSlot, WebPlayerInfo};
use crate::player::PlayerProfile;
use crate::storage::{ColdRepo, JsonLogRepo};

/// 撤离点世界坐标（草坪训练场北端撤离光垫中心，与 `map::lawn::extract_zone` 一致）。
const EXTRACTION_POINT: (f32, f32) = (0.0, -440.0);
/// 判定"进入撤离区"的触发半径（米，平面距离，忽略 Y）。撤离光垫半边长 3m，取 12m 留余量。
const EXTRACTION_RANGE: f32 = 12.0;

/// 消费本帧命令队列中的全部事件并应用到权威模拟。
///
/// 冷数据接入点（Why）：玩家档案属**冷数据**——连接期在 `conn_profiles` 持有热副本，
/// 断开时经 `repo` 落盘（append-only，至少一次持久）。热数据（实体/血量/CD）仍走
/// `sim.world`，绝不进仓库。
#[allow(clippy::too_many_arguments)]
pub fn drain_commands(
    sim: &mut GameLoop,
    rt: &Arc<NetRuntime>,
    conn_entity: &mut HashMap<u64, EntityId>,
    conn_profiles: &mut HashMap<u64, PlayerProfile>,
    conn_loadout: &mut HashMap<u64, Vec<String>>,
    conn_input: &mut HashMap<u64, PlayerInput>,
    conn_resident: &mut HashMap<u64, HashSet<u64>>,
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
                let spawn = crate::map::lawn::layout().player_spawn;
                let eid = combat::spawn_player(
                    sim.world_mut(),
                    crate::damage::Vec3::new(spawn[0], spawn[1], spawn[2]),
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
                // 模型目录：握手后一次性下发服务端权威的体素几何/动画（静态冷数据，
                // 经 OnceLock 内缓存，多连接复用同一份，不进每帧快照通道；经资源通道走 4096B 包）。
                // 同步登记"常驻资源集合"，供后续 PoolSync 淘汰上报对账。
                let models = crate::model::catalog();
                let animations = crate::model::animations();
                let mut resident = HashSet::new();
                for m in &models {
                    resident.insert(crate::net::packet::model_key(m));
                }
                for a in &animations {
                    resident.insert(crate::net::packet::animation_key(a));
                }
                conn_resident.insert(conn_id, resident);
                rt.send_to(conn_id, ServerMessage::ModelCatalog { models, animations });
                // 选装预设目录：服务端权威、静态冷数据，握手后一次性下发，供仓库浮层一键选用。
                rt.send_to(
                    conn_id,
                    ServerMessage::PresetCatalog { presets: crate::items::presets::all() },
                );
                // 世界目录：干员名册 + 活动地图布局（皆为整局不变的静态表），握手后一次性下发。
                // 客户端不再直读契约静态表，改由此灌入内存后渲染/展示（单一事实来源）。
                rt.send_to(
                    conn_id,
                    ServerMessage::WorldCatalog {
                        roster: crate::operator::roster().to_vec(),
                        layout: crate::map::lawn::layout(),
                    },
                );
            }
            NetCommand::Input { conn_id, player } => {
                // 连续量（移动/朝向）只记最新；边沿量（换弹/技能）在一个 Tick 内可能被更晚
                // 的同 Tick 输入覆盖，故按位「或」锁存，直到被某次 Tick 消费后清空（见阶段1.5），
                // 保证单次按键既不因乱序丢失、也不被重复触发。
                let slot = conn_input.entry(conn_id).or_default();
                let (pending_reload, pending_q, pending_e, pending_weapon, pending_use_slot, pending_cancel) =
                    (slot.reload, slot.skill_q, slot.skill_e, slot.weapon_slot, slot.use_slot, slot.grenade_cancel);
                *slot = player;
                slot.reload |= pending_reload;
                slot.skill_q |= pending_q;
                slot.skill_e |= pending_e;
                slot.grenade_cancel |= pending_cancel;
                // 切枪 / 速用格位 Intent 均为"最新优先"：本帧无请求时保留同 Tick 更早一次的请求
                if slot.weapon_slot.is_none() {
                    slot.weapon_slot = pending_weapon;
                }
                if slot.use_slot.is_none() {
                    slot.use_slot = pending_use_slot;
                }
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
                // 进图背包 = 玩家选装清单（**完全替换**语义）：有记录用记录、无记录即空背包，
                // 旧的开局固定配发不再作为进图来源。清单只在本局会话存放，跨局仍复用同一条。
                let Some(&eid) = conn_entity.get(&conn_id) else { continue };
                let carried = conn_loadout.get(&conn_id).cloned().unwrap_or_default();
                let placed = combat::apply_loadout(sim.world_mut(), eid, &carried);
                info!("玩家 #conn {conn_id} 进入训练场（携带 {placed} 件）");
                rt.send_to(
                    conn_id,
                    ServerMessage::Event {
                        kind: EventKind::Announce {
                            text: format!("已进入训练场 · 携带 {placed} 件 · 歼灭目标后到北端撤离"),
                        },
                    },
                );
            }
            NetCommand::ExtractRequest { conn_id } => {
                handle_extract(sim, rt, conn_entity, conn_id);
            }
            NetCommand::SwitchOperator { conn_id, operator_id } => {
                if let Some(&eid) = conn_entity.get(&conn_id) {
                    combat::switch_operator(sim.world_mut(), eid, operator_id);
                }
            }
            NetCommand::Interact { conn_id, target, choice } => {
                let Some(&eid) = conn_entity.get(&conn_id) else { continue };
                // 权威结算（距离校验 + 效果发放），完成后按需销毁被消耗的拾取物/物资箱。
                let (text, consumed) = crate::interact::settle(
                    sim.world_mut(),
                    eid,
                    EntityId::new(target),
                    choice,
                );
                if consumed {
                    sim.world_mut().despawn(EntityId::new(target));
                }
                rt.send_to(
                    conn_id,
                    ServerMessage::Event {
                        kind: EventKind::Announce { text },
                    },
                );
            }
            NetCommand::LootTransfer { conn_id, target, dir, index } => {
                let Some(&eid) = conn_entity.get(&conn_id) else { continue };
                // 权威裁决格位转移（距离校验 + 格位移动 + 即时效果），回执经 Event 播报。
                let text = handle_loot_transfer(sim, eid, EntityId::new(target), dir, index);
                rt.send_to(
                    conn_id,
                    ServerMessage::Event {
                        kind: EventKind::Announce { text },
                    },
                );
            }
            NetCommand::Ping { conn_id, seq } => {
                // 延迟探测：原样回显，客户端据此算往返延迟
                rt.send_to(conn_id, ServerMessage::Pong { seq });
            }
            NetCommand::PoolSync { conn_id, region, evicted } => {
                // 客户端对象池淘汰上报：从常驻集合移除，未来该键需要时再经资源通道重发。
                let count = evicted.len();
                if let Some(resident) = conn_resident.get_mut(&conn_id) {
                    for key in evicted {
                        resident.remove(&key);
                    }
                }
                info!("连接 #conn {conn_id} 对象池同步：区{region} 淘汰 {count} 键");
            }
            NetCommand::Disconnect { conn_id } => {
                conn_loadout.remove(&conn_id);
                conn_input.remove(&conn_id);
                conn_resident.remove(&conn_id);
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
pub fn apply_inventory_action(
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

/// 权威裁决一次物资箱逐格转移（`dir` 决定取出/放回），返回回执文本。
///
/// 服务端权威（Why）："这一格是什么、背包放不放得下、弹药入池还是武器换手"全部
/// 属于应该算的服务端职责——客户端只上报"哪一格、哪个方向"，本函数做距离校验、
/// 调 [`items::transfer`] 裁决格位，并对不占格的即时物品（弹药/武器）施加效果。
pub fn handle_loot_transfer(
    sim: &mut GameLoop,
    player: EntityId,
    target: EntityId,
    dir: TransferDir,
    index: usize,
) -> String {
    // 距离校验（与交互同口径：平面距离，忽略 Y）
    let (Some(p), Some(t)) = (sim.world().get_entity(player), sim.world().get_entity(target)) else {
        return "目标不存在".to_string();
    };
    let dx = t.position.x - p.position.x;
    let dz = t.position.z - p.position.z;
    if (dx * dx + dz * dz).sqrt() > crate::interact::INTERACT_RANGE {
        return "距离太远，无法取物".to_string();
    }

    // 两端同时可变：玩家背包 ↔ 物资箱容器（`with_pair_mut` 以安全手法做拆分借用）。
    let result = sim.world_mut().with_pair_mut(player, target, |pe, te| {
        match (pe.get_component_mut::<Backpack>(), te.get_component_mut::<Container>()) {
            (Some(bp), Some(ct)) => Some(items::transfer(bp, ct, dir, index)),
            _ => None,
        }
    });
    let Some(result) = result else {
        return "该目标不可取物".to_string();
    };

    match result {
        None => "该目标不是物资箱".to_string(),
        Some(TransferResult::Moved(text)) | Some(TransferResult::Rejected(text)) => text,
        Some(TransferResult::Apply(kind, text)) => {
            // 不占格物品：仅武器（工具）走"直接换手"；弹药已改为可堆叠背包物品，经 Moved 分支入格。
            if let PickupKind::Weapon { element } = kind {
                combat::equip_weapon(sim.world_mut(), player, element);
            }
            text
        }
    }
}

/// 撤离请求的权威判定：读取玩家在权威世界的坐标，与撤离点做平面距离校验。
///
/// 服务端权威（Why）：是否"站上撤离区"必须在服务端算，客户端只上报意图；
/// 距离校验通过才发 `ReturnToMenu`（客户端据此回主界面），否则仅 Announce 提示。
pub fn handle_extract(
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
    let b = digest.as_bytes();
    u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}

/// 供启动/加载日志展示货币钱包（软/硬通货、赛季代币）。
fn currency_summary(profile: &PlayerProfile) -> String {
    let w = &profile.inventory.currency;
    format!("货币(软{}硬{}季{})", w.soft_currency, w.hard_currency, w.season_tokens)
}

/// 把客户端输入意图应用到权威实体（位移 + 朝向 + 战斗意图），每 Tick 调用一次。
///
/// 服务器权威原则：最终位移由服务端写回 `world` 再经快照回传，客户端不做本地校订，
/// 天生免疫坐标篡改。位移数学（移速档 / WASD 分解 / 竖直积分）已抽到 `motion` 模块，
/// 本函数只做「取值 → 纯函数 → 写回 → 结算战斗」的编排；轴系与速度语义见
/// [`crate::motion::integrate_motion`]。
pub fn apply_input(sim: &mut GameLoop, eid: EntityId, input: &PlayerInput, dt: f32) {
    // 持雷 = 强制瞄准姿态（Why）：手雷"先瞄准后释放"要求持握期间移动压到 `AIM_MULT` 档，
    // 不能全速冲刺；故在算移速前把有效意图的 `aim` 置真（不改客户端上报的原始意图）。
    let holding = sim
        .world()
        .get_entity(eid)
        .and_then(|e| e.get_component::<crate::combat::HeldGrenade>())
        .map(|h| h.is_holding())
        .unwrap_or(false);
    let mut eff = *input;
    if holding {
        eff.aim = true;
    }
    // 作用域块结束 `entity` 的可变借用，之后才能把 `sim` 交给 `apply_combat_input`。
    {
        let Some(entity) = sim.world_mut().get_entity_mut(eid) else { return };
        let step = motion::integrate_motion(
            entity.move_speed, entity.position, entity.grounded, entity.vertical_velocity, &eff, dt,
        );
        // 位移 + 竖直状态：写回权威实体
        entity.position = step.position;
        entity.vertical_velocity = step.vertical_velocity;
        entity.grounded = step.grounded;
        // 刷新战斗朝向（供本帧射线 / 技能方向）
        if let Some(cb) = entity.get_component_mut::<crate::combat::Combatant>() {
            cb.update_aim(eff.aim_yaw, eff.aim_pitch);
        }
    }
    // 战斗意图独立结算（与位移解耦）
    sim.apply_combat_input(eid, CombatIntent {
        shoot: input.shoot,
        reload: input.reload,
        skill_q: input.skill_q,
        skill_e: input.skill_e,
        yaw: input.aim_yaw,
        pitch: input.aim_pitch,
        weapon_slot: input.weapon_slot,
        use_slot: input.use_slot,
        grenade_cancel: input.grenade_cancel,
    });
}

/// 把本帧战斗突发事件（击杀/命中）转发给受影响玩家的连接。
///
/// `conn_entity` 提供“连接 ↦ 实体”映射，由此反查“实体 ↦ 连接”。
/// AI 目标无连接时不下推（HUD 只关心玩家相关事件）。
pub fn forward_combat_events(
    rt: &Arc<NetRuntime>,
    conn_entity: &HashMap<u64, EntityId>,
    events: &[crate::combat::CombatEvent],
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
            crate::combat::CombatEvent::Kill { killer, victim } => {
                if let Some(&conn_id) = eid_to_conn.get(&EntityId::new(*killer)) {
                    rt.send_to(conn_id, ServerMessage::Event { kind: EventKind::Kill { killer_id: *killer, victim_id: *victim } });
                }
            }
            crate::combat::CombatEvent::Hit { source, target, is_headshot } => {
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

/// 刷新 web 侧玩家展示槽（姓名/等级来自本连接热档案；web 只读快照）。
///
/// 设计动机（Why）：门户/运维 API 展示的在线玩家列表不直接读领域数据，而由主循环把
/// 「连接 ↦ 热档案」映射成 [`WebPlayerInfo`] 后整体覆写进展示槽；web 线程只读该槽，
/// 从而对领域保持零耦合。
pub fn refresh_web_players(
    player_slot: &PlayerSlot,
    conn_entity: &HashMap<u64, EntityId>,
    conn_profiles: &HashMap<u64, PlayerProfile>,
) {
    if let Ok(mut slot) = player_slot.lock() {
        *slot = conn_entity
            .keys()
            .filter_map(|conn_id| {
                conn_profiles.get(conn_id).map(|p| WebPlayerInfo {
                    conn_id: *conn_id,
                    name: p.username.clone(),
                    level: p.level,
                })
            })
            .collect();
    }
}