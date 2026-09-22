//! 游戏引擎核心
//!
//! 设计原则：
//! - 固定Tick主时钟（60Hz）
//! - 逻辑与渲染完全解耦
//! - 确定性保证：实体按ID排序处理
//! - 双缓冲状态快照

use std::sync::Arc;
use tracing::{info, debug, warn};

use crate::element::{ElementSystem, EntityElementState};
use crate::equipment::EquipmentSystem;
use crate::entity::{World, RenderSnapshot, Entity, EntityType, EntityId};
use crate::damage::Vec3;
use crate::damage::DamageResolver;

mod double_buffer;
pub use double_buffer::DoubleBuffer;
mod pre_explosion_cache;
pub use pre_explosion_cache::PreExplosionCache;

/// Tick配置
#[derive(Debug, Clone, Copy)]
pub struct TickConfig {
    /// Tick率（Hz）
    pub tick_rate_hz: u32,
    /// 最大帧时间（毫秒）
    pub max_frame_time_ms: f32,
    /// 是否启用确定性检查
    pub enable_determinism_check: bool,
}

impl Default for TickConfig {
    fn default() -> Self {
        Self {
            tick_rate_hz: 60,
            max_frame_time_ms: 50.0,
            enable_determinism_check: true,
        }
    }
}

/// 游戏循环状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameLoopState {
    Initialization,
    Running,
    Paused,
    ShuttingDown,
    Stopped,
}

/// 游戏循环
///
/// 核心流程：
/// 1. 批量输入收集
/// 2. 并行只读Job处理
/// 3. 单线程状态提交（确定性保证）
/// 4. 生成渲染快照，双缓冲交换
/// 5. 网络序列化
pub struct GameLoop {
    config: TickConfig,
    current_tick: u64,
    state: GameLoopState,

    // 游戏世界
    world: World,

    // 世界环境状态（影响元素反应的环境修正）
    environment: EntityElementState,

    // 双缓冲
    double_buffer: DoubleBuffer,

    // 伤害结算器（内部持有Arc<ElementSystem>）
    damage_resolver: DamageResolver,

    // 确定性验证
    state_hashes: Vec<(u64, u64)>,       // 当前运行: (tick, hash)
    reference_hashes: Vec<(u64, u64)>,   // 参考运行基准: (tick, hash)

    // 统计
    tick_durations_us: Vec<u64>,
}

impl GameLoop {
    /// 创建新的游戏循环
    ///
    /// `equipment_system` 参数为装备系统预留（当前伤害结算未使用），
    /// 接入装备减伤/元素相性后在此展开。
    pub fn new(
        config: TickConfig,
        element_system: Arc<ElementSystem>,
        _equipment_system: Arc<EquipmentSystem>,
    ) -> Self {
        let world = Self::create_initial_world();

        info!("游戏世界初始化完成，实体数量: {}", world.entity_count());

        let damage_resolver = DamageResolver::new(element_system);

        Self {
            config,
            current_tick: 0,
            state: GameLoopState::Initialization,
            world,
            environment: EntityElementState::Normal,
            double_buffer: DoubleBuffer::new(),
            damage_resolver,
            state_hashes: Vec::new(),
            reference_hashes: Vec::new(),
            tick_durations_us: Vec::new(),
        }
    }

    /// 创建初始游戏世界（确定性布局：1玩家 + 2 AI）
    ///
    /// 独立成函数，供 new() 与 reset() 共用，
    /// 保证重放时初始状态与首次运行完全一致。
    fn create_initial_world() -> World {
        let mut world = World::new();
        world.spawn(Entity::new_player(0, Vec3::new(0.0, 0.0, 0.0)));
        world.spawn(Entity::new_ai(0, Vec3::new(50.0, 0.0, 30.0)));
        world.spawn(Entity::new_ai(0, Vec3::new(-30.0, 0.0, 50.0)));
        world
    }

    /// 设置世界环境状态（影响元素反应的环境修正）
    pub fn set_environment(&mut self, environment: EntityElementState) {
        self.environment = environment;
    }

    /// 运行测试Tick（实时节流版，运行有限Tick后退出）
    pub async fn run_test_ticks(&mut self, max_ticks: u64) -> Result<(), Box<dyn std::error::Error>> {
        self.run_ticks_internal(max_ticks, true).await?;
        self.print_statistics();
        Ok(())
    }

    /// 重放Tick（不节流，尽可能快地运行），用于确定性验证
    pub async fn run_replay_ticks(&mut self, max_ticks: u64) -> Result<(), Box<dyn std::error::Error>> {
        self.run_ticks_internal(max_ticks, false).await?;
        self.print_statistics();
        Ok(())
    }

