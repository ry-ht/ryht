//! Cognitive Memory Tools (12 tools)

use async_trait::async_trait;
use cortex_memory::{EpisodicMemorySystem, ProceduralMemorySystem};
use cortex_storage::ConnectionManager;
use mcp_server::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

#[derive(Clone)]
pub struct CognitiveMemoryContext {
    #[allow(dead_code)]
    storage: Arc<ConnectionManager>,
    #[allow(dead_code)]
    episodic: Arc<EpisodicMemorySystem>,
    #[allow(dead_code)]
    procedural: Arc<ProceduralMemorySystem>,
}

impl CognitiveMemoryContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        let episodic = Arc::new(EpisodicMemorySystem::new(storage.clone()));
        let procedural = Arc::new(ProceduralMemorySystem::new(storage.clone()));
        Self { storage, episodic, procedural }
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

impl_memory_tool!(MemoryFindSimilarEpisodesTool, "cortex.memory.find_similar_episodes", "Find similar past development episodes", FindSimilarEpisodesInput, FindSimilarEpisodesOutput);

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

impl_memory_tool!(MemoryRecordEpisodeTool, "cortex.memory.record_episode", "Record a development episode", RecordEpisodeInput, RecordEpisodeOutput);

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

impl_memory_tool!(MemoryGetEpisodeTool, "cortex.memory.get_episode", "Retrieve episode details", GetEpisodeInput, GetEpisodeOutput);

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

impl_memory_tool!(MemoryExtractPatternsTool, "cortex.memory.extract_patterns", "Extract patterns from episodes", ExtractPatternsInput, ExtractPatternsOutput);

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

impl_memory_tool!(MemoryConsolidateTool, "cortex.memory.consolidate", "Consolidate and optimize memory", ConsolidateInput, ConsolidateOutput);

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

impl_memory_tool!(MemoryGetRecommendationsTool, "cortex.memory.get_recommendations", "Get recommendations based on context", GetRecommendationsInput, GetRecommendationsOutput);

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
