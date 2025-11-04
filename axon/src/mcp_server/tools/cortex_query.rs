//! Cortex Query Tool

use crate::cortex_bridge::{CortexBridge, SearchFilters, WorkspaceId};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CortexQueryInput {
    pub query: String,
}

#[derive(Debug, Serialize)]
pub struct CortexQueryOutput {
    pub results: Vec<serde_json::Value>,
}

pub struct CortexQueryTool {
    cortex: Arc<CortexBridge>,
}

impl CortexQueryTool {
    /// Create a new CortexQueryTool with a reference to CortexBridge
    pub fn new(cortex: Arc<CortexBridge>) -> Self {
        Self { cortex }
    }

    /// Query the Cortex knowledge graph using semantic search
    pub async fn query(&self, input: CortexQueryInput) -> Result<CortexQueryOutput> {
        // Ensure Cortex is initialized before querying
        self.cortex.ensure_initialized().await?;

        // Use a default workspace ID - in a real scenario, this might be passed as input
        let workspace_id = WorkspaceId::from("default".to_string());

        // Create search filters with sensible defaults
        let filters = SearchFilters::default();

        // Perform semantic search
        let search_results = self
            .cortex
            .semantic_search(&input.query, &workspace_id, filters)
            .await?;

        // Convert search results to JSON values
        let results: Vec<serde_json::Value> = search_results
            .into_iter()
            .map(|result| {
                serde_json::json!({
                    "unit_id": result.unit_id,
                    "unit_type": result.unit_type,
                    "name": result.name,
                    "qualified_name": result.qualified_name,
                    "signature": result.signature,
                    "relevance_score": result.relevance_score,
                    "file": result.file,
                    "snippet": result.snippet,
                })
            })
            .collect();

        Ok(CortexQueryOutput { results })
    }
}
