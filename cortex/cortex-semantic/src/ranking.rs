//! Result ranking and scoring algorithms.

use crate::query::ProcessedQuery;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Ranking strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RankingStrategy {
    /// Pure semantic similarity
    Semantic,
    /// Hybrid keyword + semantic
    Hybrid,
    /// BM25 keyword ranking
    BM25,
    /// Custom weighted scoring
    Weighted,
}

/// Scoring algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScoringAlgorithm {
    /// Cosine similarity
    Cosine,
    /// Euclidean distance
    Euclidean,
    /// Dot product
    DotProduct,
    /// Custom scoring
    Custom,
}

/// Document to be ranked.
#[derive(Debug, Clone)]
pub struct RankableDocument {
    pub id: String,
    pub content: String,
    pub semantic_score: f32,
    pub metadata: HashMap<String, String>,
}

/// Ranked result.
#[derive(Debug, Clone)]
pub struct RankedResult {
    pub id: String,
    pub final_score: f32,
    pub semantic_score: f32,
    pub keyword_score: f32,
    pub recency_score: f32,
    pub popularity_score: f32,
    pub explanation: Option<String>,
}

/// Ranker for re-ranking search results.
pub struct Ranker {
    strategy: RankingStrategy,
    weights: ScoringWeights,
}

/// Configurable weights for different scoring components.
#[derive(Debug, Clone)]
pub struct ScoringWeights {
    pub semantic: f32,
    pub keyword: f32,
    pub recency: f32,
    pub popularity: f32,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            semantic: 0.7,
            keyword: 0.2,
            recency: 0.05,
            popularity: 0.05,
        }
    }
}

impl Ranker {
    pub fn new(strategy: RankingStrategy) -> Self {
        Self {
            strategy,
            weights: ScoringWeights::default(),
        }
    }

    pub fn with_weights(strategy: RankingStrategy, weights: ScoringWeights) -> Self {
        Self { strategy, weights }
    }

    /// Rank documents based on the configured strategy.
    pub fn rank(
        &self,
        documents: Vec<RankableDocument>,
        query: &ProcessedQuery,
    ) -> Vec<RankedResult> {
        let mut results: Vec<RankedResult> = documents
            .into_iter()
            .map(|doc| self.score_document(doc, query))
            .collect();

        // Sort by final score (descending)
        results.sort_by(|a, b| b.final_score.partial_cmp(&a.final_score).unwrap());

        results
    }

    fn score_document(&self, doc: RankableDocument, query: &ProcessedQuery) -> RankedResult {
        let semantic_score = doc.semantic_score;
        let keyword_score = self.calculate_keyword_score(&doc.content, &query.keywords);
        let recency_score = self.calculate_recency_score(&doc.metadata);
        let popularity_score = self.calculate_popularity_score(&doc.metadata);

        let final_score = match self.strategy {
            RankingStrategy::Semantic => semantic_score,
            RankingStrategy::Hybrid => {
                semantic_score * self.weights.semantic + keyword_score * self.weights.keyword
            }
            RankingStrategy::BM25 => keyword_score,
            RankingStrategy::Weighted => {
                semantic_score * self.weights.semantic
                    + keyword_score * self.weights.keyword
                    + recency_score * self.weights.recency
                    + popularity_score * self.weights.popularity
            }
        };

        let explanation = if cfg!(debug_assertions) {
            Some(format!(
                "semantic={:.3}, keyword={:.3}, recency={:.3}, popularity={:.3}",
                semantic_score, keyword_score, recency_score, popularity_score
            ))
        } else {
            None
        };

        RankedResult {
            id: doc.id,
            final_score,
            semantic_score,
            keyword_score,
            recency_score,
            popularity_score,
            explanation,
        }
    }

    fn calculate_keyword_score(&self, content: &str, keywords: &[String]) -> f32 {
        if keywords.is_empty() {
            return 0.0;
        }

        let content_lower = content.to_lowercase();
        let mut score = 0.0;

        for keyword in keywords {
            let keyword_lower = keyword.to_lowercase();

            // Count occurrences
            let count = content_lower.matches(&keyword_lower).count() as f32;

            // Apply TF-IDF-like scoring
            if count > 0.0 {
                let tf = (1.0 + count.ln()) / (1.0 + content.len() as f32).ln();
                score += tf;
            }
        }

        // Normalize by number of keywords
        (score / keywords.len() as f32).min(1.0)
    }

    fn calculate_recency_score(&self, metadata: &HashMap<String, String>) -> f32 {
        // Check for timestamp in metadata
        if let Some(timestamp_str) = metadata.get("updated_at").or_else(|| metadata.get("created_at")) {
            // Parse timestamp and calculate recency
            if let Ok(timestamp) = chrono::DateTime::parse_from_rfc3339(timestamp_str) {
                let now = chrono::Utc::now();
                let age = now.signed_duration_since(timestamp.with_timezone(&chrono::Utc));

                // Decay function: score decreases over time
                // Documents from last 7 days get full score, then exponential decay
                let days = age.num_days() as f32;
                if days < 7.0 {
                    return 1.0;
                } else {
                    return (-(days - 7.0) / 30.0).exp().max(0.1);
                }
            }
        }

        0.5 // Default neutral score
    }

