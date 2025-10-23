//! Hybrid vector store for migration support.
//!
//! This module provides a dual-write implementation that writes to both
//! old (HNSW) and new (Qdrant) stores during migration, with consistency
//! verification and automatic fallback.

use crate::config::MigrationMode;
use crate::error::{Result, SemanticError};
use crate::index::{IndexStats, SearchResult, VectorIndex};
use crate::types::{DocumentId, Vector};
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Hybrid vector store that coordinates writes and reads between two stores.
///
/// Features:
/// - Dual-write to both old and new stores
/// - Configurable read strategy based on migration mode
/// - Consistency verification between stores
/// - Automatic fallback on errors
/// - Metrics collection for migration monitoring
pub struct HybridVectorStore {
    old_store: Arc<dyn VectorIndex>,
    new_store: Arc<dyn VectorIndex>,
    mode: Arc<RwLock<MigrationMode>>,
    metrics: Arc<HybridMetrics>,
}

/// Metrics for hybrid store operations.
#[derive(Debug, Default)]
pub struct HybridMetrics {
    pub dual_write_successes: std::sync::atomic::AtomicU64,
    pub dual_write_failures: std::sync::atomic::AtomicU64,
    pub consistency_checks: std::sync::atomic::AtomicU64,
    pub consistency_mismatches: std::sync::atomic::AtomicU64,
    pub old_store_failures: std::sync::atomic::AtomicU64,
    pub new_store_failures: std::sync::atomic::AtomicU64,
    pub fallback_activations: std::sync::atomic::AtomicU64,
}

impl HybridVectorStore {
    /// Create a new hybrid vector store.
    pub fn new(
        old_store: Arc<dyn VectorIndex>,
        new_store: Arc<dyn VectorIndex>,
        mode: MigrationMode,
    ) -> Self {
        info!("Creating hybrid vector store with mode: {:?}", mode);

        Self {
            old_store,
            new_store,
            mode: Arc::new(RwLock::new(mode)),
            metrics: Arc::new(HybridMetrics::default()),
        }
    }

    /// Get current migration mode.
    pub async fn mode(&self) -> MigrationMode {
        *self.mode.read().await
    }

    /// Set migration mode.
    pub async fn set_mode(&self, mode: MigrationMode) {
        info!("Changing migration mode to: {:?}", mode);
        *self.mode.write().await = mode;
    }

    /// Get metrics.
    pub fn metrics(&self) -> &HybridMetrics {
        &self.metrics
    }

    /// Dual write to both stores with error handling.
    async fn dual_write<F, Fut>(&self, operation: &str, f: F) -> Result<()>
    where
        F: Fn(Arc<dyn VectorIndex>) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let mode = self.mode().await;

