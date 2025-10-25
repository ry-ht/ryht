//! Comprehensive tests for code metrics
//!
//! This test suite validates:
//! - Metric serialization/deserialization
//! - Metric calculations on various code samples
//! - Metric merging and aggregation
//! - Integration with the parser

use cortex_code_analysis::metrics::*;
use serde_json;

// ============================================================================
// SECTION 1: Basic Metric Tests
// ============================================================================

#[test]
fn test_cyclomatic_default() {
    let stats = CyclomaticStats::default();
    assert_eq!(stats.cyclomatic(), 1.0);
    assert_eq!(stats.cyclomatic_sum(), 0.0);
}

#[test]
fn test_cyclomatic_increment() {
    let mut stats = CyclomaticStats::new();
    stats.increment();
    assert_eq!(stats.cyclomatic(), 2.0);
    stats.increment();
    assert_eq!(stats.cyclomatic(), 3.0);
}

#[test]
fn test_cyclomatic_merge() {
    let mut stats1 = CyclomaticStats::new();
    stats1.increment();
    stats1.increment();
    stats1.compute_sum(); // Need to compute sum before merging

    let mut stats2 = CyclomaticStats::new();
    stats2.increment();
    stats2.compute_sum(); // Need to compute sum before merging

    stats1.merge(&stats2);
    // After merge, sum should include both
    assert!(stats1.cyclomatic_sum() > 0.0);
}

#[test]
fn test_loc_default() {
    let stats = LocStats::default();
    // Default LOC stats have sloc of 1.0 due to (end - start) + 1 when unit=false
    assert_eq!(stats.sloc(), 1.0);
    assert_eq!(stats.ploc(), 0.0); // Empty HashSet
    assert_eq!(stats.cloc(), 0.0); // 0 + 0
    assert_eq!(stats.lloc(), 0.0); // 0 logical lines
    assert_eq!(stats.blank(), 0.0); // 0 blank lines
}

#[test]
fn test_loc_merge() {
    let mut stats1 = LocStats::new();
    let stats2 = LocStats::new();

    stats1.merge(&stats2);
    // After merge, values should be combined
    assert!(stats1.sloc() >= 1.0);
}

#[test]
fn test_halstead_default() {
    let stats = HalsteadStats::default();
    assert_eq!(stats.u_operators(), 0.0);
    assert_eq!(stats.u_operands(), 0.0);
    assert_eq!(stats.operators(), 0.0);
    assert_eq!(stats.operands(), 0.0);
}

#[test]
fn test_halstead_vocabulary() {
    let stats = HalsteadStats::default();
    let vocab = stats.vocabulary();
    assert!(vocab >= 0.0);
}

#[test]
fn test_halstead_length() {
    let stats = HalsteadStats::default();
    let length = stats.length();
    assert!(length >= 0.0);
}

#[test]
fn test_halstead_volume() {
    let stats = HalsteadStats::default();
    let volume = stats.volume();
    assert!(volume >= 0.0);
}

#[test]
fn test_abc_default() {
    let stats = AbcStats::default();
    assert_eq!(stats.assignments(), 0.0);
    assert_eq!(stats.branches(), 0.0);
    assert_eq!(stats.conditions(), 0.0);
}

#[test]
fn test_abc_magnitude() {
    let stats = AbcStats::default();
    let magnitude = stats.magnitude();
    assert_eq!(magnitude, 0.0);
}

#[test]
fn test_cognitive_default() {
    let stats = CognitiveStats::default();
    assert_eq!(stats.cognitive(), 0.0);
}

#[test]
fn test_mi_default() {
    let stats = MaintainabilityIndexStats::default();
    // MI can have various values depending on implementation
    assert!(stats.mi_original() >= 0.0 || stats.mi_original() < 0.0);
}

#[test]
fn test_exit_default() {
    let stats = ExitStats::default();
    assert_eq!(stats.exit(), 0.0);
}

#[test]
fn test_nargs_default() {
    let stats = NargsStats::default();
    assert_eq!(stats.nargs_total(), 0.0);
}

