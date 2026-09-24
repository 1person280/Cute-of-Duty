//! 玩家档案与进度系统
//!
//! 包含：
//! - 玩家基础档案
//! - 信誉系统
//! - 赛季进度
//! - 统计数据

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 玩家档案
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerProfile {
    pub player_id: u64,
    pub username: String,
    pub level: u32,
    pub experience: u64,
    pub reputation: Reputation,
    pub stats: PlayerStats,
    pub season_progress: SeasonProgress,
    pub inventory: PlayerInventory,
    pub settings: PlayerSettings,
}

impl PlayerProfile {
    pub fn new(player_id: u64, username: &str) -> Self {
        Self {
            player_id,
            username: username.to_string(),
            level: 1,
            experience: 0,
            reputation: Reputation::new(),
            stats: PlayerStats::default(),
            season_progress: SeasonProgress::new(1),
            inventory: PlayerInventory::new(),
            settings: PlayerSettings::default(),
        }
    }

    /// 增加经验值
    pub fn add_experience(&mut self, amount: u64) {
        self.experience += amount;
        
        // 简单的等级计算：每级需要 等级×1000 经验
        let new_level = ((self.experience as f64) / 1000.0).sqrt() as u32 + 1;
        if new_level > self.level {
            self.level = new_level;
        }
    }

    /// 更新统计
    pub fn update_stats(&mut self, match_stats: &crate::gamemode::PlayerMatchData) {
        self.stats.total_matches += 1;
        self.stats.total_kills += match_stats.kills;
        self.stats.total_deaths += match_stats.deaths;
        
        if match_stats.has_extracted {
            self.stats.successful_extractions += 1;
        }
        
        self.stats.total_damage_dealt += match_stats.damage_dealt;
        self.stats.total_damage_taken += match_stats.damage_taken;
        self.stats.total_loot_value += match_stats.loot_value;
        
        // 计算KD比
        if self.stats.total_deaths > 0 {
            self.stats.kd_ratio = self.stats.total_kills as f32 / self.stats.total_deaths as f32;
        }
        
        // 计算撤离率
        self.stats.extraction_rate = self.stats.successful_extractions as f32 
            / self.stats.total_matches as f32;
    }
}

/// 信誉系统
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reputation {
    /// 信誉分（0-100）
    pub score: u32,
    /// 举报次数
    pub reports_received: u32,
    /// 被举报类型统计
    pub report_reasons: HashMap<ReportReason, u32>,
    /// 信誉历史
    pub history: Vec<ReputationChange>,
}

impl Reputation {
    pub fn new() -> Self {
        Self {
            score: 100,
            reports_received: 0,
            report_reasons: HashMap::new(),
            history: Vec::new(),
        }
    }

    /// 扣除信誉分
    pub fn deduct(&mut self, amount: u32, reason: ReportReason) {
        self.score = self.score.saturating_sub(amount);
        self.reports_received += 1;
        
        *self.report_reasons.entry(reason).or_insert(0) += 1;
        
        self.history.push(ReputationChange {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            change: -(amount as i32),
            reason,
            new_score: self.score,
        });

        if self.score < 60 {
            tracing::warn!("玩家信誉分低于60，限制组队匹配");
        }
    }

    /// 恢复信誉分
    pub fn restore(&mut self, amount: u32) {
        let old_score = self.score;
        self.score = (self.score + amount).min(100);
        
        self.history.push(ReputationChange {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            change: (self.score - old_score) as i32,
            reason: ReportReason::SystemRecovery,
            new_score: self.score,
        });
    }

    /// 是否可以组队匹配
    pub fn can_group_match(&self) -> bool {
        self.score >= 60
    }

    /// 是否被禁止匹配
    pub fn is_banned(&self) -> bool {
        self.score < 30
    }
}

impl Default for Reputation {
    fn default() -> Self {
        Self::new()
    }
}

