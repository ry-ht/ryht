//! Documentation Tools (8 tools)
//!
//! This module implements tools for generating, managing, and maintaining code documentation.
//! These tools extract docstrings, generate markdown docs, find undocumented code, and maintain
//! documentation consistency with the codebase.

use async_trait::async_trait;
use cortex_core::types::CodeUnit;
use cortex_memory::CognitiveManager;
use cortex_storage::ConnectionManager;
use mcp_sdk::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

// Import the unified service layer
use crate::services::CodeUnitService;

#[derive(Clone)]
pub struct DocumentationContext {
    storage: Arc<ConnectionManager>,
    code_unit_service: Arc<CodeUnitService>,
}

impl DocumentationContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        let code_unit_service = Arc::new(CodeUnitService::new(storage.clone()));
        Self {
            storage,
            code_unit_service,
        }
    }

    fn get_cognitive_manager(&self) -> CognitiveManager {
        CognitiveManager::new(self.storage.clone())
    }
}

// =============================================================================
// cortex.docs.generate
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocGenerateInput {
    unit_id: String,
    #[serde(default = "default_api_type")]
    doc_type: String,
    #[serde(default = "default_markdown")]
    format: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DocGenerateOutput {
    documentation: String,
    format: String,
}

pub struct DocGenerateTool {
    ctx: DocumentationContext,
}

impl DocGenerateTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }

    /// Generate markdown documentation for a code unit
    fn generate_markdown(&self, unit: &CodeUnit) -> String {
        let mut doc = String::new();

        // Title
        doc.push_str(&format!("# {}\n\n", unit.name));

        // Type and location
        doc.push_str(&format!("**Type:** {:?}\n\n", unit.unit_type));
        doc.push_str(&format!("**Location:** {}:{}:{}\n\n",
            unit.file_path, unit.start_line, unit.start_column));

        // Signature
        doc.push_str("## Signature\n\n");
        doc.push_str(&format!("```{}\n{}\n```\n\n",
            format!("{:?}", unit.language).to_lowercase(),
            unit.signature));

        // Description from docstring
        if let Some(ref docstring) = unit.docstring {
            doc.push_str("## Description\n\n");
            doc.push_str(docstring);
            doc.push_str("\n\n");
        }

        // Parameters
        if !unit.parameters.is_empty() {
            doc.push_str("## Parameters\n\n");
            for param in &unit.parameters {
                doc.push_str(&format!("- **{}**", param.name));
                if let Some(ref param_type) = param.param_type {
                    doc.push_str(&format!(": `{}`", param_type));
                }
                if param.is_optional {
                    doc.push_str(" (optional)");
                }
                doc.push_str("\n");
            }
            doc.push_str("\n");
        }

        // Return type
        if let Some(ref return_type) = unit.return_type {
            doc.push_str("## Returns\n\n");
            doc.push_str(&format!("`{}`\n\n", return_type));
        }

        // Throws/Errors
        if !unit.throws.is_empty() {
            doc.push_str("## Errors\n\n");
            for err in &unit.throws {
                doc.push_str(&format!("- `{}`\n", err));
            }
            doc.push_str("\n");
        }

        // Complexity metrics
        doc.push_str("## Metrics\n\n");
        doc.push_str(&format!("- **Cyclomatic Complexity:** {}\n", unit.complexity.cyclomatic));
        doc.push_str(&format!("- **Cognitive Complexity:** {}\n", unit.complexity.cognitive));
        doc.push_str(&format!("- **Lines of Code:** {}\n", unit.complexity.lines));
        doc.push_str(&format!("- **Has Tests:** {}\n", unit.has_tests));
        doc.push_str("\n");

        // Visibility and modifiers
        doc.push_str("## Attributes\n\n");
        doc.push_str(&format!("- **Visibility:** {:?}\n", unit.visibility));
        if !unit.modifiers.is_empty() {
            doc.push_str(&format!("- **Modifiers:** {}\n", unit.modifiers.join(", ")));
        }
        if unit.is_async {
            doc.push_str("- **Async:** true\n");
        }

        doc
    }

    /// Generate API-style documentation
    fn generate_api_doc(&self, unit: &CodeUnit) -> String {
        let mut doc = String::new();

        // API endpoint style
        doc.push_str(&format!("### `{}`\n\n", unit.qualified_name));

        if let Some(ref docstring) = unit.docstring {
            doc.push_str(docstring);
            doc.push_str("\n\n");
        }

        doc.push_str("**Request:**\n\n");
        doc.push_str(&format!("```{}\n{}\n```\n\n",
            format!("{:?}", unit.language).to_lowercase(),
            unit.signature));

        if let Some(ref return_type) = unit.return_type {
            doc.push_str("**Response:**\n\n");
            doc.push_str(&format!("`{}`\n\n", return_type));
        }

        doc
    }
}