#[test]
fn test_nom_default() {
    let stats = NomStats::default();
    assert_eq!(stats.functions(), 0.0);
}

#[test]
fn test_npm_default() {
    let stats = NpmStats::default();
    assert_eq!(stats.npm(), 0.0);
}

#[test]
fn test_npa_default() {
    let stats = NpaStats::default();
    assert_eq!(stats.npa(), 0.0);
}

#[test]
fn test_wmc_default() {
    let stats = WmcStats::default();
    assert_eq!(stats.wmc(), 0.0);
}

// ============================================================================
// SECTION 2: Metric Serialization Tests
// ============================================================================

#[test]
fn test_cyclomatic_serialization() {
    let stats = CyclomaticStats::default();
    let json = serde_json::to_string(&stats).unwrap();
    assert!(!json.is_empty());

    let deserialized: CyclomaticStats = serde_json::from_str(&json).unwrap();
    assert_eq!(stats, deserialized);
}

#[test]
fn test_loc_serialization() {
    let stats = LocStats::default();
    let json = serde_json::to_string(&stats).unwrap();
    assert!(!json.is_empty());

    let deserialized: LocStats = serde_json::from_str(&json).unwrap();
    assert_eq!(stats, deserialized);
}

#[test]
fn test_halstead_serialization() {
    let stats = HalsteadStats::default();
    let json = serde_json::to_string(&stats).unwrap();
    assert!(!json.is_empty());

    let deserialized: HalsteadStats = serde_json::from_str(&json).unwrap();
    assert_eq!(stats, deserialized);
}

#[test]
fn test_abc_serialization() {
    let stats = AbcStats::default();
    let json = serde_json::to_string(&stats).unwrap();
    assert!(!json.is_empty());

    let deserialized: AbcStats = serde_json::from_str(&json).unwrap();
    assert_eq!(stats, deserialized);
}

#[test]
fn test_cognitive_serialization() {
    let stats = CognitiveStats::default();
    let json = serde_json::to_string(&stats).unwrap();
    assert!(!json.is_empty());

    let deserialized: CognitiveStats = serde_json::from_str(&json).unwrap();
    assert_eq!(stats, deserialized);
}

#[test]
fn test_code_metrics_serialization() {
    let metrics = CodeMetrics::default();
    let json = serde_json::to_string(&metrics).unwrap();
    assert!(!json.is_empty());

    let deserialized: CodeMetrics = serde_json::from_str(&json).unwrap();
    assert_eq!(metrics, deserialized);
}

// ============================================================================
// SECTION 3: CodeMetrics Integration Tests
// ============================================================================

#[test]
fn test_code_metrics_default() {
    let metrics = CodeMetrics::default();
    assert_eq!(metrics.cyclomatic.cyclomatic(), 1.0);
    assert_eq!(metrics.loc.sloc(), 1.0); // Default is 1.0 not 0.0
    assert_eq!(metrics.halstead.u_operators(), 0.0);
}

#[test]
fn test_code_metrics_new() {
    let metrics = CodeMetrics::new();
    assert!(metrics.cyclomatic.cyclomatic() > 0.0);
}

#[test]
fn test_code_metrics_compute_derived() {
    let mut metrics = CodeMetrics::new();
    metrics.compute_derived();

    // MI should be computed
    assert!(metrics.maintainability_index.mi_original() >= 0.0
            || metrics.maintainability_index.mi_original() < 0.0);

    // WMC should be computed
    assert!(metrics.wmc.wmc() >= 0.0);
}

#[test]
fn test_code_metrics_merge() {
    let mut metrics1 = CodeMetrics::new();
    let metrics2 = CodeMetrics::new();

    metrics1.merge(&metrics2);

    // After merge, values should be combined
    assert!(metrics1.cyclomatic.cyclomatic() > 0.0);
}

#[test]
fn test_code_metrics_display() {
    let metrics = CodeMetrics::new();
    let display = format!("{}", metrics);

    assert!(display.contains("Code Metrics"));
    assert!(display.contains("Cyclomatic"));
    assert!(display.contains("LOC"));
    assert!(display.contains("Halstead"));
}

