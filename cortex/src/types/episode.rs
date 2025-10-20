use super::{EpisodeId, Outcome, TokenCount};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Context snapshot at the start of a task
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextSnapshot {
    pub active_files: Vec<String>,
    pub active_symbols: Vec<String>,
    pub working_directory: Option<String>,
}

/// Default schema version for episodes
fn default_episode_schema_version() -> u32 {
    1
}

/// Task episode - a record of working on a specific task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEpisode {
    /// Schema version for migration support
    #[serde(default = "default_episode_schema_version")]
    pub schema_version: u32,
    pub id: EpisodeId,
    pub timestamp: DateTime<Utc>,
    pub task_description: String,
    pub initial_context: ContextSnapshot,
    pub queries_made: Vec<String>,
    pub files_touched: Vec<String>,
    pub solution_path: String,
    pub outcome: Outcome,
    pub tokens_used: TokenCount,
    #[serde(default)]
    pub access_count: u32,
    #[serde(default)]
    pub pattern_value: f32,
}

impl TaskEpisode {
    pub fn new(task_description: String) -> Self {
        Self {
            schema_version: 1,
            id: EpisodeId::new(),
            timestamp: Utc::now(),
            task_description,
            initial_context: ContextSnapshot::default(),
            queries_made: Vec::new(),
            files_touched: Vec::new(),
            solution_path: String::new(),
            outcome: Outcome::Partial,
            tokens_used: TokenCount::zero(),
            access_count: 0,
            pattern_value: 0.0,
        }
    }
}

/// Code pattern extracted from episodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodePattern {
    pub id: String,
    pub name: String,
    pub description: String,
    pub typical_actions: Vec<String>,
    pub frequency: u32,
    pub success_rate: f32,
    pub context_markers: Vec<String>,
}

/// Architecture knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureKnowledge {
    pub pattern_type: String,
    pub description: String,
    pub components: Vec<String>,
    pub relationships: Vec<String>,
}

/// Coding convention
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingConvention {
    pub category: String,
    pub rule: String,
    pub examples: Vec<String>,
}
