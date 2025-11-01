//! Core types for semantic search.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A vector embedding.
pub type Vector = Vec<f32>;

/// Document identifier.
pub type DocumentId = String;

/// Embedding model identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EmbeddingModel {
    pub provider: String,
    pub model_name: String,
    pub dimension: usize,
}

impl EmbeddingModel {
    pub fn new(provider: impl Into<String>, model_name: impl Into<String>, dimension: usize) -> Self {
        Self {
            provider: provider.into(),
            model_name: model_name.into(),
            dimension,
        }
    }

    pub fn openai_small() -> Self {
        Self::new("openai", "text-embedding-3-small", 1536)
    }

    pub fn openai_large() -> Self {
        Self::new("openai", "text-embedding-3-large", 3072)
    }

    pub fn onnx_minilm() -> Self {
        Self::new("onnx", "all-MiniLM-L6-v2", 384)
    }

    pub fn ollama(model: &str, dimension: usize) -> Self {
        Self::new("ollama", model, dimension)
    }
}

/// Type of searchable entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Document,
    Chunk,
    Symbol,
    Episode,
    Code,
}

/// Indexed document with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedDocument {
    pub id: DocumentId,
    pub entity_type: EntityType,
    pub content: String,
    pub embedding: Vector,
    pub model: EmbeddingModel,
    pub metadata: HashMap<String, String>,
    pub indexed_at: chrono::DateTime<chrono::Utc>,
}

/// Similarity metric for vector comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimilarityMetric {
    Cosine,
    Euclidean,
    DotProduct,
}

impl SimilarityMetric {
    /// Calculate similarity between two vectors.
    pub fn calculate(&self, a: &[f32], b: &[f32]) -> f32 {
        match self {
            Self::Cosine => cosine_similarity(a, b),
            Self::Euclidean => -euclidean_distance(a, b), // Negative for consistency (higher is better)
            Self::DotProduct => dot_product(a, b),
        }
    }
}

/// Calculate cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());

    let dot = dot_product(a, b);
    let norm_a = (a.iter().map(|x| x * x).sum::<f32>()).sqrt();
    let norm_b = (b.iter().map(|x| x * x).sum::<f32>()).sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

/// Calculate dot product of two vectors.
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Calculate Euclidean distance between two vectors.
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    (a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum::<f32>()).sqrt()
}

/// Normalize a vector to unit length.
pub fn normalize(v: &mut [f32]) {
    let norm = (v.iter().map(|x| x * x).sum::<f32>()).sqrt();
    if norm > 0.0 {
        v.iter_mut().for_each(|x| *x /= norm);
    }
}

/// Agent-aware search result with cross-agent metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSearchResult {
    pub id: DocumentId,
    pub entity_type: EntityType,
    pub content: String,
    pub score: f32,
    pub metadata: HashMap<String, String>,
    pub explanation: Option<String>,
    /// Agent that indexed this document
    pub indexed_by: Option<String>,
    /// Namespace where document exists
    pub namespace: Option<String>,
    /// Cross-agent relevance score
    pub cross_agent_score: Option<f32>,
    /// Embedding vector for similarity calculations (optional for deduplication)
    pub embedding: Option<Vector>,
}

/// Multi-agent search statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MultiAgentSearchStats {
    /// Total agents queried
    pub agents_queried: usize,
    /// Namespaces searched
    pub namespaces_searched: Vec<String>,
    /// Results per agent
    pub results_per_agent: HashMap<String, usize>,
    /// Total search time across all agents
    pub total_search_time_ms: u64,
    /// Deduplication count
    pub deduplicated_count: usize,
    /// Cross-agent communication overhead (ms)
    pub communication_overhead_ms: u64,
}

/// Federated search configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedSearchConfig {
    /// Maximum namespaces to search
    pub max_namespaces: usize,
    /// Enable result deduplication
    pub deduplicate_results: bool,
    /// Deduplication similarity threshold
    pub dedup_threshold: f32,
    /// Enable cross-agent result aggregation
    pub aggregate_results: bool,
    /// Weight for cross-namespace results
    pub cross_namespace_weight: f32,
    /// Maximum concurrent searches to prevent DoS (rate limiting)
    pub max_concurrent_searches: Option<usize>,
}

impl Default for FederatedSearchConfig {
    fn default() -> Self {
        Self {
            max_namespaces: 10,
            deduplicate_results: true,
            dedup_threshold: 0.95,
            aggregate_results: true,
            cross_namespace_weight: 0.8,
            max_concurrent_searches: Some(10),  // Default rate limit to prevent DoS
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert_relative_eq!(cosine_similarity(&a, &b), 1.0, epsilon = 1e-6);

        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        assert_relative_eq!(cosine_similarity(&a, &b), 0.0, epsilon = 1e-6);

        let a = vec![1.0, 1.0, 0.0];
        let b = vec![1.0, 1.0, 0.0];
        assert_relative_eq!(cosine_similarity(&a, &b), 1.0, epsilon = 1e-6);
    }

    #[test]
    fn test_dot_product() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        assert_relative_eq!(dot_product(&a, &b), 32.0, epsilon = 1e-6);
    }

    #[test]
    fn test_euclidean_distance() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![3.0, 4.0, 0.0];
        assert_relative_eq!(euclidean_distance(&a, &b), 5.0, epsilon = 1e-6);
    }

    #[test]
    fn test_normalize() {
        let mut v = vec![3.0, 4.0, 0.0];
        normalize(&mut v);
        assert_relative_eq!(v[0], 0.6, epsilon = 1e-6);
        assert_relative_eq!(v[1], 0.8, epsilon = 1e-6);
        assert_relative_eq!(v[2], 0.0, epsilon = 1e-6);
    }

    #[test]
    fn test_embedding_model() {
        let model = EmbeddingModel::openai_small();
        assert_eq!(model.provider, "openai");
        assert_eq!(model.dimension, 1536);

        let model = EmbeddingModel::onnx_minilm();
        assert_eq!(model.provider, "onnx");
        assert_eq!(model.dimension, 384);
    }
}
