//! MCP Integration Patterns
//!
//! This example demonstrates advanced patterns for integrating MCP servers
//! with the Claude Code SDK.
//!
//! Patterns covered:
//! - Creating custom MCP tools with complex input/output
//! - Multiple MCP servers in one session
//! - Async tool execution with external APIs
//! - Tool with context and state management
//! - Error handling in MCP tools
//! - Tool composition and chaining
//!
//! Run with:
//! ```bash
//! cargo run --example mcp_integration_patterns
//! ```

use cc_sdk::{
    ClaudeCodeOptions, InteractiveClient, Message, Result,
    mcp::{McpServer, Tool, ToolContext, ToolResult, create_sdk_server_config},
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ============================================================================
// Pattern 1: Stateful Tool with Context
// ============================================================================

/// A counter tool that maintains state across invocations
#[derive(Clone)]
struct CounterTool {
    state: Arc<Mutex<HashMap<String, i32>>>,
}

impl CounterTool {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl Tool for CounterTool {
    fn name(&self) -> &str {
        "counter"
    }

    fn description(&self) -> &str {
        "Increment, decrement, or get a named counter"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["increment", "decrement", "get", "reset"],
                    "description": "Operation to perform"
                },
                "counter_name": {
                    "type": "string",
                    "description": "Name of the counter"
                }
            },
            "required": ["operation", "counter_name"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult, Box<dyn std::error::Error + Send + Sync>> {
        let operation = args["operation"].as_str().ok_or("Missing operation")?;
        let counter_name = args["counter_name"].as_str().ok_or("Missing counter_name")?;

        let mut state = self.state.lock().unwrap();
        let counter = state.entry(counter_name.to_string()).or_insert(0);

        let result = match operation {
            "increment" => {
                *counter += 1;
                format!("Counter '{}' incremented to {}", counter_name, counter)
            }
            "decrement" => {
                *counter -= 1;
                format!("Counter '{}' decremented to {}", counter_name, counter)
            }
            "get" => {
                format!("Counter '{}' is at {}", counter_name, counter)
            }
            "reset" => {
                *counter = 0;
                format!("Counter '{}' reset to 0", counter_name)
            }
            _ => return Err("Invalid operation".into()),
        };

        Ok(ToolResult::text(result))
    }
}

// ============================================================================
// Pattern 2: Async Tool with External API
// ============================================================================

/// A weather tool that makes async API calls (simulated)
#[derive(Clone)]
struct WeatherTool;

#[async_trait]
impl Tool for WeatherTool {
    fn name(&self) -> &str {
        "get_weather"
    }

    fn description(&self) -> &str {
        "Get current weather for a location"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City name or location"
                }
            },
            "required": ["location"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult, Box<dyn std::error::Error + Send + Sync>> {
        let location = args["location"].as_str().ok_or("Missing location")?;

        // Simulate async API call
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Simulate weather data
        let weather = json!({
            "location": location,
            "temperature": 72,
            "condition": "Sunny",
            "humidity": 45,
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        Ok(ToolResult::json(weather))
    }
}

// ============================================================================
// Pattern 3: Tool with Complex Input Validation
// ============================================================================

/// A data processor tool with complex input validation
#[derive(Clone)]
struct DataProcessorTool;

#[async_trait]
impl Tool for DataProcessorTool {
    fn name(&self) -> &str {
        "process_data"
    }

    fn description(&self) -> &str {
        "Process an array of numbers with various operations"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "data": {
                    "type": "array",
                    "items": {"type": "number"},
                    "minItems": 1,
                    "description": "Array of numbers to process"
                },
                "operation": {
                    "type": "string",
                    "enum": ["sum", "average", "min", "max", "median"],
                    "description": "Operation to perform"
                }
            },
            "required": ["data", "operation"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult, Box<dyn std::error::Error + Send + Sync>> {
        let data: Vec<f64> = args["data"]
            .as_array()
            .ok_or("Invalid data array")?
            .iter()
            .filter_map(|v| v.as_f64())
            .collect();

        if data.is_empty() {
            return Err("Data array cannot be empty".into());
        }

        let operation = args["operation"].as_str().ok_or("Missing operation")?;

        let result = match operation {
            "sum" => data.iter().sum::<f64>(),
            "average" => data.iter().sum::<f64>() / data.len() as f64,
            "min" => *data.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(),
            "max" => *data.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(),
            "median" => {
                let mut sorted = data.clone();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let mid = sorted.len() / 2;
                if sorted.len() % 2 == 0 {
                    (sorted[mid - 1] + sorted[mid]) / 2.0
                } else {
                    sorted[mid]
                }
            }
            _ => return Err("Invalid operation".into()),
        };

        Ok(ToolResult::json(json!({
            "operation": operation,
            "input_size": data.len(),
            "result": result
        })))
    }
}

// ============================================================================
// Pattern 4: Tool with Structured Output
// ============================================================================

/// A file analyzer tool that returns structured data
#[derive(Clone)]
struct FileAnalyzerTool;

#[async_trait]
impl Tool for FileAnalyzerTool {
    fn name(&self) -> &str {
        "analyze_text"
    }

    fn description(&self) -> &str {
        "Analyze text and return statistics"
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "Text to analyze"
                }
            },
            "required": ["text"]
        })
    }

    async fn execute(&self, args: Value, _ctx: &ToolContext) -> Result<ToolResult, Box<dyn std::error::Error + Send + Sync>> {
        let text = args["text"].as_str().ok_or("Missing text")?;

        let lines = text.lines().count();
        let words = text.split_whitespace().count();
        let chars = text.chars().count();
        let unique_words: std::collections::HashSet<_> =
            text.split_whitespace()
                .map(|w| w.to_lowercase())
                .collect();

        let analysis = json!({
            "lines": lines,
            "words": words,
            "characters": chars,
            "unique_words": unique_words.len(),
            "avg_word_length": if words > 0 { chars as f64 / words as f64 } else { 0.0 }
        });

        Ok(ToolResult::json(analysis))
    }
}

// ============================================================================
// Main Example
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    println!("=== MCP Integration Patterns ===\n");

    // Create utility server
    let utility_server = McpServer::builder()
        .name("utilities")
        .version("1.0.0")
        .tool(Arc::new(CounterTool::new()))
        .tool(Arc::new(DataProcessorTool))
        .tool(Arc::new(FileAnalyzerTool))
        .build()
        .map_err(|e| cc_sdk::Error::Config(format!("Failed to build utility server: {}", e)))?;

    // Create weather server
    let weather_server = McpServer::builder()
        .name("weather")
        .version("1.0.0")
        .tool(Arc::new(WeatherTool))
        .build()
        .map_err(|e| cc_sdk::Error::Config(format!("Failed to build weather server: {}", e)))?;

    // Configure Claude with multiple MCP servers
    let mut mcp_servers = HashMap::new();
    mcp_servers.insert(
        "utilities".to_string(),
        create_sdk_server_config("utilities", Arc::new(utility_server))
    );
    mcp_servers.insert(
        "weather".to_string(),
        create_sdk_server_config("weather", Arc::new(weather_server))
    );

    let options = ClaudeCodeOptions::builder()
        .mcp_servers(mcp_servers)
        .allowed_tools(vec![
            "mcp__utilities__counter".to_string(),
            "mcp__utilities__process_data".to_string(),
            "mcp__utilities__analyze_text".to_string(),
            "mcp__weather__get_weather".to_string(),
        ])
        .build();

    let mut client = InteractiveClient::new(options)?;
    client.connect().await?;

    // Demo: Stateful counter
    println!("--- Pattern 1: Stateful Tool ---");
    demo_conversation(&mut client, "Use the counter tool to increment 'session_count' three times, then get its value").await?;

    // Demo: Async external API
    println!("\n--- Pattern 2: Async External API ---");
    demo_conversation(&mut client, "What's the weather in San Francisco?").await?;

    // Demo: Complex data processing
    println!("\n--- Pattern 3: Complex Input Validation ---");
    demo_conversation(&mut client, "Process this data [5, 2, 8, 1, 9, 3] and calculate the average and median").await?;

    // Demo: Structured output
    println!("\n--- Pattern 4: Structured Output ---");
    demo_conversation(&mut client, "Analyze this text: 'The quick brown fox jumps over the lazy dog'").await?;

    // Demo: Tool composition
    println!("\n--- Pattern 5: Tool Composition ---");
    demo_conversation(&mut client, "Get weather for Paris, analyze the response text, and increment a counter called 'api_calls'").await?;

    client.disconnect().await?;
    println!("\n=== MCP Integration Patterns Complete ===");

    Ok(())
}

async fn demo_conversation(client: &mut InteractiveClient, prompt: &str) -> Result<()> {
    println!("User: {}", prompt);

    let messages = client.send_and_receive(prompt.to_string()).await?;

    for message in messages {
        match message {
            Message::Assistant { message } => {
                for content in message.content {
                    match content {
                        cc_sdk::ContentBlock::Text(text) => {
                            println!("Claude: {}", text.text);
                        }
                        cc_sdk::ContentBlock::ToolUse(tool_use) => {
                            println!("  [Using tool: {}]", tool_use.name);
                        }
                        _ => {}
                    }
                }
            }
            Message::Result { total_cost_usd, .. } => {
                if let Some(cost) = total_cost_usd {
                    println!("  (Cost: ${:.6})", cost);
                }
            }
            _ => {}
        }
    }

    Ok(())
}
