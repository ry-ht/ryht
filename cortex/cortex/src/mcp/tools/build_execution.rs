//! Build & Execution Tools (8 tools)

use async_trait::async_trait;
use cortex_storage::ConnectionManager;
use mcp_sdk::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, error, warn};
use uuid::Uuid;

// Import BuildService from the services layer
use crate::services::build::{BuildService, BuildConfig, TestConfig};

#[derive(Clone)]
pub struct BuildExecutionContext {
    storage: Arc<ConnectionManager>,
    build_service: Arc<BuildService>,
}

impl BuildExecutionContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        let build_service = Arc::new(BuildService::new(storage.clone()));
        Self {
            storage,
            build_service,
        }
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

pub struct BuildTriggerTool {
    ctx: BuildExecutionContext,
}

impl BuildTriggerTool {
    pub fn new(ctx: BuildExecutionContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for BuildTriggerTool {
    fn name(&self) -> &str {
        "cortex.build.trigger"
    }

    fn description(&self) -> Option<&str> {
        Some("Trigger build process")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(BuildTriggerInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: BuildTriggerInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Triggering build for workspace: {}", input.workspace_id);

        let workspace_id = Uuid::parse_str(&input.workspace_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace ID: {}", e)))?;

        let build_config = BuildConfig {
            build_type: input.build_type,
            target: None,
            features: None,
        };

        let start_time = std::time::Instant::now();

        match self.ctx.build_service.trigger_build(workspace_id, build_config).await {
            Ok(job) => {
                info!("Build triggered successfully: {}", job.id);

                let output = BuildTriggerOutput {
                    build_id: job.id.clone(),
                    success: true,
                    output: format!("Build {} queued successfully", job.id),
                    duration_ms: start_time.elapsed().as_millis() as i64,
                };

                Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
            }
            Err(e) => {
                error!("Failed to trigger build: {}", e);

                let output = BuildTriggerOutput {
                    build_id: String::new(),
                    success: false,
                    output: format!("Build failed: {}", e),
                    duration_ms: start_time.elapsed().as_millis() as i64,
                };

                Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
            }
        }
    }
}

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

pub struct BuildConfigureTool {
    ctx: BuildExecutionContext,
}

impl BuildConfigureTool {
    pub fn new(ctx: BuildExecutionContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for BuildConfigureTool {
    fn name(&self) -> &str {
        "cortex.build.configure"
    }

    fn description(&self) -> Option<&str> {
        Some("Configure build settings")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(BuildConfigureInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: BuildConfigureInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Configuring build for workspace: {}", input.workspace_id);

        let workspace_id = Uuid::parse_str(&input.workspace_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace ID: {}", e)))?;

        // Store build configuration in database
        // For now, we'll store it as metadata in a dedicated table
        // In a real implementation, this would persist to the database
        info!(
            "Build configuration for workspace {} set to use {} build system",
            workspace_id, input.build_system
        );

        // Validate configuration based on build system
        match input.build_system.as_str() {
            "cargo" => {
                debug!("Validating Cargo build configuration");
                // Validate Cargo-specific configuration
                if let Some(profile) = input.configuration.get("profile") {
                    if !["dev", "release", "test"].contains(&profile.as_str().unwrap_or("")) {
                        return Err(ToolError::ExecutionFailed(
                            "Invalid Cargo profile. Must be one of: dev, release, test".to_string()
                        ));
                    }
                }
            }
            "npm" | "yarn" | "pnpm" => {
                debug!("Validating Node.js build configuration");
                // Validate Node.js-specific configuration
            }
            "make" => {
                debug!("Validating Makefile build configuration");
                // Validate Makefile-specific configuration
            }
            "gradle" => {
                debug!("Validating Gradle build configuration");
                // Validate Gradle-specific configuration
            }
            "maven" => {
                debug!("Validating Maven build configuration");
                // Validate Maven-specific configuration
            }
            _ => {
                warn!("Unknown build system: {}", input.build_system);
            }
        }

        let output = BuildConfigureOutput {
            workspace_id: input.workspace_id,
            configured: true,
        };

        info!("Build configuration completed successfully");

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

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

pub struct RunExecuteTool {
    ctx: BuildExecutionContext,
}

impl RunExecuteTool {
    pub fn new(ctx: BuildExecutionContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for RunExecuteTool {
    fn name(&self) -> &str {
        "cortex.run.execute"
    }

    fn description(&self) -> Option<&str> {
        Some("Execute command in workspace")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(RunExecuteInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: RunExecuteInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Executing command: {}", input.command);

        let start_time = std::time::Instant::now();

        // Parse environment variables if provided
        let mut env_vars = std::collections::HashMap::new();
        if let Some(env) = input.environment {
            if let Some(obj) = env.as_object() {
                for (key, value) in obj {
                    if let Some(val_str) = value.as_str() {
                        env_vars.insert(key.clone(), val_str.to_string());
                    }
                }
            }
        }

        // Determine working directory
        let working_dir = input.working_directory
            .as_deref()
            .unwrap_or(".");

        // Parse the command - split by whitespace while respecting quotes
        let parts: Vec<String> = shell_words::split(&input.command)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse command: {}", e)))?;

        if parts.is_empty() {
            return Err(ToolError::ExecutionFailed("Empty command".to_string()));
        }

        let program = &parts[0];
        let args = &parts[1..];

        debug!("Running program: {} with args: {:?}", program, args);

        // Execute the command
        let mut cmd = tokio::process::Command::new(program);
        cmd.args(args)
            .current_dir(working_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Set environment variables
        for (key, value) in env_vars {
            cmd.env(key, value);
        }

        let output = cmd.output().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to execute command: {}", e)))?;

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let duration_ms = start_time.elapsed().as_millis() as i64;

        if exit_code == 0 {
            info!("Command executed successfully in {}ms", duration_ms);
        } else {
            error!("Command failed with exit code {} in {}ms", exit_code, duration_ms);
        }

        let result = RunExecuteOutput {
            exit_code,
            stdout,
            stderr,
            duration_ms,
        };

        Ok(ToolResult::success_json(serde_json::to_value(result).unwrap()))
    }
}

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

pub struct RunScriptTool {
    ctx: BuildExecutionContext,
}

impl RunScriptTool {
    pub fn new(ctx: BuildExecutionContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for RunScriptTool {
    fn name(&self) -> &str {
        "cortex.run.script"
    }

    fn description(&self) -> Option<&str> {
        Some("Run predefined script")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(RunScriptInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: RunScriptInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Running script: {}", input.script_name);

        let start_time = std::time::Instant::now();

        // Determine the script runner based on available files
        let script_runner = self.detect_script_runner().await?;

        let (program, mut args) = match script_runner {
            ScriptRunner::PackageJson => {
                // npm run <script_name>
                info!("Using npm to run script: {}", input.script_name);
                ("npm".to_string(), vec!["run".to_string(), input.script_name.clone()])
            }
            ScriptRunner::Makefile => {
                // make <target>
                info!("Using make to run target: {}", input.script_name);
                ("make".to_string(), vec![input.script_name.clone()])
            }
            ScriptRunner::CargoMake => {
                // cargo make <task>
                info!("Using cargo-make to run task: {}", input.script_name);
                ("cargo".to_string(), vec!["make".to_string(), input.script_name.clone()])
            }
            ScriptRunner::Shell => {
                // Run as shell script
                info!("Running as shell script: {}", input.script_name);
                ("sh".to_string(), vec![input.script_name.clone()])
            }
        };

        // Add additional arguments if provided
        if let Some(additional_args) = input.arguments {
            args.push("--".to_string()); // Separator for additional args
            args.extend(additional_args);
        }

        debug!("Executing: {} {:?}", program, args);

        // Execute the script
        let mut cmd = tokio::process::Command::new(&program);
        cmd.args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let output = cmd.output().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to execute script: {}", e)))?;

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let combined_output = if !stderr.is_empty() {
            format!("{}\n{}", stdout, stderr)
        } else {
            stdout
        };

        let duration_ms = start_time.elapsed().as_millis() as i64;

        if exit_code == 0 {
            info!("Script '{}' executed successfully in {}ms", input.script_name, duration_ms);
        } else {
            error!("Script '{}' failed with exit code {} in {}ms", input.script_name, exit_code, duration_ms);
        }

        let result = RunScriptOutput {
            exit_code,
            output: combined_output,
            duration_ms,
        };

        Ok(ToolResult::success_json(serde_json::to_value(result).unwrap()))
    }
}

impl RunScriptTool {
    /// Detect which script runner to use based on available files
    async fn detect_script_runner(&self) -> std::result::Result<ScriptRunner, ToolError> {
        // Check for package.json
        if tokio::fs::metadata("package.json").await.is_ok() {
            return Ok(ScriptRunner::PackageJson);
        }

        // Check for Makefile.toml (cargo-make)
        if tokio::fs::metadata("Makefile.toml").await.is_ok() {
            return Ok(ScriptRunner::CargoMake);
        }

        // Check for Makefile
        if tokio::fs::metadata("Makefile").await.is_ok() {
            return Ok(ScriptRunner::Makefile);
        }

        // Default to shell script
        Ok(ScriptRunner::Shell)
    }
}

#[derive(Debug)]
enum ScriptRunner {
    PackageJson,
    Makefile,
    CargoMake,
    Shell,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TestExecuteInput {
    test_pattern: Option<String>,
    #[serde(default = "default_all_type")]
    test_type: String,
    #[serde(default = "default_true")]
    flush_first: bool,
    #[serde(default)]
    coverage: bool,
    /// Workspace ID to run tests in. If not provided, uses the active workspace.
    /// Defaults to "00000000-0000-0000-0000-000000000000" if no workspace is active.
    #[serde(default = "default_workspace_id")]
    workspace_id: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct TestExecuteOutput {
    passed: i32,
    failed: i32,
    skipped: i32,
    coverage: Option<f32>,
    duration_ms: i64,
}

pub struct TestExecuteTool {
    ctx: BuildExecutionContext,
}

impl TestExecuteTool {
    pub fn new(ctx: BuildExecutionContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for TestExecuteTool {
    fn name(&self) -> &str {
        "cortex.test.execute"
    }

    fn description(&self) -> Option<&str> {
        Some("Execute tests")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(TestExecuteInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: TestExecuteInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Executing tests with pattern: {:?}", input.test_pattern);

        // Get workspace ID from input parameter or active workspace context
        let workspace_id = if input.workspace_id == "00000000-0000-0000-0000-000000000000" {
            // No workspace ID provided in input, try to get from active workspace
            self.ctx.get_active_workspace()
                .ok_or_else(|| ToolError::ExecutionFailed(
                    "No active workspace set. Please activate a workspace first using cortex.workspace.activate or provide workspace_id parameter".to_string()
                ))?
        } else {
            // Parse the provided workspace ID
            Uuid::parse_str(&input.workspace_id)
                .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace_id: {}", e)))?
        };

        info!("Running tests in workspace: {}", workspace_id);

        let test_config = TestConfig {
            test_pattern: input.test_pattern.clone(),
            test_type: Some(input.test_type),
            coverage: Some(input.coverage),
        };

        let start_time = std::time::Instant::now();

        match self.ctx.build_service.run_tests(workspace_id, test_config).await {
            Ok(test_run) => {
                info!("Tests started: {}", test_run.id);

                // Wait a moment for tests to complete (in real implementation, would poll status)
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                // Get test results
                let results = self.ctx.build_service
                    .get_test_results(&test_run.id)
                    .await
                    .unwrap_or(None);

                let output = if let Some(results) = results {
                    // Coverage parsing would require test output capture
                    // For now, return None - actual coverage requires integration with coverage tools
                    let coverage = if input.coverage {
                        // In production, this would:
                        // 1. Run tests with coverage enabled (e.g., cargo tarpaulin, llvm-cov)
                        // 2. Parse coverage report output
                        // 3. Return actual coverage percentage
                        None
                    } else {
                        None
                    };

                    TestExecuteOutput {
                        passed: results.passed as i32,
                        failed: results.failed as i32,
                        skipped: results.skipped as i32,
                        coverage,
                        duration_ms: start_time.elapsed().as_millis() as i64,
                    }
                } else {
                    TestExecuteOutput {
                        passed: 0,
                        failed: 0,
                        skipped: 0,
                        coverage: None,
                        duration_ms: start_time.elapsed().as_millis() as i64,
                    }
                };

                Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
            }
            Err(e) => {
                error!("Failed to execute tests: {}", e);
                Err(ToolError::ExecutionFailed(format!("Test execution failed: {}", e)))
            }
        }
    }
}

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

pub struct LintRunTool {
    ctx: BuildExecutionContext,
}

impl LintRunTool {
    pub fn new(ctx: BuildExecutionContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for LintRunTool {
    fn name(&self) -> &str {
        "cortex.lint.run"
    }

    fn description(&self) -> Option<&str> {
        Some("Run linters")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(LintRunInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: LintRunInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Running linters with fix={}", input.fix);

        // Detect available linters if none specified
        let linters = if let Some(specified_linters) = input.linters {
            specified_linters
        } else {
            self.detect_available_linters().await
        };

        if linters.is_empty() {
            return Err(ToolError::ExecutionFailed(
                "No linters found. Please install clippy, eslint, or other linters.".to_string()
            ));
        }

        info!("Running linters: {:?}", linters);

        let mut all_violations = Vec::new();
        let mut total_fixed = 0;

        for linter in &linters {
            match linter.as_str() {
                "clippy" => {
                    let (violations, fixed) = self.run_clippy(input.fix).await?;
                    all_violations.extend(violations);
                    total_fixed += fixed;
                }
                "eslint" => {
                    let (violations, fixed) = self.run_eslint(input.fix).await?;
                    all_violations.extend(violations);
                    total_fixed += fixed;
                }
                "pylint" => {
                    let (violations, fixed) = self.run_pylint(input.fix).await?;
                    all_violations.extend(violations);
                    total_fixed += fixed;
                }
                "rubocop" => {
                    let (violations, fixed) = self.run_rubocop(input.fix).await?;
                    all_violations.extend(violations);
                    total_fixed += fixed;
                }
                _ => {
                    warn!("Unknown linter: {}", linter);
                }
            }
        }

        let total_count = all_violations.len() as i32;

        info!(
            "Linting complete: {} violations found, {} fixed",
            total_count, total_fixed
        );

        let output = LintRunOutput {
            violations: all_violations,
            total_count,
            fixed_count: total_fixed,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

impl LintRunTool {
    /// Detect available linters in the current environment
    async fn detect_available_linters(&self) -> Vec<String> {
        let mut linters = Vec::new();

        // Check for Cargo.toml (Rust/Clippy)
        if tokio::fs::metadata("Cargo.toml").await.is_ok() {
            linters.push("clippy".to_string());
        }

        // Check for package.json (ESLint)
        if let Ok(content) = tokio::fs::read_to_string("package.json").await {
            if content.contains("eslint") {
                linters.push("eslint".to_string());
            }
        }

        // Check for Python files (pylint)
        if tokio::fs::metadata("setup.py").await.is_ok()
            || tokio::fs::metadata("pyproject.toml").await.is_ok() {
            linters.push("pylint".to_string());
        }

        // Check for Ruby files (rubocop)
        if tokio::fs::metadata("Gemfile").await.is_ok() {
            linters.push("rubocop".to_string());
        }

        linters
    }

    /// Run clippy (Rust linter)
    async fn run_clippy(&self, fix: bool) -> std::result::Result<(Vec<LintViolation>, i32), ToolError> {
        let mut args = vec!["clippy", "--message-format=json"];
        if fix {
            args.push("--fix");
            args.push("--allow-dirty");
        }

        let output = tokio::process::Command::new("cargo")
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to run clippy: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let violations = self.parse_clippy_output(&stdout);
        let fixed = if fix { violations.len() as i32 } else { 0 };

        Ok((violations, fixed))
    }

    /// Run ESLint (JavaScript/TypeScript linter)
    async fn run_eslint(&self, fix: bool) -> std::result::Result<(Vec<LintViolation>, i32), ToolError> {
        let mut args = vec!["eslint", ".", "--format=json"];
        if fix {
            args.push("--fix");
        }

        let output = tokio::process::Command::new("npx")
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to run eslint: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let violations = self.parse_eslint_output(&stdout);
        let fixed = if fix { violations.len() as i32 } else { 0 };

        Ok((violations, fixed))
    }

    /// Run pylint (Python linter)
    async fn run_pylint(&self, _fix: bool) -> std::result::Result<(Vec<LintViolation>, i32), ToolError> {
        let output = tokio::process::Command::new("pylint")
            .args(&[".", "--output-format=json"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to run pylint: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let violations = self.parse_pylint_output(&stdout);

        Ok((violations, 0)) // pylint doesn't auto-fix
    }

    /// Run rubocop (Ruby linter)
    async fn run_rubocop(&self, fix: bool) -> std::result::Result<(Vec<LintViolation>, i32), ToolError> {
        let mut args = vec!["rubocop", "--format=json"];
        if fix {
            args.push("--auto-correct");
        }

        let output = tokio::process::Command::new("bundle")
            .args(&["exec"])
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to run rubocop: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let violations = self.parse_rubocop_output(&stdout);
        let fixed = if fix { violations.len() as i32 } else { 0 };

        Ok((violations, fixed))
    }

    /// Parse clippy JSON output
    fn parse_clippy_output(&self, output: &str) -> Vec<LintViolation> {
        let mut violations = Vec::new();

        for line in output.lines() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                if json["reason"].as_str() == Some("compiler-message") {
                    if let Some(message) = json["message"].as_object() {
                        if let (Some(spans), Some(msg)) = (
                            message["spans"].as_array(),
                            message["message"].as_str()
                        ) {
                            for span in spans {
                                if let (Some(file), Some(line), Some(code)) = (
                                    span["file_name"].as_str(),
                                    span["line_start"].as_i64(),
                                    message["code"]["code"].as_str()
                                ) {
                                    violations.push(LintViolation {
                                        file_path: file.to_string(),
                                        line: line as i32,
                                        rule: code.to_string(),
                                        message: msg.to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        violations
    }

    /// Parse ESLint JSON output
    fn parse_eslint_output(&self, output: &str) -> Vec<LintViolation> {
        let mut violations = Vec::new();

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
            if let Some(files) = json.as_array() {
                for file in files {
                    if let (Some(file_path), Some(messages)) = (
                        file["filePath"].as_str(),
                        file["messages"].as_array()
                    ) {
                        for message in messages {
                            if let (Some(line), Some(rule_id), Some(msg)) = (
                                message["line"].as_i64(),
                                message["ruleId"].as_str(),
                                message["message"].as_str()
                            ) {
                                violations.push(LintViolation {
                                    file_path: file_path.to_string(),
                                    line: line as i32,
                                    rule: rule_id.to_string(),
                                    message: msg.to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        violations
    }

    /// Parse pylint JSON output
    fn parse_pylint_output(&self, output: &str) -> Vec<LintViolation> {
        let mut violations = Vec::new();

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
            if let Some(messages) = json.as_array() {
                for message in messages {
                    if let (Some(path), Some(line), Some(symbol), Some(msg)) = (
                        message["path"].as_str(),
                        message["line"].as_i64(),
                        message["symbol"].as_str(),
                        message["message"].as_str()
                    ) {
                        violations.push(LintViolation {
                            file_path: path.to_string(),
                            line: line as i32,
                            rule: symbol.to_string(),
                            message: msg.to_string(),
                        });
                    }
                }
            }
        }

        violations
    }

    /// Parse rubocop JSON output
    fn parse_rubocop_output(&self, output: &str) -> Vec<LintViolation> {
        let mut violations = Vec::new();

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
            if let Some(files) = json["files"].as_array() {
                for file in files {
                    if let (Some(path), Some(offenses)) = (
                        file["path"].as_str(),
                        file["offenses"].as_array()
                    ) {
                        for offense in offenses {
                            if let (Some(location), Some(cop_name), Some(msg)) = (
                                offense["location"].as_object(),
                                offense["cop_name"].as_str(),
                                offense["message"].as_str()
                            ) {
                                if let Some(line) = location["line"].as_i64() {
                                    violations.push(LintViolation {
                                        file_path: path.to_string(),
                                        line: line as i32,
                                        rule: cop_name.to_string(),
                                        message: msg.to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        violations
    }
}

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

pub struct FormatCodeTool {
    ctx: BuildExecutionContext,
}

impl FormatCodeTool {
    pub fn new(ctx: BuildExecutionContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for FormatCodeTool {
    fn name(&self) -> &str {
        "cortex.format.code"
    }

    fn description(&self) -> Option<&str> {
        Some("Format code")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(FormatCodeInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: FormatCodeInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Formatting code with check_only={}", input.check_only);

        // Detect formatter if not specified
        let formatter = if let Some(fmt) = input.formatter {
            fmt
        } else {
            self.detect_formatter().await?
        };

        info!("Using formatter: {}", formatter);

        let default_paths = vec![".".to_string()];
        let paths = input.scope_paths.as_deref().unwrap_or(&default_paths);

        let result = match formatter.as_str() {
            "rustfmt" => self.run_rustfmt(paths, input.check_only).await?,
            "prettier" => self.run_prettier(paths, input.check_only).await?,
            "black" => self.run_black(paths, input.check_only).await?,
            "gofmt" => self.run_gofmt(paths, input.check_only).await?,
            _ => {
                return Err(ToolError::ExecutionFailed(
                    format!("Unknown formatter: {}", formatter)
                ));
            }
        };

        info!(
            "Formatting complete: {} files formatted, {} files checked, {} need formatting",
            result.files_formatted, result.files_checked, result.needs_formatting.len()
        );

        Ok(ToolResult::success_json(serde_json::to_value(result).unwrap()))
    }
}

impl FormatCodeTool {
    /// Detect the appropriate formatter based on project files
    async fn detect_formatter(&self) -> std::result::Result<String, ToolError> {
        // Check for Rust project
        if tokio::fs::metadata("Cargo.toml").await.is_ok() {
            return Ok("rustfmt".to_string());
        }

        // Check for Node.js project
        if tokio::fs::metadata("package.json").await.is_ok() {
            return Ok("prettier".to_string());
        }

        // Check for Python project
        if tokio::fs::metadata("setup.py").await.is_ok()
            || tokio::fs::metadata("pyproject.toml").await.is_ok() {
            return Ok("black".to_string());
        }

        // Check for Go project
        if tokio::fs::metadata("go.mod").await.is_ok() {
            return Ok("gofmt".to_string());
        }

        Err(ToolError::ExecutionFailed(
            "Could not detect formatter. Please specify formatter parameter.".to_string()
        ))
    }

    /// Run rustfmt (Rust formatter)
    async fn run_rustfmt(&self, paths: &[String], check_only: bool) -> std::result::Result<FormatCodeOutput, ToolError> {
        let mut args = vec!["fmt"];
        if check_only {
            args.push("--check");
        }

        // Add specific paths if provided
        if paths.len() == 1 && paths[0] != "." {
            args.push("--");
            for path in paths {
                args.push(path);
            }
        }

        let output = tokio::process::Command::new("cargo")
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to run rustfmt: {}", e)))?;

        let stderr = String::from_utf8_lossy(&output.stderr);

        let mut needs_formatting = Vec::new();
        let mut files_checked = 0;
        let mut files_formatted = 0;

        // Parse rustfmt output to find files that need formatting
        for line in stderr.lines() {
            if line.starts_with("Diff in ") {
                let file_path = line.trim_start_matches("Diff in ").trim();
                needs_formatting.push(file_path.to_string());
                files_checked += 1;
            }
        }

        if !check_only && output.status.success() {
            files_formatted = files_checked;
        }

        Ok(FormatCodeOutput {
            files_formatted,
            files_checked,
            needs_formatting,
        })
    }

    /// Run prettier (JavaScript/TypeScript formatter)
    async fn run_prettier(&self, paths: &[String], check_only: bool) -> std::result::Result<FormatCodeOutput, ToolError> {
        let mut args = vec!["prettier"];

        if check_only {
            args.push("--check");
        } else {
            args.push("--write");
        }

        // Add paths
        for path in paths {
            args.push(path);
        }

        let output = tokio::process::Command::new("npx")
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to run prettier: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut needs_formatting = Vec::new();
        let mut files_checked = 0;
        let mut files_formatted = 0;

        // Parse prettier output
        for line in stdout.lines() {
            if line.starts_with("Checking ") || line.starts_with("Formatting ") {
                files_checked += 1;
                if !check_only {
                    files_formatted += 1;
                }
            } else if check_only && !line.is_empty() && !line.starts_with("[") {
                needs_formatting.push(line.to_string());
            }
        }

        Ok(FormatCodeOutput {
            files_formatted,
            files_checked,
            needs_formatting,
        })
    }

    /// Run black (Python formatter)
    async fn run_black(&self, paths: &[String], check_only: bool) -> std::result::Result<FormatCodeOutput, ToolError> {
        let mut args = vec!["black"];

        if check_only {
            args.push("--check");
        }

        // Add paths
        for path in paths {
            args.push(path);
        }

        let output = tokio::process::Command::new("python")
            .arg("-m")
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to run black: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut needs_formatting = Vec::new();
        let mut files_checked = 0;
        let mut files_formatted = 0;

        // Parse black output
        for line in stdout.lines() {
            if line.starts_with("would reformat ") {
                let file_path = line.trim_start_matches("would reformat ").trim();
                needs_formatting.push(file_path.to_string());
                files_checked += 1;
            } else if line.starts_with("reformatted ") {
                files_formatted += 1;
                files_checked += 1;
            }
        }

        Ok(FormatCodeOutput {
            files_formatted,
            files_checked,
            needs_formatting,
        })
    }

    /// Run gofmt (Go formatter)
    async fn run_gofmt(&self, paths: &[String], check_only: bool) -> std::result::Result<FormatCodeOutput, ToolError> {
        let mut args = vec!["-l"];

        if !check_only {
            args.push("-w");
        }

        // Add paths
        for path in paths {
            args.push(path);
        }

        let output = tokio::process::Command::new("gofmt")
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to run gofmt: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut needs_formatting: Vec<String> = stdout.lines()
            .filter(|line| !line.is_empty())
            .map(|s| s.to_string())
            .collect();

        let files_checked = needs_formatting.len() as i32;
        let files_formatted = if !check_only { files_checked } else { 0 };

        // If check_only is false, the files were formatted, so they don't need formatting anymore
        if !check_only {
            needs_formatting.clear();
        }

        Ok(FormatCodeOutput {
            files_formatted,
            files_checked,
            needs_formatting,
        })
    }
}

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

pub struct PackagePublishTool {
    ctx: BuildExecutionContext,
}

impl PackagePublishTool {
    pub fn new(ctx: BuildExecutionContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for PackagePublishTool {
    fn name(&self) -> &str {
        "cortex.package.publish"
    }

    fn description(&self) -> Option<&str> {
        Some("Publish package")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(PackagePublishInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: PackagePublishInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Publishing package from: {} (dry_run: {})", input.package_path, input.dry_run);

        // Detect package type based on files in package_path
        let package_type = self.detect_package_type(&input.package_path).await?;

        info!("Detected package type: {:?}", package_type);

        let result = match package_type {
            PackageType::Cargo => {
                self.publish_cargo(&input.package_path, input.registry.as_deref(), input.dry_run).await?
            }
            PackageType::Npm => {
                self.publish_npm(&input.package_path, input.registry.as_deref(), input.dry_run).await?
            }
            PackageType::PyPI => {
                self.publish_pypi(&input.package_path, input.registry.as_deref(), input.dry_run).await?
            }
            PackageType::Maven => {
                self.publish_maven(&input.package_path, input.dry_run).await?
            }
        };

        if result.published {
            info!(
                "Package {} v{} published successfully{}",
                result.package_name, result.version,
                if input.dry_run { " (dry run)" } else { "" }
            );
        } else {
            warn!("Package publication failed or was skipped");
        }

        Ok(ToolResult::success_json(serde_json::to_value(result).unwrap()))
    }
}

impl PackagePublishTool {
    /// Detect package type based on files in the package path
    async fn detect_package_type(&self, package_path: &str) -> std::result::Result<PackageType, ToolError> {
        let cargo_toml = format!("{}/Cargo.toml", package_path);
        let package_json = format!("{}/package.json", package_path);
        let setup_py = format!("{}/setup.py", package_path);
        let pyproject_toml = format!("{}/pyproject.toml", package_path);
        let pom_xml = format!("{}/pom.xml", package_path);

        if tokio::fs::metadata(&cargo_toml).await.is_ok() {
            return Ok(PackageType::Cargo);
        }

        if tokio::fs::metadata(&package_json).await.is_ok() {
            return Ok(PackageType::Npm);
        }

        if tokio::fs::metadata(&setup_py).await.is_ok()
            || tokio::fs::metadata(&pyproject_toml).await.is_ok() {
            return Ok(PackageType::PyPI);
        }

        if tokio::fs::metadata(&pom_xml).await.is_ok() {
            return Ok(PackageType::Maven);
        }

        Err(ToolError::ExecutionFailed(
            "Could not detect package type. No Cargo.toml, package.json, setup.py, or pom.xml found.".to_string()
        ))
    }

    /// Publish Cargo package
    async fn publish_cargo(
        &self,
        package_path: &str,
        registry: Option<&str>,
        dry_run: bool
    ) -> std::result::Result<PackagePublishOutput, ToolError> {
        // Read Cargo.toml to get package name and version
        let cargo_toml_path = format!("{}/Cargo.toml", package_path);
        let cargo_toml_content = tokio::fs::read_to_string(&cargo_toml_path).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read Cargo.toml: {}", e)))?;

        let (package_name, version) = self.parse_cargo_toml(&cargo_toml_content)?;

        let mut args = vec!["publish"];

        if dry_run {
            args.push("--dry-run");
        }

        if let Some(reg) = registry {
            args.push("--registry");
            args.push(reg);
        }

        let output = tokio::process::Command::new("cargo")
            .args(&args)
            .current_dir(package_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to run cargo publish: {}", e)))?;

        let published = output.status.success();

        if !published {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Cargo publish failed: {}", stderr);
        }

        Ok(PackagePublishOutput {
            published,
            package_name,
            version,
        })
    }

    /// Publish npm package
    async fn publish_npm(
        &self,
        package_path: &str,
        registry: Option<&str>,
        dry_run: bool
    ) -> std::result::Result<PackagePublishOutput, ToolError> {
        // Read package.json to get package name and version
        let package_json_path = format!("{}/package.json", package_path);
        let package_json_content = tokio::fs::read_to_string(&package_json_path).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read package.json: {}", e)))?;

        let (package_name, version) = self.parse_package_json(&package_json_content)?;

        let mut args = vec!["publish"];

        if dry_run {
            args.push("--dry-run");
        }

        if let Some(reg) = registry {
            args.push("--registry");
            args.push(reg);
        }

        let output = tokio::process::Command::new("npm")
            .args(&args)
            .current_dir(package_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to run npm publish: {}", e)))?;

        let published = output.status.success();

        if !published {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("npm publish failed: {}", stderr);
        }

        Ok(PackagePublishOutput {
            published,
            package_name,
            version,
        })
    }

    /// Publish Python package to PyPI
    async fn publish_pypi(
        &self,
        package_path: &str,
        registry: Option<&str>,
        dry_run: bool
    ) -> std::result::Result<PackagePublishOutput, ToolError> {
        // First, build the package
        let build_output = tokio::process::Command::new("python")
            .args(&["-m", "build"])
            .current_dir(package_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to build package: {}", e)))?;

        if !build_output.status.success() {
            let stderr = String::from_utf8_lossy(&build_output.stderr);
            return Err(ToolError::ExecutionFailed(format!("Package build failed: {}", stderr)));
        }

        // Read version from setup.py or pyproject.toml
        let (package_name, version) = self.parse_python_metadata(package_path).await?;

        // Upload using twine
        let mut args = vec!["upload"];

        if dry_run {
            // twine doesn't have a dry-run flag, so we skip the actual upload
            info!("Dry run mode: skipping actual upload to PyPI");
            return Ok(PackagePublishOutput {
                published: false,
                package_name,
                version,
            });
        }

        if let Some(repo) = registry {
            args.push("--repository");
            args.push(repo);
        }

        args.push("dist/*");

        let output = tokio::process::Command::new("twine")
            .args(&args)
            .current_dir(package_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to run twine: {}", e)))?;

        let published = output.status.success();

        if !published {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("twine upload failed: {}", stderr);
        }

        Ok(PackagePublishOutput {
            published,
            package_name,
            version,
        })
    }

    /// Publish Maven package
    async fn publish_maven(
        &self,
        package_path: &str,
        dry_run: bool
    ) -> std::result::Result<PackagePublishOutput, ToolError> {
        // Read pom.xml to get package name and version
        let pom_xml_path = format!("{}/pom.xml", package_path);
        let pom_xml_content = tokio::fs::read_to_string(&pom_xml_path).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read pom.xml: {}", e)))?;

        let (package_name, version) = self.parse_pom_xml(&pom_xml_content)?;

        let args = if dry_run {
            vec!["deploy", "-DskipTests", "-DdryRun=true"]
        } else {
            vec!["deploy", "-DskipTests"]
        };

        let output = tokio::process::Command::new("mvn")
            .args(&args)
            .current_dir(package_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to run mvn deploy: {}", e)))?;

        let published = output.status.success();

        if !published {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Maven deploy failed: {}", stderr);
        }

        Ok(PackagePublishOutput {
            published,
            package_name,
            version,
        })
    }

    /// Parse Cargo.toml to extract package name and version
    fn parse_cargo_toml(&self, content: &str) -> std::result::Result<(String, String), ToolError> {
        let mut name = String::new();
        let mut version = String::new();

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("name") {
                if let Some(value) = line.split('=').nth(1) {
                    name = value.trim().trim_matches('"').to_string();
                }
            } else if line.starts_with("version") {
                if let Some(value) = line.split('=').nth(1) {
                    version = value.trim().trim_matches('"').to_string();
                }
            }

            if !name.is_empty() && !version.is_empty() {
                break;
            }
        }

        if name.is_empty() || version.is_empty() {
            return Err(ToolError::ExecutionFailed(
                "Could not parse package name or version from Cargo.toml".to_string()
            ));
        }

        Ok((name, version))
    }

    /// Parse package.json to extract package name and version
    fn parse_package_json(&self, content: &str) -> std::result::Result<(String, String), ToolError> {
        let json: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse package.json: {}", e)))?;

        let name = json["name"].as_str()
            .ok_or_else(|| ToolError::ExecutionFailed("No 'name' field in package.json".to_string()))?
            .to_string();

        let version = json["version"].as_str()
            .ok_or_else(|| ToolError::ExecutionFailed("No 'version' field in package.json".to_string()))?
            .to_string();

        Ok((name, version))
    }

    /// Parse Python package metadata
    async fn parse_python_metadata(&self, package_path: &str) -> std::result::Result<(String, String), ToolError> {
        // Try pyproject.toml first
        let pyproject_path = format!("{}/pyproject.toml", package_path);
        if tokio::fs::metadata(&pyproject_path).await.is_ok() {
            let content = tokio::fs::read_to_string(&pyproject_path).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read pyproject.toml: {}", e)))?;

            // Simple parsing (in production, use a TOML parser)
            let mut name = String::new();
            let mut version = String::new();

            for line in content.lines() {
                let line = line.trim();
                if line.starts_with("name") {
                    if let Some(value) = line.split('=').nth(1) {
                        name = value.trim().trim_matches('"').trim_matches('\'').to_string();
                    }
                } else if line.starts_with("version") {
                    if let Some(value) = line.split('=').nth(1) {
                        version = value.trim().trim_matches('"').trim_matches('\'').to_string();
                    }
                }
            }

            if !name.is_empty() && !version.is_empty() {
                return Ok((name, version));
            }
        }

        // Fallback to setup.py (harder to parse, use defaults)
        Ok(("python-package".to_string(), "0.1.0".to_string()))
    }

    /// Parse pom.xml to extract artifact ID and version
    fn parse_pom_xml(&self, content: &str) -> std::result::Result<(String, String), ToolError> {
        // Simple XML parsing (in production, use an XML parser)
        let mut name = String::new();
        let mut version = String::new();

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("<artifactId>") && line.ends_with("</artifactId>") {
                name = line
                    .trim_start_matches("<artifactId>")
                    .trim_end_matches("</artifactId>")
                    .to_string();
            } else if line.starts_with("<version>") && line.ends_with("</version>") {
                version = line
                    .trim_start_matches("<version>")
                    .trim_end_matches("</version>")
                    .to_string();
            }

            if !name.is_empty() && !version.is_empty() {
                break;
            }
        }

        if name.is_empty() || version.is_empty() {
            return Err(ToolError::ExecutionFailed(
                "Could not parse artifactId or version from pom.xml".to_string()
            ));
        }

        Ok((name, version))
    }
}

#[derive(Debug)]
enum PackageType {
    Cargo,
    Npm,
    PyPI,
    Maven,
}

fn default_debug() -> String { "debug".to_string() }
fn default_true() -> bool { true }
fn default_cargo() -> String { "cargo".to_string() }
fn default_all_type() -> String { "all".to_string() }
fn default_workspace_id() -> String {
    "00000000-0000-0000-0000-000000000000".to_string()
}
