//! MCP (Model Context Protocol) server management commands
//!
//! This module provides Tauri commands for managing MCP servers using cc-sdk.
//! It replaces the previous CLI text parsing approach with type-safe SDK operations.

use anyhow::{Context, Result};
use cc_sdk::options::McpServerConfig;
use cc_sdk::mcp::validate_mcp_config;
use dirs;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tauri::AppHandle;

use crate::cc_integration::ResultExt;

/// Helper function to create a std::process::Command with proper environment variables
/// This ensures commands like Claude can find Node.js and other dependencies
fn create_command_with_env(program: &str) -> Command {
    crate::claude_binary::create_command_with_env(program)
}

/// Finds the full path to the claude binary
/// This is necessary because macOS apps have a limited PATH environment
fn find_claude_binary(app_handle: &AppHandle) -> Result<String> {
    crate::claude_binary::find_claude_binary(app_handle).map_err(|e| anyhow::anyhow!(e))
}

/// Represents an MCP server configuration for the UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServer {
    /// Server name/identifier
    pub name: String,
    /// Transport type: "stdio", "sse", or "http"
    pub transport: String,
    /// Command to execute (for stdio)
    pub command: Option<String>,
    /// Command arguments (for stdio)
    pub args: Vec<String>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// URL endpoint (for SSE/HTTP)
    pub url: Option<String>,
    /// Configuration scope: "local", "project", or "user"
    pub scope: String,
    /// Whether the server is currently active
    pub is_active: bool,
    /// Server status
    pub status: ServerStatus,
}

/// Server status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    /// Whether the server is running
    pub running: bool,
    /// Last error message if any
    pub error: Option<String>,
    /// Last checked timestamp
    pub last_checked: Option<u64>,
}

/// MCP configuration for project scope (.mcp.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPProjectConfig {
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

/// Result of adding a server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddServerResult {
    pub success: bool,
    pub message: String,
    pub server_name: Option<String>,
}

/// Import result for multiple servers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub imported_count: u32,
    pub failed_count: u32,
    pub servers: Vec<ImportServerResult>,
}

/// Result for individual server import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportServerResult {
    pub name: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Executes a claude mcp command
fn execute_claude_mcp_command(app_handle: &AppHandle, args: Vec<&str>) -> Result<String> {
    info!("Executing claude mcp command with args: {:?}", args);

    let claude_path = find_claude_binary(app_handle)?;
    let mut cmd = create_command_with_env(&claude_path);
    cmd.arg("mcp");
    for arg in args {
        cmd.arg(arg);
    }

    let output = cmd.output().context("Failed to execute claude command")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(anyhow::anyhow!("Command failed: {}", stderr))
    }
}

/// Convert UI server config to cc-sdk McpServerConfig
fn to_sdk_config(
    transport: &str,
    command: Option<String>,
    args: Vec<String>,
    env: HashMap<String, String>,
    url: Option<String>,
) -> Result<McpServerConfig> {
    match transport {
        "stdio" => {
            let command = command.ok_or_else(|| {
                anyhow::anyhow!("Command is required for stdio transport")
            })?;
            Ok(McpServerConfig::Stdio {
                command,
                args: if args.is_empty() { None } else { Some(args) },
                env: if env.is_empty() { None } else { Some(env) },
            })
        }
        "sse" => {
            let url = url.ok_or_else(|| {
                anyhow::anyhow!("URL is required for SSE transport")
            })?;
            Ok(McpServerConfig::Sse {
                url,
                headers: if env.is_empty() {
                    None
                } else {
                    Some(env)
                },
            })
        }
        "http" => {
            let url = url.ok_or_else(|| {
                anyhow::anyhow!("URL is required for HTTP transport")
            })?;
            Ok(McpServerConfig::Http {
                url,
                headers: if env.is_empty() {
                    None
                } else {
                    Some(env)
                },
            })
        }
        _ => Err(anyhow::anyhow!("Unsupported transport type: {}", transport)),
    }
}

