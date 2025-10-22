//! Claude binary discovery using cc-sdk
//!
//! This module integrates cc-sdk's binary discovery functionality with Tauri's
//! database system for storing and retrieving Claude binary paths and preferences.

use anyhow::Result;
use log::{error, info, warn};
use std::path::PathBuf;
use std::process::Command;
use tauri::Manager;

// Re-export cc-sdk types for backward compatibility
pub use cc_sdk::binary::ClaudeInstallation;

/// Main function to find the Claude binary
/// Checks database first for stored path and preference, then uses cc-sdk for discovery
pub fn find_claude_binary(app_handle: &tauri::AppHandle) -> Result<String, String> {
    info!("Searching for claude binary...");

    // First check if we have a stored path in the database
    if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
        let db_path = app_data_dir.join("agents.db");
        if db_path.exists() {
            if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                // Check for stored path first
                if let Ok(stored_path) = conn.query_row(
                    "SELECT value FROM app_settings WHERE key = 'claude_binary_path'",
                    [],
                    |row| row.get::<_, String>(0),
                ) {
                    info!("Found stored claude path in database: {}", stored_path);

                    // Check if the path still exists
                    let path_buf = PathBuf::from(&stored_path);
                    if path_buf.exists() && path_buf.is_file() {
                        return Ok(stored_path);
                    } else {
                        warn!("Stored claude path no longer exists: {}", stored_path);
                    }
                }

                // Check user preference (currently not used for filtering, but logged)
                if let Ok(preference) = conn.query_row(
                    "SELECT value FROM app_settings WHERE key = 'claude_installation_preference'",
                    [],
                    |row| row.get::<_, String>(0),
                ) {
                    info!("User preference for Claude installation: {}", preference);
                }
            }
        }
    }

    // Use cc-sdk for discovery
    match cc_sdk::binary::find_claude_binary() {
        Ok(path) => {
            info!("cc-sdk found Claude at: {}", path);

            // Store the discovered path in database for faster lookup next time
            if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
                let db_path = app_data_dir.join("agents.db");
                if db_path.exists() {
                    if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                        let _ = conn.execute(
                            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('claude_binary_path', ?1)",
                            [&path],
                        );
                    }
                }
            }

            Ok(path)
        }
        Err(e) => {
            error!("Could not find claude binary: {}", e);
            Err(e)
        }
    }
}

/// Discovers all available Claude installations using cc-sdk
/// Returns them sorted by version and source preference
pub fn discover_claude_installations() -> Vec<ClaudeInstallation> {
    info!("Discovering all Claude installations using cc-sdk...");
    cc_sdk::binary::discover_installations()
}

/// Helper function to create a Command with proper environment variables
/// This ensures commands like Claude can find Node.js and other dependencies
pub fn create_command_with_env(program: &str) -> Command {
    info!("Creating command for: {}", program);

    // Use cc-sdk's environment setup which handles:
    // - PATH configuration
    // - NVM support (NVM_DIR, NVM_BIN)
    // - Homebrew paths
    // - Proxy settings (HTTP_PROXY, HTTPS_PROXY, NO_PROXY, ALL_PROXY)
    // - Essential environment variables (HOME, USER, SHELL, etc.)
    cc_sdk::binary::create_command_with_env(program)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_installations() {
        // This test may return empty results if Claude isn't installed
        // but should not panic
        let installations = discover_claude_installations();
        println!("Found {} installations", installations.len());

        for install in installations {
            println!("  {} (version: {:?}, source: {})",
                install.path, install.version, install.source);
        }
    }

    #[test]
    fn test_create_command() {
        // Test that we can create a command
        let cmd = create_command_with_env("claude");
        println!("Created command for 'claude'");

        // Verify it's a Command (compilation check)
        let _cmd: Command = cmd;
    }
}
