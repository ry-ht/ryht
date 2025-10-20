# Axon: Quality Assurance

## Overview

Quality assurance is essential for reliable multi-agent orchestration. This document specifies validation strategies, testing patterns, monitoring systems, and quality metrics for Axon. The system focuses on runtime validation while Cortex stores test results and metrics for analysis.

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    Axon Quality Layer                    │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │   Agent     │  │   Workflow   │  │ Integration  │   │
│  │ Validation  │  │   Testing    │  │   Testing    │   │
│  └──────┬──────┘  └──────┬───────┘  └──────┬───────┘   │
│         │                │                   │           │
│         └────────────────┼───────────────────┘           │
│                          │                               │
│  ┌──────────────────────▼────────────────────────────┐  │
│  │        Performance Benchmarking System            │  │
│  └──────────────────────┬────────────────────────────┘  │
│                         │                               │
│  ┌──────────────────────▼────────────────────────────┐  │
│  │     Chaos Engineering & Error Injection           │  │
│  └──────────────────────┬────────────────────────────┘  │
│                         │                               │
│  ┌──────────────────────▼────────────────────────────┐  │
│  │    Monitoring & Observability (Metrics/Logs)      │  │
│  └──────────────────────┬────────────────────────────┘  │
└─────────────────────────┼────────────────────────────────┘
                          │ REST API
┌─────────────────────────▼────────────────────────────────┐
│                      Cortex                               │
│         (Test Results & Metrics Storage)                  │
│                                                           │
│  POST /test-results    - Store test results           │
│  GET  /test-results    - Query test history           │
│  POST /metrics         - Store quality metrics        │
│  POST /logs            - Store execution logs          │
│  GET  /analysis/quality - Quality analysis            │
└───────────────────────────────────────────────────────────┘
```

## Quality Targets

- **Agent Validation**: 100% capability verification before execution
- **Test Coverage**: > 80% for critical workflows
- **Workflow Success Rate**: > 95% for validated workflows
- **Error Detection**: < 100ms for validation failures
- **Monitoring Latency**: < 5s for metric reporting
- **Chaos Recovery**: < 30s for automatic recovery
- **Mean Time to Detection (MTTD)**: < 2 minutes
- **Mean Time to Recovery (MTTR)**: < 5 minutes

## 1. Agent Validation and Verification

All agents are validated before execution to ensure capabilities match task requirements.

### Agent Validator

```rust
use std::collections::{HashMap, HashSet};

/// Agent validator проверяет capabilities и readiness
pub struct AgentValidator {
    capability_registry: Arc<RwLock<HashMap<AgentId, HashSet<Capability>>>>,
    validation_rules: Vec<ValidationRule>,
    metrics: Arc<ValidationMetrics>,
}

impl AgentValidator {
    pub fn new() -> Self {
        Self {
            capability_registry: Arc::new(RwLock::new(HashMap::new())),
            validation_rules: vec![
                ValidationRule::CapabilityCheck,
                ValidationRule::ResourceCheck,
                ValidationRule::HealthCheck,
                ValidationRule::DependencyCheck,
            ],
            metrics: Arc::new(ValidationMetrics::new()),
        }
    }

    /// Валидирует agent перед назначением task
    pub async fn validate_agent(&self, agent: &Agent, task: &Task) -> Result<ValidationReport> {
        let start = Instant::now();
        let mut report = ValidationReport::new();

        // 1. Capability validation
        let capabilities = self.capability_registry.read().await
            .get(&agent.id)
            .cloned()
            .unwrap_or_default();

        for required_cap in &task.requirements.capabilities {
            if !capabilities.contains(required_cap) {
                report.add_error(ValidationError::MissingCapability {
                    required: required_cap.clone(),
                    agent_id: agent.id.clone(),
                });
            }
        }

        // 2. Resource validation
        if task.requirements.resources.cpu_cores > agent.resources.available_cpu {
            report.add_error(ValidationError::InsufficientResources {
                required: task.requirements.resources.cpu_cores,
                available: agent.resources.available_cpu,
            });
        }

        // 3. Health check
        match self.check_agent_health(agent).await {
            Ok(HealthStatus::Healthy) => {},
            Ok(status) => report.add_warning(ValidationWarning::UnhealthyAgent { status }),
            Err(e) => report.add_error(ValidationError::HealthCheckFailed { error: e.to_string() }),
        }

        // 4. Dependency check
        if let Err(deps) = self.check_dependencies(agent, task).await {
            report.add_error(ValidationError::MissingDependencies { dependencies: deps });
        }

        let duration = start.elapsed();

        // Store validation result in Cortex
        self.store_validation_result(&agent.id, &task.id, &report, duration).await?;

        self.metrics.record_validation(&report, duration);

        Ok(report)
    }

