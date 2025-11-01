//! RAGAS (Retrieval Augmented Generation Assessment) metrics implementation.
//!
//! This module implements state-of-the-art RAG evaluation metrics based on 2024-2025 research:
//! - Faithfulness: Measures factual accuracy based on retrieved documents
//! - Answer Relevancy: Evaluates how relevant the response is to the query
//! - Context Precision: Precision of retrieved documents
//! - Context Recall: Recall of relevant information
//! - Answer Correctness: Combined score of semantic similarity and factual overlap
//! - Hallucination Detection: Identifies unsupported claims
//!
//! Based on research from:
//! - RAGAS framework (2024): https://arxiv.org/abs/2309.15217
//! - "Evaluating RAG Applications" (2025)
//! - ARES framework for automatic RAG evaluation

use crate::error::Result;
use crate::providers::EmbeddingProvider;
use crate::types::{Vector, DocumentId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// RAGAS evaluation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagasConfig {
    /// Minimum cosine similarity for semantic matching
    pub similarity_threshold: f32,
    /// Weight for faithfulness in combined score
    pub faithfulness_weight: f32,
    /// Weight for relevancy in combined score
    pub relevancy_weight: f32,
    /// Weight for context quality in combined score
    pub context_weight: f32,
    /// Enable hallucination detection
    pub detect_hallucinations: bool,
    /// Use LLM for evaluation (future enhancement)
    pub use_llm_eval: bool,
}

impl Default for RagasConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.7,
            faithfulness_weight: 0.4,
            relevancy_weight: 0.3,
            context_weight: 0.3,
            detect_hallucinations: true,
            use_llm_eval: false,
        }
    }
}

/// RAGAS evaluation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagasEvaluation {
    /// Faithfulness score (0-1): factual accuracy based on retrieved docs
    pub faithfulness: f32,
    /// Answer relevancy score (0-1): how relevant is the answer to the query
    pub answer_relevancy: f32,
    /// Context precision (0-1): precision of retrieved documents
    pub context_precision: f32,
    /// Context recall (0-1): recall of relevant information
    pub context_recall: f32,
    /// Answer correctness (0-1): combined semantic and factual correctness
    pub answer_correctness: f32,
    /// Hallucination rate (0-1): proportion of unsupported claims
    pub hallucination_rate: f32,
    /// Overall RAGAS score (0-1)
    pub overall_score: f32,
    /// Detailed breakdown
    pub details: EvaluationDetails,
}

/// Detailed evaluation breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationDetails {
    /// Number of supported claims
    pub supported_claims: usize,
    /// Number of unsupported claims (hallucinations)
    pub unsupported_claims: usize,
    /// Total claims made
    pub total_claims: usize,
    /// Relevant sentences in answer
    pub relevant_sentences: Vec<String>,
    /// Irrelevant sentences in answer
    pub irrelevant_sentences: Vec<String>,
    /// Precision at different k values
    pub precision_at_k: HashMap<usize, f32>,
    /// Recall at different k values
    pub recall_at_k: HashMap<usize, f32>,
}

/// RAGAS evaluator for comprehensive RAG assessment.
pub struct RagasEvaluator {
    config: RagasConfig,
    embedding_provider: Arc<dyn EmbeddingProvider>,
}

impl RagasEvaluator {
    /// Create a new RAGAS evaluator.
    pub fn new(
        config: RagasConfig,
        embedding_provider: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        info!("Initializing RAGAS evaluator with 2025 metrics");
        Self {
            config,
            embedding_provider,
        }
    }

