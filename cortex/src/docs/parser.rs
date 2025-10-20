use super::{CodeBlock, DocEntry, DocType};
use anyhow::Result;
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::path::Path;

/// Supported documentation formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocFormat {
    Markdown,
    RustDoc,
    JSDoc,
    PyDoc,
    GoDoc,
}

/// Section in a markdown document
#[derive(Debug, Clone)]
pub struct DocSection {
    pub level: usize,
    pub title: String,
    pub content: String,
    pub line_start: usize,
    pub line_end: usize,
    pub path: Vec<String>,
}

/// Parsed markdown document
#[derive(Debug, Clone)]
pub struct MarkdownDoc {
    pub sections: Vec<DocSection>,
    pub code_blocks: Vec<CodeBlock>,
    pub links: Vec<String>,
}

/// Parsed documentation
#[derive(Debug, Clone)]
pub struct ParsedDoc {
    pub entries: Vec<DocEntry>,
}

/// Parse markdown content into structured documentation
pub fn parse_markdown(content: &str, _file_path: &str) -> Result<MarkdownDoc> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);

    let parser = Parser::new_ext(content, options);

    let mut sections = Vec::new();
    let mut code_blocks = Vec::new();
    let mut links = Vec::new();

    let mut current_heading = String::new();
    let mut current_level = 0;
    let mut current_content = String::new();
    let mut heading_path: Vec<String> = Vec::new();
    let mut line_number = 0;
    let mut section_start = 0;

    let mut in_code_block = false;
    let mut code_language = None;
    let mut code_content = String::new();
    let mut code_line = 0;

    for (event, range) in parser.into_offset_iter() {
        // Calculate approximate line number from byte offset
        line_number = content[..range.start].matches('\n').count() + 1;

        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                // Save previous section if exists
                if !current_heading.is_empty() {
                    sections.push(DocSection {
                        level: current_level,
                        title: current_heading.clone(),
                        content: current_content.trim().to_string(),
                        line_start: section_start,
                        line_end: line_number.saturating_sub(1),
                        path: heading_path.clone(),
                    });
                }

                current_level = level as usize;
                current_heading.clear();
                current_content.clear();
                section_start = line_number;

                // Update heading path
                while heading_path.len() >= current_level {
                    heading_path.pop();
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if !current_heading.is_empty() {
                    heading_path.push(current_heading.clone());
                }
            }
            Event::Text(text) => {
                if current_level > 0 && current_heading.is_empty() {
                    current_heading.push_str(&text);
                } else if in_code_block {
                    code_content.push_str(&text);
                } else {
                    current_content.push_str(&text);
                }
            }
            Event::Code(code) => {
                current_content.push('`');
                current_content.push_str(&code);
                current_content.push('`');
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                code_line = line_number;
                code_language = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                        if lang.is_empty() {
                            None
                        } else {
                            Some(lang.to_string())
                        }
                    }
                    pulldown_cmark::CodeBlockKind::Indented => None,
                };
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                code_blocks.push(CodeBlock {
                    language: code_language.clone(),
                    content: code_content.clone(),
                    line_number: code_line,
                });
                code_content.clear();
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                links.push(dest_url.to_string());
            }
            Event::SoftBreak | Event::HardBreak => {
                current_content.push('\n');
            }
            Event::Start(Tag::Paragraph) => {
                if !current_content.is_empty() {
                    current_content.push_str("\n\n");
                }
            }
            _ => {}
        }
    }

    // Add final section
    if !current_heading.is_empty() {
        sections.push(DocSection {
            level: current_level,
            title: current_heading,
            content: current_content.trim().to_string(),
            line_start: section_start,
            line_end: line_number,
            path: heading_path,
        });
    }

    Ok(MarkdownDoc {
        sections,
        code_blocks,
        links,
    })
}

/// Parse documentation from source code comments
pub fn parse_doc_comments(content: &str, file_path: &str, format: DocFormat) -> Result<Vec<DocEntry>> {
    let mut entries = Vec::new();

    match format {
        DocFormat::RustDoc => {
            parse_rust_doc_comments(content, file_path, &mut entries)?;
        }
        DocFormat::JSDoc => {
            parse_jsdoc_comments(content, file_path, &mut entries)?;
        }
        DocFormat::PyDoc => {
            parse_python_docstrings(content, file_path, &mut entries)?;
        }
        DocFormat::GoDoc => {
            parse_go_doc_comments(content, file_path, &mut entries)?;
        }
        DocFormat::Markdown => {
            // Already handled by parse_markdown
        }
    }

    Ok(entries)
}

