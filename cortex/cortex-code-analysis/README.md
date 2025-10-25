# cortex-code-analysis

Tree-sitter based code parsing infrastructure for the Cortex cognitive memory system.

## Overview

`cortex-code-analysis` provides high-level parsing capabilities for multiple programming languages using tree-sitter. It extracts structured information about code elements including functions, structs, enums, traits, and more.

This crate is the **foundation that unblocks 91 MCP tools** in the Cortex system by providing the parsing infrastructure needed for code analysis.

## Features

- **Rust Parsing**: Full support for parsing Rust source code
- **TypeScript/JavaScript Parsing**: Parse TypeScript and JavaScript files
- **Comprehensive Extraction**:
  - Functions with full signature details
  - Structs, enums, traits, and impl blocks
  - Documentation/docstrings
  - Visibility modifiers
  - Generics and lifetimes
  - Attributes/annotations
  - Cyclomatic complexity calculation
- **Multi-language Support**: Extensible architecture for adding more languages

## Usage

### Basic Rust Parsing

```rust
use cortex_code_analysis::RustParser;

let source = r#"
/// Adds two numbers together.
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;

let mut parser = RustParser::new()?;
let parsed = parser.parse_file("example.rs", source)?;

// Access parsed functions
for func in &parsed.functions {
    println!("Function: {} ({})", func.name, func.visibility);
    println!("  Parameters: {}", func.parameters.len());
    println!("  Returns: {:?}", func.return_type);
    if let Some(doc) = &func.docstring {
        println!("  Documentation: {}", doc);
    }
}
```

### Auto-detecting Language

```rust
use cortex_code_analysis::CodeParser;

let mut parser = CodeParser::new()?;

// Automatically detects language from file extension
let rust_result = parser.parse_file_auto("test.rs", rust_source)?;
let ts_result = parser.parse_file_auto("test.ts", ts_source)?;
```

### Language-Specific Parser

```rust
use cortex_code_analysis::{CodeParser, Language};

let mut parser = CodeParser::for_language(Language::Rust)?;
let result = parser.parse_rust("test.rs", source)?;
```

## Extracted Information

### Function Details

For each function, the parser extracts:

- **name**: Function name
- **qualified_name**: Fully qualified name (e.g., `module::Type::method`)
- **parameters**: List of parameters with:
  - name, type, default value
  - is_self, is_mut, is_reference flags
- **return_type**: Return type (if any)
- **visibility**: pub, pub(crate), private, etc.
- **attributes**: All attributes (e.g., #[test], #[inline])
- **body**: Function body text
- **start_line** / **end_line**: Source location
- **docstring**: Documentation comments
- **is_async** / **is_const** / **is_unsafe**: Function modifiers
- **generics**: Generic type parameters
- **where_clause**: Where clause constraints
- **complexity**: Cyclomatic complexity

### Struct Details

- name, qualified_name
- fields with name, type, visibility
- visibility, attributes, docstring
- generics, where clause
- is_tuple_struct, is_unit_struct flags

### Enum Details

- name, variants with fields
- visibility, attributes, docstring
- generics, where clause

### Trait Details

- name, methods, associated types
- visibility, attributes, docstring
- generics, where clause, supertraits

### Impl Blocks

- type_name, trait_name (if trait impl)
- methods, associated types
- generics, where clause

## Architecture

The crate is organized into several modules:

- **types.rs**: AST data structures (FunctionInfo, StructInfo, etc.)
- **tree_sitter_wrapper.rs**: Generic tree-sitter parser wrapper
- **extractor.rs**: Core extraction utilities
- **rust_parser.rs**: Rust-specific parsing logic
- **typescript_parser.rs**: TypeScript/JavaScript parsing logic
- **lib.rs**: Public API and language detection

## Test Coverage

The crate includes comprehensive tests:

- **18 unit tests** in lib/modules
- **26 Rust parsing tests** covering all language features
- **12 extraction tests** validating data accuracy
- **100% test pass rate**

Total: **57+ tests** covering all major functionality

## Examples

Run the examples to see the parser in action:

```bash
# Basic parsing example
cargo run --example parse_example

# Detailed function analysis
cargo run --example detailed_function
```

## Dependencies

- tree-sitter: 0.25.10
- tree-sitter-rust: 0.24.0
- tree-sitter-typescript: 0.23.2
- cortex-core: Core types and utilities

## Integration with Cortex

This crate provides the parsing foundation for:

- **Code analysis tools**: Understanding code structure
- **Symbol extraction**: Building code graphs
- **Documentation generation**: Extracting docs
- **Refactoring tools**: Understanding dependencies
- **MCP tools**: 91 tools blocked on this functionality

## Performance

- Fast incremental parsing with tree-sitter
- Efficient memory usage
- Suitable for large codebases

## Future Enhancements

Potential additions:

- Python parser
- Go parser
- Java parser
- More sophisticated complexity metrics
- Control flow graph generation
- Type inference

## License

MIT
