//! Session management adapters for cc-sdk integration.
//!
//! This module provides adapter functions that map cc-sdk session types
//! to Axon's internal types, handling additional data loading such as
//! todo data from ~/.claude/todos/{session_id}.json

use anyhow::{Context, Result};
use cc_sdk::session::{list_projects as sdk_list_projects, list_sessions as sdk_list_sessions};
use serde_json;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::commands::claude::{Project, Session};

/// Get the path to the ~/.claude directory
fn get_claude_dir() -> Result<PathBuf> {
    dirs::home_dir()
        .context("Could not find home directory")?
        .join(".claude")
        .canonicalize()
        .context("Could not find ~/.claude directory")
}

/// Adapter for list_projects that maps cc-sdk::session::Project to Axon Project.
///
/// This function:
/// 1. Calls cc-sdk's list_projects()
/// 2. Maps each Project to Axon's Project struct
/// 3. Adds created_at and most_recent_session timestamps by reading project directory metadata
pub async fn list_projects_adapter() -> Result<Vec<Project>> {
    log::info!("Listing projects via cc-sdk adapter");

    // Get projects from cc-sdk
    let sdk_projects = sdk_list_projects()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to list projects from cc-sdk: {}", e))?;

    let claude_dir = get_claude_dir()?;
    let projects_dir = claude_dir.join("projects");

    let mut projects = Vec::new();

    for sdk_project in sdk_projects {
        // Encode path to get project directory name (matching Claude Code's encoding)
        let project_id = sdk_project.id.clone();
        let project_dir = projects_dir.join(&project_id);

        // Get directory creation time
        let (created_at, most_recent_session) = if project_dir.exists() {
            let metadata = fs::metadata(&project_dir).context("Failed to read directory metadata")?;

            let created = metadata
                .created()
                .or_else(|_| metadata.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH)
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            // Find most recent session by scanning session files
            let mut most_recent: Option<u64> = None;
            if let Ok(entries) = fs::read_dir(&project_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("jsonl")
                    {
                        if let Ok(metadata) = fs::metadata(&path) {
                            let modified = metadata
                                .modified()
                                .unwrap_or(SystemTime::UNIX_EPOCH)
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();

                            most_recent = Some(match most_recent {
                                Some(current) => current.max(modified),
                                None => modified,
                            });
                        }
                    }
                }
            }

            (created, most_recent)
        } else {
            (0, None)
        };

        // Convert SessionId to String
        let sessions: Vec<String> = sdk_project
            .sessions
            .iter()
            .map(|sid| sid.as_str().to_string())
            .collect();

        projects.push(Project {
            id: project_id,
            path: sdk_project.path.to_string_lossy().to_string(),
            sessions,
            created_at,
            most_recent_session,
        });
    }

    // Sort projects by most recent session activity, then by creation time
    projects.sort_by(|a, b| {
        match (a.most_recent_session, b.most_recent_session) {
            (Some(a_time), Some(b_time)) => b_time.cmp(&a_time),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => b.created_at.cmp(&a.created_at),
        }
    });

    log::info!("Found {} projects via adapter", projects.len());
    Ok(projects)
}

