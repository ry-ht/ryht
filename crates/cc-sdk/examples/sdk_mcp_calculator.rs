//! MCP Server Example - Calculator
//!
//! This example demonstrates how to create an in-process MCP server with
//! calculator tools using the Claude Code Rust SDK with the modern mcp-sdk crate.
//!
//! Unlike external MCP servers that require separate processes, this server
//! runs directly within your Rust application, providing better performance
//! and simpler deployment.
//!
//! NOTE: This example has been updated to use the modern mcp-sdk crate
//! instead of the legacy sdk_mcp module.

use cc_sdk::{
    ClaudeCodeOptions, InteractiveClient, Message, Result,
    mcp::{McpServer, Tool, ToolContext, ToolResult, create_sdk_server_config},
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

// Calculator add tool
#[derive(Clone)]
struct AddTool;

#[async_trait]
impl Tool for AddTool {
    fn name(&self) -> &str {
        "add"
    }

    fn description(&self) -> &str {
        "Add two numbers"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "a": {
                    "type": "number",
                    "description": "First number"
                },
                "b": {
                    "type": "number",
                    "description": "Second number"
                }
            },
            "required": ["a", "b"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult, Box<dyn std::error::Error + Send + Sync>> {
        let a = args["a"].as_f64().ok_or("Invalid number a")?;
        let b = args["b"].as_f64().ok_or("Invalid number b")?;
        let result = a + b;
        Ok(ToolResult::text(format!("{a} + {b} = {result}")))
    }
}

// Calculator subtract tool
#[derive(Clone)]
struct SubtractTool;

#[async_trait]
impl Tool for SubtractTool {
    fn name(&self) -> &str {
        "subtract"
    }

    fn description(&self) -> &str {
        "Subtract one number from another"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "a": {
                    "type": "number",
                    "description": "First number"
                },
                "b": {
                    "type": "number",
                    "description": "Second number"
                }
            },
            "required": ["a", "b"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult, Box<dyn std::error::Error + Send + Sync>> {
        let a = args["a"].as_f64().ok_or("Invalid number a")?;
        let b = args["b"].as_f64().ok_or("Invalid number b")?;
        let result = a - b;
        Ok(ToolResult::text(format!("{a} - {b} = {result}")))
    }
}

// Calculator multiply tool
#[derive(Clone)]
struct MultiplyTool;

#[async_trait]
impl Tool for MultiplyTool {
    fn name(&self) -> &str {
        "multiply"
    }

    fn description(&self) -> &str {
        "Multiply two numbers"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "a": {
                    "type": "number",
                    "description": "First number"
                },
                "b": {
                    "type": "number",
                    "description": "Second number"
                }
            },
            "required": ["a", "b"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult, Box<dyn std::error::Error + Send + Sync>> {
        let a = args["a"].as_f64().ok_or("Invalid number a")?;
        let b = args["b"].as_f64().ok_or("Invalid number b")?;
        let result = a * b;
        Ok(ToolResult::text(format!("{a} × {b} = {result}")))
    }
}

// Calculator divide tool
#[derive(Clone)]
struct DivideTool;

#[async_trait]
impl Tool for DivideTool {
    fn name(&self) -> &str {
        "divide"
    }

    fn description(&self) -> &str {
        "Divide one number by another"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "a": {
                    "type": "number",
                    "description": "First number"
                },
                "b": {
                    "type": "number",
                    "description": "Second number"
                }
            },
            "required": ["a", "b"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult, Box<dyn std::error::Error + Send + Sync>> {
        let a = args["a"].as_f64().ok_or("Invalid number a")?;
        let b = args["b"].as_f64().ok_or("Invalid number b")?;
        if b == 0.0 {
            return Err("Division by zero is not allowed".into());
        }
        let result = a / b;
        Ok(ToolResult::text(format!("{a} ÷ {b} = {result}")))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create the calculator MCP server using mcp-sdk
    let calculator = McpServer::builder()
        .name("calculator")
        .version("2.0.0")
        .tool(Arc::new(AddTool))
        .tool(Arc::new(SubtractTool))
        .tool(Arc::new(MultiplyTool))
        .tool(Arc::new(DivideTool))
        .build()
        .map_err(|e| cc_sdk::Error::Config(format!("Failed to build MCP server: {}", e)))?;

    // Convert to config for cc-sdk
    let calc_config = create_sdk_server_config("calculator", Arc::new(calculator));

    // Configure Claude to use the calculator server
    let mut mcp_servers = HashMap::new();
    mcp_servers.insert("calc".to_string(), calc_config);

    let options = ClaudeCodeOptions::builder()
        .mcp_servers(mcp_servers)
        .allowed_tools(vec![
            "mcp__calc__add".to_string(),
            "mcp__calc__subtract".to_string(),
            "mcp__calc__multiply".to_string(),
            "mcp__calc__divide".to_string(),
        ])
        .build();

    // Create interactive client
    let mut client = InteractiveClient::new(options)?;
    client.connect().await?;

    // Example prompts
    let prompts = vec![
        "Calculate 15 + 27",
        "What is 100 divided by 7?",
        "Calculate (12 + 8) * 3 - 10",
    ];

    for prompt in prompts {
        println!("\n{}", "=".repeat(50));
        println!("Prompt: {prompt}");
        println!("{}", "=".repeat(50));

        // Send message and receive response
        let messages = client.send_and_receive(prompt.to_string()).await?;

        for message in messages {
            match message {
                Message::User { .. } => {}
                Message::Assistant { message } => {
                    for content in message.content {
                        match content {
                            cc_sdk::ContentBlock::Text(text) => {
                                println!("Claude: {}", text.text);
                            }
                            cc_sdk::ContentBlock::ToolUse(tool_use) => {
                                println!("Using tool: {}", tool_use.name);
                                println!("  Input: {:?}", tool_use.input);
                            }
                            _ => {}
                        }
                    }
                }
                Message::Result { total_cost_usd, .. } => {
                    if let Some(cost) = total_cost_usd {
                        println!("Cost: ${cost:.6}");
                    }
                }
                _ => {}
            }
        }
    }

    client.disconnect().await?;
    println!("\n✅ MCP Calculator demo completed!");

    Ok(())
}
