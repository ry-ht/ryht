//! Agent Runtime System
//!
//! This module provides the runtime infrastructure for executing agents in isolated
//! processes with MCP (Model Context Protocol) integration via Cortex.
//!
//! # Architecture
//!
//! The runtime system consists of several key components:
//!
//! - **AgentRuntime**: Main coordinator managing the entire runtime lifecycle
//! - **ProcessManager**: Spawns and monitors agent processes
//! - **McpServerPool**: Manages MCP stdio servers for each agent
//! - **AgentExecutor**: Executes task delegations on agent processes
//! - **RuntimeConfig**: Configuration for all runtime components
//!
//! # Features
//!
//! - **Process Isolation**: Each agent runs in its own process for safety and resource control
//! - **MCP Integration**: Agents communicate with Cortex via MCP stdio protocol
//! - **Resource Management**: CPU, memory, and execution time limits
//! - **Health Monitoring**: Automatic health checks and process recovery
//! - **Graceful Shutdown**: Clean termination with resource cleanup
//! - **Metrics & Telemetry**: Comprehensive statistics and monitoring
//!
//! # Usage
//!
//! ```rust,no_run
//! use axon::runtime::{AgentRuntime, RuntimeConfig};
//! use axon::coordination::UnifiedMessageBus;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create message bus
//!     let message_bus = Arc::new(UnifiedMessageBus::new());
//!
//!     // Create runtime with default config
//!     let config = RuntimeConfig::default();
//!     let runtime = AgentRuntime::new(config, message_bus);
//!
//!     // Start the runtime
//!     runtime.start().await?;
//!
//!     // Spawn an agent
//!     let agent_id = runtime.spawn_agent(
//!         "worker-1".to_string(),
//!         axon::agents::AgentType::Developer,
//!         "cortex",
//!         &["mcp".to_string(), "stdio".to_string()],
//!     ).await?;
//!
//!     // ... use the agent ...
//!
//!     // Shutdown
//!     runtime.shutdown().await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Process Management
//!
//! The runtime manages agent processes with comprehensive lifecycle control:
//!
//! ```rust,no_run
//! # use axon::runtime::AgentRuntime;
//! # use axon::agents::AgentId;
//! # async fn example(runtime: &AgentRuntime, agent_id: &AgentId) {
//! // Check if agent is alive
//! let is_alive = runtime.process_manager.is_alive(agent_id).await;
//!
//! // Get process statistics
//! let stats = runtime.process_manager.get_statistics().await;
//! println!("Active processes: {}", stats.active_processes);
//! # }
//! ```
//!
//! # Task Execution
//!
//! Execute tasks on agents with automatic resource management:
//!
//! ```rust,no_run
//! # use axon::runtime::AgentRuntime;
//! # use axon::agents::AgentId;
//! # use axon::orchestration::task_delegation::TaskDelegation;
//! # async fn example(runtime: &AgentRuntime, agent_id: &AgentId, task: TaskDelegation) {
//! // Execute a single task
//! let result = runtime.execute_task(agent_id, task).await?;
//!
//! // Execute multiple tasks in parallel
//! let tasks = vec![
//!     (agent_id.clone(), task1),
//!     (agent_id.clone(), task2),
//! ];
//! let results = runtime.execute_tasks_parallel(tasks).await;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! # }
//! ```
//!
//! # MCP Integration
//!
//! The runtime automatically manages MCP servers for tool execution:
//!
//! ```rust,no_run
//! # use axon::runtime::mcp_integration::{McpServerPool, ToolCall};
//! # use axon::agents::AgentId;
//! # async fn example(mcp_pool: &McpServerPool, agent_id: &AgentId) {
//! // Call a tool via MCP
//! let tool_call = ToolCall {
//!     name: "cortex_search".to_string(),
//!     arguments: serde_json::json!({"query": "test"}),
//! };
//!
//! let result = mcp_pool.call_tool(agent_id, tool_call).await?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! # }
//! ```

pub mod runtime_config;
pub mod agent_process;
pub mod mcp_integration;
pub mod agent_executor;
pub mod agent_runtime;
pub mod sub_agent_tools;

// Re-export main types
pub use runtime_config::{
    RuntimeConfig,
    ProcessConfig,
    ResourceLimits,
    McpConfig,
    MonitoringConfig,
    RecoveryConfig,
    ExecutionMode,
    RuntimeStatistics as ConfigStatistics,
};

pub use agent_process::{
    AgentProcess,
    ProcessManager,
    ProcessState,
    ResourceUsage,
    ProcessMetadata,
    ProcessManagerStatistics,
    AgentProcessError,
};

pub use mcp_integration::{
    McpServer,
    McpServerPool,
    McpRequest,
    McpResponse,
    McpErrorObject,
    ToolCall,
    ToolResult,
    ToolInfo,
    ContentItem,
    McpError,
};

pub use agent_executor::{
    AgentExecutor,
    ExecutionContext,
    ExecutionStatus,
    ExecutorStatistics,
    ExecutorError,
};

pub use agent_runtime::{
    AgentRuntime,
    AgentInfo,
    AgentStatus,
    RuntimeState,
    RuntimeStatistics,
    RuntimeError,
};

pub use sub_agent_tools::{
    SubAgentManager,
    SubAgentInstance,
    SubAgentStatus,
    SubAgentResult,
    SubAgentMetrics,
    SubAgentMessage,
    MessageType,
    SubAgentInfo,
    LaunchSubAgentTool,
    WaitForSubAgentTool,
    GetSubAgentStatusTool,
    ListSubAgentsTool,
    CancelSubAgentTool,
};

// Re-export result types
pub type Result<T> = std::result::Result<T, RuntimeError>;
