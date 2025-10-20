# Axon: Agent Types and Taxonomy

## Overview

This document defines the taxonomy of agents in the Axon system, their capabilities, roles, and interaction patterns. Agents are stateless executors that perform specific tasks, with all learning and memory handled by Cortex.

## Agent Architecture

### Core Agent Structure
```rust
pub struct Agent<S: AgentState> {
    // Identity
    id: AgentId,
    name: String,
    agent_type: AgentType,

    // State (runtime only, not persisted)
    state: S,

    // Capabilities
    capabilities: HashSet<Capability>,
    models: Vec<ModelConfig>,

    // Communication
    inbox: mpsc::Receiver<Message>,
    outbox: mpsc::Sender<Message>,

    // Metrics (runtime telemetry)
    metrics: AgentMetrics,
}

// Agent states (compile-time validated)
pub trait AgentState: Sealed {}

pub struct Idle;
pub struct Assigned { task: Task }
pub struct Working { task: Task, progress: Progress }
pub struct Completed { result: TaskResult }
pub struct Failed { error: Error }

impl AgentState for Idle {}
impl AgentState for Assigned {}
impl AgentState for Working {}
impl AgentState for Completed {}
impl AgentState for Failed {}
```

## Primary Agent Types

### 1. Orchestrator Agent
**Role**: Master coordination and task delegation

```rust
pub struct OrchestratorAgent {
    delegation_strategy: DelegationStrategy,
    agent_pool: HashMap<AgentId, AgentInfo>,
    workflow_queue: VecDeque<Workflow>,
}

impl OrchestratorAgent {
    pub async fn delegate_task(&self, task: Task) -> Result<AgentId> {
        // Find best agent for task
        let agent = self.select_agent(&task)?;

        // Send assignment
        agent.assign(task).await?;

        Ok(agent.id)
    }
}
```

**Capabilities**:
- Task analysis and decomposition
- Agent selection and assignment
- Load balancing
- Workflow coordination
- Deadline management

### 2. Developer Agent
**Role**: Code generation, modification, and refactoring

```rust
pub struct DeveloperAgent {
    // Agent configuration
    language_support: Vec<Language>,
    frameworks: Vec<Framework>,
    patterns: Vec<DesignPattern>,

    // Cortex integration
    cortex: Arc<CortexBridge>,
    session_id: Option<SessionId>,
}

impl DeveloperAgent {
    /// Generate code with full Cortex context
    pub async fn generate_code(&self, spec: CodeSpec) -> Result<Code> {
        // 1. Create isolated session for this task
        let session_id = self.cortex.create_session(
            self.id.clone(),
            spec.workspace_id.clone(),
            SessionScope {
                paths: vec![spec.target_path.clone()],
                read_only_paths: vec!["src/lib.rs".to_string()],
            }
        ).await?;

        // 2. Search for similar code implementations
        // POST /search/semantic
        let similar_code = self.cortex.semantic_search(
            &spec.description,
            &spec.workspace_id,
            SearchFilters {
                types: vec!["function".to_string(), "class".to_string()],
                languages: vec![spec.language.clone()],
                visibility: Some("public".to_string()),
                min_relevance: 0.7,
            }
        ).await?;

        // 3. Get learned patterns from past episodes
        // POST /memory/search
        let relevant_episodes = self.cortex.search_episodes(
            &format!("implement {} in {}", spec.feature_type, spec.language),
            5
        ).await?;

        // 4. Get design patterns
        // GET /memory/patterns
        let patterns = self.cortex.get_patterns().await?;

        // 5. Get code units (dependencies we might need)
        // GET /workspaces/{id}/units
        let units = self.cortex.get_code_units(
            &spec.workspace_id,
            UnitFilters {
                unit_type: Some("function".to_string()),
                language: Some(spec.language.clone()),
                visibility: Some("public".to_string()),
            }
        ).await?;

        // 6. Synthesize code with rich context
        let context = CodeGenerationContext {
            similar_implementations: similar_code,
            past_episodes: relevant_episodes,
            patterns: patterns.into_iter()
                .filter(|p| p.language == spec.language)
                .collect(),
            available_dependencies: units,
        };

        let code = self.synthesize_code(spec.clone(), context)?;

        // 7. Validate syntax and semantics
        self.validate_code(&code)?;

        // 8. Write generated code to session
        // PUT /sessions/{id}/files/{path}
        self.cortex.write_file_in_session(
            &session_id,
            &spec.target_path,
            &code.content,
        ).await?;

        // 9. Merge changes back to main workspace
        // POST /sessions/{id}/merge
        let merge_report = self.cortex.merge_session(
            &session_id,
            MergeStrategy::Auto,
        ).await?;

        if merge_report.conflicts_resolved > 0 {
            return Err(Error::MergeConflicts(merge_report.conflicts));
        }

        // 10. Store episode for future learning
        // POST /memory/episodes
        let episode = Episode {
            task_description: spec.description.clone(),
            agent_id: self.id.clone(),
            outcome: "success".to_string(),
            duration_seconds: 120,
            solution_summary: format!("Generated {} for {}", spec.feature_type, spec.target_path),
            entities_modified: vec![spec.target_path.clone()],
            files_touched: vec![spec.target_path.clone()],
            patterns_learned: vec!["code_generation".to_string()],
        };
        self.cortex.store_episode(episode).await?;

        // 11. Cleanup session
        self.cortex.close_session(&session_id, &self.id).await?;

        Ok(code)
    }

    /// Refactor existing code with context awareness
    pub async fn refactor_code(
        &self,
        file_path: &str,
        refactoring_type: RefactoringType,
    ) -> Result<RefactoringResult> {
        // 1. Get current file from Cortex VFS
        // GET /workspaces/{id}/files
        let current_code = self.cortex.read_file_in_session(
            &self.session_id.as_ref().unwrap(),
            file_path,
        ).await?;

        // 2. Get code unit details
        // GET /units/{id}
        let units = self.cortex.get_code_units(
            &self.workspace_id,
            UnitFilters {
                unit_type: None,
                language: Some("rust".to_string()),
                visibility: None,
            }
        ).await?;

        // 3. Search for similar refactorings
        // POST /memory/search
        let similar_refactorings = self.cortex.search_episodes(
            &format!("refactor {:?}", refactoring_type),
            10,
        ).await?;

        // 4. Perform refactoring with context
        let refactored = self.apply_refactoring(
            current_code,
            units,
            similar_refactorings,
            refactoring_type,
        )?;

        // 5. Write back to session
        // PUT /sessions/{id}/files/{path}
        self.cortex.write_file_in_session(
            &self.session_id.as_ref().unwrap(),
            file_path,
            &refactored.content,
        ).await?;

        Ok(refactored)
    }
}

// Supporting types
#[derive(Debug, Clone)]
pub struct CodeGenerationContext {
    pub similar_implementations: Vec<CodeSearchResult>,
    pub past_episodes: Vec<Episode>,
    pub patterns: Vec<Pattern>,
    pub available_dependencies: Vec<CodeUnit>,
}
```

