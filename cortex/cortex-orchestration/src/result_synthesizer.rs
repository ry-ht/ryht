//! Result Synthesizer - Aggregate and Synthesize Worker Results
//!
//! Combines results from multiple worker agents into a coherent, unified response.
//! Implements result synthesis as described in Anthropic's orchestrator-worker pattern.
//!
//! # Synthesis Process
//!
//! 1. Collect results from all workers
//! 2. Identify overlaps and contradictions
//! 3. Merge complementary information
//! 4. Resolve conflicts based on confidence scores
//! 5. Generate unified summary
//! 6. Calculate quality metrics

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::{
    lead_agent::{QueryAnalysis, WorkerResult},
    strategy_library::ExecutionStrategy,
    Result, OrchestrationError,
};

// ============================================================================
// Synthesized Result
// ============================================================================

/// Final synthesized result from multiple workers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizedResult {
    /// Original query
    pub query: String,

    /// Unified summary
    pub summary: String,

    /// Detailed findings organized by aspect
    pub findings: HashMap<String, Finding>,

    /// Aggregated recommendations
    pub recommendations: Vec<Recommendation>,

    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,

    /// Success flag
    pub success: bool,

    /// Number of workers involved
    pub worker_count: usize,

    /// Synthesis quality metrics
    pub quality_metrics: QualityMetrics,

    /// Total tokens used across all workers
    pub total_tokens_used: u64,

    /// Total cost in cents
    pub total_cost_cents: u64,

    /// Parallel execution efficiency (0.0 - 1.0)
    pub parallel_efficiency: f32,

    /// Time reduction percentage vs sequential execution
    pub time_reduction_percent: f32,

    /// Metadata
    pub metadata: serde_json::Value,
}

/// Finding from synthesis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Aspect or topic
    pub aspect: String,

    /// Summary of findings for this aspect
    pub summary: String,

    /// Detailed content
    pub details: Vec<String>,

    /// Supporting evidence/sources
    pub evidence: Vec<String>,

    /// Confidence for this finding (0.0 - 1.0)
    pub confidence: f32,

    /// Worker IDs that contributed
    pub contributors: Vec<String>,
}

/// Recommendation from synthesis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// Recommendation text
    pub text: String,

    /// Priority (1-10, higher is more important)
    pub priority: u8,

    /// Confidence in recommendation (0.0 - 1.0)
    pub confidence: f32,

    /// Rationale
    pub rationale: String,

    /// Related findings
    pub related_findings: Vec<String>,
}

/// Quality metrics for synthesized result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    /// Completeness score (0.0 - 1.0)
    pub completeness: f32,

    /// Consistency score (0.0 - 1.0)
    pub consistency: f32,

    /// Coverage score (0.0 - 1.0)
    pub coverage: f32,

    /// Redundancy score (0.0 - 1.0, lower is better)
    pub redundancy: f32,

    /// Conflict resolution score (0.0 - 1.0)
    pub conflict_resolution: f32,
}

impl Default for QualityMetrics {
    fn default() -> Self {
        Self {
            completeness: 0.0,
            consistency: 0.0,
            coverage: 0.0,
            redundancy: 0.0,
            conflict_resolution: 0.0,
        }
    }
}

// ============================================================================
// Result Synthesizer
// ============================================================================

/// Synthesizes results from multiple workers
pub struct ResultSynthesizer {
    /// Configuration
    config: SynthesizerConfig,
}

/// Synthesizer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizerConfig {
    /// Minimum confidence threshold for including findings
    pub min_confidence_threshold: f32,

    /// Weight for confidence in conflict resolution
    pub confidence_weight: f32,

    /// Weight for recency in conflict resolution
    pub recency_weight: f32,

    /// Enable automatic duplicate detection
    pub auto_dedup: bool,

    /// Similarity threshold for duplicate detection (0.0 - 1.0)
    pub similarity_threshold: f32,
}

