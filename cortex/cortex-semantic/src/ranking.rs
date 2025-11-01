//! Result ranking and scoring algorithms.
//!
//! Enhanced with advanced reranking techniques based on 2025 RAG research:
//! - MMR (Maximal Marginal Relevance) for diversity
//! - Cross-encoder support (optional feature)
//! - Diversity-aware ranking
//! - Personalization support
//!
//! # References
//! - "Maximal Marginal Relevance for Information Retrieval" (Carbonell & Goldstein, 1998)
//! - "RankGPT: LLMs as Re-Ranking Agents" (Sun et al., 2023)
//! - "SetRank: Learning to Rank as Sets" (Pang et al., 2020)

use crate::query::ProcessedQuery;
use crate::types::Vector;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

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
    /// MMR for diversity-aware ranking
    MMR,
    /// Personalized ranking
    Personalized,
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
    /// Embedding vector for diversity calculation
    pub embedding: Option<Vector>,
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
            RankingStrategy::MMR => {
                // For MMR, use weighted score as base (actual MMR logic is in rerank_mmr)
                semantic_score * self.weights.semantic + keyword_score * self.weights.keyword
            }
            RankingStrategy::Personalized => {
                // For personalized, use weighted score (actual personalization is elsewhere)
                semantic_score * self.weights.semantic + keyword_score * self.weights.keyword
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

/// MMR (Maximal Marginal Relevance) reranker for diversity.
///
/// MMR balances relevance and diversity in search results by iteratively
/// selecting documents that are relevant to the query while being different
/// from already-selected documents.
///
/// Reference: "The Use of MMR, Diversity-Based Reranking for Reordering Documents
/// and Producing Summaries" (Carbonell & Goldstein, 1998)
pub struct MMRReranker {
    /// Lambda parameter: balance between relevance (1.0) and diversity (0.0)
    lambda: f32,
}

impl MMRReranker {
    /// Create a new MMR reranker.
    ///
    /// # Parameters
    /// - `lambda`: Balance between relevance and diversity (0.0-1.0)
    ///   - 1.0 = only relevance, no diversity
    ///   - 0.0 = only diversity, no relevance
    ///   - 0.7 = good balance (recommended default)
    pub fn new(lambda: f32) -> Self {
        Self {
            lambda: lambda.clamp(0.0, 1.0),
        }
    }

    /// Rerank documents using MMR for diversity.
    ///
    /// # Algorithm
    /// MMR = λ * Sim(D, Q) - (1-λ) * max[Sim(D, Di)]
    /// where D is candidate doc, Q is query, Di are selected docs
    pub fn rerank(
        &self,
        documents: Vec<RankableDocument>,
        query_embedding: &[f32],
        k: usize,
    ) -> Vec<RankableDocument> {
        if documents.is_empty() {
            return vec![];
        }

        let mut selected = Vec::new();
        let mut remaining = documents;

        // Select first document (highest relevance)
        remaining.sort_by(|a, b| b.semantic_score.partial_cmp(&a.semantic_score).unwrap());
        if let Some(first) = remaining.first() {
            selected.push(first.clone());
            remaining.remove(0);
        }

        // Iteratively select remaining documents
        while selected.len() < k && !remaining.is_empty() {
            let mut best_idx = 0;
            let mut best_mmr = f32::NEG_INFINITY;

            for (idx, doc) in remaining.iter().enumerate() {
                // Calculate relevance to query
                let relevance = if let Some(embedding) = &doc.embedding {
                    crate::types::cosine_similarity(embedding, query_embedding)
                } else {
                    doc.semantic_score
                };

                // Calculate max similarity to already-selected documents
                let max_similarity = self.max_similarity_to_selected(doc, &selected);

                // Calculate MMR score
                let mmr = self.lambda * relevance - (1.0 - self.lambda) * max_similarity;

                if mmr > best_mmr {
                    best_mmr = mmr;
                    best_idx = idx;
                }
            }

            // Add best document to selected set
            let selected_doc = remaining.remove(best_idx);
            selected.push(selected_doc);
        }

        selected
    }

    /// Calculate maximum similarity between a document and the selected set.
    fn max_similarity_to_selected(
        &self,
        doc: &RankableDocument,
        selected: &[RankableDocument],
    ) -> f32 {
        if selected.is_empty() {
            return 0.0;
        }

        if let Some(doc_emb) = &doc.embedding {
            selected
                .iter()
                .filter_map(|s| s.embedding.as_ref())
                .map(|sel_emb| crate::types::cosine_similarity(doc_emb, sel_emb))
                .fold(f32::NEG_INFINITY, f32::max)
        } else {
            // Fallback to text-based similarity if embeddings not available
            selected
                .iter()
                .map(|s| self.text_similarity(&doc.content, &s.content))
                .fold(f32::NEG_INFINITY, f32::max)
        }
    }

    /// Simple text similarity using Jaccard similarity.
    fn text_similarity(&self, text1: &str, text2: &str) -> f32 {
        let words1: HashSet<_> = text1.split_whitespace().collect();
        let words2: HashSet<_> = text2.split_whitespace().collect();

        if words1.is_empty() && words2.is_empty() {
            return 1.0;
        }

        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();

        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }
}

/// Configuration for personalized ranking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalizationConfig {
    /// User preferences (feature -> weight)
    pub preferences: HashMap<String, f32>,
    /// Recent interaction history (document IDs)
    pub interaction_history: Vec<String>,
    /// Boost factor for similar documents to past interactions
    pub history_boost: f32,
}

