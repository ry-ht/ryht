use cortex_code_analysis::{remove_comments, Lang};

fn main() {
    // Test 1: Rust doc comments
    let rust_code = r#"/// This is a doc comment
/// It should be preserved
pub fn documented() {}

//! Module-level doc comment

// Regular comment to remove
fn main() {}
"#;

    println!("=== Test 1: Rust Doc Comments ===");
    println!("Original:\n{}\n", rust_code);

    match remove_comments(rust_code, Lang::Rust) {
        Ok(result) => {
            println!("Result:\n{}\n", result);

            // Verify doc comments are preserved
            if result.contains("This is a doc comment") {
                println!("✓ Doc comments preserved");
            } else {
                println!("✗ Doc comments NOT preserved");
            }

            // Verify regular comments are removed
            if !result.contains("Regular comment to remove") {
                println!("✓ Regular comments removed");
            } else {
                println!("✗ Regular comments NOT removed");
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }

    // Test 2: TypeScript JSDoc
    let ts_code = r#"/**
 * This is JSDoc
 * @param x The parameter
 */
function documented(x: number) {
    // This should be removed
    return x;
}
"#;

    println!("\n=== Test 2: TypeScript JSDoc ===");
    println!("Original:\n{}\n", ts_code);

    match remove_comments(ts_code, Lang::TypeScript) {
        Ok(result) => {
            println!("Result:\n{}\n", result);

            if result.contains("This is JSDoc") {
                println!("✓ JSDoc preserved");
            } else {
                println!("✗ JSDoc NOT preserved");
            }

            if !result.contains("This should be removed") {
                println!("✓ Regular comments removed");
            } else {
                println!("✗ Regular comments NOT removed");
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }

    // Test 3: Python encoding
    let py_code = r#"# -*- coding: utf-8 -*-
# This should be removed
def test():
    pass
"#;

    println!("\n=== Test 3: Python Encoding ===");
    println!("Original:\n{}\n", py_code);

    match remove_comments(py_code, Lang::Python) {
        Ok(result) => {
            println!("Result:\n{}\n", result);

            if result.contains("coding: utf-8") {
                println!("✓ Encoding comment preserved");
            } else {
                println!("✗ Encoding comment NOT preserved");
            }

            if !result.contains("This should be removed") {
                println!("✓ Regular comments removed");
            } else {
                println!("✗ Regular comments NOT removed");
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
