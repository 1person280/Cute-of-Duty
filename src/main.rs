//! Cute Of Duty 1: Simple
//! 战术撤离射击游戏 - 主入口（cod1 无头模拟）
//!
//! 基于固定Tick + Rust 的极致性能架构
//! 核心差异化：元素互斥生态 + 反护航经济架构
//!
//! 游戏逻辑全部位于库 crate（src/lib.rs），本文件仅做编排演示。

/// 3D Demo 入口：`cargo run --features demo` 时本包构建为 3D FPS Demo。
#[cfg(feature = "demo")]
fn main() {
    cute_of_duty::demo::run();
}

/// 默认（无 demo feature）：cod1 无头模拟二进制，全部逻辑在下方 headless 模块内。
#[cfg(not(feature = "demo"))]

use std::sync::Arc;
use tracing::{info, warn};
use cute_of_duty::engine::{GameLoop, TickConfig};
use cute_of_duty::element::ElementSystem;
use cute_of_duty::equipment::EquipmentSystem;
use cute_of_duty::gamemode::{MatchManager, MatchConfig, GameModeType};
use cute_of_duty::player::{PlayerProfile, ReportReason};

/// 游戏主入口
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
// 初始化日志
tracing_subscriber::fmt()
    .with_max_level(tracing::Level::INFO)
    .init();

info!("========================================");
info!("Cute Of Duty 1: Simple");
info!("战术撤离射击游戏");
info!("版本: 0.2.1 (Pre-Alpha)");
info!("========================================");

// 加载配置（核心库统一加载器：自动定位项目根目录，解析失败大声报错）
let (element_config, config_path) = cute_of_duty::config::load_element_config()?;
match &config_path {
    Some(path) => info!(
        "元素配置加载完成: {} ({} 种反应规则)",
        path.display(),
        element_config.reactions.len()
    ),
    None => warn!("未找到 config/element_reactions.yaml，使用内置默认配置"),
}

// 初始化元素系统
let element_system = Arc::new(ElementSystem::new(element_config));

// 初始化装备系统
let equipment_system = Arc::new(EquipmentSystem::new());

// 打印装备等级规则
print_equipment_rules(&equipment_system);

// 初始化游戏循环
let tick_config = TickConfig {
    tick_rate_hz: 60,
    max_frame_time_ms: 50.0,
    enable_determinism_check: true,
};

let mut game_loop = GameLoop::new(tick_config, element_system.clone(), equipment_system.clone());

info!("游戏循环初始化完成，Tick率: {}Hz", tick_config.tick_rate_hz);
info!("启动主循环...");

// 第一次运行：实时节流跑10秒（600 ticks）
game_loop.run_test_ticks(600).await?; // 运行10秒 (600 ticks)
info!("第一次运行完成，记录参考状态哈希");
game_loop.take_reference();

// 重放：不做实时节流，重新跑600 ticks，逐Tick比对状态哈希
game_loop.reset();
info!("开始确定性重放（不节流）...");
game_loop.run_replay_ticks(600).await?;
info!("重放完成");
info!("游戏循环结束");

info!("确定性验证: {}", if game_loop.verify_determinism() { "通过" } else { "失败" });

// 演示：创建一场战局
demo_match();

// 演示：玩家档案
demo_player_profile();

Ok(())
}

/// 演示：战局系统
fn demo_match() {
info!("========== 战局系统演示 ==========");

let config = MatchConfig {
    mode: GameModeType::Extraction,
    match_duration_minutes: 20,
    map_name: "废弃工业区".to_string(),
    max_players: 12,
    squad_size: 3,
    ai_threat_level: 5,
    gear_value_limit: 0,
    allow_gear_import: true,
    environment_state: "rain".to_string(),
};

let mut match_mgr = MatchManager::new(config);

// 添加玩家
match_mgr.add_player(1001, Some(1));
match_mgr.add_player(1002, Some(1));
match_mgr.add_player(1003, Some(1));
match_mgr.add_player(1004, Some(2));
match_mgr.add_player(1005, Some(2));

info!("战局创建: 5名玩家，2个小队");

match_mgr.start_match();

// 模拟战斗
match_mgr.record_damage(1001, 1004, 50.0);
match_mgr.record_damage(1004, 1001, 30.0);
match_mgr.player_death(1004, Some(1001));

// 模拟撤离
match_mgr.player_extract(1001);
match_mgr.player_extract(1002);
match_mgr.player_extract(1003);
match_mgr.player_extract(1005);

let report = match_mgr.generate_match_report();
info!("战局报告: {} kills, {} 成功撤离", report.total_kills, report.total_extracted);
info!("===================================");
}

