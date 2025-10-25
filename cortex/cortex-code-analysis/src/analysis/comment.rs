//! Advanced comment analysis module.
//!
//! This module provides comprehensive comment analysis capabilities including:
//! - Comment type classification (doc, inline, block, header)
//! - Comment density and coverage metrics
//! - Comment quality assessment
//! - Doc comment extraction and validation
//! - Comment association with code elements
//!
//! # Examples
//!
//! ```no_run
//! use cortex_code_analysis::analysis::comment::{CommentAnalyzer, CommentMetrics};
//! use cortex_code_analysis::{Parser, RustLanguage};
//! use std::path::Path;
//!
//! let source = r#"
//! /// This is a doc comment
//! fn example() {
//!     // This is an inline comment
//!     let x = 1;
//! }
//! "#;
//!
//! let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs"))?;
//! let analyzer = CommentAnalyzer::new(&parser, source.as_bytes());
//! let metrics = analyzer.analyze()?;
//!
//! println!("Comment density: {:.2}%", metrics.density() * 100.0);
//! println!("Doc comments: {}", metrics.doc_comments.len());
//! # Ok::<(), anyhow::Error>(())
//! ```

use crate::node::Node;
use crate::traits::{ParserTrait, Search};
use crate::Lang;
use crate::analysis::checker::{NodeChecker, DefaultNodeChecker};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Type of comment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommentType {
    /// Documentation comment (///, /** */, etc.)
    Doc,
    /// Inline comment (//, #, etc.)
    Inline,
    /// Block comment (/* */, """ """, etc.)
    Block,
    /// Header/copyright comment at file top
    Header,
    /// TODO/FIXME/NOTE comment
    Annotation,
}

/// Represents a comment in source code
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    /// Type of comment
    pub comment_type: CommentType,
    /// Comment text (without delimiters)
    pub text: String,
    /// Start line (1-indexed)
    pub start_line: usize,
    /// End line (1-indexed)
    pub end_line: usize,
    /// Start byte offset
    pub start_byte: usize,
    /// End byte offset
    pub end_byte: usize,
    /// Associated code element (if any)
    pub associated_element: Option<String>,
}

impl Comment {
    /// Get the number of lines in this comment
    pub fn line_count(&self) -> usize {
        self.end_line.saturating_sub(self.start_line) + 1
    }

    /// Get the byte length of this comment
    pub fn byte_len(&self) -> usize {
        self.end_byte - self.start_byte
    }

    /// Check if this is a documentation comment
    pub fn is_doc_comment(&self) -> bool {
        self.comment_type == CommentType::Doc
    }

    /// Check if this comment contains annotations (TODO, FIXME, etc.)
    pub fn has_annotation(&self) -> bool {
        self.comment_type == CommentType::Annotation
            || self.text.contains("TODO")
            || self.text.contains("FIXME")
            || self.text.contains("XXX")
            || self.text.contains("HACK")
            || self.text.contains("NOTE")
    }

    /// Get the annotation type if present
    pub fn annotation_type(&self) -> Option<&str> {
        if self.text.contains("TODO") {
            Some("TODO")
        } else if self.text.contains("FIXME") {
            Some("FIXME")
        } else if self.text.contains("XXX") {
            Some("XXX")
        } else if self.text.contains("HACK") {
            Some("HACK")
        } else if self.text.contains("NOTE") {
            Some("NOTE")
        } else {
            None
        }
    }

    /// Calculate a simple quality score for this comment
    /// Higher score means better quality (more informative)
    pub fn quality_score(&self) -> f64 {
        let mut score = 0.0;

        // Longer comments are generally more informative
        let word_count = self.text.split_whitespace().count();
        score += (word_count as f64 * 0.5).min(10.0);

        // Doc comments are valuable
        if self.is_doc_comment() {
            score += 5.0;
        }

        // Penalize very short comments
        if word_count < 3 {
            score *= 0.5;
        }

        // Penalize commented-out code (heuristic: lots of punctuation)
        let punct_ratio = self.text.chars().filter(|c| c.is_ascii_punctuation()).count() as f64
            / self.text.len().max(1) as f64;
        if punct_ratio > 0.3 {
            score *= 0.5;
        }

        score
    }
}

