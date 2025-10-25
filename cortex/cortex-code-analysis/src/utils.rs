//! Utility functions for file I/O, path manipulation, and language detection.
//!
//! This module provides production-ready utilities for:
//! - Reading files with BOM (Byte Order Mark) detection and handling
//! - Detecting programming language from Emacs/Vim modelines in file content
//! - Cross-platform path normalization
//! - Calculating path distances for dependency resolution

use anyhow::{Context, Result};
use regex::bytes::Regex;
use std::fs::{self, File};
use std::io::Read as IoRead;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

use crate::Lang;

/// Reads a file with BOM detection and UTF-8 validation.
///
/// This function:
/// - Detects and strips UTF-16 BE/LE and UTF-8 BOMs
/// - Validates UTF-8 content
/// - Skips files that are too small (<=3 bytes) or contain invalid UTF-8
/// - Removes trailing blank lines and adds a final newline
///
/// # Arguments
///
/// * `path` - Path to the file to read
///
/// # Returns
///
/// Returns `Ok(Some(data))` if the file is valid, `Ok(None)` if the file should be
/// skipped (too small or invalid UTF-8), or an error if the file cannot be read.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use cortex_code_analysis::read_file_with_bom;
///
/// # fn example() -> anyhow::Result<()> {
/// let path = Path::new("example.rs");
/// if let Some(content) = read_file_with_bom(path)? {
///     // Process the file content
///     println!("Read {} bytes", content.len());
/// }
/// # Ok(())
/// # }
/// ```
pub fn read_file_with_bom(path: &Path) -> Result<Option<Vec<u8>>> {
    let file_size = fs::metadata(path)
        .with_context(|| format!("Failed to get metadata for {}", path.display()))?
        .len() as usize;

    // Skip very small files (likely empty)
    if file_size <= 3 {
        return Ok(None);
    }

    let mut file = File::open(path)
        .with_context(|| format!("Failed to open file {}", path.display()))?;

    // Read the first 64 bytes to check for BOM and validate UTF-8
    let read_size = 64.min(file_size);
    let mut start = vec![0; read_size];

    file.read_exact(&mut start)
        .with_context(|| format!("Failed to read initial bytes from {}", path.display()))?;

    // Detect and skip BOM if present
    let start_slice = if start.len() >= 3 && start[..3] == [0xEF, 0xBB, 0xBF] {
        // UTF-8 BOM
        &start[3..]
    } else if start.len() >= 2 && (start[..2] == [0xFE, 0xFF] || start[..2] == [0xFF, 0xFE]) {
        // UTF-16 BE or LE BOM
        &start[2..]
    } else {
        &start[..]
    };

    // Validate UTF-8 by attempting to convert to string
    let mut head = String::from_utf8_lossy(start_slice).into_owned();

    // Remove the last character as it might be incomplete UTF-8 sequence
    head.pop();

    // Check for invalid UTF-8 sequences (represented as replacement character)
    if head.contains('\u{FFFD}') {
        return Ok(None);
    }

    // Allocate buffer for the entire file
    let mut data = Vec::with_capacity(file_size + 2);
    data.extend_from_slice(start_slice);

    // Read the rest of the file
    file.read_to_end(&mut data)
        .with_context(|| format!("Failed to read remaining bytes from {}", path.display()))?;

    // Remove trailing blank lines and add final newline
    remove_blank_lines(&mut data);

    Ok(Some(data))
}

/// Removes trailing newlines and carriage returns, then adds a single newline.
///
/// This normalizes file endings to ensure all files end with exactly one newline.
///
/// # Arguments
///
/// * `data` - The file content to normalize
fn remove_blank_lines(data: &mut Vec<u8>) {
    let count_trailing = data
        .iter()
        .rev()
        .take_while(|&&c| c == b'\n' || c == b'\r')
        .count();

    if count_trailing > 0 {
        data.truncate(data.len() - count_trailing);
    }

    data.push(b'\n');
}

