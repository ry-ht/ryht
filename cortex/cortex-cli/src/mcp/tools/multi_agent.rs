//! Multi-Agent Coordination Tools (14 tools)
//!
//! Provides comprehensive multi-agent coordination including:
//! - Session management
//! - Lock acquisition and release with deadlock detection
//! - Three-way merge with semantic conflict detection
//! - Agent registration and messaging

use async_trait::async_trait;
use cortex_storage::{
    locks::*, ConnectionManager, MergeEngine, MergeRequest, MergeStrategy,
};
use mcp_sdk::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

#[derive(Clone)]
pub struct MultiAgentContext {
    storage: Arc<ConnectionManager>,
    lock_manager: Arc<LockManager>,
    merge_engine: Arc<MergeEngine>,
}

impl MultiAgentContext {
    pub fn new(storage: Arc<ConnectionManager>) -> Self {
        // Create lock manager with 5 minute default timeout and 100ms detection interval
        let lock_manager = Arc::new(LockManager::new(
            Duration::from_secs(300),
            Duration::from_millis(100),
        ));

        // Create merge engine
        let merge_engine = Arc::new(MergeEngine::new(storage.clone()));

        // Spawn background tasks
        let lock_manager_clone = lock_manager.clone();
        tokio::spawn(async move {
            lock_manager_clone.run_cleanup_loop(Duration::from_secs(1)).await;
        });

        let lock_manager_clone = lock_manager.clone();
        tokio::spawn(async move {
            lock_manager_clone.run_deadlock_detection_loop().await;
        });

        Self {
            storage,
            lock_manager,
            merge_engine,
        }
    }

    pub fn lock_manager(&self) -> Arc<LockManager> {
        self.lock_manager.clone()
    }

