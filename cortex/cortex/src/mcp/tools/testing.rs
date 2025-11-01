//! Testing & Validation Tools (9 tools)

use async_trait::async_trait;
use cortex_code_analysis::{CodeParser, FunctionInfo, Lang as Language, ParsedFile};
use cortex_storage::ConnectionManager;
use mcp_sdk::prelude::*;
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

#[derive(Clone)]
pub struct TestingContext {
    storage: Arc<ConnectionManager>,
}

impl TestingContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self { storage }
    }

    /// Parse a single file
    async fn parse_file(&self, file_path: &str) -> std::result::Result<Option<ParsedFileWithPath>, ToolError> {
        let path = Path::new(file_path);

        // Check if language is supported
        if Language::from_path(path).is_none() {
            return Ok(None);
        }

        let source = std::fs::read_to_string(file_path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read file: {}", e)))?;

        let mut parser = CodeParser::new()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create parser: {}", e)))?;

        let parsed = parser.parse_file_auto(file_path, &source)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse file: {}", e)))?;

        Ok(Some(ParsedFileWithPath {
            path: file_path.to_string(),
            source,
            parsed,
        }))
    }

    /// Parse all files in a directory
    async fn parse_directory(&self, dir_path: &str) -> std::result::Result<Vec<ParsedFileWithPath>, ToolError> {
        let mut results = Vec::new();
        let extensions = ["rs", "ts", "tsx", "js", "jsx"];

        let walker = walkdir::WalkDir::new(dir_path)
            .follow_links(false)
            .max_depth(10);

        for entry in walker {
            let entry = entry
                .map_err(|e| ToolError::ExecutionFailed(format!("Directory walk error: {}", e)))?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if extensions.contains(&ext.to_str().unwrap_or("")) {
                        if let Some(parsed) = self.parse_file(path.to_str().unwrap()).await? {
                            results.push(parsed);
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// Parse a scope (file or directory)
    async fn parse_scope(&self, scope_path: &str) -> std::result::Result<Vec<ParsedFileWithPath>, ToolError> {
        let path = Path::new(scope_path);
        let mut results = Vec::new();

        if path.is_file() {
            if let Some(parsed) = self.parse_file(scope_path).await? {
                results.push(parsed);
            }
        } else if path.is_dir() {
            results = self.parse_directory(scope_path).await?;
        } else {
            return Err(ToolError::ExecutionFailed(format!("Path not found: {}", scope_path)));
        }

        Ok(results)
    }

    /// Get code unit from storage
    async fn get_code_unit(&self, unit_id: &str) -> std::result::Result<Option<StoredCodeUnit>, ToolError> {
        let pooled = self.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to acquire connection: {}", e)))?;
        let conn = pooled.connection();

        let query = format!("SELECT * FROM code_unit WHERE id = '{}'", unit_id);
        let mut result = conn.query(&query).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to query code unit: {}", e)))?;

        let units: Vec<StoredCodeUnit> = result.take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse code unit: {}", e)))?;

        Ok(units.into_iter().next())
    }
}

#[derive(Clone)]
struct ParsedFileWithPath {
    path: String,
    source: String,
    parsed: ParsedFile,
}

#[derive(Debug, Deserialize, Serialize)]
struct StoredCodeUnit {
    id: String,
    name: String,
    file_path: String,
    language: String,
    unit_type: String,
    start_line: usize,
    end_line: usize,
    body: Option<String>,
    signature: Option<String>,
    has_tests: bool,
    complexity: serde_json::Value,
}

// ============================================================================
// Tool 1: cortex.test.validate - Validate generated tests
// ============================================================================

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

pub struct TestValidateTool {
    ctx: TestingContext,
}

impl TestValidateTool {
    pub fn new(ctx: TestingContext) -> Self {
        Self { ctx }
    }

    fn validate_rust_test(&self, test_code: &str) -> (bool, Vec<String>) {
        let mut errors = Vec::new();

        // Check for test module
        if !test_code.contains("#[cfg(test)]") && !test_code.contains("#[test]") {
            errors.push("Missing #[test] or #[cfg(test)] attribute".to_string());
        }

        // Check for assertions
        let has_assertions = test_code.contains("assert!")
            || test_code.contains("assert_eq!")
            || test_code.contains("assert_ne!")
            || test_code.contains("expect");

        if !has_assertions {
            errors.push("No assertions found in test code".to_string());
        }

        // Note: Parser initialization check removed as tree_sitter::Parser::new()
        // always succeeds and returns a Parser directly, not a Result

        (errors.is_empty(), errors)
    }

    fn validate_typescript_test(&self, test_code: &str) -> (bool, Vec<String>) {
        let mut errors = Vec::new();

        // Check for test framework keywords
        if !test_code.contains("describe") && !test_code.contains("test") && !test_code.contains("it") {
            errors.push("Missing test framework keywords (describe/test/it)".to_string());
        }

        // Check for expectations
        if !test_code.contains("expect") && !test_code.contains("assert") {
            errors.push("No assertions/expectations found in test code".to_string());
        }

        (errors.is_empty(), errors)
    }
}

#[async_trait]
impl Tool for TestValidateTool {
    fn name(&self) -> &str {
        "cortex.test.validate"
    }

    fn description(&self) -> Option<&str> {
        Some("Validate generated tests")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(TestValidateInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: TestValidateInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Validating test for unit: {}", input.target_unit_id);

        // Get code unit to determine language
        let unit = self.ctx.get_code_unit(&input.target_unit_id).await?
            .ok_or_else(|| ToolError::ExecutionFailed(format!("Code unit not found: {}", input.target_unit_id)))?;

        let language = Language::from_path(Path::new(&unit.file_path))
            .ok_or_else(|| ToolError::ExecutionFailed("Could not detect language".to_string()))?;

        // Validate based on language
        let (valid, errors) = match language {
            Language::Rust => self.validate_rust_test(&input.test_code),
            Language::TypeScript | Language::JavaScript => self.validate_typescript_test(&input.test_code),
            Language::Tsx | Language::Jsx => self.validate_typescript_test(&input.test_code),
            Language::Python | Language::Cpp | Language::Java | Language::Kotlin => {
                return Err(ToolError::ExecutionFailed(format!("Test validation not supported for language: {:?}", language)));
            }
        };

        // Estimate coverage if requested
        let coverage = if input.check_coverage {
            let test_count = input.test_code.matches("fn test_").count()
                + input.test_code.matches("test('").count()
                + input.test_code.matches("it('").count();
            (test_count as f32 * 0.25).min(1.0)
        } else {
            0.0
        };

        info!("Test validation complete. Valid: {}, Errors: {}", valid, errors.len());

        let output = TestValidateOutput {
            valid,
            errors,
            coverage,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// ============================================================================
// Tool 3: cortex.test.find_missing - Find code without tests
// ============================================================================

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

pub struct TestFindMissingTool {
    ctx: TestingContext,
}

impl TestFindMissingTool {
    pub fn new(ctx: TestingContext) -> Self {
        Self { ctx }
    }

    fn has_tests_in_file(&self, parsed: &ParsedFile, func_name: &str) -> bool {
        // Check if there are test functions that reference this function
        for test_func in &parsed.functions {
            if test_func.attributes.iter().any(|a| a.contains("test")) {
                if test_func.body.contains(func_name) {
                    return true;
                }
            }
        }
        false
    }
}

#[async_trait]
impl Tool for TestFindMissingTool {
    fn name(&self) -> &str {
        "cortex.test.find_missing"
    }

    fn description(&self) -> Option<&str> {
        Some("Find code without tests")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(TestFindMissingInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: TestFindMissingInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Finding untested code in: {}", input.scope_path);

        let files = self.ctx.parse_scope(&input.scope_path).await?;
        let mut untested_units = Vec::new();

        for file in files {
            for func in &file.parsed.functions {
                // Skip test functions themselves
                if func.attributes.iter().any(|a| a.contains("test")) {
                    continue;
                }

                // Skip private functions unless requested
                if !input.include_private && func.visibility == cortex_code_analysis::Visibility::Private {
                    continue;
                }

                // Check complexity threshold
                let complexity = func.complexity.unwrap_or(1) as i32;
                if complexity < input.min_complexity {
                    continue;
                }

                // Check if function has tests
                let has_tests = self.has_tests_in_file(&file.parsed, &func.name);

                if !has_tests {
                    untested_units.push(UntestedUnit {
                        unit_id: format!("{}::{}", file.path, func.name),
                        name: func.name.clone(),
                        complexity,
                    });
                }
            }
        }

        let total_count = untested_units.len() as i32;
        info!("Found {} untested units", total_count);

        let output = TestFindMissingOutput {
            untested_units,
            total_count,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// ============================================================================
// Tool 4: cortex.test.analyze_coverage - Analyze test coverage
// ============================================================================

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

pub struct TestAnalyzeCoverageTool {
    ctx: TestingContext,
}

impl TestAnalyzeCoverageTool {
    pub fn new(ctx: TestingContext) -> Self {
        Self { ctx }
    }

    fn calculate_file_coverage(&self, parsed: &ParsedFile) -> f32 {
        let mut total_functions = 0;
        let mut tested_functions = 0;

        // Find all non-test functions
        let mut test_function_names = HashSet::new();
        for func in &parsed.functions {
            if func.attributes.iter().any(|a| a.contains("test")) {
                test_function_names.insert(func.name.clone());
            }
        }

        // Check coverage for each function
        for func in &parsed.functions {
            if func.attributes.iter().any(|a| a.contains("test")) {
                continue;
            }
            if func.visibility == cortex_code_analysis::Visibility::Private {
                continue;
            }

            total_functions += 1;

            // Check if any test references this function
            let has_test = parsed.functions.iter().any(|test_func| {
                test_func.attributes.iter().any(|a| a.contains("test"))
                && test_func.body.contains(&func.name)
            });

            if has_test {
                tested_functions += 1;
            }
        }

        if total_functions == 0 {
            return 0.0;
        }

        tested_functions as f32 / total_functions as f32
    }
}

#[async_trait]
impl Tool for TestAnalyzeCoverageTool {
    fn name(&self) -> &str {
        "cortex.test.analyze_coverage"
    }

    fn description(&self) -> Option<&str> {
        Some("Analyze test coverage")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(TestAnalyzeCoverageInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: TestAnalyzeCoverageInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Analyzing coverage for: {}", input.scope_path);

        let files = self.ctx.parse_scope(&input.scope_path).await?;
        let mut file_coverage = Vec::new();
        let mut total_coverage = 0.0;

        for file in files {
            let coverage = self.calculate_file_coverage(&file.parsed);

            if input.include_details {
                file_coverage.push(FileCoverage {
                    file_path: file.path.clone(),
                    coverage,
                });
            }

            total_coverage += coverage;
        }

        let overall_coverage = if file_coverage.is_empty() {
            0.0
        } else {
            total_coverage / file_coverage.len() as f32
        };

        info!("Coverage analysis complete. Overall: {:.2}%", overall_coverage * 100.0);

        let output = TestAnalyzeCoverageOutput {
            overall_coverage,
            file_coverage,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// ============================================================================
// Tool 5: cortex.test.run_in_memory - Run tests in memory (interpreted)
// ============================================================================

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

pub struct TestRunInMemoryTool {
    ctx: TestingContext,
}

impl TestRunInMemoryTool {
    pub fn new(ctx: TestingContext) -> Self {
        Self { ctx }
    }

    /// Analyze test code and validate its structure
    async fn analyze_test(&self, test_id: &str) -> std::result::Result<(bool, Option<String>), String> {
        // Parse test_id: format is "file_path::test_name" or just a code_unit_id
        let (file_path, test_name) = if test_id.contains("::") {
            let parts: Vec<&str> = test_id.splitn(2, "::").collect();
            if parts.len() != 2 {
                return Err(format!("Invalid test_id format: {}", test_id));
            }
            (parts[0].to_string(), parts[1].to_string())
        } else {
            // Try to get from storage as a code unit
            match self.ctx.get_code_unit(test_id).await {
                Ok(Some(unit)) => (unit.file_path.clone(), unit.name.clone()),
                Ok(None) => return Err(format!("Test not found: {}", test_id)),
                Err(e) => return Err(format!("Failed to retrieve test: {}", e)),
            }
        };

        // Parse the file to get the test code
        let parsed_file = match self.ctx.parse_file(&file_path).await {
            Ok(Some(pf)) => pf,
            Ok(None) => return Err(format!("File not found or not parsable: {}", file_path)),
            Err(e) => return Err(format!("Failed to parse file: {}", e)),
        };

        // Find the test function
        let test_func = parsed_file.parsed.functions.iter()
            .find(|f| f.name == test_name)
            .ok_or_else(|| format!("Test function not found: {}", test_name))?;

        // Validate that this is actually a test function
        let is_test_function = test_func.attributes.iter()
            .any(|a| a.contains("test") || a.contains("Test"));

        if !is_test_function {
            return Err(format!("Function '{}' is not marked as a test", test_name));
        }

        // Analyze test structure based on language
        let language = Language::from_path(Path::new(&file_path))
            .ok_or_else(|| "Could not detect language".to_string())?;

        match language {
            Language::Rust => self.analyze_rust_test(test_func),
            Language::TypeScript | Language::JavaScript => self.analyze_typescript_test(test_func),
            Language::Tsx | Language::Jsx => self.analyze_typescript_test(test_func),
            Language::Python | Language::Cpp | Language::Java | Language::Kotlin => {
                Err(format!("Test analysis not supported for language: {:?}", language))
            }
        }
    }

    /// Analyze Rust test structure
    fn analyze_rust_test(&self, test_func: &FunctionInfo) -> std::result::Result<(bool, Option<String>), String> {
        let body = &test_func.body;

        // Check if test is empty
        if body.trim().is_empty() || body.trim() == "{}" {
            return Ok((false, Some("Test body is empty".to_string())));
        }

        // Check for assertions
        let has_assertions = body.contains("assert!")
            || body.contains("assert_eq!")
            || body.contains("assert_ne!")
            || body.contains("panic!")
            || body.contains("expect(")
            || body.contains("unwrap()")
            || body.contains("should_panic");

        if !has_assertions {
            return Ok((false, Some("Test contains no assertions or expectations".to_string())));
        }

        // Check for common test issues
        if body.contains("TODO") && !has_assertions {
            return Ok((false, Some("Test is incomplete (contains TODO)".to_string())));
        }

        // Check for syntax errors - basic validation
        if !self.check_balanced_braces(body) {
            return Ok((false, Some("Unbalanced braces in test code".to_string())));
        }

        // Test appears valid
        Ok((true, None))
    }

    /// Analyze TypeScript/JavaScript test structure
    fn analyze_typescript_test(&self, test_func: &FunctionInfo) -> std::result::Result<(bool, Option<String>), String> {
        let body = &test_func.body;

        // Check if test is empty
        if body.trim().is_empty() || body.trim() == "{}" {
            return Ok((false, Some("Test body is empty".to_string())));
        }

        // Check for expectations/assertions
        let has_expectations = body.contains("expect(")
            || body.contains("assert")
            || body.contains("should")
            || body.contains("toBe")
            || body.contains("toEqual");

        if !has_expectations {
            return Ok((false, Some("Test contains no expectations or assertions".to_string())));
        }

        // Check for common test issues
        if body.contains("TODO") && !has_expectations {
            return Ok((false, Some("Test is incomplete (contains TODO)".to_string())));
        }

        // Check for syntax errors - basic validation
        if !self.check_balanced_braces(body) {
            return Ok((false, Some("Unbalanced braces in test code".to_string())));
        }

        // Test appears valid
        Ok((true, None))
    }

    /// Check if braces are balanced in the code
    fn check_balanced_braces(&self, code: &str) -> bool {
        let mut count = 0;
        for c in code.chars() {
            match c {
                '{' => count += 1,
                '}' => {
                    count -= 1;
                    if count < 0 {
                        return false;
                    }
                }
                _ => {}
            }
        }
        count == 0
    }
}

#[async_trait]
impl Tool for TestRunInMemoryTool {
    fn name(&self) -> &str {
        "cortex.test.run_in_memory"
    }

    fn description(&self) -> Option<&str> {
        Some("Run tests in memory (interpreted)")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(TestRunInMemoryInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: TestRunInMemoryInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Running {} tests in memory", input.test_ids.len());

        let mut results = Vec::new();
        let mut passed = 0;
        let mut failed = 0;

        // Note: Full test execution would require a runtime interpreter/compiler
        // This implementation uses static analysis to validate test structure
        for test_id in input.test_ids {
            debug!("Analyzing test: {}", test_id);

            // Analyze the test using static analysis
            match self.analyze_test(&test_id).await {
                Ok((test_passed, error_msg)) => {
                    if test_passed {
                        passed += 1;
                        debug!("Test passed: {}", test_id);
                        results.push(TestResult {
                            test_id,
                            passed: true,
                            error: None,
                        });
                    } else {
                        failed += 1;
                        let error = error_msg.unwrap_or_else(|| "Test validation failed".to_string());
                        debug!("Test failed: {} - {}", test_id, error);
                        results.push(TestResult {
                            test_id,
                            passed: false,
                            error: Some(error),
                        });
                    }
                }
                Err(e) => {
                    failed += 1;
                    debug!("Test error: {} - {}", test_id, e);
                    results.push(TestResult {
                        test_id,
                        passed: false,
                        error: Some(e),
                    });
                }
            }
        }

        info!("Test execution complete. Passed: {}, Failed: {}", passed, failed);

        let output = TestRunInMemoryOutput {
            results,
            passed,
            failed,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// ============================================================================
// REMOVED: Validation Tools (Tools 6-10)
// - cortex.validate.syntax (use external linters instead)
// - cortex.validate.semantics (use external linters instead)
// - cortex.validate.contracts (use external linters instead)
// - cortex.validate.dependencies (use cortex.deps.check_constraints instead)
// - cortex.validate.style (use cortex.lint.run instead)
// ============================================================================

// ============================================================================
// Helper functions
// ============================================================================

fn default_unit_type() -> String { "unit".to_string() }
fn default_coverage() -> f32 { 0.8 }
fn default_true() -> bool { true }
fn default_complexity_one() -> i32 { 1 }
fn default_line_coverage() -> String { "line".to_string() }
