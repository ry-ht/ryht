use anyhow::{Context as AnyhowContext, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

use crate::types::{ContextSnapshot, EpisodeId, Outcome, TaskEpisode, TokenCount};

/// Episode recorder for tracking task execution
pub struct EpisodeRecorder {
    db: Arc<Surreal<Db>>,
}

/// Handle to an active episode recording
#[derive(Debug, Clone)]
pub struct EpisodeHandle {
    pub episode_id: EpisodeId,
    started_at: DateTime<Utc>,
}

/// Action taken during an episode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub action_type: ActionType,
    pub description: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ActionType {
    CodeSearch,
    FileRead,
    FileEdit,
    ToolCall,
    Query,
    Analysis,
    Test,
    Build,
    Commit,
}

impl std::fmt::Display for ActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionType::CodeSearch => write!(f, "code_search"),
            ActionType::FileRead => write!(f, "file_read"),
            ActionType::FileEdit => write!(f, "file_edit"),
            ActionType::ToolCall => write!(f, "tool_call"),
            ActionType::Query => write!(f, "query"),
            ActionType::Analysis => write!(f, "analysis"),
            ActionType::Test => write!(f, "test"),
            ActionType::Build => write!(f, "build"),
            ActionType::Commit => write!(f, "commit"),
        }
    }
}

/// Context in which the episode occurred
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct Context {
    pub task_description: String,
    pub working_directory: Option<String>,
    pub active_files: Vec<String>,
    pub active_symbols: Vec<String>,
    pub environment: HashMap<String, String>,
    pub tags: Vec<String>,
}


/// Outcome of an episode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeOutcome {
    pub status: Outcome,
    pub description: String,
    pub files_modified: Vec<String>,
    pub tests_passed: Option<bool>,
    pub build_succeeded: Option<bool>,
    pub commit_hash: Option<String>,
}

/// Complete episode record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: EpisodeId,
    pub timestamp: DateTime<Utc>,
    pub task: String,
    pub context: Context,
    pub actions: Vec<Action>,
    pub outcome: EpisodeOutcome,
    pub learnings: Vec<String>,
    pub embedding: Option<Vec<f32>>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub duration_seconds: i64,
}

/// Pattern extracted from episodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: String,
    pub name: String,
    pub description: String,
    pub action_sequence: Vec<ActionType>,
    pub frequency: usize,
    pub success_rate: f32,
    pub context_markers: Vec<String>,
}

/// Statistics for pattern extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternStats {
    pub total_patterns: usize,
    pub high_confidence_patterns: usize,
    pub average_success_rate: f32,
}

impl EpisodeRecorder {
    /// Create a new episode recorder
    pub fn new(db: Arc<Surreal<Db>>) -> Self {
        Self { db }
    }

    /// Start recording a new episode
    pub async fn start_episode(&self, task: &str, context: Context) -> Result<EpisodeHandle> {
        let episode_id = EpisodeId::new();
        let started_at = Utc::now();

        tracing::info!(
            episode_id = %episode_id.0,
            task = %task,
            "Starting episode recording"
        );

        // Create episode record in database
        #[derive(Serialize, Deserialize)]
        struct EpisodeInit {
            id: String,
            task_description: String,
            started_at: DateTime<Utc>,
            status: String,
        }

        let init = EpisodeInit {
            id: episode_id.0.clone(),
            task_description: task.to_string(),
            started_at,
            status: "in_progress".to_string(),
        };

        // Use DELETE+CREATE to avoid Thing deserialization issues
        let query = format!(
            "DELETE episode:`{}`; CREATE episode:`{}` CONTENT $init",
            episode_id.0, episode_id.0
        );
        let _ = self.db
            .query(query)
            .bind(("init", init))
            .await
            .with_context(|| "Failed to create episode record")?;

        Ok(EpisodeHandle {
            episode_id,
            started_at,
        })
    }

