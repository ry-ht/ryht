//! Multi-Agent Coordination Tools (14 tools)
//!
//! Provides comprehensive multi-agent coordination including:
//! - Session management (create, list, update, merge, abandon)
//! - Lock acquisition and release with deadlock detection (acquire, release, list, check)
//! - Three-way merge with semantic conflict detection
//! - Agent registration and inter-agent messaging (register, send_message, get_messages)
//! - Conflict detection and resolution (list, resolve)

use async_trait::async_trait;
use cortex_storage::{
    locks::*, ConnectionManager, MergeEngine, MergeRequest, MergeStrategy,
};
use cortex_vfs::VirtualFileSystem;
use mcp_sdk::prelude::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn, error};
use uuid::Uuid;
use chrono::Utc;

// Import SessionService from the services layer
use crate::services::sessions::{SessionService, SessionMetadata, WorkSession, SessionStatus, SessionFilters, SessionUpdate};

#[derive(Clone)]
pub struct MultiAgentContext {
    storage: Arc<ConnectionManager>,
    lock_manager: Arc<LockManager>,
    merge_engine: Arc<MergeEngine>,
    session_service: Arc<SessionService>,
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

        // Create session service
        let vfs = Arc::new(VirtualFileSystem::new(storage.clone()));
        let session_service = Arc::new(SessionService::with_vfs(storage.clone(), vfs));

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
            session_service,
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

pub struct SessionCreateTool {
    ctx: MultiAgentContext,
}

impl SessionCreateTool {
    pub fn new(ctx: MultiAgentContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for SessionCreateTool {
    fn name(&self) -> &str {
        "cortex.session.create"
    }

    fn description(&self) -> Option<&str> {
        Some("Create an isolated work session")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SessionCreateInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SessionCreateInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Creating session for agent: {}", input.agent_id);

        // Generate workspace ID (in real implementation, this should be passed or retrieved)
        let workspace_id = Uuid::new_v4();

        let mut extra = serde_json::Map::new();
        extra.insert("isolation_level".to_string(), serde_json::json!(input.isolation_level));
        if let Some(paths) = input.scope_paths {
            extra.insert("scope_paths".to_string(), serde_json::json!(paths));
        }
        extra.insert("ttl_seconds".to_string(), serde_json::json!(input.ttl_seconds));
        extra.insert("agent_id".to_string(), serde_json::json!(input.agent_id.clone()));

        let metadata = SessionMetadata { extra };

        match self.ctx.session_service.create_session(
            workspace_id,
            format!("Session for {}", input.agent_id),
            input.agent_id.clone(),
            Some(metadata),
        ).await {
            Ok(session) => {
                let expires_at = session.created_at + chrono::Duration::seconds(input.ttl_seconds as i64);

                let output = SessionCreateOutput {
                    session_id: session.id.to_string(),
                    agent_id: input.agent_id,
                    created_at: session.created_at.to_rfc3339(),
                    expires_at: expires_at.to_rfc3339(),
                };

                info!("Session created: {}", session.id);
                Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
            }
            Err(e) => {
                error!("Failed to create session: {}", e);
                Err(ToolError::ExecutionFailed(format!("Failed to create session: {}", e)))
            }
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionListInput {
    workspace_id: Option<String>,
    status: Option<String>,
    agent_type: Option<String>,
    #[serde(default = "default_list_limit")]
    limit: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct SessionListOutput {
    sessions: Vec<SessionInfo>,
    total_count: i32,
}

#[derive(Debug, Serialize, JsonSchema, Default)]
pub struct SessionInfo {
    session_id: String,
    name: String,
    agent_type: String,
    status: String,
    created_at: String,
    updated_at: String,
}

pub struct SessionListTool {
    ctx: MultiAgentContext,
}

impl SessionListTool {
    pub fn new(ctx: MultiAgentContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for SessionListTool {
    fn name(&self) -> &str {
        "cortex.session.list"
    }

    fn description(&self) -> Option<&str> {
        Some("List sessions with optional filters")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SessionListInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SessionListInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Listing sessions with filters");

        let workspace_id = input.workspace_id
            .map(|id| Uuid::parse_str(&id))
            .transpose()
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid workspace_id: {}", e)))?;

        let filters = SessionFilters {
            status: input.status,
            agent_type: input.agent_type,
            limit: Some(input.limit as usize),
        };

        let sessions = self.ctx.session_service.list_sessions(workspace_id, filters).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to list sessions: {}", e)))?;

        let session_infos: Vec<SessionInfo> = sessions.iter().map(|s| SessionInfo {
            session_id: s.id.to_string(),
            name: s.name.clone(),
            agent_type: s.agent_type.clone(),
            status: format!("{:?}", s.status).to_lowercase(),
            created_at: s.created_at.to_rfc3339(),
            updated_at: s.updated_at.to_rfc3339(),
        }).collect();

        let output = SessionListOutput {
            total_count: session_infos.len() as i32,
            sessions: session_infos,
        };

        debug!("Found {} sessions", output.total_count);
        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

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

pub struct SessionUpdateTool {
    ctx: MultiAgentContext,
}

impl SessionUpdateTool {
    pub fn new(ctx: MultiAgentContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for SessionUpdateTool {
    fn name(&self) -> &str {
        "cortex.session.update"
    }

    fn description(&self) -> Option<&str> {
        Some("Update session state")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SessionUpdateInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SessionUpdateInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Updating session: {}", input.session_id);

        // Get the session first
        let session = self.ctx.session_service.get_session(&input.session_id).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get session: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed("Session not found".to_string()))?;

        // Build update from input
        let mut update = SessionUpdate {
            name: None,
            status: None,
            metadata: None,
        };

        // Update status if provided
        if let Some(status_str) = &input.status {
            let status = match status_str.as_str() {
                "active" => SessionStatus::Active,
                "paused" => SessionStatus::Paused,
                "completed" => SessionStatus::Completed,
                "failed" => SessionStatus::Failed,
                _ => return Err(ToolError::ExecutionFailed(format!("Invalid status: {}", status_str))),
            };
            update.status = Some(status);
        }

        // Extend TTL if provided
        let mut new_metadata = session.metadata.clone();
        if let Some(extend_secs) = input.extend_ttl {
            if let Some(metadata) = new_metadata.as_mut() {
                if let Some(ttl) = metadata.extra.get("ttl_seconds") {
                    if let Some(current_ttl) = ttl.as_i64() {
                        metadata.extra.insert(
                            "ttl_seconds".to_string(),
                            serde_json::json!(current_ttl + extend_secs as i64)
                        );
                    }
                }
            }
            update.metadata = new_metadata;
        }

        // Apply the update
        let updated_session = self.ctx.session_service.update_session(&input.session_id, update).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to update session: {}", e)))?;

        // Calculate new expiration time
        let ttl_seconds = updated_session.metadata
            .as_ref()
            .and_then(|m| m.extra.get("ttl_seconds"))
            .and_then(|v| v.as_i64())
            .unwrap_or(3600);

        let new_expires_at = updated_session.created_at + chrono::Duration::seconds(ttl_seconds);

        let output = SessionUpdateOutput {
            session_id: input.session_id,
            status: format!("{:?}", updated_session.status).to_lowercase(),
            new_expires_at: new_expires_at.to_rfc3339(),
        };

        info!("Session updated: {}", updated_session.id);
        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

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

pub struct SessionAbandonTool {
    ctx: MultiAgentContext,
}

impl SessionAbandonTool {
    pub fn new(ctx: MultiAgentContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for SessionAbandonTool {
    fn name(&self) -> &str {
        "cortex.session.abandon"
    }

    fn description(&self) -> Option<&str> {
        Some("Abandon session without merging")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(SessionAbandonInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: SessionAbandonInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Abandoning session: {} (reason: {:?})", input.session_id, input.reason);

        // Verify session exists
        let session = self.ctx.session_service.get_session(&input.session_id).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to get session: {}", e)))?
            .ok_or_else(|| ToolError::ExecutionFailed("Session not found".to_string()))?;

        // Release all locks held by this session
        let locks = self.ctx.lock_manager().list_session_locks(&input.session_id)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to list locks: {}", e)))?;

        for lock in locks {
            if let Err(e) = self.ctx.lock_manager().release_lock(&lock.lock_id) {
                warn!("Failed to release lock {}: {}", lock.lock_id, e);
            } else {
                debug!("Released lock: {}", lock.lock_id);
            }
        }

        // Update session status to Failed (abandoned)
        let update = SessionUpdate {
            name: None,
            status: Some(SessionStatus::Failed),
            metadata: {
                let mut metadata = session.metadata.clone().unwrap_or_else(|| SessionMetadata {
                    extra: serde_json::Map::new(),
                });
                metadata.extra.insert("abandon_reason".to_string(),
                    serde_json::json!(input.reason.unwrap_or_else(|| "Abandoned by user".to_string())));
                metadata.extra.insert("abandoned_at".to_string(),
                    serde_json::json!(Utc::now().to_rfc3339()));
                Some(metadata)
            },
        };

        self.ctx.session_service.update_session(&input.session_id, update).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to update session: {}", e)))?;

        let output = SessionAbandonOutput {
            session_id: input.session_id,
            abandoned: true,
        };

        info!("Session abandoned successfully");
        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

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

pub struct AgentRegisterTool {
    ctx: MultiAgentContext,
}

impl AgentRegisterTool {
    pub fn new(ctx: MultiAgentContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for AgentRegisterTool {
    fn name(&self) -> &str {
        "cortex.agent.register"
    }

    fn description(&self) -> Option<&str> {
        Some("Register an agent")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AgentRegisterInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AgentRegisterInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Registering agent: {} (type: {})", input.agent_id, input.agent_type);

        let conn = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to acquire connection: {}", e)))?;

        // Check if agent already exists
        let check_query = "SELECT * FROM agent WHERE agent_id = $agent_id";
        let mut result = conn.connection()
            .query(check_query)
            .bind(("agent_id", input.agent_id.clone()))
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to check agent: {}", e)))?;

        let existing: Vec<serde_json::Value> = result.take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse result: {}", e)))?;

        if !existing.is_empty() {
            debug!("Agent {} already registered, updating", input.agent_id);
        }

        // Create or update agent registration
        let agent_record = serde_json::json!({
            "agent_id": input.agent_id,
            "agent_type": input.agent_type,
            "capabilities": input.capabilities.unwrap_or_default(),
            "registered_at": Utc::now().to_rfc3339(),
            "last_seen": Utc::now().to_rfc3339(),
            "status": "active"
        });

        let agent_id = Uuid::new_v4();
        let _: Option<serde_json::Value> = conn.connection()
            .create(("agent", agent_id.to_string()))
            .content(agent_record)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to register agent: {}", e)))?;

        let output = AgentRegisterOutput {
            agent_id: input.agent_id,
            registered: true,
        };

        info!("Agent registered successfully");
        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

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

pub struct AgentSendMessageTool {
    ctx: MultiAgentContext,
}

impl AgentSendMessageTool {
    pub fn new(ctx: MultiAgentContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for AgentSendMessageTool {
    fn name(&self) -> &str {
        "cortex.agent.send_message"
    }

    fn description(&self) -> Option<&str> {
        Some("Send message to another agent")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AgentSendMessageInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AgentSendMessageInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Sending {} message to agent: {}", input.message_type, input.to_agent);

        let conn = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to acquire connection: {}", e)))?;

        // Verify target agent exists
        let check_query = "SELECT * FROM agent WHERE agent_id = $agent_id";
        let mut result = conn.connection()
            .query(check_query)
            .bind(("agent_id", input.to_agent.clone()))
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to check agent: {}", e)))?;

        let existing: Vec<serde_json::Value> = result.take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse result: {}", e)))?;

        if existing.is_empty() {
            return Err(ToolError::ExecutionFailed(format!("Target agent not found: {}", input.to_agent)));
        }

        // Create message record
        let message_id = Uuid::new_v4();
        let message_record = serde_json::json!({
            "message_id": message_id.to_string(),
            "to_agent": input.to_agent,
            "from_agent": "system", // In a real implementation, this would come from context
            "message_type": input.message_type,
            "content": input.content,
            "timestamp": Utc::now().to_rfc3339(),
            "read": false,
            "delivered": true
        });

        let _: Option<serde_json::Value> = conn.connection()
            .create(("agent_message", message_id.to_string()))
            .content(message_record)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to send message: {}", e)))?;

        let output = AgentSendMessageOutput {
            message_id: message_id.to_string(),
            sent: true,
        };

        info!("Message sent to agent {}: {}", input.to_agent, message_id);
        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

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

pub struct AgentGetMessagesTool {
    ctx: MultiAgentContext,
}

impl AgentGetMessagesTool {
    pub fn new(ctx: MultiAgentContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for AgentGetMessagesTool {
    fn name(&self) -> &str {
        "cortex.agent.get_messages"
    }

    fn description(&self) -> Option<&str> {
        Some("Retrieve agent messages")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(AgentGetMessagesInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: AgentGetMessagesInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Getting messages for agent: {}", input.agent_id);

        let conn = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to acquire connection: {}", e)))?;

        // Build query based on filters
        let mut query = format!("SELECT * FROM agent_message WHERE to_agent = '{}'", input.agent_id);

        // Add timestamp filter if provided
        if let Some(since) = &input.since {
            query.push_str(&format!(" AND timestamp > '{}'", since));
        }

        // Add message type filter if provided
        if let Some(types) = &input.message_types {
            if !types.is_empty() {
                let types_str = types.iter()
                    .map(|t| format!("'{}'", t))
                    .collect::<Vec<_>>()
                    .join(", ");
                query.push_str(&format!(" AND message_type IN [{}]", types_str));
            }
        }

        query.push_str(" ORDER BY timestamp DESC LIMIT 100");

        let mut result = conn.connection()
            .query(&query)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to query messages: {}", e)))?;

        let messages_raw: Vec<serde_json::Value> = result.take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse messages: {}", e)))?;

        // Convert to AgentMessage structs
        let messages: Vec<AgentMessage> = messages_raw.iter()
            .filter_map(|msg| {
                Some(AgentMessage {
                    message_id: msg.get("message_id")?.as_str()?.to_string(),
                    from_agent: msg.get("from_agent")?.as_str()?.to_string(),
                    message_type: msg.get("message_type")?.as_str()?.to_string(),
                    timestamp: msg.get("timestamp")?.as_str()?.to_string(),
                })
            })
            .collect();

        let output = AgentGetMessagesOutput {
            total_count: messages.len() as i32,
            messages,
        };

        debug!("Retrieved {} messages for agent {}", output.total_count, input.agent_id);
        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

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

pub struct ConflictListTool {
    ctx: MultiAgentContext,
}

impl ConflictListTool {
    pub fn new(ctx: MultiAgentContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for ConflictListTool {
    fn name(&self) -> &str {
        "cortex.conflicts.list"
    }

    fn description(&self) -> Option<&str> {
        Some("List merge conflicts for a session")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ConflictListInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: ConflictListInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        debug!("Listing conflicts for session: {}", input.session_id);

        let conn = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to acquire connection: {}", e)))?;

        // Query conflicts from database
        let query = "SELECT * FROM merge_conflict WHERE session_id = $session_id ORDER BY created_at DESC";
        let mut result = conn.connection()
            .query(query)
            .bind(("session_id", input.session_id.clone()))
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to query conflicts: {}", e)))?;

        let conflicts_raw: Vec<serde_json::Value> = result.take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse conflicts: {}", e)))?;

        // Convert to ConflictDetail structs
        let conflicts: Vec<ConflictDetail> = conflicts_raw.iter()
            .filter_map(|c| {
                Some(ConflictDetail {
                    conflict_id: c.get("conflict_id")?.as_str()?.to_string(),
                    entity_id: c.get("entity_id")?.as_str()?.to_string(),
                    conflict_type: c.get("conflict_type")?.as_str()?.to_string(),
                    file_path: c.get("file_path")?.as_str()?.to_string(),
                    line_range: c.get("line_range")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    base_version: c.get("base_version")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    session_version: c.get("session_version")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    main_version: c.get("main_version")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    suggested_resolution: c.get("suggested_resolution")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                })
            })
            .collect();

        let output = ConflictListOutput {
            session_id: input.session_id,
            total_count: conflicts.len() as i32,
            conflicts,
        };

        debug!("Found {} conflicts for session", output.total_count);
        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

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

pub struct ConflictResolveTool {
    ctx: MultiAgentContext,
}

impl ConflictResolveTool {
    pub fn new(ctx: MultiAgentContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl Tool for ConflictResolveTool {
    fn name(&self) -> &str {
        "cortex.conflicts.resolve"
    }

    fn description(&self) -> Option<&str> {
        Some("Manually resolve a conflict")
    }

    fn input_schema(&self) -> Value {
        serde_json::to_value(schemars::schema_for!(ConflictResolveInput)).unwrap()
    }

    async fn execute(&self, input: Value, _context: &ToolContext) -> std::result::Result<ToolResult, ToolError> {
        let input: ConflictResolveInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        info!("Resolving conflict: {} (apply_to_similar: {})", input.conflict_id, input.apply_to_similar);

        let conn = self.ctx.storage.acquire().await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to acquire connection: {}", e)))?;

        // Get the conflict to resolve
        let query = "SELECT * FROM merge_conflict WHERE conflict_id = $conflict_id";
        let mut result = conn.connection()
            .query(query)
            .bind(("conflict_id", input.conflict_id.clone()))
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to query conflict: {}", e)))?;

        let conflicts: Vec<serde_json::Value> = result.take(0)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse conflict: {}", e)))?;

        if conflicts.is_empty() {
            return Err(ToolError::ExecutionFailed(format!("Conflict not found: {}", input.conflict_id)));
        }

        let conflict = &conflicts[0];
        let session_id = conflict.get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ExecutionFailed("Invalid conflict record".to_string()))?;

        // Store the resolution
        let resolution_id = Uuid::new_v4();
        let resolution_record = serde_json::json!({
            "resolution_id": resolution_id.to_string(),
            "conflict_id": input.conflict_id,
            "session_id": session_id,
            "resolution": input.resolution,
            "resolved_at": Utc::now().to_rfc3339(),
            "resolved_by": "user" // In real implementation, would come from context
        });

        let _: Option<serde_json::Value> = conn.connection()
            .create(("conflict_resolution", resolution_id.to_string()))
            .content(resolution_record)
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to store resolution: {}", e)))?;

        // Mark conflict as resolved
        let update_query = format!(
            "UPDATE merge_conflict:{} SET resolved = true, resolution = $resolution, resolved_at = $timestamp",
            input.conflict_id
        );
        conn.connection()
            .query(&update_query)
            .bind(("resolution", input.resolution.clone()))
            .bind(("timestamp", Utc::now().to_rfc3339()))
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to update conflict: {}", e)))?;

        let mut conflicts_resolved = 1;

        // If apply_to_similar is true, resolve similar conflicts
        if input.apply_to_similar {
            let conflict_type = conflict.get("conflict_type")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let file_path = conflict.get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // Find similar unresolved conflicts
            let similar_query = format!(
                "SELECT * FROM merge_conflict WHERE session_id = '{}' AND conflict_type = '{}' AND file_path = '{}' AND resolved != true AND conflict_id != '{}'",
                session_id, conflict_type, file_path, input.conflict_id
            );

            let mut similar_result = conn.connection()
                .query(&similar_query)
                .await
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to query similar conflicts: {}", e)))?;

            let similar_conflicts: Vec<serde_json::Value> = similar_result.take(0)
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse similar conflicts: {}", e)))?;

            // Resolve each similar conflict
            for similar in &similar_conflicts {
                if let Some(similar_id) = similar.get("conflict_id").and_then(|v| v.as_str()) {
                    let update_similar = format!(
                        "UPDATE merge_conflict:{} SET resolved = true, resolution = $resolution, resolved_at = $timestamp, auto_resolved = true",
                        similar_id
                    );

                    if conn.connection()
                        .query(&update_similar)
                        .bind(("resolution", input.resolution.clone()))
                        .bind(("timestamp", Utc::now().to_rfc3339()))
                        .await
                        .is_ok()
                    {
                        conflicts_resolved += 1;
                        debug!("Auto-resolved similar conflict: {}", similar_id);
                    }
                }
            }
        }

        let output = ConflictResolveOutput {
            conflict_id: input.conflict_id,
            resolved: true,
            conflicts_resolved,
        };

        info!("Conflict resolution complete: {} conflicts resolved", conflicts_resolved);
        Ok(ToolResult::success_json(serde_json::to_value(output).unwrap()))
    }
}

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
fn default_list_limit() -> i32 { 50 }