    /// Регистрирует agent и его capabilities
    pub async fn register_agent(&self, agent_id: AgentId, capabilities: HashSet<Capability>) -> Result<()> {
        let mut registry = self.capability_registry.write().await;
        registry.insert(agent_id.clone(), capabilities.clone());

        // Store in Cortex
        self.store_agent_capabilities(agent_id, capabilities).await?;

        Ok(())
    }

    /// Проверяет health status агента
    async fn check_agent_health(&self, agent: &Agent) -> Result<HealthStatus> {
        // Check if agent is responsive
        let ping_result = agent.ping().await?;

        if ping_result.latency > Duration::from_millis(100) {
            return Ok(HealthStatus::Degraded);
        }

        // Check resource utilization
        if agent.resources.cpu_usage > 90.0 || agent.resources.memory_usage > 90.0 {
            return Ok(HealthStatus::Overloaded);
        }

        Ok(HealthStatus::Healthy)
    }

    /// Проверяет зависимости для task execution
    async fn check_dependencies(&self, agent: &Agent, task: &Task) -> Result<(), Vec<String>> {
        let mut missing = Vec::new();

        for dependency in &task.dependencies {
            if !agent.has_dependency(dependency) {
                missing.push(dependency.clone());
            }
        }

        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }

    /// Сохраняет результат валидации в Cortex
    async fn store_validation_result(
        &self,
        agent_id: &AgentId,
        task_id: &TaskId,
        report: &ValidationReport,
        duration: Duration,
    ) -> Result<()> {
        let result = ValidationResult {
            agent_id: agent_id.clone(),
            task_id: task_id.clone(),
            success: report.is_valid(),
            errors: report.errors.clone(),
            warnings: report.warnings.clone(),
            duration_ms: duration.as_millis() as u64,
            timestamp: chrono::Utc::now().timestamp() as u64,
        };

        // POST /test-results
        let client = reqwest::Client::new();
        client.post("http://cortex:8081/test-results")
            .json(&result)
            .send()
            .await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

impl ValidationReport {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum ValidationError {
    MissingCapability { required: Capability, agent_id: AgentId },
    InsufficientResources { required: u32, available: u32 },
    HealthCheckFailed { error: String },
    MissingDependencies { dependencies: Vec<String> },
}

#[derive(Debug, Clone)]
pub enum ValidationWarning {
    UnhealthyAgent { status: HealthStatus },
    HighLatency { latency: Duration },
    LowResources { available: f64 },
}

#[derive(Debug, Clone)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Overloaded,
    Unhealthy,
}
```

## 2. Workflow Testing Strategies

Comprehensive testing ensures workflows execute correctly.

### Workflow Test Framework

```rust
/// Test framework для workflow execution
pub struct WorkflowTestFramework {
    test_cases: Vec<WorkflowTestCase>,
    executor: Arc<WorkflowExecutor>,
    cortex_client: Arc<CortexClient>,
    metrics: Arc<TestMetrics>,
}

impl WorkflowTestFramework {
    /// Выполняет unit test для workflow
    pub async fn test_workflow_unit(&self, workflow: Workflow) -> Result<TestResult> {
        let start = Instant::now();

        // 1. Validate workflow structure
        DagValidator::validate(&workflow)?;

        // 2. Create mock agents
        let mock_agents = self.create_mock_agents(&workflow)?;

        // 3. Execute workflow with mocks
        let schedule = self.create_schedule(&workflow)?;
        let result = self.executor.execute(workflow.clone(), schedule).await?;

        // 4. Verify results
        let assertions = self.verify_workflow_output(&result)?;

        let duration = start.elapsed();

        let test_result = TestResult {
            test_name: format!("workflow_unit_{}", workflow.id),
            success: assertions.all_passed(),
            assertions,
            duration,
            workflow_id: workflow.id.clone(),
        };

        // Store in Cortex
        self.store_test_result(&test_result).await?;

        Ok(test_result)
    }

    /// Выполняет integration test для workflow
    pub async fn test_workflow_integration(&self, workflow: Workflow) -> Result<TestResult> {
        let start = Instant::now();

        // 1. Create real session in Cortex
        let session_id = self.cortex_client
            .post("/sessions", &CreateSessionRequest {
                workspace_id: "test".to_string(),
                scope: SessionScope::default(),
            })
            .await?
            .json::<Session>()
            .await?
            .session_id;

        // 2. Execute with real agents
        let result = self.executor.execute(workflow.clone(), 
            self.create_schedule(&workflow)?).await?;

        // 3. Verify Cortex state
        let cortex_state = self.verify_cortex_state(&session_id).await?;

        // 4. Cleanup session
        self.cortex_client
            .delete(&format!("/sessions/{}", session_id))
            .await?;

        let duration = start.elapsed();

        let test_result = TestResult {
            test_name: format!("workflow_integration_{}", workflow.id),
            success: result.success && cortex_state.is_valid(),
            assertions: cortex_state.assertions,
            duration,
            workflow_id: workflow.id.clone(),
        };

        self.store_test_result(&test_result).await?;

        Ok(test_result)
    }

    /// Property-based testing для workflow properties
    pub async fn test_workflow_properties(&self, workflow: Workflow) -> Result<Vec<TestResult>> {
        let mut results = Vec::new();

        // Property 1: Idempotency
        results.push(self.test_idempotency(&workflow).await?);

        // Property 2: Determinism
        results.push(self.test_determinism(&workflow).await?);

        // Property 3: Resource bounds
        results.push(self.test_resource_bounds(&workflow).await?);

        // Property 4: Timeout compliance
        results.push(self.test_timeout_compliance(&workflow).await?);

        Ok(results)
    }

    async fn test_idempotency(&self, workflow: &Workflow) -> Result<TestResult> {
        // Execute workflow twice
        let result1 = self.executor.execute(workflow.clone(), 
            self.create_schedule(workflow)?).await?;
        let result2 = self.executor.execute(workflow.clone(), 
            self.create_schedule(workflow)?).await?;

        // Verify identical results
        let idempotent = result1.task_results == result2.task_results;

        Ok(TestResult {
            test_name: format!("property_idempotency_{}", workflow.id),
            success: idempotent,
            assertions: vec![Assertion {
                name: "idempotency".to_string(),
                passed: idempotent,
                message: if idempotent { 
                    "Results are identical".to_string() 
                } else { 
                    "Results differ on re-execution".to_string() 
                },
            }],
            duration: Duration::default(),
            workflow_id: workflow.id.clone(),
        })
    }

    async fn test_determinism(&self, workflow: &Workflow) -> Result<TestResult> {
        // Execute workflow multiple times with same input
        let mut results = Vec::new();
        for _ in 0..5 {
            let result = self.executor.execute(workflow.clone(), 
                self.create_schedule(workflow)?).await?;
            results.push(result);
        }

        // Verify all results are identical
        let deterministic = results.windows(2).all(|w| w[0].task_results == w[1].task_results);

        Ok(TestResult {
            test_name: format!("property_determinism_{}", workflow.id),
            success: deterministic,
            assertions: vec![Assertion {
                name: "determinism".to_string(),
                passed: deterministic,
                message: if deterministic {
                    "All executions produced identical results".to_string()
                } else {
                    "Results vary across executions".to_string()
                },
            }],
            duration: Duration::default(),
            workflow_id: workflow.id.clone(),
        })
    }

    /// Сохраняет test result в Cortex
    async fn store_test_result(&self, result: &TestResult) -> Result<()> {
        // POST /test-results
        self.cortex_client
            .post("/test-results", result)
            .await?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TestResult {
    pub test_name: String,
    pub success: bool,
    pub assertions: Vec<Assertion>,
    pub duration: Duration,
    pub workflow_id: WorkflowId,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Assertion {
    pub name: String,
    pub passed: bool,
    pub message: String,
}
```

## 3. Integration Tests with Cortex

Test Axon-Cortex integration thoroughly.

### Cortex Integration Tests

```rust
/// Integration tests для Axon-Cortex взаимодействия
pub struct CortexIntegrationTests {
    cortex_client: Arc<CortexClient>,
    test_metrics: Arc<TestMetrics>,
}

impl CortexIntegrationTests {
    /// Тестирует session lifecycle
    pub async fn test_session_lifecycle(&self) -> Result<TestResult> {
        let mut assertions = Vec::new();

        // 1. Create session
        let session = self.cortex_client
            .post("/sessions", &CreateSessionRequest {
                workspace_id: "test".to_string(),
                scope: SessionScope::default(),
            })
            .await?
            .json::<Session>()
            .await?;

        assertions.push(Assertion {
            name: "session_created".to_string(),
            passed: !session.session_id.is_empty(),
            message: format!("Session created: {}", session.session_id),
        });

        // 2. Write file to session
        self.cortex_client
            .put(&format!("/sessions/{}/files/test.txt", session.session_id),
                 &UpdateFileRequest { content: "test".to_string(), expected_version: None })
            .await?;

        assertions.push(Assertion {
            name: "file_written".to_string(),
            passed: true,
            message: "File written to session".to_string(),
        });

        // 3. Read file from session
        let content = self.cortex_client
            .get(&format!("/sessions/{}/files/test.txt", session.session_id))
            .await?
            .json::<FileContent>()
            .await?;

        assertions.push(Assertion {
            name: "file_read".to_string(),
            passed: content.content == "test",
            message: format!("File content: {}", content.content),
        });

        // 4. Merge session
        let merge_report = self.cortex_client
            .post(&format!("/sessions/{}/merge", session.session_id), 
                  &MergeSessionRequest { strategy: MergeStrategy::Auto, conflict_resolution: None })
            .await?
            .json::<MergeReport>()
            .await?;

        assertions.push(Assertion {
            name: "session_merged".to_string(),
            passed: merge_report.conflicts_resolved == 0,
            message: format!("Merge completed with {} conflicts", merge_report.conflicts_resolved),
        });

        // 5. Close session
        self.cortex_client
            .delete(&format!("/sessions/{}", session.session_id))
            .await?;

        assertions.push(Assertion {
            name: "session_closed".to_string(),
            passed: true,
            message: "Session closed".to_string(),
        });

        Ok(TestResult {
            test_name: "cortex_session_lifecycle".to_string(),
            success: assertions.iter().all(|a| a.passed),
            assertions,
            duration: Duration::default(),
            workflow_id: WorkflowId::default(),
        })
    }

    /// Тестирует episodic memory storage и retrieval
    pub async fn test_episodic_memory(&self) -> Result<TestResult> {
        let mut assertions = Vec::new();

        // 1. Store episode
        let episode = Episode {
            task_description: "Test task".to_string(),
            agent_id: AgentId::default(),
            outcome: "success".to_string(),
            duration_seconds: 60,
            solution_summary: "Test solution".to_string(),
            entities_modified: vec!["test.rs".to_string()],
            files_touched: vec!["test.rs".to_string()],
            patterns_learned: vec!["test_pattern".to_string()],
        };

        let episode_id = self.cortex_client
            .post("/memory/episodes", &episode)
            .await?
            .json::<CreateEpisodeResponse>()
            .await?
            .episode_id;

        assertions.push(Assertion {
            name: "episode_stored".to_string(),
            passed: !episode_id.is_empty(),
            message: format!("Episode stored: {}", episode_id),
        });

        // 2. Search episodes
        let episodes = self.cortex_client
            .post("/memory/search", &SearchEpisodesRequest {
                query: "Test task".to_string(),
                limit: 5,
                min_similarity: 0.7,
            })
            .await?
            .json::<SearchEpisodesResponse>()
            .await?
            .episodes;

        assertions.push(Assertion {
            name: "episode_found".to_string(),
            passed: !episodes.is_empty(),
            message: format!("Found {} episodes", episodes.len()),
        });

        Ok(TestResult {
            test_name: "cortex_episodic_memory".to_string(),
            success: assertions.iter().all(|a| a.passed),
            assertions,
            duration: Duration::default(),
            workflow_id: WorkflowId::default(),
        })
    }
}
```

## 4. Performance Benchmarking

Measure and track performance metrics over time.

### Benchmark Framework

```rust
use criterion::{Criterion, black_box};

/// Performance benchmark framework
pub struct BenchmarkFramework {
    cortex_client: Arc<CortexClient>,
    benchmarks: Vec<Benchmark>,
}

impl BenchmarkFramework {
    /// Benchmarks workflow execution
    pub async fn benchmark_workflow_execution(&self) -> Result<BenchmarkResult> {
        let mut c = Criterion::default();

        let workflow = create_test_workflow();

        let mut durations = Vec::new();

        c.bench_function("workflow_execution", |b| {
            b.iter(|| {
                let start = Instant::now();
                let runtime = tokio::runtime::Runtime::new().unwrap();
                runtime.block_on(async {
                    self.execute_workflow(black_box(&workflow)).await.unwrap();
                });
                let duration = start.elapsed();
                durations.push(duration);
            });
        });

        let avg_duration = durations.iter().sum::<Duration>() / durations.len() as u32;
        let min_duration = durations.iter().min().unwrap();
        let max_duration = durations.iter().max().unwrap();

        let result = BenchmarkResult {
            name: "workflow_execution".to_string(),
            avg_duration_ms: avg_duration.as_millis() as u64,
            min_duration_ms: min_duration.as_millis() as u64,
            max_duration_ms: max_duration.as_millis() as u64,
            iterations: durations.len(),
            timestamp: chrono::Utc::now().timestamp() as u64,
        };

        // Store in Cortex
        self.store_benchmark_result(&result).await?;

        Ok(result)
    }

    /// Benchmarks message passing throughput
    pub async fn benchmark_message_throughput(&self) -> Result<BenchmarkResult> {
        let message_bus = ZeroCopyMessageBus::new();
        let message = Message::new(vec![0u8; 1024], MessageMetadata::default());

        let start = Instant::now();
        let num_messages = 100_000;

        for _ in 0..num_messages {
            message_bus.send(message.clone()).await?;
        }

        let duration = start.elapsed();
        let throughput = num_messages as f64 / duration.as_secs_f64();

        let result = BenchmarkResult {
            name: "message_throughput".to_string(),
            avg_duration_ms: (duration.as_millis() / num_messages) as u64,
            min_duration_ms: 0,
            max_duration_ms: 0,
            iterations: num_messages,
            timestamp: chrono::Utc::now().timestamp() as u64,
        };

        info!("Message throughput: {:.0} msgs/sec", throughput);

        self.store_benchmark_result(&result).await?;

        Ok(result)
    }

    /// Сохраняет benchmark result в Cortex
    async fn store_benchmark_result(&self, result: &BenchmarkResult) -> Result<()> {
        // POST /metrics
        self.cortex_client
            .post("/metrics", &MetricPoint {
                name: format!("benchmark_{}", result.name),
                value: result.avg_duration_ms as f64,
                timestamp: result.timestamp,
                tags: HashMap::from([
                    ("type".to_string(), "benchmark".to_string()),
                    ("iterations".to_string(), result.iterations.to_string()),
                ]),
            })
            .await?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub avg_duration_ms: u64,
    pub min_duration_ms: u64,
    pub max_duration_ms: u64,
    pub iterations: usize,
    pub timestamp: u64,
}
```

## 5. Error Injection and Chaos Engineering

Test system resilience through controlled failure injection.

### Chaos Engineering Framework

```rust
/// Chaos engineering framework для testing resilience
pub struct ChaosEngineer {
    failure_injector: Arc<FailureInjector>,
    recovery_monitor: Arc<RecoveryMonitor>,
    cortex_client: Arc<CortexClient>,
}

impl ChaosEngineer {
    /// Инжектирует network failures
    pub async fn inject_network_failure(&self, duration: Duration) -> Result<ChaosExperiment> {
        let experiment = ChaosExperiment {
            id: Uuid::new_v4().to_string(),
            experiment_type: ExperimentType::NetworkFailure,
            duration,
            start_time: Instant::now(),
        };

        info!("Starting chaos experiment: network failure for {:?}", duration);

        // Inject failure
        self.failure_injector.inject_network_failure(duration).await?;

        // Monitor recovery
        let recovery_time = self.recovery_monitor.wait_for_recovery().await?;

        let result = experiment.complete(recovery_time);

        // Store in Cortex
        self.store_experiment_result(&result).await?;

        Ok(experiment)
    }

    /// Инжектирует agent crashes
    pub async fn inject_agent_crash(&self, agent_id: AgentId) -> Result<ChaosExperiment> {
        let experiment = ChaosExperiment {
            id: Uuid::new_v4().to_string(),
            experiment_type: ExperimentType::AgentCrash { agent_id: agent_id.clone() },
            duration: Duration::default(),
            start_time: Instant::now(),
        };

        info!("Starting chaos experiment: crashing agent {}", agent_id);

        // Crash agent
        self.failure_injector.crash_agent(&agent_id).await?;

        // Monitor system response
        let recovery_time = self.recovery_monitor.wait_for_agent_replacement(&agent_id).await?;

        let result = experiment.complete(recovery_time);

        self.store_experiment_result(&result).await?;

        Ok(experiment)
    }

    /// Инжектирует resource exhaustion
    pub async fn inject_resource_exhaustion(&self, resource: ResourceType) -> Result<ChaosExperiment> {
        let experiment = ChaosExperiment {
            id: Uuid::new_v4().to_string(),
            experiment_type: ExperimentType::ResourceExhaustion { resource },
            duration: Duration::from_secs(60),
            start_time: Instant::now(),
        };

        info!("Starting chaos experiment: {} exhaustion", resource);

        // Exhaust resource
        self.failure_injector.exhaust_resource(resource).await?;

        // Monitor system degradation and recovery
        let recovery_time = self.recovery_monitor.wait_for_resource_recovery(resource).await?;

        let result = experiment.complete(recovery_time);

        self.store_experiment_result(&result).await?;

        Ok(experiment)
    }

    async fn store_experiment_result(&self, result: &ChaosExperimentResult) -> Result<()> {
        // POST /test-results
        self.cortex_client
            .post("/test-results", result)
            .await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ChaosExperiment {
    pub id: String,
    pub experiment_type: ExperimentType,
    pub duration: Duration,
    pub start_time: Instant,
}

impl ChaosExperiment {
    fn complete(self, recovery_time: Duration) -> ChaosExperimentResult {
        ChaosExperimentResult {
            id: self.id,
            experiment_type: self.experiment_type,
            planned_duration: self.duration,
            actual_recovery_time: recovery_time,
            success: recovery_time < Duration::from_secs(30), // Target: < 30s recovery
            timestamp: chrono::Utc::now().timestamp() as u64,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum ExperimentType {
    NetworkFailure,
    AgentCrash { agent_id: AgentId },
    ResourceExhaustion { resource: ResourceType },
    CorteFailure,
    MessageLoss { rate: f64 },
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum ResourceType {
    CPU,
    Memory,
    Network,
    Disk,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChaosExperimentResult {
    pub id: String,
    pub experiment_type: ExperimentType,
    pub planned_duration: Duration,
    pub actual_recovery_time: Duration,
    pub success: bool,
    pub timestamp: u64,
}
```

## 6. Monitoring and Observability

Comprehensive monitoring for production systems.

### Monitoring System

```rust
use tracing::{info, warn, error, instrument};

/// Monitoring system для Axon
pub struct MonitoringSystem {
    metrics_collector: Arc<MetricsCollector>,
    log_aggregator: Arc<LogAggregator>,
    alert_manager: Arc<AlertManager>,
    cortex_client: Arc<CortexClient>,
}

impl MonitoringSystem {
    /// Собирает и отправляет метрики в Cortex
    #[instrument]
    pub async fn collect_metrics(&self) -> Result<()> {
        let metrics = self.metrics_collector.collect().await?;

        // Send to Cortex
        // POST /metrics/batch
        self.cortex_client
            .post("/metrics/batch", &metrics)
            .await?;

        // Check for alerts
        for metric in &metrics {
            if let Some(alert) = self.alert_manager.check_threshold(metric) {
                self.trigger_alert(alert).await?;
            }
        }

        Ok(())
    }

    /// Собирает и отправляет logs в Cortex
    #[instrument]
    pub async fn collect_logs(&self) -> Result<()> {
        let logs = self.log_aggregator.collect().await?;

        // Send to Cortex
        // POST /logs
        self.cortex_client
            .post("/logs", &logs)
            .await?;

        Ok(())
    }

    async fn trigger_alert(&self, alert: Alert) -> Result<()> {
        error!("Alert triggered: {:?}", alert);

        // Store alert in Cortex
        self.cortex_client
            .post("/alerts", &alert)
            .await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MetricsCollector {
    // System metrics
    cpu_usage: Arc<Gauge>,
    memory_usage: Arc<Gauge>,
    disk_usage: Arc<Gauge>,

    // Application metrics
    active_agents: Arc<Gauge>,
    workflow_latency: Arc<Histogram>,
    workflow_throughput: Arc<Counter>,
    error_rate: Arc<Counter>,
}

impl MetricsCollector {
    pub async fn collect(&self) -> Result<Vec<MetricPoint>> {
        let mut metrics = Vec::new();

        // System metrics
        metrics.push(MetricPoint {
            name: "cpu_usage".to_string(),
            value: self.cpu_usage.get(),
            timestamp: chrono::Utc::now().timestamp() as u64,
            tags: HashMap::from([("host".to_string(), hostname::get()?.to_string_lossy().to_string())]),
        });

        metrics.push(MetricPoint {
            name: "memory_usage".to_string(),
            value: self.memory_usage.get(),
            timestamp: chrono::Utc::now().timestamp() as u64,
            tags: HashMap::from([("host".to_string(), hostname::get()?.to_string_lossy().to_string())]),
        });

        // Application metrics
        metrics.push(MetricPoint {
            name: "active_agents".to_string(),
            value: self.active_agents.get(),
            timestamp: chrono::Utc::now().timestamp() as u64,
            tags: HashMap::new(),
        });

        metrics.push(MetricPoint {
            name: "workflow_latency_p99".to_string(),
            value: self.workflow_latency.quantile(0.99),
            timestamp: chrono::Utc::now().timestamp() as u64,
            tags: HashMap::new(),
        });

        Ok(metrics)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Alert {
    pub id: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub metric: String,
    pub threshold: f64,
    pub actual_value: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}
```

## 7. Quality Metrics

Track quality metrics over time.

```rust
/// Quality metrics tracker
pub struct QualityMetricsTracker {
    cortex_client: Arc<CortexClient>,
}

impl QualityMetricsTracker {
    /// Вычисляет quality score для workflow
    pub async fn calculate_workflow_quality(&self, workflow_id: &WorkflowId) -> Result<QualityScore> {
        // Get workflow executions from Cortex
        let executions = self.get_workflow_executions(workflow_id).await?;

        let total = executions.len() as f64;
        let successful = executions.iter().filter(|e| e.success).count() as f64;

        let success_rate = successful / total;

        // Get average duration
        let avg_duration = executions.iter()
            .map(|e| e.duration.as_secs_f64())
            .sum::<f64>() / total;

        // Get test coverage
        let test_coverage = self.get_test_coverage(workflow_id).await?;

        // Calculate quality score (weighted)
        let quality_score = (success_rate * 0.4) + 
                           ((1.0 - (avg_duration / 300.0).min(1.0)) * 0.3) +
                           (test_coverage * 0.3);

        Ok(QualityScore {
            workflow_id: workflow_id.clone(),
            success_rate,
            avg_duration_secs: avg_duration,
            test_coverage,
            quality_score,
            timestamp: chrono::Utc::now().timestamp() as u64,
        })
    }

    async fn get_workflow_executions(&self, workflow_id: &WorkflowId) -> Result<Vec<WorkflowExecution>> {
        // GET /workflows/{id}/executions
        let executions = self.cortex_client
            .get(&format!("/workflows/{}/executions", workflow_id))
            .await?
            .json::<Vec<WorkflowExecution>>()
            .await?;

        Ok(executions)
    }

    async fn get_test_coverage(&self, workflow_id: &WorkflowId) -> Result<f64> {
        // GET /test-results?workflow_id={id}
        let test_results = self.cortex_client
            .get(&format!("/test-results?workflow_id={}", workflow_id))
            .await?
            .json::<Vec<TestResult>>()
            .await?;

        if test_results.is_empty() {
            return Ok(0.0);
        }

        let passed = test_results.iter().filter(|r| r.success).count() as f64;
        Ok(passed / test_results.len() as f64)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct QualityScore {
    pub workflow_id: WorkflowId,
    pub success_rate: f64,
    pub avg_duration_secs: f64,
    pub test_coverage: f64,
    pub quality_score: f64,
    pub timestamp: u64,
}
```

## Summary

Axon's quality assurance strategy includes:

1. **Agent Validation**: 100% capability verification before task assignment
2. **Workflow Testing**: Unit, integration, and property-based tests
3. **Integration Tests**: Comprehensive Cortex API testing
4. **Performance Benchmarking**: Continuous performance tracking
5. **Chaos Engineering**: Resilience testing through failure injection
6. **Monitoring**: Real-time metrics and log aggregation
7. **Quality Metrics**: Success rate, coverage, and quality scoring

All test results, metrics, and logs are stored in Cortex via REST API (`POST /test-results`, `POST /metrics`, `POST /logs`), enabling historical analysis and continuous quality improvement.
