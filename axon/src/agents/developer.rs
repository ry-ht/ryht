//! Developer Agent Implementation

use super::*;
use crate::cortex_bridge::{
    AgentId as CortexAgentId, CortexBridge, Episode, EpisodeOutcome, EpisodeType, MergeStrategy,
    Pattern, SearchFilters, SessionId, SessionScope, TokenUsage, UnitFilters, WorkspaceId,
};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, warn};

// For Claude CLI integration
use crate::cc::{query, ClaudeCodeOptions, Message};
use crate::cc::messages::ContentBlock;
use futures::StreamExt;

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
        // Build rich context for Claude
        let mut prompt = format!(
            "Generate {} code for: {}\n\n",
            spec.language, spec.description
        );

        prompt.push_str(&format!("Target file: {}\n", spec.target_path));
        prompt.push_str(&format!("Feature type: {}\n\n", spec.feature_type));

        // Add similar code context
        if !similar_code.is_empty() {
            prompt.push_str("Similar implementations found:\n");
            for (i, similar) in similar_code.iter().take(3).enumerate() {
                prompt.push_str(&format!("{}. {} (relevance: {:.2})\n",
                    i + 1, similar.name, similar.relevance_score));
                if !similar.snippet.is_empty() {
                    prompt.push_str(&format!("   {}\n", similar.snippet));
                }
            }
            prompt.push('\n');
        }

        // Add patterns context
        if !patterns.is_empty() {
            prompt.push_str("Applicable design patterns:\n");
            for (i, pattern) in patterns.iter().take(3).enumerate() {
                prompt.push_str(&format!("{}. {}: {}\n",
                    i + 1, pattern.name, pattern.description));
            }
            prompt.push('\n');
        }

        // Add episodes context
        if !episodes.is_empty() {
            prompt.push_str("Learned from past implementations:\n");
            for (i, episode) in episodes.iter().take(2).enumerate() {
                prompt.push_str(&format!("{}. {}\n", i + 1, episode.task_description));
                if !episode.lessons_learned.is_empty() {
                    prompt.push_str(&format!("   Lesson: {}\n", episode.lessons_learned[0]));
                }
            }
            prompt.push('\n');
        }

        prompt.push_str(&format!(
            "Generate complete, production-ready {} code. Include proper error handling, \
            documentation comments, and follow best practices. Return ONLY the code, \
            wrapped in a code block with the language specified.\n",
            spec.language
        ));

        // Use Claude CLI to generate code
        debug!("Calling Claude for code synthesis");
        let code = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.query_claude(&prompt).await
            })
        })?;

        // Extract code from response
        let extracted_code = self.extract_code_blocks(&code, &spec.language)?;

        if extracted_code.is_empty() {
            warn!("No code blocks found in Claude response, using raw response");
            Ok(code)
        } else {
            Ok(extracted_code)
        }
    }

    fn validate_code(&self, code: &str, language: &str) -> Result<()> {
        debug!("Validating {} code", language);

        // Basic structure validation
        if code.trim().is_empty() {
            return Err(AgentError::ValidationError("Code is empty".to_string()));
        }

        // Check balanced braces and parentheses
        self.check_balanced_delimiters(code)?;

        // Language-specific validation
        match language.to_lowercase().as_str() {
            "rust" => self.validate_rust_syntax(code),
            "python" | "py" => self.validate_python_syntax(code),
            "javascript" | "js" | "typescript" | "ts" => self.validate_js_syntax(code),
            _ => {
                debug!("No specific validation for language: {}", language);
                Ok(())
            }
        }
    }

    fn check_balanced_delimiters(&self, code: &str) -> Result<()> {
        let mut braces = 0;
        let mut parens = 0;
        let mut brackets = 0;
        let mut in_string = false;
        let mut escape_next = false;
        let mut string_char = ' ';

        for ch in code.chars() {
            if escape_next {
                escape_next = false;
                continue;
            }

            if ch == '\\' {
                escape_next = true;
                continue;
            }

            if ch == '"' || ch == '\'' {
                if !in_string {
                    in_string = true;
                    string_char = ch;
                } else if ch == string_char {
                    in_string = false;
                }
                continue;
            }

            if in_string {
                continue;
            }

            match ch {
                '{' => braces += 1,
                '}' => braces -= 1,
                '(' => parens += 1,
                ')' => parens -= 1,
                '[' => brackets += 1,
                ']' => brackets -= 1,
                _ => {}
            }

            if braces < 0 || parens < 0 || brackets < 0 {
                return Err(AgentError::ValidationError(
                    format!("Unbalanced delimiters: braces={}, parens={}, brackets={}", braces, parens, brackets)
                ));
            }
        }

        if braces != 0 || parens != 0 || brackets != 0 {
            return Err(AgentError::ValidationError(
                format!("Unbalanced delimiters: braces={}, parens={}, brackets={}", braces, parens, brackets)
            ));
        }

        Ok(())
    }

    fn validate_rust_syntax(&self, code: &str) -> Result<()> {
        use std::io::Write;
        use std::process::{Command, Stdio};

        debug!("Validating Rust syntax with rustc");

        // Create a temporary file
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("rust_validate_{}.rs", uuid::Uuid::new_v4()));

        // Write code to temp file
        std::fs::write(&temp_file, code)
            .map_err(|e| AgentError::ValidationError(format!("Failed to write temp file: {}", e)))?;

        // Run rustc --crate-type lib to check syntax
        let output = Command::new("rustc")
            .arg("--crate-type")
            .arg("lib")
            .arg("--error-format=short")
            .arg(&temp_file)
            .arg("-o")
            .arg("/dev/null")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_file);

        match output {
            Ok(output) => {
                if output.status.success() {
                    debug!("Rust syntax validation passed");
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(AgentError::ValidationError(
                        format!("Rust syntax errors:\n{}", stderr)
                    ))
                }
            }
            Err(e) => {
                warn!("rustc not available: {}, skipping Rust validation", e);
                Ok(())
            }
        }
    }

    fn validate_python_syntax(&self, code: &str) -> Result<()> {
        use std::process::{Command, Stdio};

        debug!("Validating Python syntax");

        // Create a temporary file
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("py_validate_{}.py", uuid::Uuid::new_v4()));

        // Write code to temp file
        std::fs::write(&temp_file, code)
            .map_err(|e| AgentError::ValidationError(format!("Failed to write temp file: {}", e)))?;

        // Run python -m py_compile to check syntax
        let output = Command::new("python3")
            .arg("-m")
            .arg("py_compile")
            .arg(&temp_file)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        // Clean up temp file and compiled cache
        let _ = std::fs::remove_file(&temp_file);

        match output {
            Ok(output) => {
                if output.status.success() {
                    debug!("Python syntax validation passed");
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(AgentError::ValidationError(
                        format!("Python syntax errors:\n{}", stderr)
                    ))
                }
            }
            Err(e) => {
                warn!("python3 not available: {}, skipping Python validation", e);
                Ok(())
            }
        }
    }

    fn validate_js_syntax(&self, code: &str) -> Result<()> {
        debug!("Validating JavaScript/TypeScript syntax (basic check)");

        // Basic JS/TS syntax checks
        // Check for common syntax errors
        if code.contains("function") && !code.contains("function ") && !code.contains("function(") {
            return Err(AgentError::ValidationError(
                "Invalid function declaration syntax".to_string()
            ));
        }

        // Check for unclosed template literals
        let backtick_count = code.matches('`').count();
        if backtick_count % 2 != 0 {
            return Err(AgentError::ValidationError(
                "Unclosed template literal".to_string()
            ));
        }

        debug!("JavaScript/TypeScript basic syntax checks passed");
        Ok(())
    }

    fn apply_refactoring(
        &self,
        code: &str,
        units: &[crate::cortex_bridge::CodeUnit],
        episodes: &[Episode],
        patterns: &[Pattern],
        refactoring_type: RefactoringType,
    ) -> Result<(String, Vec<String>)> {
        let mut changes = Vec::new();

        match refactoring_type {
            RefactoringType::Simplify => {
                // For Simplify, do simple cleanup without LLM
                let refactored = code
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .collect::<Vec<_>>()
                    .join("\n");
                changes.push("Removed empty lines".to_string());
                Ok((refactored, changes))
            }
            RefactoringType::ExtractMethod
            | RefactoringType::Rename
            | RefactoringType::Inline
            | RefactoringType::ExtractVariable => {
                // Use Claude for complex refactorings
                let mut prompt = format!(
                    "Apply {:?} refactoring to the following code:\n\n```\n{}\n```\n\n",
                    refactoring_type, code
                );

                // Add context from code units
                if !units.is_empty() {
                    prompt.push_str("Code structure context:\n");
                    for unit in units.iter().take(5) {
                        prompt.push_str(&format!(
                            "- {} {} (complexity: {})\n",
                            unit.unit_type, unit.name, unit.complexity.cyclomatic
                        ));
                    }
                    prompt.push('\n');
                }

                // Add refactoring patterns
                if !patterns.is_empty() {
                    prompt.push_str("Refactoring patterns to consider:\n");
                    for pattern in patterns.iter().take(3) {
                        prompt.push_str(&format!("- {}: {}\n", pattern.name, pattern.description));
                    }
                    prompt.push('\n');
                }

                // Add episodes context
                if !episodes.is_empty() {
                    prompt.push_str("Past refactoring experiences:\n");
                    for episode in episodes.iter().take(2) {
                        prompt.push_str(&format!("- {}\n", episode.task_description));
                    }
                    prompt.push('\n');
                }

                prompt.push_str(&format!(
                    "Apply the {:?} refactoring. Return the refactored code in a code block \
                    and list the changes made. Format your response as:\n\
                    1. Code block with refactored code\n\
                    2. List of changes (one per line, starting with '-')\n",
                    refactoring_type
                ));

                debug!("Calling Claude for refactoring");
                let response = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async {
                        self.query_claude(&prompt).await
                    })
                })?;

                // Extract refactored code
                let refactored_code = self.extract_code_blocks(&response, "rust")?;
                let code_to_use = if refactored_code.is_empty() {
                    warn!("No code blocks in refactoring response");
                    code.to_string()
                } else {
                    refactored_code
                };

                // Extract changes from response
                changes = self.extract_changes_list(&response);

                if changes.is_empty() {
                    changes.push(format!("Applied {:?} refactoring", refactoring_type));
                }

                Ok((code_to_use, changes))
            }
        }
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
        patterns: &[Pattern],
        episodes: &[Episode],
    ) -> Result<(String, Vec<String>)> {
        if bottlenecks.is_empty() {
            debug!("No bottlenecks identified, returning original code");
            return Ok((code.to_string(), vec!["No optimizations needed".to_string()]));
        }

        // Build optimization prompt
        let mut prompt = format!(
            "Optimize the following code to address performance bottlenecks:\n\n```rust\n{}\n```\n\n",
            code
        );

        prompt.push_str("Identified bottlenecks:\n");
        for (i, bottleneck) in bottlenecks.iter().enumerate() {
            prompt.push_str(&format!("{}. {}\n", i + 1, bottleneck));
        }
        prompt.push('\n');

        // Add optimization patterns
        if !patterns.is_empty() {
            prompt.push_str("Optimization patterns to consider:\n");
            for pattern in patterns.iter().take(3) {
                prompt.push_str(&format!("- {}: {}\n", pattern.name, pattern.description));
            }
            prompt.push('\n');
        }

        // Add past optimization experiences
        if !episodes.is_empty() {
            prompt.push_str("Past optimization experiences:\n");
            for episode in episodes.iter().take(3) {
                prompt.push_str(&format!("- {}\n", episode.task_description));
                if let Some(metrics) = episode.success_metrics.as_object() {
                    if let Some(improvement) = metrics.get("improvement_percent") {
                        prompt.push_str(&format!("  Achieved: {} improvement\n", improvement));
                    }
                }
            }
            prompt.push('\n');
        }

        prompt.push_str(
            "Apply performance optimizations to address the bottlenecks. Consider:\n\
            - Algorithm optimization (better complexity)\n\
            - Data structure improvements\n\
            - Caching strategies\n\
            - Parallel processing where applicable\n\
            - Memory allocation optimization\n\n\
            Return:\n\
            1. Optimized code in a code block\n\
            2. List of optimizations applied (one per line, starting with '-')\n\
            3. Expected performance improvement\n"
        );

        debug!("Calling Claude for optimization");
        let response = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.query_claude(&prompt).await
            })
        })?;

        // Extract optimized code
        let optimized_code = self.extract_code_blocks(&response, "rust")?;
        let code_to_use = if optimized_code.is_empty() {
            warn!("No code blocks in optimization response, using original");
            code.to_string()
        } else {
            optimized_code
        };

        // Extract optimizations list
        let mut optimizations = self.extract_changes_list(&response);

        if optimizations.is_empty() {
            optimizations = bottlenecks
                .iter()
                .map(|b| format!("Addressed: {}", b))
                .collect();
        }

        Ok((code_to_use, optimizations))
    }

    /// Query Claude CLI and collect response text
    async fn query_claude(&self, prompt: &str) -> Result<String> {
        let options = ClaudeCodeOptions::builder()
            .system_prompt(crate::cc::options::SystemPrompt::String(
                "You are an expert software engineer. Provide clear, concise, and accurate responses. \
                When generating code, always wrap it in appropriate code blocks with language tags."
                    .to_string(),
            ))
            .build();

        let mut response_stream = query(prompt, Some(options))
            .await
            .map_err(|e| AgentError::CortexError(format!("Claude query failed: {}", e)))?;

        let mut collected_text = String::new();

        while let Some(msg_result) = response_stream.next().await {
            match msg_result {
                Ok(Message::Assistant { message }) => {
                    for content_block in &message.content {
                        if let ContentBlock::Text(text_content) = content_block {
                            collected_text.push_str(&text_content.text);
                        }
                    }
                }
                Ok(Message::Result { result, is_error, .. }) => {
                    if is_error {
                        if let Some(err_msg) = result {
                            return Err(AgentError::CortexError(format!("Claude error: {}", err_msg)));
                        }
                    }
                    debug!("Claude query completed");
                }
                Ok(_) => {
                    // Ignore other message types
                }
                Err(e) => {
                    return Err(AgentError::CortexError(format!("Stream error: {}", e)));
                }
            }
        }

        if collected_text.is_empty() {
            return Err(AgentError::CortexError("No response from Claude".to_string()));
        }

        Ok(collected_text)
    }

    /// Extract code blocks from Claude's response
    fn extract_code_blocks(&self, response: &str, language: &str) -> Result<String> {
        // Look for code blocks with language tag: ```language\n...\n```
        let lang_pattern = format!("```{}", language);
        let mut code_blocks = Vec::new();
        let mut in_code_block = false;
        let mut current_block = String::new();

        for line in response.lines() {
            if line.trim().starts_with(&lang_pattern) || line.trim() == "```rust" || line.trim() == "```" {
                if in_code_block {
                    // End of code block
                    if !current_block.is_empty() {
                        code_blocks.push(current_block.clone());
                        current_block.clear();
                    }
                    in_code_block = false;
                } else {
                    // Start of code block
                    in_code_block = true;
                }
            } else if in_code_block {
                current_block.push_str(line);
                current_block.push('\n');
            }
        }

        // If we ended while still in a code block, add it
        if in_code_block && !current_block.is_empty() {
            code_blocks.push(current_block);
        }

        if code_blocks.is_empty() {
            // Try to extract any code block (without language tag)
            in_code_block = false;
            current_block = String::new();

            for line in response.lines() {
                if line.trim().starts_with("```") {
                    if in_code_block {
                        if !current_block.is_empty() {
                            code_blocks.push(current_block.clone());
                            current_block.clear();
                        }
                        in_code_block = false;
                    } else {
                        in_code_block = true;
                    }
                } else if in_code_block {
                    current_block.push_str(line);
                    current_block.push('\n');
                }
            }

            if in_code_block && !current_block.is_empty() {
                code_blocks.push(current_block);
            }
        }

        if code_blocks.is_empty() {
            return Ok(String::new());
        }

        // Return the first (or concatenated) code block
        Ok(code_blocks.join("\n\n"))
    }

    /// Extract a list of changes from Claude's response
    fn extract_changes_list(&self, response: &str) -> Vec<String> {
        let mut changes = Vec::new();

        for line in response.lines() {
            let trimmed = line.trim();
            // Look for lines starting with - or * or numbered lists
            if trimmed.starts_with("- ") {
                changes.push(trimmed[2..].to_string());
            } else if trimmed.starts_with("* ") {
                changes.push(trimmed[2..].to_string());
            } else if let Some(rest) = trimmed.strip_prefix(|c: char| c.is_numeric()) {
                if rest.starts_with(". ") || rest.starts_with(") ") {
                    changes.push(rest[2..].to_string());
                }
            }
        }

        changes
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
