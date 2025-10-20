//! Multi-Agent Coordination Tools (10 tools)

use async_trait::async_trait;
use cortex_storage::ConnectionManager;
use mcp_server::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

#[derive(Clone)]
pub struct MultiAgentContext {
    storage: Arc<ConnectionManager>,
}

impl MultiAgentContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        Self { storage }
    }
}

macro_rules! impl_agent_tool {
    ($name:ident, $tool_name:expr, $desc:expr, $input:ty, $output:ty) => {
        pub struct $name {
            ctx: MultiAgentContext,
        }

        impl $name {
            pub fn new(ctx: MultiAgentContext) -> Self {
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
pub struct SessionCreateInput {
    agent_id: String,
    #[serde(default = "default_isolation")]
    isolation_level: String,
    scope_paths: Option<Vec<String>>,
    #[serde(default = "default_ttl")]
    ttl_seconds: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct SessionCreateOutput {
    session_id: String,
    agent_id: String,
    created_at: String,
    expires_at: String,
}

impl_agent_tool!(SessionCreateTool, "cortex.session.create", "Create an isolated work session", SessionCreateInput, SessionCreateOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionUpdateInput {
    session_id: String,
    status: Option<String>,
    extend_ttl: Option<i32>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct SessionUpdateOutput {
    session_id: String,
    status: String,
    new_expires_at: String,
}

impl_agent_tool!(SessionUpdateTool, "cortex.session.update", "Update session state", SessionUpdateInput, SessionUpdateOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionMergeInput {
    session_id: String,
    #[serde(default = "default_auto_merge")]
    merge_strategy: String,
    conflict_resolution: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct SessionMergeOutput {
    session_id: String,
    changes_merged: i32,
    conflicts: Vec<String>,
}

impl_agent_tool!(SessionMergeTool, "cortex.session.merge", "Merge session changes to main", SessionMergeInput, SessionMergeOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionAbandonInput {
    session_id: String,
    reason: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct SessionAbandonOutput {
    session_id: String,
    abandoned: bool,
}

impl_agent_tool!(SessionAbandonTool, "cortex.session.abandon", "Abandon session without merging", SessionAbandonInput, SessionAbandonOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LockAcquireInput {
    entity_id: String,
    #[serde(default = "default_exclusive")]
    lock_type: String,
    #[serde(default = "default_entity_scope")]
    lock_scope: String,
    #[serde(default = "default_lock_timeout")]
    timeout_seconds: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct LockAcquireOutput {
    lock_id: String,
    entity_id: String,
    acquired: bool,
}

impl_agent_tool!(LockAcquireTool, "cortex.lock.acquire", "Acquire lock on entity", LockAcquireInput, LockAcquireOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LockReleaseInput {
    lock_id: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct LockReleaseOutput {
    lock_id: String,
    released: bool,
}

impl_agent_tool!(LockReleaseTool, "cortex.lock.release", "Release a lock", LockReleaseInput, LockReleaseOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LockListInput {
    agent_id: Option<String>,
    entity_id: Option<String>,
    #[serde(default = "default_active_status")]
    status: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct LockListOutput {
    locks: Vec<LockInfo>,
    total_count: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct LockInfo {
    lock_id: String,
    entity_id: String,
    agent_id: String,
    lock_type: String,
}

impl_agent_tool!(LockListTool, "cortex.lock.list", "List active locks", LockListInput, LockListOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AgentRegisterInput {
    agent_id: String,
    #[serde(default = "default_developer_type")]
    agent_type: String,
    capabilities: Option<Vec<String>>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct AgentRegisterOutput {
    agent_id: String,
    registered: bool,
}

impl_agent_tool!(AgentRegisterTool, "cortex.agent.register", "Register an agent", AgentRegisterInput, AgentRegisterOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AgentSendMessageInput {
    to_agent: String,
    #[serde(default = "default_request_type")]
    message_type: String,
    content: serde_json::Value,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct AgentSendMessageOutput {
    message_id: String,
    sent: bool,
}

impl_agent_tool!(AgentSendMessageTool, "cortex.agent.send_message", "Send message to another agent", AgentSendMessageInput, AgentSendMessageOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AgentGetMessagesInput {
    agent_id: String,
    since: Option<String>,
    message_types: Option<Vec<String>>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct AgentGetMessagesOutput {
    messages: Vec<AgentMessage>,
    total_count: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct AgentMessage {
    message_id: String,
    from_agent: String,
    message_type: String,
    timestamp: String,
}

impl_agent_tool!(AgentGetMessagesTool, "cortex.agent.get_messages", "Retrieve agent messages", AgentGetMessagesInput, AgentGetMessagesOutput);

fn default_isolation() -> String { "snapshot".to_string() }
fn default_ttl() -> i32 { 3600 }
fn default_auto_merge() -> String { "auto".to_string() }
fn default_exclusive() -> String { "exclusive".to_string() }
fn default_entity_scope() -> String { "entity".to_string() }
fn default_lock_timeout() -> i32 { 300 }
fn default_active_status() -> String { "active".to_string() }
fn default_developer_type() -> String { "developer".to_string() }
fn default_request_type() -> String { "request".to_string() }
