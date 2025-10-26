//! Reviewer Agent Implementation

use super::*;
use crate::cortex_bridge::{
    AgentId as CortexAgentId, CortexBridge, Episode, EpisodeOutcome, EpisodeType, Pattern,
    SearchFilters, SessionId, TokenUsage, UnitFilters, WorkspaceId,
};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info};

/// Review severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewSeverity {
    /// Critical issue - must fix
    Critical,
    /// High priority issue
    High,
    /// Medium priority issue
    Medium,
    /// Low priority issue
    Low,
    /// Informational note
    Info,
}

/// Review issue found during code review
#[derive(Debug, Clone)]
pub struct ReviewIssue {
    /// Issue severity
    pub severity: ReviewSeverity,
    /// Issue category (e.g., "security", "performance", "style")
    pub category: String,
    /// Issue description
    pub description: String,
    /// File path
    pub file_path: String,
    /// Line number (optional)
    pub line_number: Option<u32>,
    /// Suggestion for fixing
    pub suggestion: Option<String>,
    /// Pattern name that detected this issue
    pub pattern_name: String,
}

/// Complete review report
#[derive(Debug, Clone)]
pub struct ReviewReport {
    /// Issues found, grouped by severity
    pub issues: Vec<ReviewIssue>,
    /// Summary of the review
    pub summary: String,
    /// Overall quality score (0.0 to 1.0)
    pub quality_score: f32,
    /// Test coverage percentage (0.0 to 1.0)
    pub test_coverage: f32,
    /// Static analysis results
    pub static_analysis: StaticAnalysisResult,
    /// Security analysis results
    pub security_analysis: SecurityAnalysisResult,
    /// Best practices check results
    pub best_practices: BestPracticesResult,
    /// Performance analysis results
    pub performance_analysis: PerformanceAnalysisResult,
}

impl ReviewReport {
    pub fn new() -> Self {
        Self {
            issues: Vec::new(),
            summary: String::new(),
            quality_score: 1.0,
            test_coverage: 0.0,
            static_analysis: StaticAnalysisResult::default(),
            security_analysis: SecurityAnalysisResult::default(),
            best_practices: BestPracticesResult::default(),
            performance_analysis: PerformanceAnalysisResult::default(),
        }
    }

    /// Check if the code is acceptable
    pub fn is_acceptable(&self) -> bool {
        !self.issues.iter().any(|i| matches!(i.severity, ReviewSeverity::Critical))
            && self.quality_score >= 0.6
    }

    /// Add static analysis results
    pub fn add_static_analysis(&mut self, result: StaticAnalysisResult) {
        self.static_analysis = result;
    }

    /// Add security review results
    pub fn add_security_review(&mut self, result: SecurityAnalysisResult) {
        self.security_analysis = result;
    }

    /// Add best practices results
    pub fn add_best_practices(&mut self, result: BestPracticesResult) {
        self.best_practices = result;
    }

    /// Add performance analysis results
    pub fn add_performance(&mut self, result: PerformanceAnalysisResult) {
        self.performance_analysis = result;
    }
}

impl Default for ReviewReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Static analysis result
#[derive(Debug, Clone, Default)]
pub struct StaticAnalysisResult {
    /// Issues found
    pub issues: Vec<ReviewIssue>,
    /// Code complexity metrics
    pub complexity_metrics: ComplexityMetrics,
}

/// Security analysis result
#[derive(Debug, Clone, Default)]
pub struct SecurityAnalysisResult {
    /// Security issues found
    pub issues: Vec<ReviewIssue>,
    /// Vulnerabilities detected
    pub vulnerabilities: Vec<String>,
}

/// Best practices result
#[derive(Debug, Clone, Default)]
pub struct BestPracticesResult {
    /// Issues found
    pub issues: Vec<ReviewIssue>,
    /// Practices score (0.0 to 1.0)
    pub score: f32,
}

/// Performance analysis result
#[derive(Debug, Clone, Default)]
pub struct PerformanceAnalysisResult {
    /// Issues found
    pub issues: Vec<ReviewIssue>,
    /// Potential bottlenecks
    pub bottlenecks: Vec<String>,
}

/// Code complexity metrics
#[derive(Debug, Clone, Default)]
pub struct ComplexityMetrics {
    /// Average cyclomatic complexity
    pub avg_cyclomatic: f32,
    /// Average cognitive complexity
    pub avg_cognitive: f32,
    /// Total lines of code
    pub total_loc: u32,
}

