use super::{Snapshot, Storage, WriteOp};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::Surreal;

/// SurrealDB storage implementation with graph capabilities and query optimization
pub struct SurrealDBStorage {
    db: Arc<Surreal<Db>>,
    namespace: String,
    database: String,
}


impl Drop for SurrealDBStorage {
    fn drop(&mut self) {
        tracing::debug!("SurrealDBStorage drop called");
    }
}

/// Configuration for SurrealDB storage
#[derive(Debug, Clone)]
pub struct SurrealDBConfig {
    /// Namespace to use (default: "meridian")
    pub namespace: String,
    /// Database name (default: "knowledge")
    pub database: String,
}

impl Default for SurrealDBConfig {
    fn default() -> Self {
        Self {
            namespace: "meridian".to_string(),
            database: "knowledge".to_string(),
        }
    }
}

/// Key-value record for SurrealDB storage
#[derive(Debug, Serialize, Deserialize)]
struct KVRecord {
    key: String,
    // Store value as base64-encoded string to avoid serialization issues
    #[serde(with = "base64_bytes")]
    value: Vec<u8>,
}

/// Helper module for base64 encoding/decoding of bytes
mod base64_bytes {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        serializer.serialize_str(&encoded)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(serde::de::Error::custom)
    }
}

impl SurrealDBStorage {
    /// Create a new SurrealDB storage with default configuration
    pub async fn new(path: &Path) -> Result<Self> {
        Self::new_with_config(path, SurrealDBConfig::default()).await
    }

    /// Create a new SurrealDB storage with custom configuration
    pub async fn new_with_config(path: &Path, config: SurrealDBConfig) -> Result<Self> {
        tracing::info!(
            path = ?path,
            namespace = %config.namespace,
            database = %config.database,
            "Initializing SurrealDB storage"
        );

        // Connect to embedded SurrealDB with RocksDB backend
        let db = Surreal::new::<RocksDb>(path)
            .await
            .context("Failed to initialize SurrealDB")?;

        // Use namespace and database
        db.use_ns(&config.namespace)
            .use_db(&config.database)
            .await
            .context("Failed to set namespace and database")?;

        let storage = Self {
            db: Arc::new(db),
            namespace: config.namespace.clone(),
            database: config.database.clone(),
        };

        // Initialize schema
        storage.initialize_schema().await?;

        tracing::info!("SurrealDB storage initialized successfully");

        Ok(storage)
    }

    /// Initialize the database schema
    async fn initialize_schema(&self) -> Result<()> {
        tracing::debug!("Initializing SurrealDB schema");

        // Define the kv_store table for basic key-value operations
        // Note: value is stored as string (base64-encoded) to avoid serialization issues
        let query = r#"
            DEFINE TABLE IF NOT EXISTS kv_store SCHEMAFULL;
            DEFINE FIELD IF NOT EXISTS key ON TABLE kv_store TYPE string;
            DEFINE FIELD IF NOT EXISTS value ON TABLE kv_store TYPE string;
            DEFINE INDEX IF NOT EXISTS idx_key ON TABLE kv_store COLUMNS key UNIQUE;
        "#;

        self.db
            .query(query)
            .await
            .context("Failed to initialize schema")?;

        tracing::debug!("Schema initialized successfully");

        Ok(())
    }

    /// Convert bytes key to string ID
    fn key_to_id(key: &[u8]) -> String {
        // Use base64 encoding to safely represent binary keys as strings
        use base64::Engine;
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(key)
    }

    /// Get the database instance for advanced operations
    pub fn db(&self) -> Arc<Surreal<Db>> {
        self.db.clone()
    }

    /// Get namespace name
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Get database name
    pub fn database(&self) -> &str {
        &self.database
    }
}

#[async_trait]
impl Storage for SurrealDBStorage {
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let id = Self::key_to_id(key);
        let table = "kv_store";

        let result: Option<KVRecord> = self
            .db
            .select((table, &id))
            .await
            .context("Failed to get value from SurrealDB")?;