    /// Evaluate a RAG response comprehensively.
    pub async fn evaluate(
        &self,
        query: &str,
        answer: &str,
        retrieved_contexts: &[String],
        ground_truth: Option<&str>,
    ) -> Result<RagasEvaluation> {
        debug!("Evaluating RAG response with RAGAS metrics");

        // Calculate individual metrics
        let faithfulness = self.calculate_faithfulness(answer, retrieved_contexts).await?;
        let answer_relevancy = self.calculate_answer_relevancy(query, answer).await?;
        let (context_precision, precision_at_k) =
            self.calculate_context_precision(query, retrieved_contexts).await?;
        let (context_recall, recall_at_k) =
            self.calculate_context_recall(retrieved_contexts, ground_truth).await?;
        let answer_correctness =
            self.calculate_answer_correctness(answer, ground_truth).await?;
        let (hallucination_rate, details) =
            self.detect_hallucinations(answer, retrieved_contexts).await?;

        // Calculate overall score
        let overall_score = self.calculate_overall_score(
            faithfulness,
            answer_relevancy,
            context_precision,
            context_recall,
            answer_correctness,
        );

        Ok(RagasEvaluation {
            faithfulness,
            answer_relevancy,
            context_precision,
            context_recall,
            answer_correctness,
            hallucination_rate,
            overall_score,
            details: EvaluationDetails {
                supported_claims: details.0,
                unsupported_claims: details.1,
                total_claims: details.2,
                relevant_sentences: details.3,
                irrelevant_sentences: details.4,
                precision_at_k,
                recall_at_k,
            },
        })
    }

    /// Calculate faithfulness: measures factual accuracy based on retrieved documents.
    async fn calculate_faithfulness(
        &self,
        answer: &str,
        contexts: &[String],
    ) -> Result<f32> {
        if contexts.is_empty() || answer.is_empty() {
            return Ok(0.0);
        }

        // Extract claims from answer (simplified: use sentences as claims)
        let claims = self.extract_claims(answer);
        if claims.is_empty() {
            return Ok(1.0);  // No claims to verify
        }

        // Check each claim against contexts
        let mut supported_count = 0;
        let context_combined = contexts.join(" ");

        for claim in &claims {
            // Generate embeddings for semantic matching
            let claim_embedding = self.embedding_provider.embed(claim).await?;
            let context_embedding = self.embedding_provider.embed(&context_combined).await?;

            // Calculate semantic similarity
            let similarity = crate::types::cosine_similarity(
                claim_embedding.as_slice(),
                context_embedding.as_slice(),
            );

            if similarity >= self.config.similarity_threshold {
                supported_count += 1;
            } else {
                // Also check for substring match as fallback
                if context_combined.to_lowercase().contains(&claim.to_lowercase()) {
                    supported_count += 1;
                }
            }
        }

        Ok(supported_count as f32 / claims.len() as f32)
    }

    /// Calculate answer relevancy: how relevant is the answer to the query.
    async fn calculate_answer_relevancy(
        &self,
        query: &str,
        answer: &str,
    ) -> Result<f32> {
        if query.is_empty() || answer.is_empty() {
            return Ok(0.0);
        }

        // Generate embeddings
        let query_embedding = self.embedding_provider.embed(query).await?;
        let answer_embedding = self.embedding_provider.embed(answer).await?;

        // Calculate cosine similarity
        let similarity = crate::types::cosine_similarity(
            query_embedding.as_slice(),
            answer_embedding.as_slice(),
        );

        // Also check for keyword overlap
        let query_keywords = self.extract_keywords(query);
        let answer_keywords = self.extract_keywords(answer);

        let keyword_overlap = query_keywords.intersection(&answer_keywords).count() as f32
            / query_keywords.len().max(1) as f32;

        // Combine semantic and keyword similarity
        Ok(0.7 * similarity + 0.3 * keyword_overlap)
    }

    /// Calculate context precision: precision of retrieved documents.
    async fn calculate_context_precision(
        &self,
        query: &str,
        contexts: &[String],
    ) -> Result<(f32, HashMap<usize, f32>)> {
        if contexts.is_empty() {
            return Ok((0.0, HashMap::new()));
        }

        let query_embedding = self.embedding_provider.embed(query).await?;
        let mut precision_at_k = HashMap::new();
        let mut relevant_count = 0;

        for (i, context) in contexts.iter().enumerate() {
            let context_embedding = self.embedding_provider.embed(context).await?;
            let similarity = crate::types::cosine_similarity(
                query_embedding.as_slice(),
                context_embedding.as_slice(),
            );

            if similarity >= self.config.similarity_threshold {
                relevant_count += 1;
            }

            // Calculate precision at k
            let k = i + 1;
            if k <= 10 {  // Track P@1, P@3, P@5, P@10
                let precision = relevant_count as f32 / k as f32;
                if k == 1 || k == 3 || k == 5 || k == 10 {
                    precision_at_k.insert(k, precision);
                }
            }
        }

        let overall_precision = relevant_count as f32 / contexts.len() as f32;
        Ok((overall_precision, precision_at_k))
    }

