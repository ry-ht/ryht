//! Cognitive manager orchestrating all memory operations.

use crate::types::*;
use crate::{
    EpisodicMemorySystem, MemoryConsolidator, ProceduralMemorySystem, SemanticMemorySystem,
    WorkingMemorySystem,
};
use cortex_core::error::Result;
use cortex_core::id::CortexId;
use cortex_core::types::{CodeUnit, CodeUnitType as CoreCodeUnitType, Language, Visibility, Parameter, Complexity as CoreComplexity, CodeUnitStatus};
use cortex_storage::ConnectionManager;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, instrument};

/// Cognitive manager coordinating all memory systems
pub struct CognitiveManager {
    episodic: Arc<EpisodicMemorySystem>,
    semantic: Arc<SemanticMemorySystem>,
    working: Arc<WorkingMemorySystem>,
    procedural: Arc<ProceduralMemorySystem>,
    consolidator: Arc<MemoryConsolidator>,
}

impl CognitiveManager {
    /// Create a new cognitive manager
    pub fn new(connection_manager: Arc<ConnectionManager>) -> Self {
        let episodic = Arc::new(EpisodicMemorySystem::new(connection_manager.clone()));
        let semantic = Arc::new(SemanticMemorySystem::new(connection_manager.clone()));
        let working = Arc::new(WorkingMemorySystem::new(1000, 100 * 1024 * 1024)); // 100MB
        let procedural = Arc::new(ProceduralMemorySystem::new(connection_manager));

        let consolidator = Arc::new(MemoryConsolidator::new(
            episodic.clone(),
            semantic.clone(),
            procedural.clone(),
            working.clone(),
        ));

        Self {
            episodic,
            semantic,
            working,
            procedural,
            consolidator,
        }
    }

    /// Create with custom configuration
    pub fn with_config(connection_manager: Arc<ConnectionManager>, max_items: usize, max_bytes: usize) -> Self {
        let episodic = Arc::new(EpisodicMemorySystem::new(connection_manager.clone()));
        let semantic = Arc::new(SemanticMemorySystem::new(connection_manager.clone()));
        let working = Arc::new(WorkingMemorySystem::new(max_items, max_bytes));
        let procedural = Arc::new(ProceduralMemorySystem::new(connection_manager));

        let consolidator = Arc::new(MemoryConsolidator::new(
            episodic.clone(),
            semantic.clone(),
            procedural.clone(),
            working.clone(),
        ));

        Self {
            episodic,
            semantic,
            working,
            procedural,
            consolidator,
        }
    }

    // ========================================================================
    // Cognitive Operations (Remember, Recall, Associate, Forget, Dream)
    // ========================================================================

    /// Remember: Store a new episode
    #[instrument(skip(self, episode))]
    pub async fn remember_episode(&self, episode: &EpisodicMemory) -> Result<CortexId> {
        info!(episode_id = %episode.id, "Remembering episode");
        self.episodic.store_episode(episode).await
    }

    /// Remember: Store a semantic unit
    #[instrument(skip(self, unit))]
    pub async fn remember_unit(&self, unit: &SemanticUnit) -> Result<CortexId> {
        info!(unit_id = %unit.id, "Remembering semantic unit");
        // Convert SemanticUnit to CodeUnit
        let code_unit = convert_semantic_to_code_unit(unit);
        self.semantic.store_unit(&code_unit).await
    }

    /// Remember: Store a learned pattern
    #[instrument(skip(self, pattern))]
    pub async fn remember_pattern(&self, pattern: &LearnedPattern) -> Result<CortexId> {
        info!(pattern_id = %pattern.id, "Remembering learned pattern");
        self.procedural.store_pattern(pattern).await
    }

    /// Recall: Retrieve similar episodes
    #[instrument(skip(self, query, embedding))]
    pub async fn recall_episodes(
        &self,
        query: &MemoryQuery,
        embedding: &[f32],
    ) -> Result<Vec<MemorySearchResult<EpisodicMemory>>> {
        info!(query = %query.query_text, "Recalling episodes");
        self.episodic.retrieve_similar(query, embedding).await
    }