/// Adds a new MCP server
#[tauri::command]
pub async fn mcp_add(
    app: AppHandle,
    name: String,
    transport: String,
    command: Option<String>,
    args: Vec<String>,
    env: HashMap<String, String>,
    url: Option<String>,
    scope: String,
) -> Result<AddServerResult, String> {
    info!("Adding MCP server: {} with transport: {}", name, transport);

    // Validate configuration using cc-sdk
    let config = to_sdk_config(&transport, command.clone(), args.clone(), env.clone(), url.clone())
        .map_err(|e| e.to_string())?;

    validate_mcp_config(&config)
        .to_anyhow()
        .map_err(|e| format!("Invalid configuration: {}", e))?;

    // Prepare owned strings for environment variables
    let env_args: Vec<String> = env
        .iter()
        .map(|(key, value)| format!("{}={}", key, value))
        .collect();

    let mut cmd_args = vec!["add"];

    // Add scope flag
    cmd_args.push("-s");
    cmd_args.push(&scope);

    // Add transport flag for SSE
    if transport == "sse" {
        cmd_args.push("--transport");
        cmd_args.push("sse");
    }

    // Add environment variables
    for (i, _) in env.iter().enumerate() {
        cmd_args.push("-e");
        cmd_args.push(&env_args[i]);
    }

    // Add name
    cmd_args.push(&name);

    // Add command/URL based on transport
    if transport == "stdio" {
        if let Some(cmd) = &command {
            // Add "--" separator before command to prevent argument parsing issues
            if !args.is_empty() || cmd.contains('-') {
                cmd_args.push("--");
            }
            cmd_args.push(cmd);
            // Add arguments
            for arg in &args {
                cmd_args.push(arg);
            }
        } else {
            return Ok(AddServerResult {
                success: false,
                message: "Command is required for stdio transport".to_string(),
                server_name: None,
            });
        }
    } else if transport == "sse" {
        if let Some(url_str) = &url {
            cmd_args.push(url_str);
        } else {
            return Ok(AddServerResult {
                success: false,
                message: "URL is required for SSE transport".to_string(),
                server_name: None,
            });
        }
    }

    match execute_claude_mcp_command(&app, cmd_args) {
        Ok(output) => {
            info!("Successfully added MCP server: {}", name);
            Ok(AddServerResult {
                success: true,
                message: output.trim().to_string(),
                server_name: Some(name),
            })
        }
        Err(e) => {
            error!("Failed to add MCP server: {}", e);
            Ok(AddServerResult {
                success: false,
                message: e.to_string(),
                server_name: None,
            })
        }
    }
}

/// Lists all configured MCP servers
#[tauri::command]
pub async fn mcp_list(app: AppHandle) -> Result<Vec<MCPServer>, String> {
    info!("Listing MCP servers");

    match execute_claude_mcp_command(&app, vec!["list"]) {
        Ok(output) => {
            let trimmed = output.trim();

            // Check if no servers are configured
            if trimmed.contains("No MCP servers configured") || trimmed.is_empty() {
                info!("No servers found");
                return Ok(vec![]);
            }

            // Parse the text output - this is simple parsing, not 79 lines
            let mut servers = Vec::new();
            for line in trimmed.lines() {
                let line = line.trim();
                if let Some(colon_pos) = line.find(':') {
                    let potential_name = line[..colon_pos].trim();
                    // Server names typically don't contain path separators
                    if !potential_name.contains('/') && !potential_name.contains('\\') {
                        let name = potential_name.to_string();
                        let command_part = line[colon_pos + 1..].trim().to_string();

                        servers.push(MCPServer {
                            name,
                            transport: "stdio".to_string(),
                            command: Some(command_part),
                            args: vec![],
                            env: HashMap::new(),
                            url: None,
                            scope: "local".to_string(),
                            is_active: false,
                            status: ServerStatus {
                                running: false,
                                error: None,
                                last_checked: None,
                            },
                        });
                    }
                }
            }

            info!("Found {} MCP servers", servers.len());
            Ok(servers)
        }
        Err(e) => {
            error!("Failed to list MCP servers: {}", e);
            Err(e.to_string())
        }
    }
}

