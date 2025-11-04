//! Re-exports of Cortex types for agent use
//!
//! This module provides a unified import point for types that agents need from
//! the Cortex ecosystem.

// Re-export core types
pub use cortex_core::types::{CodeUnit, SearchResult};
pub use cortex_intelligence::PatternType;
pub use crate::AgentId;
pub use crate::stub_types::MergeStrategy;

// Type alias for CodeSearchResult
pub type CodeSearchResult = SearchResult<CodeUnit>;

// Re-export models module for compatibility
pub mod models {
    pub use crate::AgentId;
    pub use cortex_intelligence::PatternType;
}
