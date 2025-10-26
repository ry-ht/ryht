//! Unified service layer for cortex
//!
//! This module provides a shared service layer that eliminates duplication
//! between API and MCP modules. Each service encapsulates common business logic
//! and data access patterns.

pub mod workspace;
pub mod vfs;
pub mod search;
pub mod memory;
pub mod code_units;
pub mod dependencies;
pub mod auth;
pub mod sessions;
pub mod build;

pub use workspace::WorkspaceService;
pub use vfs::VfsService;
pub use search::SearchService;
pub use memory::MemoryService;
pub use code_units::CodeUnitService;
pub use dependencies::DependencyService;
pub use auth::AuthService;
pub use sessions::SessionService;
pub use build::BuildService;

#[cfg(test)]
mod tests;
