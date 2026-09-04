//! 数据持久化与重启恢复模块。
//!
//! 采用「JSON 快照」方案：每次数据发生变化后，把整张键值表序列化成 JSON
//! 写入磁盘文件；服务器启动时读回该文件恢复数据。数据量不大时这种方案
//! 最简单、最可靠，也便于人工检查数据文件内容。
//!
//! 关键设计点（一一对应课设的验收要求）：
//!
//! - **首次启动**：数据文件不存在 → 自动创建目录与文件，以空数据库运行；
//! - **数据文件存在**：启动时读回内容，恢复已有数据；
//! - **数据文件损坏**：抛出 [`crate::error::KvError::CorruptedData`]，
//!   给出明确错误提示，**绝不静默清空数据**。

use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::error::{KvError, Result};

/// 负责把内存中的键值表保存到磁盘、并在启动时恢复。
///
/// 实现了 `Clone`（内部只有一条路径，克隆很廉价），
/// 以便服务器为每个客户端线程各持有一份共享引用。
#[derive(Debug, Clone)]
pub struct Persistence {
    /// 数据文件的完整路径（如 `data/kv.json`）。
    path: PathBuf,
}

impl Persistence {
    /// 根据给定的数据文件路径创建持久化实例。
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Persistence { path: path.into() }
    }

    /// 返回数据文件的路径（供状态展示使用）。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 把整张表写入磁盘（覆盖写）。
    ///
    /// 推荐采用「先写临时文件、再原子重命名」的两步方式，避免写盘过程中
    /// 进程崩溃导致数据文件只写了一半而损坏。
    ///
    /// 提示：`serde_json::to_string_pretty` 序列化；先 `fs::create_dir_all`
    /// 确保父目录存在；写临时文件（如 `with_extension("json.tmp")`）后
    /// `fs::rename` 成目标文件。实现时需 `use std::fs;`。
    ///
    /// 【待实现】
    pub fn save(&self, data: &HashMap<String, String>) -> Result<()> {
        let json = serde_json::to_string_pretty(data)?;

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let temp_path = self.path.with_extension("json.tmp");
        fs::write(&temp_path, json)?;
        fs::rename(temp_path, &self.path)?;
        Ok(())
    }

    /// 从磁盘读回整张表。
    ///
    /// 返回值语义：
    /// - 文件不存在 → 返回空表（首次启动场景，不视为错误）；
    /// - 文件存在但内容非法 → 返回 `CorruptedData` 错误，**绝不静默清空**；
    /// - 文件存在且内容合法 → 返回恢复出的数据。
    ///
    /// 提示：`fs::read_to_string` 读文件；`serde_json::from_str` 解析，
    /// 解析失败用 `map_err` 转成 `KvError::CorruptedData`。
    ///
    /// 【待实现】
    pub fn load(&self) -> Result<HashMap<String, String>> {
        let contents = match fs::read_to_string(&self.path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(HashMap::new()),
            Err(error) => return Err(error.into()),
        };

        if contents.trim().is_empty() {
            return Ok(HashMap::new());
        }

        serde_json::from_str(&contents).map_err(|error| KvError::CorruptedData(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::KvError;
    use std::fs;

    /// 每个测试使用独立的临时文件路径，避免互相干扰。
    fn temp_path(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("kvstore_test_{}_{}.json", tag, nanos))
    }

    /// 保存后能完整读回（往返一致）。
    #[test]
    fn save_and_load_roundtrip() {
        let path = temp_path("roundtrip");
        let p = Persistence::new(path.clone());

        let mut data = HashMap::new();
        data.insert("课程名称".to_string(), "Rust程序设计".to_string());
        data.insert("学分".to_string(), "3".to_string());
        p.save(&data).unwrap();

        let loaded = p.load().unwrap();
        assert_eq!(loaded, data);

        let _ = fs::remove_file(&path);
    }

    /// 文件不存在时返回空表（首次启动场景）。
    #[test]
    fn load_missing_file_returns_empty() {
        let path = temp_path("missing");
        let p = Persistence::new(path);
        let loaded = p.load().unwrap();
        assert!(loaded.is_empty());
    }

    /// 文件内容非法时必须报错，而不是返回空表（拒绝静默清空）。
    #[test]
    fn load_corrupted_file_errors() {
        let path = temp_path("corrupted");
        fs::write(&path, "这不是合法的 JSON {{{").unwrap();

        let p = Persistence::new(path.clone());
        let result = p.load();
        assert!(result.is_err());
        match result {
            Err(KvError::CorruptedData(_)) => {}
            other => panic!("应返回 CorruptedData 错误，实际是 {:?}", other),
        }

        let _ = fs::remove_file(&path);
    }

    /// 空文件应视为空数据库，不报损坏。
    #[test]
    fn load_empty_file_returns_empty() {
        let path = temp_path("empty");
        fs::write(&path, "").unwrap();

        let p = Persistence::new(path.clone());
        let loaded = p.load().unwrap();
        assert!(loaded.is_empty());

        let _ = fs::remove_file(&path);
    }
}
