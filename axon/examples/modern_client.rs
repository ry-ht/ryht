//! Example demonstrating the modern ClaudeClient API with type-states.
//!
//! This example shows how to use the Phase 3 modern client API for
//! interacting with Claude Code in a type-safe manner.
//!
//! Run with:
//! ```bash
//! cargo run --example modern_client
//! ```

use cc_sdk::{ClaudeClient, Result, PermissionMode};
use cc_sdk::core::ModelId;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing for better visibility
    tracing_subscriber::fmt::init();

    println!("=== Phase 3: Modern Client API Demo ===\n");

    // Step 1: Build client with type-safe state transitions
    println!("1. Building client...");
    let client = ClaudeClient::builder()
        // NoBinary -> WithBinary (auto-discover Claude installation)
        .discover_binary().await?

        // Configure (stays in WithBinary state)
        .model(ModelId::from("claude-sonnet-4-5-20250929"))
        .permission_mode(PermissionMode::AcceptEdits)
        .add_allowed_tool("Bash")
        .add_allowed_tool("Read")

        // WithBinary -> Configured
        .configure()

        // Configured -> Connected
        .connect().await?

        // Build final client
        .build()?;

    println!("   ✓ Client connected!");
    println!("   Session ID: {}", client.session_id());
    if let Some(model) = client.model() {
        println!("   Model: {}", model);
    }

    // Step 2: Send a simple message
    println!("\n2. Sending message: 'What is 2+2?'");
    let mut stream = client.send("What is 2+2?").await?;

    println!("   Receiving responses:");
    while let Some(message_result) = stream.next().await {
        match message_result {
            Ok(message) => {
                println!("   → Message: {:?}", message);
            }
            Err(e) => {
                eprintln!("   ✗ Error: {}", e);
            }
        }
    }

    // Step 3: Send another message
    println!("\n3. Sending follow-up: 'What is the capital of France?'");
    let mut stream2 = client.send("What is the capital of France?").await?;

    while let Some(message_result) = stream2.next().await {
        match message_result {
            Ok(message) => {
                println!("   → Message: {:?}", message);
            }
            Err(e) => {
                eprintln!("   ✗ Error: {}", e);
            }
        }
    }

    // Step 4: Clean disconnect
    println!("\n4. Disconnecting...");
    let _disconnected = client.disconnect().await?;
    println!("   ✓ Disconnected successfully!");

    // Note: We can reconnect if needed
    // let reconnected = disconnected.reconnect().await?;

    println!("\n=== Demo Complete ===");
    Ok(())
}
