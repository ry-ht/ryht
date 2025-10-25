//! Core Agent Types and Data Structures

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Unique identifier for an agent
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(String);

impl AgentId {
    /// Create a new unique agent ID
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Create from string (for deserialization/testing)
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// System agent ID for framework operations
    pub fn system() -> Self {
        Self("system".to_string())
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Classification of agent types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentType {
    /// Master coordination and task delegation
    Orchestrator,

    /// Code generation, modification, and refactoring
    Developer,

    /// Code review, quality assessment, and validation
    Reviewer,

    /// Test generation, execution, and validation
    Tester,

    /// Documentation generation and maintenance
    Documenter,

    /// System design and architecture planning
    Architect,

    /// Information gathering and analysis
    Researcher,

    /// Performance and cost optimization
    Optimizer,

    /// Custom agent type
    Custom(CustomAgentType),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CustomAgentType {
    // Type identifier for custom agents
    // Using &'static str would require lifetime, so we use a type ID
    pub type_id: u32,
}

/// Current status of an agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    /// Agent is idle and available for work
    Idle,

    /// Agent is currently working on tasks
    Working,

    /// Agent is paused
    Paused,

    /// Agent has failed and needs intervention
    Failed,

    /// Agent is shutting down
    ShuttingDown,
}

/// Runtime metrics for agent performance
#[derive(Debug, Serialize, Deserialize)]
pub struct AgentMetrics {
    /// Total tasks completed
    pub tasks_completed: AtomicU64,

    /// Total tasks failed
    pub tasks_failed: AtomicU64,

    /// Average task duration in milliseconds
    pub avg_task_duration_ms: AtomicU64,

    /// Total tokens used (for LLM-based agents)
    pub tokens_used: AtomicU64,

    /// Total cost incurred in cents
    pub total_cost_cents: AtomicU64,

    /// Success rate (0-100)
    pub success_rate: AtomicU64,

    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

impl AgentMetrics {
    /// Create new metrics instance
    pub fn new() -> Self {
        Self {
            tasks_completed: AtomicU64::new(0),
            tasks_failed: AtomicU64::new(0),
            avg_task_duration_ms: AtomicU64::new(0),
            tokens_used: AtomicU64::new(0),
            total_cost_cents: AtomicU64::new(0),
            success_rate: AtomicU64::new(100),
            last_updated: Utc::now(),
        }
    }

    /// Record successful task completion
    pub fn record_success(&self, duration_ms: u64, tokens: u64, cost_cents: u64) {
        self.tasks_completed.fetch_add(1, Ordering::Relaxed);
        self.tokens_used.fetch_add(tokens, Ordering::Relaxed);
        self.total_cost_cents.fetch_add(cost_cents, Ordering::Relaxed);

        // Update average duration
        let completed = self.tasks_completed.load(Ordering::Relaxed);
        let current_avg = self.avg_task_duration_ms.load(Ordering::Relaxed);
        let new_avg = ((current_avg * (completed - 1)) + duration_ms) / completed;
        self.avg_task_duration_ms.store(new_avg, Ordering::Relaxed);

        self.update_success_rate();
    }

    /// Record task failure
    pub fn record_failure(&self) {
        self.tasks_failed.fetch_add(1, Ordering::Relaxed);
        self.update_success_rate();
    }

    fn update_success_rate(&self) {
        let completed = self.tasks_completed.load(Ordering::Relaxed);
        let failed = self.tasks_failed.load(Ordering::Relaxed);
        let total = completed + failed;

        if total > 0 {
            let rate = (completed * 100) / total;
            self.success_rate.store(rate, Ordering::Relaxed);
        }
    }

    /// Get snapshot of current metrics
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            tasks_completed: self.tasks_completed.load(Ordering::Relaxed),
            tasks_failed: self.tasks_failed.load(Ordering::Relaxed),
            avg_task_duration_ms: self.avg_task_duration_ms.load(Ordering::Relaxed),
            tokens_used: self.tokens_used.load(Ordering::Relaxed),
            total_cost_cents: self.total_cost_cents.load(Ordering::Relaxed),
            success_rate: self.success_rate.load(Ordering::Relaxed),
            timestamp: Utc::now(),
        }
    }
}

impl Default for AgentMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of agent metrics at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub avg_task_duration_ms: u64,
    pub tokens_used: u64,
    pub total_cost_cents: u64,
    pub success_rate: u64,
    pub timestamp: DateTime<Utc>,
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent name
    pub name: String,

    /// Agent type
    pub agent_type: AgentType,

    /// Capabilities to enable
    pub capabilities: HashSet<super::Capability>,

    /// Maximum concurrent tasks
    pub max_concurrent_tasks: usize,

    /// Timeout for tasks in seconds
    pub task_timeout_seconds: u64,

    /// Custom configuration
    pub custom_config: HashMap<String, serde_json::Value>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: String::from("unnamed-agent"),
            agent_type: AgentType::Developer,
            capabilities: HashSet::new(),
            max_concurrent_tasks: 1,
            task_timeout_seconds: 300,
            custom_config: HashMap::new(),
        }
    }
}

/// Agent information for discovery and registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: AgentId,
    pub name: String,
    pub agent_type: AgentType,
    pub capabilities: Vec<super::Capability>,
    pub status: AgentStatus,
    pub last_heartbeat: DateTime<Utc>,
    pub metadata: AgentMetadata,
}

/// Additional metadata about an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetadata {
    /// Agent version
    pub version: String,

    /// Specialization tags
    pub specialization: Vec<String>,

    /// Maximum concurrent tasks
    pub max_concurrent_tasks: usize,

    /// Performance score (0.0 to 1.0)
    pub performance_score: f32,

    /// Model configuration (for LLM-based agents)
    pub model_config: Option<ModelConfig>,
}

/// Model configuration for LLM-based agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub provider: String,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
    pub top_p: Option<f32>,
}

impl Default for AgentMetadata {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            specialization: Vec::new(),
            max_concurrent_tasks: 1,
            performance_score: 0.5,
            model_config: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_id() {
        let id1 = AgentId::new();
        let id2 = AgentId::new();
        assert_ne!(id1, id2);

        let system_id = AgentId::system();
        assert_eq!(system_id.to_string(), "system");
    }

    #[test]
    fn test_agent_metrics() {
        let metrics = AgentMetrics::new();

        metrics.record_success(100, 1000, 50);
        assert_eq!(metrics.tasks_completed.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.tokens_used.load(Ordering::Relaxed), 1000);
        assert_eq!(metrics.success_rate.load(Ordering::Relaxed), 100);

        metrics.record_failure();
        assert_eq!(metrics.tasks_failed.load(Ordering::Relaxed), 1);
        assert_eq!(metrics.success_rate.load(Ordering::Relaxed), 50);
    }

    #[test]
    fn test_metrics_snapshot() {
        let metrics = AgentMetrics::new();
        metrics.record_success(200, 500, 25);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.tasks_completed, 1);
        assert_eq!(snapshot.tokens_used, 500);
        assert_eq!(snapshot.total_cost_cents, 25);
    }
}
