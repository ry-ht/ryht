//! Build & Execution Tools (8 tools)

use async_trait::async_trait;
use cortex_storage::ConnectionManager;
use mcp_server::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

#[derive(Clone)]
pub struct BuildExecutionContext {
    storage: Arc<ConnectionManager>,
}

impl BuildExecutionContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self { storage }
    }
}

macro_rules! impl_build_tool {
    ($name:ident, $tool_name:expr, $desc:expr, $input:ty, $output:ty) => {
        pub struct $name {
            ctx: BuildExecutionContext,
        }

        impl $name {
            pub fn new(ctx: BuildExecutionContext) -> Self {
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
pub struct BuildTriggerInput {
    workspace_id: String,
    #[serde(default = "default_debug")]
    build_type: String,
    #[serde(default = "default_true")]
    flush_first: bool,
    #[serde(default = "default_true")]
    capture_output: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct BuildTriggerOutput {
    build_id: String,
    success: bool,
    output: String,
    duration_ms: i64,
}

impl_build_tool!(BuildTriggerTool, "cortex.build.trigger", "Trigger build process", BuildTriggerInput, BuildTriggerOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BuildConfigureInput {
    workspace_id: String,
    #[serde(default = "default_cargo")]
    build_system: String,
    configuration: serde_json::Value,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct BuildConfigureOutput {
    workspace_id: String,
    configured: bool,
}

impl_build_tool!(BuildConfigureTool, "cortex.build.configure", "Configure build settings", BuildConfigureInput, BuildConfigureOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RunExecuteInput {
    command: String,
    working_directory: Option<String>,
    environment: Option<serde_json::Value>,
    #[serde(default = "default_true")]
    flush_first: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct RunExecuteOutput {
    exit_code: i32,
    stdout: String,
    stderr: String,
    duration_ms: i64,
}

impl_build_tool!(RunExecuteTool, "cortex.run.execute", "Execute command in workspace", RunExecuteInput, RunExecuteOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RunScriptInput {
    script_name: String,
    arguments: Option<Vec<String>>,
    #[serde(default = "default_true")]
    flush_first: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct RunScriptOutput {
    exit_code: i32,
    output: String,
    duration_ms: i64,
}

impl_build_tool!(RunScriptTool, "cortex.run.script", "Run predefined script", RunScriptInput, RunScriptOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TestExecuteInput {
    test_pattern: Option<String>,
    #[serde(default = "default_all_type")]
    test_type: String,
    #[serde(default = "default_true")]
    flush_first: bool,
    #[serde(default)]
    coverage: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct TestExecuteOutput {
    passed: i32,
    failed: i32,
    skipped: i32,
    coverage: Option<f32>,
    duration_ms: i64,
}

impl_build_tool!(TestExecuteTool, "cortex.test.execute", "Execute tests", TestExecuteInput, TestExecuteOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LintRunInput {
    linters: Option<Vec<String>>,
    #[serde(default)]
    fix: bool,
    #[serde(default = "default_true")]
    flush_first: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct LintRunOutput {
    violations: Vec<LintViolation>,
    total_count: i32,
    fixed_count: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct LintViolation {
    file_path: String,
    line: i32,
    rule: String,
    message: String,
}

impl_build_tool!(LintRunTool, "cortex.lint.run", "Run linters", LintRunInput, LintRunOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FormatCodeInput {
    scope_paths: Option<Vec<String>>,
    formatter: Option<String>,
    #[serde(default)]
    check_only: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct FormatCodeOutput {
    files_formatted: i32,
    files_checked: i32,
    needs_formatting: Vec<String>,
}

impl_build_tool!(FormatCodeTool, "cortex.format.code", "Format code", FormatCodeInput, FormatCodeOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PackagePublishInput {
    package_path: String,
    registry: Option<String>,
    #[serde(default = "default_true")]
    dry_run: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct PackagePublishOutput {
    published: bool,
    package_name: String,
    version: String,
}

impl_build_tool!(PackagePublishTool, "cortex.package.publish", "Publish package", PackagePublishInput, PackagePublishOutput);

fn default_debug() -> String { "debug".to_string() }
fn default_true() -> bool { true }
fn default_cargo() -> String { "cargo".to_string() }
fn default_all_type() -> String { "all".to_string() }
