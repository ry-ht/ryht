//! Binary discovery and management for Claude Code CLI.
//!
//! This module provides comprehensive functionality for discovering and managing
//! Claude Code binary installations across different platforms and installation methods.
//!
//! # Features
//!
//! - **Automatic Discovery**: Finds Claude installations in standard locations
//! - **Version Management**: Parses and compares semantic versions
//! - **Caching**: Caches discovery results for performance
//! - **Platform Support**: Works on Unix (macOS, Linux) and Windows
//! - **Multiple Sources**: Supports system, NVM, Homebrew, npm, yarn, and custom paths
//! - **Environment Setup**: Properly configures command execution environments
//!
//! # Quick Start
//!
//! The simplest way to find Claude is using the [`find_claude_binary`] function:
//!
//! ```no_run
//! use cc_sdk::binary::find_claude_binary;
//!
//! match find_claude_binary() {
//!     Ok(path) => println!("Found Claude at: {}", path),
//!     Err(e) => eprintln!("Error: {}", e),
//! }
//! ```
//!
//! # Discovery Process
//!
//! The discovery process checks locations in this order:
//!
//! 1. `which`/`where` command output
//! 2. NVM directories (`~/.nvm/versions/node/*/bin/claude`)
//! 3. Homebrew paths (`/opt/homebrew/bin/claude`, `/usr/local/bin/claude`)
//! 4. System paths (`/usr/bin/claude`, `/bin/claude`)
//! 5. User-local paths (`~/.local/bin/claude`, `~/.claude/local/claude`)
//! 6. Package manager paths (npm, yarn, bun)
//! 7. Environment variable `CLAUDE_BINARY_PATH`
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```no_run
//! use cc_sdk::binary::find_claude_binary;
//!
//! let claude_path = find_claude_binary()
//!     .expect("Claude Code not found");
//!
//! println!("Using Claude at: {}", claude_path);
//! ```
//!
//! ## Discovering All Installations
//!
//! ```no_run
//! use cc_sdk::binary::discover_installations;
//!
//! let installations = discover_installations();
//! for install in installations {
//!     println!("Found: {} (version: {:?}, source: {})",
//!         install.path, install.version, install.source);
//! }
//! ```
//!
//! ## Custom Discovery
//!
//! ```no_run
//! use cc_sdk::binary::DiscoveryBuilder;
//!
//! let installations = DiscoveryBuilder::new()
//!     .custom_path("/opt/custom/claude")
//!     .skip_nvm(true)
//!     .discover();
//!
//! println!("Found {} installations", installations.len());
//! ```
//!
//! ## Working with Versions
//!
//! ```
//! use cc_sdk::binary::{Version, compare_versions};
//! use std::cmp::Ordering;
//!
//! let v1 = Version::parse("1.0.41").unwrap();
//! let v2 = Version::parse("1.0.40").unwrap();
//! assert!(v1 > v2);
//!
//! assert_eq!(compare_versions("2.0.0", "1.9.9"), Ordering::Greater);
//! ```
//!
//! ## Creating Commands
//!
//! ```no_run
//! use cc_sdk::binary::{find_claude_binary, create_command_with_env};
//!
//! let claude_path = find_claude_binary().unwrap();
//! let mut cmd = create_command_with_env(&claude_path);
//! cmd.arg("--version");
//!
//! let output = cmd.output().expect("Failed to execute");
//! println!("{}", String::from_utf8_lossy(&output.stdout));
//! ```
//!
//! # Environment Variables
//!
//! The module respects several environment variables:
//!
//! - `CLAUDE_BINARY_PATH`: Custom path to Claude binary (highest priority)
//! - `NVM_BIN`: Active NVM binary directory
//! - `NVM_DIR` or `NVM_HOME`: NVM installation directory
//! - `HOMEBREW_PREFIX`: Homebrew installation prefix
//! - `HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY`: Proxy settings
//!
//! # Caching
//!
//! The [`find_claude_binary`] function caches its result after the first call.
//! To force a fresh discovery, use [`discover_installations`] instead.
//!
//! # Platform Differences
//!
//! ## Unix (macOS, Linux)
//!
//! - Uses `which` command
//! - Checks `~/.nvm/versions/node/*/bin/claude`
//! - Supports Homebrew paths
//! - Checks standard Unix paths
//!
//! ## Windows
//!
//! - Uses `where` command
//! - Checks `%NVM_HOME%\*\claude.exe`
//! - Checks `%USERPROFILE%\.local\bin\claude.exe`
//! - Checks npm global directory
//!
//! # Error Handling
//!
//! Functions return `Result<T, String>` with descriptive error messages:
//!
//! ```no_run
//! use cc_sdk::binary::find_claude_binary;
//!
//! match find_claude_binary() {
//!     Ok(path) => {
//!         // Use the path
//!     }
//!     Err(msg) => {
//!         eprintln!("Claude not found: {}", msg);
//!         eprintln!("Please install Claude Code: npm install -g @anthropic-ai/claude-code");
//!     }
//! }
//! ```

