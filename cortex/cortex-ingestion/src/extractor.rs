//! Metadata extraction from documents.
//!
//! This module provides comprehensive metadata extraction including:
//! - File system metadata
//! - Language detection
//! - Content type detection
//! - Keywords extraction
//! - Document properties

use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use whatlang::{detect as detect_lang, Lang};

/// Extract metadata from file path
pub fn extract_path_metadata(path: &Path) -> HashMap<String, serde_json::Value> {
    let mut metadata = HashMap::new();

    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        metadata.insert(
            "extension".to_string(),
            serde_json::Value::String(ext.to_string()),
        );
    }

    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        metadata.insert(
            "filename".to_string(),
            serde_json::Value::String(name.to_string()),
        );
    }

    if let Some(parent) = path.parent().and_then(|p| p.to_str()) {
        metadata.insert(
            "directory".to_string(),
            serde_json::Value::String(parent.to_string()),
        );
    }

    // Detect programming language from extension
    if let Some(lang) = detect_programming_language(path) {
        metadata.insert(
            "programming_language".to_string(),
            serde_json::Value::String(lang),
        );
    }

    metadata
}

/// Extract metadata from text content
pub fn extract_content_metadata(content: &str) -> HashMap<String, serde_json::Value> {
    let mut metadata = HashMap::new();

    metadata.insert(
        "length".to_string(),
        serde_json::Value::Number(content.len().into()),
    );
    metadata.insert(
        "lines".to_string(),
        serde_json::Value::Number(content.lines().count().into()),
    );

    let word_count = content.split_whitespace().count();
    metadata.insert(
        "words".to_string(),
        serde_json::Value::Number(word_count.into()),
    );

    // Detect natural language
    if let Some(lang) = detect_language(content) {
        metadata.insert(
            "language".to_string(),
            serde_json::Value::String(lang),
        );
    }

    // Extract keywords
    let keywords = extract_keywords(content, 10);
    if !keywords.is_empty() {
        metadata.insert(
            "keywords".to_string(),
            serde_json::Value::Array(
                keywords
                    .iter()
                    .map(|k| serde_json::Value::String(k.clone()))
                    .collect(),
            ),
        );
    }

    // Calculate reading time (average 200 words per minute)
    let reading_time_minutes = (word_count as f64 / 200.0).ceil() as u64;
    metadata.insert(
        "reading_time_minutes".to_string(),
        serde_json::Value::Number(reading_time_minutes.into()),
    );

    metadata
}

/// Detect natural language from text
pub fn detect_language(text: &str) -> Option<String> {
    if text.len() < 20 {
        return None;
    }

    detect_lang(text).map(|info| match info.lang() {
        Lang::Eng => "English",
        Lang::Spa => "Spanish",
        Lang::Fra => "French",
        Lang::Deu => "German",
        Lang::Rus => "Russian",
        Lang::Jpn => "Japanese",
        Lang::Cmn => "Chinese",
        Lang::Por => "Portuguese",
        Lang::Ita => "Italian",
        Lang::Kor => "Korean",
        _ => "Other",
    }.to_string())
}

/// Detect programming language from file extension
pub fn detect_programming_language(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .and_then(|ext| {
            Some(match ext.to_lowercase().as_str() {
                "rs" => "Rust",
                "py" => "Python",
                "js" | "jsx" => "JavaScript",
                "ts" | "tsx" => "TypeScript",
                "go" => "Go",
                "java" => "Java",
                "kt" | "kts" => "Kotlin",
                "swift" => "Swift",
                "c" => "C",
                "cpp" | "cc" | "cxx" => "C++",
                "h" | "hpp" => "C/C++ Header",
                "cs" => "C#",
                "rb" => "Ruby",
                "php" => "PHP",
                "scala" => "Scala",
                "clj" | "cljs" => "Clojure",
                "ex" | "exs" => "Elixir",
                "erl" | "hrl" => "Erlang",
                "hs" => "Haskell",
                "ml" => "OCaml",
                "r" => "R",
                "jl" => "Julia",
                "sql" => "SQL",
                "sh" | "bash" | "zsh" => "Shell",
                "ps1" => "PowerShell",
                "lua" => "Lua",
                "vim" => "VimScript",
                _ => return None,
            }.to_string())
        })
}

