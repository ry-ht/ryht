use super::{Hash, Location, Reference, SymbolId, TokenCount};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Code symbol (function, class, interface, variable, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSymbol {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
    pub signature: String,
    pub body_hash: Hash,
    pub location: Location,
    pub references: Vec<Reference>,
    pub dependencies: Vec<SymbolId>,
    pub metadata: SymbolMetadata,
    /// Vector embedding for semantic search (384-dimensional for AllMiniLML6V2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Interface,
    Struct,
    Enum,
    Type,
    Variable,
    Constant,
    Module,
    Namespace,
    Trait,
}

impl SymbolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Method => "method",
            SymbolKind::Class => "class",
            SymbolKind::Interface => "interface",
            SymbolKind::Struct => "struct",
            SymbolKind::Enum => "enum",
            SymbolKind::Type => "type",
            SymbolKind::Variable => "variable",
            SymbolKind::Constant => "constant",
            SymbolKind::Module => "module",
            SymbolKind::Namespace => "namespace",
            SymbolKind::Trait => "trait",
        }
    }

    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "function" | "fn" => Some(SymbolKind::Function),
            "method" => Some(SymbolKind::Method),
            "class" => Some(SymbolKind::Class),
            "interface" => Some(SymbolKind::Interface),
            "struct" => Some(SymbolKind::Struct),
            "enum" => Some(SymbolKind::Enum),
            "type" => Some(SymbolKind::Type),
            "variable" | "var" => Some(SymbolKind::Variable),
            "constant" | "const" => Some(SymbolKind::Constant),
            "module" | "mod" => Some(SymbolKind::Module),
            "namespace" | "ns" => Some(SymbolKind::Namespace),
            "trait" => Some(SymbolKind::Trait),
            _ => None,
        }
    }
}

/// Metadata about a symbol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolMetadata {
    pub complexity: u32,
    pub token_cost: TokenCount,
    pub last_modified: Option<DateTime<Utc>>,
    pub authors: Vec<String>,
    pub doc_comment: Option<String>,
    pub test_coverage: f32,
    pub usage_frequency: u32,
}

impl Default for SymbolMetadata {
    fn default() -> Self {
        Self {
            complexity: 0,
            token_cost: TokenCount::zero(),
            last_modified: None,
            authors: Vec::new(),
            doc_comment: None,
            test_coverage: 0.0,
            usage_frequency: 0,
        }
    }
}

/// Symbol definition with optional body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolDefinition {
    pub symbol: CodeSymbol,
    pub body: Option<String>,
    pub dependencies: Vec<CodeSymbol>,
}
