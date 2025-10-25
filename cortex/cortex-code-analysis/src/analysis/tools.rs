//! Enhanced Utility Functions and Tools
//!
//! This module provides a comprehensive set of utility functions for:
//! - File I/O with BOM handling and encoding detection
//! - Language detection from file extensions and content
//! - Path manipulation and normalization
//! - Text processing and formatting
//! - Efficient data structures for code analysis
//!
//! # Examples
//!
//! ```no_run
//! use cortex_code_analysis::analysis::tools;
//! use std::path::Path;
//!
//! // Read a file with proper encoding handling
//! let content = tools::read_file_safe(Path::new("src/main.rs"))?;
//!
//! // Detect language from content and path
//! let (lang, name) = tools::detect_language(&content, "src/main.rs")?;
//! # Ok::<(), anyhow::Error>(())
//! ```

use crate::lang::Lang;
use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

/// Read a file with BOM (Byte Order Mark) detection and removal
///
/// Handles UTF-8, UTF-16LE, and UTF-16BE BOMs automatically.
///
/// # Examples
///
/// ```no_run
/// use cortex_code_analysis::analysis::tools::read_file_safe;
/// use std::path::Path;
///
/// let content = read_file_safe(Path::new("src/main.rs"))?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn read_file_safe(path: &Path) -> Result<Vec<u8>> {
    let mut file = File::open(path).context("Failed to open file")?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .context("Failed to read file")?;

    // Remove BOM if present
    let data = remove_bom(data);

    // Remove trailing blank lines
    let data = remove_trailing_blank_lines(data);

    Ok(data)
}

/// Read a file and add an EOL at its end if missing
///
/// Also handles BOM detection and validation of UTF-8 encoding.
/// Returns None if the file is too small or contains invalid UTF-8.
pub fn read_file_with_eol(path: &Path) -> Result<Option<Vec<u8>>> {
    let file_size = fs::metadata(path)
        .map(|m| m.len() as usize)
        .unwrap_or(1024 * 1024);

    // Skip files that are too small (likely empty)
    if file_size <= 3 {
        return Ok(None);
    }

    let mut file = File::open(path).context("Failed to open file")?;

    // Read first 64 bytes for BOM and encoding validation
    let buffer_size = 64.min(file_size);
    let mut start = vec![0; buffer_size];

    if file.read_exact(&mut start).is_err() {
        return Ok(None);
    }

    // Skip BOM if present
    let start_slice = skip_bom(&start);

    // Validate UTF-8 encoding on first chunk
    if !validate_utf8_chunk(start_slice) {
        return Ok(None);
    }

    // Read rest of file
    let mut data = Vec::with_capacity(file_size + 2);
    data.extend_from_slice(start_slice);
    file.read_to_end(&mut data)
        .context("Failed to read file")?;

    // Remove trailing blank lines and ensure EOL
    let data = remove_trailing_blank_lines(data);

    Ok(Some(data))
}

/// Write data to a file
pub fn write_file(path: &Path, data: &[u8]) -> Result<()> {
    let mut file = File::create(path).context("Failed to create file")?;
    file.write_all(data).context("Failed to write file")?;
    Ok(())
}

/// Remove BOM from byte array
fn remove_bom(mut data: Vec<u8>) -> Vec<u8> {
    if data.len() >= 3 && data[0] == 0xEF && data[1] == 0xBB && data[2] == 0xBF {
        // UTF-8 BOM
        data.drain(0..3);
    } else if data.len() >= 2 && ((data[0] == 0xFF && data[1] == 0xFE) || (data[0] == 0xFE && data[1] == 0xFF)) {
        // UTF-16 BOM
        data.drain(0..2);
    }
    data
}

/// Skip BOM in a byte slice (returns slice after BOM)
fn skip_bom(data: &[u8]) -> &[u8] {
    if data.len() >= 3 && data[0] == 0xEF && data[1] == 0xBB && data[2] == 0xBF {
        &data[3..]
    } else if data.len() >= 2 && ((data[0] == 0xFF && data[1] == 0xFE) || (data[0] == 0xFE && data[1] == 0xFF)) {
        &data[2..]
    } else {
        data
    }
}

/// Validate that a chunk of data is valid UTF-8
fn validate_utf8_chunk(data: &[u8]) -> bool {
    let mut text = String::from_utf8_lossy(data).into_owned();
    // Remove last char as it might be in the middle of a UTF-8 sequence
    text.pop();
    // Check for replacement character (invalid UTF-8)
    !text.contains('\u{FFFD}')
}

/// Remove trailing blank lines from data and ensure EOL at end
fn remove_trailing_blank_lines(mut data: Vec<u8>) -> Vec<u8> {
    let trailing = data
        .iter()
        .rev()
        .take_while(|&&c| c == b'\n' || c == b'\r')
        .count();

    if trailing > 0 {
        data.truncate(data.len() - trailing);
    }

    data.push(b'\n');
    data
}