        Ok(result.map(|record| record.value))
    }

    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        let id = Self::key_to_id(key);
        let table = "kv_store";

        let record = KVRecord {
            key: id.clone(),
            value: value.to_vec(),
        };

        // Use UPSERT pattern: DELETE then INSERT to ensure record exists
        // This avoids Thing deserialization issues while ensuring record is created
        let query = format!(
            "DELETE {}:`{}`; CREATE {}:`{}` CONTENT $record",
            table, id, table, id
        );
        let _ = self.db
            .query(query)
            .bind(("record", record))
            .await
            .context("Failed to put value to SurrealDB")?;

        Ok(())
    }

    async fn delete(&self, key: &[u8]) -> Result<()> {
        let id = Self::key_to_id(key);
        let table = "kv_store";

        let _: Option<KVRecord> = self
            .db
            .delete((table, id))
            .await
            .context("Failed to delete value from SurrealDB")?;

        Ok(())
    }

    async fn exists(&self, key: &[u8]) -> Result<bool> {
        Ok(self.get(key).await?.is_some())
    }

    async fn get_keys_with_prefix(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>> {
        let prefix_str = Self::key_to_id(prefix);

        // Query all records where key starts with prefix
        let query = format!(
            "SELECT * FROM kv_store WHERE key >= '{}' AND key < '{}'",
            prefix_str,
            next_prefix(&prefix_str)
        );

        let mut response = self
            .db
            .query(&query)
            .await
            .context("Failed to query keys with prefix")?;

        #[derive(Deserialize)]
        struct KeyOnly {
            key: String,
        }

        let results: Vec<KeyOnly> = response
            .take(0)
            .unwrap_or_default();

        // Convert base64 keys back to bytes
        use base64::Engine;
        let keys: Result<Vec<Vec<u8>>> = results
            .into_iter()
            .map(|record| {
                base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .decode(&record.key)
                    .context("Failed to decode key from base64")
            })
            .collect();

        keys
    }

    async fn batch_write(&self, operations: Vec<WriteOp>) -> Result<()> {
        // Optimize batch writes using SurrealDB transactions
        // This provides atomicity and better performance than individual operations
        if operations.is_empty() {
            return Ok(());
        }

        // Build a single transaction query for all operations
        let mut query_parts = vec!["BEGIN TRANSACTION;".to_string()];

        for op in &operations {
            match op {
                WriteOp::Put { key, value } => {
                    let id = Self::key_to_id(key);
                    use base64::Engine;
                    let value_b64 = base64::engine::general_purpose::STANDARD.encode(value);
                    query_parts.push(format!(
                        "CREATE kv_store:{} CONTENT {{ key: '{}', value: '{}' }};",
                        id, id, value_b64
                    ));
                }
                WriteOp::Delete { key } => {
                    let id = Self::key_to_id(key);
                    query_parts.push(format!("DELETE kv_store:{};", id));
                }
            }
        }

        query_parts.push("COMMIT TRANSACTION;".to_string());
        let transaction_query = query_parts.join("\n");

        // Execute entire transaction in one DB call
        self.db
            .query(transaction_query)
            .await
            .context("Failed to execute batch write transaction")?;

        tracing::debug!(
            operation_count = operations.len(),
            "Batch write completed in single transaction"
        );

        Ok(())
    }

    async fn snapshot(&self) -> Result<Box<dyn Snapshot>> {
        Ok(Box::new(SurrealDBSnapshot {
            db: self.db.clone(),
        }))
    }
}

/// SurrealDB snapshot implementation
struct SurrealDBSnapshot {
    db: Arc<Surreal<Db>>,
}

#[async_trait]
impl Snapshot for SurrealDBSnapshot {
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let id = SurrealDBStorage::key_to_id(key);
        let table = "kv_store";

        let result: Option<KVRecord> = self
            .db
            .select((table, &id))
            .await
            .context("Failed to get value from snapshot")?;

        Ok(result.map(|record| record.value))
    }
}

/// Helper function to calculate the next prefix for range queries
fn next_prefix(prefix: &str) -> String {
    let mut next = prefix.to_string();
    if let Some(last_char) = next.pop() {
        if let Some(incremented) = char::from_u32(last_char as u32 + 1) {
            next.push(incremented);
        } else {
            // If we can't increment, add a high Unicode character
            next.push(last_char);
            next.push('\u{FFFF}');
        }
    } else {
        // Empty prefix - return a high value
        next.push('\u{FFFF}');
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_surrealdb_basic_operations() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SurrealDBStorage::new(temp_dir.path()).await.unwrap();

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
    async fn test_surrealdb_prefix_query() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SurrealDBStorage::new(temp_dir.path()).await.unwrap();

        // Insert multiple keys with same prefix
        storage.put(b"prefix:key1", b"value1").await.unwrap();
        storage.put(b"prefix:key2", b"value2").await.unwrap();
        storage.put(b"prefix:key3", b"value3").await.unwrap();
        storage.put(b"other:key", b"other").await.unwrap();

        // Query keys with prefix
        let keys = storage.get_keys_with_prefix(b"prefix:").await.unwrap();
        assert_eq!(keys.len(), 3);

        // Verify all keys start with prefix
        for key in keys {
            assert!(key.starts_with(b"prefix:"));
        }
    }

    #[tokio::test]
    async fn test_surrealdb_batch_write() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SurrealDBStorage::new(temp_dir.path()).await.unwrap();

        let operations = vec![
            WriteOp::Put {
                key: b"key1".to_vec(),
                value: b"value1".to_vec(),
            },
            WriteOp::Put {
                key: b"key2".to_vec(),
                value: b"value2".to_vec(),
            },
            WriteOp::Put {
                key: b"key3".to_vec(),
                value: b"value3".to_vec(),
            },
        ];

        storage.batch_write(operations).await.unwrap();

        assert_eq!(
            storage.get(b"key1").await.unwrap(),
            Some(b"value1".to_vec())
        );
        assert_eq!(
            storage.get(b"key2").await.unwrap(),
            Some(b"value2".to_vec())
        );
        assert_eq!(
            storage.get(b"key3").await.unwrap(),
            Some(b"value3".to_vec())
        );
    }

    #[tokio::test]
    async fn test_surrealdb_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let storage = SurrealDBStorage::new(temp_dir.path()).await.unwrap();

        storage.put(b"key1", b"value1").await.unwrap();

        let snapshot = storage.snapshot().await.unwrap();
        let value = snapshot.get(b"key1").await.unwrap();
        assert_eq!(value, Some(b"value1".to_vec()));
    }

    #[test]
    fn test_next_prefix() {
        assert_eq!(next_prefix("abc"), "abd");
        assert_eq!(next_prefix("ab"), "ac");
        assert!(next_prefix("").len() > 0);
    }
}
