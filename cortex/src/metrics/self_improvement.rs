/// Self-improvement metrics tracking system
///
/// This module tracks code health and quality metrics to enable continuous
/// self-improvement of the codebase through automated analysis and recommendations.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

/// Self-improvement metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfImprovementMetrics {
    /// Timestamp of this metrics snapshot
    pub timestamp: DateTime<Utc>,

    /// Overall codebase health score (0.0 to 1.0)
    /// Calculated from: code quality, test coverage, complexity, and technical debt
    pub health_score: f64,

    /// Code quality score (0.0 to 1.0)
    /// Based on: low complexity, good documentation, no circular deps
    pub code_quality_score: f64,

    /// Test coverage percentage (0.0 to 100.0)
    pub test_coverage_percent: f64,

    /// Average cyclomatic complexity across all symbols
    pub avg_cyclomatic_complexity: f64,

    /// Number of circular dependencies detected
    pub circular_dependencies_count: u64,

    /// Number of symbols without tests
    pub untested_symbols_count: u64,

    /// Number of symbols without documentation
    pub undocumented_symbols_count: u64,

    /// Number of high-complexity symbols (complexity > 10)
    pub high_complexity_symbols_count: u64,

    /// Number of improvement tasks completed this week
    pub improvements_per_week: u64,

    /// Average improvement task completion time (hours)
    pub avg_improvement_time_hours: f64,

    /// Technical debt score (0.0 to 1.0, lower is better)
    /// Based on: untested code, undocumented code, high complexity
    pub technical_debt_score: f64,

    /// Breakdown by language
    pub language_breakdown: HashMap<String, LanguageMetrics>,

    /// Trend data (optional)
    pub trend_direction: TrendDirection,
}

/// Language-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageMetrics {
    pub language: String,
    pub symbol_count: u64,
    pub avg_complexity: f64,
    pub test_coverage_percent: f64,
    pub health_score: f64,
}

/// Trend direction for metrics over time
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrendDirection {
    Improving,
    Stable,
    Degrading,
}

impl SelfImprovementMetrics {
    /// Create a new metrics snapshot with default values
    pub fn new() -> Self {
        Self {
            timestamp: Utc::now(),
            health_score: 0.0,
            code_quality_score: 0.0,
            test_coverage_percent: 0.0,
            avg_cyclomatic_complexity: 0.0,
            circular_dependencies_count: 0,
            untested_symbols_count: 0,
            undocumented_symbols_count: 0,
            high_complexity_symbols_count: 0,
            improvements_per_week: 0,
            avg_improvement_time_hours: 0.0,
            technical_debt_score: 0.0,
            language_breakdown: HashMap::new(),
            trend_direction: TrendDirection::Stable,
        }
    }

    /// Calculate overall health score from component metrics
    ///
    /// Formula:
    /// health_score = (code_quality * 0.4) + (test_coverage/100 * 0.3) +
    ///                ((1 - technical_debt) * 0.2) + (complexity_score * 0.1)
    pub fn calculate_health_score(&mut self) {
        // Complexity score: 1.0 if avg < 5, scales down to 0.0 at avg >= 20
        let complexity_score = if self.avg_cyclomatic_complexity < 5.0 {
            1.0
        } else if self.avg_cyclomatic_complexity >= 20.0 {
            0.0
        } else {
            1.0 - ((self.avg_cyclomatic_complexity - 5.0) / 15.0)
        };

        self.health_score = (self.code_quality_score * 0.4)
            + (self.test_coverage_percent / 100.0 * 0.3)
            + ((1.0 - self.technical_debt_score) * 0.2)
            + (complexity_score * 0.1);

        // Clamp to [0.0, 1.0]
        self.health_score = self.health_score.clamp(0.0, 1.0);
    }

    /// Calculate code quality score from metrics
    ///
    /// Quality is high when:
    /// - Low complexity
    /// - High documentation coverage
    /// - No circular dependencies
    pub fn calculate_code_quality(&mut self, total_symbols: u64) {
        if total_symbols == 0 {
            self.code_quality_score = 1.0;
            return;
        }

        // Documentation coverage (0.0 to 1.0)
        let doc_coverage = 1.0 - (self.undocumented_symbols_count as f64 / total_symbols as f64);

        // Complexity penalty (0.0 to 1.0)
        let complexity_penalty = if self.high_complexity_symbols_count == 0 {
            1.0
        } else {
            1.0 - (self.high_complexity_symbols_count as f64 / total_symbols as f64)
        };

        // Circular dependency penalty
        let circular_penalty = if self.circular_dependencies_count == 0 {
            1.0
        } else {
            // Each circular dep reduces score by 5%, capped at 50% reduction
            (1.0 - (self.circular_dependencies_count as f64 * 0.05)).max(0.5)
        };

        self.code_quality_score = (doc_coverage * 0.4)
            + (complexity_penalty * 0.4)
            + (circular_penalty * 0.2);

        self.code_quality_score = self.code_quality_score.clamp(0.0, 1.0);
    }