/// Detect language from file extension
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::analysis::tools::language_from_extension;
/// use cortex_code_analysis::Lang;
/// use std::path::Path;
///
/// let lang = language_from_extension(Path::new("test.rs"));
/// assert_eq!(lang, Some(Lang::Rust));
/// ```
pub fn language_from_extension(path: &Path) -> Option<Lang> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .and_then(|ext| match ext.as_str() {
            "rs" => Some(Lang::Rust),
            "ts" => Some(Lang::TypeScript),
            "tsx" => Some(Lang::Tsx),
            "js" => Some(Lang::JavaScript),
            "jsx" => Some(Lang::Jsx),
            "py" => Some(Lang::Python),
            "cpp" | "cc" | "cxx" | "c++" | "hpp" | "hh" | "hxx" => Some(Lang::Cpp),
            "c" | "h" => Some(Lang::Cpp), // Treat C as C++ for now
            "java" => Some(Lang::Java),
            "kt" | "kts" => Some(Lang::Kotlin),
            _ => None,
        })
}

// Regular expressions for editor mode detection
static RE_EMACS_1: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)-\*-.*[^-\w]mode\s*:\s*([^:;\s]+)").unwrap());
static RE_EMACS_2: Lazy<Regex> = Lazy::new(|| Regex::new(r"-\*-\s*([^:;\s]+)\s*-\*-").unwrap());
static RE_VIM: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)vim\s*:.*[^\w]ft\s*=\s*([^:\s]+)").unwrap());

/// Extract editor mode from file content (Emacs/Vim modelines)
fn extract_editor_mode(content: &[u8]) -> Option<String> {
    // Check first 5 lines
    for (i, line) in content.split(|&c| c == b'\n').enumerate() {
        if i >= 5 {
            break;
        }

        if let Ok(line_str) = std::str::from_utf8(line) {
            if let Some(caps) = RE_EMACS_1.captures(line_str) {
                return Some(caps[1].to_lowercase());
            }
            if let Some(caps) = RE_EMACS_2.captures(line_str) {
                return Some(caps[1].to_lowercase());
            }
            if let Some(caps) = RE_VIM.captures(line_str) {
                return Some(caps[1].to_lowercase());
            }
        }
    }

    // Check last 5 lines for Vim modelines
    let lines: Vec<_> = content.rsplitn(6, |&c| c == b'\n').collect();
    for line in lines.iter().take(5) {
        if let Ok(line_str) = std::str::from_utf8(line) {
            if let Some(caps) = RE_VIM.captures(line_str) {
                return Some(caps[1].to_lowercase());
            }
        }
    }

    None
}

/// Map editor mode string to Lang
fn language_from_mode(mode: &str) -> Option<Lang> {
    match mode {
        "rust" => Some(Lang::Rust),
        "typescript" => Some(Lang::TypeScript),
        "javascript" | "js" => Some(Lang::JavaScript),
        "python" | "py" => Some(Lang::Python),
        "c++" | "cpp" => Some(Lang::Cpp),
        "java" => Some(Lang::Java),
        "kotlin" => Some(Lang::Kotlin),
        _ => None,
    }
}

/// Detect language from both file path and content
///
/// First tries to detect from file extension, then falls back to
/// editor modelines in the content.
///
/// # Examples
///
/// ```no_run
/// use cortex_code_analysis::analysis::tools::detect_language;
/// use std::path::Path;
///
/// let content = b"// -*- mode: rust -*-\nfn main() {}";
/// let (lang, name) = detect_language(content, Path::new("test.txt"))?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn detect_language<P: AsRef<Path>>(content: &[u8], path: P) -> Result<(Option<Lang>, &'static str)> {
    let path = path.as_ref();
    let from_ext = language_from_extension(path);
    let from_mode = extract_editor_mode(content).and_then(|mode| language_from_mode(&mode));

    match (from_ext, from_mode) {
        (Some(ext_lang), Some(mode_lang)) if ext_lang == mode_lang => {
            Ok((Some(ext_lang), ext_lang.get_name()))
        }
        (Some(ext_lang), _) => Ok((Some(ext_lang), ext_lang.get_name())),
        (None, Some(mode_lang)) => Ok((Some(mode_lang), mode_lang.get_name())),
        (None, None) => Ok((None, "")),
    }
}

/// Normalize a path by removing `.` and `..` components
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::analysis::tools::normalize_path;
/// use std::path::Path;
///
/// let path = Path::new("./src/../lib/main.rs");
/// let normalized = normalize_path(path);
/// ```
pub fn normalize_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut components = path.as_ref().components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

/// Calculate the distance between two paths (number of components)
///
/// Returns the number of path components needed to navigate from path1 to path2,
/// or None if they don't share a common ancestor.
pub fn path_distance(path1: &Path, path2: &Path) -> Option<usize> {
    for ancestor in path1.ancestors() {
        if path2.starts_with(ancestor) && !ancestor.as_os_str().is_empty() {
            let path1 = path1.strip_prefix(ancestor).unwrap();
            let path2 = path2.strip_prefix(ancestor).unwrap();
            return Some(path1.components().count() + path2.components().count());
        }
    }
    None
}

