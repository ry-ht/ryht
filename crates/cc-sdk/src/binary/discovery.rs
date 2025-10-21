//! Binary discovery logic for finding Claude Code installations.
//!
//! This module provides comprehensive discovery of Claude CLI installations
//! across different platforms and installation methods (system, NVM, Homebrew, etc.).

use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use super::cache;
use super::env::get_claude_version;
use super::version::compare_versions;

/// Type of Claude installation source.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InstallationType {
    /// System-installed binary (in standard paths)
    System,
    /// Custom path specified by user
    Custom,
}

/// Represents a discovered Claude installation with metadata.
///
/// # Examples
///
/// ```no_run
/// use cc_sdk::binary::{discover_installations, ClaudeInstallation};
///
/// let installations = discover_installations();
/// for install in installations {
///     println!("Found Claude at: {}", install.path);
///     if let Some(version) = &install.version {
///         println!("  Version: {}", version);
///     }
///     println!("  Source: {}", install.source);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ClaudeInstallation {
    /// Full path to the Claude binary
    pub path: String,
    /// Version string if available
    pub version: Option<String>,
    /// Source of discovery (e.g., "nvm", "system", "homebrew", "which")
    pub source: String,
    /// Type of installation
    pub installation_type: InstallationType,
}

/// Cached result of binary discovery.
static CACHED_BINARY: OnceLock<Option<String>> = OnceLock::new();

/// Find the Claude binary, using cached result if available.
///
/// This function performs discovery once and caches the result for subsequent calls.
/// It returns the best available installation based on version and source priority.
///
/// # Returns
///
/// * `Ok(String)` - Path to the Claude binary
/// * `Err(String)` - Error message describing why no binary was found
///
/// # Examples
///
/// ```no_run
/// use cc_sdk::binary::find_claude_binary;
///
/// match find_claude_binary() {
///     Ok(path) => println!("Found Claude at: {}", path),
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
pub fn find_claude_binary() -> Result<String, String> {
    // Check cache first
    if let Some(cached) = CACHED_BINARY.get() {
        return cached.clone().ok_or_else(|| {
            "No Claude binary found in cache".to_string()
        });
    }

    tracing::info!("Searching for Claude binary...");

    // Discover all available installations
    let installations = discover_system_installations();

    if installations.is_empty() {
        let error_msg = "Claude Code not found. Please ensure it's installed in one of these locations: \
            PATH, /usr/local/bin, /opt/homebrew/bin, ~/.nvm/versions/node/*/bin, \
            ~/.claude/local, ~/.local/bin".to_string();

        tracing::error!("{}", error_msg);

        // Cache the failure
        let _ = CACHED_BINARY.set(None);

        return Err(error_msg);
    }

    // Log all found installations
    for installation in &installations {
        tracing::info!("Found Claude installation: {:?}", installation);
    }

    // Select the best installation
    if let Some(best) = select_best_installation(installations) {
        tracing::info!(
            "Selected Claude installation: path={}, version={:?}, source={}",
            best.path,
            best.version,
            best.source
        );

        let path = best.path.clone();

        // Cache the result
        let _ = CACHED_BINARY.set(Some(path.clone()));

        Ok(path)
    } else {
        let error_msg = "No valid Claude installation found".to_string();
        tracing::error!("{}", error_msg);

        // Cache the failure
        let _ = CACHED_BINARY.set(None);

        Err(error_msg)
    }
}

/// Discover all available Claude installations.
///
/// This function attempts to use cached results if available and valid. If cache is
/// disabled or entries are expired, it performs a fresh discovery and caches the results.
///
/// # Returns
///
/// A vector of `ClaudeInstallation` sorted by preference (best first)
///
/// # Examples
///
/// ```no_run
/// use cc_sdk::binary::discover_installations;
///
/// let installations = discover_installations();
/// println!("Found {} installations", installations.len());
/// for install in installations {
///     println!("  {} (version: {:?}, source: {})",
///         install.path, install.version, install.source);
/// }
/// ```
pub fn discover_installations() -> Vec<ClaudeInstallation> {
    // Try to get from cache first
    if let Some(cached) = cache::get_cached_default() {
        tracing::debug!("Using cached discovery results ({} installations)", cached.len());
        return cached;
    }

    tracing::info!("Discovering all Claude installations...");

    let mut installations = discover_system_installations();

    // Sort by version (highest first), then by source preference
    installations.sort_by(|a, b| {
        match (&a.version, &b.version) {
            (Some(v1), Some(v2)) => {
                // Compare versions in descending order (newest first)
                match compare_versions(v2, v1) {
                    Ordering::Equal => {
                        // If versions are equal, prefer by source
                        source_preference(a).cmp(&source_preference(b))
                    }
                    other => other,
                }
            }
            (Some(_), None) => Ordering::Less, // Version comes before no version
            (None, Some(_)) => Ordering::Greater,
            (None, None) => source_preference(a).cmp(&source_preference(b)),
        }
    });

    // Cache the results
    cache::set_cached_default(installations.clone());

    installations
}

