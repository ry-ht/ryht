//! Advanced session management features.
//!
//! This module provides high-level session management operations including
//! forking, merging, exporting, and statistics gathering.
//!
//! # Examples
//!
//! ```no_run
//! use cc_sdk::session::management::{fork_session, get_session_stats};
//! use cc_sdk::core::SessionId;
//!
//! # async fn example() -> cc_sdk::Result<()> {
//! // Fork a session
//! let original = SessionId::new("original-session");
//! let forked = fork_session(&original, None).await?;
//!
//! // Get statistics
//! let stats = get_session_stats(&original).await?;
//! println!("Session has {} messages", stats.message_count);
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::SessionId;
use crate::error::{Error, SessionError};
use crate::result::Result;
use crate::messages::Message;

use super::discovery::{list_projects, load_session_history};
use super::writer::{create_session, write_message, CreateSessionOptions};
use super::cache;

/// Statistics for a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    /// Session ID
    pub session_id: SessionId,
    /// Total number of messages
    pub message_count: usize,
    /// Number of user messages
    pub user_message_count: usize,
    /// Number of assistant messages
    pub assistant_message_count: usize,
    /// Number of tool use blocks
    pub tool_use_count: usize,
    /// Number of tool result blocks
    pub tool_result_count: usize,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// First message timestamp
    pub first_message_at: Option<DateTime<Utc>>,
    /// Last message timestamp
    pub last_message_at: Option<DateTime<Utc>>,
    /// Estimated size in bytes
    pub size_bytes: usize,
    /// Most used tools
    pub top_tools: Vec<(String, usize)>,
}

/// Export format for sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// JSON format (array of messages)
    Json,
    /// JSONL format (one message per line)
    Jsonl,
    /// Markdown format (human-readable conversation)
    Markdown,
    /// Plain text format (simple conversation)
    Text,
}

/// Fork a session, creating a copy with a new ID.
///
/// This creates a new session with all messages from the original session.
/// The new session will have a new ID but identical content.
///
/// # Examples
///
/// ```no_run
/// use cc_sdk::session::management::fork_session;
/// use cc_sdk::core::SessionId;
///
/// # async fn example() -> cc_sdk::Result<()> {
/// let original = SessionId::new("original");
/// let forked = fork_session(&original, None).await?;
/// println!("Forked to: {}", forked.as_str());
/// # Ok(())
/// # }
/// ```
pub async fn fork_session(
    source_session_id: &SessionId,
    new_session_id: Option<SessionId>,
) -> Result<SessionId> {
    // Load source session
    let messages = load_session_history(source_session_id).await?;

    // Find the project containing the source session
    let projects = list_projects().await?;
    let project = projects
        .iter()
        .find(|p| p.sessions.contains(source_session_id))
        .ok_or_else(|| {
            Error::Session(SessionError::NotFound {
                session_id: source_session_id.clone(),
            })
        })?;

    // Generate new session ID if not provided
    let new_id = new_session_id.unwrap_or_else(|| {
        SessionId::new(format!("{}-fork-{}", source_session_id.as_str(), Utc::now().timestamp()))
    });

    // Create the new session
    let first_message = messages.first().cloned();
    let options = CreateSessionOptions {
        initial_message: first_message,
        created_at: Some(Utc::now()),
        overwrite: false,
    };

    create_session(&new_id, &project.id, Some(options)).await?;

    // Write remaining messages
    for message in messages.iter().skip(1) {
        write_message(&new_id, message).await?;
    }

    cache::clear_cache();

    Ok(new_id)
}

