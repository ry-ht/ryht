//! Core traits defining interfaces for Cortex components.

use crate::error::Result;
use crate::id::CortexId;
use crate::types::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Trait for storage backends.
#[async_trait]
pub trait Storage: Send + Sync {
    /// Store a project
    async fn store_project(&self, project: &Project) -> Result<()>;

    /// Get a project by ID
    async fn get_project(&self, id: CortexId) -> Result<Option<Project>>;

    /// List all projects
    async fn list_projects(&self) -> Result<Vec<Project>>;

    /// Delete a project
    async fn delete_project(&self, id: CortexId) -> Result<()>;

    /// Store a VFS document (file)
    async fn store_document(&self, document: &VfsDocument) -> Result<()>;

    /// Get a VFS document (file) by ID
    async fn get_document(&self, id: CortexId) -> Result<Option<VfsDocument>>;

    /// List VFS documents (files) in a project
    async fn list_documents(&self, project_id: CortexId) -> Result<Vec<VfsDocument>>;

    /// Delete a VFS document (file)
    async fn delete_document(&self, id: CortexId) -> Result<()>;

    /// Store an embedding
    async fn store_embedding(&self, embedding: &Embedding) -> Result<()>;

    /// Get embeddings for an entity
    async fn get_embeddings(&self, entity_id: CortexId) -> Result<Vec<Embedding>>;

    /// Store an episode
    async fn store_episode(&self, episode: &Episode) -> Result<()>;

    /// Get an episode by ID
    async fn get_episode(&self, id: CortexId) -> Result<Option<Episode>>;

    /// Get system statistics
    async fn get_stats(&self) -> Result<SystemStats>;

    /// Create a new agent session
    async fn create_agent_session(
        &self,
        session_id: String,
        name: String,
        agent_type: String,
    ) -> Result<AgentSession>;

    /// Delete an agent session
    async fn delete_agent_session(&self, session_id: &str) -> Result<()>;

    /// Get an agent session by ID
    async fn get_agent_session(&self, session_id: &str) -> Result<Option<AgentSession>>;

    /// List all agent sessions
    async fn list_agent_sessions(&self) -> Result<Vec<AgentSession>>;
}

/// Trait for VFS document (file) ingestion.
#[async_trait]
pub trait Ingester: Send + Sync {
    /// Ingest a file into the system
    async fn ingest_file(&self, project_id: CortexId, path: &Path) -> Result<VfsDocument>;

    /// Ingest a directory recursively
    async fn ingest_directory(&self, project_id: CortexId, path: &Path) -> Result<Vec<VfsDocument>>;

    /// Update a VFS document (file) when the file changes
    async fn update_document(&self, document_id: CortexId, path: &Path) -> Result<VfsDocument>;
}

/// Trait for text chunking strategies.
pub trait Chunker: Send + Sync {
    /// Chunk text content into semantic chunks
    fn chunk(&self, content: &str) -> Vec<String>;

    /// Get the maximum chunk size
    fn max_chunk_size(&self) -> usize;

    /// Get the overlap between chunks
    fn overlap(&self) -> usize;
}

/// Trait for embedding generation.
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Generate an embedding for text
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Generate embeddings for multiple texts (batch)
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;

    /// Get the model name
    fn model_name(&self) -> &str;

    /// Get the embedding dimension
    fn dimension(&self) -> usize;
}

/// Trait for semantic search.
#[async_trait]
pub trait Searcher: Send + Sync {
    /// Search for similar chunks
    async fn search_chunks(
        &self,
        query: &SearchQuery,
        project_id: Option<CortexId>,
    ) -> Result<Vec<SearchResult<Chunk>>>;

    /// Search for similar documents
    async fn search_documents(
        &self,
        query: &SearchQuery,
        project_id: Option<CortexId>,
    ) -> Result<Vec<SearchResult<Document>>>;

    /// Search for similar episodes
    async fn search_episodes(
        &self,
        query: &SearchQuery,
        project_id: Option<CortexId>,
    ) -> Result<Vec<SearchResult<Episode>>>;
}

