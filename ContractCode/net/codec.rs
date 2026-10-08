//! 指令单元二进制编解码（线上语义层）
//!
//! 设计动机（Why）：0.12 主通道把"一条消息"压缩成 32B 单元——热路径（每帧上行意图）
//! 必须一次装进单单元，故 [`PlayerInput`] 走**位打包**（15 个 bool 压 2B）。定长小指令
//! 全部走"1B opcode + 紧凑参数"；**含字符串/可变长**的控制消息（档案名、选装清单、
//! 背包 CRUD）压不进 32B，降级为 [`DataKind::Control`] 数据流（仍属"指令类"，享优先级）。
//!
//! 本模块只做"消息 ↔ 单元/字节流"，不含裁决；资源（`ModelCatalog`）不走此处，走资源通道。

use crate::interact::{InteractChoice, SupplyKind};
use crate::items::TransferDir;
use crate::net::packet::{to_json, DataKind, Unit, UNIT_BYTES};
use crate::net::protocol::{ClientMessage, PlayerInput, ServerMessage};

// —— 客户端指令 opcode ——
const C_INPUT: u8 = 0x01;
const C_PING: u8 = 0x02;
const C_START_TRAINING: u8 = 0x03;
const C_EXTRACT: u8 = 0x04;
const C_DISCONNECT: u8 = 0x05;
const C_SWITCH_OPERATOR: u8 = 0x06;
const C_INTERACT: u8 = 0x07;
const C_LOOT_TRANSFER: u8 = 0x08;
const C_POOL_SYNC: u8 = 0x09;

// —— 服务端指令 opcode ——
const S_HANDSHAKE: u8 = 0x81;
const S_PONG: u8 = 0x82;
const S_RETURN_TO_MENU: u8 = 0x83;

/// 单条指令单元可携带的对象池键数（(32-4)/8 = 3）。
pub const POOL_SYNC_KEYS_PER_UNIT: usize = 3;

/// 一条消息在线上的形态。
#[derive(Debug, Clone, PartialEq)]
pub enum WireMessage {
    /// 指令：1..N 个**自包含** 32B 单元（一包最多 7 个）。
    Command(Vec<Unit>),
    /// 数据：一条消息的字节流（JSON），按主通道负载区（224B）切片跨包发送。
    Data(DataKind, Vec<u8>),
}

/// 连接角色（与 `packet::Role` 数值一致，供绑定包负载编解码）。
pub const ROLE_CONTROL: u8 = 0;
pub const ROLE_RESOURCE: u8 = 1;

/// 把一条**客户端上行消息**编为线上形态。
pub fn encode_client(msg: &ClientMessage) -> Result<WireMessage, String> {
    let unit = match msg {
        ClientMessage::Input { player } => pack_input(player),
        ClientMessage::Ping { seq } => {
            let mut u = [0u8; UNIT_BYTES];
            u[0] = C_PING;
            u[1..9].copy_from_slice(&seq.to_le_bytes());
            u
        }
        ClientMessage::StartTraining => one(C_START_TRAINING),
        ClientMessage::ExtractRequest => one(C_EXTRACT),
        ClientMessage::Disconnect => one(C_DISCONNECT),
        ClientMessage::SwitchOperator { operator_id } => {
            let mut u = [0u8; UNIT_BYTES];
            u[0] = C_SWITCH_OPERATOR;
            u[1..5].copy_from_slice(&operator_id.to_le_bytes());
            u
        }
        ClientMessage::Interact { target, choice } => {
            let mut u = [0u8; UNIT_BYTES];
            u[0] = C_INTERACT;
            u[1..9].copy_from_slice(&target.to_le_bytes());
            let (tag, supply) = match choice {
                InteractChoice::Take => (0u8, 0u8),
                InteractChoice::OpenCrate => (2u8, 0u8),
                InteractChoice::Supply { kind } => (1u8, supply_code(*kind)),
            };
            u[9] = tag;
            u[10] = supply;
            u
        }
        ClientMessage::LootTransfer { target, dir, index } => {
            let mut u = [0u8; UNIT_BYTES];
            u[0] = C_LOOT_TRANSFER;
            u[1..9].copy_from_slice(&target.to_le_bytes());
            u[9] = match dir {
                TransferDir::Take => 0,
                TransferDir::Put => 1,
            };
            u[10] = *index as u8;
            u
        }
        ClientMessage::PoolSync { region, evicted } => {
            return Ok(WireMessage::Command(pack_pool_sync(*region, evicted)));
        }
        // 含字符串/可变长：压不进 32B，降级为控制类数据流。
        ClientMessage::Connect { .. } | ClientMessage::Loadout { .. } | ClientMessage::Inventory { .. } => {
            return Ok(WireMessage::Data(DataKind::Control, to_json(msg)?));
        }
    };
    Ok(WireMessage::Command(vec![unit]))
}

