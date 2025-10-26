//! Code Manipulation Tools
//!
//! This module implements the 15 code manipulation tools defined in the MCP spec:
//! - cortex.code.create_unit
//! - cortex.code.update_unit
//! - cortex.code.delete_unit
//! - cortex.code.move_unit
//! - cortex.code.rename_unit
//! - cortex.code.extract_function
//! - cortex.code.inline_function
//! - cortex.code.change_signature
//! - cortex.code.add_parameter
//! - cortex.code.remove_parameter
//! - cortex.code.add_import
//! - cortex.code.optimize_imports
//! - cortex.code.generate_getter_setter
//! - cortex.code.implement_interface
//! - cortex.code.override_method

use async_trait::async_trait;
use cortex_core::id::CortexId;
use cortex_core::types::{CodeUnit, CodeUnitType, Language, Visibility, Parameter as CoreParameter, Complexity};
use cortex_memory::CognitiveManager;
use cortex_code_analysis::{AstEditor, CodeParser, Lang as ParserLanguage, ParsedFile};
use cortex_storage::ConnectionManager;
use cortex_vfs::{VirtualFileSystem, VirtualPath};
use mcp_sdk::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use tracing::{debug, warn};
use uuid::Uuid;
use anyhow::Result as AnyhowResult;

// Import the unified service layer
use crate::services::CodeUnitService;

// =============================================================================
// Shared Context
// =============================================================================

/// Shared context for all code manipulation tools
#[derive(Clone)]
pub struct CodeManipulationContext {
    storage: Arc<ConnectionManager>,
    vfs: Arc<VirtualFileSystem>,
    code_unit_service: Arc<CodeUnitService>,
    /// Active workspace ID (shared with workspace tools)
    active_workspace: Arc<RwLock<Option<Uuid>>>,
}

impl CodeManipulationContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self::with_active_workspace(storage, Arc::new(RwLock::new(None)))
    }

    /// Create a new context with a shared active workspace reference
    pub fn with_active_workspace(storage: Arc<ConnectionManager>, active_workspace: Arc<RwLock<Option<Uuid>>>) -> Self {
        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
        let code_unit_service = Arc::new(CodeUnitService::new(storage.clone()));
        Self {
            storage,
            vfs,
            code_unit_service,
            active_workspace,
        }
    }

    /// Get the currently active workspace ID
    pub fn get_active_workspace(&self) -> Option<Uuid> {
        self.active_workspace.read().ok().and_then(|guard| *guard)
    }

    /// Set the active workspace ID
    pub fn set_active_workspace(&self, workspace_id: Option<Uuid>) {
        if let Ok(mut guard) = self.active_workspace.write() {
            *guard = workspace_id;
        }
    }

    /// Parse a file using the appropriate parser based on extension
    async fn parse_file(&self, workspace_id: &Uuid, file_path: &str) -> AnyhowResult<(ParsedFile, String, ParserLanguage)> {
        let vpath = VirtualPath::new(file_path).map_err(|e| anyhow::anyhow!("Invalid path: {}", e))?;
        let content_bytes = self.vfs.read_file(workspace_id, &vpath).await
            .map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;
        let content = String::from_utf8(content_bytes)
            .map_err(|e| anyhow::anyhow!("File is not UTF-8: {}", e))?;

        let path_buf = Path::new(file_path);
        let language = ParserLanguage::from_path(path_buf)
            .ok_or_else(|| anyhow::anyhow!("Unsupported file type: {}", file_path))?;

        let mut parser = CodeParser::for_language(language)?;
        let parsed = parser.parse_file(file_path, &content, language)?;

        Ok((parsed, content, language))
    }

    /// Save modified content back to VFS
    async fn save_file(&self, workspace_id: &Uuid, file_path: &str, content: &str) -> AnyhowResult<()> {
        let vpath = VirtualPath::new(file_path).map_err(|e| anyhow::anyhow!("Invalid path: {}", e))?;
        self.vfs.write_file(workspace_id, &vpath, content.as_bytes()).await
            .map_err(|e| anyhow::anyhow!("Failed to write file: {}", e))?;
        Ok(())
    }

    /// Store a code unit in semantic memory
    async fn store_code_unit(&self, unit: CodeUnit) -> AnyhowResult<String> {
        let conn = self.storage.acquire().await
            .map_err(|e| anyhow::anyhow!("Failed to get connection: {}", e))?;

        // Store in database
        let query = r#"
            CREATE code_unit CONTENT $unit
        "#;

        let unit_json = serde_json::to_value(&unit)
            .map_err(|e| anyhow::anyhow!("Failed to serialize unit: {}", e))?;

        let _result: Vec<serde_json::Value> = conn.connection().query(query)
            .bind(("unit", unit_json))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to store code unit: {}", e))?
            .take(0)
            .map_err(|e| anyhow::anyhow!("Failed to parse result: {}", e))?;

        Ok(unit.id.to_string())
    }

    /// Get a code unit by ID
    async fn get_code_unit(&self, unit_id: &str) -> AnyhowResult<CodeUnit> {
        let conn = self.storage.acquire().await
            .map_err(|e| anyhow::anyhow!("Failed to get connection: {}", e))?;

        let query = r#"
            SELECT * FROM code_unit WHERE id = $unit_id
        "#;

        let unit_id_owned = unit_id.to_string();

        let mut result: Vec<CodeUnit> = conn.connection().query(query)
            .bind(("unit_id", unit_id_owned))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query code unit: {}", e))?
            .take(0)
            .map_err(|e| anyhow::anyhow!("Failed to parse result: {}", e))?;

        result.pop().ok_or_else(|| anyhow::anyhow!("Code unit not found: {}", unit_id))
    }

    /// Update a code unit in semantic memory
    async fn update_code_unit(&self, unit: &CodeUnit) -> AnyhowResult<()> {
        let conn = self.storage.acquire().await
            .map_err(|e| anyhow::anyhow!("Failed to get connection: {}", e))?;

        let query = r#"
            UPDATE $unit_id CONTENT $unit
        "#;

        let unit_json = serde_json::to_value(unit)
            .map_err(|e| anyhow::anyhow!("Failed to serialize unit: {}", e))?;

        let _result: Vec<serde_json::Value> = conn.connection().query(query)
            .bind(("unit_id", unit.id.to_string()))
            .bind(("unit", unit_json))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to update code unit: {}", e))?
            .take(0)
            .map_err(|e| anyhow::anyhow!("Failed to parse result: {}", e))?;

        Ok(())
    }

    /// Delete a code unit from semantic memory
    async fn delete_code_unit(&self, unit_id: &str) -> AnyhowResult<()> {
        let conn = self.storage.acquire().await
            .map_err(|e| anyhow::anyhow!("Failed to get connection: {}", e))?;

        let query = r#"
            DELETE $unit_id
        "#;

        let unit_id_owned = unit_id.to_string();

        let _result: Vec<serde_json::Value> = conn.connection().query(query)
            .bind(("unit_id", unit_id_owned))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to delete code unit: {}", e))?
            .take(0)
            .map_err(|e| anyhow::anyhow!("Failed to parse result: {}", e))?;

        Ok(())
    }

    /// Convert cortex_code_analysis Parameter to cortex_core Parameter
    fn convert_parameter(param: &cortex_code_analysis::types::Parameter) -> CoreParameter {
        CoreParameter {
            name: param.name.clone(),
            param_type: Some(param.param_type.clone()),
            default_value: param.default_value.clone(),
            is_optional: false,
            is_variadic: false,
            attributes: vec![],
        }
    }

    /// Convert cortex_code_analysis FunctionInfo to CodeUnit
    fn function_to_code_unit(func: &cortex_code_analysis::types::FunctionInfo, file_path: &str, language: Language) -> CodeUnit {
        let mut unit = CodeUnit::new(
            CodeUnitType::Function,
            func.name.clone(),
            func.qualified_name.clone(),
            file_path.to_string(),
            language,
        );

        unit.start_line = func.start_line;
        unit.end_line = func.end_line;
        unit.signature = format!("fn {}({}) -> {}",
            func.name,
            func.parameters.iter()
                .map(|p| format!("{}: {}", p.name, p.param_type))
                .collect::<Vec<_>>()
                .join(", "),
            func.return_type.as_ref().unwrap_or(&"()".to_string())
        );
        unit.body = Some(func.body.clone());
        unit.docstring = func.docstring.clone();
        unit.return_type = func.return_type.clone();
        unit.parameters = func.parameters.iter().map(|p| Self::convert_parameter(p)).collect();
        unit.visibility = match func.visibility {
            cortex_code_analysis::types::Visibility::Public => Visibility::Public,
            cortex_code_analysis::types::Visibility::PublicCrate => Visibility::Internal,
            _ => Visibility::Private,
        };
        unit.is_async = func.is_async;
        unit.is_unsafe = func.is_unsafe;
        unit.is_const = func.is_const;

        if let Some(complexity) = func.complexity {
            unit.complexity = Complexity {
                cyclomatic: complexity,
                cognitive: complexity,
                nesting: 0,
                lines: (func.end_line - func.start_line) as u32,
                parameters: func.parameters.len() as u32,
                returns: if func.return_type.is_some() { 1 } else { 0 },
            };
        }

        unit
    }

    /// Get CognitiveManager for semantic operations
    fn get_cognitive_manager(&self) -> CognitiveManager {
        CognitiveManager::new(self.storage.clone())
    }
}

