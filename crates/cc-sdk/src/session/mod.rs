//! Session management module.
//!
//! This module provides functionality for managing Claude Code sessions,
//! including discovering projects, listing sessions, and loading session history.
//!
//! # Examples
//!
//! ```no_run
//! use cc_sdk::session::{list_projects, list_sessions, load_session_history};
//! use cc_sdk::core::SessionId;
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

mod manager;
mod types;

// Re-export public API
pub use manager::{
    get_claude_dir, get_projects_dir, list_projects, list_sessions,
    load_session_history, find_project_by_path,
};
pub use types::{Project, Session, SessionMetadata};
