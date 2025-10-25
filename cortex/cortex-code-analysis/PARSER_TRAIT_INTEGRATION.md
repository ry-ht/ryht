# ParserTrait Pattern Integration

This document describes the integration of the advanced ParserTrait pattern and language abstractions from `experiments/adv-rust-code-analysis` into `cortex-code-analysis`.

## Overview

The integration introduces a powerful abstraction layer that enables:

1. **Unified Language Interface**: Common interface for parsing multiple programming languages
2. **Type-Safe Language Handling**: Compile-time guarantees for language-specific operations
3. **Extensible Architecture**: Easy addition of new languages and operations
4. **Callback Pattern**: Flexible operation execution on parsers

## Architecture

### Core Components

#### 1. Lang Enum (`src/lang.rs`)

The `Lang` enum represents all supported programming languages:

```rust
pub enum Lang {
    Rust,
    TypeScript,
    Tsx,
    JavaScript,
    Jsx,
    Python,
    Java,
    Kotlin,
    Cpp,
}
```

**Features**:
- File extension detection: `Lang::from_path()`, `Lang::from_extension()`
- Tree-sitter language mapping: `get_ts_language()`
- Language metadata: `get_name()`, `display_name()`, `extensions()`
- Feature queries: `supports_generics()`, `is_statically_typed()`

#### 2. Node Wrapper (`src/node.rs`)

Provides an ergonomic wrapper around tree-sitter's `Node`:

```rust
pub struct Node<'a>(TSNode<'a>);
```

**Features**:
- Convenient tree traversal methods
- Child/sibling navigation
- Text extraction
- Position information
- Search capabilities

#### 3. ParserTrait (`src/traits.rs`)

The core trait defining the parser interface:

```rust
pub trait ParserTrait: Sized {
    fn new(code: Vec<u8>, path: &Path) -> anyhow::Result<Self>;
    fn get_language(&self) -> Lang;
    fn get_root(&self) -> Node;
    fn get_code(&self) -> &[u8];
    fn get_text(&self, node: &TSNode) -> Option<&str>;
}
```

#### 4. LanguageInfo Trait

Provides static language metadata:

```rust
pub trait LanguageInfo {
    fn get_lang() -> Lang;
    fn get_lang_name() -> &'static str;
}
```

#### 5. Generic Parser (`src/parser.rs`)

A generic parser implementation that works with any `LanguageInfo`:

```rust
pub struct Parser<T: LanguageInfo> {
    code: Vec<u8>,
    tree: tree_sitter::Tree,
    _phantom: PhantomData<T>,
}
```

#### 6. Language Implementations (`src/languages/`)

Concrete language types implementing `LanguageInfo`:

- `RustLanguage`
- `TypeScriptLanguage` / `TsxLanguage`
- `JavaScriptLanguage` / `JsxLanguage`
- `PythonLanguage`

### Callback Pattern

The `Callback` trait enables extensible operations:

```rust
pub trait Callback {
    type Res;
    type Cfg;
    fn call<T: ParserTrait>(cfg: Self::Cfg, parser: &T) -> Self::Res;
}
```

This allows defining operations that can work with any parser implementing `ParserTrait`.

## Usage Examples

### Basic Usage

```rust
use cortex_code_analysis::{Parser, RustLanguage, ParserTrait};
use std::path::Path;

// Parse Rust code
let code = b"fn main() {}".to_vec();
let parser = Parser::<RustLanguage>::new(code, Path::new("main.rs"))?;

println!("Language: {}", parser.get_language().display_name());
println!("Root: {}", parser.get_root().kind());
```

### Generic Parsing

```rust
fn parse_any<T: LanguageInfo>(code: &str, path: &str) -> anyhow::Result<()> {
    let parser = Parser::<T>::new(code.as_bytes().to_vec(), Path::new(path))?;

    let root = parser.get_root();
    for child in root.children() {
        println!("Node: {}", child.kind());
    }

    Ok(())
}

// Use with different languages
parse_any::<RustLanguage>("fn test() {}", "test.rs")?;
parse_any::<TypeScriptLanguage>("const x = 1;", "test.ts")?;
```

### Language Detection

```rust
use cortex_code_analysis::Lang;
use std::path::Path;

let lang = Lang::from_path(Path::new("example.rs"));
assert_eq!(lang, Some(Lang::Rust));

let lang = Lang::from_extension("ts");
assert_eq!(lang, Some(Lang::TypeScript));
```

## Integration with Existing Code

### Backward Compatibility