/// Comprehensive comment metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentMetrics {
    /// All comments found
    pub comments: Vec<Comment>,
    /// Documentation comments
    pub doc_comments: Vec<Comment>,
    /// Inline comments
    pub inline_comments: Vec<Comment>,
    /// Block comments
    pub block_comments: Vec<Comment>,
    /// Header comments
    pub header_comments: Vec<Comment>,
    /// Annotation comments (TODO, FIXME, etc.)
    pub annotation_comments: Vec<Comment>,
    /// Total lines of code
    pub total_lines: usize,
    /// Lines with comments
    pub commented_lines: usize,
    /// Total bytes of code
    pub total_bytes: usize,
    /// Bytes in comments
    pub comment_bytes: usize,
}

impl CommentMetrics {
    /// Create empty metrics
    pub fn new(total_lines: usize, total_bytes: usize) -> Self {
        Self {
            comments: Vec::new(),
            doc_comments: Vec::new(),
            inline_comments: Vec::new(),
            block_comments: Vec::new(),
            header_comments: Vec::new(),
            annotation_comments: Vec::new(),
            total_lines,
            commented_lines: 0,
            total_bytes,
            comment_bytes: 0,
        }
    }

    /// Add a comment to metrics
    pub fn add_comment(&mut self, comment: Comment) {
        self.commented_lines += comment.line_count();
        self.comment_bytes += comment.byte_len();

        match comment.comment_type {
            CommentType::Doc => self.doc_comments.push(comment.clone()),
            CommentType::Inline => self.inline_comments.push(comment.clone()),
            CommentType::Block => self.block_comments.push(comment.clone()),
            CommentType::Header => self.header_comments.push(comment.clone()),
            CommentType::Annotation => self.annotation_comments.push(comment.clone()),
        }

        self.comments.push(comment);
    }

    /// Calculate comment density (commented lines / total lines)
    pub fn density(&self) -> f64 {
        if self.total_lines == 0 {
            0.0
        } else {
            self.commented_lines as f64 / self.total_lines as f64
        }
    }

    /// Calculate comment to code ratio (comment bytes / total bytes)
    pub fn comment_ratio(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            self.comment_bytes as f64 / self.total_bytes as f64
        }
    }

    /// Calculate documentation coverage (functions with doc comments / total functions)
    pub fn doc_coverage(&self, total_functions: usize) -> f64 {
        if total_functions == 0 {
            0.0
        } else {
            self.doc_comments.len() as f64 / total_functions as f64
        }
    }

    /// Get average comment quality score
    pub fn average_quality(&self) -> f64 {
        if self.comments.is_empty() {
            0.0
        } else {
            self.comments.iter().map(|c| c.quality_score()).sum::<f64>()
                / self.comments.len() as f64
        }
    }

    /// Count comments by type
    pub fn count_by_type(&self, comment_type: CommentType) -> usize {
        self.comments
            .iter()
            .filter(|c| c.comment_type == comment_type)
            .count()
    }

    /// Get all annotations grouped by type
    pub fn annotations_by_type(&self) -> std::collections::HashMap<String, Vec<&Comment>> {
        let mut map = std::collections::HashMap::new();

        for comment in &self.annotation_comments {
            if let Some(annotation) = comment.annotation_type() {
                map.entry(annotation.to_string())
                    .or_insert_with(Vec::new)
                    .push(comment);
            }
        }

        map
    }

    /// Check if documentation is adequate (>= 50% of functions documented)
    pub fn is_well_documented(&self, total_functions: usize) -> bool {
        self.doc_coverage(total_functions) >= 0.5
    }

    /// Get comments in a specific line range
    pub fn comments_in_range(&self, start_line: usize, end_line: usize) -> Vec<&Comment> {
        self.comments
            .iter()
            .filter(|c| c.start_line >= start_line && c.end_line <= end_line)
            .collect()
    }
}

