//! 运行时键值存储模块。
//!
//! 这是整个系统的「内存态」：一个从字符串键到字符串值的映射。
//! 为了支持多客户端并发访问，内部用 [`std::sync::Mutex`] 保护
//! [`HashMap`]，所有读写都必须先获取锁，从而保证同一时刻只有一个
//! 线程在修改数据（对应课设要求的「并发安全」）。
//!
//! 这里选择 `Mutex` 而非 `RwLock` 的原因是：键值存储读写都很频繁、
//! 数据量通常不大，`Mutex` 的简单性与可预测性更值得优先；过早引入
//! `RwLock` 属于过度设计。

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// 内存中的键值存储。
///
/// 字段：
/// - `data`：实际的键值映射，被 `Mutex` 包裹以保证并发安全；
/// - `client_count`：当前活跃连接数，用原子类型保证线程安全地增减。
///
/// 注意：对原子计数做增减/读取时，需要 `use std::sync::atomic::Ordering;`，
/// 并在调用处传 `Ordering::SeqCst`。
#[derive(Debug, Default)]
pub struct Store {
    data: Mutex<HashMap<String, String>>,
    client_count: AtomicUsize,
}

impl Store {
    /// 新建一个空的存储实例。
    pub fn new() -> Self {
        Store {
            data: Mutex::new(HashMap::new()),
            client_count: AtomicUsize::new(0),
        }
    }

    /// 写入或覆盖一个键值对，返回被覆盖的旧值（若该键原本已存在）。
    ///
    /// 提示：`data.lock().unwrap()` 拿到 `MutexGuard`，再调用 `HashMap::insert`。
    /// 【待实现】
    pub fn set(&self, key: &str, value: &str) -> Option<String> {
        self.data
            .lock()
            .unwrap()
            .insert(key.to_owned(), value.to_owned())
    }

    /// 查询某个键对应的值，不存在时返回 `None`。
    /// 【待实现】
    pub fn get(&self, key: &str) -> Option<String> {
        self.data.lock().unwrap().get(key).cloned()
    }

    /// 删除某个键，返回被删除的值（若存在）。
    /// 【待实现】
    pub fn del(&self, key: &str) -> Option<String> {
        self.data.lock().unwrap().remove(key)
    }

    /// 列出所有键（排序后返回，保证输出稳定、便于测试与演示）。
    /// 【待实现】
    pub fn keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.data.lock().unwrap().keys().cloned().collect();
        keys.sort();
        keys
    }

    /// 返回当前键值对数量。
    /// 【待实现】
    pub fn len(&self) -> usize {
        self.data.lock().unwrap().len()
    }

    /// 返回当前是否为空。
    /// 【待实现】
    pub fn is_empty(&self) -> bool {
        self.data.lock().unwrap().is_empty()
    }

    /// 取出整张表的快照（克隆一份），供持久化模块写盘使用。
    ///
    /// 克隆是有意的：避免在持锁期间做磁盘 I/O，缩短锁的持有时间。
    /// 【待实现】
    pub fn snapshot(&self) -> HashMap<String, String> {
        self.data.lock().unwrap().clone()
    }

    /// 用整张表的内容整体替换当前数据（供启动恢复时批量载入使用）。
    /// 【待实现】
    pub fn load(&self, data: HashMap<String, String>) {
        *self.data.lock().unwrap() = data;
    }

    /// 客户端连接数加一。
    /// 【待实现】
    pub fn client_connected(&self) {
        self.client_count.fetch_add(1, Ordering::SeqCst);
    }

    /// 客户端连接数减一。
    /// 【待实现】
    pub fn client_disconnected(&self) {
        self.client_count.fetch_sub(1, Ordering::SeqCst);
    }

    /// 返回当前活跃连接数。
    /// 【待实现】
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
        assert_eq!(store.set("a", "1"), None); // 新键，无旧值
        assert_eq!(store.get("a"), Some("1".into()));
        assert_eq!(store.get("不存在"), None);
    }

    #[test]
    fn set_overwrites_existing() {
        let store = Store::new();
        store.set("a", "1");
        let old = store.set("a", "2");
        assert_eq!(old, Some("1".into())); // 返回被覆盖的旧值
        assert_eq!(store.get("a"),Some("2".into()));
    }

    #[test]
    fn del_returns_removed_value() {
        let store = Store::new();
        store.set("a", "1");
        assert_eq!(store.del("a"), Some("1".into()));
        assert_eq!(store.del("a"), None); // 再删就不存在了
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
