//! Architect Agent Implementation
//!
//! The Architect Agent specializes in system design and architecture planning.
//! It provides capabilities for:
//! - System architecture design
//! - Dependency analysis
//! - Architecture pattern detection
//! - Refactoring recommendations
//! - Integration with CortexBridge for Knowledge Graph

use super::*;
use crate::cortex_bridge::{
    CortexBridge, Episode, EpisodeOutcome, EpisodeType, PatternType,
    SearchFilters, WorkspaceId,
};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Architect agent for system design and architecture planning
pub struct ArchitectAgent {
    id: AgentId,
    name: String,
    capabilities: HashSet<Capability>,
    metrics: AgentMetrics,

    // Architecture-specific configuration
    design_patterns: Vec<String>,
    architectural_styles: Vec<ArchitecturalStyle>,

    // Cortex integration (optional)
    cortex: Option<Arc<CortexBridge>>,
    workspace_id: Option<WorkspaceId>,
}

/// Architectural styles supported by the architect agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArchitecturalStyle {
    /// Layered architecture
    Layered,

    /// Microservices architecture
    Microservices,

    /// Event-driven architecture
    EventDriven,

    /// Hexagonal (Ports and Adapters) architecture
    Hexagonal,

    /// Service-oriented architecture
    ServiceOriented,

    /// Component-based architecture
    ComponentBased,

    /// Monolithic architecture
    Monolithic,

    /// Serverless architecture
    Serverless,
}

/// Requirements for system design
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemRequirements {
    /// Type of system being designed
    pub system_type: String,

    /// Expected scale (users, requests, data volume)
    pub scale: ScaleRequirements,

    /// Quality attributes (performance, security, etc.)
    pub quality_attributes: Vec<QualityAttribute>,

    /// Technology constraints
    pub constraints: Vec<String>,

    /// Integration requirements
    pub integrations: Vec<String>,
}

/// Scale requirements for the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScaleRequirements {
    /// Expected number of users
    pub users: u64,

    /// Expected requests per second
    pub requests_per_second: u64,

    /// Expected data volume in GB
    pub data_volume_gb: u64,
}

/// Quality attributes for architecture
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityAttribute {
    Performance,
    Security,
    Scalability,
    Reliability,
    Maintainability,
    Testability,
    Usability,
    Availability,
}

/// Architecture design result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Architecture {
    /// Architecture summary
    pub summary: String,

    /// Components in the system
    pub components: Vec<Component>,

    /// Patterns used
    pub patterns_used: Vec<String>,

    /// Architecture style
    pub style: ArchitecturalStyle,

    /// Design decisions and rationale
    pub decisions: Vec<DesignDecision>,
}

/// Component in the architecture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub name: String,
    pub description: String,
    pub responsibilities: Vec<String>,
    pub dependencies: Vec<String>,
}

/// Design decision with rationale
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignDecision {
    pub decision: String,
    pub rationale: String,
    pub alternatives_considered: Vec<String>,
    pub trade_offs: Vec<String>,
}

/// Dependency analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyAnalysis {
    /// Total number of dependencies
    pub total_dependencies: usize,

    /// Circular dependencies detected
    pub circular_dependencies: Vec<CircularDependency>,

    /// Dependency depth (longest chain)
    pub max_depth: usize,

    /// Recommendations for improvement
    pub recommendations: Vec<String>,
}

/// Circular dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircularDependency {
    pub cycle: Vec<String>,
    pub severity: DependencySeverity,
}

/// Severity of dependency issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Refactoring proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringProposal {
    pub target: String,
    pub refactoring_type: RefactoringType,
    pub description: String,
    pub benefits: Vec<String>,
    pub risks: Vec<String>,
    pub estimated_effort: EstimatedEffort,
}

/// Type of refactoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RefactoringType {
    ExtractComponent,
    SplitModule,
    MergeModules,
    IntroduceInterface,
    SimplifyDependencies,
    ApplyPattern,
}