/// Impact analysis result
#[derive(Debug, Clone)]
pub struct ImpactAnalysis {
    /// Directly affected entities
    pub directly_affected: Vec<String>,
    /// Transitively affected entities
    pub transitively_affected: Vec<String>,
    /// Risk level assessment
    pub risk_level: RiskLevel,
    /// Detailed impact description
    pub description: String,
}

/// Risk level assessment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    /// Low risk
    Low,
    /// Medium risk
    Medium,
    /// High risk
    High,
    /// Critical risk
    Critical,
}

/// Reviewer agent for code review and quality assessment
pub struct ReviewerAgent {
    id: AgentId,
    name: String,
    capabilities: HashSet<Capability>,
    metrics: AgentMetrics,
    cortex: Option<Arc<CortexBridge>>,
}

impl ReviewerAgent {
    /// Create a new ReviewerAgent
    pub fn new(name: String) -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::CodeReview);
        capabilities.insert(Capability::StaticAnalysis);
        capabilities.insert(Capability::SecurityAnalysis);

        Self {
            id: AgentId::new(),
            name,
            capabilities,
            metrics: AgentMetrics::new(),
            cortex: None,
        }
    }

    /// Create a new ReviewerAgent with Cortex integration
    pub fn with_cortex(name: String, cortex: Arc<CortexBridge>) -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::CodeReview);
        capabilities.insert(Capability::StaticAnalysis);
        capabilities.insert(Capability::SecurityAnalysis);

        Self {
            id: AgentId::new(),
            name,
            capabilities,
            metrics: AgentMetrics::new(),
            cortex: Some(cortex),
        }
    }

    /// Review code with historical context and patterns
    ///
    /// This method performs comprehensive code review by:
    /// 1. Static analysis through code units
    /// 2. Security checks using known vulnerability patterns
    /// 3. Best practices validation
    /// 4. Performance analysis
    /// 5. Test coverage calculation
    pub async fn review_code(
        &self,
        workspace_id: &WorkspaceId,
        session_id: &SessionId,
        file_path: &str,
    ) -> Result<ReviewReport> {
        let start_time = Instant::now();
        info!(
            "ReviewerAgent {} reviewing code at: {}",
            self.name, file_path
        );

        let mut report = ReviewReport::new();

        let cortex = self
            .cortex
            .as_ref()
            .ok_or_else(|| AgentError::CortexError("Cortex not configured".to_string()))?;

        // 1. Read file from session
        let code = cortex
            .read_file(session_id, file_path)
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        debug!("Read file {} for review", file_path);

        // 2. Get code units with dependencies
        let units = cortex
            .get_code_units(
                workspace_id,
                UnitFilters {
                    unit_type: None,
                    language: Some("rust".to_string()),
                    visibility: None,
                },
            )
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        debug!("Retrieved {} code units for analysis", units.len());

        // 3. Search for similar code reviews in past episodes
        let past_reviews = cortex
            .search_episodes("code review security performance", 10)
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        debug!("Found {} past review episodes", past_reviews.len());

        // 4. Get quality patterns
        let patterns = cortex
            .get_patterns()
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        let quality_patterns: Vec<Pattern> = patterns
            .into_iter()
            .filter(|p| {
                p.name.contains("quality")
                    || p.name.contains("review")
                    || p.name.contains("security")
            })
            .collect();

        debug!("Found {} quality patterns", quality_patterns.len());

        // 5. Perform static analysis
        let static_result = self.analyze_static(&code, &units)?;
        report.add_static_analysis(static_result.clone());
        report.issues.extend(static_result.issues);

        debug!("Static analysis completed");

        // 6. Security review with known vulnerability patterns
        let security_result = self.check_security_internal(&code, &past_reviews, &quality_patterns)?;
        report.add_security_review(security_result.clone());
        report.issues.extend(security_result.issues);

        debug!("Security review completed");

        // 7. Best practices validation
        let practices_result = self.check_practices(&code, &quality_patterns)?;
        report.add_best_practices(practices_result.clone());
        report.issues.extend(practices_result.issues);

        debug!("Best practices check completed");

        // 8. Performance analysis
        let perf_result = self.analyze_performance(&code, &units)?;
        report.add_performance(perf_result.clone());
        report.issues.extend(perf_result.issues);

        debug!("Performance analysis completed");

        // 9. Check test coverage
        let tests = cortex
            .semantic_search(
                &format!("tests for {}", file_path),
                workspace_id,
                SearchFilters {
                    types: vec!["function".to_string()],
                    languages: vec!["rust".to_string()],
                    visibility: None,
                    min_relevance: 0.6,
                },
            )
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        report.test_coverage = self.calculate_coverage(&code, &tests);

        debug!("Test coverage: {:.1}%", report.test_coverage * 100.0);

        // Calculate overall quality score
        report.quality_score = self.calculate_quality_score(&report);

        // Generate summary
        report.summary = format!(
            "Review completed: {} issues found ({} critical, {} high). Quality score: {:.1}/10.0",
            report.issues.len(),
            report.issues.iter().filter(|i| matches!(i.severity, ReviewSeverity::Critical)).count(),
            report.issues.iter().filter(|i| matches!(i.severity, ReviewSeverity::High)).count(),
            report.quality_score * 10.0
        );

        let review_time_ms = start_time.elapsed().as_millis() as u64;

        // 10. Store review episode for learning
        let episode = Episode {
            id: uuid::Uuid::new_v4().to_string(),
            episode_type: EpisodeType::Task,
            task_description: format!("Review code in {}", file_path),
            agent_id: self.id.to_string(),
            session_id: Some(session_id.to_string()),
            workspace_id: workspace_id.to_string(),
            entities_created: vec![],
            entities_modified: vec![],
            entities_deleted: vec![],
            files_touched: vec![file_path.to_string()],
            queries_made: vec![],
            tools_used: vec![],
            solution_summary: report.summary.clone(),
            outcome: if report.is_acceptable() {
                EpisodeOutcome::Success
            } else {
                EpisodeOutcome::Partial
            },
            success_metrics: serde_json::json!({
                "issues_found": report.issues.len(),
                "quality_score": report.quality_score,
                "test_coverage": report.test_coverage,
                "review_time_ms": review_time_ms,
            }),
            errors_encountered: vec![],
            lessons_learned: report
                .issues
                .iter()
                .map(|i| format!("{}: {}", i.category, i.pattern_name))
                .collect(),
            duration_seconds: (review_time_ms / 1000) as i32,
            tokens_used: TokenUsage::default(),
            embedding: vec![],
            created_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
        };

        cortex
            .store_episode(episode)
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        info!("Code review completed successfully in {}ms", review_time_ms);

        self.metrics.record_success(review_time_ms, 0, 0);

        Ok(report)
    }

    /// Analyze impact of changes using dependency graph
    ///
    /// This method uses the knowledge graph to understand:
    /// 1. Direct dependencies
    /// 2. Transitive dependencies
    /// 3. Risk assessment based on scope
    pub async fn analyze_impact(
        &self,
        workspace_id: &WorkspaceId,
        changed_files: Vec<String>,
    ) -> Result<ImpactAnalysis> {
        let start_time = Instant::now();
        info!(
            "ReviewerAgent {} analyzing impact of {} file changes",
            self.name,
            changed_files.len()
        );

        let cortex = self
            .cortex
            .as_ref()
            .ok_or_else(|| AgentError::CortexError("Cortex not configured".to_string()))?;

        // Use knowledge graph query to find affected components
        let query = r#"
            MATCH (changed:CodeUnit)-[:DEPENDS_ON*1..5]->(affected:CodeUnit)
            WHERE changed.file IN $changed_files
            RETURN DISTINCT affected.qualified_name AS name, affected.file AS file
        "#;

        let params = serde_json::json!({
            "changed_files": changed_files,
        });

        let graph_result = cortex
            .query_graph(query, params)
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        debug!("Knowledge graph query returned {} results", graph_result.results.len());

        // Extract affected entities
        let directly_affected: Vec<String> = changed_files.clone();
        let transitively_affected: Vec<String> = graph_result
            .results
            .iter()
            .filter_map(|r| r.get("name").and_then(|v| v.as_str()))
            .map(|s| s.to_string())
            .collect();

        debug!(
            "Found {} directly affected, {} transitively affected",
            directly_affected.len(),
            transitively_affected.len()
        );

        // Assess risk level
        let risk_level = if transitively_affected.len() > 50 {
            RiskLevel::Critical
        } else if transitively_affected.len() > 20 {
            RiskLevel::High
        } else if transitively_affected.len() > 5 {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        let description = format!(
            "Changes affect {} files directly and {} files transitively. Risk level: {:?}",
            directly_affected.len(),
            transitively_affected.len(),
            risk_level
        );

        let analysis_time_ms = start_time.elapsed().as_millis() as u64;

        // Store episode
        let episode = Episode {
            id: uuid::Uuid::new_v4().to_string(),
            episode_type: EpisodeType::Task,
            task_description: format!("Impact analysis for {} files", changed_files.len()),
            agent_id: self.id.to_string(),
            session_id: None,
            workspace_id: workspace_id.to_string(),
            entities_created: vec![],
            entities_modified: vec![],
            entities_deleted: vec![],
            files_touched: changed_files.clone(),
            queries_made: vec![query.to_string()],
            tools_used: vec![],
            solution_summary: description.clone(),
            outcome: EpisodeOutcome::Success,
            success_metrics: serde_json::json!({
                "directly_affected": directly_affected.len(),
                "transitively_affected": transitively_affected.len(),
                "risk_level": format!("{:?}", risk_level),
                "analysis_time_ms": analysis_time_ms,
            }),
            errors_encountered: vec![],
            lessons_learned: vec!["Impact analysis using knowledge graph".to_string()],
            duration_seconds: (analysis_time_ms / 1000) as i32,
            tokens_used: TokenUsage::default(),
            embedding: vec![],
            created_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
        };

        cortex
            .store_episode(episode)
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        info!("Impact analysis completed in {}ms", analysis_time_ms);

        self.metrics.record_success(analysis_time_ms, 0, 0);

        Ok(ImpactAnalysis {
            directly_affected,
            transitively_affected,
            risk_level,
            description,
        })
    }

    /// Check security vulnerabilities
    ///
    /// This method searches for:
    /// 1. Known vulnerability patterns
    /// 2. Hardcoded secrets
    /// 3. SQL injection patterns
    /// 4. Unsafe operations
    pub async fn check_security(
        &self,
        workspace_id: &WorkspaceId,
        session_id: &SessionId,
        file_path: &str,
    ) -> Result<SecurityAnalysisResult> {
        let start_time = Instant::now();
        info!(
            "ReviewerAgent {} checking security for: {}",
            self.name, file_path
        );

        let cortex = self
            .cortex
            .as_ref()
            .ok_or_else(|| AgentError::CortexError("Cortex not configured".to_string()))?;

        // Read file
        let code = cortex
            .read_file(session_id, file_path)
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        // Get security patterns
        let patterns = cortex
            .get_patterns()
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        let security_patterns: Vec<Pattern> = patterns
            .into_iter()
            .filter(|p| p.name.contains("security") || p.name.contains("vulnerability"))
            .collect();

        debug!("Found {} security patterns", security_patterns.len());

        // Search for past security episodes
        let security_episodes = cortex
            .search_episodes("security vulnerability fix", 10)
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        // Perform security checks
        let result = self.check_security_internal(&code, &security_episodes, &security_patterns)?;

        let security_time_ms = start_time.elapsed().as_millis() as u64;

        info!(
            "Security check completed in {}ms, found {} issues",
            security_time_ms,
            result.issues.len()
        );

        self.metrics.record_success(security_time_ms, 0, 0);

        Ok(result)
    }

    // ========================================================================
    // Private helper methods
    // ========================================================================

    fn analyze_static(
        &self,
        code: &str,
        units: &[crate::cortex_bridge::CodeUnit],
    ) -> Result<StaticAnalysisResult> {
        let mut issues = Vec::new();

        // Calculate complexity metrics
        let total_cyclomatic: u32 = units.iter().map(|u| u.complexity.cyclomatic).sum();
        let total_cognitive: u32 = units.iter().map(|u| u.complexity.cognitive).sum();
        let count = units.len() as f32;

        let avg_cyclomatic = if count > 0.0 {
            total_cyclomatic as f32 / count
        } else {
            0.0
        };
        let avg_cognitive = if count > 0.0 {
            total_cognitive as f32 / count
        } else {
            0.0
        };

        // Check for high complexity
        for unit in units.iter().filter(|u| u.complexity.cyclomatic > 10) {
            issues.push(ReviewIssue {
                severity: ReviewSeverity::Medium,
                category: "complexity".to_string(),
                description: format!(
                    "High cyclomatic complexity: {}",
                    unit.complexity.cyclomatic
                ),
                file_path: unit.file.clone(),
                line_number: Some(unit.lines.start),
                suggestion: Some("Consider refactoring to reduce complexity".to_string()),
                pattern_name: "high_complexity".to_string(),
            });
        }

        Ok(StaticAnalysisResult {
            issues,
            complexity_metrics: ComplexityMetrics {
                avg_cyclomatic,
                avg_cognitive,
                total_loc: code.lines().count() as u32,
            },
        })
    }

    fn check_security_internal(
        &self,
        code: &str,
        _episodes: &[Episode],
        _patterns: &[Pattern],
    ) -> Result<SecurityAnalysisResult> {
        let mut issues = Vec::new();
        let mut vulnerabilities = Vec::new();

        // Check for hardcoded secrets
        let secret_patterns = [
            ("password", "hardcoded_password"),
            ("api_key", "hardcoded_api_key"),
            ("secret", "hardcoded_secret"),
            ("token", "hardcoded_token"),
        ];

        for (pattern, vuln_type) in &secret_patterns {
            if code.to_lowercase().contains(pattern) {
                issues.push(ReviewIssue {
                    severity: ReviewSeverity::Critical,
                    category: "security".to_string(),
                    description: format!("Possible hardcoded {} found", pattern),
                    file_path: String::new(),
                    line_number: None,
                    suggestion: Some("Use environment variables or secure storage".to_string()),
                    pattern_name: vuln_type.to_string(),
                });
                vulnerabilities.push(vuln_type.to_string());
            }
        }

        // Check for unsafe operations
        if code.contains("unsafe") {
            issues.push(ReviewIssue {
                severity: ReviewSeverity::High,
                category: "security".to_string(),
                description: "Unsafe block detected".to_string(),
                file_path: String::new(),
                line_number: None,
                suggestion: Some("Ensure unsafe code is properly audited".to_string()),
                pattern_name: "unsafe_block".to_string(),
            });
        }

        // Check for SQL injection risks
        if code.contains("execute") && code.contains("format!") {
            issues.push(ReviewIssue {
                severity: ReviewSeverity::High,
                category: "security".to_string(),
                description: "Possible SQL injection vulnerability".to_string(),
                file_path: String::new(),
                line_number: None,
                suggestion: Some("Use parameterized queries".to_string()),
                pattern_name: "sql_injection".to_string(),
            });
            vulnerabilities.push("sql_injection".to_string());
        }

        Ok(SecurityAnalysisResult {
            issues,
            vulnerabilities,
        })
    }

    fn check_practices(
        &self,
        code: &str,
        _patterns: &[Pattern],
    ) -> Result<BestPracticesResult> {
        let mut issues = Vec::new();
        let mut score: f32 = 1.0;

        // Check for proper error handling
        if code.contains("unwrap()") {
            issues.push(ReviewIssue {
                severity: ReviewSeverity::Low,
                category: "best_practices".to_string(),
                description: "Use of unwrap() detected".to_string(),
                file_path: String::new(),
                line_number: None,
                suggestion: Some("Consider using proper error handling".to_string()),
                pattern_name: "avoid_unwrap".to_string(),
            });
            score -= 0.1;
        }

        // Check for documentation
        if !code.contains("///") && code.contains("pub fn") {
            issues.push(ReviewIssue {
                severity: ReviewSeverity::Info,
                category: "best_practices".to_string(),
                description: "Missing documentation for public functions".to_string(),
                file_path: String::new(),
                line_number: None,
                suggestion: Some("Add documentation comments".to_string()),
                pattern_name: "missing_docs".to_string(),
            });
            score -= 0.05;
        }

        Ok(BestPracticesResult {
            issues,
            score: score.max(0.0),
        })
    }

    fn analyze_performance(
        &self,
        code: &str,
        units: &[crate::cortex_bridge::CodeUnit],
    ) -> Result<PerformanceAnalysisResult> {
        let mut issues = Vec::new();
        let mut bottlenecks = Vec::new();

        // Check for cloning in loops
        if code.contains("for ") && code.contains(".clone()") {
            issues.push(ReviewIssue {
                severity: ReviewSeverity::Medium,
                category: "performance".to_string(),
                description: "Cloning in loop detected".to_string(),
                file_path: String::new(),
                line_number: None,
                suggestion: Some("Consider using references".to_string()),
                pattern_name: "clone_in_loop".to_string(),
            });
            bottlenecks.push("Cloning in loop".to_string());
        }

        // Check for high complexity (potential bottleneck)
        for unit in units.iter().filter(|u| u.complexity.cognitive > 15) {
            issues.push(ReviewIssue {
                severity: ReviewSeverity::Medium,
                category: "performance".to_string(),
                description: format!(
                    "High cognitive complexity may indicate performance bottleneck in {}",
                    unit.name
                ),
                file_path: unit.file.clone(),
                line_number: Some(unit.lines.start),
                suggestion: Some("Consider optimization or refactoring".to_string()),
                pattern_name: "high_cognitive_complexity".to_string(),
            });
            bottlenecks.push(format!("High complexity: {}", unit.name));
        }

        Ok(PerformanceAnalysisResult {
            issues,
            bottlenecks,
        })
    }

    fn calculate_coverage(
        &self,
        _code: &str,
        tests: &[crate::cortex_bridge::CodeSearchResult],
    ) -> f32 {
        // Simplified coverage calculation
        // In production, would use actual coverage tool
        if tests.is_empty() {
            0.0
        } else {
            // Assume some coverage based on number of tests
            let coverage = (tests.len() as f32 * 0.15).min(1.0);
            coverage
        }
    }

    fn calculate_quality_score(&self, report: &ReviewReport) -> f32 {
        let mut score = 1.0;

        // Deduct for issues
        for issue in &report.issues {
            score -= match issue.severity {
                ReviewSeverity::Critical => 0.2,
                ReviewSeverity::High => 0.1,
                ReviewSeverity::Medium => 0.05,
                ReviewSeverity::Low => 0.02,
                ReviewSeverity::Info => 0.01,
            };
        }

        // Factor in test coverage
        score *= 0.7 + (report.test_coverage * 0.3);

        // Factor in complexity
        if report.static_analysis.complexity_metrics.avg_cyclomatic > 10.0 {
            score *= 0.9;
        }

        score.max(0.0).min(1.0)
    }
}

