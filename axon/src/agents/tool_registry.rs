//! Tool Registry for Specialized Agents
//!
//! This module defines which MCP tools are available to each agent type,
//! reducing context pollution by only exposing relevant tools.

use std::collections::{HashMap, HashSet};
use crate::agents::AgentType;

/// Tool registry that maps agent types to their allowed tools
#[derive(Debug, Clone)]
pub struct ToolRegistry {
    /// Mapping of agent types to tool names
    tool_sets: HashMap<AgentType, HashSet<String>>,
}

impl ToolRegistry {
    /// Create a new tool registry with default mappings
    pub fn new() -> Self {
        let mut tool_sets = HashMap::new();

        // Developer agent tools - code generation, manipulation, and analysis
        tool_sets.insert(AgentType::Developer, [
            "cortex_code_get_unit",
            "cortex_code_list_units",
            "cortex_code_find_definition",
            "cortex_code_find_references",
            "cortex_code_get_symbols",
            "cortex_code_get_imports",
            "cortex_code_get_exports",
            "cortex_code_get_signature",
            "cortex_code_get_call_hierarchy",
            "cortex_code_get_type_hierarchy",
            "cortex_vfs_create_file",
            "cortex_vfs_update_file",
            "cortex_vfs_get_node",
            "cortex_vfs_list_directory",
            "cortex_semantic_search_code",
            "cortex_semantic_find_by_meaning",
            "cortex_workspace_search",
        ].iter().map(|s| s.to_string()).collect());

        // Reviewer agent tools - code review and quality analysis
        tool_sets.insert(AgentType::Reviewer, [
            "cortex_quality_analyze_complexity",
            "cortex_quality_analyze_cohesion",
            "cortex_quality_analyze_coupling",
            "cortex_quality_check_naming",
            "cortex_quality_find_code_smells",
            "cortex_quality_find_antipatterns",
            "cortex_quality_suggest_refactorings",
            "cortex_quality_calculate_metrics",
            "cortex_security_scan",
            "cortex_security_check_dependencies",
            "cortex_security_analyze_secrets",
            "cortex_ai_review_code",
            "cortex_code_get_unit",
            "cortex_code_list_units",
            "cortex_deps_get_dependencies",
            "cortex_deps_find_cycles",
        ].iter().map(|s| s.to_string()).collect());

        // Tester agent tools - test generation and execution
        tool_sets.insert(AgentType::Tester, [
            "cortex_test_execute",
            "cortex_test_run_in_memory",
            "cortex_test_find_missing",
            "cortex_test_analyze_coverage",
            "cortex_test_analyze_flaky",
            "cortex_test_validate",
            "cortex_test_suggest_edge_cases",
            "cortex_code_get_unit",
            "cortex_code_list_units",
            "cortex_code_find_references",
            "cortex_vfs_create_file",
            "cortex_vfs_update_file",
            "cortex_build_trigger",
            "cortex_build_configure",
        ].iter().map(|s| s.to_string()).collect());

        // Optimizer agent tools - performance optimization
        tool_sets.insert(AgentType::Optimizer, [
            "cortex_ai_suggest_optimization",
            "cortex_quality_analyze_complexity",
            "cortex_quality_calculate_metrics",
            "cortex_deps_find_hubs",
            "cortex_deps_find_cycles",
            "cortex_deps_impact_analysis",
            "cortex_arch_suggest_boundaries",
            "cortex_monitor_performance",
            "cortex_analytics_code_metrics",
            "cortex_analytics_productivity",
            "cortex_code_get_unit",
            "cortex_code_list_units",
            "cortex_code_infer_types",
            "cortex_code_suggest_type_annotations",
        ].iter().map(|s| s.to_string()).collect());

        // Architect agent tools - system design and architecture
        tool_sets.insert(AgentType::Architect, [
            "cortex_arch_visualize",
            "cortex_arch_check_violations",
            "cortex_arch_analyze_drift",
            "cortex_arch_detect_patterns",
            "cortex_arch_suggest_boundaries",
            "cortex_deps_get_layers",
            "cortex_deps_find_roots",
            "cortex_deps_find_leaves",
            "cortex_deps_find_hubs",
            "cortex_deps_generate_graph",
            "cortex_deps_check_constraints",
            "cortex_quality_analyze_coupling",
            "cortex_quality_analyze_cohesion",
            "cortex_workspace_get",
            "cortex_workspace_list",
            "cortex_workspace_compare",
        ].iter().map(|s| s.to_string()).collect());

        // Researcher agent tools - information gathering and analysis
        tool_sets.insert(AgentType::Researcher, [
            "cortex_semantic_search_code",
            "cortex_semantic_search_comments",
            "cortex_semantic_search_documentation",
            "cortex_semantic_find_by_meaning",
            "cortex_semantic_search_by_natural_language",
            "cortex_semantic_search_by_example",
            "cortex_semantic_search_similar",
            "cortex_semantic_hybrid_search",
            "cortex_memory_find_similar_episodes",
            "cortex_memory_search_episodes",
            "cortex_memory_get_recommendations",
            "cortex_document_search",
            "cortex_document_list",
            "cortex_workspace_search",
            "cortex_vfs_search_files",
        ].iter().map(|s| s.to_string()).collect());

        // Documenter agent tools - documentation generation
        tool_sets.insert(AgentType::Documenter, [
            "cortex_ai_explain_code",
            "cortex_document_create",
            "cortex_document_update",
            "cortex_document_section_create",
            "cortex_document_section_update",
            "cortex_document_link_create",
            "cortex_document_publish",
            "cortex_code_get_unit",
            "cortex_code_list_units",
            "cortex_code_get_symbols",
            "cortex_code_get_exports",
            "cortex_code_get_signature",
            "cortex_vfs_create_file",
            "cortex_vfs_update_file",
        ].iter().map(|s| s.to_string()).collect());

        // Orchestrator agent tools - coordination and management
        tool_sets.insert(AgentType::Orchestrator, [
            "cortex_session_create",
            "cortex_session_list",
            "cortex_session_update",
            "cortex_session_merge",
            "cortex_session_abandon",
            "cortex_lock_acquire",
            "cortex_lock_release",
            "cortex_lock_list",
            "cortex_lock_check",
            "cortex_agent_register",
            "cortex_agent_send_message",
            "cortex_agent_get_messages",
            "cortex_conflicts_list",
            "cortex_conflicts_resolve",
            "cortex_workspace_list",
            "cortex_workspace_get",
            "cortex_monitor_health",
            "cortex_analytics_agent_activity",
            "cortex_report_generate",
        ].iter().map(|s| s.to_string()).collect());

        Self { tool_sets }
    }