**Cortex API Usage:**
- `POST /sessions` - Create isolated workspace
- `POST /search/semantic` - Find similar code
- `POST /memory/search` - Get relevant episodes
- `GET /memory/patterns` - Retrieve patterns
- `GET /workspaces/{id}/units` - Get code units
- `PUT /sessions/{id}/files/{path}` - Write code
- `POST /sessions/{id}/merge` - Merge changes
- `POST /memory/episodes` - Store learning
- `DELETE /sessions/{id}` - Cleanup

**Capabilities**:
- Context-aware code generation
- Intelligent refactoring
- Pattern-based bug fixing
- API implementation with examples
- Test generation from episodes

### 3. Reviewer Agent
**Role**: Code review, quality assessment, and validation

```rust
pub struct ReviewerAgent {
    // Agent configuration
    review_checklist: ReviewChecklist,
    quality_metrics: QualityMetrics,
    linting_rules: Vec<LintRule>,

    // Cortex integration
    cortex: Arc<CortexBridge>,
    session_id: Option<SessionId>,
}

impl ReviewerAgent {
    /// Review code with historical context and patterns
    pub async fn review_code(&self, file_path: &str, workspace_id: &WorkspaceId) -> Result<ReviewReport> {
        let mut report = ReviewReport::new();

        // 1. Read file from session
        // GET /sessions/{id}/files/{path}
        let code = self.cortex.read_file_in_session(
            &self.session_id.as_ref().unwrap(),
            file_path,
        ).await?;

        // 2. Get code units with dependencies
        // GET /workspaces/{id}/units
        let units = self.cortex.get_code_units(
            workspace_id,
            UnitFilters {
                unit_type: None,
                language: Some("rust".to_string()),
                visibility: None,
            }
        ).await?;

        // 3. Search for similar code reviews in past episodes
        // POST /memory/search
        let past_reviews = self.cortex.search_episodes(
            "code review security performance",
            10,
        ).await?;

        // 4. Get quality patterns
        // GET /memory/patterns
        let quality_patterns = self.cortex.get_patterns().await?
            .into_iter()
            .filter(|p| p.name.contains("quality") || p.name.contains("review"))
            .collect::<Vec<_>>();

        // 5. Perform static analysis
        report.add_static_analysis(self.analyze_static(&code, &units)?);

        // 6. Security review with known vulnerability patterns
        report.add_security_review(
            self.check_security(&code, &past_reviews, &quality_patterns)?
        );

        // 7. Best practices validation
        report.add_best_practices(
            self.check_practices(&code, &quality_patterns)?
        );

        // 8. Performance analysis
        report.add_performance(self.analyze_performance(&code, &units)?);

        // 9. Check test coverage
        // POST /search/semantic
        let tests = self.cortex.semantic_search(
            &format!("tests for {}", file_path),
            workspace_id,
            SearchFilters {
                types: vec!["function".to_string()],
                languages: vec!["rust".to_string()],
                visibility: None,
                min_relevance: 0.6,
            }
        ).await?;

        report.test_coverage = self.calculate_coverage(&code, &tests);

        // 10. Store review episode for learning
        // POST /memory/episodes
        let episode = Episode {
            task_description: format!("Review code in {}", file_path),
            agent_id: self.id.clone(),
            outcome: if report.is_acceptable() { "success" } else { "issues_found" }.to_string(),
            duration_seconds: 45,
            solution_summary: report.summary.clone(),
            entities_modified: vec![],
            files_touched: vec![file_path.to_string()],
            patterns_learned: report.issues.iter()
                .map(|i| i.pattern_name.clone())
                .collect(),
        };
        self.cortex.store_episode(episode).await?;

        Ok(report)
    }

    /// Analyze impact of changes using dependency graph
    pub async fn analyze_impact(
        &self,
        changed_files: Vec<String>,
        workspace_id: &WorkspaceId,
    ) -> Result<ImpactAnalysis> {
        // POST /analysis/impact
        let request = ImpactAnalysisRequest {
            changed_entities: changed_files.clone(),
            analysis_type: "full".to_string(),
            max_depth: 10,
        };

        // Note: This endpoint is defined in Cortex REST API spec
        let response = self.cortex.client
            .post(&format!("/analysis/impact"))
            .json(&request)
            .send()
            .await?;

        let analysis: ImpactAnalysis = response.json().await?;

        Ok(analysis)
    }
}
```

