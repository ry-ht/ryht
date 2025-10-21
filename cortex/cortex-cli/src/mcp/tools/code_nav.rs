//! Code Navigation Tools
//!
//! This module implements the 10 code navigation tools defined in the MCP spec.
//! These tools provide semantic code navigation capabilities using REAL data
//! from cortex-parser and semantic memory.

use async_trait::async_trait;
use cortex_core::id::CortexId;
use cortex_memory::CognitiveManager;
use cortex_storage::ConnectionManager;
use mcp_sdk::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;

// =============================================================================
// Shared Context
// =============================================================================

#[derive(Clone)]
pub struct CodeNavContext {
    storage: Arc<ConnectionManager>,
}

impl CodeNavContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self { storage }
    }

    fn get_cognitive_manager(&self) -> CognitiveManager {
        CognitiveManager::new(self.storage.clone())
    }
}

// =============================================================================
// cortex.code.get_unit
// =============================================================================

pub struct CodeGetUnitTool {
    ctx: CodeNavContext,
}

impl CodeGetUnitTool {
    pub fn new(ctx: CodeNavContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetUnitInput {
    unit_id: Option<String>,
    qualified_name: Option<String>,
    #[serde(default = "default_true")]
    include_body: bool,
    #[serde(default)]
    #[allow(dead_code)]
    include_ast: bool,
    #[serde(default)]
    include_dependencies: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, JsonSchema)]
struct GetUnitOutput {
    unit_id: String,
    unit_type: String,
    name: String,
    qualified_name: String,
    signature: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
    location: CodeLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    dependencies: Option<Vec<DependencyInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    docstring: Option<String>,
    visibility: String,
    modifiers: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    complexity: Option<ComplexityInfo>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct CodeLocation {
    file: String,
    start_line: usize,
    end_line: usize,
    start_column: usize,
    end_column: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
struct DependencyInfo {
    unit_id: String,
    dependency_type: String,
    qualified_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ComplexityInfo {
    cyclomatic: u32,
    cognitive: u32,
    nesting: u32,
    lines: u32,
}

#[async_trait]
impl Tool for CodeGetUnitTool {
    fn name(&self) -> &str {
        "cortex.code.get_unit"
    }

    fn description(&self) -> Option<&str> {
        Some("Retrieves a specific code unit (function, class, etc) with full details including signature, body, dependencies, and complexity metrics")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(GetUnitInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: GetUnitInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        // Validate input
        if input.unit_id.is_none() && input.qualified_name.is_none() {
            return Err(ToolError::ExecutionFailed(
                "Must provide either unit_id or qualified_name".to_string(),
            ));
        }

        let manager = self.ctx.get_cognitive_manager();
        let semantic = manager.semantic();

        // Retrieve the code unit
        let unit = if let Some(unit_id_str) = input.unit_id {
            let unit_id = CortexId::from_str(&unit_id_str)
                .map_err(|e| ToolError::ExecutionFailed(format!("Invalid unit_id: {}", e)))?;

            semantic.get_unit(unit_id).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Database error: {}", e)))?
        } else if let Some(qualified_name) = input.qualified_name {
            semantic.find_by_qualified_name(&qualified_name).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Database error: {}", e)))?
        } else {
            None
        };

        let unit = unit.ok_or_else(|| ToolError::ExecutionFailed("Code unit not found".to_string()))?;

        // Get dependencies if requested
        let dependencies = if input.include_dependencies {
            let deps = semantic.get_dependencies(unit.id).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get dependencies: {}", e)))?;

            let mut dep_infos = Vec::new();
            for dep in deps {
                if let Ok(Some(target_unit)) = semantic.get_unit(dep.target_id).await {
                    dep_infos.push(DependencyInfo {
                        unit_id: dep.target_id.to_string(),
                        dependency_type: format!("{:?}", dep.dependency_type),
                        qualified_name: target_unit.qualified_name,
                    });
                }
            }
            Some(dep_infos)
        } else {
            None
        };

        // Build output
        let output = GetUnitOutput {
            unit_id: unit.id.to_string(),
            unit_type: format!("{:?}", unit.unit_type),
            name: unit.name,
            qualified_name: unit.qualified_name,
            signature: unit.signature,
            body: if input.include_body { unit.body } else { None },
            location: CodeLocation {
                file: unit.file_path,
                start_line: unit.start_line,
                end_line: unit.end_line,
                start_column: unit.start_column,
                end_column: unit.end_column,
            },
            dependencies,
            docstring: unit.docstring,
            visibility: format!("{:?}", unit.visibility),
            modifiers: unit.modifiers,
            complexity: Some(ComplexityInfo {
                cyclomatic: unit.complexity.cyclomatic,
                cognitive: unit.complexity.cognitive,
                nesting: unit.complexity.nesting,
                lines: unit.complexity.lines,
            }),
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.list_units
// =============================================================================

pub struct CodeListUnitsTool {
    ctx: CodeNavContext,
}

impl CodeListUnitsTool {
    pub fn new(ctx: CodeNavContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ListUnitsInput {
    path: String,
    #[serde(default)]
    #[allow(dead_code)]
    workspace_id: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    recursive: bool,
    #[serde(default)]
    unit_types: Option<Vec<String>>,
    #[serde(default = "default_visibility")]
    visibility: String,
}

fn default_visibility() -> String {
    "all".to_string()
}

#[derive(Debug, Serialize, JsonSchema)]
struct ListUnitsOutput {
    units: Vec<UnitSummary>,
    total_count: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
struct UnitSummary {
    unit_id: String,
    unit_type: String,
    name: String,
    qualified_name: String,
    signature: String,
    location: CodeLocation,
    visibility: String,
    has_tests: bool,
    has_documentation: bool,
    complexity: ComplexityInfo,
}

#[async_trait]
impl Tool for CodeListUnitsTool {
    fn name(&self) -> &str {
        "cortex.code.list_units"
    }

    fn description(&self) -> Option<&str> {
        Some("Lists all code units in a file or directory with filtering options")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(ListUnitsInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: ListUnitsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let manager = self.ctx.get_cognitive_manager();
        let semantic = manager.semantic();

        // Get all units in the file
        let mut units = semantic.get_units_in_file(&input.path).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get units: {}", e)))?;

        // Filter by visibility if not "all"
        if input.visibility != "all" {
            units.retain(|u| format!("{:?}", u.visibility).to_lowercase() == input.visibility.to_lowercase());
        }

        // Filter by unit types if specified
        if let Some(ref types) = input.unit_types {
            units.retain(|u| {
                let unit_type = format!("{:?}", u.unit_type).to_lowercase();
                types.iter().any(|t| t.to_lowercase() == unit_type)
            });
        }

        // Convert to summaries
        let summaries: Vec<UnitSummary> = units.iter().map(|u| UnitSummary {
            unit_id: u.id.to_string(),
            unit_type: format!("{:?}", u.unit_type),
            name: u.name.clone(),
            qualified_name: u.qualified_name.clone(),
            signature: u.signature.clone(),
            location: CodeLocation {
                file: u.file_path.clone(),
                start_line: u.start_line,
                end_line: u.end_line,
                start_column: u.start_column,
                end_column: u.end_column,
            },
            visibility: format!("{:?}", u.visibility),
            has_tests: u.has_tests,
            has_documentation: u.has_documentation,
            complexity: ComplexityInfo {
                cyclomatic: u.complexity.cyclomatic,
                cognitive: u.complexity.cognitive,
                nesting: u.complexity.nesting,
                lines: u.complexity.lines,
            },
        }).collect();

        let output = ListUnitsOutput {
            total_count: summaries.len(),
            units: summaries,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.get_symbols
// =============================================================================

pub struct CodeGetSymbolsTool {
    ctx: CodeNavContext,
}

impl CodeGetSymbolsTool {
    pub fn new(ctx: CodeNavContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetSymbolsInput {
    scope: String,
    #[serde(default)]
    #[allow(dead_code)]
    workspace_id: Option<String>,
}

#[async_trait]
impl Tool for CodeGetSymbolsTool {
    fn name(&self) -> &str {
        "cortex.code.get_symbols"
    }

    fn description(&self) -> Option<&str> {
        Some("Gets all symbols (public members) in a scope (file or module)")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(GetSymbolsInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: GetSymbolsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let manager = self.ctx.get_cognitive_manager();
        let semantic = manager.semantic();

        // Get public symbols in the scope
        let units = semantic.get_units_in_file(&input.scope).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get units: {}", e)))?;

        // Filter to public symbols only
        let symbols: Vec<_> = units.into_iter()
            .filter(|u| format!("{:?}", u.visibility) == "Public")
            .map(|u| serde_json::json!({
                "unit_id": u.id.to_string(),
                "name": u.name,
                "qualified_name": u.qualified_name,
                "unit_type": format!("{:?}", u.unit_type),
                "signature": u.signature,
                "docstring": u.docstring,
            }))
            .collect();

        let output = serde_json::json!({
            "scope": input.scope,
            "symbols": symbols,
            "count": symbols.len(),
        });

        Ok(ToolResult::success_json(output))
    }
}

// =============================================================================
// cortex.code.find_definition
// =============================================================================

pub struct CodeFindDefinitionTool {
    ctx: CodeNavContext,
}

impl CodeFindDefinitionTool {
    pub fn new(ctx: CodeNavContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct FindDefinitionInput {
    symbol: String,
    #[serde(default)]
    context_file: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    workspace_id: Option<String>,
}

#[async_trait]
impl Tool for CodeFindDefinitionTool {
    fn name(&self) -> &str {
        "cortex.code.find_definition"
    }

    fn description(&self) -> Option<&str> {
        Some("Finds the definition of a symbol by name, optionally scoped to a context file")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(FindDefinitionInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: FindDefinitionInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let manager = self.ctx.get_cognitive_manager();
        let semantic = manager.semantic();

        // First try exact qualified name match
        if let Ok(Some(unit)) = semantic.find_by_qualified_name(&input.symbol).await {
            let output = serde_json::json!({
                "unit_id": unit.id.to_string(),
                "name": unit.name,
                "qualified_name": unit.qualified_name,
                "unit_type": format!("{:?}", unit.unit_type),
                "signature": unit.signature,
                "location": {
                    "file": unit.file_path,
                    "start_line": unit.start_line,
                    "end_line": unit.end_line,
                    "start_column": unit.start_column,
                    "end_column": unit.end_column,
                },
                "docstring": unit.docstring,
            });

            return Ok(ToolResult::success_json(output));
        }

        // If context file provided, search within that file
        if let Some(context_file) = input.context_file {
            let units = semantic.get_units_in_file(&context_file).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to search: {}", e)))?;

            // Find unit with matching name
            if let Some(unit) = units.into_iter().find(|u| u.name == input.symbol) {
                let output = serde_json::json!({
                    "unit_id": unit.id.to_string(),
                    "name": unit.name,
                    "qualified_name": unit.qualified_name,
                    "unit_type": format!("{:?}", unit.unit_type),
                    "signature": unit.signature,
                    "location": {
                        "file": unit.file_path,
                        "start_line": unit.start_line,
                        "end_line": unit.end_line,
                    },
                    "docstring": unit.docstring,
                });

                return Ok(ToolResult::success_json(output));
            }
        }

        Err(ToolError::ExecutionFailed(format!("Definition not found for symbol: {}", input.symbol)))
    }
}

// =============================================================================
// cortex.code.find_references
// =============================================================================

pub struct CodeFindReferencesTool {
    ctx: CodeNavContext,
}

impl CodeFindReferencesTool {
    pub fn new(ctx: CodeNavContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct FindReferencesInput {
    unit_id: Option<String>,
    qualified_name: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    workspace_id: Option<String>,
}

#[async_trait]
impl Tool for CodeFindReferencesTool {
    fn name(&self) -> &str {
        "cortex.code.find_references"
    }

    fn description(&self) -> Option<&str> {
        Some("Finds all references to a symbol (where it's called or used)")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(FindReferencesInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: FindReferencesInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        if input.unit_id.is_none() && input.qualified_name.is_none() {
            return Err(ToolError::ExecutionFailed(
                "Must provide either unit_id or qualified_name".to_string(),
            ));
        }

        let manager = self.ctx.get_cognitive_manager();
        let semantic = manager.semantic();

        // Get the target unit
        let unit_id = if let Some(unit_id_str) = input.unit_id {
            CortexId::from_str(&unit_id_str)
                .map_err(|e| ToolError::ExecutionFailed(format!("Invalid unit_id: {}", e)))?
        } else if let Some(qualified_name) = input.qualified_name {
            let unit = semantic.find_by_qualified_name(&qualified_name).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Database error: {}", e)))?
                .ok_or_else(|| ToolError::ExecutionFailed("Unit not found".to_string()))?;
            unit.id
        } else {
            return Err(ToolError::ExecutionFailed("No identifier provided".to_string()));
        };

        // Get all references (units that depend on this one)
        let reference_ids = semantic.find_references(unit_id).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to find references: {}", e)))?;

        // Fetch full unit details for each reference
        let mut references = Vec::new();
        for ref_id in reference_ids {
            if let Ok(Some(ref_unit)) = semantic.get_unit(ref_id).await {
                references.push(serde_json::json!({
                    "unit_id": ref_unit.id.to_string(),
                    "name": ref_unit.name,
                    "qualified_name": ref_unit.qualified_name,
                    "unit_type": format!("{:?}", ref_unit.unit_type),
                    "location": {
                        "file": ref_unit.file_path,
                        "start_line": ref_unit.start_line,
                        "end_line": ref_unit.end_line,
                    },
                }));
            }
        }

        let output = serde_json::json!({
            "references": references,
            "count": references.len(),
        });

        Ok(ToolResult::success_json(output))
    }
}

// =============================================================================
// cortex.code.get_signature
// =============================================================================

pub struct CodeGetSignatureTool {
    ctx: CodeNavContext,
}

impl CodeGetSignatureTool {
    pub fn new(ctx: CodeNavContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetSignatureInput {
    unit_id: String,
    #[serde(default)]
    #[allow(dead_code)]
    workspace_id: Option<String>,
}

#[async_trait]
impl Tool for CodeGetSignatureTool {
    fn name(&self) -> &str {
        "cortex.code.get_signature"
    }

    fn description(&self) -> Option<&str> {
        Some("Gets just the signature of a code unit (without body)")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(GetSignatureInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: GetSignatureInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let unit_id = CortexId::from_str(&input.unit_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid unit_id: {}", e)))?;

        let manager = self.ctx.get_cognitive_manager();
        let semantic = manager.semantic();

        let unit = semantic.get_unit(unit_id).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Database error: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed("Unit not found".to_string()))?;

        let output = serde_json::json!({
            "unit_id": unit.id.to_string(),
            "name": unit.name,
            "qualified_name": unit.qualified_name,
            "signature": unit.signature,
            "parameters": unit.parameters.iter().map(|p| serde_json::json!({
                "name": p.name,
                "param_type": p.param_type,
                "is_optional": p.is_optional,
                "default_value": p.default_value,
            })).collect::<Vec<_>>(),
            "return_type": unit.return_type,
            "visibility": format!("{:?}", unit.visibility),
            "modifiers": unit.modifiers,
        });

        Ok(ToolResult::success_json(output))
    }
}

// =============================================================================
// cortex.code.get_call_hierarchy
// =============================================================================

pub struct CodeGetCallHierarchyTool {
    ctx: CodeNavContext,
}

impl CodeGetCallHierarchyTool {
    pub fn new(ctx: CodeNavContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetCallHierarchyInput {
    unit_id: String,
    #[serde(default = "default_direction")]
    direction: String,
    #[serde(default = "default_depth")]
    #[allow(dead_code)]
    max_depth: usize,
    #[serde(default)]
    #[allow(dead_code)]
    workspace_id: Option<String>,
}

fn default_direction() -> String {
    "both".to_string()
}

fn default_depth() -> usize {
    3
}

#[async_trait]
impl Tool for CodeGetCallHierarchyTool {
    fn name(&self) -> &str {
        "cortex.code.get_call_hierarchy"
    }

    fn description(&self) -> Option<&str> {
        Some("Gets the call hierarchy (incoming/outgoing calls) for a function or method")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(GetCallHierarchyInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: GetCallHierarchyInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let unit_id = CortexId::from_str(&input.unit_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid unit_id: {}", e)))?;

        let manager = self.ctx.get_cognitive_manager();
        let semantic = manager.semantic();

        let mut outgoing = Vec::new();
        let mut incoming = Vec::new();

        // Get outgoing calls (what this unit calls)
        if input.direction == "outgoing" || input.direction == "both" {
            let deps = semantic.get_dependencies(unit_id).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get dependencies: {}", e)))?;

            for dep in deps {
                if format!("{:?}", dep.dependency_type).contains("Call") {
                    if let Ok(Some(target)) = semantic.get_unit(dep.target_id).await {
                        outgoing.push(serde_json::json!({
                            "unit_id": target.id.to_string(),
                            "name": target.name,
                            "qualified_name": target.qualified_name,
                            "file": target.file_path,
                            "line": target.start_line,
                        }));
                    }
                }
            }
        }

        // Get incoming calls (what calls this unit)
        if input.direction == "incoming" || input.direction == "both" {
            let ref_ids = semantic.find_references(unit_id).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to find references: {}", e)))?;

            for ref_id in ref_ids {
                if let Ok(Some(caller)) = semantic.get_unit(ref_id).await {
                    incoming.push(serde_json::json!({
                        "unit_id": caller.id.to_string(),
                        "name": caller.name,
                        "qualified_name": caller.qualified_name,
                        "file": caller.file_path,
                        "line": caller.start_line,
                    }));
                }
            }
        }

        let output = serde_json::json!({
            "unit_id": unit_id.to_string(),
            "outgoing_calls": outgoing,
            "incoming_calls": incoming,
            "outgoing_count": outgoing.len(),
            "incoming_count": incoming.len(),
        });

        Ok(ToolResult::success_json(output))
    }
}

// =============================================================================
// cortex.code.get_type_hierarchy
// =============================================================================

pub struct CodeGetTypeHierarchyTool {
    ctx: CodeNavContext,
}

impl CodeGetTypeHierarchyTool {
    pub fn new(ctx: CodeNavContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetTypeHierarchyInput {
    type_id: String,
    #[serde(default = "default_direction")]
    direction: String,
    #[serde(default)]
    #[allow(dead_code)]
    workspace_id: Option<String>,
}

#[async_trait]
impl Tool for CodeGetTypeHierarchyTool {
    fn name(&self) -> &str {
        "cortex.code.get_type_hierarchy"
    }

    fn description(&self) -> Option<&str> {
        Some("Gets the type inheritance hierarchy (supertypes/subtypes)")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(GetTypeHierarchyInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: GetTypeHierarchyInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let type_id = CortexId::from_str(&input.type_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid type_id: {}", e)))?;

        let manager = self.ctx.get_cognitive_manager();
        let semantic = manager.semantic();

        let mut supertypes = Vec::new();
        let mut subtypes = Vec::new();

        // Get supertypes (what this type extends/implements)
        if input.direction == "supertypes" || input.direction == "both" {
            let deps = semantic.get_dependencies(type_id).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get dependencies: {}", e)))?;

            for dep in deps {
                let dep_type_str = format!("{:?}", dep.dependency_type);
                if dep_type_str.contains("Extends") || dep_type_str.contains("Implements") {
                    if let Ok(Some(target)) = semantic.get_unit(dep.target_id).await {
                        supertypes.push(serde_json::json!({
                            "unit_id": target.id.to_string(),
                            "name": target.name,
                            "qualified_name": target.qualified_name,
                            "unit_type": format!("{:?}", target.unit_type),
                            "relationship": dep_type_str,
                        }));
                    }
                }
            }
        }

        // Get subtypes (what extends/implements this type)
        if input.direction == "subtypes" || input.direction == "both" {
            let dependents = semantic.get_dependents(type_id).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get dependents: {}", e)))?;

            for dep in dependents {
                let dep_type_str = format!("{:?}", dep.dependency_type);
                if dep_type_str.contains("Extends") || dep_type_str.contains("Implements") {
                    if let Ok(Some(source)) = semantic.get_unit(dep.source_id).await {
                        subtypes.push(serde_json::json!({
                            "unit_id": source.id.to_string(),
                            "name": source.name,
                            "qualified_name": source.qualified_name,
                            "unit_type": format!("{:?}", source.unit_type),
                            "relationship": dep_type_str,
                        }));
                    }
                }
            }
        }

        let output = serde_json::json!({
            "type_id": type_id.to_string(),
            "supertypes": supertypes,
            "subtypes": subtypes,
            "supertypes_count": supertypes.len(),
            "subtypes_count": subtypes.len(),
        });

        Ok(ToolResult::success_json(output))
    }
}

// =============================================================================
// cortex.code.get_imports
// =============================================================================

pub struct CodeGetImportsTool {
    ctx: CodeNavContext,
}

impl CodeGetImportsTool {
    pub fn new(ctx: CodeNavContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetImportsInput {
    file_path: String,
    #[serde(default)]
    #[allow(dead_code)]
    workspace_id: Option<String>,
}

#[async_trait]
impl Tool for CodeGetImportsTool {
    fn name(&self) -> &str {
        "cortex.code.get_imports"
    }

    fn description(&self) -> Option<&str> {
        Some("Gets all imports/dependencies in a file")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(GetImportsInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: GetImportsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let manager = self.ctx.get_cognitive_manager();
        let semantic = manager.semantic();

        // Get all units in the file
        let units = semantic.get_units_in_file(&input.file_path).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get units: {}", e)))?;

        let mut all_imports = Vec::new();

        // For each unit, get its imports
        for unit in units {
            let deps = semantic.get_dependencies(unit.id).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get dependencies: {}", e)))?;

            for dep in deps {
                if format!("{:?}", dep.dependency_type).contains("Import") {
                    if let Ok(Some(target)) = semantic.get_unit(dep.target_id).await {
                        all_imports.push(serde_json::json!({
                            "imported_name": target.name,
                            "qualified_name": target.qualified_name,
                            "source_file": target.file_path,
                            "imported_by": unit.name,
                        }));
                    }
                }
            }
        }

        // Deduplicate imports
        all_imports.sort_by(|a, b| {
            a["qualified_name"].as_str().cmp(&b["qualified_name"].as_str())
        });
        all_imports.dedup_by(|a, b| {
            a["qualified_name"] == b["qualified_name"]
        });

        let output = serde_json::json!({
            "file_path": input.file_path,
            "imports": all_imports,
            "count": all_imports.len(),
        });

        Ok(ToolResult::success_json(output))
    }
}

// =============================================================================
// cortex.code.get_exports
// =============================================================================

pub struct CodeGetExportsTool {
    ctx: CodeNavContext,
}

impl CodeGetExportsTool {
    pub fn new(ctx: CodeNavContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GetExportsInput {
    module_path: String,
    #[serde(default)]
    #[allow(dead_code)]
    workspace_id: Option<String>,
}

#[async_trait]
impl Tool for CodeGetExportsTool {
    fn name(&self) -> &str {
        "cortex.code.get_exports"
    }

    fn description(&self) -> Option<&str> {
        Some("Gets all exported symbols from a module or file")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(GetExportsInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: GetExportsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let manager = self.ctx.get_cognitive_manager();
        let semantic = manager.semantic();

        // Get all units in the module
        let units = semantic.get_units_in_file(&input.module_path).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get units: {}", e)))?;

        // Filter to exported units (public or explicitly exported)
        let exports: Vec<_> = units.into_iter()
            .filter(|u| u.is_exported || format!("{:?}", u.visibility) == "Public")
            .map(|u| serde_json::json!({
                "unit_id": u.id.to_string(),
                "name": u.name,
                "qualified_name": u.qualified_name,
                "unit_type": format!("{:?}", u.unit_type),
                "signature": u.signature,
                "is_default_export": u.is_default_export,
                "docstring": u.docstring,
            }))
            .collect();

        let output = serde_json::json!({
            "module_path": input.module_path,
            "exports": exports,
            "count": exports.len(),
        });

        Ok(ToolResult::success_json(output))
    }
}
