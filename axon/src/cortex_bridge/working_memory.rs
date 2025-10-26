//! Working memory management for Cortex integration
//!
//! This module provides working memory operations, allowing agents to maintain
//! temporary context during task execution.

use super::client::{CortexClient, Result};
use super::models::*;
use serde::{Deserialize, Serialize};
use tracing::info;

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct AddWorkingMemoryRequest {
    pub agent_id: String,
    pub session_id: String,
    pub item_type: String,
    pub content: String,
    pub context: serde_json::Value,
    pub priority: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkingMemoryItemsResponse {
    pub items: Vec<WorkingMemoryItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkingMemoryStatsResponse {
    pub stats: WorkingMemoryStats,
}

// ============================================================================
// Working Memory Manager
// ============================================================================

/// Working memory manager for short-term context
pub struct WorkingMemoryManager {
    client: CortexClient,
}

impl WorkingMemoryManager {
    /// Create a new working memory manager
    pub fn new(client: CortexClient) -> Self {
        Self { client }
    }

    /// Add an item to working memory
    pub async fn add_item(
        &self,
        agent_id: &AgentId,
        session_id: &SessionId,
        item: WorkingMemoryItem,
    ) -> Result<()> {
        let request = AddWorkingMemoryRequest {
            agent_id: agent_id.to_string(),
            session_id: session_id.to_string(),
            item_type: item.item_type,
            content: item.content,
            context: item.context,
            priority: item.priority,
        };

        let path = format!("/memory/working/{}", agent_id);
        let _: serde_json::Value = self.client.post(&path, &request).await?;

        info!(
            "Added working memory item for agent {} in session {}",
            agent_id, session_id
        );

        Ok(())
    }

    /// Get all working memory items for an agent session
    pub async fn get_items(
        &self,
        agent_id: &AgentId,
        session_id: &SessionId,
    ) -> Result<Vec<WorkingMemoryItem>> {
        let path = format!(
            "/memory/working/{}?session_id={}",
            agent_id, session_id
        );

        let response: WorkingMemoryItemsResponse = self.client.get(&path).await?;

        info!(
            "Retrieved {} working memory items for agent {} in session {}",
            response.items.len(),
            agent_id,
            session_id
        );

        Ok(response.items)
    }

    /// Clear working memory for a session
    pub async fn clear_session(
        &self,
        agent_id: &AgentId,
        session_id: &SessionId,
    ) -> Result<()> {
        let path = format!(
            "/memory/working/{}/clear?session_id={}",
            agent_id, session_id
        );

        let _: serde_json::Value = self.client.delete(&path).await?;

        info!(
            "Cleared working memory for agent {} in session {}",
            agent_id, session_id
        );

        Ok(())
    }

    /// Get working memory statistics for an agent
    pub async fn get_stats(&self, agent_id: &AgentId) -> Result<WorkingMemoryStats> {
        let path = format!("/memory/working/{}/stats", agent_id);

        let response: WorkingMemoryStatsResponse = self.client.get(&path).await?;

        Ok(response.stats)
    }

    /// Clear all working memory for an agent
    pub async fn clear_agent(&self, agent_id: &AgentId) -> Result<()> {
        let path = format!("/memory/working/{}/clear", agent_id);

        let _: serde_json::Value = self.client.delete(&path).await?;

        info!("Cleared all working memory for agent {}", agent_id);

        Ok(())
    }

    /// Update item priority
    pub async fn update_priority(
        &self,
        agent_id: &AgentId,
        item_id: &str,
        priority: f32,
    ) -> Result<()> {
        #[derive(Serialize)]
        struct UpdatePriorityRequest {
            priority: f32,
        }

        let request = UpdatePriorityRequest { priority };
        let path = format!("/memory/working/{}/items/{}/priority", agent_id, item_id);

        let _: serde_json::Value = self.client.put(&path, &request).await?;

        info!(
            "Updated priority to {} for working memory item {} of agent {}",
            priority, item_id, agent_id
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_working_memory_item_structure() {
        let item = WorkingMemoryItem {
            id: "test-item".to_string(),
            item_type: "code_snippet".to_string(),
            content: "fn test() {}".to_string(),
            context: serde_json::json!({"file": "test.rs"}),
            priority: 0.8,
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: 1,
        };

        assert_eq!(item.item_type, "code_snippet");
        assert_eq!(item.priority, 0.8);
    }

    #[test]
    fn test_working_memory_stats_structure() {
        let mut items_by_type = std::collections::HashMap::new();
        items_by_type.insert("code_snippet".to_string(), 5);
        items_by_type.insert("task".to_string(), 3);

        let stats = WorkingMemoryStats {
            total_items: 8,
            total_bytes: 1024,
            capacity_items: 100,
            capacity_bytes: 10240,
            items_by_type,
        };

        assert_eq!(stats.total_items, 8);
        assert_eq!(stats.items_by_type.get("code_snippet"), Some(&5));
    }
}