/// Estimated effort for refactoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EstimatedEffort {
    Small,   // < 1 day
    Medium,  // 1-3 days
    Large,   // 3-7 days
    XLarge,  // > 1 week
}

impl ArchitectAgent {
    /// Create a new architect agent with default configuration (no Cortex)
    pub fn new(name: String) -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::SystemDesign);
        capabilities.insert(Capability::ArchitectureAnalysis);
        capabilities.insert(Capability::DependencyAnalysis);
        capabilities.insert(Capability::APIDesign);
        capabilities.insert(Capability::DatabaseDesign);

        Self {
            id: AgentId::new(),
            name,
            capabilities,
            metrics: AgentMetrics::new(),
            design_patterns: vec![
                "Factory".to_string(),
                "Builder".to_string(),
                "Singleton".to_string(),
                "Observer".to_string(),
                "Strategy".to_string(),
                "Repository".to_string(),
                "CQRS".to_string(),
                "Event Sourcing".to_string(),
            ],
            architectural_styles: vec![
                ArchitecturalStyle::Layered,
                ArchitecturalStyle::Microservices,
                ArchitecturalStyle::EventDriven,
                ArchitecturalStyle::Hexagonal,
            ],
            cortex: None,
            workspace_id: None,
        }
    }

    /// Create a new architect agent with Cortex integration
    pub fn with_cortex(name: String, cortex: Arc<CortexBridge>, workspace_id: WorkspaceId) -> Self {
        let mut agent = Self::new(name);
        agent.cortex = Some(cortex);
        agent.workspace_id = Some(workspace_id);
        agent
    }

    /// Create architect agent with custom patterns and styles
    pub fn with_patterns(
        name: String,
        patterns: Vec<String>,
        styles: Vec<ArchitecturalStyle>,
    ) -> Self {
        let mut agent = Self::new(name);
        agent.design_patterns = patterns;
        agent.architectural_styles = styles;
        agent
    }

    /// Design system architecture based on requirements
    pub fn design_system(&self, requirements: SystemRequirements) -> Result<Architecture> {
        // Select appropriate architectural style based on requirements
        let style = self.select_architectural_style(&requirements);

        // Generate components based on requirements
        let components = self.generate_components(&requirements, &style);

        // Identify applicable patterns
        let patterns_used = self.identify_patterns(&requirements);

        // Make design decisions
        let decisions = self.make_design_decisions(&requirements, &style);

        Ok(Architecture {
            summary: format!(
                "Designed {} architecture for {} system with {} scale",
                style_name(&style),
                requirements.system_type,
                scale_description(&requirements.scale)
            ),
            components,
            patterns_used,
            style,
            decisions,
        })
    }

    /// Analyze dependencies in the codebase
    ///
    /// For Cortex integration, use `analyze_dependencies_async`.
    ///
    /// # Arguments
    /// * `modules` - List of module names to analyze
    ///
    /// # Returns
    /// Dependency analysis with recommendations
    pub fn analyze_dependencies(&self, modules: Vec<String>) -> Result<DependencyAnalysis> {
        info!("Starting dependency analysis for {} modules (sync)", modules.len());

        let circular_dependencies = self.detect_circular_dependencies(&modules);
        let max_depth = self.calculate_dependency_depth(&modules);
        let recommendations = self.generate_dependency_recommendations(&circular_dependencies);

        Ok(DependencyAnalysis {
            total_dependencies: modules.len(),
            circular_dependencies,
            max_depth,
            recommendations,
        })
    }

    /// Analyze dependencies in the codebase with Cortex integration (async version)
    ///
    /// This method:
    /// 1. Detects circular dependencies using DFS
    /// 2. Calculates dependency depth
    /// 3. Searches for similar architecture patterns in Cortex (if available)
    /// 4. Stores analysis results in episodic memory
    ///
    /// # Arguments
    /// * `modules` - List of module names to analyze
    ///
    /// # Returns
    /// Dependency analysis with recommendations
    pub async fn analyze_dependencies_async(&self, modules: Vec<String>) -> Result<DependencyAnalysis> {
        info!("Starting dependency analysis for {} modules", modules.len());

        let circular_dependencies = self.detect_circular_dependencies(&modules);
        let max_depth = self.calculate_dependency_depth(&modules);
        let mut recommendations = self.generate_dependency_recommendations(&circular_dependencies);

        // If Cortex is available, enhance analysis with semantic search
        if let (Some(cortex), Some(workspace_id)) = (&self.cortex, &self.workspace_id) {
            debug!("Enhancing dependency analysis with Cortex semantic search");

            // Search for similar architecture patterns
            let search_query = format!(
                "dependency analysis architecture patterns circular dependencies depth {}",
                max_depth
            );

            match cortex
                .semantic_search(
                    &search_query,
                    workspace_id,
                    SearchFilters {
                        types: vec!["architecture".to_string(), "pattern".to_string()],
                        min_relevance: 0.7,
                        ..Default::default()
                    },
                )
                .await
            {
                Ok(results) => {
                    info!("Found {} similar architecture patterns", results.len());

                    // Add recommendations based on found patterns
                    for result in results.iter().take(3) {
                        recommendations.push(format!(
                            "Consider pattern: {} (relevance: {:.2})",
                            result.name, result.relevance_score
                        ));
                    }
                }
                Err(e) => {
                    warn!("Failed to search for architecture patterns: {}", e);
                }
            }

            // Get existing architecture patterns from Cortex
            match cortex.get_patterns().await {
                Ok(patterns) => {
                    let arch_patterns: Vec<_> = patterns
                        .iter()
                        .filter(|p| matches!(p.pattern_type, PatternType::Architecture))
                        .collect();

                    if !arch_patterns.is_empty() {
                        info!("Found {} architecture patterns in memory", arch_patterns.len());

                        for pattern in arch_patterns.iter().take(3) {
                            if pattern.success_rate > 0.7 {
                                recommendations.push(format!(
                                    "Apply learned pattern '{}' (success rate: {:.1}%)",
                                    pattern.name,
                                    pattern.success_rate * 100.0
                                ));
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to retrieve patterns from Cortex: {}", e);
                }
            }

            // Store this analysis as an episode for future learning
            let episode = Episode {
                id: uuid::Uuid::new_v4().to_string(),
                episode_type: EpisodeType::Exploration,
                task_description: format!("Dependency analysis for {} modules", modules.len()),
                agent_id: self.id.to_string(),
                session_id: None,
                workspace_id: workspace_id.to_string(),
                entities_created: vec![],
                entities_modified: modules.clone(),
                entities_deleted: vec![],
                files_touched: vec![],
                queries_made: vec![search_query],
                tools_used: vec![],
                solution_summary: format!(
                    "Analyzed {} modules, found {} circular dependencies, max depth: {}",
                    modules.len(),
                    circular_dependencies.len(),
                    max_depth
                ),
                outcome: if circular_dependencies.is_empty() {
                    EpisodeOutcome::Success
                } else {
                    EpisodeOutcome::Partial
                },
                success_metrics: serde_json::json!({
                    "total_modules": modules.len(),
                    "circular_deps": circular_dependencies.len(),
                    "max_depth": max_depth,
                }),
                errors_encountered: vec![],
                lessons_learned: if !circular_dependencies.is_empty() {
                    vec!["Circular dependencies detected - refactoring recommended".to_string()]
                } else {
                    vec!["Clean dependency graph - no circular dependencies".to_string()]
                },
                duration_seconds: 0,
                tokens_used: Default::default(),
                embedding: vec![],
                created_at: chrono::Utc::now(),
                completed_at: Some(chrono::Utc::now()),
            };

            if let Err(e) = cortex.store_episode(episode).await {
                warn!("Failed to store dependency analysis episode: {}", e);
            } else {
                debug!("Dependency analysis episode stored in Cortex memory");
            }
        }

        Ok(DependencyAnalysis {
            total_dependencies: modules.len(),
            circular_dependencies,
            max_depth,
            recommendations,
        })
    }

    /// Propose refactoring based on architecture analysis
    pub fn propose_refactoring(
        &self,
        target: String,
        analysis: &DependencyAnalysis,
    ) -> Result<Vec<RefactoringProposal>> {
        let mut proposals = Vec::new();

        // Propose refactoring for circular dependencies
        for circular in &analysis.circular_dependencies {
            if matches!(circular.severity, DependencySeverity::High | DependencySeverity::Critical) {
                proposals.push(RefactoringProposal {
                    target: target.clone(),
                    refactoring_type: RefactoringType::SimplifyDependencies,
                    description: format!("Break circular dependency: {:?}", circular.cycle),
                    benefits: vec![
                        "Improved maintainability".to_string(),
                        "Better testability".to_string(),
                        "Reduced coupling".to_string(),
                    ],
                    risks: vec![
                        "May require interface changes".to_string(),
                        "Potential breaking changes".to_string(),
                    ],
                    estimated_effort: EstimatedEffort::Medium,
                });
            }
        }

        // Propose component extraction if depth is too high
        if analysis.max_depth > 5 {
            proposals.push(RefactoringProposal {
                target: target.clone(),
                refactoring_type: RefactoringType::ExtractComponent,
                description: "Extract components to reduce dependency depth".to_string(),
                benefits: vec![
                    "Simplified dependency graph".to_string(),
                    "Better separation of concerns".to_string(),
                ],
                risks: vec![
                    "May increase number of components".to_string(),
                ],
                estimated_effort: EstimatedEffort::Large,
            });
        }

        Ok(proposals)
    }

    /// Get supported design patterns
    pub fn get_design_patterns(&self) -> &[String] {
        &self.design_patterns
    }

    /// Get supported architectural styles
    pub fn get_architectural_styles(&self) -> &[ArchitecturalStyle] {
        &self.architectural_styles
    }

    // Private helper methods

    fn select_architectural_style(&self, requirements: &SystemRequirements) -> ArchitecturalStyle {
        // Simple heuristic-based selection
        if requirements.scale.users > 1_000_000 {
            ArchitecturalStyle::Microservices
        } else if requirements.quality_attributes.contains(&QualityAttribute::Scalability) {
            ArchitecturalStyle::EventDriven
        } else if requirements.integrations.len() > 5 {
            ArchitecturalStyle::Hexagonal
        } else {
            ArchitecturalStyle::Layered
        }
    }

    fn generate_components(
        &self,
        _requirements: &SystemRequirements,
        style: &ArchitecturalStyle,
    ) -> Vec<Component> {
        // Generate basic components based on style
        match style {
            ArchitecturalStyle::Layered => vec![
                Component {
                    name: "Presentation Layer".to_string(),
                    description: "User interface and API endpoints".to_string(),
                    responsibilities: vec!["Handle user input".to_string(), "Display data".to_string()],
                    dependencies: vec!["Business Layer".to_string()],
                },
                Component {
                    name: "Business Layer".to_string(),
                    description: "Core business logic".to_string(),
                    responsibilities: vec!["Business rules".to_string(), "Workflows".to_string()],
                    dependencies: vec!["Data Layer".to_string()],
                },
                Component {
                    name: "Data Layer".to_string(),
                    description: "Data access and persistence".to_string(),
                    responsibilities: vec!["Database operations".to_string(), "Data mapping".to_string()],
                    dependencies: vec![],
                },
            ],
            _ => vec![],
        }
    }

    fn identify_patterns(&self, requirements: &SystemRequirements) -> Vec<String> {
        let mut patterns = Vec::new();

        if requirements.quality_attributes.contains(&QualityAttribute::Testability) {
            patterns.push("Repository".to_string());
            patterns.push("Dependency Injection".to_string());
        }

        if requirements.quality_attributes.contains(&QualityAttribute::Scalability) {
            patterns.push("CQRS".to_string());
            patterns.push("Event Sourcing".to_string());
        }

        patterns
    }

    fn make_design_decisions(
        &self,
        _requirements: &SystemRequirements,
        style: &ArchitecturalStyle,
    ) -> Vec<DesignDecision> {
        vec![
            DesignDecision {
                decision: format!("Use {} architecture", style_name(style)),
                rationale: "Best fits the scale and quality requirements".to_string(),
                alternatives_considered: vec!["Monolithic".to_string(), "SOA".to_string()],
                trade_offs: vec![
                    "Increased complexity for better scalability".to_string(),
                ],
            },
        ]
    }

    /// Detect circular dependencies using Depth-First Search (DFS)
    ///
    /// Algorithm:
    /// 1. Build adjacency list from module dependencies
    /// 2. Track node states: Unvisited, InProgress, Visited
    /// 3. Use DFS to detect back edges (cycles)
    /// 4. When a back edge is found, reconstruct the cycle path
    ///
    /// # Arguments
    /// * `modules` - List of module names
    ///
    /// # Returns
    /// Vector of detected circular dependencies with severity
    fn detect_circular_dependencies(&self, modules: &[String]) -> Vec<CircularDependency> {
        use std::collections::HashMap;

        tracing::debug!("Detecting circular dependencies in {} modules", modules.len());

        // Build a simple dependency graph (for demo, we assume modules depend on next one)
        // In real implementation, this would parse actual code dependencies
        let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();

        for (i, module) in modules.iter().enumerate() {
            let deps: Vec<&str> = modules
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i && *j < modules.len())
                .map(|(_, m)| m.as_str())
                .collect();
            graph.insert(module.as_str(), deps);
        }

        // DFS states
        #[derive(PartialEq)]
        enum NodeState {
            Unvisited,
            InProgress,
            Visited,
        }

        let mut states: HashMap<&str, NodeState> = HashMap::new();
        let mut cycles: Vec<CircularDependency> = Vec::new();
        let mut current_path: Vec<String> = Vec::new();

        // DFS function to detect cycles
        fn dfs<'a>(
            node: &'a str,
            graph: &HashMap<&'a str, Vec<&'a str>>,
            states: &mut HashMap<&'a str, NodeState>,
            current_path: &mut Vec<String>,
            cycles: &mut Vec<CircularDependency>,
        ) {
            states.insert(node, NodeState::InProgress);
            current_path.push(node.to_string());

            if let Some(neighbors) = graph.get(node) {
                for &neighbor in neighbors {
                    match states.get(neighbor).unwrap_or(&NodeState::Unvisited) {
                        NodeState::Unvisited => {
                            dfs(neighbor, graph, states, current_path, cycles);
                        }
                        NodeState::InProgress => {
                            // Back edge detected - we found a cycle!
                            if let Some(cycle_start) = current_path.iter().position(|n| n == neighbor) {
                                let cycle: Vec<String> = current_path[cycle_start..]
                                    .iter()
                                    .chain(std::iter::once(&neighbor.to_string()))
                                    .cloned()
                                    .collect();

                                // Determine severity based on cycle length
                                let severity = match cycle.len() {
                                    2..=3 => DependencySeverity::High,
                                    4..=5 => DependencySeverity::Medium,
                                    _ => DependencySeverity::Low,
                                };

                                tracing::warn!("Circular dependency detected: {:?}", cycle);

                                cycles.push(CircularDependency { cycle, severity });
                            }
                        }
                        NodeState::Visited => {
                            // Already processed, skip
                        }
                    }
                }
            }

            current_path.pop();
            states.insert(node, NodeState::Visited);
        }

        // Initialize all nodes as unvisited
        for module in modules.iter() {
            states.insert(module.as_str(), NodeState::Unvisited);
        }

        // Run DFS from each unvisited node
        for module in modules.iter() {
            if states.get(module.as_str()) == Some(&NodeState::Unvisited) {
                dfs(module.as_str(), &graph, &mut states, &mut current_path, &mut cycles);
            }
        }

        tracing::info!("Found {} circular dependencies", cycles.len());
        cycles
    }

    /// Calculate maximum dependency depth using topological sort and DFS
    ///
    /// Algorithm:
    /// 1. Build dependency graph
    /// 2. Use DFS to compute the longest path from root nodes
    /// 3. Track maximum depth encountered
    ///
    /// The depth represents the maximum chain length in the dependency graph.
    /// Higher depth indicates more complex dependency chains.
    ///
    /// # Arguments
    /// * `modules` - List of module names
    ///
    /// # Returns
    /// Maximum depth of the dependency graph
    fn calculate_dependency_depth(&self, modules: &[String]) -> usize {
        use std::collections::{HashMap, HashSet};

        if modules.is_empty() {
            return 0;
        }

        tracing::debug!("Calculating dependency depth for {} modules", modules.len());

        // Build dependency graph
        // In real implementation, this would parse actual dependencies
        let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut all_nodes: HashSet<&str> = HashSet::new();

        for (i, module) in modules.iter().enumerate() {
            all_nodes.insert(module.as_str());
            // Simplified: each module depends on previous modules
            let deps: Vec<&str> = modules[..i]
                .iter()
                .map(|m| m.as_str())
                .collect();
            graph.insert(module.as_str(), deps);
        }

        // Find root nodes (nodes with no incoming edges)
        let mut has_incoming: HashSet<&str> = HashSet::new();
        for deps in graph.values() {
            for &dep in deps {
                has_incoming.insert(dep);
            }
        }

        let root_nodes: Vec<&str> = all_nodes
            .iter()
            .filter(|&&node| !has_incoming.contains(node))
            .copied()
            .collect();

        if root_nodes.is_empty() {
            // If no root nodes, there might be cycles - return module count as approximation
            tracing::warn!("No root nodes found - possible circular dependencies");
            return modules.len();
        }

        // Calculate depth using DFS
        let mut memo: HashMap<&str, usize> = HashMap::new();

        fn dfs_depth<'a>(
            node: &'a str,
            graph: &HashMap<&'a str, Vec<&'a str>>,
            memo: &mut HashMap<&'a str, usize>,
        ) -> usize {
            // Check memoization
            if let Some(&depth) = memo.get(node) {
                return depth;
            }

            let depth = if let Some(deps) = graph.get(node) {
                if deps.is_empty() {
                    1 // Leaf node
                } else {
                    // Max depth of dependencies + 1
                    1 + deps
                        .iter()
                        .map(|&dep| dfs_depth(dep, graph, memo))
                        .max()
                        .unwrap_or(0)
                }
            } else {
                1 // No dependencies
            };

            memo.insert(node, depth);
            depth
        }

        // Calculate depth from all nodes
        let max_depth = all_nodes
            .iter()
            .map(|&node| dfs_depth(node, &graph, &mut memo))
            .max()
            .unwrap_or(1);

        tracing::info!("Maximum dependency depth: {}", max_depth);
        max_depth
    }

    fn generate_dependency_recommendations(
        &self,
        circular_deps: &[CircularDependency],
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        if !circular_deps.is_empty() {
            recommendations.push("Break circular dependencies using interfaces".to_string());
            recommendations.push("Apply Dependency Inversion Principle".to_string());
        }

        recommendations.push("Review and simplify module boundaries".to_string());
        recommendations
    }
}

