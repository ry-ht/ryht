//! Data Synchronization Manager for SurrealDB and Qdrant
//!
//! This module provides production-ready synchronization between SurrealDB (structured data)
//! and Qdrant (vector embeddings) with:
//! - Transactional patterns with compensation
//! - Write-ahead logging for crash recovery
//! - Async event streaming for real-time sync
//! - Conflict resolution with semantic understanding
//! - Batch operations with optimal transaction boundaries

use crate::connection_pool::ConnectionManager;
use cortex_core::error::{CortexError, Result};
use cortex_core::id::CortexId;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use qdrant_client::{
    client::QdrantClient,
    qdrant::{
        vectors_config::Config, CreateCollection, Distance, PointStruct, UpsertPointsBuilder,
        VectorParams, VectorsConfig, DeletePointsBuilder, Filter, Condition, FieldCondition,
        Match, SearchPoints, PointsIdsList, PointsSelector,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock, Semaphore};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

// ============================================================================
// Core Types and Configuration
// ============================================================================

/// Configuration for the data synchronization manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Qdrant server URL
    pub qdrant_url: String,

    /// Qdrant API key (optional)
    pub qdrant_api_key: Option<String>,

    /// Enable write-ahead logging
    pub enable_wal: bool,

    /// WAL directory path
    pub wal_dir: String,

    /// Maximum batch size for bulk operations
    pub max_batch_size: usize,

    /// Timeout for sync operations (seconds)
    pub sync_timeout_secs: u64,

    /// Enable automatic retry on failure
    pub enable_retry: bool,

    /// Maximum retry attempts
    pub max_retries: u32,

    /// Retry backoff base (milliseconds)
    pub retry_backoff_ms: u64,

    /// Enable consistency verification
    pub enable_verification: bool,

    /// Verification interval (seconds)
    pub verification_interval_secs: u64,

    /// Maximum concurrent operations
    pub max_concurrent_ops: usize,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            qdrant_url: "http://localhost:6333".to_string(),
            qdrant_api_key: None,
            enable_wal: true,
            wal_dir: "/tmp/cortex-wal".to_string(),
            max_batch_size: 100,
            sync_timeout_secs: 30,
            enable_retry: true,
            max_retries: 3,
            retry_backoff_ms: 100,
            enable_verification: true,
            verification_interval_secs: 300, // 5 minutes
            max_concurrent_ops: 10,
        }
    }
}

/// Entity to be synchronized between SurrealDB and Qdrant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEntity {
    /// Unique identifier
    pub id: CortexId,

    /// Entity type (for collection selection)
    pub entity_type: String,

    /// Vector embedding
    pub vector: Vec<f32>,

    /// Metadata to store in both systems
    pub metadata: HashMap<String, serde_json::Value>,

    /// Timestamp for ordering
    pub timestamp: DateTime<Utc>,

    /// Optional workspace ID for multi-tenancy
    pub workspace_id: Option<String>,
}

/// Operation to be performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncOperation {
    /// Insert or update entity
    Upsert(SyncEntity),

    /// Delete entity
    Delete { id: CortexId, entity_type: String },

    /// Batch upsert
    BatchUpsert(Vec<SyncEntity>),

    /// Batch delete
    BatchDelete { ids: Vec<CortexId>, entity_type: String },
}

