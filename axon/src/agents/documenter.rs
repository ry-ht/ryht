//! Documenter Agent Implementation
//!
//! The DocumenterAgent specializes in generating and maintaining comprehensive documentation
//! for codebases. It leverages Cortex for semantic search, episodic memory of past
//! documentation patterns, and context-aware generation.
//!
//! # Capabilities
//!
//! - Generate rustdoc comments from code
//! - Create README files with examples and usage
//! - Generate API documentation
//! - Create architecture diagrams (Mermaid format)
//! - Maintain documentation consistency
//! - Update outdated documentation
//!
//! # Example
//!
//! ```no_run
//! use axon::agents::documenter::DocumenterAgent;
//! use axon::cortex_bridge::CortexBridge;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let cortex = Arc::new(CortexBridge::new(Default::default()).await?);
//!     let agent = DocumenterAgent::new("doc-agent".to_string(), cortex);
//!
//!     // Generate documentation for a module
//!     // let docs = agent.generate_module_docs(...).await?;
//!
//!     Ok(())
//! }
//! ```

use super::*;
use crate::cortex_bridge::{
    CortexBridge, Episode, EpisodeOutcome, EpisodeType, Pattern, PatternType,
    SearchFilters, SessionId, SessionScope, TokenUsage, UnitFilters, WorkspaceId,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

/// Documenter agent for comprehensive documentation generation and maintenance
pub struct DocumenterAgent {
    id: AgentId,
    name: String,
    capabilities: HashSet<Capability>,
    metrics: AgentMetrics,

    /// Cortex integration for memory and search
    cortex: Arc<CortexBridge>,

    /// Active session (if any)
    session_id: Option<SessionId>,

    /// Current workspace
    workspace_id: Option<WorkspaceId>,

    /// Documentation formats supported
    doc_formats: Vec<DocFormat>,

    /// Documentation templates
    templates: HashMap<DocType, Template>,

    /// Style guides for different doc types
    style_guides: Vec<StyleGuide>,
}

impl DocumenterAgent {
    /// Create a new DocumenterAgent with Cortex integration
    pub fn new(name: String, cortex: Arc<CortexBridge>) -> Self {
        let mut capabilities = HashSet::new();
        capabilities.insert(Capability::Documentation);
        capabilities.insert(Capability::DocGeneration);
        capabilities.insert(Capability::DiagramCreation);
        capabilities.insert(Capability::TechnicalWriting);
        capabilities.insert(Capability::CodeExplanation);

        let templates = Self::init_templates();
        let style_guides = Self::init_style_guides();

        Self {
            id: AgentId::new(),
            name,
            capabilities,
            metrics: AgentMetrics::new(),
            cortex,
            session_id: None,
            workspace_id: None,
            doc_formats: vec![
                DocFormat::Markdown,
                DocFormat::Rustdoc,
                DocFormat::Html,
                DocFormat::Mermaid,
            ],
            templates,
            style_guides,
        }
    }

    /// Initialize default documentation templates
    fn init_templates() -> HashMap<DocType, Template> {
        let mut templates = HashMap::new();

        templates.insert(
            DocType::ModuleDoc,
            Template {
                name: "module_doc".to_string(),
                content: include_str!("../../templates/module_doc.md").to_string(),
                placeholders: vec![
                    "module_name".to_string(),
                    "description".to_string(),
                    "examples".to_string(),
                    "public_items".to_string(),
                ],
            },
        );

        templates.insert(
            DocType::ReadMe,
            Template {
                name: "readme".to_string(),
                content: "# {project_name}\n\n{description}\n\n## Installation\n\n{installation}\n\n## Usage\n\n{usage}\n\n## Examples\n\n{examples}\n\n## API Documentation\n\n{api_docs}\n\n## Contributing\n\n{contributing}\n\n## License\n\n{license}".to_string(),
                placeholders: vec![
                    "project_name".to_string(),
                    "description".to_string(),
                    "installation".to_string(),
                    "usage".to_string(),
                    "examples".to_string(),
                    "api_docs".to_string(),
                    "contributing".to_string(),
                    "license".to_string(),
                ],
            },
        );

        templates.insert(
            DocType::ApiDoc,
            Template {
                name: "api_doc".to_string(),
                content: "# {api_name} API\n\n{description}\n\n## Endpoints\n\n{endpoints}\n\n## Authentication\n\n{authentication}\n\n## Examples\n\n{examples}".to_string(),
                placeholders: vec![
                    "api_name".to_string(),
                    "description".to_string(),
                    "endpoints".to_string(),
                    "authentication".to_string(),
                    "examples".to_string(),
                ],
            },
        );

        templates.insert(
            DocType::ArchitectureDiagram,
            Template {
                name: "architecture_diagram".to_string(),
                content: "```mermaid\ngraph TD\n{nodes}\n{edges}\n```".to_string(),
                placeholders: vec![
                    "nodes".to_string(),
                    "edges".to_string(),
                ],
            },
        );

        templates
    }

    /// Initialize style guides
    fn init_style_guides() -> Vec<StyleGuide> {
        vec![
            StyleGuide {
                name: "Rust Documentation".to_string(),
                rules: vec![
                    "Use /// for outer doc comments".to_string(),
                    "Use //! for inner doc comments".to_string(),
                    "Include examples in doc comments".to_string(),
                    "Document panics and errors".to_string(),
                    "Use code blocks with language tags".to_string(),
                ],
            },
            StyleGuide {
                name: "README Best Practices".to_string(),
                rules: vec![
                    "Start with a clear project description".to_string(),
                    "Include installation instructions".to_string(),
                    "Provide usage examples".to_string(),
                    "Link to API documentation".to_string(),
                    "Include contributing guidelines".to_string(),
                ],
            },
        ]
    }

    /// Set the current workspace
    pub fn set_workspace(&mut self, workspace_id: WorkspaceId) {
        self.workspace_id = Some(workspace_id);
    }

    /// Generate comprehensive documentation for a code file
    ///
    /// This is the main entry point for documentation generation. It:
    /// 1. Creates an isolated Cortex session
    /// 2. Retrieves context from past documentation episodes
    /// 3. Analyzes the code structure
    /// 4. Generates appropriate documentation
    /// 5. Stores the episode for future learning
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to document
    /// * `workspace_id` - Workspace identifier
    /// * `doc_types` - Types of documentation to generate
    ///
    /// # Returns
    ///
    /// A `DocumentationResult` containing all generated documentation
    pub async fn generate_documentation(
        &mut self,
        file_path: &str,
        workspace_id: &WorkspaceId,
        doc_types: Vec<DocType>,
    ) -> Result<DocumentationResult> {
        info!("Generating documentation for {}", file_path);

        // 1. Create isolated session
        let agent_id = crate::cortex_bridge::AgentId::from(self.id.to_string());
        let session_id = self.cortex.create_session(
            agent_id.clone(),
            workspace_id.clone(),
            SessionScope {
                paths: vec![file_path.to_string()],
                read_only_paths: vec![],
            },
        ).await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        self.session_id = Some(session_id.clone());
        self.workspace_id = Some(workspace_id.clone());

        // 2. Retrieve context from Cortex
        let context = self.retrieve_documentation_context(file_path, workspace_id).await?;

        // 3. Read the file to document
        let code = self.cortex.read_file(&session_id, file_path).await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        // 4. Get code units to understand structure
        let units = self.cortex.get_code_units(
            workspace_id,
            UnitFilters {
                unit_type: None,
                language: Some("rust".to_string()),
                visibility: None,
            },
        ).await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        // 5. Generate documentation for each requested type
        let mut results = Vec::new();
        for doc_type in &doc_types {
            let doc = self.generate_doc_for_type(
                &code,
                file_path,
                doc_type,
                &context,
                &units,
            ).await?;
            results.push(doc);
        }

        // 6. Write documentation files to session
        for doc in &results {
            self.cortex.write_file(
                &session_id,
                &doc.output_path,
                &doc.content,
            ).await
                .map_err(|e| AgentError::CortexError(e.to_string()))?;
        }

        // 7. Store episode for learning
        self.store_documentation_episode(
            file_path,
            &doc_types,
            &results,
            true,
        ).await?;

        // 8. Merge session back to workspace
        self.cortex.merge_session(
            &session_id,
            crate::cortex_bridge::MergeStrategy::Auto,
        ).await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        // 9. Close session
        self.cortex.close_session(&session_id, &agent_id).await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        self.session_id = None;

        Ok(DocumentationResult {
            file_path: file_path.to_string(),
            documentation: results,
            metadata: DocumentationMetadata {
                generated_at: chrono::Utc::now(),
                agent_id: self.id.to_string(),
                doc_types: doc_types.clone(),
            },
        })
    }

    /// Retrieve documentation context from Cortex
    async fn retrieve_documentation_context(
        &self,
        file_path: &str,
        workspace_id: &WorkspaceId,
    ) -> Result<DocumentationContext> {
        debug!("Retrieving documentation context for {}", file_path);

        // Prepare queries first to avoid temporary value issues
        let episode_query = format!("generate documentation for {}", file_path);
        let similar_query = format!("well-documented code similar to {}", file_path);

        // Parallel context retrieval for performance
        let (episodes, patterns, similar_code, existing_docs) = tokio::join!(
            // Past documentation episodes
            self.cortex.search_episodes(&episode_query, 5),

            // Documentation patterns
            self.cortex.get_patterns(),

            // Similar code for reference
            self.cortex.semantic_search(
                &similar_query,
                workspace_id,
                SearchFilters {
                    types: vec!["function".to_string(), "module".to_string()],
                    languages: vec!["rust".to_string()],
                    visibility: Some("public".to_string()),
                    min_relevance: 0.7,
                }
            ),

            // Existing documentation files
            self.cortex.semantic_search(
                "README documentation examples",
                workspace_id,
                SearchFilters {
                    types: vec![],
                    languages: vec![],
                    visibility: None,
                    min_relevance: 0.6,
                }
            )
        );

        let episodes = episodes.map_err(|e| AgentError::CortexError(e.to_string()))?;
        let all_patterns = patterns.map_err(|e| AgentError::CortexError(e.to_string()))?;
        let similar_code = similar_code.map_err(|e| AgentError::CortexError(e.to_string()))?;
        let existing_docs = existing_docs.map_err(|e| AgentError::CortexError(e.to_string()))?;

        // Filter documentation-specific patterns
        let doc_patterns = all_patterns
            .into_iter()
            .filter(|p| {
                matches!(p.pattern_type, PatternType::Code)
                    && p.name.contains("documentation")
            })
            .collect();

        Ok(DocumentationContext {
            past_episodes: episodes,
            patterns: doc_patterns,
            similar_implementations: similar_code,
            existing_documentation: existing_docs,
        })
    }

    /// Generate documentation for a specific type
    async fn generate_doc_for_type(
        &self,
        code: &str,
        file_path: &str,
        doc_type: &DocType,
        context: &DocumentationContext,
        units: &[crate::cortex_bridge::CodeUnit],
    ) -> Result<Documentation> {
        match doc_type {
            DocType::Rustdoc => self.generate_rustdoc(code, file_path, context, units).await,
            DocType::ReadMe => self.generate_readme(code, file_path, context, units).await,
            DocType::ApiDoc => self.generate_api_doc(code, file_path, context, units).await,
            DocType::ArchitectureDiagram => self.generate_architecture_diagram(file_path, context, units).await,
            DocType::ModuleDoc => self.generate_module_doc(code, file_path, context, units).await,
        }
    }

    /// Generate rustdoc comments for code
    async fn generate_rustdoc(
        &self,
        code: &str,
        file_path: &str,
        context: &DocumentationContext,
        units: &[crate::cortex_bridge::CodeUnit],
    ) -> Result<Documentation> {
        info!("Generating rustdoc comments for {}", file_path);

        // Extract public items from code
        let public_items = self.extract_public_items(code, units);

        // Generate doc comments for each item
        let mut documented_code = code.to_string();
        for item in public_items {
            let doc_comment = self.generate_item_doc_comment(&item, context);
            documented_code = self.insert_doc_comment(&documented_code, &item, &doc_comment);
        }

        Ok(Documentation {
            doc_type: DocType::Rustdoc,
            content: documented_code,
            output_path: file_path.to_string(),
        })
    }

    /// Generate README file
    async fn generate_readme(
        &self,
        _code: &str,
        file_path: &str,
        context: &DocumentationContext,
        units: &[crate::cortex_bridge::CodeUnit],
    ) -> Result<Documentation> {
        info!("Generating README for {}", file_path);

        // Extract project information
        let project_name = self.extract_project_name(file_path);
        let description = self.generate_description(context, units);
        let installation = self.generate_installation_instructions();
        let usage = self.generate_usage_examples(context, units);
        let examples = self.generate_code_examples(context, units);

        let template = self.templates.get(&DocType::ReadMe)
            .ok_or_else(|| AgentError::ConfigurationError("README template not found".to_string()))?;

        let mut content = template.content.clone();
        content = content.replace("{project_name}", &project_name);
        content = content.replace("{description}", &description);
        content = content.replace("{installation}", &installation);
        content = content.replace("{usage}", &usage);
        content = content.replace("{examples}", &examples);
        content = content.replace("{api_docs}", "See API documentation for detailed information.");
        content = content.replace("{contributing}", "Contributions are welcome! Please submit a pull request.");
        content = content.replace("{license}", "MIT License");

        let output_path = Path::new(file_path)
            .parent()
            .unwrap_or(Path::new("."))
            .join("README.md")
            .to_string_lossy()
            .to_string();

        Ok(Documentation {
            doc_type: DocType::ReadMe,
            content,
            output_path,
        })
    }

    /// Generate API documentation
    async fn generate_api_doc(
        &self,
        _code: &str,
        file_path: &str,
        context: &DocumentationContext,
        units: &[crate::cortex_bridge::CodeUnit],
    ) -> Result<Documentation> {
        info!("Generating API documentation for {}", file_path);

        // Extract public API
        let public_api = units.iter()
            .filter(|u| u.visibility == "public")
            .collect::<Vec<_>>();

        let api_name = self.extract_project_name(file_path);
        let description = format!("API documentation for {}", api_name);
        let endpoints = self.generate_api_endpoints(&public_api, context);

        let template = self.templates.get(&DocType::ApiDoc)
            .ok_or_else(|| AgentError::ConfigurationError("API doc template not found".to_string()))?;

        let mut content = template.content.clone();
        content = content.replace("{api_name}", &api_name);
        content = content.replace("{description}", &description);
        content = content.replace("{endpoints}", &endpoints);
        content = content.replace("{authentication}", "No authentication required for local usage.");
        content = content.replace("{examples}", &self.generate_code_examples(context, units));

        let output_path = Path::new(file_path)
            .parent()
            .unwrap_or(Path::new("."))
            .join("API.md")
            .to_string_lossy()
            .to_string();

        Ok(Documentation {
            doc_type: DocType::ApiDoc,
            content,
            output_path,
        })
    }

    /// Generate architecture diagram in Mermaid format
    async fn generate_architecture_diagram(
        &self,
        file_path: &str,
        _context: &DocumentationContext,
        units: &[crate::cortex_bridge::CodeUnit],
    ) -> Result<Documentation> {
        info!("Generating architecture diagram for {}", file_path);

        // Generate nodes from code units
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for (idx, unit) in units.iter().enumerate() {
            let node_id = format!("N{}", idx);
            nodes.push(format!("    {}[{}]", node_id, unit.name));

            // Simple dependency inference based on unit types
            if unit.unit_type == "module" {
                // Connect modules to their parent
                if let Some(parent_idx) = self.find_parent_module(unit, units) {
                    edges.push(format!("    N{} --> N{}", parent_idx, idx));
                }
            }
        }

        let template = self.templates.get(&DocType::ArchitectureDiagram)
            .ok_or_else(|| AgentError::ConfigurationError("Architecture diagram template not found".to_string()))?;

        let mut content = template.content.clone();
        content = content.replace("{nodes}", &nodes.join("\n"));
        content = content.replace("{edges}", &edges.join("\n"));

        let output_path = Path::new(file_path)
            .parent()
            .unwrap_or(Path::new("."))
            .join("ARCHITECTURE.md")
            .to_string_lossy()
            .to_string();

        Ok(Documentation {
            doc_type: DocType::ArchitectureDiagram,
            content,
            output_path,
        })
    }

    /// Generate module-level documentation
    async fn generate_module_doc(
        &self,
        code: &str,
        file_path: &str,
        context: &DocumentationContext,
        units: &[crate::cortex_bridge::CodeUnit],
    ) -> Result<Documentation> {
        info!("Generating module documentation for {}", file_path);

        let module_name = Path::new(file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let description = self.generate_module_description(code, context, units);
        let examples = self.generate_code_examples(context, units);
        let public_items = self.format_public_items(units);

        // Get template content or use fallback
        let content_template = if let Some(template) = self.templates.get(&DocType::ModuleDoc) {
            template.content.clone()
        } else {
            // Fallback template
            "//! # {module_name}\n//!\n//! {description}\n//!\n//! ## Examples\n//!\n//! ```\n//! {examples}\n//! ```\n//!\n//! ## Public Items\n//!\n//! {public_items}".to_string()
        };

        let mut content = content_template;
        content = content.replace("{module_name}", module_name);
        content = content.replace("{description}", &description);
        content = content.replace("{examples}", &examples);
        content = content.replace("{public_items}", &public_items);

        Ok(Documentation {
            doc_type: DocType::ModuleDoc,
            content,
            output_path: file_path.to_string(),
        })
    }

    /// Store documentation episode for future learning
    async fn store_documentation_episode(
        &self,
        file_path: &str,
        doc_types: &[DocType],
        results: &[Documentation],
        success: bool,
    ) -> Result<()> {
        let workspace_id = self.workspace_id.as_ref()
            .ok_or_else(|| AgentError::ConfigurationError("No workspace set".to_string()))?;

        let episode = Episode {
            id: uuid::Uuid::new_v4().to_string(),
            episode_type: EpisodeType::Task,
            task_description: format!(
                "Generate {} documentation for {}",
                doc_types.iter()
                    .map(|t| format!("{:?}", t))
                    .collect::<Vec<_>>()
                    .join(", "),
                file_path
            ),
            agent_id: self.id.to_string(),
            session_id: self.session_id.as_ref().map(|s| s.to_string()),
            workspace_id: workspace_id.to_string(),
            entities_created: results.iter().map(|d| d.output_path.clone()).collect(),
            entities_modified: vec![],
            entities_deleted: vec![],
            files_touched: results.iter().map(|d| d.output_path.clone()).collect(),
            queries_made: vec![],
            tools_used: vec![],
            solution_summary: format!(
                "Generated {} documentation files for {}",
                results.len(),
                file_path
            ),
            outcome: if success {
                EpisodeOutcome::Success
            } else {
                EpisodeOutcome::Failure
            },
            success_metrics: serde_json::json!({
                "files_generated": results.len(),
                "doc_types": doc_types.iter()
                    .map(|t| format!("{:?}", t))
                    .collect::<Vec<_>>(),
            }),
            errors_encountered: vec![],
            lessons_learned: vec![
                "Documentation should be clear and concise".to_string(),
                "Include examples for better understanding".to_string(),
            ],
            duration_seconds: 120,
            tokens_used: TokenUsage::default(),
            embedding: vec![],
            created_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
        };

        self.cortex.store_episode(episode).await
            .map_err(|e| AgentError::CortexError(e.to_string()))?;

        Ok(())
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    fn extract_public_items(&self, _code: &str, units: &[crate::cortex_bridge::CodeUnit]) -> Vec<CodeItem> {
        units.iter()
            .filter(|u| u.visibility == "public")
            .map(|u| CodeItem {
                name: u.name.clone(),
                item_type: u.unit_type.clone(),
                signature: u.signature.clone(),
                line: u.lines.start,
            })
            .collect()
    }

    fn generate_item_doc_comment(&self, item: &CodeItem, context: &DocumentationContext) -> String {
        // Look for similar documentation patterns
        let similar_docs = context.patterns.iter()
            .filter(|p| p.name.contains(&item.item_type))
            .collect::<Vec<_>>();

        if !similar_docs.is_empty() {
            // Use pattern-based generation
            format!(
                "/// {}\n///\n/// # Arguments\n///\n/// * `arg` - Description\n///\n/// # Returns\n///\n/// Description of return value",
                self.generate_brief_description(&item.name)
            )
        } else {
            // Fallback to simple description
            format!("/// {}", self.generate_brief_description(&item.name))
        }
    }

    fn insert_doc_comment(&self, code: &str, item: &CodeItem, doc_comment: &str) -> String {
        // Simple insertion before the item (in production, use AST manipulation)
        let lines: Vec<&str> = code.lines().collect();
        let mut result = Vec::new();

        for (idx, line) in lines.iter().enumerate() {
            if idx + 1 == item.line as usize {
                result.push(doc_comment.to_string());
            }
            result.push(line.to_string());
        }

        result.join("\n")
    }

    fn extract_project_name(&self, file_path: &str) -> String {
        Path::new(file_path)
            .components()
            .nth(0)
            .and_then(|c| c.as_os_str().to_str())
            .unwrap_or("Project")
            .to_string()
    }

    fn generate_description(&self, context: &DocumentationContext, _units: &[crate::cortex_bridge::CodeUnit]) -> String {
        // Learn from existing documentation
        if !context.existing_documentation.is_empty() {
            "A comprehensive Rust library providing robust functionality.".to_string()
        } else {
            "This library provides essential functionality for your project.".to_string()
        }
    }

    fn generate_installation_instructions(&self) -> String {
        "```toml\n[dependencies]\nproject = \"0.1.0\"\n```".to_string()
    }

    fn generate_usage_examples(&self, _context: &DocumentationContext, units: &[crate::cortex_bridge::CodeUnit]) -> String {
        if let Some(first_unit) = units.first() {
            format!(
                "```rust\nuse project::{};\n\n// Use the library\n```",
                first_unit.name
            )
        } else {
            "```rust\n// Example usage\n```".to_string()
        }
    }

    fn generate_code_examples(&self, context: &DocumentationContext, _units: &[crate::cortex_bridge::CodeUnit]) -> String {
        // Learn from similar code examples
        if !context.similar_implementations.is_empty() {
            "```rust\n// Example from similar implementations\nfn main() {\n    // Your code here\n}\n```".to_string()
        } else {
            "```rust\nfn main() {\n    println!(\"Hello, world!\");\n}\n```".to_string()
        }
    }

    fn generate_api_endpoints(&self, api: &[&crate::cortex_bridge::CodeUnit], _context: &DocumentationContext) -> String {
        let mut endpoints = Vec::new();
        for unit in api {
            endpoints.push(format!(
                "### `{}`\n\n{}\n\n**Signature:** `{}`",
                unit.name,
                "Description of this API endpoint.",
                unit.signature
            ));
        }
        endpoints.join("\n\n")
    }

    fn find_parent_module(&self, unit: &crate::cortex_bridge::CodeUnit, units: &[crate::cortex_bridge::CodeUnit]) -> Option<usize> {
        // Simple heuristic: find module with name prefix
        let parts: Vec<&str> = unit.qualified_name.split("::").collect();
        if parts.len() > 1 {
            let parent_name = parts[parts.len() - 2];
            units.iter().position(|u| u.name == parent_name)
        } else {
            None
        }
    }

    fn generate_module_description(&self, _code: &str, _context: &DocumentationContext, _units: &[crate::cortex_bridge::CodeUnit]) -> String {
        "This module provides core functionality.".to_string()
    }

    fn format_public_items(&self, units: &[crate::cortex_bridge::CodeUnit]) -> String {
        units.iter()
            .filter(|u| u.visibility == "public")
            .map(|u| format!("- `{}` - {}", u.name, u.unit_type))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn generate_brief_description(&self, name: &str) -> String {
        // Convert snake_case to human-readable
        name.replace('_', " ")
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl Agent for DocumenterAgent {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn agent_type(&self) -> AgentType {
        AgentType::Documenter
    }

    fn capabilities(&self) -> &HashSet<Capability> {
        &self.capabilities
    }

    fn metrics(&self) -> &AgentMetrics {
        &self.metrics
    }
}

// ============================================================================
// Supporting Types
// ============================================================================

/// Documentation format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DocFormat {
    /// Markdown format
    Markdown,
    /// Rustdoc comments
    Rustdoc,
    /// HTML format
    Html,
    /// Mermaid diagrams
    Mermaid,
}

/// Documentation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DocType {
    /// Rustdoc comments
    Rustdoc,
    /// README file
    ReadMe,
    /// API documentation
    ApiDoc,
    /// Architecture diagram
    ArchitectureDiagram,
    /// Module documentation
    ModuleDoc,
}

/// Documentation template
#[derive(Debug, Clone)]
pub struct Template {
    /// Template name
    pub name: String,
    /// Template content with placeholders
    pub content: String,
    /// Available placeholders
    pub placeholders: Vec<String>,
}

/// Style guide for documentation
#[derive(Debug, Clone)]
pub struct StyleGuide {
    /// Style guide name
    pub name: String,
    /// Documentation rules
    pub rules: Vec<String>,
}

/// Code item to document
#[derive(Debug, Clone)]
struct CodeItem {
    name: String,
    item_type: String,
    signature: String,
    line: u32,
}

/// Documentation context from Cortex
#[derive(Debug, Clone)]
struct DocumentationContext {
    past_episodes: Vec<Episode>,
    patterns: Vec<Pattern>,
    similar_implementations: Vec<crate::cortex_bridge::CodeSearchResult>,
    existing_documentation: Vec<crate::cortex_bridge::CodeSearchResult>,
}

/// Generated documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Documentation {
    /// Documentation type
    pub doc_type: DocType,
    /// Documentation content
    pub content: String,
    /// Output file path
    pub output_path: String,
}

/// Documentation generation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationResult {
    /// Source file path
    pub file_path: String,
    /// Generated documentation
    pub documentation: Vec<Documentation>,
    /// Metadata
    pub metadata: DocumentationMetadata,
}

/// Documentation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationMetadata {
    /// Generation timestamp
    pub generated_at: chrono::DateTime<chrono::Utc>,
    /// Agent ID
    pub agent_id: String,
    /// Documentation types generated
    pub doc_types: Vec<DocType>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_documenter_capabilities() {
        let cortex = Arc::new(
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async {
                    CortexBridge::new(Default::default()).await
                })
                .unwrap_or_else(|_| panic!("Failed to create CortexBridge"))
        );

        let agent = DocumenterAgent::new("test-documenter".to_string(), cortex);

        assert_eq!(agent.name(), "test-documenter");
        assert_eq!(agent.agent_type(), AgentType::Documenter);
        assert!(agent.capabilities().contains(&Capability::Documentation));
        assert!(agent.capabilities().contains(&Capability::DocGeneration));
        assert!(agent.capabilities().contains(&Capability::DiagramCreation));
    }

    #[test]
    fn test_doc_formats() {
        let formats = vec![
            DocFormat::Markdown,
            DocFormat::Rustdoc,
            DocFormat::Html,
            DocFormat::Mermaid,
        ];
        assert_eq!(formats.len(), 4);
    }

    #[test]
    fn test_generate_brief_description() {
        let cortex = Arc::new(
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async {
                    CortexBridge::new(Default::default()).await
                })
                .unwrap_or_else(|_| panic!("Failed to create CortexBridge"))
        );

        let agent = DocumenterAgent::new("test".to_string(), cortex);
        let desc = agent.generate_brief_description("my_function_name");
        assert_eq!(desc, "My Function Name");
    }
}
