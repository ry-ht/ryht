//! Session type definitions.
//!
//! This module defines the core types for session management, including
//! projects, sessions, and session metadata.

use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::SessionId;
use crate::messages::Message;

/// Represents a Claude Code project.
///
/// A project is a workspace directory that can contain multiple sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Unique project identifier (usually a hash of the path)
    pub id: String,

    /// Absolute path to the project directory
    pub path: PathBuf,

    /// List of session IDs associated with this project
    pub sessions: Vec<SessionId>,
}

impl Project {
    /// Create a new project.
    pub fn new(id: String, path: PathBuf, sessions: Vec<SessionId>) -> Self {
        Self { id, path, sessions }
    }
}

/// Represents a Claude Code session.
///
/// A session is a conversation thread within a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier
    pub id: SessionId,

    /// Path to the project this session belongs to
    pub project_path: PathBuf,

    /// When the session was created
    pub created_at: DateTime<Utc>,

    /// The first user message in the session (if any)
    pub first_message: Option<String>,

    /// Path to the session JSONL file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<PathBuf>,
}

impl Session {
    /// Create a new session.
    pub fn new(
        id: SessionId,
        project_path: PathBuf,
        created_at: DateTime<Utc>,
        first_message: Option<String>,
    ) -> Self {
        Self {
            id,
            project_path,
            created_at,
            first_message,
            file_path: None,
        }
    }

    /// Create a new session with a file path.
    pub fn with_file_path(mut self, file_path: PathBuf) -> Self {
        self.file_path = Some(file_path);
        self
    }
}

/// Metadata extracted from a session file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Session ID
    pub session_id: SessionId,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// First message preview
    pub first_message: Option<String>,

    /// Total message count
    pub message_count: usize,

    /// Last update timestamp
    pub last_updated: DateTime<Utc>,
}

impl SessionMetadata {
    /// Create new session metadata.
    pub fn new(session_id: SessionId, created_at: DateTime<Utc>) -> Self {
        Self {
            session_id,
            created_at,
            first_message: None,
            message_count: 0,
            last_updated: created_at,
        }
    }

    /// Update metadata with a message.
    pub fn add_message(&mut self, message: &Message, timestamp: DateTime<Utc>) {
        self.message_count += 1;
        self.last_updated = timestamp;

        // Capture first user message
        if self.first_message.is_none() {
            if let Message::User { message: user_msg } = message {
                // Extract text content from user message
                let text = user_msg.content.clone();
                if !text.is_empty() {
                    self.first_message = Some(text);
                }
            }
        }
    }
}
