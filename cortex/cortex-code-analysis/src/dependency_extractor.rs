//! Dependency extraction for building code graphs.
//!
//! This module extracts various types of dependencies from parsed code:
//! - Function calls (CALLS relationship)
//! - Type usage (USES_TYPE relationship)
//! - Inheritance (INHERITS relationship)
//! - Trait implementations (IMPLEMENTS relationship)
//! - Import statements (IMPORTS relationship)
//!
//! # Example
//!
//! ```no_run
//! use cortex_code_analysis::{RustParser, DependencyExtractor};
//!
//! # fn main() -> anyhow::Result<()> {
//! let source = r#"
//! use std::collections::HashMap;
//!
//! struct MyStruct {
//!     data: HashMap<String, i32>,
//! }
//!
//! impl MyStruct {
//!     fn process(&self) {
//!         println!("Processing");
//!     }
//! }
//! "#;
//!
//! let mut parser = RustParser::new()?;
//! let parsed = parser.parse_file("example.rs", source)?;
//!
//! let mut extractor = DependencyExtractor::new()?;
//! let deps = extractor.extract_all(&parsed, source)?;
//!
//! println!("Found {} dependencies", deps.len());
//! # Ok(())
//! # }
//! ```

use crate::types::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tree_sitter::{Node, Parser};

/// Location information for a dependency.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Location {
    /// File path
    pub file: String,
    /// Starting line (1-indexed)
    pub start_line: usize,
    /// Ending line (1-indexed)
    pub end_line: usize,
    /// Starting column (0-indexed)
    pub start_column: usize,
    /// Ending column (0-indexed)
    pub end_column: usize,
}

impl Location {
    pub fn from_node(node: Node, file: &str) -> Self {
        Self {
            file: file.to_string(),
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
            start_column: node.start_position().column,
            end_column: node.end_position().column,
        }
    }
}

/// Type of dependency relationship.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DependencyType {
    /// Function calls another function
    Calls,
    /// Code uses a type
    UsesType,
    /// Type inherits from another (or implements trait)
    Inherits,
    /// Type implements a trait
    Implements,
    /// Module imports another module/item
    Imports,
}

impl std::fmt::Display for DependencyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DependencyType::Calls => write!(f, "CALLS"),
            DependencyType::UsesType => write!(f, "USES_TYPE"),
            DependencyType::Inherits => write!(f, "INHERITS"),
            DependencyType::Implements => write!(f, "IMPLEMENTS"),
            DependencyType::Imports => write!(f, "IMPORTS"),
        }
    }
}

