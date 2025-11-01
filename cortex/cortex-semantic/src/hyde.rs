//! HyDE (Hypothetical Document Embeddings) implementation.
//!
//! HyDE improves retrieval by generating hypothetical answers to queries,
//! then using those hypothetical documents to find better matches.
//!
//! # References
//! - "Precise Zero-Shot Dense Retrieval without Relevance Labels" (Gao et al., 2022)
//! - HyDE generates hypothetical document answers to queries to improve embedding-based retrieval
//! - "Generate rather than Retrieve: Large Language Models are Strong Context Generators" (Yu et al., 2023)

use crate::error::{Result, SemanticError};
use crate::providers::EmbeddingProvider;
use crate::query::QueryIntent;
use crate::types::Vector;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Configuration for HyDE processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HydeConfig {
    /// Number of hypothetical documents to generate
    pub num_hypotheses: usize,
    /// Whether to include the original query embedding
    pub include_original_query: bool,
    /// Weight for original query vs hypothetical documents (0.0-1.0)
    pub original_query_weight: f32,
    /// Enable hypothesis diversity
    pub enable_diversity: bool,
    /// Temperature for generation (if using LLM)
    pub generation_temperature: f32,
}

impl Default for HydeConfig {
    fn default() -> Self {
        Self {
            num_hypotheses: 3,
            include_original_query: true,
            original_query_weight: 0.3,
            enable_diversity: true,
            generation_temperature: 0.7,
        }
    }
}

/// A hypothetical document generated for HyDE.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypotheticalDocument {
    /// The generated text
    pub text: String,
    /// Embedding of the hypothetical document
    pub embedding: Vector,
    /// Confidence/quality score
    pub confidence: f32,
}

/// Result of HyDE processing.
#[derive(Debug, Clone)]
pub struct HydeResult {
    /// Original query
    pub query: String,
    /// Generated hypothetical documents
    pub hypothetical_docs: Vec<HypotheticalDocument>,
    /// Aggregated embedding for search
    pub aggregated_embedding: Vector,
}

/// HyDE processor for generating and using hypothetical documents.
///
/// # Example
/// ```no_run
/// use cortex_semantic::hyde::{HydeProcessor, HydeConfig};
/// use cortex_semantic::providers::MockProvider;
/// use std::sync::Arc;
///
/// # async fn example() -> anyhow::Result<()> {
/// let provider = Arc::new(MockProvider::new(384));
/// let config = HydeConfig::default();
/// let hyde = HydeProcessor::new(provider, config);
///
/// let result = hyde.process_query("What is machine learning?", None).await?;
/// println!("Generated {} hypothetical documents", result.hypothetical_docs.len());
/// # Ok(())
/// # }
/// ```
pub struct HydeProcessor {
    embedding_provider: Arc<dyn EmbeddingProvider>,
    config: HydeConfig,
}

impl HydeProcessor {
    /// Create a new HyDE processor.
    pub fn new(embedding_provider: Arc<dyn EmbeddingProvider>, config: HydeConfig) -> Self {
        Self {
            embedding_provider,
            config,
        }
    }

    /// Process a query using HyDE to generate hypothetical documents.
    ///
    /// This method:
    /// 1. Generates hypothetical answer documents
    /// 2. Embeds each hypothetical document
    /// 3. Aggregates embeddings for improved retrieval
    ///
    /// Reference: "Precise Zero-Shot Dense Retrieval" (Gao et al., 2022)
    pub async fn process_query(
        &self,
        query: &str,
        intent: Option<QueryIntent>,
    ) -> Result<HydeResult> {
        // Generate hypothetical documents
        let hypothetical_texts = self.generate_hypothetical_documents(query, intent);

        // Embed each hypothetical document
        let mut hypothetical_docs = Vec::new();
        for text in hypothetical_texts {
            let embedding = self.embedding_provider.embed(&text).await?;
            hypothetical_docs.push(HypotheticalDocument {
                text,
                embedding,
                confidence: 1.0, // Default confidence; could be improved with quality scoring
            });
        }

        // Optionally include original query embedding
        let aggregated_embedding = if self.config.include_original_query {
            let query_embedding = self.embedding_provider.embed(query).await?;
            self.aggregate_embeddings_weighted(&hypothetical_docs, Some(query_embedding))
        } else {
            self.aggregate_embeddings(&hypothetical_docs)
        };

        Ok(HydeResult {
            query: query.to_string(),
            hypothetical_docs,
            aggregated_embedding,
        })
    }

