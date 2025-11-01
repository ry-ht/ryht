//! Semantic search system for Cortex with multi-agent coordination.
//!
//! This crate provides comprehensive semantic search capabilities including:
//! - Multiple embedding providers (OpenAI, ONNX Runtime, Ollama)
//! - Qdrant vector database for production-ready search
//! - Advanced features: quantization, hybrid search, batch operations
//! - Query expansion and refinement
//! - Result re-ranking and scoring
//! - **Multi-agent coordination and federated search**
//! - **Agent-specific namespaces and memory pools**
//! - **Priority-based search queuing**
//! - **Cross-agent knowledge retrieval with access control**
//! - **Context engineering for RAG (compression, HyDE)**
//! - **Evaluation metrics (NDCG, MRR, Precision@K)**
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
//! - **Agent Coordinator**: Manages multi-agent registration and resources
//! - **Search Orchestrator**: Coordinates federated search across agents
//! - **Memory Pools**: Shared semantic memory with access control
//!
//! # Single-Agent Example
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
//!
//! # Multi-Agent Example
//!
//! ```no_run
//! use cortex_semantic::prelude::*;
//! use cortex_semantic::config::SemanticConfig;
//! use std::sync::Arc;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create agent coordinator
//! let coordinator = Arc::new(AgentCoordinator::new());
//!
//! // Register agents
//! let worker1 = coordinator.register_agent(
//!     "worker-1",
//!     AgentRole::Worker,
//!     vec!["rust".to_string()]
//! ).await?;
//!
//! let worker2 = coordinator.register_agent(
//!     "worker-2",
//!     AgentRole::Worker,
//!     vec!["python".to_string()]
//! ).await?;
//!
//! // Create search orchestrator
//! let orchestrator = Arc::new(SearchOrchestrator::new(coordinator.clone()));
//!
//! // Create and register search engines for agents
//! let config = SemanticConfig::default();
//! let engine1 = Arc::new(SemanticSearchEngine::new(config.clone()).await?);
//! let engine2 = Arc::new(SemanticSearchEngine::new(config).await?);
//!
//! orchestrator.register_engine("worker-1", engine1);
//! orchestrator.register_engine("worker-2", engine2);
//!
//! // Perform federated search across all agents
//! let (results, stats) = orchestrator.federated_search(
//!     &"orchestrator".to_string(),
//!     "machine learning",
//!     10,
//!     None,
//!     SearchPriority::Normal,
//! ).await?;
//!
//! println!("Found {} results from {} agents", results.len(), stats.agents_queried);
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
pub mod agent;
pub mod orchestration;
pub mod context;
pub mod hyde;
pub mod eval;

pub use config::{
    SemanticConfig, EmbeddingProviderConfig, IndexConfig, SearchConfig, QdrantConfig,
    VectorStoreConfig, VectorStoreBackend, QuantizationType,
};
pub use providers::{EmbeddingProvider, OpenAIProvider, ONNXProvider, OllamaProvider, MockProvider};
pub use qdrant::{VectorIndex, QdrantVectorStore, QdrantMetrics, IndexStats, SearchResult as QdrantSearchResult, SearchFilter as QdrantSearchFilter, SparseVector};
pub use query::{QueryProcessor, QueryExpander, QueryIntent, QueryDecomposer, SubQuery, AnswerType, QueryDependencyGraph};
pub use search::{SemanticSearchEngine, SearchResult, SearchFilter};
pub use ranking::{
    Ranker, RankingStrategy, ScoringAlgorithm, MMRReranker, PersonalizedRanker,
    AdvancedRanker, PersonalizationConfig, DiversityConfig,
};
pub use context::{ContextCompressor, CompressionConfig, ContextChunk, CompressedContext, TokenAwareChunker};
pub use hyde::{HydeProcessor, HydeConfig, HypotheticalDocument, HydeResult};
pub use eval::{MetricEvaluator, QueryEvaluation, Metrics, AggregatedMetrics, MetricsTimeSeries};
pub use types::{Vector, DocumentId, EmbeddingModel, EntityType, AgentSearchResult, MultiAgentSearchStats, FederatedSearchConfig};
pub use error::{SemanticError, Result};
pub use agent::{
    AgentCoordinator, AgentContext, AgentId, AgentRole, AgentMetrics, Namespace,
    MemoryPool, MemoryEntry, AccessPolicy, AccessControl, SearchPriority,
    PrioritizedSearchRequest, SearchQueue,
};
pub use orchestration::{SearchOrchestrator, SearchOrchestratorStats, AggregationStrategy, DeduplicationStrategy};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::config::{
        SemanticConfig, EmbeddingProviderConfig, QdrantConfig, VectorStoreBackend,
    };
    pub use crate::providers::{EmbeddingProvider, OpenAIProvider};
    pub use crate::qdrant::{VectorIndex, QdrantVectorStore};
    pub use crate::search::{SemanticSearchEngine, SearchResult};
    pub use crate::types::{Vector, DocumentId, AgentSearchResult, MultiAgentSearchStats, FederatedSearchConfig};
    pub use crate::error::{SemanticError, Result};
    pub use crate::agent::{
        AgentCoordinator, AgentContext, AgentRole, AgentMetrics,
        MemoryPool, AccessPolicy, SearchPriority,
    };
    pub use crate::orchestration::{SearchOrchestrator, SearchOrchestratorStats};
    pub use crate::context::{ContextCompressor, CompressionConfig};
    pub use crate::hyde::{HydeProcessor, HydeConfig};
    pub use crate::eval::{MetricEvaluator, QueryEvaluation};
    pub use crate::ranking::{MMRReranker, AdvancedRanker};
    pub use crate::query::{QueryDecomposer, SubQuery};
}