// Regular expressions for detecting Emacs and Vim modelines
static RE1_EMACS: OnceLock<Regex> = OnceLock::new();
static RE2_EMACS: OnceLock<Regex> = OnceLock::new();
static RE1_VIM: OnceLock<Regex> = OnceLock::new();

const FIRST_EMACS_EXPRESSION: &str = r"(?i)-\*-.*[^-\w]mode\s*:\s*([^:;\s]+)";
const SECOND_EMACS_EXPRESSION: &str = r"-\*-\s*([^:;\s]+)\s*-\*-";
const VIM_EXPRESSION: &str = r"(?i)vim\s*:.*[^\w]ft\s*=\s*([^:\s]+)";

/// Gets a regex match from a line using a cached compiled regex.
#[inline(always)]
fn get_regex<'a>(
    once_lock: &'a OnceLock<Regex>,
    line: &'a [u8],
    regex_pattern: &str,
) -> Option<regex::bytes::Captures<'a>> {
    once_lock
        .get_or_init(|| Regex::new(regex_pattern).expect("Invalid regex pattern"))
        .captures(line)
}

/// Extracts mode information from Emacs/Vim modelines.
///
/// This function searches the first and last few lines of a file for editor
/// modelines (Emacs -*- mode -*- or Vim modeline) and extracts the language mode.
///
/// # Arguments
///
/// * `buf` - The file content to search
///
/// # Returns
///
/// Returns the extracted mode string in lowercase, or None if no modeline found.
fn get_emacs_mode(buf: &[u8]) -> Option<String> {
    // Check first 5 lines for Emacs/Vim modelines
    for (i, line) in buf.splitn(5, |c| *c == b'\n').enumerate() {
        if let Some(cap) = get_regex(&RE1_EMACS, line, FIRST_EMACS_EXPRESSION) {
            return std::str::from_utf8(&cap[1]).ok().map(|s| s.to_lowercase());
        } else if let Some(cap) = get_regex(&RE2_EMACS, line, SECOND_EMACS_EXPRESSION) {
            return std::str::from_utf8(&cap[1]).ok().map(|s| s.to_lowercase());
        } else if let Some(cap) = get_regex(&RE1_VIM, line, VIM_EXPRESSION) {
            return std::str::from_utf8(&cap[1]).ok().map(|s| s.to_lowercase());
        }
        if i == 3 {
            break;
        }
    }

    // Check last 5 lines for Vim modelines
    for (i, line) in buf.rsplitn(5, |c| *c == b'\n').enumerate() {
        if let Some(cap) = get_regex(&RE1_VIM, line, VIM_EXPRESSION) {
            return std::str::from_utf8(&cap[1]).ok().map(|s| s.to_lowercase());
        }
        if i == 3 {
            break;
        }
    }

    None
}

/// Maps Emacs/Vim mode names to cortex Lang enum.
///
/// # Arguments
///
/// * `mode` - The mode string from an editor modeline
///
/// # Returns
///
/// Returns the corresponding Lang variant, or None if the mode is not recognized.
fn mode_to_lang(mode: &str) -> Option<Lang> {
    match mode.to_lowercase().as_str() {
        "rust" | "rs" => Some(Lang::Rust),
        "typescript" | "ts" => Some(Lang::TypeScript),
        "tsx" => Some(Lang::Tsx),
        "javascript" | "js" => Some(Lang::JavaScript),
        "jsx" => Some(Lang::Jsx),
        "python" | "py" => Some(Lang::Python),
        "java" => Some(Lang::Java),
        "kotlin" | "kt" => Some(Lang::Kotlin),
        "c++" | "cpp" | "cxx" | "c" | "cc" => Some(Lang::Cpp),
        "objective-c++" | "objc++" | "obj-c++" => Some(Lang::Cpp),
        "objective-c" | "objc" => Some(Lang::Cpp),
        _ => None,
    }
}

