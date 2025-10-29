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

    /// Run MCP server
    pub async fn run(&self) -> Result<()> {
        tracing::info!("Starting Axon MCP server v{}", self.config.version);
        tracing::info!("Cortex URL: {}", self.config.cortex_url);
        tracing::info!("Working directory: {}", self.config.working_dir.display());

        // TODO: Implement actual MCP server using mcp-sdk
        // For now, just keep running
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }
}
