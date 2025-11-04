//! Orchestrate Tool - Orchestrate a complex task across multiple specialized agents
//!
//! This tool implements the Orchestrator-Worker pattern from Anthropic's multi-agent
//! research system architecture. It analyzes task complexity, spawns appropriate workers,
//! and synthesizes results.
//!
//! Direct integration with Cortex subsystems (no HTTP bridge).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error};
use mcp_sdk::prelude::*;
use async_trait::async_trait;

use crate::cortex_bridge::{CortexBridge, WorkspaceId, SessionId};
use crate::orchestration::{
    LeadAgent,
    LeadAgentConfig,
    StrategyLibrary,
    StrategyLibraryConfig,
    WorkerRegistry,
    WorkerRegistryConfig,
    ResultSynthesizer,
    SynthesizerConfig,
    ExecutionState,
};
use crate::coordination::{
    UnifiedMessageBus,
    MessageCoordinator,
};
use crate::mcp::tools::agent_registry::AgentRegistry;
use cortex_storage::ConnectionManager;
use cortex_vfs::VirtualFileSystem;
use cortex_memory::SemanticMemorySystem;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct OrchestrateInput {
    pub task: String,
    pub workspace_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OrchestrateOutput {
    pub task_id: String,
    pub status: String,
    pub message: String,
    pub worker_count: Option<usize>,
    pub complexity: Option<String>,
    pub estimated_duration: Option<u64>,
}

/// Context for orchestration with direct subsystem references
#[derive(Clone)]
pub struct OrchestrateContext {
    /// Agent registry for tracking executions
    pub registry: Arc<AgentRegistry>,
    /// Virtual filesystem for file access
    pub vfs: Arc<VirtualFileSystem>,
    /// Semantic memory system
    pub memory: Arc<SemanticMemorySystem>,
    /// Storage backend
    pub storage: Arc<ConnectionManager>,
    /// Legacy bridge for orchestration (temporary)
    pub cortex: Arc<CortexBridge>,
}

impl OrchestrateContext {
    /// Create a new OrchestrateContext
    pub fn new(
        registry: Arc<AgentRegistry>,
        vfs: Arc<VirtualFileSystem>,
        memory: Arc<SemanticMemorySystem>,
        storage: Arc<ConnectionManager>,
        cortex: Arc<CortexBridge>,
    ) -> Self {
        Self {
            registry,
            vfs,
            memory,
            storage,
            cortex,
        }
    }
}

/// Orchestrate tool for multi-agent task coordination
pub struct OrchestrateTool {
    /// Context with direct subsystem references
    context: OrchestrateContext,

    /// Lead agent for orchestration
    lead_agent: Arc<RwLock<Option<LeadAgent>>>,

    /// Strategy library
    strategy_library: Arc<StrategyLibrary>,

    /// Worker registry
    worker_registry: Arc<RwLock<WorkerRegistry>>,

    /// Result synthesizer
    result_synthesizer: Arc<ResultSynthesizer>,

    /// Message bus
    message_bus: Arc<UnifiedMessageBus>,

    /// Message coordinator
    coordinator: Arc<MessageCoordinator>,

    /// Lead agent configuration
    config: LeadAgentConfig,
}

impl OrchestrateTool {
    /// Create a new OrchestrateTool
    pub async fn new(context: OrchestrateContext) -> Result<Self> {
        info!("Initializing OrchestrateTool");

        // Initialize message bus
        let message_bus = Arc::new(UnifiedMessageBus::new());

        // Initialize message coordinator
        let coordinator = Arc::new(MessageCoordinator::new(message_bus.clone(), context.cortex.clone()));

        // Initialize strategy library in lazy mode to avoid hanging on Cortex queries
        // during MCP server initialization. Learned strategies will be loaded on first use.
        let strategy_config = StrategyLibraryConfig::default();
        let strategy_library = Arc::new(StrategyLibrary::new(context.cortex.clone(), strategy_config, true).await?);

        // Initialize worker registry
        let registry_config = WorkerRegistryConfig::default();
        let worker_registry = Arc::new(RwLock::new(WorkerRegistry::new(registry_config)));

        // Initialize result synthesizer
        let synthesizer_config = SynthesizerConfig::default();
        let result_synthesizer = Arc::new(ResultSynthesizer::new(synthesizer_config));

        // Lead agent configuration
        let config = LeadAgentConfig {
            adaptive_allocation: true,
            early_termination: true,
            dynamic_spawning: true,
            max_concurrent_executions: 5,
            default_timeout: std::time::Duration::from_secs(300),
            enable_progress_tracking: true,
        };

        Ok(Self {
            context,
            lead_agent: Arc::new(RwLock::new(None)),
            strategy_library,
            worker_registry,
            result_synthesizer,
            message_bus,
            coordinator,
            config,
        })
    }

    /// Ensure lead agent is initialized
    async fn ensure_lead_agent(&self) -> Result<()> {
        let mut lead_agent_lock = self.lead_agent.write().await;

        if lead_agent_lock.is_none() {
            info!("Initializing LeadAgent for orchestration");

            let lead_agent = LeadAgent::new(
                "MCP-Orchestrator".to_string(),
                self.context.cortex.clone(),
                self.strategy_library.clone(),
                self.worker_registry.clone(),
                self.result_synthesizer.clone(),
                self.message_bus.clone(),
                self.coordinator.clone(),
                self.config.clone(),
            );

            *lead_agent_lock = Some(lead_agent);
        }

        Ok(())
    }

    /// Orchestrate a complex task
    pub async fn orchestrate(&self, input: OrchestrateInput) -> Result<OrchestrateOutput> {
        info!("Orchestrating task: {}", input.task);

        // Ensure Cortex is initialized
        self.context.cortex.ensure_initialized().await?;

        // Load learned strategies now that Cortex is available (only loads once)
        if let Err(e) = self.strategy_library.ensure_learned_strategies_loaded().await {
            // Log error but continue - we can still use default strategies
            tracing::warn!("Failed to load learned strategies: {}", e);
        }

        // Ensure lead agent is initialized
        self.ensure_lead_agent().await?;

        // Get workspace ID (default to "mcp-workspace" if not provided)
        let workspace_id = WorkspaceId::from(
            input.workspace_id
                .clone()
                .unwrap_or_else(|| "mcp-workspace".to_string())
        );

        // Create session ID for this orchestration
        let session_id = SessionId::from(format!("orchestration-{}", uuid::Uuid::new_v4()));

        // Get the lead agent
        let lead_agent_lock = self.lead_agent.read().await;
        let lead_agent = lead_agent_lock.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Lead agent not initialized"))?;

        // Execute orchestration
        match lead_agent.handle_query(&input.task, workspace_id, session_id).await {
            Ok(result) => {
                info!(
                    "Orchestration completed successfully: {} workers, {:.1}% efficiency",
                    result.worker_count,
                    result.parallel_efficiency * 100.0
                );

                Ok(OrchestrateOutput {
                    task_id: uuid::Uuid::new_v4().to_string(),
                    status: "completed".to_string(),
                    message: format!(
                        "Orchestration completed with {} workers. Summary: {}",
                        result.worker_count,
                        truncate_string(&result.summary, 200)
                    ),
                    worker_count: Some(result.worker_count),
                    complexity: Some(format!("{:?}", result.worker_count)),
                    estimated_duration: Some(0), // Would be populated from result
                })
            }
            Err(e) => {
                error!("Orchestration failed: {}", e);

                Ok(OrchestrateOutput {
                    task_id: uuid::Uuid::new_v4().to_string(),
                    status: "failed".to_string(),
                    message: format!("Orchestration failed: {}", e),
                    worker_count: None,
                    complexity: None,
                    estimated_duration: None,
                })
            }
        }
    }

    /// Get execution status for a task
    pub async fn get_status(&self, execution_id: &str) -> Result<Option<ExecutionState>> {
        let lead_agent_lock = self.lead_agent.read().await;

        if let Some(lead_agent) = lead_agent_lock.as_ref() {
            Ok(lead_agent.get_execution_state(execution_id).await)
        } else {
            Ok(None)
        }
    }
}

/// Helper function to truncate strings
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[async_trait]
impl Tool for OrchestrateTool {
    fn name(&self) -> &str {
        "axon.orchestrate"
    }

    fn description(&self) -> Option<&str> {
        Some("Orchestrate a complex task across multiple specialized agents using the Orchestrator-Worker pattern")
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::to_value(schemars::schema_for!(OrchestrateInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input: OrchestrateInput = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let output = self.orchestrate(input).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let json_output = serde_json::to_string_pretty(&output)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult {
            content: vec![ToolContent::text(json_output)],
            is_error: false,
        })
    }
}
