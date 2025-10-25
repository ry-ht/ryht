# Function Detection Module

This document describes the function detection module that has been integrated into `cortex-code-analysis` from the `experiments/adv-rust-code-analysis` crate.

## Overview

The function detection module provides a clean, production-ready API for detecting functions in source code across multiple programming languages. It identifies function boundaries (start and end lines) and extracts function names.

## Features

- **Multi-language support**: Rust, TypeScript, JavaScript, Python, Java, and C++
- **Accurate boundary detection**: Identifies exact start and end lines for each function
- **Named and anonymous functions**: Handles both named functions and anonymous functions
- **Nested functions**: Correctly detects functions nested within other functions
- **Serializable output**: Results are serializable with `serde` for easy integration
- **Production-ready**: Clean API with comprehensive error handling and documentation

## API

### Main Function

```rust
pub fn detect_functions(code: &str, lang: Lang) -> Result<Vec<FunctionSpan>>
```

Detects all functions in the given code for the specified language.

### Types

#### `FunctionSpan`

Represents a detected function with its boundaries:

```rust
pub struct FunctionSpan {
    /// The name of the function (or "<anonymous>" for anonymous functions)
    pub name: String,

    /// The first line of the function (1-indexed)
    pub start_line: usize,

    /// The last line of the function (1-indexed)
    pub end_line: usize,
}
```

#### Helper Methods

- `line_count()` - Returns the number of lines in the function
- `contains_line(line: usize)` - Checks if a line number is within the function span

## Usage Examples

### Basic Usage

```rust
use cortex_code_analysis::{detect_functions, Lang};

fn main() -> anyhow::Result<()> {
    let code = r#"
fn hello() {
    println!("Hello, world!");
}

fn goodbye() {
    println!("Goodbye!");
}
"#;

    let functions = detect_functions(code, Lang::Rust)?;

    for func in functions {
        println!("{}: lines {}-{}", func.name, func.start_line, func.end_line);
    }

    Ok(())
}
```

### Multi-Language Detection

```rust
use cortex_code_analysis::{detect_functions, Lang};

// Rust
let rust_functions = detect_functions("fn add(a: i32, b: i32) -> i32 { a + b }", Lang::Rust)?;

// Python
let python_functions = detect_functions("def add(a, b):\n    return a + b", Lang::Python)?;

// TypeScript
let ts_functions = detect_functions("function add(a: number, b: number): number { return a + b; }", Lang::TypeScript)?;

// JavaScript
let js_functions = detect_functions("const add = (a, b) => a + b;", Lang::JavaScript)?;
```

### Working with Results

```rust
use cortex_code_analysis::{detect_functions, Lang};

let code = r#"
fn process_data() {
    // ... 100 lines of code
}
"#;

let functions = detect_functions(code, Lang::Rust)?;
let func = &functions[0];

// Get function information
println!("Function: {}", func.name);
println!("Lines: {}-{}", func.start_line, func.end_line);
println!("Size: {} lines", func.line_count());

// Check if a specific line is in this function
if func.contains_line(50) {
    println!("Line 50 is in this function");
}

// Serialize to JSON
let json = serde_json::to_string(&func)?;
println!("JSON: {}", json);
```

## Language Support

### Rust
- Function items (`fn`)
- Nested functions
- Async functions
- Methods in `impl` blocks
- Trait methods

### TypeScript / JavaScript
- Function declarations
- Arrow functions (when assigned to variables)
- Method definitions
- Async functions
- Generator functions

### Python
- Function definitions (`def`)
- Methods in classes
- Async functions

### Java
- Method declarations
- Constructor declarations

### C++
- Function definitions
- Member functions
- Template functions

## Implementation Details

### Architecture

The module is built on top of the cortex parser infrastructure:

1. **Generic Parser**: Uses `Parser<T: LanguageInfo>` for language-agnostic parsing
2. **Tree-sitter Integration**: Leverages tree-sitter for accurate AST parsing
3. **Language-Specific Logic**: Each language has custom logic for identifying functions
4. **Clean API**: Exposes a simple, high-level API while hiding implementation complexity

### Function Detection Algorithm

1. Parse the source code into an AST
2. Traverse the AST tree depth-first
3. For each node, check if it represents a function (language-specific)
4. Extract the function name from the AST node
5. Record start and end line numbers (converted to 1-indexed)
6. Return all detected functions

### Differences from Experimental Version

The integrated version has several improvements over the experimental version:

1. **No terminal output code**: Removed all coloring and terminal output
2. **Modern error handling**: Uses `anyhow::Result` consistently
3. **Simplified API**: Single `detect_functions()` function instead of traits
4. **Better type safety**: Uses cortex's `Lang` enum instead of type parameters
5. **Production documentation**: Comprehensive docs and examples
6. **Serialization support**: Added `Serialize` and `Deserialize` derives
7. **Helper methods**: Added `line_count()` and `contains_line()` utilities

## Testing

The module includes comprehensive tests covering:

- Basic function detection for all languages
- Nested functions
- Anonymous functions
- Empty code handling
- Multi-language compatibility
- Serialization/deserialization
- Edge cases

Run tests with:

```bash
cargo test --lib function
```

Run the example:

```bash
cargo run --example function_detection
```

## Integration with Cortex

The function detection module is fully integrated into the cortex ecosystem:

- Exported from `cortex-code-analysis` crate
- Uses cortex's `Lang`, `Node`, and `Parser` types
- Follows cortex coding standards and conventions
- Compatible with all cortex language parsers

## Future Enhancements

Potential improvements for future versions:

1. **Function signatures**: Extract full function signatures including parameters and return types
2. **Visibility modifiers**: Detect public/private/protected modifiers
3. **Decorators/Attributes**: Extract decorators (Python) and attributes (Rust)
4. **Lambda/Closure detection**: Option to include or exclude closures
5. **Performance optimization**: Caching and parallel processing for large codebases
6. **Additional languages**: Support for Go, Ruby, PHP, etc.

## Migration from Experimental Version

If you were using the experimental version, here's how to migrate:

### Before (Experimental)
```rust
use adv_rust_code_analysis::{function, FunctionSpan, ParserTrait};

let parser = RustParser::new(code, path)?;
let spans = function(&parser);
```

### After (Cortex)
```rust
use cortex_code_analysis::{detect_functions, Lang};

let spans = detect_functions(code, Lang::Rust)?;
```

The new API is simpler and doesn't require creating parser instances manually.

## Credits

This module is based on the function detection implementation from the `adv-rust-code-analysis` experimental crate, adapted and enhanced for production use in the cortex ecosystem.