    /// Recall: Retrieve similar code units
    #[instrument(skip(self, query, embedding))]
    pub async fn recall_units(
        &self,
        query: &MemoryQuery,
        embedding: &[f32],
    ) -> Result<Vec<MemorySearchResult<SemanticUnit>>> {
        info!(query = %query.query_text, "Recalling semantic units");
        // Convert CodeUnit results to SemanticUnit for backward compatibility
        let code_units = self.semantic.search_units(query, embedding).await?;
        let semantic_units = code_units.into_iter().map(|result| {
            MemorySearchResult {
                item: convert_code_to_semantic_unit(&result.item),
                similarity_score: result.similarity_score,
                relevance_score: result.relevance_score,
            }
        }).collect();
        Ok(semantic_units)
    }

    /// Recall: Retrieve similar patterns
    #[instrument(skip(self, query, embedding))]
    pub async fn recall_patterns(
        &self,
        query: &MemoryQuery,
        embedding: &[f32],
    ) -> Result<Vec<MemorySearchResult<LearnedPattern>>> {
        info!(query = %query.query_text, "Recalling patterns");
        self.procedural.search_patterns(query, embedding).await
    }

    /// Associate: Link related memories
    #[instrument(skip(self))]
    pub async fn associate(
        &self,
        source_id: CortexId,
        target_id: CortexId,
        dependency_type: DependencyType,
    ) -> Result<()> {
        info!(%source_id, %target_id, "Creating association");

        let dependency = Dependency {
            id: CortexId::new(),
            source_id,
            target_id,
            dependency_type,
            is_direct: true,
            is_runtime: false,
            is_dev: false,
            metadata: std::collections::HashMap::new(),
        };

        self.semantic.store_dependency(&dependency).await?;
        Ok(())
    }

    /// Forget: Remove low-importance memories
    #[instrument(skip(self))]
    pub async fn forget(&self, threshold: f32) -> Result<usize> {
        info!(threshold, "Forgetting low-importance memories");
        self.episodic.forget_unimportant(threshold).await
    }

    /// Forget: Remove memories before a specific date
    #[instrument(skip(self))]
    pub async fn forget_before(&self, before: &chrono::DateTime<chrono::Utc>, workspace: Option<&str>) -> Result<usize> {
        info!(before = %before, workspace = ?workspace, "Forgetting memories before date");

        // Parse workspace string to CortexId if provided
        let workspace_id = if let Some(ws) = workspace {
            Some(CortexId::parse(ws).map_err(|e| {
                cortex_core::error::CortexError::invalid_input(format!("Invalid workspace ID: {}", e))
            })?)
        } else {
            None
        };

        // Delete from episodic memory
        let episodic_deleted = self.episodic.forget_before(before, workspace_id).await?;

        // Delete from semantic memory (semantic units don't have workspace filtering)
        let semantic_deleted = if workspace.is_none() {
            self.semantic.forget_before(before).await?
        } else {
            0
        };

        let total_deleted = episodic_deleted + semantic_deleted;
        info!(episodic_deleted, semantic_deleted, total_deleted, "Memories forgotten");
        Ok(total_deleted)
    }

    /// Dream: Offline consolidation and pattern extraction
    #[instrument(skip(self))]
    pub async fn dream(&self) -> Result<Vec<LearnedPattern>> {
        info!("Starting dream consolidation");
        self.consolidator.dream().await
    }

    /// Consolidate: Transfer from working to long-term memory
    #[instrument(skip(self))]
    pub async fn consolidate(&self) -> Result<crate::consolidation::ConsolidationReport> {
        info!("Starting memory consolidation");
        self.consolidator.consolidate().await
    }

    /// Perform incremental consolidation with batch size
    #[instrument(skip(self))]
    pub async fn consolidate_incremental(&self, batch_size: usize) -> Result<crate::consolidation::ConsolidationReport> {
        info!(batch_size, "Starting incremental consolidation");
        self.consolidator.incremental_consolidate(batch_size).await
    }

    // ========================================================================
    // Access to Memory Systems
    // ========================================================================

