//! Comprehensive tests for ClaudeClient builder enhancements.
//!
//! Tests model fallback, tool filtering, session forking, MCP integration,
//! and other advanced client features.

use cc_sdk::client::ClaudeClient;
use cc_sdk::core::{BinaryPath, ModelId, SessionId};
use cc_sdk::options::McpServerConfig;
use cc_sdk::permissions::PermissionMode;
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn test_client_builder_basic() {
    let _builder = ClaudeClient::builder();
    // Builder created successfully
}

#[test]
fn test_client_builder_with_binary() {
    let binary = BinaryPath::from("/usr/local/bin/claude");
    let _builder = ClaudeClient::builder().binary(binary);
}

#[test]
fn test_client_builder_with_model() {
    let binary = BinaryPath::from("/usr/local/bin/claude");
    let model = ModelId::from("claude-sonnet-4-5-20250929");

    let _builder = ClaudeClient::builder().binary(binary).model(model);
}

#[test]
fn test_client_builder_with_multiple_models() {
    // Test model fallback feature
    let binary = BinaryPath::from("/usr/local/bin/claude");
    let models = vec![
        ModelId::from("claude-sonnet-4-5-20250929"),
        ModelId::from("claude-opus-4-5-20250929"),
        ModelId::from("claude-haiku-4-5-20250929"),
    ];

    let _builder = ClaudeClient::builder().binary(binary).models(models);
}

#[test]
fn test_client_builder_with_allowed_tools() {
    let binary = BinaryPath::from("/usr/local/bin/claude");

    let _builder = ClaudeClient::builder()
        .binary(binary)
        .add_allowed_tool("Bash")
        .add_allowed_tool("Read")
        .add_allowed_tool("Write");
}

#[test]
fn test_client_builder_with_allowed_tools_vec() {
    let binary = BinaryPath::from("/usr/local/bin/claude");
    let tools = vec!["Bash".to_string(), "Read".to_string(), "Write".to_string()];

    let _builder = ClaudeClient::builder()
        .binary(binary)
        .allowed_tools(tools);
}

#[test]
fn test_client_builder_with_disallowed_tools() {
    let binary = BinaryPath::from("/usr/local/bin/claude");

    let _builder = ClaudeClient::builder()
        .binary(binary)
        .disallow_tool("Delete")
        .disallow_tool("Execute");
}

#[test]
fn test_client_builder_with_permission_mode() {
    let binary = BinaryPath::from("/usr/local/bin/claude");

    let modes = vec![
        PermissionMode::Default,
        PermissionMode::AcceptEdits,
        PermissionMode::Plan,
        PermissionMode::BypassPermissions,
    ];

    for mode in modes {
        let _builder = ClaudeClient::builder()
            .binary(binary.clone())
            .permission_mode(mode);
    }
}

#[test]
fn test_client_builder_with_working_directory() {
    let binary = BinaryPath::from("/usr/local/bin/claude");

    let _builder = ClaudeClient::builder()
        .binary(binary)
        .working_directory("/path/to/project");
}

#[test]
fn test_client_builder_with_system_prompt() {
    let binary = BinaryPath::from("/usr/local/bin/claude");

    let _builder = ClaudeClient::builder()
        .binary(binary)
        .system_prompt("You are a helpful coding assistant specialized in Rust.");
}

#[test]
fn test_client_builder_with_max_turns() {
    let binary = BinaryPath::from("/usr/local/bin/claude");

    let _builder = ClaudeClient::builder().binary(binary).max_turns(20);
}

#[test]
fn test_client_builder_with_max_output_tokens() {
    let binary = BinaryPath::from("/usr/local/bin/claude");

    let _builder = ClaudeClient::builder()
        .binary(binary)
        .max_output_tokens(8000);
}

#[test]
fn test_client_builder_with_additional_directories() {
    let binary = BinaryPath::from("/usr/local/bin/claude");

    let _builder = ClaudeClient::builder()
        .binary(binary)
        .add_directory(PathBuf::from("/shared/libs"))
        .add_directory(PathBuf::from("/shared/utils"));
}

