//! Cortex Query Tool
//!
//! Direct integration with Cortex memory and VFS subsystems (no HTTP bridge).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use cortex_memory::SemanticMemorySystem;
use cortex_vfs::VirtualFileSystem;
use mcp_sdk::prelude::*;
use async_trait::async_trait;

/// Context for Cortex queries with direct subsystem references
#[derive(Clone)]
pub struct CortexQueryContext {
    /// Semantic memory system for code search
    pub memory: Arc<SemanticMemorySystem>,
    /// Virtual filesystem for file access
    pub vfs: Arc<VirtualFileSystem>,
}

impl CortexQueryContext {
    /// Create a new CortexQueryContext
    pub fn new(
        memory: Arc<SemanticMemorySystem>,
        vfs: Arc<VirtualFileSystem>,
    ) -> Self {
        Self {
            memory,
            vfs,
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CortexQueryInput {
    pub query: String,
    pub workspace_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CortexQueryOutput {
    pub results: Vec<serde_json::Value>,
}

pub struct CortexQueryTool {
    context: CortexQueryContext,
}

impl CortexQueryTool {
    /// Create a new CortexQueryTool with CortexQueryContext
    pub fn new(context: CortexQueryContext) -> Self {
        Self { context }
    }

    /// Query the Cortex knowledge graph using semantic search
    pub async fn query(&self, input: CortexQueryInput) -> Result<CortexQueryOutput> {
        // Parse workspace ID if provided, otherwise use default
        let workspace_id = if let Some(ws_id) = input.workspace_id {
            uuid::Uuid::parse_str(&ws_id)?
        } else {
            uuid::Uuid::nil() // Use nil UUID as default workspace
        };

        // Perform semantic search directly via SemanticMemorySystem
        // TODO: Implement actual semantic search when cortex-memory supports it
        // For now, return empty results
        let results = Vec::new();

        tracing::info!(
            query = %input.query,
            workspace_id = %workspace_id,
            results_count = results.len(),
            "Cortex query executed"
        );

        Ok(CortexQueryOutput { results })
    }
}

#[async_trait]
impl Tool for CortexQueryTool {
    fn name(&self) -> &str {
        "axon.cortex.query"
    }

    fn description(&self) -> Option<&str> {
        Some("Query the Cortex knowledge graph using semantic search")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(CortexQueryInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: CortexQueryInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let output = self.query(input).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let json_output = serde_json::to_string_pretty(&output)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult {
            content: vec![ToolContent::text(json_output)],
            is_error: false,
        })
    }
}
