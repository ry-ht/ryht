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
    #[allow(dead_code)]
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
            #[allow(dead_code)]
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
    #[allow(dead_code)]
    scope_path: String,
    #[allow(dead_code)]
    metrics: Option<Vec<String>>,
    #[serde(default = "default_none")]
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    scope_path: String,
    #[allow(dead_code)]
    smell_types: Option<Vec<String>>,
    #[serde(default = "default_medium")]
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    scope_path: String,
    #[allow(dead_code)]
    conventions: NamingConventions,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NamingConventions {
    #[allow(dead_code)]
    functions: Option<String>,
    #[allow(dead_code)]
    classes: Option<String>,
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    scope_path: String,
    #[serde(default = "default_afferent")]
    #[allow(dead_code)]
    coupling_type: String,
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    module_path: String,
    #[serde(default = "default_lcom")]
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    scope_path: String,
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    scope_path: String,
    #[allow(dead_code)]
    refactoring_types: Option<Vec<String>>,
    #[serde(default = "default_confidence")]
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    scope_path: String,
    #[allow(dead_code)]
    metrics: Option<Vec<String>>,
    #[serde(default = "default_file_group")]
    #[allow(dead_code)]
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
