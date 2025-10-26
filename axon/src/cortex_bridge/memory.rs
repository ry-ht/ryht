//! Episodic memory management for Cortex integration
//!
//! This module handles episode storage, retrieval, and pattern learning.

use super::client::{CortexClient, Result};
use super::models::*;
use serde::{Deserialize, Serialize};
use tracing::info;

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct CreateEpisodeRequest {
    pub episode_type: String,
    pub task_description: String,
    pub agent_id: String,
    pub session_id: Option<String>,
    pub workspace_id: String,
    pub entities_created: Vec<String>,
    pub entities_modified: Vec<String>,
    pub entities_deleted: Vec<String>,
    pub files_touched: Vec<String>,
    pub queries_made: Vec<String>,
    pub tools_used: Vec<ToolUsage>,
    pub solution_summary: String,
    pub outcome: String,
    pub success_metrics: serde_json::Value,
    pub errors_encountered: Vec<String>,
    pub lessons_learned: Vec<String>,
    pub duration_seconds: i32,
    pub tokens_used: TokenUsage,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateEpisodeResponse {
    pub episode_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchEpisodesRequest {
    pub query: String,
    pub limit: usize,
    pub min_similarity: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchEpisodesResponse {
    pub episodes: Vec<Episode>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PatternsResponse {
    pub patterns: Vec<Pattern>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreatePatternRequest {
    pub pattern_type: String,
    pub name: String,
    pub description: String,
    pub context: String,
    pub before_state: serde_json::Value,
    pub after_state: serde_json::Value,
    pub transformation: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatePatternResponse {
    pub pattern_id: String,
}

// ============================================================================
// Memory Manager
// ============================================================================

/// Memory manager for episodic learning
pub struct MemoryManager {
    client: CortexClient,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new(client: CortexClient) -> Self {
        Self { client }
    }

    /// Store an episode
    pub async fn store_episode(&self, episode: Episode) -> Result<EpisodeId> {
        let request = CreateEpisodeRequest {
            episode_type: format!("{:?}", episode.episode_type).to_lowercase(),
            task_description: episode.task_description,
            agent_id: episode.agent_id,
            session_id: episode.session_id,
            workspace_id: episode.workspace_id,
            entities_created: episode.entities_created,
            entities_modified: episode.entities_modified,
            entities_deleted: episode.entities_deleted,
            files_touched: episode.files_touched,
            queries_made: episode.queries_made,
            tools_used: episode.tools_used,
            solution_summary: episode.solution_summary,
            outcome: format!("{:?}", episode.outcome).to_lowercase(),
            success_metrics: episode.success_metrics,
            errors_encountered: episode.errors_encountered,
            lessons_learned: episode.lessons_learned,
            duration_seconds: episode.duration_seconds,
            tokens_used: episode.tokens_used,
        };

        let response: CreateEpisodeResponse = self
            .client
            .post("/memory/episodes", &request)
            .await?;

        let episode_id = EpisodeId::from(response.episode_id);
        info!("Stored episode {}", episode_id);

        Ok(episode_id)
    }

    /// Search for similar episodes
    pub async fn search_episodes(&self, query: &str, limit: usize) -> Result<Vec<Episode>> {
        let request = SearchEpisodesRequest {
            query: query.to_string(),
            limit,
            min_similarity: 0.7,
        };

        let response: SearchEpisodesResponse = self
            .client
            .post("/memory/search", &request)
            .await?;

        info!("Found {} similar episodes for query", response.episodes.len());
        Ok(response.episodes)
    }

    /// Get learned patterns
    pub async fn get_patterns(&self) -> Result<Vec<Pattern>> {
        let response: PatternsResponse = self.client.get("/memory/patterns").await?;

        info!("Retrieved {} patterns", response.patterns.len());
        Ok(response.patterns)
    }

    /// Store a new pattern
    pub async fn store_pattern(&self, pattern: Pattern) -> Result<String> {
        let request = CreatePatternRequest {
            pattern_type: format!("{:?}", pattern.pattern_type).to_lowercase(),
            name: pattern.name,
            description: pattern.description,
            context: pattern.context,
            before_state: pattern.before_state,
            after_state: pattern.after_state,
            transformation: pattern.transformation,
        };

        let response: CreatePatternResponse = self
            .client
            .post("/memory/patterns", &request)
            .await?;

        info!("Stored pattern {}", response.pattern_id);
        Ok(response.pattern_id)
    }

    /// Get a specific episode by ID
    pub async fn get_episode(&self, episode_id: &EpisodeId) -> Result<Episode> {
        let path = format!("/memory/episodes/{}", episode_id);
        let episode: Episode = self.client.get(&path).await?;

        Ok(episode)
    }

    /// Get a specific pattern by ID
    pub async fn get_pattern(&self, pattern_id: &str) -> Result<Pattern> {
        let path = format!("/memory/patterns/{}", pattern_id);
        let pattern: Pattern = self.client.get(&path).await?;

        Ok(pattern)
    }

    /// Update pattern statistics after application
    pub async fn update_pattern_stats(
        &self,
        pattern_id: &str,
        success: bool,
        improvement: serde_json::Value,
    ) -> Result<()> {
        #[derive(Serialize)]
        struct UpdatePatternStatsRequest {
            success: bool,
            improvement: serde_json::Value,
        }

        let request = UpdatePatternStatsRequest {
            success,
            improvement,
        };

        let path = format!("/memory/patterns/{}/stats", pattern_id);
        let _: serde_json::Value = self.client.put(&path, &request).await?;

        info!(
            "Updated pattern {} stats: success={}",
            pattern_id, success
        );
        Ok(())
    }

    /// Get related episodes for a pattern
    pub async fn get_pattern_episodes(&self, pattern_id: &str) -> Result<Vec<Episode>> {
        let path = format!("/memory/patterns/{}/episodes", pattern_id);

        #[derive(Deserialize)]
        struct EpisodesResponse {
            episodes: Vec<Episode>,
        }

        let response: EpisodesResponse = self.client.get(&path).await?;
        Ok(response.episodes)
    }

    /// Share an episode with other agents
    pub async fn share_episode(
        &self,
        episode_id: &EpisodeId,
        target_agents: Vec<AgentId>,
    ) -> Result<()> {
        #[derive(Serialize)]
        struct ShareEpisodeRequest {
            target_agents: Vec<String>,
        }

        let request = ShareEpisodeRequest {
            target_agents: target_agents.iter().map(|a| a.to_string()).collect(),
        };

        let path = format!("/memory/episodes/{}/share", episode_id);
        let _: serde_json::Value = self.client.post(&path, &request).await?;

        info!(
            "Shared episode {} with {} agents",
            episode_id,
            target_agents.len()
        );
        Ok(())
    }

    /// Get shared episodes from other agents
    pub async fn get_shared_episodes(
        &self,
        agent_id: &AgentId,
        limit: usize,
    ) -> Result<Vec<Episode>> {
        let path = format!("/memory/shared/{}?limit={}", agent_id, limit);

        #[derive(Deserialize)]
        struct SharedEpisodesResponse {
            episodes: Vec<Episode>,
        }

        let response: SharedEpisodesResponse = self.client.get(&path).await?;
        info!(
            "Retrieved {} shared episodes for agent {}",
            response.episodes.len(),
            agent_id
        );
        Ok(response.episodes)
    }

    /// Get collaborative insights for a workspace
    pub async fn get_collaborative_insights(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<CollaborativeInsight>> {
        let path = format!("/memory/insights/{}", workspace_id);

        #[derive(Deserialize)]
        struct InsightsResponse {
            insights: Vec<CollaborativeInsight>,
        }

        let response: InsightsResponse = self.client.get(&path).await?;
        info!(
            "Retrieved {} collaborative insights for workspace {}",
            response.insights.len(),
            workspace_id
        );
        Ok(response.insights)
    }

    /// Search for patterns by query and type
    pub async fn search_patterns(
        &self,
        query: &str,
        pattern_type: Option<PatternType>,
        limit: usize,
    ) -> Result<Vec<Pattern>> {
        #[derive(Serialize)]
        struct SearchPatternsRequest {
            query: String,
            pattern_type: Option<String>,
            limit: usize,
        }

        let request = SearchPatternsRequest {
            query: query.to_string(),
            pattern_type: pattern_type.map(|t| format!("{:?}", t).to_lowercase()),
            limit,
        };

        #[derive(Deserialize)]
        struct SearchPatternsResponse {
            patterns: Vec<Pattern>,
        }

        let response: SearchPatternsResponse = self
            .client
            .post("/memory/patterns/search", &request)
            .await?;

        info!("Found {} patterns for query: {}", response.patterns.len(), query);
        Ok(response.patterns)
    }

    /// Get pattern evolution history
    pub async fn get_pattern_history(&self, pattern_id: &str) -> Result<Vec<PatternVersion>> {
        let path = format!("/memory/patterns/{}/history", pattern_id);

        #[derive(Deserialize)]
        struct PatternHistoryResponse {
            versions: Vec<PatternVersion>,
        }

        let response: PatternHistoryResponse = self.client.get(&path).await?;
        Ok(response.versions)
    }

    /// Apply a pattern and record the outcome
    pub async fn apply_pattern(
        &self,
        pattern_id: &str,
        context: serde_json::Value,
    ) -> Result<PatternApplication> {
        #[derive(Serialize)]
        struct ApplyPatternRequest {
            context: serde_json::Value,
        }

        let request = ApplyPatternRequest { context };
        let path = format!("/memory/patterns/{}/apply", pattern_id);

        let application: PatternApplication = self.client.post(&path, &request).await?;

        info!(
            "Applied pattern {} with success: {}",
            pattern_id, application.success
        );
        Ok(application)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_episode_type_serialization() {
        let episode = Episode {
            id: "test".to_string(),
            episode_type: EpisodeType::Task,
            task_description: "test task".to_string(),
            agent_id: "agent1".to_string(),
            session_id: None,
            workspace_id: "ws1".to_string(),
            entities_created: vec![],
            entities_modified: vec![],
            entities_deleted: vec![],
            files_touched: vec![],
            queries_made: vec![],
            tools_used: vec![],
            solution_summary: "test".to_string(),
            outcome: EpisodeOutcome::Success,
            success_metrics: serde_json::json!({}),
            errors_encountered: vec![],
            lessons_learned: vec![],
            duration_seconds: 10,
            tokens_used: TokenUsage::default(),
            embedding: vec![],
            created_at: Utc::now(),
            completed_at: Some(Utc::now()),
        };

        let json = serde_json::to_string(&episode).unwrap();
        assert!(json.contains("task"));
    }

    #[test]
    fn test_pattern_default() {
        let pattern = Pattern::default();
        assert_eq!(pattern.times_applied, 0);
        assert_eq!(pattern.success_rate, 0.0);
    }
}