    async fn run_ticks_internal(&mut self, max_ticks: u64, paced: bool) -> Result<(), Box<dyn std::error::Error>> {
        self.state = GameLoopState::Running;

        let tick_interval_us = 1_000_000u64 / self.config.tick_rate_hz as u64;
        let tick_dt = tick_interval_us as f32 / 1_000_000.0;
        let mut last_tick_time = std::time::Instant::now();

        info!("开始游戏循环，Tick间隔: {}μs，模式: {}",
            tick_interval_us,
            if paced { "实时节流" } else { "重放(不节流)" });

        while self.current_tick < max_ticks && self.state == GameLoopState::Running {
            let tick_no = self.current_tick;
            let tick_start = std::time::Instant::now();

            // 执行一个Tick（内部递增current_tick）
            self.tick(tick_dt);

            let tick_duration_us = tick_start.elapsed().as_micros() as u64;
            self.tick_durations_us.push(tick_duration_us);

            if paced {
                // 计算到下一Tick的剩余时间
                let elapsed = last_tick_time.elapsed();
                let target_duration = std::time::Duration::from_micros(tick_interval_us);

                if elapsed < target_duration {
                    tokio::time::sleep(target_duration - elapsed).await;
                } else {
                    warn!("Tick {} 超时: 耗时{}μs > 目标{}μs",
                        tick_no, tick_duration_us, tick_interval_us);
                }

                last_tick_time = std::time::Instant::now();
            }
        }

        self.state = GameLoopState::Stopped;

        Ok(())
    }

    /// 将当前运行的状态哈希记录保存为参考基准（确定性验证用）
    pub fn take_reference(&mut self) {
        self.reference_hashes = self.state_hashes.clone();
        info!("已记录参考基准: {} 个Tick哈希", self.reference_hashes.len());
    }

    /// 重置到初始状态（保留参考基准），用于确定性重放
    pub fn reset(&mut self) {
        self.current_tick = 0;
        self.state = GameLoopState::Initialization;
        self.world = Self::create_initial_world();
        self.double_buffer = DoubleBuffer::new();
        self.state_hashes.clear();
        self.tick_durations_us.clear();
    }

    /// 执行单个Tick
    fn tick(&mut self, delta_time: f32) {
        // ===== 阶段1: 批量输入收集 =====
        self.collect_input();
        
        // ===== 阶段2: 并行只读Job =====
        // 简化版：在单线程中顺序执行
        // 实际实现应使用Job System调度到多个Worker线程
        self.run_jobs();
        
        // ===== 阶段3: 单线程状态提交 =====
        // 处理所有实体的收件箱和组件更新
        // 关键：按实体ID排序处理，保证确定性
        self.process_entities(delta_time);
        
        // ===== 阶段4: 生成渲染快照，双缓冲交换 =====
        self.update_render_snapshot();
        
        // ===== 阶段5: 网络序列化 =====
        self.serialize_network_state();
        
        // ===== 确定性验证 =====
        if self.config.enable_determinism_check {
            self.record_state_hash();
        }

        // Tick计数随Tick推进（确定性验证依赖逐Tick哈希对齐）
        self.current_tick += 1;
    }

    /// 收集输入
    fn collect_input(&mut self) {
        // 简化版：模拟一些AI移动
        // 实际实现应从HAL输入缓冲区读取
        
        // 模拟AI行为
        let ai_ids: Vec<_> = self.world.get_all_entities()
            .iter()
            .filter(|e| e.entity_type == EntityType::AI && e.is_alive)
            .map(|e| e.id)
            .collect();
        
        for id in ai_ids {
            if let Some(ai) = self.world.get_entity_mut(id) {
                // 简单AI：随机巡逻
                let offset = ((self.current_tick as f32 * 0.01) + id.0 as f32).sin();
                ai.position.x += offset * 0.1;
                ai.position.z += offset.cos() * 0.1;
            }
        }
    }

    /// 运行并行Job（简化版）
    fn run_jobs(&mut self) {
        // 实际实现应使用Job System：
        // - 空间查询Job
        // - 视野检测Job
        // - 路径寻路Job
        // - 伤害预计算Job
        
        // 简化：在单线程中执行
        debug!("Tick {}: 执行Job", self.current_tick);
    }

