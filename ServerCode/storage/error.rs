//! 冷数据存储错误类型
//!
//! Why: 冷数据仓库（`ColdRepo` 实现）与业务解耦，错误统一收口到本类型，
//! 调用方（服务端权威主循环 / 测试）只需匹配 `StorageError` 即可区别
//! "读写失败" 与 "存档损坏"。区分与否决定回滚策略：疑似损坏 → 大声失败避免静默丢档；
//! 仅读写失败 → 走"至少一次"重试/落盘兜底。

use std::fmt;
use std::path::PathBuf;

/// 冷数据存储错误。
///
/// - 文件读写系统错误（含目录创建失败）；
/// - 存档记录损坏（中间某条完整记录反序列化失败，属真实数据损坏）；
/// - 序列化失败（一般不应发生，收口以防 panic）。
#[derive(Debug)]
pub enum StorageError {
    /// 文件系统错误，附带出错路径。
    Io { path: PathBuf, source: std::io::Error },
    /// 存档中某条**完整**记录反序列化失败（真实损坏，非尾部半条）。
    CorruptRecord { path: PathBuf, line: usize, source: serde_json::Error },
    /// 待写记录序列化失败。
    Encode { source: serde_json::Error },
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::Io { path, source } => {
                write!(f, "冷数据读写失败 {}: {source}", path.display())
            }
            StorageError::CorruptRecord { path, line, source } => {
                write!(f, "冷数据损坏 {} (第 {line} 条): {source}", path.display())
            }
            StorageError::Encode { source } => write!(f, "冷记录序列化失败: {source}"),
        }
    }
}

impl std::error::Error for StorageError {}