/// Trait for memory systems.
#[async_trait]
pub trait Memory: Send + Sync {
    /// Store a memory
    async fn store(&self, episode: Episode) -> Result<CortexId>;

    /// Retrieve relevant memories
    async fn retrieve(&self, query: &str, limit: usize) -> Result<Vec<Episode>>;

    /// Consolidate memories (transfer from working to long-term)
    async fn consolidate(&self) -> Result<()>;

    /// Forget low-importance memories
    async fn forget(&self, threshold: f32) -> Result<usize>;
}

/// Trait for the virtual filesystem.
#[async_trait]
pub trait VirtualFilesystem: Send + Sync {
    /// Read a file from the VFS
    async fn read(&self, path: &str) -> Result<Vec<u8>>;

    /// Write a file to the VFS
    async fn write(&self, path: &str, content: &[u8]) -> Result<()>;

    /// List files in a directory
    async fn list(&self, path: &str) -> Result<Vec<String>>;

    /// Check if a file exists
    async fn exists(&self, path: &str) -> Result<bool>;

    /// Get file metadata
    async fn metadata(&self, path: &str) -> Result<FileMetadata>;

    /// Delete a file
    async fn delete(&self, path: &str) -> Result<()>;
}

/// File metadata for the VFS.
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub size: u64,
    pub is_dir: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub modified_at: chrono::DateTime<chrono::Utc>,
}

// ============================================================================
// Advanced Vector Store Abstraction
// ============================================================================

/// Advanced vector store trait for semantic search operations.
///
/// This trait provides a high-level abstraction over vector databases,
/// supporting both in-memory HNSW and distributed systems like Qdrant.
///
/// # Features
/// - Batch operations with optimal chunking
/// - Async streaming for large datasets
/// - Transactional semantics with rollback
/// - Consistency guarantees
/// - Metadata filtering during search
/// - Multi-tenancy support
/// - Query optimization hints
///
/// # Implementations
/// - `QdrantVectorStore`: Production-grade distributed vector database
/// - `HNSWVectorStore`: In-memory HNSW index for development/small datasets
/// - `HybridVectorStore`: Dual-write for migration scenarios
#[async_trait]
pub trait VectorStore: Send + Sync {
    // ========================================================================
    // Core Operations
    // ========================================================================

    /// Insert a single vector with metadata.
    ///
    /// # Arguments
    /// * `point` - Vector point with ID, embedding, and metadata
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Errors
    /// - `CortexError::InvalidInput` if vector dimension is incorrect
    /// - `CortexError::Storage` if insertion fails
    async fn insert(&self, point: VectorPoint) -> Result<()>;

    /// Insert multiple vectors in a batch operation.
    ///
    /// This method automatically handles chunking for optimal performance.
    /// Batch size is determined by the implementation's configuration.
    ///
    /// # Arguments
    /// * `points` - Vector of points to insert
    ///
    /// # Returns
    /// Result with number of points successfully inserted
    ///
    /// # Performance
    /// Batch operations are 10-100x faster than individual inserts for large datasets.
    async fn insert_batch(&self, points: Vec<VectorPoint>) -> Result<usize>;

    /// Insert vectors with a streaming interface for very large datasets.
    ///
    /// This method accepts a stream of points and processes them in optimal chunks,
    /// providing progress updates through a callback.
    ///
    /// # Arguments
    /// * `stream` - Async stream of vector points
    /// * `progress` - Optional callback for progress updates
    ///
    /// # Returns
    /// Stream statistics including total processed, success count, and errors
    async fn insert_stream(
        &self,
        stream: futures::stream::BoxStream<'_, Result<VectorPoint>>,
        progress: Option<Box<dyn Fn(StreamProgress) + Send + Sync>>,
    ) -> Result<StreamStats>;

