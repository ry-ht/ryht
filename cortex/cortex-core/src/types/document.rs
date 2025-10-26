//! Document entity types for the documentation system.
//!
//! This module provides types for managing documentation including:
//! - Documents with hierarchical sections
//! - Version control
//! - Internal and external links
//! - Search indexing

use crate::id::CortexId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of document
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    /// User guide or tutorial
    Guide,
    /// API reference documentation
    ApiReference,
    /// Architecture or design document
    Architecture,
    /// Tutorial or how-to guide
    Tutorial,
    /// Explanation or concept document
    Explanation,
    /// Troubleshooting guide
    Troubleshooting,
    /// FAQ document
    Faq,
    /// Release notes
    ReleaseNotes,
    /// Code examples
    Example,
    /// General documentation
    General,
}

/// Status of a document
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DocumentStatus {
    /// Document is in draft state
    Draft,
    /// Document is under review
    Review,
    /// Document is published and active
    Published,
    /// Document is archived
    Archived,
    /// Document is deprecated
    Deprecated,
}

/// Type of link between documents or to external resources
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum LinkType {
    /// Link to related documentation
    Related,
    /// Link to prerequisite documentation
    Prerequisite,
    /// Link to next document in sequence
    Next,
    /// Link to previous document in sequence
    Previous,
    /// Link to parent document
    Parent,
    /// Link to child document
    Child,
    /// Reference link
    Reference,
    /// External resource link
    External,
    /// Link to API reference
    ApiReference,
    /// Link to code example
    Example,
}

/// Target of a document link
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LinkTarget {
    /// Link to another document
    Document {
        /// ID of target document
        document_id: CortexId,
        /// Optional section ID within document
        section_id: Option<String>,
    },
    /// Link to a code unit
    CodeUnit {
        /// ID of target code unit
        code_unit_id: CortexId,
    },
    /// Link to external URL
    External {
        /// External URL
        url: String,
    },
    /// Link to file in workspace
    File {
        /// File path in workspace
        path: String,
    },
}

/// Represents a document in the documentation system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Document {
    /// Unique identifier
    pub id: CortexId,

    /// Document title
    pub title: String,

    /// Document slug for URLs
    pub slug: String,

    /// Type of document
    pub doc_type: DocumentType,

    /// Current status
    pub status: DocumentStatus,

    /// Document description/summary
    pub description: Option<String>,

    /// Full content in markdown format
    pub content: String,

    /// Content hash for change detection
    pub content_hash: String,

    /// Parent document ID (for hierarchical docs)
    pub parent_id: Option<CortexId>,

    /// Order within parent (for sorting)
    pub order: i32,

    /// Tags for categorization
    pub tags: Vec<String>,

    /// Keywords for search
    pub keywords: Vec<String>,

    /// Primary author
    pub author: String,

    /// Contributors
    pub contributors: Vec<String>,

    /// Version number
    pub version: String,

    /// Language code (e.g., "en", "es")
    pub language: String,

    /// Source file path if imported
    pub source_path: Option<String>,

    /// Workspace ID if associated with workspace
    pub workspace_id: Option<String>,

    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Publication timestamp
    pub published_at: Option<DateTime<Utc>>,
}

impl Document {
    /// Create a new document
    pub fn new(title: String, content: String) -> Self {
        let now = Utc::now();
        let slug = Self::generate_slug(&title);

        Self {
            id: CortexId::new(),
            title,
            slug,
            doc_type: DocumentType::General,
            status: DocumentStatus::Draft,
            description: None,
            content: content.clone(),
            content_hash: Self::hash_content(&content),
            parent_id: None,
            order: 0,
            tags: Vec::new(),
            keywords: Vec::new(),
            author: "system".to_string(),
            contributors: Vec::new(),
            version: "1.0.0".to_string(),
            language: "en".to_string(),
            source_path: None,
            workspace_id: None,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
            published_at: None,
        }
    }

