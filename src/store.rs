//! 内存里的键值存储，用 Mutex 保护 HashMap。

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// 内存键值存储。
#[derive(Debug, Default)]
pub struct Store {
    data: Mutex<HashMap<String, String>>,
    client_count: AtomicUsize,
}

impl Store {
    pub fn new() -> Self {
        Store {
            data: Mutex::new(HashMap::new()),
            client_count: AtomicUsize::new(0),
        }
    }

    /// 写入或覆盖一个键值对，返回旧值（若有）。
    pub fn set(&self, key: &str, value: &str) -> Option<String> {
        self.data
            .lock()
            .unwrap()
            .insert(key.to_owned(), value.to_owned())
    }

    /// 查询键对应的值，不存在返回 None。
    pub fn get(&self, key: &str) -> Option<String> {
        self.data.lock().unwrap().get(key).cloned()
    }

    /// 删除键，返回被删除的值（若有）。
    pub fn del(&self, key: &str) -> Option<String> {
        self.data.lock().unwrap().remove(key)
    }

    /// 列出所有键（排序）。
    pub fn keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.data.lock().unwrap().keys().cloned().collect();
        keys.sort();
        keys
    }

    pub fn len(&self) -> usize {
        self.data.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.lock().unwrap().is_empty()
    }

    /// 克隆整张表，给持久化用。
    pub fn snapshot(&self) -> HashMap<String, String> {
        self.data.lock().unwrap().clone()
    }

    /// 整体替换当前数据。
    pub fn load(&self, data: HashMap<String, String>) {
        *self.data.lock().unwrap() = data;
    }

    pub fn client_connected(&self) {
        self.client_count.fetch_add(1, Ordering::SeqCst);
    }

    pub fn client_disconnected(&self) {
        self.client_count.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn client_count(&self) -> usize {
        self.client_count.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get() {
        let store = Store::new();
        assert_eq!(store.set("a", "1"), None);
        assert_eq!(store.get("a"), Some("1".into()));
        assert_eq!(store.get("不存在"), None);
    }

    #[test]
    fn set_overwrites_existing() {
        let store = Store::new();
        store.set("a", "1");
        let old = store.set("a", "2");
        assert_eq!(old, Some("1".into()));
        assert_eq!(store.get("a"), Some("2".into()));
    }

    #[test]
    fn del_returns_removed_value() {
        let store = Store::new();
        store.set("a", "1");
        assert_eq!(store.del("a"), Some("1".into()));
        assert_eq!(store.del("a"), None);
        assert_eq!(store.get("a"), None);
    }

    #[test]
    fn keys_are_sorted() {
        let store = Store::new();
        store.set("b", "2");
        store.set("a", "1");
        store.set("c", "3");
        assert_eq!(store.keys(), vec!["a", "b", "c"]);
        assert_eq!(store.len(), 3);
    }

    #[test]
    fn snapshot_and_load() {
        let store = Store::new();
        store.set("a", "1");
        let snap = store.snapshot();

        let other = Store::new();
        other.load(snap);
        assert_eq!(other.get("a"), Some("1".into()));
    }

    #[test]
    fn client_count_tracks_connections() {
        let store = Store::new();
        assert_eq!(store.client_count(), 0);
        store.client_connected();
        store.client_connected();
        assert_eq!(store.client_count(), 2);
        store.client_disconnected();
        assert_eq!(store.client_count(), 1);
    }
}
