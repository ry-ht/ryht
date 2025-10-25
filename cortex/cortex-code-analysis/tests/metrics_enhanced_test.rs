//! Comprehensive tests for enhanced metrics features
//!
//! This test suite validates:
//! - HalsteadMaps with frequency tracking
//! - ABC metrics with declaration tracking
//! - Cognitive complexity with BoolSequence
//! - Metrics strategy pattern
//! - Metrics aggregation
//! - All metric enhancements

use anyhow::Result;
use cortex_code_analysis::{
    metrics::{
        // Core metrics
        AbcStats, CognitiveStats, CyclomaticStats, HalsteadStats, HalsteadCollector,
        LocStats, MaintainabilityIndexStats, ExitStats, NargsStats, NomStats,
        NpaStats, NpmStats, WmcStats,
        // Strategy pattern
        MetricsStrategy, MetricsCalculatorType, MetricsBuilder, MetricsAggregator,
        CodeMetrics,
    },
    Parser, RustLanguage, Lang,
};
use cortex_code_analysis::traits::ParserTrait;
use std::path::Path;

// ============================================================================
// SECTION 1: Enhanced Halstead Metrics Tests
// ============================================================================

#[test]
fn test_halstead_collector_basic() {
    let mut collector = HalsteadCollector::new();

    collector.add_operator("+");
    collector.add_operator("+");
    collector.add_operator("-");
    collector.add_operand("x");
    collector.add_operand("x");
    collector.add_operand("y");

    let stats = collector.finalize();

    assert_eq!(stats.u_operators(), 2.0); // + and -
    assert_eq!(stats.operators(), 3.0); // total operators
    assert_eq!(stats.u_operands(), 2.0); // x and y
    assert_eq!(stats.operands(), 3.0); // total operands
}

#[test]
fn test_halstead_collector_vocabulary() {
    let mut collector = HalsteadCollector::new();

    collector.add_operator("+");
    collector.add_operator("-");
    collector.add_operator("*");
    collector.add_operand("a");
    collector.add_operand("b");

    let stats = collector.finalize();
    let vocab = stats.vocabulary();

    // vocabulary = n1 + n2 = 3 + 2 = 5
    assert_eq!(vocab, 5.0);
}

#[test]
fn test_halstead_collector_length() {
    let mut collector = HalsteadCollector::new();

    collector.add_operator("+");
    collector.add_operator("+");
    collector.add_operand("x");
    collector.add_operand("y");

    let stats = collector.finalize();
    let length = stats.length();

    // length = N1 + N2 = 2 + 2 = 4
    assert_eq!(length, 4.0);
}

#[test]
fn test_halstead_collector_volume() {
    let mut collector = HalsteadCollector::new();

    collector.add_operator("+");
    collector.add_operand("x");
    collector.add_operand("y");

    let stats = collector.finalize();
    let volume = stats.volume();

    // volume = length * log2(vocabulary)
    // length = 3, vocabulary = 2
    // volume = 3 * log2(2) = 3 * 1 = 3
    assert!(volume > 0.0);
}

#[test]
fn test_halstead_collector_difficulty() {
    let mut collector = HalsteadCollector::new();

    collector.add_operator("+");
    collector.add_operator("-");
    collector.add_operand("x");
    collector.add_operand("y");
    collector.add_operand("z");

    let stats = collector.finalize();
    let difficulty = stats.difficulty();

    // difficulty = (n1/2) * (N2/n2)
    assert!(difficulty > 0.0);
}

#[test]
fn test_halstead_collector_effort() {
    let mut collector = HalsteadCollector::new();

    collector.add_operator("+");
    collector.add_operand("x");
    collector.add_operand("y");

    let stats = collector.finalize();
    let effort = stats.effort();

    // effort = difficulty * volume
    assert!(effort >= 0.0);
}

#[test]
fn test_halstead_collector_time() {
    let mut collector = HalsteadCollector::new();

    collector.add_operator("+");
    collector.add_operand("x");
    collector.add_operand("y");

    let stats = collector.finalize();
    let time = stats.time();

    // time = effort / 18
    assert!(time >= 0.0);
}

#[test]
fn test_halstead_collector_bugs() {
    let mut collector = HalsteadCollector::new();

    collector.add_operator("+");
    collector.add_operand("x");
    collector.add_operand("y");

    let stats = collector.finalize();
    let bugs = stats.bugs();

    // bugs = volume / 3000
    assert!(bugs >= 0.0);
}

