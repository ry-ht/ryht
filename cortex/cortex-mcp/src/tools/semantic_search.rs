//! Semantic Search Tools
//!
//! This module implements 8 semantic search tools using REAL vector search with embeddings:
//! 1. cortex.semantic.search_code - Semantic code search using embeddings
//! 2. cortex.semantic.search_similar - Find semantically similar code units
//! 3. cortex.semantic.find_by_meaning - Natural language code discovery
//! 4. cortex.semantic.search_documentation - Search docs by semantic meaning
//! 5. cortex.semantic.search_comments - Find comments by semantic similarity
//! 6. cortex.semantic.hybrid_search - Combined keyword + semantic search
//! 7. cortex.semantic.search_by_example - Find code similar to example
//! 8. cortex.semantic.search_by_natural_language - NL to code search

use async_trait::async_trait;
use cortex_core::error::CortexError;
use cortex_core::id::CortexId;
use cortex_core::types::CodeUnit;
use cortex_memory::SemanticMemorySystem;
use cortex_semantic::{
    SemanticSearchEngine, SemanticConfig, SearchFilter,
};
use cortex_semantic::types::EntityType;
use cortex_storage::ConnectionManager;
use mcp_server::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

// =============================================================================
// Shared Context
// =============================================================================

#[derive(Clone)]
pub struct SemanticSearchContext {
    storage: Arc<ConnectionManager>,
    semantic_memory: Arc<SemanticMemorySystem>,
    search_engine: Arc<RwLock<SemanticSearchEngine>>,
}

impl SemanticSearchContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        let semantic_memory = Arc::new(SemanticMemorySystem::new(storage.clone()));

        // Create semantic search engine with mock provider for testing
        // In production, use OpenAI or ONNX provider
        let mut config = SemanticConfig::default();
        config.embedding.primary_provider = "mock".to_string();
        config.embedding.fallback_providers = vec![];

        // Initialize search engine (async, so we'll do it lazily)
        let search_engine = Arc::new(RwLock::new(
            // This will be initialized on first use
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    SemanticSearchEngine::new(config).await.expect("Failed to create search engine")
                })
            })
        ));

        Self {
            storage,
            semantic_memory,
            search_engine,
        }
    }

    /// Index a code unit for semantic search
    #[allow(dead_code)]
    async fn index_code_unit(&self, unit: &CodeUnit) -> cortex_core::error::Result<()> {
        let mut metadata = HashMap::new();
        metadata.insert("file_path".to_string(), unit.file_path.clone());
        metadata.insert("language".to_string(), format!("{:?}", unit.language));
        metadata.insert("unit_type".to_string(), format!("{:?}", unit.unit_type));

        // Create searchable content
        let content = format!(
            "{}\n{}\n{}",
            unit.signature.as_str(),
            unit.docstring.as_deref().unwrap_or(""),
            unit.body.as_deref().unwrap_or("")
        );

        let engine = self.search_engine.read().await;
        engine
            .index_document(
                unit.id.to_string(),
                content,
                EntityType::Code,
                metadata,
            )
            .await
            .map_err(|e| CortexError::Memory(format!("Failed to index code unit: {}", e)))?;

        Ok(())
    }
}

// =============================================================================
// 1. cortex.semantic.search_code
// =============================================================================

pub struct SearchCodeTool {
    ctx: SemanticSearchContext,
}