    /// Calculate technical debt score
    ///
    /// Technical debt accumulates from:
    /// - Untested code
    /// - Undocumented code
    /// - High complexity code
    pub fn calculate_technical_debt(&mut self, total_symbols: u64) {
        if total_symbols == 0 {
            self.technical_debt_score = 0.0;
            return;
        }

        let untested_ratio = self.untested_symbols_count as f64 / total_symbols as f64;
        let undocumented_ratio = self.undocumented_symbols_count as f64 / total_symbols as f64;
        let high_complexity_ratio = self.high_complexity_symbols_count as f64 / total_symbols as f64;

        self.technical_debt_score = (untested_ratio * 0.4)
            + (undocumented_ratio * 0.3)
            + (high_complexity_ratio * 0.3);

        self.technical_debt_score = self.technical_debt_score.clamp(0.0, 1.0);
    }

    /// Update trend direction by comparing with previous metrics
    pub fn calculate_trend(&mut self, previous: Option<&SelfImprovementMetrics>) {
        if let Some(prev) = previous {
            let health_diff = self.health_score - prev.health_score;

            if health_diff > 0.05 {
                self.trend_direction = TrendDirection::Improving;
            } else if health_diff < -0.05 {
                self.trend_direction = TrendDirection::Degrading;
            } else {
                self.trend_direction = TrendDirection::Stable;
            }
        }
    }

    /// Get a human-readable health rating
    pub fn health_rating(&self) -> &'static str {
        match self.health_score {
            s if s >= 0.9 => "Excellent",
            s if s >= 0.75 => "Good",
            s if s >= 0.6 => "Fair",
            s if s >= 0.4 => "Poor",
            _ => "Critical",
        }
    }
}

impl Default for SelfImprovementMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics collector for self-improvement tracking
pub struct SelfImprovementCollector {
    db: Arc<Surreal<Db>>,
}

impl SelfImprovementCollector {
    /// Create a new self-improvement metrics collector
    pub fn new(db: Arc<Surreal<Db>>) -> Self {
        Self { db }
    }

    /// Collect current self-improvement metrics from the database
    pub async fn collect(&self) -> Result<SelfImprovementMetrics> {
        let mut metrics = SelfImprovementMetrics::new();

        // Query total symbols count
        let total_symbols = self.count_total_symbols().await?;

        // Query complexity metrics
        let complexity_data = self.query_complexity_metrics().await?;
        metrics.avg_cyclomatic_complexity = complexity_data.avg_complexity;
        metrics.high_complexity_symbols_count = complexity_data.high_complexity_count;

        // Query test coverage
        metrics.untested_symbols_count = self.count_untested_symbols().await?;
        if total_symbols > 0 {
            metrics.test_coverage_percent =
                ((total_symbols - metrics.untested_symbols_count) as f64 / total_symbols as f64) * 100.0;
        }

        // Query documentation coverage
        metrics.undocumented_symbols_count = self.count_undocumented_symbols().await?;

        // Query circular dependencies
        metrics.circular_dependencies_count = self.count_circular_dependencies().await?;

        // Query improvement tasks
        let improvement_data = self.query_improvement_tasks().await?;
        metrics.improvements_per_week = improvement_data.completed_this_week;
        metrics.avg_improvement_time_hours = improvement_data.avg_completion_time;

        // Calculate derived metrics
        metrics.calculate_technical_debt(total_symbols);
        metrics.calculate_code_quality(total_symbols);
        metrics.calculate_health_score();

        // Query previous metrics for trend
        if let Some(previous) = self.get_previous_metrics().await? {
            metrics.calculate_trend(Some(&previous));
        }

        // Get language breakdown
        metrics.language_breakdown = self.query_language_breakdown().await?;

        Ok(metrics)
    }

    /// Store metrics snapshot to database
    pub async fn store(&self, metrics: &SelfImprovementMetrics) -> Result<()> {
        let _: Option<SelfImprovementMetrics> = self
            .db
            .create("improvement_metrics")
            .content(metrics.clone())
            .await?;

        Ok(())
    }