    /// Get the allowed tools for an agent type
    pub fn get_tools_for_agent(&self, agent_type: &AgentType) -> HashSet<String> {
        self.tool_sets
            .get(agent_type)
            .cloned()
            .unwrap_or_else(|| {
                // Default minimal tool set for unknown agent types
                [
                    "cortex_workspace_list",
                    "cortex_workspace_get",
                    "cortex_vfs_get_node",
                    "cortex_vfs_list_directory",
                ].iter().map(|s| s.to_string()).collect()
            })
    }

    /// Check if a tool is allowed for an agent type
    pub fn is_tool_allowed(&self, agent_type: &AgentType, tool_name: &str) -> bool {
        self.tool_sets
            .get(agent_type)
            .map(|tools| tools.contains(tool_name))
            .unwrap_or(false)
    }

    /// Add a tool to an agent type's allowed set
    pub fn add_tool(&mut self, agent_type: AgentType, tool_name: String) {
        self.tool_sets
            .entry(agent_type)
            .or_insert_with(HashSet::new)
            .insert(tool_name);
    }

    /// Remove a tool from an agent type's allowed set
    pub fn remove_tool(&mut self, agent_type: &AgentType, tool_name: &str) -> bool {
        self.tool_sets
            .get_mut(agent_type)
            .map(|tools| tools.remove(tool_name))
            .unwrap_or(false)
    }

    /// Get statistics about tool distribution
    pub fn get_statistics(&self) -> ToolRegistryStats {
        let total_agents = self.tool_sets.len();
        let total_unique_tools: HashSet<_> = self.tool_sets
            .values()
            .flat_map(|tools| tools.iter())
            .collect();

        let avg_tools_per_agent = if total_agents > 0 {
            self.tool_sets.values().map(|tools| tools.len()).sum::<usize>() as f64
                / total_agents as f64
        } else {
            0.0
        };

        let mut tool_usage: HashMap<String, usize> = HashMap::new();
        for tools in self.tool_sets.values() {
            for tool in tools {
                *tool_usage.entry(tool.clone()).or_insert(0) += 1;
            }
        }

        ToolRegistryStats {
            total_agents,
            total_unique_tools: total_unique_tools.len(),
            avg_tools_per_agent,
            tool_usage,
        }
    }
}

/// Statistics about the tool registry
#[derive(Debug, Clone)]
pub struct ToolRegistryStats {
    /// Total number of agent types configured
    pub total_agents: usize,

    /// Total number of unique tools across all agents
    pub total_unique_tools: usize,

    /// Average number of tools per agent
    pub avg_tools_per_agent: f64,

    /// Usage count for each tool
    pub tool_usage: HashMap<String, usize>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_registry_initialization() {
        let registry = ToolRegistry::new();

        // Check that each agent type has tools
        for agent_type in [
            AgentType::Developer,
            AgentType::Reviewer,
            AgentType::Tester,
            AgentType::Optimizer,
            AgentType::Architect,
            AgentType::Researcher,
            AgentType::Documenter,
            AgentType::Orchestrator,
        ] {
            let tools = registry.get_tools_for_agent(&agent_type);
            assert!(!tools.is_empty(), "{:?} should have tools", agent_type);
        }
    }

    #[test]
    fn test_tool_access_control() {
        let registry = ToolRegistry::new();

        // Developer should have code manipulation tools
        assert!(registry.is_tool_allowed(
            &AgentType::Developer,
            "cortex_code_get_unit"
        ));

        // But not testing tools
        assert!(!registry.is_tool_allowed(
            &AgentType::Developer,
            "cortex_test_execute"
        ));

        // Tester should have testing tools
        assert!(registry.is_tool_allowed(
            &AgentType::Tester,
            "cortex_test_execute"
        ));
    }

    #[test]
    fn test_add_remove_tools() {
        let mut registry = ToolRegistry::new();

        // Add a new tool
        registry.add_tool(AgentType::Developer, "new_tool".to_string());
        assert!(registry.is_tool_allowed(&AgentType::Developer, "new_tool"));

        // Remove the tool
        assert!(registry.remove_tool(&AgentType::Developer, "new_tool"));
        assert!(!registry.is_tool_allowed(&AgentType::Developer, "new_tool"));
    }

    #[test]
    fn test_statistics() {
        let registry = ToolRegistry::new();
        let stats = registry.get_statistics();

        assert_eq!(stats.total_agents, 8);
        assert!(stats.total_unique_tools > 0);
        assert!(stats.avg_tools_per_agent > 0.0);

        // Check that common tools are used by multiple agents
        let code_get_unit_usage = stats.tool_usage.get("cortex_code_get_unit").unwrap_or(&0);
        assert!(*code_get_unit_usage > 1, "Common tools should be used by multiple agents");
    }
}