    pub fn episodic(&self) -> &Arc<EpisodicMemorySystem> {
        &self.episodic
    }

    pub fn semantic(&self) -> &Arc<SemanticMemorySystem> {
        &self.semantic
    }

    pub fn working(&self) -> &Arc<WorkingMemorySystem> {
        &self.working
    }

    pub fn procedural(&self) -> &Arc<ProceduralMemorySystem> {
        &self.procedural
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Get comprehensive memory statistics
    pub async fn get_statistics(&self) -> Result<MemoryStats> {
        Ok(MemoryStats {
            episodic: self.episodic.get_statistics().await?,
            semantic: self.semantic.get_statistics().await?,
            working: self.working.get_statistics(),
            procedural: self.procedural.get_statistics().await?,
        })
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert SemanticUnit to CodeUnit
fn convert_semantic_to_code_unit(unit: &SemanticUnit) -> CodeUnit {
    CodeUnit {
        id: unit.id,
        unit_type: convert_unit_type(unit.unit_type),
        name: unit.name.clone(),
        qualified_name: unit.qualified_name.clone(),
        display_name: unit.display_name.clone(),
        file_path: unit.file_path.clone(),
        language: Language::Unknown, // Will need to infer from file_path
        start_line: unit.start_line as usize,
        end_line: unit.end_line as usize,
        start_column: unit.start_column as usize,
        end_column: unit.end_column as usize,
        start_byte: 0,
        end_byte: 0,
        signature: unit.signature.clone(),
        body: Some(unit.body.clone()),
        docstring: unit.docstring.clone(),
        comments: Vec::new(),
        return_type: unit.return_type.clone(),
        parameters: unit.parameters.iter().map(|p| Parameter {
            name: p.clone(),
            param_type: None,
            default_value: None,
            is_optional: false,
            is_variadic: false,
            attributes: Vec::new(),
        }).collect(),
        type_parameters: Vec::new(),
        generic_constraints: Vec::new(),
        throws: Vec::new(),
        visibility: convert_visibility(&unit.visibility),
        attributes: Vec::new(),
        modifiers: unit.modifiers.clone(),
        is_async: unit.modifiers.contains(&"async".to_string()),
        is_unsafe: unit.modifiers.contains(&"unsafe".to_string()),
        is_const: unit.modifiers.contains(&"const".to_string()),
        is_static: unit.modifiers.contains(&"static".to_string()),
        is_abstract: false,
        is_virtual: false,
        is_override: false,
        is_final: false,
        is_exported: false,
        is_default_export: false,
        complexity: CoreComplexity {
            cyclomatic: unit.complexity.cyclomatic,
            cognitive: unit.complexity.cognitive,
            nesting: unit.complexity.nesting,
            lines: unit.complexity.lines,
            parameters: 0,
            returns: 0,
        },
        test_coverage: unit.test_coverage.map(|c| c as f64),
        has_tests: unit.has_tests,
        has_documentation: unit.has_documentation,
        language_specific: HashMap::new(),
        embedding: unit.embedding.clone(),
        embedding_model: Some("text-embedding-3-small".to_string()),
        summary: Some(unit.summary.clone()),
        purpose: Some(unit.purpose.clone()),
        ast_node_type: None,
        ast_metadata: None,
        status: CodeUnitStatus::Active,
        version: 1,
        created_at: unit.created_at,
        updated_at: unit.updated_at,
        created_by: "system".to_string(),
        updated_by: "system".to_string(),
        tags: Vec::new(),
        metadata: HashMap::new(),
    }
}

fn convert_unit_type(unit_type: CodeUnitType) -> CoreCodeUnitType {
    match unit_type {
        CodeUnitType::Function => CoreCodeUnitType::Function,
        CodeUnitType::Method => CoreCodeUnitType::Method,
        CodeUnitType::AsyncFunction => CoreCodeUnitType::AsyncFunction,
        CodeUnitType::Generator => CoreCodeUnitType::Generator,
        CodeUnitType::Lambda => CoreCodeUnitType::Lambda,
        CodeUnitType::Class => CoreCodeUnitType::Class,
        CodeUnitType::Struct => CoreCodeUnitType::Struct,
        CodeUnitType::Enum => CoreCodeUnitType::Enum,
        CodeUnitType::Union => CoreCodeUnitType::Union,
        CodeUnitType::Interface => CoreCodeUnitType::Interface,
        CodeUnitType::Trait => CoreCodeUnitType::Trait,
        CodeUnitType::TypeAlias => CoreCodeUnitType::TypeAlias,
        CodeUnitType::Typedef => CoreCodeUnitType::Typedef,
        CodeUnitType::Const => CoreCodeUnitType::Const,
        CodeUnitType::Static => CoreCodeUnitType::Static,
        CodeUnitType::Variable => CoreCodeUnitType::Variable,
        CodeUnitType::Module => CoreCodeUnitType::Module,
        CodeUnitType::Namespace => CoreCodeUnitType::Namespace,
        CodeUnitType::Package => CoreCodeUnitType::Package,
        CodeUnitType::ImplBlock => CoreCodeUnitType::ImplBlock,
        CodeUnitType::Decorator => CoreCodeUnitType::Decorator,
        CodeUnitType::Macro => CoreCodeUnitType::Macro,
        CodeUnitType::Template => CoreCodeUnitType::Template,
        CodeUnitType::Test => CoreCodeUnitType::Test,
        CodeUnitType::Benchmark => CoreCodeUnitType::Benchmark,
        CodeUnitType::Example => CoreCodeUnitType::Example,
    }
}

fn convert_visibility(visibility: &str) -> Visibility {
    match visibility.to_lowercase().as_str() {
        "public" => Visibility::Public,
        "private" => Visibility::Private,
        "protected" => Visibility::Protected,
        "internal" => Visibility::Internal,
        "package" => Visibility::Package,
        _ => Visibility::Private,
    }
}

/// Convert CodeUnit to SemanticUnit for backward compatibility
fn convert_code_to_semantic_unit(unit: &CodeUnit) -> SemanticUnit {
    SemanticUnit {
        id: unit.id,
        unit_type: convert_core_unit_type(unit.unit_type),
        name: unit.name.clone(),
        qualified_name: unit.qualified_name.clone(),
        display_name: unit.display_name.clone(),
        file_path: unit.file_path.clone(),
        start_line: unit.start_line as u32,
        start_column: unit.start_column as u32,
        end_line: unit.end_line as u32,
        end_column: unit.end_column as u32,
        signature: unit.signature.clone(),
        body: unit.body.clone().unwrap_or_default(),
        docstring: unit.docstring.clone(),
        visibility: convert_core_visibility(unit.visibility),
        modifiers: unit.modifiers.clone(),
        parameters: unit.parameters.iter().map(|p| p.name.clone()).collect(),
        return_type: unit.return_type.clone(),
        summary: unit.summary.clone().unwrap_or_default(),
        purpose: unit.purpose.clone().unwrap_or_default(),
        complexity: ComplexityMetrics {
            cyclomatic: unit.complexity.cyclomatic,
            cognitive: unit.complexity.cognitive,
            nesting: unit.complexity.nesting,
            lines: unit.complexity.lines,
        },
        test_coverage: unit.test_coverage.map(|c| c as f32),
        has_tests: unit.has_tests,
        has_documentation: unit.has_documentation,
        embedding: unit.embedding.clone(),
        created_at: unit.created_at,
        updated_at: unit.updated_at,
    }
}

fn convert_core_unit_type(unit_type: CoreCodeUnitType) -> CodeUnitType {
    match unit_type {
        CoreCodeUnitType::Function => CodeUnitType::Function,
        CoreCodeUnitType::Method => CodeUnitType::Method,
        CoreCodeUnitType::AsyncFunction => CodeUnitType::AsyncFunction,
        CoreCodeUnitType::Generator => CodeUnitType::Generator,
        CoreCodeUnitType::Lambda => CodeUnitType::Lambda,
        CoreCodeUnitType::Class => CodeUnitType::Class,
        CoreCodeUnitType::Struct => CodeUnitType::Struct,
        CoreCodeUnitType::Enum => CodeUnitType::Enum,
        CoreCodeUnitType::Union => CodeUnitType::Union,
        CoreCodeUnitType::Interface => CodeUnitType::Interface,
        CoreCodeUnitType::Trait => CodeUnitType::Trait,
        CoreCodeUnitType::TypeAlias => CodeUnitType::TypeAlias,
        CoreCodeUnitType::Typedef => CodeUnitType::Typedef,
        CoreCodeUnitType::Const => CodeUnitType::Const,
        CoreCodeUnitType::Static => CodeUnitType::Static,
        CoreCodeUnitType::Variable => CodeUnitType::Variable,
        CoreCodeUnitType::Module => CodeUnitType::Module,
        CoreCodeUnitType::Namespace => CodeUnitType::Namespace,
        CoreCodeUnitType::Package => CodeUnitType::Package,
        CoreCodeUnitType::ImplBlock => CodeUnitType::ImplBlock,
        CoreCodeUnitType::Decorator => CodeUnitType::Decorator,
        CoreCodeUnitType::Macro => CodeUnitType::Macro,
        CoreCodeUnitType::Template => CodeUnitType::Template,
        CoreCodeUnitType::Test => CodeUnitType::Test,
        CoreCodeUnitType::Benchmark => CodeUnitType::Benchmark,
        CoreCodeUnitType::Example => CodeUnitType::Example,
    }
}

fn convert_core_visibility(visibility: Visibility) -> String {
    match visibility {
        Visibility::Public => "public".to_string(),
        Visibility::Private => "private".to_string(),
        Visibility::Protected => "protected".to_string(),
        Visibility::Internal => "internal".to_string(),
        Visibility::Package => "package".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cortex_storage::connection_pool::{ConnectionManager, DatabaseConfig, ConnectionMode, Credentials, PoolConfig, RetryPolicy};
    use std::time::Duration;

    async fn create_test_manager() -> CognitiveManager {
        let config = DatabaseConfig {
            connection_mode: ConnectionMode::Local {
                endpoint: "memory".to_string(),
            },
            credentials: Credentials {
                username: None,
                password: None,
            },
            pool_config: PoolConfig {
                min_connections: 1,
                max_connections: 10,
                connection_timeout: Duration::from_secs(5),
                idle_timeout: None,
                max_lifetime: None,
                retry_policy: RetryPolicy {
                    max_attempts: 3,
                    initial_backoff: Duration::from_millis(100),
                    max_backoff: Duration::from_secs(10),
                    multiplier: 2.0,
                },
                warm_connections: false,
                validate_on_checkout: true,
                recycle_after_uses: Some(1000),
                shutdown_grace_period: Duration::from_secs(5),
            },
            namespace: "test".to_string(),
            database: "test".to_string(),
        };

        let manager = Arc::new(ConnectionManager::new(config).await.unwrap());
        CognitiveManager::new(manager)
    }

    #[tokio::test]
    async fn test_remember_and_recall() {
        let manager = create_test_manager().await;

        let episode = EpisodicMemory::new(
            "Test episode".to_string(),
            "test-agent".to_string(),
            CortexId::new(),
            EpisodeType::Task,
        );

        let id = manager
            .remember_episode(&episode)
            .await
            .expect("Failed to remember episode");

        assert_eq!(id, episode.id);
    }

    #[tokio::test]
    async fn test_working_memory() {
        let manager = create_test_manager().await;

        let key = "test_key".to_string();
        let value = vec![1, 2, 3];

        assert!(manager.working().store(key.clone(), value.clone(), Priority::Medium));
        assert_eq!(manager.working().retrieve(&key), Some(value));
    }

    #[tokio::test]
    async fn test_statistics() {
        let manager = create_test_manager().await;

        let stats = manager
            .get_statistics()
            .await
            .expect("Failed to get statistics");

        assert_eq!(stats.episodic.total_episodes, 0);
        assert_eq!(stats.semantic.total_units, 0);
        assert_eq!(stats.working.current_items, 0);
        assert_eq!(stats.procedural.total_patterns, 0);
    }
}
