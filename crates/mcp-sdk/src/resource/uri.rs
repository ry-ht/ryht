//! URI pattern matching utilities.
//!
//! This module provides utilities for matching URIs against patterns with
//! glob-style wildcards.

use std::fmt;

/// A URI pattern that can match against URIs using wildcards.
///
/// Patterns support the following wildcard syntax:
/// - `*` - Matches any characters within a single path segment
/// - `**` - Matches any characters across multiple path segments
///
/// # Examples
///
/// ```rust
/// use mcp_server::resource::UriPattern;
///
/// let pattern = UriPattern::new("db://users/*");
///
/// assert!(pattern.matches("db://users/123"));
/// assert!(pattern.matches("db://users/alice"));
/// assert!(!pattern.matches("db://posts/123"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UriPattern {
    pattern: String,
}

impl UriPattern {
    /// Creates a new URI pattern.
    ///
    /// # Arguments
    ///
    /// * `pattern` - The pattern string with optional wildcards
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::UriPattern;
    ///
    /// let exact = UriPattern::new("app://config");
    /// let wildcard = UriPattern::new("db://users/*");
    /// let glob = UriPattern::new("file:///**/*.txt");
    /// ```
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
        }
    }

    /// Returns the pattern string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::UriPattern;
    ///
    /// let pattern = UriPattern::new("db://users/*");
    /// assert_eq!(pattern.as_str(), "db://users/*");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.pattern
    }

    /// Checks if the given URI matches this pattern.
    ///
    /// # Arguments
    ///
    /// * `uri` - The URI to match against this pattern
    ///
    /// # Returns
    ///
    /// `true` if the URI matches the pattern, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use mcp_server::resource::UriPattern;
    ///
    /// let pattern = UriPattern::new("db://users/*");
    ///
    /// assert!(pattern.matches("db://users/123"));
    /// assert!(pattern.matches("db://users/alice"));
    /// assert!(!pattern.matches("db://posts/123"));
    /// ```
    pub fn matches(&self, uri: &str) -> bool {
        matches_pattern(&self.pattern, uri)
    }
}

impl fmt::Display for UriPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.pattern)
    }
}

/// Checks if a URI matches a pattern with glob-style wildcards.
///
/// Supports:
/// - Exact matches: `"app://config"` matches only `"app://config"`
/// - Wildcards: `"db://users/*"` matches `"db://users/123"`, `"db://users/alice"`, etc.
/// - Multi-level wildcards: `"file:///**/*.txt"` matches all `.txt` files at any depth
///
/// # Arguments
///
/// * `pattern` - The pattern string with optional wildcards
/// * `uri` - The URI to match
///
/// # Returns
///
/// `true` if the URI matches the pattern, `false` otherwise.
///
/// # Examples
///
/// ```rust
/// use mcp_server::resource::matches_pattern;
///
/// // Exact match
/// assert!(matches_pattern("app://config", "app://config"));
/// assert!(!matches_pattern("app://config", "app://other"));
///
/// // Single wildcard
/// assert!(matches_pattern("db://users/*", "db://users/123"));
/// assert!(matches_pattern("db://users/*", "db://users/alice"));
/// assert!(!matches_pattern("db://users/*", "db://posts/123"));
///
/// // Glob patterns
/// assert!(matches_pattern("file:///*.txt", "file:///doc.txt"));
/// assert!(matches_pattern("file:///*.txt", "file:///readme.txt"));
/// assert!(!matches_pattern("file:///*.txt", "file:///doc.md"));
/// ```
pub fn matches_pattern(pattern: &str, uri: &str) -> bool {
    // Handle exact match (no wildcards)
    if !pattern.contains('*') {
        return pattern == uri;
    }

    // Convert glob pattern to regex
    let regex_pattern = glob_to_regex(pattern);

    // Compile and match
    match regex::Regex::new(&regex_pattern) {
        Ok(re) => re.is_match(uri),
        Err(_) => false,
    }
}

