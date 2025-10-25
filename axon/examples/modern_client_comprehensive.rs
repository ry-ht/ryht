//! Comprehensive example of the modern ClaudeClient API.
//!
//! This example demonstrates:
//! - Type-safe state transitions
//! - Binary discovery
//! - Configuration options
//! - Message sending with streaming
//! - Session management
//! - Graceful disconnect/reconnect
//!
//! Run with:
//! ```bash
//! cargo run --example modern_client_comprehensive
//! ```

use cc_sdk::{ClaudeClient, Result, PermissionMode};
use cc_sdk::core::ModelId;
use futures::StreamExt;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("╔════════════════════════════════════════════════════════╗");
    println!("║  Modern ClaudeClient API - Comprehensive Demo         ║");
    println!("║  Phase 3: Type-Safe State Transitions                 ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    // Example 1: Automatic binary discovery
    println!("📦 Example 1: Automatic Binary Discovery");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let client = ClaudeClient::builder()
        .discover_binary().await
        .map_err(|e| {
            eprintln!("Failed to discover Claude binary: {}", e);
            eprintln!("Please install Claude Code: npm install -g @anthropic-ai/claude-code");
            e
        })?
        .model(ModelId::from("claude-sonnet-4-5-20250929"))
        .permission_mode(PermissionMode::AcceptEdits)
        .configure()
        .connect().await?
        .build()?;

    println!("✓ Client created successfully");
    println!("  Session ID: {}", client.session_id());
    println!();

    // Example 2: Sending messages and streaming responses
    println!("💬 Example 2: Sending Messages");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let question = "What are the three laws of robotics?";
    println!("Asking: '{}'", question);

    let mut stream = client.send(question).await?;
    let mut message_count = 0;

    while let Some(msg_result) = stream.next().await {
        match msg_result {
            Ok(msg) => {
                message_count += 1;
                println!("  [{}] Received: {:?}", message_count, msg);
            }
            Err(e) => {
                eprintln!("  ✗ Stream error: {}", e);
            }
        }
    }

    println!("✓ Received {} messages", message_count);
    println!();

    // Example 3: Multiple conversations in same session
    println!("🔄 Example 3: Multiple Messages in Session");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let questions = vec![
        "What is recursion?",
        "Give me an example in Python",
    ];

    for (i, question) in questions.iter().enumerate() {
        println!("Question {}: {}", i + 1, question);
        let mut stream = client.send(*question).await?;

        while let Some(Ok(_msg)) = stream.next().await {
            // Process messages silently for this demo
        }

        println!("  ✓ Response received");
    }
    println!();

    // Example 4: Graceful disconnect
    println!("🔌 Example 4: Graceful Disconnect");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let disconnected = client.disconnect().await?;
    println!("✓ Client disconnected successfully");
    println!();

    // Example 5: Reconnection (if needed)
    println!("🔄 Example 5: Reconnection");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let reconnected = disconnected.reconnect().await?;
    println!("✓ Client reconnected successfully");

    // Clean up
    let _ = reconnected.disconnect().await;
    println!();

    // Example 6: Custom binary path (if needed)
    println!("🛠️  Example 6: Custom Binary Path");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if let Ok(custom_path) = env::var("CLAUDE_BINARY_PATH") {
        println!("Using custom path: {}", custom_path);
        let _custom_client = ClaudeClient::builder()
            .binary(custom_path.as_str())
            .configure()
            .connect().await?
            .build()?;
        println!("✓ Client created with custom binary");
    } else {
        println!("(Skipped - no CLAUDE_BINARY_PATH set)");
    }
    println!();

    println!("╔════════════════════════════════════════════════════════╗");
    println!("║  ✓ All Examples Completed Successfully                ║");
    println!("╚════════════════════════════════════════════════════════╝");

    Ok(())
}
