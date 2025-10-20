//! File filtering utilities for ingestion.
//!
//! This module provides content filtering capabilities including:
//! - File type filtering
//! - Duplicate detection
//! - Quality scoring
//! - Language detection
//! - Content validation

use std::collections::HashSet;
use std::path::Path;

/// Common file extensions to ignore
const IGNORE_EXTENSIONS: &[&str] = &[
    "exe", "dll", "so", "dylib", "a", "o",
    "png", "jpg", "jpeg", "gif", "bmp", "ico",
    "mp3", "mp4", "avi", "mov", "wav",
    "zip", "tar", "gz", "rar", "7z",
];

/// Common directory names to ignore
const IGNORE_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    ".git",
    ".svn",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
];

/// Check if a file should be ignored based on extension
pub fn should_ignore_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        return IGNORE_EXTENSIONS.contains(&ext);
    }
    false
}

/// Check if a directory should be ignored
pub fn should_ignore_dir(path: &Path) -> bool {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        return IGNORE_DIRS.contains(&name);
    }
    false
}

/// Check if a file is a text file based on extension
pub fn is_text_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        matches!(
            ext,
            "rs" | "py" | "js" | "jsx" | "ts" | "tsx" | "md" | "txt" | "json" | "toml" | "yaml"
                | "yml" | "html" | "css" | "scss" | "c" | "cpp" | "h" | "hpp" | "go" | "java"
                | "kt" | "swift" | "rb" | "php" | "sh" | "bash" | "zsh"
        )
    } else {
        false
    }
}

/// Duplicate detector using content hashing
pub struct DuplicateDetector {
    seen_hashes: HashSet<String>,
}

impl DuplicateDetector {
    /// Create a new duplicate detector
    pub fn new() -> Self {
        Self {
            seen_hashes: HashSet::new(),
        }
    }

    /// Check if content is duplicate and mark as seen
    pub fn is_duplicate(&mut self, content_hash: &str) -> bool {
        !self.seen_hashes.insert(content_hash.to_string())
    }

    /// Clear all seen hashes
    pub fn clear(&mut self) {
        self.seen_hashes.clear();
    }

    /// Get count of unique documents seen
    pub fn unique_count(&self) -> usize {
        self.seen_hashes.len()
    }
}

impl Default for DuplicateDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Quality metrics for content
#[derive(Debug, Clone)]
pub struct QualityMetrics {
    /// Quality score (0.0 - 1.0)
    pub score: f32,
    /// Readability score
    pub readability: f32,
    /// Information density
    pub density: f32,
    /// Issues found
    pub issues: Vec<String>,
}

/// Calculate quality score for text content
pub fn calculate_quality_score(content: &str) -> QualityMetrics {
    let mut issues = Vec::new();
    let char_count = content.chars().count();
    let word_count = content.split_whitespace().count();
    let line_count = content.lines().count();

    // Check minimum content
    if char_count < 50 {
        issues.push("Content too short".to_string());
    }

    // Check for excessive repetition
    let unique_words: HashSet<&str> = content.split_whitespace().collect();
    let repetition_ratio = unique_words.len() as f32 / word_count.max(1) as f32;
    if repetition_ratio < 0.3 {
        issues.push("High repetition detected".to_string());
    }

    // Calculate readability (Flesch reading ease approximation)
    let avg_words_per_sentence = word_count as f32 / line_count.max(1) as f32;
    let avg_chars_per_word = char_count as f32 / word_count.max(1) as f32;
    let readability = (206.835 - 1.015 * avg_words_per_sentence - 84.6 * avg_chars_per_word / 4.0)
        .max(0.0)
        .min(100.0)
        / 100.0;

    // Calculate information density
    let density = repetition_ratio.min(1.0);

    // Calculate overall score
    let mut score = 1.0;
    if char_count < 50 {
        score *= 0.3;
    }
    if repetition_ratio < 0.3 {
        score *= 0.5;
    }
    if readability < 0.3 {
        score *= 0.7;
    }

    QualityMetrics {
        score,
        readability,
        density,
        issues,
    }
}