    /// Record an action in the current episode
    pub async fn record_action(&self, handle: &EpisodeHandle, action: Action) -> Result<()> {
        tracing::debug!(
            episode_id = %handle.episode_id.0,
            action_type = %action.action_type,
            "Recording action"
        );

        // Create action record in database
        #[derive(Serialize)]
        struct ActionRecord {
            episode_id: String,
            action_type: String,
            description: String,
            timestamp: DateTime<Utc>,
            metadata: HashMap<String, serde_json::Value>,
        }

        let record = ActionRecord {
            episode_id: handle.episode_id.0.clone(),
            action_type: action.action_type.to_string(),
            description: action.description,
            timestamp: action.timestamp,
            metadata: action.metadata,
        };

        self.db
            .query("INSERT INTO episode_action $record")
            .bind(("record", record))
            .await
            .with_context(|| "Failed to record action")?;

        Ok(())
    }

    /// Complete the episode with outcome
    pub async fn complete_episode(
        &self,
        handle: EpisodeHandle,
        outcome: EpisodeOutcome,
        learnings: Vec<String>,
    ) -> Result<Episode> {
        let completed_at = Utc::now();
        let duration = (completed_at - handle.started_at).num_seconds();

        tracing::info!(
            episode_id = %handle.episode_id.0,
            status = ?outcome.status,
            duration_seconds = duration,
            "Completing episode"
        );

        // Get all actions for this episode
        let actions = self.get_episode_actions(&handle).await?;

        // Get task description
        let task = self.get_episode_task(&handle).await?;

        // Get context
        let context = self.get_episode_context(&handle).await?;

        // Create complete episode record
        let episode = Episode {
            id: handle.episode_id.clone(),
            timestamp: handle.started_at,
            task,
            context,
            actions,
            outcome,
            learnings,
            embedding: None, // Will be generated later
            metadata: HashMap::new(),
            duration_seconds: duration,
        };

        // Update episode status in database
        let episode_id_str = handle.episode_id.0.clone();
        self.db
            .query("UPDATE $episode_id SET status = 'completed', completed_at = $completed_at, duration_seconds = $duration")
            .bind(("episode_id", ("episode", episode_id_str)))
            .bind(("completed_at", completed_at))
            .bind(("duration", duration))
            .await
            .with_context(|| "Failed to update episode status")?;

        Ok(episode)
    }

    /// Get all actions for an episode
    async fn get_episode_actions(&self, handle: &EpisodeHandle) -> Result<Vec<Action>> {
        #[derive(Deserialize)]
        struct ActionRecord {
            action_type: String,
            description: String,
            timestamp: DateTime<Utc>,
            metadata: HashMap<String, serde_json::Value>,
        }

        let mut response = self
            .db
            .query("SELECT * FROM episode_action WHERE episode_id = $episode_id ORDER BY timestamp")
            .bind(("episode_id", handle.episode_id.0.clone()))
            .await
            .with_context(|| "Failed to query episode actions")?;

        let records: Vec<ActionRecord> = response.take(0)?;

        let actions = records
            .into_iter()
            .filter_map(|r| {
                let action_type = match r.action_type.as_str() {
                    "code_search" => ActionType::CodeSearch,
                    "file_read" => ActionType::FileRead,
                    "file_edit" => ActionType::FileEdit,
                    "tool_call" => ActionType::ToolCall,
                    "query" => ActionType::Query,
                    "analysis" => ActionType::Analysis,
                    "test" => ActionType::Test,
                    "build" => ActionType::Build,
                    "commit" => ActionType::Commit,
                    _ => return None,
                };

                Some(Action {
                    action_type,
                    description: r.description,
                    timestamp: r.timestamp,
                    metadata: r.metadata,
                })
            })
            .collect();

        Ok(actions)
    }

    /// Get task description for an episode
    async fn get_episode_task(&self, handle: &EpisodeHandle) -> Result<String> {
        #[derive(Deserialize)]
        struct EpisodeTask {
            task_description: String,
        }

        let task: Option<EpisodeTask> = self
            .db
            .select(("episode", &handle.episode_id.0))
            .await
            .with_context(|| "Failed to get episode task")?;

        task.map(|t| t.task_description)
            .ok_or_else(|| anyhow::anyhow!("Episode not found"))
    }

    /// Get context for an episode
    async fn get_episode_context(&self, _handle: &EpisodeHandle) -> Result<Context> {
        // For now, return default context
        // In a real implementation, this would be stored with the episode
        Ok(Context::default())
    }

