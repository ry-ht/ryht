//! Evaluation metrics for semantic search and RAG systems.
//!
//! Implements standard IR (Information Retrieval) metrics:
//! - NDCG (Normalized Discounted Cumulative Gain)
//! - MRR (Mean Reciprocal Rank)
//! - Precision@K, Recall@K, F1@K
//! - MAP (Mean Average Precision)
//!
//! # References
//! - "Information Retrieval: Implementing and Evaluating Search Engines" (Büttcher et al., 2010)
//! - "Offline Evaluation of Recommendation Functions" (Shani & Gunawardana, 2011)
//! - "A Short Introduction to Learning to Rank" (Li, 2011)

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// A single evaluation result for a query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryEvaluation {
    /// Query ID
    pub query_id: String,
    /// Retrieved document IDs in ranked order
    pub retrieved: Vec<String>,
    /// Relevant document IDs (ground truth)
    pub relevant: HashSet<String>,
    /// Optional relevance scores (0-N scale, higher is more relevant)
    pub relevance_scores: Option<HashMap<String, u32>>,
}

/// Comprehensive evaluation metrics for a query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    /// Precision at various K values
    pub precision_at_k: HashMap<usize, f64>,
    /// Recall at various K values
    pub recall_at_k: HashMap<usize, f64>,
    /// F1 score at various K values
    pub f1_at_k: HashMap<usize, f64>,
    /// Mean Reciprocal Rank
    pub mrr: f64,
    /// Normalized Discounted Cumulative Gain at various K values
    pub ndcg_at_k: HashMap<usize, f64>,
    /// Average Precision
    pub average_precision: f64,
    /// Number of relevant documents retrieved
    pub num_relevant_retrieved: usize,
    /// Total number of relevant documents
    pub total_relevant: usize,
}

/// Aggregated metrics across multiple queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetrics {
    /// Mean metrics across all queries
    pub mean_precision_at_k: HashMap<usize, f64>,
    pub mean_recall_at_k: HashMap<usize, f64>,
    pub mean_f1_at_k: HashMap<usize, f64>,
    pub mean_ndcg_at_k: HashMap<usize, f64>,
    pub mean_reciprocal_rank: f64,
    pub mean_average_precision: f64,
    /// Number of queries evaluated
    pub num_queries: usize,
}

/// Metric evaluator for search results.
///
/// # Example
/// ```
/// use cortex_semantic::eval::{MetricEvaluator, QueryEvaluation};
/// use std::collections::HashSet;
///
/// let evaluator = MetricEvaluator::new();
///
/// let mut relevant = HashSet::new();
/// relevant.insert("doc1".to_string());
/// relevant.insert("doc2".to_string());
///
/// let query_eval = QueryEvaluation {
///     query_id: "q1".to_string(),
///     retrieved: vec!["doc1".to_string(), "doc3".to_string(), "doc2".to_string()],
///     relevant,
///     relevance_scores: None,
/// };
///
/// let metrics = evaluator.evaluate(&query_eval, &[1, 3, 5]);
/// println!("Precision@1: {:.3}", metrics.precision_at_k[&1]);
/// println!("NDCG@3: {:.3}", metrics.ndcg_at_k[&3]);
/// ```
pub struct MetricEvaluator {
    // Future: Could include configuration options
}

impl MetricEvaluator {
    pub fn new() -> Self {
        Self {}
    }

    /// Evaluate metrics for a single query.
    ///
    /// # Parameters
    /// - `query_eval`: Query evaluation data with retrieved and relevant documents
    /// - `k_values`: List of K values for Precision@K, Recall@K, etc.
    pub fn evaluate(&self, query_eval: &QueryEvaluation, k_values: &[usize]) -> Metrics {
        let mut precision_at_k = HashMap::new();
        let mut recall_at_k = HashMap::new();
        let mut f1_at_k = HashMap::new();
        let mut ndcg_at_k = HashMap::new();

        for &k in k_values {
            let p = self.precision_at_k(&query_eval.retrieved, &query_eval.relevant, k);
            let r = self.recall_at_k(&query_eval.retrieved, &query_eval.relevant, k);
            let f1 = self.f1_score(p, r);

            precision_at_k.insert(k, p);
            recall_at_k.insert(k, r);
            f1_at_k.insert(k, f1);

            let ndcg = self.ndcg_at_k(
                &query_eval.retrieved,
                &query_eval.relevant,
                query_eval.relevance_scores.as_ref(),
                k,
            );
            ndcg_at_k.insert(k, ndcg);
        }

        let mrr = self.mean_reciprocal_rank(&query_eval.retrieved, &query_eval.relevant);
        let average_precision =
            self.average_precision(&query_eval.retrieved, &query_eval.relevant);

        let num_relevant_retrieved = query_eval
            .retrieved
            .iter()
            .filter(|doc_id| query_eval.relevant.contains(*doc_id))
            .count();

        Metrics {
            precision_at_k,
            recall_at_k,
            f1_at_k,
            mrr,
            ndcg_at_k,
            average_precision,
            num_relevant_retrieved,
            total_relevant: query_eval.relevant.len(),
        }
    }