/// Returns a preference score for installation sources (lower is better).
fn source_preference(installation: &ClaudeInstallation) -> u8 {
    match installation.source.as_str() {
        "which" => 1,
        "where" => 1,
        "homebrew" => 2,
        "system" => 3,
        "nvm-active" => 4,
        source if source.starts_with("nvm") => 5,
        "local-bin" => 6,
        "claude-local" => 7,
        "npm-global" => 8,
        "yarn" | "yarn-global" => 9,
        "bun" => 10,
        "node-modules" => 11,
        "home-bin" => 12,
        "PATH" => 13,
        _ => 14,
    }
}

/// Discover all Claude installations on the system.
fn discover_system_installations() -> Vec<ClaudeInstallation> {
    let mut installations = Vec::new();

    // 1. Try 'which' command first (Unix) or 'where' (Windows)
    if let Some(installation) = try_which_command() {
        installations.push(installation);
    }

    // 2. Check NVM paths (includes current active NVM)
    installations.extend(find_nvm_installations());

    // 3. Check standard paths
    installations.extend(find_standard_installations());

    // 4. Check custom environment variable
    if let Ok(custom_path) = std::env::var("CLAUDE_BINARY_PATH") {
        if let Some(installation) = validate_custom_path(&custom_path) {
            installations.push(installation);
        }
    }

    // Remove duplicates by path
    let mut unique_paths = HashSet::new();
    installations.retain(|install| unique_paths.insert(install.path.clone()));

    installations
}

/// Try using the 'which' command (Unix) or 'where' command (Windows) to find Claude.
#[cfg(unix)]
fn try_which_command() -> Option<ClaudeInstallation> {
    tracing::debug!("Trying 'which claude' to find binary...");

    match Command::new("which").arg("claude").output() {
        Ok(output) if output.status.success() => {
            let output_str = String::from_utf8_lossy(&output.stdout).trim().to_string();

            if output_str.is_empty() {
                return None;
            }

            // Parse aliased output: "claude: aliased to /path/to/claude"
            let path = if output_str.starts_with("claude:") && output_str.contains("aliased to") {
                output_str
                    .split("aliased to")
                    .nth(1)
                    .map(|s| s.trim().to_string())
            } else {
                Some(output_str)
            }?;

            tracing::debug!("'which' found claude at: {}", path);

            // Verify the path exists
            if !PathBuf::from(&path).exists() {
                tracing::warn!("Path from 'which' does not exist: {}", path);
                return None;
            }

            // Get version
            let version = get_claude_version(&path).ok().flatten();

            Some(ClaudeInstallation {
                path,
                version,
                source: "which".to_string(),
                installation_type: InstallationType::System,
            })
        }
        _ => None,
    }
}

#[cfg(windows)]
fn try_which_command() -> Option<ClaudeInstallation> {
    tracing::debug!("Trying 'where claude' to find binary...");

    match Command::new("where").arg("claude").output() {
        Ok(output) if output.status.success() => {
            let output_str = String::from_utf8_lossy(&output.stdout).trim().to_string();

            if output_str.is_empty() {
                return None;
            }

            // On Windows, `where` can return multiple paths, newline-separated.
            // We take the first one.
            let path = output_str.lines().next().unwrap_or("").trim().to_string();

            if path.is_empty() {
                return None;
            }

            tracing::debug!("'where' found claude at: {}", path);

            // Verify the path exists
            if !PathBuf::from(&path).exists() {
                tracing::warn!("Path from 'where' does not exist: {}", path);
                return None;
            }

            // Get version
            let version = get_claude_version(&path).ok().flatten();

            Some(ClaudeInstallation {
                path,
                version,
                source: "where".to_string(),
                installation_type: InstallationType::System,
            })
        }
        _ => None,
    }
}

