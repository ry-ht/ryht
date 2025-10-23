//! Markdown document processor with frontmatter and structure parsing.

use super::{ChunkType, ContentChunk, ContentProcessor, ContentType, ProcessedContent};
use async_trait::async_trait;
use cortex_core::error::Result;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use regex::Regex;
use std::collections::HashMap;

/// Processor for Markdown documents
pub struct MarkdownProcessor {
    extract_frontmatter: bool,
    chunk_by_section: bool,
}

impl MarkdownProcessor {
    /// Create a new Markdown processor
    pub fn new() -> Self {
        Self {
            extract_frontmatter: true,
            chunk_by_section: true,
        }
    }

    /// Extract YAML frontmatter from markdown content
    fn extract_frontmatter(content: &str) -> Option<(HashMap<String, serde_json::Value>, String)> {
        let frontmatter_regex = Regex::new(r"(?s)^---\s*\n(.*?)\n---\s*\n(.*)$").ok()?;

        if let Some(captures) = frontmatter_regex.captures(content) {
            let yaml_str = captures.get(1)?.as_str();
            let content_without_frontmatter = captures.get(2)?.as_str();

            // Parse YAML frontmatter
            if let Ok(frontmatter) = serde_yaml::from_str::<serde_yaml::Value>(yaml_str) {
                // Convert YAML to JSON for uniform handling
                if let Ok(json_value) = serde_json::to_value(&frontmatter) {
                    if let Some(obj) = json_value.as_object() {
                        // Convert serde_json::Map to HashMap
                        let mut map = HashMap::new();
                        for (k, v) in obj {
                            map.insert(k.clone(), v.clone());
                        }
                        return Some((
                            map,
                            content_without_frontmatter.to_string(),
                        ));
                    }
                }
            }
        }

        None
    }

    /// Parse markdown structure and extract sections
    fn parse_structure(content: &str) -> Vec<Section> {
        let parser = Parser::new(content);
        let mut sections = Vec::new();
        let mut current_section = Section::new(0, "Root".to_string());
        let mut current_text = String::new();
        let mut in_code_block = false;
        let mut code_block_lang = None;
        let mut code_block_content = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    // Save current section if it has content
                    if !current_text.is_empty() {
                        current_section.content = current_text.trim().to_string();
                        sections.push(current_section.clone());
                        current_text.clear();
                    }
                    current_section.level = level as usize;
                }
                Event::End(TagEnd::Heading(_)) => {
                    current_section.heading = current_text.trim().to_string();
                    current_text.clear();
                }
                Event::Start(Tag::CodeBlock(kind)) => {
                    in_code_block = true;
                    code_block_lang = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(lang) => Some(lang.to_string()),
                        _ => None,
                    };
                    code_block_content.clear();
                }
                Event::End(TagEnd::CodeBlock) => {
                    in_code_block = false;
                    current_section.code_blocks.push(CodeBlock {
                        language: code_block_lang.clone(),
                        content: code_block_content.clone(),
                    });
                    code_block_content.clear();
                }
                Event::Text(text) => {
                    if in_code_block {
                        code_block_content.push_str(&text);
                    } else {
                        current_text.push_str(&text);
                    }
                }
                Event::Code(text) => {
                    current_text.push('`');
                    current_text.push_str(&text);
                    current_text.push('`');
                }
                Event::SoftBreak | Event::HardBreak => {
                    if !in_code_block {
                        current_text.push('\n');
                    }
                }
                _ => {}
            }
        }

        // Save final section
        if !current_text.is_empty() || !current_section.content.is_empty() {
            current_section.content.push_str(&current_text);
            sections.push(current_section);
        }

        sections
    }
}

impl Default for MarkdownProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
struct Section {
    level: usize,
    heading: String,
    content: String,
    code_blocks: Vec<CodeBlock>,
}

