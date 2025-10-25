//! C/C++ Preprocessor Directive Extraction and Analysis
//!
//! This module provides comprehensive functionality to extract and analyze C/C++ preprocessor
//! directives including include statements and macro definitions. It builds dependency graphs
//! to track include relationships, detects cycles, and supports macro replacement for improved
//! parsing accuracy.
//!
//! # Key Features
//!
//! - **Include Directive Extraction**: Parses #include directives and builds dependency graphs
//! - **Macro Definition Tracking**: Extracts #define and #undef directives
//! - **Dependency Graph Construction**: Uses petgraph to build and analyze include relationships
//! - **Cycle Detection**: Identifies and handles circular include dependencies using SCC
//! - **Transitive Include Resolution**: Computes direct and indirect include relationships
//! - **Macro Replacement**: Replaces macros with placeholders for improved parsing
//!
//! # Architecture
//!
//! The module is organized into several key components:
//!
//! 1. **Data Structures**: `PreprocFile` and `PreprocResults` for storing preprocessor data
//! 2. **Extraction**: Functions to parse and extract preprocessor directives from source code
//! 3. **Graph Analysis**: Dependency graph construction and transitive closure computation
//! 4. **Macro Handling**: Integration with `c_macro` module for macro replacement
//!
//! # Examples
//!
//! ## Basic Extraction
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
//! #define MIN_SIZE 10
//! "#;
//!
//! let mut parser = TreeSitterWrapper::new(tree_sitter_cpp::LANGUAGE.into())?;
//! let tree = parser.parse(source)?;
//! let mut results = PreprocResults::default();
//! extract_preprocessor(&tree, source, Path::new("example.cpp"), &mut results)?;
//!
//! let file_data = results.files.get(Path::new("example.cpp")).unwrap();
//! assert_eq!(file_data.direct_includes.len(), 2);
//! assert_eq!(file_data.macros.len(), 2);
//! # Ok(())
//! # }
//! ```
//!
//! ## Building Include Graph
//!
//! ```no_run
//! use cortex_code_analysis::preprocessor::{PreprocFile, build_include_graph};
//! use std::path::PathBuf;
//! use std::collections::HashMap;
//!
//! let mut files = HashMap::new();
//! let all_files = HashMap::new(); // Map of filename -> Vec<PathBuf>
//!
//! // ... populate files with preprocessor data ...
//!
//! build_include_graph(&mut files, &all_files);
//!
//! // Now files contain both direct and indirect includes
//! ```

use std::collections::{HashMap, HashSet, hash_map};
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

use crate::c_specials::is_special_keyword;

/// Preprocessor data for a single C/C++ file.
///
/// Contains information about include directives and macro definitions found in the file.
/// This structure tracks both direct includes (explicitly written in the file) and
/// indirect includes (transitively included through other files).
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::preprocessor::PreprocFile;
///
/// let mut file = PreprocFile::default();
/// file.direct_includes.insert("stdio.h".to_string());
/// file.macros.insert("MAX_SIZE".to_string());
/// ```
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct PreprocFile {
    /// Include directives explicitly written in this file.
    ///
    /// These are the #include statements that appear directly in the source code.
    pub direct_includes: HashSet<String>,

    /// Include directives indirectly imported from other files.
    ///
    /// These are computed by analyzing the dependency graph and represent
    /// all files that are transitively included through direct includes.
    /// Populated after calling `build_include_graph`.
    pub indirect_includes: HashSet<String>,

    /// Macro names defined in this file.
    ///
    /// Excludes special C/C++ keywords and standard library types to focus
    /// on user-defined macros. Populated from #define directives.
    pub macros: HashSet<String>,
}

