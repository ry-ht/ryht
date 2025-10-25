//! Binary validation and health checking utilities.
//!
//! This module provides functions to validate Claude binary installations,
//! check their executability, and verify their health status.
//!
//! # Examples
//!
//! ```no_run
//! use crate::cc::binary::validation::{verify_binary, BinaryHealth};
//!
//! match verify_binary("/usr/local/bin/claude") {
//!     Ok(health) => {
//!         println!("Binary is valid: version {}", health.version.unwrap_or_default());
//!         println!("Executable: {}", health.is_executable);
//!         println!("Version check passed: {}", health.version_check_passed);
//!     }
//!     Err(e) => eprintln!("Binary validation failed: {}", e),
//! }
//! ```

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use super::env::get_claude_version;
use super::version::Version;

/// Result of binary health check.
///
/// This struct contains detailed information about a Claude binary's health status.
#[derive(Debug, Clone)]
pub struct BinaryHealth {
    /// Path to the binary
    pub path: String,
    /// Whether the file exists
    pub exists: bool,
    /// Whether the file is executable
    pub is_executable: bool,
    /// Detected version string
    pub version: Option<String>,
    /// Parsed version object
    pub parsed_version: Option<Version>,
    /// Whether the --version command succeeded
    pub version_check_passed: bool,
    /// Whether this is a valid Claude binary
    pub is_valid: bool,
    /// Any warnings or issues detected
    pub warnings: Vec<String>,
}

impl BinaryHealth {
    /// Create a new health check result for a non-existent binary.
    fn nonexistent(path: String) -> Self {
        Self {
            path,
            exists: false,
            is_executable: false,
            version: None,
            parsed_version: None,
            version_check_passed: false,
            is_valid: false,
            warnings: vec!["Binary does not exist".to_string()],
        }
    }

    /// Check if the binary meets minimum version requirements.
    ///
    /// # Arguments
    ///
    /// * `min_version` - Minimum required version string (e.g., "1.0.0")
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::cc::binary::validation::verify_binary;
    ///
    /// let health = verify_binary("/usr/local/bin/claude").unwrap();
    /// if health.meets_version_requirement("1.0.0") {
    ///     println!("Version is compatible");
    /// }
    /// ```
    pub fn meets_version_requirement(&self, min_version: &str) -> bool {
        if let (Some(current), Some(minimum)) = (
            &self.parsed_version,
            Version::parse(min_version),
        ) {
            current >= &minimum
        } else {
            false
        }
    }

    /// Check if this binary has any critical issues.
    pub fn has_critical_issues(&self) -> bool {
        !self.is_valid || !self.exists || !self.is_executable
    }
}

/// Verify a Claude binary and return its health status.
///
/// This function performs comprehensive validation:
/// - Checks if the file exists
/// - Verifies it's executable
/// - Attempts to run `--version` to confirm it works
/// - Extracts and parses the version
///
/// # Arguments
///
/// * `path` - Path to the Claude binary
///
/// # Returns
///
/// * `Ok(BinaryHealth)` - Health check results
/// * `Err(String)` - Error message if validation completely fails
///
/// # Examples
///
/// ```no_run
/// use crate::cc::binary::validation::verify_binary;
///
/// match verify_binary("/usr/local/bin/claude") {
///     Ok(health) => {
///         if health.is_valid {
///             println!("Binary is valid!");
///         } else {
///             println!("Issues found: {:?}", health.warnings);
///         }
///     }
///     Err(e) => eprintln!("Validation error: {}", e),
/// }
/// ```
pub fn verify_binary(path: &str) -> Result<BinaryHealth, String> {
    let path_obj = Path::new(path);

    // Check if file exists
    if !path_obj.exists() {
        return Ok(BinaryHealth::nonexistent(path.to_string()));
    }

    let mut warnings = Vec::new();
    let mut is_executable = false;
    let mut version = None;
    let mut parsed_version = None;
    let version_check_passed: bool;

    // Check if it's a file (not a directory)
    if !path_obj.is_file() {
        warnings.push("Path is not a regular file".to_string());
    }

    // Check executability (Unix-specific)
    #[cfg(unix)]
    {
        if let Ok(metadata) = fs::metadata(path_obj) {
            let permissions = metadata.permissions();
            is_executable = permissions.mode() & 0o111 != 0;

            if !is_executable {
                warnings.push("Binary is not executable (missing execute permissions)".to_string());
            }
        } else {
            warnings.push("Failed to read file metadata".to_string());
        }
    }

    // On Windows, assume .exe files are executable
    #[cfg(windows)]
    {
        is_executable = path.ends_with(".exe") || path.ends_with(".cmd");
        if !is_executable {
            warnings.push("Binary does not have .exe or .cmd extension".to_string());
        }
    }

    // Try to get version
    match get_claude_version(path) {
        Ok(Some(ver)) => {
            version = Some(ver.clone());
            parsed_version = Version::parse(&ver);
            version_check_passed = true;

            if parsed_version.is_none() {
                warnings.push(format!("Failed to parse version string: {}", ver));
            }
        }
        Ok(None) => {
            warnings.push("Binary exists but --version returned no version".to_string());
            version_check_passed = true; // Command ran, just didn't get version
        }
        Err(e) => {
            warnings.push(format!("Failed to run --version: {}", e));
            version_check_passed = false;
        }
    }

    // Determine if binary is valid
    let is_valid = is_executable && version_check_passed;

    Ok(BinaryHealth {
        path: path.to_string(),
        exists: true,
        is_executable,
        version,
        parsed_version,
        version_check_passed,
        is_valid,
        warnings,
    })
}

