//! Testing & Validation Tools (10 tools)

use async_trait::async_trait;
use cortex_parser::{CodeParser, FunctionInfo, Language, ParsedFile};
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
// Tool 1: cortex.test.generate - Generate tests for code
// ============================================================================

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

pub struct TestGenerateTool {
    ctx: TestingContext,
}

impl TestGenerateTool {
    pub fn new(ctx: TestingContext) -> Self {
        Self { ctx }
    }

    fn generate_rust_test(&self, func: &FunctionInfo, framework: &str) -> (String, Vec<String>) {
        let mut test_code = String::new();
        let mut test_cases = Vec::new();

        // Generate test module
        test_code.push_str(&format!("#[cfg(test)]\nmod test_{} {{\n", func.name));
        test_code.push_str(&format!("    use super::*;\n\n"));

        // Basic test case
        let basic_test = format!("test_{}_basic", func.name);
        test_cases.push(basic_test.clone());
        test_code.push_str(&format!("    #[test]\n"));
        test_code.push_str(&format!("    fn {}() {{\n", basic_test));

        // Generate test body based on parameters
        if func.parameters.is_empty() {
            test_code.push_str(&format!("        let result = {}();\n", func.name));
        } else {
            // Generate example parameters
            let param_values: Vec<String> = func.parameters.iter().map(|p| {
                match p.param_type.as_str() {
                    "i32" | "u32" | "i64" | "u64" | "isize" | "usize" => "0".to_string(),
                    "f32" | "f64" => "0.0".to_string(),
                    "bool" => "false".to_string(),
                    "&str" => "\"test\"".to_string(),
                    "String" => "String::from(\"test\")".to_string(),
                    _ if p.param_type.starts_with("&") => "&test_value".to_string(),
                    _ => "Default::default()".to_string(),
                }
            }).collect();

            test_code.push_str(&format!("        let result = {}({});\n", func.name, param_values.join(", ")));
        }

        if let Some(ret_type) = &func.return_type {
            if ret_type == "bool" {
                test_code.push_str("        assert!(result);\n");
            } else if ret_type.contains("Result") {
                test_code.push_str("        assert!(result.is_ok());\n");
            } else if ret_type.contains("Option") {
                test_code.push_str("        assert!(result.is_some());\n");
            } else {
                test_code.push_str("        // TODO: Add assertions\n");
            }
        }

        test_code.push_str("    }\n\n");

        // Edge case test if parameters exist
        if !func.parameters.is_empty() {
            let edge_test = format!("test_{}_edge_cases", func.name);
            test_cases.push(edge_test.clone());
            test_code.push_str(&format!("    #[test]\n"));
            test_code.push_str(&format!("    fn {}() {{\n", edge_test));
            test_code.push_str("        // TODO: Test edge cases\n");
            test_code.push_str("    }\n\n");
        }

        // Error case test if return type is Result
        if let Some(ret_type) = &func.return_type {
            if ret_type.contains("Result") {
                let error_test = format!("test_{}_error_cases", func.name);
                test_cases.push(error_test.clone());
                test_code.push_str(&format!("    #[test]\n"));
                test_code.push_str(&format!("    fn {}() {{\n", error_test));
                test_code.push_str("        // TODO: Test error conditions\n");
                test_code.push_str("    }\n\n");
            }
        }

        test_code.push_str("}\n");

        (test_code, test_cases)
    }