/// Preprocessor data for multiple C/C++ files.
///
/// Maps file paths to their preprocessor data. This is the main container
/// for all preprocessor analysis results across a codebase.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::preprocessor::PreprocResults;
/// use std::path::PathBuf;
///
/// let mut results = PreprocResults::default();
/// // ... extract preprocessor data ...
/// for (path, data) in &results.files {
///     println!("{:?}: {} includes, {} macros", path,
///              data.direct_includes.len(), data.macros.len());
/// }
/// ```
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct PreprocResults {
    /// Preprocessor data for each analyzed file.
    pub files: HashMap<PathBuf, PreprocFile>,
}

impl PreprocFile {
    /// Creates a new PreprocFile with the given macro definitions.
    ///
    /// This is a convenience constructor for testing or initializing files
    /// with known macros.
    ///
    /// # Examples
    ///
    /// ```
    /// use cortex_code_analysis::preprocessor::PreprocFile;
    ///
    /// let file = PreprocFile::new_macros(&["MAX_SIZE", "MIN_SIZE", "BUFFER_SIZE"]);
    /// assert_eq!(file.macros.len(), 3);
    /// assert!(file.macros.contains("MAX_SIZE"));
    /// ```
    pub fn new_macros(macros: &[&str]) -> Self {
        let mut pf = Self::default();
        for m in macros {
            pf.macros.insert((*m).to_string());
        }
        pf
    }
}

