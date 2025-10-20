//! External project import functionality.
//!
//! This module provides functionality to import entire codebases and external
//! projects into the Cortex system, respecting .gitignore patterns and processing
//! files appropriately.

use crate::extractor::{extract_comprehensive_metadata, detect_programming_language};
use crate::filters::{should_ignore_dir, should_ignore_file};
use crate::processors::{detect_content_type, ProcessorFactory};
use cortex_core::error::{CortexError, Result};
use cortex_core::id::CortexId;
use ignore::WalkBuilder;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;

/// Options for project import
#[derive(Debug, Clone)]
pub struct ProjectImportOptions {
    /// Make files read-only
    pub read_only: bool,
    /// Create editable fork
    pub create_fork: bool,
    /// Include patterns (globs)
    pub include_patterns: Vec<String>,
    /// Exclude patterns (globs)
    pub exclude_patterns: Vec<String>,
    /// Maximum directory traversal depth
    pub max_depth: Option<usize>,
    /// Process and parse code files
    pub process_code: bool,
    /// Generate embeddings for content
    pub generate_embeddings: bool,
    /// Follow symbolic links
    pub follow_links: bool,
    /// Respect .gitignore files
    pub respect_gitignore: bool,
}

impl Default for ProjectImportOptions {
    fn default() -> Self {
        Self {
            read_only: false,
            create_fork: false,
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            max_depth: None,
            process_code: true,
            generate_embeddings: false,
            follow_links: false,
            respect_gitignore: true,
        }
    }
}

/// Report of import operation
#[derive(Debug, Clone)]
pub struct ImportReport {
    /// Number of files imported
    pub files_imported: usize,
    /// Number of directories created
    pub directories_created: usize,
    /// Number of files skipped
    pub files_skipped: usize,
    /// Number of errors encountered
    pub errors: usize,
    /// Total bytes processed
    pub bytes_processed: u64,
    /// Import duration
    pub duration_secs: f64,
}

impl ImportReport {
    pub fn new() -> Self {
        Self {
            files_imported: 0,
            directories_created: 0,
            files_skipped: 0,
            errors: 0,
            bytes_processed: 0,
            duration_secs: 0.0,
        }
    }
}

impl Default for ImportReport {
    fn default() -> Self {
        Self::new()
    }
}

/// File information from import
#[derive(Debug, Clone)]
pub struct ImportedFile {
    /// Unique ID
    pub id: CortexId,
    /// Relative path within project
    pub relative_path: String,
    /// Content hash
    pub content_hash: String,
    /// File size in bytes
    pub size_bytes: u64,
    /// Content type
    pub content_type: String,
    /// Metadata extracted from file
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

/// Project loader for importing external projects
pub struct ProjectLoader {
    processor_factory: Arc<ProcessorFactory>,
}

impl ProjectLoader {
    /// Create a new project loader
    pub fn new() -> Self {
        Self {
            processor_factory: Arc::new(ProcessorFactory::new()),
        }
    }

