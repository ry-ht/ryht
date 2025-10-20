use anyhow::{Context as AnyhowContext, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

use crate::storage::{deserialize, serialize, Storage};
use crate::types::{Outcome, TaskEpisode};

use super::episodic::EpisodicMemory;
use super::semantic::SemanticMemory;

/// Memory compressor for MemGPT-style compression
pub struct MemoryCompressor {
    db: Arc<Surreal<Db>>,
    storage: Arc<dyn Storage>,
    episodic_memory: Arc<EpisodicMemory>,
    semantic_memory: Arc<SemanticMemory>,
}

/// Compression statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionStats {
    pub episodes_processed: usize,
    pub episodes_compressed: usize,
    pub semantic_memories_created: usize,
    pub space_saved_bytes: usize,
    pub compression_ratio: f32,
}

/// Summary of a conversation or episode sequence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub id: String,
    pub title: String,
    pub content: String,
    pub source_count: usize,
    pub created_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

/// Message for conversation summarization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// Checkpoint for memory state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: CheckpointId,
    pub created_at: DateTime<Utc>,
    pub episode_count: usize,
    pub semantic_count: usize,
    pub core_memory_snapshot: Vec<u8>,
    pub metadata: HashMap<String, String>,
}

/// Checkpoint identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CheckpointId(pub String);

impl CheckpointId {
    pub fn new() -> Self {
        Self(format!("checkpoint_{}", uuid::Uuid::new_v4()))
    }

    pub fn from_string(s: String) -> Self {
        Self(s)
    }
}

impl Default for CheckpointId {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryCompressor {
    /// Create a new memory compressor
    pub fn new(
        db: Arc<Surreal<Db>>,
        storage: Arc<dyn Storage>,
        episodic_memory: Arc<EpisodicMemory>,
        semantic_memory: Arc<SemanticMemory>,
    ) -> Self {
        Self {
            db,
            storage,
            episodic_memory,
            semantic_memory,
        }
    }

    /// Compress old episodic memories into semantic memories
    pub async fn compress_episodes(&self, older_than: Duration) -> Result<CompressionStats> {
        tracing::info!(
            days = older_than.num_days(),
            "Starting episode compression"
        );

        let cutoff = Utc::now() - older_than;
        let mut stats = CompressionStats {
            episodes_processed: 0,
            episodes_compressed: 0,
            semantic_memories_created: 0,
            space_saved_bytes: 0,
            compression_ratio: 0.0,
        };

        // Get old episodes
        let old_episodes: Vec<_> = self
            .episodic_memory
            .episodes()
            .iter()
            .filter(|e| e.timestamp < cutoff)
            .cloned()
            .collect();

        stats.episodes_processed = old_episodes.len();

        if old_episodes.is_empty() {
            tracing::info!("No episodes to compress");
            return Ok(stats);
        }

        // Calculate original size
        let original_size: usize = old_episodes
            .iter()
            .map(|e| self.estimate_episode_size(e))
            .sum();

        // Group similar episodes
        let groups = self.group_similar_episodes(&old_episodes)?;

        // Compress each group
        let mut compressed_size = 0usize;

        for group in groups {
            if group.len() > 1 {
                let summary = self.summarize_episode_group(&group)?;

                // Store summary as compressed semantic memory in SurrealDB
                self.store_summary(&summary).await?;

                compressed_size += summary.title.len() + summary.content.len();
                stats.episodes_compressed += group.len();
                stats.semantic_memories_created += 1;

                tracing::info!(
                    "Compressed {} episodes into semantic memory: {}",
                    group.len(),
                    summary.title
                );
            }
        }

        // Calculate compression stats
        stats.space_saved_bytes = original_size.saturating_sub(compressed_size);
        stats.compression_ratio = if original_size > 0 {
            compressed_size as f32 / original_size as f32
        } else {
            0.0
        };

        tracing::info!(
            "Compression complete: {} episodes -> {} semantic memories (ratio: {:.2}x)",
            stats.episodes_compressed,
            stats.semantic_memories_created,
            1.0 / stats.compression_ratio.max(0.001)
        );

        Ok(stats)
    }

