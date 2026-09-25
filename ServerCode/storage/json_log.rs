//! JSON 追加日志冷数据仓库（默认 `ColdRepo` 实现）
//!
//! Why: "零第三方依赖 + 符合底层冻结红线 + 磁盘友好"是选型硬约束。追加式
//! JSON 行日志（每行一条完整 `PlayerProfile`，文件以 `\n` 结尾）在崩溃时天然
//! 满足"至少一次 + 重放到最后一致点"：最后一次完整记录即权威状态，末尾半条
//! 尾写忽略。相比 DB/Redis 不引入任何重依赖，也无需编译缓存，契合磁盘空间限制。

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use crate::player::PlayerProfile;

use super::cold_repo::ColdRepo;
use super::error::StorageError;

/// 每个玩家一条日志文件的子目录名（置于仓库根目录下）。
const PROFILES_SUBDIR: &str = "profiles";

/// 追加式 JSON 日志仓库。
///
/// 文件布局：`{dir}/profiles/{player_id}.jsonl`，内容为逐行 JSON 的
/// `PlayerProfile`。写入采用追加 + `sync_all`（至少一次落盘）；读取解析每条
/// 完整记录并取最后一条有效值，若中间出现损坏完整记录则大声报错（防静默丢档）。
pub struct JsonLogRepo {
    dir: PathBuf,
}

impl JsonLogRepo {
    /// 打开（必要时创建）仓库根目录。
    ///
    /// 目录不存在时递归创建；创建失败视为系统错误返回 `Err`。
    pub fn open(dir: impl Into<PathBuf>) -> Result<Self, StorageError> {
        let dir = dir.into();
        fs::create_dir_all(&dir).map_err(|source| StorageError::Io {
            path: dir.clone(),
            source,
        })?;
        Ok(Self { dir })
    }

    /// 定位某玩家日志文件的绝对路径（不会触碰文件系统）。
    fn profile_path(&self, player_id: u64) -> PathBuf {
        self.dir
            .join(PROFILES_SUBDIR)
            .join(format!("{player_id}.jsonl"))
    }

    /// 追加写入一条记录：追加完整 JSON 行 + 空行 + 落盘。
    ///
    /// 语义（Why）：每次 `save` 追加一条**完整**肖像快照，旧记录留作审计/回滚，
    /// 不覆盖既不回滚。追加成功后 `sync_all` 保证至少一次持久（崩溃不丢本次变更）。
    fn append(&self, player_id: u64, profile: &PlayerProfile) -> Result<(), StorageError> {
        let path = self.profile_path(player_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| StorageError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let mut line = serde_json::to_string(profile).map_err(|source| StorageError::Encode {
            source,
        })?;
        line.push('\n');
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|source| StorageError::Io {
                path: path.clone(),
                source,
            })?;
        file.write_all(line.as_bytes())
            .and_then(|_| file.sync_all())
            .map_err(|source| StorageError::Io {
                path: path.clone(),
                source,
            })
    }

    /// 读出最后一条有效完整记录。
    ///
    /// 读规则（Why）：逐行解析，**每一条以 `\n` 结尾的完整记录**都必须可解析，
    /// 否则判定为中途损坏并发 `Err`（大声失败，防静默丢档）；文件末尾**没有**
    /// 换行符的残留片段视为崩溃半条尾写，予以忽略（至少一次语义允许）。取最后
    /// 一条完整有效记录为该玩家当前权威状态。
    fn read_last(&self, player_id: u64) -> Result<Option<PlayerProfile>, StorageError> {
        let path = self.profile_path(player_id);
        let file = match File::open(&path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(StorageError::Io {
                    path: path.clone(),
                    source,
                })
            }
        };

        let mut reader = BufReader::new(file);
        let mut last: Option<PlayerProfile> = None;
        let mut line_no = 0usize;
        loop {
            let mut raw = String::new();
            // read_line 返回字节数；0 表示 EOF。它会把行尾 \n 吞掉。
            let n = reader
                .read_line(&mut raw)
                .map_err(|source| StorageError::Io {
                    path: path.clone(),
                    source,
                })?;
            if n == 0 {
                break;
            }
            line_no += 1;
            let is_last_byte_newline = raw.ends_with('\n');
            // 去掉行尾换行符后再判断是否完整记录
            let trimmed = raw.trim_end_matches('\n');
            if trimmed.is_empty() {
                continue;
            }
            // 末尾残留半条（无换行结尾）仅当它是最后一行时才允许忽略；
            // 若一行无换行且前面还有记录，我们也把它视作崩溃尾写忽略。
            let parsed = serde_json::from_str::<PlayerProfile>(trimmed);
            match parsed {
                Ok(profile) => last = Some(profile),
                Err(source) => {
                    if is_last_byte_newline {
                        // 完整记录却解析失败 → 真实损坏
                        return Err(StorageError::CorruptRecord {
                            path: path.clone(),
                            line: line_no,
                            source,
                        });
                    }
                    // 非换行结尾 → 崩溃半条尾写，忽略
                    break;
                }
            }
        }
        Ok(last)
    }
}