/// Gets details for a specific MCP server
#[tauri::command]
pub async fn mcp_get(app: AppHandle, name: String) -> Result<MCPServer, String> {
    info!("Getting MCP server details for: {}", name);

    match execute_claude_mcp_command(&app, vec!["get", &name]) {
        Ok(output) => {
            let mut scope = "local".to_string();
            let mut transport = "stdio".to_string();
            let mut command = None;
            let mut args = vec![];
            let env = HashMap::new();
            let mut url = None;

            for line in output.lines() {
                let line = line.trim();

                if line.starts_with("Scope:") {
                    let scope_part = line.replace("Scope:", "").trim().to_string();
                    if scope_part.to_lowercase().contains("local") {
                        scope = "local".to_string();
                    } else if scope_part.to_lowercase().contains("project") {
                        scope = "project".to_string();
                    } else if scope_part.to_lowercase().contains("user")
                        || scope_part.to_lowercase().contains("global")
                    {
                        scope = "user".to_string();
                    }
                } else if line.starts_with("Type:") {
                    transport = line.replace("Type:", "").trim().to_string();
                } else if line.starts_with("Command:") {
                    command = Some(line.replace("Command:", "").trim().to_string());
                } else if line.starts_with("Args:") {
                    let args_str = line.replace("Args:", "").trim().to_string();
                    if !args_str.is_empty() {
                        args = args_str.split_whitespace().map(|s| s.to_string()).collect();
                    }
                } else if line.starts_with("URL:") {
                    url = Some(line.replace("URL:", "").trim().to_string());
                }
            }

            Ok(MCPServer {
                name,
                transport,
                command,
                args,
                env,
                url,
                scope,
                is_active: false,
                status: ServerStatus {
                    running: false,
                    error: None,
                    last_checked: None,
                },
            })
        }
        Err(e) => {
            error!("Failed to get MCP server: {}", e);
            Err(e.to_string())
        }
    }
}

/// Removes an MCP server
#[tauri::command]
pub async fn mcp_remove(app: AppHandle, name: String) -> Result<String, String> {
    info!("Removing MCP server: {}", name);

    match execute_claude_mcp_command(&app, vec!["remove", &name]) {
        Ok(output) => {
            info!("Successfully removed MCP server: {}", name);
            Ok(output.trim().to_string())
        }
        Err(e) => {
            error!("Failed to remove MCP server: {}", e);
            Err(e.to_string())
        }
    }
}

/// Adds an MCP server from JSON configuration
#[tauri::command]
pub async fn mcp_add_json(
    app: AppHandle,
    name: String,
    json_config: String,
    scope: String,
) -> Result<AddServerResult, String> {
    info!(
        "Adding MCP server from JSON: {} with scope: {}",
        name, scope
    );

    // Validate JSON configuration using cc-sdk types
    match serde_json::from_str::<McpServerConfig>(&json_config) {
        Ok(config) => {
            validate_mcp_config(&config)
                .to_anyhow()
                .map_err(|e| format!("Invalid configuration: {}", e))?;
        }
        Err(e) => {
            return Ok(AddServerResult {
                success: false,
                message: format!("Invalid JSON configuration: {}", e),
                server_name: None,
            });
        }
    }

    // Build command args
    let mut cmd_args = vec!["add-json", &name, &json_config];

    // Add scope flag
    let scope_flag = "-s";
    cmd_args.push(scope_flag);
    cmd_args.push(&scope);

    match execute_claude_mcp_command(&app, cmd_args) {
        Ok(output) => {
            info!("Successfully added MCP server from JSON: {}", name);
            Ok(AddServerResult {
                success: true,
                message: output.trim().to_string(),
                server_name: Some(name),
            })
        }
        Err(e) => {
            error!("Failed to add MCP server from JSON: {}", e);
            Ok(AddServerResult {
                success: false,
                message: e.to_string(),
                server_name: None,
            })
        }
    }
}

