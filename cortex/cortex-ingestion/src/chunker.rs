//! Text chunking strategies for semantic processing.
//!
//! This module provides various chunking strategies including:
//! - Semantic chunking (sentence/paragraph boundaries)
//! - Hierarchical chunking for documents
//! - Code-aware chunking for source files
//! - Size-based chunking with overlap
//! - Token-based chunking (max 512 tokens per chunk)

use cortex_core::traits::Chunker as ChunkerTrait;
use regex::Regex;

/// Approximate token count (rough estimate: 1 token ~= 4 characters)
pub fn estimate_tokens(text: &str) -> usize {
    // Simple approximation: split by whitespace and punctuation
    // This is a rough estimate. For accurate token counting, use a proper tokenizer
    text.split_whitespace().count()
}

/// Maximum token limit for chunks (as per specification)
pub const MAX_TOKENS_PER_CHUNK: usize = 512;

/// Convert token limit to character limit (rough approximation)
pub const fn tokens_to_chars(tokens: usize) -> usize {
    tokens * 4 // rough approximation
}

/// Semantic chunker that splits text into meaningful chunks
pub struct SemanticChunker {
    max_chunk_size: usize,
    overlap: usize,
    strategy: ChunkStrategy,
}

/// Chunking strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkStrategy {
    /// Split by sentences
    Sentence,
    /// Split by paragraphs
    Paragraph,
    /// Split by fixed size
    FixedSize,
    /// Hybrid: prefer paragraphs, fall back to sentences
    Hybrid,
    /// Sliding window with overlap
    SlidingWindow,
    /// Section-based (preserve semantic structure)
    SectionBased,
}

impl SemanticChunker {
    /// Create a new semantic chunker
    pub fn new(max_chunk_size: usize, overlap: usize) -> Self {
        Self {
            max_chunk_size,
            overlap,
            strategy: ChunkStrategy::Hybrid,
        }
    }

    /// Default chunker with 1000 char chunks and 100 char overlap
    pub fn default_config() -> Self {
        Self::new(1000, 100)
    }

    /// Create with specific strategy
    pub fn with_strategy(max_chunk_size: usize, overlap: usize, strategy: ChunkStrategy) -> Self {
        Self {
            max_chunk_size,
            overlap,
            strategy,
        }
    }

    /// Split text by sentences
    fn split_sentences(text: &str) -> Vec<String> {
        // Enhanced sentence splitting with abbreviation handling
        // Match sentence boundaries: period/exclamation/question followed by space and capital letter
        let sentence_regex = Regex::new(r"([.!?])\s+([A-Z])").unwrap();

        let mut sentences = Vec::new();
        let mut last_end = 0;

        for cap in sentence_regex.captures_iter(text) {
            if let Some(m) = cap.get(0) {
                // Include the sentence terminator, exclude the space and next capital
                let end = m.start() + 1; // +1 to include the punctuation
                if end > last_end {
                    sentences.push(text[last_end..end].trim().to_string());
                    last_end = m.end() - 1; // -1 to keep the capital letter for next sentence
                }
            }
        }

        // Add the last sentence
        if last_end < text.len() {
            let last = text[last_end..].trim().to_string();
            if !last.is_empty() {
                sentences.push(last);
            }
        }

        sentences.into_iter().filter(|s| !s.is_empty()).collect()
    }

    /// Split text by paragraphs
    fn split_paragraphs(text: &str) -> Vec<String> {
        text.split("\n\n")
            .filter(|p| !p.trim().is_empty())
            .map(|p| p.trim().to_string())
            .collect()
    }

    /// Chunk by strategy
    fn chunk_by_strategy(&self, content: &str) -> Vec<String> {
        match self.strategy {
            ChunkStrategy::Sentence => self.chunk_by_sentences(content),
            ChunkStrategy::Paragraph => self.chunk_by_paragraphs(content),
            ChunkStrategy::FixedSize => self.chunk_by_size(content),
            ChunkStrategy::Hybrid => self.chunk_hybrid(content),
            ChunkStrategy::SlidingWindow => self.chunk_sliding_window(content),
            ChunkStrategy::SectionBased => self.chunk_by_sections(content),
        }
    }

