//! Semantic search system for Cortex.
//!
//! This crate provides comprehensive semantic search capabilities including:
//! - Multiple embedding providers (OpenAI, ONNX Runtime, Ollama)
//! - Qdrant vector database for production-ready search
//! - Advanced features: quantization, hybrid search, batch operations
//! - Query expansion and refinement
//! - Result re-ranking and scoring
//!
//! # Architecture
//!
//! The semantic search system consists of several key components:
//!
//! - **Embedding Providers**: Generate vector embeddings from text
//! - **Vector Store**: Qdrant vector database for fast similarity search
//! - **Query Processor**: Parse and expand natural language queries
//! - **Search Engine**: Orchestrates search operations
//! - **Ranking System**: Scores and re-ranks results
//!
//! # Example
//!
//! ```no_run
//! use cortex_semantic::prelude::*;
//! use cortex_semantic::config::SemanticConfig;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = SemanticConfig::default();
//! let engine = SemanticSearchEngine::new(config).await?;
//!
//! // Index documents
//! engine.index_document("doc1", "This is a test document about machine learning").await?;
//!
//! // Search
//! let results = engine.search("What is ML?", 10).await?;
//!
//! for result in results {
//!     println!("Document: {}, Score: {}", result.id, result.score);
//! }
//! # Ok(())
//! # }
//! ```

pub mod config;
pub mod providers;
pub mod query;
pub mod search;
pub mod ranking;
pub mod cache;
pub mod types;
pub mod error;
pub mod qdrant;

pub use config::{
    SemanticConfig, EmbeddingProviderConfig, IndexConfig, SearchConfig, QdrantConfig,
    VectorStoreConfig, VectorStoreBackend, QuantizationType,
};
pub use providers::{EmbeddingProvider, OpenAIProvider, ONNXProvider, OllamaProvider, MockProvider};
pub use qdrant::{VectorIndex, QdrantVectorStore, QdrantMetrics, IndexStats, SearchResult as QdrantSearchResult, SearchFilter as QdrantSearchFilter, SparseVector};
pub use query::{QueryProcessor, QueryExpander, QueryIntent};
pub use search::{SemanticSearchEngine, SearchResult, SearchFilter};
pub use ranking::{Ranker, RankingStrategy, ScoringAlgorithm};
pub use types::{Vector, DocumentId, EmbeddingModel, EntityType};
pub use error::{SemanticError, Result};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::config::{
        SemanticConfig, EmbeddingProviderConfig, QdrantConfig, VectorStoreBackend,
    };
    pub use crate::providers::{EmbeddingProvider, OpenAIProvider};
    pub use crate::qdrant::{VectorIndex, QdrantVectorStore};
    pub use crate::search::{SemanticSearchEngine, SearchResult};
    pub use crate::types::{Vector, DocumentId};
    pub use crate::error::{SemanticError, Result};
}