**Cortex API Usage:**
- `GET /sessions/{id}/files/{path}` - Read code to review
- `GET /workspaces/{id}/units` - Get code structure
- `POST /memory/search` - Find past review patterns
- `GET /memory/patterns` - Get quality patterns
- `POST /search/semantic` - Find related tests
- `POST /analysis/impact` - Analyze change impact
- `POST /memory/episodes` - Store review results

**Capabilities**:
- Pattern-aware code review
- Historical vulnerability detection
- Performance assessment with context
- Best practice validation from episodes
- Test coverage analysis

### 4. Tester Agent
**Role**: Test generation, execution, and validation

```rust
pub struct TesterAgent {
    // Agent configuration
    test_frameworks: Vec<TestFramework>,
    coverage_requirements: CoverageRequirements,
    test_strategies: Vec<TestStrategy>,

    // Cortex integration
    cortex: Arc<CortexBridge>,
    session_id: Option<SessionId>,
}

impl TesterAgent {
    /// Generate comprehensive tests using past testing patterns
    pub async fn generate_tests(
        &self,
        file_path: &str,
        workspace_id: &WorkspaceId,
    ) -> Result<TestSuite> {
        // 1. Read code to test
        // GET /sessions/{id}/files/{path}
        let code = self.cortex.read_file_in_session(
            &self.session_id.as_ref().unwrap(),
            file_path,
        ).await?;

        // 2. Get code units to understand structure
        // GET /units/{id}
        let units = self.cortex.get_code_units(
            workspace_id,
            UnitFilters {
                unit_type: Some("function".to_string()),
                language: Some("rust".to_string()),
                visibility: Some("public".to_string()),
            }
        ).await?;

        // 3. Search for similar test generation episodes
        // POST /memory/search
        let similar_tests = self.cortex.search_episodes(
            &format!("generate tests for {}", file_path),
            5,
        ).await?;

        // 4. Get testing patterns
        // GET /memory/patterns
        let test_patterns = self.cortex.get_patterns().await?
            .into_iter()
            .filter(|p| p.name.contains("test") || p.name.contains("coverage"))
            .collect::<Vec<_>>();

        // 5. Find existing tests for reference
        // POST /search/semantic
        let existing_tests = self.cortex.semantic_search(
            "unit tests integration tests",
            workspace_id,
            SearchFilters {
                types: vec!["function".to_string()],
                languages: vec!["rust".to_string()],
                visibility: None,
                min_relevance: 0.6,
            }
        ).await?;

        // 6. Analyze code structure and dependencies
        let structure = self.analyze_structure(&code, &units)?;

        // 7. Generate test cases with context
        let test_cases = self.generate_test_cases(
            &structure,
            &similar_tests,
            &test_patterns,
            &existing_tests,
        )?;

        // 8. Create test suite
        let suite = TestSuite::from_cases(test_cases);

        // 9. Write tests to session
        let test_file = format!("tests/{}_test.rs",
            Path::new(file_path).file_stem().unwrap().to_str().unwrap()
        );

        // PUT /sessions/{id}/files/{path}
        self.cortex.write_file_in_session(
            &self.session_id.as_ref().unwrap(),
            &test_file,
            &suite.to_code(),
        ).await?;

        // 10. Store episode
        // POST /memory/episodes
        let episode = Episode {
            task_description: format!("Generate tests for {}", file_path),
            agent_id: self.id.clone(),
            outcome: "success".to_string(),
            duration_seconds: 90,
            solution_summary: format!("Generated {} test cases", test_cases.len()),
            entities_modified: vec![test_file.clone()],
            files_touched: vec![test_file],
            patterns_learned: vec!["test_generation".to_string()],
        };
        self.cortex.store_episode(episode).await?;

        Ok(suite)
    }

    /// Run tests and analyze coverage
    pub async fn run_tests_with_coverage(
        &self,
        test_pattern: &str,
        workspace_id: &WorkspaceId,
    ) -> Result<TestResults> {
        // POST /test/run
        let request = RunTestsRequest {
            workspace_id: workspace_id.to_string(),
            test_pattern: test_pattern.to_string(),
            coverage: true,
        };

        let response = self.cortex.client
            .post("/test/run")
            .json(&request)
            .send()
            .await?;

        let results: TestResults = response.json().await?;

        // Store test run episode
        let episode = Episode {
            task_description: format!("Run tests {}", test_pattern),
            agent_id: self.id.clone(),
            outcome: if results.all_passed() { "success" } else { "failures" }.to_string(),
            duration_seconds: results.duration.as_secs() as i64,
            solution_summary: format!("{}/{} tests passed, coverage: {:.2}%",
                results.passed, results.total, results.coverage * 100.0),
            entities_modified: vec![],
            files_touched: vec![],
            patterns_learned: vec![],
        };
        self.cortex.store_episode(episode).await?;

        Ok(results)
    }
}
```

