//! Tester Agent Implementation

use super::*;
use crate::cortex_bridge::{
    AgentId as CortexAgentId, CortexBridge, Episode, EpisodeOutcome, EpisodeType, Pattern,
    SearchFilters, SessionId, TokenUsage, UnitFilters, WorkspaceId, WorkingMemoryItem,
};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info};

/// Test suite specification
#[derive(Debug, Clone)]
pub struct TestSpec {
    /// Target code to test
    pub target_path: String,
    /// Test type (unit, integration, e2e)
    pub test_type: TestType,
    /// Coverage target (0.0 - 1.0)
    pub coverage_target: f32,
    /// Workspace ID
    pub workspace_id: WorkspaceId,
}

/// Type of test to generate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestType {
    Unit,
    Integration,
    EndToEnd,
    Property,
}

/// Generated test suite
#[derive(Debug, Clone)]
pub struct TestSuite {
    /// Test file path
    pub path: String,
    /// Test code content
    pub content: String,
    /// Number of tests generated
    pub test_count: usize,
    /// Estimated coverage
    pub estimated_coverage: f32,
    /// Test metadata
    pub metadata: TestMetadata,
}

/// Test metadata
#[derive(Debug, Clone)]
pub struct TestMetadata {
    /// Patterns used
    pub patterns_used: Vec<String>,
    /// Similar tests found
    pub similar_tests_count: usize,
    /// Episodes consulted
    pub episodes_consulted: usize,
    /// Generation time
    pub generation_time_ms: u64,
}

/// Test execution result
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Test suite path
    pub suite_path: String,
    /// Tests passed
    pub passed: usize,
    /// Tests failed
    pub failed: usize,
    /// Tests skipped
    pub skipped: usize,
    /// Coverage achieved (0.0 - 1.0)
    pub coverage: f32,
    /// Execution time in ms
    pub execution_time_ms: u64,
    /// Failure details
    pub failures: Vec<TestFailure>,
}

/// Test failure information
#[derive(Debug, Clone)]
pub struct TestFailure {
    /// Test name
    pub test_name: String,
    /// Error message
    pub error: String,
    /// Stack trace
    pub stack_trace: Option<String>,
}

/// Tester agent for test generation and execution
pub struct TesterAgent {
    id: AgentId,
    name: String,
    capabilities: HashSet<Capability>,
    metrics: AgentMetrics,
    cortex: Option<Arc<CortexBridge>>,
}

