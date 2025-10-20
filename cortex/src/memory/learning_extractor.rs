use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

use crate::storage::{deserialize, serialize, Storage};

use super::episode_recorder::{Context, Episode};

/// Learning extractor - extracts patterns and learnings from episodes
pub struct LearningExtractor {
    db: Arc<Surreal<Db>>,
    storage: Arc<dyn Storage>,
}

/// A learning extracted from episodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Learning {
    pub id: String,
    pub pattern: String,
    pub confidence: f32,
    pub episodes: Vec<String>, // Episode IDs that support this learning
    pub applications: usize,   // Times applied successfully
    pub created_at: DateTime<Utc>,
    pub last_applied: Option<DateTime<Utc>>,
    pub category: LearningCategory,
    pub metadata: HashMap<String, String>,
}

/// Category of learning
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LearningCategory {
    /// Pattern for solving a specific type of problem
    SolutionPattern,

    /// Common workflow or sequence of actions
    Workflow,

    /// Code architecture or design pattern
    Architecture,

    /// Best practice or convention
    BestPractice,

    /// Anti-pattern to avoid
    AntiPattern,

    /// Performance optimization technique
    Optimization,
}

/// Suggestion for applying a learning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub learning_id: String,
    pub description: String,
    pub confidence: f32,
    pub reasoning: String,
    pub example_episodes: Vec<String>,
}

/// Learning statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningStats {
    pub total_learnings: usize,
    pub high_confidence_learnings: usize,
    pub frequently_applied_learnings: usize,
    pub average_confidence: f32,
    pub average_applications: f32,
}

impl LearningExtractor {
    /// Create a new learning extractor
    pub fn new(db: Arc<Surreal<Db>>, storage: Arc<dyn Storage>) -> Self {
        Self { db, storage }
    }

    /// Extract learnings from multiple episodes
    pub async fn extract_from_episodes(&self, episodes: Vec<Episode>) -> Result<Vec<Learning>> {
        tracing::info!("Extracting learnings from {} episodes", episodes.len());

        let mut learnings = Vec::new();

        // Group episodes by similarity
        let groups = self.group_similar_episodes(&episodes).await?;

        for group in groups {
            if group.len() >= 2 {
                // Only extract patterns that occurred multiple times
                if let Some(learning) = self.extract_from_group(&group).await? {
                    learnings.push(learning);
                }
            }
        }

        // Extract workflow patterns
        let workflow_learnings = self.extract_workflows(&episodes).await?;
        learnings.extend(workflow_learnings);

        // Extract anti-patterns from failed episodes
        let anti_patterns = self.extract_anti_patterns(&episodes).await?;
        learnings.extend(anti_patterns);

        // Store learnings
        for learning in &learnings {
            self.store_learning(learning).await?;
        }

        tracing::info!("Extracted {} learnings", learnings.len());
        Ok(learnings)
    }

    /// Extract a learning from a group of similar episodes
    async fn extract_from_group(&self, episodes: &[Episode]) -> Result<Option<Learning>> {
        if episodes.is_empty() {
            return Ok(None);
        }

        // Calculate success rate
        let success_count = episodes
            .iter()
            .filter(|e| matches!(e.outcome.status, crate::types::Outcome::Success))
            .count();

        let confidence = success_count as f32 / episodes.len() as f32;

        // Only create learning if success rate is high
        if confidence < 0.7 {
            return Ok(None);
        }

        // Extract common pattern
        let pattern = self.identify_common_pattern(episodes)?;

        let learning = Learning {
            id: format!("learning_{}", uuid::Uuid::new_v4()),
            pattern: pattern.clone(),
            confidence,
            episodes: episodes.iter().map(|e| e.id.0.clone()).collect(),
            applications: episodes.len(),
            created_at: Utc::now(),
            last_applied: None,
            category: LearningCategory::SolutionPattern,
            metadata: HashMap::new(),
        };

        Ok(Some(learning))
    }