**Cortex API Usage:**
- `GET /sessions/{id}/files/{path}` - Read code to test
- `GET /workspaces/{id}/units` - Analyze code structure
- `POST /memory/search` - Find test generation patterns
- `GET /memory/patterns` - Get testing patterns
- `POST /search/semantic` - Find existing test examples
- `PUT /sessions/{id}/files/{path}` - Write generated tests
- `POST /test/run` - Execute tests with coverage
- `POST /memory/episodes` - Store test results

**Capabilities**:
- Context-aware test generation
- Pattern-based test case creation
- Integration test generation
- Property-based testing with examples
- Coverage analysis and reporting

### 5. Documenter Agent
**Role**: Documentation generation and maintenance

```rust
pub struct DocumenterAgent {
    doc_formats: Vec<DocFormat>,
    templates: HashMap<DocType, Template>,
    style_guides: Vec<StyleGuide>,
}

impl DocumenterAgent {
    pub async fn generate_docs(&self, artifact: Artifact) -> Result<Documentation> {
        // Extract documentation points
        let doc_points = self.extract_doc_points(&artifact)?;

        // Generate structured documentation
        let docs = self.create_documentation(doc_points)?;

        // Format according to style
        let formatted = self.format_docs(docs)?;

        Ok(formatted)
    }
}
```

**Capabilities**:
- API documentation
- Code comments
- README generation
- Architecture diagrams
- User guides

### 6. Architect Agent
**Role**: System design and architecture planning