/// Adapter for create_project that handles path encoding.
///
/// This function:
/// 1. Encodes the path to create a project ID
/// 2. Creates the project directory if needed
/// 3. Returns an Axon Project struct
pub async fn create_project_adapter(path: String) -> Result<Project> {
    log::info!("Creating project for path via adapter: {}", path);

    // Encode the path to create a project ID (matching Claude Code's encoding)
    let project_id = path.replace('/', "-");

    // Get claude directory
    let claude_dir = get_claude_dir()?;
    let projects_dir = claude_dir.join("projects");

    // Create projects directory if it doesn't exist
    if !projects_dir.exists() {
        fs::create_dir_all(&projects_dir).context("Failed to create projects directory")?;
    }

    // Create project directory if it doesn't exist
    let project_dir = projects_dir.join(&project_id);
    if !project_dir.exists() {
        fs::create_dir_all(&project_dir).context("Failed to create project directory")?;
    }

    // Get creation time
    let metadata = fs::metadata(&project_dir).context("Failed to read directory metadata")?;

    let created_at = metadata
        .created()
        .or_else(|_| metadata.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Return the created project
    Ok(Project {
        id: project_id,
        path,
        sessions: Vec::new(),
        created_at,
        most_recent_session: None,
    })
}

/// Adapter for get_project_sessions that maps cc-sdk Sessions to Axon Sessions with todo data.
///
/// This function:
/// 1. Calls cc-sdk's list_sessions()
/// 2. Maps each Session to Axon's Session struct
/// 3. Loads todo data from ~/.claude/todos/{session_id}.json
/// 4. Extracts created_at timestamps
pub async fn get_project_sessions_adapter(project_id: String) -> Result<Vec<Session>> {
    log::info!("Getting sessions for project via adapter: {}", project_id);

    // Get the project path by decoding the project ID
    // For now, use a simple decode (this matches the fallback in the original code)
    let project_path = project_id.replace('-', "/");

    // Get sessions from cc-sdk
    let sdk_sessions = sdk_list_sessions(&project_id)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to list sessions from cc-sdk: {}", e))?;

    let claude_dir = get_claude_dir()?;
    let todos_dir = claude_dir.join("todos");

    let mut sessions = Vec::new();

    for sdk_session in sdk_sessions {
        let session_id_str = sdk_session.id.as_str().to_string();

        // Convert DateTime<Utc> to Unix timestamp
        let created_at = sdk_session.created_at.timestamp() as u64;

        // Try to load associated todo data
        let todo_path = todos_dir.join(format!("{}.json", session_id_str));
        let todo_data = if todo_path.exists() {
            fs::read_to_string(&todo_path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
        } else {
            None
        };

        // Extract message timestamp if available (convert to ISO string format)
        let message_timestamp = sdk_session
            .first_message
            .as_ref()
            .map(|_| sdk_session.created_at.to_rfc3339());

        sessions.push(Session {
            id: session_id_str,
            project_id: project_id.clone(),
            project_path: project_path.clone(),
            todo_data,
            created_at,
            first_message: sdk_session.first_message,
            message_timestamp,
        });
    }

    // Sort sessions by creation time (newest first)
    sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    log::info!(
        "Found {} sessions for project {} via adapter",
        sessions.len(),
        project_id
    );
    Ok(sessions)
}

/// Adapter for load_session_history that wraps cc-sdk's load_session_history.
///
/// This function:
/// 1. Calls cc-sdk's load_session_history()
/// 2. Serializes messages to serde_json::Value
pub async fn load_session_history_adapter(
    session_id: String,
    _project_id: String,
) -> Result<Vec<serde_json::Value>> {
    log::info!("Loading session history via adapter: {}", session_id);

    // Convert string to SessionId
    let session_id = cc_sdk::core::SessionId::new(session_id);

    // Load messages from cc-sdk
    let messages = cc_sdk::session::load_session_history(&session_id)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to load session history from cc-sdk: {}", e))?;

    // Convert messages to serde_json::Value
    let json_messages: Vec<serde_json::Value> = messages
        .into_iter()
        .filter_map(|msg| serde_json::to_value(msg).ok())
        .collect();

    log::info!("Loaded {} messages via adapter", json_messages.len());
    Ok(json_messages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_encoding() {
        let path = "/Users/test/my-project";
        let encoded = path.replace('/', "-");
        assert_eq!(encoded, "-Users-test-my-project");
    }

    #[test]
    fn test_path_with_hyphens() {
        let path = "/Users/test/data-discovery";
        let encoded = path.replace('/', "-");
        assert_eq!(encoded, "-Users-test-data-discovery");
    }
}