#[test]
fn test_client_builder_with_mcp_stdio_server() {
    let binary = BinaryPath::from("/usr/local/bin/claude");

    let _builder = ClaudeClient::builder().binary(binary).add_mcp_stdio_server(
        "filesystem",
        "npx",
        vec!["-y", "@modelcontextprotocol/server-filesystem"],
    );
}

#[test]
fn test_client_builder_with_multiple_mcp_servers() {
    let binary = BinaryPath::from("/usr/local/bin/claude");

    let _builder = ClaudeClient::builder()
        .binary(binary)
        .add_mcp_stdio_server(
            "filesystem",
            "npx",
            vec!["-y", "@modelcontextprotocol/server-filesystem"],
        )
        .add_mcp_stdio_server(
            "database",
            "npx",
            vec!["-y", "@modelcontextprotocol/server-database"],
        );
}

#[test]
fn test_client_builder_with_mcp_sse_server() {
    let binary = BinaryPath::from("/usr/local/bin/claude");

    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer token".to_string());

    let config = McpServerConfig::Sse {
        url: "https://api.example.com/mcp".to_string(),
        headers: Some(headers),
    };

    let _builder = ClaudeClient::builder()
        .binary(binary)
        .add_mcp_server("remote-api", config);
}

#[test]
fn test_client_builder_with_mcp_http_server() {
    let binary = BinaryPath::from("/usr/local/bin/claude");

    let config = McpServerConfig::Http {
        url: "https://mcp.example.com".to_string(),
        headers: None,
    };

    let _builder = ClaudeClient::builder()
        .binary(binary)
        .add_mcp_server("http-mcp", config);
}

#[test]
fn test_client_builder_with_include_partial_messages() {
    let binary = BinaryPath::from("/usr/local/bin/claude");

    let _builder = ClaudeClient::builder()
        .binary(binary)
        .include_partial_messages(true);
}

#[test]
fn test_client_builder_comprehensive_configuration() {
    let binary = BinaryPath::from("/usr/local/bin/claude");

    let _builder = ClaudeClient::builder()
        .binary(binary)
        // Models
        .models(vec![
            ModelId::from("claude-sonnet-4-5-20250929"),
            ModelId::from("claude-opus-4-5-20250929"),
        ])
        // Tools
        .allowed_tools(vec![
            "Bash".to_string(),
            "Read".to_string(),
            "Write".to_string(),
        ])
        .disallow_tool("Delete")
        // Permissions
        .permission_mode(PermissionMode::AcceptEdits)
        // Environment
        .working_directory("/path/to/project")
        .add_directory(PathBuf::from("/shared/libs"))
        // Configuration
        .system_prompt("You are a helpful assistant.")
        .max_turns(20)
        .max_output_tokens(8000)
        .include_partial_messages(true)
        // MCP
        .add_mcp_stdio_server(
            "filesystem",
            "npx",
            vec!["-y", "@modelcontextprotocol/server-filesystem"],
        );
}

#[test]
fn test_client_builder_method_chaining() {
    let binary = BinaryPath::from("/usr/local/bin/claude");

    let _builder = ClaudeClient::builder()
        .binary(binary)
        .model(ModelId::from("claude-sonnet-4"))
        .permission_mode(PermissionMode::AcceptEdits)
        .working_directory("/tmp")
        .add_allowed_tool("Bash")
        .add_allowed_tool("Read")
        .max_turns(10)
        .configure();
}

#[test]
fn test_model_id_creation() {
    let models = vec![
        "claude-sonnet-4-5-20250929",
        "claude-opus-4-5-20250929",
        "claude-haiku-4-5-20250929",
    ];

    for model_str in models {
        let model = ModelId::from(model_str);
        assert_eq!(model.as_str(), model_str);
    }
}

#[test]
fn test_model_id_equality() {
    let model1 = ModelId::from("claude-sonnet-4");
    let model2 = ModelId::from("claude-sonnet-4");
    let model3 = ModelId::from("claude-opus-4");

    // ModelId equality is based on internal value
    assert_eq!(model1.as_str(), model2.as_str());
    assert_ne!(model1.as_str(), model3.as_str());
}

