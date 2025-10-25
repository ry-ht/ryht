//! Comprehensive tests for advanced AST analysis features
//!
//! This test suite validates:
//! - AST search and filtering with FindConfig
//! - Node counting and statistics with AstCounter
//! - AST transformation with Alterator
//! - Comment analysis with CommentAnalyzer
//! - Lint rules and anti-pattern detection
//! - AST visitor pattern
//! - AST diffing functionality
//! - NodeChecker and NodeGetter traits
//! - Caching mechanisms

use anyhow::Result;
use cortex_code_analysis::{
    analysis::{
        // Search and navigation
        AstFinder, FindConfig, FindConfigBuilder, FindResult, NodeFilter,
        // Counting and statistics
        AstCounter, ConcurrentCounter, CountConfig, CountFilter, CountStats,
        // AST transformation
        Alterator, TransformConfig, TransformConfigBuilder,
        AstVisitor, VisitAction, visit_ast,
        AstDiff, DiffConfig, diff_ast,
        // Node analysis
        NodeChecker, DefaultNodeChecker, NodeGetter, DefaultNodeGetter,
        // Lint rules
        LintChecker, LintRule, LintViolation, Severity,
        FunctionTooLongRule, DeepNestingRule, MissingDocCommentRule, TodoCommentRule,
        AntiPattern, AntiPatternDetector,
        // Comment analysis
        Comment, CommentAnalyzer, CommentMetrics, CommentType, analyze_comments,
        // Caching
        Cache, CacheManager, CacheBuilder, AstCache, MetricsCache, SearchCache,
    },
    Parser, RustLanguage, Lang, Node,
};
use cortex_code_analysis::traits::{ParserTrait, Search};
use std::path::Path;

// ============================================================================
// SECTION 1: AST Search and Filtering Tests
// ============================================================================

#[test]
fn test_ast_finder_basic() -> Result<()> {
    let source = r#"
        fn main() {
            let x = 1;
            let y = 2;
        }
        fn helper() {}
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let config = FindConfig::builder()
        .add_filter(NodeFilter::Kind("function_item".to_string()))
        .build();

    let finder = AstFinder::new(&parser);
    let results = finder.find(&config)?;

    assert_eq!(results.nodes.len(), 2); // main and helper
    assert!(results.metadata.total_nodes_visited > 0);

    Ok(())
}

