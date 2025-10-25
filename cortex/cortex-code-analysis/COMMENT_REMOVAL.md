# Comment Removal Module

## Overview

The comment removal module provides production-ready functionality to remove comments from source code while preserving line numbers and useful comments like documentation and pragmas.

## Location

- **Source**: `/Users/taaliman/projects/luxquant/ry-ht/ryht/cortex/cortex-code-analysis/src/comment_removal.rs`
- **Exported from**: `cortex-code-analysis` crate

## Features

### Core Functionality

1. **Comment Detection**: Automatically detects comments in all supported languages using tree-sitter AST parsing
2. **Line Preservation**: Replaces removed comments with newlines to maintain original line numbers
3. **Smart Preservation**: Keeps useful comments such as:
   - Rust: Doc comments (`///`, `//!`, `/**`, `/*!`), cbindgen pragmas
   - Python: Encoding declarations (`# coding:`, `# -*- coding:`)
   - TypeScript/JavaScript: JSDoc (`/**`), type directives (`@ts-`, `@type`), linter directives
   - Java/Kotlin: JavaDoc/KDoc (`/**`)
   - C/C++: Special pragmas (rustbindgen)

### Supported Languages

- Rust
- Python
- TypeScript / TSX
- JavaScript / JSX
- Java
- Kotlin
- C/C++

## API

### Main Functions

```rust
pub fn remove_comments(code: &str, lang: Lang) -> Result<String>
```
Removes comments from source code while preserving line numbers and useful comments.

**Parameters**:
- `code`: The source code as a string
- `lang`: The programming language (from `Lang` enum)

**Returns**: `Result<String>` containing the code with comments removed

**Example**:
```rust
use cortex_code_analysis::{remove_comments, Lang};

let rust_code = r#"
// This is a comment
fn main() {
    println!("Hello"); // inline comment
}
"#;

let cleaned = remove_comments(rust_code, Lang::Rust)?;
```

---

```rust
pub fn extract_comments(code: &str, lang: Lang) -> Result<Vec<CommentSpan>>
```
Extracts all comment spans from source code without removing them.

**Parameters**:
- `code`: The source code as a string
- `lang`: The programming language

**Returns**: `Result<Vec<CommentSpan>>` containing all comment spans

---

### Types

```rust
pub struct CommentSpan {
    pub start: usize,  // Start byte offset
    pub end: usize,    // End byte offset
    pub lines: usize,  // Number of lines spanned
}
```

## Implementation Details

### Algorithm

1. Parse source code using tree-sitter for the specified language
2. Traverse the AST to identify all comment nodes
3. For each comment:
   - Check if it should be preserved (doc comment, pragma, etc.)
   - If not preserved, record its span (start, end, line count)
4. Process spans in reverse order to maintain byte offsets
5. Replace each comment span with equivalent newlines

### Special Handling

#### Rust Doc Comments

Tree-sitter-rust parses doc comment markers (`///`, `//!`) separately from the comment text. The module checks both the comment node and preceding text to correctly identify doc comments.

#### Python Encoding

Encoding declarations must appear in the first two lines of a Python file. The module preserves comments matching the encoding pattern in these positions.

#### JSDoc and Type Directives

TypeScript and JavaScript preserve block comments starting with `/**` and comments containing type system directives.

## Testing

The module includes comprehensive tests covering:

- Comment removal for all supported languages
- Doc comment preservation
- Pragma preservation
- Block and line comments
- Multi-line comments
- Edge cases (empty files, no comments)
- Line number preservation

**Run tests**:
```bash
cargo test comment_removal --lib
```

**Run example**:
```bash
cargo run --example test_comment_removal
```

## Performance

- Efficient single-pass AST traversal
- Pre-allocated newline buffer for fast replacement
- O(n) complexity where n is the number of AST nodes

## Integration

The module is fully integrated into the cortex-code-analysis crate:

- Exported in `lib.rs`
- Uses cortex's `Lang` enum for language detection
- Uses cortex's `Node` wrapper for tree-sitter integration
- Compatible with cortex's existing parser infrastructure

## Migration from Experiments

This module was migrated from `experiments/adv-rust-code-analysis` with the following improvements:

1. **Architecture**: Adapted to use cortex's `Lang` enum instead of custom parser traits
2. **API**: Simplified to functional API (`remove_comments`, `extract_comments`)
3. **Language Support**: Extended from Rust-only to all cortex-supported languages
4. **Documentation**: Added comprehensive API documentation and examples
5. **Testing**: Expanded test coverage for all languages
6. **Production Ready**: No placeholders, fully implemented functionality