/// Quick check if a binary path is valid and executable.
///
/// This is a lighter-weight version of [`verify_binary`] that doesn't
/// attempt to run the binary or check its version.
///
/// # Arguments
///
/// * `path` - Path to check
///
/// # Returns
///
/// * `true` if the path exists and is executable
/// * `false` otherwise
///
/// # Examples
///
/// ```no_run
/// use crate::cc::binary::validation::is_executable;
///
/// if is_executable("/usr/local/bin/claude") {
///     println!("Binary is executable");
/// }
/// ```
pub fn is_executable(path: &str) -> bool {
    let path_obj = Path::new(path);

    if !path_obj.exists() || !path_obj.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        if let Ok(metadata) = fs::metadata(path_obj) {
            let permissions = metadata.permissions();
            permissions.mode() & 0o111 != 0
        } else {
            false
        }
    }

    #[cfg(windows)]
    {
        path.ends_with(".exe") || path.ends_with(".cmd")
    }
}

/// Check if a binary meets minimum version requirements.
///
/// # Arguments
///
/// * `path` - Path to the binary
/// * `min_version` - Minimum required version (e.g., "1.0.0")
///
/// # Returns
///
/// * `Ok(true)` if version meets requirements
/// * `Ok(false)` if version is lower than required
/// * `Err(String)` if version cannot be determined
///
/// # Examples
///
/// ```no_run
/// use crate::cc::binary::validation::check_version_compatibility;
///
/// match check_version_compatibility("/usr/local/bin/claude", "1.0.0") {
///     Ok(true) => println!("Version is compatible"),
///     Ok(false) => println!("Version is too old"),
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
pub fn check_version_compatibility(path: &str, min_version: &str) -> Result<bool, String> {
    let health = verify_binary(path)?;

    if !health.version_check_passed {
        return Err("Unable to determine binary version".to_string());
    }

    Ok(health.meets_version_requirement(min_version))
}

/// Perform a comprehensive health check on multiple binary paths.
///
/// This is useful for checking all discovered installations and filtering
/// out any that have issues.
///
/// # Arguments
///
/// * `paths` - Iterator of paths to check
///
/// # Returns
///
/// A vector of health check results, one per path
///
/// # Examples
///
/// ```no_run
/// use crate::cc::binary::validation::health_check_all;
///
/// let paths = vec!["/usr/local/bin/claude", "/opt/homebrew/bin/claude"];
/// let results = health_check_all(&paths);
///
/// for health in results {
///     if health.is_valid {
///         println!("✓ {}: {}", health.path, health.version.unwrap_or_default());
///     } else {
///         println!("✗ {}: {:?}", health.path, health.warnings);
///     }
/// }
/// ```
pub fn health_check_all<'a, I>(paths: I) -> Vec<BinaryHealth>
where
    I: IntoIterator<Item = &'a str>,
{
    paths
        .into_iter()
        .filter_map(|path| verify_binary(path).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nonexistent_binary() {
        let health = verify_binary("/nonexistent/path/claude").unwrap();
        assert!(!health.exists);
        assert!(!health.is_valid);
        assert!(!health.is_executable);
        assert!(health.has_critical_issues());
    }

    #[test]
    fn test_is_executable_nonexistent() {
        assert!(!is_executable("/nonexistent/path/claude"));
    }

    #[test]
    fn test_health_meets_version_requirement() {
        let mut health = BinaryHealth::nonexistent("/test".to_string());
        health.parsed_version = Version::parse("1.0.41");

        assert!(health.meets_version_requirement("1.0.0"));
        assert!(health.meets_version_requirement("1.0.41"));
        assert!(!health.meets_version_requirement("2.0.0"));
    }

    #[test]
    fn test_health_check_all() {
        let paths = vec!["/nonexistent/1", "/nonexistent/2"];
        let results = health_check_all(paths.iter().map(|s| *s));

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|h| !h.exists));
    }

    #[test]
    fn test_binary_health_has_critical_issues() {
        let health = BinaryHealth {
            path: "/test".to_string(),
            exists: true,
            is_executable: true,
            version: Some("1.0.0".to_string()),
            parsed_version: Version::parse("1.0.0"),
            version_check_passed: true,
            is_valid: true,
            warnings: vec![],
        };

        assert!(!health.has_critical_issues());

        let mut bad_health = health.clone();
        bad_health.is_executable = false;
        assert!(bad_health.has_critical_issues());
    }
}
