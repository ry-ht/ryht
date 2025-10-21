//! Demonstration of SDK Control Protocol features
//!
//! This example shows how to use the new control protocol features including:
//! - Permission callbacks
//! - Hook callbacks
//! - SDK MCP servers
//! - Debug stderr output

use cc_sdk::{
    ClaudeCodeOptions, ClaudeSDKClient, Result,
    CanUseTool, HookCallback, HookContext, HookMatcher,
    PermissionResult, PermissionResultAllow, PermissionResultDeny,
    ToolPermissionContext,
};
use async_trait::async_trait;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::io::Write;

/// Custom permission handler
struct MyPermissionHandler;

#[async_trait]
impl CanUseTool for MyPermissionHandler {
    async fn can_use_tool(
        &self,
        tool_name: &str,
        input: &serde_json::Value,
        context: &ToolPermissionContext,
    ) -> PermissionResult {
        println!("🔒 Permission check for tool: {}", tool_name);
        println!("   Input: {:?}", input);
        println!("   Suggestions: {:?}", context.suggestions);
        
        // Example: Deny dangerous commands
        if tool_name == "Bash"
            && let Some(command) = input.get("command").and_then(|v| v.as_str())
                && (command.contains("rm -rf") || command.contains("sudo")) {
                    return PermissionResult::Deny(PermissionResultDeny {
                        message: "Dangerous command blocked".to_string(),
                        interrupt: false,
                    });
                }
        
        // Allow everything else
        PermissionResult::Allow(PermissionResultAllow {
            updated_input: None,
            updated_permissions: None,
        })
    }
}

/// Custom hook handler
struct MyHookHandler {
    name: String,
}

#[async_trait]
impl HookCallback for MyHookHandler {
    async fn execute(
        &self,
        input: &cc_sdk::HookInput,
        tool_use_id: Option<&str>,
        _context: &HookContext,
    ) -> std::result::Result<cc_sdk::HookJSONOutput, cc_sdk::SdkError> {
        println!("🪝 Hook '{}' triggered", self.name);

        // Pattern match on strongly-typed input
        match input {
            cc_sdk::HookInput::PreToolUse(pre_tool_use) => {
                println!("   Tool: {}", pre_tool_use.tool_name);
                println!("   Input: {:?}", pre_tool_use.tool_input);
            }
            cc_sdk::HookInput::PostToolUse(post_tool_use) => {
                println!("   Tool: {}", post_tool_use.tool_name);
                println!("   Response: {:?}", post_tool_use.tool_response);
            }
            cc_sdk::HookInput::UserPromptSubmit(prompt) => {
                println!("   Prompt: {}", prompt.prompt);
            }
            _ => {
                println!("   Other hook event");
            }
        }
        println!("   Tool use ID: {:?}", tool_use_id);

        // Return strongly-typed hook output
        Ok(cc_sdk::HookJSONOutput::Sync(cc_sdk::SyncHookJSONOutput {
            reason: Some(format!("Processed by hook '{}' at {}",
                self.name,
                chrono::Utc::now().to_rfc3339()
            )),
            ..Default::default()
        }))
    }
}

/// Custom debug output writer
struct DebugWriter;

impl Write for DebugWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let msg = String::from_utf8_lossy(buf);
        eprintln!("🐛 DEBUG: {}", msg.trim());
        Ok(buf.len())
    }
    
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== SDK Control Protocol Demo ===\n");
    
    // Create options with control protocol features
    let mut options = ClaudeCodeOptions::builder()
        .system_prompt("You are a helpful assistant with restricted permissions")
        .build();
    
    // Add permission handler
    options.can_use_tool = Some(Arc::new(MyPermissionHandler));
    
    // Add hook handlers
    // IMPORTANT: Hook event names must be PascalCase to match CLI expectations
    let mut hooks = std::collections::HashMap::new();
    hooks.insert(
        "PreToolUse".to_string(),  // PascalCase - matches CLI event name
        vec![HookMatcher {
            matcher: Some(serde_json::json!({ "tool": "*" })),
            hooks: vec![Arc::new(MyHookHandler {
                name: "pre_tool_validator".to_string(),
            })],
        }],
    );
    hooks.insert(
        "PostToolUse".to_string(),  // PascalCase - matches CLI event name
        vec![HookMatcher {
            matcher: Some(serde_json::json!({ "tool": "*" })),
            hooks: vec![Arc::new(MyHookHandler {
                name: "post_tool_logger".to_string(),
            })],
        }],
    );
    options.hooks = Some(hooks);
    
    // Add debug output
    options.debug_stderr = Some(Arc::new(Mutex::new(DebugWriter)));
    
    // Create client with the configured options
    let mut client = ClaudeSDKClient::new(options);
    
    println!("Connecting to Claude CLI with control protocol enabled...");
    match client.connect(None).await {
        Ok(_) => {
            println!("✅ Connected successfully\n");
            
            // Check server info
            if let Some(info) = client.get_server_info().await {
                println!("📋 Server Information:");
                if let Some(model) = info.get("model").and_then(|v| v.as_str()) {
                    println!("   Model: {model}");
                }
                if let Some(tools) = info.get("tools").and_then(|v| v.as_array()) {
                    println!("   Available tools: {} tools", tools.len());
                }
                if let Some(mode) = info.get("permissionMode").and_then(|v| v.as_str()) {
                    println!("   Permission mode: {mode}");
                }
                println!();
            }
            
            // Send a test query that might trigger permission checks
            println!("Sending test query...");
            client.query(
                "Please list the files in the current directory using the ls command".to_string(),
                None
            ).await?;
            
            // Receive response
            println!("\nReceiving response with control protocol active...");
            {
                let mut response = client.receive_response().await;
                let mut message_count = 0;
                
                while let Some(msg_result) = response.next().await {
                    message_count += 1;
                    match msg_result {
                        Ok(msg) => {
                            match msg {
                                cc_sdk::Message::User { .. } => println!("📤 User message"),
                                cc_sdk::Message::Assistant { .. } => println!("🤖 Assistant message"),
                                cc_sdk::Message::System { subtype, .. } => {
                                    println!("⚙️ System: {subtype}");
                                    if subtype.starts_with("sdk_control:") {
                                        println!("   [Control protocol message detected]");
                                    }
                                }
                                cc_sdk::Message::Result { is_error, .. } => {
                                    println!("✓ Result (error: {is_error})");
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ Error: {e}");
                            break;
                        }
                    }
                }
                
                println!("\nTotal messages: {message_count}");
            }
            
            // Test with a potentially dangerous command
            println!("\n--- Testing permission denial ---");
            client.query(
                "Run this command: sudo rm -rf /tmp/test".to_string(),
                None
            ).await?;
            
            {
                let mut response = client.receive_response().await;
                while let Some(msg_result) = response.next().await {
                    if let Ok(cc_sdk::Message::Result { .. }) = msg_result {
                        break;
                    }
                }
            }
            
            client.disconnect().await?;
            println!("\n✅ Disconnected successfully");
        }
        Err(e) => {
            eprintln!("❌ Failed to connect: {e}");
        }
    }
    
    println!("\n=== Demo Complete ===");
    Ok(())
}