/// 把一条**服务端下行消息**编为线上形态。
///
/// `ModelCatalog` 归资源通道（`resource_stream`），不经此处；误入返回错误。
pub fn encode_server(msg: &ServerMessage) -> Result<WireMessage, String> {
    match msg {
        ServerMessage::Handshake { assigned_id, tick_rate_hz } => {
            let mut u = [0u8; UNIT_BYTES];
            u[0] = S_HANDSHAKE;
            u[1..9].copy_from_slice(&assigned_id.to_le_bytes());
            u[9..13].copy_from_slice(&tick_rate_hz.to_le_bytes());
            Ok(WireMessage::Command(vec![u]))
        }
        ServerMessage::Pong { seq } => {
            let mut u = [0u8; UNIT_BYTES];
            u[0] = S_PONG;
            u[1..9].copy_from_slice(&seq.to_le_bytes());
            Ok(WireMessage::Command(vec![u]))
        }
        ServerMessage::ReturnToMenu => Ok(WireMessage::Command(vec![one(S_RETURN_TO_MENU)])),
        ServerMessage::Snapshot { .. } => Ok(WireMessage::Data(DataKind::Snapshot, to_json(msg)?)),
        ServerMessage::Event { .. } => Ok(WireMessage::Data(DataKind::Event, to_json(msg)?)),
        // 预设目录含字符串、压不进定长单元，与其它变长控制消息同路（控制类数据流）。
        ServerMessage::PresetCatalog { .. } => {
            Ok(WireMessage::Data(DataKind::Control, to_json(msg)?))
        }
        // 世界目录（名册 + 地图布局）含字符串/可变长，同样降级为控制类数据流。
        ServerMessage::WorldCatalog { .. } => {
            Ok(WireMessage::Data(DataKind::Control, to_json(msg)?))
        }
        ServerMessage::ModelCatalog { .. } => {
            Err("模型目录应走资源通道（resource_stream），不应进控制通道".to_string())
        }
    }
}

/// 解码一个**客户端指令单元**。
pub fn decode_client_unit(unit: &Unit) -> Result<ClientMessage, String> {
    match unit[0] {
        C_INPUT => Ok(ClientMessage::Input { player: unpack_input(unit) }),
        C_PING => Ok(ClientMessage::Ping { seq: read_u64(unit, 1) }),
        C_START_TRAINING => Ok(ClientMessage::StartTraining),
        C_EXTRACT => Ok(ClientMessage::ExtractRequest),
        C_DISCONNECT => Ok(ClientMessage::Disconnect),
        C_SWITCH_OPERATOR => Ok(ClientMessage::SwitchOperator { operator_id: read_u32(unit, 1) }),
        C_INTERACT => Ok(ClientMessage::Interact {
            target: read_u64(unit, 1),
            choice: match unit[9] {
                0 => InteractChoice::Take,
                1 => InteractChoice::Supply { kind: supply_from_code(unit[10])? },
                2 => InteractChoice::OpenCrate,
                other => return Err(format!("未知交互选择: {other}")),
            },
        }),
        C_LOOT_TRANSFER => Ok(ClientMessage::LootTransfer {
            target: read_u64(unit, 1),
            dir: match unit[9] {
                0 => TransferDir::Take,
                1 => TransferDir::Put,
                other => return Err(format!("未知转移方向: {other}")),
            },
            index: unit[10] as usize,
        }),
        C_POOL_SYNC => Err("对象池同步单元应成组解码（见 decode_pool_sync_units）".to_string()),
        other => Err(format!("未知客户端指令 opcode: 0x{other:02X}")),
    }
}

/// 解码一个**服务端指令单元**。
pub fn decode_server_unit(unit: &Unit) -> Result<ServerMessage, String> {
    match unit[0] {
        S_HANDSHAKE => Ok(ServerMessage::Handshake {
            assigned_id: read_u64(unit, 1),
            tick_rate_hz: read_u32(unit, 9),
        }),
        S_PONG => Ok(ServerMessage::Pong { seq: read_u64(unit, 1) }),
        S_RETURN_TO_MENU => Ok(ServerMessage::ReturnToMenu),
        other => Err(format!("未知服务端指令 opcode: 0x{other:02X}")),
    }
}