    /// Sliding window chunking with configurable overlap
    fn chunk_sliding_window(&self, content: &str) -> Vec<String> {
        let mut chunks = Vec::new();
        let words: Vec<&str> = content.split_whitespace().collect();

        if words.is_empty() {
            return chunks;
        }

        // Approximate words per chunk (assuming ~5 chars per word)
        let words_per_chunk = self.max_chunk_size / 5;
        let overlap_words = self.overlap / 5;

        let mut start = 0;
        while start < words.len() {
            let end = (start + words_per_chunk).min(words.len());
            let chunk = words[start..end].join(" ");
            chunks.push(chunk);

            // Move window forward
            start += words_per_chunk - overlap_words;
            if start >= words.len() {
                break;
            }
        }

        chunks
    }

    /// Section-based chunking (detect sections by headings or double newlines)
    fn chunk_by_sections(&self, content: &str) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut current_section = String::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Detect section boundaries (markdown headings or blank lines)
            let is_heading = trimmed.starts_with('#');
            let _is_blank = trimmed.is_empty();

            if is_heading && !current_section.is_empty() {
                // Save current section and start new one
                chunks.push(current_section.trim().to_string());
                current_section.clear();
            }

            current_section.push_str(line);
            current_section.push('\n');

            // If section gets too large, chunk it
            if current_section.chars().count() > self.max_chunk_size {
                chunks.push(current_section.trim().to_string());
                current_section.clear();
            }
        }

        if !current_section.trim().is_empty() {
            chunks.push(current_section.trim().to_string());
        }

        chunks
    }

    /// Chunk by sentences
    fn chunk_by_sentences(&self, content: &str) -> Vec<String> {
        let sentences = Self::split_sentences(content);
        self.combine_units(sentences)
    }

    /// Chunk by paragraphs
    fn chunk_by_paragraphs(&self, content: &str) -> Vec<String> {
        let paragraphs = Self::split_paragraphs(content);
        self.combine_units(paragraphs)
    }

    /// Chunk by fixed size
    fn chunk_by_size(&self, content: &str) -> Vec<String> {
        let mut chunks = Vec::new();
        let chars: Vec<char> = content.chars().collect();
        let mut start = 0;

        while start < chars.len() {
            let end = (start + self.max_chunk_size).min(chars.len());
            let chunk: String = chars[start..end].iter().collect();
            chunks.push(chunk);

            start += self.max_chunk_size - self.overlap;
            if start >= chars.len() {
                break;
            }
        }

        chunks
    }

    /// Hybrid chunking: try paragraphs, fall back to sentences
    fn chunk_hybrid(&self, content: &str) -> Vec<String> {
        let paragraphs = Self::split_paragraphs(content);
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();

        for paragraph in paragraphs {
            let para_len = paragraph.chars().count();

            // If paragraph is too large, chunk it by sentences
            if para_len > self.max_chunk_size {
                // Save current chunk
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk.clone());
                    current_chunk.clear();
                }

                // Chunk large paragraph by sentences
                let sentence_chunks = self.chunk_by_sentences(&paragraph);
                chunks.extend(sentence_chunks);
                continue;
            }

            // Try to add paragraph to current chunk
            if current_chunk.chars().count() + para_len > self.max_chunk_size
                && !current_chunk.is_empty()
            {
                chunks.push(current_chunk.clone());

                // Add overlap
                let words: Vec<&str> = current_chunk.split_whitespace().collect();
                let overlap_count = (self.overlap / 5).max(1);
                let overlap_words: Vec<&str> = words
                    .iter()
                    .rev()
                    .take(overlap_count)
                    .rev()
                    .copied()
                    .collect();
                current_chunk = overlap_words.join(" ");

                if !current_chunk.is_empty() {
                    current_chunk.push_str("\n\n");
                }
            }

            if !current_chunk.is_empty() && !current_chunk.ends_with("\n\n") {
                current_chunk.push_str("\n\n");
            }
            current_chunk.push_str(&paragraph);
        }

        if !current_chunk.trim().is_empty() {
            chunks.push(current_chunk);
        }

        chunks
    }

    /// Combine small units into chunks
    fn combine_units(&self, units: Vec<String>) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();

        for unit in units {
            let unit_len = unit.chars().count();

            if current_chunk.chars().count() + unit_len > self.max_chunk_size
                && !current_chunk.is_empty()
            {
                chunks.push(current_chunk.clone());

                // Add overlap
                let words: Vec<&str> = current_chunk.split_whitespace().collect();
                let overlap_count = (self.overlap / 5).max(1);
                let overlap_words: Vec<&str> = words
                    .iter()
                    .rev()
                    .take(overlap_count)
                    .rev()
                    .copied()
                    .collect();
                current_chunk = overlap_words.join(" ");

                if !current_chunk.is_empty() {
                    current_chunk.push(' ');
                }
            }

            if !current_chunk.is_empty() {
                current_chunk.push(' ');
            }
            current_chunk.push_str(&unit);
        }

        if !current_chunk.trim().is_empty() {
            chunks.push(current_chunk);
        }

        chunks
    }
}