/// Validate content for common issues
pub fn validate_content(content: &str) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    // Check if content is empty
    if content.trim().is_empty() {
        errors.push("Content is empty".to_string());
    }

    // Check for binary data
    let binary_chars = content.chars().filter(|c| c.is_control() && *c != '\n' && *c != '\r' && *c != '\t').count();
    if binary_chars as f32 / content.len().max(1) as f32 > 0.1 {
        errors.push("Content appears to contain binary data".to_string());
    }

    // Check for encoding issues
    if content.contains('\u{FFFD}') {
        errors.push("Content contains replacement characters (encoding issues)".to_string());
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Content filter that combines multiple filtering strategies
pub struct ContentFilter {
    duplicate_detector: DuplicateDetector,
    min_quality_score: f32,
    validate_encoding: bool,
}

impl ContentFilter {
    /// Create a new content filter
    pub fn new() -> Self {
        Self {
            duplicate_detector: DuplicateDetector::new(),
            min_quality_score: 0.3,
            validate_encoding: true,
        }
    }

    /// Set minimum quality score threshold
    pub fn with_min_quality(mut self, score: f32) -> Self {
        self.min_quality_score = score;
        self
    }

    /// Enable/disable encoding validation
    pub fn with_encoding_validation(mut self, enabled: bool) -> Self {
        self.validate_encoding = enabled;
        self
    }

    /// Check if content should be accepted
    pub fn should_accept(&mut self, content: &str, content_hash: &str) -> FilterResult {
        let mut reasons = Vec::new();

        // Check for duplicates
        if self.duplicate_detector.is_duplicate(content_hash) {
            return FilterResult {
                accepted: false,
                reasons: vec!["Duplicate content".to_string()],
                quality_score: None,
            };
        }

        // Validate content
        if self.validate_encoding {
            if let Err(errors) = validate_content(content) {
                reasons.extend(errors);
                return FilterResult {
                    accepted: false,
                    reasons,
                    quality_score: None,
                };
            }
        }

        // Check quality
        let quality = calculate_quality_score(content);
        if quality.score < self.min_quality_score {
            reasons.push(format!("Quality score too low: {:.2}", quality.score));
            reasons.extend(quality.issues);
            return FilterResult {
                accepted: false,
                reasons,
                quality_score: Some(quality.score),
            };
        }

        FilterResult {
            accepted: true,
            reasons: vec![],
            quality_score: Some(quality.score),
        }
    }

    /// Reset the filter state
    pub fn reset(&mut self) {
        self.duplicate_detector.clear();
    }
}

impl Default for ContentFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of content filtering
#[derive(Debug, Clone)]
pub struct FilterResult {
    pub accepted: bool,
    pub reasons: Vec<String>,
    pub quality_score: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ignore_file() {
        assert!(should_ignore_file(Path::new("test.exe")));
        assert!(should_ignore_file(Path::new("image.png")));
        assert!(!should_ignore_file(Path::new("code.rs")));
    }

    #[test]
    fn test_ignore_dir() {
        assert!(should_ignore_dir(Path::new("node_modules")));
        assert!(should_ignore_dir(Path::new("target")));
        assert!(!should_ignore_dir(Path::new("src")));
    }

    #[test]
    fn test_text_file() {
        assert!(is_text_file(Path::new("code.rs")));
        assert!(is_text_file(Path::new("doc.md")));
        assert!(!is_text_file(Path::new("image.png")));
    }

    #[test]
    fn test_duplicate_detector() {
        let mut detector = DuplicateDetector::new();
        assert!(!detector.is_duplicate("hash1"));
        assert!(detector.is_duplicate("hash1"));
        assert!(!detector.is_duplicate("hash2"));
    }

    #[test]
    fn test_quality_score() {
        let good_content = "This is a well-written piece of text with good variety and structure. It contains multiple sentences with different words and proper formatting.";
        let metrics = calculate_quality_score(good_content);
        assert!(metrics.score > 0.5);

        let bad_content = "a a a a a";
        let metrics = calculate_quality_score(bad_content);
        assert!(metrics.score < 0.5);
    }

    #[test]
    fn test_content_validation() {
        assert!(validate_content("Valid text content").is_ok());
        assert!(validate_content("").is_err());
    }

    #[test]
    fn test_content_filter() {
        let mut filter = ContentFilter::new().with_min_quality(0.3);

        let good_content = "This is good quality content with sufficient length and variety.";
        let result = filter.should_accept(good_content, "hash1");
        assert!(result.accepted);

        // Test duplicate detection
        let result = filter.should_accept(good_content, "hash1");
        assert!(!result.accepted);
        assert!(result.reasons.contains(&"Duplicate content".to_string()));
    }
}
