//! API route definitions

pub mod workspaces;
pub mod vfs;
pub mod sessions;
pub mod search;
pub mod memory;
pub mod health;
pub mod units;
pub mod dependencies;
pub mod build;
pub mod auth;
pub mod dashboard;
pub mod tasks;
pub mod export;
pub mod documents;

pub use workspaces::workspace_routes;
pub use vfs::vfs_routes;
pub use sessions::session_routes;
pub use search::search_routes;
pub use memory::memory_routes;
pub use health::health_routes;
pub use units::code_unit_routes;
pub use dependencies::dependency_routes;
pub use build::build_routes;
pub use auth::{auth_routes, public_auth_routes, protected_auth_routes, AuthContext};
pub use dashboard::dashboard_routes;
pub use tasks::task_routes;
pub use export::export_routes;
pub use documents::{document_routes, DocumentContext};