```rust
pub struct ArchitectAgent {
    // Agent configuration
    design_patterns: Vec<Pattern>,
    architectural_styles: Vec<ArchStyle>,
    trade_off_analyzer: TradeOffAnalyzer,

    // Cortex integration
    cortex: Arc<CortexBridge>,
    session_id: Option<SessionId>,
}

impl ArchitectAgent {
    /// Design system architecture using past architectural patterns
    pub async fn design_system(
        &self,
        requirements: Requirements,
        workspace_id: &WorkspaceId,
    ) -> Result<Architecture> {
        // 1. Search for similar architectural decisions
        // POST /memory/search
        let similar_architectures = self.cortex.search_episodes(
            &format!("system architecture design {} {}",
                requirements.system_type, requirements.scale),
            10,
        ).await?;

        // 2. Get architectural patterns
        // GET /memory/patterns
        let arch_patterns = self.cortex.get_patterns().await?
            .into_iter()
            .filter(|p| p.name.contains("architecture") || p.name.contains("design"))
            .collect::<Vec<_>>();

        // 3. Analyze existing codebase structure
        // GET /workspaces/{id}/dependencies
        let dependencies = self.cortex.client
            .get(&format!("/workspaces/{}/dependencies?format=json&level=file", workspace_id))
            .send()
            .await?
            .json::<DependencyGraph>()
            .await?;

        // 4. Get code units to understand current structure
        // GET /workspaces/{id}/units
        let units = self.cortex.get_code_units(
            workspace_id,
            UnitFilters {
                unit_type: None,
                language: None,
                visibility: Some("public".to_string()),
            }
        ).await?;

        // 5. Analyze requirements
        let analysis = self.analyze_requirements(&requirements)?;

        // 6. Synthesize architecture using context
        let architecture = self.synthesize_architecture(
            analysis,
            similar_architectures,
            arch_patterns,
            dependencies,
            units,
        )?;

        // 7. Validate design
        self.validate_architecture(&architecture)?;

        // 8. Generate architecture documentation
        let arch_doc = self.generate_architecture_doc(&architecture)?;

        // 9. Write architecture doc to session
        // PUT /sessions/{id}/files/{path}
        self.cortex.write_file_in_session(
            &self.session_id.as_ref().unwrap(),
            "docs/ARCHITECTURE.md",
            &arch_doc,
        ).await?;

        // 10. Store architectural decision
        // POST /memory/episodes
        let episode = Episode {
            task_description: format!("Design architecture for {}", requirements.system_type),
            agent_id: self.id.clone(),
            outcome: "success".to_string(),
            duration_seconds: 180,
            solution_summary: architecture.summary.clone(),
            entities_modified: vec!["docs/ARCHITECTURE.md".to_string()],
            files_touched: vec!["docs/ARCHITECTURE.md".to_string()],
            patterns_learned: architecture.patterns_used.clone(),
        };
        self.cortex.store_episode(episode).await?;

        Ok(architecture)
    }

    /// Analyze impact of architectural changes
    pub async fn analyze_arch_impact(
        &self,
        proposed_changes: Vec<ArchChange>,
        workspace_id: &WorkspaceId,
    ) -> Result<ArchImpactReport> {
        // POST /analysis/impact
        let changed_entities: Vec<String> = proposed_changes.iter()
            .flat_map(|c| c.affected_modules.clone())
            .collect();

        let request = ImpactAnalysisRequest {
            changed_entities,
            analysis_type: "full".to_string(),
            max_depth: 999,  // Full dependency tree
        };

        let response = self.cortex.client
            .post("/analysis/impact")
            .json(&request)
            .send()
            .await?;

        let impact: ImpactAnalysis = response.json().await?;

        // Generate report
        let report = ArchImpactReport {
            directly_affected: impact.directly_affected,
            transitively_affected: impact.transitively_affected,
            risk_level: impact.risk_assessment.level,
            recommendations: self.generate_recommendations(&impact),
        };

        Ok(report)
    }

    /// Detect circular dependencies
    pub async fn detect_circular_deps(
        &self,
        workspace_id: &WorkspaceId,
    ) -> Result<Vec<Cycle>> {
        // GET /analysis/cycles
        let response = self.cortex.client
            .get(&format!("/analysis/cycles?workspace_id={}", workspace_id))
            .send()
            .await?;

        let cycles: CyclesResponse = response.json().await?;

        Ok(cycles.cycles)
    }
}
```

**Cortex API Usage:**
- `POST /memory/search` - Find similar architecture decisions
- `GET /memory/patterns` - Get architectural patterns
- `GET /workspaces/{id}/dependencies` - Analyze dependency graph
- `GET /workspaces/{id}/units` - Understand current structure
- `PUT /sessions/{id}/files/{path}` - Write architecture docs
- `POST /analysis/impact` - Analyze impact of changes
- `GET /analysis/cycles` - Detect circular dependencies
- `POST /memory/episodes` - Store architectural decisions

**Capabilities**:
- Pattern-based system design
- Dependency-aware component architecture
- Impact analysis for changes
- Database schema design with examples
- API design from past successes
- Circular dependency detection

### 7. Researcher Agent
**Role**: Information gathering and analysis

```rust
pub struct ResearcherAgent {
    search_strategies: Vec<SearchStrategy>,
    information_sources: Vec<InfoSource>,
    analysis_methods: Vec<AnalysisMethod>,
}

impl ResearcherAgent {
    pub async fn research_topic(&self, query: ResearchQuery) -> Result<ResearchReport> {
        // Search for information
        let raw_info = self.gather_information(&query).await?;

        // Filter and validate
        let validated = self.validate_information(raw_info)?;

        // Analyze and synthesize
        let analysis = self.analyze_information(validated)?;

        // Create report
        let report = self.create_report(analysis)?;

        Ok(report)
    }
}
```

**Capabilities**:
- Information retrieval
- Fact checking
- Trend analysis
- Technology research
- Best practice discovery

