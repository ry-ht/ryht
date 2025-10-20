// Task Management Module
//
// Provides MCP-native task management with RocksDB persistence,
// memory integration, and specification linking.

pub mod dependency_resolver;
pub mod manager;
pub mod storage;
pub mod types;

#[cfg(test)]
mod tests;

pub use dependency_resolver::{DependencyResolver, ExecutionPlan};
pub use manager::TaskManager;
pub use storage::TaskStorage;
pub use types::*;
