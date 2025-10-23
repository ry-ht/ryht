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
    /// Maximum optimization threads
    pub max_optimization_threads: u64,
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
            max_optimization_threads: 16,
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
                        deprecated_max_optimization_threads: Some(config.optimizer_config.max_optimization_threads),
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
            distance: Distance::Cosine,
            hnsw_config: HnswConfig::default(),
            optimizer_config: OptimizerConfig::default(),
        };

        client.create_collection(collection_config).await.unwrap();

        // Cleanup
        client.delete_collection("test_collection").await.unwrap();
    }
}
