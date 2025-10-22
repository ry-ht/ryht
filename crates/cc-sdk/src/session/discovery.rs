//! Session discovery implementation.
//!
//! This module provides functions for discovering projects, listing sessions,
//! and loading session history from JSONL files. It handles the core discovery
//! operations for Claude Code sessions stored in the `~/.claude` directory.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use serde_json;

use crate::core::SessionId;
use crate::error::{Error, SessionError};
use crate::result::Result;
use crate::messages::Message;

use super::types::{Project, Session, SessionMetadata};

/// Get the Claude configuration directory.
///
/// Returns `~/.claude` on Unix-like systems.
///
/// # Errors
///
/// Returns `SessionError::HomeDirectoryNotFound` if the home directory cannot be determined.
pub fn get_claude_dir() -> Result<PathBuf> {
    dirs::home_dir()
        .map(|home| home.join(".claude"))
        .ok_or_else(|| Error::Session(SessionError::HomeDirectoryNotFound))
}

/// Get the Claude projects directory.
///
/// Returns `~/.claude/projects` on Unix-like systems.
pub fn get_projects_dir() -> Result<PathBuf> {
    Ok(get_claude_dir()?.join("projects"))
}

/// List all projects in the Claude projects directory.
///
/// This function uses caching to avoid repeated filesystem scans.
///
/// # Errors
///
/// Returns an error if the projects directory cannot be read.
pub async fn list_projects() -> Result<Vec<Project>> {
    // Check cache first
    if let Some(cached) = super::cache::get_cached_projects() {
        return Ok(cached);
    }

    let projects_dir = get_projects_dir()?;

    // Return empty list if projects directory doesn't exist
    if !projects_dir.exists() {
        return Ok(Vec::new());
    }

    // Read projects directory in blocking task
    let projects = tokio::task::spawn_blocking(move || -> Result<Vec<Project>> {
        let mut projects = Vec::new();

        for entry in fs::read_dir(&projects_dir)
            .map_err(|e| Error::Session(SessionError::IoError(e)))?
        {
            let entry = entry.map_err(|e| Error::Session(SessionError::IoError(e)))?;
            let path = entry.path();

            // Skip non-directories
            if !path.is_dir() {
                continue;
            }

            // Project ID is the directory name
            let project_id = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Read project metadata
            let metadata_path = path.join("metadata.json");
            let project_path = if metadata_path.exists() {
                // Try to read project path from metadata
                if let Ok(metadata_content) = fs::read_to_string(&metadata_path) {
                    if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&metadata_content) {
                        metadata.get("path")
                            .and_then(|p| p.as_str())
                            .map(PathBuf::from)
                            .unwrap_or_else(|| path.clone())
                    } else {
                        path.clone()
                    }
                } else {
                    path.clone()
                }
            } else {
                path.clone()
            };

            // List sessions in this project
            let sessions = list_sessions_sync(&project_id, &path)?;
            let session_ids: Vec<SessionId> = sessions.into_iter().map(|s| s.id).collect();

            projects.push(Project::new(project_id, project_path, session_ids));
        }

        Ok(projects)
    })
    .await
    .map_err(|e| Error::Protocol(format!("Task failed: {}", e)))??;

    // Cache the results
    super::cache::set_cached_projects(projects.clone());

    Ok(projects)
}

/// List all sessions for a given project (sync version for internal use).
fn list_sessions_sync(_project_id: &str, project_dir: &Path) -> Result<Vec<Session>> {
    let sessions_dir = project_dir.join("sessions");

    // Return empty list if sessions directory doesn't exist
    if !sessions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions = Vec::new();

    for entry in fs::read_dir(&sessions_dir)
        .map_err(|e| Error::Session(SessionError::IoError(e)))?
    {
        let entry = entry.map_err(|e| Error::Session(SessionError::IoError(e)))?;
        let path = entry.path();

        // Only process .jsonl files
        if !path.extension().map_or(false, |ext| ext == "jsonl") {
            continue;
        }

        // Session ID is the filename without extension
        let session_id = path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Parse session metadata from file
        if let Ok(metadata) = parse_session_metadata(&path) {
            let session = Session::new(
                SessionId::new(session_id),
                project_dir.to_path_buf(),
                metadata.created_at,
                metadata.first_message,
            )
            .with_file_path(path);

            sessions.push(session);
        }
    }

    // Sort by creation time (newest first)
    sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(sessions)
}

