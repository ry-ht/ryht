/// Resilient storage wrapper with automatic error recovery
///
/// Wraps any Storage implementation with retry logic for transient failures
use super::{Storage, WriteOp};
use crate::error_recovery::{retry_with_backoff, RetryConfig};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// Storage wrapper with automatic retry on failures
pub struct ResilientStorage<S: Storage> {
    inner: Arc<S>,
    read_config: RetryConfig,
    write_config: RetryConfig,
}

impl<S: Storage> ResilientStorage<S> {
    /// Create a new resilient storage wrapper
    pub fn new(inner: S) -> Self {
        Self {
            inner: Arc::new(inner),
            read_config: RetryConfig::aggressive(), // Reads can be retried quickly
            write_config: RetryConfig::conservative(), // Writes need more careful handling
        }
    }

    /// Create with custom retry configs
    pub fn with_configs(inner: S, read_config: RetryConfig, write_config: RetryConfig) -> Self {
        Self {
            inner: Arc::new(inner),
            read_config,
            write_config,
        }
    }
}

#[async_trait]
impl<S: Storage + Send + Sync + 'static> Storage for ResilientStorage<S> {
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let inner = self.inner.clone();
        let key = key.to_vec();

        retry_with_backoff(self.read_config.clone(), "storage_get", || {
            let inner = inner.clone();
            let key = key.clone();
            async move { inner.get(&key).await }
        })
        .await
    }

    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        let inner = self.inner.clone();
        let key = key.to_vec();
        let value = value.to_vec();

        retry_with_backoff(self.write_config.clone(), "storage_put", || {
            let inner = inner.clone();
            let key = key.clone();
            let value = value.clone();
            async move { inner.put(&key, &value).await }
        })
        .await
    }

    async fn delete(&self, key: &[u8]) -> Result<()> {
        let inner = self.inner.clone();
        let key = key.to_vec();

        retry_with_backoff(self.write_config.clone(), "storage_delete", || {
            let inner = inner.clone();
            let key = key.clone();
            async move { inner.delete(&key).await }
        })
        .await
    }

    async fn exists(&self, key: &[u8]) -> Result<bool> {
        let inner = self.inner.clone();
        let key = key.to_vec();

        retry_with_backoff(self.read_config.clone(), "storage_exists", || {
            let inner = inner.clone();
            let key = key.clone();
            async move { inner.exists(&key).await }
        })
        .await
    }

    async fn get_keys_with_prefix(&self, prefix: &[u8]) -> Result<Vec<Vec<u8>>> {
        let inner = self.inner.clone();
        let prefix = prefix.to_vec();

        retry_with_backoff(
            self.read_config.clone(),
            "storage_get_keys_with_prefix",
            || {
                let inner = inner.clone();
                let prefix = prefix.clone();
                async move { inner.get_keys_with_prefix(&prefix).await }
            },
        )
        .await
    }

    async fn batch_write(&self, operations: Vec<WriteOp>) -> Result<()> {
        let inner = self.inner.clone();

        retry_with_backoff(self.write_config.clone(), "storage_batch_write", || {
            let inner = inner.clone();
            let operations = operations.clone();
            async move { inner.batch_write(operations).await }
        })
        .await
    }

    async fn snapshot(&self) -> Result<Box<dyn super::Snapshot>> {
        let inner = self.inner.clone();

        retry_with_backoff(self.read_config.clone(), "storage_snapshot", || {
            let inner = inner.clone();
            async move { inner.snapshot().await }
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    // Tests would require a mock Storage implementation
    // In production, ResilientStorage wraps RocksDBStorage
    // Integration tests verify the retry behavior
}
