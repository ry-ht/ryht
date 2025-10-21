//! Modern error module (Phase 1).
//!
//! This module provides the modernized error hierarchy with comprehensive
//! domain-specific errors and better ergonomics.
//!
//! # Migration from Legacy Errors
//!
//! The legacy `SdkError` type is being replaced with the new `Error` type
//! which provides better categorization and more actionable error messages.
//!
//! ```rust
//! // Old way (deprecated)
//! use cc_sdk::SdkError;
//!
//! // New way (Phase 1)
//! use cc_sdk::Error;
//! use cc_sdk::error::{BinaryError, TransportError};
//! ```

// Re-export everything from errors module
pub use crate::errors::*;
