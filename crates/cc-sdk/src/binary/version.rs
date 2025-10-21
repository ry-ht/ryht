//! Version parsing and comparison utilities for Claude Code binary.
//!
//! This module provides semantic versioning support for comparing different
//! Claude installations and selecting the most appropriate one.

use std::cmp::Ordering;
use std::fmt;

/// Represents a parsed semantic version.
///
/// Supports versions in the format: `major.minor.patch[-prerelease][+build]`
///
/// # Examples
///
/// ```
/// use cc_sdk::binary::Version;
///
/// let v1 = Version::parse("1.0.41").unwrap();
/// let v2 = Version::parse("1.0.40").unwrap();
/// assert!(v1 > v2);
///
/// let v3 = Version::parse("2.0.0-beta.1").unwrap();
/// assert!(v3.is_prerelease());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    /// Major version number
    pub major: u32,
    /// Minor version number
    pub minor: u32,
    /// Patch version number
    pub patch: u32,
    /// Pre-release identifier (e.g., "beta.1", "rc.2")
    pub prerelease: Option<String>,
    /// Build metadata (e.g., "20130313144700")
    pub build: Option<String>,
}

impl Version {
    /// Parse a version string into a `Version` struct.
    ///
    /// # Arguments
    ///
    /// * `version_str` - A string slice containing a semantic version
    ///
    /// # Returns
    ///
    /// * `Some(Version)` if parsing succeeds
    /// * `None` if the version string is invalid
    ///
    /// # Examples
    ///
    /// ```
    /// use cc_sdk::binary::Version;
    ///
    /// let v = Version::parse("1.0.41").unwrap();
    /// assert_eq!(v.major, 1);
    /// assert_eq!(v.minor, 0);
    /// assert_eq!(v.patch, 41);
    ///
    /// let v_pre = Version::parse("2.0.0-beta.1+build123").unwrap();
    /// assert_eq!(v_pre.prerelease, Some("beta.1".to_string()));
    /// assert_eq!(v_pre.build, Some("build123".to_string()));
    /// ```
    pub fn parse(version_str: &str) -> Option<Self> {
        // Split by '+' to separate build metadata
        let (version_part, build) = match version_str.split_once('+') {
            Some((v, b)) => (v, Some(b.to_string())),
            None => (version_str, None),
        };

        // Split by '-' to separate prerelease
        let (core_version, prerelease) = match version_part.split_once('-') {
            Some((v, p)) => (v, Some(p.to_string())),
            None => (version_part, None),
        };

        // Parse core version (major.minor.patch)
        let parts: Vec<&str> = core_version.split('.').collect();
        if parts.len() != 3 {
            return None;
        }

        let major = parts[0].parse::<u32>().ok()?;
        let minor = parts[1].parse::<u32>().ok()?;
        let patch = parts[2].parse::<u32>().ok()?;

        Some(Version {
            major,
            minor,
            patch,
            prerelease,
            build,
        })
    }

    /// Check if this version is a pre-release version.
    ///
    /// # Examples
    ///
    /// ```
    /// use cc_sdk::binary::Version;
    ///
    /// let stable = Version::parse("1.0.0").unwrap();
    /// assert!(!stable.is_prerelease());
    ///
    /// let beta = Version::parse("1.0.0-beta.1").unwrap();
    /// assert!(beta.is_prerelease());
    /// ```
    pub fn is_prerelease(&self) -> bool {
        self.prerelease.is_some()
    }

