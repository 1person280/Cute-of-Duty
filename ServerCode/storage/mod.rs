//! 冷数据存储层（冷数据：跨局需保存、重连要续回的数据，如玩家档案/背包/信誉）。
//!
//! 分层红线（Why）：热数据（坐标/血量/CD/手感）驻内存 + 快照，**绝不落盘**；
//! 只有本层承接的冷数据才写持久存储。`ColdRepo` trait 是唯一接缝，业务侧不感知
//! 存储介质；默认实现 `JsonLogRepo` 用 append-only JSON 日志文件，零第三方依赖、
//! 契合底层冻结红线与磁盘空间限制，崩溃后"重放到最后一致点 + 至少一次持久"。
//!
//! 目录解析（Why）：仓库根目录**配置驱动**——优先读环境变量 `COD_DATA_DIR`，
//! 否则回退到 workspace 根的 `ServerCode/data/profiles`（复用 `config::project_root`
//! 逐级向上定位）。既满足"配置驱动动态切换"，又不引入配置文件类型负担。

use std::path::PathBuf;

mod cold_repo;
pub mod error;
mod json_log;

pub use cold_repo::ColdRepo;
pub use error::StorageError;
pub use json_log::JsonLogRepo;

/// 仓库根目录相对 workspace 根的路径（配置驱动默认值）。
const DATA_DIR_REL: &str = "ServerCode/data/profiles";
/// 环境变量覆盖：设置 `COD_DATA_DIR` 即可把冷数据迁到任意磁盘/目录。
const ENV_DATA_DIR: &str = "COD_DATA_DIR";

/// 解析冷数据仓库根目录。
///
/// 优先级：`COD_DATA_DIR` 环境变量 > workspace 根下的 `ServerCode/data/profiles`。
/// 解析失败（无法定位 workspace 根且未设环境变量）返回 `None`，由调用方决定兜底
/// （如告警并禁用持久化，而非静默写错目录）。
pub fn resolve_data_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var(ENV_DATA_DIR) {
        if !dir.trim().is_empty() {
            return Some(PathBuf::from(dir));
        }
    }
    crate::config::project_root().map(|root| root.join(DATA_DIR_REL))
}

/// 打开仓库的便捷入口：解析目录并 `JsonLogRepo::open`。
pub fn open_repo() -> Result<JsonLogRepo, StorageError> {
    // 解析失败时回退到当前目录下的 `data/profiles`，保证服务端总能启动，
    // 并让告警归调用方（主循环）负责。
    let dir = resolve_data_dir()
        .unwrap_or_else(|| PathBuf::from("data").join("profiles"));
    JsonLogRepo::open(dir)
}