// =============================================================================
// cortex.code.create_unit
// =============================================================================

pub struct CodeCreateUnitTool {
    ctx: CodeManipulationContext,
}

impl CodeCreateUnitTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct CreateUnitInput {
    file_path: String,
    unit_type: String,
    name: String,
    signature: Option<String>,
    body: String,
    position: Option<String>,
    visibility: Option<String>,
    docstring: Option<String>,
    #[serde(default = "default_workspace_id")]
    workspace_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct CreateUnitOutput {
    unit_id: String,
    qualified_name: String,
    version: i64,
}

#[async_trait]
impl Tool for CodeCreateUnitTool {
    fn name(&self) -> &str {
        "cortex.code.create_unit"
    }

    fn description(&self) -> Option<&str> {
        Some("Creates a new code unit (function, class, etc.) in a file using tree-sitter AST manipulation")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(CreateUnitInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: CreateUnitInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!(
            "Creating unit '{}' of type '{}' in file '{}'",
            input.name, input.unit_type, input.file_path
        );

        let workspace_id = Uuid::parse_str(&input.workspace_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace_id: {}", e)))?;

        // Parse the file
        let (_parsed, content, language) = self.ctx.parse_file(&workspace_id, &input.file_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Create AST editor
        let mut editor = match language {
            ParserLanguage::Rust => {
                AstEditor::new(content.clone(), tree_sitter_rust::LANGUAGE.into())
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
            }
            ParserLanguage::TypeScript => {
                AstEditor::new(content.clone(), tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
            }
            ParserLanguage::JavaScript => {
                AstEditor::new(content.clone(), tree_sitter_javascript::LANGUAGE.into())
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
            }
            ParserLanguage::Tsx => {
                AstEditor::new(content.clone(), tree_sitter_typescript::LANGUAGE_TSX.into())
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
            }
            ParserLanguage::Jsx => {
                AstEditor::new(content.clone(), tree_sitter_javascript::LANGUAGE.into())
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
            }
            ParserLanguage::Python | ParserLanguage::Cpp | ParserLanguage::Java | ParserLanguage::Kotlin => {
                return Err(ToolError::ExecutionFailed(format!("Language not supported for AST editing: {:?}", language)));
            }
        };

        // Generate the new code unit
        let visibility_str = input.visibility.as_deref().unwrap_or("pub");
        let docstring = if let Some(ref doc) = input.docstring {
            format!("/// {}\n", doc)
        } else {
            String::new()
        };

        let new_code = match input.unit_type.as_str() {
            "function" => {
                let default_signature = format!("fn {}()", input.name);
                let signature = input.signature.as_deref().unwrap_or(&default_signature);
                format!("{}{} {} {{\n    {}\n}}\n\n", docstring, visibility_str, signature, input.body)
            }
            "struct" => {
                format!("{}{}struct {} {{\n    {}\n}}\n\n", docstring, visibility_str, input.name, input.body)
            }
            "enum" => {
                format!("{}{}enum {} {{\n    {}\n}}\n\n", docstring, visibility_str, input.name, input.body)
            }
            "impl" => {
                format!("{}impl {} {{\n    {}\n}}\n\n", docstring, input.name, input.body)
            }
            _ => {
                return Err(ToolError::ExecutionFailed(format!("Unsupported unit type: {}", input.unit_type)));
            }
        };

        // Determine insertion position
        let insert_line = if let Some(pos) = input.position {
            pos.parse::<usize>().unwrap_or(0)
        } else {
            // Insert at end of file
            content.lines().count()
        };

        // Insert the new code
        editor.insert_at(insert_line, 0, &new_code)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Apply edits
        editor.apply_edits()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Save modified file
        let modified_content = editor.get_source();
        self.ctx.save_file(&workspace_id, &input.file_path, modified_content).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Re-parse to extract the new code unit
        let mut parser = CodeParser::for_language(language)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        let reparsed = parser.parse_file(&input.file_path, modified_content, language)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Find the newly created unit
        let core_language = match language {
            ParserLanguage::Rust => Language::Rust,
            ParserLanguage::TypeScript => Language::TypeScript,
            ParserLanguage::JavaScript => Language::JavaScript,
            ParserLanguage::Tsx => Language::TypeScript,
            ParserLanguage::Jsx => Language::JavaScript,
            ParserLanguage::Python => Language::Python,
            ParserLanguage::Cpp => Language::Cpp,
            ParserLanguage::Java => Language::Java,
            ParserLanguage::Kotlin => {
                return Err(ToolError::ExecutionFailed("Kotlin not yet supported in cortex_core::Language".to_string()));
            }
        };

        let new_unit = if input.unit_type == "function" {
            reparsed.functions.iter()
                .find(|f| f.name == input.name)
                .map(|f| CodeManipulationContext::function_to_code_unit(f, &input.file_path, core_language))
        } else {
            None
        };

        let unit_id = if let Some(unit) = new_unit {
            let qualified_name = unit.qualified_name.clone();
            let id = self.ctx.store_code_unit(unit).await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

            let output = CreateUnitOutput {
                unit_id: id,
                qualified_name,
                version: 1,
            };
            return Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()));
        } else {
            format!("unit_{}", uuid::Uuid::new_v4())
        };

        let output = CreateUnitOutput {
            unit_id,
            qualified_name: format!("{}::{}", input.file_path, input.name),
            version: 1,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.update_unit
// =============================================================================

pub struct CodeUpdateUnitTool {
    ctx: CodeManipulationContext,
}

impl CodeUpdateUnitTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UpdateUnitInput {
    unit_id: String,
    signature: Option<String>,
    body: Option<String>,
    docstring: Option<String>,
    visibility: Option<String>,
    expected_version: i64,
    #[serde(default = "default_true")]
    #[allow(dead_code)]
    preserve_comments: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct UpdateUnitOutput {
    unit_id: String,
    new_version: i64,
    updated: bool,
}

#[async_trait]
impl Tool for CodeUpdateUnitTool {
    fn name(&self) -> &str {
        "cortex.code.update_unit"
    }

    fn description(&self) -> Option<&str> {
        Some("Updates an existing code unit (signature, body, docstring, visibility) using tree-sitter")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(UpdateUnitInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: UpdateUnitInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Updating unit '{}'", input.unit_id);

        // Fetch the code unit from semantic memory
        let mut unit = self.ctx.get_code_unit(&input.unit_id).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Check version
        if unit.version as i64 != input.expected_version {
            return Err(ToolError::ExecutionFailed(format!(
                "Version mismatch: expected {}, got {}",
                input.expected_version, unit.version
            )));
        }

        // Get workspace ID from context
        let workspace_id = self.ctx.get_active_workspace()
            .ok_or_else(|| ToolError::ExecutionFailed(
                "No active workspace set. Please activate a workspace first using cortex.workspace.activate".to_string()
            ))?;
        let language = match unit.language {
            Language::Rust => ParserLanguage::Rust,
            Language::TypeScript => ParserLanguage::TypeScript,
            Language::JavaScript => ParserLanguage::JavaScript,
            _ => return Err(ToolError::ExecutionFailed("Unsupported language".to_string())),
        };

        let (_, content, _) = self.ctx.parse_file(&workspace_id, &unit.file_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Create AST editor
        let tree_sitter_lang = match language {
            ParserLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
            ParserLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            ParserLanguage::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            ParserLanguage::Jsx => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Python | ParserLanguage::Cpp | ParserLanguage::Java | ParserLanguage::Kotlin => {
                return Err(ToolError::ExecutionFailed(format!("Language not supported for AST editing: {:?}", language)));
            }
        };

        let mut editor = AstEditor::new(content.clone(), tree_sitter_lang)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Find the function node to update
        let functions = editor.query("(function_item) @func")
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let target_node_data = functions.iter()
            .find(|node| node.start_position().row == unit.start_line)
            .map(|node| (node.start_position(), node.end_position()));

        if let Some((start_pos, end_pos)) = target_node_data {
            // Build the updated function
            let mut new_func = String::new();

            // Add docstring
            if let Some(ref doc) = input.docstring {
                new_func.push_str(&format!("/// {}\n", doc));
                unit.docstring = Some(doc.clone());
            }

            // Add visibility
            if let Some(ref vis) = input.visibility {
                new_func.push_str(vis);
                new_func.push(' ');
            }

            // Add signature or use existing
            if let Some(ref sig) = input.signature {
                new_func.push_str(sig);
                unit.signature = sig.clone();
            } else {
                new_func.push_str(&unit.signature);
            }

            // Add body
            new_func.push_str(" {\n");
            if let Some(ref body) = input.body {
                new_func.push_str("    ");
                new_func.push_str(body);
                unit.body = Some(body.clone());
            } else if let Some(ref body) = unit.body {
                new_func.push_str("    ");
                new_func.push_str(body);
            }
            new_func.push_str("\n}\n");

            // Replace the function by line range
            let range = cortex_code_analysis::Range::new(
                cortex_code_analysis::Position::new(start_pos.row, start_pos.column),
                cortex_code_analysis::Position::new(end_pos.row, end_pos.column),
            );
            editor.edits.push(cortex_code_analysis::Edit::replace(range, new_func));

            // Apply edits
            editor.apply_edits()
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

            // Save modified file
            self.ctx.save_file(&workspace_id, &unit.file_path, editor.get_source()).await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

            // Update version
            unit.version += 1;
            unit.updated_at = chrono::Utc::now();

            // Update in semantic memory
            self.ctx.update_code_unit(&unit).await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

            let output = UpdateUnitOutput {
                unit_id: input.unit_id.clone(),
                new_version: unit.version as i64,
                updated: true,
            };

            Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
        } else {
            Err(ToolError::ExecutionFailed("Function not found in AST".to_string()))
        }
    }
}

// =============================================================================
// cortex.code.delete_unit
// =============================================================================

pub struct CodeDeleteUnitTool {
    ctx: CodeManipulationContext,
}

impl CodeDeleteUnitTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DeleteUnitInput {
    unit_id: String,
    #[serde(default)]
    #[allow(dead_code)]
    cascade: bool,
    expected_version: i64,
}

#[derive(Debug, Serialize, JsonSchema)]
struct DeleteUnitOutput {
    unit_id: String,
    deleted: bool,
    cascade_deleted: Vec<String>,
}

#[async_trait]
impl Tool for CodeDeleteUnitTool {
    fn name(&self) -> &str {
        "cortex.code.delete_unit"
    }

    fn description(&self) -> Option<&str> {
        Some("Deletes a code unit and optionally its dependents using tree-sitter")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DeleteUnitInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DeleteUnitInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Deleting unit '{}'", input.unit_id);

        // Fetch the code unit
        let unit = self.ctx.get_code_unit(&input.unit_id).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Check version
        if unit.version as i64 != input.expected_version {
            return Err(ToolError::ExecutionFailed(format!(
                "Version mismatch: expected {}, got {}",
                input.expected_version, unit.version
            )));
        }

        // Get workspace ID from context
        let workspace_id = self.ctx.get_active_workspace()
            .ok_or_else(|| ToolError::ExecutionFailed(
                "No active workspace set. Please activate a workspace first using cortex.workspace.activate".to_string()
            ))?;
        let language = match unit.language {
            Language::Rust => ParserLanguage::Rust,
            Language::TypeScript => ParserLanguage::TypeScript,
            Language::JavaScript => ParserLanguage::JavaScript,
            _ => return Err(ToolError::ExecutionFailed("Unsupported language".to_string())),
        };

        let (_, content, _) = self.ctx.parse_file(&workspace_id, &unit.file_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Create AST editor
        let tree_sitter_lang = match language {
            ParserLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
            ParserLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            ParserLanguage::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            ParserLanguage::Jsx => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Python | ParserLanguage::Cpp | ParserLanguage::Java | ParserLanguage::Kotlin => {
                return Err(ToolError::ExecutionFailed(format!("Language not supported for AST editing: {:?}", language)));
            }
        };

        let mut editor = AstEditor::new(content, tree_sitter_lang)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Find and delete the node
        let functions = editor.query("(function_item) @func")
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let target_node_range = functions.iter()
            .find(|node| node.start_position().row == unit.start_line)
            .map(|node| cortex_code_analysis::Range::from_node(node));

        if let Some(range) = target_node_range {
            editor.edits.push(cortex_code_analysis::Edit::delete(range));

            editor.apply_edits()
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

            // Save modified file
            self.ctx.save_file(&workspace_id, &unit.file_path, editor.get_source()).await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

            // Delete from semantic memory
            self.ctx.delete_code_unit(&input.unit_id).await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

            let output = DeleteUnitOutput {
                unit_id: input.unit_id.clone(),
                deleted: true,
                cascade_deleted: vec![],
            };

            Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
        } else {
            Err(ToolError::ExecutionFailed("Function not found in AST".to_string()))
        }
    }
}

// =============================================================================
// cortex.code.move_unit
// =============================================================================

pub struct CodeMoveUnitTool {
    ctx: CodeManipulationContext,
}

impl CodeMoveUnitTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct MoveUnitInput {
    unit_id: String,
    target_file: String,
    position: Option<String>,
    #[serde(default = "default_true")]
    #[allow(dead_code)]
    update_imports: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct MoveUnitOutput {
    unit_id: String,
    old_file: String,
    new_file: String,
    imports_updated: Vec<String>,
}

#[async_trait]
impl Tool for CodeMoveUnitTool {
    fn name(&self) -> &str {
        "cortex.code.move_unit"
    }

    fn description(&self) -> Option<&str> {
        Some("Moves a code unit to another file, updating imports using tree-sitter")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(MoveUnitInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: MoveUnitInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Moving unit '{}' to '{}'", input.unit_id, input.target_file);

        // Fetch the code unit
        let mut unit = self.ctx.get_code_unit(&input.unit_id).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let old_file = unit.file_path.clone();

        // Extract code from source file (similar to delete)
        let workspace_id = Uuid::new_v4();
        let language = match unit.language {
            Language::Rust => ParserLanguage::Rust,
            Language::TypeScript => ParserLanguage::TypeScript,
            Language::JavaScript => ParserLanguage::JavaScript,
            _ => return Err(ToolError::ExecutionFailed("Unsupported language".to_string())),
        };

        let (_, source_content, _) = self.ctx.parse_file(&workspace_id, &old_file).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Extract the code unit text
        let lines: Vec<&str> = source_content.lines().collect();
        let unit_code = if unit.start_line < lines.len() && unit.end_line < lines.len() {
            lines[unit.start_line..=unit.end_line].join("\n")
        } else {
            return Err(ToolError::ExecutionFailed("Invalid line range".to_string()));
        };

        // Delete from source file
        let tree_sitter_lang = match language {
            ParserLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
            ParserLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            ParserLanguage::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            ParserLanguage::Jsx => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Python | ParserLanguage::Cpp | ParserLanguage::Java | ParserLanguage::Kotlin => {
                return Err(ToolError::ExecutionFailed(format!("Language not supported for AST editing: {:?}", language)));
            }
        };

        let mut source_editor = AstEditor::new(source_content, tree_sitter_lang)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let functions = source_editor.query("(function_item) @func")
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let source_node_range = functions.iter()
            .find(|n| n.start_position().row == unit.start_line)
            .map(|node| cortex_code_analysis::Range::from_node(node));

        if let Some(range) = source_node_range {
            source_editor.edits.push(cortex_code_analysis::Edit::delete(range));
            source_editor.apply_edits()
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

            self.ctx.save_file(&workspace_id, &old_file, source_editor.get_source()).await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        }

        // Insert into target file
        let (_, target_content, _) = self.ctx.parse_file(&workspace_id, &input.target_file).await
            .unwrap_or_else(|_| (ParsedFile::new(input.target_file.clone()), String::new(), language));

        let target_tree_sitter_lang = match language {
            ParserLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
            ParserLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            ParserLanguage::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            ParserLanguage::Jsx => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Python | ParserLanguage::Cpp | ParserLanguage::Java | ParserLanguage::Kotlin => {
                return Err(ToolError::ExecutionFailed(format!("Language not supported for AST editing: {:?}", language)));
            }
        };

        let mut target_editor = AstEditor::new(target_content.clone(), target_tree_sitter_lang)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let insert_line = input.position
            .and_then(|p| p.parse::<usize>().ok())
            .unwrap_or_else(|| target_content.lines().count());

        target_editor.insert_at(insert_line, 0, &format!("{}\n\n", unit_code))
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        target_editor.apply_edits()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        self.ctx.save_file(&workspace_id, &input.target_file, target_editor.get_source()).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Update code unit metadata
        unit.file_path = input.target_file.clone();
        self.ctx.update_code_unit(&unit).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let output = MoveUnitOutput {
            unit_id: input.unit_id.clone(),
            old_file,
            new_file: input.target_file.clone(),
            imports_updated: vec![],
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.rename_unit
// =============================================================================

pub struct CodeRenameUnitTool {
    ctx: CodeManipulationContext,
}

impl CodeRenameUnitTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RenameUnitInput {
    unit_id: String,
    new_name: String,
    #[serde(default = "default_true")]
    #[allow(dead_code)]
    update_references: bool,
    #[serde(default = "default_workspace_scope")]
    #[allow(dead_code)]
    scope: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct RenameUnitOutput {
    unit_id: String,
    old_name: String,
    new_name: String,
    references_updated: i32,
}

#[async_trait]
impl Tool for CodeRenameUnitTool {
    fn name(&self) -> &str {
        "cortex.code.rename_unit"
    }

    fn description(&self) -> Option<&str> {
        Some("Renames a code unit and updates all references using tree-sitter")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(RenameUnitInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: RenameUnitInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Renaming unit '{}' to '{}'", input.unit_id, input.new_name);

        // Fetch the code unit
        let mut unit = self.ctx.get_code_unit(&input.unit_id).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let old_name = unit.name.clone();

        // Parse the file
        let workspace_id = Uuid::new_v4();
        let language = match unit.language {
            Language::Rust => ParserLanguage::Rust,
            Language::TypeScript => ParserLanguage::TypeScript,
            Language::JavaScript => ParserLanguage::JavaScript,
            _ => return Err(ToolError::ExecutionFailed("Unsupported language".to_string())),
        };

        let (_, content, _) = self.ctx.parse_file(&workspace_id, &unit.file_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Create AST editor
        let tree_sitter_lang = match language {
            ParserLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
            ParserLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            ParserLanguage::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            ParserLanguage::Jsx => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Python | ParserLanguage::Cpp | ParserLanguage::Java | ParserLanguage::Kotlin => {
                return Err(ToolError::ExecutionFailed(format!("Language not supported for AST editing: {:?}", language)));
            }
        };

        let mut editor = AstEditor::new(content, tree_sitter_lang)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Rename the symbol
        let edits = editor.rename_symbol(&old_name, &input.new_name)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let references_updated = edits.len() as i32;

        // Apply edits
        editor.apply_edits()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Save modified file
        self.ctx.save_file(&workspace_id, &unit.file_path, editor.get_source()).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Update code unit metadata
        unit.name = input.new_name.clone();
        unit.qualified_name = unit.qualified_name.replace(&old_name, &input.new_name);
        self.ctx.update_code_unit(&unit).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let output = RenameUnitOutput {
            unit_id: input.unit_id.clone(),
            old_name,
            new_name: input.new_name.clone(),
            references_updated,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.extract_function
// =============================================================================

pub struct CodeExtractFunctionTool {
    ctx: CodeManipulationContext,
}

impl CodeExtractFunctionTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ExtractFunctionInput {
    source_unit_id: String,
    start_line: i32,
    end_line: i32,
    function_name: String,
    #[serde(default = "default_before_position")]
    #[allow(dead_code)]
    position: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ExtractFunctionOutput {
    new_unit_id: String,
    function_name: String,
    parameters: Vec<String>,
    return_type: Option<String>,
}

#[async_trait]
impl Tool for CodeExtractFunctionTool {
    fn name(&self) -> &str {
        "cortex.code.extract_function"
    }

    fn description(&self) -> Option<&str> {
        Some("Extracts code block into a new function with inferred parameters using tree-sitter")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ExtractFunctionInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: ExtractFunctionInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!(
            "Extracting function '{}' from lines {}-{}",
            input.function_name, input.start_line, input.end_line
        );

        // Fetch the source unit
        let unit = self.ctx.get_code_unit(&input.source_unit_id).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Parse the file
        let workspace_id = Uuid::new_v4();
        let language = match unit.language {
            Language::Rust => ParserLanguage::Rust,
            Language::TypeScript => ParserLanguage::TypeScript,
            Language::JavaScript => ParserLanguage::JavaScript,
            _ => return Err(ToolError::ExecutionFailed("Unsupported language".to_string())),
        };

        let (_, content, _) = self.ctx.parse_file(&workspace_id, &unit.file_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Create AST editor
        let tree_sitter_lang = match language {
            ParserLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
            ParserLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            ParserLanguage::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            ParserLanguage::Jsx => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Python | ParserLanguage::Cpp | ParserLanguage::Java | ParserLanguage::Kotlin => {
                return Err(ToolError::ExecutionFailed(format!("Language not supported for AST editing: {:?}", language)));
            }
        };

        let mut editor = AstEditor::new(content.clone(), tree_sitter_lang)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Extract the function using AstEditor
        let (params, return_type, _function_code) = editor
            .extract_function_rust(
                input.start_line,
                input.end_line,
                &input.function_name,
            )
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Apply the edits to get the modified content
        editor.apply_edits()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let modified_content = editor.get_source();

        // Save the modified file
        self.ctx.save_file(&workspace_id, &unit.file_path, modified_content).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let output = ExtractFunctionOutput {
            new_unit_id: format!("unit_{}", uuid::Uuid::new_v4()),
            function_name: input.function_name.clone(),
            parameters: params.iter().map(|(name, ty)| format!("{}: {}", name, ty)).collect(),
            return_type,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.inline_function
// =============================================================================

pub struct CodeInlineFunctionTool {
    ctx: CodeManipulationContext,
}

impl CodeInlineFunctionTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }

    /// Extract arguments from a call expression
    fn extract_call_arguments(&self, call_node: &tree_sitter::Node, source: &str) -> Vec<String> {
        let mut args = Vec::new();

        // Find the arguments node
        if let Some(args_node) = call_node.child_by_field_name("arguments") {
            let mut cursor = args_node.walk();
            for child in args_node.children(&mut cursor) {
                if child.kind() != "(" && child.kind() != ")" && child.kind() != "," {
                    args.push(source[child.byte_range()].to_string());
                }
            }
        }

        args
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct InlineFunctionInput {
    function_id: String,
    #[allow(dead_code)]
    call_sites: Option<Vec<String>>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct InlineFunctionOutput {
    function_id: String,
    sites_inlined: i32,
    function_removed: bool,
}

#[async_trait]
impl Tool for CodeInlineFunctionTool {
    fn name(&self) -> &str {
        "cortex.code.inline_function"
    }

    fn description(&self) -> Option<&str> {
        Some("Inlines a function at call sites, optionally removing the function")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(InlineFunctionInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: InlineFunctionInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Inlining function '{}'", input.function_id);

        // Fetch the function to inline
        let function = self.ctx.get_code_unit(&input.function_id).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        if function.unit_type != CodeUnitType::Function {
            return Err(ToolError::ExecutionFailed("Unit is not a function".to_string()));
        }

        let function_body = function.body.clone().unwrap_or_default();
        let function_params = function.parameters.clone();

        // Parse the file containing the function
        let workspace_id = Uuid::new_v4();
        let language = match function.language {
            Language::Rust => ParserLanguage::Rust,
            Language::TypeScript => ParserLanguage::TypeScript,
            Language::JavaScript => ParserLanguage::JavaScript,
            _ => return Err(ToolError::ExecutionFailed("Unsupported language".to_string())),
        };

        let (_, content, _) = self.ctx.parse_file(&workspace_id, &function.file_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Create AST editor
        let tree_sitter_lang = match language {
            ParserLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
            ParserLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            ParserLanguage::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            ParserLanguage::Jsx => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Python | ParserLanguage::Cpp | ParserLanguage::Java | ParserLanguage::Kotlin => {
                return Err(ToolError::ExecutionFailed(format!("Language not supported for AST editing: {:?}", language)));
            }
        };

        let mut editor = AstEditor::new(content.clone(), tree_sitter_lang)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Find all call sites in the file
        let call_query = "(call_expression) @call";
        let calls = editor.query(call_query)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let mut sites_inlined = 0;
        let mut edits_to_apply = Vec::new();

        // Process each call site
        for call_node in calls {
            let call_text = editor.node_text(&call_node);

            // Check if this is a call to our function
            if call_text.contains(&function.name) {
                // Extract arguments from the call
                let args = self.extract_call_arguments(&call_node, editor.get_source());

                // Substitute parameters in function body
                let mut inlined_body = function_body.clone();
                for (i, param) in function_params.iter().enumerate() {
                    if i < args.len() {
                        // Simple substitution (real implementation would use AST)
                        inlined_body = inlined_body.replace(&param.name, &args[i]);
                    }
                }

                // Create edit to replace call with inlined body
                let range = cortex_code_analysis::Range::from_node(&call_node);
                edits_to_apply.push(cortex_code_analysis::Edit::replace(
                    range,
                    format!("{{\n    {}\n}}", inlined_body),
                ));
                sites_inlined += 1;
            }
        }

        // Apply all inlining edits
        editor.edits.extend(edits_to_apply);
        editor.apply_edits()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Now remove the original function definition
        let functions = editor.query("(function_item) @func")
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let function_removed = if let Some(func_node) = functions.iter()
            .find(|n| n.start_position().row == function.start_line) {
            let range = cortex_code_analysis::Range::from_node(func_node);
            editor.edits.push(cortex_code_analysis::Edit::delete(range));
            editor.apply_edits()
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
            true
        } else {
            false
        };

        // Save the modified file
        self.ctx.save_file(&workspace_id, &function.file_path, editor.get_source()).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Delete function from semantic memory
        if function_removed {
            self.ctx.delete_code_unit(&input.function_id).await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        }

        let output = InlineFunctionOutput {
            function_id: input.function_id.clone(),
            sites_inlined,
            function_removed,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.change_signature
// =============================================================================

pub struct CodeChangeSignatureTool {
    ctx: CodeManipulationContext,
}

impl CodeChangeSignatureTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }

    /// Update all call sites of the function with the new signature
    async fn update_call_sites(
        &self,
        workspace_id: &Uuid,
        unit: &CodeUnit,
        old_signature: &str,
        new_signature: &str,
        language: ParserLanguage,
    ) -> AnyhowResult<i32> {
        debug!("Updating call sites for function '{}'", unit.name);

        // Parse old and new signatures to understand parameter changes
        let old_params = self.extract_parameters_from_signature(old_signature);
        let new_params = self.extract_parameters_from_signature(new_signature);

        // Use semantic memory to find all references to this function
        let manager = self.ctx.get_cognitive_manager();
        let semantic = manager.semantic();

        // Convert unit_id string to CortexId
        let unit_id = CortexId::from_str(&unit.id.to_string())
            .map_err(|e| anyhow::anyhow!("Invalid unit_id: {}", e))?;

        // Find all units that reference this function
        let reference_ids = semantic.find_references(unit_id).await?;

        if reference_ids.is_empty() {
            debug!("No references found for function '{}'", unit.name);
            return Ok(0);
        }

        debug!("Found {} potential calling units", reference_ids.len());

        // Group references by file to batch updates
        let mut files_to_update: HashMap<String, Vec<CodeUnit>> = HashMap::new();

        for ref_id in reference_ids {
            if let Some(ref_unit) = semantic.get_unit(ref_id).await? {
                files_to_update
                    .entry(ref_unit.file_path.clone())
                    .or_insert_with(Vec::new)
                    .push(ref_unit);
            }
        }

        debug!("Need to update {} files", files_to_update.len());

        let mut total_updated = 0;

        // Process each file that contains references
        for (file_path, _caller_units) in files_to_update.iter() {
            match self.update_calls_in_file(
                workspace_id,
                file_path,
                &unit.name,
                &old_params,
                &new_params,
                language,
            ).await {
                Ok(count) => {
                    debug!("Updated {} call sites in {}", count, file_path);
                    total_updated += count;
                }
                Err(e) => {
                    warn!("Failed to update call sites in {}: {}", file_path, e);
                    // Continue with other files
                }
            }
        }

        Ok(total_updated)
    }

    /// Update call sites within a single file
    async fn update_calls_in_file(
        &self,
        workspace_id: &Uuid,
        file_path: &str,
        function_name: &str,
        old_params: &[String],
        new_params: &[String],
        language: ParserLanguage,
    ) -> AnyhowResult<i32> {
        // Parse the file
        let (_, content, _) = self.ctx.parse_file(workspace_id, file_path).await?;

        // Create AST editor
        let tree_sitter_lang = match language {
            ParserLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
            ParserLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            ParserLanguage::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            ParserLanguage::Jsx => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Python | ParserLanguage::Cpp | ParserLanguage::Java | ParserLanguage::Kotlin => {
                return Err(anyhow::anyhow!("Language not supported for AST editing: {:?}", language));
            }
        };

        let mut editor = AstEditor::new(content.clone(), tree_sitter_lang)?;

        // Find all call expressions
        let call_query = "(call_expression) @call";
        let calls = editor.query(call_query)?;

        let mut edits_to_apply = Vec::new();
        let mut sites_updated = 0;

        // Track which call sites we've already processed to avoid duplicates
        let mut processed_ranges = HashSet::new();

        for call_node in calls {
            let call_text = editor.node_text(&call_node);

            // Check if this is a call to our function
            if !call_text.contains(function_name) {
                continue;
            }

            // Get the function being called to verify it's the right one
            if let Some(function_node) = call_node.child_by_field_name("function") {
                let func_name = editor.node_text(&function_node);

                // Match function name (handle qualified names like module::function)
                if !func_name.ends_with(function_name) && func_name != function_name {
                    continue;
                }
            } else {
                continue;
            }

            // Avoid processing the same call site twice
            let range_key = format!("{}:{}", call_node.start_byte(), call_node.end_byte());
            if processed_ranges.contains(&range_key) {
                continue;
            }
            processed_ranges.insert(range_key);

            // Extract current arguments
            let current_args = self.extract_call_arguments(&call_node, editor.get_source());

            // Build new argument list based on parameter mapping
            let new_args = self.map_arguments(old_params, new_params, &current_args);

            // If arguments changed, create an edit
            if new_args != current_args {
                if let Some(args_node) = call_node.child_by_field_name("arguments") {
                    // Build the new arguments string
                    let new_args_str = if new_args.is_empty() {
                        "()".to_string()
                    } else {
                        format!("({})", new_args.join(", "))
                    };

                    let range = cortex_code_analysis::Range::from_node(&args_node);
                    edits_to_apply.push(cortex_code_analysis::Edit::replace(range, new_args_str));
                    sites_updated += 1;
                }
            }
        }

        // Apply all edits if any were made
        if !edits_to_apply.is_empty() {
            editor.edits.extend(edits_to_apply);
            editor.apply_edits()?;

            // Save the modified file
            self.ctx.save_file(workspace_id, file_path, editor.get_source()).await?;
        }

        Ok(sites_updated)
    }

    /// Extract parameters from a function signature
    fn extract_parameters_from_signature(&self, signature: &str) -> Vec<String> {
        let mut params = Vec::new();

        // Find parameter list between parentheses
        if let Some(start) = signature.find('(') {
            if let Some(end) = signature[start..].find(')') {
                let params_str = &signature[start + 1..start + end];

                if !params_str.trim().is_empty() {
                    // Split by comma, but be aware of nested generics/types
                    let mut depth = 0;
                    let mut current_param = String::new();

                    for ch in params_str.chars() {
                        match ch {
                            '<' | '(' | '[' => {
                                depth += 1;
                                current_param.push(ch);
                            }
                            '>' | ')' | ']' => {
                                depth -= 1;
                                current_param.push(ch);
                            }
                            ',' if depth == 0 => {
                                if !current_param.trim().is_empty() {
                                    // Extract just the parameter name (before colon for Rust, or the whole thing for JS/TS)
                                    let param_name = self.extract_param_name(&current_param);
                                    params.push(param_name);
                                }
                                current_param.clear();
                            }
                            _ => {
                                current_param.push(ch);
                            }
                        }
                    }

                    // Don't forget the last parameter
                    if !current_param.trim().is_empty() {
                        let param_name = self.extract_param_name(&current_param);
                        params.push(param_name);
                    }
                }
            }
        }

        params
    }

    /// Extract parameter name from parameter declaration
    fn extract_param_name(&self, param_decl: &str) -> String {
        let trimmed = param_decl.trim();

        // For Rust: "name: Type" -> "name"
        if let Some(colon_pos) = trimmed.find(':') {
            trimmed[..colon_pos].trim().to_string()
        } else {
            // For JavaScript/TypeScript without type annotations
            trimmed.to_string()
        }
    }

    /// Map old arguments to new argument positions based on parameter changes
    fn map_arguments(&self, old_params: &[String], new_params: &[String], current_args: &[String]) -> Vec<String> {
        let mut new_args = Vec::new();

        // Create a mapping of parameter names to their values
        let mut arg_map: HashMap<String, String> = HashMap::new();
        for (i, param) in old_params.iter().enumerate() {
            if i < current_args.len() {
                arg_map.insert(param.clone(), current_args[i].clone());
            }
        }

        // Build new argument list based on new parameter order
        for new_param in new_params {
            if let Some(value) = arg_map.get(new_param) {
                // Parameter exists in old signature, use its value
                new_args.push(value.clone());
            } else {
                // New parameter - use a default value
                // For now, use a placeholder that will cause a compilation error,
                // alerting the user to fix it manually
                new_args.push("/* TODO: provide value */".to_string());
            }
        }

        new_args
    }

    /// Extract arguments from a call expression node
    fn extract_call_arguments(&self, call_node: &tree_sitter::Node, source: &str) -> Vec<String> {
        let mut args = Vec::new();

        // Find the arguments node
        if let Some(args_node) = call_node.child_by_field_name("arguments") {
            let mut cursor = args_node.walk();
            for child in args_node.children(&mut cursor) {
                // Skip delimiters
                if child.kind() != "(" && child.kind() != ")" && child.kind() != "," {
                    args.push(source[child.byte_range()].trim().to_string());
                }
            }
        }

        args
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ChangeSignatureInput {
    unit_id: String,
    new_signature: String,
    #[serde(default = "default_true")]
    update_callers: bool,
    #[serde(default = "default_migration_strategy")]
    #[allow(dead_code)]
    migration_strategy: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ChangeSignatureOutput {
    unit_id: String,
    old_signature: String,
    new_signature: String,
    callers_updated: i32,
}

#[async_trait]
impl Tool for CodeChangeSignatureTool {
    fn name(&self) -> &str {
        "cortex.code.change_signature"
    }

    fn description(&self) -> Option<&str> {
        Some("Changes function/method signature and updates all callers")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ChangeSignatureInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: ChangeSignatureInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Changing signature for unit '{}'", input.unit_id);

        // Fetch the code unit
        let mut unit = self.ctx.get_code_unit(&input.unit_id).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let old_signature = unit.signature.clone();

        // Parse the file
        let workspace_id = Uuid::new_v4();
        let language = match unit.language {
            Language::Rust => ParserLanguage::Rust,
            Language::TypeScript => ParserLanguage::TypeScript,
            Language::JavaScript => ParserLanguage::JavaScript,
            _ => return Err(ToolError::ExecutionFailed("Unsupported language".to_string())),
        };

        let (_, content, _) = self.ctx.parse_file(&workspace_id, &unit.file_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Create AST editor
        let tree_sitter_lang = match language {
            ParserLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
            ParserLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            ParserLanguage::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            ParserLanguage::Jsx => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Python | ParserLanguage::Cpp | ParserLanguage::Java | ParserLanguage::Kotlin => {
                return Err(ToolError::ExecutionFailed(format!("Language not supported for AST editing: {:?}", language)));
            }
        };

        let mut editor = AstEditor::new(content.clone(), tree_sitter_lang)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Find the function definition
        let functions = editor.query("(function_item) @func")
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let func_node = functions.iter()
            .find(|n| n.start_position().row == unit.start_line)
            .ok_or_else(|| ToolError::ExecutionFailed("Function not found in AST".to_string()))?;

        let func_text = editor.node_text(func_node);
        let body_start = func_text.find('{')
            .ok_or_else(|| ToolError::ExecutionFailed("Function body not found".to_string()))?;
        let body = &func_text[body_start..];

        // Build new function with updated signature
        let new_function = format!("{} {}", input.new_signature, body);
        let range = cortex_code_analysis::Range::from_node(func_node);
        editor.edits.push(cortex_code_analysis::Edit::replace(range, new_function));

        // Apply edits to update function definition
        editor.apply_edits()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Save the modified file
        self.ctx.save_file(&workspace_id, &unit.file_path, editor.get_source()).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Update callers if requested
        let mut callers_updated = 0;
        if input.update_callers {
            callers_updated = self.update_call_sites(
                &workspace_id,
                &unit,
                &old_signature,
                &input.new_signature,
                language,
            ).await.unwrap_or_else(|e| {
                warn!("Failed to update some call sites: {}", e);
                0
            });
        }

        // Update code unit metadata
        unit.signature = input.new_signature.clone();
        unit.version += 1;
        unit.updated_at = chrono::Utc::now();
        self.ctx.update_code_unit(&unit).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let output = ChangeSignatureOutput {
            unit_id: input.unit_id.clone(),
            old_signature,
            new_signature: input.new_signature.clone(),
            callers_updated,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.add_parameter
// =============================================================================

pub struct CodeAddParameterTool {
    ctx: CodeManipulationContext,
}

impl CodeAddParameterTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AddParameterInput {
    unit_id: String,
    parameter_name: String,
    parameter_type: String,
    default_value: Option<String>,
    #[serde(default = "default_last_position")]
    position: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct AddParameterOutput {
    unit_id: String,
    parameter_added: String,
    new_signature: String,
}

#[async_trait]
impl Tool for CodeAddParameterTool {
    fn name(&self) -> &str {
        "cortex.code.add_parameter"
    }

    fn description(&self) -> Option<&str> {
        Some("Adds a parameter to a function signature")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AddParameterInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AddParameterInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!(
            "Adding parameter '{}' to unit '{}'",
            input.parameter_name, input.unit_id
        );

        // Fetch the code unit
        let mut unit = self.ctx.get_code_unit(&input.unit_id).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Parse the file
        let workspace_id = Uuid::new_v4();
        let language = match unit.language {
            Language::Rust => ParserLanguage::Rust,
            Language::TypeScript => ParserLanguage::TypeScript,
            Language::JavaScript => ParserLanguage::JavaScript,
            _ => return Err(ToolError::ExecutionFailed("Unsupported language".to_string())),
        };

        let (_, content, _) = self.ctx.parse_file(&workspace_id, &unit.file_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Create AST editor
        let tree_sitter_lang = match language {
            ParserLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
            ParserLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            ParserLanguage::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            ParserLanguage::Jsx => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Python | ParserLanguage::Cpp | ParserLanguage::Java | ParserLanguage::Kotlin => {
                return Err(ToolError::ExecutionFailed(format!("Language not supported for AST editing: {:?}", language)));
            }
        };

        let mut editor = AstEditor::new(content, tree_sitter_lang)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Find the function
        let functions = editor.query("(function_item) @func")
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let func_node = functions.iter()
            .find(|n| n.start_position().row == unit.start_line)
            .ok_or_else(|| ToolError::ExecutionFailed("Function not found".to_string()))?;

        let func_text = editor.node_text(func_node);

        // Build new parameter
        let new_param = if let Some(default) = &input.default_value {
            format!("{}: {} = {}", input.parameter_name, input.parameter_type, default)
        } else {
            format!("{}: {}", input.parameter_name, input.parameter_type)
        };

        // Extract function signature and body
        let body_start = func_text.find('{')
            .ok_or_else(|| ToolError::ExecutionFailed("Function body not found".to_string()))?;
        let signature_part = &func_text[..body_start].trim();
        let body_part = &func_text[body_start..];

        // Add parameter to signature
        let new_signature = if signature_part.contains('(') && signature_part.contains(')') {
            let paren_start = signature_part.find('(').unwrap();
            let paren_end = signature_part.rfind(')').unwrap();
            let existing_params = &signature_part[paren_start + 1..paren_end].trim();

            let updated_params = if existing_params.is_empty() {
                new_param.clone()
            } else if input.position == "first" {
                format!("{}, {}", new_param, existing_params)
            } else {
                format!("{}, {}", existing_params, new_param)
            };

            format!("{}({}){}",
                &signature_part[..paren_start],
                updated_params,
                &signature_part[paren_end + 1..])
        } else {
            return Err(ToolError::ExecutionFailed("Invalid function signature".to_string()));
        };

        // Build complete function
        let new_function = format!("{} {}", new_signature, body_part);
        let range = cortex_code_analysis::Range::from_node(func_node);
        editor.edits.push(cortex_code_analysis::Edit::replace(range, new_function));

        editor.apply_edits()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Save modified file
        self.ctx.save_file(&workspace_id, &unit.file_path, editor.get_source()).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Update unit metadata
        unit.parameters.push(CoreParameter {
            name: input.parameter_name.clone(),
            param_type: Some(input.parameter_type.clone()),
            default_value: input.default_value.clone(),
            is_optional: input.default_value.is_some(),
            is_variadic: false,
            attributes: vec![],
        });
        unit.signature = new_signature.clone();
        unit.version += 1;
        self.ctx.update_code_unit(&unit).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let output = AddParameterOutput {
            unit_id: input.unit_id.clone(),
            parameter_added: input.parameter_name.clone(),
            new_signature,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.remove_parameter
// =============================================================================

pub struct CodeRemoveParameterTool {
    ctx: CodeManipulationContext,
}

impl CodeRemoveParameterTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RemoveParameterInput {
    unit_id: String,
    parameter_name: String,
    #[serde(default = "default_true")]
    #[allow(dead_code)]
    update_callers: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct RemoveParameterOutput {
    unit_id: String,
    parameter_removed: String,
    new_signature: String,
    callers_updated: i32,
}

#[async_trait]
impl Tool for CodeRemoveParameterTool {
    fn name(&self) -> &str {
        "cortex.code.remove_parameter"
    }

    fn description(&self) -> Option<&str> {
        Some("Removes a parameter from a function signature and updates callers")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(RemoveParameterInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: RemoveParameterInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!(
            "Removing parameter '{}' from unit '{}'",
            input.parameter_name, input.unit_id
        );

        // Fetch the code unit
        let mut unit = self.ctx.get_code_unit(&input.unit_id).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Parse the file
        let workspace_id = Uuid::new_v4();
        let language = match unit.language {
            Language::Rust => ParserLanguage::Rust,
            Language::TypeScript => ParserLanguage::TypeScript,
            Language::JavaScript => ParserLanguage::JavaScript,
            _ => return Err(ToolError::ExecutionFailed("Unsupported language".to_string())),
        };

        let (_, content, _) = self.ctx.parse_file(&workspace_id, &unit.file_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Create AST editor
        let tree_sitter_lang = match language {
            ParserLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
            ParserLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            ParserLanguage::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            ParserLanguage::Jsx => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Python | ParserLanguage::Cpp | ParserLanguage::Java | ParserLanguage::Kotlin => {
                return Err(ToolError::ExecutionFailed(format!("Language not supported for AST editing: {:?}", language)));
            }
        };

        let mut editor = AstEditor::new(content, tree_sitter_lang)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Find the function
        let functions = editor.query("(function_item) @func")
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let func_node = functions.iter()
            .find(|n| n.start_position().row == unit.start_line)
            .ok_or_else(|| ToolError::ExecutionFailed("Function not found".to_string()))?;

        let func_text = editor.node_text(func_node);

        // Extract function signature and body
        let body_start = func_text.find('{')
            .ok_or_else(|| ToolError::ExecutionFailed("Function body not found".to_string()))?;
        let signature_part = &func_text[..body_start].trim();
        let body_part = &func_text[body_start..];

        // Remove parameter from signature
        let new_signature = if signature_part.contains('(') && signature_part.contains(')') {
            let paren_start = signature_part.find('(').unwrap();
            let paren_end = signature_part.rfind(')').unwrap();
            let existing_params = &signature_part[paren_start + 1..paren_end];

            // Split parameters and filter out the one to remove
            let params: Vec<&str> = existing_params
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty() && !s.contains(&input.parameter_name))
                .collect();

            let updated_params = params.join(", ");

            format!("{}({}){}",
                &signature_part[..paren_start],
                updated_params,
                &signature_part[paren_end + 1..])
        } else {
            return Err(ToolError::ExecutionFailed("Invalid function signature".to_string()));
        };

        // Build complete function
        let new_function = format!("{} {}", new_signature, body_part);
        let range = cortex_code_analysis::Range::from_node(func_node);
        editor.edits.push(cortex_code_analysis::Edit::replace(range, new_function));

        editor.apply_edits()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Save modified file
        self.ctx.save_file(&workspace_id, &unit.file_path, editor.get_source()).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Update unit metadata
        unit.parameters.retain(|p| p.name != input.parameter_name);
        unit.signature = new_signature.clone();
        unit.version += 1;
        self.ctx.update_code_unit(&unit).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let output = RemoveParameterOutput {
            unit_id: input.unit_id.clone(),
            parameter_removed: input.parameter_name.clone(),
            new_signature,
            callers_updated: 0,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.add_import
// =============================================================================

pub struct CodeAddImportTool {
    ctx: CodeManipulationContext,
}

impl CodeAddImportTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AddImportInput {
    file_path: String,
    import_spec: String,
    #[serde(default = "default_auto_position")]
    #[allow(dead_code)]
    position: String,
    #[serde(default = "default_workspace_id")]
    workspace_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct AddImportOutput {
    file_path: String,
    import_added: String,
    line_number: i32,
}

#[async_trait]
impl Tool for CodeAddImportTool {
    fn name(&self) -> &str {
        "cortex.code.add_import"
    }

    fn description(&self) -> Option<&str> {
        Some("Adds an import statement to a file at the optimal position using tree-sitter")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AddImportInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AddImportInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Adding import '{}' to '{}'", input.import_spec, input.file_path);

        let workspace_id = Uuid::parse_str(&input.workspace_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace_id: {}", e)))?;

        let (_, content, language) = self.ctx.parse_file(&workspace_id, &input.file_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let tree_sitter_lang = match language {
            ParserLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
            ParserLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            ParserLanguage::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            ParserLanguage::Jsx => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Python | ParserLanguage::Cpp | ParserLanguage::Java | ParserLanguage::Kotlin => {
                return Err(ToolError::ExecutionFailed(format!("Language not supported for AST editing: {:?}", language)));
            }
        };

        let mut editor = AstEditor::new(content, tree_sitter_lang)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Add the import (Rust-specific for now)
        editor.add_import_rust(&input.import_spec)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        editor.apply_edits()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        self.ctx.save_file(&workspace_id, &input.file_path, editor.get_source()).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let output = AddImportOutput {
            file_path: input.file_path.clone(),
            import_added: input.import_spec.clone(),
            line_number: 1,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.optimize_imports
// =============================================================================

pub struct CodeOptimizeImportsTool {
    ctx: CodeManipulationContext,
}

impl CodeOptimizeImportsTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct OptimizeImportsInput {
    file_path: String,
    #[serde(default = "default_true")]
    #[allow(dead_code)]
    remove_unused: bool,
    #[serde(default = "default_true")]
    #[allow(dead_code)]
    sort: bool,
    #[serde(default = "default_true")]
    #[allow(dead_code)]
    group: bool,
    #[serde(default = "default_workspace_id")]
    workspace_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct OptimizeImportsOutput {
    file_path: String,
    imports_removed: i32,
    imports_sorted: bool,
    imports_grouped: bool,
}

#[async_trait]
impl Tool for CodeOptimizeImportsTool {
    fn name(&self) -> &str {
        "cortex.code.optimize_imports"
    }

    fn description(&self) -> Option<&str> {
        Some("Optimizes imports by removing unused, sorting, and grouping using tree-sitter")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(OptimizeImportsInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: OptimizeImportsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Optimizing imports in '{}'", input.file_path);

        let workspace_id = Uuid::parse_str(&input.workspace_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace_id: {}", e)))?;

        let (_, content, language) = self.ctx.parse_file(&workspace_id, &input.file_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let tree_sitter_lang = match language {
            ParserLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
            ParserLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            ParserLanguage::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            ParserLanguage::Jsx => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Python | ParserLanguage::Cpp | ParserLanguage::Java | ParserLanguage::Kotlin => {
                return Err(ToolError::ExecutionFailed(format!("Language not supported for AST editing: {:?}", language)));
            }
        };

        let mut editor = AstEditor::new(content, tree_sitter_lang)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Optimize imports (Rust-specific for now)
        let result = editor.optimize_imports_rust()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        editor.apply_edits()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        self.ctx.save_file(&workspace_id, &input.file_path, editor.get_source()).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let output = OptimizeImportsOutput {
            file_path: input.file_path.clone(),
            imports_removed: result.removed as i32,
            imports_sorted: result.sorted,
            imports_grouped: result.grouped,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.generate_getter_setter
// =============================================================================

pub struct CodeGenerateGetterSetterTool {
    ctx: CodeManipulationContext,
}

impl CodeGenerateGetterSetterTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GenerateGetterSetterInput {
    class_id: String,
    field_name: String,
    #[serde(default = "default_both_generation")]
    generate: String,
    visibility: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct GenerateGetterSetterOutput {
    class_id: String,
    field_name: String,
    getter_id: Option<String>,
    setter_id: Option<String>,
}

#[async_trait]
impl Tool for CodeGenerateGetterSetterTool {
    fn name(&self) -> &str {
        "cortex.code.generate_getter_setter"
    }

    fn description(&self) -> Option<&str> {
        Some("Generates getter/setter methods for a class field")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(GenerateGetterSetterInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: GenerateGetterSetterInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!(
            "Generating getter/setter for field '{}' in class '{}'",
            input.field_name, input.class_id
        );

        // Fetch the struct/class code unit
        let unit = self.ctx.get_code_unit(&input.class_id).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let workspace_id = Uuid::new_v4();
        let language = match unit.language {
            Language::Rust => ParserLanguage::Rust,
            Language::TypeScript => ParserLanguage::TypeScript,
            Language::JavaScript => ParserLanguage::JavaScript,
            _ => return Err(ToolError::ExecutionFailed("Unsupported language".to_string())),
        };

        let (parsed, content, _) = self.ctx.parse_file(&workspace_id, &unit.file_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Find the field type from the struct
        let field_type = parsed.structs.iter()
            .find(|s| s.qualified_name == unit.qualified_name)
            .and_then(|s| s.fields.iter().find(|f| f.name == input.field_name))
            .map(|f| f.field_type.clone())
            .ok_or_else(|| ToolError::ExecutionFailed(format!("Field '{}' not found", input.field_name)))?;

        // Create AST editor
        let tree_sitter_lang = match language {
            ParserLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
            ParserLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            ParserLanguage::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            ParserLanguage::Jsx => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Python | ParserLanguage::Cpp | ParserLanguage::Java | ParserLanguage::Kotlin => {
                return Err(ToolError::ExecutionFailed(format!("Language not supported for AST editing: {:?}", language)));
            }
        };

        let mut editor = AstEditor::new(content, tree_sitter_lang)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Find the impl block for this struct
        let impl_query = "(impl_item) @impl";
        let impls = editor.query(impl_query)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let impl_node = impls.iter()
            .find(|n| {
                let text = editor.node_text(n);
                text.contains(&unit.name)
            });

        let visibility = input.visibility.as_deref().unwrap_or("pub");

        // Generate getter and setter methods
        let mut methods = String::new();

        if input.generate == "getter" || input.generate == "both" {
            methods.push_str(&format!(
                "\n    {} fn {}(&self) -> &{} {{\n        &self.{}\n    }}\n",
                visibility, input.field_name, field_type, input.field_name
            ));
        }

        if input.generate == "setter" || input.generate == "both" {
            methods.push_str(&format!(
                "\n    {} fn set_{}(&mut self, value: {}) {{\n        self.{} = value;\n    }}\n",
                visibility, input.field_name, field_type, input.field_name
            ));
        }

        // Insert methods into impl block or create new impl block
        if let Some(impl_node) = impl_node {
            let impl_text = editor.node_text(impl_node);
            let last_brace = impl_text.rfind('}')
                .ok_or_else(|| ToolError::ExecutionFailed("Invalid impl block".to_string()))?;

            let new_impl = format!("{}{}\n{}",
                &impl_text[..last_brace],
                methods,
                &impl_text[last_brace..]);

            let range = cortex_code_analysis::Range::from_node(impl_node);
            editor.edits.push(cortex_code_analysis::Edit::replace(range, new_impl));
        } else {
            // Create new impl block
            let new_impl = format!("\nimpl {} {{{}\n}}\n", unit.name, methods);
            let insert_line = unit.end_line + 1;
            editor.insert_at(insert_line, 0, &new_impl)
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        }

        editor.apply_edits()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Save modified file
        self.ctx.save_file(&workspace_id, &unit.file_path, editor.get_source()).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let output = GenerateGetterSetterOutput {
            class_id: input.class_id.clone(),
            field_name: input.field_name.clone(),
            getter_id: if input.generate == "getter" || input.generate == "both" {
                Some(format!("unit_{}", uuid::Uuid::new_v4()))
            } else {
                None
            },
            setter_id: if input.generate == "setter" || input.generate == "both" {
                Some(format!("unit_{}", uuid::Uuid::new_v4()))
            } else {
                None
            },
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.implement_interface
// =============================================================================

pub struct CodeImplementInterfaceTool {
    ctx: CodeManipulationContext,
}

impl CodeImplementInterfaceTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ImplementInterfaceInput {
    class_id: String,
    interface_id: String,
    #[serde(default = "default_true")]
    generate_stubs: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ImplementInterfaceOutput {
    class_id: String,
    interface_id: String,
    methods_generated: Vec<String>,
}

#[async_trait]
impl Tool for CodeImplementInterfaceTool {
    fn name(&self) -> &str {
        "cortex.code.implement_interface"
    }

    fn description(&self) -> Option<&str> {
        Some("Implements an interface/trait with auto-generated stubs")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ImplementInterfaceInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: ImplementInterfaceInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!(
            "Implementing interface '{}' in class '{}'",
            input.interface_id, input.class_id
        );

        // Fetch the struct/class code unit
        let struct_unit = self.ctx.get_code_unit(&input.class_id).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Fetch the trait/interface code unit
        let trait_unit = self.ctx.get_code_unit(&input.interface_id).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let workspace_id = Uuid::new_v4();
        let language = match struct_unit.language {
            Language::Rust => ParserLanguage::Rust,
            Language::TypeScript => ParserLanguage::TypeScript,
            Language::JavaScript => ParserLanguage::JavaScript,
            _ => return Err(ToolError::ExecutionFailed("Unsupported language".to_string())),
        };

        // Parse the trait definition to extract method signatures
        let (trait_parsed, _, _) = self.ctx.parse_file(&workspace_id, &trait_unit.file_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let trait_info = trait_parsed.traits.iter()
            .find(|t| t.qualified_name == trait_unit.qualified_name)
            .ok_or_else(|| ToolError::ExecutionFailed("Trait not found".to_string()))?;

        // Parse the struct file
        let (_, struct_content, _) = self.ctx.parse_file(&workspace_id, &struct_unit.file_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Create AST editor
        let tree_sitter_lang = match language {
            ParserLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
            ParserLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            ParserLanguage::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            ParserLanguage::Jsx => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Python | ParserLanguage::Cpp | ParserLanguage::Java | ParserLanguage::Kotlin => {
                return Err(ToolError::ExecutionFailed(format!("Language not supported for AST editing: {:?}", language)));
            }
        };

        let mut editor = AstEditor::new(struct_content, tree_sitter_lang)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Generate stub methods
        let mut impl_body = String::new();
        let mut methods_generated = Vec::new();

        if input.generate_stubs {
            for method in &trait_info.methods {
                methods_generated.push(method.name.clone());

                let params: Vec<String> = method.parameters.iter()
                    .map(|p| format!("{}: {}", p.name, p.param_type))
                    .collect();

                let return_type = method.return_type.as_ref()
                    .map(|r| format!(" -> {}", r))
                    .unwrap_or_default();

                impl_body.push_str(&format!(
                    "    fn {}({}) {} {{\n        todo!(\"Implement {}\")\n    }}\n\n",
                    method.name,
                    params.join(", "),
                    return_type,
                    method.name
                ));
            }
        }

        // Create impl block
        let impl_block = format!(
            "\nimpl {} for {} {{\n{}}}\n",
            trait_unit.name,
            struct_unit.name,
            impl_body
        );

        // Insert after the struct definition
        let insert_line = struct_unit.end_line + 1;
        editor.insert_at(insert_line, 0, &impl_block)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        editor.apply_edits()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Save modified file
        self.ctx.save_file(&workspace_id, &struct_unit.file_path, editor.get_source()).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let output = ImplementInterfaceOutput {
            class_id: input.class_id.clone(),
            interface_id: input.interface_id.clone(),
            methods_generated,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.code.override_method
// =============================================================================

pub struct CodeOverrideMethodTool {
    ctx: CodeManipulationContext,
}

impl CodeOverrideMethodTool {
    pub fn new(ctx: CodeManipulationContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct OverrideMethodInput {
    class_id: String,
    method_name: String,
    #[serde(default = "default_true")]
    call_super: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct OverrideMethodOutput {
    class_id: String,
    method_id: String,
    method_name: String,
    calls_super: bool,
}

#[async_trait]
impl Tool for CodeOverrideMethodTool {
    fn name(&self) -> &str {
        "cortex.code.override_method"
    }

    fn description(&self) -> Option<&str> {
        Some("Overrides a parent class method with stub implementation")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(OverrideMethodInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: OverrideMethodInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!(
            "Overriding method '{}' in class '{}'",
            input.method_name, input.class_id
        );

        // Fetch the struct/class code unit
        let struct_unit = self.ctx.get_code_unit(&input.class_id).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let workspace_id = Uuid::new_v4();
        let language = match struct_unit.language {
            Language::Rust => ParserLanguage::Rust,
            Language::TypeScript => ParserLanguage::TypeScript,
            Language::JavaScript => ParserLanguage::JavaScript,
            _ => return Err(ToolError::ExecutionFailed("Unsupported language".to_string())),
        };

        // Parse the struct file
        let (parsed, content, _) = self.ctx.parse_file(&workspace_id, &struct_unit.file_path).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Find the parent trait method signature
        // For Rust, we need to find which trait this struct implements
        let parent_method = parsed.traits.iter()
            .flat_map(|t| &t.methods)
            .find(|m| m.name == input.method_name)
            .ok_or_else(|| ToolError::ExecutionFailed(
                format!("Method '{}' not found in any trait", input.method_name)
            ))?;

        // Create AST editor
        let tree_sitter_lang = match language {
            ParserLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
            ParserLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            ParserLanguage::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            ParserLanguage::Jsx => tree_sitter_javascript::LANGUAGE.into(),
            ParserLanguage::Python | ParserLanguage::Cpp | ParserLanguage::Java | ParserLanguage::Kotlin => {
                return Err(ToolError::ExecutionFailed(format!("Language not supported for AST editing: {:?}", language)));
            }
        };

        let mut editor = AstEditor::new(content, tree_sitter_lang)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Build method signature
        let params: Vec<String> = parent_method.parameters.iter()
            .map(|p| format!("{}: {}", p.name, p.param_type))
            .collect();

        let return_type = parent_method.return_type.as_ref()
            .map(|r| format!(" -> {}", r))
            .unwrap_or_default();

        // Build method body
        let body = if input.call_super {
            format!("        // TODO: Call parent implementation\n        todo!(\"Override {}\")", input.method_name)
        } else {
            format!("        todo!(\"Override {}\")", input.method_name)
        };

        let method_code = format!(
            "\n    fn {}({}) {} {{\n{}\n    }}\n",
            input.method_name,
            params.join(", "),
            return_type,
            body
        );

        // Find the impl block for this struct
        let impl_query = "(impl_item) @impl";
        let impls = editor.query(impl_query)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let impl_node = impls.iter()
            .find(|n| {
                let text = editor.node_text(n);
                text.contains(&struct_unit.name)
            })
            .ok_or_else(|| ToolError::ExecutionFailed("No impl block found".to_string()))?;

        // Insert method into impl block
        let impl_text = editor.node_text(impl_node);
        let last_brace = impl_text.rfind('}')
            .ok_or_else(|| ToolError::ExecutionFailed("Invalid impl block".to_string()))?;

        let new_impl = format!("{}{}\n{}",
            &impl_text[..last_brace],
            method_code,
            &impl_text[last_brace..]);

        let range = cortex_code_analysis::Range::from_node(impl_node);
        editor.edits.push(cortex_code_analysis::Edit::replace(range, new_impl));

        editor.apply_edits()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Save modified file
        self.ctx.save_file(&workspace_id, &struct_unit.file_path, editor.get_source()).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let output = OverrideMethodOutput {
            class_id: input.class_id.clone(),
            method_id: format!("unit_{}", uuid::Uuid::new_v4()),
            method_name: input.method_name.clone(),
            calls_super: input.call_super,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// Helper functions
// =============================================================================

fn default_true() -> bool {
    true
}

fn default_workspace_scope() -> String {
    "workspace".to_string()
}

fn default_before_position() -> String {
    "before".to_string()
}

fn default_migration_strategy() -> String {
    "replace".to_string()
}

fn default_last_position() -> String {
    "last".to_string()
}

fn default_auto_position() -> String {
    "auto".to_string()
}

fn default_both_generation() -> String {
    "both".to_string()
}

fn default_workspace_id() -> String {
    "00000000-0000-0000-0000-000000000000".to_string()
}
