//! Session write operations module.
//!
//! This module provides functionality for creating, updating, and deleting sessions,
//! as well as writing session data to disk.
//!
//! # Examples
//!
//! ```no_run
//! use crate::cc::session::writer::{create_session, write_message};
//! use crate::cc::core::SessionId;
//! use crate::cc::types::Message;
//!
//! # async fn example() -> cc_sdk::Result<()> {
//! // Create a new session
//! let session_id = SessionId::new("new-session");
//! let project_id = "project-123";
//! create_session(&session_id, project_id, None).await?;
//!
//! // Write a message to the session
//! // let message = Message::User { ... };
//! // write_message(&session_id, &message).await?;
//! # Ok(())
//! # }
//! ```

use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use serde_json;

use crate::cc::core::SessionId;
use crate::cc::error::{Error, SessionError};
use crate::cc::result::Result;
use crate::cc::messages::Message;

use crate::cc::discovery::{get_projects_dir, list_projects};
use crate::cc::types::{Project, Session};
use crate::cc::cache;

/// Options for creating a new session.
#[derive(Debug, Clone)]
pub struct CreateSessionOptions {
    /// Initial message to write to the session
    pub initial_message: Option<Message>,
    /// Custom creation timestamp (defaults to now)
    pub created_at: Option<DateTime<Utc>>,
    /// Whether to overwrite if session already exists
    pub overwrite: bool,
}

impl Default for CreateSessionOptions {
    fn default() -> Self {
        Self {
            initial_message: None,
            created_at: None,
            overwrite: false,
        }
    }
}

/// Create a new session for a project.
///
/// This creates the session file and optionally writes an initial message.
///
/// # Errors
///
/// Returns an error if:
/// - The project directory doesn't exist
/// - The session already exists (unless `overwrite` is true)
/// - File creation fails
///
/// # Examples
///
/// ```no_run
/// use crate::cc::session::writer::create_session;
/// use crate::cc::core::SessionId;
///
/// # async fn example() -> cc_sdk::Result<()> {
/// let session_id = SessionId::new("new-session");
/// create_session(&session_id, "project-id", None).await?;
/// # Ok(())
/// # }
/// ```
pub async fn create_session(
    session_id: &SessionId,
    project_id: &str,
    options: Option<CreateSessionOptions>,
) -> Result<Session> {
    let options = options.unwrap_or_default();

    let projects_dir = get_projects_dir()?;
    let project_dir = projects_dir.join(project_id);

    if !project_dir.exists() {
        return Err(Error::Session(SessionError::InitializationFailed {
            reason: format!("Project directory not found: {}", project_id),
            source: None,
        }));
    }

    let sessions_dir = project_dir.join("sessions");
    let session_file = sessions_dir.join(format!("{}.jsonl", session_id.as_str()));

    // Check if session already exists
    if session_file.exists() && !options.overwrite {
        return Err(Error::Session(SessionError::AlreadyExists {
            session_id: session_id.to_string(),
        }));
    }

    // Create sessions directory if it doesn't exist
    let sessions_dir_clone = sessions_dir.clone();
    let session_file_clone = session_file.clone();
    let initial_message = options.initial_message.clone();

    tokio::task::spawn_blocking(move || -> Result<()> {
        fs::create_dir_all(&sessions_dir_clone)
            .map_err(|e| Error::Session(SessionError::IoError(e)))?;

        // Create or truncate the session file
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&session_file_clone)
            .map_err(|e| Error::Session(SessionError::IoError(e)))?;

        // Write initial message if provided
        if let Some(message) = initial_message {
            let mut writer = BufWriter::new(file);
            let json = serde_json::to_string(&message)
                .map_err(|e| Error::Session(SessionError::ParseError(e.to_string())))?;
            writeln!(writer, "{}", json)
                .map_err(|e| Error::Session(SessionError::IoError(e)))?;
            writer.flush()
                .map_err(|e| Error::Session(SessionError::IoError(e)))?;
        }

        Ok(())
    })
    .await
    .map_err(|e| Error::Protocol(format!("Task failed: {}", e)))??;

    // Create session object
    let created_at = options.created_at.unwrap_or_else(Utc::now);
    let first_message = if let Some(Message::User { message: ref user_msg }) = options.initial_message {
        Some(user_msg.content.clone())
    } else {
        None
    };

    let session = Session::new(
        session_id.clone(),
        project_dir.clone(),
        created_at,
        first_message,
    )
    .with_file_path(session_file);

    // Invalidate cache for this project
    cache::clear_cache();

    Ok(session)
}

