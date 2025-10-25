//! Example demonstrating the use of ParserTrait and language abstractions.
//!
//! This example shows how to use the new trait-based parser architecture
//! to parse code in different languages using a unified interface.

use cortex_code_analysis::{
    Parser, ParserTrait, Lang, LanguageInfo,
    RustLanguage, TypeScriptLanguage, JavaScriptLanguage, PythonLanguage,
};
use std::path::Path;

fn main() -> anyhow::Result<()> {
    println!("=== Parser Trait Usage Example ===\n");

    // Example 1: Parse Rust code
    example_rust()?;

    // Example 2: Parse TypeScript code
    example_typescript()?;

    // Example 3: Parse JavaScript code
    example_javascript()?;

    // Example 4: Parse Python code
    example_python()?;

    // Example 5: Generic parsing function
    example_generic_parsing()?;

    Ok(())
}

fn example_rust() -> anyhow::Result<()> {
    println!("--- Example 1: Rust Parser ---");

    let code = r#"
        /// A simple function
        pub fn greet(name: &str) -> String {
            format!("Hello, {}!", name)
        }

        pub struct User {
            pub name: String,
            pub age: u32,
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        code.as_bytes().to_vec(),
        Path::new("example.rs")
    )?;

    println!("Language: {}", parser.get_language().display_name());
    println!("Root kind: {}", parser.get_root().kind());
    println!("Code length: {} bytes", parser.get_code().len());
    println!();

    Ok(())
}

fn example_typescript() -> anyhow::Result<()> {
    println!("--- Example 2: TypeScript Parser ---");

    let code = r#"
        interface User {
            name: string;
            age: number;
        }

        function greet(user: User): string {
            return `Hello, ${user.name}!`;
        }
    "#;

    let parser = Parser::<TypeScriptLanguage>::new(
        code.as_bytes().to_vec(),
        Path::new("example.ts")
    )?;

    println!("Language: {}", parser.get_language().display_name());
    println!("Root kind: {}", parser.get_root().kind());
    println!();

    Ok(())
}

fn example_javascript() -> anyhow::Result<()> {
    println!("--- Example 3: JavaScript Parser ---");

    let code = r#"
        function greet(name) {
            return `Hello, ${name}!`;
        }

        class User {
            constructor(name, age) {
                this.name = name;
                this.age = age;
            }
        }
    "#;

    let parser = Parser::<JavaScriptLanguage>::new(
        code.as_bytes().to_vec(),
        Path::new("example.js")
    )?;

    println!("Language: {}", parser.get_language().display_name());
    println!("Root kind: {}", parser.get_root().kind());
    println!();

    Ok(())
}

fn example_python() -> anyhow::Result<()> {
    println!("--- Example 4: Python Parser ---");

    let code = r#"
def greet(name: str) -> str:
    return f"Hello, {name}!"

class User:
    def __init__(self, name: str, age: int):
        self.name = name
        self.age = age
    "#;

    let parser = Parser::<PythonLanguage>::new(
        code.as_bytes().to_vec(),
        Path::new("example.py")
    )?;

    println!("Language: {}", parser.get_language().display_name());
    println!("Root kind: {}", parser.get_root().kind());
    println!();

    Ok(())
}

fn example_generic_parsing() -> anyhow::Result<()> {
    println!("--- Example 5: Generic Parsing Function ---");

    // Demonstrate parsing with a generic function
    parse_with_trait::<RustLanguage>("fn main() {}", "rust_code.rs")?;
    parse_with_trait::<TypeScriptLanguage>("const x = 42;", "ts_code.ts")?;

    println!();
    Ok(())
}

/// Generic function that works with any language implementing ParserTrait
fn parse_with_trait<T: LanguageInfo>(code: &str, filename: &str) -> anyhow::Result<()> {
    let parser = Parser::<T>::new(
        code.as_bytes().to_vec(),
        Path::new(filename)
    )?;

    println!("Parsed {} ({}) - {} nodes",
        filename,
        parser.get_language().display_name(),
        parser.get_root().child_count()
    );

    Ok(())
}
