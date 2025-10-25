//! C/C++ Preprocessor directive extraction and analysis.
//!
//! This module provides functionality to extract and analyze C/C++ preprocessor directives
//! including include statements and macro definitions. It builds dependency graphs to track
//! include relationships and detect cycles.
//!
//! # Examples
//!
//! ```
//! use cortex_code_analysis::{TreeSitterWrapper, preprocessor::{extract_preprocessor, PreprocResults}};
//! use std::path::Path;
//!
//! # fn main() -> anyhow::Result<()> {
//! let source = r#"
//! #include <stdio.h>
//! #include "myheader.h"
//! #define MAX_SIZE 100
//! "#;
//!
//! let mut parser = TreeSitterWrapper::new(tree_sitter_cpp::LANGUAGE.into())?;
//! let tree = parser.parse(source)?;
//! let mut results = PreprocResults::default();
//! extract_preprocessor(&tree, source, Path::new("example.cpp"), &mut results)?;
//!
//! let file_data = results.files.get(Path::new("example.cpp")).unwrap();
//! assert_eq!(file_data.direct_includes.len(), 2);
//! assert_eq!(file_data.macros.len(), 1);
//! # Ok(())
//! # }
//! ```

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::Result;
use petgraph::{
    algo::kosaraju_scc,
    graph::NodeIndex,
    stable_graph::StableGraph,
    visit::Dfs,
    Direction,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use tree_sitter::Tree;

/// Preprocessor data for a single C/C++ file.
///
/// Contains information about include directives and macro definitions found in the file.
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct PreprocFile {
    /// Include directives explicitly written in this file.
    pub direct_includes: HashSet<String>,

    /// Include directives indirectly imported from other files.
    /// Populated after building the dependency graph.
    pub indirect_includes: HashSet<String>,

    /// Macro names defined in this file (excludes standard C/C++ keywords).
    pub macros: HashSet<String>,
}

/// Preprocessor data for multiple C/C++ files.
///
/// Maps file paths to their preprocessor data.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct PreprocResults {
    /// Preprocessor data for each analyzed file.
    pub files: HashMap<PathBuf, PreprocFile>,
}

impl PreprocFile {
    /// Creates a new PreprocFile with the given macro definitions.
    ///
    /// # Examples
    ///
    /// ```
    /// use cortex_code_analysis::preprocessor::PreprocFile;
    ///
    /// let file = PreprocFile::new_macros(&["MAX_SIZE", "MIN_SIZE"]);
    /// assert_eq!(file.macros.len(), 2);
    /// ```
    pub fn new_macros(macros: &[&str]) -> Self {
        let mut pf = Self::default();
        for m in macros {
            pf.macros.insert((*m).to_string());
        }
        pf
    }
}

/// Reserved C/C++ keywords and types that should not be counted as macros.
const SPECIAL_KEYWORDS: &[&str] = &[
    "NULL", "bool", "char", "char16_t", "char32_t", "char8_t",
    "const", "constexpr", "double", "explicit", "false", "float",
    "inline", "int", "int16_t", "int32_t", "int64_t", "int8_t",
    "long", "mutable", "namespace", "nullptr", "restrict", "short",
    "signed", "size_t", "ssize_t", "static", "true",
    "uint16_t", "uint32_t", "uint64_t", "uint8_t",
    "unsigned", "wchar_t", "void",
];

/// Checks if a macro name is a special keyword that should be excluded.
#[inline]
fn is_special_keyword(name: &str) -> bool {
    SPECIAL_KEYWORDS.contains(&name)
}

