//! Environment setup and command execution utilities.
//!
//! This module provides functions for creating properly configured Command instances
//! that inherit necessary environment variables and paths for Claude Code execution.

use std::process::Command;

/// Create a Command with proper environment variables.
///
/// This function ensures that commands can find Node.js, NVM installations,
/// Homebrew installations, and respect proxy settings.
///
/// # Arguments
///
/// * `program` - Path to the program to execute
///
/// # Returns
///
/// A configured `Command` instance with appropriate environment variables
///
/// # Examples
///
/// ```no_run
/// use cc_sdk::binary::create_command_with_env;
///
/// let mut cmd = create_command_with_env("/usr/local/bin/claude");
/// cmd.arg("--version");
/// let output = cmd.output().expect("Failed to execute");
/// ```
pub fn create_command_with_env(program: &str) -> Command {
    let mut cmd = Command::new(program);

    tracing::info!("Creating command for: {}", program);

    // Inherit essential environment variables from parent process
    for (key, value) in std::env::vars() {
        // Pass through PATH and other essential environment variables
        if is_essential_env_var(&key) {
            tracing::debug!("Inheriting env var: {}={}", key, value);
            cmd.env(&key, &value);
        }
    }

    // Setup platform-specific environment
    setup_platform_env(&mut cmd, program);

    // Setup proxy environment
    setup_proxy_env(&mut cmd);

    cmd
}

/// Check if an environment variable is essential and should be inherited.
fn is_essential_env_var(key: &str) -> bool {
    matches!(
        key,
        "PATH"
            | "HOME"
            | "USER"
            | "SHELL"
            | "LANG"
            | "LC_ALL"
            | "NODE_PATH"
            | "NVM_DIR"
            | "NVM_BIN"
            | "HOMEBREW_PREFIX"
            | "HOMEBREW_CELLAR"
            | "HTTP_PROXY"
            | "HTTPS_PROXY"
            | "NO_PROXY"
            | "ALL_PROXY"
    ) || key.starts_with("LC_")
}

/// Setup platform-specific environment variables.
#[cfg(unix)]
fn setup_platform_env(cmd: &mut Command, program: &str) {
    // Add NVM support if the program is in an NVM directory
    if program.contains("/.nvm/versions/node/") {
        setup_nvm_env(cmd, program);
    }

    // Add Homebrew support if the program is in a Homebrew directory
    if program.contains("/homebrew/") || program.contains("/opt/homebrew/") {
        setup_homebrew_env(cmd, program);
    }
}

#[cfg(windows)]
fn setup_platform_env(cmd: &mut Command, program: &str) {
    // Add NVM support for Windows
    if let Ok(nvm_home) = std::env::var("NVM_HOME") {
        let current_path = std::env::var("PATH").unwrap_or_default();
        if !current_path.contains(&nvm_home) {
            let new_path = format!("{};{}", nvm_home, current_path);
            tracing::debug!("Adding NVM_HOME to PATH: {}", nvm_home);
            cmd.env("PATH", new_path);
        }
    }
}

/// Setup NVM environment variables for Unix systems.
#[cfg(unix)]
fn setup_nvm_env(cmd: &mut Command, program: &str) {
    if let Some(node_bin_dir) = std::path::Path::new(program).parent() {
        // Ensure the Node.js bin directory is in PATH
        let current_path = std::env::var("PATH").unwrap_or_default();
        let node_bin_str = node_bin_dir.to_string_lossy();
        if !current_path.contains(&node_bin_str.as_ref()) {
            let new_path = format!("{}:{}", node_bin_str, current_path);
            tracing::debug!("Adding NVM bin directory to PATH: {}", node_bin_str);
            cmd.env("PATH", new_path);
        }

        // Set NVM_BIN if not already set
        if std::env::var("NVM_BIN").is_err() {
            tracing::debug!("Setting NVM_BIN: {}", node_bin_str);
            cmd.env("NVM_BIN", node_bin_str.as_ref());
        }
    }
}

