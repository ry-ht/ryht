pub mod symbol;
pub mod episode;
pub mod context;
pub mod query;
pub mod session;

pub use symbol::*;
pub use episode::*;
pub use context::*;
pub use query::*;
pub use session::*;

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for symbols
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SymbolId(pub String);

impl SymbolId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl fmt::Display for SymbolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for episodes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EpisodeId(pub String);

impl EpisodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for EpisodeId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for sessions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Location in source code
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Location {
    pub file: String,
    pub line_start: usize,
    pub line_end: usize,
    pub column_start: usize,
    pub column_end: usize,
}

impl Location {
    pub fn new(
        file: String,
        line_start: usize,
        line_end: usize,
        column_start: usize,
        column_end: usize,
    ) -> Self {
        Self {
            file,
            line_start,
            line_end,
            column_start,
            column_end,
        }
    }
}

/// Level of detail for code representation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum DetailLevel {
    /// Only structure: names and signatures (100-500 tokens)
    Skeleton,
    /// + public interfaces (500-1500 tokens)
    #[default]
    Interface,
    /// + private implementation (2000+ tokens)
    Implementation,
    /// Full code with comments
    Full,
}

/// Task outcome
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Outcome {
    Success,
    Failure,
    Partial,
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Outcome::Success => write!(f, "success"),
            Outcome::Failure => write!(f, "failure"),
            Outcome::Partial => write!(f, "partial"),
        }
    }
}

/// Reference to a symbol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    pub symbol_id: SymbolId,
    pub location: Location,
    pub kind: ReferenceKind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReferenceKind {
    Import,
    Call,
    Instantiation,
    TypeReference,
    Implementation,
}

/// Hash of content
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hash(pub String);

impl Hash {
    pub fn new(data: &[u8]) -> Self {
        let hash = blake3::hash(data);
        Self(hash.to_hex().to_string())
    }

    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

/// Token count
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct TokenCount(pub u32);

impl TokenCount {
    pub fn new(count: u32) -> Self {
        Self(count)
    }

    pub fn zero() -> Self {
        Self(0)
    }

    pub fn add(&mut self, other: TokenCount) {
        self.0 += other.0;
    }
}

impl fmt::Display for TokenCount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} tokens", self.0)
    }
}

impl From<usize> for TokenCount {
    fn from(count: usize) -> Self {
        Self(count as u32)
    }
}

impl From<TokenCount> for usize {
    fn from(count: TokenCount) -> Self {
        count.0 as usize
    }
}