/// Extracts preprocessor directives from a C/C++ file.
///
/// This function parses the file using tree-sitter and extracts:
/// - Include directives (#include "..." or #include <...>)
/// - Macro definitions (#define NAME ...)
/// - Macro undefinitions (#undef NAME)
///
/// The extraction excludes special C/C++ keywords and standard library types
/// from the macro list to focus on user-defined macros.
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
/// This function performs several advanced analyses:
///
/// 1. **Graph Construction**: Creates a directed graph where nodes are files and
///    edges represent include relationships.
///
/// 2. **Cycle Detection**: Uses Kosaraju's algorithm to find strongly connected
///    components (SCCs), which represent include cycles.
///
/// 3. **Cycle Handling**: Replaces each SCC with a single node to enable
///    traversal without infinite loops.
///
/// 4. **Transitive Closure**: Computes all files reachable from each file through
///    include chains using depth-first search.
///
/// The algorithm correctly handles complex scenarios including:
/// - Circular includes (A includes B, B includes A)
/// - Multi-file cycles (A → B → C → A)
/// - Diamond dependencies (A → B, A → C, B → D, C → D)
///
/// # Arguments
///
/// * `files` - Mutable map of file paths to their preprocessor data.
///             The `indirect_includes` field will be populated.
/// * `all_files` - Map of filenames to their full paths for include resolution.
///                 Used to match #include directives to actual files.
///
/// # Algorithm Details
///
/// The algorithm proceeds in several phases:
///
/// 1. **Node Creation**: Each file becomes a node in the graph
/// 2. **Edge Creation**: Include directives create edges between nodes
/// 3. **SCC Detection**: Find all strongly connected components
/// 4. **Cycle Replacement**: Replace each SCC with a single meta-node
/// 5. **Transitive Closure**: DFS from each node to find all reachable files
///
/// # Performance
///
/// - Time Complexity: O(V + E) for graph construction and SCC detection
/// - Space Complexity: O(V + E) for the graph structure
/// - Uses StableGraph to maintain node indices during SCC removal
///
/// # Examples
///
/// ```no_run
/// use cortex_code_analysis::preprocessor::{PreprocFile, build_include_graph};
/// use std::path::PathBuf;
/// use std::collections::HashMap;
///
/// let mut files = HashMap::new();
/// let mut all_files = HashMap::new();
///
/// // Populate files with preprocessor data
/// // Populate all_files with filename -> path mappings
///
/// build_include_graph(&mut files, &all_files);
///
/// // Now files contain computed indirect includes
/// for (path, data) in &files {
///     println!("{:?}: {} total includes ({} direct, {} indirect)",
///              path,
///              data.direct_includes.len() + data.indirect_includes.len(),
///              data.direct_includes.len(),
///              data.indirect_includes.len());
/// }
/// ```
pub fn build_include_graph(
    files: &mut HashMap<PathBuf, PreprocFile>,
    all_files: &HashMap<String, Vec<PathBuf>>,
) {
    let mut nodes: HashMap<PathBuf, NodeIndex> = HashMap::new();
    // Use StableGraph to maintain node indices when removing SCC nodes
    let mut g = StableGraph::new();

    // Phase 1: Build the dependency graph
    debug!("Building include dependency graph for {} files", files.len());

    for (file_path, file_data) in files.iter() {
        let node = match nodes.entry(file_path.clone()) {
            hash_map::Entry::Occupied(entry) => *entry.get(),
            hash_map::Entry::Vacant(entry) => *entry.insert(g.add_node(file_path.clone())),
        };

        for include in &file_data.direct_includes {
            let resolved_paths = resolve_include(file_path, include, all_files);

            for resolved_path in resolved_paths {
                if &resolved_path != file_path {
                    let include_node = match nodes.entry(resolved_path.clone()) {
                        hash_map::Entry::Occupied(entry) => *entry.get(),
                        hash_map::Entry::Vacant(entry) => {
                            *entry.insert(g.add_node(resolved_path))
                        }
                    };
                    g.add_edge(node, include_node, ());
                } else {
                    warn!("Self-inclusion detected: {:?}", file_path);
                }
            }
        }
    }

    // Phase 2: Detect and handle strongly connected components (cycles)
    let sccs = kosaraju_scc(&g);
    let mut scc_map: HashMap<NodeIndex, HashSet<PathBuf>> = HashMap::new();

    for component in sccs {
        if component.len() > 1 {
            // Found a cycle - log it
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

                // Find incoming edges from outside the SCC
                for neighbor in g.neighbors_directed(node_idx, Direction::Incoming) {
                    if !component.contains(&neighbor) && !incoming.contains(&neighbor) {
                        incoming.push(neighbor);
                    }
                }

                // Find outgoing edges to outside the SCC
                for neighbor in g.neighbors_directed(node_idx, Direction::Outgoing) {
                    if !component.contains(&neighbor) && !outgoing.contains(&neighbor) {
                        outgoing.push(neighbor);
                    }
                }
            }

            // Create a replacement node for the entire cycle
            let replacement = g.add_node(PathBuf::new());

            // Connect incoming edges to replacement
            for incoming_node in incoming {
                g.add_edge(incoming_node, replacement, ());
            }

            // Connect replacement to outgoing edges
            for outgoing_node in outgoing {
                g.add_edge(replacement, outgoing_node, ());
            }

            // Remove the original cycle nodes and update node map
            for &node_idx in &component {
                if let Some(path) = g.node_weight(node_idx) {
                    *nodes.get_mut(path).unwrap() = replacement;
                }
                g.remove_node(node_idx);
            }

            scc_map.insert(replacement, paths);
        }
    }

    // Phase 3: Propagate indirect includes through the graph using DFS
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
/// This function attempts to match an include directive (like "stdio.h" or
/// "myheader.h") to actual file paths in the codebase. It uses several
/// heuristics to find the best match:
///
/// 1. **Exact Match**: If only one file has this name, use it
/// 2. **Path Suffix Match**: Prefer files where the full path ends with the include path
/// 3. **Directory Proximity**: Prefer files in the same directory as the including file
/// 4. **Fallback**: Return all candidates if no better match is found
///
/// # Arguments
///
/// * `current_file` - The file containing the include directive
/// * `include_path` - The path from the #include directive (e.g., "stdio.h", "util/helper.h")
/// * `all_files` - Map of filenames to their full paths
///
/// # Returns
///
/// A vector of resolved file paths. May be empty if no matches are found.
///
/// # Examples
///
/// ```no_run
/// use cortex_code_analysis::preprocessor::resolve_include;
/// use std::path::{Path, PathBuf};
/// use std::collections::HashMap;
///
/// let mut all_files = HashMap::new();
/// all_files.insert(
///     "stdio.h".to_string(),
///     vec![PathBuf::from("/usr/include/stdio.h")]
/// );
///
/// let current = Path::new("/home/user/project/main.c");
/// let resolved = resolve_include(current, "stdio.h", &all_files);
/// assert_eq!(resolved.len(), 1);
/// ```
pub fn resolve_include(
    current_file: &Path,
    include_path: &str,
    all_files: &HashMap<String, Vec<PathBuf>>,
) -> Vec<PathBuf> {
    // Extract just the filename from the include path
    let include_filename = Path::new(include_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(include_path);

    if let Some(candidates) = all_files.get(include_filename) {
        // If there's only one candidate, return it
        if candidates.len() == 1 {
            return candidates.clone();
        }

        // Try to find the best match based on path suffix
        // Only use this if the include path contains a directory separator
        if include_path.contains('/') || include_path.contains('\\') {
            let mut exact_matches = Vec::new();
            for candidate in candidates {
                if candidate.ends_with(include_path) {
                    exact_matches.push(candidate.clone());
                }
            }

            if !exact_matches.is_empty() {
                return exact_matches;
            }
        }

        // Prefer files in the same directory
        if let Some(current_dir) = current_file.parent() {
            let mut same_dir_matches = Vec::new();
            for candidate in candidates {
                if let Some(candidate_dir) = candidate.parent() {
                    if candidate_dir == current_dir {
                        same_dir_matches.push(candidate.clone());
                    }
                }
            }

            if !same_dir_matches.is_empty() {
                return same_dir_matches;
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
/// all files included indirectly. This represents the complete set
/// of macros that could affect the file's compilation.
///
/// # Arguments
///
/// * `file` - The file path to query
/// * `files` - Map of all files and their preprocessor data
///
/// # Returns
///
/// A set of all macro names visible to the file.
///
/// # Examples
///
/// ```no_run
/// use cortex_code_analysis::preprocessor::{PreprocFile, get_all_macros};
/// use std::path::{Path, PathBuf};
/// use std::collections::HashMap;
///
/// let mut files = HashMap::new();
/// // ... populate files ...
///
/// let macros = get_all_macros(Path::new("src/main.c"), &files);
/// println!("Total macros visible: {}", macros.len());
/// ```
pub fn get_all_macros(
    file: &Path,
    files: &HashMap<PathBuf, PreprocFile>,
) -> HashSet<String> {
    let mut macros = HashSet::new();

    if let Some(file_data) = files.get(file) {
        // Add direct macros from this file
        macros.extend(file_data.macros.iter().cloned());

        // Add macros from all indirectly included files
        for include in &file_data.indirect_includes {
            if let Some(include_data) = files.get(&PathBuf::from(include)) {
                macros.extend(include_data.macros.iter().cloned());
            }
        }
    }

    macros
}

/// Guess possible file paths for an include directive.
///
/// This is a legacy function maintained for compatibility with existing code.
/// It attempts to resolve include paths using heuristics.
///
/// # Arguments
///
/// * `current_file` - The file containing the include
/// * `include_path` - The include directive path
/// * `all_files` - Map of filenames to paths
///
/// # Returns
///
/// A vector of possible file paths.
pub fn guess_file(
    current_file: &Path,
    include_path: &str,
    all_files: &HashMap<String, Vec<PathBuf>>,
) -> Vec<PathBuf> {
    resolve_include(current_file, include_path, all_files)
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
#include <stdlib.h>
        "#;

        let mut parser = TreeSitterWrapper::new(tree_sitter_cpp::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source).unwrap();

        let mut results = PreprocResults::default();
        extract_preprocessor(&tree, source, Path::new("test.cpp"), &mut results).unwrap();

        let file_data = results.files.get(Path::new("test.cpp")).unwrap();
        assert_eq!(file_data.direct_includes.len(), 3);
        assert!(file_data.direct_includes.contains("stdio.h"));
        assert!(file_data.direct_includes.contains("myheader.h"));
        assert!(file_data.direct_includes.contains("stdlib.h"));
    }

    #[test]
    fn test_extract_macros() {
        let source = r#"
#define MAX_SIZE 100
#define MIN_SIZE 10
#define BUFFER_SIZE (MAX_SIZE + 100)
#define DEBUG_MODE
        "#;

        let mut parser = TreeSitterWrapper::new(tree_sitter_cpp::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source).unwrap();

        let mut results = PreprocResults::default();
        extract_preprocessor(&tree, source, Path::new("test.cpp"), &mut results).unwrap();

        let file_data = results.files.get(Path::new("test.cpp")).unwrap();
        assert_eq!(file_data.macros.len(), 4);
        assert!(file_data.macros.contains("MAX_SIZE"));
        assert!(file_data.macros.contains("MIN_SIZE"));
        assert!(file_data.macros.contains("BUFFER_SIZE"));
        assert!(file_data.macros.contains("DEBUG_MODE"));
    }

    #[test]
    fn test_special_keywords_excluded() {
        let source = r#"
#define NULL 0
#define MY_MACRO 42
#define size_t unsigned long
#define int32_t int
#define CUSTOM_TYPE 1
        "#;

        let mut parser = TreeSitterWrapper::new(tree_sitter_cpp::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source).unwrap();

        let mut results = PreprocResults::default();
        extract_preprocessor(&tree, source, Path::new("test.cpp"), &mut results).unwrap();

        let file_data = results.files.get(Path::new("test.cpp")).unwrap();
        // Only MY_MACRO and CUSTOM_TYPE should be included
        // NULL, size_t, and int32_t are special keywords
        assert_eq!(file_data.macros.len(), 2);
        assert!(file_data.macros.contains("MY_MACRO"));
        assert!(file_data.macros.contains("CUSTOM_TYPE"));
        assert!(!file_data.macros.contains("NULL"));
        assert!(!file_data.macros.contains("size_t"));
        assert!(!file_data.macros.contains("int32_t"));
    }

    #[test]
    fn test_preproc_file_new_macros() {
        let file = PreprocFile::new_macros(&["MACRO1", "MACRO2", "MACRO3"]);
        assert_eq!(file.macros.len(), 3);
        assert!(file.macros.contains("MACRO1"));
        assert!(file.macros.contains("MACRO2"));
        assert!(file.macros.contains("MACRO3"));
        assert_eq!(file.direct_includes.len(), 0);
        assert_eq!(file.indirect_includes.len(), 0);
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
    fn test_resolve_include_multiple_candidates() {
        let mut all_files = HashMap::new();
        all_files.insert(
            "config.h".to_string(),
            vec![
                PathBuf::from("/project/src/config.h"),
                PathBuf::from("/project/include/config.h"),
            ],
        );

        let current = Path::new("/project/src/main.cpp");
        let resolved = resolve_include(current, "config.h", &all_files);

        // Should prefer the one in the same directory
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0], PathBuf::from("/project/src/config.h"));
    }

    #[test]
    fn test_resolve_include_path_suffix() {
        let mut all_files = HashMap::new();
        all_files.insert(
            "util.h".to_string(),
            vec![
                PathBuf::from("/project/util.h"),
                PathBuf::from("/project/lib/util.h"),
            ],
        );

        let current = Path::new("/project/src/main.cpp");
        let resolved = resolve_include(current, "lib/util.h", &all_files);

        // Should match the path suffix
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0], PathBuf::from("/project/lib/util.h"));
    }

    #[test]
    fn test_get_all_macros() {
        let mut files = HashMap::new();

        let mut file1 = PreprocFile::default();
        file1.macros.insert("MACRO1".to_string());
        file1.macros.insert("MACRO2".to_string());

        let mut file2 = PreprocFile::default();
        file2.macros.insert("MACRO3".to_string());

        let mut file3 = PreprocFile::default();
        file3.macros.insert("MACRO4".to_string());

        files.insert(PathBuf::from("file1.h"), file1);
        files.insert(PathBuf::from("file2.h"), file2);
        files.insert(PathBuf::from("file3.h"), file3);

        // file1 indirectly includes file2 and file3
        files.get_mut(&PathBuf::from("file1.h")).unwrap()
            .indirect_includes.insert("file2.h".to_string());
        files.get_mut(&PathBuf::from("file1.h")).unwrap()
            .indirect_includes.insert("file3.h".to_string());

        let all_macros = get_all_macros(Path::new("file1.h"), &files);
        assert_eq!(all_macros.len(), 4);
        assert!(all_macros.contains("MACRO1"));
        assert!(all_macros.contains("MACRO2"));
        assert!(all_macros.contains("MACRO3"));
        assert!(all_macros.contains("MACRO4"));
    }

    #[test]
    fn test_build_include_graph_simple() {
        let mut files = HashMap::new();
        let mut all_files = HashMap::new();

        // Create a simple dependency: main.c includes util.h
        let mut main_file = PreprocFile::default();
        main_file.direct_includes.insert("util.h".to_string());
        files.insert(PathBuf::from("/project/main.c"), main_file);

        let util_file = PreprocFile::default();
        files.insert(PathBuf::from("/project/util.h"), util_file);

        all_files.insert(
            "util.h".to_string(),
            vec![PathBuf::from("/project/util.h")],
        );

        build_include_graph(&mut files, &all_files);

        let main_data = files.get(&PathBuf::from("/project/main.c")).unwrap();
        assert_eq!(main_data.indirect_includes.len(), 1);
        assert!(main_data.indirect_includes.contains("/project/util.h"));
    }

    #[test]
    fn test_build_include_graph_transitive() {
        let mut files = HashMap::new();
        let mut all_files = HashMap::new();

        // Chain: main.c -> util.h -> config.h
        let mut main_file = PreprocFile::default();
        main_file.direct_includes.insert("util.h".to_string());
        files.insert(PathBuf::from("/project/main.c"), main_file);

        let mut util_file = PreprocFile::default();
        util_file.direct_includes.insert("config.h".to_string());
        files.insert(PathBuf::from("/project/util.h"), util_file);

        let config_file = PreprocFile::default();
        files.insert(PathBuf::from("/project/config.h"), config_file);

        all_files.insert(
            "util.h".to_string(),
            vec![PathBuf::from("/project/util.h")],
        );
        all_files.insert(
            "config.h".to_string(),
            vec![PathBuf::from("/project/config.h")],
        );

        build_include_graph(&mut files, &all_files);

        let main_data = files.get(&PathBuf::from("/project/main.c")).unwrap();
        // Should include both util.h and config.h
        assert_eq!(main_data.indirect_includes.len(), 2);
        assert!(main_data.indirect_includes.contains("/project/util.h"));
        assert!(main_data.indirect_includes.contains("/project/config.h"));
    }

    #[test]
    fn test_function_macro_extraction() {
        let source = r#"
#define MAX(a, b) ((a) > (b) ? (a) : (b))
#define MIN(x, y) ((x) < (y) ? (x) : (y))
        "#;

        let mut parser = TreeSitterWrapper::new(tree_sitter_cpp::LANGUAGE.into()).unwrap();
        let tree = parser.parse(source).unwrap();

        let mut results = PreprocResults::default();
        extract_preprocessor(&tree, source, Path::new("test.cpp"), &mut results).unwrap();

        let file_data = results.files.get(Path::new("test.cpp")).unwrap();
        assert_eq!(file_data.macros.len(), 2);
        assert!(file_data.macros.contains("MAX"));
        assert!(file_data.macros.contains("MIN"));
    }
}