    /// Get the most recent metrics snapshot
    pub async fn get_latest(&self) -> Result<Option<SelfImprovementMetrics>> {
        let mut result: Vec<SelfImprovementMetrics> = self
            .db
            .query("SELECT * FROM improvement_metrics ORDER BY timestamp DESC LIMIT 1")
            .await?
            .take(0)?;

        Ok(result.pop())
    }

    /// Get metrics history for the past N days
    pub async fn get_history(&self, days: i64) -> Result<Vec<SelfImprovementMetrics>> {
        let cutoff = Utc::now() - chrono::Duration::days(days);

        let results: Vec<SelfImprovementMetrics> = self
            .db
            .query("SELECT * FROM improvement_metrics WHERE timestamp >= $cutoff ORDER BY timestamp ASC")
            .bind(("cutoff", cutoff))
            .await?
            .take(0)?;

        Ok(results)
    }

    // Private helper methods

    async fn count_total_symbols(&self) -> Result<u64> {
        let result: Option<CountResult> = self
            .db
            .query("SELECT count() AS count FROM code_symbol GROUP ALL")
            .await?
            .take(0)?;

        Ok(result.map(|r| r.count).unwrap_or(0))
    }

    async fn query_complexity_metrics(&self) -> Result<ComplexityData> {
        let result: Option<ComplexityQueryResult> = self
            .db
            .query(
                "SELECT
                    math::mean(metadata.complexity) AS avg_complexity,
                    count(metadata.complexity > 10) AS high_count
                FROM code_symbol
                WHERE metadata.complexity IS NOT NONE
                GROUP ALL"
            )
            .await?
            .take(0)?;

        Ok(result.map(|r| ComplexityData {
            avg_complexity: r.avg_complexity.unwrap_or(0.0),
            high_complexity_count: r.high_count.unwrap_or(0),
        }).unwrap_or_default())
    }

    async fn count_untested_symbols(&self) -> Result<u64> {
        let result: Option<CountResult> = self
            .db
            .query(
                "SELECT count() AS count
                FROM code_symbol
                WHERE metadata.test_coverage IS NONE OR metadata.test_coverage = 0
                GROUP ALL"
            )
            .await?
            .take(0)?;

        Ok(result.map(|r| r.count).unwrap_or(0))
    }

    async fn count_undocumented_symbols(&self) -> Result<u64> {
        // Symbols are undocumented if they have no outgoing 'documents' relationship
        let result: Option<CountResult> = self
            .db
            .query(
                "SELECT count() AS count
                FROM code_symbol
                WHERE count(->documents) = 0
                GROUP ALL"
            )
            .await?
            .take(0)?;

        Ok(result.map(|r| r.count).unwrap_or(0))
    }

    async fn count_circular_dependencies(&self) -> Result<u64> {
        // This is a simplified check - production should use graph cycle detection
        // For now, return 0 as placeholder
        Ok(0)
    }

    async fn query_improvement_tasks(&self) -> Result<ImprovementTaskData> {
        let one_week_ago = Utc::now() - chrono::Duration::days(7);

        let result: Option<ImprovementQueryResult> = self
            .db
            .query(
                "SELECT
                    count() AS completed,
                    math::mean(actual_hours) AS avg_hours
                FROM task
                WHERE status = 'done'
                  AND completed_at >= $cutoff
                  AND tags CONTAINS 'refactor' OR tags CONTAINS 'code-quality'
                GROUP ALL"
            )
            .bind(("cutoff", one_week_ago))
            .await?
            .take(0)?;

        Ok(result.map(|r| ImprovementTaskData {
            completed_this_week: r.completed.unwrap_or(0),
            avg_completion_time: r.avg_hours.unwrap_or(0.0),
        }).unwrap_or_default())
    }

    async fn get_previous_metrics(&self) -> Result<Option<SelfImprovementMetrics>> {
        let mut result: Vec<SelfImprovementMetrics> = self
            .db
            .query("SELECT * FROM improvement_metrics ORDER BY timestamp DESC LIMIT 1 START AT 1")
            .await?
            .take(0)?;

        Ok(result.pop())
    }

