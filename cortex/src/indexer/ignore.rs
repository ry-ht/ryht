use anyhow::Result;
use std::path::Path;
use tracing::debug;

/// Gitignore pattern matcher
pub struct IgnoreMatcher {
    patterns: Vec<Pattern>,
    default_ignores: Vec<String>,
}

#[derive(Debug, Clone)]
struct Pattern {
    pattern: String,
    is_negation: bool,
    is_directory_only: bool,
}

impl IgnoreMatcher {
    /// Create a new ignore matcher with default patterns
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
            default_ignores: Self::default_ignore_patterns(),
        }
    }

    /// Default ignore patterns (always applied)
    fn default_ignore_patterns() -> Vec<String> {
        vec![
            "node_modules".to_string(),
            ".git".to_string(),
            "dist".to_string(),
            "build".to_string(),
            "target".to_string(),
            ".next".to_string(),
            ".turbo".to_string(),
            "coverage".to_string(),
            ".nyc_output".to_string(),
            "out".to_string(),
            ".cache".to_string(),
            "*.log".to_string(),
            ".DS_Store".to_string(),
        ]
    }

    /// Load .gitignore file from a directory
    pub async fn load_gitignore(&mut self, root: &Path) -> Result<()> {
        let gitignore_path = root.join(".gitignore");

        if !gitignore_path.exists() {
            debug!("No .gitignore found at {:?}", gitignore_path);
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&gitignore_path).await?;
        self.load_gitignore_from_string(&content);
        Ok(())
    }

    /// Load .gitignore patterns from a string (synchronous)
    pub fn load_gitignore_from_string(&mut self, content: &str) {
        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let mut pattern = Pattern {
                pattern: line.to_string(),
                is_negation: false,
                is_directory_only: false,
            };

            // Handle negation patterns (!)
            if line.starts_with('!') {
                pattern.is_negation = true;
                pattern.pattern = line[1..].to_string();
            }

            // Handle directory-only patterns (/)
            if line.ends_with('/') {
                pattern.is_directory_only = true;
                pattern.pattern = line.trim_end_matches('/').to_string();
            }

            self.patterns.push(pattern);
        }

        debug!("Loaded {} patterns from .gitignore", self.patterns.len());
    }

    /// Check if a path should be ignored
    pub fn should_ignore(&self, path: &Path, is_dir: bool) -> bool {
        let path_str = path.to_string_lossy();

        // Check default ignores first
        for default_ignore in &self.default_ignores {
            if self.matches_pattern(&path_str, default_ignore, is_dir) {
                return true;
            }
        }

        // Check gitignore patterns
        let mut should_ignore = false;

        for pattern in &self.patterns {
            if pattern.is_directory_only && !is_dir {
                continue;
            }

            if self.matches_pattern(&path_str, &pattern.pattern, is_dir) {
                if pattern.is_negation {
                    should_ignore = false; // Negation pattern
                } else {
                    should_ignore = true;
                }
            }
        }

        should_ignore
    }

    /// Simple pattern matching (supports * wildcards)
    fn matches_pattern(&self, path: &str, pattern: &str, _is_dir: bool) -> bool {
        // Handle simple cases
        if pattern == path {
            return true;
        }

        // Check if path contains the pattern (for directory names)
        if let Some(file_name) = Path::new(path).file_name() {
            if file_name.to_string_lossy() == pattern {
                return true;
            }
        }

        // Handle wildcard patterns
        if pattern.contains('*') {
            return self.glob_match(path, pattern);
        }

        // Check if path ends with pattern (for subdirectories)
        if path.ends_with(pattern) || path.contains(&format!("/{}", pattern)) {
            return true;
        }

        false
    }

    /// Simple glob pattern matching
    fn glob_match(&self, path: &str, pattern: &str) -> bool {
        // Split pattern by *
        let parts: Vec<&str> = pattern.split('*').collect();

        if parts.len() == 1 {
            return path == pattern;
        }

        // Check if path starts with first part
        if let Some(first) = parts.first() {
            if !first.is_empty() && !path.starts_with(first) {
                return false;
            }
        }

        // Check if path ends with last part
        if let Some(last) = parts.last() {
            if !last.is_empty() && !path.ends_with(last) {
                return false;
            }
        }

        // Check middle parts
        let mut current_pos = 0;
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }

            if i == 0 {
                current_pos = part.len();
                continue;
            }

            if let Some(pos) = path[current_pos..].find(part) {
                current_pos += pos + part.len();
            } else {
                return false;
            }
        }

        true
    }
}

impl Default for IgnoreMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_ignores() {
        let matcher = IgnoreMatcher::new();

        assert!(matcher.should_ignore(Path::new("node_modules"), true));
        assert!(matcher.should_ignore(Path::new("project/node_modules"), true));
        assert!(matcher.should_ignore(Path::new(".git"), true));
        assert!(matcher.should_ignore(Path::new("dist"), true));
        assert!(matcher.should_ignore(Path::new("build"), true));
        assert!(matcher.should_ignore(Path::new("target"), true));
    }

    #[test]
    fn test_wildcard_pattern() {
        let matcher = IgnoreMatcher::new();

        assert!(matcher.glob_match("test.log", "*.log"));
        assert!(matcher.glob_match("error.log", "*.log"));
        assert!(!matcher.glob_match("test.txt", "*.log"));
        assert!(matcher.glob_match("file.test.ts", "*.ts"));
    }

    #[test]
    fn test_should_ignore() {
        let matcher = IgnoreMatcher::new();

        // Default ignores
        assert!(matcher.should_ignore(Path::new("node_modules"), true));
        assert!(matcher.should_ignore(Path::new("src/node_modules"), true));
        assert!(matcher.should_ignore(Path::new(".git"), true));

        // Should not ignore
        assert!(!matcher.should_ignore(Path::new("src"), true));
        assert!(!matcher.should_ignore(Path::new("packages"), true));
        assert!(!matcher.should_ignore(Path::new("src/index.ts"), false));
    }

    #[tokio::test]
    async fn test_load_gitignore() {
        use std::io::Write;
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Create .gitignore
        let gitignore_path = root.join(".gitignore");
        let mut file = std::fs::File::create(&gitignore_path).unwrap();
        writeln!(file, "# Comment").unwrap();
        writeln!(file, "node_modules").unwrap();
        writeln!(file, "*.log").unwrap();
        writeln!(file, "temp/").unwrap();

        let mut matcher = IgnoreMatcher::new();
        matcher.load_gitignore(root).await.unwrap();

        assert!(matcher.patterns.len() >= 3);
    }
}