    /// Summarize conversation history
    pub async fn summarize_conversation(&self, messages: Vec<Message>) -> Result<Summary> {
        tracing::debug!("Summarizing {} messages", messages.len());

        if messages.is_empty() {
            return Ok(Summary {
                id: uuid::Uuid::new_v4().to_string(),
                title: "Empty conversation".to_string(),
                content: "No messages to summarize".to_string(),
                source_count: 0,
                created_at: Utc::now(),
                metadata: HashMap::new(),
            });
        }

        // Extract key information
        let user_messages: Vec<_> = messages
            .iter()
            .filter(|m| m.role == "user")
            .collect();

        let assistant_messages: Vec<_> = messages
            .iter()
            .filter(|m| m.role == "assistant")
            .collect();

        // Create summary
        let title = if let Some(first_msg) = user_messages.first() {
            let truncated = first_msg
                .content
                .chars()
                .take(50)
                .collect::<String>();
            format!("Conversation: {}", truncated)
        } else {
            "Conversation summary".to_string()
        };

        let content = format!(
            "Conversation with {} user messages and {} assistant responses.\n\n\
             Key topics:\n{}\n\n\
             Main outcomes:\n{}",
            user_messages.len(),
            assistant_messages.len(),
            self.extract_topics(&messages),
            self.extract_outcomes(&messages)
        );

        Ok(Summary {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            content,
            source_count: messages.len(),
            created_at: Utc::now(),
            metadata: HashMap::new(),
        })
    }

    /// Create memory checkpoint
    pub async fn create_checkpoint(&self) -> Result<CheckpointId> {
        let checkpoint_id = CheckpointId::new();

        tracing::info!(checkpoint_id = %checkpoint_id.0, "Creating memory checkpoint");

        // Get current state
        let episode_count = self.episodic_memory.episodes().len();
        let semantic_count = self.semantic_memory.knowledge_count();

        // Get core memory snapshot
        let core_memory_data = self
            .storage
            .get(b"core_memory")
            .await?
            .unwrap_or_default();

        let checkpoint = Checkpoint {
            id: checkpoint_id.clone(),
            created_at: Utc::now(),
            episode_count,
            semantic_count,
            core_memory_snapshot: core_memory_data,
            metadata: HashMap::new(),
        };

        // Store checkpoint
        let key = format!("checkpoint:{}", checkpoint_id.0);
        let value = serialize(&checkpoint)?;
        self.storage.put(key.as_bytes(), &value).await?;

        tracing::info!(
            "Checkpoint created: {} episodes, {} semantic memories",
            episode_count,
            semantic_count
        );

        Ok(checkpoint_id)
    }

    /// Restore from checkpoint
    pub async fn restore_checkpoint(&self, id: CheckpointId) -> Result<()> {
        tracing::info!(checkpoint_id = %id.0, "Restoring memory checkpoint");

        // Load checkpoint
        let key = format!("checkpoint:{}", id.0);
        let data = self
            .storage
            .get(key.as_bytes())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Checkpoint not found: {}", id.0))?;

        let checkpoint: Checkpoint = deserialize(&data)?;

        // Restore core memory
        self.storage
            .put(b"core_memory", &checkpoint.core_memory_snapshot)
            .await?;

        tracing::info!(
            "Checkpoint restored: {} episodes, {} semantic memories",
            checkpoint.episode_count,
            checkpoint.semantic_count
        );

        Ok(())
    }

    /// List all checkpoints
    pub async fn list_checkpoints(&self) -> Result<Vec<Checkpoint>> {
        let keys = self.storage.get_keys_with_prefix(b"checkpoint:").await?;

        let mut checkpoints = Vec::new();
        for key in keys {
            if let Some(data) = self.storage.get(&key).await? {
                if let Ok(checkpoint) = deserialize::<Checkpoint>(&data) {
                    checkpoints.push(checkpoint);
                }
            }
        }

        // Sort by creation time (newest first)
        checkpoints.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(checkpoints)
    }

    /// Delete a checkpoint
    pub async fn delete_checkpoint(&self, id: CheckpointId) -> Result<()> {
        let key = format!("checkpoint:{}", id.0);
        self.storage.delete(key.as_bytes()).await?;
        tracing::info!(checkpoint_id = %id.0, "Checkpoint deleted");
        Ok(())
    }

    /// Group similar episodes for compression
    fn group_similar_episodes(&self, episodes: &[TaskEpisode]) -> Result<Vec<Vec<TaskEpisode>>> {
        let mut groups: Vec<Vec<TaskEpisode>> = Vec::new();
        let mut processed: std::collections::HashSet<String> = std::collections::HashSet::new();

        for episode in episodes {
            if processed.contains(&episode.id.0) {
                continue;
            }

            let mut group = vec![episode.clone()];
            processed.insert(episode.id.0.clone());

            // Find similar episodes in the same set
            for other in episodes {
                if processed.contains(&other.id.0) {
                    continue;
                }

                if self.calculate_similarity(&episode.task_description, &other.task_description) > 0.4 {
                    group.push(other.clone());
                    processed.insert(other.id.0.clone());
                }
            }

            if !group.is_empty() {
                groups.push(group);
            }
        }

        Ok(groups)
    }