The existing `RustParser` and `TypeScriptParser` continue to work as before. The new abstractions are additive:

```rust
// Old API (still works)
let mut parser = RustParser::new()?;
let parsed = parser.parse_file("test.rs", source)?;

// New API (trait-based)
let parser = Parser::<RustLanguage>::new(code, Path::new("test.rs"))?;
let root = parser.get_root();
```

### CodeParser Updates

The `CodeParser` now uses `Lang` instead of the old `Language` enum:

```rust
let mut parser = CodeParser::for_language(Lang::Rust)?;
let parsed = parser.parse_rust("test.rs", source)?;
```

## Benefits

### 1. Type Safety

The trait-based approach provides compile-time guarantees:

```rust
// Type checker ensures correct language
let rust_parser = Parser::<RustLanguage>::new(code, path)?;
assert_eq!(rust_parser.get_language(), Lang::Rust);
```

### 2. Extensibility

Adding a new language requires:

1. Add to `Lang` enum
2. Create a type implementing `LanguageInfo`
3. Implement language-specific token enum

```rust
pub struct GoLanguage;

impl LanguageInfo for GoLanguage {
    fn get_lang() -> Lang { Lang::Go }
    fn get_lang_name() -> &'static str { "go" }
}
```

### 3. Generic Operations

Write operations that work with any language:

```rust
fn count_nodes<T: LanguageInfo>(code: &str) -> usize {
    let parser = Parser::<T>::new(code.as_bytes().to_vec(), Path::new("file"))?;
    count_recursive(&parser.get_root())
}
```

### 4. Language Abstraction

The abstraction enables building tools that work across languages:

```rust
trait CodeAnalyzer {
    fn analyze<T: ParserTrait>(&self, parser: &T) -> AnalysisResult;
}
```

## Migration Guide

### For Library Users

Most existing code continues to work unchanged. To use the new abstractions:

```rust
// Before
use cortex_code_analysis::{Language, CodeParser};
let mut parser = CodeParser::for_language(Language::Rust)?;

// After
use cortex_code_analysis::{Lang, CodeParser};
let mut parser = CodeParser::for_language(Lang::Rust)?;
```

### For Library Developers

To implement language-specific features:

1. Use `ParserTrait` for operations that work across languages
2. Use `Parser<T: LanguageInfo>` for generic implementations
3. Use the `Callback` pattern for extensible operations

## Future Enhancements

### Planned Additions

1. **Language-Specific Checkers**: Implement checker traits for syntax validation
2. **Metric Traits**: Add traits for code metrics (complexity, LOC, etc.)
3. **Transformation Operations**: AST manipulation operations via callbacks
4. **Query System**: Tree-sitter query integration with type safety

### Pattern Extensions

The architecture supports additional patterns:

- **Getter Trait**: Extract specific information from ASTs
- **Alterator Trait**: Modify AST structures
- **Cognitive/Cyclomatic**: Code complexity metrics
- **Halstead Metrics**: Volume, difficulty, effort calculations

## Files Added/Modified

### New Files

- `src/lang.rs` - Language enumeration and metadata
- `src/node.rs` - Node abstraction layer
- `src/traits.rs` - Core trait definitions
- `src/parser.rs` - Generic parser implementation
- `src/languages/mod.rs` - Language module registry
- `src/languages/rust.rs` - Rust language implementation
- `src/languages/typescript.rs` - TypeScript/TSX implementations
- `src/languages/javascript.rs` - JavaScript/JSX implementations
- `src/languages/python.rs` - Python implementation
- `examples/parser_trait_usage.rs` - Usage examples

### Modified Files

- `src/lib.rs` - Added new module exports and updated API
- `Cargo.toml` - Added dependencies (num, num-derive, tree-sitter-mozcpp, tree-sitter-kotlin-ng)

## Dependencies

### New Dependencies

```toml
num = "0.4"
num-derive = "0.4"
tree-sitter-mozcpp = "0.25.0"
tree-sitter-kotlin-ng = "0.0.2"
```

These enable:
- `num` / `num-derive`: Token enum conversions
- `tree-sitter-mozcpp`: Enhanced C/C++ parsing
- `tree-sitter-kotlin-ng`: Kotlin language support

## Testing

Run the example:

```bash
cargo run --example parser_trait_usage
```

Run tests:

```bash
cargo test
```

## References

- Source: `experiments/adv-rust-code-analysis/`
- Tree-sitter documentation: https://tree-sitter.github.io/
- Rust trait documentation: https://doc.rust-lang.org/book/ch10-02-traits.html
