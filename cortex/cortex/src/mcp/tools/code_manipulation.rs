//! Code Manipulation Utilities
//!
//! This module provides shared context and utilities for code manipulation operations.
//! Read-only tools are located in separate tool modules.

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
use crate::mcp::context::CortexToolContext;

// =============================================================================
// Shared Context
// =============================================================================

/// Shared context for all code manipulation tools
#[derive(Clone)]
pub struct CodeManipulationContext {
    storage: Arc<ConnectionManager>,
    vfs: Arc<VirtualFileSystem>,
    code_unit_service: Arc<CodeUnitService>,
}

impl CodeManipulationContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
        let code_unit_service = Arc::new(CodeUnitService::new(storage.clone()));
        Self {
            storage,
            vfs,
            code_unit_service,
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