/// Detects programming language from file content using editor modelines.
///
/// This function checks for Emacs -*- mode -*- and Vim modeline syntax in the
/// first and last few lines of a file to detect the programming language.
///
/// # Arguments
///
/// * `content` - The file content to analyze
///
/// # Returns
///
/// Returns the detected Lang if a valid modeline is found, or None otherwise.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::{guess_language_from_content, Lang};
///
/// let content = b"// -*- mode: rust -*-\nfn main() {}";
/// assert_eq!(guess_language_from_content(content), Some(Lang::Rust));
///
/// let content = b"// vim: ft=python\nprint('hello')";
/// assert_eq!(guess_language_from_content(content), Some(Lang::Python));
/// ```
pub fn guess_language_from_content(content: &[u8]) -> Option<Lang> {
    get_emacs_mode(content).and_then(|mode| mode_to_lang(&mode))
}

/// Normalizes a path by resolving `.` and `..` components.
///
/// This function performs cross-platform path normalization:
/// - Resolves `.` (current directory) by skipping
/// - Resolves `..` (parent directory) by popping from the path
/// - Preserves Windows drive prefixes
/// - Handles root directory components
///
/// # Arguments
///
/// * `path` - The path to normalize
///
/// # Returns
///
/// Returns the normalized PathBuf.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use cortex_code_analysis::normalize_path;
///
/// let path = Path::new("foo/./bar/../baz");
/// let normalized = normalize_path(path);
/// assert_eq!(normalized, Path::new("foo/baz"));
/// ```
pub fn normalize_path<P: AsRef<Path>>(path: P) -> PathBuf {
    // Adapted from Cargo sources: https://github.com/rust-lang/cargo/blob/master/src/cargo/util/paths.rs#L65
    let mut components = path.as_ref().components().peekable();

    // Handle Windows drive prefix
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!("Prefix should have been handled"),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {
                // Skip current directory markers
            }
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

/// Calculates the path distance between two paths.
///
/// This function finds the closest common ancestor of two paths and returns the
/// sum of path components from the ancestor to each path. This is useful for
/// determining which of multiple possible include/import paths is "closest" to
/// the current file.
///
/// # Arguments
///
/// * `path1` - First path (typically the current file)
/// * `path2` - Second path (typically a potential include/import target)
///
/// # Returns
///
/// Returns Some(distance) if the paths share a common ancestor, or None if they
/// don't share any common ancestor.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use cortex_code_analysis::get_paths_dist;
///
/// let path1 = Path::new("/project/src/module/file.rs");
/// let path2 = Path::new("/project/src/lib.rs");
///
/// // Distance is 2 (module/file.rs -> src) + 1 (src -> lib.rs) = 3
/// assert_eq!(get_paths_dist(path1, path2), Some(3));
/// ```
pub fn get_paths_dist(path1: &Path, path2: &Path) -> Option<usize> {
    for ancestor in path1.ancestors() {
        if path2.starts_with(ancestor) && !ancestor.as_os_str().is_empty() {
            let path1_relative = path1.strip_prefix(ancestor).ok()?;
            let path2_relative = path2.strip_prefix(ancestor).ok()?;

            return Some(
                path1_relative.components().count() + path2_relative.components().count()
            );
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_read_file_with_bom_utf8() {
        let mut file = NamedTempFile::new().unwrap();

        // UTF-8 BOM + content
        let content = b"\xEF\xBB\xBFfn main() {}";
        file.write_all(content).unwrap();
        file.flush().unwrap();

        let result = read_file_with_bom(file.path()).unwrap();
        assert!(result.is_some());

        let data = result.unwrap();
        // Should strip BOM and keep content, adding newline
        assert_eq!(&data[..], b"fn main() {}\n");
    }

    #[test]
    fn test_read_file_with_bom_utf16_be() {
        let mut file = NamedTempFile::new().unwrap();

        // UTF-16 BE BOM + content
        let content = b"\xFE\xFFabc";
        file.write_all(content).unwrap();
        file.flush().unwrap();

        let result = read_file_with_bom(file.path()).unwrap();
        assert!(result.is_some());

        let data = result.unwrap();
        // Should strip BOM
        assert_eq!(&data[..3], b"abc");
    }

    #[test]
    fn test_read_file_with_bom_utf16_le() {
        let mut file = NamedTempFile::new().unwrap();

        // UTF-16 LE BOM + content
        let content = b"\xFF\xFEabc";
        file.write_all(content).unwrap();
        file.flush().unwrap();

        let result = read_file_with_bom(file.path()).unwrap();
        assert!(result.is_some());

        let data = result.unwrap();
        // Should strip BOM
        assert_eq!(&data[..3], b"abc");
    }

    #[test]
    fn test_read_file_with_bom_no_bom() {
        let mut file = NamedTempFile::new().unwrap();

        let content = b"fn main() {}";
        file.write_all(content).unwrap();
        file.flush().unwrap();

        let result = read_file_with_bom(file.path()).unwrap();
        assert!(result.is_some());

        let data = result.unwrap();
        assert_eq!(&data[..], b"fn main() {}\n");
    }

    #[test]
    fn test_read_file_with_bom_too_small() {
        let mut file = NamedTempFile::new().unwrap();

        // File with 3 bytes or less should be skipped
        let content = b"abc";
        file.write_all(content).unwrap();
        file.flush().unwrap();

        let result = read_file_with_bom(file.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_read_file_with_bom_trailing_newlines() {
        let mut file = NamedTempFile::new().unwrap();

        let content = b"fn main() {}\n\n\n";
        file.write_all(content).unwrap();
        file.flush().unwrap();

        let result = read_file_with_bom(file.path()).unwrap();
        assert!(result.is_some());

        let data = result.unwrap();
        // Should remove trailing newlines and add one
        assert_eq!(&data[..], b"fn main() {}\n");
    }

    #[test]
    fn test_guess_language_emacs_mode_rust() {
        let content = b"// -*- mode: rust -*-\nfn main() {}";
        assert_eq!(guess_language_from_content(content), Some(Lang::Rust));
    }

    #[test]
    fn test_guess_language_emacs_mode_python() {
        let content = b"# -*- mode: python -*-\nprint('hello')";
        assert_eq!(guess_language_from_content(content), Some(Lang::Python));
    }

    #[test]
    fn test_guess_language_emacs_mode_typescript() {
        let content = b"// -*- mode: typescript -*-\nconst x = 1;";
        assert_eq!(guess_language_from_content(content), Some(Lang::TypeScript));
    }

    #[test]
    fn test_guess_language_emacs_short_form() {
        let content = b"// -*- rust -*-\nfn main() {}";
        assert_eq!(guess_language_from_content(content), Some(Lang::Rust));
    }

    #[test]
    fn test_guess_language_vim_modeline() {
        let content = b"// vim: ft=rust\nfn main() {}";
        assert_eq!(guess_language_from_content(content), Some(Lang::Rust));
    }

    #[test]
    fn test_guess_language_vim_modeline_at_end() {
        let content = b"fn main() {}\n\n\n// vim: set ft=rust ts=4 sw=4:";
        assert_eq!(guess_language_from_content(content), Some(Lang::Rust));
    }

    #[test]
    fn test_guess_language_cpp_variants() {
        let content = b"// -*- mode: c++ -*-\nint main() {}";
        assert_eq!(guess_language_from_content(content), Some(Lang::Cpp));

        let content = b"// vim: ft=cpp\nint main() {}";
        assert_eq!(guess_language_from_content(content), Some(Lang::Cpp));
    }

    #[test]
    fn test_guess_language_no_modeline() {
        let content = b"fn main() {}";
        assert_eq!(guess_language_from_content(content), None);
    }

    #[test]
    fn test_normalize_path_simple() {
        let path = Path::new("foo/bar");
        assert_eq!(normalize_path(path), Path::new("foo/bar"));
    }

    #[test]
    fn test_normalize_path_current_dir() {
        let path = Path::new("foo/./bar");
        assert_eq!(normalize_path(path), Path::new("foo/bar"));
    }

    #[test]
    fn test_normalize_path_parent_dir() {
        let path = Path::new("foo/bar/../baz");
        assert_eq!(normalize_path(path), Path::new("foo/baz"));
    }

    #[test]
    fn test_normalize_path_complex() {
        let path = Path::new("foo/./bar/../baz/./qux/..");
        assert_eq!(normalize_path(path), Path::new("foo/baz"));
    }

    #[test]
    fn test_normalize_path_multiple_parent() {
        let path = Path::new("foo/bar/baz/../../qux");
        assert_eq!(normalize_path(path), Path::new("foo/qux"));
    }

    #[test]
    fn test_get_paths_dist_same_directory() {
        let path1 = Path::new("/project/src/file1.rs");
        let path2 = Path::new("/project/src/file2.rs");

        // Both in same directory: distance = 1 + 1 = 2
        assert_eq!(get_paths_dist(path1, path2), Some(2));
    }

    #[test]
    fn test_get_paths_dist_parent_child() {
        let path1 = Path::new("/project/src/module/file.rs");
        let path2 = Path::new("/project/src/lib.rs");

        // Distance: 2 steps up (module/file.rs) + 1 step (lib.rs) = 3
        assert_eq!(get_paths_dist(path1, path2), Some(3));
    }

    #[test]
    fn test_get_paths_dist_siblings() {
        let path1 = Path::new("/project/src/module1/file.rs");
        let path2 = Path::new("/project/src/module2/file.rs");

        // Distance: 2 (module1/file.rs) + 2 (module2/file.rs) = 4
        assert_eq!(get_paths_dist(path1, path2), Some(4));
    }

    #[test]
    fn test_get_paths_dist_no_common_ancestor() {
        let path1 = Path::new("/project1/src/file.rs");
        let path2 = Path::new("/project2/src/file.rs");

        // Different root paths on Unix systems will still share "/" as ancestor
        // On Windows, different drives would have no common ancestor
        assert!(get_paths_dist(path1, path2).is_some());
    }

    #[test]
    fn test_remove_blank_lines() {
        let mut data = b"content\n\n\n".to_vec();
        remove_blank_lines(&mut data);
        assert_eq!(&data[..], b"content\n");

        let mut data = b"content\r\n\r\n".to_vec();
        remove_blank_lines(&mut data);
        assert_eq!(&data[..], b"content\n");

        let mut data = b"content".to_vec();
        remove_blank_lines(&mut data);
        assert_eq!(&data[..], b"content\n");
    }

    #[test]
    fn test_mode_to_lang_mappings() {
        assert_eq!(mode_to_lang("rust"), Some(Lang::Rust));
        assert_eq!(mode_to_lang("rs"), Some(Lang::Rust));
        assert_eq!(mode_to_lang("RUST"), Some(Lang::Rust)); // Case insensitive

        assert_eq!(mode_to_lang("python"), Some(Lang::Python));
        assert_eq!(mode_to_lang("py"), Some(Lang::Python));

        assert_eq!(mode_to_lang("typescript"), Some(Lang::TypeScript));
        assert_eq!(mode_to_lang("ts"), Some(Lang::TypeScript));

        assert_eq!(mode_to_lang("javascript"), Some(Lang::JavaScript));
        assert_eq!(mode_to_lang("js"), Some(Lang::JavaScript));

        assert_eq!(mode_to_lang("c++"), Some(Lang::Cpp));
        assert_eq!(mode_to_lang("cpp"), Some(Lang::Cpp));
        assert_eq!(mode_to_lang("objective-c++"), Some(Lang::Cpp));

        assert_eq!(mode_to_lang("unknown"), None);
    }
}
