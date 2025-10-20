//! Core types used across the Cortex system.

use crate::id::CortexId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Represents a project in the Cortex system.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub id: CortexId,
    pub name: String,
    pub path: PathBuf,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

impl Project {
    /// Create a new project
    pub fn new(name: String, path: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: CortexId::new(),
            name,
            path,
            description: None,
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }
}

/// Represents a document in the virtual filesystem.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Document {
    pub id: CortexId,
    pub project_id: CortexId,
    pub path: String,
    pub content_hash: String,
    pub size: u64,
    pub mime_type: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

/// Represents a chunk of content for semantic processing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Chunk {
    pub id: CortexId,
    pub document_id: CortexId,
    pub content: String,
    pub start_offset: usize,
    pub end_offset: usize,
    pub chunk_index: usize,
    pub metadata: HashMap<String, String>,
}

/// Represents an embedding vector.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Embedding {
    pub id: CortexId,
    pub entity_id: CortexId,
    pub entity_type: EntityType,
    pub vector: Vec<f32>,
    pub model: String,
    pub created_at: DateTime<Utc>,
}

/// Types of entities that can be embedded.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Document,
    Chunk,
    Symbol,
    Episode,
}

/// Represents a code symbol (function, class, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Symbol {
    pub id: CortexId,
    pub document_id: CortexId,
    pub name: String,
    pub kind: SymbolKind,
    pub range: Range,
    pub signature: Option<String>,
    pub documentation: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Types of code symbols.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Interface,
    Trait,
    Type,
    Constant,
    Variable,
    Module,
    Namespace,
}

/// Represents a range in a document (line/column).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Range {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

/// Represents a relationship between entities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Relation {
    pub id: CortexId,
    pub source_id: CortexId,
    pub target_id: CortexId,
    pub relation_type: RelationType,
    pub weight: f32,
    pub metadata: HashMap<String, String>,
}

/// Types of relationships between entities.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    Contains,
    References,
    Imports,
    Extends,
    Implements,
    Calls,
    DependsOn,
    SimilarTo,
    PartOf,
}

/// Represents an episodic memory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Episode {
    pub id: CortexId,
    pub project_id: CortexId,
    pub session_id: Option<String>,
    pub content: String,
    pub context: HashMap<String, serde_json::Value>,
    pub importance: f32,
    pub created_at: DateTime<Utc>,
    pub accessed_count: u32,
    pub last_accessed_at: Option<DateTime<Utc>>,
}

/// Query parameters for searching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub limit: usize,
    pub threshold: Option<f32>,
    pub filters: HashMap<String, String>,
}

impl SearchQuery {
    pub fn new(query: String) -> Self {
        Self {
            query,
            limit: 10,
            threshold: None,
            filters: HashMap::new(),
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = Some(threshold);
        self
    }

    pub fn with_filter(mut self, key: String, value: String) -> Self {
        self.filters.insert(key, value);
        self
    }
}

/// Search result with score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult<T> {
    pub item: T,
    pub score: f32,
}

/// Statistics about the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    pub total_projects: u64,
    pub total_documents: u64,
    pub total_chunks: u64,
    pub total_embeddings: u64,
    pub total_episodes: u64,
    pub storage_size_bytes: u64,
    pub last_updated: DateTime<Utc>,
}