    /// Generate hypothetical documents for a query.
    ///
    /// In a production system, this would use an LLM to generate diverse,
    /// plausible answers. For now, we use template-based generation.
    fn generate_hypothetical_documents(
        &self,
        query: &str,
        intent: Option<QueryIntent>,
    ) -> Vec<String> {
        let mut hypotheses = Vec::new();

        let intent = intent.unwrap_or(QueryIntent::General);

        match intent {
            QueryIntent::Code => {
                hypotheses.extend(self.generate_code_hypotheses(query));
            }
            QueryIntent::Documentation => {
                hypotheses.extend(self.generate_doc_hypotheses(query));
            }
            QueryIntent::Examples => {
                hypotheses.extend(self.generate_example_hypotheses(query));
            }
            QueryIntent::Definition => {
                hypotheses.extend(self.generate_definition_hypotheses(query));
            }
            _ => {
                hypotheses.extend(self.generate_general_hypotheses(query));
            }
        }

        // Limit to configured number
        hypotheses.truncate(self.config.num_hypotheses);
        hypotheses
    }

    /// Generate code-focused hypothetical documents.
    ///
    /// Creates plausible code implementations that might answer the query.
    fn generate_code_hypotheses(&self, query: &str) -> Vec<String> {
        vec![
            format!("Here's a function that solves {}: \
                     It uses best practices and is well-documented. \
                     The implementation is efficient and handles edge cases.", query),
            format!("To implement {}, you can use the following approach. \
                     This code is production-ready and includes error handling.", query),
            format!("Example implementation for {}: \
                     This solution is based on industry standards and patterns.", query),
        ]
    }

    /// Generate documentation-focused hypothetical documents.
    fn generate_doc_hypotheses(&self, query: &str) -> Vec<String> {
        vec![
            format!("Documentation for {}: \
                     This is a comprehensive guide explaining the concept. \
                     It includes definitions, use cases, and best practices.", query),
            format!("{} is an important concept in software development. \
                     Here's what you need to know about it and how it works.", query),
            format!("Understanding {}: \
                     This overview covers the key aspects and applications.", query),
        ]
    }

    /// Generate example-focused hypothetical documents.
    fn generate_example_hypotheses(&self, query: &str) -> Vec<String> {
        vec![
            format!("Example of {}: \
                     Here's a practical demonstration showing how to use it. \
                     This example includes common use cases and patterns.", query),
            format!("Tutorial for {}: \
                     Step-by-step guide with code examples and explanations.", query),
            format!("Sample implementation of {}: \
                     This shows a real-world usage scenario with best practices.", query),
        ]
    }

    /// Generate definition-focused hypothetical documents.
    fn generate_definition_hypotheses(&self, query: &str) -> Vec<String> {
        vec![
            format!("Definition of {}: \
                     A clear explanation of what it is and how it's used. \
                     Includes context and related concepts.", query),
            format!("{} refers to a concept in software engineering. \
                     Here's a detailed definition and its significance.", query),
        ]
    }

    /// Generate general hypothetical documents.
    fn generate_general_hypotheses(&self, query: &str) -> Vec<String> {
        vec![
            format!("Information about {}: \
                     This document provides relevant details and context.", query),
            format!("Regarding {}: \
                     Here's comprehensive information covering the main aspects.", query),
            format!("Details on {}: \
                     An informative overview with key points and insights.", query),
        ]
    }

    /// Aggregate embeddings from hypothetical documents.
    ///
    /// Uses mean pooling to combine multiple embeddings.
    fn aggregate_embeddings(&self, hypothetical_docs: &[HypotheticalDocument]) -> Vector {
        if hypothetical_docs.is_empty() {
            return vec![];
        }

        let dim = hypothetical_docs[0].embedding.len();
        let mut aggregated = vec![0.0; dim];

        for doc in hypothetical_docs {
            for (i, &val) in doc.embedding.iter().enumerate() {
                aggregated[i] += val * doc.confidence;
            }
        }

        // Normalize by sum of confidences
        let total_confidence: f32 = hypothetical_docs.iter().map(|d| d.confidence).sum();
        if total_confidence > 0.0 {
            for val in &mut aggregated {
                *val /= total_confidence;
            }
        }

        // L2 normalize
        crate::types::normalize(&mut aggregated);

        aggregated
    }