impl Agent for ArchitectAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Architect
    }

    fn capabilities(&self) -> &HashSet<Capability> {
        &self.capabilities
    }

    fn metrics(&self) -> &AgentMetrics {
        &self.metrics
    }
}

// Helper functions

fn style_name(style: &ArchitecturalStyle) -> &'static str {
    match style {
        ArchitecturalStyle::Layered => "Layered",
        ArchitecturalStyle::Microservices => "Microservices",
        ArchitecturalStyle::EventDriven => "Event-Driven",
        ArchitecturalStyle::Hexagonal => "Hexagonal",
        ArchitecturalStyle::ServiceOriented => "Service-Oriented",
        ArchitecturalStyle::ComponentBased => "Component-Based",
        ArchitecturalStyle::Monolithic => "Monolithic",
        ArchitecturalStyle::Serverless => "Serverless",
    }
}

fn scale_description(scale: &ScaleRequirements) -> String {
    format!(
        "{} users, {} req/s, {} GB data",
        scale.users, scale.requests_per_second, scale.data_volume_gb
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_architect_agent_creation() {
        let agent = ArchitectAgent::new("TestArchitect".to_string());
        assert_eq!(agent.name(), "TestArchitect");
        assert_eq!(agent.agent_type(), AgentType::Architect);
        assert!(agent.capabilities().contains(&Capability::SystemDesign));
        assert!(agent.capabilities().contains(&Capability::ArchitectureAnalysis));
    }

    #[test]
    fn test_design_patterns() {
        let agent = ArchitectAgent::new("TestArchitect".to_string());
        let patterns = agent.get_design_patterns();
        assert!(!patterns.is_empty());
        assert!(patterns.contains(&"Factory".to_string()));
        assert!(patterns.contains(&"Builder".to_string()));
    }

    #[test]
    fn test_system_design() {
        let agent = ArchitectAgent::new("TestArchitect".to_string());
        let requirements = SystemRequirements {
            system_type: "E-Commerce Platform".to_string(),
            scale: ScaleRequirements {
                users: 100_000,
                requests_per_second: 1000,
                data_volume_gb: 500,
            },
            quality_attributes: vec![
                QualityAttribute::Performance,
                QualityAttribute::Scalability,
            ],
            constraints: vec![],
            integrations: vec![],
        };

        let result = agent.design_system(requirements);
        assert!(result.is_ok());

        let architecture = result.unwrap();
        assert!(!architecture.summary.is_empty());
        assert!(!architecture.components.is_empty());
    }

    #[test]
    fn test_dependency_analysis() {
        let agent = ArchitectAgent::new("TestArchitect".to_string());
        let modules = vec![
            "module_a".to_string(),
            "module_b".to_string(),
            "module_c".to_string(),
        ];

        let result = agent.analyze_dependencies(modules);
        assert!(result.is_ok());

        let analysis = result.unwrap();
        assert_eq!(analysis.total_dependencies, 3);
    }

    #[test]
    fn test_refactoring_proposals() {
        let agent = ArchitectAgent::new("TestArchitect".to_string());
        let analysis = DependencyAnalysis {
            total_dependencies: 10,
            circular_dependencies: vec![
                CircularDependency {
                    cycle: vec!["A".to_string(), "B".to_string(), "A".to_string()],
                    severity: DependencySeverity::High,
                },
            ],
            max_depth: 8,
            recommendations: vec![],
        };

        let result = agent.propose_refactoring("test_module".to_string(), &analysis);
        assert!(result.is_ok());

        let proposals = result.unwrap();
        assert!(!proposals.is_empty());
    }

    #[test]
    fn test_custom_patterns() {
        let custom_patterns = vec!["MVVM".to_string(), "Clean Architecture".to_string()];
        let custom_styles = vec![ArchitecturalStyle::Hexagonal];

        let agent = ArchitectAgent::with_patterns(
            "CustomArchitect".to_string(),
            custom_patterns.clone(),
            custom_styles,
        );

        assert_eq!(agent.get_design_patterns(), &custom_patterns);
    }
}