/// 举报原因
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReportReason {
    IntentionalTeamKill,    // 故意击杀队友
    ElementGriefing,        // 恶意元素坑人（故意装备冲突元素）
    Cheating,               // 作弊
    AbusiveBehavior,        // 辱骂/不当行为
    AFK,                    // 挂机
    SystemRecovery,         // 系统恢复
}

impl ReportReason {
    pub fn description(&self) -> &'static str {
        match self {
            ReportReason::IntentionalTeamKill => "故意击杀队友",
            ReportReason::ElementGriefing => "恶意元素坑人",
            ReportReason::Cheating => "作弊",
            ReportReason::AbusiveBehavior => "辱骂/不当行为",
            ReportReason::AFK => "挂机",
            ReportReason::SystemRecovery => "系统恢复",
        }
    }
}

/// 信誉变更记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationChange {
    pub timestamp: u64,
    pub change: i32,
    pub reason: ReportReason,
    pub new_score: u32,
}

/// 玩家统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerStats {
    pub total_matches: u32,
    pub total_kills: u32,
    pub total_deaths: u32,
    pub kd_ratio: f32,
    pub successful_extractions: u32,
    pub extraction_rate: f32,
    pub total_damage_dealt: f32,
    pub total_damage_taken: f32,
    pub total_loot_value: u32,
    pub headshot_count: u32,
    pub longest_shot: f32,
    pub favorite_element: Option<String>,
    pub favorite_weapon: Option<String>,
}

/// 赛季进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonProgress {
    pub season_number: u32,
    pub battle_pass_level: u32,
    pub battle_pass_xp: u64,
    pub rank_points: u32,
    pub rank_tier: RankTier,
    pub weekly_challenges_completed: u32,
    pub season_challenges_completed: u32,
}

impl SeasonProgress {
    pub fn new(season: u32) -> Self {
        Self {
            season_number: season,
            battle_pass_level: 1,
            battle_pass_xp: 0,
            rank_points: 0,
            rank_tier: RankTier::Unranked,
            weekly_challenges_completed: 0,
            season_challenges_completed: 0,
        }
    }

    /// 增加通行证经验
    pub fn add_battle_pass_xp(&mut self, amount: u64) {
        self.battle_pass_xp += amount;
        
        // 每10000XP升一级
        let new_level = (self.battle_pass_xp / 10000) as u32 + 1;
        if new_level > self.battle_pass_level {
            self.battle_pass_level = new_level.min(100);
        }
    }

    /// 增加排位分
    pub fn add_rank_points(&mut self, amount: i32) {
        let new_points = (self.rank_points as i32 + amount).max(0) as u32;
        self.rank_points = new_points;
        self.update_rank_tier();
    }

    /// 更新排位等级
    fn update_rank_tier(&mut self) {
        self.rank_tier = match self.rank_points {
            0..=99 => RankTier::Bronze,
            100..=299 => RankTier::Silver,
            300..=599 => RankTier::Gold,
            600..=999 => RankTier::Platinum,
            1000..=1499 => RankTier::Diamond,
            1500..=1999 => RankTier::Master,
            _ => RankTier::Legend,
        };
    }
}

/// 排位等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RankTier {
    Unranked,
    Bronze,
    Silver,
    Gold,
    Platinum,
    Diamond,
    Master,
    Legend,
}

impl RankTier {
    pub fn name(&self) -> &'static str {
        match self {
            RankTier::Unranked => "未定级",
            RankTier::Bronze => "青铜",
            RankTier::Silver => "白银",
            RankTier::Gold => "黄金",
            RankTier::Platinum => "铂金",
            RankTier::Diamond => "钻石",
            RankTier::Master => "大师",
            RankTier::Legend => "传说",
        }
    }
}

/// 玩家仓库
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerInventory {
    pub max_slots: u32,
    pub used_slots: u32,
    pub equipment_ids: Vec<u64>,
    pub currency: CurrencyWallet,
}