    /// Update an existing vector point.
    ///
    /// # Arguments
    /// * `id` - Point ID to update
    /// * `update` - Update specification (vector, metadata, or both)
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Errors
    /// - `CortexError::NotFound` if point doesn't exist
    async fn update(&self, id: &str, update: VectorUpdate) -> Result<()>;

    /// Delete a vector by ID.
    ///
    /// # Arguments
    /// * `id` - Point ID to delete
    ///
    /// # Returns
    /// Result indicating whether the point was found and deleted
    async fn delete(&self, id: &str) -> Result<bool>;

    /// Delete multiple vectors by ID.
    ///
    /// # Arguments
    /// * `ids` - Vector of point IDs to delete
    ///
    /// # Returns
    /// Number of points successfully deleted
    async fn delete_batch(&self, ids: Vec<String>) -> Result<usize>;

    /// Delete vectors matching a filter.
    ///
    /// # Arguments
    /// * `filter` - Filter criteria for deletion
    ///
    /// # Returns
    /// Number of points deleted
    ///
    /// # Warning
    /// This is a potentially destructive operation. Consider using soft deletes instead.
    async fn delete_by_filter(&self, filter: VectorFilter) -> Result<usize>;

    /// Get a vector point by ID.
    ///
    /// # Arguments
    /// * `id` - Point ID to retrieve
    /// * `include_vector` - Whether to include the embedding vector in the result
    ///
    /// # Returns
    /// Optional vector point if found
    async fn get(&self, id: &str, include_vector: bool) -> Result<Option<VectorPoint>>;

    /// Get multiple vectors by ID in a batch operation.
    ///
    /// # Arguments
    /// * `ids` - Vector of point IDs to retrieve
    /// * `include_vectors` - Whether to include embedding vectors
    ///
    /// # Returns
    /// Map of ID to vector point (only includes found points)
    async fn get_batch(
        &self,
        ids: Vec<String>,
        include_vectors: bool,
    ) -> Result<std::collections::HashMap<String, VectorPoint>>;

    // ========================================================================
    // Search Operations
    // ========================================================================

    /// Perform vector similarity search.
    ///
    /// # Arguments
    /// * `request` - Search request with query vector, filters, and parameters
    ///
    /// # Returns
    /// Scored search results ordered by similarity
    async fn search(&self, request: SearchRequest) -> Result<Vec<ScoredPoint>>;

    /// Perform batch search with multiple query vectors.
    ///
    /// More efficient than multiple individual searches.
    ///
    /// # Arguments
    /// * `requests` - Vector of search requests
    ///
    /// # Returns
    /// Vector of search results corresponding to each request
    async fn search_batch(&self, requests: Vec<SearchRequest>) -> Result<Vec<Vec<ScoredPoint>>>;

    /// Recommend similar points based on positive and negative examples.
    ///
    /// This is useful for "more like this" functionality.
    ///
    /// # Arguments
    /// * `request` - Recommendation request with positive/negative examples
    ///
    /// # Returns
    /// Scored recommendations
    async fn recommend(&self, request: RecommendRequest) -> Result<Vec<ScoredPoint>>;

    /// Perform hybrid search combining vector similarity and full-text search.
    ///
    /// # Arguments
    /// * `request` - Hybrid search request
    ///
    /// # Returns
    /// Results combining semantic and keyword matches
    async fn hybrid_search(&self, request: HybridSearchRequest) -> Result<Vec<ScoredPoint>>;

    // ========================================================================
    // Transactional Operations
    // ========================================================================

    /// Begin a transaction for atomic operations.
    ///
    /// # Returns
    /// Transaction handle
    ///
    /// # Notes
    /// Not all implementations support transactions. Check `capabilities()`.
    async fn begin_transaction(&self) -> Result<Box<dyn VectorTransaction>>;

    /// Execute a batch of operations atomically.
    ///
    /// If any operation fails, all operations are rolled back.
    ///
    /// # Arguments
    /// * `operations` - Vector of operations to execute
    ///
    /// # Returns
    /// Result of the batch operation
    async fn execute_batch(&self, operations: Vec<VectorOperation>) -> Result<BatchResult>;

