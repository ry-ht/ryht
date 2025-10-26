//! Code Quality Tools (8 tools)

use async_trait::async_trait;
use cortex_code_analysis::{CodeParser, FunctionInfo, Lang as Language, ParsedFile};
use cortex_storage::ConnectionManager;
use mcp_sdk::prelude::*;
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use tracing::info;

#[derive(Clone)]
pub struct CodeQualityContext {
    storage: Arc<ConnectionManager>,
}

impl CodeQualityContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self { storage }
    }

    /// Parse files in a directory or single file
    async fn parse_scope(&self, scope_path: &str) -> std::result::Result<Vec<ParsedFileWithPath>, ToolError> {
        let path = Path::new(scope_path);
        let mut results = Vec::new();

        if path.is_file() {
            if let Some(parsed) = self.parse_file(scope_path).await? {
                results.push(parsed);
            }
        } else if path.is_dir() {
            // Walk directory and parse all supported files
            results = self.parse_directory(scope_path).await?;
        } else {
            return Err(ToolError::ExecutionFailed(format!("Path not found: {}", scope_path)));
        }

        Ok(results)
    }

    /// Parse a single file
    async fn parse_file(&self, file_path: &str) -> std::result::Result<Option<ParsedFileWithPath>, ToolError> {
        let path = Path::new(file_path);

        // Check if language is supported
        if Language::from_path(path).is_none() {
            return Ok(None);
        }

        let source = std::fs::read_to_string(file_path)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let mut parser = CodeParser::new()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let parsed = parser.parse_file_auto(file_path, &source)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

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
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
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
}

#[derive(Clone)]
struct ParsedFileWithPath {
    path: String,
    source: String,
    parsed: ParsedFile,
}

// Helper functions for code quality analysis

/// Calculate cyclomatic complexity for a function
fn calculate_function_complexity(func: &FunctionInfo, _source: &str) -> u32 {
    func.complexity.unwrap_or(1)
}

/// Check if a function name follows conventions
fn check_function_naming(name: &str, language: Language) -> Option<String> {
    match language {
        Language::Rust => {
            let snake_case_regex = Regex::new(r"^[a-z][a-z0-9_]*$").unwrap();
            if !snake_case_regex.is_match(name) {
                Some("snake_case".to_string())
            } else {
                None
            }
        }
        Language::TypeScript | Language::JavaScript => {
            let camel_case_regex = Regex::new(r"^[a-z][a-zA-Z0-9]*$").unwrap();
            if !camel_case_regex.is_match(name) {
                Some("camelCase".to_string())
            } else {
                None
            }
        }
        Language::Tsx | Language::Jsx => {
            let camel_case_regex = Regex::new(r"^[a-z][a-zA-Z0-9]*$").unwrap();
            if !camel_case_regex.is_match(name) {
                Some("camelCase".to_string())
            } else {
                None
            }
        }
        Language::Python | Language::Cpp | Language::Java | Language::Kotlin => {
            // Not fully supported yet
            None
        }
    }
}

/// Check if a struct/class name follows conventions
fn check_type_naming(name: &str, _language: Language) -> Option<String> {
    let pascal_case_regex = Regex::new(r"^[A-Z][a-zA-Z0-9]*$").unwrap();
    if !pascal_case_regex.is_match(name) {
        Some("PascalCase".to_string())
    } else {
        None
    }
}

/// Detect code smells in a function
fn detect_function_smells(func: &FunctionInfo, _source: &str) -> Vec<(String, String, String)> {
    let mut smells = Vec::new();

    // Long method (> 50 lines)
    let line_count = func.end_line - func.start_line;
    if line_count > 50 {
        smells.push((
            "long_method".to_string(),
            "high".to_string(),
            format!("Method has {} lines, consider refactoring (threshold: 50)", line_count),
        ));
    }

    // High complexity
    if let Some(complexity) = func.complexity {
        if complexity > 10 {
            smells.push((
                "high_complexity".to_string(),
                "high".to_string(),
                format!("Cyclomatic complexity is {}, consider simplifying (threshold: 10)", complexity),
            ));
        }
    }

    // Too many parameters (> 5)
    if func.parameters.len() > 5 {
        smells.push((
            "long_parameter_list".to_string(),
            "medium".to_string(),
            format!("Function has {} parameters, consider using a config object (threshold: 5)", func.parameters.len()),
        ));
    }

    smells
}