/// Parse Rust doc comments (/// and //!)
fn parse_rust_doc_comments(content: &str, file_path: &str, entries: &mut Vec<DocEntry>) -> Result<()> {
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        if line.starts_with("///") || line.starts_with("//!") {
            let start_line = i + 1;
            let mut doc_content = String::new();
            let mut title = String::new();

            // Collect consecutive doc comments
            while i < lines.len() {
                let current = lines[i].trim();
                if current.starts_with("///") {
                    let comment = current.trim_start_matches("///").trim();
                    if title.is_empty() && !comment.is_empty() {
                        title = comment.to_string();
                    }
                    doc_content.push_str(comment);
                    doc_content.push('\n');
                } else if current.starts_with("//!") {
                    let comment = current.trim_start_matches("//!").trim();
                    if title.is_empty() && !comment.is_empty() {
                        title = comment.to_string();
                    }
                    doc_content.push_str(comment);
                    doc_content.push('\n');
                } else {
                    break;
                }
                i += 1;
            }

            if !doc_content.is_empty() {
                let id = format!("{}:{}:{}", file_path, start_line, i);
                entries.push(DocEntry::new(
                    id,
                    title.chars().take(100).collect(),
                    doc_content.trim().to_string(),
                    file_path.to_string(),
                    start_line,
                    i + 1,
                    DocType::DocComment,
                ));
            }
        }
        i += 1;
    }

    Ok(())
}

/// Parse JSDoc comments (/** ... */)
fn parse_jsdoc_comments(content: &str, file_path: &str, entries: &mut Vec<DocEntry>) -> Result<()> {
    let re = regex::Regex::new(r"(?s)/\*\*\s*(.*?)\s*\*/")?;

    for cap in re.captures_iter(content) {
        let doc = cap.get(1).unwrap().as_str();
        let start = cap.get(0).unwrap().start();
        let line_start = content[..start].matches('\n').count() + 1;

        let lines: Vec<&str> = doc.lines()
            .map(|l| l.trim_start_matches('*').trim())
            .filter(|l| !l.is_empty())
            .collect();

        if !lines.is_empty() {
            let title = lines[0].chars().take(100).collect();
            let doc_content = lines.join("\n");

            let id = format!("{}:{}", file_path, line_start);
            entries.push(DocEntry::new(
                id,
                title,
                doc_content,
                file_path.to_string(),
                line_start,
                line_start + lines.len(),
                DocType::DocComment,
            ));
        }
    }

    Ok(())
}

/// Parse Python docstrings (""" ... """)
fn parse_python_docstrings(content: &str, file_path: &str, entries: &mut Vec<DocEntry>) -> Result<()> {
    let re = regex::Regex::new(r#"(?s)"{3}(.*?)"{3}"#)?;

    for cap in re.captures_iter(content) {
        let doc = cap.get(1).unwrap().as_str();
        let start = cap.get(0).unwrap().start();
        let line_start = content[..start].matches('\n').count() + 1;

        let lines: Vec<&str> = doc.lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();

        if !lines.is_empty() {
            let title = lines[0].chars().take(100).collect();
            let doc_content = lines.join("\n");

            let id = format!("{}:{}", file_path, line_start);
            entries.push(DocEntry::new(
                id,
                title,
                doc_content,
                file_path.to_string(),
                line_start,
                line_start + lines.len(),
                DocType::DocComment,
            ));
        }
    }

    Ok(())
}

/// Parse Go doc comments
fn parse_go_doc_comments(content: &str, file_path: &str, entries: &mut Vec<DocEntry>) -> Result<()> {
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        if line.starts_with("//") && !line.starts_with("///") {
            let start_line = i + 1;
            let mut doc_content = String::new();
            let mut title = String::new();

            while i < lines.len() {
                let current = lines[i].trim();
                if current.starts_with("//") && !current.starts_with("///") {
                    let comment = current.trim_start_matches("//").trim();
                    if title.is_empty() && !comment.is_empty() {
                        title = comment.to_string();
                    }
                    doc_content.push_str(comment);
                    doc_content.push('\n');
                } else {
                    break;
                }
                i += 1;
            }

            if !doc_content.is_empty() && doc_content.len() > 10 {
                let id = format!("{}:{}:{}", file_path, start_line, i);
                entries.push(DocEntry::new(
                    id,
                    title.chars().take(100).collect(),
                    doc_content.trim().to_string(),
                    file_path.to_string(),
                    start_line,
                    i + 1,
                    DocType::DocComment,
                ));
            }
        }
        i += 1;
    }

    Ok(())
}

/// Detect documentation format from file extension
pub fn detect_format(path: &Path) -> Option<DocFormat> {
    path.extension()?.to_str().and_then(|ext| match ext {
        "md" | "markdown" => Some(DocFormat::Markdown),
        "rs" => Some(DocFormat::RustDoc),
        "js" | "ts" | "jsx" | "tsx" => Some(DocFormat::JSDoc),
        "py" => Some(DocFormat::PyDoc),
        "go" => Some(DocFormat::GoDoc),
        _ => None,
    })
}
