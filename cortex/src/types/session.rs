use super::{SessionId, SymbolId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Work session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub task_description: String,
    pub scope: Vec<PathBuf>,
    pub base_commit: Option<String>,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Session {
    pub fn new(task_description: String, scope: Vec<PathBuf>) -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::new(),
            task_description,
            scope,
            base_commit: None,
            started_at: now,
            updated_at: now,
        }
    }
}

/// Session change delta
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub change_type: ChangeType,
    pub affected_symbols: Vec<SymbolId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    FileAdded { path: String },
    FileModified { path: String },
    FileDeleted { path: String },
    SymbolAdded { symbol_id: SymbolId },
    SymbolModified { symbol_id: SymbolId },
    SymbolDeleted { symbol_id: SymbolId },
}

/// Workspace information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub root: PathBuf,
    pub projects: Vec<ProjectInfo>,
}

/// Project information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub path: PathBuf,
    pub language: String,
}