    /// Calculate text similarity using Jaccard index
    fn calculate_similarity(&self, text1: &str, text2: &str) -> f32 {
        let words1: std::collections::HashSet<_> =
            text1.split_whitespace().map(|s| s.to_lowercase()).collect();
        let words2: std::collections::HashSet<_> =
            text2.split_whitespace().map(|s| s.to_lowercase()).collect();

        if words1.is_empty() || words2.is_empty() {
            return 0.0;
        }

        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();

        intersection as f32 / union as f32
    }

    /// Summarize a group of episodes
    fn summarize_episode_group(&self, episodes: &[TaskEpisode]) -> Result<Summary> {
        // Extract common files and queries
        let common_files: std::collections::HashSet<_> = episodes
            .iter()
            .flat_map(|e| e.files_touched.clone())
            .collect();

        let common_queries: std::collections::HashSet<_> = episodes
            .iter()
            .flat_map(|e| e.queries_made.clone())
            .collect();

        // Calculate success rate
        let success_count = episodes
            .iter()
            .filter(|e| e.outcome == Outcome::Success)
            .count();
        let success_rate = success_count as f32 / episodes.len() as f32;

        // Create title
        let title = if let Some(first) = episodes.first() {
            let task_preview = first
                .task_description
                .chars()
                .take(40)
                .collect::<String>();
            format!("Pattern: {} ({} episodes)", task_preview, episodes.len())
        } else {
            "Episode group".to_string()
        };

        // Create content
        let content = format!(
            "Summary of {} related tasks:\n\n\
             Success rate: {:.1}%\n\
             Common files ({}):\n{}\n\n\
             Common queries ({}):\n{}\n\n\
             Solutions:\n{}",
            episodes.len(),
            success_rate * 100.0,
            common_files.len(),
            common_files
                .iter()
                .take(5)
                .map(|f| format!("  - {}", f))
                .collect::<Vec<_>>()
                .join("\n"),
            common_queries.len(),
            common_queries
                .iter()
                .take(5)
                .map(|q| format!("  - {}", q))
                .collect::<Vec<_>>()
                .join("\n"),
            episodes
                .iter()
                .filter(|e| e.outcome == Outcome::Success && !e.solution_path.is_empty())
                .take(3)
                .map(|e| format!("  - {}", e.solution_path))
                .collect::<Vec<_>>()
                .join("\n")
        );

        Ok(Summary {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            content,
            source_count: episodes.len(),
            created_at: Utc::now(),
            metadata: HashMap::new(),
        })
    }

    /// Estimate size of an episode in bytes
    fn estimate_episode_size(&self, episode: &TaskEpisode) -> usize {
        episode.task_description.len()
            + episode.solution_path.len()
            + episode
                .files_touched
                .iter()
                .map(|f| f.len())
                .sum::<usize>()
            + episode.queries_made.iter().map(|q| q.len()).sum::<usize>()
    }