### 8. Optimizer Agent
**Role**: Performance and cost optimization

```rust
pub struct OptimizerAgent {
    optimization_strategies: Vec<OptimizationStrategy>,
    profiling_tools: Vec<ProfilingTool>,
    cost_models: Vec<CostModel>,
}

impl OptimizerAgent {
    pub async fn optimize(&self, target: OptimizationTarget) -> Result<OptimizationReport> {
        // Profile current state
        let profile = self.create_profile(&target)?;

        // Identify bottlenecks
        let bottlenecks = self.identify_bottlenecks(&profile)?;

        // Generate optimizations
        let optimizations = self.generate_optimizations(bottlenecks)?;

        // Validate improvements
        let validated = self.validate_optimizations(optimizations)?;

        Ok(OptimizationReport::new(validated))
    }
}
```

**Capabilities**:
- Performance profiling
- Code optimization
- Resource optimization
- Cost reduction
- Scalability improvements

## Capability Model

### Core Capabilities
```rust
pub enum Capability {
    // Code Operations
    CodeGeneration,
    CodeReview,
    CodeRefactoring,
    CodeOptimization,

    // Testing
    TestGeneration,
    TestExecution,
    CoverageAnalysis,

    // Documentation
    DocGeneration,
    DiagramCreation,

    // Analysis
    StaticAnalysis,
    SecurityAnalysis,
    PerformanceAnalysis,

    // Design
    SystemDesign,
    APIDesign,
    DatabaseDesign,

    // Research
    InformationRetrieval,
    FactChecking,
    TrendAnalysis,
}
```

### Capability Matching
```rust
pub struct CapabilityMatcher {
    agent_capabilities: HashMap<AgentId, HashSet<Capability>>,
    task_requirements: HashMap<TaskType, HashSet<Capability>>,
}

impl CapabilityMatcher {
    pub fn find_capable_agents(&self, task: &Task) -> Vec<AgentId> {
        let required = &self.task_requirements[&task.task_type];

        self.agent_capabilities
            .iter()
            .filter(|(_, capabilities)| {
                required.is_subset(capabilities)
            })
            .map(|(id, _)| id.clone())
            .collect()
    }
}
```

## Agent Lifecycle

### State Transitions
```
┌──────┐
│ Idle │◀────────────────┐
└───┬──┘                 │
    │ assign()           │
┌───▼─────┐              │
│Assigned │              │
└───┬─────┘              │
    │ start()            │ complete()
┌───▼────┐               │
│Working │───────────────┤
└───┬────┘               │
    │                    │
    │ fail()       ┌─────▼────┐
    └─────────────▶│Completed │
                   └──────────┘
         ┌────────┐
         │ Failed │
         └────────┘
```

### Lifecycle Management
```rust
impl<S: AgentState> Agent<S> {
    // State transitions enforced at compile time
}

impl Agent<Idle> {
    pub fn assign(self, task: Task) -> Agent<Assigned> {
        Agent {
            state: Assigned { task },
            ..self
        }
    }
}

impl Agent<Assigned> {
    pub fn start(self) -> Agent<Working> {
        Agent {
            state: Working {
                task: self.state.task,
                progress: Progress::new()
            },
            ..self
        }
    }
}

impl Agent<Working> {
    pub fn complete(self, result: TaskResult) -> Agent<Completed> {
        Agent {
            state: Completed { result },
            ..self
        }
    }

    pub fn fail(self, error: Error) -> Agent<Failed> {
        Agent {
            state: Failed { error },
            ..self
        }
    }
}
```

## Agent Communication

### Message Types
```rust
pub enum AgentMessage {
    // Task Management
    TaskAssignment { task: Task },
    TaskUpdate { progress: Progress },
    TaskComplete { result: TaskResult },

    // Coordination
    CoordinationRequest { context: Context },
    CoordinationResponse { decision: Decision },

    // Information Sharing
    InfoRequest { query: Query },
    InfoResponse { data: Data },

    // Control
    Pause,
    Resume,
    Terminate,
}
```

### Communication Patterns
```rust
// Direct Communication
impl Agent {
    pub async fn send_to(&self, target: AgentId, message: AgentMessage) -> Result<()> {
        self.outbox.send((target, message)).await
    }
}

// Broadcast
impl Agent {
    pub async fn broadcast(&self, message: AgentMessage) -> Result<()> {
        self.outbox.send((AgentId::BROADCAST, message)).await
    }
}

// Request-Response
impl Agent {
    pub async fn request(&self, target: AgentId, request: Request) -> Result<Response> {
        let (tx, rx) = oneshot::channel();
        self.send_to(target, AgentMessage::Request { request, reply: tx }).await?;
        Ok(rx.await?)
    }
}
```

## Agent Specialization

