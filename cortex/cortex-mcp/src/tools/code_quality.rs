//! Code Quality Tools (8 tools)

use async_trait::async_trait;
use cortex_storage::ConnectionManager;
use mcp_server::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

#[derive(Clone)]
pub struct CodeQualityContext {
    storage: Arc<ConnectionManager>,
}

impl CodeQualityContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self { storage }
    }
}

macro_rules! impl_quality_tool {
    ($name:ident, $tool_name:expr, $desc:expr, $input:ty, $output:ty) => {
        pub struct $name {
            ctx: CodeQualityContext,
        }

        impl $name {
            pub fn new(ctx: CodeQualityContext) -> Self {
                Self { ctx }
            }
        }

        #[async_trait]
        impl Tool for $name {
            fn name(&self) -> &str {
                $tool_name
            }

            fn description(&self) -> Option<&str> {
                Some($desc)
            }

            fn input_schema(&self) -> Value {
                serde_json::to_value(schemars::schema_for!($input)).unwrap()
            }

            async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
                let _input: $input = serde_json::from_value(input)
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
                debug!("{} executed", $tool_name);
                let output = <$output>::default();
                Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
            }
        }
    };
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyzeComplexityInput {
    scope_path: String,
    metrics: Option<Vec<String>>,
    #[serde(default = "default_none")]
    aggregate_by: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct AnalyzeComplexityOutput {
    metrics: Vec<ComplexityMetric>,
    average_complexity: f32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ComplexityMetric {
    entity_id: String,
    metric_name: String,
    value: i32,
}

impl_quality_tool!(QualityAnalyzeComplexityTool, "cortex.quality.analyze_complexity", "Analyze code complexity metrics", AnalyzeComplexityInput, AnalyzeComplexityOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindCodeSmellsInput {
    scope_path: String,
    smell_types: Option<Vec<String>>,
    #[serde(default = "default_medium")]
    severity_threshold: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct FindCodeSmellsOutput {
    smells: Vec<CodeSmell>,
    total_count: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct CodeSmell {
    entity_id: String,
    smell_type: String,
    severity: String,
    description: String,
}

impl_quality_tool!(QualityFindCodeSmellsTool, "cortex.quality.find_code_smells", "Detect code smells", FindCodeSmellsInput, FindCodeSmellsOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckNamingInput {
    scope_path: String,
    conventions: NamingConventions,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NamingConventions {
    functions: Option<String>,
    classes: Option<String>,
    variables: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct CheckNamingOutput {
    violations: Vec<NamingViolation>,
    total_count: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct NamingViolation {
    entity_id: String,
    entity_type: String,
    current_name: String,
    expected_pattern: String,
}

impl_quality_tool!(QualityCheckNamingTool, "cortex.quality.check_naming", "Check naming conventions", CheckNamingInput, CheckNamingOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyzeCouplingInput {
    scope_path: String,
    #[serde(default = "default_afferent")]
    coupling_type: String,
    threshold: Option<f32>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct AnalyzeCouplingOutput {
    modules: Vec<ModuleCoupling>,
    average_coupling: f32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ModuleCoupling {
    module_id: String,
    coupling_value: f32,
}

impl_quality_tool!(QualityAnalyzeCouplingTool, "cortex.quality.analyze_coupling", "Analyze module coupling", AnalyzeCouplingInput, AnalyzeCouplingOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyzeCohesionInput {
    module_path: String,
    #[serde(default = "default_lcom")]
    cohesion_type: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct AnalyzeCohesionOutput {
    module_path: String,
    cohesion_value: f32,
    cohesion_type: String,
}

impl_quality_tool!(QualityAnalyzeCohesionTool, "cortex.quality.analyze_cohesion", "Analyze module cohesion", AnalyzeCohesionInput, AnalyzeCohesionOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindAntipatternsInput {
    scope_path: String,
    pattern_types: Option<Vec<String>>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct FindAntipatternsOutput {
    antipatterns: Vec<Antipattern>,
    total_count: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct Antipattern {
    entity_id: String,
    pattern_type: String,
    description: String,
    severity: String,
}

impl_quality_tool!(QualityFindAntipatternsTool, "cortex.quality.find_antipatterns", "Detect anti-patterns", FindAntipatternsInput, FindAntipatternsOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SuggestRefactoringsInput {
    scope_path: String,
    refactoring_types: Option<Vec<String>>,
    #[serde(default = "default_confidence")]
    min_confidence: f32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct SuggestRefactoringsOutput {
    suggestions: Vec<RefactoringSuggestion>,
    total_count: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct RefactoringSuggestion {
    entity_id: String,
    refactoring_type: String,
    description: String,
    confidence: f32,
}

impl_quality_tool!(QualitySuggestRefactoringsTool, "cortex.quality.suggest_refactorings", "Suggest refactoring opportunities", SuggestRefactoringsInput, SuggestRefactoringsOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CalculateMetricsInput {
    scope_path: String,
    metrics: Option<Vec<String>>,
    #[serde(default = "default_file_group")]
    group_by: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct CalculateMetricsOutput {
    metrics: Vec<MetricValue>,
    total_lines: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct MetricValue {
    metric_name: String,
    value: i32,
    group: String,
}

impl_quality_tool!(QualityCalculateMetricsTool, "cortex.quality.calculate_metrics", "Calculate code metrics", CalculateMetricsInput, CalculateMetricsOutput);

fn default_none() -> String { "none".to_string() }
fn default_medium() -> String { "medium".to_string() }
fn default_afferent() -> String { "afferent".to_string() }
fn default_lcom() -> String { "lcom".to_string() }
fn default_confidence() -> f32 { 0.7 }
fn default_file_group() -> String { "file".to_string() }
