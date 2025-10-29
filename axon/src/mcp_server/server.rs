//! Axon MCP Server Implementation

use super::{AgentRegistry, McpServerConfig};
use crate::cortex_bridge::CortexBridge;
use anyhow::Result;
use std::sync::Arc;

/// Axon MCP Server
pub struct AxonMcpServer {
    config: Arc<McpServerConfig>,
    registry: Arc<AgentRegistry>,
    cortex: Arc<CortexBridge>,
}

impl AxonMcpServer {
    /// Create new Axon MCP server
    pub fn new(config: McpServerConfig, cortex: Arc<CortexBridge>) -> Self {
        Self {
            config: Arc::new(config),
            registry: Arc::new(AgentRegistry::new()),
            cortex,
        }
    }

    /// Get server configuration
    pub fn config(&self) -> &McpServerConfig {
        &self.config
    }

    /// Get agent registry
    pub fn registry(&self) -> &AgentRegistry {
        &self.registry
    }

    /// Get Cortex bridge
    pub fn cortex(&self) -> &CortexBridge {
        &self.cortex
    }

    /// Run MCP server with stdio transport
    pub async fn run(&self) -> Result<()> {
        tracing::info!("Starting Axon MCP server v{}", self.config.version);
        tracing::info!("Cortex URL: {}", self.config.cortex_url);
        tracing::info!("Working directory: {}", self.config.working_dir.display());

        // Build MCP server with all tools registered
        let server = self.build_server().await?;

        // Serve over stdio
        self.serve_stdio(server).await
    }

    /// Build MCP server with all tools registered
    async fn build_server(&self) -> Result<mcp_sdk::McpServer> {
        use super::tools::*;
        use mcp_sdk::prelude::*;

        tracing::info!("Registering Axon MCP tools");

        // Create tool instances with shared context
        let agent_launch = AgentLaunchTool::new(
            self.config.clone(),
            self.registry.clone(),
            self.cortex.clone(),
        );
        let agent_status = AgentStatusTool::new(self.registry.clone());
        let agent_stop = AgentStopTool::new(self.registry.clone());
        let orchestrate = OrchestrateTool;
        let cortex_query = CortexQueryTool;
        let session_create = SessionCreateTool;
        let session_merge = SessionMergeTool;

        // Build server
        let server = mcp_sdk::McpServer::builder()
            .name(&self.config.name)
            .version(&self.config.version)
            .tool(AxonAgentLaunchToolWrapper::new(agent_launch))
            .tool(AxonAgentStatusToolWrapper::new(agent_status))
            .tool(AxonAgentStopToolWrapper::new(agent_stop))
            .tool(AxonOrchestrateToolWrapper::new(orchestrate))
            .tool(AxonCortexQueryToolWrapper::new(cortex_query))
            .tool(AxonSessionCreateToolWrapper::new(session_create))
            .tool(AxonSessionMergeToolWrapper::new(session_merge))
            .build();

        tracing::info!("Registered 7 Axon MCP tools");

        Ok(server)
    }

    /// Serve MCP server over stdio (standard input/output)
    async fn serve_stdio(&self, server: mcp_sdk::McpServer) -> Result<()> {
        use mcp_sdk::prelude::*;

        tracing::info!("Starting Axon MCP Server on stdio");
        let transport = StdioTransport::new();

        // Serve using the SDK's built-in serve method
        server.serve(transport).await
            .map_err(|e| anyhow::anyhow!("Server error: {}", e))
    }

    /// Serve MCP server over HTTP with SSE (optional, for future use)
    #[allow(dead_code)]
    pub async fn serve_http(&self, bind_addr: &str) -> Result<()> {
        #[cfg(feature = "http")]
        {
            tracing::info!("Starting Axon MCP Server on HTTP: {}", bind_addr);
            let addr: std::net::SocketAddr = bind_addr.parse()?;
            let transport = mcp_sdk::transport::HttpTransport::new(addr);

            let server = self.build_server().await?;
            server.serve(transport).await
                .map_err(|e| anyhow::anyhow!("HTTP server error: {}", e))
        }

        #[cfg(not(feature = "http"))]
        {
            tracing::warn!("HTTP transport not enabled. Compile with --features http");
            Err(anyhow::anyhow!("HTTP transport not available"))
        }
    }
}

// =============================================================================
// Tool Wrappers - Implement mcp_sdk::Tool trait for Axon tools
// =============================================================================

use async_trait::async_trait;
use mcp_sdk::prelude::*;
use schemars::JsonSchema;

// Agent Launch Tool Wrapper
pub struct AxonAgentLaunchToolWrapper {
    inner: super::tools::AgentLaunchTool,
}