    /// Extract patterns from multiple episodes
    pub async fn extract_patterns(&self, episodes: &[Episode]) -> Result<Vec<Pattern>> {
        let mut patterns = Vec::new();
        let mut action_sequences: HashMap<Vec<ActionType>, Vec<(usize, Outcome)>> = HashMap::new();

        // Collect action sequences
        for (idx, episode) in episodes.iter().enumerate() {
            let sequence: Vec<ActionType> = episode.actions.iter().map(|a| a.action_type.clone()).collect();

            if !sequence.is_empty() {
                action_sequences
                    .entry(sequence)
                    .or_default()
                    .push((idx, episode.outcome.status));
            }
        }

        // Create patterns from frequent sequences
        for (sequence, occurrences) in action_sequences {
            if occurrences.len() >= 2 {
                // Only patterns that occurred at least twice
                let success_count = occurrences
                    .iter()
                    .filter(|(_, outcome)| *outcome == Outcome::Success)
                    .count();

                let success_rate = success_count as f32 / occurrences.len() as f32;

                // Extract context markers from first episode with this pattern
                let context_markers = if let Some((idx, _)) = occurrences.first() {
                    extract_keywords(&episodes[*idx].task)
                } else {
                    Vec::new()
                };

                patterns.push(Pattern {
                    id: format!("pattern_{}", uuid::Uuid::new_v4()),
                    name: format!("{:?} sequence", sequence.first().unwrap_or(&ActionType::Query)),
                    description: format!(
                        "Common pattern: {} -> ... ({} steps)",
                        sequence.first().map(|a| format!("{:?}", a)).unwrap_or_default(),
                        sequence.len()
                    ),
                    action_sequence: sequence,
                    frequency: occurrences.len(),
                    success_rate,
                    context_markers,
                });
            }
        }

        // Sort by frequency and success rate
        patterns.sort_by(|a, b| {
            (b.frequency as f32 * b.success_rate)
                .partial_cmp(&(a.frequency as f32 * a.success_rate))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(patterns)
    }

    /// Get pattern statistics
    pub async fn get_pattern_stats(&self, patterns: &[Pattern]) -> Result<PatternStats> {
        let total_patterns = patterns.len();
        let high_confidence_patterns = patterns.iter().filter(|p| p.success_rate >= 0.8).count();

        let average_success_rate = if total_patterns > 0 {
            patterns.iter().map(|p| p.success_rate).sum::<f32>() / total_patterns as f32
        } else {
            0.0
        };

        Ok(PatternStats {
            total_patterns,
            high_confidence_patterns,
            average_success_rate,
        })
    }

    /// Convert Episode to TaskEpisode for compatibility
    pub fn to_task_episode(&self, episode: Episode) -> TaskEpisode {
        let files_touched: Vec<String> = episode
            .actions
            .iter()
            .filter(|a| a.action_type == ActionType::FileEdit)
            .filter_map(|a| a.metadata.get("file_path"))
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect();

        let queries_made: Vec<String> = episode
            .actions
            .iter()
            .filter(|a| matches!(a.action_type, ActionType::Query | ActionType::CodeSearch))
            .map(|a| a.description.clone())
            .collect();

        TaskEpisode {
            schema_version: 1,
            id: episode.id,
            timestamp: episode.timestamp,
            task_description: episode.task,
            initial_context: ContextSnapshot {
                active_files: episode.context.active_files,
                active_symbols: episode.context.active_symbols,
                working_directory: episode.context.working_directory,
            },
            queries_made,
            files_touched,
            solution_path: episode.outcome.description,
            outcome: episode.outcome.status,
            tokens_used: TokenCount::zero(), // Would need to track this separately
            access_count: 0,
            pattern_value: episode.outcome.status.into_pattern_value(),
        }
    }
}

/// Extract keywords from text
fn extract_keywords(text: &str) -> Vec<String> {
    let stop_words: std::collections::HashSet<&str> = [
        "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "is",
        "are", "was", "were", "be", "been", "being",
    ]
    .iter()
    .copied()
    .collect();

    text.split_whitespace()
        .filter(|w| w.len() > 2 && !stop_words.contains(&w.to_lowercase().as_str()))
        .map(|w| w.to_lowercase())
        .collect()
}

/// Extension trait for Outcome to convert to pattern value
trait OutcomeExt {
    fn into_pattern_value(self) -> f32;
}

impl OutcomeExt for Outcome {
    fn into_pattern_value(self) -> f32 {
        match self {
            Outcome::Success => 1.0,
            Outcome::Partial => 0.5,
            Outcome::Failure => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_db() -> (Arc<Surreal<Db>>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db = Surreal::new::<surrealdb::engine::local::RocksDb>(temp_dir.path())
            .await
            .unwrap();

        db.use_ns("test").use_db("test").await.unwrap();

        // Initialize schema
        db.query(
            r#"
            DEFINE TABLE IF NOT EXISTS episode SCHEMAFULL;
            DEFINE FIELD IF NOT EXISTS id ON TABLE episode TYPE string;
            DEFINE FIELD IF NOT EXISTS task_description ON TABLE episode TYPE string;
            DEFINE FIELD IF NOT EXISTS started_at ON TABLE episode TYPE datetime;
            DEFINE FIELD IF NOT EXISTS status ON TABLE episode TYPE string;

            DEFINE TABLE IF NOT EXISTS episode_action SCHEMAFULL;
            DEFINE FIELD IF NOT EXISTS episode_id ON TABLE episode_action TYPE string;
            DEFINE FIELD IF NOT EXISTS action_type ON TABLE episode_action TYPE string;
            DEFINE FIELD IF NOT EXISTS description ON TABLE episode_action TYPE string;
            DEFINE FIELD IF NOT EXISTS timestamp ON TABLE episode_action TYPE datetime;
        "#,
        )
        .await
        .unwrap();

        (Arc::new(db), temp_dir)
    }

    #[tokio::test]
    async fn test_episode_recording() {
        let (db, _temp) = create_test_db().await;
        let recorder = EpisodeRecorder::new(db);

        let context = Context::default();
        let handle = recorder.start_episode("Test task", context).await.unwrap();

        assert!(!handle.episode_id.0.is_empty());

        let action = Action {
            action_type: ActionType::CodeSearch,
            description: "Searching for auth module".to_string(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };

        recorder.record_action(&handle, action).await.unwrap();

        let outcome = EpisodeOutcome {
            status: Outcome::Success,
            description: "Task completed successfully".to_string(),
            files_modified: vec!["auth.rs".to_string()],
            tests_passed: Some(true),
            build_succeeded: Some(true),
            commit_hash: Some("abc123".to_string()),
        };

        let episode = recorder
            .complete_episode(handle, outcome, vec!["Learned to use auth module".to_string()])
            .await
            .unwrap();

        assert_eq!(episode.task, "Test task");
        assert_eq!(episode.actions.len(), 1);
        assert_eq!(episode.learnings.len(), 1);
    }

    #[tokio::test]
    async fn test_pattern_extraction() {
        let (db, _temp) = create_test_db().await;
        let recorder = EpisodeRecorder::new(db);

        let mut episodes = Vec::new();

        for i in 0..3 {
            let episode = Episode {
                id: EpisodeId::new(),
                timestamp: Utc::now(),
                task: format!("Task {}", i),
                context: Context::default(),
                actions: vec![
                    Action {
                        action_type: ActionType::CodeSearch,
                        description: "Search".to_string(),
                        timestamp: Utc::now(),
                        metadata: HashMap::new(),
                    },
                    Action {
                        action_type: ActionType::FileEdit,
                        description: "Edit".to_string(),
                        timestamp: Utc::now(),
                        metadata: HashMap::new(),
                    },
                ],
                outcome: EpisodeOutcome {
                    status: Outcome::Success,
                    description: "Success".to_string(),
                    files_modified: Vec::new(),
                    tests_passed: Some(true),
                    build_succeeded: Some(true),
                    commit_hash: None,
                },
                learnings: Vec::new(),
                embedding: None,
                metadata: HashMap::new(),
                duration_seconds: 60,
            };

            episodes.push(episode);
        }

        let patterns = recorder.extract_patterns(&episodes).await.unwrap();
        assert!(!patterns.is_empty());
        assert_eq!(patterns[0].frequency, 3);
        assert_eq!(patterns[0].success_rate, 1.0);
    }
}