#[async_trait]
impl Tool for DocGenerateTool {
    fn name(&self) -> &str {
        "cortex.docs.generate"
    }

    fn description(&self) -> Option<&str> {
        Some("Generate documentation from code unit with options for markdown, API, or custom formats")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DocGenerateInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DocGenerateInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        info!("Generating {} documentation for unit: {}", input.doc_type, input.unit_id);

        // Get full code unit with body
        let manager = self.ctx.get_cognitive_manager();
        let semantic = manager.semantic();
        let cortex_id = cortex_core::id::CortexId::from_str(&input.unit_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid unit_id: {}", e)))?;

        let full_unit = semantic.get_unit(cortex_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Database error: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed("Code unit not found".to_string()))?;

        // Generate documentation based on type and format
        let documentation = match input.doc_type.as_str() {
            "api" => self.generate_api_doc(&full_unit),
            "markdown" | _ => self.generate_markdown(&full_unit),
        };

        let output = DocGenerateOutput {
            documentation,
            format: input.format,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.docs.update
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocUpdateInput {
    unit_id: String,
    doc_content: String,
    #[serde(default = "default_docstring")]
    doc_type: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DocUpdateOutput {
    unit_id: String,
    updated: bool,
}

pub struct DocUpdateTool {
    ctx: DocumentationContext,
}

impl DocUpdateTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for DocUpdateTool {
    fn name(&self) -> &str {
        "cortex.docs.update"
    }

    fn description(&self) -> Option<&str> {
        Some("Update existing documentation for a code unit")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DocUpdateInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DocUpdateInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        info!("Updating {} documentation for unit: {}", input.doc_type, input.unit_id);

        // Update the code unit's docstring
        let updated_unit = self.ctx.code_unit_service
            .update_code_unit(&input.unit_id, None, Some(input.doc_content.clone()), None)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to update code unit: {}", e)))?;

        info!("Successfully updated documentation for unit: {}", input.unit_id);

        let output = DocUpdateOutput {
            unit_id: updated_unit.id,
            updated: true,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.docs.extract
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocExtractInput {
    scope_path: String,
    #[serde(default)]
    include_private: bool,
    #[serde(default = "default_markdown")]
    format: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DocExtractOutput {
    documentation: String,
    units_documented: i32,
}

pub struct DocExtractTool {
    ctx: DocumentationContext,
}

impl DocExtractTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }

    /// Extract and compile documentation from multiple code units
    fn compile_documentation(&self, units: Vec<CodeUnit>, _format: &str) -> String {
        let mut doc = String::new();

        // Add header
        doc.push_str("# API Documentation\n\n");
        doc.push_str(&format!("Generated from {} code units\n\n", units.len()));

        // Group by file
        let mut by_file: HashMap<String, Vec<&CodeUnit>> = HashMap::new();
        for unit in &units {
            by_file.entry(unit.file_path.clone())
                .or_insert_with(Vec::new)
                .push(unit);
        }

        // Generate documentation for each file
        for (file_path, file_units) in by_file.iter() {
            doc.push_str(&format!("## {}\n\n", file_path));

            for unit in file_units {
                // Skip units without documentation if requested
                if unit.docstring.is_none() {
                    continue;
                }

                doc.push_str(&format!("### {}\n\n", unit.name));

                // Location
                doc.push_str(&format!("*Location: {}:{}-{}*\n\n",
                    file_path, unit.start_line, unit.end_line));

                // Signature
                doc.push_str(&format!("```{}\n{}\n```\n\n",
                    format!("{:?}", unit.language).to_lowercase(),
                    unit.signature));

                // Docstring
                if let Some(ref docstring) = unit.docstring {
                    doc.push_str(docstring);
                    doc.push_str("\n\n");
                }

                // Parameters
                if !unit.parameters.is_empty() {
                    doc.push_str("**Parameters:**\n\n");
                    for param in &unit.parameters {
                        doc.push_str(&format!("- `{}`: ", param.name));
                        if let Some(ref param_type) = param.param_type {
                            doc.push_str(&format!("`{}`", param_type));
                        }
                        doc.push_str("\n");
                    }
                    doc.push_str("\n");
                }

                // Return type
                if let Some(ref return_type) = unit.return_type {
                    doc.push_str(&format!("**Returns:** `{}`\n\n", return_type));
                }

                doc.push_str("---\n\n");
            }
        }

        doc
    }
}

#[async_trait]
impl Tool for DocExtractTool {
    fn name(&self) -> &str {
        "cortex.docs.extract"
    }

    fn description(&self) -> Option<&str> {
        Some("Extract documentation from code in a given scope (file, directory, or workspace)")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DocExtractInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DocExtractInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        info!("Extracting documentation from scope: {}", input.scope_path);

        // Parse workspace_id from scope_path (format: workspace_id or workspace_id/path)
        let parts: Vec<&str> = input.scope_path.split('/').collect();
        let workspace_id = Uuid::parse_str(parts[0])
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace_id in scope_path: {}", e)))?;

        // Query code units in this scope
        let pooled = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Database error: {}", e)))?;
        let conn = pooled.connection();

        // Build query based on scope
        let mut query = if parts.len() > 1 {
            // Specific file or directory path
            let path = parts[1..].join("/");
            format!(
                "SELECT * FROM code_unit WHERE file_path CONTAINS '{}' AND file_path CONTAINS '{}'",
                workspace_id, path
            )
        } else {
            // Entire workspace
            format!("SELECT * FROM code_unit WHERE file_path CONTAINS '{}'", workspace_id)
        };

        // Filter by visibility
        if !input.include_private {
            query.push_str(" AND visibility = 'Public'");
        }

        // Only include units with documentation
        query.push_str(" AND has_documentation = true");

        query.push_str(" LIMIT 500");

        let mut result = conn.query(&query).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Query error: {}", e)))?;
        let units: Vec<CodeUnit> = result.take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse results: {}", e)))?;

        let units_documented = units.len() as i32;
        info!("Found {} documented code units", units_documented);

        // Compile documentation
        let documentation = self.compile_documentation(units, &input.format);

        let output = DocExtractOutput {
            documentation,
            units_documented,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.docs.find_undocumented
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocFindUndocumentedInput {
    scope_path: String,
    #[serde(default = "default_public")]
    visibility: String,
    #[serde(default = "default_complexity_one")]
    min_complexity: i32,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DocFindUndocumentedOutput {
    undocumented_units: Vec<UndocumentedUnit>,
    total_count: i32,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct UndocumentedUnit {
    unit_id: String,
    name: String,
    unit_type: String,
    file_path: String,
    start_line: usize,
    complexity_score: f64,
}

pub struct DocFindUndocumentedTool {
    ctx: DocumentationContext,
}

impl DocFindUndocumentedTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for DocFindUndocumentedTool {
    fn name(&self) -> &str {
        "cortex.docs.find_undocumented"
    }

    fn description(&self) -> Option<&str> {
        Some("Find undocumented code units that should have documentation based on visibility and complexity")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DocFindUndocumentedInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DocFindUndocumentedInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        info!("Finding undocumented code in scope: {}", input.scope_path);

        // Parse workspace_id from scope_path
        let parts: Vec<&str> = input.scope_path.split('/').collect();
        let workspace_id = Uuid::parse_str(parts[0])
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace_id in scope_path: {}", e)))?;

        // Query code units that lack documentation
        let pooled = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Database error: {}", e)))?;
        let conn = pooled.connection();

        // Build query
        let mut query = if parts.len() > 1 {
            let path = parts[1..].join("/");
            format!(
                "SELECT * FROM code_unit WHERE file_path CONTAINS '{}' AND file_path CONTAINS '{}'",
                workspace_id, path
            )
        } else {
            format!("SELECT * FROM code_unit WHERE file_path CONTAINS '{}'", workspace_id)
        };

        // Filter by visibility
        let visibility_filter = match input.visibility.to_lowercase().as_str() {
            "public" => "Public",
            "private" => "Private",
            "protected" => "Protected",
            _ => "Public",
        };
        query.push_str(&format!(" AND visibility = '{}'", visibility_filter));

        // Filter by documentation status
        query.push_str(" AND has_documentation = false");

        // Filter by complexity
        if input.min_complexity > 1 {
            query.push_str(&format!(" AND complexity.cyclomatic >= {}", input.min_complexity));
        }

        query.push_str(" LIMIT 500");

        let mut result = conn.query(&query).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Query error: {}", e)))?;
        let units: Vec<CodeUnit> = result.take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse results: {}", e)))?;

        info!("Found {} undocumented code units", units.len());

        // Build output
        let undocumented_units: Vec<UndocumentedUnit> = units
            .into_iter()
            .map(|unit| {
                let complexity_score = unit.complexity_score();
                UndocumentedUnit {
                    unit_id: unit.id.to_string(),
                    name: unit.name,
                    unit_type: format!("{:?}", unit.unit_type),
                    file_path: unit.file_path,
                    start_line: unit.start_line,
                    complexity_score,
                }
            })
            .collect();

        let total_count = undocumented_units.len() as i32;

        let output = DocFindUndocumentedOutput {
            undocumented_units,
            total_count,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.docs.check_consistency
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocCheckConsistencyInput {
    scope_path: String,
    #[serde(default = "default_true")]
    check_parameters: bool,
    #[serde(default = "default_true")]
    check_returns: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DocCheckConsistencyOutput {
    inconsistencies: Vec<DocInconsistency>,
    total_count: i32,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DocInconsistency {
    unit_id: String,
    unit_name: String,
    file_path: String,
    start_line: usize,
    inconsistency_type: String,
    description: String,
}

pub struct DocCheckConsistencyTool {
    ctx: DocumentationContext,
}

impl DocCheckConsistencyTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }

    /// Check if documentation is consistent with code
    fn check_unit_consistency(&self, unit: &CodeUnit, check_parameters: bool, check_returns: bool) -> Vec<DocInconsistency> {
        let mut inconsistencies = Vec::new();

        // Check if public unit has documentation
        if unit.visibility == cortex_core::types::Visibility::Public && !unit.has_documentation {
            inconsistencies.push(DocInconsistency {
                unit_id: unit.id.to_string(),
                unit_name: unit.name.clone(),
                file_path: unit.file_path.clone(),
                start_line: unit.start_line,
                inconsistency_type: "missing_documentation".to_string(),
                description: "Public unit lacks documentation".to_string(),
            });
        }

        if let Some(ref docstring) = unit.docstring {
            // Check parameter documentation
            if check_parameters && !unit.parameters.is_empty() {
                for param in &unit.parameters {
                    // Check if parameter is mentioned in docstring
                    if !docstring.contains(&param.name) {
                        inconsistencies.push(DocInconsistency {
                            unit_id: unit.id.to_string(),
                            unit_name: unit.name.clone(),
                            file_path: unit.file_path.clone(),
                            start_line: unit.start_line,
                            inconsistency_type: "undocumented_parameter".to_string(),
                            description: format!("Parameter '{}' not documented", param.name),
                        });
                    }
                }
            }

            // Check return type documentation
            if check_returns && unit.return_type.is_some() {
                let has_return_docs = docstring.to_lowercase().contains("return")
                    || docstring.to_lowercase().contains("@return")
                    || docstring.to_lowercase().contains("@returns");

                if !has_return_docs {
                    inconsistencies.push(DocInconsistency {
                        unit_id: unit.id.to_string(),
                        unit_name: unit.name.clone(),
                        file_path: unit.file_path.clone(),
                        start_line: unit.start_line,
                        inconsistency_type: "undocumented_return".to_string(),
                        description: "Return value not documented".to_string(),
                    });
                }
            }

            // Check for outdated complexity warnings
            if unit.complexity.cyclomatic > 10 {
                let has_complexity_warning = docstring.to_lowercase().contains("complex")
                    || docstring.to_lowercase().contains("complicated");

                if !has_complexity_warning {
                    inconsistencies.push(DocInconsistency {
                        unit_id: unit.id.to_string(),
                        unit_name: unit.name.clone(),
                        file_path: unit.file_path.clone(),
                        start_line: unit.start_line,
                        inconsistency_type: "missing_complexity_warning".to_string(),
                        description: format!("High complexity ({}) not mentioned in docs", unit.complexity.cyclomatic),
                    });
                }
            }

            // Check for async documentation
            if unit.is_async {
                let has_async_docs = docstring.to_lowercase().contains("async")
                    || docstring.to_lowercase().contains("await");

                if !has_async_docs {
                    inconsistencies.push(DocInconsistency {
                        unit_id: unit.id.to_string(),
                        unit_name: unit.name.clone(),
                        file_path: unit.file_path.clone(),
                        start_line: unit.start_line,
                        inconsistency_type: "undocumented_async".to_string(),
                        description: "Async nature not documented".to_string(),
                    });
                }
            }
        }

        inconsistencies
    }
}

#[async_trait]
impl Tool for DocCheckConsistencyTool {
    fn name(&self) -> &str {
        "cortex.docs.check_consistency"
    }

    fn description(&self) -> Option<&str> {
        Some("Check consistency between documentation and code, finding mismatches in parameters, return types, and behavior")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DocCheckConsistencyInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DocCheckConsistencyInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        info!("Checking documentation consistency in scope: {}", input.scope_path);

        // Parse workspace_id from scope_path
        let parts: Vec<&str> = input.scope_path.split('/').collect();
        let workspace_id = Uuid::parse_str(parts[0])
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace_id in scope_path: {}", e)))?;

        // Query all code units in scope
        let pooled = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Database error: {}", e)))?;
        let conn = pooled.connection();

        let query = if parts.len() > 1 {
            let path = parts[1..].join("/");
            format!(
                "SELECT * FROM code_unit WHERE file_path CONTAINS '{}' AND file_path CONTAINS '{}' LIMIT 500",
                workspace_id, path
            )
        } else {
            format!("SELECT * FROM code_unit WHERE file_path CONTAINS '{}' LIMIT 500", workspace_id)
        };

        let mut result = conn.query(&query).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Query error: {}", e)))?;
        let units: Vec<CodeUnit> = result.take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse results: {}", e)))?;

        info!("Checking {} code units for consistency", units.len());

        // Check each unit for inconsistencies
        let mut all_inconsistencies = Vec::new();
        for unit in units {
            let unit_inconsistencies = self.check_unit_consistency(&unit, input.check_parameters, input.check_returns);
            all_inconsistencies.extend(unit_inconsistencies);
        }

        let total_count = all_inconsistencies.len() as i32;
        info!("Found {} documentation inconsistencies", total_count);

        let output = DocCheckConsistencyOutput {
            inconsistencies: all_inconsistencies,
            total_count,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.docs.link_to_code
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocLinkToCodeInput {
    doc_id: String,
    unit_id: String,
    #[serde(default = "default_describes")]
    link_type: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DocLinkToCodeOutput {
    link_id: String,
    created: bool,
    link_details: LinkDetails,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct LinkDetails {
    doc_id: String,
    unit_id: String,
    unit_name: String,
    file_path: String,
    start_line: usize,
    link_type: String,
}

pub struct DocLinkToCodeTool {
    ctx: DocumentationContext,
}

impl DocLinkToCodeTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for DocLinkToCodeTool {
    fn name(&self) -> &str {
        "cortex.docs.link_to_code"
    }

    fn description(&self) -> Option<&str> {
        Some("Create a bidirectional link between documentation and code units for traceability")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DocLinkToCodeInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DocLinkToCodeInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        info!("Creating {} link from doc {} to unit {}", input.link_type, input.doc_id, input.unit_id);

        // Get the code unit details
        let unit = self.ctx.code_unit_service
            .get_code_unit(&input.unit_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get code unit: {}", e)))?;

        // Create a relation record in the database
        let pooled = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Database error: {}", e)))?;
        let conn = pooled.connection();

        // Generate a unique link ID
        let link_id = Uuid::new_v4().to_string();

        // Create the link relation
        let link_record = serde_json::json!({
            "id": link_id,
            "doc_id": input.doc_id,
            "unit_id": input.unit_id,
            "link_type": input.link_type,
            "created_at": chrono::Utc::now().to_rfc3339(),
        });

        // Store the link (using a doc_link table/record)
        let _: Option<serde_json::Value> = conn
            .create(("doc_link", link_id.clone()))
            .content(link_record)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create link: {}", e)))?;

        info!("Successfully created link: {}", link_id);

        let output = DocLinkToCodeOutput {
            link_id: link_id.clone(),
            created: true,
            link_details: LinkDetails {
                doc_id: input.doc_id,
                unit_id: input.unit_id.clone(),
                unit_name: unit.name,
                file_path: unit.file_path,
                start_line: unit.start_line,
                link_type: input.link_type,
            },
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.docs.generate_readme
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocGenerateReadmeInput {
    scope_path: String,
    sections: Option<Vec<String>>,
    #[serde(default = "default_true")]
    include_api: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DocGenerateReadmeOutput {
    readme_content: String,
    sections_included: Vec<String>,
}

pub struct DocGenerateReadmeTool {
    ctx: DocumentationContext,
}

impl DocGenerateReadmeTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }

    /// Generate a comprehensive README
    async fn generate_readme(
        &self,
        workspace_id: &Uuid,
        sections: Option<Vec<String>>,
        include_api: bool,
    ) -> std::result::Result<(String, Vec<String>), String> {
        let mut readme = String::new();
        let mut included_sections = Vec::new();

        let default_sections = vec![
            "overview".to_string(),
            "installation".to_string(),
            "usage".to_string(),
            "api".to_string(),
            "contributing".to_string(),
            "license".to_string(),
        ];

        let sections_to_include = sections.unwrap_or(default_sections);

        // Get workspace details
        let pooled = self.ctx.storage.acquire().await
            .map_err(|e| format!("Database error: {}", e))?;
        let conn = pooled.connection();

        let workspace_query = format!("SELECT * FROM workspace WHERE id = '{}'", workspace_id);
        let mut result = conn.query(&workspace_query).await
            .map_err(|e| format!("Query error: {}", e))?;
        let workspaces: Vec<serde_json::Value> = result.take(0)
            .map_err(|e| format!("Failed to parse workspace: {}", e))?;

        let workspace = workspaces.first();
        let workspace_name = workspace
            .and_then(|w| w.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("Project");

        // Title
        readme.push_str(&format!("# {}\n\n", workspace_name));

        // Overview section
        if sections_to_include.contains(&"overview".to_string()) {
            included_sections.push("overview".to_string());
            readme.push_str("## Overview\n\n");
            readme.push_str(&format!("This is the {} project.\n\n", workspace_name));

            // Get project statistics
            let stats_query = format!(
                "SELECT count() as total FROM code_unit WHERE file_path CONTAINS '{}' GROUP ALL",
                workspace_id
            );
            let mut stats_result = conn.query(&stats_query).await
                .map_err(|e| format!("Stats query error: {}", e))?;
            let stats: Vec<serde_json::Value> = stats_result.take(0).unwrap_or_default();

            if let Some(stat) = stats.first() {
                if let Some(total) = stat.get("total").and_then(|t| t.as_u64()) {
                    readme.push_str(&format!("- **Total Code Units:** {}\n", total));
                }
            }
            readme.push_str("\n");
        }

        // Installation section
        if sections_to_include.contains(&"installation".to_string()) {
            included_sections.push("installation".to_string());
            readme.push_str("## Installation\n\n");
            readme.push_str("```bash\n");
            readme.push_str("# Add installation instructions here\n");
            readme.push_str("```\n\n");
        }

        // Usage section
        if sections_to_include.contains(&"usage".to_string()) {
            included_sections.push("usage".to_string());
            readme.push_str("## Usage\n\n");
            readme.push_str("```bash\n");
            readme.push_str("# Add usage examples here\n");
            readme.push_str("```\n\n");
        }

        // API section
        if sections_to_include.contains(&"api".to_string()) && include_api {
            included_sections.push("api".to_string());
            readme.push_str("## API Reference\n\n");

            // Get public functions/methods
            let api_query = format!(
                "SELECT * FROM code_unit WHERE file_path CONTAINS '{}' AND visibility = 'Public' AND has_documentation = true LIMIT 50",
                workspace_id
            );
            let mut api_result = conn.query(&api_query).await
                .map_err(|e| format!("API query error: {}", e))?;
            let api_units: Vec<CodeUnit> = api_result.take(0).unwrap_or_default();

            if !api_units.is_empty() {
                for unit in api_units.iter().take(10) {
                    readme.push_str(&format!("### {}\n\n", unit.name));
                    readme.push_str(&format!("```{}\n{}\n```\n\n",
                        format!("{:?}", unit.language).to_lowercase(),
                        unit.signature));

                    if let Some(ref doc) = unit.docstring {
                        readme.push_str(doc);
                        readme.push_str("\n\n");
                    }
                }
            } else {
                readme.push_str("No public API documentation available.\n\n");
            }
        }

        // Contributing section
        if sections_to_include.contains(&"contributing".to_string()) {
            included_sections.push("contributing".to_string());
            readme.push_str("## Contributing\n\n");
            readme.push_str("Contributions are welcome! Please feel free to submit a Pull Request.\n\n");
        }

        // License section
        if sections_to_include.contains(&"license".to_string()) {
            included_sections.push("license".to_string());
            readme.push_str("## License\n\n");
            readme.push_str("This project is licensed under the MIT License.\n\n");
        }

        Ok((readme, included_sections))
    }
}

#[async_trait]
impl Tool for DocGenerateReadmeTool {
    fn name(&self) -> &str {
        "cortex.docs.generate_readme"
    }

    fn description(&self) -> Option<&str> {
        Some("Generate a comprehensive README.md file for a workspace or project")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DocGenerateReadmeInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DocGenerateReadmeInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        info!("Generating README for scope: {}", input.scope_path);

        // Parse workspace_id from scope_path
        let parts: Vec<&str> = input.scope_path.split('/').collect();
        let workspace_id = Uuid::parse_str(parts[0])
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace_id in scope_path: {}", e)))?;

        let (readme_content, sections_included) = self
            .generate_readme(&workspace_id, input.sections, input.include_api)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Generated README with {} sections", sections_included.len());

        let output = DocGenerateReadmeOutput {
            readme_content,
            sections_included,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.docs.generate_changelog
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocGenerateChangelogInput {
    from_version: Option<String>,
    to_version: Option<String>,
    #[serde(default = "default_keepachangelog")]
    format: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DocGenerateChangelogOutput {
    changelog_content: String,
    format: String,
}

pub struct DocGenerateChangelogTool {
    ctx: DocumentationContext,
}

impl DocGenerateChangelogTool {
    pub fn new(ctx: DocumentationContext) -> Self {
        Self { ctx }
    }

    /// Generate a changelog in Keep a Changelog format
    async fn generate_changelog(
        &self,
        from_version: Option<String>,
        to_version: Option<String>,
    ) -> std::result::Result<String, String> {
        let mut changelog = String::new();

        // Header
        changelog.push_str("# Changelog\n\n");
        changelog.push_str("All notable changes to this project will be documented in this file.\n\n");
        changelog.push_str("The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),\n");
        changelog.push_str("and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).\n\n");

        // Query for code unit changes (using version history if available)
        let pooled = self.ctx.storage.acquire().await
            .map_err(|e| format!("Database error: {}", e))?;
        let conn = pooled.connection();

        // Get recent changes (we'll use updated_at to simulate version history)
        let query = "SELECT * FROM code_unit ORDER BY updated_at DESC LIMIT 100";
        let mut result = conn.query(query).await
            .map_err(|e| format!("Query error: {}", e))?;
        let units: Vec<CodeUnit> = result.take(0).unwrap_or_default();

        // Group changes by date
        let mut changes_by_date: HashMap<String, Vec<&CodeUnit>> = HashMap::new();
        for unit in &units {
            let date = unit.updated_at.format("%Y-%m-%d").to_string();
            changes_by_date.entry(date)
                .or_insert_with(Vec::new)
                .push(unit);
        }

        // Generate version sections
        let version = to_version.unwrap_or_else(|| "Unreleased".to_string());
        changelog.push_str(&format!("## [{}]\n\n", version));

        if !changes_by_date.is_empty() {
            // Categorize changes
            let mut added = Vec::new();
            let mut changed = Vec::new();
            let mut deprecated = Vec::new();

            for unit in &units {
                if unit.version == 1 {
                    added.push(unit);
                } else if unit.status == cortex_core::types::CodeUnitStatus::Deprecated {
                    deprecated.push(unit);
                } else {
                    changed.push(unit);
                }
            }

            // Added section
            if !added.is_empty() {
                changelog.push_str("### Added\n\n");
                for unit in added.iter().take(10) {
                    changelog.push_str(&format!("- `{}` in {} ({}:{})\n",
                        unit.name,
                        unit.file_path,
                        unit.file_path,
                        unit.start_line));
                }
                changelog.push_str("\n");
            }

            // Changed section
            if !changed.is_empty() {
                changelog.push_str("### Changed\n\n");
                for unit in changed.iter().take(10) {
                    changelog.push_str(&format!("- Updated `{}` (v{})\n", unit.name, unit.version));
                }
                changelog.push_str("\n");
            }

            // Deprecated section
            if !deprecated.is_empty() {
                changelog.push_str("### Deprecated\n\n");
                for unit in deprecated.iter().take(10) {
                    changelog.push_str(&format!("- `{}` is now deprecated\n", unit.name));
                }
                changelog.push_str("\n");
            }
        } else {
            changelog.push_str("No changes recorded.\n\n");
        }

        // Add previous version if specified
        if let Some(from_ver) = from_version {
            changelog.push_str(&format!("## [{}]\n\n", from_ver));
            changelog.push_str("See previous releases for details.\n\n");
        }

        Ok(changelog)
    }
}

#[async_trait]
impl Tool for DocGenerateChangelogTool {
    fn name(&self) -> &str {
        "cortex.docs.generate_changelog"
    }

    fn description(&self) -> Option<&str> {
        Some("Generate a CHANGELOG.md file tracking code changes between versions")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DocGenerateChangelogInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DocGenerateChangelogInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        info!("Generating CHANGELOG from {:?} to {:?}", input.from_version, input.to_version);

        let changelog_content = self
            .generate_changelog(input.from_version.clone(), input.to_version.clone())
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Generated CHANGELOG in {} format", input.format);

        let output = DocGenerateChangelogOutput {
            changelog_content,
            format: input.format,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// Default value functions
// =============================================================================

fn default_api_type() -> String { "api".to_string() }
fn default_markdown() -> String { "markdown".to_string() }
fn default_docstring() -> String { "docstring".to_string() }
fn default_public() -> String { "public".to_string() }
fn default_complexity_one() -> i32 { 1 }
fn default_true() -> bool { true }
fn default_describes() -> String { "describes".to_string() }
fn default_keepachangelog() -> String { "keepachangelog".to_string() }
