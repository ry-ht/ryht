//! Result type alias for the Claude Code SDK.
//!
//! This module provides a convenient `Result` type alias that uses the SDK's
//! error type by default, reducing boilerplate in function signatures.
//!
//! # Examples
//!
//! ```rust
//! use cc_sdk::Result;
//!
//! fn my_function() -> Result<String> {
//!     Ok("success".to_string())
//! }
//! ```

/// Result type alias for SDK operations.
///
/// This is a convenience alias for `std::result::Result<T, crate::Error>`,
/// where `crate::Error` is the top-level error type for the SDK.
///
/// # Examples
///
/// ```rust
/// use cc_sdk::Result;
/// use cc_sdk::error::Error;
///
/// fn parse_config() -> Result<String> {
///     Ok("config".to_string())
/// }
///
/// fn with_custom_error() -> Result<i32, std::io::Error> {
///     Ok(42)
/// }
/// ```
pub type Result<T, E = crate::Error> = std::result::Result<T, E>;
