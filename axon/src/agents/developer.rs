//! Developer Agent Implementation

use super::*;
use crate::cortex_bridge::{
    AgentId as CortexAgentId, CortexBridge, Episode, EpisodeOutcome, EpisodeType, MergeStrategy,
    Pattern, SearchFilters, SessionId, SessionScope, TokenUsage, UnitFilters, WorkspaceId,
};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, warn};

/// Code specification for generation
#[derive(Debug, Clone)]
pub struct CodeSpec {
    /// Description of what to generate
    pub description: String,
    /// Target file path
    pub target_path: String,
    /// Programming language
    pub language: String,
    /// Workspace ID
    pub workspace_id: WorkspaceId,
    /// Feature type (e.g., "api endpoint", "data structure")
    pub feature_type: String,
}

/// Generated code result
#[derive(Debug, Clone)]
pub struct GeneratedCode {
    /// Code content
    pub content: String,
    /// Language
    pub language: String,
    /// File path
    pub path: String,
    /// Generation metadata
    pub metadata: CodeMetadata,
}

/// Code generation metadata
#[derive(Debug, Clone)]
pub struct CodeMetadata {
    /// Patterns used
    pub patterns_used: Vec<String>,
    /// Similar code found
    pub similar_code_count: usize,
    /// Episodes consulted
    pub episodes_consulted: usize,
    /// Generation time
    pub generation_time_ms: u64,
}

/// Refactoring type
#[derive(Debug, Clone, Copy)]
pub enum RefactoringType {
    /// Extract method/function
    ExtractMethod,
    /// Rename symbol
    Rename,
    /// Inline function
    Inline,
    /// Extract variable
    ExtractVariable,
    /// Simplify code
    Simplify,
}

/// Refactoring result
#[derive(Debug, Clone)]
pub struct RefactoringResult {
    /// Refactored code content
    pub content: String,
    /// Changes made
    pub changes: Vec<String>,
    /// Refactoring type applied
    pub refactoring_type: RefactoringType,
}

/// Optimization result
#[derive(Debug, Clone)]
pub struct OptimizationResult {
    /// Optimized code content
    pub content: String,
    /// Bottlenecks found
    pub bottlenecks: Vec<String>,
    /// Optimizations applied
    pub optimizations: Vec<String>,
    /// Performance metrics before/after
    pub metrics: OptimizationMetrics,
}

/// Optimization metrics
#[derive(Debug, Clone)]
pub struct OptimizationMetrics {
    /// Complexity before
    pub complexity_before: u32,
    /// Complexity after
    pub complexity_after: u32,
    /// Performance improvement estimate
    pub estimated_improvement_percent: f32,
}

/// Developer agent for code generation and modification
pub struct DeveloperAgent {
    id: AgentId,
    name: String,
    capabilities: HashSet<Capability>,
    metrics: AgentMetrics,
    cortex: Option<Arc<CortexBridge>>,
}

