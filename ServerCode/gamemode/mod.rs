//! 游戏模式系统
//!
//! 支持多种游戏模式：
//! - 战术撤离（Extraction）：核心模式，搜刮→战斗→撤离
//! - 团队死斗（TDM）：练习模式
//! - 合约模式：完成特定任务目标

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{info, debug};

/// 游戏模式类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameModeType {
    /// 战术撤离（核心模式）
    Extraction,
    /// 团队死斗
    TeamDeathmatch,
    /// 合约模式
    Contract,
    /// 训练场
    Training,
}

impl GameModeType {
    pub fn name(&self) -> &'static str {
        match self {
            GameModeType::Extraction => "战术撤离",
            GameModeType::TeamDeathmatch => "团队死斗",
            GameModeType::Contract => "合约任务",
            GameModeType::Training => "训练场",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            GameModeType::Extraction => "搜刮战利品并抵达撤离点",
            GameModeType::TeamDeathmatch => "团队对抗，无限复活",
            GameModeType::Contract => "完成特定动态合约目标",
            GameModeType::Training => "安全区测试装备和元素Build",
        }
    }
}

/// 战局配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchConfig {
    pub mode: GameModeType,
    /// 战局时长（分钟）
    pub match_duration_minutes: u32,
    /// 地图名称
    pub map_name: String,
    /// 最大玩家数
    pub max_players: u32,
    /// 小队大小（1=独狼, 3=三人队）
    pub squad_size: u32,
    /// AI威胁等级（0-10）
    pub ai_threat_level: u32,
    /// 装备价值上限（0=无限制）
    pub gear_value_limit: u32,
    /// 是否允许带入装备
    pub allow_gear_import: bool,
    /// 天气/环境状态
    pub environment_state: String,
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self {
            mode: GameModeType::Extraction,
            match_duration_minutes: 20,
            map_name: "废弃工业区".to_string(),
            max_players: 12,
            squad_size: 3,
            ai_threat_level: 5,
            gear_value_limit: 0,
            allow_gear_import: true,
            environment_state: "normal".to_string(),
        }
    }
}

/// 战局状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchState {
    /// 等待玩家
    Waiting,
    /// 配装阶段
    Loadout,
    /// 战局进行中
    InProgress,
    /// 战局结束
    Finished,
    /// 结算中
    Settling,
}

/// 玩家战局数据
#[derive(Debug, Clone)]
pub struct PlayerMatchData {
    pub player_id: u64,
    pub squad_id: Option<u32>,
    pub is_alive: bool,
    pub kills: u32,
    pub deaths: u32,
    pub damage_dealt: f32,
    pub damage_taken: f32,
    pub loot_value: u32,
    pub has_extracted: bool,
    pub extraction_time: Option<Instant>,
}

impl PlayerMatchData {
    pub fn new(player_id: u64) -> Self {
        Self {
            player_id,
            squad_id: None,
            is_alive: true,
            kills: 0,
            deaths: 0,
            damage_dealt: 0.0,
            damage_taken: 0.0,
            loot_value: 0,
            has_extracted: false,
            extraction_time: None,
        }
    }
}

/// 战局管理器
pub struct MatchManager {
    config: MatchConfig,
    state: MatchState,
    start_time: Option<Instant>,
    end_time: Option<Instant>,
    players: Vec<PlayerMatchData>,
    current_tick: u64,
}

impl MatchManager {
    pub fn new(config: MatchConfig) -> Self {
        info!("创建战局: {:?} - {}", config.mode, config.map_name);
        
        Self {
            config,
            state: MatchState::Waiting,
            start_time: None,
            end_time: None,
            players: Vec::new(),
            current_tick: 0,
        }
    }

    /// 添加玩家
    pub fn add_player(&mut self, player_id: u64, squad_id: Option<u32>) {
        let mut data = PlayerMatchData::new(player_id);
        data.squad_id = squad_id;
        self.players.push(data);
        debug!("玩家 {} 加入战局", player_id);
    }

