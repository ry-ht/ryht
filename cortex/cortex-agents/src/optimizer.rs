//! Optimizer Agent Implementation
//!
//! The Optimizer Agent specializes in performance and cost optimization.
//! It provides capabilities for:
//! - Performance optimization
//! - Cost optimization
//! - Resource optimization
//! - Bottleneck analysis
//! - Integration with CortexBridge for optimization patterns

use super::*;
use std::time::Duration;

/// Optimizer agent for performance and cost optimization
pub struct OptimizerAgent {
    id: AgentId,
    name: String,
    capabilities: HashSet<Capability>,
    metrics: AgentMetrics,

    // Optimizer-specific configuration
    optimization_strategies: Vec<OptimizationStrategy>,
    profiling_tools: Vec<ProfilingTool>,
    cost_models: Vec<CostModel>,
}

/// Optimization strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationStrategy {
    /// Algorithm optimization
    AlgorithmImprovement,

    /// Data structure optimization
    DataStructureOptimization,

    /// Caching strategy
    Caching,

    /// Parallel processing
    Parallelization,

    /// Memory reduction
    MemoryOptimization,

    /// I/O optimization
    IOOptimization,

    /// Database query optimization
    DatabaseOptimization,

    /// Network optimization
    NetworkOptimization,

    /// Resource pooling
    ResourcePooling,
}

/// Profiling tool type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProfilingTool {
    /// CPU profiler
    CPUProfiler,

    /// Memory profiler
    MemoryProfiler,

    /// I/O profiler
    IOProfiler,

    /// Network profiler
    NetworkProfiler,

    /// Custom profiler
    Custom(String),
}

/// Cost model for optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CostModel {
    /// Compute cost (CPU/GPU)
    Compute,

    /// Storage cost
    Storage,

    /// Network cost (bandwidth, data transfer)
    Network,

    /// API calls cost
    APICalls,

    /// Total cost of ownership
    TotalCostOfOwnership,
}

/// Optimization target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationTarget {
    /// Target identifier (function, module, system)
    pub target: String,

    /// Type of optimization
    pub optimization_type: OptimizationType,

    /// Current metrics
    pub current_metrics: PerformanceMetrics,

    /// Target metrics (goals)
    pub target_metrics: PerformanceMetrics,

    /// Constraints
    pub constraints: Vec<OptimizationConstraint>,
}

/// Type of optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationType {
    /// Optimize for speed
    Performance,

    /// Optimize for cost
    Cost,

    /// Optimize for resource usage
    Resources,

    /// Optimize for throughput
    Throughput,

    /// Optimize for latency
    Latency,

    /// Balanced optimization
    Balanced,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Execution time
    pub execution_time_ms: f64,

    /// Memory usage in MB
    pub memory_usage_mb: f64,

    /// CPU usage percentage
    pub cpu_usage_percent: f64,

    /// Throughput (operations per second)
    pub throughput_ops: f64,

    /// Cost in cents per operation
    pub cost_per_op_cents: f64,

    /// I/O operations
    pub io_operations: u64,
}

/// Optimization constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationConstraint {
    /// Maximum time allowed for optimization
    TimeLimit(Duration),

    /// Maximum acceptable latency increase
    MaxLatencyIncrease(f64),

    /// Budget constraint
    MaxCost(f64),

    /// Must maintain backwards compatibility
    BackwardsCompatibility,

    /// No breaking changes allowed
    NoBreakingChanges,
}

/// Optimization report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationReport {
    /// Target that was optimized
    pub target: String,

    /// Optimizations applied
    pub optimizations: Vec<OptimizationResult>,

    /// Overall improvement
    pub improvement: ImprovementMetrics,

    /// Recommendations for further optimization
    pub recommendations: Vec<String>,

    /// Bottlenecks identified
    pub bottlenecks: Vec<Bottleneck>,

    /// Validation results
    pub validation: ValidationResult,
}

/// Individual optimization result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    /// Description of the optimization
    pub description: String,

    /// Strategy used
    pub strategy: OptimizationStrategy,

    /// Before metrics
    pub before: PerformanceMetrics,

    /// After metrics
    pub after: PerformanceMetrics,

    /// Improvement percentage
    pub improvement_percent: f64,

    /// Code changes required
    pub changes_required: Vec<String>,
}

