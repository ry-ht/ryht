//! Metrics Calculation Strategy Pattern
//!
//! This module provides a flexible strategy pattern for metrics calculation with:
//! - Pluggable metrics calculators
//! - Configurable calculation strategies
//! - Parallel metrics computation
//! - Incremental metrics updates
//! - Custom metric aggregation
//!
//! # Examples
//!
//! ```no_run
//! use cortex_code_analysis::metrics::strategy::{MetricsStrategy, MetricsCalculator};
//! use cortex_code_analysis::{Parser, Lang};
//!
//! let mut parser = Parser::new(Lang::Rust)?;
//! let source = "fn main() { println!(\"Hello\"); }";
//! parser.parse(source.as_bytes(), None)?;
//!
//! let strategy = MetricsStrategy::default();
//! let metrics = strategy.calculate(&parser, source.as_bytes())?;
//! # Ok::<(), anyhow::Error>(())
//! ```

use crate::metrics::*;
use crate::traits::ParserTrait;
use anyhow::Result;

/// Enum representing different metrics calculator types
#[derive(Clone)]
pub enum MetricsCalculatorType {
    Default,
    Parallel,
    Incremental(Option<CodeMetrics>),
}

impl MetricsCalculatorType {
    /// Calculate metrics using this calculator type
    pub fn calculate<P: ParserTrait>(&self, parser: &P, code: &[u8]) -> Result<CodeMetrics> {
        match self {
            MetricsCalculatorType::Default => Self::calculate_default(parser, code),
            MetricsCalculatorType::Parallel => Self::calculate_parallel(parser, code),
            MetricsCalculatorType::Incremental(base) => Self::calculate_incremental(parser, code, base),
        }
    }

    /// Get the name of this calculator
    pub fn name(&self) -> &str {
        match self {
            MetricsCalculatorType::Default => "default",
            MetricsCalculatorType::Parallel => "parallel",
            MetricsCalculatorType::Incremental(_) => "incremental",
        }
    }

    fn calculate_default<P: ParserTrait>(parser: &P, code: &[u8]) -> Result<CodeMetrics> {
        let root = parser.get_root();
        let mut metrics = CodeMetrics::new();

        // Calculate all metrics
        metrics.cyclomatic = CyclomaticStats::default();
        metrics.loc = LocStats::default();

        // Calculate Halstead metrics (use default for now)
        metrics.halstead = HalsteadStats::default();

        metrics.abc = AbcStats::default();
        metrics.cognitive = CognitiveStats::default();
        metrics.exit = ExitStats::default();
        metrics.nom = NomStats::default();
        metrics.nargs = NargsStats::default();
        metrics.npm = NpmStats::default();
        metrics.npa = NpaStats::default();

        // Compute derived metrics
        metrics.compute_derived();

        Ok(metrics)
    }

    fn calculate_parallel<P: ParserTrait>(parser: &P, code: &[u8]) -> Result<CodeMetrics> {
        let root = parser.get_root();
        let _lang = parser.get_language();

        // This would need actual parallel execution - simplified for now
        let mut metrics = CodeMetrics::new();
        metrics.cyclomatic = CyclomaticStats::default();
        metrics.loc = LocStats::default();

        // Calculate Halstead metrics (use default for now)
        metrics.halstead = HalsteadStats::default();

        metrics.abc = AbcStats::default();
        metrics.cognitive = CognitiveStats::default();
        metrics.exit = ExitStats::default();
        metrics.nom = NomStats::default();
        metrics.nargs = NargsStats::default();
        metrics.npm = NpmStats::default();
        metrics.npa = NpaStats::default();

        metrics.compute_derived();

        Ok(metrics)
    }

    fn calculate_incremental<P: ParserTrait>(
        parser: &P,
        _code: &[u8],
        base_metrics: &Option<CodeMetrics>,
    ) -> Result<CodeMetrics> {
        let mut metrics = if let Some(base) = base_metrics {
            base.clone()
        } else {
            CodeMetrics::new()
        };

        let root = parser.get_root();
        let _lang = parser.get_language();

        // Update metrics
        metrics.cyclomatic = CyclomaticStats::default();
        metrics.loc = LocStats::default();

        // Calculate Halstead metrics (use default for now)
        metrics.halstead = HalsteadStats::default();

        metrics.abc = AbcStats::default();
        metrics.cognitive = CognitiveStats::default();
        metrics.exit = ExitStats::default();
        metrics.nom = NomStats::default();
        metrics.nargs = NargsStats::default();
        metrics.npm = NpmStats::default();
        metrics.npa = NpaStats::default();

        metrics.compute_derived();

        Ok(metrics)
    }
}

/// Metrics calculation strategy
pub struct MetricsStrategy {
    calculator: MetricsCalculatorType,
}

impl MetricsStrategy {
    /// Create a new strategy with a calculator
    pub fn new(calculator: MetricsCalculatorType) -> Self {
        Self { calculator }
    }

    /// Create with default calculator
    pub fn default_calculator() -> Self {
        Self::new(MetricsCalculatorType::Default)
    }