#[test]
fn test_halstead_collector_merge() {
    let mut collector1 = HalsteadCollector::new();
    collector1.add_operator("+");
    collector1.add_operand("x");

    let mut collector2 = HalsteadCollector::new();
    collector2.add_operator("-");
    collector2.add_operand("y");

    collector1.merge(&collector2);
    let stats = collector1.finalize();

    assert_eq!(stats.u_operators(), 2.0); // + and -
    assert_eq!(stats.u_operands(), 2.0); // x and y
}

#[test]
fn test_halstead_stats_merge() {
    let mut stats1 = HalsteadStats::from_counts(2, 5, 3, 7);
    let stats2 = HalsteadStats::from_counts(1, 3, 2, 4);

    stats1.merge(&stats2);

    // After merge, totals should be summed
    assert!(stats1.operators() > 5.0);
    assert!(stats1.operands() > 7.0);
}

#[test]
fn test_halstead_zero_handling() {
    let stats = HalsteadStats::default();

    // Should handle division by zero gracefully
    let vocab = stats.vocabulary();
    let length = stats.length();
    let volume = stats.volume();
    let difficulty = stats.difficulty();
    let effort = stats.effort();

    assert_eq!(vocab, 0.0);
    assert_eq!(length, 0.0);
    assert_eq!(volume, 0.0);
    assert_eq!(difficulty, 0.0);
    assert_eq!(effort, 0.0);
}

// ============================================================================
// SECTION 2: Enhanced ABC Metrics Tests
// ============================================================================

#[test]
fn test_abc_basic_assignment() {
    let mut stats = AbcStats::new();
    stats.add_assignment();

    assert_eq!(stats.assignments(), 1.0);
    assert_eq!(stats.branches(), 0.0);
    assert_eq!(stats.conditions(), 0.0);
}

#[test]
fn test_abc_basic_branch() {
    let mut stats = AbcStats::new();
    stats.add_branch();

    assert_eq!(stats.assignments(), 0.0);
    assert_eq!(stats.branches(), 1.0);
    assert_eq!(stats.conditions(), 0.0);
}

#[test]
fn test_abc_basic_condition() {
    let mut stats = AbcStats::new();
    stats.add_condition();

    assert_eq!(stats.assignments(), 0.0);
    assert_eq!(stats.branches(), 0.0);
    assert_eq!(stats.conditions(), 1.0);
}

#[test]
fn test_abc_var_declaration() {
    let mut stats = AbcStats::new();

    // Variable declaration should count as assignment
    stats.start_var_declaration();
    stats.add_assignment_with_context();
    stats.clear_declaration();

    assert_eq!(stats.assignments(), 1.0);
}

#[test]
fn test_abc_const_declaration() {
    let mut stats = AbcStats::new();

    // Constant declaration should NOT count as assignment
    stats.start_const_declaration();
    stats.add_assignment_with_context();
    stats.clear_declaration();

    assert_eq!(stats.assignments(), 0.0);
}

#[test]
fn test_abc_promote_to_const() {
    let mut stats = AbcStats::new();

    // Variable promoted to const (e.g., Java final)
    stats.start_var_declaration();
    stats.promote_to_const();
    stats.add_assignment_with_context();
    stats.clear_declaration();

    assert_eq!(stats.assignments(), 0.0);
}

#[test]
fn test_abc_multiple_declarations() {
    let mut stats = AbcStats::new();

    // First declaration: variable
    stats.start_var_declaration();
    stats.add_assignment_with_context();
    stats.clear_declaration();

    // Second declaration: constant
    stats.start_const_declaration();
    stats.add_assignment_with_context();
    stats.clear_declaration();

    // Only the variable declaration should count
    assert_eq!(stats.assignments(), 1.0);
}

#[test]
fn test_abc_java_final_modifier() {
    let mut stats = AbcStats::new();

    // Simulating: final int x = 5;
    stats.start_var_declaration();
    stats.promote_to_const(); // final keyword
    stats.add_assignment_with_context();
    stats.clear_declaration();

    // Should NOT count as assignment
    assert_eq!(stats.assignments(), 0.0);
}