/// Write a message to an existing session.
///
/// # Errors
///
/// Returns an error if:
/// - The session file doesn't exist
/// - File write fails
/// - JSON serialization fails
///
/// # Examples
///
/// ```no_run
/// use crate::cc::session::writer::write_message;
/// use crate::cc::core::SessionId;
/// use crate::cc::types::Message;
///
/// # async fn example() -> cc_sdk::Result<()> {
/// let session_id = SessionId::new("session-id");
/// // let message = Message::User { ... };
/// // write_message(&session_id, &message).await?;
/// # Ok(())
/// # }
/// ```
pub async fn write_message(session_id: &SessionId, message: &Message) -> Result<()> {
    let session_file = find_session_file(session_id).await?;

    let message = message.clone();
    let session_file_clone = session_file.clone();

    tokio::task::spawn_blocking(move || -> Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&session_file_clone)
            .map_err(|e| Error::Session(SessionError::IoError(e)))?;

        let mut writer = BufWriter::new(file);
        let json = serde_json::to_string(&message)
            .map_err(|e| Error::Session(SessionError::ParseError(e.to_string())))?;
        writeln!(writer, "{}", json)
            .map_err(|e| Error::Session(SessionError::IoError(e)))?;
        writer.flush()
            .map_err(|e| Error::Session(SessionError::IoError(e)))?;

        Ok(())
    })
    .await
    .map_err(|e| Error::Protocol(format!("Task failed: {}", e)))??;

    // Invalidate cache
    cache::clear_cache();

    Ok(())
}

