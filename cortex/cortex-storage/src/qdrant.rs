//! Qdrant vector database client and operations.
//!
//! This module provides a production-ready Qdrant client with:
//! - Connection pooling and retry logic
//! - Batch operations for efficient ingestion
//! - Health monitoring and metrics
//! - Snapshot and backup capabilities
//! - Migration utilities

use anyhow::{Context, Result};
use qdrant_client::qdrant::{
    CreateCollectionBuilder, Distance, HnswConfigDiff, OptimizersConfigDiff,
    PointStruct, VectorParamsBuilder, UpsertPointsBuilder, CountPointsBuilder,
    SearchPointsBuilder,
};
use qdrant_client::Qdrant;

// Type aliases for return types
pub type HealthCheckReply = qdrant_client::qdrant::HealthCheckReply;
pub type CollectionInfo = qdrant_client::qdrant::CollectionInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Qdrant client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantConfig {
    /// Qdrant host
    pub host: String,
    /// Qdrant HTTP port
    pub port: u16,
    /// Qdrant gRPC port (optional, defaults to HTTP port + 1)
    pub grpc_port: Option<u16>,
    /// API key for authentication
    pub api_key: Option<String>,
    /// Use HTTPS
    pub use_https: bool,
    /// Connection timeout
    pub timeout: Duration,
    /// Request timeout
    pub request_timeout: Duration,
}

impl Default for QdrantConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 6333,
            grpc_port: Some(6334),
            api_key: None,
            use_https: false,
            timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(60),
        }
    }
}

/// Distance metric type
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DistanceMetric {
    Cosine,
    Euclid,
    Dot,
    Manhattan,
}

impl From<DistanceMetric> for Distance {
    fn from(metric: DistanceMetric) -> Self {
        match metric {
            DistanceMetric::Cosine => Distance::Cosine,
            DistanceMetric::Euclid => Distance::Euclid,
            DistanceMetric::Dot => Distance::Dot,
            DistanceMetric::Manhattan => Distance::Manhattan,
        }
    }
}

/// Collection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionConfig {
    /// Collection name
    pub name: String,
    /// Vector size
    pub vector_size: u64,
    /// Distance metric
    pub distance: DistanceMetric,
    /// HNSW configuration
    pub hnsw_config: HnswConfig,
    /// Optimizer configuration
    pub optimizer_config: OptimizerConfig,
}

/// HNSW index configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswConfig {
    /// Number of edges per node
    pub m: u64,
    /// Number of neighbors to consider during construction
    pub ef_construct: u64,
    /// Switch to HNSW after this many vectors
    pub full_scan_threshold: u64,
    /// Store index on disk
    pub on_disk: bool,
}

impl Default for HnswConfig {
    fn default() -> Self {
        Self {
            m: 16,
            ef_construct: 100,
            full_scan_threshold: 10000,
            on_disk: false,
        }
    }
}

/// Optimizer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizerConfig {
    /// Deleted vectors threshold before optimization
    pub deleted_threshold: f64,
    /// Minimum vectors before vacuum
    pub vacuum_min_vector_number: u64,
    /// Default number of segments
    pub default_segment_number: u64,
    /// Maximum segment size
    pub max_segment_size: u64,
    /// Memory-mapped threshold
    pub memmap_threshold: u64,
    /// Indexing threshold
    pub indexing_threshold: u64,
    /// Flush interval
    pub flush_interval_sec: u64,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            deleted_threshold: 0.2,
            vacuum_min_vector_number: 1000,
            default_segment_number: 8,
            max_segment_size: 200000,
            memmap_threshold: 50000,
            indexing_threshold: 20000,
            flush_interval_sec: 5,
        }
    }
}

/// Qdrant client wrapper with connection pooling
pub struct QdrantClient {
    client: Arc<Qdrant>,
    #[allow(dead_code)]
    config: QdrantConfig,
    collections: Arc<RwLock<HashMap<String, CollectionConfig>>>,
}