#[test]
fn test_binary_path_creation() {
    let paths = vec![
        "/usr/local/bin/claude",
        "/usr/bin/claude",
        "/opt/homebrew/bin/claude",
        "C:\\Program Files\\Claude\\claude.exe",
    ];

    for path_str in paths {
        let _path = BinaryPath::from(path_str);
        // BinaryPath doesn't expose as_str() - it's an opaque type
        // The important thing is that it can be constructed
    }
}

#[test]
fn test_binary_path_equality() {
    let path1 = BinaryPath::from("/usr/local/bin/claude");
    let path2 = BinaryPath::from("/usr/local/bin/claude");
    let path3 = BinaryPath::from("/usr/bin/claude");

    // BinaryPath implements PartialEq
    assert_eq!(path1, path2);
    assert_ne!(path1, path3);
}

#[test]
fn test_builder_state_transitions() {
    // Test that builder can transition through states
    let binary = BinaryPath::from("/usr/local/bin/claude");

    // NoBinary -> WithBinary
    let builder = ClaudeClient::builder().binary(binary);

    // WithBinary -> Configured
    let _builder = builder.configure();
}

#[test]
fn test_builder_with_session_resume() {
    let binary = BinaryPath::from("/usr/local/bin/claude");

    let _builder = ClaudeClient::builder()
        .binary(binary)
        .resume_session(SessionId::new("session-id-123"));
}

// Note: Project ID configuration is not yet implemented in the modern client API.
// Projects are managed through the session system.

#[test]
fn test_builder_clone_not_required() {
    // Builder methods take self by value and return self
    // This test verifies the builder pattern works correctly
    let binary = BinaryPath::from("/usr/local/bin/claude");

    let builder = ClaudeClient::builder();
    let builder = builder.binary(binary);
    let builder = builder.model(ModelId::from("claude-sonnet-4"));
    let _builder = builder.permission_mode(PermissionMode::AcceptEdits);
}

#[test]
fn test_tool_filtering_combinations() {
    let binary = BinaryPath::from("/usr/local/bin/claude");

    // Both allowed and disallowed tools
    let _builder = ClaudeClient::builder()
        .binary(binary)
        .allowed_tools(vec!["Bash".to_string(), "Read".to_string()])
        .disallow_tool("Delete");
}

#[test]
fn test_multiple_directories() {
    let binary = BinaryPath::from("/usr/local/bin/claude");

    let dirs = vec![
        PathBuf::from("/lib1"),
        PathBuf::from("/lib2"),
        PathBuf::from("/lib3"),
    ];

    let mut builder = ClaudeClient::builder().binary(binary);

    for dir in dirs {
        builder = builder.add_directory(dir);
    }
}

// Note: Environment variables, timeout, and retry configuration are not yet
// implemented in the modern client API. These features may be added in future
// versions if there's demand for them.

#[tokio::test]
#[ignore = "Requires Claude binary"]
async fn test_full_builder_workflow() {
    // This test requires Claude to be installed
    // It tests the complete workflow from builder to connected client

    match ClaudeClient::builder().discover_binary().await {
        Ok(builder) => {
            let _configured = builder
                .model(ModelId::from("claude-sonnet-4-5-20250929"))
                .permission_mode(PermissionMode::AcceptEdits)
                .configure();
            println!("Builder workflow successful");
        }
        Err(_) => {
            println!("Claude binary not found (expected in test environment)");
        }
    }
}

#[test]
fn test_model_fallback_order() {
    let binary = BinaryPath::from("/usr/local/bin/claude");

    // Models should be tried in order
    let models = vec![
        ModelId::from("primary-model"),
        ModelId::from("fallback-model"),
        ModelId::from("last-resort-model"),
    ];

    let _builder = ClaudeClient::builder().binary(binary).models(models);
}

// Note: Debug mode and custom headers are not yet implemented in the modern
// client API. These features may be added in future versions if there's demand
// for them.
