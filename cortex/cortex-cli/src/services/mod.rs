//! Unified service layer for cortex-cli
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

pub use workspace::WorkspaceService;
pub use vfs::VfsService;
pub use search::SearchService;
pub use memory::MemoryService;
pub use code_units::CodeUnitService;
pub use dependencies::DependencyService;
