//! Comprehensive tests for options and configuration types.
//!
//! Tests ClaudeCodeOptions, builders, MCP server configs, and control protocol formats.

use cc_sdk::options::{ClaudeCodeOptions, ControlProtocolFormat, McpServerConfig};
use cc_sdk::permissions::PermissionMode;
use std::collections::HashMap;

#[test]
fn test_claude_code_options_builder_basic() {
    let options = ClaudeCodeOptions::builder()
        .model("claude-sonnet-4-5-20250929")
        .permission_mode(PermissionMode::AcceptEdits)
        .cwd("/tmp/test")
        .build();

    assert_eq!(options.model, Some("claude-sonnet-4-5-20250929".to_string()));
    assert_eq!(options.permission_mode, PermissionMode::AcceptEdits);
    assert_eq!(options.cwd, Some("/tmp/test".into()));
}

#[test]
fn test_claude_code_options_builder_default() {
    let options = ClaudeCodeOptions::builder().build();

    assert_eq!(options.model, None);
    assert_eq!(options.permission_mode, PermissionMode::default());
    assert_eq!(options.cwd, None);
    assert_eq!(options.max_turns, None);
    assert_eq!(options.max_output_tokens, None);
}

#[test]
fn test_claude_code_options_builder_with_tools() {
    let options = ClaudeCodeOptions::builder()
        .allow_tool("Bash")
        .allow_tool("Read")
        .allow_tool("Write")
        .build();

    assert_eq!(options.allowed_tools.len(), 3);
    assert!(options.allowed_tools.contains(&"Bash".to_string()));
    assert!(options.allowed_tools.contains(&"Read".to_string()));
    assert!(options.allowed_tools.contains(&"Write".to_string()));
}

#[test]
fn test_claude_code_options_builder_with_disallowed_tools() {
    let options = ClaudeCodeOptions::builder()
        .disallow_tool("Delete")
        .disallow_tool("Execute")
        .build();

    assert_eq!(options.disallowed_tools.len(), 2);
    assert!(options.disallowed_tools.contains(&"Delete".to_string()));
    assert!(options.disallowed_tools.contains(&"Execute".to_string()));
}

#[test]
fn test_claude_code_options_builder_max_values() {
    let options = ClaudeCodeOptions::builder()
        .max_turns(20)
        .max_output_tokens(8000)
        .build();

    assert_eq!(options.max_turns, Some(20));
    assert_eq!(options.max_output_tokens, Some(8000));
}

#[test]
#[allow(deprecated)]
fn test_claude_code_options_builder_system_prompt() {
    let options = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful assistant.")
        .build();

    assert_eq!(
        options.system_prompt,
        Some("You are a helpful assistant.".to_string())
    );
}

#[test]
fn test_claude_code_options_builder_additional_directories() {
    let options = ClaudeCodeOptions::builder()
        .add_dir("/path/to/lib")
        .add_dir("/path/to/src")
        .build();

    assert_eq!(options.add_dirs.len(), 2);
    assert!(options.add_dirs.contains(&"/path/to/lib".into()));
    assert!(options.add_dirs.contains(&"/path/to/src".into()));
}

#[test]
fn test_control_protocol_format_default() {
    let format = ControlProtocolFormat::default();
    assert_eq!(format, ControlProtocolFormat::SdkControlRequest);
}

#[test]
fn test_control_protocol_format_variants() {
    let sdk_control_request = ControlProtocolFormat::SdkControlRequest;
    let control = ControlProtocolFormat::Control;
    let auto = ControlProtocolFormat::Auto;

    assert_eq!(sdk_control_request, ControlProtocolFormat::SdkControlRequest);
    assert_eq!(control, ControlProtocolFormat::Control);
    assert_eq!(auto, ControlProtocolFormat::Auto);

    assert_ne!(sdk_control_request, control);
    assert_ne!(sdk_control_request, auto);
    assert_ne!(control, auto);
}