    /// Import an external project from a directory
    pub async fn import_project(
        &self,
        source_path: &Path,
        options: ProjectImportOptions,
    ) -> Result<(Vec<ImportedFile>, ImportReport)> {
        let start_time = std::time::Instant::now();

        if !source_path.exists() {
            return Err(CortexError::ingestion(format!(
                "Source path does not exist: {}",
                source_path.display()
            )));
        }

        if !source_path.is_dir() {
            return Err(CortexError::ingestion(format!(
                "Source path is not a directory: {}",
                source_path.display()
            )));
        }

        tracing::info!("Importing project from: {}", source_path.display());

        let mut report = ImportReport::new();
        let mut imported_files = Vec::new();

        // Configure walker
        let mut walker = WalkBuilder::new(source_path);
        walker
            .hidden(false)
            .git_ignore(options.respect_gitignore)
            .follow_links(options.follow_links);

        if let Some(max_depth) = options.max_depth {
            walker.max_depth(Some(max_depth));
        }

        // Walk directory tree
        for entry in walker.build() {
            match entry {
                Ok(entry) => {
                    let path = entry.path();

                    // Skip if it's the root directory
                    if path == source_path {
                        continue;
                    }

                    // Get relative path
                    let relative_path = match path.strip_prefix(source_path) {
                        Ok(rel) => rel,
                        Err(_) => {
                            tracing::warn!("Could not get relative path for: {}", path.display());
                            continue;
                        }
                    };

                    if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                        // Directory
                        if should_ignore_dir(path) {
                            report.files_skipped += 1;
                            continue;
                        }
                        report.directories_created += 1;
                    } else if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                        // File
                        if should_ignore_file(path) {
                            report.files_skipped += 1;
                            continue;
                        }

                        // Import file
                        match self.import_file(path, relative_path, &options).await {
                            Ok(imported_file) => {
                                report.bytes_processed += imported_file.size_bytes;
                                report.files_imported += 1;
                                imported_files.push(imported_file);
                            }
                            Err(e) => {
                                tracing::warn!("Failed to import {}: {}", path.display(), e);
                                report.errors += 1;
                                report.files_skipped += 1;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Walker error: {}", e);
                    report.errors += 1;
                }
            }
        }

        report.duration_secs = start_time.elapsed().as_secs_f64();

        tracing::info!(
            "Project import completed: {} files imported, {} skipped, {} errors in {:.2}s",
            report.files_imported,
            report.files_skipped,
            report.errors,
            report.duration_secs
        );

        Ok((imported_files, report))
    }

    /// Import a single file
    async fn import_file(
        &self,
        physical_path: &Path,
        relative_path: &Path,
        options: &ProjectImportOptions,
    ) -> Result<ImportedFile> {
        // Read file content
        let content = fs::read(physical_path).await.map_err(|e| {
            CortexError::ingestion(format!("Failed to read file: {}", e))
        })?;

        // Calculate content hash
        let content_hash = blake3::hash(&content).to_hex().to_string();

        // Detect content type
        let content_type = detect_content_type(physical_path);

        // Extract metadata
        let content_str = String::from_utf8_lossy(&content);
        let metadata = extract_comprehensive_metadata(physical_path, &content_str);

        // Process content if requested
        if options.process_code {
            if let Some(processor) = self.processor_factory.get_for_path(physical_path) {
                match processor.process(&content).await {
                    Ok(processed) => {
                        tracing::debug!(
                            "Processed {}: {} chunks",
                            relative_path.display(),
                            processed.chunks.len()
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to process {}: {}",
                            relative_path.display(),
                            e
                        );
                    }
                }
            }
        }

        Ok(ImportedFile {
            id: CortexId::new(),
            relative_path: relative_path.to_string_lossy().to_string(),
            content_hash,
            size_bytes: content.len() as u64,
            content_type: format!("{:?}", content_type),
            metadata,
        })
    }

    /// Get project statistics without full import
    pub async fn analyze_project(
        &self,
        source_path: &Path,
        options: ProjectImportOptions,
    ) -> Result<ProjectStats> {
        let mut stats = ProjectStats::default();

        let mut walker = WalkBuilder::new(source_path);
        walker
            .hidden(false)
            .git_ignore(options.respect_gitignore)
            .follow_links(options.follow_links);

        if let Some(max_depth) = options.max_depth {
            walker.max_depth(Some(max_depth));
        }

        for entry in walker.build() {
            if let Ok(entry) = entry {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    stats.directory_count += 1;
                } else if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                    stats.file_count += 1;

                    if let Ok(metadata) = entry.metadata() {
                        stats.total_size_bytes += metadata.len();
                    }

                    // Count by file type
                    if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                        *stats.file_types.entry(ext.to_string()).or_insert(0) += 1;
                    }

                    // Count by language
                    if let Some(lang) = detect_programming_language(entry.path()) {
                        *stats.languages.entry(lang).or_insert(0) += 1;
                    }
                }
            }
        }

        Ok(stats)
    }
}

impl Default for ProjectLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Project statistics
#[derive(Debug, Clone, Default)]
pub struct ProjectStats {
    pub file_count: usize,
    pub directory_count: usize,
    pub total_size_bytes: u64,
    pub file_types: std::collections::HashMap<String, usize>,
    pub languages: std::collections::HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

    async fn create_test_project() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create directory structure
        fs::create_dir(base_path.join("src")).await.unwrap();
        fs::create_dir(base_path.join("tests")).await.unwrap();

        // Create some files
        let mut file = File::create(base_path.join("README.md")).await.unwrap();
        file.write_all(b"# Test Project\n\nA test project.").await.unwrap();

        let mut file = File::create(base_path.join("src/main.rs")).await.unwrap();
        file.write_all(b"fn main() {\n    println!(\"Hello\");\n}").await.unwrap();

        let mut file = File::create(base_path.join("tests/test.rs")).await.unwrap();
        file.write_all(b"#[test]\nfn test_something() {\n    assert!(true);\n}").await.unwrap();

        temp_dir
    }

    #[tokio::test]
    async fn test_project_import() {
        let temp_dir = create_test_project().await;
        let loader = ProjectLoader::new();

        let options = ProjectImportOptions::default();
        let result = loader.import_project(temp_dir.path(), options).await;

        assert!(result.is_ok());
        let (files, report) = result.unwrap();
        assert!(files.len() >= 3);
        assert!(report.files_imported >= 3);
    }

    #[tokio::test]
    async fn test_project_analysis() {
        let temp_dir = create_test_project().await;
        let loader = ProjectLoader::new();

        let options = ProjectImportOptions::default();
        let stats = loader.analyze_project(temp_dir.path(), options).await.unwrap();

        assert!(stats.file_count >= 3);
        assert!(stats.directory_count >= 2);
    }
}