/// 解码一包内的**全部**客户端指令单元（第 `i` 个单元为 `i*32..`）。
pub fn decode_client_units(units: &[Unit]) -> Result<Vec<ClientMessage>, String> {
    // 对象池同步可能占多个连续单元：遇到首单元时整体收拢。
    let mut out = Vec::new();
    let mut i = 0;
    while i < units.len() {
        if units[i][0] == C_POOL_SYNC {
            // 一条淘汰清单可能横跨多个连续单元：把它们整体收拢为一条消息。
            let region = units[i][1];
            let mut keys = Vec::new();
            while i < units.len() && units[i][0] == C_POOL_SYNC {
                let count = (units[i][2] as usize).min(POOL_SYNC_KEYS_PER_UNIT);
                for k in 0..count {
                    keys.push(read_u64(&units[i], 3 + k * 8));
                }
                i += 1;
            }
            out.push(ClientMessage::PoolSync { region, evicted: keys });
        } else {
            out.push(decode_client_unit(&units[i])?);
            i += 1;
        }
    }
    Ok(out)
}

/// 解码一包内的**全部**服务端指令单元（与 [`decode_client_units`] 对称）。
pub fn decode_server_units(units: &[Unit]) -> Result<Vec<ServerMessage>, String> {
    units.iter().map(decode_server_unit).collect()
}

/// 把指令包的负载区（`unit_count × 32B` 连续字节）拆成定长单元。
///
/// 设计动机（Why）：`PacketReader` 只交回扁平负载字节，而编解码入口要吃 `&[Unit]`；
/// 本函数是二者之间唯一的桥，集中校验 `unit_count` 与长度，避免各调用点各写一遍。
pub fn units_from_payload(unit_count: u8, payload: &[u8]) -> Result<Vec<Unit>, String> {
    let n = unit_count as usize;
    if n == 0 {
        return Err("指令包 unit_count 不应为 0".to_string());
    }
    if n > crate::net::packet::UNITS_PER_PACKET {
        return Err(format!("unit_count 超单包上限: {n}"));
    }
    if payload.len() < n * UNIT_BYTES {
        return Err(format!("指令包负载不足: {} < {}", payload.len(), n * UNIT_BYTES));
    }
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let mut unit = [0u8; UNIT_BYTES];
        unit.copy_from_slice(&payload[i * UNIT_BYTES..(i + 1) * UNIT_BYTES]);
        out.push(unit);
    }
    Ok(out)
}

/// 把若干定长单元拼成指令包负载区（供调度器/测试复用）。
pub fn units_to_payload(units: &[Unit]) -> Vec<u8> {
    let mut out = Vec::with_capacity(units.len() * UNIT_BYTES);
    for unit in units {
        out.extend_from_slice(unit);
    }
    out
}

/// 解码一条客户端数据流（控制类，JSON）。
pub fn decode_client_data(_kind: DataKind, bytes: &[u8]) -> Result<ClientMessage, String> {
    serde_json::from_slice(bytes).map_err(|e| format!("解析客户端数据失败: {e}"))
}

/// 解码一条服务端数据流（快照/事件/控制类，JSON）。
pub fn decode_server_data(_kind: DataKind, bytes: &[u8]) -> Result<ServerMessage, String> {
    serde_json::from_slice(bytes).map_err(|e| format!("解析服务端数据失败: {e}"))
}

/// 位打包 [`PlayerInput`]（opcode + seq8 + 掩码2 + 俯仰4 + 偏航4 + 轮盘1 + 两槽各1 = 22B ≤ 32）。
fn pack_input(p: &PlayerInput) -> Unit {
    let mut u = [0u8; UNIT_BYTES];
    u[0] = C_INPUT;
    u[1..9].copy_from_slice(&p.seq.to_le_bytes());
    let bools = [
        p.move_forward, p.move_backward, p.move_left, p.move_right, p.jump, p.sprint, p.aim,
        p.shoot, p.reload, p.interact, p.inventory, p.use_item, p.skill_q, p.skill_e,
        p.grenade_cancel,
    ];
    let mut mask = 0u16;
    for (i, b) in bools.iter().enumerate() {
        if *b {
            mask |= 1 << i;
        }
    }
    u[9..11].copy_from_slice(&mask.to_le_bytes());
    u[11..15].copy_from_slice(&p.aim_pitch.to_le_bytes());
    u[15..19].copy_from_slice(&p.aim_yaw.to_le_bytes());
    u[19] = p.wheel_pick as u8;
    u[20] = p.weapon_slot.unwrap_or(0xFF);
    u[21] = p.use_slot.unwrap_or(0xFF);
    u
}