    /// Calculate Precision@K.
    ///
    /// Precision@K = (# of relevant items in top K) / K
    pub fn precision_at_k(
        &self,
        retrieved: &[String],
        relevant: &HashSet<String>,
        k: usize,
    ) -> f64 {
        if k == 0 || retrieved.is_empty() {
            return 0.0;
        }

        let top_k = retrieved.iter().take(k);
        let relevant_in_top_k = top_k.filter(|doc_id| relevant.contains(*doc_id)).count();

        relevant_in_top_k as f64 / k.min(retrieved.len()) as f64
    }

    /// Calculate Recall@K.
    ///
    /// Recall@K = (# of relevant items in top K) / (total # of relevant items)
    pub fn recall_at_k(
        &self,
        retrieved: &[String],
        relevant: &HashSet<String>,
        k: usize,
    ) -> f64 {
        if relevant.is_empty() {
            return 0.0;
        }

        let top_k = retrieved.iter().take(k);
        let relevant_in_top_k = top_k.filter(|doc_id| relevant.contains(*doc_id)).count();

        relevant_in_top_k as f64 / relevant.len() as f64
    }

    /// Calculate F1 score from precision and recall.
    ///
    /// F1 = 2 * (Precision * Recall) / (Precision + Recall)
    pub fn f1_score(&self, precision: f64, recall: f64) -> f64 {
        if precision + recall == 0.0 {
            return 0.0;
        }

        2.0 * precision * recall / (precision + recall)
    }

    /// Calculate Mean Reciprocal Rank (MRR).
    ///
    /// MRR = 1 / rank of first relevant document
    /// Returns 0 if no relevant documents found.
    ///
    /// Reference: "Question Answering" (Voorhees, 1999)
    pub fn mean_reciprocal_rank(&self, retrieved: &[String], relevant: &HashSet<String>) -> f64 {
        for (i, doc_id) in retrieved.iter().enumerate() {
            if relevant.contains(doc_id) {
                return 1.0 / (i + 1) as f64;
            }
        }
        0.0
    }

    /// Calculate Average Precision (AP).
    ///
    /// AP = (sum of P@k for each relevant doc at position k) / (total relevant docs)
    ///
    /// Reference: "The TREC-8 Question Answering Track" (Voorhees, 2000)
    pub fn average_precision(&self, retrieved: &[String], relevant: &HashSet<String>) -> f64 {
        if relevant.is_empty() {
            return 0.0;
        }

        let mut sum_precision = 0.0;
        let mut num_relevant_seen = 0;

        for (i, doc_id) in retrieved.iter().enumerate() {
            if relevant.contains(doc_id) {
                num_relevant_seen += 1;
                let precision_at_i = num_relevant_seen as f64 / (i + 1) as f64;
                sum_precision += precision_at_i;
            }
        }

        sum_precision / relevant.len() as f64
    }

    /// Calculate Normalized Discounted Cumulative Gain (NDCG@K).
    ///
    /// NDCG accounts for position of relevant documents and graded relevance.
    ///
    /// DCG@K = sum_{i=1}^{K} (2^{rel_i} - 1) / log_2(i + 1)
    /// NDCG@K = DCG@K / IDCG@K
    ///
    /// Reference: "Cumulated Gain-Based Evaluation of IR Techniques" (Järvelin & Kekäläinen, 2002)
    pub fn ndcg_at_k(
        &self,
        retrieved: &[String],
        relevant: &HashSet<String>,
        relevance_scores: Option<&HashMap<String, u32>>,
        k: usize,
    ) -> f64 {
        if k == 0 || relevant.is_empty() {
            return 0.0;
        }

        // Calculate DCG@K
        let dcg = self.dcg_at_k(retrieved, relevant, relevance_scores, k);

        // Calculate ideal DCG@K (IDCG)
        let ideal_retrieved = self.create_ideal_ranking(relevant, relevance_scores);
        let idcg = self.dcg_at_k(&ideal_retrieved, relevant, relevance_scores, k);

        if idcg == 0.0 {
            return 0.0;
        }

        dcg / idcg
    }

