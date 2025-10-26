//! Type definitions for the Cortex system.
//!
//! This module provides all core types used across Cortex components.

pub mod core;
pub mod document;

// Re-export all core types
pub use core::*;

// Re-export document types for convenience
pub use document::{
    Document,
    DocumentSection,
    DocumentLink,
    DocumentVersion,
    DocumentType,
    DocumentStatus,
    LinkType,
    LinkTarget,
};