    /// 开始战局
    pub fn start_match(&mut self) {
        if self.state != MatchState::Waiting && self.state != MatchState::Loadout {
            return;
        }
        
        self.state = MatchState::InProgress;
        self.start_time = Some(Instant::now());
        self.current_tick = 0;
        
        info!("战局开始! 地图: {}, 玩家数: {}", 
            self.config.map_name, self.players.len());
    }

    /// 结束战局
    pub fn end_match(&mut self) {
        self.state = MatchState::Finished;
        self.end_time = Some(Instant::now());
        
        info!("战局结束! 存活玩家: {}", 
            self.players.iter().filter(|p| p.is_alive).count());
    }

    /// 玩家撤离
    pub fn player_extract(&mut self, player_id: u64) -> bool {
        if let Some(player) = self.players.iter_mut().find(|p| p.player_id == player_id) {
            if player.is_alive && !player.has_extracted {
                player.has_extracted = true;
                player.extraction_time = Some(Instant::now());
                info!("玩家 {} 成功撤离! 战利品价值: {}", player_id, player.loot_value);
                return true;
            }
        }
        false
    }

    /// 玩家死亡
    pub fn player_death(&mut self, player_id: u64, killer_id: Option<u64>) {
        if let Some(player) = self.players.iter_mut().find(|p| p.player_id == player_id) {
            player.is_alive = false;
            player.deaths += 1;
            
            if let Some(killer) = killer_id {
                if let Some(killer_data) = self.players.iter_mut().find(|p| p.player_id == killer) {
                    killer_data.kills += 1;
                }
            }
            
            debug!("玩家 {} 死亡 (击杀者: {:?})", player_id, killer_id);
        }
    }

    /// 记录伤害
    pub fn record_damage(&mut self, from_player: u64, to_player: u64, amount: f32) {
        if let Some(player) = self.players.iter_mut().find(|p| p.player_id == from_player) {
            player.damage_dealt += amount;
        }
        if let Some(player) = self.players.iter_mut().find(|p| p.player_id == to_player) {
            player.damage_taken += amount;
        }
    }

    /// 记录战利品价值
    pub fn add_loot_value(&mut self, player_id: u64, value: u32) {
        if let Some(player) = self.players.iter_mut().find(|p| p.player_id == player_id) {
            player.loot_value += value;
        }
    }

    /// 更新（每Tick调用）
    pub fn tick(&mut self, _delta_time: f32) {
        if self.state != MatchState::InProgress {
            return;
        }
        
        self.current_tick += 1;
        
        // 检查战局时间是否结束
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed();
            let max_duration = Duration::from_secs(self.config.match_duration_minutes as u64 * 60);
            
            if elapsed >= max_duration {
                info!("战局时间到! 强制结束");
                self.end_match();
            }
        }
        
        // 检查是否所有玩家都已撤离或死亡
        let all_done = self.players.iter().all(|p| !p.is_alive || p.has_extracted);
        if all_done && !self.players.is_empty() {
            info!("所有玩家已撤离或死亡，战局结束");
            self.end_match();
        }
    }

    /// 获取剩余时间
    pub fn get_remaining_time(&self) -> Option<Duration> {
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed();
            let max_duration = Duration::from_secs(self.config.match_duration_minutes as u64 * 60);
            if elapsed < max_duration {
                return Some(max_duration - elapsed);
            }
        }
        None
    }

    /// 生成战局报告
    pub fn generate_match_report(&self) -> MatchReport {
        MatchReport {
            mode: self.config.mode,
            map_name: self.config.map_name.clone(),
            duration: self.start_time.map(|s| s.elapsed()),
            player_stats: self.players.clone(),
            total_kills: self.players.iter().map(|p| p.kills).sum(),
            total_extracted: self.players.iter().filter(|p| p.has_extracted).count() as u32,
        }
    }

    // Getters
    pub fn state(&self) -> MatchState { self.state }
    pub fn config(&self) -> &MatchConfig { &self.config }
    pub fn player_count(&self) -> usize { self.players.len() }
    pub fn alive_count(&self) -> usize { self.players.iter().filter(|p| p.is_alive && !p.has_extracted).count() }
}