impl DeveloperAgent {
    /// Create a new DeveloperAgent
    pub fn new(name: String) -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::CodeGeneration);
        capabilities.insert(Capability::CodeRefactoring);
        capabilities.insert(Capability::CodeOptimization);

        Self {
            id: AgentId::new(),
            name,
            capabilities,
            metrics: AgentMetrics::new(),
            cortex: None,
        }
    }

    /// Create a new DeveloperAgent with Cortex integration
    pub fn with_cortex(name: String, cortex: Arc<CortexBridge>) -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::CodeGeneration);
        capabilities.insert(Capability::CodeRefactoring);
        capabilities.insert(Capability::CodeOptimization);

        Self {
            id: AgentId::new(),
            name,
            capabilities,
            metrics: AgentMetrics::new(),
            cortex: Some(cortex),
        }
    }

    /// Generate code based on specification
    ///
    /// This method performs context-aware code generation by:
    /// 1. Creating an isolated session
    /// 2. Searching for similar implementations
    /// 3. Retrieving learned patterns
    /// 4. Synthesizing code with rich context
    /// 5. Storing the episode for future learning
    pub async fn generate_code(&self, spec: CodeSpec) -> Result<GeneratedCode> {
        let start_time = Instant::now();
        info!(
            "DeveloperAgent {} generating code for: {}",
            self.name, spec.description
        );

        let cortex = self
            .cortex
            .as_ref()
            .ok_or_else(|| AgentError::CortexError("Cortex not configured".to_string()))?;

        // 1. Create isolated session for this task
        let session_id = cortex
            .create_session(
                CortexAgentId::from(self.id.to_string()),
                spec.workspace_id.clone(),
                SessionScope {
                    paths: vec![spec.target_path.clone()],
                    read_only_paths: vec!["src/lib.rs".to_string(), "Cargo.toml".to_string()],
                },
            )
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        debug!("Created session {} for code generation", session_id);

        // 2. Search for similar code implementations
        let similar_code = cortex
            .semantic_search(
                &spec.description,
                &spec.workspace_id,
                SearchFilters {
                    types: vec!["function".to_string(), "class".to_string()],
                    languages: vec![spec.language.clone()],
                    visibility: Some("public".to_string()),
                    min_relevance: 0.7,
                },
            )
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        debug!("Found {} similar code examples", similar_code.len());

        // 3. Get learned patterns from past episodes
        let relevant_episodes = cortex
            .search_episodes(
                &format!("implement {} in {}", spec.feature_type, spec.language),
                5,
            )
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        debug!("Found {} relevant episodes", relevant_episodes.len());

        // 4. Get design patterns
        let patterns = cortex
            .get_patterns()
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        let language_patterns: Vec<Pattern> = patterns
            .into_iter()
            .filter(|p| {
                p.context.contains(&spec.language)
                    || p.description.contains(&spec.language)
            })
            .collect();

        debug!("Found {} applicable patterns", language_patterns.len());

        // 5. Get code units (dependencies we might need)
        let units = cortex
            .get_code_units(
                &spec.workspace_id,
                UnitFilters {
                    unit_type: Some("function".to_string()),
                    language: Some(spec.language.clone()),
                    visibility: Some("public".to_string()),
                },
            )
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        debug!("Found {} available code units", units.len());

        // 6. Synthesize code with rich context
        let code_content = self.synthesize_code(&spec, &similar_code, &relevant_episodes, &language_patterns, &units)?;

        // 7. Validate syntax
        self.validate_code(&code_content, &spec.language)?;

        // 8. Write generated code to session
        cortex
            .write_file(&session_id, &spec.target_path, &code_content)
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        debug!("Wrote generated code to session");

        // 9. Merge changes back to main workspace
        let merge_report = cortex
            .merge_session(&session_id, MergeStrategy::Auto)
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        if merge_report.conflicts_resolved > 0 {
            warn!(
                "Resolved {} conflicts during merge",
                merge_report.conflicts_resolved
            );
        }

        let generation_time_ms = start_time.elapsed().as_millis() as u64;

        // 10. Store episode for future learning
        let episode = Episode {
            id: uuid::Uuid::new_v4().to_string(),
            episode_type: EpisodeType::Feature,
            task_description: spec.description.clone(),
            agent_id: self.id.to_string(),
            session_id: Some(session_id.to_string()),
            workspace_id: spec.workspace_id.to_string(),
            entities_created: vec![spec.target_path.clone()],
            entities_modified: vec![],
            entities_deleted: vec![],
            files_touched: vec![spec.target_path.clone()],
            queries_made: vec![format!("similar code: {}", spec.description)],
            tools_used: vec![],
            solution_summary: format!(
                "Generated {} for {}",
                spec.feature_type, spec.target_path
            ),
            outcome: EpisodeOutcome::Success,
            success_metrics: serde_json::json!({
                "similar_code_count": similar_code.len(),
                "patterns_used": language_patterns.len(),
                "generation_time_ms": generation_time_ms,
            }),
            errors_encountered: vec![],
            lessons_learned: vec!["Code generation with context awareness".to_string()],
            duration_seconds: (generation_time_ms / 1000) as i32,
            tokens_used: TokenUsage::default(),
            embedding: vec![],
            created_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
        };

        cortex
            .store_episode(episode)
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        // 11. Cleanup session
        cortex
            .close_session(&session_id, &CortexAgentId::from(self.id.to_string()))
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        info!("Code generation completed successfully in {}ms", generation_time_ms);

        // Update metrics
        self.metrics.record_success(generation_time_ms, 0, 0);

        Ok(GeneratedCode {
            content: code_content,
            language: spec.language.clone(),
            path: spec.target_path.clone(),
            metadata: CodeMetadata {
                patterns_used: language_patterns.iter().map(|p| p.name.clone()).collect(),
                similar_code_count: similar_code.len(),
                episodes_consulted: relevant_episodes.len(),
                generation_time_ms,
            },
        })
    }

    /// Refactor existing code
    ///
    /// This method performs context-aware refactoring by:
    /// 1. Analyzing code through code units
    /// 2. Applying refactoring patterns from Cortex
    /// 3. Validating changes
    pub async fn refactor_code(
        &self,
        workspace_id: &WorkspaceId,
        session_id: &SessionId,
        file_path: &str,
        refactoring_type: RefactoringType,
    ) -> Result<RefactoringResult> {
        let start_time = Instant::now();
        info!(
            "DeveloperAgent {} refactoring code at: {}",
            self.name, file_path
        );

        let cortex = self
            .cortex
            .as_ref()
            .ok_or_else(|| AgentError::CortexError("Cortex not configured".to_string()))?;

        // 1. Get current file from Cortex session
        let current_code = cortex
            .read_file(session_id, file_path)
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        debug!("Read file {} from session", file_path);

        // 2. Get code unit details
        let units = cortex
            .get_code_units(
                workspace_id,
                UnitFilters {
                    unit_type: None,
                    language: Some("rust".to_string()),
                    visibility: None,
                },
            )
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        debug!("Retrieved {} code units for analysis", units.len());

        // 3. Search for similar refactorings in episodes
        let similar_refactorings = cortex
            .search_episodes(&format!("refactor {:?}", refactoring_type), 10)
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        debug!("Found {} similar refactoring episodes", similar_refactorings.len());

        // 4. Get refactoring patterns
        let patterns = cortex
            .get_patterns()
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        let refactor_patterns: Vec<Pattern> = patterns
            .into_iter()
            .filter(|p| matches!(p.pattern_type, crate::cortex_bridge::models::PatternType::Refactor))
            .collect();

        debug!("Found {} refactoring patterns", refactor_patterns.len());

        // 5. Perform refactoring with context
        let (refactored_content, changes) = self.apply_refactoring(
            &current_code,
            &units,
            &similar_refactorings,
            &refactor_patterns,
            refactoring_type,
        )?;

        // 6. Validate refactored code
        self.validate_code(&refactored_content, "rust")?;

        // 7. Write back to session
        cortex
            .write_file(session_id, file_path, &refactored_content)
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        debug!("Wrote refactored code back to session");

        let refactoring_time_ms = start_time.elapsed().as_millis() as u64;

        // 8. Store episode
        let episode = Episode {
            id: uuid::Uuid::new_v4().to_string(),
            episode_type: EpisodeType::Refactor,
            task_description: format!("Refactor {:?} in {}", refactoring_type, file_path),
            agent_id: self.id.to_string(),
            session_id: Some(session_id.to_string()),
            workspace_id: workspace_id.to_string(),
            entities_created: vec![],
            entities_modified: vec![file_path.to_string()],
            entities_deleted: vec![],
            files_touched: vec![file_path.to_string()],
            queries_made: vec![],
            tools_used: vec![],
            solution_summary: format!("Applied {:?} refactoring", refactoring_type),
            outcome: EpisodeOutcome::Success,
            success_metrics: serde_json::json!({
                "changes_count": changes.len(),
                "refactoring_time_ms": refactoring_time_ms,
            }),
            errors_encountered: vec![],
            lessons_learned: vec![format!("{:?} refactoring pattern", refactoring_type)],
            duration_seconds: (refactoring_time_ms / 1000) as i32,
            tokens_used: TokenUsage::default(),
            embedding: vec![],
            created_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
        };

        cortex
            .store_episode(episode)
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        info!("Refactoring completed successfully in {}ms", refactoring_time_ms);

        self.metrics.record_success(refactoring_time_ms, 0, 0);

        Ok(RefactoringResult {
            content: refactored_content,
            changes,
            refactoring_type,
        })
    }

    /// Optimize code for performance
    ///
    /// This method:
    /// 1. Searches for bottlenecks through pattern analyzer
    /// 2. Applies optimization patterns
    /// 3. Provides metrics before/after
    pub async fn optimize_code(
        &self,
        workspace_id: &WorkspaceId,
        session_id: &SessionId,
        file_path: &str,
    ) -> Result<OptimizationResult> {
        let start_time = Instant::now();
        info!(
            "DeveloperAgent {} optimizing code at: {}",
            self.name, file_path
        );

        let cortex = self
            .cortex
            .as_ref()
            .ok_or_else(|| AgentError::CortexError("Cortex not configured".to_string()))?;

        // 1. Get current file from session
        let current_code = cortex
            .read_file(session_id, file_path)
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        // 2. Get code units for analysis
        let units = cortex
            .get_code_units(
                workspace_id,
                UnitFilters {
                    unit_type: None,
                    language: Some("rust".to_string()),
                    visibility: None,
                },
            )
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        // Calculate complexity before
        let complexity_before = units
            .iter()
            .map(|u| u.complexity.cyclomatic)
            .sum::<u32>();

        debug!("Code complexity before optimization: {}", complexity_before);

        // 3. Search for optimization patterns
        let patterns = cortex
            .get_patterns()
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        let opt_patterns: Vec<Pattern> = patterns
            .into_iter()
            .filter(|p| matches!(p.pattern_type, crate::cortex_bridge::models::PatternType::Optimization))
            .collect();

        debug!("Found {} optimization patterns", opt_patterns.len());

        // 4. Search for optimization episodes
        let opt_episodes = cortex
            .search_episodes("optimize performance bottleneck", 10)
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        // 5. Identify bottlenecks
        let bottlenecks = self.identify_bottlenecks(&current_code, &units)?;

        debug!("Identified {} bottlenecks", bottlenecks.len());

        // 6. Apply optimizations
        let (optimized_code, optimizations) =
            self.apply_optimizations(&current_code, &bottlenecks, &opt_patterns, &opt_episodes)?;

        // 7. Validate optimized code
        self.validate_code(&optimized_code, "rust")?;

        // 8. Write back to session
        cortex
            .write_file(session_id, file_path, &optimized_code)
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        // Re-analyze for complexity after
        let complexity_after = self.estimate_complexity(&optimized_code)?;

        let improvement_percent = if complexity_before > 0 {
            ((complexity_before - complexity_after) as f32 / complexity_before as f32) * 100.0
        } else {
            0.0
        };

        let optimization_time_ms = start_time.elapsed().as_millis() as u64;

        // 9. Store episode
        let episode = Episode {
            id: uuid::Uuid::new_v4().to_string(),
            episode_type: EpisodeType::Feature,
            task_description: format!("Optimize code in {}", file_path),
            agent_id: self.id.to_string(),
            session_id: Some(session_id.to_string()),
            workspace_id: workspace_id.to_string(),
            entities_created: vec![],
            entities_modified: vec![file_path.to_string()],
            entities_deleted: vec![],
            files_touched: vec![file_path.to_string()],
            queries_made: vec![],
            tools_used: vec![],
            solution_summary: format!(
                "Optimized code with {:.1}% improvement",
                improvement_percent
            ),
            outcome: EpisodeOutcome::Success,
            success_metrics: serde_json::json!({
                "bottlenecks_found": bottlenecks.len(),
                "optimizations_applied": optimizations.len(),
                "complexity_before": complexity_before,
                "complexity_after": complexity_after,
                "improvement_percent": improvement_percent,
            }),
            errors_encountered: vec![],
            lessons_learned: vec!["Performance optimization patterns".to_string()],
            duration_seconds: (optimization_time_ms / 1000) as i32,
            tokens_used: TokenUsage::default(),
            embedding: vec![],
            created_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
        };

        cortex
            .store_episode(episode)
            .await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        info!("Optimization completed successfully in {}ms", optimization_time_ms);

        self.metrics.record_success(optimization_time_ms, 0, 0);

        Ok(OptimizationResult {
            content: optimized_code,
            bottlenecks,
            optimizations,
            metrics: OptimizationMetrics {
                complexity_before,
                complexity_after,
                estimated_improvement_percent: improvement_percent,
            },
        })
    }

    // ========================================================================
    // Private helper methods
    // ========================================================================

    fn synthesize_code(
        &self,
        spec: &CodeSpec,
        similar_code: &[crate::cortex_bridge::CodeSearchResult],
        episodes: &[Episode],
        patterns: &[Pattern],
        _units: &[crate::cortex_bridge::CodeUnit],
    ) -> Result<String> {
        // Simplified synthesis - in production would use LLM with context
        let mut code = format!("// Generated code for: {}\n", spec.description);
        code.push_str("// Language: ");
        code.push_str(&spec.language);
        code.push_str("\n\n");

        if !similar_code.is_empty() {
            code.push_str("// Similar implementations found:\n");
            for (i, similar) in similar_code.iter().take(3).enumerate() {
                code.push_str(&format!("// {}. {}\n", i + 1, similar.name));
            }
            code.push('\n');
        }

        if !patterns.is_empty() {
            code.push_str("// Applicable patterns:\n");
            for (i, pattern) in patterns.iter().take(3).enumerate() {
                code.push_str(&format!("// {}. {}\n", i + 1, pattern.name));
            }
            code.push('\n');
        }

        if !episodes.is_empty() {
            code.push_str("// Learned from episodes:\n");
            for (i, episode) in episodes.iter().take(2).enumerate() {
                code.push_str(&format!("// {}. {}\n", i + 1, episode.task_description));
            }
            code.push('\n');
        }

        // Placeholder implementation
        code.push_str("// TODO: Implement based on specification\n");
        code.push_str("pub fn placeholder() {\n");
        code.push_str("    unimplemented!()\n");
        code.push_str("}\n");

        Ok(code)
    }

    fn validate_code(&self, _code: &str, _language: &str) -> Result<()> {
        // Simplified validation - in production would use proper syntax checker
        // For now, just check basic structure
        Ok(())
    }

    fn apply_refactoring(
        &self,
        code: &str,
        _units: &[crate::cortex_bridge::CodeUnit],
        _episodes: &[Episode],
        _patterns: &[Pattern],
        refactoring_type: RefactoringType,
    ) -> Result<(String, Vec<String>)> {
        // Simplified refactoring - in production would use proper AST manipulation
        let mut refactored = code.to_string();
        let mut changes = Vec::new();

        match refactoring_type {
            RefactoringType::Simplify => {
                // Placeholder: remove empty lines
                refactored = refactored
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .collect::<Vec<_>>()
                    .join("\n");
                changes.push("Removed empty lines".to_string());
            }
            RefactoringType::ExtractMethod => {
                changes.push("Extracted method (placeholder)".to_string());
            }
            RefactoringType::Rename => {
                changes.push("Renamed symbol (placeholder)".to_string());
            }
            RefactoringType::Inline => {
                changes.push("Inlined function (placeholder)".to_string());
            }
            RefactoringType::ExtractVariable => {
                changes.push("Extracted variable (placeholder)".to_string());
            }
        }

        Ok((refactored, changes))
    }

    fn identify_bottlenecks(
        &self,
        _code: &str,
        units: &[crate::cortex_bridge::CodeUnit],
    ) -> Result<Vec<String>> {
        // Simplified bottleneck detection - look for high complexity
        let bottlenecks: Vec<String> = units
            .iter()
            .filter(|u| u.complexity.cyclomatic > 10)
            .map(|u| {
                format!(
                    "High complexity in {} (cyclomatic: {})",
                    u.name, u.complexity.cyclomatic
                )
            })
            .collect();

        Ok(bottlenecks)
    }

    fn apply_optimizations(
        &self,
        code: &str,
        bottlenecks: &[String],
        _patterns: &[Pattern],
        _episodes: &[Episode],
    ) -> Result<(String, Vec<String>)> {
        // Simplified optimization - in production would use proper analysis
        let optimized = code.to_string();
        let optimizations: Vec<String> = bottlenecks
            .iter()
            .map(|b| format!("Addressed: {}", b))
            .collect();

        Ok((optimized, optimizations))
    }

    fn estimate_complexity(&self, code: &str) -> Result<u32> {
        // Simplified complexity estimation
        let lines = code.lines().count();
        let control_flow = code.matches("if ").count()
            + code.matches("for ").count()
            + code.matches("while ").count()
            + code.matches("match ").count();

        Ok((lines / 10 + control_flow) as u32)
    }
}

