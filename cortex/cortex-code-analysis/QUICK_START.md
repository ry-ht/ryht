# Quick Start Guide: ParserTrait Integration

## 5-Minute Introduction

### Basic Usage

```rust
use cortex_code_analysis::{Parser, RustLanguage, ParserTrait};
use std::path::Path;

fn main() -> anyhow::Result<()> {
    // 1. Create a parser for Rust
    let code = b"fn main() { println!(\"Hello\"); }".to_vec();
    let parser = Parser::<RustLanguage>::new(code, Path::new("main.rs"))?;

    // 2. Get the root node
    let root = parser.get_root();
    println!("Root kind: {}", root.kind());  // "source_file"

    // 3. Traverse children
    for child in root.children() {
        println!("Child: {}", child.kind());
    }

    Ok(())
}
```

### Language Detection

```rust
use cortex_code_analysis::Lang;
use std::path::Path;

// From file path
let lang = Lang::from_path(Path::new("example.rs")).unwrap();
assert_eq!(lang, Lang::Rust);

// From extension
let lang = Lang::from_extension("ts").unwrap();
assert_eq!(lang, Lang::TypeScript);

// Get metadata
println!("{}", lang.display_name());  // "TypeScript"
println!("{:?}", lang.extensions());  // ["ts"]
```

### Generic Parsing

```rust
use cortex_code_analysis::{Parser, LanguageInfo, ParserTrait};
use std::path::Path;

// Function works with any language
fn count_nodes<T: LanguageInfo>(code: &str) -> usize {
    let parser = Parser::<T>::new(
        code.as_bytes().to_vec(),
        Path::new("file")
    ).unwrap();

    count_recursive(&parser.get_root())
}

fn count_recursive(node: &cortex_code_analysis::Node) -> usize {
    1 + node.children()
        .map(|child| count_recursive(&child))
        .sum::<usize>()
}

// Use with different languages
let rust_nodes = count_nodes::<RustLanguage>("fn main() {}");
let ts_nodes = count_nodes::<TypeScriptLanguage>("const x = 1;");
```

## Common Patterns

### Pattern 1: Parse and Analyze

```rust
use cortex_code_analysis::{Parser, RustLanguage, ParserTrait};

fn analyze_rust_file(source: &str) -> anyhow::Result<()> {
    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("input.rs")
    )?;

    let root = parser.get_root();

    // Find all function nodes
    for child in root.children() {
        if child.kind() == "function_item" {
            if let Some(name) = child.child_by_field_name("name") {
                let text = name.utf8_text(parser.get_code()).unwrap();
                println!("Found function: {}", text);
            }
        }
    }

    Ok(())
}
```

### Pattern 2: Language-Agnostic Tool

```rust
use cortex_code_analysis::{Parser, LanguageInfo, ParserTrait, Node};

trait CodeMetrics {
    fn complexity(&self) -> u32;
}

impl<'a> CodeMetrics for Node<'a> {
    fn complexity(&self) -> u32 {
        // Simple node count as complexity
        self.children()
            .map(|child| child.complexity())
            .sum::<u32>() + 1
    }
}

fn analyze_any_language<T: LanguageInfo>(source: &str) -> u32 {
    let parser = Parser::<T>::new(
        source.as_bytes().to_vec(),
        Path::new("file")
    ).unwrap();

    parser.get_root().complexity()
}
```

### Pattern 3: Multi-Language Project

```rust
use cortex_code_analysis::{Lang, Parser, RustLanguage, TypeScriptLanguage};
use std::path::Path;

fn analyze_project(files: Vec<(String, String)>) -> anyhow::Result<()> {
    for (path, content) in files {
        let path_obj = Path::new(&path);

        match Lang::from_path(path_obj) {
            Some(Lang::Rust) => {
                let parser = Parser::<RustLanguage>::new(
                    content.as_bytes().to_vec(),
                    path_obj
                )?;
                println!("Rust file: {} nodes", parser.get_root().child_count());
            }
            Some(Lang::TypeScript) => {
                let parser = Parser::<TypeScriptLanguage>::new(
                    content.as_bytes().to_vec(),
                    path_obj
                )?;
                println!("TS file: {} nodes", parser.get_root().child_count());
            }
            _ => println!("Unsupported language: {}", path),
        }
    }

    Ok(())
}
```

## Supported Languages