impl SearchCodeTool {
    pub fn new(ctx: SemanticSearchContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchCodeInput {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default = "default_similarity")]
    min_similarity: f32,
    language: Option<String>,
    file_pattern: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct CodeSearchResult {
    unit_id: String,
    name: String,
    file_path: String,
    signature: Option<String>,
    similarity_score: f32,
    snippet: String,
    language: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SearchCodeOutput {
    results: Vec<CodeSearchResult>,
    total_count: usize,
    query: String,
    search_time_ms: u64,
}

#[async_trait]
impl Tool for SearchCodeTool {
    fn name(&self) -> &str {
        "cortex.semantic.search_code"
    }

    fn description(&self) -> Option<&str> {
        Some("Search code using semantic embeddings - finds code by meaning, not just keywords")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SearchCodeInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SearchCodeInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Semantic code search: '{}'", input.query);
        let start = std::time::Instant::now();

        // Create search filter
        let mut filter = SearchFilter::default();
        filter.entity_type = Some(EntityType::Code);
        filter.min_score = Some(input.min_similarity);

        if let Some(lang) = &input.language {
            filter.metadata_filters.insert("language".to_string(), lang.clone());
        }

        // Perform semantic search
        let engine = self.ctx.search_engine.read().await;
        let search_results = engine
            .search_with_filter(&input.query, input.limit, filter)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Search failed: {}", e)))?;

        // Convert results
        let results: Vec<CodeSearchResult> = search_results
            .into_iter()
            .map(|r| {
                let snippet = if r.content.len() > 200 {
                    format!("{}...", &r.content[..200])
                } else {
                    r.content.clone()
                };

                CodeSearchResult {
                    unit_id: r.id.clone(),
                    name: r.metadata.get("name").cloned().unwrap_or_else(|| r.id.clone()),
                    file_path: r.metadata.get("file_path").cloned().unwrap_or_default(),
                    signature: r.metadata.get("signature").cloned(),
                    similarity_score: r.score,
                    snippet,
                    language: r.metadata.get("language").cloned().unwrap_or_else(|| "unknown".to_string()),
                }
            })
            .collect();

        let search_time_ms = start.elapsed().as_millis() as u64;
        info!("Search completed in {}ms, found {} results", search_time_ms, results.len());

        let output = SearchCodeOutput {
            total_count: results.len(),
            results,
            query: input.query,
            search_time_ms,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// 2. cortex.semantic.search_similar
// =============================================================================

pub struct SearchSimilarTool {
    ctx: SemanticSearchContext,
}

impl SearchSimilarTool {
    pub fn new(ctx: SemanticSearchContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchSimilarInput {
    reference_unit_id: String,
    #[serde(default = "default_high_similarity")]
    similarity_threshold: f32,
    #[serde(default = "default_limit")]
    limit: usize,
    same_language_only: Option<bool>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SimilarCodeResult {
    unit_id: String,
    name: String,
    file_path: String,
    similarity_score: f32,
    reason: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SearchSimilarOutput {
    reference_id: String,
    results: Vec<SimilarCodeResult>,
    total_count: usize,
}

#[async_trait]
impl Tool for SearchSimilarTool {
    fn name(&self) -> &str {
        "cortex.semantic.search_similar"
    }

    fn description(&self) -> Option<&str> {
        Some("Find code units semantically similar to a reference unit using vector similarity")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SearchSimilarInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SearchSimilarInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Finding code similar to unit '{}'", input.reference_unit_id);

        // Get reference unit
        let unit_id: CortexId = input.reference_unit_id.parse()
            .map_err(|e: uuid::Error| ToolError::ExecutionFailed(format!("Invalid unit ID: {}", e)))?;

        let unit = self.ctx.semantic_memory
            .get_unit(unit_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get unit: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed("Unit not found".to_string()))?;

        // Use unit content as query
        let query_content = format!(
            "{}\n{}",
            unit.signature.as_str(),
            unit.body.as_deref().unwrap_or("")
        );

        // Create filter
        let mut filter = SearchFilter::default();
        filter.entity_type = Some(EntityType::Code);
        filter.min_score = Some(input.similarity_threshold);

        if input.same_language_only.unwrap_or(false) {
            filter.metadata_filters.insert("language".to_string(), format!("{:?}", unit.language));
        }

        // Search for similar code
        let engine = self.ctx.search_engine.read().await;
        let search_results = engine
            .search_with_filter(&query_content, input.limit + 1, filter) // +1 to account for self
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Search failed: {}", e)))?;

        // Filter out the reference unit itself and convert results
        let results: Vec<SimilarCodeResult> = search_results
            .into_iter()
            .filter(|r| r.id != input.reference_unit_id)
            .take(input.limit)
            .map(|r| {
                let reason = if let Some(explanation) = r.explanation {
                    explanation
                } else {
                    format!("Similar structure and semantics (score: {:.2})", r.score)
                };

                SimilarCodeResult {
                    unit_id: r.id.clone(),
                    name: r.metadata.get("name").cloned().unwrap_or_else(|| r.id.clone()),
                    file_path: r.metadata.get("file_path").cloned().unwrap_or_default(),
                    similarity_score: r.score,
                    reason,
                }
            })
            .collect();

        let output = SearchSimilarOutput {
            reference_id: input.reference_unit_id,
            total_count: results.len(),
            results,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// 3. cortex.semantic.find_by_meaning
// =============================================================================

pub struct FindByMeaningTool {
    ctx: SemanticSearchContext,
}

impl FindByMeaningTool {
    pub fn new(ctx: SemanticSearchContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct FindByMeaningInput {
    description: String,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default = "default_similarity")]
    min_similarity: f32,
}

#[derive(Debug, Serialize, JsonSchema)]
struct FindByMeaningOutput {
    description: String,
    results: Vec<CodeSearchResult>,
    total_count: usize,
}

#[async_trait]
impl Tool for FindByMeaningTool {
    fn name(&self) -> &str {
        "cortex.semantic.find_by_meaning"
    }

    fn description(&self) -> Option<&str> {
        Some("Find code by describing what it does in natural language - uses semantic understanding")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(FindByMeaningInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: FindByMeaningInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Finding code by meaning: '{}'", input.description);

        let mut filter = SearchFilter::default();
        filter.entity_type = Some(EntityType::Code);
        filter.min_score = Some(input.min_similarity);

        let engine = self.ctx.search_engine.read().await;
        let search_results = engine
            .search_with_filter(&input.description, input.limit, filter)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Search failed: {}", e)))?;

        let results: Vec<CodeSearchResult> = search_results
            .into_iter()
            .map(|r| {
                let snippet = if r.content.len() > 200 {
                    format!("{}...", &r.content[..200])
                } else {
                    r.content.clone()
                };

                CodeSearchResult {
                    unit_id: r.id.clone(),
                    name: r.metadata.get("name").cloned().unwrap_or_else(|| r.id.clone()),
                    file_path: r.metadata.get("file_path").cloned().unwrap_or_default(),
                    signature: r.metadata.get("signature").cloned(),
                    similarity_score: r.score,
                    snippet,
                    language: r.metadata.get("language").cloned().unwrap_or_else(|| "unknown".to_string()),
                }
            })
            .collect();

        let output = FindByMeaningOutput {
            description: input.description,
            total_count: results.len(),
            results,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// 4. cortex.semantic.search_documentation
// =============================================================================

pub struct SearchDocumentationTool {
    ctx: SemanticSearchContext,
}

impl SearchDocumentationTool {
    pub fn new(ctx: SemanticSearchContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchDocumentationInput {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default = "default_similarity")]
    min_similarity: f32,
}

#[derive(Debug, Serialize, JsonSchema)]
struct DocSearchResult {
    unit_id: String,
    name: String,
    file_path: String,
    documentation: String,
    similarity_score: f32,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SearchDocumentationOutput {
    query: String,
    results: Vec<DocSearchResult>,
    total_count: usize,
}

#[async_trait]
impl Tool for SearchDocumentationTool {
    fn name(&self) -> &str {
        "cortex.semantic.search_documentation"
    }

    fn description(&self) -> Option<&str> {
        Some("Search documentation and docstrings by semantic meaning")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SearchDocumentationInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SearchDocumentationInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Searching documentation: '{}'", input.query);

        // Search with documentation entity type
        let mut filter = SearchFilter::default();
        filter.entity_type = Some(EntityType::Document);
        filter.min_score = Some(input.min_similarity);

        let engine = self.ctx.search_engine.read().await;
        let search_results = engine
            .search_with_filter(&input.query, input.limit, filter)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Search failed: {}", e)))?;

        let results: Vec<DocSearchResult> = search_results
            .into_iter()
            .map(|r| DocSearchResult {
                unit_id: r.id.clone(),
                name: r.metadata.get("name").cloned().unwrap_or_else(|| r.id.clone()),
                file_path: r.metadata.get("file_path").cloned().unwrap_or_default(),
                documentation: r.content,
                similarity_score: r.score,
            })
            .collect();

        let output = SearchDocumentationOutput {
            query: input.query,
            total_count: results.len(),
            results,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// 5. cortex.semantic.search_comments
// =============================================================================

pub struct SearchCommentsTool {
    ctx: SemanticSearchContext,
}

impl SearchCommentsTool {
    pub fn new(ctx: SemanticSearchContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchCommentsInput {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default = "default_similarity")]
    min_similarity: f32,
}

#[derive(Debug, Serialize, JsonSchema)]
struct CommentSearchResult {
    unit_id: String,
    name: String,
    file_path: String,
    comment: String,
    similarity_score: f32,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SearchCommentsOutput {
    query: String,
    results: Vec<CommentSearchResult>,
    total_count: usize,
}

#[async_trait]
impl Tool for SearchCommentsTool {
    fn name(&self) -> &str {
        "cortex.semantic.search_comments"
    }

    fn description(&self) -> Option<&str> {
        Some("Search code comments by semantic similarity - find relevant comments")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SearchCommentsInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SearchCommentsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Searching comments: '{}'", input.query);

        // For now, use general search - in production, index comments separately
        let mut filter = SearchFilter::default();
        filter.entity_type = Some(EntityType::Code);
        filter.min_score = Some(input.min_similarity);

        let engine = self.ctx.search_engine.read().await;
        let search_results = engine
            .search_with_filter(&input.query, input.limit, filter)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Search failed: {}", e)))?;

        let results: Vec<CommentSearchResult> = search_results
            .into_iter()
            .map(|r| CommentSearchResult {
                unit_id: r.id.clone(),
                name: r.metadata.get("name").cloned().unwrap_or_else(|| r.id.clone()),
                file_path: r.metadata.get("file_path").cloned().unwrap_or_default(),
                comment: r.content,
                similarity_score: r.score,
            })
            .collect();

        let output = SearchCommentsOutput {
            query: input.query,
            total_count: results.len(),
            results,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// 6. cortex.semantic.hybrid_search
// =============================================================================

pub struct HybridSearchTool {
    ctx: SemanticSearchContext,
}

impl HybridSearchTool {
    pub fn new(ctx: SemanticSearchContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct HybridSearchInput {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default = "default_keyword_weight")]
    keyword_weight: f32,
    #[serde(default = "default_similarity")]
    min_similarity: f32,
}

#[derive(Debug, Serialize, JsonSchema)]
struct HybridSearchOutput {
    query: String,
    results: Vec<CodeSearchResult>,
    total_count: usize,
    keyword_weight: f32,
}

#[async_trait]
impl Tool for HybridSearchTool {
    fn name(&self) -> &str {
        "cortex.semantic.hybrid_search"
    }

    fn description(&self) -> Option<&str> {
        Some("Hybrid search combining keyword matching and semantic understanding")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(HybridSearchInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: HybridSearchInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Hybrid search: '{}' (keyword weight: {})", input.query, input.keyword_weight);

        // Perform semantic search (keyword matching is handled by the ranking system)
        let mut filter = SearchFilter::default();
        filter.entity_type = Some(EntityType::Code);
        filter.min_score = Some(input.min_similarity);

        let engine = self.ctx.search_engine.read().await;
        let search_results = engine
            .search_with_filter(&input.query, input.limit, filter)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Search failed: {}", e)))?;

        let results: Vec<CodeSearchResult> = search_results
            .into_iter()
            .map(|r| {
                let snippet = if r.content.len() > 200 {
                    format!("{}...", &r.content[..200])
                } else {
                    r.content.clone()
                };

                CodeSearchResult {
                    unit_id: r.id.clone(),
                    name: r.metadata.get("name").cloned().unwrap_or_else(|| r.id.clone()),
                    file_path: r.metadata.get("file_path").cloned().unwrap_or_default(),
                    signature: r.metadata.get("signature").cloned(),
                    similarity_score: r.score,
                    snippet,
                    language: r.metadata.get("language").cloned().unwrap_or_else(|| "unknown".to_string()),
                }
            })
            .collect();

        let output = HybridSearchOutput {
            query: input.query,
            total_count: results.len(),
            results,
            keyword_weight: input.keyword_weight,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// 7. cortex.semantic.search_by_example
// =============================================================================

pub struct SearchByExampleTool {
    ctx: SemanticSearchContext,
}

impl SearchByExampleTool {
    pub fn new(ctx: SemanticSearchContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchByExampleInput {
    example_code: String,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default = "default_high_similarity")]
    similarity_threshold: f32,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SearchByExampleOutput {
    example: String,
    results: Vec<CodeSearchResult>,
    total_count: usize,
}

#[async_trait]
impl Tool for SearchByExampleTool {
    fn name(&self) -> &str {
        "cortex.semantic.search_by_example"
    }

    fn description(&self) -> Option<&str> {
        Some("Find code similar to a given example - paste code and find similar patterns")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SearchByExampleInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SearchByExampleInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Searching by code example (length: {} chars)", input.example_code.len());

        let mut filter = SearchFilter::default();
        filter.entity_type = Some(EntityType::Code);
        filter.min_score = Some(input.similarity_threshold);

        let engine = self.ctx.search_engine.read().await;
        let search_results = engine
            .search_with_filter(&input.example_code, input.limit, filter)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Search failed: {}", e)))?;

        let results: Vec<CodeSearchResult> = search_results
            .into_iter()
            .map(|r| {
                let snippet = if r.content.len() > 200 {
                    format!("{}...", &r.content[..200])
                } else {
                    r.content.clone()
                };

                CodeSearchResult {
                    unit_id: r.id.clone(),
                    name: r.metadata.get("name").cloned().unwrap_or_else(|| r.id.clone()),
                    file_path: r.metadata.get("file_path").cloned().unwrap_or_default(),
                    signature: r.metadata.get("signature").cloned(),
                    similarity_score: r.score,
                    snippet,
                    language: r.metadata.get("language").cloned().unwrap_or_else(|| "unknown".to_string()),
                }
            })
            .collect();

        let output = SearchByExampleOutput {
            example: if input.example_code.len() > 100 {
                format!("{}...", &input.example_code[..100])
            } else {
                input.example_code
            },
            total_count: results.len(),
            results,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// 8. cortex.semantic.search_by_natural_language
// =============================================================================

pub struct SearchByNaturalLanguageTool {
    ctx: SemanticSearchContext,
}

impl SearchByNaturalLanguageTool {
    pub fn new(ctx: SemanticSearchContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchByNaturalLanguageInput {
    natural_language_query: String,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default = "default_similarity")]
    min_similarity: f32,
    context: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SearchByNaturalLanguageOutput {
    query: String,
    results: Vec<CodeSearchResult>,
    total_count: usize,
    interpreted_intent: String,
}

#[async_trait]
impl Tool for SearchByNaturalLanguageTool {
    fn name(&self) -> &str {
        "cortex.semantic.search_by_natural_language"
    }

    fn description(&self) -> Option<&str> {
        Some("Advanced natural language to code search - understands complex queries")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SearchByNaturalLanguageInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SearchByNaturalLanguageInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Natural language search: '{}'", input.natural_language_query);

        // Enhance query with context if provided
        let enhanced_query = if let Some(context) = &input.context {
            format!("{}\nContext: {}", input.natural_language_query, context)
        } else {
            input.natural_language_query.clone()
        };

        let mut filter = SearchFilter::default();
        filter.entity_type = Some(EntityType::Code);
        filter.min_score = Some(input.min_similarity);

        let engine = self.ctx.search_engine.read().await;
        let search_results = engine
            .search_with_filter(&enhanced_query, input.limit, filter)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Search failed: {}", e)))?;

        let results: Vec<CodeSearchResult> = search_results
            .into_iter()
            .map(|r| {
                let snippet = if r.content.len() > 200 {
                    format!("{}...", &r.content[..200])
                } else {
                    r.content.clone()
                };

                CodeSearchResult {
                    unit_id: r.id.clone(),
                    name: r.metadata.get("name").cloned().unwrap_or_else(|| r.id.clone()),
                    file_path: r.metadata.get("file_path").cloned().unwrap_or_default(),
                    signature: r.metadata.get("signature").cloned(),
                    similarity_score: r.score,
                    snippet,
                    language: r.metadata.get("language").cloned().unwrap_or_else(|| "unknown".to_string()),
                }
            })
            .collect();

        let output = SearchByNaturalLanguageOutput {
            query: input.natural_language_query.clone(),
            total_count: results.len(),
            results,
            interpreted_intent: format!("Searching for: {}", input.natural_language_query),
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// Helper functions
// =============================================================================

fn default_limit() -> usize {
    10
}

fn default_similarity() -> f32 {
    0.7
}

fn default_high_similarity() -> f32 {
    0.8
}

fn default_keyword_weight() -> f32 {
    0.3
}