impl AxonAgentLaunchToolWrapper {
    pub fn new(inner: super::tools::AgentLaunchTool) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl Tool for AxonAgentLaunchToolWrapper {
    fn name(&self) -> &str {
        "axon.agent.launch"
    }

    fn description(&self) -> Option<&str> {
        Some("Launch a specialized agent (developer, tester, reviewer, architect, researcher, optimizer, documenter) to perform a specific task. Returns agent_id for tracking.")
    }

    fn input_schema(&self) -> serde_json::Value {
        use super::tools::agent_launch::AgentLaunchInput;
        serde_json::to_value(schemars::schema_for!(AgentLaunchInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let output = self.inner.launch(input).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let result = serde_json::to_value(output)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success_json(result))
    }
}

// Agent Status Tool Wrapper
pub struct AxonAgentStatusToolWrapper {
    inner: super::tools::AgentStatusTool,
}

impl AxonAgentStatusToolWrapper {
    pub fn new(inner: super::tools::AgentStatusTool) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl Tool for AxonAgentStatusToolWrapper {
    fn name(&self) -> &str {
        "axon.agent.status"
    }

    fn description(&self) -> Option<&str> {
        Some("Check the status of a running agent by agent_id. Returns current status, progress, and results if completed.")
    }

    fn input_schema(&self) -> serde_json::Value {
        use super::tools::agent_status::AgentStatusInput;
        serde_json::to_value(schemars::schema_for!(AgentStatusInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let output = self.inner.check_status(input).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let result = serde_json::to_value(output)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success_json(result))
    }
}

// Agent Stop Tool Wrapper
pub struct AxonAgentStopToolWrapper {
    inner: super::tools::AgentStopTool,
}

impl AxonAgentStopToolWrapper {
    pub fn new(inner: super::tools::AgentStopTool) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl Tool for AxonAgentStopToolWrapper {
    fn name(&self) -> &str {
        "axon.agent.stop"
    }

    fn description(&self) -> Option<&str> {
        Some("Stop a running agent by agent_id. The agent will be cancelled and resources released.")
    }

    fn input_schema(&self) -> serde_json::Value {
        use super::tools::agent_stop::AgentStopInput;
        serde_json::to_value(schemars::schema_for!(AgentStopInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let output = self.inner.stop_agent(input).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let result = serde_json::to_value(output)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success_json(result))
    }
}

// Orchestrate Tool Wrapper
pub struct AxonOrchestrateToolWrapper {
    inner: super::tools::OrchestrateTool,
}

impl AxonOrchestrateToolWrapper {
    pub fn new(inner: super::tools::OrchestrateTool) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl Tool for AxonOrchestrateToolWrapper {
    fn name(&self) -> &str {
        "axon.orchestrate.task"
    }

    fn description(&self) -> Option<&str> {
        Some("Orchestrate a complex task across multiple specialized agents. The orchestrator will decompose the task and coordinate agent execution.")
    }

    fn input_schema(&self) -> serde_json::Value {
        use super::tools::orchestrate::OrchestrateInput;
        serde_json::to_value(schemars::schema_for!(OrchestrateInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let output = self.inner.orchestrate(input).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let result = serde_json::to_value(output)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success_json(result))
    }
}

// Cortex Query Tool Wrapper
pub struct AxonCortexQueryToolWrapper {
    inner: super::tools::CortexQueryTool,
}

impl AxonCortexQueryToolWrapper {
    pub fn new(inner: super::tools::CortexQueryTool) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl Tool for AxonCortexQueryToolWrapper {
    fn name(&self) -> &str {
        "axon.cortex.query"
    }

    fn description(&self) -> Option<&str> {
        Some("Query the Cortex knowledge graph for code, patterns, and semantic information. Supports semantic search across codebases.")
    }

    fn input_schema(&self) -> serde_json::Value {
        use super::tools::cortex_query::CortexQueryInput;
        serde_json::to_value(schemars::schema_for!(CortexQueryInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let output = self.inner.query(input).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let result = serde_json::to_value(output)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success_json(result))
    }
}

// Session Create Tool Wrapper
pub struct AxonSessionCreateToolWrapper {
    inner: super::tools::SessionCreateTool,
}

impl AxonSessionCreateToolWrapper {
    pub fn new(inner: super::tools::SessionCreateTool) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl Tool for AxonSessionCreateToolWrapper {
    fn name(&self) -> &str {
        "axon.session.create"
    }

    fn description(&self) -> Option<&str> {
        Some("Create an isolated work session for experimental changes. Sessions allow agents to work without affecting the main workspace.")
    }

    fn input_schema(&self) -> serde_json::Value {
        use super::tools::session::{SessionCreateInput};
        serde_json::to_value(schemars::schema_for!(SessionCreateInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let output = self.inner.create(input).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let result = serde_json::to_value(output)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success_json(result))
    }
}

// Session Merge Tool Wrapper
pub struct AxonSessionMergeToolWrapper {
    inner: super::tools::SessionMergeTool,
}

impl AxonSessionMergeToolWrapper {
    pub fn new(inner: super::tools::SessionMergeTool) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl Tool for AxonSessionMergeToolWrapper {
    fn name(&self) -> &str {
        "axon.session.merge"
    }

    fn description(&self) -> Option<&str> {
        Some("Merge a session's changes back into the main workspace. Handles conflict detection and resolution.")
    }

    fn input_schema(&self) -> serde_json::Value {
        use super::tools::session::{SessionMergeInput};
        serde_json::to_value(schemars::schema_for!(SessionMergeInput)).unwrap()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> std::result::Result<ToolResult, ToolError> {
        let input = serde_json::from_value(input)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid input: {}", e)))?;

        let output = self.inner.merge(input).await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let result = serde_json::to_value(output)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(ToolResult::success_json(result))
    }
}
