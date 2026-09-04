//! 数据持久化：把键值表存成 JSON 文件，启动时读回。

use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::error::{KvError, Result};

/// 负责把键值表保存到磁盘、并在启动时恢复。
#[derive(Debug, Clone)]
pub struct Persistence {
    path: PathBuf,
}

impl Persistence {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Persistence { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 把整张表写入磁盘。先写临时文件再重命名，避免写一半损坏。
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

    /// 读回整张表。文件不存在返回空表，内容非法返回错误。
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

    fn temp_path(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("kvstore_test_{}_{}.json", tag, nanos))
    }

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

    #[test]
    fn load_missing_file_returns_empty() {
        let path = temp_path("missing");
        let p = Persistence::new(path);
        let loaded = p.load().unwrap();
        assert!(loaded.is_empty());
    }

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
