//! Error Handling Best Practices
//!
//! This example demonstrates comprehensive error handling patterns using the cc-sdk.
//!
//! Key topics covered:
//! - Using the modern Result type and error module
//! - Handling different error variants
//! - Retry strategies for transient errors
//! - Graceful degradation
//! - Custom error mapping
//! - Error logging and debugging
//!
//! Run with:
//! ```bash
//! cargo run --example error_handling
//! ```

use cc_sdk::{
    ClaudeClient, Result,
    error::{SdkError, ClientError, BinaryError},
    PermissionMode,
};
use cc_sdk::core::ModelId;
use futures::StreamExt;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing for better error visibility
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    println!("=== Error Handling Best Practices ===\n");

    // Example 1: Handling binary discovery errors
    demonstrate_binary_errors().await;

    // Example 2: Handling connection errors with retry
    demonstrate_connection_retry().await;

    // Example 3: Handling message streaming errors
    demonstrate_stream_errors().await?;

    // Example 4: Custom error mapping
    demonstrate_error_mapping().await;

    // Example 5: Graceful degradation
    demonstrate_graceful_degradation().await;

    println!("\n=== Error Handling Examples Complete ===");
    Ok(())
}

/// Example 1: Binary discovery error handling
async fn demonstrate_binary_errors() {
    println!("--- Binary Discovery Errors ---");

    // Attempt to use a non-existent binary path
    let result = ClaudeClient::builder()
        .binary("/nonexistent/path/to/claude")
        .configure()
        .connect()
        .await;

    match result {
        Err(SdkError::Binary(BinaryError::NotFound(path))) => {
            println!("✓ Caught binary not found error: {}", path);
            println!("  Recommendation: Install Claude Code or use discover_binary()");
        }
        Err(SdkError::Binary(BinaryError::PermissionDenied(path))) => {
            println!("✓ Caught permission denied error: {}", path);
            println!("  Recommendation: Check file permissions");
        }
        Err(e) => {
            println!("✗ Unexpected error: {}", e);
        }
        Ok(_) => {
            println!("✗ Expected error but got success");
        }
    }

    // Better approach: automatic discovery with fallback
    println!("\n  Better approach: automatic discovery");
    match ClaudeClient::builder()
        .discover_binary()
        .await
    {
        Ok(builder) => {
            println!("✓ Binary discovered successfully");
            // Continue with configuration...
            let _ = builder;
        }
        Err(e) => {
            println!("✗ Discovery failed: {}", e);
            println!("  Fallback: prompt user to install or provide path");
        }
    }

    println!();
}

/// Example 2: Connection retry with exponential backoff
async fn demonstrate_connection_retry() {
    println!("--- Connection Retry Strategy ---");

    let max_retries = 3;
    let mut retry_delay = Duration::from_millis(500);

    for attempt in 1..=max_retries {
        println!("  Attempt {}/{}", attempt, max_retries);

        match ClaudeClient::builder()
            .discover_binary()
            .await
            .and_then(|b| b.configure().connect())
            .await
        {
            Ok(builder) => {
                println!("✓ Connected successfully on attempt {}", attempt);
                let client = builder.build().expect("Failed to build client");
                let _ = client.disconnect().await;
                return;
            }
            Err(SdkError::Client(ClientError::ConnectionFailed(msg))) => {
                if attempt < max_retries {
                    println!("  Connection failed: {}", msg);
                    println!("  Retrying after {:?}...", retry_delay);
                    sleep(retry_delay).await;
                    retry_delay *= 2; // Exponential backoff
                } else {
                    println!("✗ All retry attempts exhausted: {}", msg);
                }
            }
            Err(e) => {
                println!("✗ Non-retryable error: {}", e);
                return;
            }
        }
    }

    println!();
}