#[test]
fn test_mcp_server_config_stdio() {
    let config = McpServerConfig::Stdio {
        command: "npx".to_string(),
        args: Some(vec!["-y".to_string(), "@modelcontextprotocol/server-filesystem".to_string()]),
        env: None,
    };

    match config {
        McpServerConfig::Stdio { command, args, env } => {
            assert_eq!(command, "npx");
            assert!(args.is_some());
            assert_eq!(args.unwrap().len(), 2);
            assert!(env.is_none());
        }
        _ => panic!("Expected Stdio config"),
    }
}

#[test]
fn test_mcp_server_config_stdio_with_env() {
    let mut env_vars = HashMap::new();
    env_vars.insert("PATH".to_string(), "/custom/path".to_string());
    env_vars.insert("DEBUG".to_string(), "true".to_string());

    let config = McpServerConfig::Stdio {
        command: "node".to_string(),
        args: Some(vec!["server.js".to_string()]),
        env: Some(env_vars.clone()),
    };

    match config {
        McpServerConfig::Stdio { command, args, env } => {
            assert_eq!(command, "node");
            assert!(env.is_some());
            let env = env.unwrap();
            assert_eq!(env.get("PATH"), Some(&"/custom/path".to_string()));
            assert_eq!(env.get("DEBUG"), Some(&"true".to_string()));
        }
        _ => panic!("Expected Stdio config"),
    }
}

#[test]
fn test_mcp_server_config_sse() {
    let config = McpServerConfig::Sse {
        url: "https://example.com/mcp/events".to_string(),
        headers: None,
    };

    match config {
        McpServerConfig::Sse { url, headers } => {
            assert_eq!(url, "https://example.com/mcp/events");
            assert!(headers.is_none());
        }
        _ => panic!("Expected SSE config"),
    }
}

#[test]
fn test_mcp_server_config_sse_with_headers() {
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer token123".to_string());
    headers.insert("X-API-Key".to_string(), "key456".to_string());

    let config = McpServerConfig::Sse {
        url: "https://api.example.com/mcp".to_string(),
        headers: Some(headers.clone()),
    };

    match config {
        McpServerConfig::Sse { url, headers } => {
            assert!(headers.is_some());
            let headers = headers.unwrap();
            assert_eq!(
                headers.get("Authorization"),
                Some(&"Bearer token123".to_string())
            );
            assert_eq!(headers.get("X-API-Key"), Some(&"key456".to_string()));
        }
        _ => panic!("Expected SSE config"),
    }
}

#[test]
fn test_mcp_server_config_http() {
    let config = McpServerConfig::Http {
        url: "https://mcp.example.com".to_string(),
        headers: None,
    };

    match config {
        McpServerConfig::Http { url, headers } => {
            assert_eq!(url, "https://mcp.example.com");
            assert!(headers.is_none());
        }
        _ => panic!("Expected HTTP config"),
    }
}

#[test]
fn test_mcp_server_config_http_with_headers() {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());

    let config = McpServerConfig::Http {
        url: "https://mcp.example.com/api".to_string(),
        headers: Some(headers),
    };

    match config {
        McpServerConfig::Http { url, headers } => {
            assert!(headers.is_some());
        }
        _ => panic!("Expected HTTP config"),
    }
}

#[test]
fn test_mcp_server_config_debug() {
    let config = McpServerConfig::Stdio {
        command: "test".to_string(),
        args: None,
        env: None,
    };

    let debug_str = format!("{:?}", config);
    assert!(debug_str.contains("Stdio"));
    assert!(debug_str.contains("test"));
}