/// 战局报告
#[derive(Debug, Clone)]
pub struct MatchReport {
    pub mode: GameModeType,
    pub map_name: String,
    pub duration: Option<Duration>,
    pub player_stats: Vec<PlayerMatchData>,
    pub total_kills: u32,
    pub total_extracted: u32,
}

/// 撤离点
#[derive(Debug, Clone)]
pub struct ExtractionPoint {
    pub id: u32,
    pub name: String,
    pub position: (f32, f32, f32),
    pub radius: f32,
    pub is_active: bool,
    pub activation_delay_seconds: u32,
}

impl ExtractionPoint {
    pub fn new(id: u32, name: &str, x: f32, y: f32, z: f32, radius: f32) -> Self {
        Self {
            id,
            name: name.to_string(),
            position: (x, y, z),
            radius,
            is_active: true,
            activation_delay_seconds: 30,
        }
    }

    /// 检查实体是否在撤离范围内
    pub fn is_in_range(&self, entity_x: f32, entity_y: f32, entity_z: f32) -> bool {
        let dx = entity_x - self.position.0;
        let dy = entity_y - self.position.1;
        let dz = entity_z - self.position.2;
        let dist_sq = dx * dx + dy * dy + dz * dz;
        dist_sq <= self.radius * self.radius
    }
}

/// 动态合约系统
#[derive(Debug, Clone)]
pub struct Contract {
    pub id: u32,
    pub contract_type: ContractType,
    pub description: String,
    pub reward_value: u32,
    pub is_completed: bool,
    pub target_id: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractType {
    EliminateHVT,      // 击杀高价值目标
    ExtractItem,       // 护送/提取物资
    CapturePoint,      // 占领据点
    Recon,             // 侦察
}

impl Contract {
    pub fn eliminate_hvt(target_id: u64, reward: u32) -> Self {
        Self {
            id: rand::random(),
            contract_type: ContractType::EliminateHVT,
            description: format!("击杀高价值目标 {}", target_id),
            reward_value: reward,
            is_completed: false,
            target_id: Some(target_id),
        }
    }

    pub fn extract_item(item_name: &str, reward: u32) -> Self {
        Self {
            id: rand::random(),
            contract_type: ContractType::ExtractItem,
            description: format!("提取物资: {}", item_name),
            reward_value: reward,
            is_completed: false,
            target_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_lifecycle() {
        let config = MatchConfig::default();
        let mut manager = MatchManager::new(config);
        
        assert_eq!(manager.state(), MatchState::Waiting);
        
        manager.add_player(1, Some(1));
        manager.add_player(2, Some(1));
        manager.add_player(3, Some(2));
        
        assert_eq!(manager.player_count(), 3);
        
        manager.start_match();
        assert_eq!(manager.state(), MatchState::InProgress);
        
        // 模拟玩家1撤离
        assert!(manager.player_extract(1));
        
        // 模拟玩家2死亡
        manager.player_death(2, Some(3));
        
        assert_eq!(manager.alive_count(), 1);
        
        // 模拟玩家3撤离
        assert!(manager.player_extract(3));
        
        // 所有玩家都已撤离或死亡
        manager.tick(0.016);
        assert_eq!(manager.state(), MatchState::Finished);
    }

    #[test]
    fn test_extraction_point() {
        let point = ExtractionPoint::new(1, "主撤离点", 100.0, 0.0, 100.0, 10.0);
        
        assert!(point.is_in_range(100.0, 0.0, 100.0)); // 中心点
        assert!(point.is_in_range(105.0, 0.0, 105.0)); // 范围内
        assert!(!point.is_in_range(120.0, 0.0, 120.0)); // 范围外
    }

    #[test]
    fn test_contract_creation() {
        let contract = Contract::eliminate_hvt(42, 5000);
        assert_eq!(contract.contract_type, ContractType::EliminateHVT);
        assert_eq!(contract.reward_value, 5000);
        assert!(!contract.is_completed);
    }
}