    /// Get a string representation of just the core version (major.minor.patch).
    ///
    /// # Examples
    ///
    /// ```
    /// use cc_sdk::binary::Version;
    ///
    /// let v = Version::parse("1.0.41-beta.1+build").unwrap();
    /// assert_eq!(v.core_version(), "1.0.41");
    /// ```
    pub fn core_version(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(ref pre) = self.prerelease {
            write!(f, "-{}", pre)?;
        }
        if let Some(ref build) = self.build {
            write!(f, "+{}", build)?;
        }
        Ok(())
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    /// Compare two versions according to semantic versioning rules.
    ///
    /// Precedence rules:
    /// 1. Compare major, minor, patch numerically
    /// 2. Pre-release versions have lower precedence than normal versions
    /// 3. Build metadata is ignored in comparisons
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare major.minor.patch
        match self.major.cmp(&other.major) {
            Ordering::Equal => {}
            other => return other,
        }

        match self.minor.cmp(&other.minor) {
            Ordering::Equal => {}
            other => return other,
        }

        match self.patch.cmp(&other.patch) {
            Ordering::Equal => {}
            other => return other,
        }

        // Handle pre-release versions
        // According to semver: pre-release < release
        match (&self.prerelease, &other.prerelease) {
            (None, None) => Ordering::Equal,
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (Some(a), Some(b)) => {
                // Compare pre-release identifiers lexicographically
                // This is a simplified comparison; full semver has more complex rules
                a.cmp(b)
            }
        }
    }
}

/// Extract a version string from Claude CLI output.
///
/// This function parses the output of `claude --version` and extracts
/// the semantic version string.
///
/// # Arguments
///
/// * `output` - Raw bytes from command output (stdout)
///
/// # Returns
///
/// * `Some(String)` containing the version if found
/// * `None` if no valid version pattern is found
///
/// # Examples
///
/// ```
/// use cc_sdk::binary::extract_version_from_output;
///
/// let output = b"claude version 1.0.41\n";
/// let version = extract_version_from_output(output);
/// assert_eq!(version, Some("1.0.41".to_string()));
/// ```
pub fn extract_version_from_output(output: &[u8]) -> Option<String> {
    let output_str = String::from_utf8_lossy(output);

    tracing::debug!("Extracting version from output: {:?}", output_str);

    // Regex pattern for semantic version:
    // - One or more digits, followed by a dot
    // - One or more digits, followed by a dot
    // - One or more digits
    // - Optionally followed by pre-release identifier (-xxx)
    // - Optionally followed by build metadata (+xxx)
    let version_pattern = r"(\d+\.\d+\.\d+(?:-[a-zA-Z0-9.-]+)?(?:\+[a-zA-Z0-9.-]+)?)";

    // Try to match the pattern
    for line in output_str.lines() {
        if let Some(caps) = extract_version_with_pattern(line, version_pattern) {
            tracing::debug!("Extracted version: {}", caps);
            return Some(caps);
        }
    }

    tracing::debug!("No version found in output");
    None
}

/// Helper function to extract version using a regex pattern.
fn extract_version_with_pattern(text: &str, _pattern: &str) -> Option<String> {
    // Simple regex implementation without external dependencies
    // We'll use a manual approach for the basic pattern

    // Look for pattern like X.Y.Z where X, Y, Z are digits
    let mut chars = text.chars().peekable();
    let mut result = String::new();

    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            result.push(c);

            // Collect the rest of the version string
            while let Some(&next) = chars.peek() {
                if next.is_ascii_digit() || next == '.' || next == '-' || next == '+'
                    || next.is_ascii_alphabetic() {
                    result.push(chars.next().unwrap());
                } else {
                    break;
                }
            }

            // Validate it looks like a version (has at least two dots)
            if result.matches('.').count() >= 2 {
                // Trim any trailing non-version characters
                while result.ends_with(|c: char| !c.is_ascii_alphanumeric()) {
                    result.pop();
                }
                return Some(result);
            }

            result.clear();
        }
    }

    None
}