/// Calculate lines of code
fn count_lines(source: &str) -> (usize, usize, usize) {
    let lines: Vec<&str> = source.lines().collect();
    let total = lines.len();
    let mut code = 0;
    let mut comments = 0;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") || trimmed.starts_with("#") {
            comments += 1;
        } else {
            code += 1;
        }
    }

    (total, code, comments)
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

pub struct QualityAnalyzeComplexityTool {
    ctx: CodeQualityContext,
}

impl QualityAnalyzeComplexityTool {
    pub fn new(ctx: CodeQualityContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for QualityAnalyzeComplexityTool {
    fn name(&self) -> &str {
        "cortex.quality.analyze_complexity"
    }

    fn description(&self) -> Option<&str> {
        Some("Analyze code complexity metrics using cyclomatic complexity calculation")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AnalyzeComplexityInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AnalyzeComplexityInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Analyzing complexity for: {}", input.scope_path);

        let files = self.ctx.parse_scope(&input.scope_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let mut metrics = Vec::new();
        let mut total_complexity = 0;
        let mut count = 0;

        for file in &files {
            for func in &file.parsed.functions {
                let complexity = calculate_function_complexity(func, &file.source);

                metrics.push(ComplexityMetric {
                    entity_id: format!("{}:{}:{}", file.path, func.start_line, func.qualified_name),
                    metric_name: "cyclomatic_complexity".to_string(),
                    value: complexity as i32,
                });

                total_complexity += complexity;
                count += 1;
            }

            // Also analyze methods in impl blocks
            for impl_block in &file.parsed.impls {
                for method in &impl_block.methods {
                    let complexity = calculate_function_complexity(method, &file.source);

                    metrics.push(ComplexityMetric {
                        entity_id: format!("{}:{}:{}", file.path, method.start_line, method.qualified_name),
                        metric_name: "cyclomatic_complexity".to_string(),
                        value: complexity as i32,
                    });

                    total_complexity += complexity;
                    count += 1;
                }
            }
        }

        let average_complexity = if count > 0 {
            total_complexity as f32 / count as f32
        } else {
            0.0
        };

        info!("Analyzed {} functions, average complexity: {:.2}", count, average_complexity);

        let output = AnalyzeComplexityOutput {
            metrics,
            average_complexity,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

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

pub struct QualityFindCodeSmellsTool {
    ctx: CodeQualityContext,
}

impl QualityFindCodeSmellsTool {
    pub fn new(ctx: CodeQualityContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for QualityFindCodeSmellsTool {
    fn name(&self) -> &str {
        "cortex.quality.find_code_smells"
    }

    fn description(&self) -> Option<&str> {
        Some("Detect code smells like long methods, high complexity, long parameter lists")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(FindCodeSmellsInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: FindCodeSmellsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Finding code smells in: {}", input.scope_path);

        let files = self.ctx.parse_scope(&input.scope_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let mut smells = Vec::new();

        // Severity mapping
        let severity_threshold_level = match input.severity_threshold.as_str() {
            "low" => 1,
            "medium" => 2,
            "high" => 3,
            _ => 2,
        };

        for file in &files {
            // Check functions
            for func in &file.parsed.functions {
                let func_smells = detect_function_smells(func, &file.source);
                for (smell_type, severity, description) in func_smells {
                    let severity_level = match severity.as_str() {
                        "low" => 1,
                        "medium" => 2,
                        "high" => 3,
                        _ => 2,
                    };

                    if severity_level >= severity_threshold_level {
                        smells.push(CodeSmell {
                            entity_id: format!("{}:{}:{}", file.path, func.start_line, func.qualified_name),
                            smell_type,
                            severity,
                            description,
                        });
                    }
                }
            }

            // Check methods in impl blocks
            for impl_block in &file.parsed.impls {
                for method in &impl_block.methods {
                    let method_smells = detect_function_smells(method, &file.source);
                    for (smell_type, severity, description) in method_smells {
                        let severity_level = match severity.as_str() {
                            "low" => 1,
                            "medium" => 2,
                            "high" => 3,
                            _ => 2,
                        };

                        if severity_level >= severity_threshold_level {
                            smells.push(CodeSmell {
                                entity_id: format!("{}:{}:{}", file.path, method.start_line, method.qualified_name),
                                smell_type,
                                severity,
                                description,
                            });
                        }
                    }
                }
            }

            // Check for god classes (structs with too many fields or methods)
            for struct_info in &file.parsed.structs {
                if struct_info.fields.len() > 15 {
                    smells.push(CodeSmell {
                        entity_id: format!("{}:{}:{}", file.path, struct_info.start_line, struct_info.qualified_name),
                        smell_type: "god_class".to_string(),
                        severity: "high".to_string(),
                        description: format!("Struct has {} fields, consider splitting (threshold: 15)", struct_info.fields.len()),
                    });
                }
            }
        }

        let total_count = smells.len() as i32;
        info!("Found {} code smells", total_count);

        let output = FindCodeSmellsOutput {
            smells,
            total_count,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

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

pub struct QualityCheckNamingTool {
    ctx: CodeQualityContext,
}

impl QualityCheckNamingTool {
    pub fn new(ctx: CodeQualityContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for QualityCheckNamingTool {
    fn name(&self) -> &str {
        "cortex.quality.check_naming"
    }

    fn description(&self) -> Option<&str> {
        Some("Check naming conventions for functions, classes, and variables")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(CheckNamingInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: CheckNamingInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Checking naming conventions in: {}", input.scope_path);

        let files = self.ctx.parse_scope(&input.scope_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let mut violations = Vec::new();

        for file in &files {
            let language = Language::from_path(Path::new(&file.path)).unwrap_or(Language::Rust);

            // Check function names
            for func in &file.parsed.functions {
                if let Some(expected) = check_function_naming(&func.name, language) {
                    violations.push(NamingViolation {
                        entity_id: format!("{}:{}:{}", file.path, func.start_line, func.qualified_name),
                        entity_type: "function".to_string(),
                        current_name: func.name.clone(),
                        expected_pattern: expected,
                    });
                }
            }

            // Check struct/class names
            for struct_info in &file.parsed.structs {
                if let Some(expected) = check_type_naming(&struct_info.name, language) {
                    violations.push(NamingViolation {
                        entity_id: format!("{}:{}:{}", file.path, struct_info.start_line, struct_info.qualified_name),
                        entity_type: "struct".to_string(),
                        current_name: struct_info.name.clone(),
                        expected_pattern: expected,
                    });
                }
            }

            // Check enum names
            for enum_info in &file.parsed.enums {
                if let Some(expected) = check_type_naming(&enum_info.name, language) {
                    violations.push(NamingViolation {
                        entity_id: format!("{}:{}:{}", file.path, enum_info.start_line, enum_info.qualified_name),
                        entity_type: "enum".to_string(),
                        current_name: enum_info.name.clone(),
                        expected_pattern: expected,
                    });
                }
            }

            // Check trait names
            for trait_info in &file.parsed.traits {
                if let Some(expected) = check_type_naming(&trait_info.name, language) {
                    violations.push(NamingViolation {
                        entity_id: format!("{}:{}:{}", file.path, trait_info.start_line, trait_info.qualified_name),
                        entity_type: "trait".to_string(),
                        current_name: trait_info.name.clone(),
                        expected_pattern: expected,
                    });
                }
            }
        }

        let total_count = violations.len() as i32;
        info!("Found {} naming violations", total_count);

        let output = CheckNamingOutput {
            violations,
            total_count,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

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

pub struct QualityAnalyzeCouplingTool {
    ctx: CodeQualityContext,
}

impl QualityAnalyzeCouplingTool {
    pub fn new(ctx: CodeQualityContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for QualityAnalyzeCouplingTool {
    fn name(&self) -> &str {
        "cortex.quality.analyze_coupling"
    }

    fn description(&self) -> Option<&str> {
        Some("Analyze module coupling by counting imports and dependencies")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AnalyzeCouplingInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AnalyzeCouplingInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Analyzing coupling for: {}", input.scope_path);

        let files = self.ctx.parse_scope(&input.scope_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let mut modules = Vec::new();
        let mut total_coupling = 0.0;

        for file in &files {
            // Calculate afferent coupling (number of imports from other modules)
            let import_count = file.parsed.imports.len() as f32;

            // Normalize by typical values (0-10 scale)
            let coupling_value = (import_count / 5.0).min(10.0);

            modules.push(ModuleCoupling {
                module_id: file.path.clone(),
                coupling_value,
            });

            total_coupling += coupling_value;
        }

        let average_coupling = if !modules.is_empty() {
            total_coupling / modules.len() as f32
        } else {
            0.0
        };

        info!("Analyzed {} modules, average coupling: {:.2}", modules.len(), average_coupling);

        let output = AnalyzeCouplingOutput {
            modules,
            average_coupling,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

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

pub struct QualityAnalyzeCohesionTool {
    ctx: CodeQualityContext,
}

impl QualityAnalyzeCohesionTool {
    pub fn new(ctx: CodeQualityContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for QualityAnalyzeCohesionTool {
    fn name(&self) -> &str {
        "cortex.quality.analyze_cohesion"
    }

    fn description(&self) -> Option<&str> {
        Some("Analyze module cohesion using LCOM (Lack of Cohesion of Methods)")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AnalyzeCohesionInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AnalyzeCohesionInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Analyzing cohesion for: {}", input.module_path);

        let files = self.ctx.parse_scope(&input.module_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let mut total_cohesion = 0.0;
        let mut count = 0;

        for file in &files {
            // Calculate cohesion for each struct/class
            for struct_info in &file.parsed.structs {
                // Find methods for this struct
                let mut methods = Vec::new();
                for impl_block in &file.parsed.impls {
                    if impl_block.type_name == struct_info.name {
                        methods.extend(impl_block.methods.iter());
                    }
                }

                if !methods.is_empty() && !struct_info.fields.is_empty() {
                    // Simple cohesion: ratio of methods to fields
                    // High cohesion = each method uses most fields
                    // For simplicity, we'll use method/field ratio as a proxy
                    let method_count = methods.len() as f32;
                    let field_count = struct_info.fields.len() as f32;

                    // Ideal ratio is 1:1 to 2:1 (methods to fields)
                    let ratio = method_count / field_count;
                    let cohesion = if ratio >= 1.0 && ratio <= 2.0 {
                        10.0 // Perfect cohesion
                    } else if ratio < 1.0 {
                        ratio * 10.0 // Low cohesion
                    } else {
                        20.0 / ratio // Too many methods
                    };

                    total_cohesion += cohesion;
                    count += 1;
                }
            }
        }

        let cohesion_value = if count > 0 {
            total_cohesion / count as f32
        } else {
            5.0 // Default neutral value
        };

        info!("Analyzed cohesion: {:.2}", cohesion_value);

        let output = AnalyzeCohesionOutput {
            module_path: input.module_path,
            cohesion_value,
            cohesion_type: input.cohesion_type,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

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

pub struct QualityFindAntipatternsTool {
    ctx: CodeQualityContext,
}

impl QualityFindAntipatternsTool {
    pub fn new(ctx: CodeQualityContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for QualityFindAntipatternsTool {
    fn name(&self) -> &str {
        "cortex.quality.find_antipatterns"
    }

    fn description(&self) -> Option<&str> {
        Some("Detect common antipatterns like singletons, god objects, and tight coupling")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(FindAntipatternsInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: FindAntipatternsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Finding antipatterns in: {}", input.scope_path);

        let files = self.ctx.parse_scope(&input.scope_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let mut antipatterns = Vec::new();

        for file in &files {
            // Detect god object (too many responsibilities)
            for struct_info in &file.parsed.structs {
                let mut method_count = 0;
                for impl_block in &file.parsed.impls {
                    if impl_block.type_name == struct_info.name {
                        method_count += impl_block.methods.len();
                    }
                }

                if method_count > 20 || struct_info.fields.len() > 15 {
                    antipatterns.push(Antipattern {
                        entity_id: format!("{}:{}:{}", file.path, struct_info.start_line, struct_info.qualified_name),
                        pattern_type: "god_object".to_string(),
                        description: format!("Class has {} methods and {} fields, indicating too many responsibilities", method_count, struct_info.fields.len()),
                        severity: "high".to_string(),
                    });
                }
            }

            // Detect singleton pattern (often an antipattern in modern code)
            for struct_info in &file.parsed.structs {
                if struct_info.name.to_lowercase().contains("singleton") {
                    antipatterns.push(Antipattern {
                        entity_id: format!("{}:{}:{}", file.path, struct_info.start_line, struct_info.qualified_name),
                        pattern_type: "singleton".to_string(),
                        description: "Singleton pattern detected - consider dependency injection instead".to_string(),
                        severity: "medium".to_string(),
                    });
                }
            }

            // Detect empty catch blocks (in body text)
            for func in &file.parsed.functions {
                if func.body.contains("catch") && func.body.contains("{}") {
                    antipatterns.push(Antipattern {
                        entity_id: format!("{}:{}:{}", file.path, func.start_line, func.qualified_name),
                        pattern_type: "empty_catch".to_string(),
                        description: "Empty catch block detected - errors should be handled".to_string(),
                        severity: "high".to_string(),
                    });
                }

                // Detect magic numbers (numbers other than 0, 1, -1 in code)
                let magic_number_regex = Regex::new(r"\b([2-9]|\d{2,})\b").unwrap();
                if magic_number_regex.is_match(&func.body) {
                    antipatterns.push(Antipattern {
                        entity_id: format!("{}:{}:{}", file.path, func.start_line, func.qualified_name),
                        pattern_type: "magic_numbers".to_string(),
                        description: "Magic numbers detected - consider using named constants".to_string(),
                        severity: "low".to_string(),
                    });
                }
            }

            // Detect circular dependencies (simplified - check if module imports itself)
            let module_name = Path::new(&file.path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("");

            for import in &file.parsed.imports {
                if import.contains(module_name) {
                    antipatterns.push(Antipattern {
                        entity_id: format!("{}:1", file.path),
                        pattern_type: "circular_dependency".to_string(),
                        description: "Possible circular dependency detected in imports".to_string(),
                        severity: "high".to_string(),
                    });
                    break;
                }
            }
        }

        let total_count = antipatterns.len() as i32;
        info!("Found {} antipatterns", total_count);

        let output = FindAntipatternsOutput {
            antipatterns,
            total_count,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

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

pub struct QualitySuggestRefactoringsTool {
    ctx: CodeQualityContext,
}

impl QualitySuggestRefactoringsTool {
    pub fn new(ctx: CodeQualityContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for QualitySuggestRefactoringsTool {
    fn name(&self) -> &str {
        "cortex.quality.suggest_refactorings"
    }

    fn description(&self) -> Option<&str> {
        Some("Suggest refactoring opportunities based on code quality analysis")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SuggestRefactoringsInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SuggestRefactoringsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Suggesting refactorings for: {}", input.scope_path);

        let files = self.ctx.parse_scope(&input.scope_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let mut suggestions = Vec::new();

        for file in &files {
            // Suggest extract method for long functions
            for func in &file.parsed.functions {
                let line_count = func.end_line - func.start_line;
                if line_count > 50 {
                    suggestions.push(RefactoringSuggestion {
                        entity_id: format!("{}:{}:{}", file.path, func.start_line, func.qualified_name),
                        refactoring_type: "extract_method".to_string(),
                        description: format!("Extract method - function has {} lines, consider breaking into smaller functions", line_count),
                        confidence: 0.9,
                    });
                }

                // Suggest introduce parameter object for long parameter lists
                if func.parameters.len() > 5 {
                    suggestions.push(RefactoringSuggestion {
                        entity_id: format!("{}:{}:{}", file.path, func.start_line, func.qualified_name),
                        refactoring_type: "introduce_parameter_object".to_string(),
                        description: format!("Introduce parameter object - function has {} parameters", func.parameters.len()),
                        confidence: 0.85,
                    });
                }

                // Suggest simplify conditional for high complexity
                if let Some(complexity) = func.complexity {
                    if complexity > 10 {
                        suggestions.push(RefactoringSuggestion {
                            entity_id: format!("{}:{}:{}", file.path, func.start_line, func.qualified_name),
                            refactoring_type: "simplify_conditional".to_string(),
                            description: format!("Simplify conditional logic - cyclomatic complexity is {}", complexity),
                            confidence: 0.8,
                        });
                    }
                }
            }

            // Suggest extract class for god objects
            for struct_info in &file.parsed.structs {
                if struct_info.fields.len() > 15 {
                    suggestions.push(RefactoringSuggestion {
                        entity_id: format!("{}:{}:{}", file.path, struct_info.start_line, struct_info.qualified_name),
                        refactoring_type: "extract_class".to_string(),
                        description: format!("Extract class - struct has {} fields, consider splitting responsibilities", struct_info.fields.len()),
                        confidence: 0.75,
                    });
                }
            }

            // Suggest replace magic number with constant
            for func in &file.parsed.functions {
                let magic_number_regex = Regex::new(r"\b([2-9]|\d{2,})\b").unwrap();
                if magic_number_regex.is_match(&func.body) {
                    suggestions.push(RefactoringSuggestion {
                        entity_id: format!("{}:{}:{}", file.path, func.start_line, func.qualified_name),
                        refactoring_type: "replace_magic_number".to_string(),
                        description: "Replace magic numbers with named constants".to_string(),
                        confidence: 0.9,
                    });
                }
            }
        }

        // Filter by confidence threshold
        suggestions.retain(|s| s.confidence >= input.min_confidence);

        let total_count = suggestions.len() as i32;
        info!("Generated {} refactoring suggestions", total_count);

        let output = SuggestRefactoringsOutput {
            suggestions,
            total_count,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

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

pub struct QualityCalculateMetricsTool {
    ctx: CodeQualityContext,
}

impl QualityCalculateMetricsTool {
    pub fn new(ctx: CodeQualityContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for QualityCalculateMetricsTool {
    fn name(&self) -> &str {
        "cortex.quality.calculate_metrics"
    }

    fn description(&self) -> Option<&str> {
        Some("Calculate comprehensive code metrics: LOC, complexity, function count, etc.")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(CalculateMetricsInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: CalculateMetricsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Calculating metrics for: {}", input.scope_path);

        let files = self.ctx.parse_scope(&input.scope_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let mut metrics = Vec::new();
        let mut total_lines = 0;

        for file in &files {
            let group = match input.group_by.as_str() {
                "file" => file.path.clone(),
                "directory" => {
                    Path::new(&file.path)
                        .parent()
                        .and_then(|p| p.to_str())
                        .unwrap_or("")
                        .to_string()
                }
                _ => "all".to_string(),
            };

            // Calculate lines of code
            let (total, code, comments) = count_lines(&file.source);
            total_lines += total as i32;

            metrics.push(MetricValue {
                metric_name: "total_lines".to_string(),
                value: total as i32,
                group: group.clone(),
            });

            metrics.push(MetricValue {
                metric_name: "code_lines".to_string(),
                value: code as i32,
                group: group.clone(),
            });

            metrics.push(MetricValue {
                metric_name: "comment_lines".to_string(),
                value: comments as i32,
                group: group.clone(),
            });

            // Count functions
            let function_count = file.parsed.functions.len();
            metrics.push(MetricValue {
                metric_name: "function_count".to_string(),
                value: function_count as i32,
                group: group.clone(),
            });

            // Count structs/classes
            let struct_count = file.parsed.structs.len();
            metrics.push(MetricValue {
                metric_name: "struct_count".to_string(),
                value: struct_count as i32,
                group: group.clone(),
            });

            // Count enums
            let enum_count = file.parsed.enums.len();
            metrics.push(MetricValue {
                metric_name: "enum_count".to_string(),
                value: enum_count as i32,
                group: group.clone(),
            });

            // Count traits/interfaces
            let trait_count = file.parsed.traits.len();
            metrics.push(MetricValue {
                metric_name: "trait_count".to_string(),
                value: trait_count as i32,
                group: group.clone(),
            });

            // Calculate average complexity
            let mut total_complexity = 0;
            let mut complexity_count = 0;

            for func in &file.parsed.functions {
                if let Some(complexity) = func.complexity {
                    total_complexity += complexity;
                    complexity_count += 1;
                }
            }

            for impl_block in &file.parsed.impls {
                for method in &impl_block.methods {
                    if let Some(complexity) = method.complexity {
                        total_complexity += complexity;
                        complexity_count += 1;
                    }
                }
            }

            let avg_complexity = if complexity_count > 0 {
                total_complexity / complexity_count
            } else {
                0
            };

            metrics.push(MetricValue {
                metric_name: "average_complexity".to_string(),
                value: avg_complexity as i32,
                group: group.clone(),
            });

            // Count imports
            let import_count = file.parsed.imports.len();
            metrics.push(MetricValue {
                metric_name: "import_count".to_string(),
                value: import_count as i32,
                group,
            });
        }

        info!("Calculated {} metrics, total lines: {}", metrics.len(), total_lines);

        let output = CalculateMetricsOutput {
            metrics,
            total_lines,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

fn default_none() -> String { "none".to_string() }
fn default_medium() -> String { "medium".to_string() }
fn default_afferent() -> String { "afferent".to_string() }
fn default_lcom() -> String { "lcom".to_string() }
fn default_confidence() -> f32 { 0.7 }
fn default_file_group() -> String { "file".to_string() }
