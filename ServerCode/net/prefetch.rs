//! AOI 边缘预取预测（服务端权威）
//!
//! 设计动机（Why）：实体因 AOI（[`crate::net::aoi`]，半径 60m）出入视野时，客户端此前
//! 只能等资源随下一批数据抵达才能出画。0.11 起客户端有一个 16MB/64KB 固定槽位资源池，
//! 故服务端可以在实体**尚未进入视野**时，就把它最可能出现的资源提前塞进预取区——
//! 实体真正进入视野时资源已在池中，直接复用、零等待。
//!
//! 判据（owner 定）：对被 AOI 剔除的实体，按「到 AOI 边界的距离 ÷ 该实体最快速度」
//! 升序排序，取最靠前的 `k` 个——即"按当前运动最快会在多久后进入视野"最短者，
//! 预测它们会最先出现。

use crate::entity::{Entity, EntityId, World};
use crate::net::aoi::{in_interest, AOI_RADIUS};

/// 预测的实体数量（与客户端池"预取区"槽位数一致：6 × 64KB = 384KB）。
pub const PREFETCH_COUNT: usize = 6;

/// 速度下限（米/秒）：静止实体也按"很小但非零"的速度估计，避免除零放大静止目标。
const MIN_SPEED: f32 = 0.5;

/// 预测最可能进入观察者 AOI 的实体（按预计进入时间升序，最多 `k` 个）。
///
/// 只考虑当前**不在** AOI 内、且存活的实体；`observer` 为接收者世界坐标。
pub fn predict_prefetch(
    world: &World,
    observer: (f32, f32, f32),
    radius: f32,
    k: usize,
) -> Vec<EntityId> {
    let mut ranked: Vec<(f32, EntityId)> = world
        .get_all_entities()
        .iter()
        .filter(|e| e.is_alive)
        .filter(|e| !in_interest((e.position.x, e.position.y, e.position.z), observer, radius))
        .map(|e| (time_to_enter(e, observer, radius), e.id))
        .collect();
    // 时间升序；同刻按 ID 兜底，保证确定性（与全仓"实体按 ID 排序"一致）。
    ranked.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1.0.cmp(&b.1.0)));
    ranked.truncate(k);
    ranked.into_iter().map(|(_, id)| id).collect()
}

/// 便捷封装：使用默认 AOI 半径与预取槽数。
pub fn predict_default(world: &World, observer: (f32, f32, f32)) -> Vec<EntityId> {
    predict_prefetch(world, observer, AOI_RADIUS, PREFETCH_COUNT)
}

/// 估计实体"最快多久进入 AOI"（秒）：到边界距离 ÷ 最大速度。
fn time_to_enter(e: &Entity, observer: (f32, f32, f32), radius: f32) -> f32 {
    let dx = e.position.x - observer.0;
    let dy = e.position.y - observer.1;
    let dz = e.position.z - observer.2;
    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
    let to_boundary = (dist - radius).max(0.0);
    let speed = e.move_speed.max(MIN_SPEED);
    to_boundary / speed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat;
    use crate::damage::Vec3;

    /// 只有 AOI 外的实体进入预取候选；已在视野内的实体不重复预取。
    #[test]
    fn excludes_entities_already_in_interest() {
        let mut world = World::new();
        let near = combat::spawn_player(&mut world, Vec3::new(0.0, 0.0, 0.0), 0);
        let _far = combat::spawn_player(&mut world, Vec3::new(500.0, 0.0, 0.0), 1);
        let picks = predict_prefetch(&world, (0.0, 0.0, 0.0), AOI_RADIUS, PREFETCH_COUNT);
        assert!(!picks.contains(&near), "视野内实体不应进入预取候选");
        assert_eq!(picks.len(), 1, "只有 AOI 外的实体入候选");
    }

    /// 排序按"到边界距离 ÷ 速度"：更近的实体排在更前。
    #[test]
    fn closer_entity_ranked_first() {
        let mut world = World::new();
        // 两实体均在 AOI 外：一个接近边界、一个很远。
        let close = combat::spawn_player(&mut world, Vec3::new(70.0, 0.0, 0.0), 0);
        let far = combat::spawn_player(&mut world, Vec3::new(400.0, 0.0, 0.0), 1);
        let picks = predict_prefetch(&world, (0.0, 0.0, 0.0), AOI_RADIUS, PREFETCH_COUNT);
        assert_eq!(picks.first().copied(), Some(close), "更接近边界的实体应排更前");
        assert!(picks.contains(&far));
    }

    /// 候选数量不超过 k（预取槽位上限）。
    #[test]
    fn caps_at_slot_count() {
        let mut world = World::new();
        for i in 0..20 {
            let _ = combat::spawn_player(&mut world, Vec3::new(200.0 + i as f32 * 10.0, 0.0, 0.0), 0);
        }
        let picks = predict_default(&world, (0.0, 0.0, 0.0));
        assert_eq!(picks.len(), PREFETCH_COUNT, "预取候选应封顶为槽位数");
    }
}
