//! MCP Integration for Agent Runtime
//!
//! This module provides integration with Cortex via the MCP (Model Context Protocol)
//! stdio mode, allowing agents to execute tools through the Cortex memory system.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;
use tracing::{debug, error, info};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::agents::AgentId;
use super::runtime_config::McpConfig;

/// Result type for MCP operations
pub type Result<T> = std::result::Result<T, McpError>;

/// MCP integration errors
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("Failed to start MCP server: {0}")]
    ServerStartFailed(String),

    #[error("MCP request timeout: {0}")]
    Timeout(String),

    #[error("MCP protocol error: {0}")]
    ProtocolError(String),

    #[error("Tool execution failed: {0}")]
    ToolExecutionFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Server not running")]
    ServerNotRunning,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

/// MCP server instance for an agent
pub struct McpServer {
    /// Agent ID
    agent_id: AgentId,

    /// Server process
    process: Option<Child>,

    /// Request sender
    request_tx: mpsc::UnboundedSender<McpRequest>,

    /// Pending requests
    pending_requests: Arc<RwLock<HashMap<String, tokio::sync::oneshot::Sender<McpResponse>>>>,

    /// Configuration
    config: McpConfig,

    /// Server state
    state: Arc<RwLock<ServerState>>,
}

/// Server state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ServerState {
    Starting,
    Running,
    Stopped,
    Failed,
}

/// MCP JSON-RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
    pub params: JsonValue,
}

/// MCP JSON-RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpErrorObject>,
}

/// MCP error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpErrorObject {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<JsonValue>,
}

/// Tool call request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool name
    pub name: String,

    /// Tool arguments
    pub arguments: JsonValue,
}

/// Tool call result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Success flag
    pub success: bool,

    /// Result content
    pub content: Vec<ContentItem>,

    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Content item in tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentItem {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "resource")]
    Resource {
        uri: String,
        mime_type: Option<String>,
        data: Option<String>,
    },
}

impl McpServer {
    /// Start a new MCP server for an agent
    pub async fn start(agent_id: AgentId, config: McpConfig) -> Result<Self> {
        info!("Starting MCP server for agent: {}", agent_id);

        // Find cortex binary
        let cortex_path = config.cortex_binary_path
            .clone()
            .or_else(|| {
                // Auto-discover cortex binary
                which::which("cortex").ok()
            })
            .ok_or_else(|| McpError::ServerStartFailed(
                "cortex binary not found".to_string()
            ))?;

        debug!("Using cortex binary: {:?}", cortex_path);

        // Build command
        let mut cmd = Command::new(&cortex_path);
        cmd.args(&config.cortex_args);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Spawn process
        let mut child = cmd.spawn()
            .map_err(|e| McpError::ServerStartFailed(e.to_string()))?;

        let pid = child.id();
        info!("MCP server spawned with PID: {}", pid);

        // Setup communication channels
        let (request_tx, mut request_rx) = mpsc::unbounded_channel::<McpRequest>();
        let (response_tx, response_rx) = mpsc::unbounded_channel::<McpResponse>();

        let pending_requests: Arc<RwLock<HashMap<String, tokio::sync::oneshot::Sender<McpResponse>>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let state = Arc::new(RwLock::new(ServerState::Starting));

        // Get stdin/stdout handles
        let mut stdin = child.stdin.take()
            .ok_or_else(|| McpError::ServerStartFailed("Failed to get stdin".to_string()))?;

        let stdout = child.stdout.take()
            .ok_or_else(|| McpError::ServerStartFailed("Failed to get stdout".to_string()))?;

        // Spawn request writer task
        tokio::spawn(async move {
            while let Some(request) = request_rx.recv().await {
                // Serialize and send request
                if let Ok(json) = serde_json::to_string(&request) {
                    debug!("Sending MCP request: {}", json);
                    if let Err(e) = writeln!(stdin, "{}", json) {
                        error!("Failed to write request: {}", e);
                        break;
                    }
                    if let Err(e) = stdin.flush() {
                        error!("Failed to flush stdin: {}", e);
                        break;
                    }
                }
            }
        });

        // Spawn response reader task
        let response_tx_clone = response_tx.clone();
        tokio::task::spawn_blocking(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        debug!("Received MCP response: {}", line);
                        if let Ok(response) = serde_json::from_str::<McpResponse>(&line) {
                            if let Err(e) = response_tx_clone.send(response) {
                                error!("Failed to send response: {}", e);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to read response: {}", e);
                        break;
                    }
                }
            }
        });

        // Spawn response dispatcher task
        let pending_requests_clone = pending_requests.clone();
        tokio::spawn(async move {
            let mut rx = response_rx;
            while let Some(response) = rx.recv().await {
                let mut pending = pending_requests_clone.write().await;
                if let Some(sender) = pending.remove(&response.id) {
                    let _ = sender.send(response);
                }
            }
        });

        // Update state
        *state.write().await = ServerState::Running;

        Ok(Self {
            agent_id,
            process: Some(child),
            request_tx,
            pending_requests,
            config,
            state,
        })
    }

    /// Call a tool via MCP
    pub async fn call_tool(&self, tool_call: ToolCall) -> Result<ToolResult> {
        let state = *self.state.read().await;
        if state != ServerState::Running {
            return Err(McpError::ServerNotRunning);
        }

        let request_id = uuid::Uuid::new_v4().to_string();

        debug!("Calling tool: {} (request: {})", tool_call.name, request_id);

        // Create request
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: request_id.clone(),
            method: "tools/call".to_string(),
            params: serde_json::json!({
                "name": tool_call.name,
                "arguments": tool_call.arguments,
            }),
        };

        // Create response channel
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        // Register pending request
        self.pending_requests.write().await.insert(request_id.clone(), response_tx);

        // Send request
        self.request_tx.send(request)
            .map_err(|e| McpError::ProtocolError(e.to_string()))?;

        // Wait for response with timeout
        let response = timeout(self.config.request_timeout, response_rx)
            .await
            .map_err(|_| McpError::Timeout(format!("Tool call timeout: {}", tool_call.name)))?
            .map_err(|_| McpError::ProtocolError("Response channel closed".to_string()))?;

        // Handle response
        if let Some(error) = response.error {
            return Err(McpError::ToolExecutionFailed(error.message));
        }

        let result = response.result
            .ok_or_else(|| McpError::InvalidResponse("No result in response".to_string()))?;

        // Parse tool result
        let tool_result: ToolResult = serde_json::from_value(result)
            .map_err(|e| McpError::InvalidResponse(e.to_string()))?;

        debug!("Tool call completed: {} (success: {})", tool_call.name, tool_result.success);

        Ok(tool_result)
    }

    /// List available tools
    pub async fn list_tools(&self) -> Result<Vec<ToolInfo>> {
        let state = *self.state.read().await;
        if state != ServerState::Running {
            return Err(McpError::ServerNotRunning);
        }

        let request_id = uuid::Uuid::new_v4().to_string();

        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: request_id.clone(),
            method: "tools/list".to_string(),
            params: serde_json::json!({}),
        };

        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        self.pending_requests.write().await.insert(request_id, response_tx);

        self.request_tx.send(request)
            .map_err(|e| McpError::ProtocolError(e.to_string()))?;

        let response = timeout(self.config.request_timeout, response_rx)
            .await
            .map_err(|_| McpError::Timeout("List tools timeout".to_string()))?
            .map_err(|_| McpError::ProtocolError("Response channel closed".to_string()))?;

        if let Some(error) = response.error {
            return Err(McpError::ProtocolError(error.message));
        }

        let result = response.result
            .ok_or_else(|| McpError::InvalidResponse("No result".to_string()))?;

        let tools: Vec<ToolInfo> = serde_json::from_value(result)
            .map_err(|e| McpError::InvalidResponse(e.to_string()))?;

        Ok(tools)
    }

    /// Shutdown the MCP server
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down MCP server for agent: {}", self.agent_id);

        *self.state.write().await = ServerState::Stopped;

        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
        }

        Ok(())
    }

    /// Check if server is running
    pub async fn is_running(&self) -> bool {
        *self.state.read().await == ServerState::Running
    }
}