/// 还原位打包的 [`PlayerInput`]。
fn unpack_input(u: &Unit) -> PlayerInput {
    let mask = u16::from_le_bytes([u[9], u[10]]);
    let bit = |i: usize| mask & (1 << i) != 0;
    let opt = |b: u8| if b == 0xFF { None } else { Some(b) };
    PlayerInput {
        seq: read_u64(u, 1),
        move_forward: bit(0),
        move_backward: bit(1),
        move_left: bit(2),
        move_right: bit(3),
        jump: bit(4),
        sprint: bit(5),
        aim: bit(6),
        shoot: bit(7),
        reload: bit(8),
        interact: bit(9),
        inventory: bit(10),
        use_item: bit(11),
        skill_q: bit(12),
        skill_e: bit(13),
        grenade_cancel: bit(14),
        aim_pitch: read_f32(u, 11),
        aim_yaw: read_f32(u, 15),
        wheel_pick: u[19] as i8,
        weapon_slot: opt(u[20]),
        use_slot: opt(u[21]),
    }
}

/// 把对象池淘汰清单切成若干 32B 单元（每单元最多 [`POOL_SYNC_KEYS_PER_UNIT`] 个键）。
fn pack_pool_sync(region: u8, evicted: &[u64]) -> Vec<Unit> {
    if evicted.is_empty() {
        let mut u = [0u8; UNIT_BYTES];
        u[0] = C_POOL_SYNC;
        u[1] = region;
        return vec![u];
    }
    evicted
        .chunks(POOL_SYNC_KEYS_PER_UNIT)
        .map(|keys| {
            let mut u = [0u8; UNIT_BYTES];
            u[0] = C_POOL_SYNC;
            u[1] = region;
            u[2] = keys.len() as u8;
            for (k, key) in keys.iter().enumerate() {
                u[3 + k * 8..11 + k * 8].copy_from_slice(&key.to_le_bytes());
            }
            u
        })
        .collect()
}

fn one(op: u8) -> Unit {
    let mut u = [0u8; UNIT_BYTES];
    u[0] = op;
    u
}

fn supply_code(kind: SupplyKind) -> u8 {
    match kind {
        SupplyKind::Ammo => 0,
        SupplyKind::Health => 1,
        SupplyKind::Armor => 2,
    }
}

fn supply_from_code(v: u8) -> Result<SupplyKind, String> {
    match v {
        0 => Ok(SupplyKind::Ammo),
        1 => Ok(SupplyKind::Health),
        2 => Ok(SupplyKind::Armor),
        other => Err(format!("未知补给予类: {other}")),
    }
}

fn read_u64(u: &Unit, off: usize) -> u64 {
    u64::from_le_bytes([u[off], u[off + 1], u[off + 2], u[off + 3], u[off + 4], u[off + 5], u[off + 6], u[off + 7]])
}

fn read_u32(u: &Unit, off: usize) -> u32 {
    u32::from_le_bytes([u[off], u[off + 1], u[off + 2], u[off + 3]])
}