    /// Calculate context recall: recall of relevant information.
    async fn calculate_context_recall(
        &self,
        contexts: &[String],
        ground_truth: Option<&str>,
    ) -> Result<(f32, HashMap<usize, f32>)> {
        let ground_truth = match ground_truth {
            Some(gt) => gt,
            None => {
                // No ground truth, can't calculate recall
                return Ok((0.0, HashMap::new()));
            }
        };

        if contexts.is_empty() {
            return Ok((0.0, HashMap::new()));
        }

        // Extract key information from ground truth
        let gt_sentences = self.extract_sentences(ground_truth);
        if gt_sentences.is_empty() {
            return Ok((1.0, HashMap::new()));
        }

        let mut recall_at_k = HashMap::new();
        let mut recalled_count = 0;
        let context_combined = contexts[..contexts.len().min(10)].join(" ");

        // Check how many ground truth sentences are covered
        for sentence in &gt_sentences {
            let sentence_embedding = self.embedding_provider.embed(sentence).await?;
            let context_embedding = self.embedding_provider.embed(&context_combined).await?;

            let similarity = crate::types::cosine_similarity(
                sentence_embedding.as_slice(),
                context_embedding.as_slice(),
            );

            if similarity >= self.config.similarity_threshold {
                recalled_count += 1;
            }
        }

        // Calculate recall at different k values
        for k in [1, 3, 5, 10] {
            if k <= contexts.len() {
                let k_context = contexts[..k].join(" ");
                let mut k_recalled = 0;

                for sentence in &gt_sentences {
                    if k_context.to_lowercase().contains(&sentence.to_lowercase()) {
                        k_recalled += 1;
                    }
                }

                recall_at_k.insert(k, k_recalled as f32 / gt_sentences.len() as f32);
            }
        }

        let overall_recall = recalled_count as f32 / gt_sentences.len() as f32;
        Ok((overall_recall, recall_at_k))
    }

    /// Calculate answer correctness: combined semantic and factual correctness.
    async fn calculate_answer_correctness(
        &self,
        answer: &str,
        ground_truth: Option<&str>,
    ) -> Result<f32> {
        let ground_truth = match ground_truth {
            Some(gt) => gt,
            None => return Ok(0.5),  // No ground truth, return neutral score
        };

        // Semantic similarity
        let answer_embedding = self.embedding_provider.embed(answer).await?;
        let gt_embedding = self.embedding_provider.embed(ground_truth).await?;
        let semantic_similarity = crate::types::cosine_similarity(
            answer_embedding.as_slice(),
            gt_embedding.as_slice(),
        );

        // Factual overlap (F1 score on keywords)
        let answer_keywords = self.extract_keywords(answer);
        let gt_keywords = self.extract_keywords(ground_truth);

        let intersection = answer_keywords.intersection(&gt_keywords).count() as f32;
        let precision = if answer_keywords.is_empty() {
            0.0
        } else {
            intersection / answer_keywords.len() as f32
        };
        let recall = if gt_keywords.is_empty() {
            0.0
        } else {
            intersection / gt_keywords.len() as f32
        };

        let f1_score = if precision + recall > 0.0 {
            2.0 * precision * recall / (precision + recall)
        } else {
            0.0
        };

        // Combine semantic and factual scores
        Ok(0.6 * semantic_similarity + 0.4 * f1_score)
    }

    /// Detect hallucinations in the answer.
    async fn detect_hallucinations(
        &self,
        answer: &str,
        contexts: &[String],
    ) -> Result<(f32, (usize, usize, usize, Vec<String>, Vec<String>))> {
        if !self.config.detect_hallucinations {
            return Ok((0.0, (0, 0, 0, vec![], vec![])));
        }

        let claims = self.extract_claims(answer);
        if claims.is_empty() {
            return Ok((0.0, (0, 0, 0, vec![], vec![])));
        }

        let context_combined = contexts.join(" ");
        let mut unsupported = 0;
        let mut supported = 0;
        let mut relevant_sentences = Vec::new();
        let mut irrelevant_sentences = Vec::new();

        for claim in &claims {
            let claim_embedding = self.embedding_provider.embed(claim).await?;
            let context_embedding = self.embedding_provider.embed(&context_combined).await?;

            let similarity = crate::types::cosine_similarity(
                claim_embedding.as_slice(),
                context_embedding.as_slice(),
            );

            if similarity >= self.config.similarity_threshold {
                supported += 1;
                relevant_sentences.push(claim.clone());
            } else {
                unsupported += 1;
                irrelevant_sentences.push(claim.clone());
            }
        }

        let hallucination_rate = unsupported as f32 / claims.len() as f32;
        Ok((
            hallucination_rate,
            (supported, unsupported, claims.len(), relevant_sentences, irrelevant_sentences),
        ))
    }

