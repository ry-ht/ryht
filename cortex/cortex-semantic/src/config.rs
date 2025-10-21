//! Configuration for semantic search system.

use crate::types::SimilarityMetric;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration for the semantic search system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticConfig {
    pub embedding: EmbeddingProviderConfig,
    pub index: IndexConfig,
    pub search: SearchConfig,
    pub cache: CacheConfig,
}

impl Default for SemanticConfig {
    fn default() -> Self {
        Self {
            embedding: EmbeddingProviderConfig::default(),
            index: IndexConfig::default(),
            search: SearchConfig::default(),
            cache: CacheConfig::default(),
        }
    }
}

/// Embedding provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingProviderConfig {
    /// Primary provider (openai, onnx, ollama, mock)
    pub primary_provider: String,

    /// Fallback providers in order
    pub fallback_providers: Vec<String>,

    /// OpenAI configuration
    pub openai: OpenAIConfig,

    /// ONNX configuration
    pub onnx: ONNXConfig,

    /// Ollama configuration
    pub ollama: OllamaConfig,

    /// Batch size for embedding generation
    pub batch_size: usize,

    /// Request timeout in seconds
    pub timeout_seconds: u64,

    /// Maximum retries
    pub max_retries: usize,
}

impl Default for EmbeddingProviderConfig {
    fn default() -> Self {
        Self {
            primary_provider: "openai".to_string(),
            fallback_providers: vec!["onnx".to_string()],
            openai: OpenAIConfig::default(),
            onnx: ONNXConfig::default(),
            ollama: OllamaConfig::default(),
            batch_size: 32,
            timeout_seconds: 30,
            max_retries: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConfig {
    /// API key (can be set via OPENAI_API_KEY env var)
    pub api_key: Option<String>,

    /// Model name
    pub model: String,

    /// API endpoint
    pub endpoint: String,

    /// Dimension override
    pub dimension: Option<usize>,
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("OPENAI_API_KEY").ok(),
            model: "text-embedding-3-small".to_string(),
            endpoint: "https://api.openai.com/v1/embeddings".to_string(),
            dimension: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ONNXConfig {
    /// Path to ONNX model file
    pub model_path: Option<PathBuf>,

    /// Model name
    pub model_name: String,

    /// Dimension
    pub dimension: usize,

    /// Use GPU if available
    pub use_gpu: bool,
}

impl Default for ONNXConfig {
    fn default() -> Self {
        Self {
            model_path: None,
            model_name: "all-MiniLM-L6-v2".to_string(),
            dimension: 384,
            use_gpu: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    /// Ollama server URL
    pub endpoint: String,

    /// Model name
    pub model: String,

    /// Dimension
    pub dimension: usize,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:11434".to_string(),
            model: "nomic-embed-text".to_string(),
            dimension: 768,
        }
    }
}

/// Vector index configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    /// HNSW M parameter (number of bi-directional links)
    pub hnsw_m: usize,

    /// HNSW ef_construction parameter
    pub hnsw_ef_construction: usize,

    /// HNSW ef_search parameter
    pub hnsw_ef_search: usize,

    /// Similarity metric
    pub similarity_metric: SimilarityMetric,

    /// Index persistence path
    pub persist_path: Option<PathBuf>,

    /// Auto-save interval in seconds (0 = disabled)
    pub auto_save_interval_seconds: u64,

    /// Maximum index size (number of vectors)
    pub max_index_size: usize,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            hnsw_m: 16,                     // Optimized: Lower M for faster search (was 32)
            hnsw_ef_construction: 200,      // Optimized: Higher construction for better recall
            hnsw_ef_search: 50,             // Optimized: Lower search for faster queries (was 100)
            similarity_metric: SimilarityMetric::Cosine,
            persist_path: None,
            auto_save_interval_seconds: 300, // 5 minutes
            max_index_size: 1_000_000,
        }
    }
}

/// Search configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Default result limit
    pub default_limit: usize,

    /// Maximum result limit
    pub max_limit: usize,

    /// Default similarity threshold (0.0 - 1.0)
    pub default_threshold: f32,

    /// Enable query expansion
    pub enable_query_expansion: bool,

    /// Enable hybrid search (keyword + semantic)
    pub enable_hybrid_search: bool,

    /// Hybrid search keyword weight (0.0 - 1.0)
    pub hybrid_keyword_weight: f32,

    /// Enable result re-ranking
    pub enable_reranking: bool,

    /// Search timeout in milliseconds
    pub timeout_ms: u64,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            default_limit: 10,
            max_limit: 100,
            default_threshold: 0.5,
            enable_query_expansion: true,
            enable_hybrid_search: true,
            hybrid_keyword_weight: 0.3,
            enable_reranking: true,
            timeout_ms: 1000,
        }
    }
}

/// Cache configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable embedding cache
    pub enable_embedding_cache: bool,

    /// Embedding cache size (number of entries)
    pub embedding_cache_size: u64,

    /// Embedding cache TTL in seconds
    pub embedding_cache_ttl_seconds: u64,

    /// Enable query cache
    pub enable_query_cache: bool,

    /// Query cache size (number of entries)
    pub query_cache_size: u64,

    /// Query cache TTL in seconds
    pub query_cache_ttl_seconds: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enable_embedding_cache: true,
            embedding_cache_size: 10_000,
            embedding_cache_ttl_seconds: 3600, // 1 hour
            enable_query_cache: true,
            query_cache_size: 1_000,
            query_cache_ttl_seconds: 300, // 5 minutes
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SemanticConfig::default();
        assert_eq!(config.embedding.primary_provider, "openai");
        assert_eq!(config.index.hnsw_m, 32);
        assert_eq!(config.search.default_limit, 10);
    }

    #[test]
    fn test_serialization() {
        let config = SemanticConfig::default();
        let toml = toml::to_string(&config).unwrap();
        let deserialized: SemanticConfig = toml::from_str(&toml).unwrap();
        assert_eq!(config.embedding.primary_provider, deserialized.embedding.primary_provider);
    }
}