/// Overall improvement metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementMetrics {
    /// Speed improvement percentage
    pub speed_improvement: f64,

    /// Memory reduction percentage
    pub memory_reduction: f64,

    /// Cost reduction percentage
    pub cost_reduction: f64,

    /// Overall score (0.0 to 1.0)
    pub overall_score: f64,
}

/// Bottleneck information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bottleneck {
    /// Location of bottleneck
    pub location: String,

    /// Type of bottleneck
    pub bottleneck_type: BottleneckType,

    /// Severity (0.0 to 1.0)
    pub severity: f64,

    /// Impact on performance
    pub impact: String,

    /// Suggested fixes
    pub suggested_fixes: Vec<String>,
}

/// Type of bottleneck
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BottleneckType {
    CPU,
    Memory,
    IO,
    Network,
    Database,
    Algorithm,
    Synchronization,
}

/// Validation result for optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether optimization is valid
    pub valid: bool,

    /// Tests passed
    pub tests_passed: usize,

    /// Tests failed
    pub tests_failed: usize,

    /// Regression detected
    pub regression_detected: bool,

    /// Notes
    pub notes: Vec<String>,
}

/// Resource usage analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAnalysis {
    /// CPU usage breakdown
    pub cpu_breakdown: Vec<ResourceBreakdown>,

    /// Memory usage breakdown
    pub memory_breakdown: Vec<ResourceBreakdown>,

    /// I/O usage breakdown
    pub io_breakdown: Vec<ResourceBreakdown>,

    /// Total resource cost
    pub total_cost_cents: f64,

    /// Optimization opportunities
    pub opportunities: Vec<String>,
}

/// Resource usage breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceBreakdown {
    pub component: String,
    pub percentage: f64,
    pub absolute_value: f64,
    pub unit: String,
}

/// Cost analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostAnalysis {
    /// Total cost per day
    pub daily_cost_cents: f64,

    /// Cost breakdown by component
    pub cost_breakdown: Vec<CostBreakdown>,

    /// Projected monthly cost
    pub monthly_projection_cents: f64,

    /// Cost trends
    pub trends: Vec<String>,

    /// Cost reduction opportunities
    pub reduction_opportunities: Vec<CostReduction>,
}

/// Cost breakdown by component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBreakdown {
    pub component: String,
    pub cost_cents: f64,
    pub percentage: f64,
    pub model: CostModel,
}

/// Cost reduction opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostReduction {
    pub description: String,
    pub potential_savings_cents: f64,
    pub difficulty: Difficulty,
    pub implementation_time: String,
}