/// Analyzer for extracting and analyzing comments
pub struct CommentAnalyzer<'a, T: ParserTrait> {
    parser: &'a T,
    code: &'a [u8],
    language: Lang,
}

impl<'a, T: ParserTrait> CommentAnalyzer<'a, T> {
    /// Create a new comment analyzer
    pub fn new(parser: &'a T, code: &'a [u8]) -> Self {
        Self {
            parser,
            code,
            language: parser.get_language(),
        }
    }

    /// Analyze all comments in the code
    pub fn analyze(&self) -> Result<CommentMetrics> {
        let root = self.parser.get_root();
        let total_lines = self.code.iter().filter(|&&b| b == b'\n').count() + 1;
        let total_bytes = self.code.len();

        let mut metrics = CommentMetrics::new(total_lines, total_bytes);

        root.act_on_node(&mut |node| {
            if DefaultNodeChecker::is_comment(node, self.language) {
                if let Some(comment) = self.extract_comment(node) {
                    metrics.add_comment(comment);
                }
            }
        });

        Ok(metrics)
    }

    /// Extract a comment from a node
    fn extract_comment(&self, node: &Node<'a>) -> Option<Comment> {
        let text = self.get_comment_text(node)?;
        let comment_type = self.classify_comment(node, &text);

        Some(Comment {
            comment_type,
            text,
            start_line: node.start_row() + 1,
            end_line: node.end_row() + 1,
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
            associated_element: self.find_associated_element(node),
        })
    }

    /// Get the text content of a comment (without delimiters)
    fn get_comment_text(&self, node: &Node<'a>) -> Option<String> {
        let full_text = node.utf8_text(self.code)?;
        let trimmed = self.strip_comment_delimiters(full_text);
        Some(trimmed.to_string())
    }

    /// Strip comment delimiters based on language
    fn strip_comment_delimiters<'b>(&self, text: &'b str) -> &'b str {
        match self.language {
            Lang::Rust | Lang::TypeScript | Lang::JavaScript | Lang::Java | Lang::Cpp => {
                if text.starts_with("///") {
                    text.trim_start_matches("///").trim()
                } else if text.starts_with("//!") {
                    text.trim_start_matches("//!").trim()
                } else if text.starts_with("//") {
                    text.trim_start_matches("//").trim()
                } else if text.starts_with("/*") && text.ends_with("*/") {
                    text.trim_start_matches("/*")
                        .trim_end_matches("*/")
                        .trim()
                } else {
                    text.trim()
                }
            }
            Lang::Python => {
                if text.starts_with('#') {
                    text.trim_start_matches('#').trim()
                } else {
                    text.trim()
                }
            }
            _ => text.trim(),
        }
    }

    /// Classify the type of comment
    fn classify_comment(&self, node: &Node<'a>, text: &str) -> CommentType {
        // Check for annotations
        if text.contains("TODO")
            || text.contains("FIXME")
            || text.contains("XXX")
            || text.contains("HACK")
        {
            return CommentType::Annotation;
        }

        // Check for doc comments
        if self.is_doc_comment(node) {
            return CommentType::Doc;
        }

        // Check for header comments (at top of file)
        if node.start_row() < 10
            && (text.contains("Copyright")
                || text.contains("License")
                || text.contains("SPDX")
                || text.len() > 100)
        {
            return CommentType::Header;
        }

        // Distinguish block vs inline
        if node.end_row() - node.start_row() > 0 {
            CommentType::Block
        } else {
            CommentType::Inline
        }
    }

    /// Check if this is a documentation comment
    fn is_doc_comment(&self, node: &Node<'a>) -> bool {
        let text = match node.utf8_text(self.code) {
            Some(t) => t,
            None => return false,
        };

        match self.language {
            Lang::Rust => {
                text.starts_with("///") || text.starts_with("//!") || text.starts_with("/**")
            }
            Lang::TypeScript | Lang::JavaScript => text.starts_with("/**"),
            Lang::Java => text.starts_with("/**"),
            Lang::Python => {
                // Python docstrings are handled differently - they're string literals
                false
            }
            Lang::Cpp => text.starts_with("///") || text.starts_with("/**"),
            _ => false,
        }
    }

    /// Find the code element associated with this comment
    fn find_associated_element(&self, node: &Node<'a>) -> Option<String> {
        // Look for the next named sibling
        let mut current = *node;
        while let Some(next) = current.next_sibling() {
            if next.is_named() {
                // Try to extract a name
                if let Some(name_node) = next.child_by_field_name("name") {
                    return name_node.utf8_text(self.code).map(|s| s.to_string());
                }
                return Some(next.kind().to_string());
            }
            current = next;
        }

        None
    }

    /// Extract documentation comments with their associated elements
    pub fn extract_doc_comments(&self) -> Result<Vec<(Comment, Option<String>)>> {
        let metrics = self.analyze()?;
        Ok(metrics
            .doc_comments
            .into_iter()
            .map(|c| {
                let element = c.associated_element.clone();
                (c, element)
            })
            .collect())
    }

    /// Find all TODO/FIXME comments
    pub fn find_annotations(&self) -> Result<Vec<Comment>> {
        let metrics = self.analyze()?;
        Ok(metrics.annotation_comments)
    }
}