impl PlayerInventory {
    pub fn new() -> Self {
        Self {
            max_slots: 100,
            used_slots: 0,
            equipment_ids: Vec::new(),
            currency: CurrencyWallet::new(),
        }
    }

    pub fn add_equipment(&mut self, equipment_id: u64) -> bool {
        if self.used_slots >= self.max_slots {
            return false;
        }
        self.equipment_ids.push(equipment_id);
        self.used_slots += 1;
        true
    }

    pub fn remove_equipment(&mut self, equipment_id: u64) -> bool {
        if let Some(pos) = self.equipment_ids.iter().position(|&id| id == equipment_id) {
            self.equipment_ids.remove(pos);
            self.used_slots -= 1;
            true
        } else {
            false
        }
    }
}

/// 货币钱包
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CurrencyWallet {
    pub soft_currency: u32,    // 游戏币（可通过游戏获得）
    pub hard_currency: u32,    // 点券（付费货币）
    pub season_tokens: u32,    // 赛季代币
}

impl CurrencyWallet {
    pub fn new() -> Self {
        Self {
            soft_currency: 10000,
            hard_currency: 0,
            season_tokens: 0,
        }
    }
}

/// 玩家设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSettings {
    pub sensitivity: f32,
    pub fov: f32,
    pub audio_volume: f32,
    pub voice_chat_enabled: bool,
    pub push_to_talk: bool,
    pub preferred_element: Option<String>,
    pub auto_matchmake: bool,
}

impl Default for PlayerSettings {
    fn default() -> Self {
        Self {
            sensitivity: 1.0,
            fov: 90.0,
            audio_volume: 1.0,
            voice_chat_enabled: true,
            push_to_talk: true,
            preferred_element: None,
            auto_matchmake: true,
        }
    }
}

/// 匹配偏好
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchPreference {
    pub preferred_mode: crate::gamemode::GameModeType,
    pub preferred_map: Option<String>,
    pub squad_size: u32,
    pub element_preference: Option<String>,
    pub gear_value_range: (u32, u32),
}

impl Default for MatchPreference {
    fn default() -> Self {
        Self {
            preferred_mode: crate::gamemode::GameModeType::Extraction,
            preferred_map: None,
            squad_size: 3,
            element_preference: None,
            gear_value_range: (0, 0), // 0 = 无限制
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_profile() {
        let mut profile = PlayerProfile::new(1, "TestPlayer");
        assert_eq!(profile.level, 1);
        assert_eq!(profile.experience, 0);
        
        profile.add_experience(2500);
        assert_eq!(profile.level, 2);
    }

    #[test]
    fn test_reputation_system() {
        let mut rep = Reputation::new();
        assert_eq!(rep.score, 100);
        assert!(rep.can_group_match());
        
        rep.deduct(20, ReportReason::ElementGriefing);
        assert_eq!(rep.score, 80);
        assert!(rep.can_group_match());
        
        rep.deduct(30, ReportReason::IntentionalTeamKill);
        assert_eq!(rep.score, 50);
        assert!(!rep.can_group_match());
        
        rep.restore(20);
        assert_eq!(rep.score, 70);
        assert!(rep.can_group_match());
    }

    #[test]
    fn test_season_progress() {
        let mut progress = SeasonProgress::new(1);
        assert_eq!(progress.battle_pass_level, 1);
        assert_eq!(progress.rank_tier, RankTier::Unranked);
        
        progress.add_battle_pass_xp(25000);
        assert_eq!(progress.battle_pass_level, 3);
        
        progress.add_rank_points(350);
        assert_eq!(progress.rank_tier, RankTier::Gold);
    }

    #[test]
    fn test_inventory_management() {
        let mut inv = PlayerInventory::new();
        assert_eq!(inv.max_slots, 100);
        
        assert!(inv.add_equipment(1));
        assert!(inv.add_equipment(2));
        assert_eq!(inv.used_slots, 2);
        
        assert!(inv.remove_equipment(1));
        assert_eq!(inv.used_slots, 1);
    }
}
