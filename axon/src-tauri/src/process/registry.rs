use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Type of process being tracked
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessType {
    AgentRun { agent_id: i64, agent_name: String },
    ClaudeSession { session_id: String },
}

/// Information about a running agent process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub run_id: i64,
    pub process_type: ProcessType,
    pub pid: u32,
    pub started_at: DateTime<Utc>,
    pub project_path: String,
    pub task: String,
    pub model: String,
}

/// Global process registry state - now using cc-sdk's ProcessRegistry
pub struct ProcessRegistryState(pub Arc<cc_sdk::process::ProcessRegistry>);

impl Default for ProcessRegistryState {
    fn default() -> Self {
        Self(Arc::new(cc_sdk::process::ProcessRegistry::new()))
    }
}