### Specialization Hierarchy
```
Agent (Base)
├── Orchestrator
│   ├── WorkflowOrchestrator
│   └── SwarmOrchestrator
├── Developer
│   ├── FrontendDeveloper
│   ├── BackendDeveloper
│   └── FullStackDeveloper
├── Reviewer
│   ├── SecurityReviewer
│   └── PerformanceReviewer
├── Tester
│   ├── UnitTester
│   └── IntegrationTester
├── Documenter
│   ├── APIDocumenter
│   └── UserDocumenter
├── Architect
│   ├── SolutionArchitect
│   └── DataArchitect
├── Researcher
│   ├── TechResearcher
│   └── MarketResearcher
└── Optimizer
    ├── CodeOptimizer
    └── CostOptimizer
```

### Custom Agent Creation
```rust
pub trait CustomAgent: Agent {
    fn custom_capability(&self) -> Capability;
    async fn custom_action(&self, input: Input) -> Result<Output>;
}

// Example custom agent
pub struct SecurityAuditorAgent {
    base: Agent<Idle>,
    security_tools: Vec<SecurityTool>,
    vulnerability_db: VulnerabilityDB,
}

impl CustomAgent for SecurityAuditorAgent {
    fn custom_capability(&self) -> Capability {
        Capability::SecurityAudit
    }

    async fn custom_action(&self, input: Input) -> Result<Output> {
        // Custom security audit logic
    }
}
```

## Performance Characteristics

### Agent Metrics
```rust
pub struct AgentMetrics {
    // Performance
    tasks_completed: Counter,
    tasks_failed: Counter,
    avg_task_duration: Histogram,

    // Resource Usage
    memory_usage: Gauge,
    cpu_usage: Gauge,

    // Quality
    success_rate: Gauge,
    quality_score: Gauge,
}
```

### Agent Scoring
```rust
pub struct AgentScorer {
    pub fn score_agent(&self, agent: &Agent, task: &Task) -> f32 {
        let mut score = 0.0;

        // Capability match
        score += self.capability_score(agent, task) * 0.4;

        // Historical performance
        score += self.performance_score(agent, task) * 0.3;

        // Current load
        score += self.availability_score(agent) * 0.2;

        // Specialization
        score += self.specialization_score(agent, task) * 0.1;

        score
    }
}
```

## Agent-Cortex Integration Patterns

### Common Pattern: Agent Task Execution

All agents follow this standard pattern when executing tasks:

```rust
pub async fn execute_task_with_cortex<A: Agent>(
    agent: &A,
    task: Task,
    cortex: &CortexBridge,
) -> Result<TaskResult> {
    // 1. SESSION CREATION - Isolated workspace
    let session_id = cortex.create_session(
        agent.id(),
        task.workspace_id.clone(),
        SessionScope {
            paths: task.scope_paths.clone(),
            read_only_paths: task.readonly_paths.clone(),
        },
    ).await?;

    // 2. CONTEXT RETRIEVAL - Learn from past
    let context = retrieve_context(cortex, &task).await?;

    // 3. EXECUTION - Agent works in isolation
    let result = agent.execute_with_context(
        task.clone(),
        session_id.clone(),
        context,
    ).await?;

    // 4. MERGE - Integrate changes
    let merge_report = cortex.merge_session(
        &session_id,
        MergeStrategy::Auto,
    ).await?;

    // 5. LEARNING - Store episode
    store_episode(cortex, &agent, &task, &result).await?;

    // 6. CLEANUP
    cortex.close_session(&session_id, &agent.id()).await?;

    Ok(result)
}

async fn retrieve_context(
    cortex: &CortexBridge,
    task: &Task,
) -> Result<AgentContext> {
    // Parallel context retrieval for performance
    let (episodes, patterns, units, similar_code) = tokio::join!(
        // Past episodes
        cortex.search_episodes(&task.description, 5),

        // Learned patterns
        cortex.get_patterns(),

        // Code structure
        cortex.get_code_units(&task.workspace_id, UnitFilters::default()),

        // Similar implementations
        cortex.semantic_search(&task.description, &task.workspace_id, SearchFilters::default()),
    );

    Ok(AgentContext {
        episodes: episodes?,
        patterns: patterns?,
        code_units: units?,
        similar_implementations: similar_code?,
    })
}

async fn store_episode(
    cortex: &CortexBridge,
    agent: &impl Agent,
    task: &Task,
    result: &TaskResult,
) -> Result<()> {
    let episode = Episode {
        task_description: task.description.clone(),
        agent_id: agent.id().clone(),
        outcome: if result.success { "success" } else { "failure" }.to_string(),
        duration_seconds: result.duration.as_secs() as i64,
        solution_summary: result.summary.clone(),
        entities_modified: result.modified_entities.clone(),
        files_touched: result.modified_files.clone(),
        patterns_learned: result.patterns_discovered.clone(),
    };

    cortex.store_episode(episode).await?;
    Ok(())
}
```