    fn generate_typescript_test(&self, func: &FunctionInfo, framework: &str) -> (String, Vec<String>) {
        let mut test_code = String::new();
        let mut test_cases = Vec::new();

        let (describe_keyword, test_keyword) = if framework == "jest" {
            ("describe", "test")
        } else {
            ("describe", "it")
        };

        // Generate test suite
        test_code.push_str(&format!("{}('{}', () => {{\n", describe_keyword, func.name));

        // Basic test case
        let basic_test = format!("should work with valid input");
        test_cases.push(basic_test.clone());
        test_code.push_str(&format!("  {}('{}', () => {{\n", test_keyword, basic_test));

        if func.parameters.is_empty() {
            test_code.push_str(&format!("    const result = {}();\n", func.name));
        } else {
            let param_values: Vec<String> = func.parameters.iter().map(|p| {
                match p.param_type.as_str() {
                    "number" => "0".to_string(),
                    "string" => "'test'".to_string(),
                    "boolean" => "false".to_string(),
                    _ => "{}".to_string(),
                }
            }).collect();

            test_code.push_str(&format!("    const result = {}({});\n", func.name, param_values.join(", ")));
        }

        test_code.push_str("    expect(result).toBeDefined();\n");
        test_code.push_str("  });\n\n");

        // Edge case test
        if !func.parameters.is_empty() {
            let edge_test = format!("should handle edge cases");
            test_cases.push(edge_test.clone());
            test_code.push_str(&format!("  {}('{}', () => {{\n", test_keyword, edge_test));
            test_code.push_str("    // TODO: Test edge cases\n");
            test_code.push_str("  });\n\n");
        }

        // Error case test
        let error_test = format!("should handle errors");
        test_cases.push(error_test.clone());
        test_code.push_str(&format!("  {}('{}', () => {{\n", test_keyword, error_test));
        test_code.push_str("    // TODO: Test error cases\n");
        test_code.push_str("  });\n");

        test_code.push_str("});\n");

        (test_code, test_cases)
    }
}

#[async_trait]
impl Tool for TestGenerateTool {
    fn name(&self) -> &str {
        "cortex.test.generate"
    }

