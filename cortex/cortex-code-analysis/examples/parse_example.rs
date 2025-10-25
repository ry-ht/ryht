//! Example demonstrating cortex-code-analysis capabilities

use cortex_code_analysis::{RustParser, FunctionInfo};

fn main() -> anyhow::Result<()> {
    // Example Rust code to parse
    let source = r#"
use std::collections::HashMap;

/// A simple cache implementation.
///
/// # Examples
/// ```
/// let cache = Cache::new();
/// cache.set("key", "value");
/// ```
#[derive(Debug, Clone)]
pub struct Cache {
    /// Internal storage
    pub data: HashMap<String, String>,

    /// Maximum size
    max_size: usize,
}

impl Cache {
    /// Create a new cache with default size.
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            max_size: 100,
        }
    }

    /// Set a value in the cache.
    pub fn set(&mut self, key: String, value: String) {
        if self.data.len() >= self.max_size {
            self.data.clear();
        }
        self.data.insert(key, value);
    }

    /// Get a value from the cache.
    pub fn get(&self, key: &str) -> Option<&String> {
        self.data.get(key)
    }
}
"#;

    // Parse the source code
    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("cache.rs", source)?;

    println!("=== Parsing Results ===\n");

    // Display imports
    println!("Imports: {}", parsed.imports.len());
    for import in &parsed.imports {
        println!("  - {}", import);
    }
    println!();

    // Display structs
    println!("Structs: {}", parsed.structs.len());
    for s in &parsed.structs {
        println!("  - {} (visibility: {}, fields: {})",
            s.name, s.visibility, s.fields.len());
        if let Some(doc) = &s.docstring {
            println!("    Doc: {}", doc.lines().next().unwrap_or(""));
        }
        for field in &s.fields {
            println!("    Field: {} : {} ({})",
                field.name, field.field_type, field.visibility);
        }
    }
    println!();

    // Display impl blocks
    println!("Impl Blocks: {}", parsed.impls.len());
    for impl_block in &parsed.impls {
        println!("  - impl {} (methods: {})",
            impl_block.type_name, impl_block.methods.len());
    }
    println!();

    // Display functions (including methods)
    let all_methods: Vec<&FunctionInfo> = parsed.impls.iter()
        .flat_map(|i| &i.methods)
        .collect();

    println!("Total Functions/Methods: {}",
        parsed.functions.len() + all_methods.len());

    for method in &all_methods {
        println!("\n  Function: {} ({})", method.name, method.visibility);
        println!("    Qualified: {}", method.qualified_name);
        println!("    Parameters: {}", method.parameters.len());
        for param in &method.parameters {
            println!("      - {} : {} (self: {}, mut: {}, ref: {})",
                param.name, param.param_type,
                param.is_self, param.is_mut, param.is_reference);
        }
        if let Some(ret) = &method.return_type {
            println!("    Returns: {}", ret);
        }
        if let Some(doc) = &method.docstring {
            println!("    Doc: {}", doc.lines().next().unwrap_or(""));
        }
        println!("    Lines: {} - {}", method.start_line, method.end_line);
        if let Some(complexity) = method.complexity {
            println!("    Complexity: {}", complexity);
        }
    }

    println!("\n=== Summary ===");
    println!("Total items parsed:");
    println!("  Imports: {}", parsed.imports.len());
    println!("  Structs: {}", parsed.structs.len());
    println!("  Enums: {}", parsed.enums.len());
    println!("  Traits: {}", parsed.traits.len());
    println!("  Impl blocks: {}", parsed.impls.len());
    println!("  Functions: {}", parsed.functions.len());
    println!("  Methods: {}", all_methods.len());

    Ok(())
}