mod discovery;
mod env;
mod version;
pub mod cache;

// Re-export main types
pub use discovery::{
    find_claude_binary, discover_installations, ClaudeInstallation,
    DiscoveryBuilder, InstallationType,
};
pub use env::{create_command_with_env, get_claude_version};
pub use version::{
    compare_versions, extract_version_from_output, Version,
};
pub use cache::{DiscoveryCache, CacheConfig};

// Async discovery (optional feature)
#[cfg(feature = "async-discovery")]
pub mod async_discovery {
    //! Async variants of binary discovery functions.
    //!
    //! These functions are useful when you want to avoid blocking the current thread
    //! during binary discovery, which may involve filesystem operations and process
    //! execution.
    //!
    //! # Examples
    //!
    //! ```no_run
    //! use cc_sdk::binary::async_discovery::find_claude_binary_async;
    //!
    //! #[tokio::main]
    //! async fn main() {
    //!     match find_claude_binary_async().await {
    //!         Ok(path) => println!("Found Claude at: {}", path),
    //!         Err(e) => eprintln!("Error: {}", e),
    //!     }
    //! }
    //! ```

    use super::*;

    /// Async version of [`find_claude_binary`].
    ///
    /// Performs binary discovery in a blocking thread pool to avoid blocking
    /// the async runtime.
    pub async fn find_claude_binary_async() -> Result<String, String> {
        tokio::task::spawn_blocking(find_claude_binary)
            .await
            .map_err(|e| format!("Discovery task failed: {}", e))?
    }

    /// Async version of [`discover_installations`].
    ///
    /// Performs comprehensive discovery in a blocking thread pool.
    pub async fn discover_installations_async() -> Vec<ClaudeInstallation> {
        tokio::task::spawn_blocking(discover_installations)
            .await
            .unwrap_or_default()
    }

    /// Async version of [`get_claude_version`].
    ///
    /// Gets the version of a Claude binary without blocking the async runtime.
    pub async fn get_claude_version_async(path: String) -> Result<Option<String>, String> {
        tokio::task::spawn_blocking(move || get_claude_version(&path))
            .await
            .map_err(|e| format!("Version check task failed: {}", e))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Ensure all main types are accessible
        let _ = find_claude_binary();
        let _ = discover_installations();
        let builder = DiscoveryBuilder::new();
        let _ = builder.discover();
    }

    #[test]
    fn test_version_parsing() {
        let v = Version::parse("1.0.41");
        assert!(v.is_some());
    }

    #[test]
    fn test_version_comparison() {
        use std::cmp::Ordering;
        assert_eq!(compare_versions("1.0.41", "1.0.40"), Ordering::Greater);
    }

    #[cfg(feature = "async-discovery")]
    #[tokio::test]
    async fn test_async_discovery() {
        use async_discovery::*;

        // These may fail if Claude isn't installed, but should not panic
        let _ = find_claude_binary_async().await;
        let _ = discover_installations_async().await;
    }
}