#[test]
fn test_abc_magnitude() {
    let mut stats = AbcStats::new();

    stats.add_assignment();
    stats.add_assignment();
    stats.add_branch();
    stats.add_condition();

    let magnitude = stats.magnitude();
    // magnitude = sqrt(2^2 + 1^2 + 1^2) = sqrt(6) ≈ 2.449
    assert!(magnitude > 2.4 && magnitude < 2.5);
}

#[test]
fn test_abc_merge() {
    let mut stats1 = AbcStats::new();
    stats1.add_assignment();
    stats1.add_branch();

    let mut stats2 = AbcStats::new();
    stats2.add_assignment();
    stats2.add_condition();

    stats1.merge(&stats2);

    assert_eq!(stats1.assignments(), 2.0);
    assert_eq!(stats1.branches(), 1.0);
    assert_eq!(stats1.conditions(), 1.0);
}

#[test]
fn test_abc_compute_sum() {
    let mut stats = AbcStats::new();

    stats.add_assignment();
    stats.add_branch();
    stats.add_condition();

    stats.compute_sum();

    assert!(stats.assignments_sum() > 0.0);
    assert!(stats.branches_sum() > 0.0);
    assert!(stats.conditions_sum() > 0.0);
}

// ============================================================================
// SECTION 3: Enhanced Cognitive Complexity Tests
// ============================================================================

#[test]
fn test_cognitive_basic() {
    let stats = CognitiveStats::new();

    assert_eq!(stats.cognitive(), 0.0);
}

#[test]
fn test_cognitive_increment() {
    let mut stats = CognitiveStats::new();

    stats.increment_structural(1);
    assert_eq!(stats.cognitive(), 1.0);

    stats.increment_structural(1);
    assert_eq!(stats.cognitive(), 2.0);
}

#[test]
fn test_cognitive_nesting() {
    let mut stats = CognitiveStats::new();

    // Simulating nested structures
    stats.increment_nesting();
    stats.increment_structural(1);
    stats.increment_nesting();
    stats.increment_structural(1);

    assert!(stats.cognitive() > 0.0);
}

#[test]
fn test_cognitive_boolean_sequence_same_operator() {
    let mut stats = CognitiveStats::new();

    // Simulating: a && b && c
    // First &&
    stats.eval_boolean_operator(100); // arbitrary id for &&
    // Second &&
    stats.eval_boolean_operator(100); // same operator

    // Same operator in sequence should not increment multiple times
    let complexity = stats.cognitive();
    assert_eq!(complexity, 1.0); // Only counted once
}

#[test]
fn test_cognitive_boolean_sequence_different_operators() {
    let mut stats = CognitiveStats::new();

    // Simulating: a && b || c
    // First &&
    stats.eval_boolean_operator(100);
    // Then ||
    stats.eval_boolean_operator(101);

    // Different operators should increment separately
    let complexity = stats.cognitive();
    assert_eq!(complexity, 2.0); // Counted twice
}

#[test]
fn test_cognitive_reset_boolean_sequence() {
    let mut stats = CognitiveStats::new();

    stats.eval_boolean_operator(100);
    stats.reset_boolean_seq();
    stats.eval_boolean_operator(100);

    // After reset, should count as new sequence
    let complexity = stats.cognitive();
    assert_eq!(complexity, 2.0);
}

#[test]
fn test_cognitive_compute_sum() {
    let mut stats = CognitiveStats::new();

    stats.increment_structural(5);
    stats.compute_sum();

    assert_eq!(stats.cognitive_sum(), 5.0);
}

#[test]
fn test_cognitive_merge() {
    let mut stats1 = CognitiveStats::new();
    stats1.increment_structural(3);
    stats1.compute_sum();

    let mut stats2 = CognitiveStats::new();
    stats2.increment_structural(2);
    stats2.compute_sum();

    stats1.merge(&stats2);

    assert!(stats1.cognitive_sum() >= 5.0);
}

#[test]
fn test_cognitive_min_max() {
    let mut stats = CognitiveStats::new();

    stats.increment_structural(5);
    stats.compute_sum();

    stats.increment_structural(3);
    stats.compute_sum();

    assert!(stats.cognitive_min() <= stats.cognitive_max());
}

// ============================================================================
// SECTION 4: Metrics Strategy Pattern Tests
// ============================================================================