/// Compare two version strings.
///
/// This is a convenience function that parses and compares versions in one step.
///
/// # Arguments
///
/// * `a` - First version string
/// * `b` - Second version string
///
/// # Returns
///
/// * `Ordering::Greater` if a > b
/// * `Ordering::Less` if a < b
/// * `Ordering::Equal` if a == b (or if both are invalid)
///
/// # Examples
///
/// ```
/// use cc_sdk::binary::compare_versions;
/// use std::cmp::Ordering;
///
/// assert_eq!(compare_versions("1.0.41", "1.0.40"), Ordering::Greater);
/// assert_eq!(compare_versions("2.0.0", "1.9.9"), Ordering::Greater);
/// assert_eq!(compare_versions("1.0.0", "1.0.0"), Ordering::Equal);
/// ```
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    match (Version::parse(a), Version::parse(b)) {
        (Some(va), Some(vb)) => va.cmp(&vb),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parse_basic() {
        let v = Version::parse("1.0.41").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 41);
        assert_eq!(v.prerelease, None);
        assert_eq!(v.build, None);
    }

    #[test]
    fn test_version_parse_prerelease() {
        let v = Version::parse("2.0.0-beta.1").unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 0);
        assert_eq!(v.prerelease, Some("beta.1".to_string()));
        assert_eq!(v.build, None);
    }

    #[test]
    fn test_version_parse_build() {
        let v = Version::parse("1.0.0+20130313144700").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 0);
        assert_eq!(v.prerelease, None);
        assert_eq!(v.build, Some("20130313144700".to_string()));
    }

    #[test]
    fn test_version_parse_full() {
        let v = Version::parse("1.2.3-beta.1+build.123").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert_eq!(v.prerelease, Some("beta.1".to_string()));
        assert_eq!(v.build, Some("build.123".to_string()));
    }

    #[test]
    fn test_version_comparison() {
        let v1 = Version::parse("1.0.41").unwrap();
        let v2 = Version::parse("1.0.40").unwrap();
        assert!(v1 > v2);

        let v3 = Version::parse("2.0.0").unwrap();
        assert!(v3 > v1);

        let v4 = Version::parse("1.0.41").unwrap();
        assert_eq!(v1, v4);
    }

    #[test]
    fn test_version_prerelease_comparison() {
        let stable = Version::parse("1.0.0").unwrap();
        let beta = Version::parse("1.0.0-beta.1").unwrap();
        assert!(stable > beta);

        let beta1 = Version::parse("1.0.0-beta.1").unwrap();
        let beta2 = Version::parse("1.0.0-beta.2").unwrap();
        assert!(beta2 > beta1);
    }

    #[test]
    fn test_extract_version_from_output() {
        let output = b"claude version 1.0.41\n";
        let version = extract_version_from_output(output);
        assert_eq!(version, Some("1.0.41".to_string()));

        let output2 = b"1.2.3-beta.1\n";
        let version2 = extract_version_from_output(output2);
        assert_eq!(version2, Some("1.2.3-beta.1".to_string()));
    }

    #[test]
    fn test_compare_versions() {
        assert_eq!(compare_versions("1.0.41", "1.0.40"), Ordering::Greater);
        assert_eq!(compare_versions("2.0.0", "1.9.9"), Ordering::Greater);
        assert_eq!(compare_versions("1.0.0", "1.0.0"), Ordering::Equal);
        assert_eq!(compare_versions("1.0.0-beta", "1.0.0"), Ordering::Less);
    }

    #[test]
    fn test_version_display() {
        let v1 = Version::parse("1.0.41").unwrap();
        assert_eq!(v1.to_string(), "1.0.41");

        let v2 = Version::parse("2.0.0-beta.1+build").unwrap();
        assert_eq!(v2.to_string(), "2.0.0-beta.1+build");
    }

    #[test]
    fn test_is_prerelease() {
        let stable = Version::parse("1.0.0").unwrap();
        assert!(!stable.is_prerelease());

        let beta = Version::parse("1.0.0-beta.1").unwrap();
        assert!(beta.is_prerelease());
    }

    #[test]
    fn test_core_version() {
        let v = Version::parse("1.0.41-beta.1+build").unwrap();
        assert_eq!(v.core_version(), "1.0.41");
    }
}