/// 演示：玩家档案系统
fn demo_player_profile() {
info!("========== 玩家档案演示 ==========");

let mut profile = PlayerProfile::new(1001, "元素大师");
info!("创建玩家: {} (ID: {})", profile.username, profile.player_id);

// 升级
profile.add_experience(5000);
info!("获得5000经验 -> 等级: {}", profile.level);

// 添加装备到仓库
profile.inventory.add_equipment(1);
profile.inventory.add_equipment(2);
info!("仓库: {}/{}  slots", profile.inventory.used_slots, profile.inventory.max_slots);

// 信誉系统
info!("初始信誉分: {}", profile.reputation.score);
profile.reputation.deduct(15, ReportReason::ElementGriefing);
info!("被举报恶意元素 -> 信誉分: {}", profile.reputation.score);
info!("可组队匹配: {}", if profile.reputation.can_group_match() { "是" } else { "否" });

info!("===================================");
}

/// 打印装备规则说明
fn print_equipment_rules(_equipment_system: &EquipmentSystem) {
info!("========== 装备等级规则 ==========");
info!("1级: 无元素、无副作用、无组合 (新手保护舱)");
info!("2-6级: 获得时真随机元素，可付费指定 (成本翻倍)");
info!("7-9级: 获得时真随机元素，不可指定 (混沌区)");
info!("转售/给予: 元素属性重新真随机生成");
info!("===================================");
}

#[cfg(test)]
mod tests {
use cute_of_duty::element::{ElementConfig, ElementSystem, EntityElementState, ElementType, ReactionResult};
use cute_of_duty::equipment::{EquipmentSystem, EquipmentTier};

#[test]
fn test_element_reactions() {
    let config = ElementConfig::default();
    let system = ElementSystem::new(config);

    // 测试: 潮湿 + 火 = 蒸发
    let result = system.query_reaction(
        &EntityElementState::Wet,
        &ElementType::Fire,
    );
    assert!(result.is_some());
    let reaction = result.unwrap();
    assert_eq!(reaction.result, ReactionResult::Vaporize);
    assert_eq!(reaction.damage_multiplier, 1.5);

    // 测试: 冰冻 + 火 = 融化
    let result = system.query_reaction(
        &EntityElementState::Frozen,
        &ElementType::Fire,
    );
    assert!(result.is_some());
    let reaction = result.unwrap();
    assert_eq!(reaction.result, ReactionResult::Melt);
    assert_eq!(reaction.damage_multiplier, 2.0);
}

#[test]
fn test_equipment_tier_rules() {
    let system = EquipmentSystem::new();

    // 1级装备: 无元素
    let tier1 = system.get_tier_rules(EquipmentTier::Tier1);
    assert!(!tier1.has_element);
    assert!(!tier1.can_specify_element);

    // 5级装备: 有元素，可指定
    let tier5 = system.get_tier_rules(EquipmentTier::Tier5);
    assert!(tier5.has_element);
    assert!(tier5.can_specify_element);

    // 8级装备: 有元素，不可指定
    let tier8 = system.get_tier_rules(EquipmentTier::Tier8);
    assert!(tier8.has_element);
    assert!(!tier8.can_specify_element);
}

#[test]
fn test_mutual_exclusion_calculation() {
    let config = ElementConfig::default();
    let system = ElementSystem::new(config);

    // 测试队友互斥距离衰减
    // 冰甲与火枪，距离3米，最大惩罚15%
    // 线性衰减: 15% * (1 - 3/5) = 6%
    let penalty = system.calculate_teammate_penalty(
        &ElementType::Ice,
        &ElementType::Fire,
        3.0,
    );
    assert!((penalty - 0.06).abs() < 0.001);

    // 距离6米，超出范围，无惩罚
    let penalty = system.calculate_teammate_penalty(
        &ElementType::Ice,
        &ElementType::Fire,
        6.0,
    );
    assert_eq!(penalty, 0.0);
}
}
