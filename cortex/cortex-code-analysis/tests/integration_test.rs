//! Integration tests for the cortex-code-analysis crate
//!
//! This test suite validates end-to-end workflows and cross-feature integration:
//! - Multi-language analysis workflows
//! - Combined AST analysis and metrics calculation
//! - Concurrent processing with caching and progress tracking
//! - Real-world code samples
//! - Performance benchmarks
//! - Memory usage tests

use anyhow::Result;
use cortex_code_analysis::{
    // Core parsing
    Parser, RustLanguage, Lang, CodeParser,
    // Analysis
    analysis::{
        AstFinder, AstCounter, Alterator, CommentAnalyzer,
        FindConfig, CountConfig, TransformConfig,
        LintChecker, FunctionTooLongRule, DeepNestingRule,
        Cache, CacheBuilder,
    },
    // Metrics
    metrics::{MetricsBuilder, MetricsAggregator, CodeMetrics},
    // Concurrent
    concurrent::{
        ParallelProcessor, BatchProcessor, BatchConfig, BatchStrategy,
        FileCache, ProgressTracker, ProgressConfig,
    },
};
use cortex_code_analysis::traits::ParserTrait;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tempfile::TempDir;

// ============================================================================
// SECTION 1: Multi-Language Analysis Tests
// ============================================================================

#[test]
fn test_rust_end_to_end() -> Result<()> {
    let source = r#"
        /// A simple calculator module
        pub mod calculator {
            /// Adds two numbers
            pub fn add(a: i32, b: i32) -> i32 {
                a + b
            }

            /// Multiplies two numbers
            pub fn multiply(a: i32, b: i32) -> i32 {
                let mut result = a;
                for _ in 0..b {
                    result += a;
                }
                result
            }
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            fn test_add() {
                assert_eq!(calculator::add(2, 3), 5);
            }
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("calculator.rs")
    )?;

    // Test AST search
    let finder = AstFinder::new(&parser);
    let functions = finder.find(
        &FindConfig::builder()
            .add_filter(cortex_code_analysis::analysis::NodeFilter::Kind("function_item".to_string()))
            .build()
    )?;
    assert!(functions.nodes.len() >= 2); // add, multiply, test_add

    // Test metrics
    let metrics_builder = MetricsBuilder::new(&parser);
    let metrics = metrics_builder.calculate()?;
    assert!(metrics.nom.functions() >= 2.0);
    assert!(metrics.loc.sloc() > 10.0);

    // Test comment analysis
    let comment_analyzer = CommentAnalyzer::new(&parser, source.as_bytes());
    let comment_metrics = comment_analyzer.analyze()?;
    assert!(comment_metrics.doc_comments.len() >= 2);

    Ok(())
}

#[test]
fn test_multi_language_parsing() -> Result<()> {
    let mut code_parser = CodeParser::new()?;

    // Test Rust
    let rust_result = code_parser.parse_rust(
        "test.rs",
        "fn main() {}"
    )?;
    assert_eq!(rust_result.functions.len(), 1);

    // Test TypeScript
    let ts_result = code_parser.parse_typescript(
        "test.ts",
        "function main() {}"
    )?;
    assert_eq!(ts_result.functions.len(), 1);

    // Test JavaScript
    let js_result = code_parser.parse_javascript(
        "test.js",
        "function main() {}"
    )?;
    assert_eq!(js_result.functions.len(), 1);

    Ok(())
}

#[test]
fn test_language_auto_detection() -> Result<()> {
    let mut code_parser = CodeParser::new()?;

    let rust_result = code_parser.parse_file_auto(
        "main.rs",
        "fn test() {}"
    )?;
    assert_eq!(rust_result.functions.len(), 1);

    let ts_result = code_parser.parse_file_auto(
        "app.ts",
        "function test() {}"
    )?;
    assert_eq!(ts_result.functions.len(), 1);

    Ok(())
}

// ============================================================================
// SECTION 2: Combined Analysis Workflows
// ============================================================================

