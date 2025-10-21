//! Example showing detailed function parsing

use cortex_parser::RustParser;

fn main() -> anyhow::Result<()> {
    let source = r#"
/// Calculates the factorial of a number.
///
/// # Arguments
/// * `n` - The number to calculate factorial for
///
/// # Returns
/// The factorial of n
///
/// # Examples
/// ```
/// let result = factorial(5);
/// assert_eq!(result, 120);
/// ```
#[inline]
#[must_use]
pub fn factorial(n: u64) -> u64 {
    if n <= 1 {
        1
    } else {
        n * factorial(n - 1)
    }
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("example.rs", source)?;

    if let Some(func) = parsed.functions.first() {
        println!("=== COMPLETE FUNCTION DETAILS ===\n");
        println!("Name: {}", func.name);
        println!("Qualified Name: {}", func.qualified_name);
        println!("Visibility: {}", func.visibility);
        println!("Start Line: {}", func.start_line);
        println!("End Line: {}", func.end_line);
        println!("Is Async: {}", func.is_async);
        println!("Is Const: {}", func.is_const);
        println!("Is Unsafe: {}", func.is_unsafe);

        println!("\n--- Parameters ---");
        for (i, param) in func.parameters.iter().enumerate() {
            println!("Parameter {}:", i + 1);
            println!("  Name: {}", param.name);
            println!("  Type: {}", param.param_type);
            println!("  Is Self: {}", param.is_self);
            println!("  Is Mutable: {}", param.is_mut);
            println!("  Is Reference: {}", param.is_reference);
            if let Some(default) = &param.default_value {
                println!("  Default: {}", default);
            }
        }

        println!("\n--- Return Type ---");
        if let Some(ret_type) = &func.return_type {
            println!("Returns: {}", ret_type);
        } else {
            println!("Returns: () (unit)");
        }

        println!("\n--- Attributes ---");
        for attr in &func.attributes {
            println!("  {}", attr);
        }

        println!("\n--- Documentation ---");
        if let Some(doc) = &func.docstring {
            println!("{}", doc);
        } else {
            println!("(no documentation)");
        }

        println!("\n--- Generics ---");
        if !func.generics.is_empty() {
            for generic in &func.generics {
                println!("  {}", generic);
            }
        } else {
            println!("(no generic parameters)");
        }

        println!("\n--- Where Clause ---");
        if let Some(where_clause) = &func.where_clause {
            println!("{}", where_clause);
        } else {
            println!("(no where clause)");
        }

        println!("\n--- Complexity ---");
        if let Some(complexity) = func.complexity {
            println!("Cyclomatic Complexity: {}", complexity);
        } else {
            println!("(complexity not calculated)");
        }

        println!("\n--- Function Body ---");
        println!("{}", func.body);
    }

    Ok(())
}