    /// Calculate Discounted Cumulative Gain (DCG@K).
    fn dcg_at_k(
        &self,
        retrieved: &[String],
        relevant: &HashSet<String>,
        relevance_scores: Option<&HashMap<String, u32>>,
        k: usize,
    ) -> f64 {
        let mut dcg = 0.0;

        for (i, doc_id) in retrieved.iter().take(k).enumerate() {
            if relevant.contains(doc_id) {
                let rel = if let Some(scores) = relevance_scores {
                    *scores.get(doc_id).unwrap_or(&1) as f64
                } else {
                    1.0 // Binary relevance
                };

                let position = i + 1;
                let gain = (2_f64.powf(rel) - 1.0) / (position as f64 + 1.0).log2();
                dcg += gain;
            }
        }

        dcg
    }

    /// Create ideal ranking for IDCG calculation.
    fn create_ideal_ranking(
        &self,
        relevant: &HashSet<String>,
        relevance_scores: Option<&HashMap<String, u32>>,
    ) -> Vec<String> {
        let mut ranked: Vec<_> = relevant.iter().cloned().collect();

        if let Some(scores) = relevance_scores {
            // Sort by relevance score descending
            ranked.sort_by(|a, b| {
                let score_a = scores.get(a).unwrap_or(&0);
                let score_b = scores.get(b).unwrap_or(&0);
                score_b.cmp(score_a)
            });
        }

        ranked
    }

    /// Aggregate metrics across multiple queries.
    ///
    /// Calculates mean metrics across all query evaluations.
    pub fn aggregate(&self, metrics: &[Metrics]) -> AggregatedMetrics {
        if metrics.is_empty() {
            return AggregatedMetrics {
                mean_precision_at_k: HashMap::new(),
                mean_recall_at_k: HashMap::new(),
                mean_f1_at_k: HashMap::new(),
                mean_ndcg_at_k: HashMap::new(),
                mean_reciprocal_rank: 0.0,
                mean_average_precision: 0.0,
                num_queries: 0,
            };
        }

        let num_queries = metrics.len() as f64;

        // Aggregate MRR and MAP
        let mean_reciprocal_rank = metrics.iter().map(|m| m.mrr).sum::<f64>() / num_queries;
        let mean_average_precision =
            metrics.iter().map(|m| m.average_precision).sum::<f64>() / num_queries;

        // Aggregate metrics at K
        let mut all_k_values: HashSet<usize> = HashSet::new();
        for m in metrics {
            all_k_values.extend(m.precision_at_k.keys());
        }

        let mut mean_precision_at_k = HashMap::new();
        let mut mean_recall_at_k = HashMap::new();
        let mut mean_f1_at_k = HashMap::new();
        let mut mean_ndcg_at_k = HashMap::new();

        for k in all_k_values {
            let sum_p: f64 = metrics
                .iter()
                .filter_map(|m| m.precision_at_k.get(&k))
                .sum();
            let sum_r: f64 = metrics.iter().filter_map(|m| m.recall_at_k.get(&k)).sum();
            let sum_f1: f64 = metrics.iter().filter_map(|m| m.f1_at_k.get(&k)).sum();
            let sum_ndcg: f64 = metrics.iter().filter_map(|m| m.ndcg_at_k.get(&k)).sum();

            mean_precision_at_k.insert(k, sum_p / num_queries);
            mean_recall_at_k.insert(k, sum_r / num_queries);
            mean_f1_at_k.insert(k, sum_f1 / num_queries);
            mean_ndcg_at_k.insert(k, sum_ndcg / num_queries);
        }

        AggregatedMetrics {
            mean_precision_at_k,
            mean_recall_at_k,
            mean_f1_at_k,
            mean_ndcg_at_k,
            mean_reciprocal_rank,
            mean_average_precision,
            num_queries: metrics.len(),
        }
    }
}

impl Default for MetricEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

/// Time-series metrics tracker for monitoring performance over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsTimeSeries {
    /// Timestamp -> Aggregated metrics
    pub data_points: Vec<(chrono::DateTime<chrono::Utc>, AggregatedMetrics)>,
}

