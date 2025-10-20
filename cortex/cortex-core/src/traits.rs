//! Core traits defining interfaces for Cortex components.

use crate::error::Result;
use crate::id::CortexId;
use crate::types::*;
use async_trait::async_trait;
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

    /// Store a document
    async fn store_document(&self, document: &Document) -> Result<()>;

    /// Get a document by ID
    async fn get_document(&self, id: CortexId) -> Result<Option<Document>>;

    /// List documents in a project
    async fn list_documents(&self, project_id: CortexId) -> Result<Vec<Document>>;

    /// Delete a document
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
}

/// Trait for document ingestion.
#[async_trait]
pub trait Ingester: Send + Sync {
    /// Ingest a file into the system
    async fn ingest_file(&self, project_id: CortexId, path: &Path) -> Result<Document>;

    /// Ingest a directory recursively
    async fn ingest_directory(&self, project_id: CortexId, path: &Path) -> Result<Vec<Document>>;

    /// Update a document when the file changes
    async fn update_document(&self, document_id: CortexId, path: &Path) -> Result<Document>;
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
