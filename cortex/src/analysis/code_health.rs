/// Code health analyzer
///
/// Analyzes codebase health by detecting:
/// - High complexity code requiring refactoring
/// - Circular dependencies
/// - Untested code
/// - Undocumented code
///
/// Provides actionable recommendations for improvement.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

/// Code health analyzer
pub struct CodeHealthAnalyzer {
    db: Arc<Surreal<Db>>,
}

/// Health analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAnalysisResult {
    /// Overall health score (0.0 to 1.0)
    pub health_score: f64,

    /// Issues found
    pub issues: Vec<HealthIssue>,

    /// Recommendations for improvement
    pub recommendations: Vec<Recommendation>,

    /// Summary statistics
    pub summary: HealthSummary,
}

/// Health issue detected in the codebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthIssue {
    /// Issue severity
    pub severity: IssueSeverity,

    /// Issue category
    pub category: IssueCategory,

    /// Issue title
    pub title: String,

    /// Detailed description
    pub description: String,

    /// Affected symbols/files
    pub affected: Vec<String>,

    /// Estimated impact (0.0 to 1.0)
    pub impact: f64,
}

/// Issue severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum IssueSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// Issue categories
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IssueCategory {
    Complexity,
    Testing,
    Documentation,
    Architecture,
    Performance,
    Security,
}

/// Recommendation for improving code health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// Priority (1 = highest)
    pub priority: u8,

    /// Recommendation title
    pub title: String,

    /// Action to take
    pub action: String,

    /// Expected benefit
    pub expected_benefit: String,

    /// Estimated effort (hours)
    pub estimated_effort: f64,
}

/// Health analysis summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummary {
    pub total_symbols: u64,
    pub high_complexity_count: u64,
    pub untested_count: u64,
    pub undocumented_count: u64,
    pub circular_deps_count: u64,
    pub avg_complexity: f64,
    pub test_coverage_percent: f64,
}

impl CodeHealthAnalyzer {
    /// Create a new code health analyzer
    pub fn new(db: Arc<Surreal<Db>>) -> Self {
        Self { db }
    }

    /// Perform comprehensive health analysis
    pub async fn analyze(&self) -> Result<HealthAnalysisResult> {
        let mut issues = Vec::new();

        // Collect summary statistics
        let summary = self.collect_summary().await?;

        // Find high complexity code
        let complex_issues = self.find_complex_code().await?;
        issues.extend(complex_issues);

        // Find circular dependencies
        let circular_issues = self.find_circular_dependencies().await?;
        issues.extend(circular_issues);

        // Find untested code
        let test_issues = self.find_untested_code().await?;
        issues.extend(test_issues);

        // Find undocumented code
        let doc_issues = self.find_undocumented_code().await?;
        issues.extend(doc_issues);

        // Generate recommendations based on issues
        let recommendations = self.generate_recommendations(&issues, &summary);

        // Calculate overall health score
        let health_score = self.calculate_health_score(&summary);

        Ok(HealthAnalysisResult {
            health_score,
            issues,
            recommendations,
            summary,
        })
    }

    /// Find code with high cyclomatic complexity
    pub async fn find_complex_code(&self) -> Result<Vec<HealthIssue>> {
        let results: Vec<ComplexSymbol> = self
            .db
            .query(
                "SELECT id, name, file_path, metadata.complexity AS complexity
                FROM code_symbol
                WHERE metadata.complexity > 10
                ORDER BY metadata.complexity DESC
                LIMIT 50"
            )
            .await?
            .take(0)?;

        let mut issues = Vec::new();

        for symbol in results {
            let severity = if symbol.complexity > 20.0 {
                IssueSeverity::Critical
            } else if symbol.complexity > 15.0 {
                IssueSeverity::High
            } else {
                IssueSeverity::Medium
            };

            issues.push(HealthIssue {
                severity,
                category: IssueCategory::Complexity,
                title: format!("High complexity in {}", symbol.name),
                description: format!(
                    "Symbol '{}' has cyclomatic complexity of {:.1}, which exceeds recommended threshold of 10",
                    symbol.name, symbol.complexity
                ),
                affected: vec![format!("{}:{}", symbol.file_path, symbol.name)],
                impact: (symbol.complexity / 30.0).min(1.0),
            });
        }

        Ok(issues)
    }

