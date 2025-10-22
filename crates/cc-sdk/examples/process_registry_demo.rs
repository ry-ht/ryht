//! Demonstration of the ProcessRegistry module for concurrent session tracking.
//!
//! This example shows how to use the ProcessRegistry to track multiple concurrent
//! Claude processes, monitor their output, and manage their lifecycle.

use cc_sdk::process::{ProcessHandle, ProcessRegistry};
use cc_sdk::core::SessionId;
use tokio::process::Command;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== ProcessRegistry Demo ===\n");

    // Create a new process registry
    let registry = ProcessRegistry::new();
    println!("Created process registry\n");

    // Spawn and register multiple processes
    println!("Spawning 3 test processes...");
    let mut session_ids = Vec::new();

    for i in 1..=3 {
        let session_id = SessionId::new(format!("session-{}", i));

        // Spawn a simple process (sleep for demonstration)
        let child = Command::new("sleep")
            .arg(format!("{}", i))
            .spawn()?;

        let handle = registry.register(session_id.clone(), child)?;
        println!("  [{}] Registered with PID: {}", session_id, handle.pid);

        // Append some test output
        handle.append_output(&format!("Output from session {}\n", i))?;

        session_ids.push(session_id);
    }

    println!("\nActive sessions: {:?}", registry.list_active());

    // Get output from a specific session
    println!("\nRetrieving output from session-1:");
    if let Some(output) = registry.get_output(&session_ids[0]) {
        println!("  {}", output);
    }

    // Wait a bit
    println!("\nWaiting 2 seconds...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Clean up finished processes
    let cleaned = registry.cleanup_finished().await;
    println!("Cleaned up {} finished processes", cleaned);
    println!("Remaining active sessions: {:?}", registry.list_active());

    // Demonstrate graceful shutdown
    if let Some(session_id) = registry.list_active().first() {
        println!("\nPerforming graceful shutdown on {}...", session_id);
        registry.kill(session_id, true).await?;
        println!("Successfully killed {}", session_id);
    }

    // Force kill remaining processes
    for session_id in registry.list_active() {
        println!("Force killing {}...", session_id);
        registry.kill(&session_id, false).await?;
    }

    println!("\nFinal active sessions: {:?}", registry.list_active());
    println!("\n=== Demo Complete ===");

    Ok(())
}