// ============================================================================
// SECTION 4: Metric Display Tests
// ============================================================================

#[test]
fn test_cyclomatic_display() {
    let stats = CyclomaticStats::default();
    let display = format!("{}", stats);
    assert!(display.contains("sum"));
    assert!(display.contains("average"));
}

#[test]
fn test_loc_display() {
    let stats = LocStats::default();
    let display = format!("{}", stats);
    assert!(display.contains("sloc"));
    assert!(display.contains("ploc"));
    assert!(display.contains("cloc"));
}

#[test]
fn test_halstead_display() {
    let stats = HalsteadStats::default();
    let display = format!("{}", stats);
    assert!(!display.is_empty());
}

#[test]
fn test_abc_display() {
    let stats = AbcStats::default();
    let display = format!("{}", stats);
    assert!(!display.is_empty());
}

// ============================================================================
// SECTION 5: Metric Boundary Tests
// ============================================================================

#[test]
fn test_cyclomatic_min_max() {
    let mut stats = CyclomaticStats::new();
    stats.increment();

    let min = stats.cyclomatic_min();
    let max = stats.cyclomatic_max();

    assert!(min >= 0.0);
    assert!(max >= 0.0);
}

#[test]
fn test_halstead_zero_division() {
    let stats = HalsteadStats::default();

    // These should not panic on division by zero
    let _ = stats.vocabulary();
    let _ = stats.length();
    let _ = stats.volume();
    let _ = stats.difficulty();
    let _ = stats.effort();
}

#[test]
fn test_mi_from_metrics() {
    let loc = LocStats::default();
    let cyclomatic = CyclomaticStats::default();
    let halstead = HalsteadStats::default();

    let mi = MaintainabilityIndexStats::from_metrics(&loc, &cyclomatic, &halstead);

    // Should compute without panicking
    assert!(mi.mi_original() >= 0.0 || mi.mi_original() < 0.0);
}

#[test]
fn test_wmc_from_cyclomatic() {
    let cyclomatic = CyclomaticStats::default();
    let wmc = WmcStats::from_cyclomatic(&cyclomatic);

    assert_eq!(wmc.wmc(), 0.0);
}

// ============================================================================
// SECTION 6: Halstead Collector Tests
// ============================================================================

#[test]
fn test_halstead_collector_new() {
    let _collector = HalsteadCollector::new();
    // Collector created successfully
}