    /// Find circular dependencies in the codebase
    pub async fn find_circular_dependencies(&self) -> Result<Vec<HealthIssue>> {
        // This is a simplified implementation
        // Production version should use proper cycle detection algorithm
        let mut issues = Vec::new();

        // Query for potential cycles using bidirectional dependencies
        let results: Vec<CircularDep> = self
            .db
            .query(
                "SELECT in.id AS from_id, in.name AS from_name, out.id AS to_id, out.name AS to_name
                FROM depends_on
                WHERE EXISTS (
                    SELECT * FROM depends_on WHERE in = $parent.out AND out = $parent.in
                )
                LIMIT 20"
            )
            .await?
            .take(0)?;

        let mut seen = HashSet::new();

        for dep in results {
            let key = if dep.from_id < dep.to_id {
                (dep.from_id.clone(), dep.to_id.clone())
            } else {
                (dep.to_id.clone(), dep.from_id.clone())
            };

            if seen.insert(key) {
                issues.push(HealthIssue {
                    severity: IssueSeverity::High,
                    category: IssueCategory::Architecture,
                    title: "Circular dependency detected".to_string(),
                    description: format!(
                        "Circular dependency between '{}' and '{}'",
                        dep.from_name, dep.to_name
                    ),
                    affected: vec![dep.from_name.clone(), dep.to_name.clone()],
                    impact: 0.8,
                });
            }
        }

        Ok(issues)
    }

    /// Find code without test coverage
    pub async fn find_untested_code(&self) -> Result<Vec<HealthIssue>> {
        let results: Vec<UntestedSymbol> = self
            .db
            .query(
                "SELECT id, name, file_path, symbol_type
                FROM code_symbol
                WHERE (metadata.test_coverage IS NONE OR metadata.test_coverage = 0)
                  AND symbol_type IN ['function', 'class', 'method']
                ORDER BY metadata.usage_frequency DESC
                LIMIT 30"
            )
            .await?
            .take(0)?;

        let mut issues = Vec::new();

        for symbol in results {
            issues.push(HealthIssue {
                severity: IssueSeverity::Medium,
                category: IssueCategory::Testing,
                title: format!("No tests for {}", symbol.name),
                description: format!(
                    "{} '{}' has no test coverage",
                    symbol.symbol_type, symbol.name
                ),
                affected: vec![format!("{}:{}", symbol.file_path, symbol.name)],
                impact: 0.5,
            });
        }

        Ok(issues)
    }

    /// Find code without documentation
    pub async fn find_undocumented_code(&self) -> Result<Vec<HealthIssue>> {
        let results: Vec<UndocumentedSymbol> = self
            .db
            .query(
                "SELECT id, name, file_path, symbol_type
                FROM code_symbol
                WHERE count(->documents) = 0
                  AND symbol_type IN ['function', 'class', 'interface', 'type']
                ORDER BY metadata.usage_frequency DESC
                LIMIT 30"
            )
            .await?
            .take(0)?;

        let mut issues = Vec::new();

        for symbol in results {
            issues.push(HealthIssue {
                severity: IssueSeverity::Low,
                category: IssueCategory::Documentation,
                title: format!("Missing documentation for {}", symbol.name),
                description: format!(
                    "{} '{}' lacks documentation",
                    symbol.symbol_type, symbol.name
                ),
                affected: vec![format!("{}:{}", symbol.file_path, symbol.name)],
                impact: 0.3,
            });
        }

        Ok(issues)
    }

