//! 权威快照构建（服务器权威 + AOI 过滤）
//!
//! 每个固定 Tick 由服务端主循环调用 `build_snapshot`：把模拟世界（`World`）的
//! 真实状态映射为线格式 `ServerMessage::Snapshot`，并围绕观察者位置做 AOI 剔除，
//! 再经 `session` 分发给各客户端。客户端收到的永远是服务端算好的结果。

use crate::combat::Combatant;
use crate::entity::World;
use crate::net::aoi::{self, AOI_RADIUS};
use crate::net::protocol::{EntitySnapshot, ServerMessage};

/// 从权威世界构建单帧快照，并按 AOI 兴趣区域过滤。
///
/// `observer` 为本次快照的接收者的世界坐标；`seq` 为该帧的全局 Tick 序号。
/// 设计动机：快照内容完全由服务端模拟决定（位置/血量/元素状态都是权威值），
/// 客户端只负责反序列化显示，绝无本地校订——服务器权威架构的落点。
pub fn build_snapshot(world: &World, seq: u64, observer: (f32, f32, f32)) -> ServerMessage {
    // 先把世界实体映射为线格式（确定性顺序：实体按 ID 排序）
    let mut entries: Vec<EntitySnapshot> = world
        .get_all_entities()
        .iter()
        .map(|e| {
            // 战斗展示字段：玩家实体读权威 Combatant，其余实体无战斗态则占位
            let (weapon_index, ammo, operator_id, skill_cd_q, skill_cd_e) = match e.get_component::<Combatant>() {
                Some(cb) => (0u8, cb.ammo, cb.operator_idx as u32, cb.skill_q_cd, cb.skill_e_cd),
                None => (0u8, -1, 0u32, 0.0, 0.0),
            };
            EntitySnapshot {
                entity_id: e.id.as_u64(),
                x: e.position.x,
                y: e.position.y,
                z: e.position.z,
                hp: e.hp,
                is_alive: e.is_alive,
                model_preset: crate::model::ModelPreset::from_entity_type(e.entity_type),
                element_state: e.element_state.clone(),
                armor: e.armor,
                weapon_index,
                ammo,
                operator_id,
                skill_cd_q,
                skill_cd_e,
            }
        })
        .collect();

    // AOI 兴趣区域剔除：视野外实体不进入本客户端快照（控带宽 + 防 ESP）
    entries.retain(|e| aoi::in_interest((e.x, e.y, e.z), observer, AOI_RADIUS));

    ServerMessage::Snapshot { seq, entries }
}