#[test]
fn test_mcp_server_config_serialization_stdio() {
    let config = McpServerConfig::Stdio {
        command: "npx".to_string(),
        args: Some(vec!["server".to_string()]),
        env: None,
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains(r#""type":"stdio""#));
    assert!(json.contains(r#""command":"npx""#));
}

#[test]
fn test_mcp_server_config_serialization_sse() {
    let config = McpServerConfig::Sse {
        url: "https://example.com".to_string(),
        headers: None,
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains(r#""type":"sse""#));
    assert!(json.contains(r#""url":"https://example.com""#));
}

#[test]
fn test_mcp_server_config_serialization_http() {
    let config = McpServerConfig::Http {
        url: "https://example.com".to_string(),
        headers: None,
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains(r#""type":"http""#));
    assert!(json.contains(r#""url":"https://example.com""#));
}

#[test]
#[allow(deprecated)]
fn test_claude_code_options_builder_chaining() {
    let options = ClaudeCodeOptions::builder()
        .model("claude-sonnet-4")
        .permission_mode(PermissionMode::AcceptEdits)
        .cwd("/tmp")
        .max_turns(10)
        .max_output_tokens(4000)
        .system_prompt("Test prompt")
        .allow_tool("Bash")
        .allow_tool("Read")
        .disallow_tool("Delete")
        .add_dir("/lib")
        .build();

    assert_eq!(options.model, Some("claude-sonnet-4".to_string()));
    assert_eq!(options.permission_mode, PermissionMode::AcceptEdits);
    assert_eq!(options.cwd, Some("/tmp".into()));
    assert_eq!(options.max_turns, Some(10));
    assert_eq!(options.max_output_tokens, Some(4000));
    assert_eq!(options.system_prompt, Some("Test prompt".to_string()));
    assert_eq!(options.allowed_tools.len(), 2);
    assert_eq!(options.disallowed_tools.len(), 1);
    assert_eq!(options.add_dirs.len(), 1);
}

#[test]
fn test_claude_code_options_partial_configuration() {
    // Test that we can build with only some fields set
    let options1 = ClaudeCodeOptions::builder().model("claude-sonnet-4").build();

    assert!(options1.model.is_some());
    assert_eq!(options1.permission_mode, PermissionMode::default());

    let options2 = ClaudeCodeOptions::builder()
        .permission_mode(PermissionMode::Plan)
        .build();

    assert!(options2.model.is_none());
    assert_eq!(options2.permission_mode, PermissionMode::Plan);
}

#[test]
fn test_mcp_server_config_clone() {
    let config = McpServerConfig::Stdio {
        command: "test".to_string(),
        args: None,
        env: None,
    };

    let cloned = config.clone();

    // Both should have same command
    match (config, cloned) {
        (
            McpServerConfig::Stdio { command: c1, .. },
            McpServerConfig::Stdio { command: c2, .. },
        ) => {
            assert_eq!(c1, c2);
        }
        _ => panic!("Expected both to be Stdio"),
    }
}

#[test]
fn test_control_protocol_format_copy() {
    let format1 = ControlProtocolFormat::SdkControlRequest;
    let format2 = format1; // Should copy, not move

    assert_eq!(format1, format2);
    assert_eq!(format1, ControlProtocolFormat::SdkControlRequest); // format1 still usable
}

#[test]
fn test_permission_mode_in_options() {
    let modes = vec![
        PermissionMode::Default,
        PermissionMode::AcceptEdits,
        PermissionMode::Plan,
        PermissionMode::BypassPermissions,
    ];

    for mode in modes {
        let options = ClaudeCodeOptions::builder()
            .permission_mode(mode)
            .build();

        assert_eq!(options.permission_mode, mode);
    }
}

#[test]
#[allow(deprecated)]
fn test_empty_options() {
    let options = ClaudeCodeOptions::builder().build();

    // All fields should be None or empty
    assert!(options.model.is_none());
    assert_eq!(options.permission_mode, PermissionMode::default());
    assert!(options.cwd.is_none());
    assert!(options.max_turns.is_none());
    assert!(options.max_output_tokens.is_none());
    assert!(options.system_prompt.is_none());
    assert!(options.allowed_tools.is_empty());
    assert!(options.disallowed_tools.is_empty());
    assert!(options.add_dirs.is_empty());
}
