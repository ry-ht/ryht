//! Legacy error module (DEPRECATED)
//!
//! **⚠️ This module is deprecated and will be removed in v0.4.0.**
//!
//! All error types have been consolidated into the `error` module (singular).
//! Please update your imports:
//!
//! ```rust
//! // Old (deprecated)
//! use cc_sdk::errors::{Error, SdkError, BinaryError};
//!
//! // New (recommended)
//! use cc_sdk::error::{Error, BinaryError};
//! use cc_sdk::Result;  // Modern result type
//! ```
//!
//! # Migration Guide
//!
//! ## Import Changes
//!
//! | Old Import | New Import |
//! |-----------|-----------|
//! | `use cc_sdk::errors::Error;` | `use cc_sdk::error::Error;` |
//! | `use cc_sdk::errors::SdkError;` | `use cc_sdk::Error;` (re-exported at crate root) |
//! | `use cc_sdk::errors::Result;` | `use cc_sdk::Result;` (re-exported at crate root) |
//! | `use cc_sdk::errors::BinaryError;` | `use cc_sdk::error::BinaryError;` |
//! | `use cc_sdk::errors::TransportError;` | `use cc_sdk::error::TransportError;` |
//! | `use cc_sdk::errors::SessionError;` | `use cc_sdk::error::SessionError;` |
//! | `use cc_sdk::errors::SettingsError;` | `use cc_sdk::error::SettingsError;` |
//! | `use cc_sdk::errors::ClientError;` | `use cc_sdk::error::ClientError;` |
//!
//! ## Type Alias Deprecations
//!
//! - `SdkError` → Use `Error` instead
//! - `errors::Result<T>` → Use `crate::Result<T>` instead
//!
//! ## No Code Changes Needed
//!
//! The error types themselves are unchanged. Only the module location has changed.
//! Simply update your import statements and your code will work as before.
//!
//! ## Why This Change?
//!
//! 1. **Singular module name** - Follows Rust conventions (`error` vs `errors`)
//! 2. **Single source of truth** - All error types in one canonical location
//! 3. **Cleaner exports** - Simpler re-export structure in `lib.rs`
//! 4. **Better organization** - Matches standard library patterns (`std::error`)

#![deprecated(
    since = "0.3.0",
    note = "Use the error module (singular) instead. Import from `cc_sdk::error::*`."
)]

// Re-export everything from the modern error module for backward compatibility
#[allow(deprecated)]
pub use crate::error::*;