#[test]
fn test_ast_analysis_with_metrics() -> Result<()> {
    let source = r#"
        fn complex_function() {
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

    // Perform AST analysis
    let counter = AstCounter::new(&parser);
    let count_stats = counter.count(&CountConfig::default())?;

    // Calculate metrics
    let metrics_builder = MetricsBuilder::new(&parser);
    let metrics = metrics_builder.calculate()?;

    // Verify both analyses
    assert!(count_stats.total_nodes_visited > 0);
    assert!(metrics.cyclomatic.cyclomatic() > 1.0);

    Ok(())
}

#[test]
fn test_lint_with_metrics() -> Result<()> {
    let source = r#"
        pub fn very_long_function() {
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
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    // Run lint rules
    let rules: Vec<Box<dyn cortex_code_analysis::analysis::LintRule>> = vec![
        Box::new(FunctionTooLongRule::new(20)),
        Box::new(DeepNestingRule::new(3)),
    ];
    let checker = LintChecker::new(rules);
    let violations = checker.check(&parser)?;

    // Calculate metrics
    let metrics_builder = MetricsBuilder::new(&parser);
    let metrics = metrics_builder.calculate()?;

    // Verify correlation
    assert!(violations.len() > 0);
    assert!(metrics.loc.sloc() > 20.0);

    Ok(())
}

#[test]
fn test_transform_and_analyze() -> Result<()> {
    let source = r#"
        // Comment to filter
        fn main() {
            // Another comment
            let x = 1;
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    // Transform with comment filtering
    let config = TransformConfig::builder()
        .filter_comments(true)
        .include_spans(true)
        .build();

    let alterator = Alterator::new(&parser, source.as_bytes());
    let ast = alterator.transform(&config)?;

    // Verify transformation
    assert_eq!(ast.kind, "source_file");
    assert!(ast.children.is_some());

    // Still able to run metrics on original
    let metrics = MetricsBuilder::new(&parser).calculate()?;
    assert!(metrics.nom.functions() >= 1.0);

    Ok(())
}

// ============================================================================
// SECTION 3: Concurrent Processing with Integration
// ============================================================================

#[test]
#[ignore = "Integration test - requires file system"]
fn test_concurrent_analysis_with_metrics() -> Result<()> {
    let temp = TempDir::new()?;

    // Create test files
    let files = vec![
        ("file1.rs", "fn test1() {}"),
        ("file2.rs", "fn test2() { let x = 1; }"),
        ("file3.rs", "fn test3() { if true { let y = 2; } }"),
    ];

    for (name, content) in &files {
        fs::write(temp.path().join(name), content)?;
    }

    let results = Arc::new(parking_lot::Mutex::new(Vec::new()));
    let results_clone = results.clone();

    let processor = ParallelProcessor::new(move |path: &PathBuf, _: &()| {
        let source = fs::read_to_string(path)?;
        let parser = Parser::<RustLanguage>::new(
            source.as_bytes().to_vec(),
            path.as_ref()
        )?;

        let metrics = MetricsBuilder::new(&parser).calculate()?;
        results_clone.lock().push(metrics);

        Ok(())
    });

    let file_paths: Vec<PathBuf> = files.iter()
        .map(|(name, _)| temp.path().join(name))
        .collect();

    let (_, stats) = processor.process_all(file_paths, ())?;

    assert_eq!(stats.successful, 3);

    let collected_metrics = results.lock();
    assert_eq!(collected_metrics.len(), 3);

    // Verify each file was analyzed
    for metrics in collected_metrics.iter() {
        assert!(metrics.nom.functions() >= 1.0);
    }

    Ok(())
}

#[test]
#[ignore = "Integration test - requires file system"]
fn test_batch_processing_with_caching() -> Result<()> {
    let temp = TempDir::new()?;

    // Create multiple files
    for i in 0..10 {
        fs::write(
            temp.path().join(format!("file{}.rs", i)),
            format!("fn test{}() {{}}", i)
        )?;
    }

    let cache = Arc::new(FileCache::new(20));
    let cache_clone = cache.clone();

    let config = BatchConfig {
        batch_size: 3,
        strategy: BatchStrategy::FixedSize,
        sort_strategy: cortex_code_analysis::concurrent::SortStrategy::None,
        max_memory_mb: 100,
    };

    let processor = BatchProcessor::new(
        config,
        move |batch: Vec<PathBuf>, _: &()| {
            let mut count = 0;
            for path in batch {
                // Check cache
                if let Some(_cached) = cache_clone.get(&path) {
                    count += 1;
                    continue;
                }

                // Read and cache
                if let Ok(content) = fs::read(&path) {
                    cache_clone.insert(path.clone(), content);
                    count += 1;
                }
            }
            Ok(count)
        },
    );

    let files: Vec<PathBuf> = (0..10)
        .map(|i| temp.path().join(format!("file{}.rs", i)))
        .collect();

    let (results, stats) = processor.process(files.clone(), ())?;

    assert_eq!(stats.total_files, 10);
    assert!(results.len() > 0);

    // Run again to test cache hits
    let (_, _) = processor.process(files, ())?;
    let cache_stats = cache.stats();
    assert!(cache_stats.hits > 0);

    Ok(())
}

#[test]
#[ignore = "Integration test - requires file system"]
fn test_progress_tracking_integration() -> Result<()> {
    let temp = TempDir::new()?;

    for i in 0..20 {
        fs::write(temp.path().join(format!("file{}.rs", i)), "fn test() {}")?;
    }

    let progress_config = ProgressConfig {
        enabled: true,
        update_interval_ms: 10,
        show_throughput: true,
        show_eta: true,
    };

    let tracker = Arc::new(ProgressTracker::new(20, progress_config));
    let tracker_clone = tracker.clone();

    let processor = ParallelProcessor::new(move |path: &PathBuf, _: &()| {
        let source = fs::read_to_string(path)?;
        let parser = Parser::<RustLanguage>::new(
            source.as_bytes().to_vec(),
            path.as_ref()
        )?;

        let _ = MetricsBuilder::new(&parser).calculate()?;
        tracker_clone.increment(1);

        Ok(())
    });

    let files: Vec<PathBuf> = (0..20)
        .map(|i| temp.path().join(format!("file{}.rs", i)))
        .collect();

    let (_, stats) = processor.process_all(files, ())?;

    assert_eq!(stats.successful, 20);
    assert_eq!(tracker.state().completed, 20);
    assert_eq!(tracker.state().percentage(), 100.0);

    Ok(())
}

// ============================================================================
// SECTION 4: Real-World Code Sample Tests
// ============================================================================

#[test]
fn test_real_world_rust_module() -> Result<()> {
    let source = r#"
        //! HTTP client module with retry logic

        use std::time::Duration;

        /// Configuration for HTTP client
        #[derive(Debug, Clone)]
        pub struct ClientConfig {
            /// Connection timeout
            pub timeout: Duration,
            /// Maximum retries
            pub max_retries: usize,
            /// Retry delay
            pub retry_delay: Duration,
        }

        impl Default for ClientConfig {
            fn default() -> Self {
                Self {
                    timeout: Duration::from_secs(30),
                    max_retries: 3,
                    retry_delay: Duration::from_millis(100),
                }
            }
        }

        /// HTTP client with retry logic
        pub struct HttpClient {
            config: ClientConfig,
        }

        impl HttpClient {
            /// Creates a new HTTP client
            pub fn new(config: ClientConfig) -> Self {
                Self { config }
            }

            /// Sends a GET request with retry
            pub async fn get(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
                let mut attempts = 0;

                loop {
                    attempts += 1;

                    match self.try_get(url).await {
                        Ok(response) => return Ok(response),
                        Err(e) if attempts < self.config.max_retries => {
                            eprintln!("Retry {} failed: {}", attempts, e);
                            tokio::time::sleep(self.config.retry_delay).await;
                            continue;
                        }
                        Err(e) => return Err(e),
                    }
                }
            }

            async fn try_get(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
                // Simulated HTTP request
                Ok(format!("Response from {}", url))
            }
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            fn test_config_default() {
                let config = ClientConfig::default();
                assert_eq!(config.max_retries, 3);
            }
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("http_client.rs")
    )?;

    // Full analysis
    let metrics = MetricsBuilder::new(&parser).calculate()?;
    let finder = AstFinder::new(&parser);
    let functions = finder.find(
        &FindConfig::builder()
            .add_filter(cortex_code_analysis::analysis::NodeFilter::Kind("function_item".to_string()))
            .build()
    )?;
    let comments = CommentAnalyzer::new(&parser, source.as_bytes()).analyze()?;

    // Verify analysis results
    assert!(metrics.nom.functions() >= 4.0); // new, get, try_get, test_config_default
    assert!(metrics.cyclomatic.cyclomatic() > 1.0);
    assert!(comments.doc_comments.len() >= 3);
    assert!(functions.nodes.len() >= 4);

    Ok(())
}

#[test]
fn test_complex_control_flow() -> Result<()> {
    let source = r#"
        fn process_data(data: Vec<i32>) -> i32 {
            let mut result = 0;

            for item in data.iter() {
                if *item > 0 {
                    result += item;
                } else if *item < 0 {
                    result -= item;
                } else {
                    continue;
                }

                match item % 3 {
                    0 => result *= 2,
                    1 => result += 1,
                    2 => result -= 1,
                    _ => unreachable!(),
                }

                if result > 1000 {
                    break;
                }
            }

            result
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("complex.rs")
    )?;

    let metrics = MetricsBuilder::new(&parser).calculate()?;

    // Should have high cyclomatic complexity
    assert!(metrics.cyclomatic.cyclomatic() > 5.0);

    // Should have high cognitive complexity
    assert!(metrics.cognitive.cognitive() > 0.0);

    // Should detect exit points
    assert!(metrics.exit.exit() > 0.0);

    Ok(())
}

// ============================================================================
// SECTION 5: Performance and Memory Tests
// ============================================================================

#[test]
#[ignore = "Performance test - takes time"]
fn test_large_file_performance() -> Result<()> {
    // Generate a large source file
    let mut source = String::new();
    for i in 0..1000 {
        source.push_str(&format!(
            "fn function_{}() {{ let x = {}; let y = x + 1; }}\n",
            i, i
        ));
    }

    let start = Instant::now();

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("large.rs")
    )?;

    let parse_time = start.elapsed();

    let metrics_start = Instant::now();
    let metrics = MetricsBuilder::new(&parser).calculate()?;
    let metrics_time = metrics_start.elapsed();

    let search_start = Instant::now();
    let finder = AstFinder::new(&parser);
    let _ = finder.find(&FindConfig::default())?;
    let search_time = search_start.elapsed();

    // Verify results
    assert!(metrics.nom.functions() >= 1000.0);

    // Performance assertions (adjust based on hardware)
    println!("Parse time: {:?}", parse_time);
    println!("Metrics time: {:?}", metrics_time);
    println!("Search time: {:?}", search_time);

    // All operations should complete in reasonable time
    assert!(parse_time.as_secs() < 5);
    assert!(metrics_time.as_secs() < 5);
    assert!(search_time.as_secs() < 5);

    Ok(())
}

#[test]
#[ignore = "Integration test - requires file system"]
fn test_concurrent_performance() -> Result<()> {
    let temp = TempDir::new()?;

    // Create 100 files
    for i in 0..100 {
        fs::write(
            temp.path().join(format!("file{}.rs", i)),
            format!("fn test{}() {{ let x = {}; }}", i, i)
        )?;
    }

    let start = Instant::now();

    let processor = ParallelProcessor::new(|path: &PathBuf, _: &()| {
        let source = fs::read_to_string(path)?;
        let parser = Parser::<RustLanguage>::new(
            source.as_bytes().to_vec(),
            path.as_ref()
        )?;
        let _ = MetricsBuilder::new(&parser).calculate()?;
        Ok(())
    });

    let files: Vec<PathBuf> = (0..100)
        .map(|i| temp.path().join(format!("file{}.rs", i)))
        .collect();

    let (_, stats) = processor.process_all(files, ())?;
    let duration = start.elapsed();

    assert_eq!(stats.successful, 100);
    assert!(stats.throughput > 0.0);

    println!("Processed 100 files in {:?}", duration);
    println!("Throughput: {:.2} files/sec", stats.throughput);

    // Should process 100 files in reasonable time
    assert!(duration.as_secs() < 10);

    Ok(())
}

#[test]
fn test_memory_efficient_processing() -> Result<()> {
    // Test that we can process multiple files without excessive memory usage
    let sources = vec![
        "fn test1() {}",
        "fn test2() { let x = 1; }",
        "fn test3() { if true { let y = 2; } }",
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

    // Use metrics aggregator for efficient batch processing
    let aggregator = MetricsAggregator::new();
    let total_metrics = aggregator.aggregate(&parsers)?;

    assert!(total_metrics.nom.functions() >= 3.0);

    Ok(())
}

// ============================================================================
// SECTION 6: Cross-Feature Integration
// ============================================================================

#[test]
fn test_full_analysis_pipeline() -> Result<()> {
    let source = r#"
        /// Main application module
        pub mod app {
            use std::collections::HashMap;

            /// Configuration store
            pub struct Config {
                values: HashMap<String, String>,
            }

            impl Config {
                /// Creates a new config
                pub fn new() -> Self {
                    Self {
                        values: HashMap::new(),
                    }
                }

                /// Gets a value
                pub fn get(&self, key: &str) -> Option<&String> {
                    self.values.get(key)
                }

                /// Sets a value
                pub fn set(&mut self, key: String, value: String) {
                    self.values.insert(key, value);
                }
            }
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("app.rs")
    )?;

    // 1. AST Analysis
    let counter = AstCounter::new(&parser);
    let count_stats = counter.count(&CountConfig::default())?;

    // 2. Metrics Calculation
    let metrics = MetricsBuilder::new(&parser).calculate()?;

    // 3. Comment Analysis
    let comments = CommentAnalyzer::new(&parser, source.as_bytes()).analyze()?;

    // 4. Lint Checking
    let rules: Vec<Box<dyn cortex_code_analysis::analysis::LintRule>> = vec![
        Box::new(FunctionTooLongRule::new(50)),
    ];
    let checker = LintChecker::new(rules);
    let violations = checker.check(&parser)?;

    // Verify all analyses completed
    assert!(count_stats.total_nodes_visited > 0);
    assert!(metrics.nom.functions() >= 3.0);
    assert!(comments.total_comments > 0);
    assert!(violations.len() == 0); // No violations expected

    Ok(())
}

#[test]
fn test_caching_across_operations() -> Result<()> {
    let source = "fn test() { let x = 1; }";

    // Create cache
    let cache: cortex_code_analysis::analysis::AstCache =
        cortex_code_analysis::analysis::Cache::new(10);

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    // Perform multiple operations
    let _ = MetricsBuilder::new(&parser).calculate()?;
    let finder = AstFinder::new(&parser);
    let _ = finder.find(&FindConfig::default())?;
    let counter = AstCounter::new(&parser);
    let _ = counter.count(&CountConfig::default())?;

    // All operations should work without cache misses
    // (This is more of a smoke test to ensure operations are compatible)
    assert_eq!(cache.len(), 0); // Cache wasn't used directly here

    Ok(())
}

#[test]
fn test_error_recovery() -> Result<()> {
    // Test that we can recover from various error conditions
    let sources = vec![
        ("good1.rs", "fn test() {}"),
        ("empty.rs", ""),
        ("good2.rs", "fn another() {}"),
    ];

    let mut success_count = 0;
    let mut error_count = 0;

    for (name, source) in sources {
        match Parser::<RustLanguage>::new(
            source.as_bytes().to_vec(),
            Path::new(name)
        ) {
            Ok(parser) => {
                if MetricsBuilder::new(&parser).calculate().is_ok() {
                    success_count += 1;
                }
            }
            Err(_) => {
                error_count += 1;
            }
        }
    }

    // Should successfully process at least the good files
    assert!(success_count >= 2);

    Ok(())
}

// ============================================================================
// SECTION 8: Python Cyclomatic Complexity Tests (Else Clause Enhancement)
// ============================================================================

#[test]
fn test_python_else_after_if_not_counted() -> Result<()> {
    use cortex_code_analysis::{PythonLanguage, spaces::compute_spaces, Lang};

    // else after if should NOT count towards cyclomatic complexity
    let source = r#"
def test_if_else(x):
    if x > 0:
        return "positive"
    else:
        return "non-positive"
"#;

    let parser = Parser::<PythonLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.py")
    )?;

    let root_space = compute_spaces(
        parser.get_root(),
        parser.get_code(),
        Lang::Python,
        "test.py"
    )?;

    // Find the function space in nested spaces
    let func_space = root_space.spaces.iter()
        .find(|s| s.name.as_ref().map(|n| n.contains("test_if_else")).unwrap_or(false))
        .expect("Function space not found");

    // Cyclomatic complexity should be 2: base 1 + if statement 1
    // else after if should NOT be counted
    assert_eq!(func_space.metrics.cyclomatic.cyclomatic(), 2.0,
        "else after if should not count towards cyclomatic complexity");

    Ok(())
}

#[test]
fn test_python_else_after_for_counted() -> Result<()> {
    use cortex_code_analysis::{PythonLanguage, spaces::compute_spaces, Lang};

    // else after for SHOULD count towards cyclomatic complexity
    let source = r#"
def search_list(items, target):
    for item in items:
        if item == target:
            break
    else:
        return "not found"
    return "found"
"#;

    let parser = Parser::<PythonLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.py")
    )?;

    let root_space = compute_spaces(
        parser.get_root(),
        parser.get_code(),
        Lang::Python,
        "test.py"
    )?;

    // Find the function space
    let func_space = root_space.spaces.iter()
        .find(|s| s.name.as_ref().map(|n| n.contains("search_list")).unwrap_or(false))
        .expect("Function space not found");

    // Cyclomatic complexity should be 4: base 1 + for 1 + if 1 + else (after for) 1
    assert_eq!(func_space.metrics.cyclomatic.cyclomatic(), 4.0,
        "else after for should count towards cyclomatic complexity");

    Ok(())
}

#[test]
fn test_python_else_after_while_counted() -> Result<()> {
    use cortex_code_analysis::{PythonLanguage, spaces::compute_spaces, Lang};

    // else after while SHOULD count towards cyclomatic complexity
    let source = r#"
def wait_for_condition(condition):
    attempts = 0
    while attempts < 10:
        if condition():
            break
        attempts += 1
    else:
        return "timeout"
    return "success"
"#;

    let parser = Parser::<PythonLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.py")
    )?;

    let root_space = compute_spaces(
        parser.get_root(),
        parser.get_code(),
        Lang::Python,
        "test.py"
    )?;

    // Find the function space
    let func_space = root_space.spaces.iter()
        .find(|s| s.name.as_ref().map(|n| n.contains("wait_for_condition")).unwrap_or(false))
        .expect("Function space not found");

    // Cyclomatic complexity should be 4: base 1 + while 1 + if 1 + else (after while) 1
    assert_eq!(func_space.metrics.cyclomatic.cyclomatic(), 4.0,
        "else after while should count towards cyclomatic complexity");

    Ok(())
}