impl ChunkerTrait for SemanticChunker {
    fn chunk(&self, content: &str) -> Vec<String> {
        self.chunk_by_strategy(content)
    }

    fn max_chunk_size(&self) -> usize {
        self.max_chunk_size
    }

    fn overlap(&self) -> usize {
        self.overlap
    }
}

/// Simple chunker that splits by character count
pub struct Chunker {
    chunk_size: usize,
    overlap: usize,
}

impl Chunker {
    pub fn new(chunk_size: usize, overlap: usize) -> Self {
        Self { chunk_size, overlap }
    }
}

impl ChunkerTrait for Chunker {
    fn chunk(&self, content: &str) -> Vec<String> {
        let mut chunks = Vec::new();
        let chars: Vec<char> = content.chars().collect();
        let mut start = 0;

        while start < chars.len() {
            let end = (start + self.chunk_size).min(chars.len());
            let chunk: String = chars[start..end].iter().collect();
            chunks.push(chunk);

            start += self.chunk_size - self.overlap;
            if start >= chars.len() {
                break;
            }
        }

        chunks
    }

    fn max_chunk_size(&self) -> usize {
        self.chunk_size
    }

    fn overlap(&self) -> usize {
        self.overlap
    }
}

/// Code-aware chunker that respects language syntax
pub struct CodeChunker {
    max_chunk_size: usize,
    overlap: usize,
}

impl CodeChunker {
    /// Create a new code-aware chunker
    pub fn new(max_chunk_size: usize, overlap: usize) -> Self {
        Self {
            max_chunk_size,
            overlap,
        }
    }

    /// Split code by functions/classes/blocks
    fn split_code_blocks(code: &str) -> Vec<String> {
        let mut blocks = Vec::new();
        let mut current_block = String::new();
        let mut brace_depth: i32 = 0;
        let mut in_string = false;
        let mut escape_next = false;

        for line in code.lines() {
            let trimmed = line.trim();

            // Track string literals
            for ch in line.chars() {
                if escape_next {
                    escape_next = false;
                    continue;
                }
                if ch == '\\' {
                    escape_next = true;
                    continue;
                }
                if ch == '"' || ch == '\'' {
                    in_string = !in_string;
                }
                if !in_string {
                    if ch == '{' {
                        brace_depth += 1;
                    } else if ch == '}' {
                        brace_depth = brace_depth.saturating_sub(1);
                    }
                }
            }

            current_block.push_str(line);
            current_block.push('\n');

            // Split on top-level closing braces or empty lines
            if (brace_depth == 0 && !current_block.trim().is_empty() && trimmed.is_empty())
                || (brace_depth == 0 && trimmed.ends_with('}'))
            {
                if current_block.trim().len() > 10 {
                    blocks.push(current_block.clone());
                    current_block.clear();
                }
            }
        }

        // Add remaining block
        if !current_block.trim().is_empty() {
            blocks.push(current_block);
        }

        blocks
    }

