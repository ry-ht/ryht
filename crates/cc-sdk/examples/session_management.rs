//! Example demonstrating advanced session management features.
//!
//! This example shows:
//! - Session discovery with caching
//! - Creating and writing sessions
//! - Filtering and searching sessions
//! - Forking and exporting sessions
//! - Gathering session statistics

use cc_sdk::core::SessionId;
use cc_sdk::session::{
    cache::{CacheConfig, SessionCache},
    filter::{SessionFilter, SortBy},
    management::{export_session, fork_session, get_session_stats, ExportFormat},
    writer::{create_session, CreateSessionOptions},
    *,
};
use chrono::{Duration, Utc};
use std::path::PathBuf;
use std::time::Duration as StdDuration;

#[tokio::main]
async fn main() -> cc_sdk::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Session Management Example ===\n");

    // 1. Configure custom cache
    demonstrate_caching().await?;

    // 2. List projects and sessions
    demonstrate_discovery().await?;

    // 3. Search and filter sessions
    demonstrate_filtering().await?;

    // 4. Create a new session (commented out to avoid modifying real data)
    // demonstrate_write_operations().await?;

    // 5. Session management features (also commented out)
    // demonstrate_management().await?;

    println!("\n=== Example Complete ===");
    Ok(())
}

async fn demonstrate_caching() -> cc_sdk::Result<()> {
    println!("--- Caching ---");

    // Create a custom cache with 10-minute TTL
    let config = CacheConfig {
        ttl: StdDuration::from_secs(600),
        enabled: true,
    };
    let cache = SessionCache::new(config);

    println!("Created cache with 10-minute TTL");
    println!("Cache is empty: {}", cache.is_empty());

    // The global cache is used by default in list_projects() and list_sessions()
    println!("Global cache automatically caches discovery results\n");

    Ok(())
}

async fn demonstrate_discovery() -> cc_sdk::Result<()> {
    println!("--- Discovery ---");

    // List all projects (uses caching)
    let projects = list_projects().await?;
    println!("Found {} projects", projects.len());

    for (i, project) in projects.iter().take(3).enumerate() {
        println!("  {}. {} at {:?}", i + 1, project.id, project.path);

        // List sessions for this project (uses caching)
        let sessions = list_sessions(&project.id).await?;
        println!("     - {} sessions", sessions.len());

        if let Some(session) = sessions.first() {
            println!(
                "       Latest: {} (created: {})",
                session.id.as_str(),
                session.created_at.format("%Y-%m-%d %H:%M")
            );
        }
    }

    println!();
    Ok(())
}

async fn demonstrate_filtering() -> cc_sdk::Result<()> {
    println!("--- Filtering and Search ---");

    // Search for sessions in the last 7 days
    let filter = SessionFilter::default()
        .with_date_range(Some(Utc::now() - Duration::days(7)), Some(Utc::now()))
        .with_sort_by(SortBy::CreatedDesc)
        .with_limit(5);

    println!("Searching for sessions from the last 7 days...");
    let recent_sessions = search_sessions(filter).await?;
    println!("Found {} recent sessions", recent_sessions.len());

    for (i, info) in recent_sessions.iter().enumerate() {
        println!(
            "  {}. {} - {} messages (created: {})",
            i + 1,
            info.session.id.as_str(),
            info.message_count,
            info.session.created_at.format("%Y-%m-%d %H:%M")
        );
    }

    // Search by content (example - requires sessions to be loaded)
    println!("\nSearching for sessions containing 'error'...");
    let error_sessions = search_by_content("error", false, false).await?;
    println!("Found {} sessions with 'error'", error_sessions.len());

    println!();
    Ok(())
}

#[allow(dead_code)]
async fn demonstrate_write_operations() -> cc_sdk::Result<()> {
    println!("--- Write Operations ---");

    // Create a new session
    let new_session_id = SessionId::new(&format!("example-session-{}", Utc::now().timestamp()));
    let project_id = "example-project";

    println!("Creating new session: {}", new_session_id.as_str());

    let options = CreateSessionOptions {
        initial_message: None,
        created_at: Some(Utc::now()),
        overwrite: false,
    };

    let _session = create_session(&new_session_id, project_id, Some(options)).await?;
    println!("Session created successfully");

    // Write messages would go here
    // write_message(&new_session_id, &message).await?;

    println!();
    Ok(())
}

#[allow(dead_code)]
async fn demonstrate_management() -> cc_sdk::Result<()> {
    println!("--- Session Management ---");

    // Assuming we have a session to work with
    let projects = list_projects().await?;
    if let Some(project) = projects.first() {
        let sessions = list_sessions(&project.id).await?;
        if let Some(session) = sessions.first() {
            let session_id = &session.id;

            // Get statistics
            println!("Getting statistics for session: {}", session_id.as_str());
            let stats = get_session_stats(session_id).await?;

            println!("  Total messages: {}", stats.message_count);
            println!("  User messages: {}", stats.user_message_count);
            println!("  Assistant messages: {}", stats.assistant_message_count);
            println!("  Tool uses: {}", stats.tool_use_count);
            println!("  Size: {} bytes", stats.size_bytes);

            if !stats.top_tools.is_empty() {
                println!("  Top tools:");
                for (tool, count) in stats.top_tools.iter().take(5) {
                    println!("    - {}: {} uses", tool, count);
                }
            }

            // Fork the session
            println!("\nForking session...");
            let forked_id = fork_session(session_id, None).await?;
            println!("Forked to: {}", forked_id.as_str());

            // Export to markdown
            let export_path = PathBuf::from(format!("/tmp/session-export-{}.md", Utc::now().timestamp()));
            println!("\nExporting to: {:?}", export_path);
            export_session(session_id, &export_path, ExportFormat::Markdown).await?;
            println!("Export complete");
        }
    }

    println!();
    Ok(())
}