impl Drop for McpServer {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
        }
    }
}

/// Tool information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: JsonValue,
}

/// MCP server pool for managing multiple agent servers
pub struct McpServerPool {
    /// Active servers
    servers: Arc<RwLock<HashMap<AgentId, McpServer>>>,

    /// Configuration
    config: McpConfig,
}

impl McpServerPool {
    /// Create a new server pool
    pub fn new(config: McpConfig) -> Self {
        info!("Initializing MCP Server Pool");

        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Get or create server for agent
    pub async fn get_or_create(&self, agent_id: &AgentId) -> Result<()> {
        let mut servers = self.servers.write().await;

        if !servers.contains_key(agent_id) {
            let server = McpServer::start(agent_id.clone(), self.config.clone()).await?;
            servers.insert(agent_id.clone(), server);
        }

        Ok(())
    }

    /// Call tool on agent's server
    pub async fn call_tool(&self, agent_id: &AgentId, tool_call: ToolCall) -> Result<ToolResult> {
        let servers = self.servers.read().await;
        let server = servers.get(agent_id)
            .ok_or(McpError::ServerNotRunning)?;

        server.call_tool(tool_call).await
    }

    /// Shutdown server for agent
    pub async fn shutdown(&self, agent_id: &AgentId) -> Result<()> {
        let mut servers = self.servers.write().await;

        if let Some(mut server) = servers.remove(agent_id) {
            server.shutdown().await?;
        }

        Ok(())
    }

    /// Shutdown all servers
    pub async fn shutdown_all(&self) -> Result<()> {
        let mut servers = self.servers.write().await;

        for (_, mut server) in servers.drain() {
            let _ = server.shutdown().await;
        }

        Ok(())
    }

    /// Get active server count
    pub async fn active_count(&self) -> usize {
        self.servers.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_request_serialization() {
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: "test-123".to_string(),
            method: "tools/call".to_string(),
            params: serde_json::json!({"name": "test_tool"}),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test-123"));
        assert!(json.contains("tools/call"));
    }

    #[test]
    fn test_tool_result_deserialization() {
        let json = r#"{
            "success": true,
            "content": [
                {"type": "text", "text": "Result"}
            ]
        }"#;

        let result: ToolResult = serde_json::from_str(json).unwrap();
        assert!(result.success);
        assert_eq!(result.content.len(), 1);
    }
}