/// Find Claude installations in NVM directories (Unix).
#[cfg(unix)]
fn find_nvm_installations() -> Vec<ClaudeInstallation> {
    let mut installations = Vec::new();

    // First check NVM_BIN environment variable (current active NVM)
    if let Ok(nvm_bin) = std::env::var("NVM_BIN") {
        let claude_path = PathBuf::from(&nvm_bin).join("claude");
        if claude_path.exists() && claude_path.is_file() {
            tracing::debug!("Found Claude via NVM_BIN: {:?}", claude_path);
            let version = get_claude_version(&claude_path.to_string_lossy())
                .ok()
                .flatten();
            installations.push(ClaudeInstallation {
                path: claude_path.to_string_lossy().to_string(),
                version,
                source: "nvm-active".to_string(),
                installation_type: InstallationType::System,
            });
        }
    }

    // Then check all NVM directories
    if let Ok(home) = std::env::var("HOME") {
        let nvm_dir = PathBuf::from(&home)
            .join(".nvm")
            .join("versions")
            .join("node");

        tracing::debug!("Checking NVM directory: {:?}", nvm_dir);

        if let Ok(entries) = std::fs::read_dir(&nvm_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let claude_path = entry.path().join("bin").join("claude");

                    if claude_path.exists() && claude_path.is_file() {
                        let path_str = claude_path.to_string_lossy().to_string();
                        let node_version = entry.file_name().to_string_lossy().to_string();

                        tracing::debug!("Found Claude in NVM node {}: {}", node_version, path_str);

                        // Get Claude version
                        let version = get_claude_version(&path_str).ok().flatten();

                        installations.push(ClaudeInstallation {
                            path: path_str,
                            version,
                            source: format!("nvm ({})", node_version),
                            installation_type: InstallationType::System,
                        });
                    }
                }
            }
        }
    }

    installations
}

/// Find Claude installations in NVM directories (Windows).
#[cfg(windows)]
fn find_nvm_installations() -> Vec<ClaudeInstallation> {
    let mut installations = Vec::new();

    if let Ok(nvm_home) = std::env::var("NVM_HOME") {
        tracing::debug!("Checking NVM_HOME directory: {:?}", nvm_home);

        if let Ok(entries) = std::fs::read_dir(&nvm_home) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let claude_path = entry.path().join("claude.exe");

                    if claude_path.exists() && claude_path.is_file() {
                        let path_str = claude_path.to_string_lossy().to_string();
                        let node_version = entry.file_name().to_string_lossy().to_string();

                        tracing::debug!("Found Claude in NVM node {}: {}", node_version, path_str);

                        // Get Claude version
                        let version = get_claude_version(&path_str).ok().flatten();

                        installations.push(ClaudeInstallation {
                            path: path_str,
                            version,
                            source: format!("nvm ({})", node_version),
                            installation_type: InstallationType::System,
                        });
                    }
                }
            }
        }
    }

    installations
}

/// Check standard installation paths (Unix).
#[cfg(unix)]
fn find_standard_installations() -> Vec<ClaudeInstallation> {
    let mut installations = Vec::new();

    // Common installation paths for claude
    let mut paths_to_check: Vec<(String, String)> = vec![
        ("/usr/local/bin/claude".to_string(), "system".to_string()),
        (
            "/opt/homebrew/bin/claude".to_string(),
            "homebrew".to_string(),
        ),
        ("/usr/bin/claude".to_string(), "system".to_string()),
        ("/bin/claude".to_string(), "system".to_string()),
    ];

    // Also check user-specific paths
    if let Ok(home) = std::env::var("HOME") {
        paths_to_check.extend(vec![
            (
                format!("{}/.claude/local/claude", home),
                "claude-local".to_string(),
            ),
            (
                format!("{}/.local/bin/claude", home),
                "local-bin".to_string(),
            ),
            (
                format!("{}/.npm-global/bin/claude", home),
                "npm-global".to_string(),
            ),
            (format!("{}/.yarn/bin/claude", home), "yarn".to_string()),
            (format!("{}/.bun/bin/claude", home), "bun".to_string()),
            (format!("{}/bin/claude", home), "home-bin".to_string()),
            // Check common node_modules locations
            (
                format!("{}/node_modules/.bin/claude", home),
                "node-modules".to_string(),
            ),
            (
                format!("{}/.config/yarn/global/node_modules/.bin/claude", home),
                "yarn-global".to_string(),
            ),
        ]);
    }

    // Check each path
    for (path, source) in paths_to_check {
        let path_buf = PathBuf::from(&path);
        if path_buf.exists() && path_buf.is_file() {
            tracing::debug!("Found claude at standard path: {} ({})", path, source);

            // Get version
            let version = get_claude_version(&path).ok().flatten();

            installations.push(ClaudeInstallation {
                path,
                version,
                source,
                installation_type: InstallationType::System,
            });
        }
    }

    // Also check if claude is available in PATH (without full path)
    if let Ok(output) = Command::new("claude").arg("--version").output() {
        if output.status.success() {
            tracing::debug!("claude is available in PATH");
            let version = super::version::extract_version_from_output(&output.stdout);

            installations.push(ClaudeInstallation {
                path: "claude".to_string(),
                version,
                source: "PATH".to_string(),
                installation_type: InstallationType::System,
            });
        }
    }

    installations
}