impl Section {
    fn new(level: usize, heading: String) -> Self {
        Self {
            level,
            heading,
            content: String::new(),
            code_blocks: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct CodeBlock {
    language: Option<String>,
    content: String,
}

#[async_trait]
impl ContentProcessor for MarkdownProcessor {
    async fn process(&self, input: &[u8]) -> Result<ProcessedContent> {
        tracing::debug!("Processing Markdown document ({} bytes)", input.len());

        // Decode text content
        let content = String::from_utf8_lossy(input).to_string();

        let mut metadata = HashMap::new();
        let mut text_content = content.clone();

        // Extract frontmatter if enabled
        if self.extract_frontmatter {
            if let Some((frontmatter, content_without_fm)) = Self::extract_frontmatter(&content) {
                metadata = frontmatter;
                text_content = content_without_fm;
            }
        }

        let mut chunks = Vec::new();

        if self.chunk_by_section {
            // Parse structure and create section chunks
            let sections = Self::parse_structure(&text_content);

            for section in sections {
                if !section.content.trim().is_empty() || !section.heading.is_empty() {
                    let mut chunk_metadata = HashMap::new();
                    chunk_metadata.insert(
                        "heading_level".to_string(),
                        serde_json::Value::Number(section.level.into()),
                    );
                    chunk_metadata.insert(
                        "heading".to_string(),
                        serde_json::Value::String(section.heading.clone()),
                    );

                    let chunk_content = if section.heading.is_empty() {
                        section.content.clone()
                    } else {
                        format!("# {}\n\n{}", section.heading, section.content)
                    };

                    let mut chunk = ContentChunk::new(chunk_content, ChunkType::Section);
                    chunk.metadata = chunk_metadata;
                    chunks.push(chunk);
                }

                // Add code blocks as separate chunks
                for code_block in section.code_blocks {
                    let mut chunk_metadata = HashMap::new();
                    if let Some(lang) = &code_block.language {
                        chunk_metadata.insert(
                            "language".to_string(),
                            serde_json::Value::String(lang.clone()),
                        );
                    }
                    chunk_metadata.insert(
                        "section".to_string(),
                        serde_json::Value::String(section.heading.clone()),
                    );

                    let mut chunk = ContentChunk::new(code_block.content, ChunkType::CodeBlock);
                    chunk.metadata = chunk_metadata;
                    chunks.push(chunk);
                }
            }
        } else {
            // Single chunk for entire document
            chunks.push(ContentChunk::new(
                text_content.clone(),
                ChunkType::Document,
            ));
        }

        // Add format metadata
        metadata.insert(
            "format".to_string(),
            serde_json::Value::String("markdown".to_string()),
        );

        // Create ProcessedContent with metadata
        let mut processed = ProcessedContent::new(ContentType::Markdown, text_content);
        processed.metadata = metadata;
        processed.chunks = chunks;

        Ok(processed)
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["md", "markdown", "mdown", "mkdn", "mkd"]
    }

    fn supported_mime_types(&self) -> Vec<&str> {
        vec!["text/markdown", "text/x-markdown"]
    }

    fn content_type(&self) -> ContentType {
        ContentType::Markdown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_markdown_processor_simple() {
        let processor = MarkdownProcessor::new();
        let content = b"# Hello\n\nThis is a test.";

        let result = processor.process(content).await.unwrap();
        assert_eq!(result.content_type, ContentType::Markdown);
        assert!(result.text_content.contains("Hello"));
        assert!(!result.chunks.is_empty());
    }

    #[tokio::test]
    async fn test_markdown_frontmatter() {
        let processor = MarkdownProcessor::new();
        let content = b"---\ntitle: Test\nauthor: John\n---\n# Content\n\nBody text.";

        let result = processor.process(content).await.unwrap();
        assert!(result.metadata.contains_key("title"));
        assert!(result.text_content.contains("Content"));
    }

    #[tokio::test]
    async fn test_markdown_code_blocks() {
        let processor = MarkdownProcessor::new();
        let content = b"# Test\n\n```rust\nfn main() {}\n```\n\nMore text.";

        let result = processor.process(content).await.unwrap();
        let code_chunks: Vec<_> = result
            .chunks
            .iter()
            .filter(|c| matches!(c.chunk_type, ChunkType::CodeBlock))
            .collect();
        assert!(!code_chunks.is_empty());
    }

    #[test]
    fn test_markdown_extensions() {
        let processor = MarkdownProcessor::new();
        assert!(processor.supported_extensions().contains(&"md"));
        assert_eq!(processor.content_type(), ContentType::Markdown);
    }
}
