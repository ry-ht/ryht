//! Global architecture components for multi-monorepo support
//!
//! This module implements the global architecture as specified in global-architecture-spec.md,
//! providing:
//! - Identity-based project IDs (not path-based)
//! - Global project registry
//! - Cross-monorepo dependency resolution
//! - File watching and synchronization

pub mod identity;
pub mod registry;
pub mod storage;
pub mod dependencies;
pub mod watcher;
pub mod sync;

pub use identity::{ProjectIdentity, ProjectType};
pub use registry::{ProjectRegistry, ProjectRegistryManager, ProjectStatus, PathHistoryEntry};
pub use storage::GlobalStorage;
pub use dependencies::{DependencyGraph, DependencyGraphManager, DependencyType, DependencyEdge, ProjectNode};
pub use watcher::{GlobalFileWatcher, WatcherConfig, FileChangeEvent, FileChangeKind, WatcherStats};
pub use sync::{SyncManager, SyncResult, SyncDirection, SyncStats};
