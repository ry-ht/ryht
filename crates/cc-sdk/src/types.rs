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
//! # Type Safety
//!
//! The SDK uses strong typing and better ergonomics throughout:
//!
//! - Use newtypes (`SessionId`, `BinaryPath`, `ModelId`) instead of raw strings
//! - Use strongly-typed enums instead of string constants
//! - Leverage the type-state pattern for compile-time safety

#![allow(missing_docs)]

// Re-export core types for convenience

// Re-export from submodules
pub use crate::messages::*;
pub use crate::options::*;
pub use crate::permissions::*;
pub use crate::hooks::*;
pub use crate::requests::*;
