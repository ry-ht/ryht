//! Tester Agent Implementation

use super::*;
use crate::cortex_bridge::{
    CortexBridge, Episode, EpisodeOutcome, EpisodeType, Pattern,
    SearchFilters, TokenUsage, UnitFilters, WorkspaceId,
};
use crate::cc::{query, ClaudeCodeOptions, Message};
use crate::cc::messages::ContentBlock;
use futures::StreamExt;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, warn};

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
        existing_tests: &[crate::cortex_bridge::CodeSearchResult],
    ) -> Result<(String, usize)> {
        // Build rich context for Claude
        let mut prompt = format!(
            "Generate comprehensive {:?} tests for: {}\n\n",
            spec.test_type, spec.target_path
        );

        prompt.push_str(&format!("Target coverage: {:.1}%\n", spec.coverage_target * 100.0));
        prompt.push_str(&format!("Test type: {:?}\n\n", spec.test_type));

        // Add code units context
        if !units.is_empty() {
            prompt.push_str("Code units to test:\n");
            for unit in units.iter().take(10) {
                prompt.push_str(&format!(
                    "- {} {}: {} (complexity: {})\n",
                    unit.unit_type, unit.name,
                    unit.signature,
                    unit.complexity.cyclomatic
                ));
            }
            prompt.push('\n');
        }

        // Add test patterns context
        if !patterns.is_empty() {
            prompt.push_str("Test patterns to apply:\n");
            for (i, pattern) in patterns.iter().take(3).enumerate() {
                prompt.push_str(&format!("{}. {}: {}\n", i + 1, pattern.name, pattern.description));
            }
            prompt.push('\n');
        }

        // Add existing tests as reference
        if !existing_tests.is_empty() {
            prompt.push_str("Existing test examples (for style reference):\n");
            for (i, test) in existing_tests.iter().take(3).enumerate() {
                prompt.push_str(&format!("{}. {}\n", i + 1, test.name));
                if !test.snippet.is_empty() {
                    prompt.push_str(&format!("   {}\n", test.snippet));
                }
            }
            prompt.push('\n');
        }

        // Add episodes context
        if !episodes.is_empty() {
            prompt.push_str("Learned from past test generation:\n");
            for (i, episode) in episodes.iter().take(2).enumerate() {
                prompt.push_str(&format!("{}. {}\n", i + 1, episode.task_description));
                if !episode.lessons_learned.is_empty() {
                    prompt.push_str(&format!("   Lesson: {}\n", episode.lessons_learned[0]));
                }
            }
            prompt.push('\n');
        }

        prompt.push_str(&format!(
            "Generate complete, production-ready Rust tests. Include:\n\
            1. Proper test structure with #[test] attribute\n\
            2. Comprehensive test cases covering:\n\
               - Happy path scenarios\n\
               - Edge cases (empty inputs, boundary values)\n\
               - Error conditions\n\
            3. Clear test names describing what is being tested\n\
            4. Meaningful assertions with descriptive messages\n\
            5. Test fixtures and setup code if needed\n\
            6. Follow Rust testing best practices\n\n\
            Return ONLY the Rust test code, wrapped in a ```rust code block.\n"
        ));

        // Use Claude CLI to generate tests
        debug!("Calling Claude for test synthesis");
        let test_code = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.query_claude(&prompt).await
            })
        })?;

        // Extract code from response
        let extracted_code = self.extract_code_blocks(&test_code)?;

        if extracted_code.is_empty() {
            warn!("No test code blocks found in Claude response, using raw response");
            let test_count = self.count_tests(&test_code);
            return Ok((test_code, test_count));
        }

        let test_count = self.count_tests(&extracted_code);
        Ok((extracted_code, test_count))
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

    fn run_tests(&self, test_suite_path: &str) -> Result<(usize, usize, usize)> {
        use std::process::Command;

        debug!("Running tests for: {}", test_suite_path);

        // Detect language and test framework
        let (language, framework) = self.detect_test_framework(test_suite_path)?;
        debug!("Detected language: {}, framework: {}", language, framework);

        // Build PATH with standard directories
        let path_env = self.build_path_env();

        // Run appropriate test command based on language/framework
        let output = match language.as_str() {
            "rust" => {
                debug!("Running Rust tests with cargo test");
                Command::new("cargo")
                    .arg("test")
                    .arg("--")
                    .arg("--test-threads=1")
                    .env("PATH", &path_env)
                    .current_dir(self.get_project_root(test_suite_path)?)
                    .output()
            }
            "python" => {
                let cmd = if framework == "pytest" { "pytest" } else { "python" };
                debug!("Running Python tests with {}", cmd);
                if framework == "pytest" {
                    Command::new(cmd)
                        .arg(test_suite_path)
                        .arg("-v")
                        .env("PATH", &path_env)
                        .output()
                } else {
                    Command::new(cmd)
                        .arg("-m")
                        .arg("unittest")
                        .arg(test_suite_path)
                        .env("PATH", &path_env)
                        .output()
                }
            }
            "javascript" | "typescript" => {
                debug!("Running JS/TS tests with npm test");
                Command::new("npm")
                    .arg("test")
                    .arg("--")
                    .arg(test_suite_path)
                    .env("PATH", &path_env)
                    .current_dir(self.get_project_root(test_suite_path)?)
                    .output()
            }
            _ => {
                warn!("Unsupported language: {}, cannot run tests", language);
                return Ok((0, 0, 0));
            }
        };

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                debug!("Test output:\n{}\n{}", stdout, stderr);

                // Parse test results based on framework
                let (passed, failed, skipped) = self.parse_test_output(&language, &framework, &stdout, &stderr)?;

                info!("Test results: {} passed, {} failed, {} skipped", passed, failed, skipped);
                Ok((passed, failed, skipped))
            }
            Err(e) => {
                warn!("Failed to run tests: {}", e);
                // Return 0,0,0 if test runner not available (graceful fallback)
                Ok((0, 0, 0))
            }
        }
    }

    fn measure_coverage(&self, test_suite_path: &str) -> Result<f32> {
        use std::process::Command;

        debug!("Measuring coverage for: {}", test_suite_path);

        // Detect language
        let (language, _) = self.detect_test_framework(test_suite_path)?;
        debug!("Detected language: {}", language);

        // Build PATH with standard directories
        let path_env = self.build_path_env();

        // Run appropriate coverage tool based on language
        let output = match language.as_str() {
            "rust" => {
                debug!("Running Rust coverage with cargo-tarpaulin (or llvm-cov if available)");
                // Try cargo-llvm-cov first, fallback to tarpaulin
                let llvm_result = Command::new("cargo")
                    .arg("llvm-cov")
                    .arg("--all-features")
                    .arg("--workspace")
                    .arg("--")
                    .arg("--test-threads=1")
                    .env("PATH", &path_env)
                    .current_dir(self.get_project_root(test_suite_path)?)
                    .output();

                if llvm_result.is_ok() {
                    llvm_result
                } else {
                    debug!("cargo-llvm-cov not available, trying tarpaulin");
                    Command::new("cargo")
                        .arg("tarpaulin")
                        .arg("--out")
                        .arg("Stdout")
                        .env("PATH", &path_env)
                        .current_dir(self.get_project_root(test_suite_path)?)
                        .output()
                }
            }
            "python" => {
                debug!("Running Python coverage with coverage.py");
                Command::new("coverage")
                    .arg("run")
                    .arg("-m")
                    .arg("pytest")
                    .arg(test_suite_path)
                    .env("PATH", &path_env)
                    .output()
                    .and_then(|_| {
                        Command::new("coverage")
                            .arg("report")
                            .env("PATH", &path_env)
                            .output()
                    })
            }
            "javascript" | "typescript" => {
                debug!("Running JS/TS coverage with nyc or jest --coverage");
                // Try jest with coverage first
                let jest_result = Command::new("npm")
                    .arg("test")
                    .arg("--")
                    .arg("--coverage")
                    .arg(test_suite_path)
                    .env("PATH", &path_env)
                    .current_dir(self.get_project_root(test_suite_path)?)
                    .output();

                if jest_result.is_ok() {
                    jest_result
                } else {
                    debug!("jest not available, trying nyc");
                    Command::new("nyc")
                        .arg("npm")
                        .arg("test")
                        .env("PATH", &path_env)
                        .current_dir(self.get_project_root(test_suite_path)?)
                        .output()
                }
            }
            _ => {
                warn!("Unsupported language: {}, cannot measure coverage", language);
                return Ok(0.0);
            }
        };

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                debug!("Coverage output:\n{}\n{}", stdout, stderr);

                // Parse coverage percentage from output
                let coverage = self.parse_coverage_output(&language, &stdout, &stderr)?;

                info!("Coverage: {:.1}%", coverage * 100.0);
                Ok(coverage)
            }
            Err(e) => {
                warn!("Failed to measure coverage: {}", e);
                // Return 0.0 if coverage tool not available (graceful fallback)
                Ok(0.0)
            }
        }
    }

    // ========================================================================
    // Helper methods for test execution and coverage
    // ========================================================================

    /// Query Claude CLI and collect response text
    async fn query_claude(&self, prompt: &str) -> Result<String> {
        let options = ClaudeCodeOptions::builder()
            .system_prompt(crate::cc::options::SystemPrompt::String(
                "You are an expert software testing engineer. Generate comprehensive, \
                production-ready tests that cover edge cases, error conditions, and \
                happy paths. Always wrap code in appropriate code blocks with language tags."
                    .to_string(),
            ))
            .build();

        let mut response_stream = query(prompt, Some(options))
            .await
            .map_err(|e| AgentError::CortexError(format!("Claude query failed: {}", e)))?;

        let mut collected_text = String::new();

        while let Some(msg_result) = response_stream.next().await {
            match msg_result {
                Ok(Message::Assistant { message }) => {
                    for content_block in &message.content {
                        if let ContentBlock::Text(text_content) = content_block {
                            collected_text.push_str(&text_content.text);
                        }
                    }
                }
                Ok(Message::Result { result, is_error, .. }) => {
                    if is_error {
                        if let Some(err_msg) = result {
                            return Err(AgentError::CortexError(format!("Claude error: {}", err_msg)));
                        }
                    }
                    debug!("Claude query completed");
                }
                Ok(_) => {
                    // Ignore other message types
                }
                Err(e) => {
                    return Err(AgentError::CortexError(format!("Stream error: {}", e)));
                }
            }
        }

        if collected_text.is_empty() {
            return Err(AgentError::CortexError("No response from Claude".to_string()));
        }

        Ok(collected_text)
    }

    /// Extract code blocks from Claude's response
    fn extract_code_blocks(&self, response: &str) -> Result<String> {
        let mut code_blocks = Vec::new();
        let mut in_code_block = false;
        let mut current_block = String::new();

        for line in response.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("```rust") || trimmed.starts_with("```") {
                if in_code_block {
                    // End of code block
                    if !current_block.is_empty() {
                        code_blocks.push(current_block.clone());
                        current_block.clear();
                    }
                    in_code_block = false;
                } else {
                    // Start of code block
                    in_code_block = true;
                }
            } else if in_code_block {
                current_block.push_str(line);
                current_block.push('\n');
            }
        }

        // If we ended while still in a code block, add it
        if in_code_block && !current_block.is_empty() {
            code_blocks.push(current_block);
        }

        if code_blocks.is_empty() {
            return Ok(String::new());
        }

        // Return the concatenated code blocks
        Ok(code_blocks.join("\n\n"))
    }

    /// Count tests in generated code
    fn count_tests(&self, code: &str) -> usize {
        code.matches("#[test]").count()
            + code.matches("#[tokio::test]").count()
            + code.matches("def test_").count() // Python
            + code.matches("it(").count() // JavaScript/TypeScript
            + code.matches("test(").count() // JavaScript/TypeScript
    }

    /// Detect test framework from file path and project structure
    fn detect_test_framework(&self, test_path: &str) -> Result<(String, String)> {
        use std::path::Path;

        let path = Path::new(test_path);

        // Detect language from file extension
        let language = match path.extension().and_then(|e| e.to_str()) {
            Some("rs") => "rust",
            Some("py") => "python",
            Some("js") => "javascript",
            Some("ts") => "typescript",
            Some("jsx") | Some("tsx") => "javascript",
            _ => {
                // Try to detect from parent directories
                if test_path.contains("/tests/") || test_path.contains("/test/") {
                    // Look for Cargo.toml nearby for Rust
                    if Path::new(&test_path.replace("/tests/", "/Cargo.toml")).exists()
                        || Path::new(&test_path.replace("/test/", "/Cargo.toml")).exists() {
                        "rust"
                    } else {
                        "unknown"
                    }
                } else {
                    "unknown"
                }
            }
        };

        // Detect framework based on language and project files
        let framework = match language {
            "rust" => "cargo",
            "python" => {
                // Check for pytest.ini or pytest in parent directories
                let parent = path.parent().unwrap_or(path);
                if Path::new(&format!("{}/pytest.ini", parent.display())).exists()
                    || Path::new(&format!("{}/setup.cfg", parent.display())).exists() {
                    "pytest"
                } else {
                    "unittest"
                }
            }
            "javascript" | "typescript" => {
                // Check for jest.config.js or package.json with jest
                let parent = path.parent().unwrap_or(path);
                if Path::new(&format!("{}/jest.config.js", parent.display())).exists()
                    || Path::new(&format!("{}/jest.config.ts", parent.display())).exists() {
                    "jest"
                } else {
                    "npm"
                }
            }
            _ => "unknown",
        };

        Ok((language.to_string(), framework.to_string()))
    }

    /// Build PATH environment variable with standard directories
    fn build_path_env(&self) -> String {
        let standard_paths = vec![
            "/Users/taaliman/.cargo/bin",
            "/usr/local/bin",
            "/usr/bin",
            "/bin",
        ];

        // Get current PATH and append standard paths
        let current_path = std::env::var("PATH").unwrap_or_default();
        let mut all_paths = vec![current_path];
        all_paths.extend(standard_paths.iter().map(|s| s.to_string()));

        all_paths.join(":")
    }

    /// Get project root from test file path
    fn get_project_root(&self, test_path: &str) -> Result<String> {
        use std::path::Path;

        let path = Path::new(test_path);

        // Walk up the directory tree to find project root
        let mut current = path.parent();
        while let Some(dir) = current {
            // Check for common project root markers
            if dir.join("Cargo.toml").exists()
                || dir.join("package.json").exists()
                || dir.join("setup.py").exists()
                || dir.join("pyproject.toml").exists() {
                return Ok(dir.to_string_lossy().to_string());
            }
            current = dir.parent();
        }

        // Fallback to current directory
        Ok(".".to_string())
    }

    /// Parse test output to extract pass/fail/skip counts
    fn parse_test_output(&self, language: &str, framework: &str, stdout: &str, stderr: &str) -> Result<(usize, usize, usize)> {
        let output = format!("{}\n{}", stdout, stderr);

        match (language, framework) {
            ("rust", "cargo") => {
                // Parse Rust test output
                // Format: "test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
                let passed = output
                    .lines()
                    .find(|line| line.contains("test result:"))
                    .and_then(|line| {
                        line.split_whitespace()
                            .find(|word| word.chars().all(|c| c.is_numeric()))
                            .and_then(|n| n.parse::<usize>().ok())
                    })
                    .unwrap_or(0);

                let failed = output
                    .lines()
                    .find(|line| line.contains("failed;"))
                    .and_then(|line| {
                        line.split("failed;")
                            .next()
                            .and_then(|s| s.split_whitespace().last())
                            .and_then(|n| n.parse::<usize>().ok())
                    })
                    .unwrap_or(0);

                let skipped = output
                    .lines()
                    .find(|line| line.contains("ignored;"))
                    .and_then(|line| {
                        line.split("ignored;")
                            .next()
                            .and_then(|s| s.split_whitespace().last())
                            .and_then(|n| n.parse::<usize>().ok())
                    })
                    .unwrap_or(0);

                Ok((passed, failed, skipped))
            }
            ("python", "pytest") => {
                // Parse pytest output
                // Format: "5 passed, 1 failed, 2 skipped in 0.12s"
                let passed = output
                    .lines()
                    .find(|line| line.contains("passed"))
                    .and_then(|line| {
                        line.split("passed")
                            .next()
                            .and_then(|s| s.split_whitespace().last())
                            .and_then(|n| n.parse::<usize>().ok())
                    })
                    .unwrap_or(0);

                let failed = output
                    .lines()
                    .find(|line| line.contains("failed"))
                    .and_then(|line| {
                        line.split("failed")
                            .next()
                            .and_then(|s| s.split_whitespace().last())
                            .and_then(|n| n.parse::<usize>().ok())
                    })
                    .unwrap_or(0);

                let skipped = output
                    .lines()
                    .find(|line| line.contains("skipped"))
                    .and_then(|line| {
                        line.split("skipped")
                            .next()
                            .and_then(|s| s.split_whitespace().last())
                            .and_then(|n| n.parse::<usize>().ok())
                    })
                    .unwrap_or(0);

                Ok((passed, failed, skipped))
            }
            ("javascript" | "typescript", "jest") | ("javascript" | "typescript", "npm") => {
                // Parse Jest output
                // Format: "Tests: 1 failed, 5 passed, 6 total"
                let passed = output
                    .lines()
                    .find(|line| line.contains("Tests:") && line.contains("passed"))
                    .and_then(|line| {
                        line.split("passed")
                            .next()
                            .and_then(|s| s.split_whitespace().last())
                            .and_then(|n| n.parse::<usize>().ok())
                    })
                    .unwrap_or(0);

                let failed = output
                    .lines()
                    .find(|line| line.contains("Tests:") && line.contains("failed"))
                    .and_then(|line| {
                        line.split("failed")
                            .next()
                            .and_then(|s| s.split(',').next())
                            .and_then(|s| s.split_whitespace().last())
                            .and_then(|n| n.parse::<usize>().ok())
                    })
                    .unwrap_or(0);

                let skipped = output
                    .lines()
                    .find(|line| line.contains("Tests:") && line.contains("skipped"))
                    .and_then(|line| {
                        line.split("skipped")
                            .next()
                            .and_then(|s| s.split_whitespace().last())
                            .and_then(|n| n.parse::<usize>().ok())
                    })
                    .unwrap_or(0);

                Ok((passed, failed, skipped))
            }
            _ => Ok((0, 0, 0)),
        }
    }

    /// Parse coverage output to extract percentage
    fn parse_coverage_output(&self, language: &str, stdout: &str, stderr: &str) -> Result<f32> {
        let output = format!("{}\n{}", stdout, stderr);

        match language {
            "rust" => {
                // Parse tarpaulin or llvm-cov output
                // Format: "75.00% coverage"
                output
                    .lines()
                    .find(|line| line.contains("coverage") || line.contains("TOTAL"))
                    .and_then(|line| {
                        // Try to extract percentage
                        line.split_whitespace()
                            .find(|word| word.contains('%'))
                            .and_then(|percent| {
                                percent.trim_end_matches('%').parse::<f32>().ok()
                            })
                            .map(|p| p / 100.0)
                    })
                    .ok_or_else(|| AgentError::ValidationError("Could not parse coverage percentage".to_string()))
            }
            "python" => {
                // Parse coverage.py output
                // Format: "TOTAL 123 45 75%"
                output
                    .lines()
                    .find(|line| line.contains("TOTAL"))
                    .and_then(|line| {
                        line.split_whitespace()
                            .last()
                            .and_then(|percent| {
                                percent.trim_end_matches('%').parse::<f32>().ok()
                            })
                            .map(|p| p / 100.0)
                    })
                    .ok_or_else(|| AgentError::ValidationError("Could not parse coverage percentage".to_string()))
            }
            "javascript" | "typescript" => {
                // Parse Jest or nyc output
                // Format: "All files | 75.00 | 80.00 | 70.00 | 75.00 |"
                output
                    .lines()
                    .find(|line| line.contains("All files") || line.contains("Statements"))
                    .and_then(|line| {
                        line.split('|')
                            .nth(1)
                            .and_then(|percent| {
                                percent.trim().parse::<f32>().ok()
                            })
                            .map(|p| p / 100.0)
                    })
                    .ok_or_else(|| AgentError::ValidationError("Could not parse coverage percentage".to_string()))
            }
            _ => Ok(0.0),
        }
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