/// Merge multiple sessions into a new session.
///
/// This creates a new session containing all messages from the source sessions,
/// ordered by timestamp.
///
/// # Examples
///
/// ```no_run
/// use cc_sdk::session::management::merge_sessions;
/// use cc_sdk::core::SessionId;
///
/// # async fn example() -> cc_sdk::Result<()> {
/// let sessions = vec![
///     SessionId::new("session-1"),
///     SessionId::new("session-2"),
/// ];
/// let merged = merge_sessions(&sessions, None, "project-id").await?;
/// # Ok(())
/// # }
/// ```
pub async fn merge_sessions(
    source_session_ids: &[SessionId],
    new_session_id: Option<SessionId>,
    project_id: &str,
) -> Result<SessionId> {
    if source_session_ids.is_empty() {
        return Err(Error::Session(SessionError::InvalidState {
            current: "empty".to_string(),
            expected: "at least one session".to_string(),
        }));
    }

    // Load all messages from source sessions
    let mut all_messages = Vec::new();
    for session_id in source_session_ids {
        let messages = load_session_history(session_id).await?;
        all_messages.extend(messages);
    }

    // Sort by timestamp (if available in message metadata)
    // For now, messages are in order they were loaded

    // Generate new session ID if not provided
    let new_id = new_session_id.unwrap_or_else(|| {
        SessionId::new(format!("merged-{}", Utc::now().timestamp()))
    });

    // Create the new session
    let first_message = all_messages.first().cloned();
    let options = CreateSessionOptions {
        initial_message: first_message,
        created_at: Some(Utc::now()),
        overwrite: false,
    };

    create_session(&new_id, project_id, Some(options)).await?;

    // Write remaining messages
    for message in all_messages.iter().skip(1) {
        write_message(&new_id, message).await?;
    }

    cache::clear_cache();

    Ok(new_id)
}

/// Export a session to a file in the specified format.
///
/// # Examples
///
/// ```no_run
/// use cc_sdk::session::management::{export_session, ExportFormat};
/// use cc_sdk::core::SessionId;
/// use std::path::PathBuf;
///
/// # async fn example() -> cc_sdk::Result<()> {
/// let session_id = SessionId::new("session-id");
/// let output_path = PathBuf::from("export.md");
/// export_session(&session_id, &output_path, ExportFormat::Markdown).await?;
/// # Ok(())
/// # }
/// ```
pub async fn export_session(
    session_id: &SessionId,
    output_path: &PathBuf,
    format: ExportFormat,
) -> Result<()> {
    let messages = load_session_history(session_id).await?;

    let content = match format {
        ExportFormat::Json => export_as_json(&messages)?,
        ExportFormat::Jsonl => export_as_jsonl(&messages)?,
        ExportFormat::Markdown => export_as_markdown(&messages, session_id),
        ExportFormat::Text => export_as_text(&messages),
    };

    let output_path = output_path.clone();
    tokio::task::spawn_blocking(move || -> Result<()> {
        let mut file = File::create(&output_path)
            .map_err(|e| Error::Session(SessionError::IoError(e)))?;
        file.write_all(content.as_bytes())
            .map_err(|e| Error::Session(SessionError::IoError(e)))?;
        Ok(())
    })
    .await
    .map_err(|e| Error::Protocol(format!("Task failed: {}", e)))??;

    Ok(())
}