/// Converts a glob pattern to a regex pattern.
///
/// Handles:
/// - `*` → matches any characters except `/`
/// - `**` → matches any characters including `/`
/// - Escapes special regex characters
fn glob_to_regex(pattern: &str) -> String {
    let mut regex = String::from("^");
    let mut chars = pattern.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '*' => {
                // Check for `**` (matches across segments)
                if chars.peek() == Some(&'*') {
                    chars.next();
                    regex.push_str(".*");
                } else {
                    // `*` matches within a segment (no `/`)
                    regex.push_str("[^/]*");
                }
            }
            '?' => regex.push('.'),
            '.' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '+' | '\\' => {
                regex.push('\\');
                regex.push(ch);
            }
            _ => regex.push(ch),
        }
    }

    regex.push('$');
    regex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        assert!(matches_pattern("app://config", "app://config"));
        assert!(!matches_pattern("app://config", "app://other"));
    }

    #[test]
    fn test_wildcard_single_segment() {
        let pattern = "db://users/*";
        assert!(matches_pattern(pattern, "db://users/123"));
        assert!(matches_pattern(pattern, "db://users/alice"));
        assert!(matches_pattern(pattern, "db://users/bob"));
        assert!(!matches_pattern(pattern, "db://posts/123"));
        assert!(!matches_pattern(pattern, "db://users"));
    }

    #[test]
    fn test_wildcard_at_end() {
        let pattern = "file:///*.txt";
        assert!(matches_pattern(pattern, "file:///doc.txt"));
        assert!(matches_pattern(pattern, "file:///readme.txt"));
        assert!(!matches_pattern(pattern, "file:///doc.md"));
        assert!(!matches_pattern(pattern, "file:///subdir/doc.txt"));
    }

    #[test]
    fn test_wildcard_middle() {
        let pattern = "api://v1/*/status";
        assert!(matches_pattern(pattern, "api://v1/users/status"));
        assert!(matches_pattern(pattern, "api://v1/posts/status"));
        assert!(!matches_pattern(pattern, "api://v1/users/health"));
        assert!(!matches_pattern(pattern, "api://v2/users/status"));
    }

    #[test]
    fn test_double_wildcard() {
        let pattern = "file://**/*.txt";
        assert!(matches_pattern(pattern, "file:///doc.txt"));
        assert!(matches_pattern(pattern, "file:///a/doc.txt"));
        assert!(matches_pattern(pattern, "file:///a/b/doc.txt"));
        assert!(!matches_pattern(pattern, "file:///doc.md"));
    }

    #[test]
    fn test_multiple_wildcards() {
        let pattern = "api://*/v*/users";
        assert!(matches_pattern(pattern, "api://prod/v1/users"));
        assert!(matches_pattern(pattern, "api://dev/v2/users"));
        assert!(!matches_pattern(pattern, "api://prod/v1/posts"));
    }

    #[test]
    fn test_no_match() {
        assert!(!matches_pattern("db://users/*", "db://posts/123"));
        assert!(!matches_pattern("file:///*.txt", "file:///doc.md"));
        assert!(!matches_pattern("app://config", "app://settings"));
    }

    #[test]
    fn test_empty_pattern() {
        assert!(matches_pattern("", ""));
        assert!(!matches_pattern("", "something"));
    }

    #[test]
    fn test_empty_uri() {
        assert!(matches_pattern("", ""));
        assert!(!matches_pattern("something", ""));
    }

    #[test]
    fn test_special_characters() {
        assert!(matches_pattern("api://test.com/path", "api://test.com/path"));
        assert!(matches_pattern("db://user@host:5432/*", "db://user@host:5432/mydb"));
    }

    #[test]
    fn test_uri_pattern_new() {
        let pattern = UriPattern::new("db://users/*");
        assert_eq!(pattern.as_str(), "db://users/*");
    }

    #[test]
    fn test_uri_pattern_matches() {
        let pattern = UriPattern::new("db://users/*");
        assert!(pattern.matches("db://users/123"));
        assert!(!pattern.matches("db://posts/123"));
    }

    #[test]
    fn test_uri_pattern_display() {
        let pattern = UriPattern::new("db://users/*");
        assert_eq!(format!("{}", pattern), "db://users/*");
    }

    #[test]
    fn test_uri_pattern_clone() {
        let pattern1 = UriPattern::new("db://users/*");
        let pattern2 = pattern1.clone();
        assert_eq!(pattern1, pattern2);
    }

    #[test]
    fn test_case_sensitive() {
        assert!(matches_pattern("app://Config", "app://Config"));
        assert!(!matches_pattern("app://Config", "app://config"));
    }

    #[test]
    fn test_glob_to_regex() {
        assert_eq!(glob_to_regex("exact"), "^exact$");
        assert_eq!(glob_to_regex("test*"), "^test[^/]*$");
        assert_eq!(glob_to_regex("test**"), "^test.*$");
        assert_eq!(glob_to_regex("a.b"), "^a\\.b$");
    }

    #[test]
    fn test_complex_patterns() {
        // Pattern with multiple wildcards
        let pattern = "repo://*/*/file.txt";
        assert!(matches_pattern(pattern, "repo://org/project/file.txt"));
        assert!(matches_pattern(pattern, "repo://company/app/file.txt"));
        assert!(!matches_pattern(pattern, "repo://org/file.txt"));
        assert!(!matches_pattern(pattern, "repo://org/project/other.txt"));
    }

    #[test]
    fn test_wildcard_only() {
        assert!(matches_pattern("*", "anything"));
        assert!(!matches_pattern("*", "multiple/segments")); // Single * doesn't match across segments
        assert!(matches_pattern("**", "anything"));
        assert!(matches_pattern("**", "multiple/segments"));
    }

    #[test]
    fn test_prefix_wildcard() {
        let pattern = "*://config";
        assert!(matches_pattern(pattern, "app://config"));
        assert!(matches_pattern(pattern, "db://config"));
        assert!(!matches_pattern(pattern, "app://other"));
    }

    #[test]
    fn test_suffix_wildcard() {
        let pattern = "app://*";
        assert!(matches_pattern(pattern, "app://config"));
        assert!(matches_pattern(pattern, "app://settings"));
        assert!(!matches_pattern(pattern, "db://config"));
    }
}