| Language   | Type              | Extensions           |
|------------|-------------------|----------------------|
| Rust       | `RustLanguage`    | `.rs`                |
| TypeScript | `TypeScriptLanguage` | `.ts`             |
| TSX        | `TsxLanguage`     | `.tsx`               |
| JavaScript | `JavaScriptLanguage` | `.js`, `.mjs`, `.cjs` |
| JSX        | `JsxLanguage`     | `.jsx`               |
| Python     | `PythonLanguage`  | `.py`                |
| Java       | (coming soon)     | `.java`              |
| Kotlin     | (coming soon)     | `.kt`, `.kts`        |
| C/C++      | (coming soon)     | `.c`, `.cpp`, `.h`   |

## Node Traversal Cheat Sheet

```rust
use cortex_code_analysis::Node;

fn traverse_examples(node: &Node) {
    // Get node info
    let kind = node.kind();                    // Node type as string
    let kind_id = node.kind_id();              // Node type as u16
    let has_error = node.has_error();          // Check for syntax errors

    // Position info
    let (start_row, start_col) = node.start_position();
    let (end_row, end_col) = node.end_position();

    // Children access
    let count = node.child_count();            // Number of children
    let first = node.child(0);                 // Child by index
    let name = node.child_by_field_name("name"); // Named field

    // Iteration
    for child in node.children() {
        // Process each child
    }

    // Navigation
    let parent = node.parent();                // Parent node
    let next = node.next_sibling();            // Next sibling
    let prev = node.previous_sibling();        // Previous sibling
}
```

## Integration with Existing Code

### Using the Old API (still works)

```rust
use cortex_code_analysis::RustParser;

let mut parser = RustParser::new()?;
let result = parser.parse_file("main.rs", source)?;
// Returns ParsedFile with extracted information
```

### Using the New API (more flexible)

```rust
use cortex_code_analysis::{Parser, RustLanguage, ParserTrait};

let parser = Parser::<RustLanguage>::new(code, path)?;
let root = parser.get_root();
// Returns Node for direct AST access
```

### Both Together

```rust
// High-level extraction
let mut rust_parser = RustParser::new()?;
let parsed = rust_parser.parse_file("lib.rs", source)?;
println!("Functions: {}", parsed.functions.len());

// Low-level AST access
let parser = Parser::<RustLanguage>::new(code, path)?;
for node in parser.get_root().children() {
    println!("Node: {}", node.kind());
}
```

## Common Tasks

### Task: Find all functions

```rust
fn find_functions(parser: &impl ParserTrait) -> Vec<String> {
    let root = parser.get_root();
    let mut functions = Vec::new();

    for child in root.children() {
        if child.kind() == "function_item" {
            if let Some(name_node) = child.child_by_field_name("name") {
                let name = name_node.utf8_text(parser.get_code()).unwrap();
                functions.push(name.to_string());
            }
        }
    }

    functions
}
```

### Task: Count lines of code

```rust
fn count_lines(parser: &impl ParserTrait) -> usize {
    let root = parser.get_root();
    let code = parser.get_code();

    code.iter().filter(|&&b| b == b'\n').count() + 1
}
```

### Task: Extract comments

```rust
fn extract_comments(parser: &impl ParserTrait) -> Vec<String> {
    let root = parser.get_root();
    let code = parser.get_code();
    let mut comments = Vec::new();

    for child in root.children() {
        if child.kind().contains("comment") {
            let text = child.utf8_text(code).unwrap();
            comments.push(text.to_string());
        }
    }

    comments
}
```

## Next Steps

1. **Read the full documentation**: `PARSER_TRAIT_INTEGRATION.md`
2. **Run the example**: `cargo run --example parser_trait_usage`
3. **Explore the API**: Check `src/traits.rs`, `src/lang.rs`, `src/node.rs`
4. **Build your tool**: Use the patterns above as starting points

## Getting Help

- Check the documentation: `PARSER_TRAIT_INTEGRATION.md`
- Run tests: `cargo test`
- Read examples: `examples/parser_trait_usage.rs`
- Consult tree-sitter docs: https://tree-sitter.github.io/

## Tips

1. **Use generics** for code that works with multiple languages
2. **Check `has_error()`** before processing nodes
3. **Use field names** (`child_by_field_name`) instead of indices when possible
4. **Cache the root node** if you'll use it multiple times
5. **Use `utf8_text()`** to get the text content of nodes

Happy parsing! 🎉