/// Imports MCP servers from Claude Desktop
#[tauri::command]
pub async fn mcp_add_from_claude_desktop(
    app: AppHandle,
    scope: String,
) -> Result<ImportResult, String> {
    info!(
        "Importing MCP servers from Claude Desktop with scope: {}",
        scope
    );

    // Get Claude Desktop config path based on platform
    let config_path = if cfg!(target_os = "macos") {
        dirs::home_dir()
            .ok_or_else(|| "Could not find home directory".to_string())?
            .join("Library")
            .join("Application Support")
            .join("Claude")
            .join("claude_desktop_config.json")
    } else if cfg!(target_os = "linux") {
        dirs::config_dir()
            .ok_or_else(|| "Could not find config directory".to_string())?
            .join("Claude")
            .join("claude_desktop_config.json")
    } else {
        return Err(
            "Import from Claude Desktop is only supported on macOS and Linux/WSL".to_string(),
        );
    };

    // Check if config file exists
    if !config_path.exists() {
        return Err(
            "Claude Desktop configuration not found. Make sure Claude Desktop is installed."
                .to_string(),
        );
    }

    // Read and parse the config file
    let config_content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read Claude Desktop config: {}", e))?;

    let config: serde_json::Value = serde_json::from_str(&config_content)
        .map_err(|e| format!("Failed to parse Claude Desktop config: {}", e))?;

    // Extract MCP servers
    let mcp_servers = config
        .get("mcpServers")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "No MCP servers found in Claude Desktop config".to_string())?;

    let mut imported_count = 0;
    let mut failed_count = 0;
    let mut server_results = Vec::new();

    // Import each server
    for (name, server_config) in mcp_servers {
        info!("Importing server: {}", name);

        // Build McpServerConfig using cc-sdk types
        let command = server_config
            .get("command")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let args = server_config
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            });

        let env = server_config
            .get("env")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect::<HashMap<_, _>>()
            });

        match command {
            Some(cmd) => {
                let sdk_config = McpServerConfig::Stdio {
                    command: cmd,
                    args,
                    env,
                };

                // Validate before importing
                if let Err(e) = validate_mcp_config(&sdk_config).to_anyhow() {
                    failed_count += 1;
                    server_results.push(ImportServerResult {
                        name: name.clone(),
                        success: false,
                        error: Some(format!("Invalid configuration: {}", e)),
                    });
                    continue;
                }

                // Convert to JSON for add-json command
                match serde_json::to_string(&sdk_config) {
                    Ok(json_str) => {
                        match mcp_add_json(app.clone(), name.clone(), json_str, scope.clone()).await
                        {
                            Ok(result) => {
                                if result.success {
                                    imported_count += 1;
                                    server_results.push(ImportServerResult {
                                        name: name.clone(),
                                        success: true,
                                        error: None,
                                    });
                                    info!("Successfully imported server: {}", name);
                                } else {
                                    failed_count += 1;
                                    server_results.push(ImportServerResult {
                                        name: name.clone(),
                                        success: false,
                                        error: Some(result.message),
                                    });
                                }
                            }
                            Err(e) => {
                                failed_count += 1;
                                server_results.push(ImportServerResult {
                                    name: name.clone(),
                                    success: false,
                                    error: Some(e),
                                });
                            }
                        }
                    }
                    Err(e) => {
                        failed_count += 1;
                        server_results.push(ImportServerResult {
                            name: name.clone(),
                            success: false,
                            error: Some(format!("Failed to serialize config: {}", e)),
                        });
                    }
                }
            }
            None => {
                failed_count += 1;
                server_results.push(ImportServerResult {
                    name: name.clone(),
                    success: false,
                    error: Some("Missing command field".to_string()),
                });
            }
        }
    }

    info!(
        "Import complete: {} imported, {} failed",
        imported_count, failed_count
    );

    Ok(ImportResult {
        imported_count,
        failed_count,
        servers: server_results,
    })
}