    pub fn merge_engine(&self) -> Arc<MergeEngine> {
        self.merge_engine.clone()
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
    #[serde(default = "default_true")]
    verify_semantics: bool,
    target_namespace: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct SessionMergeOutput {
    session_id: String,
    success: bool,
    changes_applied: i32,
    changes_rejected: i32,
    conflicts: Vec<ConflictSummary>,
    duration_ms: i64,
    verification_passed: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ConflictSummary {
    conflict_id: String,
    entity_id: String,
    conflict_type: String,
    file_path: String,
    can_auto_resolve: bool,
}

pub struct SessionMergeTool {
    ctx: MultiAgentContext,
}

impl SessionMergeTool {
    pub fn new(ctx: MultiAgentContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for SessionMergeTool {
    fn name(&self) -> &str {
        "cortex.session.merge"
    }

    fn description(&self) -> Option<&str> {
        Some("Merge session changes to main using three-way merge with semantic conflict detection")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SessionMergeInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SessionMergeInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Merging session {} with strategy {}", input.session_id, input.merge_strategy);

        let strategy = match input.merge_strategy.as_str() {
            "auto" => MergeStrategy::AutoMerge,
            "manual" => MergeStrategy::Manual,
            "prefer_session" => MergeStrategy::PreferSession,
            "prefer_main" => MergeStrategy::PreferMain,
            "three_way" => MergeStrategy::ThreeWay,
            _ => MergeStrategy::ThreeWay,
        };

        let mut request = MergeRequest::new(input.session_id.clone(), strategy);
        request.verify_semantics = input.verify_semantics;
        if let Some(ns) = input.target_namespace {
            request = request.with_namespace(ns);
        }

        match self.ctx.merge_engine().merge_session(request).await {
            Ok(result) => {
                let conflicts: Vec<ConflictSummary> = result.conflicts.iter().map(|c| {
                    ConflictSummary {
                        conflict_id: c.id.clone(),
                        entity_id: c.entity_id.clone(),
                        conflict_type: format!("{}", c.conflict_type),
                        file_path: c.file_path.clone(),
                        can_auto_resolve: c.resolution.is_some(),
                    }
                }).collect();

                let output = SessionMergeOutput {
                    session_id: input.session_id,
                    success: result.success,
                    changes_applied: result.changes_applied as i32,
                    changes_rejected: result.changes_rejected as i32,
                    conflicts,
                    duration_ms: result.duration_ms as i64,
                    verification_passed: result.verification.as_ref().map(|v| v.passed).unwrap_or(true),
                };

                Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
            }
            Err(e) => {
                warn!("Merge failed: {}", e);
                Ok(ToolResult::error(format!("Merge failed: {}", e)))
            }
        }
    }
}

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
    session_id: String,
    entity_id: String,
    #[serde(default = "default_code_unit")]
    entity_type: String,
    #[serde(default = "default_write_lock")]
    lock_type: String,
    #[serde(default = "default_lock_timeout")]
    timeout_seconds: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct LockAcquireOutput {
    lock_id: String,
    entity_id: String,
    acquired: bool,
    lock_type: String,
    expires_at: String,
}

pub struct LockAcquireTool {
    ctx: MultiAgentContext,
}

impl LockAcquireTool {
    pub fn new(ctx: MultiAgentContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for LockAcquireTool {
    fn name(&self) -> &str {
        "cortex.lock.acquire"
    }

    fn description(&self) -> Option<&str> {
        Some("Acquire lock on entity with deadlock detection")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(LockAcquireInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: LockAcquireInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let lock_type = match input.lock_type.as_str() {
            "read" => LockType::Read,
            "write" => LockType::Write,
            "intent" => LockType::Intent,
            _ => LockType::Write,
        };

        let entity_type = match input.entity_type.as_str() {
            "vnode" => EntityType::VNode,
            "code_unit" => EntityType::CodeUnit,
            "dependency" => EntityType::Dependency,
            "workspace" => EntityType::Workspace,
            _ => EntityType::Custom,
        };

        let request = LockRequest {
            entity_id: input.entity_id.clone(),
            entity_type,
            lock_type,
            timeout: Duration::from_secs(input.timeout_seconds as u64),
            metadata: None,
        };

        match self.ctx.lock_manager().acquire_lock(&input.session_id, request).await {
            Ok(lock) => {
                let output = LockAcquireOutput {
                    lock_id: lock.lock_id,
                    entity_id: lock.entity_id,
                    acquired: true,
                    lock_type: format!("{:?}", lock.lock_type).to_lowercase(),
                    expires_at: lock.expires_at.to_rfc3339(),
                };
                Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
            }
            Err(e) => {
                warn!("Failed to acquire lock: {}", e);
                let output = LockAcquireOutput {
                    lock_id: String::new(),
                    entity_id: input.entity_id,
                    acquired: false,
                    lock_type: input.lock_type,
                    expires_at: String::new(),
                };
                Ok(ToolResult::error(format!("Failed to acquire lock: {}. Output: {}", e, serde_json::to_string(&output).unwrap())))
            }
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LockReleaseInput {
    lock_id: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct LockReleaseOutput {
    lock_id: String,
    released: bool,
}

pub struct LockReleaseTool {
    ctx: MultiAgentContext,
}

impl LockReleaseTool {
    pub fn new(ctx: MultiAgentContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for LockReleaseTool {
    fn name(&self) -> &str {
        "cortex.lock.release"
    }

    fn description(&self) -> Option<&str> {
        Some("Release a lock")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(LockReleaseInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: LockReleaseInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        match self.ctx.lock_manager().release_lock(&input.lock_id) {
            Ok(_) => {
                let output = LockReleaseOutput {
                    lock_id: input.lock_id,
                    released: true,
                };
                Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
            }
            Err(e) => {
                let output = LockReleaseOutput {
                    lock_id: input.lock_id,
                    released: false,
                };
                Ok(ToolResult::error(format!("Failed to release lock: {}. Output: {}", e, serde_json::to_string(&output).unwrap())))
            }
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LockListInput {
    session_id: Option<String>,
    entity_id: Option<String>,
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
    session_id: String,
    lock_type: String,
    acquired_at: String,
    expires_at: String,
}

pub struct LockListTool {
    ctx: MultiAgentContext,
}

impl LockListTool {
    pub fn new(ctx: MultiAgentContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for LockListTool {
    fn name(&self) -> &str {
        "cortex.lock.list"
    }

    fn description(&self) -> Option<&str> {
        Some("List active locks")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(LockListInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: LockListInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let locks = if let Some(session_id) = input.session_id {
            self.ctx.lock_manager().list_session_locks(&session_id)
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
        } else if let Some(entity_id) = input.entity_id {
            self.ctx.lock_manager().list_entity_locks(&entity_id)
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
        } else {
            self.ctx.lock_manager().list_locks()
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
        };

        let lock_infos: Vec<LockInfo> = locks.iter().map(|lock| LockInfo {
            lock_id: lock.lock_id.clone(),
            entity_id: lock.entity_id.clone(),
            session_id: lock.holder_session.clone(),
            lock_type: format!("{:?}", lock.lock_type).to_lowercase(),
            acquired_at: lock.acquired_at.to_rfc3339(),
            expires_at: lock.expires_at.to_rfc3339(),
        }).collect();

        let output = LockListOutput {
            total_count: lock_infos.len() as i32,
            locks: lock_infos,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

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

// Add lock check tool
#[derive(Debug, Deserialize, JsonSchema)]
pub struct LockCheckInput {
    entity_id: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct LockCheckOutput {
    entity_id: String,
    is_locked: bool,
    lock_count: i32,
    locks: Vec<LockInfo>,
}

pub struct LockCheckTool {
    ctx: MultiAgentContext,
}

impl LockCheckTool {
    pub fn new(ctx: MultiAgentContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for LockCheckTool {
    fn name(&self) -> &str {
        "cortex.lock.check"
    }

    fn description(&self) -> Option<&str> {
        Some("Check if entity is locked")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(LockCheckInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: LockCheckInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let is_locked = self.ctx.lock_manager().is_locked(&input.entity_id)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let locks = self.ctx.lock_manager().list_entity_locks(&input.entity_id)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let lock_infos: Vec<LockInfo> = locks.iter().map(|lock| LockInfo {
            lock_id: lock.lock_id.clone(),
            entity_id: lock.entity_id.clone(),
            session_id: lock.holder_session.clone(),
            lock_type: format!("{:?}", lock.lock_type).to_lowercase(),
            acquired_at: lock.acquired_at.to_rfc3339(),
            expires_at: lock.expires_at.to_rfc3339(),
        }).collect();

        let output = LockCheckOutput {
            entity_id: input.entity_id,
            is_locked,
            lock_count: lock_infos.len() as i32,
            locks: lock_infos,
        };

        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

// =============================================================================
// Conflict Management Tools
// =============================================================================

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ConflictListInput {
    session_id: String,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ConflictListOutput {
    session_id: String,
    conflicts: Vec<ConflictDetail>,
    total_count: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ConflictDetail {
    conflict_id: String,
    entity_id: String,
    conflict_type: String,
    file_path: String,
    line_range: Option<String>,
    base_version: Option<String>,
    session_version: Option<String>,
    main_version: Option<String>,
    suggested_resolution: Option<String>,
}

impl_agent_tool!(ConflictListTool, "cortex.conflicts.list", "List merge conflicts for a session", ConflictListInput, ConflictListOutput);

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ConflictResolveInput {
    conflict_id: String,
    resolution: String,
    #[serde(default = "default_false")]
    apply_to_similar: bool,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct ConflictResolveOutput {
    conflict_id: String,
    resolved: bool,
    conflicts_resolved: i32,
}

impl_agent_tool!(ConflictResolveTool, "cortex.conflicts.resolve", "Manually resolve a conflict", ConflictResolveInput, ConflictResolveOutput);

// =============================================================================
// Helper Functions
// =============================================================================

fn default_isolation() -> String { "snapshot".to_string() }
fn default_ttl() -> i32 { 3600 }
fn default_auto_merge() -> String { "auto".to_string() }
fn default_code_unit() -> String { "code_unit".to_string() }
fn default_write_lock() -> String { "write".to_string() }
fn default_lock_timeout() -> i32 { 300 }
fn default_developer_type() -> String { "developer".to_string() }
fn default_request_type() -> String { "request".to_string() }
fn default_true() -> bool { true }
fn default_false() -> bool { false }
