use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, BufRead, Write};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 Error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    pub fn parse_error(message: String) -> Self {
        Self {
            code: -32700,
            message,
            data: None,
        }
    }

    pub fn invalid_request(message: String) -> Self {
        Self {
            code: -32600,
            message,
            data: None,
        }
    }

    pub fn method_not_found(message: String) -> Self {
        Self {
            code: -32601,
            message,
            data: None,
        }
    }

    pub fn invalid_params(message: String) -> Self {
        Self {
            code: -32602,
            message,
            data: None,
        }
    }

    pub fn internal_error(message: String) -> Self {
        Self {
            code: -32603,
            message,
            data: None,
        }
    }

    pub fn server_error(code: i32, message: String) -> Self {
        Self {
            code,
            message,
            data: None,
        }
    }
}

impl JsonRpcResponse {
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<Value>, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }

    pub fn from_error(id: Option<Value>, err: anyhow::Error) -> Self {
        Self::error(
            id,
            JsonRpcError::internal_error(err.to_string()),
        )
    }
}

/// Stdio transport for MCP communication
pub struct StdioTransport {
    tx: mpsc::UnboundedSender<JsonRpcResponse>,
    rx: mpsc::UnboundedReceiver<JsonRpcRequest>,
}

impl StdioTransport {
    /// Create a new stdio transport
    pub fn new() -> Self {
        let (tx, mut response_rx) = mpsc::unbounded_channel::<JsonRpcResponse>();
        let (request_tx, rx) = mpsc::unbounded_channel::<JsonRpcRequest>();

        // Spawn reader task
        tokio::spawn(async move {
            let stdin = tokio::io::stdin();
            let mut reader = BufReader::new(stdin);
            let mut line = String::new();

            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => {
                        // EOF - stdin closed
                        debug!("stdin closed, exiting reader task");
                        break;
                    }
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }

                        debug!("Received: {}", trimmed);

                        match serde_json::from_str::<JsonRpcRequest>(trimmed) {
                            Ok(request) => {
                                if let Err(e) = request_tx.send(request) {
                                    error!("Failed to send request to handler: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse JSON-RPC request: {}", e);
                                // We can't send error response without request ID
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to read from stdin: {}", e);
                        break;
                    }
                }
            }
        });

        // Spawn writer task
        tokio::spawn(async move {
            let mut stdout = tokio::io::stdout();

            while let Some(response) = response_rx.recv().await {
                match serde_json::to_string(&response) {
                    Ok(json) => {
                        let output = format!("{}\n", json);
                        debug!("Sending: {}", output.trim());

                        if let Err(e) = stdout.write_all(output.as_bytes()).await {
                            error!("Failed to write to stdout: {}", e);
                            break;
                        }

                        if let Err(e) = stdout.flush().await {
                            error!("Failed to flush stdout: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Failed to serialize JSON-RPC response: {}", e);
                    }
                }
            }
        });

        Self { tx, rx }
    }

    /// Receive next request
    pub async fn recv(&mut self) -> Option<JsonRpcRequest> {
        self.rx.recv().await
    }

    /// Send response
    pub fn send(&self, response: JsonRpcResponse) -> Result<()> {
        self.tx
            .send(response)
            .map_err(|e| anyhow::anyhow!("Failed to send response: {}", e))
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

/// Synchronous stdio transport for simpler use cases
pub struct SyncStdioTransport;

impl SyncStdioTransport {
    /// Run the transport loop with a request handler
    pub fn run<F>(mut handler: F) -> Result<()>
    where
        F: FnMut(JsonRpcRequest) -> Result<JsonRpcResponse>,
    {
        info!("Starting synchronous stdio transport");

        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut stdout = stdout.lock();

        for line in stdin.lock().lines() {
            let line = line?;
            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            debug!("Received: {}", trimmed);

            let response = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
                Ok(request) => handler(request).unwrap_or_else(|e| {
                    JsonRpcResponse::error(
                        None,
                        JsonRpcError::internal_error(e.to_string()),
                    )
                }),
                Err(e) => {
                    warn!("Failed to parse JSON-RPC request: {}", e);
                    JsonRpcResponse::error(
                        None,
                        JsonRpcError::parse_error(e.to_string()),
                    )
                }
            };

            let json = serde_json::to_string(&response)?;
            writeln!(stdout, "{}", json)?;
            stdout.flush()?;

            debug!("Sent: {}", json);
        }

        info!("Stdio transport finished");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsonrpc_request_serialization() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::Number(1.into())),
            method: "test_method".to_string(),
            params: Some(serde_json::json!({"key": "value"})),
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: JsonRpcRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.jsonrpc, "2.0");
        assert_eq!(parsed.method, "test_method");
    }

    #[test]
    fn test_jsonrpc_response_success() {
        let response = JsonRpcResponse::success(
            Some(Value::Number(1.into())),
            serde_json::json!({"result": "success"}),
        );

        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_jsonrpc_response_error() {
        let response = JsonRpcResponse::error(
            Some(Value::Number(1.into())),
            JsonRpcError::invalid_request("Bad request".to_string()),
        );

        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_none());
        assert!(response.error.is_some());

        let error = response.error.unwrap();
        assert_eq!(error.code, -32600);
        assert_eq!(error.message, "Bad request");
    }

    #[test]
    fn test_error_codes() {
        let parse_error = JsonRpcError::parse_error("Parse failed".to_string());
        assert_eq!(parse_error.code, -32700);

        let invalid_request = JsonRpcError::invalid_request("Invalid".to_string());
        assert_eq!(invalid_request.code, -32600);

        let method_not_found = JsonRpcError::method_not_found("Not found".to_string());
        assert_eq!(method_not_found.code, -32601);

        let invalid_params = JsonRpcError::invalid_params("Bad params".to_string());
        assert_eq!(invalid_params.code, -32602);

        let internal_error = JsonRpcError::internal_error("Internal".to_string());
        assert_eq!(internal_error.code, -32603);
    }
}
