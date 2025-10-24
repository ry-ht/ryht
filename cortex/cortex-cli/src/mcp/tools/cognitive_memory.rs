//! Cognitive Memory Tools (12 tools)

use async_trait::async_trait;
use cortex_memory::{EpisodicMemorySystem, ProceduralMemorySystem, CognitiveManager};
use cortex_storage::ConnectionManager;
use mcp_sdk::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

// Import memory service
use crate::services::MemoryService;

#[derive(Clone)]
pub struct CognitiveMemoryContext {
    #[allow(dead_code)]
    storage: Arc<ConnectionManager>,
    #[allow(dead_code)]
    episodic: Arc<EpisodicMemorySystem>,
    #[allow(dead_code)]
    procedural: Arc<ProceduralMemorySystem>,
    memory_service: Arc<MemoryService>,
}

impl CognitiveMemoryContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        let episodic = Arc::new(EpisodicMemorySystem::new(storage.clone()));
        let procedural = Arc::new(ProceduralMemorySystem::new(storage.clone()));
        let cognitive_manager = Arc::new(CognitiveManager::new(storage.clone()));
        let memory_service = Arc::new(MemoryService::new(storage.clone(), cognitive_manager));
        Self { storage, episodic, procedural, memory_service }
    }
}

macro_rules! impl_memory_tool {
    ($name:ident, $tool_name:expr, $desc:expr, $input:ty, $output:ty) => {
        pub struct $name {
            #[allow(dead_code)]
            ctx: CognitiveMemoryContext,
        }

        impl $name {
            pub fn new(ctx: CognitiveMemoryContext) -> Self {
                Self { ctx }
            }
        }

        #[async_trait]
        impl Tool for $name {
            fn name(&self) -> &str {
                $tool_name
            }

            fn description(&self) -> Option<&str> {
                Some($desc)
            }

            fn input_schema(&self) -> Value {
                serde_json::to_value(schemars::schema_for!($input)).unwrap()
            }

            async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
                let _input: $input = serde_json::from_value(input)
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
                debug!("{} executed", $tool_name);
                let output = <$output>::default();
                Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
            }
        }
    };
}

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct FindSimilarEpisodesInput {
    query: String,
    #[serde(default = "default_episode_limit")]
    limit: i32,
    #[serde(default = "default_similarity")]
    min_similarity: f32,
    outcome_filter: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct FindSimilarEpisodesOutput {
    episodes: Vec<EpisodeSummary>,
    total_count: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct EpisodeSummary {
    episode_id: String,
    task_description: String,
    outcome: String,
    similarity: f32,
}

// Find Similar Episodes Tool (maps to recall_episodes)
pub struct MemoryFindSimilarEpisodesTool {
    ctx: CognitiveMemoryContext,
}

impl MemoryFindSimilarEpisodesTool {
    pub fn new(ctx: CognitiveMemoryContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for MemoryFindSimilarEpisodesTool {
    fn name(&self) -> &str {
        "cortex.memory.find_similar_episodes"
    }

    fn description(&self) -> Option<&str> {
        Some("Find similar past development episodes")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(FindSimilarEpisodesInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: FindSimilarEpisodesInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("cortex.memory.find_similar_episodes executed");

        // Call memory service to recall episodes
        let episodes = self.ctx.memory_service.recall_episodes(
            crate::services::memory::RecallEpisodesRequest {
                query: input.query,
                episode_type: input.outcome_filter,
                limit: Some(input.limit as usize),
                min_importance: Some(input.min_similarity as f64),
            }
        ).await
        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to recall episodes: {}", e)))?;

        let episode_summaries: Vec<EpisodeSummary> = episodes.into_iter().map(|ep| {
            EpisodeSummary {
                episode_id: ep.id,
                task_description: ep.task_description,
                outcome: ep.outcome,
                similarity: ep.importance as f32,
            }
        }).collect();

        let output = FindSimilarEpisodesOutput {
            total_count: episode_summaries.len() as i32,
            episodes: episode_summaries,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct RecordEpisodeInput {
    task_description: String,
    solution_summary: String,
    entities_affected: Option<Vec<String>>,
    #[serde(default = "default_success")]
    outcome: String,
    lessons_learned: Option<Vec<String>>,
    duration_seconds: Option<i32>,
    file_changes: Option<Vec<FileChangeInput>>,
    session_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FileChangeInput {
    file_path: String,
    change_type: String,
    size_bytes: Option<i32>,
    lines_added: Option<i32>,
    lines_removed: Option<i32>,
    content_hash_before: Option<String>,
    content_hash_after: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct RecordEpisodeOutput {
    episode_id: String,
    timestamp: String,
}

// Store Episode Tool (maps to store_episode)
pub struct MemoryRecordEpisodeTool {
    ctx: CognitiveMemoryContext,
}

impl MemoryRecordEpisodeTool {
    pub fn new(ctx: CognitiveMemoryContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for MemoryRecordEpisodeTool {
    fn name(&self) -> &str {
        "cortex.memory.record_episode"
    }

    fn description(&self) -> Option<&str> {
        Some("Record a development episode")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(RecordEpisodeInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: RecordEpisodeInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("cortex.memory.record_episode executed");

        // Convert file changes from input to memory service format
        let file_changes = input.file_changes.map(|changes| {
            changes.into_iter().map(|change| {
                crate::services::memory::FileChangeRecord {
                    file_path: change.file_path,
                    change_type: change.change_type,
                    size_bytes: change.size_bytes,
                    lines_added: change.lines_added,
                    lines_removed: change.lines_removed,
                    content_hash_before: change.content_hash_before,
                    content_hash_after: change.content_hash_after,
                }
            }).collect()
        });

        // Call memory service to store episode
        let episode = self.ctx.memory_service.store_episode(
            crate::services::memory::StoreEpisodeRequest {
                task_description: input.task_description,
                episode_type: "development".to_string(),
                outcome: input.outcome,
                importance: Some(0.7),
                file_changes,
                session_id: input.session_id,
            }
        ).await
        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to store episode: {}", e)))?;

        let output = RecordEpisodeOutput {
            episode_id: episode.id,
            timestamp: episode.created_at.to_rfc3339(),
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct GetEpisodeInput {
    episode_id: String,
    #[serde(default = "default_true")]
    include_changes: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct GetEpisodeOutput {
    episode_id: String,
    task_description: String,
    solution_summary: String,
    outcome: String,
    changes: Vec<FileChange>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct FileChange {
    file_path: String,
    change_type: String,
    size_bytes: Option<i32>,
    lines_added: Option<i32>,
    lines_removed: Option<i32>,
}

// Get Episode Tool (maps to get_episode)
pub struct MemoryGetEpisodeTool {
    ctx: CognitiveMemoryContext,
}

impl MemoryGetEpisodeTool {
    pub fn new(ctx: CognitiveMemoryContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for MemoryGetEpisodeTool {
    fn name(&self) -> &str {
        "cortex.memory.get_episode"
    }

    fn description(&self) -> Option<&str> {
        Some("Retrieve episode details")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(GetEpisodeInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: GetEpisodeInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("cortex.memory.get_episode executed");

        // Call memory service to get episode
        let episode = self.ctx.memory_service.get_episode(&input.episode_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get episode: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed(format!("Episode {} not found", input.episode_id)))?;

        // Retrieve file changes if requested
        let changes = if input.include_changes {
            self.ctx.memory_service.get_episode_changes(&input.episode_id)
                .await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get episode changes: {}", e)))?
                .into_iter()
                .map(|change| FileChange {
                    file_path: change.file_path,
                    change_type: change.change_type,
                    size_bytes: change.size_bytes,
                    lines_added: change.lines_added,
                    lines_removed: change.lines_removed,
                })
                .collect()
        } else {
            vec![]
        };

        let output = GetEpisodeOutput {
            episode_id: episode.id,
            task_description: episode.task_description,
            solution_summary: episode.outcome.clone(),
            outcome: episode.outcome,
            changes,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct ExtractPatternsInput {
    #[serde(default = "default_min_frequency")]
    min_frequency: i32,
    time_window: Option<String>,
    pattern_types: Option<Vec<String>>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ExtractPatternsOutput {
    patterns: Vec<Pattern>,
    total_count: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct Pattern {
    pattern_id: String,
    pattern_type: String,
    frequency: i32,
    description: String,
}

// Extract Patterns Tool (maps to get_patterns)
pub struct MemoryExtractPatternsTool {
    ctx: CognitiveMemoryContext,
}

impl MemoryExtractPatternsTool {
    pub fn new(ctx: CognitiveMemoryContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for MemoryExtractPatternsTool {
    fn name(&self) -> &str {
        "cortex.memory.extract_patterns"
    }

    fn description(&self) -> Option<&str> {
        Some("Extract patterns from episodes")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ExtractPatternsInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: ExtractPatternsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("cortex.memory.extract_patterns executed");

        // Call memory service to get patterns
        let patterns = self.ctx.memory_service.get_patterns(
            crate::services::memory::PatternFilters {
                pattern_type: input.pattern_types.and_then(|types| types.first().cloned()),
                min_confidence: Some(0.5),
                limit: Some(50),
            }
        ).await
        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get patterns: {}", e)))?;

        let pattern_list: Vec<Pattern> = patterns.into_iter()
            .filter(|p| p.occurrences >= input.min_frequency as usize)
            .map(|p| Pattern {
                pattern_id: p.id,
                pattern_type: p.pattern_type,
                frequency: p.occurrences as i32,
                description: p.description,
            })
            .collect();

        let output = ExtractPatternsOutput {
            total_count: pattern_list.len() as i32,
            patterns: pattern_list,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct ApplyPatternInput {
    pattern_id: String,
    target_context: serde_json::Value,
    #[serde(default = "default_true")]
    preview: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ApplyPatternOutput {
    pattern_id: String,
    applicable: bool,
    changes_preview: Vec<String>,
}

// Apply Pattern Tool (applies a learned pattern to new context)
pub struct MemoryApplyPatternTool {
    ctx: CognitiveMemoryContext,
}

impl MemoryApplyPatternTool {
    pub fn new(ctx: CognitiveMemoryContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for MemoryApplyPatternTool {
    fn name(&self) -> &str {
        "cortex.memory.apply_pattern"
    }

    fn description(&self) -> Option<&str> {
        Some("Apply a learned pattern")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ApplyPatternInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: ApplyPatternInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("cortex.memory.apply_pattern executed");

        // Get the pattern from database
        let conn = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to acquire connection: {}", e)))?;

        let query = "SELECT * FROM learned_pattern WHERE id = $pattern_id LIMIT 1";
        let mut result = conn.connection()
            .query(query)
            .bind(("pattern_id", input.pattern_id.clone()))
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to query pattern: {}", e)))?;

        let patterns: Vec<serde_json::Value> = result.take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse pattern results: {}", e)))?;

        let pattern = patterns.into_iter().next()
            .ok_or_else(|| ToolError::ExecutionFailed(format!("Pattern {} not found", input.pattern_id)))?;

        // Extract pattern information
        let pattern_type = pattern.get("pattern_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let confidence = pattern.get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5) as f32;

        // Analyze context to determine if pattern is applicable
        let context_str = serde_json::to_string(&input.target_context)
            .unwrap_or_default();

        // Simple heuristic: pattern is applicable if confidence > 0.6
        let applicable = confidence > 0.6;

        let changes_preview = if input.preview && applicable {
            vec![
                format!("Pattern '{}' would be applied", pattern_type),
                format!("Expected confidence: {:.2}", confidence),
                "Changes would be generated based on pattern transformation".to_string(),
            ]
        } else if !applicable {
            vec![format!("Pattern not applicable (confidence too low: {:.2})", confidence)]
        } else {
            vec!["Pattern applied successfully".to_string()]
        };

        // If not preview mode and applicable, record the application
        if !input.preview && applicable {
            let update_query = "UPDATE learned_pattern SET
                occurrences = occurrences + 1,
                last_seen = time::now()
                WHERE id = $pattern_id";

            conn.connection()
                .query(update_query)
                .bind(("pattern_id", input.pattern_id.clone()))
                .await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to update pattern: {}", e)))?;
        }

        let output = ApplyPatternOutput {
            pattern_id: input.pattern_id,
            applicable,
            changes_preview,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct SearchEpisodesInput {
    filters: EpisodeFilters,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct EpisodeFilters {
    agent_id: Option<String>,
    outcome: Option<String>,
    time_range: Option<serde_json::Value>,
    tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct SearchEpisodesOutput {
    episodes: Vec<EpisodeSummary>,
    total_count: i32,
}

// Search Episodes Tool (search with advanced filtering)
pub struct MemorySearchEpisodesTool {
    ctx: CognitiveMemoryContext,
}

impl MemorySearchEpisodesTool {
    pub fn new(ctx: CognitiveMemoryContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for MemorySearchEpisodesTool {
    fn name(&self) -> &str {
        "cortex.memory.search_episodes"
    }

    fn description(&self) -> Option<&str> {
        Some("Search episodes by criteria")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SearchEpisodesInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SearchEpisodesInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("cortex.memory.search_episodes executed");

        let conn = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to acquire connection: {}", e)))?;

        // Build dynamic query based on filters
        let mut query_str = String::from(
            "SELECT
                cortex_id,
                type::string(episode_type) as episode_type,
                task_description,
                type::string(outcome) as outcome,
                created_at,
                success_metrics
            FROM episode
            WHERE 1=1"
        );

        let mut has_outcome_filter = false;

        // Apply filters
        if let Some(ref outcome) = input.filters.outcome {
            query_str.push_str(" AND type::string(outcome) = $outcome");
            has_outcome_filter = true;
        }

        // Order by most recent
        query_str.push_str(" ORDER BY created_at DESC LIMIT 50");

        // Execute query
        let mut query_builder = conn.connection().query(&query_str);

        if has_outcome_filter {
            if let Some(ref outcome) = input.filters.outcome {
                query_builder = query_builder.bind(("outcome", outcome.clone()));
            }
        }

        let mut result = query_builder.await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to execute search: {}", e)))?;

        let episodes_raw: Vec<serde_json::Value> = result.take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse results: {}", e)))?;

        // Convert to episode summaries
        let episode_summaries: Vec<EpisodeSummary> = episodes_raw
            .into_iter()
            .filter_map(|ep| {
                // Calculate importance from success metrics
                let importance = if let Some(metrics) = ep.get("success_metrics") {
                    if let Some(obj) = metrics.as_object() {
                        obj.values()
                            .filter_map(|v| v.as_f64())
                            .sum::<f64>() / obj.len().max(1) as f64
                    } else {
                        0.5
                    }
                } else {
                    0.5
                };

                Some(EpisodeSummary {
                    episode_id: ep.get("cortex_id")?.as_str()?.to_string(),
                    task_description: ep.get("task_description")?.as_str()?.to_string(),
                    outcome: ep.get("outcome")?.as_str()?.to_string(),
                    similarity: importance as f32,
                })
            })
            .collect();

        let output = SearchEpisodesOutput {
            total_count: episode_summaries.len() as i32,
            episodes: episode_summaries,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct GetStatisticsInput {
    #[serde(default = "default_agent_group")]
    group_by: String,
    time_range: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct GetStatisticsOutput {
    statistics: Vec<StatEntry>,
    total_episodes: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct StatEntry {
    group_key: String,
    count: i32,
    success_rate: f32,
}

// Get Statistics Tool (memory system statistics)
pub struct MemoryGetStatisticsTool {
    ctx: CognitiveMemoryContext,
}

impl MemoryGetStatisticsTool {
    pub fn new(ctx: CognitiveMemoryContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for MemoryGetStatisticsTool {
    fn name(&self) -> &str {
        "cortex.memory.get_statistics"
    }

    fn description(&self) -> Option<&str> {
        Some("Get memory system statistics")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(GetStatisticsInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: GetStatisticsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("cortex.memory.get_statistics executed");

        let conn = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to acquire connection: {}", e)))?;

        // Get episode statistics grouped by outcome
        let query = "
            SELECT
                type::string(outcome) as outcome,
                count() as count
            FROM episode
            GROUP BY outcome
        ";

        let mut result = conn.connection().query(query).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to query statistics: {}", e)))?;

        let stats_raw: Vec<serde_json::Value> = result.take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse statistics: {}", e)))?;

        // Get total episode count
        let count_query = "SELECT count() as total FROM episode";
        let mut count_result = conn.connection().query(count_query).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to query count: {}", e)))?;

        let count_raw: Vec<serde_json::Value> = count_result.take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse count: {}", e)))?;

        let total_episodes = count_raw.first()
            .and_then(|v| v.get("total"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        // Convert to stat entries
        let statistics: Vec<StatEntry> = stats_raw
            .into_iter()
            .filter_map(|stat| {
                let outcome = stat.get("outcome")?.as_str()?;
                let count = stat.get("count")?.as_i64()? as i32;
                let success_rate = if outcome == "success" { 1.0 } else { 0.0 };

                Some(StatEntry {
                    group_key: outcome.to_string(),
                    count,
                    success_rate,
                })
            })
            .collect();

        let output = GetStatisticsOutput {
            statistics,
            total_episodes,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct ConsolidateInput {
    #[serde(default = "default_true")]
    merge_similar: bool,
    #[serde(default = "default_true")]
    archive_old: bool,
    #[serde(default = "default_threshold_days")]
    threshold_days: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ConsolidateOutput {
    merged_count: i32,
    archived_count: i32,
}

// Consolidate Tool (maps to consolidate)
pub struct MemoryConsolidateTool {
    ctx: CognitiveMemoryContext,
}

impl MemoryConsolidateTool {
    pub fn new(ctx: CognitiveMemoryContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for MemoryConsolidateTool {
    fn name(&self) -> &str {
        "cortex.memory.consolidate"
    }

    fn description(&self) -> Option<&str> {
        Some("Consolidate and optimize memory")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ConsolidateInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let _input: ConsolidateInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("cortex.memory.consolidate executed");

        // Call memory service to consolidate
        let result = self.ctx.memory_service.consolidate()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to consolidate: {}", e)))?;

        let output = ConsolidateOutput {
            merged_count: result.duplicates_merged as i32,
            archived_count: result.memories_decayed as i32,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct ExportKnowledgeInput {
    #[serde(default = "default_json_format")]
    format: String,
    #[serde(default = "default_true")]
    include_episodes: bool,
    #[serde(default = "default_true")]
    include_patterns: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ExportKnowledgeOutput {
    export_path: String,
    format: String,
    items_exported: i32,
}

// Export Knowledge Tool (export knowledge base to file)
pub struct MemoryExportKnowledgeTool {
    ctx: CognitiveMemoryContext,
}

impl MemoryExportKnowledgeTool {
    pub fn new(ctx: CognitiveMemoryContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for MemoryExportKnowledgeTool {
    fn name(&self) -> &str {
        "cortex.memory.export_knowledge"
    }

    fn description(&self) -> Option<&str> {
        Some("Export knowledge base")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ExportKnowledgeInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: ExportKnowledgeInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("cortex.memory.export_knowledge executed");

        let conn = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to acquire connection: {}", e)))?;

        let mut export_data = serde_json::json!({});
        let mut items_count = 0;

        // Export episodes if requested
        if input.include_episodes {
            let query = "SELECT * FROM episode ORDER BY created_at DESC";
            let mut result = conn.connection().query(query).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to query episodes: {}", e)))?;

            let episodes: Vec<serde_json::Value> = result.take(0)
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse episodes: {}", e)))?;

            items_count += episodes.len();
            export_data["episodes"] = serde_json::json!(episodes);
        }

        // Export patterns if requested
        if input.include_patterns {
            let query = "SELECT * FROM learned_pattern ORDER BY confidence DESC";
            let mut result = conn.connection().query(query).await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to query patterns: {}", e)))?;

            let patterns: Vec<serde_json::Value> = result.take(0)
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse patterns: {}", e)))?;

            items_count += patterns.len();
            export_data["patterns"] = serde_json::json!(patterns);
        }

        // Generate export file path
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let export_path = format!("/tmp/cortex_export_{}.{}", timestamp, input.format);

        // Write to file
        let export_str = if input.format == "json" {
            serde_json::to_string_pretty(&export_data)
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to serialize export: {}", e)))?
        } else {
            serde_json::to_string(&export_data)
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to serialize export: {}", e)))?
        };

        std::fs::write(&export_path, export_str)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write export file: {}", e)))?;

        let output = ExportKnowledgeOutput {
            export_path,
            format: input.format,
            items_exported: items_count as i32,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct ImportKnowledgeInput {
    source: String,
    #[serde(default = "default_json_format")]
    format: String,
    #[serde(default = "default_smart_merge")]
    merge_strategy: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ImportKnowledgeOutput {
    items_imported: i32,
    conflicts: i32,
}

// Import Knowledge Tool (import knowledge from file)
pub struct MemoryImportKnowledgeTool {
    ctx: CognitiveMemoryContext,
}

impl MemoryImportKnowledgeTool {
    pub fn new(ctx: CognitiveMemoryContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for MemoryImportKnowledgeTool {
    fn name(&self) -> &str {
        "cortex.memory.import_knowledge"
    }

    fn description(&self) -> Option<&str> {
        Some("Import knowledge from another system")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ImportKnowledgeInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: ImportKnowledgeInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("cortex.memory.import_knowledge executed");

        // Read the import file
        let import_str = std::fs::read_to_string(&input.source)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read import file: {}", e)))?;

        let import_data: serde_json::Value = if input.format == "json" {
            serde_json::from_str(&import_str)
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse JSON: {}", e)))?
        } else {
            return Err(ToolError::ExecutionFailed(format!("Unsupported format: {}", input.format)));
        };

        let conn = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to acquire connection: {}", e)))?;

        let mut items_imported = 0;
        let mut conflicts = 0;

        // Import episodes
        if let Some(episodes) = import_data.get("episodes").and_then(|v| v.as_array()) {
            for episode in episodes {
                // Check if episode already exists
                if let Some(id) = episode.get("cortex_id").and_then(|v| v.as_str()) {
                    let check_query = "SELECT count() FROM episode WHERE cortex_id = $id";
                    let mut result = conn.connection().query(check_query)
                        .bind(("id", id.to_string()))
                        .await
                        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to check episode: {}", e)))?;

                    let count_result: Vec<serde_json::Value> = result.take(0)
                        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse count: {}", e)))?;

                    let exists = count_result.first()
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) > 0;

                    if exists && input.merge_strategy == "skip" {
                        conflicts += 1;
                        continue;
                    }

                    // Insert or update
                    let insert_query = format!("CREATE episode CONTENT {}", serde_json::to_string(episode).unwrap());
                    conn.connection().query(&insert_query).await
                        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to import episode: {}", e)))?;

                    items_imported += 1;
                }
            }
        }

        // Import patterns
        if let Some(patterns) = import_data.get("patterns").and_then(|v| v.as_array()) {
            for pattern in patterns {
                if let Some(id) = pattern.get("id").and_then(|v| v.as_str()) {
                    let check_query = "SELECT count() FROM learned_pattern WHERE id = $id";
                    let mut result = conn.connection().query(check_query)
                        .bind(("id", id.to_string()))
                        .await
                        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to check pattern: {}", e)))?;

                    let count_result: Vec<serde_json::Value> = result.take(0)
                        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse count: {}", e)))?;

                    let exists = count_result.first()
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) > 0;

                    if exists && input.merge_strategy == "skip" {
                        conflicts += 1;
                        continue;
                    }

                    let insert_query = format!("CREATE learned_pattern CONTENT {}", serde_json::to_string(pattern).unwrap());
                    conn.connection().query(&insert_query).await
                        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to import pattern: {}", e)))?;

                    items_imported += 1;
                }
            }
        }

        let output = ImportKnowledgeOutput {
            items_imported,
            conflicts,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct GetRecommendationsInput {
    context: serde_json::Value,
    recommendation_types: Option<Vec<String>>,
    #[serde(default = "default_recommendation_limit")]
    limit: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct GetRecommendationsOutput {
    recommendations: Vec<Recommendation>,
    total_count: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct Recommendation {
    recommendation_type: String,
    description: String,
    confidence: f32,
}

// Get Recommendations Tool (maps to get_context)
pub struct MemoryGetRecommendationsTool {
    ctx: CognitiveMemoryContext,
}

impl MemoryGetRecommendationsTool {
    pub fn new(ctx: CognitiveMemoryContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for MemoryGetRecommendationsTool {
    fn name(&self) -> &str {
        "cortex.memory.get_recommendations"
    }

    fn description(&self) -> Option<&str> {
        Some("Get recommendations based on context")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(GetRecommendationsInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: GetRecommendationsInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("cortex.memory.get_recommendations executed");

        // Extract description from context
        let description = input.context.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("Get recommendations")
            .to_string();

        // Call memory service to get context
        let context = self.ctx.memory_service.get_context(
            crate::services::memory::GetContextRequest {
                description,
            }
        ).await
        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get context: {}", e)))?;

        // Convert episodes and patterns to recommendations
        let mut recommendations = Vec::new();

        for episode in context.relevant_episodes.iter().take(input.limit as usize / 2) {
            recommendations.push(Recommendation {
                recommendation_type: "episode".to_string(),
                description: format!("Similar task: {} (outcome: {})", episode.task_description, episode.outcome),
                confidence: episode.importance as f32,
            });
        }

        for pattern in context.relevant_patterns.iter().take(input.limit as usize / 2) {
            recommendations.push(Recommendation {
                recommendation_type: "pattern".to_string(),
                description: format!("{}: {}", pattern.pattern_name, pattern.description),
                confidence: pattern.confidence as f32,
            });
        }

        let output = GetRecommendationsOutput {
            total_count: recommendations.len() as i32,
            recommendations,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct LearnFromFeedbackInput {
    pattern_id: String,
    #[serde(default = "default_positive")]
    feedback_type: String,
    context: Option<serde_json::Value>,
    #[serde(default = "default_adjustment")]
    adjustment_factor: f32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct LearnFromFeedbackOutput {
    pattern_id: String,
    new_confidence: f32,
}

// Learn from Feedback Tool (update patterns based on feedback)
pub struct MemoryLearnFromFeedbackTool {
    ctx: CognitiveMemoryContext,
}

impl MemoryLearnFromFeedbackTool {
    pub fn new(ctx: CognitiveMemoryContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for MemoryLearnFromFeedbackTool {
    fn name(&self) -> &str {
        "cortex.memory.learn_from_feedback"
    }

    fn description(&self) -> Option<&str> {
        Some("Update patterns based on feedback")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(LearnFromFeedbackInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: LearnFromFeedbackInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("cortex.memory.learn_from_feedback executed");

        let conn = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to acquire connection: {}", e)))?;

        // Get current pattern
        let query = "SELECT * FROM learned_pattern WHERE id = $pattern_id LIMIT 1";
        let mut result = conn.connection()
            .query(query)
            .bind(("pattern_id", input.pattern_id.clone()))
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to query pattern: {}", e)))?;

        let patterns: Vec<serde_json::Value> = result.take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse pattern results: {}", e)))?;

        let pattern = patterns.into_iter().next()
            .ok_or_else(|| ToolError::ExecutionFailed(format!("Pattern {} not found", input.pattern_id)))?;

        let current_confidence = pattern.get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        // Calculate new confidence based on feedback
        let adjustment = if input.feedback_type == "positive" {
            input.adjustment_factor as f64
        } else if input.feedback_type == "negative" {
            -(input.adjustment_factor as f64)
        } else {
            0.0
        };

        let new_confidence = (current_confidence + adjustment).max(0.0).min(1.0);

        // Update pattern
        let update_query = "UPDATE learned_pattern SET
            confidence = $confidence,
            last_seen = time::now()
            WHERE id = $pattern_id";

        conn.connection()
            .query(update_query)
            .bind(("pattern_id", input.pattern_id.clone()))
            .bind(("confidence", new_confidence))
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to update pattern: {}", e)))?;

        let output = LearnFromFeedbackOutput {
            pattern_id: input.pattern_id,
            new_confidence: new_confidence as f32,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

fn default_episode_limit() -> i32 { 10 }
fn default_similarity() -> f32 { 0.7 }
fn default_success() -> String { "success".to_string() }
fn default_true() -> bool { true }
fn default_min_frequency() -> i32 { 3 }
fn default_agent_group() -> String { "agent".to_string() }
fn default_threshold_days() -> i32 { 90 }
fn default_json_format() -> String { "json".to_string() }
fn default_smart_merge() -> String { "smart".to_string() }
fn default_recommendation_limit() -> i32 { 5 }
fn default_positive() -> String { "positive".to_string() }
fn default_adjustment() -> f32 { 0.1 }