    // ========================================================================
    // Collection Management
    // ========================================================================

    /// Create a new collection/index.
    ///
    /// # Arguments
    /// * `config` - Collection configuration
    ///
    /// # Returns
    /// Result indicating success or failure
    async fn create_collection(&self, config: CollectionConfig) -> Result<()>;

    /// Delete a collection and all its data.
    ///
    /// # Arguments
    /// * `name` - Collection name
    ///
    /// # Returns
    /// Result indicating success or failure
    ///
    /// # Warning
    /// This permanently deletes all vectors in the collection.
    async fn delete_collection(&self, name: &str) -> Result<()>;

    /// List all collections.
    ///
    /// # Returns
    /// Vector of collection information
    async fn list_collections(&self) -> Result<Vec<CollectionInfo>>;

    /// Get collection statistics.
    ///
    /// # Arguments
    /// * `name` - Collection name
    ///
    /// # Returns
    /// Collection statistics
    async fn collection_stats(&self, name: &str) -> Result<CollectionStats>;

    /// Update collection configuration.
    ///
    /// # Arguments
    /// * `name` - Collection name
    /// * `update` - Configuration updates
    ///
    /// # Returns
    /// Result indicating success or failure
    async fn update_collection(&self, name: &str, update: CollectionUpdate) -> Result<()>;

    // ========================================================================
    // Index Management
    // ========================================================================

    /// Create a payload index for efficient filtering.
    ///
    /// # Arguments
    /// * `collection` - Collection name
    /// * `field` - Payload field name
    /// * `index_type` - Type of index to create
    ///
    /// # Returns
    /// Result indicating success or failure
    async fn create_payload_index(
        &self,
        collection: &str,
        field: &str,
        index_type: PayloadIndexType,
    ) -> Result<()>;

    /// Delete a payload index.
    ///
    /// # Arguments
    /// * `collection` - Collection name
    /// * `field` - Payload field name
    ///
    /// # Returns
    /// Result indicating success or failure
    async fn delete_payload_index(&self, collection: &str, field: &str) -> Result<()>;

    // ========================================================================
    // Persistence and Snapshots
    // ========================================================================

    /// Create a snapshot of the vector store.
    ///
    /// # Arguments
    /// * `name` - Snapshot name
    ///
    /// # Returns
    /// Snapshot metadata
    async fn create_snapshot(&self, name: &str) -> Result<SnapshotInfo>;

    /// Restore from a snapshot.
    ///
    /// # Arguments
    /// * `name` - Snapshot name
    ///
    /// # Returns
    /// Result indicating success or failure
    async fn restore_snapshot(&self, name: &str) -> Result<()>;

    /// List available snapshots.
    ///
    /// # Returns
    /// Vector of snapshot information
    async fn list_snapshots(&self) -> Result<Vec<SnapshotInfo>>;

    // ========================================================================
    // Consistency and Health
    // ========================================================================

    /// Check if the vector store is healthy and reachable.
    ///
    /// # Returns
    /// Health status
    async fn health_check(&self) -> Result<HealthStatus>;

    /// Verify data consistency.
    ///
    /// # Returns
    /// Consistency report
    async fn verify_consistency(&self) -> Result<ConsistencyReport>;

    /// Repair inconsistencies if possible.
    ///
    /// # Arguments
    /// * `dry_run` - If true, only report what would be repaired
    ///
    /// # Returns
    /// Repair report
    async fn repair(&self, dry_run: bool) -> Result<RepairReport>;

    // ========================================================================
    // Query Optimization
    // ========================================================================

    /// Get optimization hints for a query.
    ///
    /// # Arguments
    /// * `request` - Search request to optimize
    ///
    /// # Returns
    /// Optimization recommendations
    async fn get_query_plan(&self, request: &SearchRequest) -> Result<QueryPlan>;

    /// Analyze query performance.
    ///
    /// # Arguments
    /// * `request` - Search request to analyze
    ///
    /// # Returns
    /// Performance metrics and suggestions
    async fn explain_query(&self, request: &SearchRequest) -> Result<QueryExplanation>;