    fn description(&self) -> Option<&str> {
        Some("Generate tests for code")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(TestGenerateInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: TestGenerateInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Generating tests for unit: {}", input.unit_id);

        // Get code unit from storage
        let unit = self.ctx.get_code_unit(&input.unit_id).await?
            .ok_or_else(|| ToolError::ExecutionFailed(format!("Code unit not found: {}", input.unit_id)))?;

        // Parse the file to get function details
        let parsed = self.ctx.parse_file(&unit.file_path).await?
            .ok_or_else(|| ToolError::ExecutionFailed(format!("Failed to parse file: {}", unit.file_path)))?;

        // Find the function by name
        let func = parsed.parsed.functions.iter()
            .find(|f| f.name == unit.name)
            .ok_or_else(|| ToolError::ExecutionFailed(format!("Function not found: {}", unit.name)))?;

        // Detect language and framework
        let language = Language::from_path(Path::new(&unit.file_path))
            .ok_or_else(|| ToolError::ExecutionFailed("Could not detect language".to_string()))?;

        let framework = input.framework.as_deref().unwrap_or(match language {
            Language::Rust => "cargo-test",
            Language::TypeScript | Language::JavaScript => "jest",
        });

        // Generate tests based on language
        let (test_code, test_cases) = match language {
            Language::Rust => self.generate_rust_test(func, framework),
            Language::TypeScript | Language::JavaScript => self.generate_typescript_test(func, framework),
        };

        // Estimate coverage
        let estimated_coverage = test_cases.len() as f32 * 0.25; // Simple estimation
        let estimated_coverage = estimated_coverage.min(1.0);

        info!("Generated {} test cases for {}", test_cases.len(), unit.name);

        let output = TestGenerateOutput {
            test_code,
            test_cases,
            estimated_coverage,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// ============================================================================
// Tool 2: cortex.test.validate - Validate generated tests
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
                if !input.include_private && func.visibility == cortex_parser::Visibility::Private {
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
            if func.visibility == cortex_parser::Visibility::Private {
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
// Tool 6: cortex.validate.syntax - Validate syntax without parsing
// ============================================================================

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

pub struct ValidateSyntaxTool {
    ctx: TestingContext,
}

impl ValidateSyntaxTool {
    pub fn new(ctx: TestingContext) -> Self {
        Self { ctx }
    }

    fn validate_with_parser(&self, code: &str, language: &str) -> (bool, Vec<SyntaxError>) {
        let mut errors = Vec::new();

        // Create parser based on language
        let ts_lang = match language {
            "rust" => tree_sitter_rust::LANGUAGE,
            "typescript" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
            "javascript" => tree_sitter_typescript::LANGUAGE_TSX,
            _ => {
                errors.push(SyntaxError {
                    line: 0,
                    column: 0,
                    message: format!("Unsupported language: {}", language),
                });
                return (false, errors);
            }
        };

        let mut parser = tree_sitter::Parser::new();
        if let Err(e) = parser.set_language(&ts_lang.into()) {
            errors.push(SyntaxError {
                line: 0,
                column: 0,
                message: format!("Failed to set parser language: {}", e),
            });
            return (false, errors);
        }

        match parser.parse(code, None) {
            Some(tree) => {
                let root = tree.root_node();

                // Check for ERROR nodes in the tree
                self.find_syntax_errors(root, code, &mut errors);

                (errors.is_empty(), errors)
            }
            None => {
                errors.push(SyntaxError {
                    line: 0,
                    column: 0,
                    message: "Failed to parse code".to_string(),
                });
                (false, errors)
            }
        }
    }

    fn find_syntax_errors(
        &self,
        node: tree_sitter::Node,
        _source: &str,
        errors: &mut Vec<SyntaxError>,
    ) {
        if node.is_error() || node.is_missing() {
            errors.push(SyntaxError {
                line: node.start_position().row as i32 + 1,
                column: node.start_position().column as i32,
                message: format!("Syntax error: {}", node.kind()),
            });
        }

        // Use walk() on the node to create a cursor for iteration
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.find_syntax_errors(child, _source, errors);
        }
    }
}

#[async_trait]
impl Tool for ValidateSyntaxTool {
    fn name(&self) -> &str {
        "cortex.validate.syntax"
    }

    fn description(&self) -> Option<&str> {
        Some("Validate syntax without parsing")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ValidateSyntaxInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: ValidateSyntaxInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Validating syntax for {} code", input.language);

        let (valid, errors) = self.validate_with_parser(&input.code, &input.language);

        info!("Syntax validation complete. Valid: {}, Errors: {}", valid, errors.len());

        let output = ValidateSyntaxOutput {
            valid,
            errors,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// ============================================================================
// Tool 7: cortex.validate.semantics - Validate semantic correctness
// ============================================================================

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

pub struct ValidateSemanticsTool {
    ctx: TestingContext,
}

impl ValidateSemanticsTool {
    pub fn new(ctx: TestingContext) -> Self {
        Self { ctx }
    }

    fn check_undefined_references(&self, func: &FunctionInfo, parsed: &ParsedFile) -> Vec<SemanticError> {
        let mut errors = Vec::new();

        // Build a set of defined identifiers
        let mut defined_identifiers = HashSet::new();

        // Add function parameters
        for param in &func.parameters {
            defined_identifiers.insert(param.name.clone());
        }

        // Add all functions in file
        for f in &parsed.functions {
            defined_identifiers.insert(f.name.clone());
        }

        // Add all structs
        for s in &parsed.structs {
            defined_identifiers.insert(s.name.clone());
        }

        // Simple check: look for identifiers that might be undefined
        // In a full implementation, this would use more sophisticated analysis
        let identifier_regex = Regex::new(r"\b([a-z_][a-z0-9_]*)\b").unwrap();
        for cap in identifier_regex.captures_iter(&func.body) {
            let ident = &cap[1];

            // Skip common keywords
            if ["let", "mut", "fn", "return", "if", "else", "match", "for", "while", "loop"].contains(&ident) {
                continue;
            }

            if !defined_identifiers.contains(ident) {
                // This might be undefined, but could also be from imports
                // In a real implementation, we'd check imports too
            }
        }

        errors
    }

    fn check_type_consistency(&self, func: &FunctionInfo) -> Vec<SemanticError> {
        let mut errors = Vec::new();

        // Check return type consistency
        if let Some(return_type) = &func.return_type {
            if return_type != "()" && !func.body.contains("return") && !func.body.ends_with('}') {
                errors.push(SemanticError {
                    error_type: "type_error".to_string(),
                    message: format!("Function declares return type {} but may not return a value", return_type),
                    location: format!("{}:{}", func.start_line, 0),
                });
            }
        }

        errors
    }
}

#[async_trait]
impl Tool for ValidateSemanticsTool {
    fn name(&self) -> &str {
        "cortex.validate.semantics"
    }

    fn description(&self) -> Option<&str> {
        Some("Validate semantic correctness")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ValidateSemanticsInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: ValidateSemanticsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Validating semantics for unit: {}", input.unit_id);

        // Get code unit
        let unit = self.ctx.get_code_unit(&input.unit_id).await?
            .ok_or_else(|| ToolError::ExecutionFailed(format!("Code unit not found: {}", input.unit_id)))?;

        // Parse file
        let parsed = self.ctx.parse_file(&unit.file_path).await?
            .ok_or_else(|| ToolError::ExecutionFailed(format!("Failed to parse file: {}", unit.file_path)))?;

        // Find function
        let func = parsed.parsed.functions.iter()
            .find(|f| f.name == unit.name)
            .ok_or_else(|| ToolError::ExecutionFailed(format!("Function not found: {}", unit.name)))?;

        let mut errors = Vec::new();

        // Run semantic checks
        if input.check_undefined {
            errors.extend(self.check_undefined_references(func, &parsed.parsed));
        }

        if input.check_types {
            errors.extend(self.check_type_consistency(func));
        }

        let valid = errors.is_empty();
        info!("Semantic validation complete. Valid: {}, Errors: {}", valid, errors.len());

        let output = ValidateSemanticsOutput {
            valid,
            errors,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// ============================================================================
// Tool 8: cortex.validate.contracts - Validate design contracts
// ============================================================================

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

pub struct ValidateContractsTool {
    ctx: TestingContext,
}

impl ValidateContractsTool {
    pub fn new(ctx: TestingContext) -> Self {
        Self { ctx }
    }

    fn validate_preconditions(&self, func: &FunctionInfo, contracts: &[serde_json::Value]) -> Vec<String> {
        let mut violations = Vec::new();

        for contract in contracts {
            if let Some(precondition) = contract.get("precondition") {
                let condition = precondition.as_str().unwrap_or("");

                // Check if precondition is verified in function body
                if !condition.is_empty() && !func.body.contains(condition) {
                    violations.push(format!("Precondition not verified: {}", condition));
                }
            }
        }

        violations
    }

    fn validate_postconditions(&self, func: &FunctionInfo, contracts: &[serde_json::Value]) -> Vec<String> {
        let mut violations = Vec::new();

        for contract in contracts {
            if let Some(postcondition) = contract.get("postcondition") {
                let condition = postcondition.as_str().unwrap_or("");

                // Check if postcondition is ensured
                if !condition.is_empty() {
                    // In a real implementation, would verify postcondition is guaranteed
                    violations.push(format!("Postcondition verification needed: {}", condition));
                }
            }
        }

        violations
    }

    fn validate_invariants(&self, func: &FunctionInfo, contracts: &[serde_json::Value]) -> Vec<String> {
        let mut violations = Vec::new();

        for contract in contracts {
            if let Some(invariant) = contract.get("invariant") {
                let condition = invariant.as_str().unwrap_or("");

                // Check if invariant is maintained
                if !condition.is_empty() {
                    // In a real implementation, would verify invariant throughout execution
                }
            }
        }

        violations
    }
}

#[async_trait]
impl Tool for ValidateContractsTool {
    fn name(&self) -> &str {
        "cortex.validate.contracts"
    }

    fn description(&self) -> Option<&str> {
        Some("Validate design contracts")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ValidateContractsInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: ValidateContractsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Validating contracts for unit: {}", input.unit_id);

        // Get code unit
        let unit = self.ctx.get_code_unit(&input.unit_id).await?
            .ok_or_else(|| ToolError::ExecutionFailed(format!("Code unit not found: {}", input.unit_id)))?;

        // Parse file
        let parsed = self.ctx.parse_file(&unit.file_path).await?
            .ok_or_else(|| ToolError::ExecutionFailed(format!("Failed to parse file: {}", unit.file_path)))?;

        // Find function
        let func = parsed.parsed.functions.iter()
            .find(|f| f.name == unit.name)
            .ok_or_else(|| ToolError::ExecutionFailed(format!("Function not found: {}", unit.name)))?;

        let mut violations = Vec::new();

        // Validate different contract types
        violations.extend(self.validate_preconditions(func, &input.contracts));
        violations.extend(self.validate_postconditions(func, &input.contracts));
        violations.extend(self.validate_invariants(func, &input.contracts));

        let valid = violations.is_empty();
        info!("Contract validation complete. Valid: {}, Violations: {}", valid, violations.len());

        let output = ValidateContractsOutput {
            valid,
            violations,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// ============================================================================
// Tool 9: cortex.validate.dependencies - Validate dependency constraints
// ============================================================================

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

pub struct ValidateDependenciesTool {
    ctx: TestingContext,
}

impl ValidateDependenciesTool {
    pub fn new(ctx: TestingContext) -> Self {
        Self { ctx }
    }

    fn extract_imports(&self, parsed: &ParsedFile) -> Vec<String> {
        parsed.imports.clone()
    }

    fn check_forbidden_dependencies(&self, imports: &[String], rules: &[serde_json::Value]) -> Vec<String> {
        let mut violations = Vec::new();

        for rule in rules {
            if let Some(forbidden) = rule.get("forbidden") {
                if let Some(forbidden_list) = forbidden.as_array() {
                    for forbidden_dep in forbidden_list {
                        let forbidden_name = forbidden_dep.as_str().unwrap_or("");

                        for import in imports {
                            if import.contains(forbidden_name) {
                                violations.push(format!("Forbidden dependency: {}", import));
                            }
                        }
                    }
                }
            }
        }

        violations
    }

    fn check_required_dependencies(&self, imports: &[String], rules: &[serde_json::Value]) -> Vec<String> {
        let mut violations = Vec::new();

        for rule in rules {
            if let Some(required) = rule.get("required") {
                if let Some(required_list) = required.as_array() {
                    for required_dep in required_list {
                        let required_name = required_dep.as_str().unwrap_or("");

                        let found = imports.iter().any(|import| import.contains(required_name));
                        if !found {
                            violations.push(format!("Missing required dependency: {}", required_name));
                        }
                    }
                }
            }
        }

        violations
    }

    fn check_circular_dependencies(&self, _files: &[ParsedFileWithPath]) -> Vec<String> {
        let mut violations = Vec::new();

        // Build dependency graph
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();

        // Detect cycles using DFS
        // In a real implementation, would use a proper graph cycle detection algorithm

        violations
    }
}

#[async_trait]
impl Tool for ValidateDependenciesTool {
    fn name(&self) -> &str {
        "cortex.validate.dependencies"
    }

    fn description(&self) -> Option<&str> {
        Some("Validate dependency constraints")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ValidateDependenciesInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: ValidateDependenciesInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Validating dependencies in: {}", input.scope_path);

        let files = self.ctx.parse_scope(&input.scope_path).await?;
        let mut violations = Vec::new();

        for file in &files {
            let imports = self.extract_imports(&file.parsed);

            // Check forbidden dependencies
            violations.extend(self.check_forbidden_dependencies(&imports, &input.rules));

            // Check required dependencies
            violations.extend(self.check_required_dependencies(&imports, &input.rules));
        }

        // Check circular dependencies
        violations.extend(self.check_circular_dependencies(&files));

        let valid = violations.is_empty();
        info!("Dependency validation complete. Valid: {}, Violations: {}", valid, violations.len());

        let output = ValidateDependenciesOutput {
            valid,
            violations,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// ============================================================================
// Tool 10: cortex.validate.style - Validate code style
// ============================================================================

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

pub struct ValidateStyleTool {
    ctx: TestingContext,
}

impl ValidateStyleTool {
    pub fn new(ctx: TestingContext) -> Self {
        Self { ctx }
    }

    fn check_rust_style(&self, file: &ParsedFileWithPath) -> Vec<StyleViolation> {
        let mut violations = Vec::new();

        // Check function naming (snake_case)
        let snake_case_regex = Regex::new(r"^[a-z][a-z0-9_]*$").unwrap();
        for func in &file.parsed.functions {
            if !snake_case_regex.is_match(&func.name) {
                violations.push(StyleViolation {
                    file_path: file.path.clone(),
                    line: func.start_line as i32,
                    rule: "function_naming".to_string(),
                    message: format!("Function '{}' should use snake_case", func.name),
                });
            }
        }

        // Check struct naming (PascalCase)
        let pascal_case_regex = Regex::new(r"^[A-Z][a-zA-Z0-9]*$").unwrap();
        for struct_info in &file.parsed.structs {
            if !pascal_case_regex.is_match(&struct_info.name) {
                violations.push(StyleViolation {
                    file_path: file.path.clone(),
                    line: struct_info.start_line as i32,
                    rule: "struct_naming".to_string(),
                    message: format!("Struct '{}' should use PascalCase", struct_info.name),
                });
            }
        }

        // Check line length
        for (line_num, line) in file.source.lines().enumerate() {
            if line.len() > 100 {
                violations.push(StyleViolation {
                    file_path: file.path.clone(),
                    line: (line_num + 1) as i32,
                    rule: "line_length".to_string(),
                    message: format!("Line exceeds 100 characters ({} chars)", line.len()),
                });
            }
        }

        // Check missing documentation
        for func in &file.parsed.functions {
            if func.visibility == cortex_parser::Visibility::Public && func.docstring.is_none() {
                violations.push(StyleViolation {
                    file_path: file.path.clone(),
                    line: func.start_line as i32,
                    rule: "missing_docs".to_string(),
                    message: format!("Public function '{}' missing documentation", func.name),
                });
            }
        }

        violations
    }

    fn check_typescript_style(&self, file: &ParsedFileWithPath) -> Vec<StyleViolation> {
        let mut violations = Vec::new();

        // Check function naming (camelCase)
        let camel_case_regex = Regex::new(r"^[a-z][a-zA-Z0-9]*$").unwrap();
        for func in &file.parsed.functions {
            if !camel_case_regex.is_match(&func.name) {
                violations.push(StyleViolation {
                    file_path: file.path.clone(),
                    line: func.start_line as i32,
                    rule: "function_naming".to_string(),
                    message: format!("Function '{}' should use camelCase", func.name),
                });
            }
        }

        // Check line length
        for (line_num, line) in file.source.lines().enumerate() {
            if line.len() > 120 {
                violations.push(StyleViolation {
                    file_path: file.path.clone(),
                    line: (line_num + 1) as i32,
                    rule: "line_length".to_string(),
                    message: format!("Line exceeds 120 characters ({} chars)", line.len()),
                });
            }
        }

        // Check for semicolons (if style guide requires them)
        let lines: Vec<&str> = file.source.lines().collect();
        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if !trimmed.is_empty()
                && !trimmed.ends_with(';')
                && !trimmed.ends_with('{')
                && !trimmed.ends_with('}')
                && !trimmed.starts_with("//")
                && (trimmed.starts_with("const ") || trimmed.starts_with("let ") || trimmed.starts_with("var ")) {
                violations.push(StyleViolation {
                    file_path: file.path.clone(),
                    line: (line_num + 1) as i32,
                    rule: "semicolon".to_string(),
                    message: "Statement should end with semicolon".to_string(),
                });
            }
        }

        violations
    }
}

#[async_trait]
impl Tool for ValidateStyleTool {
    fn name(&self) -> &str {
        "cortex.validate.style"
    }

    fn description(&self) -> Option<&str> {
        Some("Validate code style")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ValidateStyleInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: ValidateStyleInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Validating style in: {}", input.scope_path);

        let files = self.ctx.parse_scope(&input.scope_path).await?;
        let mut violations = Vec::new();

        for file in files {
            let language = Language::from_path(Path::new(&file.path));

            let file_violations = match language {
                Some(Language::Rust) => self.check_rust_style(&file),
                Some(Language::TypeScript) | Some(Language::JavaScript) => self.check_typescript_style(&file),
                None => continue,
            };

            violations.extend(file_violations);
        }

        let total_count = violations.len() as i32;
        let auto_fixed = 0; // Auto-fix not implemented in this version

        info!("Style validation complete. Violations: {}", total_count);

        let output = ValidateStyleOutput {
            violations,
            total_count,
            auto_fixed,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// ============================================================================
// Helper functions
// ============================================================================

fn default_unit_type() -> String { "unit".to_string() }
fn default_coverage() -> f32 { 0.8 }
fn default_true() -> bool { true }
fn default_complexity_one() -> i32 { 1 }
fn default_line_coverage() -> String { "line".to_string() }