    /// 处理所有实体（确定性关键：按ID排序）
    fn process_entities(&mut self, delta_time: f32) {
        // 收集所有实体ID并排序
        let mut ids: Vec<_> = self.world.get_all_entities()
            .iter()
            .map(|e| e.id)
            .collect();
        ids.sort_by_key(|id| id.0);

        // DamageResolver内部是Arc，克隆开销极小；
        // 先克隆到局部变量，避免与世界可变借用冲突
        let resolver = self.damage_resolver.clone();

        // 处理每个实体
        for id in ids {
            if let Some(entity) = self.world.get_entity_mut(id) {
                // 处理收件箱（结算所有待处理的伤害包）
                entity.process_inbox(&resolver, &self.environment);

                // 执行实体Tick（组件更新）
                entity.tick(delta_time);
            }
        }

        // 清理死亡实体
        self.cleanup_dead_entities();
    }

    /// 清理死亡实体
    fn cleanup_dead_entities(&mut self) {
        let dead_ids: Vec<_> = self.world.get_all_entities()
            .iter()
            .filter(|e| !e.is_alive)
            .map(|e| e.id)
            .collect();
        
        for id in dead_ids {
            self.world.despawn(id);
            debug!("实体 {:?} 已死亡并移除", id);
        }
    }

    /// 更新渲染快照
    fn update_render_snapshot(&mut self) {
        let snapshot = RenderSnapshot::from_world(&self.world, self.current_tick);
        
        // 写入逻辑缓冲区
        self.double_buffer.get_logic_buffer().tick = snapshot.tick;
        self.double_buffer.get_logic_buffer().entity_positions = snapshot.entity_positions;
        self.double_buffer.get_logic_buffer().entity_hp = snapshot.entity_hp;
        self.double_buffer.get_logic_buffer().entity_states = snapshot.entity_states;
        
        // 原子交换双缓冲
        self.double_buffer.swap();
    }

    /// 序列化网络状态
    fn serialize_network_state(&mut self) {
        // 简化版：记录状态哈希
        // 实际实现应使用rkyv零拷贝序列化
        debug!("Tick {}: 网络序列化", self.current_tick);
    }

    /// 记录状态哈希（确定性验证）
    fn record_state_hash(&mut self) {
        let snapshot = RenderSnapshot::from_world(&self.world, self.current_tick);
        let hash = snapshot.compute_hash();
        self.state_hashes.push((self.current_tick, hash));
    }

    /// 验证确定性
    ///
    /// 完整验证流程（见 main.rs）：
    /// 1. 第一次运行后调用 take_reference() 保存逐Tick状态哈希
    /// 2. reset() 回到初始状态后重放同样的Tick
    /// 3. 本方法逐Tick比对当前哈希与参考基准
    ///
    /// 同时校验记录完整性：无重复Tick、长度与基准一致。
    /// 若尚未建立参考基准，则仅做完整性检查。
    pub fn verify_determinism(&self) -> bool {
        if self.state_hashes.is_empty() {
            warn!("确定性验证失败：没有任何Tick哈希记录");
            return false;
        }

        // 检查是否有重复的Tick（不应该发生）
        let mut ticks: Vec<_> = self.state_hashes.iter().map(|(t, _)| *t).collect();
        ticks.sort_unstable();
        ticks.dedup();

        if ticks.len() != self.state_hashes.len() {
            warn!("确定性验证失败：存在重复的Tick记录");
            return false;
        }

        // 无参考基准：仅完成完整性检查
        if self.reference_hashes.is_empty() {
            info!("未建立参考基准，仅完成记录完整性检查: {} 个Tick", self.state_hashes.len());
            return true;
        }

        // 长度校验
        if self.reference_hashes.len() != self.state_hashes.len() {
            warn!("确定性验证失败：Tick数量不一致（参考{} vs 当前{}）",
                self.reference_hashes.len(), self.state_hashes.len());
            return false;
        }

        // 逐Tick比对状态哈希
        for ((ref_tick, ref_hash), (tick, hash)) in
            self.reference_hashes.iter().zip(self.state_hashes.iter())
        {
            if ref_tick != tick {
                warn!("确定性验证失败：Tick序号错位（参考{} vs 当前{}）", ref_tick, tick);
                return false;
            }
            if ref_hash != hash {
                warn!("确定性验证失败：Tick {} 状态哈希不一致（参考{:016x} vs 当前{:016x}）",
                    tick, ref_hash, hash);
                return false;
            }
        }

        info!("确定性验证通过：{} 个Tick状态哈希与参考运行完全一致", self.state_hashes.len());
        true
    }