/// Extract keywords from text using simple frequency analysis
pub fn extract_keywords(text: &str, max_keywords: usize) -> Vec<String> {
    // Common stop words to filter out
    let stop_words: Vec<&str> = vec![
        "the", "be", "to", "of", "and", "a", "in", "that", "have", "i", "it", "for", "not", "on",
        "with", "he", "as", "you", "do", "at", "this", "but", "his", "by", "from", "they", "we",
        "say", "her", "she", "or", "an", "will", "my", "one", "all", "would", "there", "their",
        "what", "so", "up", "out", "if", "about", "who", "get", "which", "go", "me", "when",
        "make", "can", "like", "time", "no", "just", "him", "know", "take", "people", "into",
        "year", "your", "good", "some", "could", "them", "see", "other", "than", "then", "now",
        "look", "only", "come", "its", "over", "think", "also", "back", "after", "use", "two",
        "how", "our", "work", "first", "well", "way", "even", "new", "want", "because", "any",
        "these", "give", "day", "most", "us", "is", "was", "are", "been", "has", "had", "were",
        "said", "did", "having", "may", "should", "does", "am",
    ];

    // Extract words
    let word_regex = Regex::new(r"\b[a-zA-Z]{3,}\b").unwrap();
    let mut word_freq: HashMap<String, usize> = HashMap::new();

    for word in word_regex.find_iter(text) {
        let word_lower = word.as_str().to_lowercase();
        if !stop_words.contains(&word_lower.as_str()) {
            *word_freq.entry(word_lower).or_insert(0) += 1;
        }
    }

    // Sort by frequency
    let mut word_vec: Vec<(String, usize)> = word_freq.into_iter().collect();
    word_vec.sort_by(|a, b| b.1.cmp(&a.1));

    // Return top N keywords
    word_vec
        .into_iter()
        .take(max_keywords)
        .map(|(word, _)| word)
        .collect()
}

/// Extract document title from content
pub fn extract_title(content: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().take(10).collect();

    for line in lines {
        let trimmed = line.trim();
        // Look for markdown heading
        if trimmed.starts_with("# ") {
            return Some(trimmed.trim_start_matches('#').trim().to_string());
        }
        // Look for first non-empty line
        if !trimmed.is_empty() && trimmed.len() < 200 {
            return Some(trimmed.to_string());
        }
    }

    None
}

/// Extract author information from content (simple heuristic)
pub fn extract_author(content: &str) -> Option<String> {
    let author_regex = Regex::new(r"(?i)(?:author|by|written by):\s*(.+)").unwrap();

    if let Some(captures) = author_regex.captures(content) {
        if let Some(author) = captures.get(1) {
            return Some(author.as_str().trim().to_string());
        }
    }

    None
}

/// Extract dates from content
pub fn extract_dates(content: &str) -> Vec<String> {
    let date_regex = Regex::new(r"\b\d{4}-\d{2}-\d{2}\b|\b\d{1,2}/\d{1,2}/\d{2,4}\b").unwrap();

    date_regex
        .find_iter(content)
        .take(10)
        .map(|m| m.as_str().to_string())
        .collect()
}

/// Extract code blocks from markdown-style content
pub fn extract_code_blocks(content: &str) -> Vec<CodeBlock> {
    let mut code_blocks = Vec::new();
    let code_block_regex = Regex::new(r"```(\w+)?\n(.*?)```").unwrap();

    for cap in code_block_regex.captures_iter(content) {
        let language = cap.get(1).map(|m| m.as_str().to_string());
        let code = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();

        code_blocks.push(CodeBlock {
            language,
            code,
            start_line: content[..cap.get(0).unwrap().start()].lines().count(),
        });
    }

    code_blocks
}

/// Represents a code block
#[derive(Debug, Clone)]
pub struct CodeBlock {
    pub language: Option<String>,
    pub code: String,
    pub start_line: usize,
}

/// Extract links from content
pub fn extract_links(content: &str) -> Vec<Link> {
    let mut links = Vec::new();

    // Markdown links: [text](url)
    let md_link_regex = Regex::new(r"\[([^\]]+)\]\(([^\)]+)\)").unwrap();
    for cap in md_link_regex.captures_iter(content) {
        if let (Some(text), Some(url)) = (cap.get(1), cap.get(2)) {
            links.push(Link {
                text: text.as_str().to_string(),
                url: url.as_str().to_string(),
                link_type: LinkType::Markdown,
            });
        }
    }

    // Plain URLs
    let url_regex = Regex::new(r"https?://[^\s\)]+").unwrap();
    for url_match in url_regex.find_iter(content) {
        links.push(Link {
            text: url_match.as_str().to_string(),
            url: url_match.as_str().to_string(),
            link_type: LinkType::Plain,
        });
    }

    links
}