    /// Calculate overall health score
    fn calculate_health_score(&self, summary: &HealthSummary) -> f64 {
        if summary.total_symbols == 0 {
            return 1.0;
        }

        // Test coverage contribution (30%)
        let test_score = summary.test_coverage_percent / 100.0 * 0.3;

        // Complexity contribution (30%)
        let complexity_score = if summary.avg_complexity < 5.0 {
            0.3
        } else if summary.avg_complexity >= 15.0 {
            0.0
        } else {
            (1.0 - (summary.avg_complexity - 5.0) / 10.0) * 0.3
        };

        // Documentation contribution (20%)
        let doc_coverage = 1.0 - (summary.undocumented_count as f64 / summary.total_symbols as f64);
        let doc_score = doc_coverage * 0.2;

        // Architecture contribution (20%)
        let arch_score = if summary.circular_deps_count == 0 {
            0.2
        } else {
            (1.0 - (summary.circular_deps_count as f64 * 0.1).min(1.0)) * 0.2
        };

        (test_score + complexity_score + doc_score + arch_score).clamp(0.0, 1.0)
    }

    /// Generate recommendations based on issues
    fn generate_recommendations(
        &self,
        issues: &[HealthIssue],
        summary: &HealthSummary,
    ) -> Vec<Recommendation> {
        let mut recommendations = Vec::new();

        // Group issues by category
        let mut category_counts: HashMap<IssueCategory, usize> = HashMap::new();
        for issue in issues {
            *category_counts.entry(issue.category.clone()).or_insert(0) += 1;
        }

        // Recommend refactoring for high complexity
        if let Some(&count) = category_counts.get(&IssueCategory::Complexity) {
            if count > 0 {
                recommendations.push(Recommendation {
                    priority: 1,
                    title: "Refactor high complexity code".to_string(),
                    action: format!(
                        "Break down {} high-complexity functions into smaller, testable units",
                        count
                    ),
                    expected_benefit: format!(
                        "Reduce average complexity from {:.1} to <10, improve maintainability",
                        summary.avg_complexity
                    ),
                    estimated_effort: count as f64 * 2.0,
                });
            }
        }

        // Recommend adding tests
        if summary.test_coverage_percent < 80.0 {
            recommendations.push(Recommendation {
                priority: 2,
                title: "Increase test coverage".to_string(),
                action: format!(
                    "Add tests for {} untested symbols, focusing on high-usage functions",
                    summary.untested_count
                ),
                expected_benefit: format!(
                    "Increase coverage from {:.1}% to 80%+, reduce regression risk",
                    summary.test_coverage_percent
                ),
                estimated_effort: summary.untested_count as f64 * 0.5,
            });
        }

        // Recommend fixing circular dependencies
        if summary.circular_deps_count > 0 {
            recommendations.push(Recommendation {
                priority: 1,
                title: "Eliminate circular dependencies".to_string(),
                action: format!(
                    "Refactor {} circular dependencies using dependency inversion",
                    summary.circular_deps_count
                ),
                expected_benefit: "Improve code modularity and testability".to_string(),
                estimated_effort: summary.circular_deps_count as f64 * 3.0,
            });
        }

        // Recommend adding documentation
        if summary.undocumented_count > 10 {
            recommendations.push(Recommendation {
                priority: 3,
                title: "Add missing documentation".to_string(),
                action: format!(
                    "Document {} public APIs and key internal functions",
                    summary.undocumented_count
                ),
                expected_benefit: "Improve code discoverability and onboarding".to_string(),
                estimated_effort: summary.undocumented_count as f64 * 0.25,
            });
        }

        // Sort by priority
        recommendations.sort_by_key(|r| r.priority);

        recommendations
    }