    async fn query_language_breakdown(&self) -> Result<HashMap<String, LanguageMetrics>> {
        let results: Vec<LanguageBreakdownResult> = self
            .db
            .query(
                "SELECT
                    language,
                    count() AS symbol_count,
                    math::mean(metadata.complexity) AS avg_complexity,
                    count(metadata.test_coverage > 0) / count() * 100 AS coverage
                FROM code_symbol
                GROUP BY language"
            )
            .await?
            .take(0)?;

        let mut breakdown = HashMap::new();
        for result in results {
            let health = (1.0 - (result.avg_complexity.min(20.0) / 20.0)) * 0.5
                + (result.coverage / 100.0) * 0.5;

            breakdown.insert(
                result.language.clone(),
                LanguageMetrics {
                    language: result.language,
                    symbol_count: result.symbol_count,
                    avg_complexity: result.avg_complexity,
                    test_coverage_percent: result.coverage,
                    health_score: health,
                },
            );
        }

        Ok(breakdown)
    }
}

// Helper types for database queries

#[derive(Debug, Deserialize)]
struct CountResult {
    count: u64,
}

#[derive(Debug, Deserialize)]
struct ComplexityQueryResult {
    avg_complexity: Option<f64>,
    high_count: Option<u64>,
}

#[derive(Debug, Default)]
struct ComplexityData {
    avg_complexity: f64,
    high_complexity_count: u64,
}

#[derive(Debug, Deserialize)]
struct ImprovementQueryResult {
    completed: Option<u64>,
    avg_hours: Option<f64>,
}

#[derive(Debug, Default)]
struct ImprovementTaskData {
    completed_this_week: u64,
    avg_completion_time: f64,
}

#[derive(Debug, Deserialize)]
struct LanguageBreakdownResult {
    language: String,
    symbol_count: u64,
    avg_complexity: f64,
    coverage: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_score_calculation() {
        let mut metrics = SelfImprovementMetrics::new();
        metrics.code_quality_score = 0.8;
        metrics.test_coverage_percent = 75.0;
        metrics.technical_debt_score = 0.2;
        metrics.avg_cyclomatic_complexity = 5.0;

        metrics.calculate_health_score();

        // Expected: (0.8 * 0.4) + (0.75 * 0.3) + (0.8 * 0.2) + (1.0 * 0.1)
        //         = 0.32 + 0.225 + 0.16 + 0.1 = 0.805
        assert!((metrics.health_score - 0.805).abs() < 0.01);
    }

    #[test]
    fn test_code_quality_calculation() {
        let mut metrics = SelfImprovementMetrics::new();
        metrics.undocumented_symbols_count = 20;
        metrics.high_complexity_symbols_count = 10;
        metrics.circular_dependencies_count = 2;

        metrics.calculate_code_quality(100);

        // Doc coverage: 80% (80/100)
        // Complexity penalty: 90% (90/100)
        // Circular penalty: 90% (1 - 2*0.05)
        // Score: (0.8 * 0.4) + (0.9 * 0.4) + (0.9 * 0.2) = 0.86
        assert!((metrics.code_quality_score - 0.86).abs() < 0.01);
    }

    #[test]
    fn test_technical_debt_calculation() {
        let mut metrics = SelfImprovementMetrics::new();
        metrics.untested_symbols_count = 30;
        metrics.undocumented_symbols_count = 20;
        metrics.high_complexity_symbols_count = 10;

        metrics.calculate_technical_debt(100);

        // Untested: 30% * 0.4 = 0.12
        // Undocumented: 20% * 0.3 = 0.06
        // High complexity: 10% * 0.3 = 0.03
        // Total: 0.21
        assert!((metrics.technical_debt_score - 0.21).abs() < 0.01);
    }

    #[test]
    fn test_health_rating() {
        let mut metrics = SelfImprovementMetrics::new();

        metrics.health_score = 0.95;
        assert_eq!(metrics.health_rating(), "Excellent");

        metrics.health_score = 0.80;
        assert_eq!(metrics.health_rating(), "Good");

        metrics.health_score = 0.65;
        assert_eq!(metrics.health_rating(), "Fair");

        metrics.health_score = 0.45;
        assert_eq!(metrics.health_rating(), "Poor");

        metrics.health_score = 0.25;
        assert_eq!(metrics.health_rating(), "Critical");
    }

    #[test]
    fn test_trend_calculation() {
        let mut current = SelfImprovementMetrics::new();
        current.health_score = 0.8;

        let mut previous = SelfImprovementMetrics::new();
        previous.health_score = 0.7;

        current.calculate_trend(Some(&previous));
        assert_eq!(current.trend_direction, TrendDirection::Improving);

        previous.health_score = 0.9;
        current.calculate_trend(Some(&previous));
        assert_eq!(current.trend_direction, TrendDirection::Degrading);

        previous.health_score = 0.81;
        current.calculate_trend(Some(&previous));
        assert_eq!(current.trend_direction, TrendDirection::Stable);
    }
}