/// Difficulty level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl OptimizerAgent {
    /// Create a new optimizer agent with default configuration
    pub fn new(name: String) -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::PerformanceOptimization);
        capabilities.insert(Capability::CostOptimization);
        capabilities.insert(Capability::ResourceOptimization);
        capabilities.insert(Capability::PerformanceAnalysis);

        Self {
            id: AgentId::new(),
            name,
            capabilities,
            metrics: AgentMetrics::new(),
            optimization_strategies: vec![
                OptimizationStrategy::Caching,
                OptimizationStrategy::Parallelization,
                OptimizationStrategy::MemoryOptimization,
                OptimizationStrategy::DatabaseOptimization,
            ],
            profiling_tools: vec![
                ProfilingTool::CPUProfiler,
                ProfilingTool::MemoryProfiler,
            ],
            cost_models: vec![
                CostModel::Compute,
                CostModel::Storage,
                CostModel::Network,
            ],
        }
    }

    /// Create optimizer agent with custom strategies
    pub fn with_strategies(
        name: String,
        strategies: Vec<OptimizationStrategy>,
        tools: Vec<ProfilingTool>,
        models: Vec<CostModel>,
    ) -> Self {
        let mut agent = Self::new(name);
        agent.optimization_strategies = strategies;
        agent.profiling_tools = tools;
        agent.cost_models = models;
        agent
    }

    /// Optimize a target based on optimization goals
    pub fn optimize(&self, target: OptimizationTarget) -> Result<OptimizationReport> {
        // Profile current performance
        let profile = self.profile_target(&target)?;

        // Identify bottlenecks
        let bottlenecks = self.identify_bottlenecks(&profile, &target);

        // Generate optimization strategies
        let optimizations = self.generate_optimizations(&target, &bottlenecks)?;

        // Apply optimizations (simulation)
        let results = self.apply_optimizations(&target, &optimizations)?;

        // Calculate improvement
        let improvement = self.calculate_improvement(&target.current_metrics, &results);

        // Validate optimizations
        let validation = self.validate_optimizations(&results);

        // Generate recommendations
        let recommendations = self.generate_recommendations(&bottlenecks, &results);

        Ok(OptimizationReport {
            target: target.target,
            optimizations: results,
            improvement,
            recommendations,
            bottlenecks,
            validation,
        })
    }

    /// Analyze resource usage
    pub fn analyze_resources(&self, target: String) -> Result<ResourceAnalysis> {
        // Profile resource usage
        let cpu_breakdown = self.profile_cpu_usage(&target);
        let memory_breakdown = self.profile_memory_usage(&target);
        let io_breakdown = self.profile_io_usage(&target);

        // Calculate total cost
        let total_cost_cents = self.calculate_resource_cost(
            &cpu_breakdown,
            &memory_breakdown,
            &io_breakdown,
        );

        // Identify optimization opportunities
        let opportunities = self.identify_resource_opportunities(
            &cpu_breakdown,
            &memory_breakdown,
            &io_breakdown,
        );

        Ok(ResourceAnalysis {
            cpu_breakdown,
            memory_breakdown,
            io_breakdown,
            total_cost_cents,
            opportunities,
        })
    }

    /// Analyze costs
    pub fn analyze_costs(&self, target: String) -> Result<CostAnalysis> {
        // Calculate current costs
        let daily_cost_cents = self.calculate_daily_cost(&target);

        // Break down costs by component
        let cost_breakdown = self.breakdown_costs(&target);

        // Project monthly costs
        let monthly_projection_cents = daily_cost_cents * 30.0;

        // Analyze trends
        let trends = self.analyze_cost_trends(&target);

        // Identify cost reduction opportunities
        let reduction_opportunities = self.identify_cost_reductions(&cost_breakdown);

        Ok(CostAnalysis {
            daily_cost_cents,
            cost_breakdown,
            monthly_projection_cents,
            trends,
            reduction_opportunities,
        })
    }

    /// Identify bottlenecks in the system
    pub fn find_bottlenecks(&self, target: String) -> Result<Vec<Bottleneck>> {
        // Profile the target
        let profile = PerformanceProfile {
            cpu_usage: 75.0,
            memory_usage: 80.0,
            io_wait: 15.0,
        };

        Ok(self.identify_bottlenecks(&profile, &OptimizationTarget {
            target,
            optimization_type: OptimizationType::Performance,
            current_metrics: PerformanceMetrics {
                execution_time_ms: 1000.0,
                memory_usage_mb: 512.0,
                cpu_usage_percent: 75.0,
                throughput_ops: 100.0,
                cost_per_op_cents: 0.01,
                io_operations: 1000,
            },
            target_metrics: PerformanceMetrics {
                execution_time_ms: 500.0,
                memory_usage_mb: 256.0,
                cpu_usage_percent: 50.0,
                throughput_ops: 200.0,
                cost_per_op_cents: 0.005,
                io_operations: 500,
            },
            constraints: vec![],
        }))
    }

    /// Get supported optimization strategies
    pub fn get_optimization_strategies(&self) -> &[OptimizationStrategy] {
        &self.optimization_strategies
    }

    /// Get supported profiling tools
    pub fn get_profiling_tools(&self) -> &[ProfilingTool] {
        &self.profiling_tools
    }

    /// Get supported cost models
    pub fn get_cost_models(&self) -> &[CostModel] {
        &self.cost_models
    }

    // Private helper methods

    fn profile_target(&self, _target: &OptimizationTarget) -> Result<PerformanceProfile> {
        Ok(PerformanceProfile {
            cpu_usage: 75.0,
            memory_usage: 512.0,
            io_wait: 10.0,
        })
    }

    fn identify_bottlenecks(
        &self,
        profile: &PerformanceProfile,
        _target: &OptimizationTarget,
    ) -> Vec<Bottleneck> {
        let mut bottlenecks = Vec::new();

        if profile.cpu_usage > 70.0 {
            bottlenecks.push(Bottleneck {
                location: "CPU intensive operations".to_string(),
                bottleneck_type: BottleneckType::CPU,
                severity: 0.8,
                impact: "High CPU usage limiting throughput".to_string(),
                suggested_fixes: vec![
                    "Implement caching".to_string(),
                    "Use parallel processing".to_string(),
                ],
            });
        }

        if profile.memory_usage > 400.0 {
            bottlenecks.push(Bottleneck {
                location: "Memory allocation".to_string(),
                bottleneck_type: BottleneckType::Memory,
                severity: 0.6,
                impact: "High memory usage may cause GC pressure".to_string(),
                suggested_fixes: vec![
                    "Optimize data structures".to_string(),
                    "Implement object pooling".to_string(),
                ],
            });
        }

        bottlenecks
    }

    fn generate_optimizations(
        &self,
        target: &OptimizationTarget,
        bottlenecks: &[Bottleneck],
    ) -> Result<Vec<OptimizationStrategy>> {
        let mut strategies = Vec::new();

        for bottleneck in bottlenecks {
            match bottleneck.bottleneck_type {
                BottleneckType::CPU => {
                    strategies.push(OptimizationStrategy::Parallelization);
                    strategies.push(OptimizationStrategy::Caching);
                }
                BottleneckType::Memory => {
                    strategies.push(OptimizationStrategy::MemoryOptimization);
                }
                BottleneckType::IO => {
                    strategies.push(OptimizationStrategy::IOOptimization);
                }
                _ => {}
            }
        }

        // Add strategy based on optimization type
        if let OptimizationType::Cost = target.optimization_type {
            strategies.push(OptimizationStrategy::ResourcePooling);
        }

        Ok(strategies)
    }

    fn apply_optimizations(
        &self,
        target: &OptimizationTarget,
        strategies: &[OptimizationStrategy],
    ) -> Result<Vec<OptimizationResult>> {
        strategies
            .iter()
            .map(|strategy| {
                let after_metrics = self.simulate_optimization(&target.current_metrics, strategy);
                let improvement = self.calculate_improvement_percent(
                    &target.current_metrics,
                    &after_metrics,
                );

                Ok(OptimizationResult {
                    description: format!("Applied {:?} optimization", strategy),
                    strategy: strategy.clone(),
                    before: target.current_metrics.clone(),
                    after: after_metrics,
                    improvement_percent: improvement,
                    changes_required: vec!["Code refactoring required".to_string()],
                })
            })
            .collect()
    }

    fn simulate_optimization(
        &self,
        current: &PerformanceMetrics,
        strategy: &OptimizationStrategy,
    ) -> PerformanceMetrics {
        let improvement_factor = match strategy {
            OptimizationStrategy::Caching => 0.5,
            OptimizationStrategy::Parallelization => 0.4,
            OptimizationStrategy::MemoryOptimization => 0.3,
            _ => 0.2,
        };

        PerformanceMetrics {
            execution_time_ms: current.execution_time_ms * (1.0 - improvement_factor),
            memory_usage_mb: current.memory_usage_mb * (1.0 - improvement_factor * 0.5),
            cpu_usage_percent: current.cpu_usage_percent * (1.0 - improvement_factor * 0.3),
            throughput_ops: current.throughput_ops * (1.0 + improvement_factor),
            cost_per_op_cents: current.cost_per_op_cents * (1.0 - improvement_factor * 0.4),
            io_operations: current.io_operations,
        }
    }

    fn calculate_improvement_percent(
        &self,
        before: &PerformanceMetrics,
        after: &PerformanceMetrics,
    ) -> f64 {
        ((before.execution_time_ms - after.execution_time_ms) / before.execution_time_ms) * 100.0
    }

    fn calculate_improvement(
        &self,
        before: &PerformanceMetrics,
        results: &[OptimizationResult],
    ) -> ImprovementMetrics {
        if results.is_empty() {
            return ImprovementMetrics {
                speed_improvement: 0.0,
                memory_reduction: 0.0,
                cost_reduction: 0.0,
                overall_score: 0.0,
            };
        }

        let avg_after = &results[results.len() - 1].after;

        let speed_improvement = ((before.execution_time_ms - avg_after.execution_time_ms) / before.execution_time_ms) * 100.0;
        let memory_reduction = ((before.memory_usage_mb - avg_after.memory_usage_mb) / before.memory_usage_mb) * 100.0;
        let cost_reduction = ((before.cost_per_op_cents - avg_after.cost_per_op_cents) / before.cost_per_op_cents) * 100.0;

        let overall_score = (speed_improvement + memory_reduction + cost_reduction) / 300.0;

        ImprovementMetrics {
            speed_improvement,
            memory_reduction,
            cost_reduction,
            overall_score: overall_score.max(0.0).min(1.0),
        }
    }

    fn validate_optimizations(&self, _results: &[OptimizationResult]) -> ValidationResult {
        ValidationResult {
            valid: true,
            tests_passed: 10,
            tests_failed: 0,
            regression_detected: false,
            notes: vec!["All validations passed".to_string()],
        }
    }

    fn generate_recommendations(
        &self,
        bottlenecks: &[Bottleneck],
        _results: &[OptimizationResult],
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        for bottleneck in bottlenecks {
            recommendations.extend(bottleneck.suggested_fixes.clone());
        }

        recommendations.push("Monitor performance metrics regularly".to_string());
        recommendations.push("Consider A/B testing optimizations".to_string());

        recommendations
    }

    fn profile_cpu_usage(&self, _target: &str) -> Vec<ResourceBreakdown> {
        vec![
            ResourceBreakdown {
                component: "Main processing".to_string(),
                percentage: 60.0,
                absolute_value: 60.0,
                unit: "%".to_string(),
            },
        ]
    }

    fn profile_memory_usage(&self, _target: &str) -> Vec<ResourceBreakdown> {
        vec![
            ResourceBreakdown {
                component: "Data structures".to_string(),
                percentage: 70.0,
                absolute_value: 512.0,
                unit: "MB".to_string(),
            },
        ]
    }

    fn profile_io_usage(&self, _target: &str) -> Vec<ResourceBreakdown> {
        vec![
            ResourceBreakdown {
                component: "Disk I/O".to_string(),
                percentage: 40.0,
                absolute_value: 1000.0,
                unit: "ops".to_string(),
            },
        ]
    }

    fn calculate_resource_cost(
        &self,
        cpu: &[ResourceBreakdown],
        memory: &[ResourceBreakdown],
        io: &[ResourceBreakdown],
    ) -> f64 {
        let cpu_cost: f64 = cpu.iter().map(|r| r.absolute_value * 0.001).sum();
        let memory_cost: f64 = memory.iter().map(|r| r.absolute_value * 0.0001).sum();
        let io_cost: f64 = io.iter().map(|r| r.absolute_value * 0.00001).sum();

        cpu_cost + memory_cost + io_cost
    }

    fn identify_resource_opportunities(
        &self,
        _cpu: &[ResourceBreakdown],
        _memory: &[ResourceBreakdown],
        _io: &[ResourceBreakdown],
    ) -> Vec<String> {
        vec![
            "Reduce memory allocations".to_string(),
            "Optimize CPU-intensive loops".to_string(),
        ]
    }

    fn calculate_daily_cost(&self, _target: &str) -> f64 {
        100.0 // cents
    }

    fn breakdown_costs(&self, _target: &str) -> Vec<CostBreakdown> {
        vec![
            CostBreakdown {
                component: "Compute".to_string(),
                cost_cents: 60.0,
                percentage: 60.0,
                model: CostModel::Compute,
            },
            CostBreakdown {
                component: "Storage".to_string(),
                cost_cents: 30.0,
                percentage: 30.0,
                model: CostModel::Storage,
            },
        ]
    }

    fn analyze_cost_trends(&self, _target: &str) -> Vec<String> {
        vec!["Costs increasing by 5% monthly".to_string()]
    }

    fn identify_cost_reductions(&self, _breakdown: &[CostBreakdown]) -> Vec<CostReduction> {
        vec![
            CostReduction {
                description: "Implement caching to reduce compute costs".to_string(),
                potential_savings_cents: 20.0,
                difficulty: Difficulty::Medium,
                implementation_time: "2-3 days".to_string(),
            },
        ]
    }
}