    /// 打印统计信息
    fn print_statistics(&self) {
        if self.tick_durations_us.is_empty() {
            return;
        }
        
        let total_ticks = self.tick_durations_us.len();
        let avg_duration = self.tick_durations_us.iter().sum::<u64>() / total_ticks as u64;
        let max_duration = *self.tick_durations_us.iter().max().unwrap_or(&0);
        let min_duration = *self.tick_durations_us.iter().min().unwrap_or(&0);
        
        let target_us = 1_000_000u64 / self.config.tick_rate_hz as u64;
        let overruns = self.tick_durations_us.iter().filter(|&&d| d > target_us).count();
        
        info!("========== 游戏循环统计 ==========");
        info!("总Tick数: {}", total_ticks);
        info!("平均Tick耗时: {}μs ({}ms)", avg_duration, avg_duration as f32 / 1000.0);
        info!("最大Tick耗时: {}μs ({}ms)", max_duration, max_duration as f32 / 1000.0);
        info!("最小Tick耗时: {}μs ({}ms)", min_duration, min_duration as f32 / 1000.0);
        info!("目标Tick耗时: {}μs ({}ms)", target_us, target_us as f32 / 1000.0);
        info!("超时次数: {}/{} ({:.1}%)", overruns, total_ticks, 
            overruns as f32 / total_ticks as f32 * 100.0);
        info!("===================================");
    }

    /// 获取当前Tick
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// 获取世界引用
    pub fn world(&self) -> &World {
        &self.world
    }

    /// 获取世界可变引用
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// 获取渲染快照
    pub fn get_render_snapshot(&self) -> &RenderSnapshot {
        self.double_buffer.get_render_buffer()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::{ElementConfig, ElementType};
    use crate::damage::DamagePacket;

    fn create_test_game_loop() -> GameLoop {
        let element_config = ElementConfig::default();
        let element_system = Arc::new(ElementSystem::new(element_config));
        let equipment_system = Arc::new(EquipmentSystem::new());
        
        GameLoop::new(
            TickConfig::default(),
            element_system,
            equipment_system,
        )
    }

    #[test]
    fn test_game_loop_initialization() {
        let game_loop = create_test_game_loop();
        assert_eq!(game_loop.current_tick(), 0);
        assert_eq!(game_loop.world().entity_count(), 3); // 1 player + 2 AI
    }

    #[test]
    fn test_double_buffer_swap() {
        let mut buffer = DoubleBuffer::new();
        
        buffer.get_logic_buffer().tick = 100;
        buffer.swap();
        
        assert_eq!(buffer.get_render_buffer().tick, 100);
    }

    #[test]
    fn test_inbox_damage_processed_in_tick() {
        let mut game_loop = create_test_game_loop();

        // 向第一个实体（玩家）的收件箱推送一个伤害包
        let player_id = game_loop.world().get_all_entities()[0].id;
        let hp_before = game_loop.world().get_entity(player_id).unwrap().hp;

        let packet = DamagePacket::new(ElementType::Fire, 100.0, EntityId::new(99));
        game_loop.world_mut()
            .get_entity_mut(player_id)
            .unwrap()
            .push_damage(packet);

        // 执行一个Tick，收件箱应被消费并结算
        game_loop.tick(1.0 / 60.0);

        // 目标要么血量下降，要么已被结算致死并清理
        match game_loop.world().get_entity(player_id) {
            Some(entity) => assert!(entity.hp < hp_before, "收件箱中的伤害包应被结算"),
            None => {} // 伤害致死被清理，同样说明结算生效
        }
    }

    #[test]
    fn test_determinism_replay_matches() {
        let mut game_loop = create_test_game_loop();

        // 第一次运行
        for _ in 0..120 {
            game_loop.tick(1.0 / 60.0);
        }
        game_loop.take_reference();

        // 重放：重置到相同初始状态后再跑相同Tick数
        game_loop.reset();
        for _ in 0..120 {
            game_loop.tick(1.0 / 60.0);
        }

        assert!(game_loop.verify_determinism(), "相同初始状态的重放应产出完全一致的状态哈希");
    }

    #[test]
    fn test_determinism_detects_divergence() {
        let mut game_loop = create_test_game_loop();

        // 参考运行
        for _ in 0..10 {
            game_loop.tick(1.0 / 60.0);
        }
        game_loop.take_reference();
        game_loop.reset();

        // 篡改初始状态后重放 → 哈希应当不一致（证明比对逻辑有效）
        if let Some(entity) = game_loop.world_mut().get_all_entities_mut().next() {
            entity.position.x += 5.0;
        }
        for _ in 0..10 {
            game_loop.tick(1.0 / 60.0);
        }

        assert!(!game_loop.verify_determinism(), "状态被篡改后的重放应被判定为不确定");
    }
}
