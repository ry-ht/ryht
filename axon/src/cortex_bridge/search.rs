//! Semantic search functionality for Cortex integration
//!
//! This module provides semantic code search and code unit discovery.

use super::client::{CortexClient, Result};
use super::models::*;
use serde::{Deserialize, Serialize};
use tracing::info;

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct SemanticSearchRequest {
    pub query: String,
    pub workspace_id: Option<String>,
    pub filters: SearchFiltersRequest,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchFiltersRequest {
    pub types: Vec<String>,
    pub languages: Vec<String>,
    pub visibility: Option<String>,
    pub min_relevance: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SemanticSearchResponse {
    pub results: Vec<CodeSearchResult>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UnitsResponse {
    pub units: Vec<CodeUnit>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphQueryRequest {
    pub query: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphQueryResponse {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub results: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub node_type: String,
    pub properties: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub edge_type: String,
    pub properties: serde_json::Value,
}

// ============================================================================
// Search Manager
// ============================================================================

/// Search manager for semantic code search
pub struct SearchManager {
    client: CortexClient,
}

impl SearchManager {
    /// Create a new search manager
    pub fn new(client: CortexClient) -> Self {
        Self { client }
    }

    /// Perform semantic code search
    pub async fn semantic_search(
        &self,
        query: &str,
        workspace_id: &WorkspaceId,
        filters: SearchFilters,
    ) -> Result<Vec<CodeSearchResult>> {
        let request = SemanticSearchRequest {
            query: query.to_string(),
            workspace_id: Some(workspace_id.0.clone()),
            filters: SearchFiltersRequest {
                types: filters.types,
                languages: filters.languages,
                visibility: filters.visibility,
                min_relevance: filters.min_relevance,
            },
            limit: 20,
        };

        let response: SemanticSearchResponse = self
            .client
            .post("/search/semantic", &request)
            .await?;

        info!(
            "Semantic search returned {} results for query: {}",
            response.results.len(),
            query
        );

        Ok(response.results)
    }

    /// Get code units from workspace
    pub async fn get_code_units(
        &self,
        workspace_id: &WorkspaceId,
        filters: UnitFilters,
    ) -> Result<Vec<CodeUnit>> {
        let mut query_params = vec![];

        if let Some(unit_type) = &filters.unit_type {
            query_params.push(format!("unit_type={}", unit_type));
        }
        if let Some(language) = &filters.language {
            query_params.push(format!("language={}", language));
        }
        if let Some(visibility) = &filters.visibility {
            query_params.push(format!("visibility={}", visibility));
        }

        let query_string = if query_params.is_empty() {
            String::new()
        } else {
            format!("?{}", query_params.join("&"))
        };

        let path = format!("/workspaces/{}/units{}", workspace_id, query_string);
        let response: UnitsResponse = self.client.get(&path).await?;

        info!(
            "Retrieved {} code units from workspace {}",
            response.units.len(),
            workspace_id
        );

        Ok(response.units)
    }

    /// Get a specific code unit by ID
    pub async fn get_code_unit(
        &self,
        workspace_id: &WorkspaceId,
        unit_id: &str,
    ) -> Result<CodeUnit> {
        let path = format!("/workspaces/{}/units/{}", workspace_id, unit_id);
        let unit: CodeUnit = self.client.get(&path).await?;

        Ok(unit)
    }

    /// Search for code units by name
    pub async fn search_units_by_name(
        &self,
        workspace_id: &WorkspaceId,
        name: &str,
    ) -> Result<Vec<CodeUnit>> {
        let path = format!("/workspaces/{}/units?name={}", workspace_id, urlencoding::encode(name));
        let response: UnitsResponse = self.client.get(&path).await?;

        Ok(response.units)
    }

    /// Get dependencies for a code unit
    pub async fn get_unit_dependencies(
        &self,
        workspace_id: &WorkspaceId,
        unit_id: &str,
    ) -> Result<Vec<CodeUnit>> {
        let path = format!("/workspaces/{}/units/{}/dependencies", workspace_id, unit_id);
        let response: UnitsResponse = self.client.get(&path).await?;

        Ok(response.units)
    }

    /// Get dependents of a code unit
    pub async fn get_unit_dependents(
        &self,
        workspace_id: &WorkspaceId,
        unit_id: &str,
    ) -> Result<Vec<CodeUnit>> {
        let path = format!("/workspaces/{}/units/{}/dependents", workspace_id, unit_id);
        let response: UnitsResponse = self.client.get(&path).await?;

        Ok(response.units)
    }

    /// Query the knowledge graph
    pub async fn query_graph(
        &self,
        query: &str,
        parameters: serde_json::Value,
    ) -> Result<GraphQueryResponse> {
        let request = GraphQueryRequest {
            query: query.to_string(),
            parameters,
        };

        let response: GraphQueryResponse = self
            .client
            .post("/graph/query", &request)
            .await?;

        info!(
            "Graph query returned {} nodes and {} edges",
            response.nodes.len(),
            response.edges.len()
        );

        Ok(response)
    }

    /// Find all references to a code unit
    pub async fn find_references(
        &self,
        workspace_id: &WorkspaceId,
        unit_id: &str,
    ) -> Result<Vec<CodeSearchResult>> {
        let path = format!("/workspaces/{}/units/{}/references", workspace_id, unit_id);

        #[derive(Deserialize)]
        struct ReferencesResponse {
            references: Vec<CodeSearchResult>,
        }

        let response: ReferencesResponse = self.client.get(&path).await?;
        info!("Found {} references to unit {}", response.references.len(), unit_id);

        Ok(response.references)
    }

    /// Get call graph for a function
    pub async fn get_call_graph(
        &self,
        workspace_id: &WorkspaceId,
        unit_id: &str,
        depth: u32,
    ) -> Result<GraphQueryResponse> {
        let path = format!(
            "/workspaces/{}/units/{}/callgraph?depth={}",
            workspace_id, unit_id, depth
        );

        let response: GraphQueryResponse = self.client.get(&path).await?;
        Ok(response)
    }

    /// Analyze and index code for semantic search
    pub async fn analyze_and_index(
        &self,
        workspace_id: &WorkspaceId,
        file_path: &str,
        content: &str,
    ) -> Result<CodeAnalysisResult> {
        #[derive(Serialize)]
        struct AnalyzeRequest {
            workspace_id: String,
            file_path: String,
            content: String,
        }

        let request = AnalyzeRequest {
            workspace_id: workspace_id.to_string(),
            file_path: file_path.to_string(),
            content: content.to_string(),
        };

        #[derive(Deserialize)]
        struct AnalyzeResponse {
            result: CodeAnalysisResult,
        }

        let response: AnalyzeResponse = self
            .client
            .post("/code/analyze", &request)
            .await?;

        info!(
            "Analyzed and indexed {} in workspace {}: {} units extracted",
            file_path, workspace_id, response.result.units_extracted
        );

        Ok(response.result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_filters_default() {
        let filters = SearchFilters::default();
        assert_eq!(filters.types.len(), 0);
        assert_eq!(filters.languages.len(), 0);
        assert_eq!(filters.min_relevance, 0.7);
    }

    #[test]
    fn test_unit_filters_default() {
        let filters = UnitFilters::default();
        assert!(filters.unit_type.is_none());
        assert!(filters.language.is_none());
        assert!(filters.visibility.is_none());
    }

    #[test]
    fn test_query_params_construction() {
        let filters = UnitFilters {
            unit_type: Some("function".to_string()),
            language: Some("rust".to_string()),
            visibility: None,
        };

        let mut params = vec![];
        if let Some(ut) = &filters.unit_type {
            params.push(format!("unit_type={}", ut));
        }
        if let Some(lang) = &filters.language {
            params.push(format!("language={}", lang));
        }

        assert_eq!(params.len(), 2);
        assert!(params.contains(&"unit_type=function".to_string()));
        assert!(params.contains(&"language=rust".to_string()));
    }
}