/// Represents a link
#[derive(Debug, Clone)]
pub struct Link {
    pub text: String,
    pub url: String,
    pub link_type: LinkType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LinkType {
    Markdown,
    Html,
    Plain,
}

/// Extract lists from content
pub fn extract_lists(content: &str) -> Vec<List> {
    let mut lists = Vec::new();
    let mut current_list: Option<List> = None;

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Bullet lists
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
            let item = trimmed[2..].trim().to_string();
            if let Some(ref mut list) = current_list {
                list.items.push(item);
            } else {
                current_list = Some(List {
                    list_type: ListType::Unordered,
                    items: vec![item],
                    start_line: line_num,
                });
            }
        }
        // Numbered lists
        else if let Some(cap) = Regex::new(r"^\d+\.\s+(.+)").unwrap().captures(trimmed) {
            let item = cap.get(1).unwrap().as_str().to_string();
            if let Some(ref mut list) = current_list {
                list.items.push(item);
            } else {
                current_list = Some(List {
                    list_type: ListType::Ordered,
                    items: vec![item],
                    start_line: line_num,
                });
            }
        }
        // Empty line or non-list content
        else if !trimmed.is_empty() {
            if let Some(list) = current_list.take() {
                lists.push(list);
            }
        }
    }

    // Add final list if exists
    if let Some(list) = current_list {
        lists.push(list);
    }

    lists
}

/// Represents a list
#[derive(Debug, Clone)]
pub struct List {
    pub list_type: ListType,
    pub items: Vec<String>,
    pub start_line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ListType {
    Ordered,
    Unordered,
}

/// Extract headings from content
pub fn extract_headings(content: &str) -> Vec<Heading> {
    let mut headings = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Markdown headings
        if trimmed.starts_with('#') {
            let level = trimmed.chars().take_while(|&c| c == '#').count();
            let text = trimmed[level..].trim().to_string();

            if !text.is_empty() {
                headings.push(Heading {
                    level,
                    text,
                    line: line_num,
                });
            }
        }
    }

    headings
}

/// Represents a heading
#[derive(Debug, Clone)]
pub struct Heading {
    pub level: usize,
    pub text: String,
    pub line: usize,
}

/// Comprehensive metadata extraction
pub fn extract_comprehensive_metadata(
    path: &Path,
    content: &str,
) -> HashMap<String, serde_json::Value> {
    let mut metadata = extract_path_metadata(path);
    let content_meta = extract_content_metadata(content);

    // Merge content metadata
    metadata.extend(content_meta);

    // Extract title
    if let Some(title) = extract_title(content) {
        metadata.insert(
            "title".to_string(),
            serde_json::Value::String(title),
        );
    }

    // Extract author
    if let Some(author) = extract_author(content) {
        metadata.insert(
            "author".to_string(),
            serde_json::Value::String(author),
        );
    }

    // Extract dates
    let dates = extract_dates(content);
    if !dates.is_empty() {
        metadata.insert(
            "dates_found".to_string(),
            serde_json::Value::Array(
                dates
                    .iter()
                    .map(|d| serde_json::Value::String(d.clone()))
                    .collect(),
            ),
        );
    }

    // Extract code blocks
    let code_blocks = extract_code_blocks(content);
    if !code_blocks.is_empty() {
        metadata.insert(
            "code_block_count".to_string(),
            serde_json::Value::Number(code_blocks.len().into()),
        );
        let languages: Vec<String> = code_blocks
            .iter()
            .filter_map(|cb| cb.language.clone())
            .collect();
        if !languages.is_empty() {
            metadata.insert(
                "code_languages".to_string(),
                serde_json::Value::Array(
                    languages.iter().map(|l| serde_json::Value::String(l.clone())).collect()
                ),
            );
        }
    }

    // Extract links
    let links = extract_links(content);
    if !links.is_empty() {
        metadata.insert(
            "link_count".to_string(),
            serde_json::Value::Number(links.len().into()),
        );
    }

    // Extract headings
    let headings = extract_headings(content);
    if !headings.is_empty() {
        metadata.insert(
            "heading_count".to_string(),
            serde_json::Value::Number(headings.len().into()),
        );
    }

    metadata
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_metadata() {
        let path = Path::new("/test/dir/file.rs");
        let metadata = extract_path_metadata(path);

        assert_eq!(metadata.get("extension"), Some(&serde_json::Value::String("rs".to_string())));
        assert_eq!(metadata.get("filename"), Some(&serde_json::Value::String("file.rs".to_string())));
    }

    #[test]
    fn test_content_metadata() {
        let content = "Hello world\nSecond line";
        let metadata = extract_content_metadata(content);

        assert_eq!(metadata.get("lines"), Some(&serde_json::Value::Number(2.into())));
        assert_eq!(metadata.get("words"), Some(&serde_json::Value::Number(4.into()))); // "Hello", "world", "Second", "line"
    }
}