    // ========================================================================
    // Capabilities and Metadata
    // ========================================================================

    /// Get vector store capabilities.
    ///
    /// # Returns
    /// Capability information
    fn capabilities(&self) -> VectorStoreCapabilities;

    /// Get vector store metadata.
    ///
    /// # Returns
    /// Metadata including version, type, etc.
    fn metadata(&self) -> VectorStoreMetadata;
}

/// Transaction interface for atomic vector operations.
#[async_trait]
pub trait VectorTransaction: Send + Sync {
    /// Insert a point within the transaction.
    async fn insert(&mut self, point: VectorPoint) -> Result<()>;

    /// Delete a point within the transaction.
    async fn delete(&mut self, id: &str) -> Result<()>;

    /// Update a point within the transaction.
    async fn update(&mut self, id: &str, update: VectorUpdate) -> Result<()>;

    /// Commit the transaction.
    async fn commit(self: Box<Self>) -> Result<()>;

    /// Rollback the transaction.
    async fn rollback(self: Box<Self>) -> Result<()>;
}

// ============================================================================
// Data Structures
// ============================================================================

/// A point in vector space with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorPoint {
    /// Unique identifier
    pub id: String,

    /// Collection/namespace this point belongs to
    pub collection: String,

    /// Dense embedding vector
    pub vector: Vec<f32>,

    /// Optional sparse vector for hybrid search
    pub sparse_vector: Option<SparseVector>,

    /// Metadata payload
    pub payload: VectorPayload,

    /// Creation timestamp
    #[serde(default = "chrono::Utc::now")]
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last update timestamp
    #[serde(default = "chrono::Utc::now")]
    pub updated_at: chrono::DateTime<chrono::Utc>,

    /// Version for optimistic locking
    #[serde(default)]
    pub version: u64,
}

/// Sparse vector representation for hybrid search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseVector {
    /// Indices of non-zero values
    pub indices: Vec<u32>,

    /// Non-zero values
    pub values: Vec<f32>,
}

/// Payload containing metadata for a vector point.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorPayload {
    /// Entity ID from primary storage (SurrealDB)
    pub entity_id: String,

    /// Entity type
    pub entity_type: String,

    /// Workspace/tenant ID for multi-tenancy
    pub workspace_id: Option<String>,

    /// Additional metadata fields
    #[serde(flatten)]
    pub fields: std::collections::HashMap<String, serde_json::Value>,
}

/// Update specification for vector points.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VectorUpdate {
    /// Update only the vector
    Vector(Vec<f32>),

    /// Update only metadata
    Payload(VectorPayload),

    /// Update both vector and metadata
    Full {
        vector: Vec<f32>,
        payload: VectorPayload,
    },
}

/// Search request parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    /// Collection to search in
    pub collection: String,

    /// Query vector
    pub vector: Vec<f32>,

    /// Number of results to return
    pub limit: usize,

    /// Minimum score threshold
    pub score_threshold: Option<f32>,

    /// Metadata filters
    pub filter: Option<VectorFilter>,

    /// Include vector data in results
    pub with_vector: bool,

    /// Include payload in results
    pub with_payload: bool,

    /// Query optimization hints
    pub hints: Option<QueryHints>,

    /// Offset for pagination
    pub offset: Option<usize>,
}

/// Vector filter for metadata-based filtering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorFilter {
    /// Conditions that must all be true (AND)
    #[serde(default)]
    pub must: Vec<Condition>,

    /// At least one condition must be true (OR)
    #[serde(default)]
    pub should: Vec<Condition>,

    /// Conditions that must not be true (NOT)
    #[serde(default)]
    pub must_not: Vec<Condition>,
}

