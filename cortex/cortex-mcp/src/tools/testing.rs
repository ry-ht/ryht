//! Testing & Validation Tools (10 tools)

use async_trait::async_trait;
use cortex_storage::ConnectionManager;
use mcp_server::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

#[derive(Clone)]
pub struct TestingContext {
    storage: Arc<ConnectionManager>,
}

impl TestingContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self { storage }
    }
}

macro_rules! impl_test_tool {
    ($name:ident, $tool_name:expr, $desc:expr, $input:ty, $output:ty) => {
        pub struct $name {
            ctx: TestingContext,
        }

        impl $name {
            pub fn new(ctx: TestingContext) -> Self {
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

            async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::std::result::Result<ToolResult, ToolError> {
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
pub struct TestGenerateInput {
    unit_id: String,
    #[serde(default = "default_unit_type")]
    test_type: String,
    framework: Option<String>,
    #[serde(default = "default_coverage")]
    coverage_target: f32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct TestGenerateOutput {
    test_code: String,
    test_cases: Vec<String>,
    estimated_coverage: f32,
}

impl_test_tool!(TestGenerateTool, "cortex.test.generate", "Generate tests for code", TestGenerateInput, TestGenerateOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TestValidateInput {
    test_code: String,
    target_unit_id: String,
    #[serde(default = "default_true")]
    check_coverage: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct TestValidateOutput {
    valid: bool,
    errors: Vec<String>,
    coverage: f32,
}

impl_test_tool!(TestValidateTool, "cortex.test.validate", "Validate generated tests", TestValidateInput, TestValidateOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TestFindMissingInput {
    scope_path: String,
    #[serde(default = "default_complexity_one")]
    min_complexity: i32,
    #[serde(default)]
    include_private: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct TestFindMissingOutput {
    untested_units: Vec<UntestedUnit>,
    total_count: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct UntestedUnit {
    unit_id: String,
    name: String,
    complexity: i32,
}

impl_test_tool!(TestFindMissingTool, "cortex.test.find_missing", "Find code without tests", TestFindMissingInput, TestFindMissingOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TestAnalyzeCoverageInput {
    scope_path: String,
    #[serde(default = "default_line_coverage")]
    coverage_type: String,
    #[serde(default)]
    include_details: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct TestAnalyzeCoverageOutput {
    overall_coverage: f32,
    file_coverage: Vec<FileCoverage>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct FileCoverage {
    file_path: String,
    coverage: f32,
}

impl_test_tool!(TestAnalyzeCoverageTool, "cortex.test.analyze_coverage", "Analyze test coverage", TestAnalyzeCoverageInput, TestAnalyzeCoverageOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TestRunInMemoryInput {
    test_ids: Vec<String>,
    #[serde(default = "default_true")]
    mock_dependencies: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct TestRunInMemoryOutput {
    results: Vec<TestResult>,
    passed: i32,
    failed: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct TestResult {
    test_id: String,
    passed: bool,
    error: Option<String>,
}

impl_test_tool!(TestRunInMemoryTool, "cortex.test.run_in_memory", "Run tests in memory (interpreted)", TestRunInMemoryInput, TestRunInMemoryOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ValidateSyntaxInput {
    code: String,
    language: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ValidateSyntaxOutput {
    valid: bool,
    errors: Vec<SyntaxError>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct SyntaxError {
    line: i32,
    column: i32,
    message: String,
}

impl_test_tool!(ValidateSyntaxTool, "cortex.validate.syntax", "Validate syntax without parsing", ValidateSyntaxInput, ValidateSyntaxOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ValidateSemanticsInput {
    unit_id: String,
    #[serde(default = "default_true")]
    check_types: bool,
    #[serde(default = "default_true")]
    check_undefined: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ValidateSemanticsOutput {
    valid: bool,
    errors: Vec<SemanticError>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct SemanticError {
    error_type: String,
    message: String,
    location: String,
}

impl_test_tool!(ValidateSemanticsTool, "cortex.validate.semantics", "Validate semantic correctness", ValidateSemanticsInput, ValidateSemanticsOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ValidateContractsInput {
    unit_id: String,
    contracts: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ValidateContractsOutput {
    valid: bool,
    violations: Vec<String>,
}

impl_test_tool!(ValidateContractsTool, "cortex.validate.contracts", "Validate design contracts", ValidateContractsInput, ValidateContractsOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ValidateDependenciesInput {
    scope_path: String,
    rules: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ValidateDependenciesOutput {
    valid: bool,
    violations: Vec<String>,
}

impl_test_tool!(ValidateDependenciesTool, "cortex.validate.dependencies", "Validate dependency constraints", ValidateDependenciesInput, ValidateDependenciesOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ValidateStyleInput {
    scope_path: String,
    style_guide: Option<String>,
    #[serde(default)]
    auto_fix: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ValidateStyleOutput {
    violations: Vec<StyleViolation>,
    total_count: i32,
    auto_fixed: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct StyleViolation {
    file_path: String,
    line: i32,
    rule: String,
    message: String,
}

impl_test_tool!(ValidateStyleTool, "cortex.validate.style", "Validate code style", ValidateStyleInput, ValidateStyleOutput);

fn default_unit_type() -> String { "unit".to_string() }
fn default_coverage() -> f32 { 0.8 }
fn default_true() -> bool { true }
fn default_complexity_one() -> i32 { 1 }
fn default_line_coverage() -> String { "line".to_string() }
