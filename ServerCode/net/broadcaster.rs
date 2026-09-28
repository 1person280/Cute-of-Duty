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
            let (weapon_elements, active_slot, ammo, ammo_max, reload_remaining, operator_id, skill_cd_q, skill_cd_e) =
                match e.get_component::<Combatant>() {
                    Some(cb) => (
                        Some([cb.weapons[0].element, cb.weapons[1].element]),
                        cb.active_slot as u8,
                        cb.active().ammo,
                        cb.active().max_ammo,
                        cb.reload_timer,
                        cb.operator_idx as u32,
                        cb.skill_q_cd,
                        cb.skill_e_cd,
                    ),
                    None => (None, 0u8, -1, -1, 0.0, 0u32, 0.0, 0.0),
                };
            // 格位内容：玩家背包 / 物资箱容器（各自权威，客户端只画两个 4×3 网格）。
            let backpack = e
                .get_component::<crate::items::Backpack>()
                .map(|bp| bp.slots.clone());
            // 权威备用弹药总量 = 背包内全部弹药堆合计（备弹唯一宿主是背包格位；无战斗态的
            // 实体置 -1，客户端显示 `--`）。
            let ammo_reserve = match (e.get_component::<crate::items::Backpack>(), e.get_component::<Combatant>()) {
                (Some(bp), Some(_)) => bp.ammo_total(),
                _ => -1,
            };
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
            // 持握手雷：玩家处于"先瞄准后释放"的中间态时给出元素，客户端据此渲染持雷提示
            // 与强制越肩；非玩家实体无此组件，恒为 None。
            let held_grenade = e
                .get_component::<crate::combat::HeldGrenade>()
                .and_then(|h| h.item.as_ref())
                .and_then(|it| match it.kind {
                    crate::map::PickupKind::Grenade { element } => Some(element),
                    _ => None,
                });
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
                ammo_reserve,
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
                held_grenade,
            }
        })
        .collect();

    // AOI 兴趣区域剔除：视野外实体不进入本客户端快照（控带宽 + 防 ESP）
    entries.retain(|e| aoi::in_interest((e.x, e.y, e.z), observer, AOI_RADIUS));

    ServerMessage::Snapshot { seq, entries }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat;
    use crate::element::ElementType;
    use crate::entity::World;
    use crate::items::{Backpack, ItemCategory};

    /// 从快照里取某个实体的 `held_grenade`。
    fn held_of(msg: &ServerMessage, id: u64) -> Option<ElementType> {
        match msg {
            ServerMessage::Snapshot { entries, .. } => {
                entries.iter().find(|e| e.entity_id == id).and_then(|e| e.held_grenade)
            }
            _ => None,
        }
    }

    /// 权威备用弹药总量 = 背包内全部弹药堆合计（开局两叠 64 → 128；从背包抽弹后随格位同步下降）。
    #[test]
    fn snapshot_reports_true_ammo_reserve() {
        let mut world = World::new();
        let pid = combat::spawn_player(&mut world, crate::damage::Vec3::default(), 0);

        let reserve_of = |msg: &ServerMessage| match msg {
            ServerMessage::Snapshot { entries, .. } => {
                entries.iter().find(|e| e.entity_id == pid.as_u64()).map(|e| e.ammo_reserve)
            }
            _ => None,
        };

        // 开局：背包 2×64 = 128（备弹唯一宿主是背包格位）。
        let snap = build_snapshot(&world, 1, (0.0, 0.0, 0.0));
        assert_eq!(reserve_of(&snap), Some(128), "开局备用弹药应为 128");

        // 模拟一次换弹抽走 30 发（直接读背包格位，无中间池）：128 - 30 = 98。
        {
            let e = world.get_entity_mut(pid).unwrap();
            let got = e.get_component_mut::<Backpack>().unwrap().draw_ammo(30);
            assert_eq!(got, 30, "应从背包抽出 30 发");
        }
        let snap = build_snapshot(&world, 2, (0.0, 0.0, 0.0));
        assert_eq!(reserve_of(&snap), Some(98), "抽弹后备用弹药应降到 98");
    }

    /// 持雷是服务端权威：进入持握后快照带 `held_grenade` 元素，未持雷玩家恒为 `None`。
    #[test]
    fn snapshot_reports_held_grenade() {
        let mut world = World::new();
        let pid = combat::spawn_player(&mut world, crate::damage::Vec3::default(), 0);
        let other = combat::spawn_player(&mut world, crate::damage::Vec3::default(), 1);

        // 未持雷：两名玩家均 None。
        let snap = build_snapshot(&world, 1, (0.0, 0.0, 0.0));
        assert!(held_of(&snap, pid.as_u64()).is_none(), "未持雷时 held_grenade 应为 None");
        assert!(held_of(&snap, other.as_u64()).is_none(), "另一玩家亦应为 None");

        // 使用第一格手雷 → 进入持握 → 快照给出元素（开局手雷为火元素）。
        let slot = world
            .get_entity(pid)
            .and_then(|e| e.get_component::<Backpack>())
            .and_then(|bp| {
                bp.slots
                    .iter()
                    .position(|s| s.as_ref().map(|it| it.kind.category()) == Some(Some(ItemCategory::Tactical)))
            })
            .expect("开局应携带手雷");
        combat::use_item_at(&mut world, pid, slot);

        let snap = build_snapshot(&world, 2, (0.0, 0.0, 0.0));
        assert_eq!(held_of(&snap, pid.as_u64()), Some(ElementType::Fire), "持雷应上报元素");
        assert!(held_of(&snap, other.as_u64()).is_none(), "未持雷玩家不受影响");
    }
}