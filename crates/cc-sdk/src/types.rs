//! Type definitions for the Claude Code SDK
//!
//! This module re-exports all core types from focused submodules.
//! The types have been organized into logical modules for better maintainability:
//!
//! - [`messages`] - Message and content types
//! - [`options`] - Configuration and builder types
//! - [`permissions`] - Permission-related types
//! - [`hooks`] - Hook system types and traits
//! - [`requests`] - SDK Control Protocol types
//!
//! # Modern Types (Phase 1)
//!
//! The SDK is being modernized with stronger typing and better ergonomics.
//! New code should prefer the modern types in the `core` module over
//! the legacy types defined at the module root.
//!
//! ## Type Safety
//!
//! - Use newtypes (`SessionId`, `BinaryPath`, `ModelId`) instead of raw strings
//! - Use strongly-typed enums instead of string constants
//! - Leverage the type-state pattern for compile-time safety
//!
//! ## Migration
//!
//! Legacy types are maintained for backward compatibility but will be
//! deprecated in future versions.

#![allow(missing_docs)]

// Re-export core types for convenience
pub use crate::core::{BinaryPath, ModelId, SessionId, Version};

// Re-export from submodules
pub use crate::messages::*;
pub use crate::options::*;
pub use crate::permissions::*;
pub use crate::hooks::*;
pub use crate::requests::*;