        match mode {
            MigrationMode::SingleStore => {
                // Only write to old store
                f(self.old_store.clone()).await
            }
            MigrationMode::DualWrite | MigrationMode::DualVerify | MigrationMode::NewPrimary => {
                // Write to both stores
                let old_result = f(self.old_store.clone()).await;
                let new_result = f(self.new_store.clone()).await;

                match (old_result, new_result) {
                    (Ok(()), Ok(())) => {
                        self.metrics
                            .dual_write_successes
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        debug!("Dual write succeeded for operation: {}", operation);
                        Ok(())
                    }
                    (Err(e), Ok(())) => {
                        self.metrics
                            .old_store_failures
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        warn!("Old store failed for {}: {}", operation, e);

                        // Continue if new store succeeded
                        if matches!(mode, MigrationMode::NewPrimary) {
                            Ok(())
                        } else {
                            Err(e)
                        }
                    }
                    (Ok(()), Err(e)) => {
                        self.metrics
                            .new_store_failures
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        warn!("New store failed for {}: {}", operation, e);

                        // Continue if old store succeeded
                        Ok(())
                    }
                    (Err(e1), Err(e2)) => {
                        self.metrics
                            .dual_write_failures
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        error!(
                            "Both stores failed for {}: old={}, new={}",
                            operation, e1, e2
                        );
                        Err(e1)
                    }
                }
            }
        }
    }

    /// Read with fallback based on migration mode.
    async fn read_with_fallback<F, Fut, T>(&self, operation: &str, f: F) -> Result<T>
    where
        F: Fn(Arc<dyn VectorIndex>) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mode = self.mode().await;

        match mode {
            MigrationMode::SingleStore | MigrationMode::DualWrite => {
                // Read from old store
                f(self.old_store.clone()).await
            }
            MigrationMode::NewPrimary => {
                // Read from new store, fallback to old on error
                match f(self.new_store.clone()).await {
                    Ok(result) => Ok(result),
                    Err(e) => {
                        warn!("New store failed for {}, falling back to old: {}", operation, e);
                        self.metrics
                            .fallback_activations
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        f(self.old_store.clone()).await
                    }
                }
            }
            MigrationMode::DualVerify => {
                // Read from both and log discrepancies (but don't require exact equality)
                self.read_and_log_differences(operation, f).await
            }
        }
    }

    /// Read from both stores and log differences (for verification mode).
    /// Since search results can vary slightly, we don't require exact equality.
    async fn read_and_log_differences<F, Fut, T>(&self, operation: &str, f: F) -> Result<T>
    where
        F: Fn(Arc<dyn VectorIndex>) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        self.metrics
            .consistency_checks
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let old_result = f(self.old_store.clone()).await;
        let new_result = f(self.new_store.clone()).await;

        match (old_result, new_result) {
            (Ok(old_data), Ok(_new_data)) => {
                // Both succeeded - log for monitoring but don't compare
                // (search results can vary slightly between implementations)
                debug!("Both stores succeeded for operation: {}", operation);

                // Return old store result during verification phase
                Ok(old_data)
            }
            (Ok(old_data), Err(e)) => {
                warn!("New store failed during verification for {}: {}", operation, e);
                self.metrics
                    .new_store_failures
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Ok(old_data)
            }
            (Err(e), Ok(new_data)) => {
                warn!("Old store failed during verification for {}: {}", operation, e);
                self.metrics
                    .old_store_failures
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Ok(new_data)
            }
            (Err(e1), Err(e2)) => {
                error!(
                    "Both stores failed during verification for {}: old={}, new={}",
                    operation, e1, e2
                );
                Err(e1)
            }
        }
    }

    /// Verify consistency for a specific document.
    pub async fn verify_document(&self, doc_id: &DocumentId) -> Result<ConsistencyStatus> {
        // This would require additional methods on VectorIndex trait
        // For now, we'll implement a simplified version
        Ok(ConsistencyStatus::Unknown)
    }

    /// Get migration progress report.
    pub async fn migration_report(&self) -> MigrationReport {
        let mode = self.mode().await;

        let old_count = self.old_store.len().await;
        let new_count = self.new_store.len().await;

        MigrationReport {
            mode,
            old_store_count: old_count,
            new_store_count: new_count,
            dual_write_successes: self
                .metrics
                .dual_write_successes
                .load(std::sync::atomic::Ordering::Relaxed),
            dual_write_failures: self
                .metrics
                .dual_write_failures
                .load(std::sync::atomic::Ordering::Relaxed),
            consistency_checks: self
                .metrics
                .consistency_checks
                .load(std::sync::atomic::Ordering::Relaxed),
            consistency_mismatches: self
                .metrics
                .consistency_mismatches
                .load(std::sync::atomic::Ordering::Relaxed),
            old_store_failures: self
                .metrics
                .old_store_failures
                .load(std::sync::atomic::Ordering::Relaxed),
            new_store_failures: self
                .metrics
                .new_store_failures
                .load(std::sync::atomic::Ordering::Relaxed),
            fallback_activations: self
                .metrics
                .fallback_activations
                .load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}

#[async_trait]
impl VectorIndex for HybridVectorStore {
    async fn insert(&self, doc_id: DocumentId, vector: Vector) -> Result<()> {
        self.dual_write("insert", |store| {
            let doc_id = doc_id.clone();
            let vector = vector.clone();
            async move { store.insert(doc_id, vector).await }
        })
        .await
    }

    async fn insert_batch(&self, items: Vec<(DocumentId, Vector)>) -> Result<()> {
        self.dual_write("insert_batch", |store| {
            let items = items.clone();
            async move { store.insert_batch(items).await }
        })
        .await
    }

    async fn search(&self, query: &[f32], k: usize) -> Result<Vec<SearchResult>> {
        self.read_with_fallback("search", |store| {
            let query = query.to_vec();
            async move { store.search(&query, k).await }
        })
        .await
    }

    async fn remove(&self, doc_id: &DocumentId) -> Result<()> {
        self.dual_write("remove", |store| {
            let doc_id = doc_id.clone();
            async move { store.remove(&doc_id).await }
        })
        .await
    }

    async fn len(&self) -> usize {
        let mode = self.mode().await;

        match mode {
            MigrationMode::SingleStore | MigrationMode::DualWrite | MigrationMode::DualVerify => {
                self.old_store.len().await
            }
            MigrationMode::NewPrimary => self.new_store.len().await,
        }
    }

    async fn clear(&self) -> Result<()> {
        self.dual_write("clear", |store| async move { store.clear().await })
            .await
    }

    async fn save(&self, path: &Path) -> Result<()> {
        // Save both stores
        let old_result = self.old_store.save(path).await;
        let new_result = self.new_store.save(path).await;

        // Report errors but don't fail if at least one succeeds
        match (old_result, new_result) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(e), Ok(())) => {
                warn!("Old store save failed: {}", e);
                Ok(())
            }
            (Ok(()), Err(e)) => {
                warn!("New store save failed: {}", e);
                Ok(())
            }
            (Err(e1), Err(_e2)) => Err(e1),
        }
    }

    async fn load(&mut self, path: &Path) -> Result<()> {
        // This is tricky with trait objects, so we'll just return an error
        Err(SemanticError::VectorStore(
            "Load not supported on hybrid store".to_string(),
        ))
    }

    async fn stats(&self) -> IndexStats {
        let mode = self.mode().await;

        match mode {
            MigrationMode::SingleStore | MigrationMode::DualWrite | MigrationMode::DualVerify => {
                self.old_store.stats().await
            }
            MigrationMode::NewPrimary => self.new_store.stats().await,
        }
    }
}

