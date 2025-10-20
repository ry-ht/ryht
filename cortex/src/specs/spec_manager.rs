use super::markdown_analyzer::{MarkdownAnalyzer, MarkdownDocument};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecificationRegistry {
    pub specs: Vec<SpecificationInfo>,
    pub base_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecificationInfo {
    pub name: String,
    pub path: PathBuf,
    pub version: String,
    pub status: String,
    pub sections: Vec<String>,
    pub size_bytes: u64,
    pub last_modified: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub issues: Vec<ValidationIssue>,
    pub completeness_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub severity: IssueSeverity,
    pub message: String,
    pub section: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

pub struct SpecificationManager {
    base_path: PathBuf,
    cache: HashMap<String, MarkdownDocument>,
}

impl SpecificationManager {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            cache: HashMap::new(),
        }
    }

    /// Discover all specifications in the base path
    pub fn discover_specs(&self) -> Result<SpecificationRegistry> {
        let mut specs = Vec::new();

        if !self.base_path.exists() {
            fs::create_dir_all(&self.base_path)?;
        }

        for entry in fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Ok(metadata) = fs::metadata(&path) {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(doc) = MarkdownAnalyzer::parse(
                            path.to_str().unwrap_or("unknown"),
                            &content,
                        ) {
                            let name = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown")
                                .to_string();

                            let last_modified = metadata
                                .modified()
                                .ok()
                                .and_then(|t| t
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .ok())
                                .map(|d| {
                                    chrono::DateTime::from_timestamp(d.as_secs() as i64, 0)
                                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                        .unwrap_or_else(|| "unknown".to_string())
                                })
                                .unwrap_or_else(|| "unknown".to_string());

                            specs.push(SpecificationInfo {
                                name,
                                path: path.clone(),
                                version: doc
                                    .metadata
                                    .version
                                    .unwrap_or_else(|| "unknown".to_string()),
                                status: doc
                                    .metadata
                                    .status
                                    .unwrap_or_else(|| "draft".to_string()),
                                sections: doc.sections.iter().map(|s| s.title.clone()).collect(),
                                size_bytes: metadata.len(),
                                last_modified,
                            });
                        }
                    }
                }
            }
        }

        // Sort by name for consistent ordering
        specs.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(SpecificationRegistry {
            specs,
            base_path: self.base_path.clone(),
        })
    }

    /// Get a specification by name
    pub fn get_spec(&mut self, name: &str) -> Result<MarkdownDocument> {
        // Check cache first
        if let Some(doc) = self.cache.get(name) {
            return Ok(doc.clone());
        }

        // Try to load from file
        let path = self.base_path.join(format!("{}.md", name));
        if !path.exists() {
            anyhow::bail!("Specification not found: {}", name);
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read specification: {}", name))?;

        let doc = MarkdownAnalyzer::parse(path.to_str().unwrap_or(name), &content)
            .with_context(|| format!("Failed to parse specification: {}", name))?;

        // Cache for future use
        self.cache.insert(name.to_string(), doc.clone());

        Ok(doc)
    }

    /// Get a specific section from a specification
    pub fn get_section(&mut self, spec_name: &str, section_name: &str) -> Result<String> {
        let doc = self.get_spec(spec_name)?;

        if let Some(section) = MarkdownAnalyzer::extract_section(&doc, section_name) {
            // The markdown parser now automatically collects subsection content into parent sections
            // via populate_section_content(), so we can directly return the section content
            Ok(format!(
                "# {}\n\n{}",
                section.title,
                section.content.trim()
            ))
        } else {
            anyhow::bail!(
                "Section '{}' not found in specification '{}'",
                section_name,
                spec_name
            )
        }
    }

    /// List all sections in a specification
    pub fn list_sections(&mut self, spec_name: &str) -> Result<Vec<String>> {
        let doc = self.get_spec(spec_name)?;
        Ok(doc.sections.iter().map(|s| s.title.clone()).collect())
    }

    /// Get the structure of a specification
    pub fn get_structure(&mut self, spec_name: &str) -> Result<String> {
        let doc = self.get_spec(spec_name)?;
        Ok(MarkdownAnalyzer::get_structure_summary(&doc))
    }

    /// Search across all specifications
    pub fn search_all(&mut self, query: &str) -> Result<Vec<SpecSearchResult>> {
        let registry = self.discover_specs()?;
        let mut all_results = Vec::new();

        for spec_info in registry.specs {
            let spec_name = spec_info.name.clone();

            // Try to get the spec
            if let Ok(doc) = self.get_spec(&spec_name) {
                let results = MarkdownAnalyzer::search(&doc, query);

                for result in results {
                    all_results.push(SpecSearchResult {
                        spec_name: spec_name.clone(),
                        spec_path: spec_info.path.clone(),
                        section_title: result.section_title,
                        line_start: result.line_start,
                        line_end: result.line_end,
                        snippet: result.snippet,
                    });
                }
            }
        }

        Ok(all_results)
    }

    /// Validate specification completeness
    pub fn validate(&mut self, spec_name: &str) -> Result<ValidationResult> {
        let doc = self.get_spec(spec_name)?;
        let mut issues = Vec::new();
        let mut score: f64 = 100.0;

        // Check for title
        if doc.title.is_empty() || doc.title == "Untitled" {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Error,
                message: "Missing document title".to_string(),
                section: None,
            });
            score -= 20.0;
        }

        // Check for version
        if doc.metadata.version.is_none() {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Warning,
                message: "Missing version information".to_string(),
                section: None,
            });
            score -= 10.0;
        }

        // Check for status
        if doc.metadata.status.is_none() {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Warning,
                message: "Missing status information".to_string(),
                section: None,
            });
            score -= 10.0;
        }

        // Check for empty sections
        for section in &doc.sections {
            if section.content.trim().is_empty() {
                issues.push(ValidationIssue {
                    severity: IssueSeverity::Warning,
                    message: format!("Empty section: {}", section.title),
                    section: Some(section.title.clone()),
                });
                score -= 5.0;
            }
        }

        // Check minimum sections
        if doc.sections.len() < 2 {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Warning,
                message: "Very few sections (< 2)".to_string(),
                section: None,
            });
            score -= 15.0;
        }

        // Ensure score is between 0 and 100
        score = score.max(0.0).min(100.0);

        Ok(ValidationResult {
            valid: score >= 70.0,
            issues,
            completeness_score: score,
        })
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecSearchResult {
    pub spec_name: String,
    pub spec_path: PathBuf,
    pub section_title: String,
    pub line_start: usize,
    pub line_end: usize,
    pub snippet: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_test_spec(dir: &Path, name: &str, content: &str) -> Result<()> {
        let path = dir.join(format!("{}.md", name));
        fs::write(path, content)?;
        Ok(())
    }

    #[test]
    fn test_discover_specs() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let base_path = temp_dir.path().to_path_buf();

        create_test_spec(
            &base_path,
            "spec1",
            "# Spec 1\n\nVersion: 1.0.0\n\n## Section A\n\nContent.",
        )?;

        create_test_spec(
            &base_path,
            "spec2",
            "# Spec 2\n\nStatus: Draft\n\n## Section B\n\nMore content.",
        )?;

        let manager = SpecificationManager::new(base_path);
        let registry = manager.discover_specs()?;

        assert_eq!(registry.specs.len(), 2);
        assert_eq!(registry.specs[0].name, "spec1");
        assert_eq!(registry.specs[1].name, "spec2");

        Ok(())
    }

    #[test]
    fn test_get_spec() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let base_path = temp_dir.path().to_path_buf();

        create_test_spec(
            &base_path,
            "test",
            "# Test Spec\n\n## Introduction\n\nThis is a test.",
        )?;

        let mut manager = SpecificationManager::new(base_path);
        let doc = manager.get_spec("test")?;

        assert_eq!(doc.title, "Test Spec");
        assert!(doc.sections.iter().any(|s| s.title == "Introduction"));

        Ok(())
    }

    #[test]
    fn test_get_section() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let base_path = temp_dir.path().to_path_buf();

        create_test_spec(
            &base_path,
            "test",
            "# Test\n\n## Overview\n\nOverview content.\n\n## Details\n\nDetailed info.",
        )?;

        let mut manager = SpecificationManager::new(base_path);
        let section = manager.get_section("test", "overview")?;

        assert!(section.contains("Overview"));
        assert!(section.contains("Overview content"));

        Ok(())
    }

    #[test]
    fn test_search_all() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let base_path = temp_dir.path().to_path_buf();

        create_test_spec(
            &base_path,
            "spec1",
            "# Spec 1\n\n## Section\n\nFindable keyword here.",
        )?;

        create_test_spec(
            &base_path,
            "spec2",
            "# Spec 2\n\n## Other\n\nNo match here.",
        )?;

        let mut manager = SpecificationManager::new(base_path);
        let results = manager.search_all("Findable")?;

        // After hierarchical content collection, text appears in both:
        // 1. Section (original location)
        // 2. Spec 1 (parent heading, contains all subsection content)
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].spec_name, "spec1");
        assert_eq!(results[1].spec_name, "spec1");

        Ok(())
    }

    #[test]
    fn test_validate() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let base_path = temp_dir.path().to_path_buf();

        create_test_spec(
            &base_path,
            "complete",
            "# Complete Spec\n\nVersion: 1.0.0\nStatus: Final\n\n## Section 1\n\nContent.\n\n## Section 2\n\nMore content.",
        )?;

        create_test_spec(
            &base_path,
            "incomplete",
            "# \n\n## Section\n\n",  // Empty title should trigger error
        )?;

        let mut manager = SpecificationManager::new(base_path);

        let result1 = manager.validate("complete")?;
        assert!(result1.valid);
        assert!(result1.completeness_score > 70.0);

        let result2 = manager.validate("incomplete")?;
        // Empty title should make it invalid
        assert!(!result2.valid || result2.completeness_score < 70.0);

        Ok(())
    }

    #[test]
    fn test_cache() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let base_path = temp_dir.path().to_path_buf();

        create_test_spec(
            &base_path,
            "cached",
            "# Cached Spec\n\n## Section\n\nContent.",
        )?;

        let mut manager = SpecificationManager::new(base_path);

        assert_eq!(manager.cache_size(), 0);

        manager.get_spec("cached")?;
        assert_eq!(manager.cache_size(), 1);

        manager.get_spec("cached")?;
        assert_eq!(manager.cache_size(), 1); // Still 1, not 2

        manager.clear_cache();
        assert_eq!(manager.cache_size(), 0);

        Ok(())
    }

    #[test]
    fn test_get_section_with_hierarchical_structure() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let base_path = temp_dir.path().to_path_buf();

        // Create spec with hierarchical structure (mimics real spec.md)
        create_test_spec(
            &base_path,
            "hierarchical",
            r#"# Test Spec

## Introduction

### Concept

This is the concept text.

### Key Principles

1. First principle
2. Second principle

## Implementation

Direct implementation text.

### Details

Implementation details here."#,
        )?;

        let mut manager = SpecificationManager::new(base_path);

        // Test getting Introduction section - should include subsection CONTENT (not titles)
        let intro_section = manager.get_section("hierarchical", "introduction")?;

        assert!(intro_section.contains("Introduction"), "Should have section title");
        assert!(intro_section.contains("This is the concept text"), "Should include Concept subsection content");
        assert!(intro_section.contains("First principle"), "Should include Key Principles subsection content");
        assert!(intro_section.contains("Second principle"), "Should include principles list");

        // Test getting Implementation section
        let impl_section = manager.get_section("hierarchical", "implementation")?;

        assert!(impl_section.contains("Implementation"), "Should have section title");
        assert!(impl_section.contains("Direct implementation text"), "Should include direct content");
        assert!(impl_section.contains("Implementation details here"), "Should include Details subsection content");

        Ok(())
    }
}