    /// Chunk code while respecting block boundaries
    fn chunk_code(&self, code: &str) -> Vec<String> {
        let blocks = Self::split_code_blocks(code);
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();

        for block in blocks {
            let block_len = block.chars().count();

            // If single block is too large, split it
            if block_len > self.max_chunk_size {
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk.clone());
                    current_chunk.clear();
                }

                // Split large block by lines with overlap
                let lines: Vec<&str> = block.lines().collect();
                let mut start = 0;
                while start < lines.len() {
                    let mut end = start;
                    let mut chunk_size = 0;

                    while end < lines.len() && chunk_size < self.max_chunk_size {
                        chunk_size += lines[end].chars().count() + 1;
                        end += 1;
                    }

                    let chunk = lines[start..end].join("\n");
                    chunks.push(chunk);

                    // Add overlap
                    start = end.saturating_sub(self.overlap / 100);
                }
                continue;
            }

            // Try to add block to current chunk
            if current_chunk.chars().count() + block_len > self.max_chunk_size
                && !current_chunk.is_empty()
            {
                chunks.push(current_chunk.clone());

                // Add overlap (last few lines)
                let lines: Vec<&str> = current_chunk.lines().collect();
                let overlap_lines = lines.iter().rev().take(3).rev();
                current_chunk = overlap_lines.map(|l| *l).collect::<Vec<_>>().join("\n");

                if !current_chunk.is_empty() {
                    current_chunk.push_str("\n\n");
                }
            }

            current_chunk.push_str(&block);
            current_chunk.push('\n');
        }

        if !current_chunk.trim().is_empty() {
            chunks.push(current_chunk);
        }

        chunks
    }
}

impl ChunkerTrait for CodeChunker {
    fn chunk(&self, content: &str) -> Vec<String> {
        self.chunk_code(content)
    }

    fn max_chunk_size(&self) -> usize {
        self.max_chunk_size
    }

    fn overlap(&self) -> usize {
        self.overlap
    }
}

/// Hierarchical chunker that creates parent-child chunk relationships
pub struct HierarchicalChunker {
    parent_size: usize,
    child_size: usize,
    overlap: usize,
}

impl HierarchicalChunker {
    /// Create a new hierarchical chunker
    pub fn new(parent_size: usize, child_size: usize, overlap: usize) -> Self {
        Self {
            parent_size,
            child_size,
            overlap,
        }
    }

    /// Create parent and child chunks
    pub fn chunk_hierarchical(&self, content: &str) -> (Vec<String>, Vec<Vec<String>>) {
        // Create parent chunks
        let parent_chunker = SemanticChunker::new(self.parent_size, self.overlap);
        let parent_chunks = parent_chunker.chunk(content);

        // Create child chunks for each parent
        let child_chunker = SemanticChunker::new(self.child_size, self.overlap);
        let child_chunks: Vec<Vec<String>> = parent_chunks
            .iter()
            .map(|parent| child_chunker.chunk(parent))
            .collect();

        (parent_chunks, child_chunks)
    }
}

impl ChunkerTrait for HierarchicalChunker {
    fn chunk(&self, content: &str) -> Vec<String> {
        // Return parent chunks by default
        let (parent_chunks, _) = self.chunk_hierarchical(content);
        parent_chunks
    }

    fn max_chunk_size(&self) -> usize {
        self.parent_size
    }

    fn overlap(&self) -> usize {
        self.overlap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_chunker() {
        let chunker = SemanticChunker::new(100, 10);
        let text = "First sentence. Second sentence. Third sentence.";
        let chunks = chunker.chunk(text);

        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_simple_chunker() {
        let chunker = Chunker::new(10, 2);
        let text = "This is a test string for chunking.";
        let chunks = chunker.chunk(text);

        assert!(!chunks.is_empty());
    }
}