fn read_f32(u: &Unit, off: usize) -> f32 {
    f32::from_le_bytes([u[off], u[off + 1], u[off + 2], u[off + 3]])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 热路径：`PlayerInput` 位打包后仍 ≤32B，且往返逐字段一致（含 None 槽）。
    #[test]
    fn input_bitpacks_into_one_unit_and_roundtrips() {
        let p = PlayerInput {
            seq: 12345,
            move_forward: true,
            move_backward: false,
            move_left: true,
            move_right: false,
            jump: true,
            sprint: false,
            aim: true,
            shoot: true,
            reload: false,
            interact: false,
            inventory: false,
            use_item: true,
            wheel_pick: -1,
            skill_q: true,
            skill_e: false,
            weapon_slot: Some(1),
            use_slot: None,
            aim_pitch: -0.25,
            aim_yaw: 1.75,
            grenade_cancel: false,
        };
        let WireMessage::Command(units) = encode_client(&ClientMessage::Input { player: p }).unwrap() else {
            panic!("输入应为指令单元");
        };
        assert_eq!(units.len(), 1, "输入应压进单单元");
        let back = decode_client_unit(&units[0]).unwrap();
        assert_eq!(back, ClientMessage::Input { player: p });
    }

    /// 交互/转移/切干员等定长指令往返一致。
    #[test]
    fn fixed_commands_roundtrip() {
        let cases = vec![
            ClientMessage::Ping { seq: 99 },
            ClientMessage::StartTraining,
            ClientMessage::ExtractRequest,
            ClientMessage::Disconnect,
            ClientMessage::SwitchOperator { operator_id: 3 },
            ClientMessage::Interact { target: 77, choice: InteractChoice::Take },
            ClientMessage::Interact { target: 78, choice: InteractChoice::Supply { kind: SupplyKind::Armor } },
            ClientMessage::Interact { target: 79, choice: InteractChoice::OpenCrate },
            ClientMessage::LootTransfer { target: 80, dir: TransferDir::Put, index: 5 },
        ];
        for msg in cases {
            let WireMessage::Command(units) = encode_client(&msg).unwrap() else {
                panic!("应为指令单元: {msg:?}");
            };
            assert_eq!(decode_client_unit(&units[0]).unwrap(), msg);
        }
    }

    /// 含字符串的控制消息降级为控制类数据流，且 JSON 可往返。
    #[test]
    fn string_commands_become_control_data() {
        let msg = ClientMessage::Loadout { carried: vec!["医疗包".into(), "弹药".into()] };
        let WireMessage::Data(kind, bytes) = encode_client(&msg).unwrap() else {
            panic!("超长控制应降级为数据流");
        };
        assert_eq!(kind, DataKind::Control);
        assert_eq!(decode_client_data(kind, &bytes).unwrap(), msg);
    }

    /// 服务端控制指令往返；快照/事件走数据流并可 JSON 还原。
    #[test]
    fn server_messages_roundtrip() {
        let hs = ServerMessage::Handshake { assigned_id: 42, tick_rate_hz: 60 };
        let WireMessage::Command(u) = encode_server(&hs).unwrap() else { panic!("握手应为指令") };
        assert_eq!(decode_server_unit(&u[0]).unwrap(), hs);

        let snap = ServerMessage::Snapshot {
            seq: 7,
            entries: vec![crate::net::protocol::EntitySnapshot::placeholder(1)],
        };
        let WireMessage::Data(kind, bytes) = encode_server(&snap).unwrap() else { panic!("快照应为数据流") };
        assert_eq!(kind, DataKind::Snapshot);
        assert_eq!(decode_server_data(kind, &bytes).unwrap(), snap);

        // Hit 事件（0.15 扩 damage/reaction 字段）走事件数据流且 JSON 往返无损。
        let hit = ServerMessage::Event {
            kind: crate::net::protocol::EventKind::Hit {
                source_id: 1,
                target_id: 2,
                is_headshot: true,
                damage: 37.5,
                reaction: Some("蒸发".into()),
            },
        };
        let WireMessage::Data(kind, bytes) = encode_server(&hit).unwrap() else { panic!("事件应为数据流") };
        assert_eq!(kind, DataKind::Event);
        assert_eq!(decode_server_data(kind, &bytes).unwrap(), hit);
    }

    /// 预设目录含字符串，走控制类数据流且 JSON 往返无损。
    #[test]
    fn preset_catalog_becomes_control_data() {
        let msg = ServerMessage::PresetCatalog {
            presets: vec![
                crate::net::protocol::LoadoutPreset {
                    name: "标准".into(),
                    items: vec!["大型医疗包".into(), "烈焰手雷".into()],
                },
                crate::net::protocol::LoadoutPreset {
                    name: "爆破".into(),
                    items: vec!["破片手雷".into(), "步枪弹药".into()],
                },
            ],
        };
        let WireMessage::Data(kind, bytes) = encode_server(&msg).unwrap() else {
            panic!("预设目录应降级为数据流");
        };
        assert_eq!(kind, DataKind::Control);
        assert_eq!(decode_server_data(kind, &bytes).unwrap(), msg);
    }

    /// 世界目录（名册 + 地图布局）含字符串/可变长，走控制类数据流且 JSON 往返无损。
    #[test]
    fn world_catalog_becomes_control_data() {
        let msg = ServerMessage::WorldCatalog {
            roster: crate::operator::roster().to_vec(),
            layout: crate::map::lawn::layout(),
        };
        let WireMessage::Data(kind, bytes) = encode_server(&msg).unwrap() else {
            panic!("世界目录应降级为数据流");
        };
        assert_eq!(kind, DataKind::Control);
        assert_eq!(decode_server_data(kind, &bytes).unwrap(), msg);
    }

    /// 对象池同步：键数跨单元（4 键 → 2 单元），成组解码后完全还原。
    #[test]
    fn pool_sync_spans_units() {
        let msg = ClientMessage::PoolSync { region: 0, evicted: vec![11, 22, 33, 44] };
        let WireMessage::Command(units) = encode_client(&msg).unwrap() else { panic!("应为指令单元") };
        assert_eq!(units.len(), 2, "4 键应切成 2 单元");
        let back = decode_client_units(&units).unwrap();
        assert_eq!(back, vec![msg]);
    }
}
