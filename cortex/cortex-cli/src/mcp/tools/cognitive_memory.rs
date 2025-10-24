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

        // Call memory service to store episode
        let episode = self.ctx.memory_service.store_episode(
            crate::services::memory::StoreEpisodeRequest {
                task_description: input.task_description,
                episode_type: "development".to_string(),
                outcome: input.outcome,
                importance: Some(0.7),
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
    changes: Vec<String>,
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

        let output = GetEpisodeOutput {
            episode_id: episode.id,
            task_description: episode.task_description,
            solution_summary: episode.outcome.clone(),
            outcome: episode.outcome,
            changes: vec![], // Changes tracking is not yet implemented
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

impl_memory_tool!(MemoryApplyPatternTool, "cortex.memory.apply_pattern", "Apply a learned pattern", ApplyPatternInput, ApplyPatternOutput);

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

impl_memory_tool!(MemorySearchEpisodesTool, "cortex.memory.search_episodes", "Search episodes by criteria", SearchEpisodesInput, SearchEpisodesOutput);

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

impl_memory_tool!(MemoryGetStatisticsTool, "cortex.memory.get_statistics", "Get memory system statistics", GetStatisticsInput, GetStatisticsOutput);

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

impl_memory_tool!(MemoryExportKnowledgeTool, "cortex.memory.export_knowledge", "Export knowledge base", ExportKnowledgeInput, ExportKnowledgeOutput);

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

impl_memory_tool!(MemoryImportKnowledgeTool, "cortex.memory.import_knowledge", "Import knowledge from another system", ImportKnowledgeInput, ImportKnowledgeOutput);

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

impl_memory_tool!(MemoryLearnFromFeedbackTool, "cortex.memory.learn_from_feedback", "Update patterns based on feedback", LearnFromFeedbackInput, LearnFromFeedbackOutput);

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
