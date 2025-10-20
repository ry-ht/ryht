use super::{Snapshot, Storage, WriteOp};
use anyhow::Result;
use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::sync::Arc;

/// In-memory storage implementation for testing and development
///
/// This storage backend uses a HashMap with RwLock for thread-safe access.
/// It's useful as a fallback when RocksDB has issues (e.g., on macOS) or for testing.
pub struct MemoryStorage {
    data: Arc<RwLock<BTreeMap<Vec<u8>, Vec<u8>>>>,
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStorage {
    /// Create a new in-memory storage
    pub fn new() -> Self {
        tracing::info!("Creating in-memory storage (non-persistent)");
        Self {
            data: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    /// Get the number of keys in storage
    pub fn len(&self) -> usize {
        self.data.read().len()
    }

    /// Check if storage is empty
    pub fn is_empty(&self) -> bool {
        self.data.read().is_empty()
    }

    /// Clear all data
    pub fn clear(&self) {
        self.data.write().clear();
    }
}

#[async_trait]
impl Storage for MemoryStorage {
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(self.data.read().get(key).cloned())
    }

    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.data.write().insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    async fn delete(&self, key: &[u8]) -> Result<()> {
        self.data.write().remove(key);
        Ok(())
    }

    async fn exists(&self, key: &[u8]) -> Result<bool> {
        Ok(self.data.read().contains_key(key))
    }

    async fn get_keys_with_prefix(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>> {
        let data = self.data.read();
        let keys: Vec<Vec<u8>> = data
            .range(prefix.to_vec()..)
            .take_while(|(k, _)| k.starts_with(prefix))
            .map(|(k, _)| k.clone())
            .collect();
        Ok(keys)
    }

    async fn batch_write(&self, operations: Vec<WriteOp>) -> Result<()> {
        let mut data = self.data.write();

        for op in operations {
            match op {
                WriteOp::Put { key, value } => {
                    data.insert(key, value);
                }
                WriteOp::Delete { key } => {
                    data.remove(&key);
                }
            }
        }

        Ok(())
    }

    async fn snapshot(&self) -> Result<Box<dyn Snapshot>> {
        // Clone the entire data structure for the snapshot
        let snapshot_data = self.data.read().clone();

        Ok(Box::new(MemorySnapshot {
            data: Arc::new(snapshot_data),
        }))
    }
}

/// Memory snapshot - immutable view of the data at a point in time
struct MemorySnapshot {
    data: Arc<BTreeMap<Vec<u8>, Vec<u8>>>,
}

#[async_trait]
impl Snapshot for MemorySnapshot {
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(self.data.get(key).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_operations() {
        let storage = MemoryStorage::new();

        // Test put and get
        storage.put(b"key1", b"value1").await.unwrap();
        let value = storage.get(b"key1").await.unwrap();
        assert_eq!(value, Some(b"value1".to_vec()));

        // Test exists
        assert!(storage.exists(b"key1").await.unwrap());
        assert!(!storage.exists(b"key2").await.unwrap());

        // Test delete
        storage.delete(b"key1").await.unwrap();
        assert!(!storage.exists(b"key1").await.unwrap());
    }

    #[tokio::test]
    async fn test_prefix_query() {
        let storage = MemoryStorage::new();

        storage.put(b"prefix:key1", b"value1").await.unwrap();
        storage.put(b"prefix:key2", b"value2").await.unwrap();
        storage.put(b"other:key3", b"value3").await.unwrap();

        let keys = storage.get_keys_with_prefix(b"prefix:").await.unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&b"prefix:key1".to_vec()));
        assert!(keys.contains(&b"prefix:key2".to_vec()));
    }

    #[tokio::test]
    async fn test_batch_write() {
        let storage = MemoryStorage::new();

        let ops = vec![
            WriteOp::Put {
                key: b"key1".to_vec(),
                value: b"value1".to_vec(),
            },
            WriteOp::Put {
                key: b"key2".to_vec(),
                value: b"value2".to_vec(),
            },
            WriteOp::Delete {
                key: b"key1".to_vec(),
            },
        ];

        storage.batch_write(ops).await.unwrap();

        assert!(!storage.exists(b"key1").await.unwrap());
        assert!(storage.exists(b"key2").await.unwrap());
    }

    #[tokio::test]
    async fn test_snapshot() {
        let storage = MemoryStorage::new();

        storage.put(b"key1", b"value1").await.unwrap();

        let snapshot = storage.snapshot().await.unwrap();

        // Modify storage after snapshot
        storage.put(b"key1", b"value2").await.unwrap();

        // Snapshot should still have old value
        let snap_value = snapshot.get(b"key1").await.unwrap();
        assert_eq!(snap_value, Some(b"value1".to_vec()));

        // Storage should have new value
        let storage_value = storage.get(b"key1").await.unwrap();
        assert_eq!(storage_value, Some(b"value2".to_vec()));
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let storage = Arc::new(MemoryStorage::new());

        let mut handles = vec![];

        for i in 0..10 {
            let storage_clone = Arc::clone(&storage);
            let handle = tokio::spawn(async move {
                let key = format!("key{}", i);
                let value = format!("value{}", i);
                storage_clone.put(key.as_bytes(), value.as_bytes()).await.unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(storage.len(), 10);
    }
}
