//! Comprehensive AST analysis demonstration.
//!
//! This example showcases the advanced AST analysis capabilities including:
//! - Node traversal and navigation
//! - AST visitor pattern
//! - AST diff and comparison
//! - Pattern matching
//! - Comment analysis
//! - Lint rule checking
//! - Anti-pattern detection

use cortex_code_analysis::*;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    println!("=== Cortex AST Analysis Demo ===\n");

    // Sample Rust code to analyze
    let source = r#"
/// Calculate the factorial of a number
fn factorial(n: u32) -> u32 {
    if n == 0 {
        1
    } else {
        // Recursive call
        n * factorial(n - 1)
    }
}

// TODO: Optimize this function
fn fibonacci(n: u32) -> u32 {
    if n <= 1 {
        n
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
}

/// A very long function that does many things
fn complex_function(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) -> i32 {
    let x = a + b;
    let y = c + d;
    let z = e + f;

    if x > 0 {
        if y > 0 {
            if z > 0 {
                if x + y + z > 100 {
                    return 42;
                }
            }
        }
    }

    x + y + z
}
"#;

    let parser = Parser::<RustLanguage>::new(source.as_bytes().to_vec(), Path::new("example.rs"))?;

    // =========================================================================
    // 1. Node Navigation and Traversal
    // =========================================================================
    println!("1. Node Navigation Examples");
    println!("{}", "=".repeat(60));

    let root = parser.get_root();
    println!("Root node: {}", root.kind());
    println!("Root has {} children", root.child_count());
    println!("Root depth: {}", root.depth());

    // Find all function nodes
    let functions = root.find_descendants_of_kind("function_item");
    println!("\nFound {} functions:", functions.len());
    for func in &functions {
        println!("  - {} at line {}", func.kind(), func.start_row() + 1);
        println!("    Path: {:?}", func.path_kinds());
        println!("    Line count: {}", func.line_count());
    }

    // =========================================================================
    // 2. Visitor Pattern
    // =========================================================================
    println!("\n2. Visitor Pattern Example");
    println!("{}", "=".repeat(60));

    struct FunctionVisitor {
        function_count: usize,
        total_depth: usize,
    }

    impl<'a> AstVisitor<'a> for FunctionVisitor {
        fn visit_enter(&mut self, node: &Node<'a>, depth: usize) -> VisitAction {
            if node.kind() == "function_item" {
                self.function_count += 1;
                self.total_depth += depth;
            }
            VisitAction::Continue
        }

        fn visit_leave(&mut self, _node: &Node<'a>, _depth: usize) {}
    }

    let mut visitor = FunctionVisitor {
        function_count: 0,
        total_depth: 0,
    };

    visit_ast(&parser, &mut visitor);
    println!("Functions found: {}", visitor.function_count);
    println!("Average function depth: {:.2}",
        visitor.total_depth as f64 / visitor.function_count as f64);

    // =========================================================================
    // 3. AST Diff
    // =========================================================================
    println!("\n3. AST Diff Example");
    println!("{}", "=".repeat(60));

    let old_code = b"fn test() { let x = 1; }";
    let new_code = b"fn test() { let x = 2; }";

    let old_parser = Parser::<RustLanguage>::new(old_code.to_vec(), Path::new("old.rs"))?;
    let new_parser = Parser::<RustLanguage>::new(new_code.to_vec(), Path::new("new.rs"))?;

    let diffs = diff_ast(
        &old_parser.get_root(),
        &new_parser.get_root(),
        old_code,
        new_code,
        &DiffConfig::default(),
    );

    println!("Found {} differences:", diffs.len());
    for diff in &diffs {
        match diff {
            AstDiff::Modified { kind, old_text, new_text, .. } => {
                println!("  Modified {}: '{}' -> '{}'", kind, old_text, new_text);
            }
            AstDiff::Added { kind, text, .. } => {
                println!("  Added {}: '{}'", kind, text);
            }
            AstDiff::Removed { kind, text, .. } => {
                println!("  Removed {}: '{}'", kind, text);
            }
            AstDiff::KindChanged { old_kind, new_kind, .. } => {
                println!("  Kind changed: {} -> {}", old_kind, new_kind);
            }
        }
    }

    // =========================================================================
    // 4. Pattern Matching
    // =========================================================================
    println!("\n4. Pattern Matching Example");
    println!("{}", "=".repeat(60));

    // Find all function items
    let pattern = AstPattern::kind("function_item");
    let matches = pattern.find_matches(&root, source.as_bytes());
    println!("Pattern matched {} function items", matches.len());

    // Find if expressions
    let if_pattern = AstPattern::kind("if_expression");
    let if_matches = if_pattern.find_matches(&root, source.as_bytes());
    println!("Pattern matched {} if expressions", if_matches.len());

    // =========================================================================
    // 5. Comment Analysis
    // =========================================================================
    println!("\n5. Comment Analysis Example");
    println!("{}", "=".repeat(60));

    let comment_analyzer = CommentAnalyzer::new(&parser, source.as_bytes());
    let metrics = comment_analyzer.analyze()?;

    println!("Total comments: {}", metrics.comments.len());
    println!("Doc comments: {}", metrics.doc_comments.len());
    println!("Inline comments: {}", metrics.inline_comments.len());
    println!("Comment density: {:.2}%", metrics.density() * 100.0);
    println!("Comment to code ratio: {:.2}%", metrics.comment_ratio() * 100.0);
    println!("Average comment quality: {:.2}", metrics.average_quality());

    // Find annotations (TODO, FIXME, etc.)
    let annotations = comment_analyzer.find_annotations()?;
    println!("\nAnnotations found: {}", annotations.len());
    for annotation in &annotations {
        println!(
            "  - {} at line {}: {}",
            annotation.annotation_type().unwrap_or("Unknown"),
            annotation.start_line,
            annotation.text.chars().take(50).collect::<String>()
        );
    }

    // =========================================================================
    // 6. Lint Checking
    // =========================================================================
    println!("\n6. Lint Rule Checking Example");
    println!("{}", "=".repeat(60));

    let lint_checker = LintChecker::with_default_rules();
    let violations = lint_checker.check_tree(&root, source.as_bytes(), Lang::Rust);

    println!("Found {} lint violations:", violations.len());
    for violation in &violations {
        println!(
            "  [{:?}] {} at line {}",
            violation.severity, violation.rule_id, violation.start_line
        );
        println!("      {}", violation.message);
        if let Some(suggestion) = &violation.suggestion {
            println!("      Suggestion: {}", suggestion);
        }
    }

    // =========================================================================
    // 7. Anti-pattern Detection
    // =========================================================================
    println!("\n7. Anti-pattern Detection Example");
    println!("{}", "=".repeat(60));

    // Detect magic numbers
    let magic_numbers = AntiPatternDetector::detect_magic_numbers(&root, source.as_bytes());
    println!("Magic numbers found: {}", magic_numbers.len());
    for number in &magic_numbers {
        if let Some(text) = number.utf8_text(source.as_bytes()) {
            println!("  - {} at line {}", text, number.start_row() + 1);
        }
    }

    // Detect long parameter lists
    let long_params = AntiPatternDetector::detect_long_parameter_lists(&root, Lang::Rust);
    println!("\nFunctions with long parameter lists: {}", long_params.len());
    for func in &long_params {
        println!("  - At line {}", func.start_row() + 1);
    }

    // =========================================================================
    // 8. AST Rewrite
    // =========================================================================
    println!("\n8. AST Rewrite Example");
    println!("{}", "=".repeat(60));

    let simple_code = "fn test() { let x = 1; }";
    let mut rewrites = vec![
        Rewrite::new(16..17, "answer".to_string()),
        Rewrite::new(20..21, "42".to_string()),
    ];

    let rewritten = apply_rewrites(simple_code, &mut rewrites);
    println!("Original: {}", simple_code);
    println!("Rewritten: {}", rewritten);

    // =========================================================================
    // 9. Advanced Node Operations
    // =========================================================================
    println!("\n9. Advanced Node Operations Example");
    println!("{}", "=".repeat(60));

    if let Some(first_func) = functions.first() {
        println!("Analyzing first function:");
        println!("  - Named children: {}", first_func.named_child_count());
        println!("  - Is multiline: {}", first_func.is_multiline());
        println!("  - Byte length: {}", first_func.byte_len());

        let siblings = first_func.siblings_excluding_self();
        println!("  - Siblings: {}", siblings.len());

        if let Some(parent) = first_func.parent() {
            println!("  - Parent kind: {}", parent.kind());
        }

        let descendants = first_func.descendants_bfs();
        println!("  - Total descendants (BFS): {}", descendants.len());
    }

    println!("\n=== Demo Complete ===");
    Ok(())
}
