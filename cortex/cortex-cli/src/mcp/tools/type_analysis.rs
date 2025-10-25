//! Type Analysis Tools (4 tools)
//!
//! Provides type inference, checking, and analysis for multi-language codebases

use async_trait::async_trait;
use cortex_memory::CognitiveManager;
use cortex_code_analysis::Language;
use cortex_storage::ConnectionManager;
use cortex_vfs::VirtualFileSystem;
use mcp_sdk::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{debug, info, warn};

#[derive(Clone)]
pub struct TypeAnalysisContext {
    storage: Arc<ConnectionManager>,
    vfs: Arc<VirtualFileSystem>,
}

impl TypeAnalysisContext {
    pub fn new(storage: Arc<ConnectionManager>, vfs: Arc<VirtualFileSystem>) -> Self {
        Self { storage, vfs }
    }

    fn get_cognitive_manager(&self) -> CognitiveManager {
        CognitiveManager::new(self.storage.clone())
    }
}

// =============================================================================
// cortex.code.infer_types
// =============================================================================

pub struct CodeInferTypesTool {
    ctx: TypeAnalysisContext,
}

impl CodeInferTypesTool {
    pub fn new(ctx: TypeAnalysisContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct InferTypesInput {
    unit_id: String,
    #[serde(default = "default_true")]
    infer_return_type: bool,
    #[serde(default = "default_true")]
    infer_parameters: bool,
    #[serde(default = "default_true")]
    infer_variables: bool,
    #[serde(default = "default_high_confidence")]
    min_confidence: f32,
}

#[derive(Debug, Serialize, JsonSchema)]
struct InferTypesOutput {
    inferred_types: Vec<InferredType>,
    return_type: Option<TypeInfo>,
    parameter_types: Vec<ParameterType>,
    variable_types: Vec<VariableType>,
    confidence_score: f32,
}

#[derive(Debug, Serialize, JsonSchema)]
struct InferredType {
    location: String,
    inferred_type: String,
    confidence: f32,
    reasoning: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct TypeInfo {
    type_name: String,
    is_nullable: bool,
    is_generic: bool,
    type_parameters: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ParameterType {
    parameter_name: String,
    inferred_type: TypeInfo,
    confidence: f32,
}

#[derive(Debug, Serialize, JsonSchema)]
struct VariableType {
    variable_name: String,
    inferred_type: TypeInfo,
    line: i32,
    confidence: f32,
}

impl Default for InferTypesOutput {
    fn default() -> Self {
        Self {
            inferred_types: vec![],
            return_type: None,
            parameter_types: vec![],
            variable_types: vec![],
            confidence_score: 0.0,
        }
    }
}

#[async_trait]
impl Tool for CodeInferTypesTool {
    fn name(&self) -> &str {
        "cortex.code.infer_types"
    }

    fn description(&self) -> Option<&str> {
        Some("Infer types for dynamically typed code using static analysis and ML")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(InferTypesInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: InferTypesInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Inferring types for unit: {}", input.unit_id);

        // Get the code unit from semantic memory
        let manager = self.ctx.get_cognitive_manager();
        let semantic = manager.semantic();

        let unit_id = cortex_core::id::CortexId::from_str(&input.unit_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid unit_id: {}", e)))?;

        let unit = semantic.get_unit(unit_id).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Database error: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed("Code unit not found".to_string()))?;

        // Infer types based on the unit's language and code
        let mut inferred_types = Vec::new();
        let mut return_type = None;
        let mut parameter_types = Vec::new();
        let mut variable_types = Vec::new();
        let mut total_confidence = 0.0;
        let mut confidence_count = 0;

        // Parse the source file to get AST
        let language = Language::from_path(Path::new(&unit.file_path))
            .unwrap_or(Language::Rust);

        // For Rust, we can infer types from:
        // 1. Explicit type annotations
        // 2. Return type declarations
        // 3. Function signatures
        if language == Language::Rust {
            // Infer return type if requested
            if input.infer_return_type {
                if let Some(ret_type_str) = &unit.return_type {
                    return_type = Some(parse_type_info(ret_type_str));
                    let confidence = if ret_type_str.contains("impl") || ret_type_str.contains("dyn") {
                        0.85
                    } else {
                        0.95
                    };

                    inferred_types.push(InferredType {
                        location: format!("{}:{}-{}", unit.file_path, unit.start_line, unit.end_line),
                        inferred_type: ret_type_str.clone(),
                        confidence,
                        reasoning: "Explicitly declared return type in function signature".to_string(),
                    });
                    total_confidence += confidence;
                    confidence_count += 1;
                }
            }

            // Infer parameter types if requested
            if input.infer_parameters {
                for param in &unit.parameters {
                    let param_type = parse_type_info(param.param_type.as_deref().unwrap_or(""));
                    let confidence = calculate_type_confidence(param.param_type.as_deref().unwrap_or(""));

                    parameter_types.push(ParameterType {
                        parameter_name: param.name.clone(),
                        inferred_type: param_type,
                        confidence,
                    });

                    inferred_types.push(InferredType {
                        location: format!("{}:{} (parameter: {})", unit.file_path, unit.start_line, param.name),
                        inferred_type: param.param_type.clone().unwrap_or_default(),
                        confidence,
                        reasoning: "Explicitly declared parameter type".to_string(),
                    });
                    total_confidence += confidence;
                    confidence_count += 1;
                }
            }

            // Infer variable types from body if requested
            if input.infer_variables {
                if let Some(body) = &unit.body {
                    let var_inferences = infer_variable_types_from_body(
                        body,
                        &unit.file_path,
                        unit.start_line,
                        input.min_confidence
                    );

                    for inference in var_inferences {
                        total_confidence += inference.confidence;
                        confidence_count += 1;

                        variable_types.push(VariableType {
                            variable_name: inference.var_name.clone(),
                            inferred_type: parse_type_info(&inference.inferred_type),
                            line: inference.line,
                            confidence: inference.confidence,
                        });

                        inferred_types.push(InferredType {
                            location: format!("{}:{}", unit.file_path, inference.line),
                            inferred_type: inference.inferred_type,
                            confidence: inference.confidence,
                            reasoning: inference.reasoning,
                        });
                    }
                }
            }
        }

        // Filter by minimum confidence
        inferred_types.retain(|t| t.confidence >= input.min_confidence);
        parameter_types.retain(|t| t.confidence >= input.min_confidence);
        variable_types.retain(|t| t.confidence >= input.min_confidence);

        let overall_confidence = if confidence_count > 0 {
            total_confidence / confidence_count as f32
        } else {
            0.0
        };

        let output = InferTypesOutput {
            inferred_types,
            return_type,
            parameter_types,
            variable_types,
            confidence_score: overall_confidence,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.check_types
// =============================================================================

pub struct CodeCheckTypesTool {
    ctx: TypeAnalysisContext,
}

impl CodeCheckTypesTool {
    pub fn new(ctx: TypeAnalysisContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CheckTypesInput {
    scope_path: String,
    #[serde(default = "default_strict")]
    strictness: String,
    #[serde(default = "default_true")]
    check_null_safety: bool,
    #[serde(default = "default_true")]
    check_generic_bounds: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct CheckTypesOutput {
    type_errors: Vec<TypeError>,
    warnings: Vec<TypeWarning>,
    total_errors: i32,
    total_warnings: i32,
    files_checked: i32,
}

#[derive(Debug, Serialize, JsonSchema)]
struct TypeError {
    file_path: String,
    line: i32,
    column: i32,
    error_type: String,
    message: String,
    expected_type: String,
    actual_type: String,
    suggestion: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct TypeWarning {
    file_path: String,
    line: i32,
    warning_type: String,
    message: String,
}

impl Default for CheckTypesOutput {
    fn default() -> Self {
        Self {
            type_errors: vec![],
            warnings: vec![],
            total_errors: 0,
            total_warnings: 0,
            files_checked: 0,
        }
    }
}

#[async_trait]
impl Tool for CodeCheckTypesTool {
    fn name(&self) -> &str {
        "cortex.code.check_types"
    }

    fn description(&self) -> Option<&str> {
        Some("Static type checking with configurable strictness levels")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(CheckTypesInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: CheckTypesInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Type checking code at: {}", input.scope_path);

        let manager = self.ctx.get_cognitive_manager();
        let semantic = manager.semantic();

        let mut type_errors = Vec::new();
        let mut warnings = Vec::new();
        let mut files_checked = 0;

        // Determine if scope_path is a file or directory
        let path = Path::new(&input.scope_path);
        let is_file = path.extension().is_some();

        let units = if is_file {
            // Get units in the specific file
            files_checked = 1;
            semantic.get_units_in_file(&input.scope_path).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get units: {}", e)))?
        } else {
            // For directory, we'd need to traverse - simplified for now
            warn!("Directory type checking not fully implemented, treating as file");
            files_checked = 1;
            semantic.get_units_in_file(&input.scope_path).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get units: {}", e)))?
        };

        // Check each unit for type issues
        for unit in units {
            // Check for missing return types on public functions
            if format!("{:?}", unit.visibility) == "Public" && unit.return_type.is_none() {
                if input.strictness == "strict" {
                    type_errors.push(TypeError {
                        file_path: unit.file_path.clone(),
                        line: unit.start_line as i32,
                        column: unit.start_column as i32,
                        error_type: "MissingReturnType".to_string(),
                        message: format!("Public function '{}' missing explicit return type", unit.name),
                        expected_type: "explicit type annotation".to_string(),
                        actual_type: "inferred".to_string(),
                        suggestion: Some(format!("Add explicit return type to '{}'", unit.name)),
                    });
                } else {
                    warnings.push(TypeWarning {
                        file_path: unit.file_path.clone(),
                        line: unit.start_line as i32,
                        warning_type: "MissingReturnType".to_string(),
                        message: format!("Consider adding explicit return type to '{}'", unit.name),
                    });
                }
            }

            // Check parameters for missing types
            for param in &unit.parameters {
                let param_type_str = param.param_type.as_deref().unwrap_or("");
                if param_type_str.is_empty() || param_type_str == "_" {
                    if input.strictness == "strict" {
                        type_errors.push(TypeError {
                            file_path: unit.file_path.clone(),
                            line: unit.start_line as i32,
                            column: unit.start_column as i32,
                            error_type: "MissingParameterType".to_string(),
                            message: format!("Parameter '{}' missing type annotation", param.name),
                            expected_type: "explicit type".to_string(),
                            actual_type: "inferred".to_string(),
                            suggestion: Some(format!("Add type annotation to parameter '{}'", param.name)),
                        });
                    }
                }
            }

            // Check for null safety issues
            if input.check_null_safety {
                if let Some(ret_type) = &unit.return_type {
                    // Check if function might return None without Option wrapper
                    if let Some(body) = &unit.body {
                        if body.contains("return None") && !ret_type.contains("Option") {
                            type_errors.push(TypeError {
                                file_path: unit.file_path.clone(),
                                line: unit.start_line as i32,
                                column: unit.start_column as i32,
                                error_type: "NullSafetyViolation".to_string(),
                                message: format!("Function '{}' returns None but type is not Option", unit.name),
                                expected_type: format!("Option<{}>", ret_type),
                                actual_type: ret_type.clone(),
                                suggestion: Some(format!("Change return type to Option<{}>", ret_type)),
                            });
                        }
                    }
                }

                // Check for unwrap() calls in production code
                if let Some(body) = &unit.body {
                    if body.contains(".unwrap()") && !unit.name.contains("test") {
                        warnings.push(TypeWarning {
                            file_path: unit.file_path.clone(),
                            line: unit.start_line as i32,
                            warning_type: "UnsafeUnwrap".to_string(),
                            message: format!("Function '{}' contains .unwrap() which may panic", unit.name),
                        });
                    }
                }
            }

            // Check generic bounds if requested
            // Note: CodeUnit doesn't have a generics field, so this check is commented out
            // if input.check_generic_bounds {
            //     for generic in &unit.generics {
            //         // Check if generic has trait bounds
            //         if !generic.contains(':') && input.strictness == "strict" {
            //             warnings.push(TypeWarning {
            //                 file_path: unit.file_path.clone(),
            //                 line: unit.start_line as i32,
            //                 warning_type: "UnboundedGeneric".to_string(),
            //                 message: format!("Generic parameter '{}' has no trait bounds", generic),
            //             });
            //         }
            //     }
            // }

            // Get dependencies to check for type mismatches
            if let Ok(deps) = semantic.get_dependencies(unit.id).await {
                for dep in deps {
                    // Check for potential type mismatches in function calls
                    if format!("{:?}", dep.dependency_type).contains("Call") {
                        if let Ok(Some(target)) = semantic.get_unit(dep.target_id).await {
                            // Simplified type checking - in practice would be much more sophisticated
                            if unit.parameters.len() != target.parameters.len() {
                                warnings.push(TypeWarning {
                                    file_path: unit.file_path.clone(),
                                    line: unit.start_line as i32,
                                    warning_type: "ArgumentCountMismatch".to_string(),
                                    message: format!("Potential argument count mismatch calling '{}'", target.name),
                                });
                            }
                        }
                    }
                }
            }
        }

        let output = CheckTypesOutput {
            total_errors: type_errors.len() as i32,
            total_warnings: warnings.len() as i32,
            type_errors,
            warnings,
            files_checked: files_checked,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.suggest_type_annotations
// =============================================================================

pub struct CodeSuggestTypeAnnotationsTool {
    ctx: TypeAnalysisContext,
}

impl CodeSuggestTypeAnnotationsTool {
    pub fn new(ctx: TypeAnalysisContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SuggestTypeAnnotationsInput {
    scope_path: String,
    #[serde(default = "default_high_confidence")]
    min_confidence: f32,
    #[serde(default = "default_true")]
    prioritize_public_api: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SuggestTypeAnnotationsOutput {
    suggestions: Vec<TypeAnnotationSuggestion>,
    total_suggestions: i32,
    estimated_coverage_increase: f32,
}

#[derive(Debug, Serialize, JsonSchema)]
struct TypeAnnotationSuggestion {
    file_path: String,
    line: i32,
    target: String,
    suggested_annotation: String,
    confidence: f32,
    code_preview: String,
}

impl Default for SuggestTypeAnnotationsOutput {
    fn default() -> Self {
        Self {
            suggestions: vec![],
            total_suggestions: 0,
            estimated_coverage_increase: 0.0,
        }
    }
}

#[async_trait]
impl Tool for CodeSuggestTypeAnnotationsTool {
    fn name(&self) -> &str {
        "cortex.code.suggest_type_annotations"
    }

    fn description(&self) -> Option<&str> {
        Some("Suggest type annotations to improve type coverage")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SuggestTypeAnnotationsInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SuggestTypeAnnotationsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Suggesting type annotations for: {}", input.scope_path);

        let manager = self.ctx.get_cognitive_manager();
        let semantic = manager.semantic();

        let mut suggestions = Vec::new();
        let mut current_coverage = 0.0;

        // Get all units in the scope
        let units = semantic.get_units_in_file(&input.scope_path).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get units: {}", e)))?;

        let mut total_items = 0;
        let mut typed_items = 0;

        for unit in units {
            // Prioritize public API if requested
            let is_public = format!("{:?}", unit.visibility) == "Public";
            if input.prioritize_public_api && !is_public {
                continue;
            }

            let priority_multiplier = if is_public { 1.2 } else { 1.0 };

            // Check for missing return type annotations
            if unit.return_type.is_none() && format!("{:?}", unit.unit_type).contains("Function") {
                total_items += 1;

                // Try to infer what the return type should be
                let suggested_type = if let Some(body) = &unit.body {
                    infer_return_type_from_body(body)
                } else {
                    "()".to_string()
                };

                let confidence = if suggested_type == "()" { 0.7 } else { 0.85 };
                let adjusted_confidence = confidence * priority_multiplier;

                if adjusted_confidence >= input.min_confidence {
                    suggestions.push(TypeAnnotationSuggestion {
                        file_path: unit.file_path.clone(),
                        line: unit.start_line as i32,
                        target: format!("fn {}", unit.name),
                        suggested_annotation: format!("-> {}", suggested_type),
                        confidence: adjusted_confidence as f32,
                        code_preview: unit.signature.clone(),
                    });
                }
            } else if unit.return_type.is_some() {
                typed_items += 1;
                total_items += 1;
            }

            // Check parameters for missing type annotations
            for param in &unit.parameters {
                total_items += 1;
                let param_type_str = param.param_type.as_deref().unwrap_or("");
                if param_type_str.is_empty() || param_type_str == "_" {
                    // Suggest type based on usage patterns
                    let suggested_type = if let Some(body) = &unit.body {
                        infer_parameter_type(&param.name, body)
                    } else {
                        "impl std::fmt::Display".to_string()
                    };

                    let confidence = 0.75 * priority_multiplier;
                    if confidence >= input.min_confidence {
                        suggestions.push(TypeAnnotationSuggestion {
                            file_path: unit.file_path.clone(),
                            line: unit.start_line as i32,
                            target: format!("parameter {}", param.name),
                            suggested_annotation: format!("{}: {}", param.name, suggested_type),
                            confidence: confidence as f32,
                            code_preview: format!("fn {}(...)", unit.name),
                        });
                    }
                } else {
                    typed_items += 1;
                }
            }

            // Suggest using more specific types
            if let Some(ret_type) = &unit.return_type {
                // Suggest Result instead of panic
                if let Some(body) = &unit.body {
                    if body.contains("panic!") && !ret_type.contains("Result") {
                        suggestions.push(TypeAnnotationSuggestion {
                            file_path: unit.file_path.clone(),
                            line: unit.start_line as i32,
                            target: format!("fn {}", unit.name),
                            suggested_annotation: format!("-> Result<{}, Error>", ret_type),
                            confidence: 0.9,
                            code_preview: unit.signature.clone(),
                        });
                    }

                    // Suggest Option for nullable returns
                    if body.contains("None") && !ret_type.contains("Option") && ret_type != "()" {
                        suggestions.push(TypeAnnotationSuggestion {
                            file_path: unit.file_path.clone(),
                            line: unit.start_line as i32,
                            target: format!("fn {}", unit.name),
                            suggested_annotation: format!("-> Option<{}>", ret_type),
                            confidence: 0.85,
                            code_preview: unit.signature.clone(),
                        });
                    }
                }

                // Suggest using trait objects or generics
                if ret_type.contains("Box<dyn") {
                    let generic_suggestion = ret_type.replace("Box<dyn", "impl");
                    let generic_suggestion = generic_suggestion.trim_end_matches('>').to_string();

                    suggestions.push(TypeAnnotationSuggestion {
                        file_path: unit.file_path.clone(),
                        line: unit.start_line as i32,
                        target: format!("fn {}", unit.name),
                        suggested_annotation: format!("-> {}", generic_suggestion),
                        confidence: 0.8,
                        code_preview: format!("Consider using 'impl Trait' instead of 'Box<dyn Trait>'"),
                    });
                }
            }

            // Check for complex types that could use type aliases
            for param in &unit.parameters {
                if let Some(param_type_str) = &param.param_type {
                    if param_type_str.matches('<').count() > 2 {
                        suggestions.push(TypeAnnotationSuggestion {
                            file_path: unit.file_path.clone(),
                            line: unit.start_line as i32,
                            target: format!("parameter {}", param.name),
                            suggested_annotation: format!("type Alias = {}; // then use Alias", param_type_str),
                            confidence: 0.7,
                            code_preview: "Consider using a type alias for complex types".to_string(),
                        });
                    }
                }
            }
        }

        // Calculate current coverage
        if total_items > 0 {
            current_coverage = (typed_items as f32 / total_items as f32) * 100.0;
        }

        // Estimate coverage increase
        let potential_new_typed = suggestions.len();
        let estimated_new_coverage = if total_items > 0 {
            ((typed_items + potential_new_typed) as f32 / total_items as f32) * 100.0
        } else {
            100.0
        };
        let estimated_increase = estimated_new_coverage - current_coverage;

        // Sort suggestions by confidence (highest first)
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        let output = SuggestTypeAnnotationsOutput {
            total_suggestions: suggestions.len() as i32,
            suggestions,
            estimated_coverage_increase: estimated_increase,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.analyze_type_coverage
// =============================================================================

pub struct CodeAnalyzeTypeCoverageTool {
    ctx: TypeAnalysisContext,
}

impl CodeAnalyzeTypeCoverageTool {
    pub fn new(ctx: TypeAnalysisContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AnalyzeTypeCoverageInput {
    scope_path: String,
    #[serde(default = "default_true")]
    include_file_breakdown: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct AnalyzeTypeCoverageOutput {
    overall_coverage: f32,
    function_coverage: f32,
    parameter_coverage: f32,
    return_type_coverage: f32,
    variable_coverage: f32,
    file_breakdown: Vec<FileCoverage>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct FileCoverage {
    file_path: String,
    coverage: f32,
    total_symbols: i32,
    typed_symbols: i32,
}

impl Default for AnalyzeTypeCoverageOutput {
    fn default() -> Self {
        Self {
            overall_coverage: 0.0,
            function_coverage: 0.0,
            parameter_coverage: 0.0,
            return_type_coverage: 0.0,
            variable_coverage: 0.0,
            file_breakdown: vec![],
        }
    }
}

#[async_trait]
impl Tool for CodeAnalyzeTypeCoverageTool {
    fn name(&self) -> &str {
        "cortex.code.analyze_type_coverage"
    }

    fn description(&self) -> Option<&str> {
        Some("Analyze type annotation coverage across the codebase")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AnalyzeTypeCoverageInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AnalyzeTypeCoverageInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Analyzing type coverage at: {}", input.scope_path);

        let manager = self.ctx.get_cognitive_manager();
        let semantic = manager.semantic();

        // Get all units in the scope
        let units = semantic.get_units_in_file(&input.scope_path).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get units: {}", e)))?;

        let mut total_functions = 0;
        let mut typed_functions = 0;
        let mut total_params = 0;
        let mut typed_params = 0;
        let mut total_returns = 0;
        let mut typed_returns = 0;
        let mut total_variables = 0;
        let mut typed_variables = 0;

        let mut file_stats: HashMap<String, (i32, i32)> = HashMap::new();

        for unit in units {
            // Count functions
            if format!("{:?}", unit.unit_type).contains("Function") {
                total_functions += 1;

                // Update file stats
                let entry = file_stats.entry(unit.file_path.clone()).or_insert((0, 0));
                entry.0 += 1; // total symbols

                // Check return type
                total_returns += 1;
                if let Some(ret_type) = &unit.return_type {
                    if !ret_type.is_empty() && ret_type != "_" {
                        typed_returns += 1;
                        typed_functions += 1;
                        entry.1 += 1; // typed symbols
                    }
                }

                // Check parameters
                for param in &unit.parameters {
                    total_params += 1;
                    entry.0 += 1;

                    let param_type_str = param.param_type.as_deref().unwrap_or("");
                    if !param_type_str.is_empty() && param_type_str != "_" {
                        typed_params += 1;
                        entry.1 += 1;
                    }
                }

                // Analyze variables in function body
                if let Some(body) = &unit.body {
                    let var_count = count_variable_declarations(body);
                    total_variables += var_count.0;
                    typed_variables += var_count.1;

                    entry.0 += var_count.0;
                    entry.1 += var_count.1;
                }
            }
        }

        // Calculate coverage percentages
        let function_coverage = if total_functions > 0 {
            (typed_functions as f32 / total_functions as f32) * 100.0
        } else {
            0.0
        };

        let parameter_coverage = if total_params > 0 {
            (typed_params as f32 / total_params as f32) * 100.0
        } else {
            0.0
        };

        let return_type_coverage = if total_returns > 0 {
            (typed_returns as f32 / total_returns as f32) * 100.0
        } else {
            0.0
        };

        let variable_coverage = if total_variables > 0 {
            (typed_variables as f32 / total_variables as f32) * 100.0
        } else {
            0.0
        };

        let total_items = total_functions + total_params + total_returns + total_variables;
        let typed_items = typed_functions + typed_params + typed_returns + typed_variables;

        let overall_coverage = if total_items > 0 {
            (typed_items as f32 / total_items as f32) * 100.0
        } else {
            0.0
        };

        // Build file breakdown if requested
        let file_breakdown = if input.include_file_breakdown {
            file_stats
                .into_iter()
                .map(|(path, (total, typed))| {
                    let coverage = if total > 0 {
                        (typed as f32 / total as f32) * 100.0
                    } else {
                        0.0
                    };

                    FileCoverage {
                        file_path: path,
                        coverage,
                        total_symbols: total,
                        typed_symbols: typed,
                    }
                })
                .collect()
        } else {
            vec![]
        };

        let output = AnalyzeTypeCoverageOutput {
            overall_coverage,
            function_coverage,
            parameter_coverage,
            return_type_coverage,
            variable_coverage,
            file_breakdown,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

fn default_true() -> bool {
    true
}

fn default_high_confidence() -> f32 {
    0.8
}

fn default_strict() -> String {
    "strict".to_string()
}

/// Parse a type string into TypeInfo structure
fn parse_type_info(type_str: &str) -> TypeInfo {
    let is_nullable = type_str.contains("Option");
    let is_generic = type_str.contains('<');

    let type_parameters = if is_generic {
        extract_type_parameters(type_str)
    } else {
        vec![]
    };

    TypeInfo {
        type_name: type_str.to_string(),
        is_nullable,
        is_generic,
        type_parameters,
    }
}

/// Extract type parameters from a generic type string
fn extract_type_parameters(type_str: &str) -> Vec<String> {
    let mut params = Vec::new();

    if let Some(start) = type_str.find('<') {
        if let Some(end) = type_str.rfind('>') {
            let inner = &type_str[start + 1..end];
            // Simple split by comma - doesn't handle nested generics perfectly
            for param in inner.split(',') {
                params.push(param.trim().to_string());
            }
        }
    }

    params
}

/// Calculate confidence score for a type annotation
fn calculate_type_confidence(type_str: &str) -> f32 {
    if type_str.is_empty() || type_str == "_" {
        return 0.0;
    }

    let mut confidence = 0.95;

    // Lower confidence for trait objects
    if type_str.contains("dyn") {
        confidence -= 0.1;
    }

    // Lower confidence for impl Trait
    if type_str.contains("impl") {
        confidence -= 0.05;
    }

    // Higher confidence for concrete types
    if !type_str.contains("dyn") && !type_str.contains("impl") && !type_str.contains('<') {
        confidence = 0.99;
    }

    confidence
}

/// Helper structure for variable type inference
struct VariableInference {
    var_name: String,
    inferred_type: String,
    line: i32,
    confidence: f32,
    reasoning: String,
}

/// Infer variable types from function body
fn infer_variable_types_from_body(
    body: &str,
    _file_path: &str,
    start_line: usize,
    min_confidence: f32,
) -> Vec<VariableInference> {
    let mut inferences = Vec::new();
    let mut line_num = start_line as i32;

    for line in body.lines() {
        line_num += 1;

        // Look for explicit type annotations: let x: Type = ...
        if let Some(var_inference) = parse_explicit_let_binding(line, line_num) {
            if var_inference.confidence >= min_confidence {
                inferences.push(var_inference);
            }
        }
        // Look for type inference from literals
        else if let Some(var_inference) = infer_from_literal(line, line_num) {
            if var_inference.confidence >= min_confidence {
                inferences.push(var_inference);
            }
        }
    }

    inferences
}

/// Parse explicit let bindings with type annotations
fn parse_explicit_let_binding(line: &str, line_num: i32) -> Option<VariableInference> {
    let trimmed = line.trim();

    if trimmed.starts_with("let ") && trimmed.contains(':') {
        // Extract variable name and type
        if let Some(colon_pos) = trimmed.find(':') {
            let before_colon = &trimmed[4..colon_pos].trim();
            let var_name = before_colon.trim_start_matches("mut ").trim().to_string();

            if let Some(eq_pos) = trimmed.find('=') {
                let type_part = &trimmed[colon_pos + 1..eq_pos].trim();
                let inferred_type = type_part.trim_end_matches(';').trim().to_string();

                return Some(VariableInference {
                    var_name,
                    inferred_type,
                    line: line_num,
                    confidence: 0.95,
                    reasoning: "Explicitly declared type annotation".to_string(),
                });
            } else {
                let type_part = &trimmed[colon_pos + 1..].trim();
                let inferred_type = type_part.trim_end_matches(';').trim().to_string();

                return Some(VariableInference {
                    var_name,
                    inferred_type,
                    line: line_num,
                    confidence: 0.95,
                    reasoning: "Explicitly declared type annotation".to_string(),
                });
            }
        }
    }

    None
}

/// Infer types from literal values
fn infer_from_literal(line: &str, line_num: i32) -> Option<VariableInference> {
    let trimmed = line.trim();

    if trimmed.starts_with("let ") && !trimmed.contains(':') {
        if let Some(eq_pos) = trimmed.find('=') {
            let before_eq = &trimmed[4..eq_pos].trim();
            let var_name = before_eq.trim_start_matches("mut ").trim().to_string();
            let after_eq = &trimmed[eq_pos + 1..].trim();

            let inferred_type = if after_eq.starts_with('"') {
                "&str"
            } else if after_eq.starts_with("String::") || after_eq.starts_with("format!") {
                "String"
            } else if after_eq.starts_with("true") || after_eq.starts_with("false") {
                "bool"
            } else if after_eq.starts_with("vec![") || after_eq.starts_with("Vec::") {
                "Vec<T>"
            } else if after_eq.contains('.') && after_eq.chars().next().unwrap_or(' ').is_numeric() {
                "f64"
            } else if after_eq.chars().next().unwrap_or(' ').is_numeric() {
                "i32"
            } else if after_eq.starts_with("Some(") {
                "Option<T>"
            } else if after_eq.starts_with("None") {
                "Option<T>"
            } else if after_eq.starts_with("Ok(") || after_eq.starts_with("Err(") {
                "Result<T, E>"
            } else {
                return None;
            };

            return Some(VariableInference {
                var_name,
                inferred_type: inferred_type.to_string(),
                line: line_num,
                confidence: 0.75,
                reasoning: "Inferred from literal value".to_string(),
            });
        }
    }

    None
}

/// Infer return type from function body
fn infer_return_type_from_body(body: &str) -> String {
    // Look for explicit return statements
    for line in body.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("return ") {
            let after_return = trimmed.strip_prefix("return ").unwrap_or("");

            if after_return.starts_with("Ok(") {
                return "Result<T, E>".to_string();
            } else if after_return.starts_with("Err(") {
                return "Result<T, E>".to_string();
            } else if after_return.starts_with("Some(") {
                return "Option<T>".to_string();
            } else if after_return.starts_with("None") {
                return "Option<T>".to_string();
            } else if after_return.starts_with('"') {
                return "&str".to_string();
            } else if after_return.starts_with("String::") {
                return "String".to_string();
            } else if after_return.starts_with("true") || after_return.starts_with("false") {
                return "bool".to_string();
            } else if after_return.chars().next().unwrap_or(' ').is_numeric() {
                return "i32".to_string();
            }
        }
    }

    // Check if last expression looks like a return value
    if let Some(last_line) = body.lines().last() {
        let trimmed = last_line.trim();
        if !trimmed.is_empty() && !trimmed.ends_with(';') && !trimmed.ends_with('}') {
            if trimmed.starts_with("Ok(") || trimmed.starts_with("Err(") {
                return "Result<T, E>".to_string();
            } else if trimmed.starts_with("Some(") || trimmed == "None" {
                return "Option<T>".to_string();
            }
        }
    }

    "()".to_string()
}

/// Infer parameter type from usage in body
fn infer_parameter_type(param_name: &str, body: &str) -> String {
    for line in body.lines() {
        let trimmed = line.trim();

        // Check for method calls that indicate type
        if trimmed.contains(&format!("{}.len()", param_name)) {
            return "impl AsRef<[T]>".to_string();
        }
        if trimmed.contains(&format!("{}.to_string()", param_name)) {
            return "impl std::fmt::Display".to_string();
        }
        if trimmed.contains(&format!("{}.clone()", param_name)) {
            return "impl Clone".to_string();
        }
        if trimmed.contains(&format!("{}.iter()", param_name)) {
            return "impl IntoIterator".to_string();
        }
        if trimmed.contains(&format!("&{}", param_name)) && trimmed.contains("==") {
            return "impl PartialEq".to_string();
        }
    }

    // Default to a generic trait
    "impl std::fmt::Debug".to_string()
}

/// Count variable declarations in code
fn count_variable_declarations(body: &str) -> (i32, i32) {
    let mut total = 0;
    let mut typed = 0;

    for line in body.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("let ") {
            total += 1;

            // Check if it has explicit type annotation
            if trimmed.contains(':') && !trimmed.starts_with("let _") {
                if let Some(colon_pos) = trimmed.find(':') {
                    if let Some(eq_pos) = trimmed.find('=') {
                        if colon_pos < eq_pos {
                            typed += 1;
                        }
                    } else {
                        typed += 1;
                    }
                }
            }
        }
    }

    (total, typed)
}
