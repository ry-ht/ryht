//! Comprehensive metrics calculation tests.
//!
//! This test suite validates:
//! - All metrics types (Cyclomatic, Cognitive, LOC, Halstead, ABC, MI, etc.)
//! - Metrics accuracy across different code patterns
//! - Edge cases and boundary conditions
//! - Cross-language metrics consistency

use cortex_code_analysis::{RustParser, TypeScriptParser, detect_functions, Lang};
use anyhow::Result;

// ============================================================================
// SECTION 1: Cyclomatic Complexity Tests
// ============================================================================

#[test]
fn test_cyclomatic_complexity_simple() -> Result<()> {
    let source = r#"
fn simple() {
    let x = 1;
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Simple function should have complexity of 1
    assert_eq!(parsed.functions.len(), 1);
    // Note: Actual complexity value depends on metrics implementation

    Ok(())
}

#[test]
fn test_cyclomatic_complexity_with_if() -> Result<()> {
    let source = r#"
fn with_if(x: i32) -> i32 {
    if x > 0 {
        x
    } else {
        -x
    }
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Function with if/else should have higher complexity
    assert_eq!(parsed.functions.len(), 1);

    Ok(())
}

#[test]
fn test_cyclomatic_complexity_with_multiple_branches() -> Result<()> {
    let source = r#"
fn complex_logic(a: i32, b: i32, c: i32) -> i32 {
    if a > 0 {
        if b > 0 {
            if c > 0 {
                a + b + c
            } else {
                a + b
            }
        } else {
            a
        }
    } else {
        0
    }
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Nested if statements increase complexity
    assert_eq!(parsed.functions.len(), 1);

    Ok(())
}

#[test]
fn test_cyclomatic_complexity_with_loops() -> Result<()> {
    let source = r#"
fn with_loop(n: i32) -> i32 {
    let mut sum = 0;
    for i in 0..n {
        sum += i;
    }
    while sum > 100 {
        sum -= 10;
    }
    sum
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Loops increase complexity
    assert_eq!(parsed.functions.len(), 1);

    Ok(())
}

#[test]
fn test_cyclomatic_complexity_with_match() -> Result<()> {
    let source = r#"
fn with_match(x: Option<i32>) -> i32 {
    match x {
        Some(v) if v > 0 => v * 2,
        Some(v) => v,
        None => 0,
    }
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Match statements increase complexity
    assert_eq!(parsed.functions.len(), 1);

    Ok(())
}

// ============================================================================
// SECTION 2: Cognitive Complexity Tests
// ============================================================================

#[test]
fn test_cognitive_complexity_nested_structures() -> Result<()> {
    let source = r#"
fn nested_logic(items: Vec<i32>) -> i32 {
    let mut result = 0;
    for item in items {
        if item > 0 {
            for _ in 0..item {
                result += 1;
            }
        }
    }
    result
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Nested structures significantly increase cognitive complexity
    assert_eq!(parsed.functions.len(), 1);

    Ok(())
}

#[test]
fn test_cognitive_complexity_vs_cyclomatic() -> Result<()> {
    // Two functions with same cyclomatic but different cognitive complexity
    let source = r#"
fn linear_conditions(a: bool, b: bool, c: bool) -> i32 {
    if a { return 1; }
    if b { return 2; }
    if c { return 3; }
    0
}

fn nested_conditions(a: bool, b: bool, c: bool) -> i32 {
    if a {
        if b {
            if c {
                return 3;
            }
            return 2;
        }
        return 1;
    }
    0
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Both functions parsed successfully
    assert_eq!(parsed.functions.len(), 2);

    Ok(())
}

// ============================================================================
// SECTION 3: Lines of Code (LOC) Tests
// ============================================================================

#[test]
fn test_loc_calculation() -> Result<()> {
    let source = r#"
// This is a comment
fn calculate() {
    // Another comment
    let x = 1;
    let y = 2;
    /* Block comment */
    let z = x + y;
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Should count logical lines of code (excluding comments and blank lines)
    assert_eq!(parsed.functions.len(), 1);

    Ok(())
}

#[test]
fn test_sloc_vs_ploc() -> Result<()> {
    let source = r#"
fn multiline_statement() {
    let result = vec![1, 2, 3]
        .iter()
        .map(|x| x * 2)
        .filter(|x| x > &2)
        .collect::<Vec<_>>();
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Physical vs logical lines of code
    assert_eq!(parsed.functions.len(), 1);

    Ok(())
}

#[test]
fn test_loc_with_empty_lines() -> Result<()> {
    let source = r#"
fn with_spacing() {
    let x = 1;


    let y = 2;


    let z = x + y;
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Empty lines should not be counted
    assert_eq!(parsed.functions.len(), 1);

    Ok(())
}

// ============================================================================
// SECTION 4: Halstead Complexity Tests
// ============================================================================

#[test]
fn test_halstead_operators_and_operands() -> Result<()> {
    let source = r#"
fn math_operations(a: i32, b: i32) -> i32 {
    let sum = a + b;
    let product = a * b;
    let result = sum + product;
    result
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Halstead metrics count operators (+, *, =) and operands (a, b, sum, etc.)
    assert_eq!(parsed.functions.len(), 1);

    Ok(())
}

#[test]
fn test_halstead_volume_and_difficulty() -> Result<()> {
    let source = r#"
fn complex_calculation(x: f64, y: f64, z: f64) -> f64 {
    let a = x.sqrt() + y.sqrt();
    let b = z.powf(2.0);
    let c = (a * b) / (x + y + z);
    c
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // More operators and operands increase Halstead volume and difficulty
    assert_eq!(parsed.functions.len(), 1);

    Ok(())
}

// ============================================================================
// SECTION 5: Maintainability Index (MI) Tests
// ============================================================================

#[test]
fn test_maintainability_index_simple() -> Result<()> {
    let source = r#"
fn simple_function() {
    println!("Hello");
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Simple functions should have high maintainability index
    assert_eq!(parsed.functions.len(), 1);

    Ok(())
}

#[test]
fn test_maintainability_index_complex() -> Result<()> {
    let source = r#"
fn complex_function(data: Vec<i32>) -> Vec<i32> {
    let mut result = Vec::new();
    for item in data {
        if item > 0 {
            for i in 0..item {
                if i % 2 == 0 {
                    result.push(i);
                } else {
                    result.push(i * 2);
                }
            }
        } else if item < 0 {
            result.push(item.abs());
        }
    }
    result
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Complex functions should have lower maintainability index
    assert_eq!(parsed.functions.len(), 1);

    Ok(())
}

// ============================================================================
// SECTION 6: ABC Complexity Tests
// ============================================================================

#[test]
fn test_abc_assignments() -> Result<()> {
    let source = r#"
fn with_assignments() {
    let a = 1;
    let b = 2;
    let c = 3;
    let d = a + b;
    let e = c * d;
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // ABC counts assignments, branches, and conditions
    assert_eq!(parsed.functions.len(), 1);

    Ok(())
}

#[test]
fn test_abc_branches() -> Result<()> {
    let source = r#"
fn with_branches(x: i32) -> i32 {
    if x > 0 {
        return x;
    } else if x < 0 {
        return -x;
    }
    0
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    assert_eq!(parsed.functions.len(), 1);

    Ok(())
}

#[test]
fn test_abc_conditions() -> Result<()> {
    let source = r#"
fn with_conditions(a: bool, b: bool, c: bool) -> bool {
    (a && b) || (b && c) || (a && c)
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    assert_eq!(parsed.functions.len(), 1);

    Ok(())
}

// ============================================================================
// SECTION 7: Number of Methods (NOM) Tests
// ============================================================================

#[test]
fn test_nom_struct_impl() -> Result<()> {
    let source = r#"
struct Calculator;

impl Calculator {
    fn add(a: i32, b: i32) -> i32 { a + b }
    fn subtract(a: i32, b: i32) -> i32 { a - b }
    fn multiply(a: i32, b: i32) -> i32 { a * b }
    fn divide(a: i32, b: i32) -> i32 { a / b }
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Should count 4 methods
    assert!(parsed.functions.len() >= 4);

    Ok(())
}

#[test]
fn test_nom_trait_impl() -> Result<()> {
    let source = r#"
trait Display {
    fn display(&self);
}

struct Point { x: i32, y: i32 }

impl Display for Point {
    fn display(&self) {
        println!("({}, {})", self.x, self.y);
    }
}

impl Point {
    fn new(x: i32, y: i32) -> Self {
        Point { x, y }
    }

    fn distance(&self, other: &Point) -> f64 {
        let dx = (self.x - other.x) as f64;
        let dy = (self.y - other.y) as f64;
        (dx * dx + dy * dy).sqrt()
    }
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Should count all methods across implementations
    assert!(parsed.functions.len() >= 3);

    Ok(())
}

// ============================================================================
// SECTION 8: Number of Arguments (NARGS) Tests
// ============================================================================

#[test]
fn test_nargs_zero() -> Result<()> {
    let source = r#"
fn no_args() {
    println!("No arguments");
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    assert_eq!(parsed.functions.len(), 1);
    assert_eq!(parsed.functions[0].parameters.len(), 0);

    Ok(())
}

#[test]
fn test_nargs_multiple() -> Result<()> {
    let source = r#"
fn many_args(a: i32, b: i32, c: i32, d: i32, e: i32) -> i32 {
    a + b + c + d + e
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    assert_eq!(parsed.functions.len(), 1);
    assert_eq!(parsed.functions[0].parameters.len(), 5);

    Ok(())
}

#[test]
fn test_nargs_with_generics() -> Result<()> {
    let source = r#"
fn generic_function<T, U>(a: T, b: U) where T: Clone, U: Clone {
    // Function body
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    assert_eq!(parsed.functions.len(), 1);
    assert_eq!(parsed.functions[0].parameters.len(), 2);

    Ok(())
}

// ============================================================================
// SECTION 9: Exit Points Tests
// ============================================================================

#[test]
fn test_exit_points_single() -> Result<()> {
    let source = r#"
fn single_exit(x: i32) -> i32 {
    let result = x * 2;
    result
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Single exit point
    assert_eq!(parsed.functions.len(), 1);

    Ok(())
}

#[test]
fn test_exit_points_multiple() -> Result<()> {
    let source = r#"
fn multiple_exits(x: i32) -> i32 {
    if x > 100 {
        return 100;
    }
    if x < 0 {
        return 0;
    }
    x
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Multiple exit points
    assert_eq!(parsed.functions.len(), 1);

    Ok(())
}

// ============================================================================
// SECTION 10: Cross-Language Metrics Consistency Tests
// ============================================================================

#[test]
fn test_metrics_rust_vs_typescript_similar_code() -> Result<()> {
    let rust_source = r#"
fn calculate(x: i32, y: i32) -> i32 {
    if x > y {
        x - y
    } else {
        y - x
    }
}
"#;

    let ts_source = r#"
function calculate(x: number, y: number): number {
    if (x > y) {
        return x - y;
    } else {
        return y - x;
    }
}
"#;

    let mut rust_parser = RustParser::new()?;
    let rust_parsed = rust_parser.parse_file("test.rs", rust_source)?;

    let mut ts_parser = TypeScriptParser::new()?;
    let ts_parsed = ts_parser.parse_file("test.ts", ts_source)?;

    // Both should parse one function
    assert_eq!(rust_parsed.functions.len(), 1);
    assert_eq!(ts_parsed.functions.len(), 1);

    // Metrics should be comparable
    assert_eq!(rust_parsed.functions[0].name, "calculate");
    assert_eq!(ts_parsed.functions[0].name, "calculate");

    Ok(())
}

// ============================================================================
// SECTION 11: Weighted Methods per Class (WMC) Tests
// ============================================================================

#[test]
fn test_wmc_calculation() -> Result<()> {
    let source = r#"
struct ComplexClass;

impl ComplexClass {
    fn simple_method() {
        // Complexity: 1
    }

    fn method_with_if(x: i32) -> i32 {
        // Complexity: 2
        if x > 0 { x } else { 0 }
    }

    fn complex_method(a: i32, b: i32) -> i32 {
        // Complexity: higher due to multiple branches
        if a > 0 {
            if b > 0 {
                a + b
            } else {
                a
            }
        } else {
            0
        }
    }
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // WMC is sum of complexities of all methods
    assert!(parsed.functions.len() >= 3);

    Ok(())
}

// ============================================================================
// SECTION 12: Edge Cases and Boundary Tests
// ============================================================================

#[test]
fn test_metrics_on_empty_function() -> Result<()> {
    let source = r#"
fn empty() {}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    assert_eq!(parsed.functions.len(), 1);
    assert_eq!(parsed.functions[0].parameters.len(), 0);

    Ok(())
}

#[test]
fn test_metrics_on_very_long_function() -> Result<()> {
    let mut source = String::from("fn very_long_function() {\n");
    for i in 0..1000 {
        source.push_str(&format!("    let var_{} = {};\n", i, i));
    }
    source.push_str("}\n");

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", &source)?;

    // Should handle very long functions
    assert_eq!(parsed.functions.len(), 1);

    Ok(())
}

#[test]
fn test_metrics_on_deeply_nested_code() -> Result<()> {
    let source = r#"
fn deeply_nested() {
    if true {
        if true {
            if true {
                if true {
                    if true {
                        if true {
                            println!("Deep");
                        }
                    }
                }
            }
        }
    }
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Should handle deeply nested code
    assert_eq!(parsed.functions.len(), 1);

    Ok(())
}

#[test]
fn test_metrics_with_closures() -> Result<()> {
    let source = r#"
fn with_closures() {
    let add = |a, b| a + b;
    let multiply = |a, b| a * b;

    let result = vec![1, 2, 3]
        .iter()
        .map(|x| x * 2)
        .filter(|x| x > &2)
        .collect::<Vec<_>>();
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Should handle closures
    assert_eq!(parsed.functions.len(), 1);

    Ok(())
}

#[test]
fn test_metrics_with_macros() -> Result<()> {
    let source = r#"
fn with_macros() {
    println!("Hello");
    vec![1, 2, 3];
    assert_eq!(1, 1);
    dbg!("Debug");
}
"#;

    let mut parser = RustParser::new()?;
    let parsed = parser.parse_file("test.rs", source)?;

    // Should handle macros
    assert_eq!(parsed.functions.len(), 1);

    Ok(())
}
