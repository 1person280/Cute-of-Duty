//! 视线原点偏移（0.15.8）的验收测试。
//!
//! 为何独立成文件：`combat/tests.rs` 已逼近 600 行红线，本批回归锚点单列——
//! 覆盖「准星命中 ≠ 服务端命中」修复的三个断面：起点校验、合法偏移命中、超界回退。

use super::*;
use crate::element::{ElementConfig, ElementSystem};

/// 最小战斗环境（与 `combat/tests.rs` 的 harness 同构；独立成文以免跨测试模块借私字段）。
struct OriginHarness {
    world: World,
    resolver: DamageResolver,
    system: CombatSystem,
    env: EntityElementState,
}

impl OriginHarness {
    fn new() -> Self {
        let element_system = std::sync::Arc::new(ElementSystem::new(ElementConfig::default()));
        Self {
            world: World::new(),
            resolver: DamageResolver::new(element_system),
            system: CombatSystem::new(),
            env: EntityElementState::Normal,
        }
    }
}

/// 起点计算：全零/超界回退旧起点，合法偏移如实生效。
#[test]
fn aim_origin_validates_offset() {
    let base = Vec3::new(10.0, 0.0, 20.0);
    // 全零（旧端输入）→ 旧起点 脚底+1.5m
    assert_eq!(shooter::aim_origin(base, [0, 0, 0]), Vec3::new(10.0, 1.5, 20.0));
    // 合法偏移（厘米）→ 脚底 + 偏移
    assert_eq!(shooter::aim_origin(base, [100, 250, -50]), Vec3::new(11.0, 2.5, 19.5));
    // 超界（每轴 9m，模长 ≈15.6m > 8m）→ 整体回退旧起点
    assert_eq!(shooter::aim_origin(base, [900, 900, 900]), Vec3::new(10.0, 1.5, 20.0));
}

/// 偏移全链路：靶放在旧起点射线打不到的高位——旧语义 miss、携带相机偏移命中、
/// 超界偏移再回退 miss。修复「准星命中 ≠ 服务端命中」的回归锚点。
#[test]
fn aim_origin_offset_shifts_hit_ray() {
    let mut h = OriginHarness::new();
    let pid = spawn_player(&mut h.world, Vec3::default(), 0);
    // 靶心 (0, 2.5, 8)：旧起点 (0,1.5,0) 水平射线与靶面垂直距 1.0m > 0.85 半径 → miss
    let tid = range::spawn_target(&mut h.world, Vec3::new(0.0, 2.5, 8.0), "高位靶", None, false);

    // ① 旧语义（零偏移）：miss，无命中事件
    h.system.apply_input(&mut h.world, &h.resolver, pid, 0.016, &h.env,
        CombatIntent { shoot: true, ..CombatIntent::default() });
    assert!(h.system.drain_events().iter().all(|e| !matches!(e, CombatEvent::Hit { .. })), "旧起点射线不应命中高位靶");

    // ② 相机偏移 +2.5m 高度（厘米 i16）：起点 (0,2.5,0) 正对靶心 → 命中计分
    h.world.get_entity_mut(pid).unwrap().get_component_mut::<Combatant>().unwrap().shoot_cooldown = 0.0;
    h.system.apply_input(&mut h.world, &h.resolver, pid, 0.016, &h.env,
        CombatIntent { shoot: true, aim_origin_offset: [0, 250, 0], ..CombatIntent::default() });
    assert!(
        h.system.drain_events().iter().any(|e| matches!(e, CombatEvent::Hit { source, .. } if *source == pid.as_u64())),
        "携带合法相机偏移应命中高位靶"
    );
    assert_eq!(h.world.get_entity(tid).unwrap().get_component::<range::RangeTarget>().unwrap().hits, 1);

    // ③ 超界偏移（每轴 9m）：校验回退旧起点 → 再 miss
    h.world.get_entity_mut(pid).unwrap().get_component_mut::<Combatant>().unwrap().shoot_cooldown = 0.0;
    h.system.apply_input(&mut h.world, &h.resolver, pid, 0.016, &h.env,
        CombatIntent { shoot: true, aim_origin_offset: [900, 900, 900], ..CombatIntent::default() });
    assert!(h.system.drain_events().iter().all(|e| !matches!(e, CombatEvent::Hit { .. })), "超界偏移应回退旧起点");
    assert_eq!(h.world.get_entity(tid).unwrap().get_component::<range::RangeTarget>().unwrap().hits, 1);
}
