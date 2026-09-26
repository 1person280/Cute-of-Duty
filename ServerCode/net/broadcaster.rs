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
            let (weapon_elements, active_slot, ammo, ammo_max, ammo_pool, reload_remaining, operator_id, skill_cd_q, skill_cd_e) =
                match e.get_component::<Combatant>() {
                    Some(cb) => (
                        Some([cb.weapons[0].element, cb.weapons[1].element]),
                        cb.active_slot as u8,
                        cb.active().ammo,
                        cb.active().max_ammo,
                        cb.ammo_pool,
                        cb.reload_timer,
                        cb.operator_idx as u32,
                        cb.skill_q_cd,
                        cb.skill_e_cd,
                    ),
                    None => (None, 0u8, -1, -1, -1, 0.0, 0u32, 0.0, 0.0),
                };
            // 格位内容：玩家背包 / 物资箱容器（各自权威，客户端只画两个 4×3 网格）。
            let backpack = e
                .get_component::<crate::items::Backpack>()
                .map(|bp| bp.slots.clone());
            let container = e
                .get_component::<crate::items::Container>()
                .map(|ct| ct.slots.clone());
            // 造型裁决：物资箱站点虽是 Station 实体，但语义上是"木箱"，覆盖为对应预设，
            // 其余实体沿用按类型派生的默认造型（"这个实体长什么样"归服务端）。
            let model_preset = match e.get_component::<crate::interact::Interactable>() {
                Some(it) if it.kind == crate::interact::InteractKind::Station(crate::map::StationKind::SupplyCrate) => {
                    crate::model::ModelPreset::SupplyCrate
                }
                _ => crate::model::ModelPreset::from_entity_type(e.entity_type),
            };
            EntitySnapshot {
                entity_id: e.id.as_u64(),
                x: e.position.x,
                y: e.position.y,
                z: e.position.z,
                hp: e.hp,
                is_alive: e.is_alive,
                model_preset,
                element_state: e.element_state.clone(),
                armor: e.armor,
                weapon_elements,
                active_slot,
                ammo,
                ammo_max,
                ammo_pool,
                reload_remaining,
                operator_id,
                skill_cd_q,
                skill_cd_e,
                // 可交互语义（拾取物/功能站点）；其余实体为 None。
                interact: e
                    .get_component::<crate::interact::Interactable>()
                    .map(|it| it.info()),
                backpack,
                container,
            }
        })
        .collect();

    // AOI 兴趣区域剔除：视野外实体不进入本客户端快照（控带宽 + 防 ESP）
    entries.retain(|e| aoi::in_interest((e.x, e.y, e.z), observer, AOI_RADIUS));

    ServerMessage::Snapshot { seq, entries }
}