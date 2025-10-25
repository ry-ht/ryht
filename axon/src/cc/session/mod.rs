//! Session management module.
//!
//! This module provides comprehensive functionality for managing Claude Code sessions,
//! including:
//!
//! - **Discovery**: Finding projects and listing sessions
//! - **Caching**: In-memory caching with TTL for improved performance
//! - **Write Operations**: Creating, updating, and deleting sessions
//! - **Filtering & Search**: Advanced filtering and content search
//! - **Management**: Forking, merging, exporting, and statistics
//!
//! # Examples
//!
//! ## Basic Session Discovery
//!
//! ```no_run
//! use crate::cc::session::{list_projects, list_sessions, load_session_history};
//! use crate::cc::core::SessionId;
//!
//! # async fn example() -> cc_sdk::Result<()> {
//! // List all projects
//! let projects = list_projects().await?;
//! for project in projects {
//!     println!("Project: {} at {:?}", project.id, project.path);
//!
//!     // List sessions for this project
//!     let sessions = list_sessions(&project.id).await?;
//!     for session in sessions {
//!         println!("  Session: {:?}", session.id);
//!     }
//! }
//!
//! // Load a specific session's history
//! let session_id = SessionId::new("abc123");
//! let messages = load_session_history(&session_id).await?;
//! println!("Loaded {} messages", messages.len());
//! # Ok(())
//! # }
//! ```
//!
//! ## Creating and Writing Sessions
//!
//! ```no_run
//! use crate::cc::session::writer::{create_session, write_message};
//! use crate::cc::core::SessionId;
//!
//! # async fn example() -> cc_sdk::Result<()> {
//! // Create a new session
//! let session_id = SessionId::new("new-session");
//! create_session(&session_id, "project-id", None).await?;
//!
//! // Write messages to the session
//! // let message = Message::User { ... };
//! // write_message(&session_id, &message).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Filtering and Search
//!
//! ```no_run
//! use crate::cc::session::filter::{SessionFilter, SortBy, search_sessions};
//! use chrono::{Utc, Duration};
//!
//! # async fn example() -> cc_sdk::Result<()> {
//! // Search for sessions in the last week
//! let filter = SessionFilter::default()
//!     .with_date_range(
//!         Some(Utc::now() - Duration::days(7)),
//!         Some(Utc::now())
//!     )
//!     .with_content_search("error")
//!     .with_sort_by(SortBy::CreatedDesc)
//!     .with_limit(10);
//!
//! let sessions = search_sessions(filter).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Session Management
//!
//! ```no_run
//! use crate::cc::session::management::{fork_session, export_session, ExportFormat};
//! use crate::cc::core::SessionId;
//! use std::path::PathBuf;
//!
//! # async fn example() -> cc_sdk::Result<()> {
//! let session_id = SessionId::new("session-id");
//!
//! // Fork a session
//! let forked_id = fork_session(&session_id, None).await?;
//!
//! // Export to markdown
//! export_session(&session_id, &PathBuf::from("export.md"), ExportFormat::Markdown).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Caching
//!
//! ```no_run
//! use crate::cc::session::cache::{SessionCache, CacheConfig};
//! use std::time::Duration;
//!
//! // Create a custom cache
//! let config = CacheConfig {
//!     ttl: Duration::from_secs(600), // 10 minutes
//!     enabled: true,
//! };
//! let cache = SessionCache::new(config);
//!
//! // Or use the global cache
//! use crate::cc::session::cache;
//! cache::set_cached_projects(vec![]);
//! ```

mod cache;
mod discovery;
mod filter;
mod management;
mod types;
mod writer;

#[cfg(test)]
mod tests;

// Re-export public API

// Core discovery and loading
pub use discovery::{
    find_project_by_path, get_claude_dir, get_projects_dir, list_projects, list_sessions,
    load_session_history,
};

// Types
pub use types::{Project, Session, SessionMetadata};

// Caching
pub use cache::{
    clear_cache, get_cached_projects, get_cached_sessions, set_cached_projects,
    set_cached_sessions, CacheConfig, SessionCache,
};

// Write operations
pub use writer::{
    create_project, create_session, delete_project, delete_session, update_session_metadata,
    write_message, CreateSessionOptions,
};

// Filtering and search
pub use filter::{
    filter_by_date_range, filter_by_project, search_by_content, search_sessions, SessionFilter,
    SessionInfo, SortBy,
};

// Management
pub use management::{
    export_session, fork_session, get_bulk_stats, get_session_stats, merge_sessions,
    ExportFormat, SessionStats,
};