    /// Generate a URL-friendly slug from title
    fn generate_slug(title: &str) -> String {
        title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }

    /// Generate content hash
    fn hash_content(content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Update content and hash
    pub fn update_content(&mut self, content: String) {
        self.content = content.clone();
        self.content_hash = Self::hash_content(&content);
        self.updated_at = Utc::now();
    }

    /// Publish the document
    pub fn publish(&mut self) {
        self.status = DocumentStatus::Published;
        self.published_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Archive the document
    pub fn archive(&mut self) {
        self.status = DocumentStatus::Archived;
        self.updated_at = Utc::now();
    }

    /// Check if document is published
    pub fn is_published(&self) -> bool {
        self.status == DocumentStatus::Published
    }
}

/// Represents a section within a document
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocumentSection {
    /// Unique identifier
    pub id: CortexId,

    /// Document this section belongs to
    pub document_id: CortexId,

    /// Section identifier (e.g., "introduction", "getting-started")
    pub section_id: String,

    /// Section title
    pub title: String,

    /// Section content
    pub content: String,

    /// Section level (1 = top level, 2 = subsection, etc.)
    pub level: u32,

    /// Parent section ID
    pub parent_section_id: Option<String>,

    /// Order within parent
    pub order: i32,

    /// Anchor for linking
    pub anchor: String,

    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl DocumentSection {
    /// Create a new section
    pub fn new(document_id: CortexId, title: String, content: String, level: u32) -> Self {
        let now = Utc::now();
        let section_id = Self::generate_section_id(&title);
        let anchor = format!("#{}", section_id);

        Self {
            id: CortexId::new(),
            document_id,
            section_id,
            title,
            content,
            level,
            parent_section_id: None,
            order: 0,
            anchor,
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Generate section ID from title
    fn generate_section_id(title: &str) -> String {
        title
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }
}

/// Represents a link between documents or to external resources
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocumentLink {
    /// Unique identifier
    pub id: CortexId,

    /// Source document ID
    pub source_document_id: CortexId,

    /// Source section ID (optional)
    pub source_section_id: Option<String>,

    /// Type of link
    pub link_type: LinkType,

    /// Target of the link
    pub target: LinkTarget,

    /// Link description
    pub description: Option<String>,

    /// Link weight/importance (0.0 - 1.0)
    pub weight: f32,

    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl DocumentLink {
    /// Create a new link
    pub fn new(
        source_document_id: CortexId,
        link_type: LinkType,
        target: LinkTarget,
    ) -> Self {
        Self {
            id: CortexId::new(),
            source_document_id,
            source_section_id: None,
            link_type,
            target,
            description: None,
            weight: 1.0,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Set source section
    pub fn with_source_section(mut self, section_id: String) -> Self {
        self.source_section_id = Some(section_id);
        self
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Set weight
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight.clamp(0.0, 1.0);
        self
    }
}

/// Represents a version of a document
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocumentVersion {
    /// Unique identifier
    pub id: CortexId,

    /// Document this version belongs to
    pub document_id: CortexId,

    /// Version number
    pub version: String,

    /// Document content at this version
    pub content: String,

    /// Content hash
    pub content_hash: String,

    /// Version author
    pub author: String,

    /// Commit message/description
    pub message: String,

    /// Parent version ID
    pub parent_version_id: Option<CortexId>,

    /// Tags associated with this version
    pub tags: Vec<String>,

    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl DocumentVersion {
    /// Create a new version
    pub fn new(
        document_id: CortexId,
        version: String,
        content: String,
        author: String,
        message: String,
    ) -> Self {
        let content_hash = Self::hash_content(&content);

        Self {
            id: CortexId::new(),
            document_id,
            version,
            content: content.clone(),
            content_hash,
            author,
            message,
            parent_version_id: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Generate content hash
    fn hash_content(content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Set parent version
    pub fn with_parent(mut self, parent_version_id: CortexId) -> Self {
        self.parent_version_id = Some(parent_version_id);
        self
    }
}