/// Individual filter condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    /// Match exact value
    Match {
        field: String,
        value: serde_json::Value,
    },

    /// Range condition
    Range {
        field: String,
        gte: Option<serde_json::Value>,
        lte: Option<serde_json::Value>,
        gt: Option<serde_json::Value>,
        lt: Option<serde_json::Value>,
    },

    /// Geo-spatial radius search
    GeoRadius {
        field: String,
        lat: f64,
        lon: f64,
        radius_meters: f64,
    },

    /// Full-text search
    FullText {
        field: String,
        query: String,
    },

    /// Check if field exists
    Exists {
        field: String,
    },

    /// Check if value is in a list
    In {
        field: String,
        values: Vec<serde_json::Value>,
    },
}

/// Search result with score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredPoint {
    /// Point ID
    pub id: String,

    /// Similarity score (higher is better)
    pub score: f32,

    /// Vector (if requested)
    pub vector: Option<Vec<f32>>,

    /// Payload (if requested)
    pub payload: Option<VectorPayload>,

    /// Version
    pub version: u64,
}

/// Recommendation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendRequest {
    /// Collection to search in
    pub collection: String,

    /// Positive examples (similar to these)
    pub positive: Vec<String>,

    /// Negative examples (dissimilar to these)
    pub negative: Vec<String>,

    /// Number of results
    pub limit: usize,

    /// Optional filter
    pub filter: Option<VectorFilter>,

    /// Strategy for combining examples
    pub strategy: RecommendStrategy,
}

/// Strategy for combining positive/negative examples.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendStrategy {
    /// Average positive, subtract negative
    AverageVector,

    /// Best score among positive examples
    BestScore,
}

/// Hybrid search combining vector and keyword search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchRequest {
    /// Collection to search in
    pub collection: String,

    /// Dense vector query
    pub vector: Vec<f32>,

    /// Sparse vector query (keywords)
    pub sparse_vector: Option<SparseVector>,

    /// Full-text query
    pub text_query: Option<String>,

    /// Number of results
    pub limit: usize,

    /// Weight for dense vector (0.0-1.0)
    pub dense_weight: f32,

    /// Weight for sparse vector (0.0-1.0)
    pub sparse_weight: f32,

    /// Weight for text query (0.0-1.0)
    pub text_weight: f32,

    /// Optional filter
    pub filter: Option<VectorFilter>,
}

/// Query optimization hints.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueryHints {
    /// Use exact search (no approximation)
    pub exact: bool,

    /// HNSW ef parameter (higher = more accurate, slower)
    pub hnsw_ef: Option<usize>,

    /// Enable quantization for speed
    pub quantization: bool,

    /// Rescore top results with full precision
    pub rescore: bool,
}

/// Vector operation for batch execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VectorOperation {
    Insert { point: VectorPoint },
    Update { id: String, update: VectorUpdate },
    Delete { id: String },
}

/// Result of batch operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    /// Number of successful operations
    pub success_count: usize,

    /// Number of failed operations
    pub failed_count: usize,

    /// Errors for failed operations
    pub errors: Vec<OperationError>,
}

/// Error for a specific operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationError {
    /// Index of the operation
    pub index: usize,

    /// Error message
    pub error: String,
}

/// Collection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionConfig {
    /// Collection name
    pub name: String,

    /// Vector dimension
    pub vector_size: usize,

    /// Distance metric
    pub distance: DistanceMetric,

    /// HNSW configuration
    pub hnsw_config: Option<HnswConfig>,

    /// Quantization configuration
    pub quantization: Option<QuantizationConfig>,

    /// Replication factor
    pub replication_factor: Option<usize>,

    /// Number of shards
    pub shard_number: Option<usize>,

    /// Store vectors on disk
    pub on_disk: bool,
}

/// Distance metric for similarity calculation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistanceMetric {
    Cosine,
    Euclidean,
    DotProduct,
}

/// HNSW index configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswConfig {
    /// Number of edges per node
    pub m: usize,

    /// Size of the dynamic candidate list
    pub ef_construct: usize,

    /// Full scan threshold
    pub full_scan_threshold: Option<usize>,
}

