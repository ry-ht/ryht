//! Example demonstrating the use of the settings parameter
//!
//! This example shows how to use a custom settings file with Claude Code

use cc_sdk::{ClaudeCodeOptions, Result, query};
use futures::StreamExt;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("cc_sdk=debug")
        .init();

    println!("Testing settings parameter...\n");

    // Get absolute path for settings file
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let settings_path = current_dir.join("examples/claude-settings.json");
    let settings_str = settings_path.to_str().expect("Invalid path");

    println!("Using settings file: {settings_str}");

    // Create options with a custom settings file
    let options = ClaudeCodeOptions::builder()
        .settings(settings_str) // Use absolute path
        .system_prompt("You are a helpful assistant")
        .model("claude-3-opus-20240229")
        .permission_mode(cc_sdk::PermissionMode::AcceptEdits)
        .build();
    println!("Querying Claude Code with custom settings...\n");

    // Make a simple query
    let mut messages = query(
        "What are the benefits of using a settings file in Claude Code?",
        Some(options),
    )
    .await?;

    // Process the response
    while let Some(msg) = messages.next().await {
        match msg? {
            cc_sdk::Message::Assistant { message } => {
                for block in message.content {
                    if let cc_sdk::ContentBlock::Text(text) = block {
                        println!("Claude: {}", text.text);
                    }
                }
            }
            cc_sdk::Message::Result {
                duration_ms,
                total_cost_usd,
                ..
            } => {
                println!("\n---");
                println!("Completed in {duration_ms}ms");
                if let Some(cost) = total_cost_usd {
                    println!("Cost: ${cost:.6}");
                }
            }
            _ => {}
        }
    }

    Ok(())
}