/// Check standard installation paths (Windows).
#[cfg(windows)]
fn find_standard_installations() -> Vec<ClaudeInstallation> {
    let mut installations = Vec::new();

    // Common installation paths for claude on Windows
    let mut paths_to_check: Vec<(String, String)> = vec![];

    // Check user-specific paths
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        paths_to_check.extend(vec![
            (
                format!("{}\\.claude\\local\\claude.exe", user_profile),
                "claude-local".to_string(),
            ),
            (
                format!("{}\\.local\\bin\\claude.exe", user_profile),
                "local-bin".to_string(),
            ),
            (
                format!("{}\\AppData\\Roaming\\npm\\claude.cmd", user_profile),
                "npm-global".to_string(),
            ),
            (
                format!("{}\\.yarn\\bin\\claude.cmd", user_profile),
                "yarn".to_string(),
            ),
            (
                format!("{}\\.bun\\bin\\claude.exe", user_profile),
                "bun".to_string(),
            ),
        ]);
    }

    // Check each path
    for (path, source) in paths_to_check {
        let path_buf = PathBuf::from(&path);
        if path_buf.exists() && path_buf.is_file() {
            tracing::debug!("Found claude at standard path: {} ({})", path, source);

            // Get version
            let version = get_claude_version(&path).ok().flatten();

            installations.push(ClaudeInstallation {
                path,
                version,
                source,
                installation_type: InstallationType::System,
            });
        }
    }

    // Also check if claude is available in PATH (without full path)
    if let Ok(output) = Command::new("claude.exe").arg("--version").output() {
        if output.status.success() {
            tracing::debug!("claude.exe is available in PATH");
            let version = super::version::extract_version_from_output(&output.stdout);

            installations.push(ClaudeInstallation {
                path: "claude.exe".to_string(),
                version,
                source: "PATH".to_string(),
                installation_type: InstallationType::System,
            });
        }
    }

    installations
}

/// Validate a custom binary path.
fn validate_custom_path(path: &str) -> Option<ClaudeInstallation> {
    let path_buf = PathBuf::from(path);

    if !path_buf.exists() {
        tracing::warn!("Custom path does not exist: {}", path);
        return None;
    }

    if !path_buf.is_file() {
        tracing::warn!("Custom path is not a file: {}", path);
        return None;
    }

    tracing::info!("Found custom Claude installation at: {}", path);

    let version = get_claude_version(path).ok().flatten();

    Some(ClaudeInstallation {
        path: path.to_string(),
        version,
        source: "custom".to_string(),
        installation_type: InstallationType::Custom,
    })
}

/// Select the best installation based on version and source priority.
fn select_best_installation(installations: Vec<ClaudeInstallation>) -> Option<ClaudeInstallation> {
    // In production builds, version information may not be retrievable because
    // spawning external processes can be restricted. We therefore no longer
    // discard installations that lack a detected version – the mere presence
    // of a readable binary on disk is enough to consider it valid. We still
    // prefer binaries with version information when it is available.
    installations.into_iter().max_by(|a, b| {
        match (&a.version, &b.version) {
            // If both have versions, compare them semantically.
            (Some(v1), Some(v2)) => compare_versions(v1, v2),
            // Prefer the entry that actually has version information.
            (Some(_), None) => Ordering::Greater,
            (None, Some(_)) => Ordering::Less,
            // Neither have version info: prefer the one that is not just
            // the bare "claude" lookup from PATH, because that may fail
            // at runtime if PATH is modified.
            (None, None) => {
                if a.path == "claude" && b.path != "claude" {
                    Ordering::Less
                } else if a.path != "claude" && b.path == "claude" {
                    Ordering::Greater
                } else {
                    // Compare by source preference
                    source_preference(b).cmp(&source_preference(a))
                }
            }
        }
    })
}