/// Extracts preprocessor directives from a C/C++ file.
///
/// This function parses the file using tree-sitter and extracts:
/// - Include directives (#include "..." or #include <...>)
/// - Macro definitions (#define NAME ...)
///
/// # Arguments
///
/// * `tree` - The parsed tree-sitter Tree
/// * `source` - The source code as a string
/// * `path` - Path to the file being analyzed
/// * `results` - PreprocResults to store the extracted data
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if parsing fails.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::{TreeSitterWrapper, preprocessor::{extract_preprocessor, PreprocResults}};
/// use std::path::Path;
///
/// # fn main() -> anyhow::Result<()> {
/// let source = "#include <stdio.h>\n#define MAX 100";
/// let mut parser = TreeSitterWrapper::new(tree_sitter_cpp::LANGUAGE.into())?;
/// let tree = parser.parse(source)?;
/// let mut results = PreprocResults::default();
/// extract_preprocessor(&tree, source, Path::new("test.cpp"), &mut results)?;
/// # Ok(())
/// # }
/// ```
pub fn extract_preprocessor(
    tree: &Tree,
    source: &str,
    path: &Path,
    results: &mut PreprocResults,
) -> Result<()> {
    let root = tree.root_node();

    let mut file_result = PreprocFile::default();
    let mut cursor = root.walk();
    let mut stack = vec![root];

    // Traverse the syntax tree
    while let Some(node) = stack.pop() {
        cursor.reset(node);

        // Add children to stack for traversal
        if cursor.goto_first_child() {
            loop {
                stack.push(cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        let kind = node.kind();

        match kind {
            // Handle #define and #undef directives
            "preproc_def" | "preproc_function_def" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    if let Ok(macro_name) = name_node.utf8_text(source.as_bytes()) {
                        if !is_special_keyword(macro_name) {
                            file_result.macros.insert(macro_name.to_string());
                            debug!("Found macro: {}", macro_name);
                        }
                    }
                }
            }

            // Handle #include directives
            "preproc_include" => {
                if let Some(path_node) = node.child_by_field_name("path") {
                    if let Ok(include_path) = path_node.utf8_text(source.as_bytes()) {
                        // Remove quotes or angle brackets
                        let cleaned_path = include_path
                            .trim_start_matches('"')
                            .trim_end_matches('"')
                            .trim_start_matches('<')
                            .trim_end_matches('>')
                            .trim();

                        if !cleaned_path.is_empty() {
                            file_result.direct_includes.insert(cleaned_path.to_string());
                            debug!("Found include: {}", cleaned_path);
                        }
                    }
                }
            }

            _ => {}
        }
    }

    results.files.insert(path.to_path_buf(), file_result);
    Ok(())
}

/// Builds a dependency graph from include directives and resolves indirect includes.
///
/// This function:
/// 1. Constructs a directed graph of include dependencies
/// 2. Detects and handles include cycles using strongly connected components
/// 3. Propagates indirect includes through the dependency graph
///
/// # Arguments
///
/// * `files` - Mutable map of file paths to their preprocessor data
/// * `all_files` - Map of filenames to their full paths (for include resolution)
///
/// # Examples
///
/// ```no_run
/// use cortex_code_analysis::preprocessor::{PreprocFile, build_include_graph};
/// use std::path::PathBuf;
/// use std::collections::HashMap;
///
/// let mut files = HashMap::new();
/// let all_files = HashMap::new();
/// build_include_graph(&mut files, &all_files);
/// ```
pub fn build_include_graph(
    files: &mut HashMap<PathBuf, PreprocFile>,
    all_files: &HashMap<String, Vec<PathBuf>>,
) {
    let mut nodes: HashMap<PathBuf, NodeIndex> = HashMap::new();
    let mut g = StableGraph::new();

    // Build the dependency graph
    debug!("Building include dependency graph for {} files", files.len());

    for (file_path, file_data) in files.iter() {
        let node = *nodes.entry(file_path.clone())
            .or_insert_with(|| g.add_node(file_path.clone()));

        for include in &file_data.direct_includes {
            let resolved_paths = resolve_include(file_path, include, all_files);

            for resolved_path in resolved_paths {
                if &resolved_path != file_path {
                    let include_node = *nodes.entry(resolved_path.clone())
                        .or_insert_with(|| g.add_node(resolved_path));
                    g.add_edge(node, include_node, ());
                } else {
                    warn!("Self-inclusion detected: {:?}", file_path);
                }
            }
        }
    }

    // Detect and handle strongly connected components (cycles)
    let sccs = kosaraju_scc(&g);
    let mut scc_map: HashMap<NodeIndex, HashSet<PathBuf>> = HashMap::new();

    for component in sccs {
        if component.len() > 1 {
            // Found a cycle
            warn!("Include cycle detected with {} files:", component.len());

            let mut incoming = Vec::new();
            let mut outgoing = Vec::new();
            let mut paths = HashSet::new();

            // Collect paths in the cycle and their external connections
            for &node_idx in &component {
                if let Some(path) = g.node_weight(node_idx) {
                    warn!("  - {:?}", path);
                    paths.insert(path.clone());
                }

                for neighbor in g.neighbors_directed(node_idx, Direction::Incoming) {
                    if !component.contains(&neighbor) && !incoming.contains(&neighbor) {
                        incoming.push(neighbor);
                    }
                }

                for neighbor in g.neighbors_directed(node_idx, Direction::Outgoing) {
                    if !component.contains(&neighbor) && !outgoing.contains(&neighbor) {
                        outgoing.push(neighbor);
                    }
                }
            }

            // Create a replacement node for the cycle
            let replacement = g.add_node(PathBuf::new());

            for incoming_node in incoming {
                g.add_edge(incoming_node, replacement, ());
            }

            for outgoing_node in outgoing {
                g.add_edge(replacement, outgoing_node, ());
            }

            // Remove the cycle nodes
            for &node_idx in &component {
                if let Some(path) = g.node_weight(node_idx) {
                    *nodes.get_mut(path).unwrap() = replacement;
                }
                g.remove_node(node_idx);
            }

            scc_map.insert(replacement, paths);
        }
    }

    // Propagate indirect includes through the graph
    for (path, node_idx) in &nodes {
        if let Some(file_data) = files.get_mut(path) {
            let mut dfs = Dfs::new(&g, *node_idx);

            while let Some(visited_node) = dfs.next(&g) {
                if let Some(visited_path) = g.node_weight(visited_node) {
                    if visited_path.as_os_str().is_empty() {
                        // This is a cycle replacement node
                        if let Some(cycle_paths) = scc_map.get(&visited_node) {
                            for cycle_path in cycle_paths {
                                file_data.indirect_includes.insert(
                                    cycle_path.to_string_lossy().to_string()
                                );
                            }
                        }
                    } else if visited_path != path {
                        file_data.indirect_includes.insert(
                            visited_path.to_string_lossy().to_string()
                        );
                    }
                }
            }
        }
    }

    debug!("Include graph built successfully");
}

/// Resolves an include path to actual file paths.
///
/// This is a simplified version that tries to match include paths
/// to known files by filename.
fn resolve_include(
    current_file: &Path,
    include_path: &str,
    all_files: &HashMap<String, Vec<PathBuf>>,
) -> Vec<PathBuf> {
    // Get just the filename from the include path
    let include_filename = Path::new(include_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(include_path);

    if let Some(candidates) = all_files.get(include_filename) {
        if candidates.len() == 1 {
            return candidates.clone();
        }

        // Try to find the best match based on directory proximity
        let mut best_matches = Vec::new();

        // First, try exact path match
        for candidate in candidates {
            if candidate.ends_with(include_path) {
                best_matches.push(candidate.clone());
            }
        }

        if !best_matches.is_empty() {
            return best_matches;
        }

        // Otherwise, prefer files in the same directory
        if let Some(current_dir) = current_file.parent() {
            for candidate in candidates {
                if candidate.starts_with(current_dir) {
                    best_matches.push(candidate.clone());
                }
            }

            if !best_matches.is_empty() {
                return best_matches;
            }
        }

        // Fall back to all candidates
        return candidates.clone();
    }

    Vec::new()
}

/// Retrieves all macros visible to a file (direct and indirect).
///
/// Returns all macros defined in the file itself plus macros from
/// all files included indirectly.
///
/// # Examples
///
/// ```no_run
/// use cortex_code_analysis::preprocessor::{PreprocFile, get_all_macros};
/// use std::path::{Path, PathBuf};
/// use std::collections::HashMap;
///
/// let files = HashMap::new();
/// let macros = get_all_macros(Path::new("test.cpp"), &files);
/// ```
pub fn get_all_macros(
    file: &Path,
    files: &HashMap<PathBuf, PreprocFile>,
) -> HashSet<String> {
    let mut macros = HashSet::new();

    if let Some(file_data) = files.get(file) {
        // Add direct macros
        macros.extend(file_data.macros.iter().cloned());

        // Add macros from indirect includes
        for include in &file_data.indirect_includes {
            if let Some(include_data) = files.get(&PathBuf::from(include)) {
                macros.extend(include_data.macros.iter().cloned());
            }
        }
    }

    macros
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TreeSitterWrapper;

    #[test]
    fn test_extract_includes() {
        let source = r#"
#include <stdio.h>
#include "myheader.h"
        "#;

        let mut parser = TreeSitterWrapper::new(tree_sitter_cpp::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source).unwrap();

        let mut results = PreprocResults::default();
        extract_preprocessor(&tree, source, Path::new("test.cpp"), &mut results).unwrap();

        let file_data = results.files.get(Path::new("test.cpp")).unwrap();
        assert_eq!(file_data.direct_includes.len(), 2);
        assert!(file_data.direct_includes.contains("stdio.h"));
        assert!(file_data.direct_includes.contains("myheader.h"));
    }

    #[test]
    fn test_extract_macros() {
        let source = r#"
#define MAX_SIZE 100
#define MIN_SIZE 10
#define BUFFER_SIZE (MAX_SIZE + 100)
        "#;

        let mut parser = TreeSitterWrapper::new(tree_sitter_cpp::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source).unwrap();

        let mut results = PreprocResults::default();
        extract_preprocessor(&tree, source, Path::new("test.cpp"), &mut results).unwrap();

        let file_data = results.files.get(Path::new("test.cpp")).unwrap();
        assert_eq!(file_data.macros.len(), 3);
        assert!(file_data.macros.contains("MAX_SIZE"));
        assert!(file_data.macros.contains("MIN_SIZE"));
        assert!(file_data.macros.contains("BUFFER_SIZE"));
    }

    #[test]
    fn test_special_keywords_excluded() {
        let source = r#"
#define NULL 0
#define MY_MACRO 42
#define size_t unsigned long
        "#;

        let mut parser = TreeSitterWrapper::new(tree_sitter_cpp::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source).unwrap();

        let mut results = PreprocResults::default();
        extract_preprocessor(&tree, source, Path::new("test.cpp"), &mut results).unwrap();

        let file_data = results.files.get(Path::new("test.cpp")).unwrap();
        // Only MY_MACRO should be included (NULL and size_t are special keywords)
        assert_eq!(file_data.macros.len(), 1);
        assert!(file_data.macros.contains("MY_MACRO"));
    }

    #[test]
    fn test_preproc_file_new_macros() {
        let file = PreprocFile::new_macros(&["MACRO1", "MACRO2", "MACRO3"]);
        assert_eq!(file.macros.len(), 3);
        assert!(file.macros.contains("MACRO1"));
        assert!(file.macros.contains("MACRO2"));
        assert!(file.macros.contains("MACRO3"));
    }

    #[test]
    fn test_resolve_include_simple() {
        let mut all_files = HashMap::new();
        all_files.insert(
            "header.h".to_string(),
            vec![PathBuf::from("/project/include/header.h")],
        );

        let current = Path::new("/project/src/main.cpp");
        let resolved = resolve_include(current, "header.h", &all_files);

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0], PathBuf::from("/project/include/header.h"));
    }

    #[test]
    fn test_get_all_macros() {
        let mut files = HashMap::new();

        let mut file1 = PreprocFile::default();
        file1.macros.insert("MACRO1".to_string());

        let mut file2 = PreprocFile::default();
        file2.macros.insert("MACRO2".to_string());

        files.insert(PathBuf::from("file1.h"), file1);
        files.insert(PathBuf::from("file2.h"), file2);

        // Add indirect include
        files.get_mut(&PathBuf::from("file1.h")).unwrap()
            .indirect_includes.insert("file2.h".to_string());

        let all_macros = get_all_macros(Path::new("file1.h"), &files);
        assert_eq!(all_macros.len(), 2);
        assert!(all_macros.contains("MACRO1"));
        assert!(all_macros.contains("MACRO2"));
    }
}