### Agent Type → Cortex API Mapping

| Agent Type | Primary Cortex APIs | Purpose |
|-----------|---------------------|---------|
| **Developer** | `POST /sessions`<br>`POST /search/semantic`<br>`POST /memory/search`<br>`PUT /sessions/{id}/files/{path}` | Session isolation<br>Find similar code<br>Get patterns<br>Write code |
| **Reviewer** | `GET /workspaces/{id}/units`<br>`POST /memory/search`<br>`POST /analysis/impact` | Analyze structure<br>Past review patterns<br>Impact analysis |
| **Tester** | `GET /workspaces/{id}/units`<br>`POST /search/semantic`<br>`POST /test/run` | Understand code<br>Find test examples<br>Execute tests |
| **Architect** | `GET /workspaces/{id}/dependencies`<br>`POST /memory/search`<br>`GET /analysis/cycles` | Dependency graph<br>Architecture patterns<br>Detect cycles |
| **Documenter** | `POST /search/semantic`<br>`GET /workspaces/{id}/units`<br>`PUT /sessions/{id}/files/{path}` | Find examples<br>Code structure<br>Write docs |
| **Optimizer** | `GET /workspaces/{id}/units`<br>`POST /memory/search`<br>`POST /analysis/impact` | Performance analysis<br>Optimization patterns<br>Impact assessment |

### Episodic Memory Learning Cycle

```
┌────────────────────────────────────────────────────────────────┐
│                  Episodic Memory Learning                       │
└────────────────────────────────────────────────────────────────┘

   Task Execution
        │
        ▼
   ┌────────────────┐
   │  Agent works   │
   │  in session    │
   └────────┬───────┘
            │
            │ Captures:
            │ - Task description
            │ - Solution approach
            │ - Patterns used
            │ - Outcomes
            │ - Duration
            ▼
   ┌────────────────┐
   │ POST /      │
   │ memory/        │───────┐
   │ episodes       │       │
   └────────────────┘       │
                            │
                            ▼
                   ╔════════════════════╗
                   ║  Cortex stores &   ║
                   ║  analyzes episode  ║
                   ║  - Extracts patterns
                   ║  - Updates knowledge
                   ║  - Builds connections
                   ╚════════════════════╝
                            │
                            │ Available for
                            │ future agents
                            ▼
   ┌────────────────────────────────────┐
   │  Next similar task                 │
   │  GET /memory/search            │
   │  → Returns relevant episodes       │
   │  → Agent learns from past          │
   └────────────────────────────────────┘
```

### Session Isolation Benefits

**For Individual Agents:**
- Work without fear of conflicts
- See consistent snapshot of codebase
- Changes isolated until merge
- Automatic rollback on failure

**For Multi-Agent Workflows:**
- Parallel execution without blocking
- Fine-grained lock acquisition only when needed
- Automatic conflict detection and resolution
- Clear audit trail of all changes

**For System Reliability:**
- Atomic operations at session level
- ACID guarantees from Cortex
- Version history maintained
- Easy debugging with session replay

### Performance Optimizations

**Cortex Bridge implements:**
1. **Connection Pooling** - Reuse HTTP connections
2. **Response Caching** - Cache frequent queries (episodes, patterns)
3. **Batch Operations** - Multiple API calls in parallel
4. **WebSocket Events** - Real-time notifications instead of polling
5. **Lazy Loading** - Only fetch data when needed

**Example: Parallel Context Retrieval**
```rust
// Bad: Sequential (slow)
let episodes = cortex.search_episodes(query, 5).await?;
let patterns = cortex.get_patterns().await?;
let units = cortex.get_code_units(workspace_id, filters).await?;
// Total: ~300ms

// Good: Parallel (fast)
let (episodes, patterns, units) = tokio::join!(
    cortex.search_episodes(query, 5),
    cortex.get_patterns(),
    cortex.get_code_units(workspace_id, filters),
);
// Total: ~100ms (3x faster)
```

---

## Summary

This architecture provides:

1. **Clear Separation**: Axon orchestrates, Cortex persists
2. **Agent Statelessness**: All state in Cortex sessions
3. **Shared Learning**: Episodic memory across all agents
4. **Safe Concurrency**: Session isolation prevents conflicts
5. **Context Awareness**: Semantic search and patterns
6. **Performance**: Caching and parallel operations
7. **Reliability**: ACID transactions and rollback

Every agent type leverages Cortex differently based on its role, but all follow the same fundamental pattern: create session, retrieve context, execute, merge, learn, cleanup. This consistency makes the system maintainable and predictable while allowing each agent to specialize in its domain.