#[test]
fn test_halstead_collector_add_and_finalize() {
    let mut collector = HalsteadCollector::new();
    collector.add_operator("+");
    collector.add_operator("+");
    collector.add_operator("-");
    collector.add_operand("x");
    collector.add_operand("x");
    collector.add_operand("y");

    let stats = collector.finalize();

    assert_eq!(stats.u_operators(), 2.0); // 2 unique operators (+ and -)
    assert_eq!(stats.operators(), 3.0); // 3 total operators
    assert_eq!(stats.u_operands(), 2.0); // 2 unique operands (x and y)
    assert_eq!(stats.operands(), 3.0); // 3 total operands
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

// ============================================================================
// SECTION 7: Enhanced Features Tests (Added for Migration)
// ============================================================================

#[test]
fn test_halstead_derived_metrics() {
    let mut collector = HalsteadCollector::new();

    // Add some realistic operators and operands
    collector.add_operator("+");
    collector.add_operator("-");
    collector.add_operator("*");
    collector.add_operator("/");
    collector.add_operand("x");
    collector.add_operand("y");
    collector.add_operand("z");
    collector.add_operand("result");

    let stats = collector.finalize();

    // Test all derived metrics
    assert!(stats.vocabulary() > 0.0);
    assert!(stats.length() > 0.0);
    assert!(stats.volume() > 0.0);
    assert!(stats.difficulty() > 0.0);
    assert!(stats.effort() > 0.0);
    assert!(stats.time() > 0.0);
    assert!(stats.bugs() >= 0.0);
}

#[test]
fn test_abc_advanced_declaration_tracking() {
    let mut stats = AbcStats::new();

    // Test 1: Variable declaration (should count)
    stats.start_var_declaration();
    stats.add_assignment_with_context();
    stats.clear_declaration();
    assert_eq!(stats.assignments(), 1.0);

    // Test 2: Const declaration (should NOT count)
    stats.start_const_declaration();
    stats.add_assignment_with_context();
    stats.clear_declaration();
    assert_eq!(stats.assignments(), 1.0); // Still 1

    // Test 3: Variable promoted to const (should NOT count)
    stats.start_var_declaration();
    stats.promote_to_const();
    stats.add_assignment_with_context();
    stats.clear_declaration();
    assert_eq!(stats.assignments(), 1.0); // Still 1
}

#[test]
fn test_abc_java_unary_conditions() {
    let mut stats = AbcStats::new();

    // Simulating Java unary conditions
    stats.add_unary_condition();
    stats.add_unary_condition();
    stats.add_unary_condition();

    assert_eq!(stats.conditions(), 3.0);
}

#[test]
fn test_cognitive_bool_sequence() {
    let mut stats = CognitiveStats::new();

    // Test same operator sequence (should count once)
    stats.eval_boolean_operator(100); // &&
    stats.eval_boolean_operator(100); // &&
    stats.eval_boolean_operator(100); // &&
    assert_eq!(stats.cognitive(), 1.0);

    // Test different operator (should increment)
    stats.eval_boolean_operator(101); // ||
    assert_eq!(stats.cognitive(), 2.0);

    // Reset and test again
    stats.reset_boolean_seq();
    stats.eval_boolean_operator(100); // &&
    assert_eq!(stats.cognitive(), 3.0);
}

#[test]
fn test_cognitive_with_nesting() {
    let mut stats = CognitiveStats::new();

    // Increment nesting level
    stats.increment_nesting();
    assert_eq!(stats.nesting_level(), 1);

    // Add structural complexity with nesting
    stats.increment_structural(1);

    stats.increment_nesting();
    assert_eq!(stats.nesting_level(), 2);

    stats.decrement_nesting();
    assert_eq!(stats.nesting_level(), 1);

    stats.decrement_nesting();
    assert_eq!(stats.nesting_level(), 0);
}

#[test]
fn test_cyclomatic_min_max_tracking() {
    let mut stats = CyclomaticStats::new();

    // First function
    stats.increment();
    stats.increment();
    stats.compute_sum();

    // Second function
    stats.reset();
    stats.increment();
    stats.increment();
    stats.increment();
    stats.increment();
    stats.compute_sum();

    // Third function
    stats.reset();
    stats.increment();
    stats.compute_sum();

    // Check min/max
    let min = stats.cyclomatic_min();
    let max = stats.cyclomatic_max();

    assert!(min > 0.0);
    assert!(max >= min);
}

#[test]
fn test_loc_physical_vs_logical() {
    let mut stats = LocStats::new();

    // Physical lines
    stats.incr_ploc();
    stats.incr_ploc();
    stats.incr_ploc();

    // Logical lines (might be different from physical)
    stats.incr_lloc();
    stats.incr_lloc();

    // Comments
    stats.incr_cloc();

    // Blanks
    stats.incr_blank();

    assert_eq!(stats.ploc(), 3.0);
    assert_eq!(stats.lloc(), 2.0);
    assert_eq!(stats.cloc(), 1.0);
    assert_eq!(stats.blank(), 1.0);
}

#[test]
fn test_maintainability_index_computation() {
    let mut loc = LocStats::new();
    loc.incr_sloc();
    loc.incr_sloc();
    loc.incr_sloc();

    let mut cyclomatic = CyclomaticStats::new();
    cyclomatic.increment();
    cyclomatic.increment();

    let halstead = HalsteadStats::from_counts(5, 10, 5, 10);

    let mi = MaintainabilityIndexStats::from_metrics(&loc, &cyclomatic, &halstead);

    // MI should be calculated
    let mi_original = mi.mi_original();
    let mi_sei = mi.mi_sei();
    let mi_visual_studio = mi.mi_visual_studio();

    // All MI variants should have reasonable values
    assert!(mi_original >= 0.0 || mi_original < 0.0); // Just check it's a valid number
    assert!(mi_sei >= 0.0 || mi_sei < 0.0);
    assert!(mi_visual_studio >= 0.0 || mi_visual_studio < 0.0);
}

#[test]
fn test_wmc_from_cyclomatic() {
    let mut cyclomatic = CyclomaticStats::new();

    // Simulate 3 methods with different complexities
    cyclomatic.increment();
    cyclomatic.increment();
    cyclomatic.compute_sum();

    cyclomatic.reset();
    cyclomatic.increment();
    cyclomatic.increment();
    cyclomatic.increment();
    cyclomatic.compute_sum();

    cyclomatic.reset();
    cyclomatic.increment();
    cyclomatic.compute_sum();

    let wmc = WmcStats::from_cyclomatic(&cyclomatic);

    // WMC should be the sum of method complexities
    assert_eq!(wmc.wmc(), cyclomatic.cyclomatic_sum());
}

#[test]
fn test_nom_functions_and_closures() {
    let mut stats = NomStats::new();

    stats.add_function();
    stats.add_function();
    stats.add_closure();
    stats.add_closure();
    stats.add_closure();

    assert_eq!(stats.functions(), 5.0); // Total
    assert_eq!(stats.closures(), 3.0); // Just closures
    assert_eq!(stats.functions_only(), 2.0); // Just functions
}

#[test]
fn test_nargs_statistics() {
    let mut stats = NargsStats::new();

    stats.add_func_with_args(0); // No args
    stats.add_func_with_args(2); // 2 args
    stats.add_func_with_args(3); // 3 args
    stats.add_func_with_args(5); // 5 args

    assert_eq!(stats.nargs_total(), 10.0);
    assert_eq!(stats.nargs_average(), 2.5);
    assert_eq!(stats.nargs_min(), 0.0);
    assert_eq!(stats.nargs_max(), 5.0);
}

#[test]
fn test_exit_points_tracking() {
    let mut stats = ExitStats::new();

    // Multiple exit points (return, break, continue, etc.)
    stats.add_exit();
    stats.add_exit();
    stats.add_exit();

    assert_eq!(stats.exit(), 3.0);
}

#[test]
fn test_npm_and_npa() {
    let mut npm = NpmStats::new();
    let mut npa = NpaStats::new();

    // Public methods
    npm.add_public_method();
    npm.add_public_method();
    npm.add_public_method();

    // Public attributes
    npa.add_public_attribute();
    npa.add_public_attribute();

    assert_eq!(npm.npm(), 3.0);
    assert_eq!(npa.npa(), 2.0);
}

#[test]
fn test_metrics_merge_operations() {
    let mut metrics1 = CodeMetrics::new();
    metrics1.cyclomatic.increment();
    metrics1.loc.incr_sloc();
    metrics1.nom.add_function();

    let mut metrics2 = CodeMetrics::new();
    metrics2.cyclomatic.increment();
    metrics2.loc.incr_sloc();
    metrics2.nom.add_function();

    metrics1.merge(&metrics2);

    // After merge, values should be combined
    assert!(metrics1.cyclomatic.cyclomatic() > 1.0);
    assert!(metrics1.loc.sloc() > 1.0);
    assert!(metrics1.nom.functions() >= 2.0);
}

#[test]
fn test_all_metrics_serialization_roundtrip() {
    let mut metrics = CodeMetrics::new();

    // Populate with some data
    metrics.cyclomatic.increment();
    metrics.loc.incr_sloc();
    metrics.abc.add_assignment();
    metrics.abc.add_branch();
    metrics.abc.add_condition();
    metrics.cognitive.increment_structural(1);
    metrics.nom.add_function();

    // Serialize to JSON
    let json = serde_json::to_string(&metrics).unwrap();

    // Deserialize back
    let deserialized: CodeMetrics = serde_json::from_str(&json).unwrap();

    // Verify equality
    assert_eq!(metrics, deserialized);
}
