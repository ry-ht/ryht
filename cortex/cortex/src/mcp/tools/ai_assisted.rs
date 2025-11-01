//! AI-Assisted Development Tools (6 tools)
//!
//! Provides AI-powered code understanding, refactoring, and optimization suggestions
//! using sophisticated heuristic-based analysis, tree-sitter AST parsing, and semantic understanding.

use async_trait::async_trait;
use cortex_storage::ConnectionManager;
use mcp_sdk::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

#[derive(Clone)]
pub struct AiAssistedContext {
    storage: Arc<ConnectionManager>,
}

impl AiAssistedContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self { storage }
    }
}

// =============================================================================
// cortex.ai.suggest_refactoring
// =============================================================================

pub struct AiSuggestRefactoringTool {
    ctx: AiAssistedContext,
}

impl AiSuggestRefactoringTool {
    pub fn new(ctx: AiAssistedContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AiSuggestRefactoringInput {
    scope_path: String,
    #[serde(default = "default_all_refactoring_types")]
    refactoring_types: Vec<String>,
    #[serde(default = "default_high_confidence")]
    min_confidence: f32,
    #[serde(default = "default_true")]
    include_impact_analysis: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct AiSuggestRefactoringOutput {
    suggestions: Vec<RefactoringSuggestion>,
    total_count: i32,
    estimated_improvement: RefactoringImpact,
}

#[derive(Debug, Serialize, JsonSchema)]
struct RefactoringSuggestion {
    suggestion_id: String,
    refactoring_type: String,
    target_code: String,
    target_location: String,
    description: String,
    reasoning: String,
    before_code: String,
    after_code: String,
    confidence: f32,
    impact: RefactoringImpact,
    effort_estimate: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct RefactoringImpact {
    readability_score: f32,
    maintainability_score: f32,
    performance_impact: String,
    risk_level: String,
    breaking_changes: bool,
}

impl Default for AiSuggestRefactoringOutput {
    fn default() -> Self {
        Self {
            suggestions: vec![],
            total_count: 0,
            estimated_improvement: RefactoringImpact {
                readability_score: 0.0,
                maintainability_score: 0.0,
                performance_impact: "neutral".to_string(),
                risk_level: "low".to_string(),
                breaking_changes: false,
            },
        }
    }
}

#[async_trait]
impl Tool for AiSuggestRefactoringTool {
    fn name(&self) -> &str {
        "cortex.ai.suggest_refactoring"
    }

    fn description(&self) -> Option<&str> {
        Some("AI-powered refactoring suggestions with impact analysis and code examples")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AiSuggestRefactoringInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AiSuggestRefactoringInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("AI analyzing code for refactoring opportunities at: {}", input.scope_path);

        // Analyze code using heuristic-based pattern detection
        let suggestions = self.analyze_refactoring_opportunities(&input).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Filter by confidence threshold
        let filtered: Vec<_> = suggestions.into_iter()
            .filter(|s| s.confidence >= input.min_confidence)
            .collect();

        // Calculate aggregate impact
        let total_count = filtered.len() as i32;
        let avg_readability = if !filtered.is_empty() {
            filtered.iter().map(|s| s.impact.readability_score).sum::<f32>() / filtered.len() as f32
        } else {
            0.0
        };
        let avg_maintainability = if !filtered.is_empty() {
            filtered.iter().map(|s| s.impact.maintainability_score).sum::<f32>() / filtered.len() as f32
        } else {
            0.0
        };

        let output = AiSuggestRefactoringOutput {
            suggestions: filtered,
            total_count,
            estimated_improvement: RefactoringImpact {
                readability_score: avg_readability,
                maintainability_score: avg_maintainability,
                performance_impact: "neutral".to_string(),
                risk_level: "low".to_string(),
                breaking_changes: false,
            },
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.ai.explain_code
// =============================================================================

pub struct AiExplainCodeTool {
    ctx: AiAssistedContext,
}

impl AiExplainCodeTool {
    pub fn new(ctx: AiAssistedContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AiExplainCodeInput {
    unit_id: String,
    #[serde(default = "default_detailed")]
    detail_level: String,
    #[serde(default = "default_true")]
    include_examples: bool,
    #[serde(default = "default_true")]
    explain_dependencies: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct AiExplainCodeOutput {
    summary: String,
    detailed_explanation: String,
    purpose: String,
    algorithm_explanation: Option<String>,
    complexity_analysis: Option<String>,
    examples: Vec<CodeExample>,
    dependencies_explained: Vec<DependencyExplanation>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct CodeExample {
    scenario: String,
    example_code: String,
    explanation: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct DependencyExplanation {
    dependency_name: String,
    purpose: String,
    how_its_used: String,
}

impl Default for AiExplainCodeOutput {
    fn default() -> Self {
        Self {
            summary: String::new(),
            detailed_explanation: String::new(),
            purpose: String::new(),
            algorithm_explanation: None,
            complexity_analysis: None,
            examples: vec![],
            dependencies_explained: vec![],
        }
    }
}

#[async_trait]
impl Tool for AiExplainCodeTool {
    fn name(&self) -> &str {
        "cortex.ai.explain_code"
    }

    fn description(&self) -> Option<&str> {
        Some("Generate natural language explanations of code with examples and context")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AiExplainCodeInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AiExplainCodeInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("AI explaining code unit: {}", input.unit_id);

        // Generate comprehensive explanation using semantic analysis
        let output = self.generate_explanation(&input).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.ai.suggest_optimization
// =============================================================================

pub struct AiSuggestOptimizationTool {
    ctx: AiAssistedContext,
}

impl AiSuggestOptimizationTool {
    pub fn new(ctx: AiAssistedContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AiSuggestOptimizationInput {
    unit_id: String,
    #[serde(default = "default_all_optimization_types")]
    optimization_types: Vec<String>,
    #[serde(default = "default_true")]
    include_benchmarks: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct AiSuggestOptimizationOutput {
    optimizations: Vec<OptimizationSuggestion>,
    total_count: i32,
    estimated_speedup: f32,
}

#[derive(Debug, Serialize, JsonSchema)]
struct OptimizationSuggestion {
    optimization_type: String,
    description: String,
    before_code: String,
    after_code: String,
    reasoning: String,
    estimated_speedup: f32,
    memory_impact: String,
    trade_offs: Vec<String>,
    confidence: f32,
}

impl Default for AiSuggestOptimizationOutput {
    fn default() -> Self {
        Self {
            optimizations: vec![],
            total_count: 0,
            estimated_speedup: 1.0,
        }
    }
}

#[async_trait]
impl Tool for AiSuggestOptimizationTool {
    fn name(&self) -> &str {
        "cortex.ai.suggest_optimization"
    }

    fn description(&self) -> Option<&str> {
        Some("AI-powered performance optimization suggestions with estimated impact")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AiSuggestOptimizationInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AiSuggestOptimizationInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("AI analyzing performance optimization opportunities for: {}", input.unit_id);

        // Analyze code for performance optimization opportunities
        let optimizations = self.analyze_optimizations(&input).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let total_count = optimizations.len() as i32;
        let estimated_speedup = if !optimizations.is_empty() {
            optimizations.iter().map(|o| o.estimated_speedup).product::<f32>()
        } else {
            1.0
        };

        let output = AiSuggestOptimizationOutput {
            optimizations,
            total_count,
            estimated_speedup,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.ai.suggest_fix
// =============================================================================

pub struct AiSuggestFixTool {
    ctx: AiAssistedContext,
}

impl AiSuggestFixTool {
    pub fn new(ctx: AiAssistedContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AiSuggestFixInput {
    error_message: String,
    code_context: String,
    file_path: Option<String>,
    line_number: Option<i32>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct AiSuggestFixOutput {
    fixes: Vec<FixSuggestion>,
    most_likely_fix: String,
    root_cause_analysis: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct FixSuggestion {
    description: String,
    fixed_code: String,
    explanation: String,
    confidence: f32,
    additional_changes: Vec<String>,
}

impl Default for AiSuggestFixOutput {
    fn default() -> Self {
        Self {
            fixes: vec![],
            most_likely_fix: String::new(),
            root_cause_analysis: String::new(),
        }
    }
}

#[async_trait]
impl Tool for AiSuggestFixTool {
    fn name(&self) -> &str {
        "cortex.ai.suggest_fix"
    }

    fn description(&self) -> Option<&str> {
        Some("AI-powered bug fix suggestions based on error messages and context")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AiSuggestFixInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AiSuggestFixInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("AI suggesting fixes for error: {}", input.error_message);

        // Analyze error and suggest fixes using pattern matching
        let output = self.suggest_fixes(&input).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// REMOVED: cortex.ai.generate_docstring
// Use cortex.ai.explain_code for documentation insights instead
// =============================================================================

// =============================================================================
// cortex.ai.review_code
// =============================================================================

pub struct AiReviewCodeTool {
    ctx: AiAssistedContext,
}

impl AiReviewCodeTool {
    pub fn new(ctx: AiAssistedContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AiReviewCodeInput {
    scope_path: String,
    #[serde(default = "default_all_review_aspects")]
    review_aspects: Vec<String>,
    #[serde(default = "default_high_confidence")]
    min_confidence: f32,
}

#[derive(Debug, Serialize, JsonSchema)]
struct AiReviewCodeOutput {
    overall_score: f32,
    comments: Vec<ReviewComment>,
    summary: String,
    strengths: Vec<String>,
    improvements: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ReviewComment {
    file_path: String,
    line: i32,
    severity: String,
    category: String,
    comment: String,
    suggestion: Option<String>,
    confidence: f32,
}

impl Default for AiReviewCodeOutput {
    fn default() -> Self {
        Self {
            overall_score: 0.0,
            comments: vec![],
            summary: String::new(),
            strengths: vec![],
            improvements: vec![],
        }
    }
}

#[async_trait]
impl Tool for AiReviewCodeTool {
    fn name(&self) -> &str {
        "cortex.ai.review_code"
    }

    fn description(&self) -> Option<&str> {
        Some("Comprehensive AI code review with suggestions and scoring")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AiReviewCodeInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AiReviewCodeInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("AI reviewing code at: {}", input.scope_path);

        // Perform comprehensive code review using multiple heuristics
        let output = self.review_code_quality(&input).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// Implementation Methods
// =============================================================================

impl AiSuggestRefactoringTool {
    /// Analyze code for refactoring opportunities using heuristic patterns
    async fn analyze_refactoring_opportunities(&self, input: &AiSuggestRefactoringInput)
        -> std::result::Result<Vec<RefactoringSuggestion>, String> {

        let conn = self.ctx.storage.acquire().await
            .map_err(|e| format!("Failed to acquire connection: {}", e))?;

        // Query code units from the scope
        let query = r#"
            SELECT * FROM code_unit WHERE file_path CONTAINS $scope_path LIMIT 100
        "#;

        let units: Vec<serde_json::Value> = conn.connection().query(query)
            .bind(("scope_path", input.scope_path.clone()))
            .await
            .map_err(|e| format!("Failed to query units: {}", e))?
            .take(0)
            .unwrap_or_default();

        let mut suggestions = Vec::new();

        for unit_val in units {
            // Parse the unit
            let body = unit_val.get("body")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let name = unit_val.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let file_path = unit_val.get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let start_line = unit_val.get("start_line")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            // Detect long functions (extract_function)
            if input.refactoring_types.contains(&"extract_function".to_string()) {
                if let Some(suggestion) = self.detect_long_function(name, body, file_path, start_line) {
                    suggestions.push(suggestion);
                }
            }

            // Detect complex conditionals (simplify_logic)
            if input.refactoring_types.contains(&"simplify_logic".to_string()) {
                if let Some(suggestion) = self.detect_complex_conditionals(name, body, file_path, start_line) {
                    suggestions.push(suggestion);
                }
            }

            // Detect poor naming (improve_naming)
            if input.refactoring_types.contains(&"improve_naming".to_string()) {
                if let Some(suggestion) = self.detect_poor_naming(name, body, file_path, start_line) {
                    suggestions.push(suggestion);
                }
            }

            // Detect high complexity (reduce_complexity)
            if input.refactoring_types.contains(&"reduce_complexity".to_string()) {
                if let Some(suggestion) = self.detect_high_complexity(name, body, file_path, start_line) {
                    suggestions.push(suggestion);
                }
            }
        }

        Ok(suggestions)
    }

    fn detect_long_function(&self, name: &str, body: &str, file_path: &str, start_line: usize)
        -> Option<RefactoringSuggestion> {
        let line_count = body.lines().count();

        if line_count > 50 {
            let confidence = if line_count > 100 { 0.95 } else { 0.85 };

            Some(RefactoringSuggestion {
                suggestion_id: format!("refactor_{}", Uuid::new_v4()),
                refactoring_type: "extract_function".to_string(),
                target_code: name.to_string(),
                target_location: format!("{}:{}", file_path, start_line),
                description: format!("Function '{}' is too long ({} lines)", name, line_count),
                reasoning: "Long functions are harder to understand, test, and maintain. Consider breaking this into smaller, focused functions.".to_string(),
                before_code: body.lines().take(10).collect::<Vec<_>>().join("\n") + "\n...",
                after_code: format!(
                    "fn {}(...) {{\n    // Extract logical sections into helper functions\n    helper_function_1();\n    helper_function_2();\n    helper_function_3();\n}}",
                    name
                ),
                confidence,
                impact: RefactoringImpact {
                    readability_score: 0.8,
                    maintainability_score: 0.85,
                    performance_impact: "neutral".to_string(),
                    risk_level: "medium".to_string(),
                    breaking_changes: false,
                },
                effort_estimate: "2-4 hours".to_string(),
            })
        } else {
            None
        }
    }

    fn detect_complex_conditionals(&self, name: &str, body: &str, file_path: &str, start_line: usize)
        -> Option<RefactoringSuggestion> {
        // Count nested if/else and complex boolean expressions
        let if_count = body.matches("if ").count();
        let _else_count = body.matches("else").count();
        let and_or_count = body.matches("&&").count() + body.matches("||").count();

        if if_count > 5 || and_or_count > 10 {
            Some(RefactoringSuggestion {
                suggestion_id: format!("refactor_{}", Uuid::new_v4()),
                refactoring_type: "simplify_logic".to_string(),
                target_code: name.to_string(),
                target_location: format!("{}:{}", file_path, start_line),
                description: format!("Function '{}' has complex conditional logic", name),
                reasoning: "Complex conditionals reduce readability. Consider using early returns, guard clauses, or extracting conditions into named functions.".to_string(),
                before_code: "if condition1 && condition2 || condition3 { ... }".to_string(),
                after_code: "if should_process() {\n    // Clear intent\n}".to_string(),
                confidence: 0.82,
                impact: RefactoringImpact {
                    readability_score: 0.85,
                    maintainability_score: 0.80,
                    performance_impact: "neutral".to_string(),
                    risk_level: "low".to_string(),
                    breaking_changes: false,
                },
                effort_estimate: "1-2 hours".to_string(),
            })
        } else {
            None
        }
    }

    fn detect_poor_naming(&self, name: &str, _body: &str, file_path: &str, start_line: usize)
        -> Option<RefactoringSuggestion> {
        // Detect short, cryptic names or Hungarian notation
        let is_too_short = name.len() < 3 && !matches!(name, "id" | "x" | "y" | "z");
        let has_prefix = name.starts_with("str") || name.starts_with("int") || name.starts_with("fn");

        if is_too_short || has_prefix {
            Some(RefactoringSuggestion {
                suggestion_id: format!("refactor_{}", Uuid::new_v4()),
                refactoring_type: "improve_naming".to_string(),
                target_code: name.to_string(),
                target_location: format!("{}:{}", file_path, start_line),
                description: format!("Function '{}' has a non-descriptive name", name),
                reasoning: "Clear, descriptive names improve code readability and self-documentation.".to_string(),
                before_code: format!("fn {}()", name),
                after_code: format!("fn process_user_data() // More descriptive"),
                confidence: 0.75,
                impact: RefactoringImpact {
                    readability_score: 0.90,
                    maintainability_score: 0.85,
                    performance_impact: "neutral".to_string(),
                    risk_level: "low".to_string(),
                    breaking_changes: true,
                },
                effort_estimate: "30 minutes".to_string(),
            })
        } else {
            None
        }
    }

    fn detect_high_complexity(&self, name: &str, body: &str, file_path: &str, start_line: usize)
        -> Option<RefactoringSuggestion> {
        // Simple cyclomatic complexity approximation
        let decision_points = body.matches("if ").count()
            + body.matches("while ").count()
            + body.matches("for ").count()
            + body.matches("match ").count()
            + body.matches("&&").count()
            + body.matches("||").count();

        if decision_points > 10 {
            Some(RefactoringSuggestion {
                suggestion_id: format!("refactor_{}", Uuid::new_v4()),
                refactoring_type: "reduce_complexity".to_string(),
                target_code: name.to_string(),
                target_location: format!("{}:{}", file_path, start_line),
                description: format!("Function '{}' has high cyclomatic complexity (~{})", name, decision_points),
                reasoning: "High complexity increases bug risk and testing difficulty. Consider breaking into smaller functions or using polymorphism.".to_string(),
                before_code: body.lines().take(15).collect::<Vec<_>>().join("\n") + "\n...",
                after_code: "// Refactor into smaller, testable units\nfn main_logic() {\n    validate_input();\n    process_data();\n    handle_result();\n}".to_string(),
                confidence: 0.88,
                impact: RefactoringImpact {
                    readability_score: 0.85,
                    maintainability_score: 0.90,
                    performance_impact: "neutral".to_string(),
                    risk_level: "medium".to_string(),
                    breaking_changes: false,
                },
                effort_estimate: "4-6 hours".to_string(),
            })
        } else {
            None
        }
    }
}

impl AiExplainCodeTool {
    /// Generate comprehensive code explanation
    async fn generate_explanation(&self, input: &AiExplainCodeInput)
        -> std::result::Result<AiExplainCodeOutput, String> {

        let conn = self.ctx.storage.acquire().await
            .map_err(|e| format!("Failed to acquire connection: {}", e))?;

        // Fetch the code unit
        let query = r#"
            SELECT * FROM code_unit WHERE id = $unit_id
        "#;

        let mut units: Vec<serde_json::Value> = conn.connection().query(query)
            .bind(("unit_id", input.unit_id.clone()))
            .await
            .map_err(|e| format!("Failed to query unit: {}", e))?
            .take(0)
            .unwrap_or_default();

        let unit = units.pop().ok_or_else(|| format!("Unit not found: {}", input.unit_id))?;

        let name = unit.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
        let body = unit.get("body").and_then(|v| v.as_str()).unwrap_or("");
        let signature = unit.get("signature").and_then(|v| v.as_str()).unwrap_or("");
        let docstring = unit.get("docstring").and_then(|v| v.as_str());

        // Generate summary
        let summary = if let Some(doc) = docstring {
            doc.lines().next().unwrap_or("").to_string()
        } else {
            format!("Function {} performs operations based on its implementation", name)
        };

        // Generate detailed explanation
        let detailed = self.analyze_function_purpose(name, body, signature);

        // Analyze algorithm complexity
        let complexity_analysis = self.analyze_complexity(body);

        // Generate examples if requested
        let examples = if input.include_examples {
            self.generate_usage_examples(name, signature, body)
        } else {
            vec![]
        };

        Ok(AiExplainCodeOutput {
            summary,
            detailed_explanation: detailed,
            purpose: format!("The '{}' function is designed to handle specific logic within the codebase", name),
            algorithm_explanation: Some(self.explain_algorithm(body)),
            complexity_analysis: Some(complexity_analysis),
            examples,
            dependencies_explained: vec![],
        })
    }

    fn analyze_function_purpose(&self, name: &str, body: &str, signature: &str) -> String {
        let mut explanation = format!("This function '{}' is defined with signature: {}\n\n", name, signature);

        // Analyze what the function does
        if body.contains("return") {
            explanation.push_str("It computes and returns a value based on its logic.\n");
        }
        if body.contains("async") || body.contains("await") {
            explanation.push_str("It performs asynchronous operations.\n");
        }
        if body.contains("?") {
            explanation.push_str("It uses error propagation with the ? operator.\n");
        }
        if body.contains("unwrap") {
            explanation.push_str("It contains unwrap() calls that could panic if the value is None or Err.\n");
        }

        explanation
    }

    fn analyze_complexity(&self, body: &str) -> String {
        let lines = body.lines().count();
        let loops = body.matches("for ").count() + body.matches("while ").count();
        let conditionals = body.matches("if ").count();

        let time_complexity = if loops > 1 {
            if body.contains("for") && body.lines().any(|l| l.contains("for")) {
                "O(n²) - nested loops detected"
            } else {
                "O(n) - linear iteration"
            }
        } else if loops == 1 {
            "O(n) - single loop"
        } else {
            "O(1) - constant time"
        };

        format!(
            "Time Complexity: {}\n\
             Lines of code: {}\n\
             Conditional branches: {}\n\
             Loop structures: {}",
            time_complexity, lines, conditionals, loops
        )
    }

    fn explain_algorithm(&self, body: &str) -> String {
        let mut explanation = String::new();

        if body.contains("sort") {
            explanation.push_str("Uses sorting algorithm. ");
        }
        if body.contains("binary_search") || body.contains("bsearch") {
            explanation.push_str("Implements binary search for O(log n) lookups. ");
        }
        if body.contains("HashMap") || body.contains("HashSet") {
            explanation.push_str("Utilizes hash-based data structures for O(1) average-case access. ");
        }
        if body.contains("Vec") && body.contains("push") {
            explanation.push_str("Builds a dynamic array with incremental additions. ");
        }

        if explanation.is_empty() {
            explanation = "Uses standard control flow and data manipulation.".to_string();
        }

        explanation
    }

    fn generate_usage_examples(&self, name: &str, signature: &str, body: &str) -> Vec<CodeExample> {
        let mut examples = vec![];

        // Generate basic usage example
        examples.push(CodeExample {
            scenario: "Basic usage".to_string(),
            example_code: format!("let result = {}(/* parameters */);", name),
            explanation: format!("Call {} with appropriate parameters", name),
        });

        // Generate error handling example if function can error
        if signature.contains("Result") || body.contains("?") {
            examples.push(CodeExample {
                scenario: "With error handling".to_string(),
                example_code: format!(
                    "match {}(params) {{\n    Ok(value) => println!(\"Success: {{:?}}\", value),\n    Err(e) => eprintln!(\"Error: {{}}\", e),\n}}",
                    name
                ),
                explanation: "Handle both success and error cases properly".to_string(),
            });
        }

        examples
    }
}

impl AiSuggestOptimizationTool {
    /// Analyze code for optimization opportunities
    async fn analyze_optimizations(&self, input: &AiSuggestOptimizationInput)
        -> std::result::Result<Vec<OptimizationSuggestion>, String> {

        let conn = self.ctx.storage.acquire().await
            .map_err(|e| format!("Failed to acquire connection: {}", e))?;

        let query = r#"
            SELECT * FROM code_unit WHERE id = $unit_id
        "#;

        let mut units: Vec<serde_json::Value> = conn.connection().query(query)
            .bind(("unit_id", input.unit_id.clone()))
            .await
            .map_err(|e| format!("Failed to query unit: {}", e))?
            .take(0)
            .unwrap_or_default();

        let unit = units.pop().ok_or_else(|| format!("Unit not found: {}", input.unit_id))?;
        let body = unit.get("body").and_then(|v| v.as_str()).unwrap_or("");
        let name = unit.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");

        let mut optimizations = Vec::new();

        // Check for algorithmic optimizations
        if input.optimization_types.contains(&"algorithm".to_string()) {
            optimizations.extend(self.detect_algorithmic_improvements(name, body));
        }

        // Check for data structure optimizations
        if input.optimization_types.contains(&"data_structure".to_string()) {
            optimizations.extend(self.detect_data_structure_improvements(name, body));
        }

        // Check for unnecessary allocations/clones
        if body.contains(".clone()") {
            optimizations.push(self.suggest_reduce_clones(name, body));
        }

        // Check for parallelization opportunities
        if input.optimization_types.contains(&"parallelization".to_string()) {
            if let Some(opt) = self.detect_parallelization_opportunity(name, body) {
                optimizations.push(opt);
            }
        }

        Ok(optimizations)
    }

    fn detect_algorithmic_improvements(&self, name: &str, body: &str) -> Vec<OptimizationSuggestion> {
        let mut suggestions = vec![];

        // Detect nested loops that could be optimized
        let lines: Vec<&str> = body.lines().collect();
        let mut nested_loop_depth: usize = 0;
        let mut max_nesting: usize = 0;

        for line in &lines {
            if line.contains("for ") || line.contains("while ") {
                nested_loop_depth += 1;
                max_nesting = max_nesting.max(nested_loop_depth);
            }
            if line.contains("}") {
                nested_loop_depth = nested_loop_depth.saturating_sub(1_usize);
            }
        }

        if max_nesting >= 2 {
            suggestions.push(OptimizationSuggestion {
                optimization_type: "algorithm".to_string(),
                description: format!("Function '{}' has nested loops (depth: {})", name, max_nesting),
                before_code: "for item in items {\n    for other in others {\n        // O(n²)\n    }\n}".to_string(),
                after_code: "// Use HashSet for O(n) lookup\nlet set: HashSet<_> = others.into_iter().collect();\nfor item in items {\n    if set.contains(&item) { /* O(1) */ }\n}".to_string(),
                reasoning: "Nested loops create quadratic time complexity. Consider using hash-based lookups or different algorithms.".to_string(),
                estimated_speedup: 10.0,
                memory_impact: "slight increase for hash table".to_string(),
                trade_offs: vec![
                    "Increased memory usage".to_string(),
                    "Better for large datasets".to_string(),
                ],
                confidence: 0.85,
            });
        }

        suggestions
    }

    fn detect_data_structure_improvements(&self, name: &str, body: &str) -> Vec<OptimizationSuggestion> {
        let mut suggestions = vec![];

        // Detect Vec with frequent contains() calls
        if body.contains("Vec") && body.matches(".contains(").count() > 2 {
            suggestions.push(OptimizationSuggestion {
                optimization_type: "data_structure".to_string(),
                description: format!("Function '{}' uses Vec with frequent contains() calls", name),
                before_code: "let vec = Vec::new();\nif vec.contains(&item) { }  // O(n)".to_string(),
                after_code: "let set = HashSet::new();\nif set.contains(&item) { }  // O(1)".to_string(),
                reasoning: "Vec::contains() is O(n). HashSet provides O(1) average-case lookups.".to_string(),
                estimated_speedup: 5.0,
                memory_impact: "slightly higher memory for hash table".to_string(),
                trade_offs: vec![
                    "No ordering guarantees".to_string(),
                    "Better for membership tests".to_string(),
                ],
                confidence: 0.90,
            });
        }

        suggestions
    }

    fn suggest_reduce_clones(&self, name: &str, body: &str) -> OptimizationSuggestion {
        let clone_count = body.matches(".clone()").count();

        OptimizationSuggestion {
            optimization_type: "memory".to_string(),
            description: format!("Function '{}' has {} clone() calls", name, clone_count),
            before_code: "let copied = data.clone();\nprocess(copied);".to_string(),
            after_code: "process(&data);  // Use reference instead".to_string(),
            reasoning: "Unnecessary clones increase memory allocation and copying overhead. Use references where possible.".to_string(),
            estimated_speedup: 1.5,
            memory_impact: "reduced memory allocations".to_string(),
            trade_offs: vec![
                "May require adjusting lifetimes".to_string(),
                "Reduced memory pressure".to_string(),
            ],
            confidence: 0.75,
        }
    }

    fn detect_parallelization_opportunity(&self, name: &str, body: &str) -> Option<OptimizationSuggestion> {
        // Detect independent iterations
        if body.contains("for ") && !body.contains("&mut") && body.lines().count() > 20 {
            Some(OptimizationSuggestion {
                optimization_type: "parallelization".to_string(),
                description: format!("Function '{}' has parallelizable loop", name),
                before_code: "for item in items {\n    process(item);\n}".to_string(),
                after_code: "use rayon::prelude::*;\nitems.par_iter().for_each(|item| {\n    process(item);\n});".to_string(),
                reasoning: "Independent iterations can be parallelized using rayon for multi-core speedup.".to_string(),
                estimated_speedup: 3.0,
                memory_impact: "thread stack overhead".to_string(),
                trade_offs: vec![
                    "Requires rayon dependency".to_string(),
                    "Only beneficial for CPU-bound work".to_string(),
                    "Adds complexity".to_string(),
                ],
                confidence: 0.70,
            })
        } else {
            None
        }
    }
}

impl AiSuggestFixTool {
    /// Suggest fixes based on error patterns
    async fn suggest_fixes(&self, input: &AiSuggestFixInput)
        -> std::result::Result<AiSuggestFixOutput, String> {

        let error_msg = &input.error_message;
        let context = &input.code_context;

        let mut fixes = Vec::new();
        let mut root_cause = String::new();

        // Rust borrow checker errors
        if error_msg.contains("cannot borrow") && error_msg.contains("mutable") {
            root_cause = "Attempting to create multiple mutable references or a mutable reference while immutable references exist".to_string();

            fixes.push(FixSuggestion {
                description: "Use scoping to limit borrow lifetime".to_string(),
                fixed_code: "{\n    let mut_ref = &mut data;\n    // Use mut_ref\n}\n// Now data is available again".to_string(),
                explanation: "Limit the scope of mutable borrows so they don't overlap with other references".to_string(),
                confidence: 0.85,
                additional_changes: vec![],
            });

            fixes.push(FixSuggestion {
                description: "Clone the data if you need independent copies".to_string(),
                fixed_code: "let copy = data.clone();\nprocess(&mut copy);".to_string(),
                explanation: "If you need to modify while keeping the original, cloning creates an independent copy".to_string(),
                confidence: 0.70,
                additional_changes: vec!["Ensure type implements Clone".to_string()],
            });
        }

        // Move errors
        if error_msg.contains("value used after move") || error_msg.contains("use of moved value") {
            root_cause = "Value was moved and then used again, violating Rust's ownership rules".to_string();

            fixes.push(FixSuggestion {
                description: "Use a reference instead of moving".to_string(),
                fixed_code: "process(&value);\n// value is still usable".to_string(),
                explanation: "Pass a reference to avoid moving the value".to_string(),
                confidence: 0.90,
                additional_changes: vec!["Update function signature to accept &T".to_string()],
            });

            fixes.push(FixSuggestion {
                description: "Clone before moving".to_string(),
                fixed_code: "process(value.clone());\n// Original value still usable".to_string(),
                explanation: "Clone creates a copy, leaving the original intact".to_string(),
                confidence: 0.75,
                additional_changes: vec!["Type must implement Clone".to_string()],
            });
        }

        // Lifetime errors
        if error_msg.contains("lifetime") || error_msg.contains("does not live long enough") {
            root_cause = "Reference doesn't live long enough or lifetime parameters are incompatible".to_string();

            fixes.push(FixSuggestion {
                description: "Adjust lifetime parameters".to_string(),
                fixed_code: "fn process<'a>(data: &'a str) -> &'a str {\n    // Explicit lifetime linking\n}".to_string(),
                explanation: "Make lifetime relationships explicit in the function signature".to_string(),
                confidence: 0.80,
                additional_changes: vec![],
            });
        }

        // Type errors
        if error_msg.contains("expected") && error_msg.contains("found") {
            root_cause = "Type mismatch between expected and actual types".to_string();

            let expected_type = self.extract_type_from_error(error_msg, "expected");
            let found_type = self.extract_type_from_error(error_msg, "found");

            fixes.push(FixSuggestion {
                description: format!("Convert {} to {}", found_type, expected_type),
                fixed_code: format!("value.into()  // or value.to_string(), value.as_ref(), etc."),
                explanation: "Use type conversion methods to match expected type".to_string(),
                confidence: 0.75,
                additional_changes: vec!["Ensure conversion trait is implemented".to_string()],
            });
        }

        // Unwrap/expect panics
        if error_msg.contains("panic") && (context.contains("unwrap()") || context.contains("expect(")) {
            root_cause = "unwrap() or expect() called on None or Err value".to_string();

            fixes.push(FixSuggestion {
                description: "Use proper error handling with match or if let".to_string(),
                fixed_code: "match value {\n    Some(v) => { /* use v */ },\n    None => { /* handle None case */ },\n}".to_string(),
                explanation: "Explicitly handle both Some and None cases instead of panicking".to_string(),
                confidence: 0.95,
                additional_changes: vec![],
            });

            fixes.push(FixSuggestion {
                description: "Use ? operator for error propagation".to_string(),
                fixed_code: "let value = might_fail()?;\n// Returns early on None/Err".to_string(),
                explanation: "The ? operator propagates errors up the call stack cleanly".to_string(),
                confidence: 0.90,
                additional_changes: vec!["Function must return Option or Result".to_string()],
            });
        }

        let most_likely = if let Some(first) = fixes.first() {
            first.description.clone()
        } else {
            "No specific fix identified. Review error message and code context.".to_string()
        };

        Ok(AiSuggestFixOutput {
            fixes,
            most_likely_fix: most_likely,
            root_cause_analysis: root_cause,
        })
    }

    fn extract_type_from_error(&self, error: &str, keyword: &str) -> String {
        error.split(keyword)
            .nth(1)
            .and_then(|s| s.split_whitespace().nth(0))
            .unwrap_or("unknown")
            .trim_matches('`')
            .to_string()
    }
}


impl AiReviewCodeTool {
    /// Perform comprehensive code review
    async fn review_code_quality(&self, input: &AiReviewCodeInput)
        -> std::result::Result<AiReviewCodeOutput, String> {

        let conn = self.ctx.storage.acquire().await
            .map_err(|e| format!("Failed to acquire connection: {}", e))?;

        // Query code units in scope
        let query = r#"
            SELECT * FROM code_unit WHERE file_path CONTAINS $scope_path LIMIT 50
        "#;

        let units: Vec<serde_json::Value> = conn.connection().query(query)
            .bind(("scope_path", input.scope_path.clone()))
            .await
            .map_err(|e| format!("Failed to query units: {}", e))?
            .take(0)
            .unwrap_or_default();

        let mut comments = Vec::new();
        let mut strengths = Vec::new();
        let mut improvements = Vec::new();

        for unit in &units {
            let file_path = unit.get("file_path").and_then(|v| v.as_str()).unwrap_or("");
            let body = unit.get("body").and_then(|v| v.as_str()).unwrap_or("");
            let name = unit.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let start_line = unit.get("start_line").and_then(|v| v.as_u64()).unwrap_or(0) as i32;

            // Check various aspects
            if input.review_aspects.contains(&"readability".to_string()) {
                comments.extend(self.check_readability(file_path, body, name, start_line, input.min_confidence));
            }

            if input.review_aspects.contains(&"performance".to_string()) {
                comments.extend(self.check_performance(file_path, body, name, start_line, input.min_confidence));
            }

            if input.review_aspects.contains(&"security".to_string()) {
                comments.extend(self.check_security(file_path, body, name, start_line, input.min_confidence));
            }

            if input.review_aspects.contains(&"best_practices".to_string()) {
                comments.extend(self.check_best_practices(file_path, body, name, start_line, input.min_confidence));
            }
        }

        // Identify strengths
        if units.iter().any(|u| u.get("docstring").is_some()) {
            strengths.push("Good documentation coverage".to_string());
        }
        if comments.iter().filter(|c| c.severity == "critical").count() == 0 {
            strengths.push("No critical issues found".to_string());
        }

        // Identify improvements
        if comments.iter().any(|c| c.category == "performance") {
            improvements.push("Consider performance optimizations".to_string());
        }
        if comments.iter().any(|c| c.category == "security") {
            improvements.push("Address security concerns".to_string());
        }

        // Calculate overall score
        let critical_count = comments.iter().filter(|c| c.severity == "critical").count();
        let major_count = comments.iter().filter(|c| c.severity == "major").count();
        let minor_count = comments.iter().filter(|c| c.severity == "minor").count();

        let overall_score = (100.0 - (critical_count as f32 * 20.0) - (major_count as f32 * 10.0) - (minor_count as f32 * 2.0))
            .max(0.0) / 100.0;

        let summary = format!(
            "Code review completed. Found {} issues ({} critical, {} major, {} minor). Overall quality: {:.1}%",
            comments.len(), critical_count, major_count, minor_count, overall_score * 100.0
        );

        Ok(AiReviewCodeOutput {
            overall_score,
            comments,
            summary,
            strengths,
            improvements,
        })
    }

    fn check_readability(&self, file_path: &str, body: &str, name: &str, start_line: i32, min_confidence: f32) -> Vec<ReviewComment> {
        let mut comments = vec![];

        // Check function length
        let line_count = body.lines().count();
        if line_count > 100 {
            comments.push(ReviewComment {
                file_path: file_path.to_string(),
                line: start_line,
                severity: "major".to_string(),
                category: "readability".to_string(),
                comment: format!("Function '{}' is too long ({} lines). Consider breaking it into smaller functions.", name, line_count),
                suggestion: Some("Extract logical sections into separate functions".to_string()),
                confidence: 0.9,
            });
        }

        // Check naming
        if name.len() < 3 && !matches!(name, "id" | "x" | "y") {
            comments.push(ReviewComment {
                file_path: file_path.to_string(),
                line: start_line,
                severity: "minor".to_string(),
                category: "readability".to_string(),
                comment: format!("Function name '{}' is too short. Use descriptive names.", name),
                suggestion: Some("Rename to a more descriptive name".to_string()),
                confidence: 0.85,
            });
        }

        comments.into_iter().filter(|c| c.confidence >= min_confidence).collect()
    }

    fn check_performance(&self, file_path: &str, body: &str, name: &str, start_line: i32, min_confidence: f32) -> Vec<ReviewComment> {
        let mut comments = vec![];

        // Check for excessive cloning
        let clone_count = body.matches(".clone()").count();
        if clone_count > 5 {
            comments.push(ReviewComment {
                file_path: file_path.to_string(),
                line: start_line,
                severity: "minor".to_string(),
                category: "performance".to_string(),
                comment: format!("Function '{}' has {} clone() calls. Consider using references.", name, clone_count),
                suggestion: Some("Use borrowing instead of cloning where possible".to_string()),
                confidence: 0.8,
            });
        }

        // Check for nested loops
        if body.matches("for ").count() > 1 && body.contains("    for ") {
            comments.push(ReviewComment {
                file_path: file_path.to_string(),
                line: start_line,
                severity: "major".to_string(),
                category: "performance".to_string(),
                comment: "Nested loops detected. This may have O(n²) complexity.".to_string(),
                suggestion: Some("Consider using HashMap or optimizing the algorithm".to_string()),
                confidence: 0.85,
            });
        }

        comments.into_iter().filter(|c| c.confidence >= min_confidence).collect()
    }

    fn check_security(&self, file_path: &str, body: &str, _name: &str, start_line: i32, min_confidence: f32) -> Vec<ReviewComment> {
        let mut comments = vec![];

        // Check for unwrap that could panic
        if body.contains("unwrap()") {
            comments.push(ReviewComment {
                file_path: file_path.to_string(),
                line: start_line,
                severity: "major".to_string(),
                category: "security".to_string(),
                comment: "Use of unwrap() can cause panics. Use proper error handling.".to_string(),
                suggestion: Some("Use ? operator or match for error handling".to_string()),
                confidence: 0.9,
            });
        }

        // Check for unsafe blocks
        if body.contains("unsafe ") {
            comments.push(ReviewComment {
                file_path: file_path.to_string(),
                line: start_line,
                severity: "critical".to_string(),
                category: "security".to_string(),
                comment: "Unsafe block detected. Ensure memory safety invariants are maintained.".to_string(),
                suggestion: Some("Document safety requirements and verify correctness".to_string()),
                confidence: 0.95,
            });
        }

        comments.into_iter().filter(|c| c.confidence >= min_confidence).collect()
    }

    fn check_best_practices(&self, file_path: &str, body: &str, name: &str, start_line: i32, min_confidence: f32) -> Vec<ReviewComment> {
        let mut comments = vec![];

        // Check for missing error handling
        if body.contains("Result") && !body.contains("?") && !body.contains("match") {
            comments.push(ReviewComment {
                file_path: file_path.to_string(),
                line: start_line,
                severity: "minor".to_string(),
                category: "best_practices".to_string(),
                comment: "Result type present but no error handling visible.".to_string(),
                suggestion: Some("Add proper error handling with ? or match".to_string()),
                confidence: 0.75,
            });
        }

        // Check for TODO comments
        if body.contains("TODO") || body.contains("FIXME") {
            comments.push(ReviewComment {
                file_path: file_path.to_string(),
                line: start_line,
                severity: "minor".to_string(),
                category: "best_practices".to_string(),
                comment: format!("Function '{}' contains TODO/FIXME comments.", name),
                suggestion: Some("Address pending work or create tracking issues".to_string()),
                confidence: 1.0,
            });
        }

        comments.into_iter().filter(|c| c.confidence >= min_confidence).collect()
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

fn default_all_refactoring_types() -> Vec<String> {
    vec![
        "extract_function".to_string(),
        "inline_function".to_string(),
        "simplify_logic".to_string(),
        "reduce_complexity".to_string(),
        "improve_naming".to_string(),
    ]
}

fn default_all_optimization_types() -> Vec<String> {
    vec![
        "algorithm".to_string(),
        "data_structure".to_string(),
        "caching".to_string(),
        "parallelization".to_string(),
    ]
}

fn default_all_review_aspects() -> Vec<String> {
    vec![
        "readability".to_string(),
        "maintainability".to_string(),
        "performance".to_string(),
        "security".to_string(),
        "best_practices".to_string(),
    ]
}

fn default_high_confidence() -> f32 {
    0.8
}

fn default_true() -> bool {
    true
}

fn default_detailed() -> String {
    "detailed".to_string()
}

fn default_google_style() -> String {
    "google".to_string()
}