/// Represents a dependency between code units.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Dependency {
    /// Source code unit (fully qualified name)
    pub from_unit: String,
    /// Target code unit (fully qualified name or external module)
    pub to_unit: String,
    /// Type of dependency
    pub dep_type: DependencyType,
    /// Location where dependency occurs
    pub location: Location,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl Dependency {
    pub fn new(
        from_unit: String,
        to_unit: String,
        dep_type: DependencyType,
        location: Location,
    ) -> Self {
        Self {
            from_unit,
            to_unit,
            dep_type,
            location,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Import statement information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Import {
    /// Module or crate being imported from
    pub module: String,
    /// Specific items imported (empty for wildcard imports)
    pub items: Vec<String>,
    /// Whether it's a glob import (use foo::*)
    pub is_glob: bool,
    /// Location of import
    pub location: Location,
}

/// Main dependency extraction engine.
pub struct DependencyExtractor {
    /// Parser for re-parsing function bodies
    parser: Parser,
}

impl DependencyExtractor {
    /// Create a new dependency extractor.
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into())?;
        Ok(Self { parser })
    }

    /// Extract all dependencies from a parsed file.
    pub fn extract_all(&mut self, parsed: &ParsedFile, source: &str) -> Result<Vec<Dependency>> {
        let mut dependencies = Vec::new();

        // Extract import dependencies
        dependencies.extend(self.extract_import_dependencies(parsed, source)?);

        // Extract function dependencies
        for func in &parsed.functions {
            dependencies.extend(self.extract_from_function(func, source)?);
        }

        // Extract struct dependencies
        for struct_info in &parsed.structs {
            dependencies.extend(self.extract_from_struct(struct_info)?);
        }

        // Extract enum dependencies
        for enum_info in &parsed.enums {
            dependencies.extend(self.extract_from_enum(enum_info)?);
        }

        // Extract trait dependencies
        for trait_info in &parsed.traits {
            dependencies.extend(self.extract_from_trait(trait_info, source)?);
        }

        // Extract impl dependencies
        for impl_info in &parsed.impls {
            dependencies.extend(self.extract_from_impl(impl_info, source)?);
        }

        Ok(dependencies)
    }

    /// Extract dependencies from function body.
    pub fn extract_from_function(
        &mut self,
        function: &FunctionInfo,
        _source: &str,
    ) -> Result<Vec<Dependency>> {
        let mut dependencies = Vec::new();

        // Parse the function body
        if function.body.is_empty() {
            return Ok(dependencies);
        }

        let tree = self.parser.parse(&function.body, None);
        if tree.is_none() {
            return Ok(dependencies);
        }
        let tree = tree.unwrap();
        let root = tree.root_node();

        // Extract function calls
        dependencies.extend(self.extract_function_calls(
            root,
            &function.body,
            &function.qualified_name,
            &Location {
                file: "".to_string(),
                start_line: function.start_line,
                end_line: function.end_line,
                start_column: 0,
                end_column: 0,
            },
        )?);

        // Extract type usage from function body
        dependencies.extend(self.extract_type_usage(
            root,
            &function.body,
            &function.qualified_name,
            &Location {
                file: "".to_string(),
                start_line: function.start_line,
                end_line: function.end_line,
                start_column: 0,
                end_column: 0,
            },
        )?);

        // Extract type usage from parameters
        for param in &function.parameters {
            if !param.is_self && !param.param_type.is_empty() {
                let type_name = self.extract_type_name(&param.param_type);
                if !self.is_primitive_type(&type_name) {
                    dependencies.push(Dependency::new(
                        function.qualified_name.clone(),
                        type_name,
                        DependencyType::UsesType,
                        Location {
                            file: "".to_string(),
                            start_line: function.start_line,
                            end_line: function.start_line,
                            start_column: 0,
                            end_column: 0,
                        },
                    ));
                }
            }
        }

        // Extract type usage from return type
        if let Some(return_type) = &function.return_type {
            let type_name = self.extract_type_name(return_type);
            if !self.is_primitive_type(&type_name) {
                dependencies.push(Dependency::new(
                    function.qualified_name.clone(),
                    type_name,
                    DependencyType::UsesType,
                    Location {
                        file: "".to_string(),
                        start_line: function.start_line,
                        end_line: function.start_line,
                        start_column: 0,
                        end_column: 0,
                    },
                ));
            }
        }

        Ok(dependencies)
    }

    /// Extract dependencies from struct.
    fn extract_from_struct(&self, struct_info: &StructInfo) -> Result<Vec<Dependency>> {
        let mut dependencies = Vec::new();

        // Extract type usage from fields
        for field in &struct_info.fields {
            let type_name = self.extract_type_name(&field.field_type);
            if !self.is_primitive_type(&type_name) {
                dependencies.push(Dependency::new(
                    struct_info.qualified_name.clone(),
                    type_name,
                    DependencyType::UsesType,
                    Location {
                        file: "".to_string(),
                        start_line: struct_info.start_line,
                        end_line: struct_info.end_line,
                        start_column: 0,
                        end_column: 0,
                    },
                ));
            }
        }

        Ok(dependencies)
    }

    /// Extract dependencies from enum.
    fn extract_from_enum(&self, enum_info: &EnumInfo) -> Result<Vec<Dependency>> {
        let mut dependencies = Vec::new();

        // Extract type usage from variant fields
        for variant in &enum_info.variants {
            for field in &variant.fields {
                let type_name = self.extract_type_name(&field.field_type);
                if !self.is_primitive_type(&type_name) {
                    dependencies.push(Dependency::new(
                        enum_info.qualified_name.clone(),
                        type_name,
                        DependencyType::UsesType,
                        Location {
                            file: "".to_string(),
                            start_line: enum_info.start_line,
                            end_line: enum_info.end_line,
                            start_column: 0,
                            end_column: 0,
                        },
                    ));
                }
            }

            // Extract from tuple fields
            for tuple_field in &variant.tuple_fields {
                let type_name = self.extract_type_name(tuple_field);
                if !self.is_primitive_type(&type_name) {
                    dependencies.push(Dependency::new(
                        enum_info.qualified_name.clone(),
                        type_name,
                        DependencyType::UsesType,
                        Location {
                            file: "".to_string(),
                            start_line: enum_info.start_line,
                            end_line: enum_info.end_line,
                            start_column: 0,
                            end_column: 0,
                        },
                    ));
                }
            }
        }

        Ok(dependencies)
    }

    /// Extract dependencies from trait.
    fn extract_from_trait(
        &mut self,
        trait_info: &TraitInfo,
        source: &str,
    ) -> Result<Vec<Dependency>> {
        let mut dependencies = Vec::new();

        // Extract supertrait dependencies
        for supertrait in &trait_info.supertraits {
            dependencies.push(Dependency::new(
                trait_info.qualified_name.clone(),
                supertrait.clone(),
                DependencyType::Inherits,
                Location {
                    file: "".to_string(),
                    start_line: trait_info.start_line,
                    end_line: trait_info.start_line,
                    start_column: 0,
                    end_column: 0,
                },
            ));
        }

        // Extract from trait methods
        for method in &trait_info.methods {
            dependencies.extend(self.extract_from_function(method, source)?);
        }

        Ok(dependencies)
    }

    /// Extract dependencies from impl block.
    fn extract_from_impl(&mut self, impl_info: &ImplInfo, source: &str) -> Result<Vec<Dependency>> {
        let mut dependencies = Vec::new();

        // Extract trait implementation dependency
        if let Some(trait_name) = &impl_info.trait_name {
            dependencies.push(Dependency::new(
                impl_info.type_name.clone(),
                trait_name.clone(),
                DependencyType::Implements,
                Location {
                    file: "".to_string(),
                    start_line: impl_info.start_line,
                    end_line: impl_info.start_line,
                    start_column: 0,
                    end_column: 0,
                },
            ));
        }

        // Extract from impl methods
        for method in &impl_info.methods {
            dependencies.extend(self.extract_from_function(method, source)?);
        }

        Ok(dependencies)
    }

    /// Extract import statements.
    pub fn extract_imports(&self, parsed: &ParsedFile, _source: &str) -> Result<Vec<Import>> {
        let mut imports = Vec::new();

        for import_str in &parsed.imports {
            if let Some(import) = self.parse_import_statement(import_str) {
                imports.push(import);
            }
        }

        Ok(imports)
    }

    /// Extract import dependencies.
    fn extract_import_dependencies(
        &self,
        parsed: &ParsedFile,
        _source: &str,
    ) -> Result<Vec<Dependency>> {
        let mut dependencies = Vec::new();
        let file_path = &parsed.path;

        for import_str in &parsed.imports {
            if let Some(import) = self.parse_import_statement(import_str) {
                // Create dependency for each imported item
                if import.items.is_empty() || import.is_glob {
                    // Whole module import
                    dependencies.push(Dependency::new(
                        file_path.clone(),
                        import.module.clone(),
                        DependencyType::Imports,
                        import.location.clone(),
                    ));
                } else {
                    // Individual item imports
                    for item in &import.items {
                        let full_path = if import.module.is_empty() {
                            item.clone()
                        } else {
                            format!("{}::{}", import.module, item)
                        };
                        dependencies.push(Dependency::new(
                            file_path.clone(),
                            full_path,
                            DependencyType::Imports,
                            import.location.clone(),
                        ));
                    }
                }
            }
        }

        Ok(dependencies)
    }

    /// Parse a use statement into an Import.
    fn parse_import_statement(&self, import_str: &str) -> Option<Import> {
        let trimmed = import_str.trim();
        if !trimmed.starts_with("use ") {
            return None;
        }

        let use_clause = trimmed.strip_prefix("use ")?.trim_end_matches(';').trim();

        let is_glob = use_clause.ends_with("::*");
        let mut module = String::new();
        let mut items = Vec::new();

        if is_glob {
            module = use_clause.strip_suffix("::*")?.to_string();
        } else if use_clause.contains('{') {
            // use foo::{bar, baz}
            if let Some(pos) = use_clause.rfind("::") {
                module = use_clause[..pos].to_string();
                if let Some(start) = use_clause.find('{') {
                    if let Some(end) = use_clause.find('}') {
                        let items_str = &use_clause[start + 1..end];
                        items = items_str
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                }
            }
        } else {
            // Simple import: use foo::bar::Baz
            if let Some(pos) = use_clause.rfind("::") {
                module = use_clause[..pos].to_string();
                items.push(use_clause[pos + 2..].to_string());
            } else {
                module = use_clause.to_string();
            }
        }

        Some(Import {
            module,
            items,
            is_glob,
            location: Location {
                file: "".to_string(),
                start_line: 0,
                end_line: 0,
                start_column: 0,
                end_column: 0,
            },
        })
    }

    /// Extract function calls from AST.
    fn extract_function_calls(
        &self,
        node: Node,
        source: &str,
        from_unit: &str,
        base_location: &Location,
    ) -> Result<Vec<Dependency>> {
        let mut dependencies = Vec::new();
        fn visit_node(
            node: Node,
            source: &str,
            from_unit: &str,
            base_location: &Location,
            dependencies: &mut Vec<Dependency>,
        ) {
            if node.kind() == "call_expression" {
                if let Some(function_node) = node.child_by_field_name("function") {
                    let function_name = extract_function_name(function_node, source);
                    if !function_name.is_empty() {
                        let mut location = base_location.clone();
                        location.start_line = node.start_position().row + 1;
                        location.end_line = node.end_position().row + 1;

                        dependencies.push(Dependency::new(
                            from_unit.to_string(),
                            function_name,
                            DependencyType::Calls,
                            location,
                        ));
                    }
                }
            }

            // Recursively visit children
            let mut child_cursor = node.walk();
            for child in node.children(&mut child_cursor) {
                visit_node(child, source, from_unit, base_location, dependencies);
            }
        }

        visit_node(node, source, from_unit, base_location, &mut dependencies);
        Ok(dependencies)
    }

    /// Extract type usage from AST.
    fn extract_type_usage(
        &self,
        node: Node,
        source: &str,
        from_unit: &str,
        base_location: &Location,
    ) -> Result<Vec<Dependency>> {
        let mut dependencies = Vec::new();
        let mut seen_types = HashSet::new();

        fn visit_node(
            node: Node,
            source: &str,
            from_unit: &str,
            base_location: &Location,
            dependencies: &mut Vec<Dependency>,
            seen_types: &mut HashSet<String>,
            extractor: &DependencyExtractor,
        ) {
            match node.kind() {
                "type_identifier" | "generic_type" | "reference_type" => {
                    let type_name = node.utf8_text(source.as_bytes()).unwrap_or("");
                    let clean_type = extractor.extract_type_name(type_name);

                    if !clean_type.is_empty()
                        && !extractor.is_primitive_type(&clean_type)
                        && !seen_types.contains(&clean_type)
                    {
                        seen_types.insert(clean_type.clone());
                        let mut location = base_location.clone();
                        location.start_line = node.start_position().row + 1;
                        location.end_line = node.end_position().row + 1;

                        dependencies.push(Dependency::new(
                            from_unit.to_string(),
                            clean_type,
                            DependencyType::UsesType,
                            location,
                        ));
                    }
                }
                _ => {}
            }

            // Recursively visit children
            let mut child_cursor = node.walk();
            for child in node.children(&mut child_cursor) {
                visit_node(
                    child,
                    source,
                    from_unit,
                    base_location,
                    dependencies,
                    seen_types,
                    extractor,
                );
            }
        }

        visit_node(
            node,
            source,
            from_unit,
            base_location,
            &mut dependencies,
            &mut seen_types,
            self,
        );
        Ok(dependencies)
    }

    /// Extract the base type name from a complex type string.
    fn extract_type_name(&self, type_str: &str) -> String {
        // Remove common type wrappers and extract the core type
        let mut clean = type_str.trim();

        // Remove reference markers
        clean = clean.trim_start_matches('&').trim();
        clean = clean.trim_start_matches("mut ").trim();

        // Remove -> for return types
        clean = clean.trim_start_matches("->").trim();

        // Extract from generic types (e.g., "Vec<String>" -> "Vec")
        if let Some(pos) = clean.find('<') {
            clean = &clean[..pos];
        }

        // Extract from array types (e.g., "[u8; 32]" -> array, but we'll skip primitives)
        if clean.starts_with('[') {
            if let Some(pos) = clean.find(';') {
                clean = &clean[1..pos].trim();
            } else if let Some(pos) = clean.find(']') {
                clean = &clean[1..pos].trim();
            }
        }

        clean.to_string()
    }

    /// Check if a type is a primitive type that shouldn't be tracked.
    fn is_primitive_type(&self, type_name: &str) -> bool {
        matches!(
            type_name,
            "i8" | "i16"
                | "i32"
                | "i64"
                | "i128"
                | "isize"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "u128"
                | "usize"
                | "f32"
                | "f64"
                | "bool"
                | "char"
                | "str"
                | "()"
                | ""
        )
    }

    /// Build dependency graph from parsed file.
    pub fn build_dependency_graph(&mut self, parsed: &ParsedFile, source: &str) -> Result<DependencyGraph> {
        let dependencies = self.extract_all(parsed, source)?;
        Ok(DependencyGraph::from_dependencies(dependencies))
    }
}

/// Helper function to extract function name from call expression.
fn extract_function_name(node: Node, source: &str) -> String {
    match node.kind() {
        "identifier" => node.utf8_text(source.as_bytes()).unwrap_or("").to_string(),
        "field_expression" => {
            // For method calls like obj.method()
            if let Some(field) = node.child_by_field_name("field") {
                field.utf8_text(source.as_bytes()).unwrap_or("").to_string()
            } else {
                node.utf8_text(source.as_bytes()).unwrap_or("").to_string()
            }
        }
        "scoped_identifier" => {
            // For calls like Module::function()
            node.utf8_text(source.as_bytes()).unwrap_or("").to_string()
        }
        _ => node.utf8_text(source.as_bytes()).unwrap_or("").to_string(),
    }
}

/// Dependency graph representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    /// All nodes in the graph (code units)
    pub nodes: HashSet<String>,
    /// All edges in the graph
    pub edges: Vec<Dependency>,
    /// Adjacency list for fast traversal
    pub adjacency: HashMap<String, Vec<String>>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph.
    pub fn new() -> Self {
        Self {
            nodes: HashSet::new(),
            edges: Vec::new(),
            adjacency: HashMap::new(),
        }
    }

    /// Build graph from dependencies.
    pub fn from_dependencies(dependencies: Vec<Dependency>) -> Self {
        let mut graph = Self::new();

        for dep in dependencies {
            graph.nodes.insert(dep.from_unit.clone());
            graph.nodes.insert(dep.to_unit.clone());

            graph
                .adjacency
                .entry(dep.from_unit.clone())
                .or_insert_with(Vec::new)
                .push(dep.to_unit.clone());

            graph.edges.push(dep);
        }

        graph
    }

    /// Get all dependencies from a given unit.
    pub fn get_dependencies(&self, unit: &str) -> Vec<&Dependency> {
        self.edges
            .iter()
            .filter(|dep| dep.from_unit == unit)
            .collect()
    }

    /// Get all dependents of a given unit (reverse dependencies).
    pub fn get_dependents(&self, unit: &str) -> Vec<&Dependency> {
        self.edges
            .iter()
            .filter(|dep| dep.to_unit == unit)
            .collect()
    }

    /// Get statistics about the graph.
    pub fn stats(&self) -> GraphStats {
        let mut by_type = HashMap::new();
        for dep in &self.edges {
            *by_type.entry(dep.dep_type).or_insert(0) += 1;
        }

        GraphStats {
            total_nodes: self.nodes.len(),
            total_edges: self.edges.len(),
            edges_by_type: by_type,
        }
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about a dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub edges_by_type: HashMap<DependencyType, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RustParser;

    #[test]
    fn test_extract_function_calls() -> Result<()> {
        let source = r#"
fn main() {
    println!("Hello");
    process_data();
}

fn process_data() {
    helper_function();
}

fn helper_function() {}
"#;

        let mut parser = RustParser::new()?;
        let parsed = parser.parse_file("test.rs", source)?;

        let mut extractor = DependencyExtractor::new()?;
        let deps = extractor.extract_all(&parsed, source)?;

        // Should find calls from main to println and process_data
        let main_calls: Vec<_> = deps
            .iter()
            .filter(|d| d.from_unit == "main" && d.dep_type == DependencyType::Calls)
            .collect();

        assert!(!main_calls.is_empty(), "Should find function calls from main");

        Ok(())
    }

    #[test]
    fn test_extract_type_usage() -> Result<()> {
        let source = r#"
use std::collections::HashMap;

struct MyStruct {
    data: HashMap<String, i32>,
    count: usize,
}

impl MyStruct {
    fn new() -> Self {
        MyStruct {
            data: HashMap::new(),
            count: 0,
        }
    }
}
"#;

        let mut parser = RustParser::new()?;
        let parsed = parser.parse_file("test.rs", source)?;

        let mut extractor = DependencyExtractor::new()?;
        let deps = extractor.extract_all(&parsed, source)?;

        // Should find HashMap usage
        let type_deps: Vec<_> = deps
            .iter()
            .filter(|d| d.dep_type == DependencyType::UsesType)
            .collect();

        assert!(!type_deps.is_empty(), "Should find type usage");

        // Verify HashMap is found
        let has_hashmap = type_deps
            .iter()
            .any(|d| d.to_unit.contains("HashMap"));
        assert!(has_hashmap, "Should find HashMap dependency");

        Ok(())
    }

    #[test]
    fn test_extract_imports() -> Result<()> {
        let source = r#"
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::*;
"#;

        let mut parser = RustParser::new()?;
        let parsed = parser.parse_file("test.rs", source)?;

        let extractor = DependencyExtractor::new()?;
        let imports = extractor.extract_imports(&parsed, source)?;

        assert_eq!(imports.len(), 3, "Should find 3 import statements");

        // Check glob import
        let glob_import = imports.iter().find(|i| i.is_glob);
        assert!(glob_import.is_some(), "Should find glob import");

        Ok(())
    }

    #[test]
    fn test_extract_trait_implementation() -> Result<()> {
        let source = r#"
trait MyTrait {
    fn do_something(&self);
}

struct MyStruct;

impl MyTrait for MyStruct {
    fn do_something(&self) {}
}
"#;

        let mut parser = RustParser::new()?;
        let parsed = parser.parse_file("test.rs", source)?;

        let mut extractor = DependencyExtractor::new()?;
        let deps = extractor.extract_all(&parsed, source)?;

        // Should find IMPLEMENTS relationship
        let impl_deps: Vec<_> = deps
            .iter()
            .filter(|d| d.dep_type == DependencyType::Implements)
            .collect();

        assert!(!impl_deps.is_empty(), "Should find trait implementation");

        Ok(())
    }

    #[test]
    fn test_dependency_graph() -> Result<()> {
        let source = r#"
fn main() {
    process();
}

fn process() {
    helper();
}

fn helper() {}
"#;

        let mut parser = RustParser::new()?;
        let parsed = parser.parse_file("test.rs", source)?;

        let mut extractor = DependencyExtractor::new()?;
        let graph = extractor.build_dependency_graph(&parsed, source)?;

        let stats = graph.stats();
        assert!(stats.total_nodes > 0, "Graph should have nodes");
        assert!(stats.total_edges > 0, "Graph should have edges");

        Ok(())
    }

    #[test]
    fn test_parse_import_statement() -> Result<()> {
        let extractor = DependencyExtractor::new()?;

        // Simple import
        let import1 = extractor.parse_import_statement("use std::collections::HashMap;");
        assert!(import1.is_some());
        let import1 = import1.unwrap();
        assert_eq!(import1.module, "std::collections");
        assert_eq!(import1.items, vec!["HashMap"]);
        assert!(!import1.is_glob);

        // Multiple items
        let import2 = extractor.parse_import_statement("use std::fs::{File, OpenOptions};");
        assert!(import2.is_some());
        let import2 = import2.unwrap();
        assert_eq!(import2.module, "std::fs");
        assert_eq!(import2.items.len(), 2);

        // Glob import
        let import3 = extractor.parse_import_statement("use std::io::*;");
        assert!(import3.is_some());
        let import3 = import3.unwrap();
        assert_eq!(import3.module, "std::io");
        assert!(import3.is_glob);

        Ok(())
    }

    #[test]
    fn test_extract_type_name() {
        let extractor = DependencyExtractor::new().unwrap();

        assert_eq!(extractor.extract_type_name("Vec<String>"), "Vec");
        assert_eq!(extractor.extract_type_name("&str"), "str");
        assert_eq!(extractor.extract_type_name("&mut HashMap<K, V>"), "HashMap");
        assert_eq!(extractor.extract_type_name("-> Result<()>"), "Result");
        assert_eq!(extractor.extract_type_name("[u8; 32]"), "u8");
    }

    #[test]
    fn test_is_primitive_type() {
        let extractor = DependencyExtractor::new().unwrap();

        assert!(extractor.is_primitive_type("i32"));
        assert!(extractor.is_primitive_type("u64"));
        assert!(extractor.is_primitive_type("bool"));
        assert!(extractor.is_primitive_type("char"));
        assert!(!extractor.is_primitive_type("String"));
        assert!(!extractor.is_primitive_type("Vec"));
        assert!(!extractor.is_primitive_type("HashMap"));
    }
}
