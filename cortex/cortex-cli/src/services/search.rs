//! Search service layer
//!
//! Provides unified search operations for both API and MCP modules.

use anyhow::Result;
use cortex_semantic::{SemanticConfig, SemanticSearchEngine, SearchFilter};
use cortex_semantic::types::EntityType;
use cortex_storage::ConnectionManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Search service for code and semantic search operations
#[derive(Clone)]
pub struct SearchService {
    storage: Arc<ConnectionManager>,
    semantic_engine: Arc<RwLock<SemanticSearchEngine>>,
}

impl SearchService {
    /// Create a new search service
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        // Create semantic search engine with mock provider for testing
        let mut config = SemanticConfig::default();
        config.embedding.primary_provider = "mock".to_string();
        config.embedding.fallback_providers = vec![];

        let semantic_engine = Arc::new(RwLock::new(
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    SemanticSearchEngine::new(config)
                        .await
                        .expect("Failed to create search engine")
                })
            })
        ));

        Self {
            storage,
            semantic_engine,
        }
    }

    /// Search code using semantic embeddings
    pub async fn search_code(&self, request: SearchCodeRequest) -> Result<Vec<SearchResult>> {
        info!("Semantic code search: '{}'", request.query);

        let mut filter = SearchFilter::default();
        filter.entity_type = Some(EntityType::Code);
        filter.min_score = Some(request.min_similarity);

        if let Some(lang) = &request.language {
            filter.metadata_filters.insert("language".to_string(), lang.clone());
        }

        let engine = self.semantic_engine.read().await;
        let search_results = engine
            .search_with_filter(&request.query, request.limit, filter)
            .await?;

        let results = search_results
            .into_iter()
            .map(|r| SearchResult {
                id: r.id.clone(),
                title: r.metadata.get("name").cloned().unwrap_or_else(|| r.id.clone()),
                content: if r.content.len() > 200 {
                    format!("{}...", &r.content[..200])
                } else {
                    r.content.clone()
                },
                score: r.score,
                result_type: "code".to_string(),
                file_path: r.metadata.get("file_path").cloned(),
                language: r.metadata.get("language").cloned(),
                metadata: r.metadata,
            })
            .collect();

        Ok(results)
    }

    /// Search for similar code units
    pub async fn search_similar(&self, request: SearchSimilarRequest) -> Result<Vec<SearchResult>> {
        info!("Finding similar code to unit: {}", request.reference_unit_id);

        // Get reference unit content
        let conn = self.storage.acquire().await?;

        let query = "SELECT * FROM code_unit WHERE id = $unit_id LIMIT 1";
        let mut response = conn
            .connection()
            .query(query)
            .bind(("unit_id", request.reference_unit_id.clone()))
            .await?;

        let units: Vec<serde_json::Value> = response.take(0)?;
        let unit = units
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Unit not found: {}", request.reference_unit_id))?;

        // Build query from unit content
        let signature = unit.get("signature").and_then(|v| v.as_str()).unwrap_or("");
        let body = unit.get("body").and_then(|v| v.as_str()).unwrap_or("");
        let query_content = format!("{}\n{}", signature, body);

        // Search
        let mut filter = SearchFilter::default();
        filter.entity_type = Some(EntityType::Code);
        filter.min_score = Some(request.similarity_threshold);

        let engine = self.semantic_engine.read().await;
        let search_results = engine
            .search_with_filter(&query_content, request.limit + 1, filter)
            .await?;

        // Filter out the reference unit itself
        let results = search_results
            .into_iter()
            .filter(|r| r.id != request.reference_unit_id)
            .take(request.limit)
            .map(|r| SearchResult {
                id: r.id.clone(),
                title: r.metadata.get("name").cloned().unwrap_or_else(|| r.id.clone()),
                content: r.content.clone(),
                score: r.score,
                result_type: "code".to_string(),
                file_path: r.metadata.get("file_path").cloned(),
                language: r.metadata.get("language").cloned(),
                metadata: r.metadata,
            })
            .collect();

        Ok(results)
    }

    /// Search by natural language description
    pub async fn search_by_meaning(&self, request: SearchByMeaningRequest) -> Result<Vec<SearchResult>> {
        info!("Searching by meaning: '{}'", request.description);

        let mut filter = SearchFilter::default();
        filter.entity_type = Some(EntityType::Code);
        filter.min_score = Some(request.min_similarity);

        let engine = self.semantic_engine.read().await;
        let search_results = engine
            .search_with_filter(&request.description, request.limit, filter)
            .await?;

        let results = search_results
            .into_iter()
            .map(|r| SearchResult {
                id: r.id.clone(),
                title: r.metadata.get("name").cloned().unwrap_or_else(|| r.id.clone()),
                content: if r.content.len() > 200 {
                    format!("{}...", &r.content[..200])
                } else {
                    r.content.clone()
                },
                score: r.score,
                result_type: "code".to_string(),
                file_path: r.metadata.get("file_path").cloned(),
                language: r.metadata.get("language").cloned(),
                metadata: r.metadata,
            })
            .collect();

        Ok(results)
    }

    /// Text-based search (fallback for non-semantic search)
    pub async fn search_text(&self, request: TextSearchRequest) -> Result<Vec<SearchResult>> {
        debug!("Text search: '{}' (type: {})", request.query, request.search_type);

        let conn = self.storage.acquire().await?;

        let (query, result_type) = match request.search_type.as_str() {
            "code_units" => (
                format!(
                    "SELECT * FROM code_unit WHERE
                     name CONTAINS $query OR
                     signature CONTAINS $query OR
                     summary CONTAINS $query
                     LIMIT $limit"
                ),
                "code_unit",
            ),
            "patterns" => (
                format!(
                    "SELECT * FROM learned_pattern WHERE
                     pattern_name CONTAINS $query OR
                     description CONTAINS $query
                     LIMIT $limit"
                ),
                "pattern",
            ),
            _ => anyhow::bail!("Invalid search type: {}", request.search_type),
        };

        let mut response = conn
            .connection()
            .query(&query)
            .bind(("query", request.query.clone()))
            .bind(("limit", request.limit))
            .await?;

        let items: Vec<serde_json::Value> = response.take(0)?;

        let results = items
            .into_iter()
            .enumerate()
            .map(|(i, item)| {
                let title = if result_type == "code_unit" {
                    item.get("name")
                } else {
                    item.get("pattern_name")
                }
                .and_then(|v| v.as_str())
                .unwrap_or("Unnamed")
                .to_string();

                let content = if result_type == "code_unit" {
                    item.get("signature")
                } else {
                    item.get("description")
                }
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

                SearchResult {
                    id: item
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    title,
                    content,
                    score: 1.0 - (i as f32 / request.limit as f32),
                    result_type: result_type.to_string(),
                    file_path: item.get("file_path").and_then(|v| v.as_str()).map(String::from),
                    language: item.get("language").and_then(|v| v.as_str()).map(String::from),
                    metadata: HashMap::new(),
                }
            })
            .collect();

        Ok(results)
    }

    /// Find references to a code unit
    pub async fn find_references(&self, unit_id: &str) -> Result<Vec<CodeReference>> {
        debug!("Finding references to unit: {}", unit_id);

        let conn = self.storage.acquire().await?;

        // Get the code unit details
        let unit_query = "SELECT * FROM code_unit WHERE id = $unit_id LIMIT 1";
        let mut unit_response = conn
            .connection()
            .query(unit_query)
            .bind(("unit_id", unit_id.to_string()))
            .await?;

        let units: Vec<serde_json::Value> = unit_response.take(0)?;
        let _unit = units
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Code unit {} not found", unit_id))?;

        // Search for references
        let references_query = "SELECT * FROM code_reference WHERE target_unit_id = $unit_id LIMIT 100";
        let mut ref_response = conn
            .connection()
            .query(references_query)
            .bind(("unit_id", unit_id.to_string()))
            .await?;

        let references_raw: Vec<serde_json::Value> = ref_response.take(0)?;

        let references = references_raw
            .into_iter()
            .filter_map(|r| {
                Some(CodeReference {
                    id: r.get("id")?.as_str()?.to_string(),
                    file_path: r.get("file_path")?.as_str()?.to_string(),
                    line: r.get("line")?.as_u64()? as usize,
                    column: r.get("column")?.as_u64().unwrap_or(0) as usize,
                    reference_type: r.get("reference_type")?.as_str()?.to_string(),
                    context: r.get("context")?.as_str().unwrap_or("").to_string(),
                })
            })
            .collect();

        Ok(references)
    }
}