/// Update session metadata.
///
/// This updates the metadata.json file for the session's project.
///
/// # Errors
///
/// Returns an error if:
/// - The session cannot be found
/// - The metadata file cannot be read or written
/// - JSON serialization fails
///
/// # Examples
///
/// ```no_run
/// use crate::cc::session::writer::update_session_metadata;
/// use crate::cc::core::SessionId;
///
/// # async fn example() -> cc_sdk::Result<()> {
/// let session_id = SessionId::new("session-id");
/// let metadata = serde_json::json!({
///     "custom_field": "value",
///     "tags": ["important", "production"]
/// });
/// update_session_metadata(&session_id, metadata).await?;
/// # Ok(())
/// # }
/// ```
pub async fn update_session_metadata(
    session_id: &SessionId,
    metadata: serde_json::Value,
) -> Result<()> {
    use crate::cc::discovery::{list_projects, list_sessions};
    use tokio::fs;

    // Find the session across all projects
    let projects = list_projects().await?;
    let mut session = None;

    for project in projects {
        let sessions = list_sessions(&project.id).await?;
        if let Some(found) = sessions.into_iter().find(|s| &s.id == session_id) {
            session = Some(found);
            break;
        }
    }

    let session = session.ok_or_else(|| {
        Error::Session(crate::error::SessionError::NotFound {
            session_id: session_id.clone(),
        })
    })?;

    // Get the project directory from the session's project_path
    let project_dir = session.project_path;
    let metadata_path = project_dir.join("metadata.json");

    // Read existing metadata or create new
    let mut existing_metadata = if metadata_path.exists() {
        let content = fs::read_to_string(&metadata_path).await.map_err(|e| {
            Error::Session(crate::error::SessionError::IoError(e))
        })?;
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Merge new metadata into existing
    if let (Some(existing_obj), Some(new_obj)) = (existing_metadata.as_object_mut(), metadata.as_object()) {
        for (key, value) in new_obj {
            existing_obj.insert(key.clone(), value.clone());
        }
    }

    // Write updated metadata
    let metadata_content = serde_json::to_string_pretty(&existing_metadata).map_err(|e| {
        Error::Session(crate::error::SessionError::ParseError(
            format!("Failed to serialize metadata: {}", e)
        ))
    })?;

    fs::write(&metadata_path, metadata_content)
        .await
        .map_err(|e| {
            Error::Session(crate::error::SessionError::IoError(e))
        })?;

    // Clear cache to ensure next read gets updated data
    cache::clear_cache();
    Ok(())
}

/// Delete a session.
///
/// This removes the session file from disk. Use with caution!
///
/// # Safety
///
/// This permanently deletes the session file. There is no recovery.
///
/// # Errors
///
/// Returns an error if:
/// - The session file doesn't exist
/// - File deletion fails
///
/// # Examples
///
/// ```no_run
/// use crate::cc::session::writer::delete_session;
/// use crate::cc::core::SessionId;
///
/// # async fn example() -> cc_sdk::Result<()> {
/// let session_id = SessionId::new("session-id");
/// delete_session(&session_id, false).await?;
/// # Ok(())
/// # }
/// ```
pub async fn delete_session(session_id: &SessionId, force: bool) -> Result<()> {
    let session_file = find_session_file(session_id).await?;

    // Safety check: don't delete if file is not empty (unless forced)
    if !force {
        let session_file_clone = session_file.clone();
        let is_empty = tokio::task::spawn_blocking(move || -> Result<bool> {
            let metadata = fs::metadata(&session_file_clone)
                .map_err(|e| Error::Session(SessionError::IoError(e)))?;
            Ok(metadata.len() == 0)
        })
        .await
        .map_err(|e| Error::Protocol(format!("Task failed: {}", e)))??;

        if !is_empty {
            return Err(Error::Session(SessionError::InvalidState {
                current: "non-empty".to_string(),
                expected: "empty or force=true".to_string(),
            }));
        }
    }

    let session_file_clone = session_file.clone();
    tokio::task::spawn_blocking(move || -> Result<()> {
        fs::remove_file(&session_file_clone)
            .map_err(|e| Error::Session(SessionError::IoError(e)))?;
        Ok(())
    })
    .await
    .map_err(|e| Error::Protocol(format!("Task failed: {}", e)))??;

    // Invalidate cache
    cache::clear_cache();

    Ok(())
}

/// Create a new project directory.
///
/// # Examples
///
/// ```no_run
/// use crate::cc::session::writer::create_project;
/// use std::path::PathBuf;
///
/// # async fn example() -> cc_sdk::Result<()> {
/// let project_path = PathBuf::from("/path/to/project");
/// let project = create_project("project-id", &project_path).await?;
/// # Ok(())
/// # }
/// ```
pub async fn create_project(project_id: &str, project_path: &Path) -> Result<Project> {
    let projects_dir = get_projects_dir()?;
    let project_dir = projects_dir.join(project_id);

    if project_dir.exists() {
        return Err(Error::Session(SessionError::AlreadyExists {
            session_id: format!("project:{}", project_id),
        }));
    }

    let project_dir_clone = project_dir.clone();
    let project_path_clone = project_path.to_path_buf();

    tokio::task::spawn_blocking(move || -> Result<()> {
        // Create project directory
        fs::create_dir_all(&project_dir_clone)
            .map_err(|e| Error::Session(SessionError::IoError(e)))?;

        // Create sessions subdirectory
        fs::create_dir_all(project_dir_clone.join("sessions"))
            .map_err(|e| Error::Session(SessionError::IoError(e)))?;

        // Write metadata
        let metadata = serde_json::json!({
            "path": project_path_clone,
            "created_at": Utc::now().to_rfc3339(),
        });

        let metadata_file = project_dir_clone.join("metadata.json");
        fs::write(&metadata_file, serde_json::to_string_pretty(&metadata).unwrap())
            .map_err(|e| Error::Session(SessionError::IoError(e)))?;

        Ok(())
    })
    .await
    .map_err(|e| Error::Protocol(format!("Task failed: {}", e)))??;

    // Invalidate cache
    cache::clear_cache();

    Ok(Project::new(
        project_id.to_string(),
        project_path.to_path_buf(),
        vec![],
    ))
}

/// Delete a project and all its sessions.
///
/// This is a destructive operation that removes the entire project directory.
///
/// # Safety
///
/// This permanently deletes all sessions in the project. Use with extreme caution!
///
/// # Examples
///
/// ```no_run
/// use crate::cc::session::writer::delete_project;
///
/// # async fn example() -> cc_sdk::Result<()> {
/// delete_project("project-id", false).await?;
/// # Ok(())
/// # }
/// ```
pub async fn delete_project(project_id: &str, force: bool) -> Result<()> {
    let projects_dir = get_projects_dir()?;
    let project_dir = projects_dir.join(project_id);

    if !project_dir.exists() {
        return Err(Error::Session(SessionError::NotFound {
            session_id: SessionId::new(format!("project:{}", project_id)),
        }));
    }

    // Safety check: don't delete if project has sessions (unless forced)
    if !force {
        let sessions_dir = project_dir.join("sessions");
        if sessions_dir.exists() {
            let has_sessions = tokio::task::spawn_blocking(move || -> Result<bool> {
                let entries = fs::read_dir(&sessions_dir)
                    .map_err(|e| Error::Session(SessionError::IoError(e)))?;
                Ok(entries.count() > 0)
            })
            .await
            .map_err(|e| Error::Protocol(format!("Task failed: {}", e)))??;

            if has_sessions {
                return Err(Error::Session(SessionError::InvalidState {
                    current: "has sessions".to_string(),
                    expected: "empty or force=true".to_string(),
                }));
            }
        }
    }

    let project_dir_clone = project_dir.clone();
    tokio::task::spawn_blocking(move || -> Result<()> {
        fs::remove_dir_all(&project_dir_clone)
            .map_err(|e| Error::Session(SessionError::IoError(e)))?;
        Ok(())
    })
    .await
    .map_err(|e| Error::Protocol(format!("Task failed: {}", e)))??;

    // Invalidate cache
    cache::clear_cache();

    Ok(())
}

/// Find the session file path for a session ID.
///
/// This searches all projects for the session file.
async fn find_session_file(session_id: &SessionId) -> Result<PathBuf> {
    let projects = list_projects().await?;
    let projects_dir = get_projects_dir()?;

    for project in projects {
        let session_file = projects_dir
            .join(&project.id)
            .join("sessions")
            .join(format!("{}.jsonl", session_id.as_str()));

        if session_file.exists() {
            return Ok(session_file);
        }
    }

    Err(Error::Session(SessionError::NotFound {
        session_id: session_id.clone(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_and_delete_session() {
        let temp_dir = TempDir::new().unwrap();
        let project_id = "test-project";
        let projects_dir = temp_dir.path().join("projects");
        let project_dir = projects_dir.join(project_id);

        // Create project directory
        fs::create_dir_all(&project_dir).unwrap();

        // Note: This test would need to mock get_projects_dir()
        // For now, this is a placeholder showing the test structure
    }

    #[tokio::test]
    async fn test_write_message() {
        // Similar to above, would need proper setup
    }
}
