//! Strategy Library - Query Patterns Mapped to Execution Strategies
//!
//! This module maintains a library of execution strategies for different types
//! of queries. Strategies are learned from successful past executions stored
//! in episodic memory and can be continuously updated based on outcomes.
//!
//! # Strategy Components
//!
//! - Pattern matching: Identify query type
//! - Resource recommendations: Worker count, tool limits
//! - Tool selection: Which tools are most effective
//! - Output formatting: Expected result structure
//! - Success criteria: How to measure success

use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::cortex_bridge::CortexBridge;
use super::{lead_agent::QueryAnalysis, Result};

// ============================================================================
// Strategy Types
// ============================================================================

/// Execution strategy for a type of query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStrategy {
    /// Strategy ID
    pub id: String,

    /// Strategy name
    pub name: String,

    /// Description
    pub description: String,

    /// Query patterns this strategy matches
    pub patterns: Vec<QueryPattern>,

    /// Recommended worker count
    pub recommended_workers: usize,

    /// Maximum parallel workers
    pub max_parallel: usize,

    /// Allowed tools for this strategy
    pub allowed_tools: Vec<String>,

    /// Expected output format
    pub output_format: OutputFormat,

    /// Success criteria
    pub success_criteria: SuccessCriteria,

    /// Times applied
    pub times_applied: u64,

    /// Success rate (0.0 - 1.0)
    pub success_rate: f32,

    /// Average improvement metrics
    pub avg_time_saved_percent: f32,

    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Query pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPattern {
    /// Pattern type
    pub pattern_type: PatternType,

    /// Keywords to match
    pub keywords: Vec<String>,

    /// Required capabilities
    pub required_capabilities: Vec<String>,

    /// Complexity indicator
    pub complexity_indicator: String,
}

/// Pattern type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PatternType {
    /// Code generation task
    CodeGeneration,

    /// Code review task
    CodeReview,

    /// Bug investigation
    BugInvestigation,

    /// Refactoring task
    Refactoring,

    /// Research task
    Research,

    /// Comparison task
    Comparison,

    /// Architecture design
    ArchitectureDesign,

    /// Testing task
    Testing,

    /// Documentation task
    Documentation,

    /// General query
    General,
}

/// Output format specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputFormat {
    /// Format type (json, markdown, code, etc.)
    pub format_type: String,

    /// Required sections
    pub required_sections: Vec<String>,

    /// Optional sections
    pub optional_sections: Vec<String>,

    /// Schema for structured output
    pub schema: Option<serde_json::Value>,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self {
            format_type: "markdown".to_string(),
            required_sections: vec![
                "summary".to_string(),
                "findings".to_string(),
                "recommendations".to_string(),
            ],
            optional_sections: vec![
                "code_examples".to_string(),
                "references".to_string(),
            ],
            schema: None,
        }
    }
}

/// Success criteria for strategy evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessCriteria {
    /// Minimum confidence score
    pub min_confidence: f32,

    /// Maximum acceptable time (seconds)
    pub max_time_seconds: u64,

    /// Maximum acceptable cost (cents)
    pub max_cost_cents: u64,

    /// Required completeness (0.0 - 1.0)
    pub required_completeness: f32,

    /// Quality metrics
    pub quality_metrics: HashMap<String, f32>,
}

impl Default for SuccessCriteria {
    fn default() -> Self {
        Self {
            min_confidence: 0.7,
            max_time_seconds: 300,
            max_cost_cents: 100,
            required_completeness: 0.8,
            quality_metrics: HashMap::new(),
        }
    }
}

// ============================================================================
// Strategy Library
// ============================================================================

/// Library of execution strategies
pub struct StrategyLibrary {
    /// Strategies indexed by ID
    strategies: Arc<RwLock<HashMap<String, ExecutionStrategy>>>,

    /// Pattern type to strategy ID mapping
    pattern_index: Arc<RwLock<HashMap<PatternType, Vec<String>>>>,