/// Builder for configuring binary discovery.
///
/// # Examples
///
/// ```no_run
/// use cc_sdk::binary::DiscoveryBuilder;
///
/// let builder = DiscoveryBuilder::new()
///     .custom_path("/opt/custom/claude")
///     .skip_nvm(true)
///     .use_cache(true);
///
/// let installations = builder.discover();
/// ```
#[derive(Debug, Clone, Default)]
pub struct DiscoveryBuilder {
    custom_paths: Vec<String>,
    skip_nvm: bool,
    skip_homebrew: bool,
    skip_system: bool,
    use_cache: bool,
}

impl DiscoveryBuilder {
    /// Create a new discovery builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a custom path to check.
    pub fn custom_path(mut self, path: impl Into<String>) -> Self {
        self.custom_paths.push(path.into());
        self
    }

    /// Skip NVM directory scanning.
    pub fn skip_nvm(mut self, skip: bool) -> Self {
        self.skip_nvm = skip;
        self
    }

    /// Skip Homebrew directory scanning.
    pub fn skip_homebrew(mut self, skip: bool) -> Self {
        self.skip_homebrew = skip;
        self
    }

    /// Skip system directory scanning.
    pub fn skip_system(mut self, skip: bool) -> Self {
        self.skip_system = skip;
        self
    }

    /// Enable or disable caching for this discovery.
    ///
    /// When enabled, results will be cached and retrieved from cache if available.
    /// Default is false (no caching for custom configurations).
    pub fn use_cache(mut self, use_cache: bool) -> Self {
        self.use_cache = use_cache;
        self
    }

    /// Perform discovery with the configured options.
    pub fn discover(self) -> Vec<ClaudeInstallation> {
        // Check cache if enabled
        if self.use_cache {
            if let Some(cached) = cache::get_cached_default() {
                // For now, we only cache default configurations
                // Custom configurations could have a more sophisticated cache key
                if self.custom_paths.is_empty()
                    && !self.skip_nvm
                    && !self.skip_homebrew
                    && !self.skip_system
                {
                    tracing::debug!("Using cached discovery results");
                    return cached;
                }
            }
        }

        let mut installations = Vec::new();

        // Check custom paths first
        for path in &self.custom_paths {
            if let Some(installation) = validate_custom_path(path) {
                installations.push(installation);
            }
        }

        // Try which/where command
        if let Some(installation) = try_which_command() {
            installations.push(installation);
        }

        // Check NVM if not skipped
        if !self.skip_nvm {
            installations.extend(find_nvm_installations());
        }

        // Check standard paths if not skipped
        if !self.skip_system {
            installations.extend(find_standard_installations());
        }

        // Remove duplicates
        let mut unique_paths = HashSet::new();
        installations.retain(|install| unique_paths.insert(install.path.clone()));

        // Cache the results if using default configuration
        if self.use_cache
            && self.custom_paths.is_empty()
            && !self.skip_nvm
            && !self.skip_homebrew
            && !self.skip_system
        {
            cache::set_cached_default(installations.clone());
        }

        installations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_preference() {
        let which_install = ClaudeInstallation {
            path: "/usr/local/bin/claude".to_string(),
            version: None,
            source: "which".to_string(),
            installation_type: InstallationType::System,
        };

        let path_install = ClaudeInstallation {
            path: "claude".to_string(),
            version: None,
            source: "PATH".to_string(),
            installation_type: InstallationType::System,
        };

        assert!(source_preference(&which_install) < source_preference(&path_install));
    }

    #[test]
    fn test_discovery_builder() {
        let builder = DiscoveryBuilder::new()
            .custom_path("/opt/test/claude")
            .skip_nvm(true);

        assert_eq!(builder.custom_paths.len(), 1);
        assert!(builder.skip_nvm);
    }

    #[test]
    fn test_installation_type_equality() {
        assert_eq!(InstallationType::System, InstallationType::System);
        assert_ne!(InstallationType::System, InstallationType::Custom);
    }
}