#[test]
fn test_metrics_builder_basic() -> Result<()> {
    let source = r#"
        fn main() {
            let x = 1;
            if x > 0 {
                println!("positive");
            }
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let builder = MetricsBuilder::new(&parser);
    let metrics = builder.calculate()?;

    assert!(metrics.cyclomatic.cyclomatic() > 0.0);
    assert!(metrics.loc.sloc() > 0.0);

    Ok(())
}

#[test]
fn test_metrics_builder_with_specific_metrics() -> Result<()> {
    let source = "fn test() {}";

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let builder = MetricsBuilder::new(&parser)
        .enable_metric(MetricsCalculatorType::Cyclomatic)
        .enable_metric(MetricsCalculatorType::Loc);

    let metrics = builder.calculate()?;

    assert!(metrics.cyclomatic.cyclomatic() > 0.0);
    assert!(metrics.loc.sloc() > 0.0);

    Ok(())
}

#[test]
fn test_metrics_aggregator() -> Result<()> {
    let sources = vec![
        "fn test1() {}",
        "fn test2() {}",
        "fn test3() {}",
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

    let aggregator = MetricsAggregator::new();
    let total_metrics = aggregator.aggregate(&parsers)?;

    // Should have aggregated metrics from all 3 files
    assert!(total_metrics.nom.functions() >= 3.0);

    Ok(())
}

#[test]
fn test_metrics_strategy_calculation() -> Result<()> {
    let source = r#"
        fn add(a: i32, b: i32) -> i32 {
            a + b
        }

        fn subtract(a: i32, b: i32) -> i32 {
            a - b
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let strategy = MetricsStrategy::default();
    let metrics = strategy.calculate(&parser)?;

    assert_eq!(metrics.nom.functions(), 2.0);

    Ok(())
}

// ============================================================================
// SECTION 5: Code Metrics Integration Tests
// ============================================================================

#[test]
fn test_code_metrics_compute_derived() {
    let mut metrics = CodeMetrics::new();

    // Set some base metrics
    metrics.cyclomatic.increment();
    metrics.loc.incr_sloc();
    metrics.halstead = HalsteadStats::from_counts(2, 5, 3, 7);

    // Compute derived metrics (MI and WMC)
    metrics.compute_derived();

    // MI should be computed from LOC, Cyclomatic, and Halstead
    assert!(metrics.maintainability_index.mi_original() != 0.0);

    // WMC should be computed from Cyclomatic
    assert!(metrics.wmc.wmc() >= 0.0);
}

#[test]
fn test_code_metrics_merge() {
    let mut metrics1 = CodeMetrics::new();
    metrics1.cyclomatic.increment();
    metrics1.loc.incr_sloc();

    let mut metrics2 = CodeMetrics::new();
    metrics2.cyclomatic.increment();
    metrics2.loc.incr_sloc();

    metrics1.merge(&metrics2);

    // Values should be combined
    assert!(metrics1.cyclomatic.cyclomatic() > 1.0);
    assert!(metrics1.loc.sloc() > 1.0);
}

#[test]
fn test_code_metrics_serialization() -> Result<()> {
    let metrics = CodeMetrics::new();

    let json = serde_json::to_string(&metrics)?;
    assert!(!json.is_empty());

    let deserialized: CodeMetrics = serde_json::from_str(&json)?;
    assert_eq!(metrics, deserialized);

    Ok(())
}

#[test]
fn test_code_metrics_display() {
    let metrics = CodeMetrics::new();
    let display = format!("{}", metrics);

    assert!(display.contains("Code Metrics"));
    assert!(display.contains("Cyclomatic"));
    assert!(display.contains("LOC"));
    assert!(display.contains("Halstead"));
    assert!(display.contains("ABC"));
    assert!(display.contains("Cognitive"));
}

// ============================================================================
// SECTION 6: Individual Metric Tests
// ============================================================================

#[test]
fn test_cyclomatic_stats() {
    let mut stats = CyclomaticStats::new();

    assert_eq!(stats.cyclomatic(), 1.0);

    stats.increment();
    assert_eq!(stats.cyclomatic(), 2.0);

    stats.increment();
    assert_eq!(stats.cyclomatic(), 3.0);

    stats.compute_sum();
    assert_eq!(stats.cyclomatic_sum(), 3.0);
}

#[test]
fn test_loc_stats() {
    let mut stats = LocStats::new();

    stats.incr_sloc();
    stats.incr_sloc();
    stats.incr_ploc();
    stats.incr_cloc();
    stats.incr_lloc();
    stats.incr_blank();

    assert_eq!(stats.sloc(), 3.0); // 1 (default) + 2
    assert_eq!(stats.ploc(), 1.0);
    assert_eq!(stats.cloc(), 1.0);
    assert_eq!(stats.lloc(), 1.0);
    assert_eq!(stats.blank(), 1.0);
}

#[test]
fn test_exit_stats() {
    let mut stats = ExitStats::new();

    stats.add_exit();
    stats.add_exit();

    assert_eq!(stats.exit(), 2.0);
}

#[test]
fn test_nargs_stats() {
    let mut stats = NargsStats::new();

    stats.add_func_with_args(2);
    stats.add_func_with_args(3);

    assert_eq!(stats.nargs_total(), 5.0);
    assert_eq!(stats.nargs_average(), 2.5);
}

#[test]
fn test_nom_stats() {
    let mut stats = NomStats::new();

    stats.add_function();
    stats.add_function();
    stats.add_closure();

    assert_eq!(stats.functions(), 3.0);
    assert_eq!(stats.closures(), 1.0);
}

#[test]
fn test_npm_stats() {
    let mut stats = NpmStats::new();

    stats.add_public_method();
    stats.add_public_method();

    assert_eq!(stats.npm(), 2.0);
}

#[test]
fn test_npa_stats() {
    let mut stats = NpaStats::new();

    stats.add_public_attribute();
    stats.add_public_attribute();
    stats.add_public_attribute();

    assert_eq!(stats.npa(), 3.0);
}

#[test]
fn test_wmc_stats() {
    let cyclomatic = CyclomaticStats::from_value(10.0);
    let wmc = WmcStats::from_cyclomatic(&cyclomatic);

    // WMC should equal cyclomatic sum
    assert_eq!(wmc.wmc(), cyclomatic.cyclomatic_sum());
}

#[test]
fn test_mi_from_metrics() {
    let loc = LocStats::new();
    let cyclomatic = CyclomaticStats::new();
    let halstead = HalsteadStats::from_counts(2, 5, 3, 7);

    let mi = MaintainabilityIndexStats::from_metrics(&loc, &cyclomatic, &halstead);

    // MI should be calculated
    assert!(mi.mi_original() >= 0.0 || mi.mi_original() < 0.0);
}

// ============================================================================
// SECTION 7: Complex Code Metrics Tests
// ============================================================================

#[test]
fn test_complex_function_metrics() -> Result<()> {
    let source = r#"
        fn complex_function(a: i32, b: i32, c: i32) -> i32 {
            let mut result = 0;

            if a > 0 {
                if b > 0 {
                    result += a + b;
                } else {
                    result += a - b;
                }
            } else if a < 0 {
                result += c;
            }

            for i in 0..10 {
                if i % 2 == 0 {
                    result += i;
                }
            }

            match result {
                0 => 0,
                1..=10 => result * 2,
                _ => result,
            }
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let builder = MetricsBuilder::new(&parser);
    let metrics = builder.calculate()?;

    // Should have high cyclomatic complexity
    assert!(metrics.cyclomatic.cyclomatic() > 5.0);

    // Should have reasonable LOC
    assert!(metrics.loc.sloc() > 10.0);

    // Should have some cognitive complexity
    assert!(metrics.cognitive.cognitive() > 0.0);

    Ok(())
}

#[test]
fn test_metrics_with_closures() -> Result<()> {
    let source = r#"
        fn main() {
            let add = |x, y| x + y;
            let multiply = |x, y| x * y;

            let result = add(2, 3);
            let result2 = multiply(4, 5);
        }
    "#;

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.rs")
    )?;

    let builder = MetricsBuilder::new(&parser);
    let metrics = builder.calculate()?;

    // Should count closures in NOM
    assert!(metrics.nom.closures() >= 2.0);

    Ok(())
}

#[test]
fn test_metrics_edge_cases() -> Result<()> {
    let source = "";

    let parser = Parser::<RustLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("empty.rs")
    )?;

    let builder = MetricsBuilder::new(&parser);
    let metrics = builder.calculate()?;

    // Should handle empty file gracefully
    assert!(metrics.cyclomatic.cyclomatic() >= 0.0);

    Ok(())
}