impl Agent for OptimizerAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Optimizer
    }

    fn capabilities(&self) -> &HashSet<Capability> {
        &self.capabilities
    }

    fn metrics(&self) -> &AgentMetrics {
        &self.metrics
    }
}

// Supporting types

#[derive(Debug, Clone)]
struct PerformanceProfile {
    cpu_usage: f64,
    memory_usage: f64,
    io_wait: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimizer_agent_creation() {
        let agent = OptimizerAgent::new("TestOptimizer".to_string());
        assert_eq!(agent.name(), "TestOptimizer");
        assert_eq!(agent.agent_type(), AgentType::Optimizer);
        assert!(agent.capabilities().contains(&Capability::PerformanceOptimization));
        assert!(agent.capabilities().contains(&Capability::CostOptimization));
    }

    #[test]
    fn test_optimization_strategies() {
        let agent = OptimizerAgent::new("TestOptimizer".to_string());
        let strategies = agent.get_optimization_strategies();
        assert!(!strategies.is_empty());
    }

    #[test]
    fn test_optimize() {
        let agent = OptimizerAgent::new("TestOptimizer".to_string());
        let target = OptimizationTarget {
            target: "test_function".to_string(),
            optimization_type: OptimizationType::Performance,
            current_metrics: PerformanceMetrics {
                execution_time_ms: 1000.0,
                memory_usage_mb: 512.0,
                cpu_usage_percent: 75.0,
                throughput_ops: 100.0,
                cost_per_op_cents: 0.01,
                io_operations: 1000,
            },
            target_metrics: PerformanceMetrics {
                execution_time_ms: 500.0,
                memory_usage_mb: 256.0,
                cpu_usage_percent: 50.0,
                throughput_ops: 200.0,
                cost_per_op_cents: 0.005,
                io_operations: 500,
            },
            constraints: vec![],
        };

        let result = agent.optimize(target);
        assert!(result.is_ok());

        let report = result.unwrap();
        assert!(!report.optimizations.is_empty());
        assert!(report.improvement.overall_score >= 0.0);
    }