/// Example 3: Stream error handling
async fn demonstrate_stream_errors() -> Result<()> {
    println!("--- Stream Error Handling ---");

    let client = ClaudeClient::builder()
        .discover_binary().await?
        .model(ModelId::from("claude-sonnet-4-5-20250929"))
        .permission_mode(PermissionMode::AcceptEdits)
        .configure()
        .connect().await?
        .build()?;

    println!("  Sending message with stream error handling...");
    let mut stream = client.send("What is 2+2?").await?;

    let mut message_count = 0;
    let mut error_count = 0;

    while let Some(message_result) = stream.next().await {
        match message_result {
            Ok(message) => {
                message_count += 1;
                println!("  ✓ Message {}: {:?}", message_count, message);
            }
            Err(SdkError::Client(ClientError::StreamClosed)) => {
                println!("  ⚠ Stream closed unexpectedly");
                error_count += 1;
                break;
            }
            Err(SdkError::Client(ClientError::Timeout)) => {
                println!("  ⚠ Message timeout - continuing...");
                error_count += 1;
                // Could retry or continue
                continue;
            }
            Err(e) => {
                println!("  ✗ Stream error: {}", e);
                error_count += 1;
                // Decide whether to continue or break
                break;
            }
        }
    }

    println!("  Summary: {} messages, {} errors", message_count, error_count);

    client.disconnect().await?;
    println!();
    Ok(())
}

/// Example 4: Custom error mapping
async fn demonstrate_error_mapping() {
    println!("--- Custom Error Mapping ---");

    #[derive(Debug)]
    enum AppError {
        ClaudeUnavailable(String),
        InvalidInput(String),
        RateLimitExceeded,
        Other(String),
    }

    impl From<SdkError> for AppError {
        fn from(err: SdkError) -> Self {
            match err {
                SdkError::Binary(BinaryError::NotFound(_)) => {
                    AppError::ClaudeUnavailable("Claude CLI not installed".to_string())
                }
                SdkError::Client(ClientError::ConnectionFailed(msg)) => {
                    AppError::ClaudeUnavailable(format!("Connection failed: {}", msg))
                }
                SdkError::Client(ClientError::Timeout) => {
                    AppError::RateLimitExceeded
                }
                SdkError::Config(msg) => {
                    AppError::InvalidInput(msg)
                }
                e => AppError::Other(e.to_string()),
            }
        }
    }

    // Simulate error conversion
    let sdk_error = SdkError::Binary(BinaryError::NotFound("/usr/bin/claude".to_string()));
    let app_error: AppError = sdk_error.into();

    match app_error {
        AppError::ClaudeUnavailable(msg) => {
            println!("✓ Mapped to ClaudeUnavailable: {}", msg);
            println!("  User-friendly action: Show installation instructions");
        }
        _ => println!("✗ Unexpected mapping"),
    }

    println!();
}

/// Example 5: Graceful degradation
async fn demonstrate_graceful_degradation() {
    println!("--- Graceful Degradation ---");

    // Try preferred model, fallback to alternatives
    let models = vec![
        "claude-opus-4-1-20250514",      // Preferred
        "claude-sonnet-4-5-20250929",    // Good alternative
        "claude-3-5-sonnet-20241022",    // Fallback
    ];

    let mut client = None;

    for model in models {
        println!("  Trying model: {}", model);

        match ClaudeClient::builder()
            .discover_binary()
            .await
            .and_then(|b| {
                Ok(b.model(ModelId::from(model))
                    .permission_mode(PermissionMode::AcceptEdits)
                    .configure())
            })
            .and_then(|b| b.connect())
            .await
            .and_then(|b| b.build())
        {
            Ok(c) => {
                println!("✓ Successfully connected with model: {}", model);
                client = Some(c);
                break;
            }
            Err(e) => {
                println!("  ✗ Failed with {}: {}", model, e);
                println!("    Trying next model...");
                continue;
            }
        }
    }

    match client {
        Some(c) => {
            println!("\n✓ System operational with degraded service");
            let _ = c.disconnect().await;
        }
        None => {
            println!("\n✗ All fallback options exhausted - service unavailable");
            println!("  Action: Queue request for later or show maintenance message");
        }
    }

    println!();
}