impl MetricsTimeSeries {
    pub fn new() -> Self {
        Self {
            data_points: Vec::new(),
        }
    }

    /// Add a new data point with current timestamp.
    pub fn add(&mut self, metrics: AggregatedMetrics) {
        let timestamp = chrono::Utc::now();
        self.data_points.push((timestamp, metrics));
    }

    /// Get metrics for a specific time range.
    pub fn get_range(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Vec<&AggregatedMetrics> {
        self.data_points
            .iter()
            .filter(|(ts, _)| *ts >= start && *ts <= end)
            .map(|(_, metrics)| metrics)
            .collect()
    }

    /// Calculate trend (improvement/degradation) for a metric over time.
    pub fn calculate_trend(&self, metric_extractor: fn(&AggregatedMetrics) -> f64) -> f64 {
        if self.data_points.len() < 2 {
            return 0.0;
        }

        let values: Vec<f64> = self
            .data_points
            .iter()
            .map(|(_, m)| metric_extractor(m))
            .collect();

        // Simple linear regression slope
        let n = values.len() as f64;
        let x_mean = (n - 1.0) / 2.0;
        let y_mean = values.iter().sum::<f64>() / n;

        let mut numerator = 0.0;
        let mut denominator = 0.0;

        for (i, &y) in values.iter().enumerate() {
            let x = i as f64;
            numerator += (x - x_mean) * (y - y_mean);
            denominator += (x - x_mean).powi(2);
        }

        if denominator == 0.0 {
            0.0
        } else {
            numerator / denominator
        }
    }
}

impl Default for MetricsTimeSeries {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_eval() -> QueryEvaluation {
        let mut relevant = HashSet::new();
        relevant.insert("doc1".to_string());
        relevant.insert("doc2".to_string());
        relevant.insert("doc3".to_string());

        QueryEvaluation {
            query_id: "q1".to_string(),
            retrieved: vec![
                "doc1".to_string(),
                "doc4".to_string(),
                "doc2".to_string(),
                "doc5".to_string(),
                "doc3".to_string(),
            ],
            relevant,
            relevance_scores: None,
        }
    }

    #[test]
    fn test_precision_at_k() {
        let evaluator = MetricEvaluator::new();
        let eval = create_test_eval();

        let p1 = evaluator.precision_at_k(&eval.retrieved, &eval.relevant, 1);
        assert_eq!(p1, 1.0); // doc1 is relevant

        let p3 = evaluator.precision_at_k(&eval.retrieved, &eval.relevant, 3);
        assert!((p3 - 0.666).abs() < 0.01); // 2 out of 3 are relevant

        let p5 = evaluator.precision_at_k(&eval.retrieved, &eval.relevant, 5);
        assert_eq!(p5, 0.6); // 3 out of 5 are relevant
    }

    #[test]
    fn test_recall_at_k() {
        let evaluator = MetricEvaluator::new();
        let eval = create_test_eval();

        let r1 = evaluator.recall_at_k(&eval.retrieved, &eval.relevant, 1);
        assert!((r1 - 0.333).abs() < 0.01); // 1 out of 3 relevant docs

        let r5 = evaluator.recall_at_k(&eval.retrieved, &eval.relevant, 5);
        assert_eq!(r5, 1.0); // All 3 relevant docs retrieved
    }

    #[test]
    fn test_f1_score() {
        let evaluator = MetricEvaluator::new();

        let f1 = evaluator.f1_score(0.5, 0.5);
        assert_eq!(f1, 0.5);

        let f1 = evaluator.f1_score(1.0, 0.5);
        assert!((f1 - 0.666).abs() < 0.01);

        let f1 = evaluator.f1_score(0.0, 0.0);
        assert_eq!(f1, 0.0);
    }

    #[test]
    fn test_mrr() {
        let evaluator = MetricEvaluator::new();
        let eval = create_test_eval();

        let mrr = evaluator.mean_reciprocal_rank(&eval.retrieved, &eval.relevant);
        assert_eq!(mrr, 1.0); // First result is relevant

        let mut eval2 = eval.clone();
        eval2.retrieved = vec!["doc4".to_string(), "doc5".to_string(), "doc1".to_string()];
        let mrr2 = evaluator.mean_reciprocal_rank(&eval2.retrieved, &eval2.relevant);
        assert!((mrr2 - 0.333).abs() < 0.01); // First relevant at position 3
    }