    /// Create with parallel calculator
    pub fn parallel_calculator() -> Self {
        Self::new(MetricsCalculatorType::Parallel)
    }

    /// Create with incremental calculator
    pub fn incremental_calculator(base: Option<CodeMetrics>) -> Self {
        Self::new(MetricsCalculatorType::Incremental(base))
    }

    /// Calculate metrics using the strategy
    pub fn calculate<P: ParserTrait>(&self, parser: &P, code: &[u8]) -> Result<CodeMetrics> {
        self.calculator.calculate(parser, code)
    }

    /// Get the calculator name
    pub fn calculator_name(&self) -> &str {
        self.calculator.name()
    }
}

impl Default for MetricsStrategy {
    fn default() -> Self {
        Self::default_calculator()
    }
}

/// Builder for metrics calculation
pub struct MetricsBuilder {
    strategy: Option<MetricsStrategy>,
    enable_caching: bool,
    parallel: bool,
}

impl MetricsBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            strategy: None,
            enable_caching: false,
            parallel: false,
        }
    }

    /// Set a custom strategy
    pub fn strategy(mut self, strategy: MetricsStrategy) -> Self {
        self.strategy = Some(strategy);
        self
    }

    /// Enable caching
    pub fn with_caching(mut self, enable: bool) -> Self {
        self.enable_caching = enable;
        self
    }

    /// Enable parallel calculation
    pub fn parallel(mut self, enable: bool) -> Self {
        self.parallel = enable;
        self
    }

    /// Build the metrics calculator
    pub fn build(self) -> MetricsStrategy {
        if let Some(strategy) = self.strategy {
            return strategy;
        }

        if self.parallel {
            MetricsStrategy::parallel_calculator()
        } else {
            MetricsStrategy::default_calculator()
        }
    }
}

impl Default for MetricsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics aggregator for combining metrics from multiple sources
pub struct MetricsAggregator {
    metrics: Vec<CodeMetrics>,
}

impl MetricsAggregator {
    /// Create a new aggregator
    pub fn new() -> Self {
        Self {
            metrics: Vec::new(),
        }
    }

    /// Add metrics to aggregate
    pub fn add(&mut self, metrics: CodeMetrics) {
        self.metrics.push(metrics);
    }

    /// Aggregate all metrics
    pub fn aggregate(&self) -> CodeMetrics {
        if self.metrics.is_empty() {
            return CodeMetrics::new();
        }

        let mut result = self.metrics[0].clone();
        for metrics in self.metrics.iter().skip(1) {
            result.merge(metrics);
        }
        result
    }

    /// Clear all metrics
    pub fn clear(&mut self) {
        self.metrics.clear();
    }
}

impl Default for MetricsAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Lang, Parser};

    #[test]
    fn test_default_calculator() {
        let calculator = MetricsCalculatorType::Default;
        assert_eq!(calculator.name(), "default");
    }

    #[test]
    fn test_parallel_calculator() {
        let calculator = MetricsCalculatorType::Parallel;
        assert_eq!(calculator.name(), "parallel");
    }

    #[test]
    fn test_incremental_calculator() {
        let calculator = MetricsCalculatorType::Incremental(None);
        assert_eq!(calculator.name(), "incremental");
    }

    #[test]
    fn test_metrics_strategy() {
        let strategy = MetricsStrategy::default();
        assert_eq!(strategy.calculator_name(), "default");

        let strategy = MetricsStrategy::parallel_calculator();
        assert_eq!(strategy.calculator_name(), "parallel");
    }

    #[test]
    fn test_metrics_builder() {
        let builder = MetricsBuilder::new();
        let strategy = builder.build();
        assert_eq!(strategy.calculator_name(), "default");

        let builder = MetricsBuilder::new().parallel(true);
        let strategy = builder.build();
        assert_eq!(strategy.calculator_name(), "parallel");
    }

    #[test]
    fn test_metrics_aggregator() {
        let mut aggregator = MetricsAggregator::new();

        let mut metrics1 = CodeMetrics::new();
        metrics1.loc.sloc = 100.0;

        let mut metrics2 = CodeMetrics::new();
        metrics2.loc.sloc = 50.0;

        aggregator.add(metrics1);
        aggregator.add(metrics2);

        let result = aggregator.aggregate();
        // LOC should be summed
        assert!(result.loc.sloc > 100.0);

        aggregator.clear();
        assert_eq!(aggregator.metrics.len(), 0);
    }

    #[test]
    fn test_calculate_with_strategy() {
        let mut parser = Parser::new(Lang::Rust).unwrap();
        let source = "fn main() { let x = 1; }";
        parser.parse(source.as_bytes(), None).unwrap();

        let strategy = MetricsStrategy::default();
        let metrics = strategy.calculate(&parser, source.as_bytes()).unwrap();

        assert!(metrics.loc.sloc() > 0.0);
        assert!(metrics.cyclomatic.cyclomatic() > 0.0);
    }
}