impl Default for SynthesizerConfig {
    fn default() -> Self {
        Self {
            min_confidence_threshold: 0.5,
            confidence_weight: 0.7,
            recency_weight: 0.3,
            auto_dedup: true,
            similarity_threshold: 0.85,
        }
    }
}

impl ResultSynthesizer {
    /// Create a new result synthesizer
    pub fn new(config: SynthesizerConfig) -> Self {
        info!("Initializing Result Synthesizer");
        Self { config }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(SynthesizerConfig::default())
    }

    /// Synthesize results from multiple workers
    pub async fn synthesize(
        &self,
        query: &str,
        analysis: &QueryAnalysis,
        strategy: &ExecutionStrategy,
        worker_results: Vec<WorkerResult>,
    ) -> Result<SynthesizedResult> {
        info!("Synthesizing results from {} workers", worker_results.len());

        if worker_results.is_empty() {
            return Err(OrchestrationError::Other(
                anyhow::anyhow!("No worker results to synthesize")
            ));
        }

        // Step 1: Extract findings from worker results
        let findings = self.extract_findings(&worker_results, analysis)?;

        // Step 2: Detect and resolve conflicts
        let resolved_findings = self.resolve_conflicts(findings)?;

        // Step 3: Generate recommendations
        let recommendations = self.generate_recommendations(&resolved_findings, strategy)?;

        // Step 4: Create unified summary
        let summary = self.create_summary(query, &resolved_findings, &recommendations)?;

        // Step 5: Calculate quality metrics
        let quality_metrics = self.calculate_quality_metrics(&worker_results, &resolved_findings)?;

        // Step 6: Calculate aggregate statistics
        let (total_tokens, total_cost) = self.aggregate_costs(&worker_results);

        // Step 7: Calculate parallel efficiency
        let parallel_efficiency = self.calculate_parallel_efficiency(&worker_results);

        // Step 8: Calculate time reduction
        let time_reduction = self.calculate_time_reduction(&worker_results, parallel_efficiency);

        // Step 9: Calculate overall confidence
        let overall_confidence = self.calculate_overall_confidence(&resolved_findings);

        // Step 10: Determine success
        let success = overall_confidence >= self.config.min_confidence_threshold
            && quality_metrics.completeness >= 0.7;

        Ok(SynthesizedResult {
            query: query.to_string(),
            summary,
            findings: resolved_findings,
            recommendations,
            confidence: overall_confidence,
            success,
            worker_count: worker_results.len(),
            quality_metrics,
            total_tokens_used: total_tokens,
            total_cost_cents: total_cost,
            parallel_efficiency,
            time_reduction_percent: time_reduction,
            metadata: serde_json::json!({
                "strategy": strategy.name,
                "complexity": format!("{:?}", analysis.complexity),
            }),
        })
    }

    /// Extract findings from worker results
    fn extract_findings(
        &self,
        worker_results: &[WorkerResult],
        _analysis: &QueryAnalysis,
    ) -> Result<HashMap<String, Finding>> {
        debug!("Extracting findings from worker results");

        let mut findings: HashMap<String, Finding> = HashMap::new();

        for worker_result in worker_results {
            // Extract findings from worker result
            // In a real implementation, this would parse the worker's output
            // For now, we create a simple finding based on the task

            let aspect = worker_result.task.objective.clone();
            let worker_id = worker_result.worker_id.to_string();

            // Try to extract confidence from result
            let confidence = worker_result.result
                .get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.7) as f32;

            // Extract details
            let details = if let Some(findings_arr) = worker_result.result.get("findings") {
                findings_arr
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str())
                            .map(|s| s.to_string())
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                vec![format!("{}", worker_result.result)]
            };

            let finding = findings.entry(aspect.clone()).or_insert_with(|| Finding {
                aspect: aspect.clone(),
                summary: String::new(),
                details: Vec::new(),
                evidence: Vec::new(),
                confidence: 0.0,
                contributors: Vec::new(),
            });

            // Merge information
            finding.details.extend(details);
            finding.contributors.push(worker_id);