    fn calculate_popularity_score(&self, metadata: &HashMap<String, String>) -> f32 {
        // Check for popularity metrics
        let mut score = 0.0;

        if let Some(views_str) = metadata.get("views") {
            if let Ok(views) = views_str.parse::<f32>() {
                score += (1.0 + views).ln() / 10.0;
            }
        }

        if let Some(refs_str) = metadata.get("references") {
            if let Ok(refs) = refs_str.parse::<f32>() {
                score += (1.0 + refs).ln() / 5.0;
            }
        }

        score.min(1.0)
    }
}

/// BM25 scorer for keyword-based ranking.
pub struct BM25Scorer {
    k1: f32,
    b: f32,
    avg_doc_length: f32,
}

impl BM25Scorer {
    pub fn new(avg_doc_length: f32) -> Self {
        Self {
            k1: 1.2,
            b: 0.75,
            avg_doc_length,
        }
    }

    pub fn score(&self, doc: &str, query_terms: &[String], idf_scores: &HashMap<String, f32>) -> f32 {
        let doc_length = doc.split_whitespace().count() as f32;
        let mut score = 0.0;

        for term in query_terms {
            let tf = doc.matches(term.as_str()).count() as f32;
            let idf = idf_scores.get(term).copied().unwrap_or(0.0);

            let numerator = tf * (self.k1 + 1.0);
            let denominator = tf + self.k1 * (1.0 - self.b + self.b * (doc_length / self.avg_doc_length));

            score += idf * (numerator / denominator);
        }

        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_doc(id: &str, content: &str, semantic_score: f32) -> RankableDocument {
        RankableDocument {
            id: id.to_string(),
            content: content.to_string(),
            semantic_score,
            metadata: HashMap::new(),
        }
    }

    fn create_test_query() -> ProcessedQuery {
        ProcessedQuery {
            original: "test query".to_string(),
            normalized: "test query".to_string(),
            expanded: vec!["test query".to_string()],
            intent: crate::query::QueryIntent::General,
            keywords: vec!["test".to_string(), "query".to_string()],
            filters: Default::default(),
        }
    }

    #[test]
    fn test_semantic_ranking() {
        let ranker = Ranker::new(RankingStrategy::Semantic);
        let query = create_test_query();

        let docs = vec![
            create_test_doc("doc1", "content", 0.9),
            create_test_doc("doc2", "content", 0.7),
            create_test_doc("doc3", "content", 0.8),
        ];

        let results = ranker.rank(docs, &query);

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].id, "doc1");
        assert_eq!(results[1].id, "doc3");
        assert_eq!(results[2].id, "doc2");
    }

    #[test]
    fn test_keyword_score() {
        let ranker = Ranker::new(RankingStrategy::Semantic);
        let keywords = vec!["test".to_string(), "function".to_string()];

        let score1 = ranker.calculate_keyword_score("This is a test function", &keywords);
        let score2 = ranker.calculate_keyword_score("This is something else", &keywords);

        assert!(score1 > score2);
    }

    #[test]
    fn test_hybrid_ranking() {
        let ranker = Ranker::new(RankingStrategy::Hybrid);
        let query = create_test_query();

        let docs = vec![
            create_test_doc("doc1", "test query example", 0.6),
            create_test_doc("doc2", "something else", 0.9),
        ];

        let results = ranker.rank(docs, &query);

        // doc1 should rank higher due to keyword match despite lower semantic score
        assert_eq!(results[0].id, "doc1");
    }

    #[test]
    fn test_weighted_ranking() {
        let weights = ScoringWeights {
            semantic: 0.5,
            keyword: 0.3,
            recency: 0.1,
            popularity: 0.1,
        };
        let ranker = Ranker::with_weights(RankingStrategy::Weighted, weights);
        let query = create_test_query();

        let mut doc = create_test_doc("doc1", "test query", 0.8);
        doc.metadata.insert("views".to_string(), "100".to_string());

        let results = ranker.rank(vec![doc], &query);
        assert!(!results.is_empty());
        assert!(results[0].final_score > 0.0);
    }

    #[test]
    fn test_bm25_scorer() {
        let scorer = BM25Scorer::new(100.0);
        let query_terms = vec!["test".to_string(), "function".to_string()];
        let mut idf_scores = HashMap::new();
        idf_scores.insert("test".to_string(), 1.5);
        idf_scores.insert("function".to_string(), 2.0);

        let score = scorer.score("This is a test function for testing", &query_terms, &idf_scores);
        assert!(score > 0.0);
    }
}
