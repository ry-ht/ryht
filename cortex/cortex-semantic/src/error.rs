//! Error types for semantic search.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, SemanticError>;

#[derive(Debug, Error)]
pub enum SemanticError {
    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Index error: {0}")]
    Index(String),

    #[error("Search error: {0}")]
    Search(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Database error: {0}")]
    Database(#[from] surrealdb::Error),

    #[error("Invalid dimension: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    #[error("Model not loaded: {0}")]
    ModelNotLoaded(String),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Concurrent operation error: {0}")]
    Concurrent(String),
}