/// Find the best matching file from a set of possibilities
///
/// Uses heuristics to find the most likely matching file:
/// 1. Files with matching full path
/// 2. Files in the same directory
/// 3. Files with minimum path distance
pub fn find_best_match(
    current_path: &Path,
    include_path: &str,
    all_files: &HashMap<String, Vec<PathBuf>>,
) -> Vec<PathBuf> {
    let include_path = normalize_path(include_path);

    let filename = match include_path.file_name() {
        Some(name) => name.to_string_lossy(),
        None => return Vec::new(),
    };

    let possibilities = match all_files.get(filename.as_ref()) {
        Some(paths) => paths,
        None => return Vec::new(),
    };

    if possibilities.len() == 1 {
        return possibilities.clone();
    }

    // Try exact path match
    let mut matches: Vec<_> = possibilities
        .iter()
        .filter(|p| p.ends_with(&include_path) && *p != current_path)
        .cloned()
        .collect();

    if matches.len() == 1 {
        return matches;
    }
    matches.clear();

    // Try same directory match
    if let Some(parent) = current_path.parent() {
        matches = possibilities
            .iter()
            .filter(|p| p.starts_with(parent) && *p != current_path)
            .cloned()
            .collect();

        if matches.len() == 1 {
            return matches;
        }
        matches.clear();
    }

    // Find minimum distance matches
    let mut min_dist = usize::MAX;
    let mut min_paths = Vec::new();

    for p in possibilities.iter() {
        if p == current_path {
            continue;
        }

        if let Some(dist) = path_distance(current_path, p) {
            match dist.cmp(&min_dist) {
                std::cmp::Ordering::Less => {
                    min_dist = dist;
                    min_paths.clear();
                    min_paths.push(p.clone());
                }
                std::cmp::Ordering::Equal => {
                    min_paths.push(p.clone());
                }
                std::cmp::Ordering::Greater => {}
            }
        }
    }

    min_paths
}

/// Format a number with thousand separators
pub fn format_number(num: usize) -> String {
    num.to_string()
        .as_bytes()
        .rchunks(3)
        .rev()
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<_>>()
        .join(",")
}

/// Check if a path points to a hidden file or directory
pub fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_remove_bom() {
        let data_utf8 = vec![0xEF, 0xBB, 0xBF, b'h', b'e', b'l', b'l', b'o'];
        let result = remove_bom(data_utf8);
        assert_eq!(result, b"hello");

        let data_utf16le = vec![0xFF, 0xFE, b'h', b'i'];
        let result = remove_bom(data_utf16le);
        assert_eq!(result, b"hi");
    }

    #[test]
    fn test_language_from_extension() {
        assert_eq!(
            language_from_extension(Path::new("test.rs")),
            Some(Lang::Rust)
        );
        assert_eq!(
            language_from_extension(Path::new("test.ts")),
            Some(Lang::TypeScript)
        );
        assert_eq!(
            language_from_extension(Path::new("test.py")),
            Some(Lang::Python)
        );
        assert_eq!(language_from_extension(Path::new("test.txt")), None);
    }

    #[test]
    fn test_detect_language_from_mode() {
        let content = b"// -*- mode: rust -*-\nfn main() {}";
        let (lang, _) = detect_language(content, Path::new("test.txt")).unwrap();
        assert_eq!(lang, Some(Lang::Rust));

        let content = b"# vim: ft=python\ndef main(): pass";
        let (lang, _) = detect_language(content, Path::new("test.txt")).unwrap();
        assert_eq!(lang, Some(Lang::Python));
    }

    #[test]
    fn test_normalize_path() {
        let path = Path::new("./src/../lib/main.rs");
        let normalized = normalize_path(path);
        assert_eq!(normalized, Path::new("lib/main.rs"));
    }

    #[test]
    fn test_path_distance() {
        let path1 = Path::new("/home/user/project/src/main.rs");
        let path2 = Path::new("/home/user/project/lib/util.rs");
        let dist = path_distance(path1, path2);
        assert_eq!(dist, Some(4)); // 2 up from main.rs, 2 down to util.rs
    }

    #[test]
    fn test_is_hidden() {
        assert!(is_hidden(Path::new(".hidden")));
        assert!(is_hidden(Path::new("/path/.hidden")));
        assert!(!is_hidden(Path::new("visible")));
        assert!(!is_hidden(Path::new("/path/visible")));
    }

    #[test]
    fn test_read_write_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Hello, World!";

        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let read_data = read_file_safe(temp_file.path()).unwrap();
        assert_eq!(&read_data[..test_data.len()], test_data);
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1000000), "1,000,000");
        assert_eq!(format_number(123), "123");
    }
}
