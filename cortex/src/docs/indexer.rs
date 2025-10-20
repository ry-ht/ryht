use super::parser::{detect_format, parse_doc_comments, parse_markdown, DocFormat};
use super::{DocEntry, DocResult, DocType};
use anyhow::{Context, Result};
use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::debug;

/// Documentation indexer for markdown files and code documentation
pub struct DocIndexer {
    /// Indexed documentation entries
    entries: Arc<DashMap<String, DocEntry>>,
    /// Index by file path
    file_index: Arc<DashMap<PathBuf, Vec<String>>>,
    /// Inverted index for search
    search_index: Arc<DashMap<String, Vec<String>>>,
    /// Symbol to documentation mapping
    symbol_docs: Arc<DashMap<String, Vec<String>>>,
}

impl DocIndexer {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
            file_index: Arc::new(DashMap::new()),
            search_index: Arc::new(DashMap::new()),
            symbol_docs: Arc::new(DashMap::new()),
        }
    }

    /// Index a markdown file
    pub async fn index_markdown_file(&self, path: &Path) -> Result<usize> {
        let content = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read markdown file: {}", path.display()))?;

        let doc = parse_markdown(&content, &path.to_string_lossy())?;
        let mut entry_ids = Vec::new();

        // Index sections
        for section in doc.sections {
            let id = format!(
                "{}:{}:{}",
                path.display(),
                section.line_start,
                section.line_end
            );

            let mut entry = DocEntry::new(
                id.clone(),
                section.title.clone(),
                section.content,
                path.to_string_lossy().to_string(),
                section.line_start,
                section.line_end,
                DocType::Markdown,
            );

            entry.section_path = section.path;
            entry_ids.push(id.clone());

            // Add to search index
            self.add_to_search_index(&entry);

            self.entries.insert(id, entry);
        }

        // Store code blocks
        for entry_id in entry_ids.iter() {
            if let Some(mut entry) = self.entries.get_mut(entry_id) {
                entry.code_blocks = doc
                    .code_blocks
                    .iter()
                    .filter(|cb| {
                        cb.line_number >= entry.line_start && cb.line_number <= entry.line_end
                    })
                    .cloned()
                    .collect();
            }
        }

        // Store links in all entries
        for entry_id in &entry_ids {
            if let Some(mut entry) = self.entries.get_mut(entry_id) {
                entry.links = doc.links.clone();
            }
        }

        self.file_index.insert(path.to_path_buf(), entry_ids.clone());

        debug!(
            "Indexed markdown file: {} ({} sections, {} code blocks)",
            path.display(),
            entry_ids.len(),
            doc.code_blocks.len()
        );

        Ok(entry_ids.len())
    }

    /// Index documentation comments from a source file
    pub async fn index_source_file(&self, path: &Path) -> Result<usize> {
        let format = detect_format(path);
        if format.is_none() || format == Some(DocFormat::Markdown) {
            return Ok(0);
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read source file: {}", path.display()))?;

        let entries = parse_doc_comments(&content, &path.to_string_lossy(), format.unwrap())?;
        let mut entry_ids = Vec::new();

        for entry in entries {
            let id = entry.id.clone();
            entry_ids.push(id.clone());

            self.add_to_search_index(&entry);
            self.entries.insert(id, entry);
        }

        if !entry_ids.is_empty() {
            self.file_index.insert(path.to_path_buf(), entry_ids.clone());

            debug!(
                "Indexed source file: {} ({} doc comments)",
                path.display(),
                entry_ids.len()
            );
        }

        Ok(entry_ids.len())
    }

    /// Link documentation to a code symbol
    pub fn link_symbol_to_docs(&self, symbol_name: &str, doc_ids: Vec<String>) {
        self.symbol_docs.insert(symbol_name.to_string(), doc_ids);
    }

    /// Search documentation by query
    pub async fn search_docs(&self, query: &str, limit: usize) -> Result<Vec<DocResult>> {
        let start = std::time::Instant::now();

        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut results: Vec<(DocResult, f32)> = Vec::new();

        // Search through all entries
        for entry_ref in self.entries.iter() {
            let entry = entry_ref.value();
            let mut score = 0.0f32;

            // Check title match
            let title_lower = entry.title.to_lowercase();
            for term in &query_terms {
                if title_lower.contains(term) {
                    score += 3.0; // Title matches are worth more
                }
            }

            // Check content match
            let content_lower = entry.content.to_lowercase();
            for term in &query_terms {
                if content_lower.contains(term) {
                    score += 1.0;
                }
            }

            // Check section path
            for section_part in &entry.section_path {
                let section_lower = section_part.to_lowercase();
                for term in &query_terms {
                    if section_lower.contains(term) {
                        score += 2.0;
                    }
                }
            }

            if score > 0.0 {
                let result = DocResult {
                    title: entry.title.clone(),
                    content: self.extract_context(&entry.content, &query_terms, 200),
                    file: entry.file.clone(),
                    line_start: entry.line_start,
                    line_end: entry.line_end,
                    section_path: entry.section_path.clone(),
                    relevance: score,
                    doc_type: entry.doc_type.clone(),
                };

                results.push((result, score));
            }
        }

        // Sort by relevance
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let final_results: Vec<DocResult> = results
            .into_iter()
            .take(limit)
            .map(|(result, _)| result)
            .collect();

        let elapsed = start.elapsed();
        debug!(
            "Documentation search completed in {:?}: found {} results for '{}'",
            elapsed,
            final_results.len(),
            query
        );

        Ok(final_results)
    }

    /// Get documentation for a specific symbol
    pub async fn get_docs_for_symbol(&self, symbol_name: &str) -> Result<Vec<DocResult>> {
        let mut results = Vec::new();

        // First, check if we have explicit links
        if let Some(doc_ids) = self.symbol_docs.get(symbol_name) {
            for doc_id in doc_ids.value() {
                if let Some(entry) = self.entries.get(doc_id) {
                    results.push(DocResult {
                        title: entry.title.clone(),
                        content: entry.content.clone(),
                        file: entry.file.clone(),
                        line_start: entry.line_start,
                        line_end: entry.line_end,
                        section_path: entry.section_path.clone(),
                        relevance: 1.0,
                        doc_type: entry.doc_type.clone(),
                    });
                }
            }
        }

        // If no explicit links, search by symbol name
        if results.is_empty() {
            results = self.search_docs(symbol_name, 5).await?;
        }

        Ok(results)
    }

    /// Get all documentation entries for a file
    pub fn get_docs_for_file(&self, file_path: &Path) -> Vec<DocResult> {
        let mut results = Vec::new();

        if let Some(entry_ids) = self.file_index.get(file_path) {
            for entry_id in entry_ids.value() {
                if let Some(entry) = self.entries.get(entry_id) {
                    results.push(DocResult {
                        title: entry.title.clone(),
                        content: entry.content.clone(),
                        file: entry.file.clone(),
                        line_start: entry.line_start,
                        line_end: entry.line_end,
                        section_path: entry.section_path.clone(),
                        relevance: 1.0,
                        doc_type: entry.doc_type.clone(),
                    });
                }
            }
        }

        results
    }

    /// Extract context around query terms
    fn extract_context(&self, content: &str, query_terms: &[&str], max_len: usize) -> String {
        // Find first match position
        let content_lower = content.to_lowercase();
        let mut best_pos = None;
        let mut best_term_len = 0;

        for term in query_terms {
            if let Some(pos) = content_lower.find(term) {
                if best_pos.is_none() || pos < best_pos.unwrap() {
                    best_pos = Some(pos);
                    best_term_len = term.len();
                }
            }
        }

        if let Some(pos) = best_pos {
            // Extract context around the match
            let start = pos.saturating_sub(max_len / 2);
            let end = (pos + best_term_len + max_len / 2).min(content.len());

            let mut context = content[start..end].to_string();

            // Add ellipsis if truncated
            if start > 0 {
                context = format!("...{}", context);
            }
            if end < content.len() {
                context = format!("{}...", context);
            }

            context
        } else {
            // No match found, return beginning of content
            let end = max_len.min(content.len());
            let mut result = content[..end].to_string();
            if end < content.len() {
                result.push_str("...");
            }
            result
        }
    }

    /// Add entry to search index
    fn add_to_search_index(&self, entry: &DocEntry) {
        let mut terms = Vec::new();

        // Tokenize title
        for word in entry.title.to_lowercase().split_whitespace() {
            terms.push(word.to_string());
        }

        // Tokenize content
        for word in entry.content.to_lowercase().split_whitespace() {
            if word.len() > 2 {
                // Skip very short words
                terms.push(word.to_string());
            }
        }

        // Add to inverted index
        for term in terms {
            self.search_index
                .entry(term)
                .or_default()
                .push(entry.id.clone());
        }
    }

    /// Get statistics about the documentation index
    pub fn stats(&self) -> DocIndexStats {
        let mut total_markdown = 0;
        let mut total_doc_comments = 0;
        let mut total_code_blocks = 0;

        for entry in self.entries.iter() {
            match entry.doc_type {
                DocType::Markdown => total_markdown += 1,
                DocType::DocComment => total_doc_comments += 1,
                _ => {}
            }
            total_code_blocks += entry.code_blocks.len();
        }

        DocIndexStats {
            total_entries: self.entries.len(),
            total_files: self.file_index.len(),
            total_markdown,
            total_doc_comments,
            total_code_blocks,
            total_search_terms: self.search_index.len(),
        }
    }

    /// Clear the index
    pub fn clear(&self) {
        self.entries.clear();
        self.file_index.clear();
        self.search_index.clear();
        self.symbol_docs.clear();
    }
}

impl Default for DocIndexer {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the documentation index
#[derive(Debug, Clone)]
pub struct DocIndexStats {
    pub total_entries: usize,
    pub total_files: usize,
    pub total_markdown: usize,
    pub total_doc_comments: usize,
    pub total_code_blocks: usize,
    pub total_search_terms: usize,
}