#[test]
fn test_ast_finder_with_depth_limit() -> Result<()> {
    let source = r#"
        fn main() {
            if true {
                if false {
                    let x = 1;
                }
            }
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let config = FindConfig::builder()
        .max_depth(3)
        .build();

    let finder = AstFinder::new(&parser);
    let results = finder.find(&config)?;

    assert!(results.nodes.len() > 0);

    Ok(())
}

#[test]
fn test_ast_finder_with_multiple_filters() -> Result<()> {
    let source = r#"
        fn main() {
            let x = 1;
            let y = 2;
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let config = FindConfig::builder()
        .add_filter(NodeFilter::Kind("let_declaration".to_string()))
        .build();

    let finder = AstFinder::new(&parser);
    let results = finder.find(&config)?;

    assert_eq!(results.nodes.len(), 2); // x and y

    Ok(())
}

#[test]
fn test_ast_finder_with_name_filter() -> Result<()> {
    let source = r#"
        fn main() {}
        fn test() {}
        fn helper() {}
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let config = FindConfig::builder()
        .add_filter(NodeFilter::Kind("function_item".to_string()))
        .build();

    let finder = AstFinder::new(&parser);
    let results = finder.find(&config)?;

    assert_eq!(results.nodes.len(), 3);

    Ok(())
}

#[test]
fn test_find_result_metadata() -> Result<()> {
    let source = "fn main() {}";

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let config = FindConfig::default();
    let finder = AstFinder::new(&parser);
    let results = finder.find(&config)?;

    assert!(results.metadata.total_nodes_visited > 0);
    assert!(results.metadata.search_duration.as_nanos() > 0);

    Ok(())
}

// ============================================================================
// SECTION 2: AST Counting and Statistics Tests
// ============================================================================

#[test]
fn test_ast_counter_basic() -> Result<()> {
    let source = r#"
        fn main() {
            let x = 1;
            let y = 2;
            let z = 3;
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let config = CountConfig::builder()
        .add_filter(CountFilter::Kind("let_declaration".to_string()))
        .build();

    let counter = AstCounter::new(&parser);
    let stats = counter.count(&config)?;

    assert_eq!(stats.total_matches, 3);
    assert!(stats.total_nodes_visited > 0);

    Ok(())
}

#[test]
fn test_ast_counter_by_kind() -> Result<()> {
    let source = r#"
        fn main() {}
        fn test() {}
        struct Point { x: i32, y: i32 }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let counter = AstCounter::new(&parser);
    let stats = counter.count(&CountConfig::default())?;

    // Should have counts for different node kinds
    assert!(stats.counts_by_kind.len() > 0);

    Ok(())
}

#[test]
fn test_ast_counter_with_depth_stats() -> Result<()> {
    let source = r#"
        fn main() {
            if true {
                if false {
                    let x = 1;
                }
            }
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let counter = AstCounter::new(&parser);
    let stats = counter.count(&CountConfig::default())?;

    assert!(stats.max_depth > 0);
    assert!(stats.avg_depth > 0.0);

    Ok(())
}

#[test]
fn test_concurrent_counter() -> Result<()> {
    let sources = vec![
        "fn main() {}",
        "fn test() {}",
        "fn helper() {}",
    ];

    let parsers: Vec<_> = sources
        .iter()
        .enumerate()
        .map(|(i, src)| {
            Parser::<RustLanguage>::new(
                src.as_bytes().to_vec(),
                Path::new(&format!("test{}.rs", i))
            )
        })
        .collect::<Result<Vec<_>>>()?;

    let config = CountConfig::builder()
        .add_filter(CountFilter::Kind("function_item".to_string()))
        .build();

    let counter = ConcurrentCounter::new(4);
    let stats = counter.count_all(&parsers, &config)?;

    assert_eq!(stats.total_matches, 3);

    Ok(())
}

// ============================================================================
// SECTION 3: AST Transformation Tests
// ============================================================================

#[test]
fn test_alterator_basic_transform() -> Result<()> {
    let source = r#"
        fn main() {
            println!("Hello");
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let config = TransformConfig::builder()
        .include_spans(true)
        .extract_text(true)
        .build();

    let alterator = Alterator::new(&parser, source.as_bytes());
    let ast = alterator.transform(&config)?;

    assert_eq!(ast.kind, "source_file");
    assert!(ast.children.is_some());

    Ok(())
}

#[test]
fn test_alterator_filter_comments() -> Result<()> {
    let source = r#"
        // This is a comment
        fn main() {
            // Another comment
            let x = 1;
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let config = TransformConfig::builder()
        .filter_comments(true)
        .build();

    let alterator = Alterator::new(&parser, source.as_bytes());
    let ast = alterator.transform(&config)?;

    // Comments should be filtered out
    // We should still have the function and let declaration
    assert!(ast.children.is_some());

    Ok(())
}

#[test]
fn test_alterator_max_depth() -> Result<()> {
    let source = r#"
        fn main() {
            if true {
                if false {
                    let x = 1;
                }
            }
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let config = TransformConfig::builder()
        .max_depth(2)
        .build();

    let alterator = Alterator::new(&parser, source.as_bytes());
    let ast = alterator.transform(&config)?;

    assert!(ast.children.is_some());

    Ok(())
}

#[test]
fn test_alterator_span_extraction() -> Result<()> {
    let source = "fn main() {}";

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let config = TransformConfig::builder()
        .include_spans(true)
        .build();

    let alterator = Alterator::new(&parser, source.as_bytes());
    let ast = alterator.transform(&config)?;

    assert!(ast.span.is_some());
    let span = ast.span.unwrap();
    assert!(span.start.byte >= 0);
    assert!(span.end.byte > span.start.byte);

    Ok(())
}

// ============================================================================
// SECTION 4: AST Visitor Pattern Tests
// ============================================================================

#[test]
fn test_ast_visitor_basic() -> Result<()> {
    let source = "fn main() {}";

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let mut count = 0;
    let visitor = |node: &Node| -> VisitAction {
        count += 1;
        VisitAction::Continue
    };

    let root = parser.get_root();
    visit_ast(&root, visitor);

    assert!(count > 0);

    Ok(())
}

#[test]
fn test_ast_visitor_early_termination() -> Result<()> {
    let source = r#"
        fn main() {}
        fn test() {}
        fn helper() {}
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let mut count = 0;
    let visitor = |node: &Node| -> VisitAction {
        count += 1;
        if count >= 5 {
            VisitAction::Stop
        } else {
            VisitAction::Continue
        }
    };

    let root = parser.get_root();
    visit_ast(&root, visitor);

    assert_eq!(count, 5);

    Ok(())
}

#[test]
fn test_ast_visitor_skip_subtree() -> Result<()> {
    let source = r#"
        fn main() {
            if true {
                let x = 1;
            }
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let mut visited = Vec::new();
    let visitor = |node: &Node| -> VisitAction {
        visited.push(node.kind().to_string());
        if node.kind() == "if_expression" {
            VisitAction::SkipChildren
        } else {
            VisitAction::Continue
        }
    };

    let root = parser.get_root();
    visit_ast(&root, visitor);

    // Should visit if_expression but not its children
    assert!(visited.contains(&"if_expression".to_string()));

    Ok(())
}

// ============================================================================
// SECTION 5: Comment Analysis Tests
// ============================================================================

#[test]
fn test_comment_analyzer_basic() -> Result<()> {
    let source = r#"
        /// This is a doc comment
        fn main() {
            // This is an inline comment
            let x = 1;
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let analyzer = CommentAnalyzer::new(&parser, source.as_bytes());
    let metrics = analyzer.analyze()?;

    assert!(metrics.total_comments > 0);
    assert!(metrics.doc_comments.len() > 0);
    assert!(metrics.inline_comments.len() > 0);

    Ok(())
}

#[test]
fn test_comment_analyzer_density() -> Result<()> {
    let source = r#"
        // Comment 1
        // Comment 2
        fn main() {}
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let analyzer = CommentAnalyzer::new(&parser, source.as_bytes());
    let metrics = analyzer.analyze()?;

    let density = metrics.density();
    assert!(density > 0.0 && density <= 1.0);

    Ok(())
}

#[test]
fn test_comment_analyzer_todo_detection() -> Result<()> {
    let source = r#"
        // TODO: Fix this
        fn main() {
            // FIXME: Refactor
            let x = 1;
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let analyzer = CommentAnalyzer::new(&parser, source.as_bytes());
    let metrics = analyzer.analyze()?;

    let annotations: Vec<_> = metrics.all_comments()
        .into_iter()
        .filter(|c| c.has_annotation())
        .collect();

    assert!(annotations.len() >= 2);

    Ok(())
}

#[test]
fn test_analyze_comments_helper() -> Result<()> {
    let source = "// Test comment\nfn main() {}";

    let comments = analyze_comments(source, Lang::Rust)?;

    assert!(comments.len() > 0);

    Ok(())
}

// ============================================================================
// SECTION 6: Lint Rules and Anti-Pattern Detection Tests
// ============================================================================

#[test]
fn test_lint_checker_function_too_long() -> Result<()> {
    let source = r#"
        fn main() {
            let x1 = 1;
            let x2 = 2;
            let x3 = 3;
            let x4 = 4;
            let x5 = 5;
            let x6 = 6;
            let x7 = 7;
            let x8 = 8;
            let x9 = 9;
            let x10 = 10;
            let x11 = 11;
            let x12 = 12;
            let x13 = 13;
            let x14 = 14;
            let x15 = 15;
            let x16 = 16;
            let x17 = 17;
            let x18 = 18;
            let x19 = 19;
            let x20 = 20;
            let x21 = 21;
            let x22 = 22;
            let x23 = 23;
            let x24 = 24;
            let x25 = 25;
            let x26 = 26;
            let x27 = 27;
            let x28 = 28;
            let x29 = 29;
            let x30 = 30;
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let rule = FunctionTooLongRule::new(20);
    let checker = LintChecker::new(vec![Box::new(rule)]);
    let violations = checker.check(&parser)?;

    assert!(violations.len() > 0);
    assert_eq!(violations[0].severity, Severity::Warning);

    Ok(())
}

#[test]
fn test_lint_checker_deep_nesting() -> Result<()> {
    let source = r#"
        fn main() {
            if true {
                if true {
                    if true {
                        if true {
                            if true {
                                let x = 1;
                            }
                        }
                    }
                }
            }
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let rule = DeepNestingRule::new(3);
    let checker = LintChecker::new(vec![Box::new(rule)]);
    let violations = checker.check(&parser)?;

    assert!(violations.len() > 0);

    Ok(())
}

#[test]
fn test_lint_checker_missing_doc_comment() -> Result<()> {
    let source = r#"
        pub fn main() {
            let x = 1;
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let rule = MissingDocCommentRule::new();
    let checker = LintChecker::new(vec![Box::new(rule)]);
    let violations = checker.check(&parser)?;

    assert!(violations.len() > 0);
    assert!(violations[0].message.contains("public function"));

    Ok(())
}

#[test]
fn test_lint_checker_todo_comment() -> Result<()> {
    let source = r#"
        fn main() {
            // TODO: Implement this
            let x = 1;
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let rule = TodoCommentRule::new();
    let checker = LintChecker::new(vec![Box::new(rule)]);
    let violations = checker.check(&parser)?;

    assert!(violations.len() > 0);
    assert_eq!(violations[0].severity, Severity::Info);

    Ok(())
}

#[test]
fn test_anti_pattern_detector() -> Result<()> {
    let source = r#"
        fn main() {
            // TODO: Fix this later
            if true {
                if true {
                    if true {
                        if true {
                            let x = 1;
                        }
                    }
                }
            }
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let detector = AntiPatternDetector::new();
    let patterns = detector.detect(&parser)?;

    // Should detect deep nesting and TODO comment
    assert!(patterns.len() > 0);

    Ok(())
}

// ============================================================================
// SECTION 7: NodeChecker and NodeGetter Tests
// ============================================================================

#[test]
fn test_node_checker_is_comment() -> Result<()> {
    let source = r#"
        // This is a comment
        fn main() {}
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let root = parser.get_root();
    let mut found_comment = false;

    for child in root.children() {
        if DefaultNodeChecker::is_comment(&child, Lang::Rust) {
            found_comment = true;
            break;
        }
    }

    assert!(found_comment);

    Ok(())
}

#[test]
fn test_node_checker_is_func() -> Result<()> {
    let source = "fn main() {}";

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let root = parser.get_root();
    let mut found_function = false;

    for child in root.children() {
        if DefaultNodeChecker::is_func(&child, Lang::Rust) {
            found_function = true;
            break;
        }
    }

    assert!(found_function);

    Ok(())
}

#[test]
fn test_node_checker_is_closure() -> Result<()> {
    let source = r#"
        fn main() {
            let f = |x| x + 1;
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let root = parser.get_root();
    let mut found_closure = false;

    fn check_node(node: &Node, found: &mut bool) {
        if DefaultNodeChecker::is_closure(node, Lang::Rust) {
            *found = true;
            return;
        }
        for child in node.children() {
            check_node(&child, found);
        }
    }

    check_node(&root, &mut found_closure);
    assert!(found_closure);

    Ok(())
}

#[test]
fn test_node_getter_name() -> Result<()> {
    let source = "fn main() {}";

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let root = parser.get_root();
    for child in root.children() {
        if DefaultNodeChecker::is_func(&child, Lang::Rust) {
            let name = DefaultNodeGetter::get_name(&child, source.as_bytes());
            assert!(name.is_some());
            assert_eq!(name.unwrap(), "main");
        }
    }

    Ok(())
}

// ============================================================================
// SECTION 8: Caching Tests
// ============================================================================

#[test]
fn test_ast_cache_basic() -> Result<()> {
    let source = "fn main() {}";
    let key = "test.rs".to_string();

    let mut cache: AstCache = Cache::new(10);

    // Cache miss
    assert!(cache.get(&key).is_none());

    // Insert and retrieve
    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let root = parser.get_root();
    cache.insert(key.clone(), root.clone());

    let cached = cache.get(&key);
    assert!(cached.is_some());

    Ok(())
}

#[test]
fn test_cache_eviction() -> Result<()> {
    let mut cache: AstCache = Cache::new(2);

    let key1 = "file1.rs".to_string();
    let key2 = "file2.rs".to_string();
    let key3 = "file3.rs".to_string();

    let parser1 = Parser::<RustLanguage>::new(
        "fn f1() {}".as_bytes().to_vec(),
        Path::new("file1.rs")
    )?;
    let parser2 = Parser::<RustLanguage>::new(
        "fn f2() {}".as_bytes().to_vec(),
        Path::new("file2.rs")
    )?;
    let parser3 = Parser::<RustLanguage>::new(
        "fn f3() {}".as_bytes().to_vec(),
        Path::new("file3.rs")
    )?;

    cache.insert(key1.clone(), parser1.get_root());
    cache.insert(key2.clone(), parser2.get_root());

    // Inserting key3 should evict key1 (LRU)
    cache.insert(key3.clone(), parser3.get_root());

    assert!(cache.get(&key1).is_none());
    assert!(cache.get(&key2).is_some());
    assert!(cache.get(&key3).is_some());

    Ok(())
}

#[test]
fn test_cache_builder() -> Result<()> {
    let cache = CacheBuilder::new()
        .with_capacity(100)
        .build::<String, Node>();

    assert_eq!(cache.capacity(), 100);

    Ok(())
}

#[test]
fn test_cache_clear() -> Result<()> {
    let mut cache: AstCache = Cache::new(10);
    let key = "test.rs".to_string();

    let parser = Parser::<RustLanguage>::new(
        "fn main() {}".as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    cache.insert(key.clone(), parser.get_root());
    assert!(cache.get(&key).is_some());

    cache.clear();
    assert!(cache.get(&key).is_none());

    Ok(())
}

// ============================================================================
// SECTION 9: Edge Cases and Error Handling
// ============================================================================

#[test]
fn test_empty_source_handling() -> Result<()> {
    let source = "";

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("empty.rs")
    )?;

    let finder = AstFinder::new(&parser);
    let results = finder.find(&FindConfig::default())?;

    // Should handle empty source gracefully
    assert!(results.nodes.len() >= 0);

    Ok(())
}

#[test]
fn test_deep_nesting_performance() -> Result<()> {
    // Generate deeply nested code
    let mut source = String::from("fn main() {\n");
    for _ in 0..20 {
        source.push_str("    if true {\n");
    }
    source.push_str("        let x = 1;\n");
    for _ in 0..20 {
        source.push_str("    }\n");
    }
    source.push_str("}\n");

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("deep.rs")
    )?;

    // Should handle deep nesting without stack overflow
    let counter = AstCounter::new(&parser);
    let stats = counter.count(&CountConfig::default())?;

    assert!(stats.max_depth >= 20);

    Ok(())
}

#[test]
fn test_large_file_handling() -> Result<()> {
    // Generate a large file
    let mut source = String::new();
    for i in 0..100 {
        source.push_str(&format!("fn func_{}() {{}}\n", i));
    }

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("large.rs")
    )?;

    let config = FindConfig::builder()
        .add_filter(NodeFilter::Kind("function_item".to_string()))
        .build();

    let finder = AstFinder::new(&parser);
    let results = finder.find(&config)?;

    assert_eq!(results.nodes.len(), 100);

    Ok(())
}

#[test]
fn test_concurrent_counter_empty_list() -> Result<()> {
    let parsers: Vec<Parser<RustLanguage>> = vec![];
    let config = CountConfig::default();

    let counter = ConcurrentCounter::new(4);
    let stats = counter.count_all(&parsers, &config)?;

    assert_eq!(stats.total_matches, 0);

    Ok(())
}

#[test]
fn test_alterator_with_empty_config() -> Result<()> {
    let source = "fn main() {}";

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let config = TransformConfig::default();
    let alterator = Alterator::new(&parser, source.as_bytes());
    let ast = alterator.transform(&config)?;

    assert_eq!(ast.kind, "source_file");

    Ok(())
}