    /// Cortex bridge for loading historical strategies
    cortex: Arc<CortexBridge>,

    /// Configuration
    config: StrategyLibraryConfig,
}

/// Strategy library configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyLibraryConfig {
    /// Enable automatic strategy learning
    pub auto_learning: bool,

    /// Minimum applications before trusting strategy
    pub min_applications: u64,

    /// Minimum success rate to keep strategy
    pub min_success_rate: f32,

    /// Enable strategy evolution
    pub enable_evolution: bool,
}

impl Default for StrategyLibraryConfig {
    fn default() -> Self {
        Self {
            auto_learning: true,
            min_applications: 5,
            min_success_rate: 0.6,
            enable_evolution: true,
        }
    }
}

impl StrategyLibrary {
    /// Create a new strategy library
    pub async fn new(cortex: Arc<CortexBridge>, config: StrategyLibraryConfig) -> Result<Self> {
        info!("Initializing Strategy Library");

        let library = Self {
            strategies: Arc::new(RwLock::new(HashMap::new())),
            pattern_index: Arc::new(RwLock::new(HashMap::new())),
            cortex,
            config,
        };

        // Load default strategies
        library.load_default_strategies().await?;

        // Load learned strategies from Cortex
        library.load_learned_strategies().await?;

        info!("Strategy Library initialized with {} strategies",
              library.strategies.read().await.len());

        Ok(library)
    }

    /// Load default built-in strategies
    async fn load_default_strategies(&self) -> Result<()> {
        debug!("Loading default strategies");

        let strategies = vec![
            self.create_code_generation_strategy(),
            self.create_code_review_strategy(),
            self.create_bug_investigation_strategy(),
            self.create_refactoring_strategy(),
            self.create_research_strategy(),
            self.create_comparison_strategy(),
            self.create_testing_strategy(),
        ];

        let mut strategy_map = self.strategies.write().await;
        let mut pattern_idx = self.pattern_index.write().await;

        for strategy in strategies {
            // Index by pattern types
            for pattern in &strategy.patterns {
                pattern_idx
                    .entry(pattern.pattern_type)
                    .or_insert_with(Vec::new)
                    .push(strategy.id.clone());
            }

            strategy_map.insert(strategy.id.clone(), strategy);
        }

        Ok(())
    }