    /// Identify common pattern across episodes
    fn identify_common_pattern(&self, episodes: &[Episode]) -> Result<String> {
        // Find common actions
        let mut action_frequency: HashMap<String, usize> = HashMap::new();

        for episode in episodes {
            for action in &episode.actions {
                *action_frequency
                    .entry(action.description.clone())
                    .or_insert(0) += 1;
            }
        }

        // Find actions that appear in most episodes
        let threshold = (episodes.len() as f32 * 0.7) as usize;
        let common_actions: Vec<_> = action_frequency
            .iter()
            .filter(|(_, &count)| count >= threshold)
            .map(|(action, _)| action.clone())
            .collect();

        if common_actions.is_empty() {
            return Ok(format!(
                "Pattern for: {}",
                episodes.first().map(|e| e.task.as_str()).unwrap_or("unknown")
            ));
        }

        Ok(format!(
            "Common steps: {} (from {} episodes)",
            common_actions.join(" → "),
            episodes.len()
        ))
    }

    /// Extract workflow patterns (common action sequences)
    async fn extract_workflows(&self, episodes: &[Episode]) -> Result<Vec<Learning>> {
        let mut workflows = Vec::new();
        let mut sequence_frequency: HashMap<String, Vec<String>> = HashMap::new();

        for episode in episodes {
            if episode.actions.len() < 3 {
                continue;
            }

            // Extract action sequence
            let sequence: Vec<_> = episode
                .actions
                .iter()
                .map(|a| format!("{:?}", a.action_type))
                .collect();

            let sequence_key = sequence.join("|");
            sequence_frequency
                .entry(sequence_key)
                .or_default()
                .push(episode.id.0.clone());
        }

        // Create learnings for frequent sequences
        for (sequence, episode_ids) in sequence_frequency {
            if episode_ids.len() >= 3 {
                workflows.push(Learning {
                    id: format!("workflow_{}", uuid::Uuid::new_v4()),
                    pattern: format!("Workflow: {}", sequence.replace('|', " → ")),
                    confidence: 0.8,
                    episodes: episode_ids.clone(),
                    applications: episode_ids.len(),
                    created_at: Utc::now(),
                    last_applied: None,
                    category: LearningCategory::Workflow,
                    metadata: HashMap::new(),
                });
            }
        }

        Ok(workflows)
    }

    /// Extract anti-patterns from failed episodes
    async fn extract_anti_patterns(&self, episodes: &[Episode]) -> Result<Vec<Learning>> {
        let mut anti_patterns = Vec::new();

        let failed_episodes: Vec<_> = episodes
            .iter()
            .filter(|e| matches!(e.outcome.status, crate::types::Outcome::Failure))
            .collect();

        if failed_episodes.is_empty() {
            return Ok(anti_patterns);
        }

        // Group failed episodes by common characteristics
        let mut failure_patterns: HashMap<String, Vec<String>> = HashMap::new();

        for episode in failed_episodes {
            // Look for common failure indicators
            let failure_key = if !episode.outcome.files_modified.is_empty() {
                format!(
                    "Failed after modifying: {:?}",
                    episode.outcome.files_modified.first()
                )
            } else {
                format!("Failed: {}", episode.task)
            };

            failure_patterns
                .entry(failure_key)
                .or_default()
                .push(episode.id.0.clone());
        }

        // Create anti-pattern learnings
        for (pattern, episode_ids) in failure_patterns {
            if episode_ids.len() >= 2 {
                anti_patterns.push(Learning {
                    id: format!("antipattern_{}", uuid::Uuid::new_v4()),
                    pattern: format!("Anti-pattern: {}", pattern),
                    confidence: episode_ids.len() as f32 / episodes.len() as f32,
                    episodes: episode_ids.clone(),
                    applications: episode_ids.len(),
                    created_at: Utc::now(),
                    last_applied: None,
                    category: LearningCategory::AntiPattern,
                    metadata: HashMap::new(),
                });
            }
        }

        Ok(anti_patterns)
    }