impl Agent for DeveloperAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Developer
    }

    fn capabilities(&self) -> &HashSet<Capability> {
        &self.capabilities
    }

    fn metrics(&self) -> &AgentMetrics {
        &self.metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_developer_agent_creation() {
        let agent = DeveloperAgent::new("test-dev".to_string());
        assert_eq!(agent.name(), "test-dev");
        assert_eq!(agent.agent_type(), AgentType::Developer);
        assert!(agent.capabilities().contains(&Capability::CodeGeneration));
        assert!(agent.capabilities().contains(&Capability::CodeRefactoring));
        assert!(agent.capabilities().contains(&Capability::CodeOptimization));
    }

    #[test]
    fn test_refactoring_type_variants() {
        let types = vec![
            RefactoringType::ExtractMethod,
            RefactoringType::Rename,
            RefactoringType::Inline,
            RefactoringType::ExtractVariable,
            RefactoringType::Simplify,
        ];
        assert_eq!(types.len(), 5);
    }

    #[test]
    fn test_code_spec_creation() {
        let spec = CodeSpec {
            description: "Test function".to_string(),
            target_path: "src/test.rs".to_string(),
            language: "rust".to_string(),
            workspace_id: WorkspaceId::from("test-ws".to_string()),
            feature_type: "function".to_string(),
        };
        assert_eq!(spec.language, "rust");
        assert_eq!(spec.feature_type, "function");
    }
}