    /// Aggregate embeddings with weighted contribution from original query.
    ///
    /// Reference: Balancing original query intent with hypothetical expansions
    fn aggregate_embeddings_weighted(
        &self,
        hypothetical_docs: &[HypotheticalDocument],
        query_embedding: Option<Vector>,
    ) -> Vector {
        if let Some(query_emb) = query_embedding {
            let hypothesis_emb = self.aggregate_embeddings(hypothetical_docs);

            if hypothesis_emb.is_empty() {
                return query_emb;
            }

            // Weighted combination
            let mut combined = vec![0.0; query_emb.len()];
            let query_weight = self.config.original_query_weight;
            let hypothesis_weight = 1.0 - query_weight;

            for i in 0..query_emb.len() {
                combined[i] = query_emb[i] * query_weight + hypothesis_emb[i] * hypothesis_weight;
            }

            // L2 normalize
            crate::types::normalize(&mut combined);

            combined
        } else {
            self.aggregate_embeddings(hypothetical_docs)
        }
    }

    /// Get embedding for search using HyDE result.
    pub fn get_search_embedding(&self, hyde_result: &HydeResult) -> Vector {
        hyde_result.aggregated_embedding.clone()
    }
}

/// Trait for LLM-based hypothesis generation (for future enhancement).
///
/// This trait can be implemented to use real LLMs for generating
/// higher-quality hypothetical documents.
#[async_trait::async_trait]
pub trait HypothesisGenerator: Send + Sync {
    /// Generate a hypothetical answer to a query.
    async fn generate(&self, query: &str, intent: QueryIntent) -> Result<String>;

    /// Generate multiple diverse hypotheses.
    async fn generate_diverse(
        &self,
        query: &str,
        intent: QueryIntent,
        count: usize,
    ) -> Result<Vec<String>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::MockProvider;

    #[tokio::test]
    async fn test_hyde_process_query() {
        let provider = Arc::new(MockProvider::new(384));
        let config = HydeConfig::default();
        let hyde = HydeProcessor::new(provider, config);

        let result = hyde
            .process_query("What is machine learning?", Some(QueryIntent::Documentation))
            .await
            .unwrap();

        assert_eq!(result.query, "What is machine learning?");
        assert!(!result.hypothetical_docs.is_empty());
        assert!(!result.aggregated_embedding.is_empty());
        assert_eq!(result.aggregated_embedding.len(), 384);
    }

    #[tokio::test]
    async fn test_code_hypotheses_generation() {
        let provider = Arc::new(MockProvider::new(384));
        let config = HydeConfig::default();
        let hyde = HydeProcessor::new(provider, config);

        let hypotheses = hyde.generate_code_hypotheses("authentication");

        assert!(!hypotheses.is_empty());
        assert!(hypotheses.iter().any(|h| h.contains("function") || h.contains("implementation")));
    }

    #[tokio::test]
    async fn test_embedding_aggregation() {
        let provider = Arc::new(MockProvider::new(384));
        let config = HydeConfig::default();
        let hyde = HydeProcessor::new(provider, config);

        let mut docs = Vec::new();
        for i in 0..3 {
            let embedding = vec![i as f32; 384];
            docs.push(HypotheticalDocument {
                text: format!("Doc {}", i),
                embedding,
                confidence: 1.0,
            });
        }

        let aggregated = hyde.aggregate_embeddings(&docs);

        assert_eq!(aggregated.len(), 384);
        // Check that it's normalized
        let norm: f32 = aggregated.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_weighted_aggregation() {
        let provider = Arc::new(MockProvider::new(384));
        let config = HydeConfig {
            original_query_weight: 0.5,
            ..Default::default()
        };
        let hyde = HydeProcessor::new(provider, config);

        let query_emb = vec![1.0; 384];
        let mut docs = Vec::new();
        docs.push(HypotheticalDocument {
            text: "Doc 1".to_string(),
            embedding: vec![0.0; 384],
            confidence: 1.0,
        });

        let aggregated = hyde.aggregate_embeddings_weighted(&docs, Some(query_emb));

        assert_eq!(aggregated.len(), 384);
        // Result should be between query and hypothesis embeddings
        assert!(aggregated[0] > 0.0);
        assert!(aggregated[0] < 1.0);
    }

    #[test]
    fn test_hypothesis_types() {
        let provider = Arc::new(MockProvider::new(384));
        let hyde = HydeProcessor::new(provider, HydeConfig::default());

        let code_hyp = hyde.generate_code_hypotheses("test");
        let doc_hyp = hyde.generate_doc_hypotheses("test");
        let example_hyp = hyde.generate_example_hypotheses("test");

        assert!(!code_hyp.is_empty());
        assert!(!doc_hyp.is_empty());
        assert!(!example_hyp.is_empty());

        // Verify they're different types
        assert_ne!(code_hyp[0], doc_hyp[0]);
        assert_ne!(doc_hyp[0], example_hyp[0]);
    }
}
