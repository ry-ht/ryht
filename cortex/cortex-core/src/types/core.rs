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

/// Represents a file/document in the virtual filesystem.
/// This is for VFS files, not documentation system documents.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VfsDocument {
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

/// Represents an agent session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentSession {
    pub id: String,
    pub name: String,
    pub agent_type: String,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

impl AgentSession {
    /// Create a new agent session
    pub fn new(id: String, name: String, agent_type: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            agent_type,
            created_at: now,
            last_active: now,
            metadata: HashMap::new(),
        }
    }

    /// Update the last active timestamp
    pub fn update_last_active(&mut self) {
        self.last_active = Utc::now();
    }
}

// ============================================================================
// Code Unit Schema - Comprehensive Semantic Memory
// ============================================================================

/// Programming language for code units.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    Cpp,
    C,
    CSharp,
    Ruby,
    Php,
    Swift,
    Kotlin,
    Scala,
    Haskell,
    Elixir,
    Clojure,
    Zig,
    Unknown,
}

impl Language {
    /// Detect language from file extension.
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Language::Rust,
            "ts" | "tsx" => Language::TypeScript,
            "js" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
            "py" | "pyi" => Language::Python,
            "go" => Language::Go,
            "java" => Language::Java,
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Language::Cpp,
            "c" | "h" => Language::C,
            "cs" => Language::CSharp,
            "rb" => Language::Ruby,
            "php" => Language::Php,
            "swift" => Language::Swift,
            "kt" | "kts" => Language::Kotlin,
            "scala" | "sc" => Language::Scala,
            "hs" | "lhs" => Language::Haskell,
            "ex" | "exs" => Language::Elixir,
            "clj" | "cljs" | "cljc" | "edn" => Language::Clojure,
            "zig" => Language::Zig,
            _ => Language::Unknown,
        }
    }
}

/// Type of code unit.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CodeUnitType {
    Function,
    Method,
    AsyncFunction,
    Generator,
    Lambda,
    Class,
    Struct,
    Enum,
    Union,
    Interface,
    Trait,
    TypeAlias,
    Typedef,
    Const,
    Static,
    Variable,
    Module,
    Namespace,
    Package,
    ImplBlock,
    Decorator,
    Macro,
    Template,
    Test,
    Benchmark,
    Example,
}

/// Visibility level for code units.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    Private,
    Protected,
    Internal,
    Package,
}

/// Function or method parameter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub param_type: Option<String>,
    pub default_value: Option<String>,
    pub is_optional: bool,
    pub is_variadic: bool,
    pub attributes: Vec<Attribute>,
}

/// Type parameter (generics).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TypeParameter {
    pub name: String,
    pub bounds: Vec<String>,
    pub default_type: Option<String>,
    pub variance: Option<Variance>,
}

/// Variance for type parameters.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Variance {
    Covariant,
    Contravariant,
    Invariant,
}

/// Attribute/Annotation/Decorator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub arguments: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Code complexity metrics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Complexity {
    pub cyclomatic: u32,
    pub cognitive: u32,
    pub nesting: u32,
    pub lines: u32,
    pub parameters: u32,
    pub returns: u32,
}

impl Default for Complexity {
    fn default() -> Self {
        Self {
            cyclomatic: 1,
            cognitive: 1,
            nesting: 0,
            lines: 0,
            parameters: 0,
            returns: 0,
        }
    }
}

/// Complete code unit with all metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeUnit {
    // Identity
    pub id: CortexId,
    pub unit_type: CodeUnitType,
    pub name: String,
    pub qualified_name: String,
    pub display_name: String,
    pub file_path: String,
    pub language: Language,

    // Location
    pub start_line: usize,
    pub end_line: usize,
    pub start_column: usize,
    pub end_column: usize,
    pub start_byte: usize,
    pub end_byte: usize,

    // Content
    pub signature: String,
    pub body: Option<String>,
    pub docstring: Option<String>,
    pub comments: Vec<String>,

    // Type information
    pub return_type: Option<String>,
    pub parameters: Vec<Parameter>,
    pub type_parameters: Vec<TypeParameter>,
    pub generic_constraints: Vec<String>,
    pub throws: Vec<String>,

    // Metadata
    pub visibility: Visibility,
    pub attributes: Vec<Attribute>,
    pub modifiers: Vec<String>,
    pub is_async: bool,
    pub is_unsafe: bool,
    pub is_const: bool,
    pub is_static: bool,
    pub is_abstract: bool,
    pub is_virtual: bool,
    pub is_override: bool,
    pub is_final: bool,
    pub is_exported: bool,
    pub is_default_export: bool,

    // Metrics
    pub complexity: Complexity,
    pub test_coverage: Option<f64>,
    pub has_tests: bool,
    pub has_documentation: bool,

    // Language-specific metadata
    pub language_specific: HashMap<String, serde_json::Value>,

    // Embedding
    pub embedding: Option<Vec<f32>>,
    pub embedding_model: Option<String>,

    // Semantic analysis
    pub summary: Option<String>,
    pub purpose: Option<String>,

    // Tree-sitter AST reference
    pub ast_node_type: Option<String>,
    pub ast_metadata: Option<serde_json::Value>,

    // Versioning and status
    pub status: CodeUnitStatus,
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_by: String,

    // Additional metadata
    pub tags: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Status of a code unit.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CodeUnitStatus {
    Active,
    Deprecated,
    Deleted,
    Moved,
}