    #[test]
    fn test_average_precision() {
        let evaluator = MetricEvaluator::new();
        let eval = create_test_eval();

        let ap = evaluator.average_precision(&eval.retrieved, &eval.relevant);
        // AP = (1/1 + 2/3 + 3/5) / 3 = (1.0 + 0.666 + 0.6) / 3 ≈ 0.755
        assert!((ap - 0.755).abs() < 0.01);
    }

    #[test]
    fn test_ndcg() {
        let evaluator = MetricEvaluator::new();
        let eval = create_test_eval();

        let ndcg3 = evaluator.ndcg_at_k(&eval.retrieved, &eval.relevant, None, 3);
        assert!(ndcg3 > 0.0 && ndcg3 <= 1.0);

        let ndcg5 = evaluator.ndcg_at_k(&eval.retrieved, &eval.relevant, None, 5);
        assert!(ndcg5 > 0.0 && ndcg5 <= 1.0);
        assert!(ndcg5 >= ndcg3); // More results should not decrease NDCG significantly
    }

    #[test]
    fn test_ndcg_with_graded_relevance() {
        let evaluator = MetricEvaluator::new();

        let mut relevant = HashSet::new();
        relevant.insert("doc1".to_string());
        relevant.insert("doc2".to_string());

        let mut relevance_scores = HashMap::new();
        relevance_scores.insert("doc1".to_string(), 3); // Highly relevant
        relevance_scores.insert("doc2".to_string(), 1); // Somewhat relevant

        let eval = QueryEvaluation {
            query_id: "q1".to_string(),
            retrieved: vec!["doc1".to_string(), "doc2".to_string()],
            relevant: relevant.clone(),
            relevance_scores: Some(relevance_scores.clone()),
        };

        let ndcg = evaluator.ndcg_at_k(&eval.retrieved, &eval.relevant, eval.relevance_scores.as_ref(), 2);
        assert_eq!(ndcg, 1.0); // Perfect ranking

        // Test with reversed order
        let eval2 = QueryEvaluation {
            query_id: "q1".to_string(),
            retrieved: vec!["doc2".to_string(), "doc1".to_string()],
            relevant,
            relevance_scores: Some(relevance_scores),
        };

        let ndcg2 = evaluator.ndcg_at_k(&eval2.retrieved, &eval2.relevant, eval2.relevance_scores.as_ref(), 2);
        assert!(ndcg2 < 1.0); // Suboptimal ranking
    }

    #[test]
    fn test_full_evaluation() {
        let evaluator = MetricEvaluator::new();
        let eval = create_test_eval();

        let metrics = evaluator.evaluate(&eval, &[1, 3, 5, 10]);

        assert!(metrics.precision_at_k.contains_key(&1));
        assert!(metrics.recall_at_k.contains_key(&1));
        assert!(metrics.ndcg_at_k.contains_key(&1));
        assert!(metrics.mrr > 0.0);
        assert!(metrics.average_precision > 0.0);
        assert_eq!(metrics.num_relevant_retrieved, 3);
        assert_eq!(metrics.total_relevant, 3);
    }

    #[test]
    fn test_aggregation() {
        let evaluator = MetricEvaluator::new();

        let eval1 = create_test_eval();
        let metrics1 = evaluator.evaluate(&eval1, &[1, 3, 5]);

        let mut eval2 = create_test_eval();
        eval2.query_id = "q2".to_string();
        let metrics2 = evaluator.evaluate(&eval2, &[1, 3, 5]);

        let aggregated = evaluator.aggregate(&[metrics1, metrics2]);

        assert_eq!(aggregated.num_queries, 2);
        assert!(aggregated.mean_reciprocal_rank > 0.0);
        assert!(aggregated.mean_average_precision > 0.0);
        assert!(aggregated.mean_precision_at_k.contains_key(&1));
    }

    #[test]
    fn test_metrics_time_series() {
        let mut ts = MetricsTimeSeries::new();

        let aggregated = AggregatedMetrics {
            mean_precision_at_k: HashMap::new(),
            mean_recall_at_k: HashMap::new(),
            mean_f1_at_k: HashMap::new(),
            mean_ndcg_at_k: HashMap::new(),
            mean_reciprocal_rank: 0.8,
            mean_average_precision: 0.75,
            num_queries: 10,
        };

        ts.add(aggregated.clone());
        ts.add(aggregated);

        assert_eq!(ts.data_points.len(), 2);
    }
}