    /// Extract topics from messages
    fn extract_topics(&self, messages: &[Message]) -> String {
        // Simple keyword extraction
        let mut word_freq: HashMap<String, usize> = HashMap::new();

        for msg in messages {
            for word in msg.content.split_whitespace() {
                let word_lower = word.to_lowercase();
                if word_lower.len() > 4 {
                    *word_freq.entry(word_lower).or_insert(0) += 1;
                }
            }
        }

        // Get top keywords
        let mut sorted: Vec<_> = word_freq.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        sorted
            .into_iter()
            .take(5)
            .map(|(word, _)| format!("  - {}", word))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Extract outcomes from messages
    fn extract_outcomes(&self, messages: &[Message]) -> String {
        // Look for outcome indicators
        let outcome_keywords = ["completed", "fixed", "implemented", "resolved", "done"];

        let outcomes: Vec<_> = messages
            .iter()
            .filter(|m| {
                outcome_keywords
                    .iter()
                    .any(|kw| m.content.to_lowercase().contains(kw))
            })
            .take(3)
            .map(|m| {
                let preview = m.content.chars().take(60).collect::<String>();
                format!("  - {}", preview)
            })
            .collect();

        if outcomes.is_empty() {
            "  - No specific outcomes identified".to_string()
        } else {
            outcomes.join("\n")
        }
    }

    /// Store a summary as semantic memory in SurrealDB
    async fn store_summary(&self, summary: &Summary) -> Result<()> {
        #[derive(Serialize, Deserialize)]
        struct SummaryRecord {
            id: String,
            title: String,
            content: String,
            source_count: usize,
            created_at: DateTime<Utc>,
            metadata: HashMap<String, String>,
            summary_type: String,
        }

        let record = SummaryRecord {
            id: summary.id.clone(),
            title: summary.title.clone(),
            content: summary.content.clone(),
            source_count: summary.source_count,
            created_at: summary.created_at,
            metadata: summary.metadata.clone(),
            summary_type: "episode_compression".to_string(),
        };

        // Store in SurrealDB
        let _: Option<SummaryRecord> = self
            .db
            .create(("semantic_memory", &summary.id))
            .content(record)
            .await
            .with_context(|| "Failed to store summary in SurrealDB")?;

        // Also store in key-value storage for compatibility
        let key = format!("summary:{}", summary.id);
        let value = serialize(summary)?;
        self.storage.put(key.as_bytes(), &value).await?;

        tracing::debug!(
            summary_id = %summary.id,
            title = %summary.title,
            "Stored summary in semantic memory"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStorage;
    use crate::types::{ContextSnapshot, EpisodeId, TokenCount};
    use tempfile::TempDir;

    async fn create_test_setup() -> (Arc<Surreal<Db>>, Arc<dyn Storage>, TempDir, TempDir) {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();

        let db = Surreal::new::<surrealdb::engine::local::RocksDb>(temp_dir1.path())
            .await
            .unwrap();
        db.use_ns("test").use_db("test").await.unwrap();

        let storage: Arc<dyn Storage> = Arc::new(MemoryStorage::new());

        (Arc::new(db), storage, temp_dir1, temp_dir2)
    }

    #[tokio::test]
    async fn test_create_and_restore_checkpoint() {
        let (db, storage, _temp1, _temp2) = create_test_setup().await;

        let episodic = Arc::new(EpisodicMemory::new(storage.clone(), 30).unwrap());
        let semantic = Arc::new(SemanticMemory::new(storage.clone()).unwrap());
        let compressor = MemoryCompressor::new(db, storage, episodic, semantic);

        // Create checkpoint
        let checkpoint_id = compressor.create_checkpoint().await.unwrap();

        // Restore checkpoint
        compressor.restore_checkpoint(checkpoint_id.clone()).await.unwrap();

        // List checkpoints
        let checkpoints = compressor.list_checkpoints().await.unwrap();
        assert_eq!(checkpoints.len(), 1);
        assert_eq!(checkpoints[0].id, checkpoint_id);
    }

    #[tokio::test]
    async fn test_summarize_conversation() {
        let (db, storage, _temp1, _temp2) = create_test_setup().await;

        let episodic = Arc::new(EpisodicMemory::new(storage.clone(), 30).unwrap());
        let semantic = Arc::new(SemanticMemory::new(storage.clone()).unwrap());
        let compressor = MemoryCompressor::new(db, storage, episodic, semantic);

        let messages = vec![
            Message {
                role: "user".to_string(),
                content: "Help me implement authentication".to_string(),
                timestamp: Utc::now(),
            },
            Message {
                role: "assistant".to_string(),
                content: "I'll help you implement authentication using JWT".to_string(),
                timestamp: Utc::now(),
            },
        ];

        let summary = compressor.summarize_conversation(messages).await.unwrap();
        assert_eq!(summary.source_count, 2);
        assert!(summary.content.contains("authentication"));
    }

    #[test]
    fn test_similarity_calculation() {
        let (db, storage, _temp1, _temp2) = tokio_test::block_on(create_test_setup());

        let episodic = Arc::new(EpisodicMemory::new(storage.clone(), 30).unwrap());
        let semantic = Arc::new(SemanticMemory::new(storage.clone()).unwrap());
        let compressor = MemoryCompressor::new(db, storage, episodic, semantic);

        let sim1 = compressor.calculate_similarity(
            "implement user authentication",
            "add user authentication system",
        );
        let sim2 = compressor.calculate_similarity(
            "implement user authentication",
            "fix database connection",
        );

        assert!(sim1 > sim2);
        assert!(sim1 > 0.2, "Similarity should be > 0.2, got {}", sim1);
    }
}