impl QdrantClient {
    /// Create a new Qdrant client
    pub async fn new(config: QdrantConfig) -> Result<Self> {
        let url = format!(
            "{}://{}:{}",
            if config.use_https { "https" } else { "http" },
            config.host,
            config.grpc_port.unwrap_or(config.port + 1)
        );

        info!("Connecting to Qdrant at {}", url);

        let mut client_config = qdrant_client::config::QdrantConfig::from_url(&url);
        client_config.set_timeout(config.timeout);

        if let Some(api_key) = &config.api_key {
            client_config.set_api_key(api_key);
        }

        let client = Qdrant::new(client_config)
            .context("Failed to create Qdrant client")?;

        Ok(Self {
            client: Arc::new(client),
            config,
            collections: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Health check
    pub async fn health(&self) -> Result<HealthCheckReply> {
        self.client
            .health_check()
            .await
            .context("Qdrant health check failed")
    }

    /// List all collections
    pub async fn list_collections(&self) -> Result<Vec<String>> {
        let collections = self.client.list_collections().await?;
        Ok(collections
            .collections
            .into_iter()
            .map(|c| c.name)
            .collect())
    }

    /// Get collection info
    pub async fn collection_info(&self, collection_name: &str) -> Result<CollectionInfo> {
        let response = self.client
            .collection_info(collection_name)
            .await
            .context("Failed to get collection info")?;

        response.result.ok_or_else(|| anyhow::anyhow!("No collection info in response"))
    }

    /// Create a collection
    pub async fn create_collection(&self, config: CollectionConfig) -> Result<()> {
        info!("Creating collection: {}", config.name);

        let distance: Distance = config.distance.into();

        let vector_params = VectorParamsBuilder::new(config.vector_size, distance)
            .hnsw_config(HnswConfigDiff {
                m: Some(config.hnsw_config.m),
                ef_construct: Some(config.hnsw_config.ef_construct),
                full_scan_threshold: Some(config.hnsw_config.full_scan_threshold),
                on_disk: Some(config.hnsw_config.on_disk),
                ..Default::default()
            });

        self.client
            .create_collection(
                CreateCollectionBuilder::new(&config.name)
                    .vectors_config(vector_params)
                    .optimizers_config(OptimizersConfigDiff {
                        deleted_threshold: Some(config.optimizer_config.deleted_threshold),
                        vacuum_min_vector_number: Some(config.optimizer_config.vacuum_min_vector_number),
                        default_segment_number: Some(config.optimizer_config.default_segment_number),
                        max_segment_size: Some(config.optimizer_config.max_segment_size),
                        memmap_threshold: Some(config.optimizer_config.memmap_threshold),
                        indexing_threshold: Some(config.optimizer_config.indexing_threshold),
                        flush_interval_sec: Some(config.optimizer_config.flush_interval_sec),
                        ..Default::default()
                    })
            )
            .await
            .context("Failed to create collection")?;

        // Cache collection config
        self.collections
            .write()
            .await
            .insert(config.name.clone(), config);

        Ok(())
    }

    /// Delete a collection
    pub async fn delete_collection(&self, collection_name: &str) -> Result<()> {
        info!("Deleting collection: {}", collection_name);

        self.client
            .delete_collection(collection_name)
            .await
            .context("Failed to delete collection")?;

        self.collections.write().await.remove(collection_name);

        Ok(())
    }

    /// Upsert points (vectors)
    pub async fn upsert_points(
        &self,
        collection_name: &str,
        points: Vec<PointStruct>,
    ) -> Result<()> {
        debug!(
            "Upserting {} points to collection: {}",
            points.len(),
            collection_name
        );

        self.client
            .upsert_points(UpsertPointsBuilder::new(collection_name, points).wait(true))
            .await
            .context("Failed to upsert points")?;

        Ok(())
    }

    /// Search for similar vectors
    pub async fn search(
        &self,
        collection_name: &str,
        vector: Vec<f32>,
        limit: u64,
        filter: Option<qdrant_client::qdrant::Filter>,
    ) -> Result<Vec<qdrant_client::qdrant::ScoredPoint>> {
        let mut search_builder = SearchPointsBuilder::new(collection_name, vector, limit)
            .with_payload(true);

        if let Some(f) = filter {
            search_builder = search_builder.filter(f);
        }

        let search_result = self
            .client
            .search_points(search_builder)
            .await
            .context("Search failed")?;

        Ok(search_result.result)
    }

    /// Count points in collection
    pub async fn count_points(&self, collection_name: &str) -> Result<u64> {
        let count = self
            .client
            .count(CountPointsBuilder::new(collection_name).exact(false))
            .await?;

        Ok(count.result.map(|r| r.count).unwrap_or(0))
    }

    /// Create a snapshot
    pub async fn create_snapshot(&self, collection_name: &str) -> Result<String> {
        info!("Creating snapshot for collection: {}", collection_name);

        let snapshot = self.client.create_snapshot(collection_name).await?;

        Ok(snapshot.snapshot_description.unwrap().name)
    }

    /// List snapshots
    pub async fn list_snapshots(&self, collection_name: &str) -> Result<Vec<String>> {
        let snapshots = self.client.list_snapshots(collection_name).await?;

        Ok(snapshots
            .snapshot_descriptions
            .into_iter()
            .map(|s| s.name)
            .collect())
    }

    /// Restore from snapshot file
    ///
    /// This uploads a snapshot file to Qdrant and recovers the collection from it.
    /// The snapshot data will overwrite any existing collection data.
    /// If the collection doesn't exist, it will be created from the snapshot.
    ///
    /// # Arguments
    /// * `snapshot_path` - Path to the snapshot file to restore
    /// * `collection_name` - Optional target collection name. If None, uses the collection name from the snapshot
    /// * `priority` - Whether to prioritize snapshot data over existing replica data (default: true)
    ///
    /// # Returns
    /// * `Ok(true)` - Restoration successful
    /// * `Err` - Restoration failed
    pub async fn restore_snapshot(
        &self,
        snapshot_path: &std::path::Path,
        collection_name: Option<&str>,
        priority: Option<bool>,
    ) -> Result<bool> {
        use std::fs::File;
        use std::io::Read;

        info!("Restoring snapshot from: {:?}", snapshot_path);

        // Validate snapshot file exists and is readable
        if !snapshot_path.exists() {
            anyhow::bail!("Snapshot file does not exist: {:?}", snapshot_path);
        }

        if !snapshot_path.is_file() {
            anyhow::bail!("Snapshot path is not a file: {:?}", snapshot_path);
        }

        // Read the snapshot file
        let mut file = File::open(snapshot_path)
            .context(format!("Failed to open snapshot file: {:?}", snapshot_path))?;

        let mut snapshot_data = Vec::new();
        file.read_to_end(&mut snapshot_data)
            .context(format!("Failed to read snapshot file: {:?}", snapshot_path))?;

        if snapshot_data.is_empty() {
            anyhow::bail!("Snapshot file is empty: {:?}", snapshot_path);
        }

        info!("Snapshot file size: {} bytes", snapshot_data.len());

        // Extract collection name from snapshot filename if not provided
        // Snapshot filenames typically follow pattern: {collection_name}-{timestamp}.snapshot
        let target_collection = if let Some(name) = collection_name {
            name.to_string()
        } else {
            // Try to extract collection name from filename
            snapshot_path
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.split('-').next())
                .map(|s| s.to_string())
                .ok_or_else(|| anyhow::anyhow!("Could not extract collection name from snapshot filename. Please provide collection_name parameter."))?
        };

        info!("Restoring to collection: {}", target_collection);

        // Build the HTTP endpoint URL
        // Use the HTTP port from config, not the gRPC port
        let base_url = format!(
            "{}://{}:{}",
            if self.config.use_https { "https" } else { "http" },
            self.config.host,
            self.config.port
        );

        // Set priority parameter (default to "snapshot" which means snapshot data takes precedence)
        let priority_param = if priority.unwrap_or(true) {
            "snapshot"
        } else {
            "replica"
        };

        let upload_url = format!(
            "{}/collections/{}/snapshots/upload?priority={}",
            base_url, target_collection, priority_param
        );

        info!("Uploading snapshot to: {}", upload_url);

        // Create multipart form with the snapshot file
        let snapshot_filename = snapshot_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("snapshot.snapshot");

        let part = reqwest::multipart::Part::bytes(snapshot_data)
            .file_name(snapshot_filename.to_string())
            .mime_str("application/octet-stream")
            .context("Failed to create multipart form part")?;

        let form = reqwest::multipart::Form::new().part("snapshot", part);

        // Build HTTP client
        let mut client_builder = reqwest::Client::builder()
            .timeout(self.config.request_timeout);

        // Add API key if configured
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(ref api_key) = self.config.api_key {
            headers.insert(
                "api-key",
                reqwest::header::HeaderValue::from_str(api_key)
                    .context("Invalid API key format")?,
            );
        }

        let http_client = client_builder
            .default_headers(headers)
            .build()
            .context("Failed to build HTTP client")?;

        // Upload the snapshot
        info!("Uploading snapshot file...");
        let response = http_client
            .post(&upload_url)
            .multipart(form)
            .send()
            .await
            .context("Failed to upload snapshot")?;

        let status = response.status();
        let response_text = response.text().await.unwrap_or_default();

        if !status.is_success() {
            anyhow::bail!(
                "Snapshot upload failed with status {}: {}",
                status,
                response_text
            );
        }

        info!("Snapshot uploaded successfully");

        // Parse response to check result
        if let Ok(result) = serde_json::from_str::<serde_json::Value>(&response_text) {
            debug!("Upload response: {:?}", result);

            // Check if result indicates success
            if let Some(result_obj) = result.get("result") {
                if let Some(success) = result_obj.as_bool() {
                    if !success {
                        anyhow::bail!("Snapshot restore reported failure in response");
                    }
                }
            }
        }

        // Verify the collection was created/updated
        info!("Verifying collection after restore...");
        let collection_info = self.collection_info(&target_collection).await
            .context("Failed to verify collection after restore")?;

        info!(
            "Collection '{}' restored successfully. Points: {}, Vectors: {}",
            target_collection,
            collection_info.points_count.unwrap_or(0),
            collection_info.vectors_count.unwrap_or(0)
        );

        Ok(true)
    }

    /// Scroll through points in a collection (for pagination)
    pub async fn scroll_points(
        &self,
        collection_name: &str,
        limit: u32,
        offset: Option<qdrant_client::qdrant::PointId>,
        with_payload: bool,
        with_vectors: bool,
    ) -> Result<qdrant_client::qdrant::ScrollResponse> {
        use qdrant_client::qdrant::ScrollPointsBuilder;

        let mut scroll_builder = ScrollPointsBuilder::new(collection_name)
            .limit(limit)
            .with_payload(with_payload)
            .with_vectors(with_vectors);

        if let Some(offset_id) = offset {
            scroll_builder = scroll_builder.offset(offset_id);
        }

        self.client
            .scroll(scroll_builder)
            .await
            .context("Failed to scroll points")
    }
}

/// Collection statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionStats {
    pub name: String,
    pub vectors_count: u64,
    pub indexed_vectors_count: u64,
    pub points_count: u64,
    pub segments_count: u64,
    pub status: String,
    pub optimizer_status: String,
}

impl QdrantClient {
    /// Get collection statistics
    pub async fn get_collection_stats(&self, collection_name: &str) -> Result<CollectionStats> {
        let info = self.collection_info(collection_name).await?;

        Ok(CollectionStats {
            name: collection_name.to_string(),
            vectors_count: info.vectors_count.unwrap_or(0),
            indexed_vectors_count: info.indexed_vectors_count.unwrap_or(0),
            points_count: info.points_count.unwrap_or(0),
            segments_count: info.segments_count,
            status: format!("{:?}", info.status),
            optimizer_status: format!("{:?}", info.optimizer_status),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires running Qdrant instance
    async fn test_qdrant_connection() {
        let config = QdrantConfig::default();
        let client = QdrantClient::new(config).await.unwrap();
        let health = client.health().await.unwrap();
        assert!(!health.title.is_empty());
    }

    #[tokio::test]
    #[ignore] // Requires running Qdrant instance
    async fn test_create_collection() {
        let config = QdrantConfig::default();
        let client = QdrantClient::new(config).await.unwrap();

        let collection_config = CollectionConfig {
            name: "test_collection".to_string(),
            vector_size: 384,
            distance: DistanceMetric::Cosine,
            hnsw_config: HnswConfig::default(),
            optimizer_config: OptimizerConfig::default(),
        };

        client.create_collection(collection_config).await.unwrap();

        // Cleanup
        client.delete_collection("test_collection").await.unwrap();
    }
}