impl CodeUnit {
    /// Create a new code unit with minimal required fields.
    pub fn new(
        unit_type: CodeUnitType,
        name: String,
        qualified_name: String,
        file_path: String,
        language: Language,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: CortexId::new(),
            unit_type,
            name: name.clone(),
            qualified_name,
            display_name: name,
            file_path,
            language,
            start_line: 0,
            end_line: 0,
            start_column: 0,
            end_column: 0,
            start_byte: 0,
            end_byte: 0,
            signature: String::new(),
            body: None,
            docstring: None,
            comments: Vec::new(),
            return_type: None,
            parameters: Vec::new(),
            type_parameters: Vec::new(),
            generic_constraints: Vec::new(),
            throws: Vec::new(),
            visibility: Visibility::Private,
            attributes: Vec::new(),
            modifiers: Vec::new(),
            is_async: false,
            is_unsafe: false,
            is_const: false,
            is_static: false,
            is_abstract: false,
            is_virtual: false,
            is_override: false,
            is_final: false,
            is_exported: false,
            is_default_export: false,
            complexity: Complexity::default(),
            test_coverage: None,
            has_tests: false,
            has_documentation: false,
            language_specific: HashMap::new(),
            embedding: None,
            embedding_model: None,
            summary: None,
            purpose: None,
            ast_node_type: None,
            ast_metadata: None,
            status: CodeUnitStatus::Active,
            version: 1,
            created_at: now,
            updated_at: now,
            created_by: "system".to_string(),
            updated_by: "system".to_string(),
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Mark as deprecated.
    pub fn deprecate(&mut self) {
        self.status = CodeUnitStatus::Deprecated;
        self.updated_at = Utc::now();
    }

    /// Check if unit is a function-like construct.
    pub fn is_callable(&self) -> bool {
        matches!(
            self.unit_type,
            CodeUnitType::Function
                | CodeUnitType::Method
                | CodeUnitType::AsyncFunction
                | CodeUnitType::Generator
                | CodeUnitType::Lambda
        )
    }

    /// Check if unit is a type definition.
    pub fn is_type_definition(&self) -> bool {
        matches!(
            self.unit_type,
            CodeUnitType::Class
                | CodeUnitType::Struct
                | CodeUnitType::Enum
                | CodeUnitType::Union
                | CodeUnitType::Interface
                | CodeUnitType::Trait
                | CodeUnitType::TypeAlias
                | CodeUnitType::Typedef
        )
    }

    /// Check if unit is a test.
    pub fn is_test(&self) -> bool {
        matches!(
            self.unit_type,
            CodeUnitType::Test | CodeUnitType::Benchmark
        )
    }

    /// Calculate complexity score (0.0 - 1.0, higher = more complex).
    pub fn complexity_score(&self) -> f64 {
        let cyclo_score = (self.complexity.cyclomatic as f64 / 50.0).min(1.0);
        let cognitive_score = (self.complexity.cognitive as f64 / 100.0).min(1.0);
        let nesting_score = (self.complexity.nesting as f64 / 10.0).min(1.0);
        let lines_score = (self.complexity.lines as f64 / 500.0).min(1.0);

        (cyclo_score * 0.3 + cognitive_score * 0.4 + nesting_score * 0.2 + lines_score * 0.1)
            .min(1.0)
    }

    /// Check if unit needs documentation.
    pub fn needs_documentation(&self) -> bool {
        self.visibility == Visibility::Public && !self.has_documentation
    }

    /// Check if unit needs tests.
    pub fn needs_tests(&self) -> bool {
        self.is_callable() && !self.has_tests && self.visibility == Visibility::Public
    }
}