/// List all sessions for a given project ID.
///
/// This function uses caching to avoid repeated filesystem scans.
///
/// # Errors
///
/// Returns an error if the project directory cannot be read.
pub async fn list_sessions(project_id: &str) -> Result<Vec<Session>> {
    // Check cache first
    if let Some(cached) = super::cache::get_cached_sessions(project_id) {
        return Ok(cached);
    }

    let projects_dir = get_projects_dir()?;
    let project_dir = projects_dir.join(project_id);

    if !project_dir.exists() {
        return Ok(Vec::new());
    }

    let project_id_str = project_id.to_string();
    let project_id_clone = project_id.to_string();
    let sessions = tokio::task::spawn_blocking(move || {
        list_sessions_sync(&project_id_str, &project_dir)
    })
    .await
    .map_err(|e| Error::Protocol(format!("Task failed: {}", e)))??;

    // Cache the results
    super::cache::set_cached_sessions(project_id_clone, sessions.clone());

    Ok(sessions)
}

/// Parse session metadata from a JSONL file.
fn parse_session_metadata(file_path: &Path) -> Result<SessionMetadata> {
    let file = fs::File::open(file_path)
        .map_err(|e| Error::Session(SessionError::IoError(e)))?;
    let reader = BufReader::new(file);

    let session_id = file_path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    let mut metadata = SessionMetadata::new(
        SessionId::new(session_id),
        Utc::now(), // Will be updated from first message
    );

    let mut is_first = true;

    for line in reader.lines() {
        let line = line.map_err(|e| Error::Session(SessionError::IoError(e)))?;

        if line.trim().is_empty() {
            continue;
        }

        // Try to parse as a message
        if let Ok(message) = serde_json::from_str::<Message>(&line) {
            // Use file modification time as approximation for timestamp
            let timestamp = if is_first {
                fs::metadata(file_path)
                    .ok()
                    .and_then(|m| m.created().ok())
                    .and_then(|t| DateTime::from_timestamp(
                        t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64,
                        0
                    ))
                    .unwrap_or_else(Utc::now)
            } else {
                fs::metadata(file_path)
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| DateTime::from_timestamp(
                        t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64,
                        0
                    ))
                    .unwrap_or_else(Utc::now)
            };

            if is_first {
                metadata.created_at = timestamp;
                is_first = false;
            }

            metadata.add_message(&message, timestamp);
        }
    }

    Ok(metadata)
}

/// Load the complete message history for a session.
///
/// # Errors
///
/// Returns an error if the session file cannot be found or parsed.
pub async fn load_session_history(session_id: &SessionId) -> Result<Vec<Message>> {
    // Search all projects for this session
    let projects = list_projects().await?;

    for project in projects {
        if project.sessions.contains(session_id) {
            let project_dir = get_projects_dir()?.join(&project.id);
            let session_file = project_dir
                .join("sessions")
                .join(format!("{}.jsonl", session_id.as_str()));

            if session_file.exists() {
                return load_session_file(&session_file).await;
            }
        }
    }

    Err(Error::Session(SessionError::NotFound {
        session_id: session_id.clone(),
    }))
}

/// Load messages from a session JSONL file.
async fn load_session_file(file_path: &Path) -> Result<Vec<Message>> {
    let file_path = file_path.to_path_buf();

    let messages = tokio::task::spawn_blocking(move || -> Result<Vec<Message>> {
        let file = fs::File::open(&file_path)
            .map_err(|e| Error::Session(SessionError::IoError(e)))?;
        let reader = BufReader::new(file);

        let mut messages = Vec::new();

        for line in reader.lines() {
            let line = line.map_err(|e| Error::Session(SessionError::IoError(e)))?;

            if line.trim().is_empty() {
                continue;
            }

            let message: Message = serde_json::from_str(&line)
                .map_err(|e| Error::Session(SessionError::ParseError(e.to_string())))?;

            messages.push(message);
        }

        Ok(messages)
    })
    .await
    .map_err(|e| Error::Protocol(format!("Task failed: {}", e)))??;

    Ok(messages)
}

/// Find the project directory for a given workspace path.
///
/// This function searches for an existing project that matches the workspace path.
pub async fn find_project_by_path(workspace_path: &Path) -> Result<Option<Project>> {
    let projects = list_projects().await?;

    // Normalize the workspace path for comparison
    let workspace_path = workspace_path.canonicalize().ok();

    for project in projects {
        if let Ok(project_path) = project.path.canonicalize() {
            if Some(&project_path) == workspace_path.as_ref() {
                return Ok(Some(project));
            }
        }
    }

    Ok(None)
}