// =============================================================================
// Request/Response Types
// =============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct SearchCodeRequest {
    pub query: String,
    pub limit: usize,
    pub min_similarity: f32,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchSimilarRequest {
    pub reference_unit_id: String,
    pub similarity_threshold: f32,
    pub limit: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchByMeaningRequest {
    pub description: String,
    pub limit: usize,
    pub min_similarity: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TextSearchRequest {
    pub query: String,
    pub search_type: String,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub content: String,
    pub score: f32,
    pub result_type: String,
    pub file_path: Option<String>,
    pub language: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodeReference {
    pub id: String,
    pub file_path: String,
    pub line: usize,
    pub column: usize,
    pub reference_type: String,
    pub context: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_result_serialization() {
        let result = SearchResult {
            id: "test-id".to_string(),
            title: "Test Function".to_string(),
            content: "fn test() {}".to_string(),
            score: 0.95,
            result_type: "code".to_string(),
            file_path: Some("/test.rs".to_string()),
            language: Some("rust".to_string()),
            metadata: HashMap::new(),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Test Function"));
    }

    #[test]
    fn test_code_reference_serialization() {
        let reference = CodeReference {
            id: "ref-id".to_string(),
            file_path: "/main.rs".to_string(),
            line: 42,
            column: 10,
            reference_type: "call".to_string(),
            context: "test();".to_string(),
        };

        let json = serde_json::to_string(&reference).unwrap();
        assert!(json.contains("main.rs"));
    }
}
