//! Example demonstrating streaming and metrics capabilities.
//!
//! This example shows how to:
//! - Use JsonlReader to parse JSONL output
//! - Track metrics in real-time with SessionMetrics
//! - Buffer output for later access
//! - Extract session IDs from messages
//! - Calculate costs and token usage

use cc_sdk::{
    metrics::SessionMetrics,
    streaming::{extract_session_id, JsonlReader, OutputBuffer},
    Result,
};
use futures::StreamExt;
use std::sync::Arc;
use tokio::io::BufReader;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Streaming and Metrics Example ===\n");

    // Example 1: Basic JSONL parsing
    example_basic_jsonl_parsing().await?;

    // Example 2: Real-time metrics tracking
    example_realtime_metrics().await?;

    // Example 3: Output buffering
    example_output_buffering();

    // Example 4: Session ID extraction
    example_session_id_extraction().await?;

    // Example 5: Custom pricing
    example_custom_pricing().await?;

    Ok(())
}

async fn example_basic_jsonl_parsing() -> Result<()> {
    println!("--- Example 1: Basic JSONL Parsing ---");

    let sample_output = r#"{"type":"user","message":{"content":"Hello, Claude!"}}
{"type":"assistant","message":{"content":[{"text":"Hello! How can I help you today?"}]}}
{"type":"result","session_id":"demo-123","subtype":"done","duration_ms":1500,"is_error":false,"num_turns":1}
"#;

    let reader = BufReader::new(sample_output.as_bytes());
    let mut jsonl_reader = JsonlReader::new(reader);

    let mut count = 0;
    while let Some(result) = jsonl_reader.next().await {
        let message = result?;
        count += 1;
        println!("  Message {}: {:?}", count, message);
    }

    println!("  Total messages parsed: {}\n", count);
    Ok(())
}

async fn example_realtime_metrics() -> Result<()> {
    println!("--- Example 2: Real-time Metrics Tracking ---");

    let session_data = r#"{"type":"user","message":{"content":"Calculate 2+2"}}
{"type":"assistant","message":{"content":[{"text":"2 + 2 = 4"}]}}
{"type":"result","duration_ms":2000,"usage":{"input_tokens":50,"output_tokens":30},"is_error":false,"num_turns":1,"session_id":"calc-456"}
"#;

    let reader = BufReader::new(session_data.as_bytes());
    let mut metrics_stream = SessionMetrics::from_jsonl_stream(reader);

    println!("  Tracking metrics in real-time:");
    while let Some(metrics) = metrics_stream.next().await {
        println!("    Messages: {}", metrics.message_count);
        println!("    User messages: {}", metrics.user_message_count);
        println!("    Assistant messages: {}", metrics.assistant_message_count);
        if let Some(tokens) = metrics.total_tokens {
            println!("    Total tokens: {}", tokens);
        }
        if let Some(cost) = metrics.cost_usd {
            println!("    Estimated cost: ${:.6}", cost);
        }
        if let Some(duration) = metrics.duration_ms {
            println!("    Duration: {}ms", duration);
        }
        println!();
    }

    Ok(())
}

fn example_output_buffering() {
    println!("--- Example 3: Output Buffering ---");

    let buffer = Arc::new(OutputBuffer::new());

    // Simulate adding output lines
    buffer.push("Starting session...");
    buffer.push("User: What is the capital of France?");
    buffer.push("Assistant: The capital of France is Paris.");
    buffer.push("error: Something went wrong");
    buffer.push("Session completed successfully");

    println!("  Total lines buffered: {}", buffer.len());

    // Get all lines
    println!("\n  All buffered output:");
    for line in buffer.get_all() {
        println!("    {}", line);
    }

    // Get last N lines
    println!("\n  Last 2 lines:");
    for line in buffer.get_last(2) {
        println!("    {}", line);
    }

    // Filter lines
    println!("\n  Error lines:");
    for line in buffer.filter(|l| l.contains("error")) {
        println!("    {}", line);
    }

    println!();
}

async fn example_session_id_extraction() -> Result<()> {
    println!("--- Example 4: Session ID Extraction ---");

    let messages = r#"{"type":"system","subtype":"init","data":{"session_id":"init-session-789"}}
{"type":"user","message":{"content":"Test"}}
{"type":"result","session_id":"result-session-789","duration_ms":1000,"is_error":false,"num_turns":1}
"#;

    let reader = BufReader::new(messages.as_bytes());
    let mut jsonl_reader = JsonlReader::new(reader);

    while let Some(result) = jsonl_reader.next().await {
        let message = result?;
        if let Some(session_id) = extract_session_id(&message) {
            println!("  Found session ID: {}", session_id);
        }
    }

    println!();
    Ok(())
}

async fn example_custom_pricing() -> Result<()> {
    println!("--- Example 5: Custom Pricing ---");

    let usage_data = r#"{"type":"result","usage":{"input_tokens":1000000,"output_tokens":500000},"is_error":false}
"#;

    // Default pricing
    let mut metrics_default = SessionMetrics::new();
    metrics_default.update_from_line(usage_data.trim())?;
    println!("  Default pricing ($3/1M input, $15/1M output):");
    println!("    Input tokens: 1,000,000");
    println!("    Output tokens: 500,000");
    println!("    Cost: ${:.2}", metrics_default.cost_usd.unwrap_or(0.0));

    // Custom pricing
    let mut metrics_custom = SessionMetrics::with_pricing(1.0, 5.0);
    metrics_custom.update_from_line(usage_data.trim())?;
    println!("\n  Custom pricing ($1/1M input, $5/1M output):");
    println!("    Input tokens: 1,000,000");
    println!("    Output tokens: 500,000");
    println!("    Cost: ${:.2}", metrics_custom.cost_usd.unwrap_or(0.0));

    // Show the difference
    let savings = metrics_default.cost_usd.unwrap_or(0.0) - metrics_custom.cost_usd.unwrap_or(0.0);
    println!("\n  Savings with custom pricing: ${:.2}", savings);

    println!();
    Ok(())
}
