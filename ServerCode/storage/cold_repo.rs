//! 冷数据仓库抽象（跨局需保存、重连要续回的数据统一入口）。
//!
//! Why: 遵循"热数据驻内存、冷数据落盘"的分层红线。战斗/位移/血量这类高频
//! 热数据绝不进仓库，只在内存 + 快照；而玩家档案（背包/货币/战绩/信誉/设置）
//! 走本 trait 持久化。引入 trait 而非直接用具体仓库，是为将来"本地文件 ↔ 远端
//! 存储/kv"按配置切换留出唯一接缝——业务侧只依赖本 trait，不感知存储介质。

use crate::player::PlayerProfile;

use super::error::StorageError;

/// 冷数据仓库：以玩家 ID 为键读写全量 `PlayerProfile`。
///
/// 语义约定（Why）：`save` 持**全量覆盖写**语义（append-only 仓库内部自动追加
/// 新完整记录），满足"崩溃后重放至最后一致点 + 至少一次持久"。调用方无需关心
/// 增量 diff；对每条完整记录追加写是最简单且不丢档的兜底。
///
/// 线程约定：本 trait 仅供服务端单一权威主循环串行调用，实现无需自身加锁；
/// 若未来多线程并行落盘，应在实现内部自行同步，而不抬高业务侧心智负担。
pub trait ColdRepo {
    /// 读取玩家档案，不存在时返回 `Ok(None)`（由调用方决定新建默认档案）。
    fn load(&self, player_id: u64) -> Result<Option<PlayerProfile>, StorageError>;

    /// 持久化（覆盖语义）玩家档案。
    fn save(&mut self, player_id: u64, profile: &PlayerProfile) -> Result<(), StorageError>;
}