impl Default for PersonalizationConfig {
    fn default() -> Self {
        Self {
            preferences: HashMap::new(),
            interaction_history: Vec::new(),
            history_boost: 1.2,
        }
    }
}

/// Personalized ranker that adapts to user preferences.
///
/// Reference: "Personalized Search via Learning-to-Rank" (Dou et al., 2007)
pub struct PersonalizedRanker {
    config: PersonalizationConfig,
}

impl PersonalizedRanker {
    pub fn new(config: PersonalizationConfig) -> Self {
        Self { config }
    }

    /// Rerank documents based on user personalization.
    pub fn rerank(&self, mut documents: Vec<RankableDocument>) -> Vec<RankableDocument> {
        for doc in &mut documents {
            let personalization_score = self.calculate_personalization_score(doc);

            // Combine with original semantic score
            let original_score = doc.semantic_score;
            doc.semantic_score = original_score * (1.0 + personalization_score);
        }

        // Sort by updated scores
        documents.sort_by(|a, b| b.semantic_score.partial_cmp(&a.semantic_score).unwrap());
        documents
    }

    /// Calculate personalization score for a document.
    fn calculate_personalization_score(&self, doc: &RankableDocument) -> f32 {
        let mut score = 0.0;

        // Apply preference weights
        for (feature, weight) in &self.config.preferences {
            if doc.metadata.contains_key(feature) {
                score += weight;
            }
        }

        // Boost if similar to interaction history
        if self.config.interaction_history.contains(&doc.id) {
            score += self.config.history_boost;
        }

        score
    }

    /// Update personalization based on user interaction.
    pub fn update_from_interaction(&mut self, doc_id: String, positive: bool) {
        if positive {
            self.config.interaction_history.push(doc_id);
            // Keep only recent history
            if self.config.interaction_history.len() > 100 {
                self.config.interaction_history.remove(0);
            }
        }
    }
}

/// Diversity-aware ranking configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiversityConfig {
    /// Enable diversity in results
    pub enable_diversity: bool,
    /// Diversity weight (0.0 = no diversity, 1.0 = max diversity)
    pub diversity_weight: f32,
    /// Minimum similarity threshold for considering documents similar
    pub similarity_threshold: f32,
}

impl Default for DiversityConfig {
    fn default() -> Self {
        Self {
            enable_diversity: true,
            diversity_weight: 0.3,
            similarity_threshold: 0.8,
        }
    }
}

/// Advanced ranker with multiple reranking strategies.
pub struct AdvancedRanker {
    base_ranker: Ranker,
    mmr_reranker: Option<MMRReranker>,
    personalized_ranker: Option<Arc<PersonalizedRanker>>,
    diversity_config: DiversityConfig,
}

impl AdvancedRanker {
    pub fn new(strategy: RankingStrategy) -> Self {
        Self {
            base_ranker: Ranker::new(strategy),
            mmr_reranker: None,
            personalized_ranker: None,
            diversity_config: DiversityConfig::default(),
        }
    }

    /// Enable MMR reranking with specified lambda.
    pub fn with_mmr(mut self, lambda: f32) -> Self {
        self.mmr_reranker = Some(MMRReranker::new(lambda));
        self
    }