/// Quantization configuration for memory reduction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QuantizationConfig {
    Scalar {
        /// Quantization type
        quantile: f32,
        /// Always use quantized vectors
        always_ram: bool,
    },
    Product {
        /// Number of segments
        segments: usize,
        /// Compression ratio
        compression: f32,
    },
}

/// Collection information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionInfo {
    pub name: String,
    pub vector_size: usize,
    pub points_count: u64,
    pub status: CollectionStatus,
}

/// Collection status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionStatus {
    Green,  // Healthy
    Yellow, // Degraded
    Red,    // Unhealthy
}

/// Collection statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionStats {
    pub name: String,
    pub vectors_count: u64,
    pub indexed_vectors_count: u64,
    pub points_count: u64,
    pub segments_count: u64,
    pub disk_size_bytes: u64,
    pub ram_size_bytes: u64,
    pub config: CollectionConfig,
}

/// Collection update specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionUpdate {
    /// Update optimizer config
    pub optimizer_config: Option<OptimizerConfig>,

    /// Update HNSW parameters
    pub hnsw_config: Option<HnswConfig>,

    /// Update quantization
    pub quantization: Option<QuantizationConfig>,
}

/// Optimizer configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizerConfig {
    /// Deleted threshold
    pub deleted_threshold: f64,

    /// Vacuum min vector number
    pub vacuum_min_vector_number: usize,

    /// Default segment number
    pub default_segment_number: usize,
}

/// Payload index type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayloadIndexType {
    Keyword,
    Integer,
    Float,
    Geo,
    Text,
    Datetime,
}

/// Snapshot information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotInfo {
    pub name: String,
    pub size_bytes: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub collections: Vec<String>,
}

/// Health status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub version: String,
    pub message: Option<String>,
}

/// Consistency report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyReport {
    pub consistent: bool,
    pub total_points: u64,
    pub inconsistent_points: Vec<String>,
    pub missing_payloads: Vec<String>,
    pub orphaned_vectors: Vec<String>,
}

/// Repair report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairReport {
    pub dry_run: bool,
    pub repaired_count: usize,
    pub failed_count: usize,
    pub actions_taken: Vec<String>,
}

/// Query execution plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPlan {
    pub estimated_cost: f64,
    pub index_used: bool,
    pub filter_efficiency: f64,
    pub recommendations: Vec<String>,
}

/// Query explanation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExplanation {
    pub query_time_ms: f64,
    pub candidates_checked: u64,
    pub results_returned: usize,
    pub index_efficiency: f64,
    pub suggestions: Vec<String>,
}

/// Vector store capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreCapabilities {
    /// Supports transactions
    pub transactions: bool,

    /// Supports streaming inserts
    pub streaming: bool,

    /// Supports hybrid search
    pub hybrid_search: bool,

    /// Supports geo-spatial search
    pub geo_search: bool,

    /// Supports full-text search
    pub full_text_search: bool,

    /// Supports snapshots
    pub snapshots: bool,

    /// Supports replication
    pub replication: bool,

    /// Supports sharding
    pub sharding: bool,

    /// Maximum vector dimension
    pub max_vector_dimension: usize,

    /// Maximum points per collection
    pub max_points: Option<u64>,
}

/// Vector store metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreMetadata {
    /// Store type (e.g., "qdrant", "hnsw")
    pub store_type: String,

    /// Version
    pub version: String,

    /// Total collections
    pub collections_count: usize,

    /// Total points across all collections
    pub total_points: u64,

    /// Memory usage in bytes
    pub memory_bytes: u64,

    /// Disk usage in bytes
    pub disk_bytes: u64,
}

/// Stream progress information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamProgress {
    pub processed: usize,
    pub successful: usize,
    pub failed: usize,
    pub current_chunk: usize,
    pub total_chunks: Option<usize>,
}

/// Stream statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStats {
    pub total_processed: usize,
    pub successful: usize,
    pub failed: usize,
    pub duration_ms: u64,
    pub throughput_per_sec: f64,
}