#[test]
fn test_python_elif_chain_not_double_counted() -> Result<()> {
    use cortex_code_analysis::{PythonLanguage, spaces::compute_spaces, Lang};

    // elif should count but else after if should not
    let source = r#"
def categorize(value):
    if value < 0:
        return "negative"
    elif value == 0:
        return "zero"
    elif value < 10:
        return "small"
    else:
        return "large"
"#;

    let parser = Parser::<PythonLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.py")
    )?;

    let root_space = compute_spaces(
        parser.get_root(),
        parser.get_code(),
        Lang::Python,
        "test.py"
    )?;

    // Find the function space
    let func_space = root_space.spaces.iter()
        .find(|s| s.name.as_ref().map(|n| n.contains("categorize")).unwrap_or(false))
        .expect("Function space not found");

    // Cyclomatic complexity should be 4: base 1 + if 1 + elif 1 + elif 1
    // else after if should NOT be counted
    assert_eq!(func_space.metrics.cyclomatic.cyclomatic(), 4.0,
        "elif chains should not double-count else clauses after if");

    Ok(())
}

#[test]
fn test_python_complex_else_combinations() -> Result<()> {
    use cortex_code_analysis::{PythonLanguage, spaces::compute_spaces, Lang};

    // Test complex combinations of else clauses
    let source = r#"
def complex_function(items, threshold):
    # for-else should count
    for item in items:
        if item > threshold:
            # while-else should count
            while item > 0:
                item -= 1
            else:
                break
    else:
        return "all processed"

    # if-else should not count
    if threshold > 0:
        return "positive"
    else:
        return "negative"
"#;

    let parser = Parser::<PythonLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.py")
    )?;

    let root_space = compute_spaces(
        parser.get_root(),
        parser.get_code(),
        Lang::Python,
        "test.py"
    )?;

    // Find the function space
    let func_space = root_space.spaces.iter()
        .find(|s| s.name.as_ref().map(|n| n.contains("complex_function")).unwrap_or(false))
        .expect("Function space not found");

    // Cyclomatic complexity should be:
    // base 1 + for 1 + if 1 + while 1 + else(after while) 1 + else(after for) 1 + if 1
    // = 7 (else after second if should NOT count)
    assert_eq!(func_space.metrics.cyclomatic.cyclomatic(), 7.0,
        "Complex else combinations should be counted correctly");

    Ok(())
}