    /// Collect summary statistics
    async fn collect_summary(&self) -> Result<HealthSummary> {
        // Total symbols
        let total: Option<CountResult> = self
            .db
            .query("SELECT count() AS count FROM code_symbol GROUP ALL")
            .await?
            .take(0)?;
        let total_symbols = total.map(|r| r.count).unwrap_or(0);

        // High complexity count
        let high_complex: Option<CountResult> = self
            .db
            .query("SELECT count() AS count FROM code_symbol WHERE metadata.complexity > 10 GROUP ALL")
            .await?
            .take(0)?;
        let high_complexity_count = high_complex.map(|r| r.count).unwrap_or(0);

        // Untested count
        let untested: Option<CountResult> = self
            .db
            .query(
                "SELECT count() AS count FROM code_symbol
                WHERE metadata.test_coverage IS NONE OR metadata.test_coverage = 0
                GROUP ALL"
            )
            .await?
            .take(0)?;
        let untested_count = untested.map(|r| r.count).unwrap_or(0);

        // Undocumented count
        let undoc: Option<CountResult> = self
            .db
            .query("SELECT count() AS count FROM code_symbol WHERE count(->documents) = 0 GROUP ALL")
            .await?
            .take(0)?;
        let undocumented_count = undoc.map(|r| r.count).unwrap_or(0);

        // Average complexity
        let avg_complex: Option<AvgResult> = self
            .db
            .query("SELECT math::mean(metadata.complexity) AS avg FROM code_symbol WHERE metadata.complexity IS NOT NONE GROUP ALL")
            .await?
            .take(0)?;
        let avg_complexity = avg_complex.and_then(|r| r.avg).unwrap_or(0.0);

        // Test coverage
        let test_coverage_percent = if total_symbols > 0 {
            ((total_symbols - untested_count) as f64 / total_symbols as f64) * 100.0
        } else {
            0.0
        };

        Ok(HealthSummary {
            total_symbols,
            high_complexity_count,
            untested_count,
            undocumented_count,
            circular_deps_count: 0, // Placeholder, computed separately
            avg_complexity,
            test_coverage_percent,
        })
    }
}

// Helper types for database queries

#[derive(Debug, Deserialize)]
struct ComplexSymbol {
    id: String,
    name: String,
    file_path: String,
    complexity: f64,
}

#[derive(Debug, Deserialize)]
struct CircularDep {
    from_id: String,
    from_name: String,
    to_id: String,
    to_name: String,
}

#[derive(Debug, Deserialize)]
struct UntestedSymbol {
    id: String,
    name: String,
    file_path: String,
    symbol_type: String,
}

#[derive(Debug, Deserialize)]
struct UndocumentedSymbol {
    id: String,
    name: String,
    file_path: String,
    symbol_type: String,
}

#[derive(Debug, Deserialize)]
struct CountResult {
    count: u64,
}

#[derive(Debug, Deserialize)]
struct AvgResult {
    avg: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_score_calculation() {
        // Create a mock analyzer for testing the health score calculation
        // Note: We're testing the calculation logic, not database operations
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Create a RocksDB-backed SurrealDB instance
        let rt = tokio::runtime::Runtime::new().unwrap();
        let db = rt.block_on(async {
            let db = surrealdb::Surreal::new::<surrealdb::engine::local::RocksDb>(db_path).await.unwrap();
            db.use_ns("test").use_db("test").await.unwrap();
            Arc::new(db)
        });

        let analyzer = CodeHealthAnalyzer::new(db);

        let summary = HealthSummary {
            total_symbols: 100,
            high_complexity_count: 10,
            untested_count: 20,
            undocumented_count: 15,
            circular_deps_count: 0,
            avg_complexity: 7.5,
            test_coverage_percent: 80.0,
        };

        let score = analyzer.calculate_health_score(&summary);

        // Test coverage: 0.8 * 0.3 = 0.24
        // Complexity (7.5): (1 - 2.5/10) * 0.3 = 0.225
        // Documentation: 0.85 * 0.2 = 0.17
        // Architecture: 0.2
        // Total: ~0.835
        assert!(score > 0.8 && score < 0.9);
    }

    #[test]
    fn test_issue_severity_ordering() {
        assert!(IssueSeverity::Critical > IssueSeverity::High);
        assert!(IssueSeverity::High > IssueSeverity::Medium);
        assert!(IssueSeverity::Medium > IssueSeverity::Low);
        assert!(IssueSeverity::Low > IssueSeverity::Info);
    }
}