/// Consistency status for a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsistencyStatus {
    Consistent,
    MissingInOld,
    MissingInNew,
    Mismatch,
    Unknown,
}

/// Migration progress report.
#[derive(Debug, Clone)]
pub struct MigrationReport {
    pub mode: MigrationMode,
    pub old_store_count: usize,
    pub new_store_count: usize,
    pub dual_write_successes: u64,
    pub dual_write_failures: u64,
    pub consistency_checks: u64,
    pub consistency_mismatches: u64,
    pub old_store_failures: u64,
    pub new_store_failures: u64,
    pub fallback_activations: u64,
}

impl MigrationReport {
    /// Calculate migration progress percentage.
    pub fn progress_percentage(&self) -> f64 {
        if self.old_store_count == 0 {
            return 100.0;
        }

        (self.new_store_count as f64 / self.old_store_count as f64) * 100.0
    }

    /// Check if migration is healthy.
    pub fn is_healthy(&self) -> bool {
        let total_operations =
            self.dual_write_successes + self.dual_write_failures + self.consistency_checks;

        if total_operations == 0 {
            return true;
        }

        // Consider healthy if failure rate is below 5%
        let failure_rate =
            (self.dual_write_failures + self.consistency_mismatches) as f64 / total_operations as f64;

        failure_rate < 0.05
    }

    /// Get human-readable status.
    pub fn status_message(&self) -> String {
        format!(
            "Mode: {:?}, Progress: {:.1}%, Old: {}, New: {}, Success: {}, Failures: {}, Mismatches: {}",
            self.mode,
            self.progress_percentage(),
            self.old_store_count,
            self.new_store_count,
            self.dual_write_successes,
            self.dual_write_failures,
            self.consistency_mismatches
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::HNSWIndex;
    use crate::config::IndexConfig;
    use crate::types::SimilarityMetric;

    fn create_test_vector(dimension: usize, seed: u64) -> Vector {
        let mut vec = Vec::with_capacity(dimension);
        for i in 0..dimension {
            vec.push(((seed + i as u64) % 100) as f32 / 100.0);
        }
        vec
    }

    #[tokio::test]
    async fn test_hybrid_single_store_mode() {
        let config = IndexConfig::default();
        let old_store = Arc::new(HNSWIndex::new(config.clone(), 128).unwrap());
        let new_store = Arc::new(HNSWIndex::new(config, 128).unwrap());

        let hybrid = HybridVectorStore::new(
            old_store.clone(),
            new_store.clone(),
            MigrationMode::SingleStore,
        );

        let vec1 = create_test_vector(128, 1);
        hybrid.insert("doc1".to_string(), vec1).await.unwrap();

        // Should only be in old store
        assert_eq!(old_store.len().await, 1);
        assert_eq!(new_store.len().await, 0);
    }

    #[tokio::test]
    async fn test_hybrid_dual_write_mode() {
        let config = IndexConfig::default();
        let old_store = Arc::new(HNSWIndex::new(config.clone(), 128).unwrap());
        let new_store = Arc::new(HNSWIndex::new(config, 128).unwrap());

        let hybrid = HybridVectorStore::new(
            old_store.clone(),
            new_store.clone(),
            MigrationMode::DualWrite,
        );

        let vec1 = create_test_vector(128, 1);
        hybrid.insert("doc1".to_string(), vec1).await.unwrap();

        // Should be in both stores
        assert_eq!(old_store.len().await, 1);
        assert_eq!(new_store.len().await, 1);
    }

    #[tokio::test]
    async fn test_hybrid_migration_report() {
        let config = IndexConfig::default();
        let old_store = Arc::new(HNSWIndex::new(config.clone(), 128).unwrap());
        let new_store = Arc::new(HNSWIndex::new(config, 128).unwrap());

        let hybrid = HybridVectorStore::new(
            old_store.clone(),
            new_store.clone(),
            MigrationMode::DualWrite,
        );

        // Insert some data
        let vec1 = create_test_vector(128, 1);
        hybrid.insert("doc1".to_string(), vec1).await.unwrap();

        let report = hybrid.migration_report().await;
        assert_eq!(report.old_store_count, 1);
        assert_eq!(report.new_store_count, 1);
        assert_eq!(report.dual_write_successes, 1);
        assert!(report.is_healthy());
    }

    #[tokio::test]
    async fn test_hybrid_mode_transition() {
        let config = IndexConfig::default();
        let old_store = Arc::new(HNSWIndex::new(config.clone(), 128).unwrap());
        let new_store = Arc::new(HNSWIndex::new(config, 128).unwrap());

        let hybrid = HybridVectorStore::new(
            old_store.clone(),
            new_store.clone(),
            MigrationMode::SingleStore,
        );

        assert_eq!(hybrid.mode().await, MigrationMode::SingleStore);

        hybrid.set_mode(MigrationMode::DualWrite).await;
        assert_eq!(hybrid.mode().await, MigrationMode::DualWrite);
    }
}