    /// Calculate overall RAGAS score.
    fn calculate_overall_score(
        &self,
        faithfulness: f32,
        answer_relevancy: f32,
        context_precision: f32,
        context_recall: f32,
        answer_correctness: f32,
    ) -> f32 {
        let mut weighted_sum = 0.0;
        let mut weight_sum = 0.0;

        // Use configured weights
        weighted_sum += faithfulness * self.config.faithfulness_weight;
        weight_sum += self.config.faithfulness_weight;

        weighted_sum += answer_relevancy * self.config.relevancy_weight;
        weight_sum += self.config.relevancy_weight;

        weighted_sum += (context_precision + context_recall) * 0.5 * self.config.context_weight;
        weight_sum += self.config.context_weight;

        // Add answer correctness if available
        if answer_correctness > 0.0 {
            weighted_sum += answer_correctness * 0.2;
            weight_sum += 0.2;
        }

        if weight_sum > 0.0 {
            weighted_sum / weight_sum
        } else {
            0.0
        }
    }

    /// Extract claims/sentences from text.
    fn extract_claims(&self, text: &str) -> Vec<String> {
        text.split('.')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && s.len() > 10)
            .map(|s| s.to_string())
            .collect()
    }

    /// Extract sentences from text.
    fn extract_sentences(&self, text: &str) -> Vec<String> {
        text.split('.')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    }

    /// Extract keywords from text (simplified).
    fn extract_keywords(&self, text: &str) -> HashSet<String> {
        text.split_whitespace()
            .map(|w| w.to_lowercase())
            .filter(|w| w.len() > 3)  // Skip short words
            .filter(|w| !Self::is_stopword(w))
            .collect()
    }

    /// Check if word is a stopword.
    fn is_stopword(word: &str) -> bool {
        matches!(
            word,
            "the" | "and" | "or" | "but" | "in" | "on" | "at" | "to" | "for" |
            "of" | "with" | "by" | "from" | "as" | "is" | "was" | "are" | "were" |
            "been" | "being" | "have" | "has" | "had" | "do" | "does" | "did" |
            "will" | "would" | "could" | "should" | "may" | "might" | "must" |
            "can" | "this" | "that" | "these" | "those" | "a" | "an"
        )
    }

    /// Batch evaluate multiple queries.
    pub async fn batch_evaluate(
        &self,
        evaluations: Vec<(String, String, Vec<String>, Option<String>)>,
    ) -> Result<Vec<RagasEvaluation>> {
        let mut results = Vec::new();

        for (query, answer, contexts, ground_truth) in evaluations {
            let eval = self.evaluate(
                &query,
                &answer,
                &contexts,
                ground_truth.as_deref(),
            ).await?;
            results.push(eval);
        }

        Ok(results)
    }

    /// Generate evaluation report.
    pub fn generate_report(&self, evaluations: &[RagasEvaluation]) -> String {
        if evaluations.is_empty() {
            return "No evaluations to report".to_string();
        }

        let mut report = String::from("# RAGAS Evaluation Report\n\n");

        // Calculate averages
        let avg_faithfulness = evaluations.iter().map(|e| e.faithfulness).sum::<f32>()
            / evaluations.len() as f32;
        let avg_relevancy = evaluations.iter().map(|e| e.answer_relevancy).sum::<f32>()
            / evaluations.len() as f32;
        let avg_precision = evaluations.iter().map(|e| e.context_precision).sum::<f32>()
            / evaluations.len() as f32;
        let avg_recall = evaluations.iter().map(|e| e.context_recall).sum::<f32>()
            / evaluations.len() as f32;
        let avg_correctness = evaluations.iter().map(|e| e.answer_correctness).sum::<f32>()
            / evaluations.len() as f32;
        let avg_hallucination = evaluations.iter().map(|e| e.hallucination_rate).sum::<f32>()
            / evaluations.len() as f32;
        let avg_overall = evaluations.iter().map(|e| e.overall_score).sum::<f32>()
            / evaluations.len() as f32;

        report.push_str(&format!("## Summary (n={})\n\n", evaluations.len()));
        report.push_str(&format!("- **Overall Score**: {:.3}\n", avg_overall));
        report.push_str(&format!("- **Faithfulness**: {:.3}\n", avg_faithfulness));
        report.push_str(&format!("- **Answer Relevancy**: {:.3}\n", avg_relevancy));
        report.push_str(&format!("- **Context Precision**: {:.3}\n", avg_precision));
        report.push_str(&format!("- **Context Recall**: {:.3}\n", avg_recall));
        report.push_str(&format!("- **Answer Correctness**: {:.3}\n", avg_correctness));
        report.push_str(&format!("- **Hallucination Rate**: {:.3}\n\n", avg_hallucination));

        // Distribution analysis
        report.push_str("## Score Distribution\n\n");
        report.push_str(&self.generate_distribution_stats("Overall",
            &evaluations.iter().map(|e| e.overall_score).collect::<Vec<_>>()));
        report.push_str(&self.generate_distribution_stats("Faithfulness",
            &evaluations.iter().map(|e| e.faithfulness).collect::<Vec<_>>()));

        // Hallucination analysis
        let total_claims: usize = evaluations.iter()
            .map(|e| e.details.total_claims)
            .sum();
        let total_hallucinations: usize = evaluations.iter()
            .map(|e| e.details.unsupported_claims)
            .sum();

        report.push_str(&format!("\n## Hallucination Analysis\n\n"));
        report.push_str(&format!("- Total Claims: {}\n", total_claims));
        report.push_str(&format!("- Hallucinated Claims: {} ({:.1}%)\n",
            total_hallucinations,
            (total_hallucinations as f32 / total_claims.max(1) as f32) * 100.0
        ));

        report
    }

    /// Generate distribution statistics for a metric.
    fn generate_distribution_stats(&self, name: &str, scores: &[f32]) -> String {
        if scores.is_empty() {
            return format!("### {}: No data\n", name);
        }

        let min = scores.iter().copied().fold(f32::INFINITY, f32::min);
        let max = scores.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let mean = scores.iter().sum::<f32>() / scores.len() as f32;

        // Calculate median
        let mut sorted = scores.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = if sorted.len() % 2 == 0 {
            (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
        } else {
            sorted[sorted.len() / 2]
        };

        format!(
            "### {}\n- Min: {:.3}, Max: {:.3}\n- Mean: {:.3}, Median: {:.3}\n\n",
            name, min, max, mean, median
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::MockProvider;

    #[tokio::test]
    async fn test_ragas_faithfulness() {
        let provider = Arc::new(MockProvider::new(384));
        let evaluator = RagasEvaluator::new(RagasConfig::default(), provider);

        let answer = "The capital of France is Paris.";
        let contexts = vec!["Paris is the capital city of France.".to_string()];

        let faithfulness = evaluator.calculate_faithfulness(answer, &contexts).await.unwrap();
        assert!(faithfulness > 0.5);
    }

    #[tokio::test]
    async fn test_ragas_relevancy() {
        let provider = Arc::new(MockProvider::new(384));
        let evaluator = RagasEvaluator::new(RagasConfig::default(), provider);

        let query = "What is the capital of France?";
        let answer = "The capital of France is Paris.";

        let relevancy = evaluator.calculate_answer_relevancy(query, answer).await.unwrap();
        assert!(relevancy > 0.3);
    }

    #[tokio::test]
    async fn test_ragas_full_evaluation() {
        let provider = Arc::new(MockProvider::new(384));
        let evaluator = RagasEvaluator::new(RagasConfig::default(), provider);

        let eval = evaluator.evaluate(
            "What is machine learning?",
            "Machine learning is a subset of artificial intelligence.",
            &vec!["Machine learning is a type of AI that enables computers to learn from data.".to_string()],
            Some("Machine learning is a branch of artificial intelligence."),
        ).await.unwrap();

        assert!(eval.overall_score > 0.0);
        assert!(eval.overall_score <= 1.0);
    }
}