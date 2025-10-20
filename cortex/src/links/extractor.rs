use super::types::{
    ExtractionMethod, KnowledgeLevel, LinkTarget, LinkType, SemanticLink,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use tree_sitter::{Language, Node, Parser};

/// Trait for extracting semantic links from different sources
#[async_trait]
pub trait LinkExtractor: Send + Sync {
    /// Extract links from content
    async fn extract(
        &self,
        file_path: &Path,
        content: &str,
        level: KnowledgeLevel,
    ) -> Result<Vec<SemanticLink>>;
}

/// Extract links from code comments and annotations
pub struct CommentExtractor {
    annotation_regex: Regex,
}

impl CommentExtractor {
    pub fn new() -> Result<Self> {
        // Matches patterns like: @meridian:implemented_by code:Application
        // Note: Use [^\r\n]+ instead of .+ to avoid matching across lines
        // [\w-]+ allows both underscores and hyphens in link types
        // Context group stops before the next @meridian annotation or end of line
        let annotation_regex = Regex::new(
            r"@meridian:([\w-]+)\s+(spec|code|docs|examples|tests):([^\s\)]+)(?:\s+([^@\r\n]+))?",
        )?;

        Ok(Self { annotation_regex })
    }

    /// Extract annotations from a comment block
    fn extract_annotations(&self, comment: &str) -> Vec<(String, String, String, Option<String>)> {
        let mut annotations = Vec::new();

        for cap in self.annotation_regex.captures_iter(comment) {
            let link_type_str = cap.get(1).map(|m| m.as_str().to_string()).unwrap();
            let target_level = cap.get(2).map(|m| m.as_str().to_string()).unwrap();
            let target_id = cap.get(3).map(|m| m.as_str().to_string()).unwrap();
            let context = cap.get(4).map(|m| m.as_str().trim().to_string()).filter(|s| !s.is_empty());

            annotations.push((link_type_str, target_level, target_id, context));
        }

        annotations
    }

    /// Extract markdown frontmatter annotations
    fn extract_frontmatter_annotations(
        &self,
        content: &str,
    ) -> Vec<(String, String, String, Option<String>)> {
        let mut annotations = Vec::new();

        // Simple YAML frontmatter parser
        if content.starts_with("---") {
            if let Some(end_idx) = content[3..].find("---") {
                let frontmatter = &content[3..end_idx + 3];

                // Look for meridian section
                for line in frontmatter.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("meridian:") {
                        continue;
                    }

                    // Parse link entries
                    if let Some(colon_idx) = trimmed.find(':') {
                        let key = trimmed[..colon_idx].trim();
                        let value = trimmed[colon_idx + 1..].trim();

                        if let Some(link_type) = LinkType::from_str(key) {
                            // Parse target
                            if let Some((level_str, id)) = value.split_once(':') {
                                if let Some(level) = KnowledgeLevel::from_str(level_str) {
                                    annotations.push((
                                        link_type.as_str().to_string(),
                                        level.as_str().to_string(),
                                        id.to_string(),
                                        None,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        annotations
    }
}

impl Default for CommentExtractor {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[async_trait]
impl LinkExtractor for CommentExtractor {
    async fn extract(
        &self,
        file_path: &Path,
        content: &str,
        level: KnowledgeLevel,
    ) -> Result<Vec<SemanticLink>> {
        let mut links = Vec::new();

        // Determine source entity based on file and level
        let source_id = match level {
            KnowledgeLevel::Spec | KnowledgeLevel::Docs => {
                // For spec/docs, use the file path
                file_path.to_string_lossy().to_string()
            }
            KnowledgeLevel::Code | KnowledgeLevel::Tests | KnowledgeLevel::Examples => {
                // For code, extract the primary symbol name from the file
                file_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string()
            }
        };

        let source = LinkTarget::new(level, source_id);

        // Extract from markdown frontmatter if markdown file
        if file_path.extension().and_then(|s| s.to_str()) == Some("md") {
            let frontmatter_annotations = self.extract_frontmatter_annotations(content);
            for (link_type_str, target_level_str, target_id, context) in frontmatter_annotations {
                if let Some(link_type) = LinkType::from_str(&link_type_str) {
                    if let Some(target_level) = KnowledgeLevel::from_str(&target_level_str) {
                        let target = LinkTarget::new(target_level, target_id);

                        let mut link = SemanticLink::new(
                            link_type,
                            source.clone(),
                            target,
                            1.0, // Frontmatter annotations have high confidence
                            ExtractionMethod::Annotation,
                            "frontmatter".to_string(),
                        );

                        if let Some(ctx) = context {
                            link = link.with_context(ctx);
                        }

                        links.push(link);
                    }
                }
            }
        }

        // Extract from comments
        let comment_blocks = self.extract_comment_blocks(content);
        for comment in comment_blocks {
            let annotations = self.extract_annotations(&comment);

            for (link_type_str, target_level_str, target_id, context) in annotations {
                if let Some(link_type) = LinkType::from_str(&link_type_str) {
                    if let Some(target_level) = KnowledgeLevel::from_str(&target_level_str) {
                        let target = LinkTarget::new(target_level, target_id);

                        let mut link = SemanticLink::new(
                            link_type,
                            source.clone(),
                            target,
                            0.95, // Comment annotations have high confidence
                            ExtractionMethod::Annotation,
                            "comment".to_string(),
                        );

                        if let Some(ctx) = context {
                            link = link.with_context(ctx);
                        }

                        links.push(link);
                    }
                }
            }
        }

        Ok(links)
    }
}

impl CommentExtractor {
    /// Extract comment blocks from source code
    fn extract_comment_blocks(&self, content: &str) -> Vec<String> {
        let mut blocks = Vec::new();
        let mut current_block = String::new();
        let mut in_block_comment = false;

        for line in content.lines() {
            let trimmed = line.trim();

            // Handle block comments (/* */ or /** */)
            if trimmed.starts_with("/*") {
                in_block_comment = true;
                current_block.push_str(line);
                current_block.push('\n');

                if trimmed.ends_with("*/") {
                    in_block_comment = false;
                    blocks.push(current_block.clone());
                    current_block.clear();
                }
                continue;
            }

            if in_block_comment {
                current_block.push_str(line);
                current_block.push('\n');

                if trimmed.ends_with("*/") {
                    in_block_comment = false;
                    blocks.push(current_block.clone());
                    current_block.clear();
                }
                continue;
            }

            // Handle line comments (// or /// or #)
            if trimmed.starts_with("///") || trimmed.starts_with("//") || trimmed.starts_with("#")
            {
                current_block.push_str(line);
                current_block.push('\n');
            } else if !current_block.is_empty() {
                blocks.push(current_block.clone());
                current_block.clear();
            }
        }

        if !current_block.is_empty() {
            blocks.push(current_block);
        }

        blocks
    }
}

/// Extract links from code structure using tree-sitter
pub struct TreeSitterExtractor {
    languages: HashMap<&'static str, Language>,
}

impl TreeSitterExtractor {
    pub fn new() -> Result<Self> {
        let mut languages = HashMap::new();

        // Add language support
        languages.insert("rust", tree_sitter_rust::LANGUAGE.into());
        languages.insert("typescript", tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into());
        languages.insert("tsx", tree_sitter_typescript::LANGUAGE_TSX.into());
        languages.insert("javascript", tree_sitter_javascript::LANGUAGE.into());
        languages.insert("python", tree_sitter_python::LANGUAGE.into());
        languages.insert("go", tree_sitter_go::LANGUAGE.into());

        Ok(Self { languages })
    }

    /// Detect language from file extension
    fn detect_language(&self, path: &Path) -> Option<&'static str> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| match ext {
                "rs" => Some("rust"),
                "ts" => Some("typescript"),
                "tsx" => Some("tsx"),
                "js" | "jsx" => Some("javascript"),
                "py" => Some("python"),
                "go" => Some("go"),
                _ => None,
            })
    }

    /// Extract import statements
    fn extract_imports(&self, tree: &tree_sitter::Tree, content: &str) -> Vec<String> {
        let mut imports = Vec::new();
        let root = tree.root_node();

        fn visit_node(node: Node, content: &str, imports: &mut Vec<String>) {
            match node.kind() {
                "use_declaration" | "import_statement" | "import_declaration" => {
                    if let Ok(text) = node.utf8_text(content.as_bytes()) {
                        imports.push(text.to_string());
                    }
                }
                _ => {
                    let mut child_cursor = node.walk();
                    for child in node.children(&mut child_cursor) {
                        visit_node(child, content, imports);
                    }
                }
            }
        }

        visit_node(root, content, &mut imports);
        imports
    }

    /// Parse import to extract referenced module/symbol
    fn parse_import(&self, import: &str) -> Option<(String, Vec<String>)> {
        // TypeScript/JavaScript: import { X, Y } from 'module'
        if import.contains("from") {
            let parts: Vec<&str> = import.split("from").collect();
            if parts.len() == 2 {
                let module = parts[1].trim().trim_matches(|c| c == '\'' || c == '"' || c == ';');

                // Extract symbols
                let symbols_part = parts[0].trim();
                let symbols = if symbols_part.contains('{') {
                    // Named imports
                    let start = symbols_part.find('{').unwrap();
                    let end = symbols_part.find('}').unwrap();
                    symbols_part[start + 1..end]
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect()
                } else {
                    // Default import
                    vec![symbols_part
                        .replace("import", "")
                        .trim()
                        .to_string()]
                };

                return Some((module.to_string(), symbols));
            }
        }

        // Rust: use module::Symbol
        if import.starts_with("use") {
            let path = import
                .trim_start_matches("use")
                .trim()
                .trim_end_matches(';');
            let parts: Vec<&str> = path.split("::").collect();
            if !parts.is_empty() {
                let module = parts[..parts.len() - 1].join("::");
                let symbol = parts.last().unwrap().to_string();
                return Some((module, vec![symbol]));
            }
        }

        None
    }
}

impl Default for TreeSitterExtractor {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[async_trait]
impl LinkExtractor for TreeSitterExtractor {
    async fn extract(
        &self,
        file_path: &Path,
        content: &str,
        level: KnowledgeLevel,
    ) -> Result<Vec<SemanticLink>> {
        let mut links = Vec::new();

        // Only extract from code files
        if !matches!(
            level,
            KnowledgeLevel::Code | KnowledgeLevel::Tests | KnowledgeLevel::Examples
        ) {
            return Ok(links);
        }

        let lang = self
            .detect_language(file_path)
            .ok_or_else(|| anyhow!("Unsupported language"))?;

        let language = self
            .languages
            .get(lang)
            .ok_or_else(|| anyhow!("Language not found"))?;

        let mut parser = Parser::new();
        parser
            .set_language(language)
            .map_err(|e| anyhow!("Failed to set language: {}", e))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| anyhow!("Failed to parse file"))?;

        // Extract imports
        let imports = self.extract_imports(&tree, content);

        let source_id = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let source = LinkTarget::new(level, source_id);

        for import in imports {
            if let Some((module, symbols)) = self.parse_import(&import) {
                // Infer links based on import patterns
                for symbol in symbols {
                    // If importing from a relative path, it's likely code
                    let target_level = if module.starts_with('.') || module.starts_with("./") {
                        KnowledgeLevel::Code
                    } else {
                        KnowledgeLevel::Code // External dependencies
                    };

                    let target = LinkTarget::new(target_level, symbol);

                    let link = SemanticLink::new(
                        LinkType::DependsOn,
                        source.clone(),
                        target,
                        0.7, // Inferred links have medium confidence
                        ExtractionMethod::Inference,
                        "tree-sitter".to_string(),
                    )
                    .with_context(format!("Import: {}", import));

                    links.push(link);
                }
            }
        }

        Ok(links)
    }
}

/// Extract links from markdown files
pub struct MarkdownExtractor {
    comment_extractor: CommentExtractor,
    link_regex: Regex,
}

impl MarkdownExtractor {
    pub fn new() -> Result<Self> {
        let link_regex = Regex::new(r"\[([^\]]+)\]\(([^\)]+)\)")?;

        Ok(Self {
            comment_extractor: CommentExtractor::new()?,
            link_regex,
        })
    }

    /// Extract markdown links
    fn extract_markdown_links(&self, content: &str) -> Vec<(String, String)> {
        let mut links = Vec::new();

        for cap in self.link_regex.captures_iter(content) {
            let text = cap.get(1).map(|m| m.as_str().to_string()).unwrap();
            let url = cap.get(2).map(|m| m.as_str().to_string()).unwrap();
            links.push((text, url));
        }

        links
    }
}

impl Default for MarkdownExtractor {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[async_trait]
impl LinkExtractor for MarkdownExtractor {
    async fn extract(
        &self,
        file_path: &Path,
        content: &str,
        level: KnowledgeLevel,
    ) -> Result<Vec<SemanticLink>> {
        let mut links = Vec::new();

        // Use comment extractor for annotations
        let annotation_links = self.comment_extractor.extract(file_path, content, level).await?;
        links.extend(annotation_links);

        // Extract markdown links
        let md_links = self.extract_markdown_links(content);

        let source_id = file_path.to_string_lossy().to_string();
        let source = LinkTarget::new(level, source_id);

        for (text, url) in md_links {
            // Infer link type from URL patterns
            let (target_level, target_id) = if url.ends_with(".md") {
                (KnowledgeLevel::Docs, url.clone())
            } else if url.contains("/examples/") {
                (KnowledgeLevel::Examples, url.clone())
            } else if url.contains("/tests/") {
                (KnowledgeLevel::Tests, url.clone())
            } else if url.contains("/src/") || url.contains("/lib/") {
                (KnowledgeLevel::Code, url.clone())
            } else {
                continue; // Skip external links
            };

            let target = LinkTarget::new(target_level, target_id);

            let link = SemanticLink::new(
                LinkType::RelatesTo,
                source.clone(),
                target,
                0.6, // Markdown links have lower confidence
                ExtractionMethod::Inference,
                "markdown".to_string(),
            )
            .with_context(format!("Link text: {}", text));

            links.push(link);
        }

        Ok(links)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_comment_extractor_annotation() {
        let extractor = CommentExtractor::new().unwrap();

        let content = r#"
/**
 * Application class
 * @meridian:realizes spec:spec.md#application-lifecycle
 * @meridian:documented_in docs:api.md#application
 */
export class Application {
}
"#;

        let path = PathBuf::from("application.ts");
        let links = extractor
            .extract(&path, content, KnowledgeLevel::Code)
            .await
            .unwrap();

        assert_eq!(links.len(), 2);
        assert!(links.iter().any(|l| l.link_type == LinkType::Realizes));
        assert!(links
            .iter()
            .any(|l| l.link_type == LinkType::DocumentedIn));
    }

    #[tokio::test]
    async fn test_comment_extractor_frontmatter() {
        let extractor = CommentExtractor::new().unwrap();

        let content = r#"---
meridian:
  documents: code:Application
  shows_example: examples:app-basic
---

# Application API
"#;

        let path = PathBuf::from("api.md");
        let links = extractor
            .extract(&path, content, KnowledgeLevel::Docs)
            .await
            .unwrap();

        assert!(links.len() >= 1);
    }

    #[tokio::test]
    async fn test_tree_sitter_extractor() {
        let extractor = TreeSitterExtractor::new().unwrap();

        let content = r#"
use std::collections::HashMap;
use crate::types::Symbol;

pub struct MyStruct {}
"#;

        let path = PathBuf::from("test.rs");
        let links = extractor
            .extract(&path, content, KnowledgeLevel::Code)
            .await
            .unwrap();

        // Should extract dependency links from imports
        assert!(!links.is_empty());
        assert!(links.iter().all(|l| l.link_type == LinkType::DependsOn));
    }

    #[tokio::test]
    async fn test_markdown_extractor() {
        let extractor = MarkdownExtractor::new().unwrap();

        let content = r#"
# Documentation

See [API Reference](api.md) for details.
Check [Examples](../examples/basic.md) for usage.
"#;

        let path = PathBuf::from("index.md");
        let links = extractor
            .extract(&path, content, KnowledgeLevel::Docs)
            .await
            .unwrap();

        assert!(!links.is_empty());
    }
}