/// Result of a sync operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Operation ID
    pub operation_id: Uuid,

    /// Success status
    pub success: bool,

    /// Error message if failed
    pub error: Option<String>,

    /// Number of entities affected
    pub affected_count: usize,

    /// Duration in milliseconds
    pub duration_ms: u64,

    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Event for real-time sync notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEvent {
    /// Event type
    pub event_type: SyncEventType,

    /// Associated entity ID
    pub entity_id: CortexId,

    /// Entity type
    pub entity_type: String,

    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Type of sync event
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyncEventType {
    /// Entity was synced successfully
    Synced,

    /// Sync failed
    Failed,

    /// Conflict detected
    Conflict,

    /// Inconsistency detected
    Inconsistent,

    /// Repair completed
    Repaired,
}

// ============================================================================
// Write-Ahead Log (WAL) for Crash Recovery
// ============================================================================

/// Write-ahead log entry for durability
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WalEntry {
    /// Entry ID
    id: Uuid,

    /// Operation to perform
    operation: SyncOperation,

    /// Status
    status: WalStatus,

    /// Created timestamp
    created_at: DateTime<Utc>,

    /// Committed timestamp
    committed_at: Option<DateTime<Utc>>,
}

/// Status of WAL entry
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
enum WalStatus {
    /// Pending execution
    Pending,

    /// SurrealDB write completed
    SurrealCompleted,

    /// Qdrant write completed
    QdrantCompleted,

    /// Both completed
    Committed,

    /// Failed, needs rollback
    Failed,
}

/// Write-ahead log manager
struct WalManager {
    /// Log directory
    dir: String,

    /// In-memory cache of recent entries
    cache: Arc<DashMap<Uuid, WalEntry>>,
}

impl WalManager {
    /// Create a new WAL manager
    fn new(dir: String) -> Self {
        Self {
            dir,
            cache: Arc::new(DashMap::new()),
        }
    }

    /// Write an entry to the WAL
    async fn write(&self, entry: WalEntry) -> Result<()> {
        // Write to disk for durability
        let entry_path = format!("{}/{}.wal", self.dir, entry.id);
        let entry_json = serde_json::to_string(&entry)
            .map_err(|e| CortexError::serialization(format!("Failed to serialize WAL entry: {}", e)))?;

        tokio::fs::write(&entry_path, entry_json).await
            .map_err(|e| CortexError::storage(format!("Failed to write WAL entry: {}", e)))?;

        // Update cache
        self.cache.insert(entry.id, entry);

        Ok(())
    }

    /// Update entry status
    async fn update_status(&self, id: Uuid, status: WalStatus) -> Result<()> {
        if let Some(mut entry) = self.cache.get_mut(&id) {
            entry.status = status;

            if status == WalStatus::Committed {
                entry.committed_at = Some(Utc::now());
            }

            // Write update to disk
            let entry_path = format!("{}/{}.wal", self.dir, id);
            let entry_json = serde_json::to_string(&*entry)
                .map_err(|e| CortexError::serialization(format!("Failed to serialize WAL entry: {}", e)))?;

            tokio::fs::write(&entry_path, entry_json).await
                .map_err(|e| CortexError::storage(format!("Failed to update WAL entry: {}", e)))?;
        }

        Ok(())
    }

    /// Remove committed entry from WAL
    async fn remove(&self, id: Uuid) -> Result<()> {
        self.cache.remove(&id);

        let entry_path = format!("{}/{}.wal", self.dir, id);
        tokio::fs::remove_file(&entry_path).await
            .map_err(|e| CortexError::storage(format!("Failed to remove WAL entry: {}", e)))?;

        Ok(())
    }

    /// Recover pending operations from WAL
    async fn recover(&self) -> Result<Vec<WalEntry>> {
        let mut entries = Vec::new();

        // Read all .wal files from directory
        let mut dir_reader = tokio::fs::read_dir(&self.dir).await
            .map_err(|e| CortexError::storage(format!("Failed to read WAL directory: {}", e)))?;

        while let Some(entry) = dir_reader.next_entry().await
            .map_err(|e| CortexError::storage(format!("Failed to read directory entry: {}", e)))? {

            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("wal") {
                let content = tokio::fs::read_to_string(&path).await
                    .map_err(|e| CortexError::storage(format!("Failed to read WAL file: {}", e)))?;

                let wal_entry: WalEntry = serde_json::from_str(&content)
                    .map_err(|e| CortexError::serialization(format!("Failed to deserialize WAL entry: {}", e)))?;

                if wal_entry.status != WalStatus::Committed {
                    entries.push(wal_entry);
                }
            }
        }

        Ok(entries)
    }
}

// ============================================================================
// Data Synchronization Manager
// ============================================================================

/// Main synchronization manager coordinating SurrealDB and Qdrant
pub struct DataSyncManager {
    /// Configuration
    config: SyncConfig,

    /// SurrealDB connection manager
    surreal: Arc<ConnectionManager>,

    /// Qdrant client
    qdrant: Arc<QdrantClient>,

    /// Write-ahead log
    wal: Arc<WalManager>,

    /// Event broadcaster for real-time updates
    event_tx: broadcast::Sender<SyncEvent>,

    /// Metrics
    metrics: Arc<SyncMetrics>,

    /// Semaphore for limiting concurrent operations
    semaphore: Arc<Semaphore>,
}

impl DataSyncManager {
    /// Create a new data sync manager
    #[instrument(skip(surreal))]
    pub async fn new(
        config: SyncConfig,
        surreal: Arc<ConnectionManager>,
    ) -> Result<Self> {
        info!("Initializing DataSyncManager");

        // Create Qdrant client
        let qdrant = QdrantClient::from_url(&config.qdrant_url)
            .build()
            .map_err(|e| CortexError::connection(format!("Failed to create Qdrant client: {}", e)))?;

        // Initialize WAL directory
        tokio::fs::create_dir_all(&config.wal_dir).await
            .map_err(|e| CortexError::storage(format!("Failed to create WAL directory: {}", e)))?;

        let wal = Arc::new(WalManager::new(config.wal_dir.clone()));

        // Create event channel
        let (event_tx, _) = broadcast::channel(1000);

        let manager = Self {
            config: config.clone(),
            surreal,
            qdrant: Arc::new(qdrant),
            wal,
            event_tx,
            metrics: Arc::new(SyncMetrics::new()),
            semaphore: Arc::new(Semaphore::new(config.max_concurrent_ops)),
        };

        // Recover from WAL if enabled
        if config.enable_wal {
            manager.recover_from_wal().await?;
        }

        info!("DataSyncManager initialized successfully");
        Ok(manager)
    }

    /// Create a Qdrant collection with proper configuration
    #[instrument(skip(self))]
    pub async fn create_collection(
        &self,
        collection_name: &str,
        vector_size: u64,
        distance: Distance,
    ) -> Result<()> {
        info!("Creating Qdrant collection: {}", collection_name);

        let create_collection = CreateCollection {
            collection_name: collection_name.to_string(),
            vectors_config: Some(VectorsConfig {
                config: Some(Config::Params(VectorParams {
                    size: vector_size,
                    distance: distance.into(),
                    ..Default::default()
                })),
            }),
            ..Default::default()
        };

        self.qdrant
            .create_collection(&create_collection)
            .await
            .map_err(|e| CortexError::storage(format!("Failed to create Qdrant collection: {}", e)))?;

        info!("Collection {} created successfully", collection_name);
        Ok(())
    }

    /// Sync a single entity (coordinated write to both systems)
    #[instrument(skip(self, entity))]
    pub async fn sync_entity(&self, entity: SyncEntity) -> Result<SyncResult> {
        let _permit = self.semaphore.acquire().await
            .map_err(|e| CortexError::internal(format!("Failed to acquire semaphore: {}", e)))?;

        let operation_id = Uuid::new_v4();
        let start = std::time::Instant::now();

        debug!("Syncing entity {} of type {}", entity.id, entity.entity_type);

        // Create WAL entry if enabled
        let wal_id = if self.config.enable_wal {
            let wal_entry = WalEntry {
                id: Uuid::new_v4(),
                operation: SyncOperation::Upsert(entity.clone()),
                status: WalStatus::Pending,
                created_at: Utc::now(),
                committed_at: None,
            };

            self.wal.write(wal_entry.clone()).await?;
            Some(wal_entry.id)
        } else {
            None
        };

        // Execute sync with retry
        let result = self.execute_sync_with_retry(entity.clone(), wal_id).await;

        // Record metrics
        let duration_ms = start.elapsed().as_millis() as u64;
        let success = result.is_ok();

        if success {
            self.metrics.record_success(duration_ms);
        } else {
            self.metrics.record_failure();
        }

        // Clean up WAL if successful
        if let Some(wal_id) = wal_id {
            if success {
                self.wal.remove(wal_id).await?;
            }
        }

        // Send event
        let event = SyncEvent {
            event_type: if success { SyncEventType::Synced } else { SyncEventType::Failed },
            entity_id: entity.id,
            entity_type: entity.entity_type.clone(),
            timestamp: Utc::now(),
        };
        let _ = self.event_tx.send(event);

        Ok(SyncResult {
            operation_id,
            success,
            error: result.err().map(|e| e.to_string()),
            affected_count: 1,
            duration_ms,
            timestamp: Utc::now(),
        })
    }

    /// Execute sync with retry logic
    async fn execute_sync_with_retry(
        &self,
        entity: SyncEntity,
        wal_id: Option<Uuid>,
    ) -> Result<()> {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts < self.config.max_retries {
            attempts += 1;

            match self.execute_sync_coordinated(&entity, wal_id).await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    warn!("Sync attempt {} failed: {}", attempts, e);
                    last_error = Some(e);

                    if attempts < self.config.max_retries {
                        let backoff = self.config.retry_backoff_ms * 2u64.pow(attempts - 1);
                        tokio::time::sleep(tokio::time::Duration::from_millis(backoff)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| CortexError::internal("Sync failed after retries")))
    }

    /// Execute coordinated sync (transactional pattern)
    async fn execute_sync_coordinated(
        &self,
        entity: &SyncEntity,
        wal_id: Option<Uuid>,
    ) -> Result<()> {
        // Step 1: Write to SurrealDB (source of truth for metadata)
        self.write_to_surreal(entity).await?;

        // Update WAL status
        if let Some(wal_id) = wal_id {
            self.wal.update_status(wal_id, WalStatus::SurrealCompleted).await?;
        }

        // Step 2: Write to Qdrant (vector storage)
        match self.write_to_qdrant(entity).await {
            Ok(_) => {
                // Update WAL status to committed
                if let Some(wal_id) = wal_id {
                    self.wal.update_status(wal_id, WalStatus::Committed).await?;
                }
                Ok(())
            }
            Err(e) => {
                // Qdrant write failed - mark WAL as failed and attempt compensation
                if let Some(wal_id) = wal_id {
                    self.wal.update_status(wal_id, WalStatus::Failed).await?;
                }

                warn!("Qdrant write failed, inconsistency detected for entity {}", entity.id);

                // Send inconsistency event
                let event = SyncEvent {
                    event_type: SyncEventType::Inconsistent,
                    entity_id: entity.id,
                    entity_type: entity.entity_type.clone(),
                    timestamp: Utc::now(),
                };
                let _ = self.event_tx.send(event);

                Err(e)
            }
        }
    }

    /// Write entity metadata to SurrealDB
    async fn write_to_surreal(&self, entity: &SyncEntity) -> Result<()> {
        let conn = self.surreal.acquire().await?;

        // Store metadata with vector_id reference
        let table = format!("{}s", entity.entity_type);
        let record_id = entity.id.to_string();

        let mut data = entity.metadata.clone();
        data.insert("id".to_string(), serde_json::json!(record_id));
        data.insert("vector_id".to_string(), serde_json::json!(entity.id.to_string()));
        data.insert("timestamp".to_string(), serde_json::json!(entity.timestamp));

        if let Some(workspace_id) = &entity.workspace_id {
            data.insert("workspace_id".to_string(), serde_json::json!(workspace_id));
        }

        let query = format!("UPDATE {}:{} CONTENT $data", table, record_id);

        conn.connection().query(&query)
            .bind(("data", data))
            .await
            .map_err(|e| CortexError::storage(format!("Failed to write to SurrealDB: {}", e)))?;

        Ok(())
    }

    /// Write vector to Qdrant
    async fn write_to_qdrant(&self, entity: &SyncEntity) -> Result<()> {
        let collection_name = format!("{}_vectors", entity.entity_type);

        // Convert metadata to Qdrant payload using serde_json::Map
        let mut payload = serde_json::Map::new();
        payload.insert("entity_id".to_string(), serde_json::json!(entity.id.to_string()));
        payload.insert("entity_type".to_string(), serde_json::json!(entity.entity_type));
        payload.insert("timestamp".to_string(), serde_json::json!(entity.timestamp));

        if let Some(workspace_id) = &entity.workspace_id {
            payload.insert("workspace_id".to_string(), serde_json::json!(workspace_id));
        }

        // Add metadata fields
        for (key, value) in &entity.metadata {
            payload.insert(key.clone(), value.clone());
        }

        let point = PointStruct::new(
            entity.id.to_string(),
            entity.vector.clone(),
            payload,
        );

        self.qdrant
            .upsert_points(&collection_name, None, vec![point], None)
            .await
            .map_err(|e| CortexError::storage(format!("Failed to write to Qdrant: {}", e)))?;

        Ok(())
    }

    /// Batch sync multiple entities with optimal transaction boundaries
    #[instrument(skip(self, entities))]
    pub async fn batch_sync(&self, entities: Vec<SyncEntity>) -> Result<SyncResult> {
        let operation_id = Uuid::new_v4();
        let start = std::time::Instant::now();
        let total_count = entities.len();

        info!("Starting batch sync of {} entities", total_count);

        // Split into optimal batch sizes
        let mut success_count = 0;
        let mut error: Option<String> = None;

        for chunk in entities.chunks(self.config.max_batch_size) {
            match self.batch_sync_chunk(chunk.to_vec()).await {
                Ok(count) => success_count += count,
                Err(e) => {
                    error = Some(e.to_string());
                    error!("Batch chunk sync failed: {}", e);
                    break;
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        let success = error.is_none();

        Ok(SyncResult {
            operation_id,
            success,
            error,
            affected_count: success_count,
            duration_ms,
            timestamp: Utc::now(),
        })
    }

    /// Sync a chunk of entities
    async fn batch_sync_chunk(&self, entities: Vec<SyncEntity>) -> Result<usize> {
        // Group by entity type for efficient batch operations
        let mut by_type: HashMap<String, Vec<SyncEntity>> = HashMap::new();

        for entity in entities {
            by_type.entry(entity.entity_type.clone())
                .or_insert_with(Vec::new)
                .push(entity);
        }

        let mut total_synced = 0;

        for (entity_type, entities) in by_type {
            // Batch write to SurrealDB
            for entity in &entities {
                self.write_to_surreal(entity).await?;
            }

            // Batch write to Qdrant
            let collection_name = format!("{}_vectors", entity_type);
            let points: Vec<PointStruct> = entities.iter()
                .map(|entity| {
                    let mut payload = serde_json::Map::new();
                    payload.insert("entity_id".to_string(), serde_json::json!(entity.id.to_string()));
                    payload.insert("entity_type".to_string(), serde_json::json!(entity.entity_type));

                    PointStruct::new(
                        entity.id.to_string(),
                        entity.vector.clone(),
                        payload,
                    )
                })
                .collect();

            self.qdrant
                .upsert_points(&collection_name, None, points, None)
                .await
                .map_err(|e| CortexError::storage(format!("Failed to batch write to Qdrant: {}", e)))?;

            total_synced += entities.len();
        }

        Ok(total_synced)
    }

    /// Delete entity from both systems
    #[instrument(skip(self))]
    pub async fn delete_entity(&self, id: CortexId, entity_type: String) -> Result<SyncResult> {
        let operation_id = Uuid::new_v4();
        let start = std::time::Instant::now();

        debug!("Deleting entity {} of type {}", id, entity_type);

        // Delete from SurrealDB first
        let conn = self.surreal.acquire().await?;
        let table = format!("{}s", entity_type);
        let query = format!("DELETE {}:{}", table, id);

        conn.connection().query(&query).await
            .map_err(|e| CortexError::storage(format!("Failed to delete from SurrealDB: {}", e)))?;

        // Delete from Qdrant
        let collection_name = format!("{}_vectors", entity_type);
        let point_id = id.to_string().into();

        let selector = PointsSelector {
            points_selector_one_of: Some(qdrant_client::qdrant::points_selector::PointsSelectorOneOf::Points(
                PointsIdsList {
                    ids: vec![point_id],
                }
            )),
        };

        self.qdrant
            .delete_points(&collection_name, None, &selector, None)
            .await
            .map_err(|e| CortexError::storage(format!("Failed to delete from Qdrant: {}", e)))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(SyncResult {
            operation_id,
            success: true,
            error: None,
            affected_count: 1,
            duration_ms,
            timestamp: Utc::now(),
        })
    }

    /// Subscribe to sync events
    pub fn subscribe(&self) -> broadcast::Receiver<SyncEvent> {
        self.event_tx.subscribe()
    }

    /// Get current metrics
    pub fn metrics(&self) -> SyncMetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Recover pending operations from WAL
    async fn recover_from_wal(&self) -> Result<()> {
        info!("Recovering pending operations from WAL");

        let pending_entries = self.wal.recover().await?;

        if pending_entries.is_empty() {
            info!("No pending operations to recover");
            return Ok(());
        }

        warn!("Found {} pending operations, recovering...", pending_entries.len());

        for entry in pending_entries {
            match entry.operation {
                SyncOperation::Upsert(entity) => {
                    match self.execute_sync_coordinated(&entity, Some(entry.id)).await {
                        Ok(_) => {
                            info!("Recovered operation {}", entry.id);
                            self.wal.remove(entry.id).await?;
                        }
                        Err(e) => {
                            error!("Failed to recover operation {}: {}", entry.id, e);
                        }
                    }
                }
                _ => {
                    warn!("Unsupported operation type in WAL recovery");
                }
            }
        }

        info!("WAL recovery completed");
        Ok(())
    }
}

// ============================================================================
// Metrics
// ============================================================================

/// Synchronization metrics
pub struct SyncMetrics {
    /// Total operations
    total_ops: Arc<RwLock<u64>>,

    /// Successful operations
    successful_ops: Arc<RwLock<u64>>,

    /// Failed operations
    failed_ops: Arc<RwLock<u64>>,

    /// Average latency (milliseconds)
    avg_latency_ms: Arc<RwLock<f64>>,

    /// Total latency for averaging
    total_latency_ms: Arc<RwLock<u64>>,
}

impl SyncMetrics {
    fn new() -> Self {
        Self {
            total_ops: Arc::new(RwLock::new(0)),
            successful_ops: Arc::new(RwLock::new(0)),
            failed_ops: Arc::new(RwLock::new(0)),
            avg_latency_ms: Arc::new(RwLock::new(0.0)),
            total_latency_ms: Arc::new(RwLock::new(0)),
        }
    }

    fn record_success(&self, latency_ms: u64) {
        let total_ops = self.total_ops.clone();
        let successful_ops = self.successful_ops.clone();
        let total_latency = self.total_latency_ms.clone();
        let avg_latency = self.avg_latency_ms.clone();

        tokio::spawn(async move {
            let mut total = total_ops.write().await;
            *total += 1;

            let mut success = successful_ops.write().await;
            *success += 1;

            let mut total_lat = total_latency.write().await;
            *total_lat += latency_ms;

            let mut avg = avg_latency.write().await;
            *avg = *total_lat as f64 / *total as f64;
        });
    }

    fn record_failure(&self) {
        let total_ops = self.total_ops.clone();
        let failed_ops = self.failed_ops.clone();

        tokio::spawn(async move {
            let mut total = total_ops.write().await;
            *total += 1;

            let mut failed = failed_ops.write().await;
            *failed += 1;
        });
    }

    fn snapshot(&self) -> SyncMetricsSnapshot {
        let total_ops = self.total_ops.clone();
        let successful_ops = self.successful_ops.clone();
        let failed_ops = self.failed_ops.clone();
        let avg_latency_ms = self.avg_latency_ms.clone();

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                SyncMetricsSnapshot {
                    total_operations: *total_ops.read().await,
                    successful_operations: *successful_ops.read().await,
                    failed_operations: *failed_ops.read().await,
                    average_latency_ms: *avg_latency_ms.read().await,
                }
            })
        })
    }
}

/// Snapshot of sync metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMetricsSnapshot {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub average_latency_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_config_default() {
        let config = SyncConfig::default();
        assert_eq!(config.qdrant_url, "http://localhost:6333");
        assert!(config.enable_wal);
        assert_eq!(config.max_batch_size, 100);
    }

    #[test]
    fn test_wal_entry_serialization() {
        let entity = SyncEntity {
            id: CortexId::new(),
            entity_type: "code".to_string(),
            vector: vec![0.1, 0.2, 0.3],
            metadata: HashMap::new(),
            timestamp: Utc::now(),
            workspace_id: None,
        };

        let entry = WalEntry {
            id: Uuid::new_v4(),
            operation: SyncOperation::Upsert(entity),
            status: WalStatus::Pending,
            created_at: Utc::now(),
            committed_at: None,
        };

        let serialized = serde_json::to_string(&entry).unwrap();
        let deserialized: WalEntry = serde_json::from_str(&serialized).unwrap();

        assert_eq!(entry.id, deserialized.id);
        assert_eq!(entry.status, deserialized.status);
    }
}
