# Utils Module Integration

This document describes the integration of utility functions from the experimental `adv-rust-code-analysis` project into `cortex-code-analysis`.

## Overview

The `utils` module provides production-ready utility functions for:
- File I/O with BOM (Byte Order Mark) detection
- Language detection from editor modelines
- Cross-platform path normalization
- Path distance calculation for dependency resolution

## Integrated Utilities

### 1. `read_file_with_bom(path: &Path) -> Result<Option<Vec<u8>>>`

Reads files with intelligent BOM detection and UTF-8 validation.

**Features:**
- Detects and strips UTF-16 BE/LE and UTF-8 BOMs
- Validates UTF-8 content
- Skips very small files (≤3 bytes)
- Skips files with invalid UTF-8
- Normalizes line endings (removes trailing newlines, adds final newline)

**Example:**
```rust
use cortex_code_analysis::read_file_with_bom;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let path = Path::new("example.rs");
    if let Some(content) = read_file_with_bom(path)? {
        // Process the file content
        println!("Read {} bytes", content.len());
    } else {
        println!("File skipped (too small or invalid UTF-8)");
    }
    Ok(())
}
```

### 2. `guess_language_from_content(content: &[u8]) -> Option<Lang>`

Detects programming language from Emacs/Vim modelines in file content.

**Features:**
- Searches first and last 5 lines for modelines
- Supports Emacs `-*- mode: lang -*-` syntax
- Supports Vim `vim: ft=lang` syntax
- Case-insensitive matching
- Maps editor modes to Cortex `Lang` enum

**Example:**
```rust
use cortex_code_analysis::{guess_language_from_content, Lang};

let content = b"// -*- mode: rust -*-\nfn main() {}";
assert_eq!(guess_language_from_content(content), Some(Lang::Rust));

let content = b"# vim: ft=python\nprint('hello')";
assert_eq!(guess_language_from_content(content), Some(Lang::Python));
```

**Supported Mode Mappings:**
- `rust`, `rs` → `Lang::Rust`
- `python`, `py` → `Lang::Python`
- `typescript`, `ts` → `Lang::TypeScript`
- `javascript`, `js` → `Lang::JavaScript`
- `c++`, `cpp`, `cxx`, `c` → `Lang::Cpp`
- `objective-c++`, `objc++` → `Lang::Cpp`
- And more...

### 3. `normalize_path<P: AsRef<Path>>(path: P) -> PathBuf`

Normalizes paths by resolving `.` and `..` components.

**Features:**
- Resolves `.` (current directory) by skipping
- Resolves `..` (parent directory) by popping from path
- Preserves Windows drive prefixes
- Cross-platform compatible

**Example:**
```rust
use cortex_code_analysis::normalize_path;
use std::path::Path;

let path = Path::new("foo/./bar/../baz");
let normalized = normalize_path(path);
assert_eq!(normalized, Path::new("foo/baz"));

let path = Path::new("foo/bar/baz/../../qux");
let normalized = normalize_path(path);
assert_eq!(normalized, Path::new("foo/qux"));
```

### 4. `get_paths_dist(path1: &Path, path2: &Path) -> Option<usize>`

Calculates the distance between two paths based on their closest common ancestor.

**Features:**
- Finds closest common ancestor
- Returns sum of component counts from ancestor to each path
- Useful for determining "closest" import/include paths
- Returns `None` if paths have no common ancestor

**Example:**
```rust
use cortex_code_analysis::get_paths_dist;
use std::path::Path;

let path1 = Path::new("/project/src/module/file.rs");
let path2 = Path::new("/project/src/lib.rs");

// Distance: 2 (module/file.rs to src) + 1 (src to lib.rs) = 3
assert_eq!(get_paths_dist(path1, path2), Some(3));
```

## Architecture Adaptations

The utilities were adapted from the experimental project to fit Cortex's architecture:

1. **Error Handling**: Changed from `std::io::Result` to `anyhow::Result`
2. **Language Enum**: Uses Cortex's `Lang` enum instead of experimental `LANG`
3. **Documentation**: Added comprehensive rustdoc with examples
4. **Tests**: Wrote 25 comprehensive unit tests using `tempfile` for file I/O tests
5. **Production Ready**: Removed experimental/placeholder code, added proper error context

## Test Coverage

All utilities have comprehensive test coverage:

```
running 25 tests
test utils::tests::test_read_file_with_bom_utf8 ... ok
test utils::tests::test_read_file_with_bom_utf16_be ... ok
test utils::tests::test_read_file_with_bom_utf16_le ... ok
test utils::tests::test_read_file_with_bom_no_bom ... ok
test utils::tests::test_read_file_with_bom_too_small ... ok
test utils::tests::test_read_file_with_bom_trailing_newlines ... ok
test utils::tests::test_guess_language_emacs_mode_rust ... ok
test utils::tests::test_guess_language_emacs_mode_python ... ok
test utils::tests::test_guess_language_emacs_mode_typescript ... ok
test utils::tests::test_guess_language_emacs_short_form ... ok
test utils::tests::test_guess_language_vim_modeline ... ok
test utils::tests::test_guess_language_vim_modeline_at_end ... ok
test utils::tests::test_guess_language_cpp_variants ... ok
test utils::tests::test_guess_language_no_modeline ... ok
test utils::tests::test_normalize_path_simple ... ok
test utils::tests::test_normalize_path_current_dir ... ok
test utils::tests::test_normalize_path_parent_dir ... ok
test utils::tests::test_normalize_path_complex ... ok
test utils::tests::test_normalize_path_multiple_parent ... ok
test utils::tests::test_get_paths_dist_same_directory ... ok
test utils::tests::test_get_paths_dist_parent_child ... ok
test utils::tests::test_get_paths_dist_siblings ... ok
test utils::tests::test_get_paths_dist_no_common_ancestor ... ok
test utils::tests::test_remove_blank_lines ... ok
test utils::tests::test_mode_to_lang_mappings ... ok

test result: ok. 25 passed; 0 failed; 0 ignored
```

## Use Cases

### File Reading with BOM Handling

When reading source files that might have BOMs (common in files edited on Windows or with certain editors):

```rust
use cortex_code_analysis::{read_file_with_bom, Lang};
use std::path::Path;

fn process_file(path: &Path) -> anyhow::Result<()> {
    if let Some(content) = read_file_with_bom(path)? {
        // Content is guaranteed to be valid UTF-8 with BOM stripped
        let source = std::str::from_utf8(&content)?;
        // Process the source code...
    }
    Ok(())
}
```

### Enhanced Language Detection

Combine file extension and modeline detection for more accurate language detection:

```rust
use cortex_code_analysis::{Lang, guess_language_from_content};
use std::path::Path;

fn detect_language(path: &Path, content: &[u8]) -> Option<Lang> {
    // Try extension first
    if let Some(lang) = Lang::from_path(path) {
        // Verify with modeline if available
        if let Some(modeline_lang) = guess_language_from_content(content) {
            // Modeline takes precedence (e.g., .h file with C++ modeline)
            return Some(modeline_lang);
        }
        return Some(lang);
    }

    // Fall back to modeline detection
    guess_language_from_content(content)
}
```

### Dependency Resolution

Use path distance to find the closest matching include/import:

```rust
use cortex_code_analysis::get_paths_dist;
use std::path::Path;

fn find_closest_match<'a>(
    current_file: &Path,
    candidates: &'a [&Path],
) -> Option<&'a Path> {
    candidates
        .iter()
        .filter_map(|&candidate| {
            get_paths_dist(current_file, candidate)
                .map(|dist| (candidate, dist))
        })
        .min_by_key(|(_, dist)| *dist)
        .map(|(path, _)| path)
}
```

## Differences from Experimental Code

### Removed Functions
- `read_file()` - Simplified, basic version not needed
- `write_file()` - Not relevant for code analysis
- `get_language_for_file()` - Cortex uses `Lang::from_path()`
- `guess_language()` - Split into `Lang::from_path()` + `guess_language_from_content()`
- `guess_file()` - Complex C++ include resolution, not needed for general use
- Color functions - Terminal output not needed in library
- Test helpers - Cortex has its own test infrastructure

### Modified Functions
- `read_file_with_eol()` → `read_file_with_bom()`: Better name, improved docs
- `get_emacs_mode()` → Internal helper for `guess_language_from_content()`
- Language mapping now uses Cortex's `Lang` enum

## Dependencies

The utils module uses:
- `anyhow` - Error handling
- `regex` - Modeline parsing
- `std::sync::OnceLock` - Cached regex compilation

Test dependencies:
- `tempfile` - Temporary file creation for tests

## Future Enhancements

Potential improvements for future iterations:

1. **Async File I/O**: Add async version of `read_file_with_bom()`
2. **Encoding Detection**: Extend BOM detection to handle more encodings
3. **Path Resolution**: Add more sophisticated path resolution for complex project structures
4. **Caching**: Add optional caching for language detection results

## Integration Checklist

- [x] Created `utils.rs` module with selected utilities
- [x] Added module to `lib.rs` exports
- [x] Adapted to use `Lang` enum
- [x] Adapted to use `anyhow::Result`
- [x] Added comprehensive documentation
- [x] Wrote 25 unit tests
- [x] All tests passing
- [x] Verified public API exports
- [x] Created integration documentation