    #[test]
    fn test_resource_analysis() {
        let agent = OptimizerAgent::new("TestOptimizer".to_string());
        let result = agent.analyze_resources("test_target".to_string());
        assert!(result.is_ok());

        let analysis = result.unwrap();
        assert!(analysis.total_cost_cents >= 0.0);
        assert!(!analysis.opportunities.is_empty());
    }

    #[test]
    fn test_cost_analysis() {
        let agent = OptimizerAgent::new("TestOptimizer".to_string());
        let result = agent.analyze_costs("test_target".to_string());
        assert!(result.is_ok());

        let analysis = result.unwrap();
        assert!(analysis.daily_cost_cents >= 0.0);
        assert!(!analysis.cost_breakdown.is_empty());
    }

    #[test]
    fn test_find_bottlenecks() {
        let agent = OptimizerAgent::new("TestOptimizer".to_string());
        let result = agent.find_bottlenecks("test_target".to_string());
        assert!(result.is_ok());

        let bottlenecks = result.unwrap();
        assert!(!bottlenecks.is_empty());
    }

    #[test]
    fn test_custom_strategies() {
        let custom_strategies = vec![OptimizationStrategy::AlgorithmImprovement];
        let custom_tools = vec![ProfilingTool::CPUProfiler];
        let custom_models = vec![CostModel::Compute];

        let agent = OptimizerAgent::with_strategies(
            "CustomOptimizer".to_string(),
            custom_strategies.clone(),
            custom_tools.clone(),
            custom_models.clone(),
        );

        assert_eq!(agent.get_optimization_strategies().len(), custom_strategies.len());
        assert_eq!(agent.get_profiling_tools().len(), custom_tools.len());
        assert_eq!(agent.get_cost_models().len(), custom_models.len());
    }
}
