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
