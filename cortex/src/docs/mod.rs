pub mod indexer;
pub mod parser;

#[cfg(test)]
mod tests;

pub use indexer::DocIndexer;
pub use parser::{DocFormat, DocSection, MarkdownDoc, ParsedDoc};

use serde::{Deserialize, Serialize};

/// Documentation search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocResult {
    pub title: String,
    pub content: String,
    pub file: String,
    pub line_start: usize,
    pub line_end: usize,
    pub section_path: Vec<String>,
    pub relevance: f32,
    pub doc_type: DocType,
}

/// Type of documentation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DocType {
    Markdown,
    InlineComment,
    DocComment,
    CodeBlock,
}

/// Documentation index entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocEntry {
    pub id: String,
    pub title: String,
    pub content: String,
    pub file: String,
    pub line_start: usize,
    pub line_end: usize,
    pub section_path: Vec<String>,
    pub doc_type: DocType,
    pub language: Option<String>,
    pub links: Vec<String>,
    pub code_blocks: Vec<CodeBlock>,
}

/// Code block extracted from documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBlock {
    pub language: Option<String>,
    pub content: String,
    pub line_number: usize,
}

impl DocResult {
    pub fn new(
        title: String,
        content: String,
        file: String,
        line_start: usize,
        line_end: usize,
        doc_type: DocType,
    ) -> Self {
        Self {
            title,
            content,
            file,
            line_start,
            line_end,
            section_path: Vec::new(),
            relevance: 1.0,
            doc_type,
        }
    }

    pub fn with_section_path(mut self, path: Vec<String>) -> Self {
        self.section_path = path;
        self
    }

    pub fn with_relevance(mut self, relevance: f32) -> Self {
        self.relevance = relevance;
        self
    }
}

impl DocEntry {
    pub fn new(
        id: String,
        title: String,
        content: String,
        file: String,
        line_start: usize,
        line_end: usize,
        doc_type: DocType,
    ) -> Self {
        Self {
            id,
            title,
            content,
            file,
            line_start,
            line_end,
            section_path: Vec::new(),
            doc_type,
            language: None,
            links: Vec::new(),
            code_blocks: Vec::new(),
        }
    }
}