    /// Group similar episodes
    async fn group_similar_episodes(&self, episodes: &[Episode]) -> Result<Vec<Vec<Episode>>> {
        let mut groups: Vec<Vec<Episode>> = Vec::new();
        let mut processed: std::collections::HashSet<String> = std::collections::HashSet::new();

        for episode in episodes {
            if processed.contains(&episode.id.0) {
                continue;
            }

            let mut group = vec![episode.clone()];
            processed.insert(episode.id.0.clone());

            // Find similar episodes
            for other in episodes {
                if processed.contains(&other.id.0) {
                    continue;
                }

                if self.are_similar(episode, other) {
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

    /// Check if two episodes are similar
    fn are_similar(&self, e1: &Episode, e2: &Episode) -> bool {
        // Simple similarity based on task description overlap
        let words1: std::collections::HashSet<_> =
            e1.task.split_whitespace().map(|w| w.to_lowercase()).collect();
        let words2: std::collections::HashSet<_> =
            e2.task.split_whitespace().map(|w| w.to_lowercase()).collect();

        let intersection = words1.intersection(&words2).count();
        let union = words1.union(&words2).count();

        if union == 0 {
            return false;
        }

        let similarity = intersection as f32 / union as f32;
        similarity > 0.4
    }

    /// Apply learning to new situation
    pub async fn apply_learning(&self, learning: &Learning, context: &Context) -> Result<Suggestion> {
        tracing::debug!(
            learning_id = %learning.id,
            task = %context.task_description,
            "Applying learning to new context"
        );

        // Calculate confidence for this application
        let mut confidence = learning.confidence;

        // Adjust confidence based on context similarity
        if !context.task_description.is_empty() {
            let keywords: Vec<_> = extract_keywords(&context.task_description);
            let pattern_keywords: Vec<_> = extract_keywords(&learning.pattern);

            let overlap = keywords
                .iter()
                .filter(|k| pattern_keywords.contains(k))
                .count();

            if !pattern_keywords.is_empty() {
                let similarity = overlap as f32 / pattern_keywords.len() as f32;
                confidence *= 0.5 + similarity * 0.5; // Blend with similarity
            }
        }

        let suggestion = Suggestion {
            learning_id: learning.id.clone(),
            description: format!("Apply pattern: {}", learning.pattern),
            confidence,
            reasoning: format!(
                "This pattern has been successfully applied {} times with {:.0}% success rate",
                learning.applications,
                learning.confidence * 100.0
            ),
            example_episodes: learning.episodes.iter().take(3).cloned().collect(),
        };

        Ok(suggestion)
    }

    /// Update learning confidence based on outcome
    pub async fn update_confidence(&self, learning_id: &str, success: bool) -> Result<()> {
        tracing::debug!(
            learning_id = %learning_id,
            success = %success,
            "Updating learning confidence"
        );

        // Load learning
        let key = format!("learning:{}", learning_id);
        let data = self
            .storage
            .get(key.as_bytes())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Learning not found"))?;

        let mut learning: Learning = deserialize(&data)?;

        // Update confidence using running average
        let old_total = learning.confidence * learning.applications as f32;
        learning.applications += 1;

        let new_value = if success { 1.0 } else { 0.0 };
        learning.confidence = (old_total + new_value) / learning.applications as f32;
        learning.last_applied = Some(Utc::now());

        // Store updated learning
        self.store_learning(&learning).await?;

        tracing::info!(
            "Updated learning confidence: {:.2} (based on {} applications)",
            learning.confidence,
            learning.applications
        );

        Ok(())
    }

    /// Store a learning in storage
    async fn store_learning(&self, learning: &Learning) -> Result<()> {
        let key = format!("learning:{}", learning.id);
        let value = serialize(learning)?;
        self.storage.put(key.as_bytes(), &value).await?;
        Ok(())
    }

    /// Load all learnings from storage
    pub async fn load_learnings(&self) -> Result<Vec<Learning>> {
        let keys = self.storage.get_keys_with_prefix(b"learning:").await?;

        let mut learnings = Vec::new();
        for key in keys {
            if let Some(data) = self.storage.get(&key).await? {
                let learning: Learning = deserialize(&data)?;
                learnings.push(learning);
            }
        }

        tracing::info!("Loaded {} learnings from storage", learnings.len());
        Ok(learnings)
    }

    /// Get learning statistics
    pub async fn get_statistics(&self) -> Result<LearningStats> {
        let learnings = self.load_learnings().await?;

        let total_learnings = learnings.len();
        let high_confidence_learnings = learnings.iter().filter(|l| l.confidence >= 0.8).count();
        let frequently_applied_learnings = learnings.iter().filter(|l| l.applications >= 5).count();

        let average_confidence = if total_learnings > 0 {
            learnings.iter().map(|l| l.confidence).sum::<f32>() / total_learnings as f32
        } else {
            0.0
        };

        let average_applications = if total_learnings > 0 {
            learnings.iter().map(|l| l.applications).sum::<usize>() as f32 / total_learnings as f32
        } else {
            0.0
        };

        Ok(LearningStats {
            total_learnings,
            high_confidence_learnings,
            frequently_applied_learnings,
            average_confidence,
            average_applications,
        })
    }

    /// Find learnings relevant to a context
    pub async fn find_relevant_learnings(&self, context: &Context, limit: usize) -> Result<Vec<Learning>> {
        let all_learnings = self.load_learnings().await?;

        let keywords = extract_keywords(&context.task_description);

        let mut scored: Vec<_> = all_learnings
            .into_iter()
            .map(|learning| {
                let pattern_keywords = extract_keywords(&learning.pattern);
                let overlap = keywords
                    .iter()
                    .filter(|k| pattern_keywords.contains(k))
                    .count();

                let relevance = if !pattern_keywords.is_empty() {
                    overlap as f32 / pattern_keywords.len() as f32
                } else {
                    0.0
                };

                (learning, relevance)
            })
            .filter(|(_, relevance)| *relevance > 0.1)
            .collect();

        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(scored.into_iter().take(limit).map(|(l, _)| l).collect())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStorage;
    use crate::types::{EpisodeId, Outcome};
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
    async fn test_extract_learnings() {
        let (db, storage, _temp1, _temp2) = create_test_setup().await;
        let extractor = LearningExtractor::new(db, storage);

        let episodes = vec![
            Episode {
                id: EpisodeId::new(),
                timestamp: Utc::now(),
                task: "Implement authentication".to_string(),
                context: Context::default(),
                actions: vec![],
                outcome: super::super::episode_recorder::EpisodeOutcome {
                    status: Outcome::Success,
                    description: "Success".to_string(),
                    files_modified: vec!["auth.rs".to_string()],
                    tests_passed: Some(true),
                    build_succeeded: Some(true),
                    commit_hash: None,
                },
                learnings: Vec::new(),
                embedding: None,
                metadata: HashMap::new(),
                duration_seconds: 60,
            },
            Episode {
                id: EpisodeId::new(),
                timestamp: Utc::now(),
                task: "Fix authentication bug".to_string(),
                context: Context::default(),
                actions: vec![],
                outcome: super::super::episode_recorder::EpisodeOutcome {
                    status: Outcome::Success,
                    description: "Success".to_string(),
                    files_modified: vec!["auth.rs".to_string()],
                    tests_passed: Some(true),
                    build_succeeded: Some(true),
                    commit_hash: None,
                },
                learnings: Vec::new(),
                embedding: None,
                metadata: HashMap::new(),
                duration_seconds: 45,
            },
        ];

        let learnings = extractor.extract_from_episodes(episodes).await.unwrap();
        assert!(!learnings.is_empty());
    }

    #[tokio::test]
    async fn test_update_confidence() {
        let (db, storage, _temp1, _temp2) = create_test_setup().await;
        let extractor = LearningExtractor::new(db, storage);

        let learning = Learning {
            id: "test_learning".to_string(),
            pattern: "Test pattern".to_string(),
            confidence: 0.8,
            episodes: vec!["ep1".to_string()],
            applications: 5,
            created_at: Utc::now(),
            last_applied: None,
            category: LearningCategory::SolutionPattern,
            metadata: HashMap::new(),
        };

        extractor.store_learning(&learning).await.unwrap();

        extractor
            .update_confidence(&learning.id, true)
            .await
            .unwrap();

        let learnings = extractor.load_learnings().await.unwrap();
        assert_eq!(learnings.len(), 1);
        assert_eq!(learnings[0].applications, 6);
    }
}