    /// Create code generation strategy
    fn create_code_generation_strategy(&self) -> ExecutionStrategy {
        ExecutionStrategy {
            id: "strategy_code_generation".to_string(),
            name: "Code Generation".to_string(),
            description: "Generate code from specifications with review and testing".to_string(),
            patterns: vec![QueryPattern {
                pattern_type: PatternType::CodeGeneration,
                keywords: vec![
                    "generate".to_string(),
                    "create".to_string(),
                    "implement".to_string(),
                    "write code".to_string(),
                ],
                required_capabilities: vec!["CodeGeneration".to_string()],
                complexity_indicator: "medium".to_string(),
            }],
            recommended_workers: 3,
            max_parallel: 3,
            allowed_tools: vec![
                "code_writer".to_string(),
                "code_analyzer".to_string(),
                "test_generator".to_string(),
            ],
            output_format: OutputFormat {
                format_type: "code".to_string(),
                required_sections: vec![
                    "implementation".to_string(),
                    "tests".to_string(),
                    "documentation".to_string(),
                ],
                optional_sections: vec!["examples".to_string()],
                schema: None,
            },
            success_criteria: SuccessCriteria {
                min_confidence: 0.8,
                max_time_seconds: 180,
                max_cost_cents: 75,
                required_completeness: 0.9,
                quality_metrics: HashMap::new(),
            },
            times_applied: 0,
            success_rate: 0.0,
            avg_time_saved_percent: 0.0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    /// Create code review strategy
    fn create_code_review_strategy(&self) -> ExecutionStrategy {
        ExecutionStrategy {
            id: "strategy_code_review".to_string(),
            name: "Code Review".to_string(),
            description: "Comprehensive code review with multiple specialized reviewers".to_string(),
            patterns: vec![QueryPattern {
                pattern_type: PatternType::CodeReview,
                keywords: vec![
                    "review".to_string(),
                    "check".to_string(),
                    "validate".to_string(),
                    "audit".to_string(),
                ],
                required_capabilities: vec!["CodeReview".to_string()],
                complexity_indicator: "medium".to_string(),
            }],
            recommended_workers: 4,
            max_parallel: 4,
            allowed_tools: vec![
                "code_reader".to_string(),
                "static_analyzer".to_string(),
                "security_scanner".to_string(),
                "complexity_analyzer".to_string(),
            ],
            output_format: OutputFormat {
                format_type: "markdown".to_string(),
                required_sections: vec![
                    "summary".to_string(),
                    "issues_found".to_string(),
                    "recommendations".to_string(),
                    "security_concerns".to_string(),
                ],
                optional_sections: vec!["code_quality_metrics".to_string()],
                schema: None,
            },
            success_criteria: SuccessCriteria::default(),
            times_applied: 0,
            success_rate: 0.0,
            avg_time_saved_percent: 0.0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    /// Create bug investigation strategy
    fn create_bug_investigation_strategy(&self) -> ExecutionStrategy {
        ExecutionStrategy {
            id: "strategy_bug_investigation".to_string(),
            name: "Bug Investigation".to_string(),
            description: "Parallel investigation of bug causes with root cause analysis".to_string(),
            patterns: vec![QueryPattern {
                pattern_type: PatternType::BugInvestigation,
                keywords: vec![
                    "bug".to_string(),
                    "error".to_string(),
                    "issue".to_string(),
                    "debug".to_string(),
                    "fix".to_string(),
                ],
                required_capabilities: vec![
                    "CodeAnalysis".to_string(),
                    "DebuggingAssistance".to_string(),
                ],
                complexity_indicator: "high".to_string(),
            }],
            recommended_workers: 5,
            max_parallel: 5,
            allowed_tools: vec![
                "code_reader".to_string(),
                "log_analyzer".to_string(),
                "trace_analyzer".to_string(),
                "dependency_tracker".to_string(),
            ],
            output_format: OutputFormat {
                format_type: "markdown".to_string(),
                required_sections: vec![
                    "root_cause".to_string(),
                    "affected_areas".to_string(),
                    "proposed_fix".to_string(),
                ],
                optional_sections: vec!["test_cases".to_string()],
                schema: None,
            },
            success_criteria: SuccessCriteria {
                min_confidence: 0.75,
                max_time_seconds: 300,
                max_cost_cents: 150,
                required_completeness: 0.85,
                quality_metrics: HashMap::new(),
            },
            times_applied: 0,
            success_rate: 0.0,
            avg_time_saved_percent: 0.0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    /// Create refactoring strategy
    fn create_refactoring_strategy(&self) -> ExecutionStrategy {
        ExecutionStrategy {
            id: "strategy_refactoring".to_string(),
            name: "Code Refactoring".to_string(),
            description: "Systematic code refactoring with safety checks".to_string(),
            patterns: vec![QueryPattern {
                pattern_type: PatternType::Refactoring,
                keywords: vec![
                    "refactor".to_string(),
                    "improve".to_string(),
                    "restructure".to_string(),
                    "optimize".to_string(),
                ],
                required_capabilities: vec![
                    "CodeRefactoring".to_string(),
                    "Testing".to_string(),
                ],
                complexity_indicator: "medium".to_string(),
            }],
            recommended_workers: 3,
            max_parallel: 2,
            allowed_tools: vec![
                "code_reader".to_string(),
                "code_writer".to_string(),
                "test_runner".to_string(),
                "dependency_analyzer".to_string(),
            ],
            output_format: OutputFormat {
                format_type: "code".to_string(),
                required_sections: vec![
                    "refactored_code".to_string(),
                    "changes_summary".to_string(),
                    "test_results".to_string(),
                ],
                optional_sections: vec!["migration_guide".to_string()],
                schema: None,
            },
            success_criteria: SuccessCriteria::default(),
            times_applied: 0,
            success_rate: 0.0,
            avg_time_saved_percent: 0.0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    /// Create research strategy
    fn create_research_strategy(&self) -> ExecutionStrategy {
        ExecutionStrategy {
            id: "strategy_research".to_string(),
            name: "Information Research".to_string(),
            description: "Parallel research with multiple specialized agents".to_string(),
            patterns: vec![QueryPattern {
                pattern_type: PatternType::Research,
                keywords: vec![
                    "research".to_string(),
                    "investigate".to_string(),
                    "find".to_string(),
                    "explore".to_string(),
                ],
                required_capabilities: vec!["InformationRetrieval".to_string()],
                complexity_indicator: "high".to_string(),
            }],
            recommended_workers: 10,
            max_parallel: 10,
            allowed_tools: vec![
                "search".to_string(),
                "semantic_search".to_string(),
                "documentation_reader".to_string(),
                "web_search".to_string(),
            ],
            output_format: OutputFormat {
                format_type: "markdown".to_string(),
                required_sections: vec![
                    "summary".to_string(),
                    "key_findings".to_string(),
                    "sources".to_string(),
                ],
                optional_sections: vec![
                    "related_topics".to_string(),
                    "recommendations".to_string(),
                ],
                schema: None,
            },
            success_criteria: SuccessCriteria {
                min_confidence: 0.7,
                max_time_seconds: 300,
                max_cost_cents: 200,
                required_completeness: 0.8,
                quality_metrics: HashMap::new(),
            },
            times_applied: 0,
            success_rate: 0.0,
            avg_time_saved_percent: 0.0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    /// Create comparison strategy
    fn create_comparison_strategy(&self) -> ExecutionStrategy {
        ExecutionStrategy {
            id: "strategy_comparison".to_string(),
            name: "Comparative Analysis".to_string(),
            description: "Parallel comparison of options with synthesis".to_string(),
            patterns: vec![QueryPattern {
                pattern_type: PatternType::Comparison,
                keywords: vec![
                    "compare".to_string(),
                    "versus".to_string(),
                    "vs".to_string(),
                    "difference".to_string(),
                ],
                required_capabilities: vec![
                    "InformationRetrieval".to_string(),
                    "CodeAnalysis".to_string(),
                ],
                complexity_indicator: "medium".to_string(),
            }],
            recommended_workers: 4,
            max_parallel: 4,
            allowed_tools: vec![
                "code_reader".to_string(),
                "semantic_search".to_string(),
                "documentation_reader".to_string(),
            ],
            output_format: OutputFormat {
                format_type: "markdown".to_string(),
                required_sections: vec![
                    "comparison_summary".to_string(),
                    "option_a_analysis".to_string(),
                    "option_b_analysis".to_string(),
                    "recommendation".to_string(),
                ],
                optional_sections: vec!["use_cases".to_string()],
                schema: None,
            },
            success_criteria: SuccessCriteria::default(),
            times_applied: 0,
            success_rate: 0.0,
            avg_time_saved_percent: 0.0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    /// Create testing strategy
    fn create_testing_strategy(&self) -> ExecutionStrategy {
        ExecutionStrategy {
            id: "strategy_testing".to_string(),
            name: "Test Generation and Execution".to_string(),
            description: "Comprehensive testing with multiple test types".to_string(),
            patterns: vec![QueryPattern {
                pattern_type: PatternType::Testing,
                keywords: vec![
                    "test".to_string(),
                    "testing".to_string(),
                    "verify".to_string(),
                    "validate".to_string(),
                ],
                required_capabilities: vec!["Testing".to_string(), "TestGeneration".to_string()],
                complexity_indicator: "medium".to_string(),
            }],
            recommended_workers: 4,
            max_parallel: 4,
            allowed_tools: vec![
                "test_generator".to_string(),
                "test_runner".to_string(),
                "coverage_analyzer".to_string(),
            ],
            output_format: OutputFormat {
                format_type: "code".to_string(),
                required_sections: vec![
                    "test_suite".to_string(),
                    "coverage_report".to_string(),
                    "test_results".to_string(),
                ],
                optional_sections: vec!["performance_tests".to_string()],
                schema: None,
            },
            success_criteria: SuccessCriteria::default(),
            times_applied: 0,
            success_rate: 0.0,
            avg_time_saved_percent: 0.0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    /// Load learned strategies from Cortex episodic memory
    async fn load_learned_strategies(&self) -> Result<()> {
        if !self.config.auto_learning {
            return Ok(());
        }

        debug!("Loading learned strategies from Cortex");

        // Query Cortex for successful workflow executions
        let episodes = match self.cortex.search_episodes("workflow execution task", 100).await {
            Ok(eps) => eps,
            Err(e) => {
                debug!("Failed to query Cortex episodes, skipping learned strategies: {}", e);
                return Ok(());
            }
        };

        info!("Retrieved {} episodes from Cortex for pattern extraction", episodes.len());

        // Filter for successful episodes only
        let successful_episodes: Vec<_> = episodes
            .into_iter()
            .filter(|ep| matches!(ep.outcome, crate::cortex_bridge::models::EpisodeOutcome::Success))
            .collect();

        if successful_episodes.is_empty() {
            debug!("No successful episodes found, skipping learned strategy extraction");
            return Ok(());
        }

        info!("Found {} successful episodes for analysis", successful_episodes.len());

        // Extract patterns from successful episodes
        let learned_strategies = self.extract_strategies_from_episodes(&successful_episodes);

        if learned_strategies.is_empty() {
            debug!("No patterns extracted from episodes");
            return Ok(());
        }

        info!("Extracted {} learned strategies from episodes", learned_strategies.len());

        // Add learned strategies to the library (limited to prevent memory bloat)
        let max_learned_strategies = 50;
        let strategies_to_add = learned_strategies
            .into_iter()
            .take(max_learned_strategies);

        let mut strategy_map = self.strategies.write().await;
        let mut pattern_idx = self.pattern_index.write().await;

        for strategy in strategies_to_add {
            // Index by pattern types
            for pattern in &strategy.patterns {
                pattern_idx
                    .entry(pattern.pattern_type)
                    .or_insert_with(Vec::new)
                    .push(strategy.id.clone());
            }

            info!("Added learned strategy: {}", strategy.name);
            strategy_map.insert(strategy.id.clone(), strategy);
        }

        Ok(())
    }

    /// Extract execution strategies from successful episodes
    fn extract_strategies_from_episodes(&self, episodes: &[crate::cortex_bridge::models::Episode]) -> Vec<ExecutionStrategy> {
        use std::collections::HashMap;

        // Group episodes by task description patterns
        let mut pattern_groups: HashMap<String, Vec<&crate::cortex_bridge::models::Episode>> = HashMap::new();

        for episode in episodes {
            // Extract key terms from task description for pattern matching
            let key_terms = self.extract_key_terms(&episode.task_description);
            let pattern_key = key_terms.join("_");

            pattern_groups
                .entry(pattern_key)
                .or_insert_with(Vec::new)
                .push(episode);
        }

        let mut strategies = Vec::new();
        let total_groups = pattern_groups.len();

        // Create strategies for patterns that appear multiple times (indicating a learnable pattern)
        for (pattern_key, group_episodes) in pattern_groups {
            // Only create strategy if pattern appears at least 3 times
            if group_episodes.len() < 3 {
                continue;
            }

            // Analyze the group to extract common characteristics
            let avg_duration = group_episodes.iter()
                .map(|e| e.duration_seconds as f32)
                .sum::<f32>() / group_episodes.len() as f32;

            let avg_workers = self.infer_worker_count(&group_episodes);
            let common_tools = self.extract_common_tools(&group_episodes);
            let pattern_type = self.infer_pattern_type(&group_episodes);
            let keywords = self.extract_key_terms(&group_episodes[0].task_description);

            // Calculate success metrics
            let success_rate = group_episodes.len() as f32 / (group_episodes.len() as f32 + 0.1); // All are successful
            let avg_improvement = self.calculate_avg_improvement(&group_episodes);

            // Create learned strategy
            let strategy_id = format!("learned_strategy_{}", pattern_key.to_lowercase());
            let strategy_name = format!("Learned: {}", self.humanize_pattern_key(&pattern_key));

            let strategy = ExecutionStrategy {
                id: strategy_id,
                name: strategy_name,
                description: format!(
                    "Learned strategy from {} successful executions with {:.1}s avg duration",
                    group_episodes.len(),
                    avg_duration
                ),
                patterns: vec![QueryPattern {
                    pattern_type,
                    keywords: keywords.clone(),
                    required_capabilities: self.infer_capabilities(&group_episodes),
                    complexity_indicator: if avg_duration < 30.0 {
                        "low".to_string()
                    } else if avg_duration < 120.0 {
                        "medium".to_string()
                    } else {
                        "high".to_string()
                    },
                }],
                recommended_workers: avg_workers,
                max_parallel: avg_workers.max(2),
                allowed_tools: common_tools,
                output_format: OutputFormat::default(),
                success_criteria: SuccessCriteria {
                    min_confidence: 0.7,
                    max_time_seconds: (avg_duration * 1.5) as u64,
                    max_cost_cents: 100,
                    required_completeness: 0.8,
                    quality_metrics: HashMap::new(),
                },
                times_applied: group_episodes.len() as u64,
                success_rate,
                avg_time_saved_percent: avg_improvement,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };

            strategies.push(strategy);
        }

        debug!("Extracted {} strategies from {} episode groups", strategies.len(), total_groups);
        strategies
    }

    /// Extract key terms from task description
    fn extract_key_terms(&self, description: &str) -> Vec<String> {
        let description_lower = description.to_lowercase();
        let mut terms = Vec::new();

        // Common task patterns
        let patterns = [
            "implement", "refactor", "optimize", "debug", "review", "test",
            "generate", "analyze", "fix", "create", "update", "investigate",
            "parallel", "sequential", "distributed", "search", "query",
        ];

        for pattern in &patterns {
            if description_lower.contains(pattern) {
                terms.push(pattern.to_string());
            }
        }

        // If no patterns found, use first 2 words
        if terms.is_empty() {
            terms = description
                .split_whitespace()
                .take(2)
                .map(|s| s.to_lowercase())
                .collect();
        }

        terms.truncate(3); // Limit to 3 key terms
        terms
    }

    /// Infer pattern type from episodes
    fn infer_pattern_type(&self, episodes: &[&crate::cortex_bridge::models::Episode]) -> PatternType {
        // Analyze episode types and task descriptions
        for episode in episodes.iter().take(5) {
            let desc_lower = episode.task_description.to_lowercase();

            if desc_lower.contains("code") && desc_lower.contains("review") {
                return PatternType::CodeReview;
            } else if desc_lower.contains("bug") || desc_lower.contains("debug") || desc_lower.contains("fix") {
                return PatternType::BugInvestigation;
            } else if desc_lower.contains("refactor") {
                return PatternType::Refactoring;
            } else if desc_lower.contains("test") {
                return PatternType::Testing;
            } else if desc_lower.contains("generate") || desc_lower.contains("implement") || desc_lower.contains("create") {
                return PatternType::CodeGeneration;
            } else if desc_lower.contains("research") || desc_lower.contains("investigate") || desc_lower.contains("explore") {
                return PatternType::Research;
            } else if desc_lower.contains("compare") || desc_lower.contains("versus") {
                return PatternType::Comparison;
            } else if desc_lower.contains("architect") || desc_lower.contains("design") {
                return PatternType::ArchitectureDesign;
            } else if desc_lower.contains("document") {
                return PatternType::Documentation;
            }
        }

        PatternType::General
    }

    /// Infer worker count from episodes
    fn infer_worker_count(&self, episodes: &[&crate::cortex_bridge::models::Episode]) -> usize {
        // Analyze task complexity and parallelization patterns
        let avg_files = episodes.iter()
            .map(|e| e.files_touched.len())
            .sum::<usize>() as f32 / episodes.len() as f32;

        let avg_queries = episodes.iter()
            .map(|e| e.queries_made.len())
            .sum::<usize>() as f32 / episodes.len() as f32;

        // More files and queries suggest higher parallelization
        if avg_files > 10.0 || avg_queries > 5.0 {
            10
        } else if avg_files > 5.0 || avg_queries > 3.0 {
            5
        } else if avg_files > 2.0 || avg_queries > 1.0 {
            3
        } else {
            2
        }
    }

    /// Extract common tools used across episodes
    fn extract_common_tools(&self, episodes: &[&crate::cortex_bridge::models::Episode]) -> Vec<String> {
        use std::collections::HashMap;

        let mut tool_counts: HashMap<String, usize> = HashMap::new();

        for episode in episodes {
            for tool in &episode.tools_used {
                *tool_counts.entry(tool.tool_name.clone()).or_insert(0) += 1;
            }
        }

        // Return tools that appear in at least 50% of episodes
        let threshold = episodes.len() / 2;
        tool_counts
            .into_iter()
            .filter(|(_, count)| *count >= threshold)
            .map(|(tool, _)| tool)
            .collect()
    }

    /// Infer required capabilities from episodes
    fn infer_capabilities(&self, episodes: &[&crate::cortex_bridge::models::Episode]) -> Vec<String> {
        let mut capabilities = Vec::new();

        for episode in episodes.iter().take(5) {
            let desc_lower = episode.task_description.to_lowercase();

            if desc_lower.contains("code") || desc_lower.contains("implement") {
                if !capabilities.contains(&"CodeGeneration".to_string()) {
                    capabilities.push("CodeGeneration".to_string());
                }
            }
            if desc_lower.contains("review") || desc_lower.contains("analyze") {
                if !capabilities.contains(&"CodeReview".to_string()) {
                    capabilities.push("CodeReview".to_string());
                }
            }
            if desc_lower.contains("test") {
                if !capabilities.contains(&"Testing".to_string()) {
                    capabilities.push("Testing".to_string());
                }
            }
            if desc_lower.contains("refactor") {
                if !capabilities.contains(&"CodeRefactoring".to_string()) {
                    capabilities.push("CodeRefactoring".to_string());
                }
            }
            if desc_lower.contains("search") || desc_lower.contains("find") {
                if !capabilities.contains(&"InformationRetrieval".to_string()) {
                    capabilities.push("InformationRetrieval".to_string());
                }
            }
        }

        if capabilities.is_empty() {
            capabilities.push("General".to_string());
        }

        capabilities
    }

    /// Calculate average improvement from episodes
    fn calculate_avg_improvement(&self, episodes: &[&crate::cortex_bridge::models::Episode]) -> f32 {
        let mut total_improvement = 0.0;
        let mut count = 0;

        for episode in episodes {
            if let Some(improvement) = episode.success_metrics.get("improvement_percent") {
                if let Some(val) = improvement.as_f64() {
                    total_improvement += val as f32;
                    count += 1;
                }
            }
        }

        if count > 0 {
            total_improvement / count as f32
        } else {
            0.0
        }
    }

    /// Humanize pattern key for display
    fn humanize_pattern_key(&self, key: &str) -> String {
        key.replace('_', " ")
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Find the best strategy for a query analysis
    pub async fn find_best_strategy(&self, analysis: &QueryAnalysis) -> Result<ExecutionStrategy> {
        debug!("Finding best strategy for query: {}", analysis.query);

        let strategies = self.strategies.read().await;
        let mut best_match: Option<(&String, &ExecutionStrategy, f32)> = None;

        for (id, strategy) in strategies.iter() {
            let score = self.score_strategy_match(strategy, analysis);

            if score > 0.0 {
                if let Some((_, _, best_score)) = best_match {
                    if score > best_score {
                        best_match = Some((id, strategy, score));
                    }
                } else {
                    best_match = Some((id, strategy, score));
                }
            }
        }

        if let Some((_, strategy, score)) = best_match {
            debug!("Selected strategy '{}' with score {:.2}", strategy.name, score);
            Ok(strategy.clone())
        } else {
            // Return general fallback strategy
            Ok(self.create_general_strategy())
        }
    }

    /// Score how well a strategy matches the query analysis
    fn score_strategy_match(&self, strategy: &ExecutionStrategy, analysis: &QueryAnalysis) -> f32 {
        let mut score = 0.0;
        let query_lower = analysis.query.to_lowercase();

        // Check pattern matching
        for pattern in &strategy.patterns {
            for keyword in &pattern.keywords {
                if query_lower.contains(&keyword.to_lowercase()) {
                    score += 1.0;
                }
            }

            // Check capability overlap
            for capability in &analysis.required_capabilities {
                if pattern.required_capabilities.contains(capability) {
                    score += 2.0;
                }
            }
        }

        // Boost score based on strategy success rate
        if strategy.times_applied >= self.config.min_applications {
            score *= 1.0 + strategy.success_rate;
        }

        score
    }

    /// Create general fallback strategy
    fn create_general_strategy(&self) -> ExecutionStrategy {
        ExecutionStrategy {
            id: "strategy_general".to_string(),
            name: "General Task".to_string(),
            description: "General purpose task execution".to_string(),
            patterns: vec![QueryPattern {
                pattern_type: PatternType::General,
                keywords: vec![],
                required_capabilities: vec![],
                complexity_indicator: "medium".to_string(),
            }],
            recommended_workers: 2,
            max_parallel: 2,
            allowed_tools: vec![],
            output_format: OutputFormat::default(),
            success_criteria: SuccessCriteria::default(),
            times_applied: 0,
            success_rate: 0.0,
            avg_time_saved_percent: 0.0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    /// Update strategy statistics after execution
    pub async fn update_strategy_stats(
        &self,
        strategy_id: &str,
        success: bool,
        time_saved_percent: f32,
    ) -> Result<()> {
        let mut strategies = self.strategies.write().await;

        if let Some(strategy) = strategies.get_mut(strategy_id) {
            strategy.times_applied += 1;

            // Update success rate with running average
            let new_success = if success { 1.0 } else { 0.0 };
            strategy.success_rate = (strategy.success_rate * (strategy.times_applied - 1) as f32 + new_success)
                / strategy.times_applied as f32;

            // Update time saved average
            strategy.avg_time_saved_percent = (strategy.avg_time_saved_percent * (strategy.times_applied - 1) as f32
                + time_saved_percent) / strategy.times_applied as f32;

            strategy.updated_at = chrono::Utc::now();

            debug!(
                "Updated strategy '{}': applied={}, success_rate={:.2}, time_saved={:.1}%",
                strategy.name,
                strategy.times_applied,
                strategy.success_rate,
                strategy.avg_time_saved_percent
            );
        }

        Ok(())
    }

    /// Get all strategies
    pub async fn get_all_strategies(&self) -> Vec<ExecutionStrategy> {
        self.strategies.read().await.values().cloned().collect()
    }

    /// Get strategy by ID
    pub async fn get_strategy(&self, id: &str) -> Option<ExecutionStrategy> {
        self.strategies.read().await.get(id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_type() {
        let pattern = PatternType::CodeGeneration;
        assert_eq!(pattern, PatternType::CodeGeneration);
    }

    #[test]
    fn test_output_format_default() {
        let format = OutputFormat::default();
        assert_eq!(format.format_type, "markdown");
        assert!(format.required_sections.contains(&"summary".to_string()));
    }
}