/// Setup Homebrew environment variables for Unix systems.
#[cfg(unix)]
fn setup_homebrew_env(cmd: &mut Command, program: &str) {
    if let Some(program_dir) = std::path::Path::new(program).parent() {
        // Ensure the Homebrew bin directory is in PATH
        let current_path = std::env::var("PATH").unwrap_or_default();
        let homebrew_bin_str = program_dir.to_string_lossy();
        if !current_path.contains(&homebrew_bin_str.as_ref()) {
            let new_path = format!("{}:{}", homebrew_bin_str, current_path);
            tracing::debug!(
                "Adding Homebrew bin directory to PATH: {}",
                homebrew_bin_str
            );
            cmd.env("PATH", new_path);
        }

        // Set HOMEBREW_PREFIX if not already set (for Apple Silicon Macs)
        if program.contains("/opt/homebrew/") && std::env::var("HOMEBREW_PREFIX").is_err() {
            tracing::debug!("Setting HOMEBREW_PREFIX: /opt/homebrew");
            cmd.env("HOMEBREW_PREFIX", "/opt/homebrew");
        }
    }
}

/// Setup proxy environment variables.
///
/// This function logs proxy settings for debugging and ensures they're
/// properly configured in the command environment.
fn setup_proxy_env(cmd: &mut Command) {
    tracing::info!("Command will use proxy settings:");

    if let Ok(http_proxy) = std::env::var("HTTP_PROXY") {
        tracing::info!("  HTTP_PROXY={}", http_proxy);
        cmd.env("HTTP_PROXY", &http_proxy);
    }

    if let Ok(https_proxy) = std::env::var("HTTPS_PROXY") {
        tracing::info!("  HTTPS_PROXY={}", https_proxy);
        cmd.env("HTTPS_PROXY", &https_proxy);
    }

    if let Ok(no_proxy) = std::env::var("NO_PROXY") {
        tracing::info!("  NO_PROXY={}", no_proxy);
        cmd.env("NO_PROXY", &no_proxy);
    }

    if let Ok(all_proxy) = std::env::var("ALL_PROXY") {
        tracing::info!("  ALL_PROXY={}", all_proxy);
        cmd.env("ALL_PROXY", &all_proxy);
    }
}

/// Get the Claude version from a binary path.
///
/// # Arguments
///
/// * `path` - Path to the Claude binary
///
/// # Returns
///
/// * `Ok(Some(String))` - Version string if successfully retrieved
/// * `Ok(None)` - Binary exists but version couldn't be determined
/// * `Err(String)` - Error message if execution failed
///
/// # Examples
///
/// ```no_run
/// use cc_sdk::binary::get_claude_version;
///
/// match get_claude_version("/usr/local/bin/claude") {
///     Ok(Some(version)) => println!("Claude version: {}", version),
///     Ok(None) => println!("Version not available"),
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
pub fn get_claude_version(path: &str) -> Result<Option<String>, String> {
    let mut cmd = create_command_with_env(path);
    cmd.arg("--version");

    match cmd.output() {
        Ok(output) => {
            if output.status.success() {
                Ok(super::version::extract_version_from_output(&output.stdout))
            } else {
                tracing::warn!(
                    "Failed to get version for {}: non-zero exit code",
                    path
                );
                Ok(None)
            }
        }
        Err(e) => {
            tracing::warn!("Failed to execute version command for {}: {}", path, e);
            Err(format!("Failed to execute {}: {}", path, e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_essential_env_var() {
        assert!(is_essential_env_var("PATH"));
        assert!(is_essential_env_var("HOME"));
        assert!(is_essential_env_var("HTTP_PROXY"));
        assert!(is_essential_env_var("LC_ALL"));
        assert!(is_essential_env_var("LC_CTYPE"));
        assert!(!is_essential_env_var("RANDOM_VAR"));
    }

    #[test]
    fn test_create_command_with_env() {
        let cmd = create_command_with_env("claude");
        // Basic test to ensure command is created
        // More comprehensive testing would require mocking
        assert_eq!(cmd.get_program(), "claude");
    }

    #[test]
    #[cfg(unix)]
    fn test_nvm_path_detection() {
        let program = "/home/user/.nvm/versions/node/v18.0.0/bin/claude";
        let mut cmd = Command::new(program);
        setup_nvm_env(&mut cmd, program);
        // The command should have NVM paths configured
        // Actual verification would require checking environment
    }

    #[test]
    #[cfg(unix)]
    fn test_homebrew_path_detection() {
        let program = "/opt/homebrew/bin/claude";
        let mut cmd = Command::new(program);
        setup_homebrew_env(&mut cmd, program);
        // The command should have Homebrew paths configured
    }
}
