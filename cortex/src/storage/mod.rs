pub mod backup;
pub mod surrealdb_storage;
pub mod surreal_manager;
pub mod memory_storage;
pub mod factory;
pub mod resilient;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Current schema version for data structures
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

pub use backup::{BackupConfig, BackupManager, BackupMetadata, BackupStats, BackupType};
pub use surrealdb_storage::{SurrealDBStorage, SurrealDBConfig};
pub use surreal_manager::{SurrealManager, SurrealManagerConfig, SurrealMode};
pub use memory_storage::MemoryStorage;
pub use factory::{create_storage, create_default_storage, StorageConfig, StorageBackend};
pub use resilient::ResilientStorage;

/// Storage backend trait
#[async_trait]
pub trait Storage: Send + Sync + std::any::Any {
    /// Get a value by key
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;

    /// Put a key-value pair
    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()>;

    /// Delete a key
    async fn delete(&self, key: &[u8]) -> Result<()>;

    /// Check if key exists
    async fn exists(&self, key: &[u8]) -> Result<bool>;

    /// Get all keys with a prefix
    async fn get_keys_with_prefix(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>>;

    /// Batch write operations
    async fn batch_write(&self, operations: Vec<WriteOp>) -> Result<()>;

    /// Create a snapshot
    async fn snapshot(&self) -> Result<Box<dyn Snapshot>>;
}

/// Write operation
#[derive(Debug, Clone)]
pub enum WriteOp {
    Put { key: Vec<u8>, value: Vec<u8> },
    Delete { key: Vec<u8> },
}

/// Snapshot trait
#[async_trait]
pub trait Snapshot: Send + Sync {
    /// Get a value from the snapshot
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;
}

/// Helper functions for serialization
pub fn serialize<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(value)?)
}

pub fn deserialize<T: for<'de> Deserialize<'de>>(data: &[u8]) -> Result<T> {
    Ok(serde_json::from_slice(data)?)
}