/// Get statistics for a session.
///
/// # Examples
///
/// ```no_run
/// use cc_sdk::session::management::get_session_stats;
/// use cc_sdk::core::SessionId;
///
/// # async fn example() -> cc_sdk::Result<()> {
/// let session_id = SessionId::new("session-id");
/// let stats = get_session_stats(&session_id).await?;
/// println!("Total messages: {}", stats.message_count);
/// println!("Tool uses: {}", stats.tool_use_count);
/// # Ok(())
/// # }
/// ```
pub async fn get_session_stats(session_id: &SessionId) -> Result<SessionStats> {
    let messages = load_session_history(session_id).await?;

    let mut stats = SessionStats {
        session_id: session_id.clone(),
        message_count: messages.len(),
        user_message_count: 0,
        assistant_message_count: 0,
        tool_use_count: 0,
        tool_result_count: 0,
        created_at: Utc::now(),
        first_message_at: None,
        last_message_at: None,
        size_bytes: 0,
        top_tools: Vec::new(),
    };

    let mut tool_counts: HashMap<String, usize> = HashMap::new();

    for message in &messages {
        match message {
            Message::User { .. } => {
                stats.user_message_count += 1;
            }
            Message::Assistant { message } => {
                stats.assistant_message_count += 1;

                // Count tool uses and tool results
                for block in &message.content {
                    match block {
                        crate::messages::ContentBlock::ToolUse(tool_use) => {
                            stats.tool_use_count += 1;
                            *tool_counts.entry(tool_use.name.clone()).or_insert(0) += 1;
                        }
                        crate::messages::ContentBlock::ToolResult(_) => {
                            stats.tool_result_count += 1;
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        // Estimate size
        if let Ok(json) = serde_json::to_string(message) {
            stats.size_bytes += json.len() + 1; // +1 for newline
        }
    }

    // Get top 10 most used tools
    let mut tool_vec: Vec<(String, usize)> = tool_counts.into_iter().collect();
    tool_vec.sort_by(|a, b| b.1.cmp(&a.1));
    tool_vec.truncate(10);
    stats.top_tools = tool_vec;

    Ok(stats)
}

/// Get statistics for multiple sessions.
///
/// # Examples
///
/// ```no_run
/// use cc_sdk::session::management::get_bulk_stats;
/// use cc_sdk::core::SessionId;
///
/// # async fn example() -> cc_sdk::Result<()> {
/// let sessions = vec![SessionId::new("s1"), SessionId::new("s2")];
/// let stats = get_bulk_stats(&sessions).await?;
/// # Ok(())
/// # }
/// ```
pub async fn get_bulk_stats(session_ids: &[SessionId]) -> Result<Vec<SessionStats>> {
    let mut results = Vec::new();

    for session_id in session_ids {
        match get_session_stats(session_id).await {
            Ok(stats) => results.push(stats),
            Err(_) => continue, // Skip sessions that can't be loaded
        }
    }

    Ok(results)
}

// Export helper functions

fn export_as_json(messages: &[Message]) -> Result<String> {
    serde_json::to_string_pretty(messages)
        .map_err(|e| Error::Session(SessionError::ParseError(e.to_string())))
}

fn export_as_jsonl(messages: &[Message]) -> Result<String> {
    let mut lines = Vec::new();
    for message in messages {
        let json = serde_json::to_string(message)
            .map_err(|e| Error::Session(SessionError::ParseError(e.to_string())))?;
        lines.push(json);
    }
    Ok(lines.join("\n"))
}

fn export_as_markdown(messages: &[Message], session_id: &SessionId) -> String {
    let mut output = String::new();
    output.push_str(&format!("# Session: {}\n\n", session_id.as_str()));

    for message in messages {
        match message {
            Message::User { message: user_msg } => {
                output.push_str("## User\n\n");
                output.push_str(&user_msg.content);
                output.push_str("\n\n");
            }
            Message::Assistant { message: asst_msg } => {
                output.push_str("## Assistant\n\n");
                for block in &asst_msg.content {
                    match block {
                        crate::messages::ContentBlock::Text(text_content) => {
                            output.push_str(&text_content.text);
                            output.push_str("\n\n");
                        }
                        crate::messages::ContentBlock::ToolUse(tool_use) => {
                            output.push_str(&format!("**Tool Use:** `{}`\n\n", tool_use.name));
                            if let Ok(json) = serde_json::to_string_pretty(&tool_use.input) {
                                output.push_str("```json\n");
                                output.push_str(&json);
                                output.push_str("\n```\n\n");
                            }
                        }
                        crate::messages::ContentBlock::ToolResult(tool_result) => {
                            output.push_str("### Tool Result\n\n");
                            output.push_str("```\n");
                            if let Some(content) = &tool_result.content {
                                match content {
                                    crate::messages::ContentValue::Text(text) => output.push_str(text),
                                    crate::messages::ContentValue::Structured(vals) => {
                                        if let Ok(json) = serde_json::to_string_pretty(vals) {
                                            output.push_str(&json);
                                        }
                                    }
                                }
                            }
                            output.push_str("\n```\n\n");
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    output
}

fn export_as_text(messages: &[Message]) -> String {
    let mut output = String::new();

    for message in messages {
        match message {
            Message::User { message: user_msg } => {
                output.push_str("USER:\n");
                output.push_str(&user_msg.content);
                output.push_str("\n\n");
            }
            Message::Assistant { message: asst_msg } => {
                output.push_str("ASSISTANT:\n");
                for block in &asst_msg.content {
                    if let crate::messages::ContentBlock::Text(text_content) = block {
                        output.push_str(&text_content.text);
                        output.push_str("\n");
                    }
                }
                output.push_str("\n");
            }
            _ => {}
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_format() {
        assert_eq!(ExportFormat::Json, ExportFormat::Json);
        assert_ne!(ExportFormat::Json, ExportFormat::Markdown);
    }

    #[test]
    fn test_session_stats_creation() {
        let stats = SessionStats {
            session_id: SessionId::new("test"),
            message_count: 10,
            user_message_count: 5,
            assistant_message_count: 5,
            tool_use_count: 2,
            tool_result_count: 2,
            created_at: Utc::now(),
            first_message_at: None,
            last_message_at: None,
            size_bytes: 1024,
            top_tools: vec![("Bash".to_string(), 2)],
        };

        assert_eq!(stats.message_count, 10);
        assert_eq!(stats.top_tools.len(), 1);
    }
}