impl ColdRepo for JsonLogRepo {
    fn load(&self, player_id: u64) -> Result<Option<PlayerProfile>, StorageError> {
        self.read_last(player_id)
    }

    fn save(&mut self, player_id: u64, profile: &PlayerProfile) -> Result<(), StorageError> {
        self.append(player_id, profile)
    }
}

impl JsonLogRepo {
    /// 测试用：仓库根目录是否已创建（验证 open 的副作用）。
    #[cfg(test)]
    fn opened_dir(&self) -> &PathBuf {
        &self.dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("cod_cold_repo_test")
            .join(tag)
            .join(format!("{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn profile(id: u64, name: &str, level: u32) -> PlayerProfile {
        let mut p = PlayerProfile::new(id, name);
        p.level = level;
        p
    }

    #[test]
    fn load_missing_returns_none() {
        let dir = temp_dir("missing");
        let repo = JsonLogRepo::open(&dir).unwrap();
        let p = repo.load(42).unwrap();
        assert!(p.is_none());
    }

    #[test]
    fn save_then_load_roundtrip() {
        let dir = temp_dir("roundtrip");
        let mut repo = JsonLogRepo::open(&dir).unwrap();
        let p = profile(7, "Alice", 3);

        repo.save(7, &p).unwrap();
        // 再次落盘，验证追加语义取最后一条
        let p2 = profile(7, "Alice", 5);
        repo.save(7, &p2).unwrap();

        let loaded = repo.load(7).unwrap().expect("应读到档案");
        assert_eq!(loaded.level, 5, "追加日志应取最后一条完整记录");
        assert_eq!(loaded.username, "Alice");
    }

    #[test]
    fn open_creates_root_dir() {
        let dir = temp_dir("create_root");
        assert!(!dir.exists());
        let repo = JsonLogRepo::open(&dir).unwrap();
        assert!(repo.opened_dir().exists());
    }

    #[test]
    fn trailing_partial_line_is_ignored() {
        let dir = temp_dir("partial");
        let mut repo = JsonLogRepo::open(&dir).unwrap();
        let p = profile(1, "Bob", 2);
        repo.save(1, &p).unwrap();

        // 模拟崩溃半条尾写：手动追加一段无换行的残缺 JSON
        let path = repo.profile_path(1);
        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(b"{\"player_id\":1,\"username\":\"Bob\"").unwrap();
        drop(file);

        let loaded = repo.load(1).unwrap().expect("应仍读到完整记录");
        assert_eq!(loaded.level, 2, "尾部半条损坏不应影响最后完整记录");
    }

    #[test]
    fn mid_file_corruption_is_loud() {
        let dir = temp_dir("corrupt");
        let mut repo = JsonLogRepo::open(&dir).unwrap();
        repo.save(1, &profile(1, "Carol", 1)).unwrap();

        // 在中间插入一条完整但无法解析的记录（含换行）→ 应判定真实损坏
        let path = repo.profile_path(1);
        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(b"this is not json\n").unwrap();
        drop(file);

        let result = repo.load(1);
        assert!(
            matches!(result, Err(StorageError::CorruptRecord { .. })),
            "中间完整记录损坏应大声报错，而非静默丢档"
        );
    }

    #[test]
    fn per_player_files_isolated() {
        let dir = temp_dir("isolation");
        let mut repo = JsonLogRepo::open(&dir).unwrap();
        repo.save(1, &profile(1, "A", 1)).unwrap();
        repo.save(2, &profile(2, "B", 9)).unwrap();

        assert_eq!(repo.load(1).unwrap().unwrap().username, "A");
        assert_eq!(repo.load(2).unwrap().unwrap().username, "B");
        assert!(repo.load(3).unwrap().is_none());
    }
}