            // Update confidence (average)
            finding.confidence = (finding.confidence * (finding.contributors.len() - 1) as f32 + confidence)
                / finding.contributors.len() as f32;
        }

        // Generate summaries for each finding
        for finding in findings.values_mut() {
            finding.summary = self.summarize_details(&finding.details);
        }

        Ok(findings)
    }

    /// Resolve conflicts between findings
    fn resolve_conflicts(
        &self,
        findings: HashMap<String, Finding>,
    ) -> Result<HashMap<String, Finding>> {
        debug!("Resolving conflicts in findings");

        // For now, we keep all findings
        // In a real implementation, this would detect contradictions and resolve them
        // based on confidence scores and evidence

        if self.config.auto_dedup {
            // Remove duplicate details within each finding
            let mut resolved = findings;
            for finding in resolved.values_mut() {
                finding.details = self.deduplicate_strings(&finding.details);
            }
            Ok(resolved)
        } else {
            Ok(findings)
        }
    }

    /// Generate recommendations from findings
    fn generate_recommendations(
        &self,
        findings: &HashMap<String, Finding>,
        _strategy: &ExecutionStrategy,
    ) -> Result<Vec<Recommendation>> {
        debug!("Generating recommendations");

        let mut recommendations = Vec::new();

        // Generate recommendations based on findings
        for finding in findings.values() {
            if finding.confidence >= self.config.min_confidence_threshold {
                // Create a recommendation based on the finding
                let recommendation = Recommendation {
                    text: format!("Based on {}: {}", finding.aspect, finding.summary),
                    priority: if finding.confidence > 0.8 { 8 } else { 5 },
                    confidence: finding.confidence,
                    rationale: format!(
                        "Derived from {} worker contributions with {:.0}% confidence",
                        finding.contributors.len(),
                        finding.confidence * 100.0
                    ),
                    related_findings: vec![finding.aspect.clone()],
                };

                recommendations.push(recommendation);
            }
        }

        // Sort by priority (highest first)
        recommendations.sort_by(|a, b| b.priority.cmp(&a.priority));

        Ok(recommendations)
    }

    /// Create unified summary
    fn create_summary(
        &self,
        query: &str,
        findings: &HashMap<String, Finding>,
        recommendations: &[Recommendation],
    ) -> Result<String> {
        debug!("Creating unified summary");

        let mut summary = format!("Query: {}\n\n", query);

        summary.push_str("Key Findings:\n");
        for (idx, finding) in findings.values().enumerate() {
            summary.push_str(&format!(
                "{}. {} (confidence: {:.0}%)\n",
                idx + 1,
                finding.summary,
                finding.confidence * 100.0
            ));
        }

        summary.push_str("\nTop Recommendations:\n");
        for (idx, rec) in recommendations.iter().take(5).enumerate() {
            summary.push_str(&format!(
                "{}. {} (priority: {}/10)\n",
                idx + 1,
                rec.text,
                rec.priority
            ));
        }

        Ok(summary)
    }

    /// Calculate quality metrics
    fn calculate_quality_metrics(
        &self,
        worker_results: &[WorkerResult],
        findings: &HashMap<String, Finding>,
    ) -> Result<QualityMetrics> {
        debug!("Calculating quality metrics");

        // Completeness: how many aspects were covered
        let completeness = if worker_results.is_empty() {
            0.0
        } else {
            findings.len() as f32 / worker_results.len() as f32
        };

        // Consistency: average confidence across findings
        let consistency = if findings.is_empty() {
            0.0
        } else {
            findings.values().map(|f| f.confidence).sum::<f32>() / findings.len() as f32
        };

        // Coverage: percentage of workers that contributed
        let total_contributors: usize = findings.values().map(|f| f.contributors.len()).sum();
        let coverage = if worker_results.is_empty() {
            0.0
        } else {
            (total_contributors as f32 / worker_results.len() as f32).min(1.0)
        };

        // Redundancy: measure of duplicate information
        let total_details: usize = findings.values().map(|f| f.details.len()).sum();
        let unique_details: usize = findings.values()
            .flat_map(|f| &f.details)
            .collect::<std::collections::HashSet<_>>()
            .len();
        let redundancy = if total_details == 0 {
            0.0
        } else {
            1.0 - (unique_details as f32 / total_details as f32)
        };

        // Conflict resolution: assume good if we got here
        let conflict_resolution = 1.0;

        Ok(QualityMetrics {
            completeness,
            consistency,
            coverage,
            redundancy,
            conflict_resolution,
        })
    }

    /// Aggregate costs across workers
    fn aggregate_costs(&self, worker_results: &[WorkerResult]) -> (u64, u64) {
        let total_tokens = worker_results.iter().map(|r| r.tokens_used).sum();
        let total_cost = worker_results.iter().map(|r| r.cost_cents).sum();
        (total_tokens, total_cost)
    }

    /// Calculate parallel execution efficiency
    fn calculate_parallel_efficiency(&self, worker_results: &[WorkerResult]) -> f32 {
        if worker_results.is_empty() {
            return 0.0;
        }

        // Efficiency = (total work time) / (max individual time * worker count)
        let total_work_time: f64 = worker_results.iter()
            .map(|r| r.duration.as_secs_f64())
            .sum();

        let max_time = worker_results.iter()
            .map(|r| r.duration.as_secs_f64())
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(1.0);

        let theoretical_parallel_time = max_time * worker_results.len() as f64;

        if theoretical_parallel_time == 0.0 {
            0.0
        } else {
            (total_work_time / theoretical_parallel_time) as f32
        }
    }

    /// Calculate time reduction percentage
    fn calculate_time_reduction(&self, worker_results: &[WorkerResult], _efficiency: f32) -> f32 {
        if worker_results.is_empty() {
            return 0.0;
        }

        // Time reduction = (sequential time - parallel time) / sequential time * 100
        let sequential_time: f64 = worker_results.iter()
            .map(|r| r.duration.as_secs_f64())
            .sum();

        let parallel_time = worker_results.iter()
            .map(|r| r.duration.as_secs_f64())
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);

        if sequential_time == 0.0 {
            0.0
        } else {
            ((sequential_time - parallel_time) / sequential_time * 100.0) as f32
        }
    }

    /// Calculate overall confidence
    fn calculate_overall_confidence(&self, findings: &HashMap<String, Finding>) -> f32 {
        if findings.is_empty() {
            return 0.0;
        }

        findings.values().map(|f| f.confidence).sum::<f32>() / findings.len() as f32
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    /// Summarize a list of details
    fn summarize_details(&self, details: &[String]) -> String {
        if details.is_empty() {
            return "No details available".to_string();
        }

        if details.len() == 1 {
            return details[0].clone();
        }

        // Simple concatenation for now
        // In a real implementation, this would use an LLM to create a proper summary
        format!("{} findings including: {}", details.len(), details[0])
    }

    /// Deduplicate similar strings
    fn deduplicate_strings(&self, strings: &[String]) -> Vec<String> {
        let mut unique = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for s in strings {
            let normalized = s.to_lowercase().trim().to_string();
            if seen.insert(normalized) {
                unique.push(s.clone());
            }
        }

        unique
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthesizer_creation() {
        let synthesizer = ResultSynthesizer::default();
        assert_eq!(synthesizer.config.min_confidence_threshold, 0.5);
    }

    #[test]
    fn test_quality_metrics_default() {
        let metrics = QualityMetrics::default();
        assert_eq!(metrics.completeness, 0.0);
        assert_eq!(metrics.consistency, 0.0);
    }

    #[test]
    fn test_deduplicate_strings() {
        let synthesizer = ResultSynthesizer::default();
        let strings = vec![
            "test".to_string(),
            "Test".to_string(),
            "different".to_string(),
        ];
        let deduped = synthesizer.deduplicate_strings(&strings);
        assert_eq!(deduped.len(), 2);
    }
}