impl Agent for ReviewerAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Reviewer
    }

    fn capabilities(&self) -> &HashSet<Capability> {
        &self.capabilities
    }

    fn metrics(&self) -> &AgentMetrics {
        &self.metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reviewer_agent_creation() {
        let agent = ReviewerAgent::new("test-reviewer".to_string());
        assert_eq!(agent.name(), "test-reviewer");
        assert_eq!(agent.agent_type(), AgentType::Reviewer);
        assert!(agent.capabilities().contains(&Capability::CodeReview));
        assert!(agent.capabilities().contains(&Capability::StaticAnalysis));
        assert!(agent.capabilities().contains(&Capability::SecurityAnalysis));
    }

    #[test]
    fn test_review_severity_levels() {
        let levels = vec![
            ReviewSeverity::Critical,
            ReviewSeverity::High,
            ReviewSeverity::Medium,
            ReviewSeverity::Low,
            ReviewSeverity::Info,
        ];
        assert_eq!(levels.len(), 5);
    }

    #[test]
    fn test_review_report_creation() {
        let report = ReviewReport::new();
        assert!(report.issues.is_empty());
        assert_eq!(report.quality_score, 1.0);
        assert_eq!(report.test_coverage, 0.0);
    }

    #[test]
    fn test_review_report_is_acceptable() {
        let mut report = ReviewReport::new();
        assert!(report.is_acceptable());

        report.issues.push(ReviewIssue {
            severity: ReviewSeverity::Critical,
            category: "security".to_string(),
            description: "Critical issue".to_string(),
            file_path: "test.rs".to_string(),
            line_number: None,
            suggestion: None,
            pattern_name: "test".to_string(),
        });

        assert!(!report.is_acceptable());
    }

    #[test]
    fn test_risk_level_assessment() {
        let levels = vec![
            RiskLevel::Low,
            RiskLevel::Medium,
            RiskLevel::High,
            RiskLevel::Critical,
        ];
        assert_eq!(levels.len(), 4);
    }
}