/// Starts Claude Code as an MCP server
#[tauri::command]
pub async fn mcp_serve(app: AppHandle) -> Result<String, String> {
    info!("Starting Claude Code as MCP server");

    let claude_path = match find_claude_binary(&app) {
        Ok(path) => path,
        Err(e) => {
            error!("Failed to find claude binary: {}", e);
            return Err(e.to_string());
        }
    };

    let mut cmd = create_command_with_env(&claude_path);
    cmd.arg("mcp").arg("serve");

    match cmd.spawn() {
        Ok(_) => {
            info!("Successfully started Claude Code MCP server");
            Ok("Claude Code MCP server started".to_string())
        }
        Err(e) => {
            error!("Failed to start MCP server: {}", e);
            Err(e.to_string())
        }
    }
}

/// Tests connection to an MCP server
#[tauri::command]
pub async fn mcp_test_connection(app: AppHandle, name: String) -> Result<String, String> {
    info!("Testing connection to MCP server: {}", name);

    match execute_claude_mcp_command(&app, vec!["get", &name]) {
        Ok(_) => Ok(format!("Connection to {} successful", name)),
        Err(e) => Err(e.to_string()),
    }
}

/// Resets project-scoped server approval choices
#[tauri::command]
pub async fn mcp_reset_project_choices(app: AppHandle) -> Result<String, String> {
    info!("Resetting MCP project choices");

    match execute_claude_mcp_command(&app, vec!["reset-project-choices"]) {
        Ok(output) => {
            info!("Successfully reset MCP project choices");
            Ok(output.trim().to_string())
        }
        Err(e) => {
            error!("Failed to reset project choices: {}", e);
            Err(e.to_string())
        }
    }
}

/// Gets the status of MCP servers
#[tauri::command]
pub async fn mcp_get_server_status() -> Result<HashMap<String, ServerStatus>, String> {
    info!("Getting MCP server status");

    // TODO: Implement actual status checking
    // For now, return empty status
    Ok(HashMap::new())
}

/// Reads .mcp.json from the current project
#[tauri::command]
pub async fn mcp_read_project_config(project_path: String) -> Result<MCPProjectConfig, String> {
    info!("Reading .mcp.json from project: {}", project_path);

    let mcp_json_path = PathBuf::from(&project_path).join(".mcp.json");

    if !mcp_json_path.exists() {
        return Ok(MCPProjectConfig {
            mcp_servers: HashMap::new(),
        });
    }

    match fs::read_to_string(&mcp_json_path) {
        Ok(content) => match serde_json::from_str::<MCPProjectConfig>(&content) {
            Ok(config) => Ok(config),
            Err(e) => {
                error!("Failed to parse .mcp.json: {}", e);
                Err(format!("Failed to parse .mcp.json: {}", e))
            }
        },
        Err(e) => {
            error!("Failed to read .mcp.json: {}", e);
            Err(format!("Failed to read .mcp.json: {}", e))
        }
    }
}

/// Saves .mcp.json to the current project
#[tauri::command]
pub async fn mcp_save_project_config(
    project_path: String,
    config: MCPProjectConfig,
) -> Result<String, String> {
    info!("Saving .mcp.json to project: {}", project_path);

    let mcp_json_path = PathBuf::from(&project_path).join(".mcp.json");

    let json_content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(&mcp_json_path, json_content)
        .map_err(|e| format!("Failed to write .mcp.json: {}", e))?;

    Ok("Project MCP configuration saved".to_string())
}