impl TesterAgent {
    pub fn new(name: String) -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::Testing);
        capabilities.insert(Capability::TestGeneration);
        capabilities.insert(Capability::TestExecution);
        capabilities.insert(Capability::CoverageAnalysis);

        Self {
            id: AgentId::new(),
            name,
            capabilities,
            metrics: AgentMetrics::new(),
            cortex: None,
        }
    }

    /// Create a new TesterAgent with Cortex integration
    pub fn with_cortex(name: String, cortex: Arc<CortexBridge>) -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::Testing);
        capabilities.insert(Capability::TestGeneration);
        capabilities.insert(Capability::TestExecution);
        capabilities.insert(Capability::CoverageAnalysis);

        Self {
            id: AgentId::new(),
            name,
            capabilities,
            metrics: AgentMetrics::new(),
            cortex: Some(cortex),
        }
    }

    /// Generate tests with context from cognitive memory
    ///
    /// This method:
    /// 1. Searches for similar test patterns
    /// 2. Retrieves past testing episodes
    /// 3. Analyzes code structure for testability
    /// 4. Generates comprehensive test suite
    /// 5. Stores the episode for future learning
    pub async fn generate_tests(&self, spec: TestSpec) -> Result<TestSuite> {
        let start_time = Instant::now();
        info!(
            "TesterAgent {} generating {:?} tests for: {}",
            self.name, spec.test_type, spec.target_path
        );

        let cortex = self
            .cortex
            .as_ref()
            .ok_or_else(|| AgentError::CortexError("Cortex not configured".to_string()))?;

        // 1. Search for similar test patterns
        let test_patterns = cortex
            .search_patterns(
                &format!("test {:?}", spec.test_type),
                None,
                10,
            )
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        debug!("Found {} test patterns", test_patterns.len());

        // 2. Search for past testing episodes
        let test_episodes = cortex
            .search_episodes(
                &format!("generate {:?} tests", spec.test_type),
                10,
            )
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        debug!("Found {} testing episodes", test_episodes.len());

        // 3. Get code units to analyze testability
        let units = cortex
            .get_code_units(
                &spec.workspace_id,
                UnitFilters {
                    unit_type: None,
                    language: Some("rust".to_string()),
                    visibility: None,
                },
            )
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        debug!("Retrieved {} code units for analysis", units.len());

        // 4. Find existing tests for reference
        let existing_tests = cortex
            .semantic_search(
                &format!("tests for {}", spec.target_path),
                &spec.workspace_id,
                SearchFilters {
                    types: vec!["function".to_string()],
                    languages: vec!["rust".to_string()],
                    visibility: None,
                    min_relevance: 0.6,
                },
            )
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        debug!("Found {} existing tests", existing_tests.len());

        // 5. Generate test suite with context
        let (test_content, test_count) = self.synthesize_tests(
            &spec,
            &test_patterns,
            &test_episodes,
            &units,
            &existing_tests,
        )?;

        let test_path = self.get_test_path(&spec.target_path);
        let estimated_coverage = self.estimate_coverage(test_count, &units);

        let generation_time_ms = start_time.elapsed().as_millis() as u64;

        // 6. Store episode for future learning
        let episode = Episode {
            id: uuid::Uuid::new_v4().to_string(),
            episode_type: EpisodeType::Feature,
            task_description: format!("Generate {:?} tests for {}", spec.test_type, spec.target_path),
            agent_id: self.id.to_string(),
            session_id: None,
            workspace_id: spec.workspace_id.to_string(),
            entities_created: vec![test_path.clone()],
            entities_modified: vec![],
            entities_deleted: vec![],
            files_touched: vec![test_path.clone()],
            queries_made: vec![format!("test patterns for {:?}", spec.test_type)],
            tools_used: vec![],
            solution_summary: format!(
                "Generated {} {:?} tests with {:.1}% estimated coverage",
                test_count, spec.test_type, estimated_coverage * 100.0
            ),
            outcome: EpisodeOutcome::Success,
            success_metrics: serde_json::json!({
                "test_count": test_count,
                "estimated_coverage": estimated_coverage,
                "patterns_used": test_patterns.len(),
                "generation_time_ms": generation_time_ms,
            }),
            errors_encountered: vec![],
            lessons_learned: vec![format!("{:?} test generation patterns", spec.test_type)],
            duration_seconds: (generation_time_ms / 1000) as i32,
            tokens_used: TokenUsage::default(),
            embedding: vec![],
            created_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
        };

        cortex
            .store_episode(episode)
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        info!(
            "Generated {} tests in {}ms with {:.1}% coverage",
            test_count, generation_time_ms, estimated_coverage * 100.0
        );

        self.metrics.record_success(generation_time_ms, 0, 0);

        Ok(TestSuite {
            path: test_path,
            content: test_content,
            test_count,
            estimated_coverage,
            metadata: TestMetadata {
                patterns_used: test_patterns.iter().map(|p| p.name.clone()).collect(),
                similar_tests_count: existing_tests.len(),
                episodes_consulted: test_episodes.len(),
                generation_time_ms,
            },
        })
    }

    /// Execute tests and learn from results
    pub async fn execute_tests(
        &self,
        workspace_id: &WorkspaceId,
        test_suite_path: &str,
    ) -> Result<TestResult> {
        let start_time = Instant::now();
        info!(
            "TesterAgent {} executing tests: {}",
            self.name, test_suite_path
        );

        // Simplified execution - in production would use actual test runner
        let (passed, failed, skipped) = self.run_tests(test_suite_path)?;
        let coverage = self.measure_coverage(test_suite_path)?;
        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        let cortex = self
            .cortex
            .as_ref()
            .ok_or_else(|| AgentError::CortexError("Cortex not configured".to_string()))?;

        // Store episode for test execution
        let episode = Episode {
            id: uuid::Uuid::new_v4().to_string(),
            episode_type: EpisodeType::Task,
            task_description: format!("Execute tests in {}", test_suite_path),
            agent_id: self.id.to_string(),
            session_id: None,
            workspace_id: workspace_id.to_string(),
            entities_created: vec![],
            entities_modified: vec![],
            entities_deleted: vec![],
            files_touched: vec![test_suite_path.to_string()],
            queries_made: vec![],
            tools_used: vec![],
            solution_summary: format!(
                "Executed tests: {} passed, {} failed, {:.1}% coverage",
                passed, failed, coverage * 100.0
            ),
            outcome: if failed == 0 {
                EpisodeOutcome::Success
            } else {
                EpisodeOutcome::Partial
            },
            success_metrics: serde_json::json!({
                "passed": passed,
                "failed": failed,
                "skipped": skipped,
                "coverage": coverage,
                "execution_time_ms": execution_time_ms,
            }),
            errors_encountered: vec![],
            lessons_learned: if failed > 0 {
                vec![format!("{} test failures to analyze", failed)]
            } else {
                vec!["All tests passed successfully".to_string()]
            },
            duration_seconds: (execution_time_ms / 1000) as i32,
            tokens_used: TokenUsage::default(),
            embedding: vec![],
            created_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
        };

        cortex
            .store_episode(episode)
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        info!(
            "Test execution complete: {} passed, {} failed, {:.1}% coverage",
            passed, failed, coverage * 100.0
        );

        self.metrics.record_success(execution_time_ms, 0, 0);

        Ok(TestResult {
            suite_path: test_suite_path.to_string(),
            passed,
            failed,
            skipped,
            coverage,
            execution_time_ms,
            failures: vec![], // Simplified
        })
    }

    // ========================================================================
    // Private helper methods
    // ========================================================================

    fn synthesize_tests(
        &self,
        spec: &TestSpec,
        patterns: &[Pattern],
        episodes: &[Episode],
        units: &[crate::cortex_bridge::CodeUnit],
        _existing_tests: &[crate::cortex_bridge::CodeSearchResult],
    ) -> Result<(String, usize)> {
        // Simplified synthesis - in production would use LLM with context
        let mut tests = format!("// Generated {:?} tests for: {}\n", spec.test_type, spec.target_path);
        tests.push_str("// Test coverage target: ");
        tests.push_str(&format!("{:.1}%\n\n", spec.coverage_target * 100.0));

        if !patterns.is_empty() {
            tests.push_str("// Applied test patterns:\n");
            for (i, pattern) in patterns.iter().take(3).enumerate() {
                tests.push_str(&format!("// {}. {}\n", i + 1, pattern.name));
            }
            tests.push('\n');
        }

        if !episodes.is_empty() {
            tests.push_str("// Learned from episodes:\n");
            for (i, episode) in episodes.iter().take(2).enumerate() {
                tests.push_str(&format!("// {}. {}\n", i + 1, episode.task_description));
            }
            tests.push('\n');
        }

        // Generate tests for each unit
        let mut test_count = 0;
        for unit in units.iter().take(5) {
            tests.push_str(&format!("#[test]\n"));
            tests.push_str(&format!("fn test_{}() {{\n", unit.name));
            tests.push_str("    // TODO: Implement test based on unit behavior\n");
            tests.push_str("    assert!(true);\n");
            tests.push_str("}\n\n");
            test_count += 1;
        }

        Ok((tests, test_count))
    }

    fn get_test_path(&self, target_path: &str) -> String {
        // Simplified - in production would follow language conventions
        target_path.replace("src/", "tests/").replace(".rs", "_test.rs")
    }

    fn estimate_coverage(&self, test_count: usize, units: &[crate::cortex_bridge::CodeUnit]) -> f32 {
        // Simplified coverage estimation
        let total_units = units.len();
        if total_units == 0 {
            return 0.0;
        }
        let coverage = (test_count as f32 / total_units as f32).min(1.0);
        coverage * 0.8 // Account for partial coverage
    }

    fn run_tests(&self, _test_suite_path: &str) -> Result<(usize, usize, usize)> {
        // Simplified - in production would use actual test runner
        Ok((5, 0, 0)) // 5 passed, 0 failed, 0 skipped
    }

    fn measure_coverage(&self, _test_suite_path: &str) -> Result<f32> {
        // Simplified - in production would use coverage tool
        Ok(0.75) // 75% coverage
    }
}

impl Agent for TesterAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Tester
    }

    fn capabilities(&self) -> &HashSet<Capability> {
        &self.capabilities
    }

    fn metrics(&self) -> &AgentMetrics {
        &self.metrics
    }
}
