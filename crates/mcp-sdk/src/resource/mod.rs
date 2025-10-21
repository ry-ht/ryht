//! Resource management for the MCP server framework.
//!
//! This module provides traits, types, and utilities for registering and serving
//! resources in an MCP server. Resources represent URI-addressable content that
//! can be read by MCP clients.
//!
//! # Overview
//!
//! The resource system consists of:
//! - [`Resource`] trait - Define custom resources
//! - [`ResourceRegistry`] - Thread-safe registry for managing resources
//! - [`ResourceContent`] - Enum for text and binary content
//! - [`ResourceContext`] - Context passed to resource read operations
//! - URI pattern matching with glob support
//!
//! # Examples
//!
//! ## Basic Static Resource
//!
//! ```rust
//! use mcp_server::resource::{Resource, ResourceContent, ResourceContext};
//! use mcp_server::error::ResourceError;
//! use async_trait::async_trait;
//!
//! struct ConfigResource;
//!
//! #[async_trait]
//! impl Resource for ConfigResource {
//!     fn uri_pattern(&self) -> &str {
//!         "app://config"
//!     }
//!
//!     fn name(&self) -> Option<&str> {
//!         Some("Application Config")
//!     }
//!
//!     fn description(&self) -> Option<&str> {
//!         Some("Application configuration file")
//!     }
//!
//!     fn mime_type(&self) -> Option<&str> {
//!         Some("application/json")
//!     }
//!
//!     async fn read(&self, _uri: &str, _context: &ResourceContext) -> Result<ResourceContent, ResourceError> {
//!         Ok(ResourceContent::text(
//!             r#"{"name": "my-app", "version": "1.0.0"}"#,
//!             "application/json"
//!         ))
//!     }
//! }
//! ```
//!
//! ## Dynamic Resource with Wildcards
//!
//! ```rust
//! use mcp_server::resource::{Resource, ResourceContent, ResourceContext};
//! use mcp_server::error::ResourceError;
//! use async_trait::async_trait;
//!
//! struct UserResource;
//!
//! #[async_trait]
//! impl Resource for UserResource {
//!     fn uri_pattern(&self) -> &str {
//!         "db://users/*"
//!     }
//!
//!     fn name(&self) -> Option<&str> {
//!         Some("User Database")
//!     }
//!
//!     async fn read(&self, uri: &str, _context: &ResourceContext) -> Result<ResourceContent, ResourceError> {
//!         // Extract user ID from URI
//!         let user_id = uri.strip_prefix("db://users/")
//!             .ok_or_else(|| ResourceError::InvalidUri(uri.to_string()))?;
//!
//!         // Fetch user data (simulated)
//!         let user_json = format!(r#"{{"id": "{}", "name": "User {}" }}"#, user_id, user_id);
//!
//!         Ok(ResourceContent::text(user_json, "application/json"))
//!     }
//! }
//! ```
//!
//! ## Using the Resource Registry
//!
//! ```rust
//! use mcp_server::resource::ResourceRegistry;
//! # use mcp_server::resource::{Resource, ResourceContent, ResourceContext};
//! # use mcp_server::error::ResourceError;
//! # use async_trait::async_trait;
//! #
//! # struct ConfigResource;
//! # #[async_trait]
//! # impl Resource for ConfigResource {
//! #     fn uri_pattern(&self) -> &str { "app://config" }
//! #     async fn read(&self, _uri: &str, _context: &ResourceContext) -> Result<ResourceContent, ResourceError> {
//! #         Ok(ResourceContent::text("config", "application/json"))
//! #     }
//! # }
//! # struct UserResource;
//! # #[async_trait]
//! # impl Resource for UserResource {
//! #     fn uri_pattern(&self) -> &str { "db://users/*" }
//! #     async fn read(&self, _uri: &str, _context: &ResourceContext) -> Result<ResourceContent, ResourceError> {
//! #         Ok(ResourceContent::text("user", "application/json"))
//! #     }
//! # }
//!
//! # async fn example() {
//! let registry = ResourceRegistry::new();
//!
//! // Register resources
//! registry.register(ConfigResource).await;
//! registry.register(UserResource).await;
//!
//! // Find resource by URI
//! let resource = registry.find_by_uri("db://users/123").await;
//! assert!(resource.is_some());
//!
//! // List all resources
//! let definitions = registry.list().await;
//! assert_eq!(definitions.len(), 2);
//! # }
//! ```

pub mod content;
pub mod context;
pub mod registry;
pub mod traits;
pub mod uri;

// Re-exports
pub use content::ResourceContent;
pub use context::ResourceContext;
pub use registry::{ResourceDefinition, ResourceRegistry};
pub use traits::Resource;
pub use uri::{UriPattern, matches_pattern};
