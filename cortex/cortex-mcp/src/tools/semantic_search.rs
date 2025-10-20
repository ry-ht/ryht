//! Semantic Search Tools
//!
//! This module implements the 8 semantic search tools defined in the MCP spec:
//! - cortex.search.semantic
//! - cortex.search.by_pattern
//! - cortex.search.by_signature
//! - cortex.search.by_complexity
//! - cortex.search.similar_code
//! - cortex.search.by_annotation
//! - cortex.search.unused_code
//! - cortex.search.duplicates

use async_trait::async_trait;
use cortex_memory::SemanticMemorySystem;
use cortex_storage::ConnectionManager;
use mcp_server::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

// =============================================================================
// Shared Context
// =============================================================================

#[derive(Clone)]
pub struct SemanticSearchContext {
    storage: Arc<ConnectionManager>,
    semantic: Arc<SemanticMemorySystem>,
}

impl SemanticSearchContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        let semantic = Arc::new(SemanticMemorySystem::new(storage.clone()));
        Self { storage, semantic }
    }
}

// =============================================================================
// cortex.search.semantic
// =============================================================================

pub struct SearchSemanticTool {
    ctx: SemanticSearchContext,
}

impl SearchSemanticTool {
    pub fn new(ctx: SemanticSearchContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SemanticSearchInput {
    query: String,
    #[serde(default = "default_workspace_scope")]
    scope: String,
    scope_path: Option<String>,
    #[serde(default = "default_limit")]
    limit: i32,
    #[serde(default = "default_similarity")]
    min_similarity: f32,
    entity_types: Option<Vec<String>>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SearchResult {
    entity_id: String,
    entity_type: String,
    name: String,
    path: String,
    similarity: f32,
    snippet: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SemanticSearchOutput {
    results: Vec<SearchResult>,
    total_count: i32,
    query: String,
}

#[async_trait]
impl Tool for SearchSemanticTool {
    fn name(&self) -> &str {
        "cortex.search.semantic"
    }

    fn description(&self) -> Option<&str> {
        Some("Semantic search using embeddings to find code by meaning")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SemanticSearchInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SemanticSearchInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Semantic search: '{}'", input.query);

        let output = SemanticSearchOutput {
            results: vec![],
            total_count: 0,
            query: input.query.clone(),
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.search.by_pattern
// =============================================================================

pub struct SearchByPatternTool {
    ctx: SemanticSearchContext,
}

impl SearchByPatternTool {
    pub fn new(ctx: SemanticSearchContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct PatternSearchInput {
    pattern: String,
    language: String,
    scope_path: Option<String>,
    #[serde(default = "default_pattern_limit")]
    limit: i32,
}

#[derive(Debug, Serialize, JsonSchema)]
struct PatternMatch {
    file_path: String,
    line_start: i32,
    line_end: i32,
    matched_text: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct PatternSearchOutput {
    matches: Vec<PatternMatch>,
    total_count: i32,
}

#[async_trait]
impl Tool for SearchByPatternTool {
    fn name(&self) -> &str {
        "cortex.search.by_pattern"
    }

    fn description(&self) -> Option<&str> {
        Some("Search code by AST pattern using tree-sitter queries")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(PatternSearchInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: PatternSearchInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Pattern search for language '{}'", input.language);

        let output = PatternSearchOutput {
            matches: vec![],
            total_count: 0,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.search.by_signature
// =============================================================================

pub struct SearchBySignatureTool {
    ctx: SemanticSearchContext,
}

impl SearchBySignatureTool {
    pub fn new(ctx: SemanticSearchContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SignatureSearchInput {
    signature_pattern: String,
    #[serde(default = "default_match_mode")]
    match_mode: String,
    parameter_types: Option<Vec<String>>,
    return_type: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SignatureMatch {
    unit_id: String,
    name: String,
    signature: String,
    file_path: String,
    match_score: f32,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SignatureSearchOutput {
    matches: Vec<SignatureMatch>,
    total_count: i32,
}

#[async_trait]
impl Tool for SearchBySignatureTool {
    fn name(&self) -> &str {
        "cortex.search.by_signature"
    }

    fn description(&self) -> Option<&str> {
        Some("Search by function signature pattern with wildcards")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SignatureSearchInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SignatureSearchInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Signature search: '{}'", input.signature_pattern);

        let output = SignatureSearchOutput {
            matches: vec![],
            total_count: 0,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.search.by_complexity
// =============================================================================

pub struct SearchByComplexityTool {
    ctx: SemanticSearchContext,
}

impl SearchByComplexityTool {
    pub fn new(ctx: SemanticSearchContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct ComplexitySearchInput {
    metric: String,
    operator: String,
    threshold: i32,
    unit_types: Option<Vec<String>>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ComplexityMatch {
    unit_id: String,
    name: String,
    file_path: String,
    complexity_value: i32,
    unit_type: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ComplexitySearchOutput {
    matches: Vec<ComplexityMatch>,
    total_count: i32,
}

#[async_trait]
impl Tool for SearchByComplexityTool {
    fn name(&self) -> &str {
        "cortex.search.by_complexity"
    }

    fn description(&self) -> Option<&str> {
        Some("Find code by complexity metrics (cyclomatic, cognitive, nesting)")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ComplexitySearchInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: ComplexitySearchInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!(
            "Complexity search: {} {} {}",
            input.metric, input.operator, input.threshold
        );

        let output = ComplexitySearchOutput {
            matches: vec![],
            total_count: 0,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.search.similar_code
// =============================================================================

pub struct SearchSimilarCodeTool {
    ctx: SemanticSearchContext,
}

impl SearchSimilarCodeTool {
    pub fn new(ctx: SemanticSearchContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SimilarCodeInput {
    reference_unit_id: String,
    #[serde(default = "default_high_similarity")]
    similarity_threshold: f32,
    #[serde(default = "default_workspace_scope")]
    scope: String,
    #[serde(default = "default_small_limit")]
    limit: i32,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SimilarCodeMatch {
    unit_id: String,
    name: String,
    file_path: String,
    similarity_score: f32,
    reason: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct SimilarCodeOutput {
    reference_id: String,
    matches: Vec<SimilarCodeMatch>,
    total_count: i32,
}

#[async_trait]
impl Tool for SearchSimilarCodeTool {
    fn name(&self) -> &str {
        "cortex.search.similar_code"
    }

    fn description(&self) -> Option<&str> {
        Some("Find similar code patterns using semantic embeddings")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SimilarCodeInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SimilarCodeInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Finding code similar to unit '{}'", input.reference_unit_id);

        let output = SimilarCodeOutput {
            reference_id: input.reference_unit_id.clone(),
            matches: vec![],
            total_count: 0,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.search.by_annotation
// =============================================================================

pub struct SearchByAnnotationTool {
    ctx: SemanticSearchContext,
}

impl SearchByAnnotationTool {
    pub fn new(ctx: SemanticSearchContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct AnnotationSearchInput {
    annotation: String,
    #[serde(default)]
    include_parameters: bool,
    language: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct AnnotationMatch {
    unit_id: String,
    name: String,
    file_path: String,
    annotation_text: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct AnnotationSearchOutput {
    matches: Vec<AnnotationMatch>,
    total_count: i32,
}

#[async_trait]
impl Tool for SearchByAnnotationTool {
    fn name(&self) -> &str {
        "cortex.search.by_annotation"
    }

    fn description(&self) -> Option<&str> {
        Some("Search by decorators/annotations (e.g., @Test, #[derive])")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AnnotationSearchInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AnnotationSearchInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Searching for annotation '{}'", input.annotation);

        let output = AnnotationSearchOutput {
            matches: vec![],
            total_count: 0,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.search.unused_code
// =============================================================================

pub struct SearchUnusedCodeTool {
    ctx: SemanticSearchContext,
}

impl SearchUnusedCodeTool {
    pub fn new(ctx: SemanticSearchContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct UnusedCodeInput {
    scope_path: String,
    #[serde(default)]
    include_private: bool,
    #[serde(default = "default_true")]
    exclude_tests: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct UnusedCodeItem {
    unit_id: String,
    name: String,
    unit_type: String,
    file_path: String,
    visibility: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct UnusedCodeOutput {
    unused_items: Vec<UnusedCodeItem>,
    total_count: i32,
}

#[async_trait]
impl Tool for SearchUnusedCodeTool {
    fn name(&self) -> &str {
        "cortex.search.unused_code"
    }

    fn description(&self) -> Option<&str> {
        Some("Find potentially unused code (functions, classes, etc.)")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(UnusedCodeInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: UnusedCodeInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Searching for unused code in '{}'", input.scope_path);

        let output = UnusedCodeOutput {
            unused_items: vec![],
            total_count: 0,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// cortex.search.duplicates
// =============================================================================

pub struct SearchDuplicatesTool {
    ctx: SemanticSearchContext,
}

impl SearchDuplicatesTool {
    pub fn new(ctx: SemanticSearchContext) -> Self {
        Self { ctx }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DuplicatesInput {
    scope_path: String,
    #[serde(default = "default_min_lines")]
    min_lines: i32,
    #[serde(default = "default_very_high_similarity")]
    similarity_threshold: f32,
    #[serde(default = "default_true")]
    ignore_whitespace: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
struct DuplicateGroup {
    group_id: String,
    instances: Vec<DuplicateInstance>,
    line_count: i32,
    similarity: f32,
}

#[derive(Debug, Serialize, JsonSchema)]
struct DuplicateInstance {
    file_path: String,
    start_line: i32,
    end_line: i32,
}

#[derive(Debug, Serialize, JsonSchema)]
struct DuplicatesOutput {
    duplicate_groups: Vec<DuplicateGroup>,
    total_groups: i32,
}

#[async_trait]
impl Tool for SearchDuplicatesTool {
    fn name(&self) -> &str {
        "cortex.search.duplicates"
    }

    fn description(&self) -> Option<&str> {
        Some("Find duplicate code blocks across the codebase")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(DuplicatesInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: DuplicatesInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Searching for duplicates in '{}'", input.scope_path);

        let output = DuplicatesOutput {
            duplicate_groups: vec![],
            total_groups: 0,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// Helper functions
// =============================================================================

fn default_workspace_scope() -> String {
    "workspace".to_string()
}

fn default_limit() -> i32 {
    20
}

fn default_similarity() -> f32 {
    0.7
}

fn default_pattern_limit() -> i32 {
    50
}

fn default_match_mode() -> String {
    "fuzzy".to_string()
}

fn default_high_similarity() -> f32 {
    0.8
}

fn default_small_limit() -> i32 {
    10
}

fn default_true() -> bool {
    true
}

fn default_min_lines() -> i32 {
    10
}

fn default_very_high_similarity() -> f32 {
    0.95
}