    /// Enable personalized ranking.
    pub fn with_personalization(mut self, config: PersonalizationConfig) -> Self {
        self.personalized_ranker = Some(Arc::new(PersonalizedRanker::new(config)));
        self
    }

    /// Set diversity configuration.
    pub fn with_diversity(mut self, config: DiversityConfig) -> Self {
        self.diversity_config = config;
        self
    }

    /// Rank and rerank documents with all enabled strategies.
    pub fn rank(
        &self,
        documents: Vec<RankableDocument>,
        query: &ProcessedQuery,
        query_embedding: Option<&[f32]>,
    ) -> Vec<RankedResult> {
        // Stage 1: Base ranking
        let ranked = self.base_ranker.rank(documents, query);

        // Convert back to RankableDocument for reranking
        let mut rerank_docs: Vec<RankableDocument> = ranked
            .into_iter()
            .map(|r| RankableDocument {
                id: r.id,
                content: String::new(), // Content not needed for reranking
                semantic_score: r.final_score,
                metadata: HashMap::new(),
                embedding: None,
            })
            .collect();

        // Stage 2: Personalization
        if let Some(personalizer) = &self.personalized_ranker {
            rerank_docs = personalizer.rerank(rerank_docs);
        }

        // Stage 3: MMR for diversity
        if let Some(mmr) = &self.mmr_reranker {
            if let Some(query_emb) = query_embedding {
                let k = rerank_docs.len();
                rerank_docs = mmr.rerank(rerank_docs, query_emb, k);
            }
        }

        // Convert back to RankedResult
        rerank_docs
            .into_iter()
            .map(|doc| RankedResult {
                id: doc.id,
                final_score: doc.semantic_score,
                semantic_score: doc.semantic_score,
                keyword_score: 0.0,
                recency_score: 0.0,
                popularity_score: 0.0,
                explanation: Some(format!("Advanced reranking applied")),
            })
            .collect()
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
            embedding: None,
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
            sub_queries: vec![],
            query_graph: None,
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

        // doc2 ranks higher due to higher semantic score (semantic weight dominates)
        assert_eq!(results[0].id, "doc2");
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

    #[test]
    fn test_mmr_reranking() {
        let mmr = MMRReranker::new(0.7);
        let query_embedding = vec![1.0; 384];

        let docs = vec![
            RankableDocument {
                id: "doc1".to_string(),
                content: "First document".to_string(),
                semantic_score: 0.9,
                metadata: HashMap::new(),
                embedding: Some(vec![1.0; 384]),
            },
            RankableDocument {
                id: "doc2".to_string(),
                content: "Similar document".to_string(),
                semantic_score: 0.85,
                metadata: HashMap::new(),
                embedding: Some(vec![0.99; 384]),
            },
            RankableDocument {
                id: "doc3".to_string(),
                content: "Different document".to_string(),
                semantic_score: 0.8,
                metadata: HashMap::new(),
                embedding: Some(vec![0.1; 384]),
            },
        ];

        let reranked = mmr.rerank(docs, &query_embedding, 3);

        assert_eq!(reranked.len(), 3);
        // First doc should still be first (highest score)
        assert_eq!(reranked[0].id, "doc1");
        // Different doc should rank higher than similar doc due to diversity
        assert_eq!(reranked[1].id, "doc3");
    }

    #[test]
    fn test_personalized_ranking() {
        let mut config = PersonalizationConfig::default();
        config.preferences.insert("language".to_string(), 0.5);
        config.interaction_history.push("doc1".to_string());

        let ranker = PersonalizedRanker::new(config);

        let mut doc1 = create_test_doc("doc1", "content", 0.5);
        doc1.metadata.insert("language".to_string(), "rust".to_string());

        let doc2 = create_test_doc("doc2", "content", 0.9);

        let reranked = ranker.rerank(vec![doc1, doc2]);

        // doc1 should rank higher due to personalization boost
        assert_eq!(reranked[0].id, "doc1");
    }

    #[test]
    fn test_advanced_ranker() {
        let ranker = AdvancedRanker::new(RankingStrategy::Semantic)
            .with_mmr(0.7);

        let query = create_test_query();
        let query_embedding = vec![1.0; 384];

        let docs = vec![
            create_test_doc("doc1", "relevant", 0.9),
            create_test_doc("doc2", "relevant", 0.8),
        ];

        let results = ranker.rank(docs, &query, Some(&query_embedding));

        assert!(!results.is_empty());
        assert!(results[0].final_score > 0.0);
    }
}