/// Convenience function to analyze comments
pub fn analyze_comments<T: ParserTrait>(
    parser: &T,
    code: &[u8],
) -> Result<CommentMetrics> {
    let analyzer = CommentAnalyzer::new(parser, code);
    analyzer.analyze()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Parser, RustLanguage};
    use std::path::Path;

    #[test]
    fn test_comment_analysis() {
        let source = r#"
/// This is a doc comment
fn example() {
    // This is an inline comment
    let x = 1; // Another inline
}

/* Block comment
   spanning multiple lines */
fn test() {}
"#;

        let parser =
            Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs")).unwrap();
        let analyzer = CommentAnalyzer::new(&parser, source.as_bytes());
        let metrics = analyzer.analyze().unwrap();

        assert!(metrics.comments.len() >= 3);
        assert!(metrics.doc_comments.len() >= 1);
        assert!(metrics.inline_comments.len() >= 2);
    }

    #[test]
    fn test_comment_density() {
        let source = "// Comment\nfn main() {}\n";
        let parser =
            Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs")).unwrap();
        let analyzer = CommentAnalyzer::new(&parser, source.as_bytes());
        let metrics = analyzer.analyze().unwrap();

        assert!(metrics.density() > 0.0);
    }

    #[test]
    fn test_annotation_detection() {
        let source = "// TODO: implement this\nfn main() {}";
        let parser =
            Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("test.rs")).unwrap();
        let analyzer = CommentAnalyzer::new(&parser, source.as_bytes());
        let annotations = analyzer.find_annotations().unwrap();

        assert!(!annotations.is_empty());
        assert_eq!(annotations[0].annotation_type(), Some("TODO"));
    }

    #[test]
    fn test_comment_quality() {
        let good = Comment {
            comment_type: CommentType::Doc,
            text: "This is a comprehensive documentation comment explaining the function".to_string(),
            start_line: 1,
            end_line: 1,
            start_byte: 0,
            end_byte: 50,
            associated_element: Some("example".to_string()),
        };

        let poor = Comment {
            comment_type: CommentType::Inline,
            text: "x".to_string(),
            start_line: 1,
            end_line: 1,
            start_byte: 0,
            end_byte: 1,
            associated_element: None,
        };

        assert!(good.quality_score() > poor.quality_score());
    }
}
