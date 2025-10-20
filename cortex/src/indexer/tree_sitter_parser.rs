use crate::types::{
    CodeSymbol, Hash, Location, Reference, ReferenceKind, SymbolId, SymbolKind, SymbolMetadata,
    TokenCount,
};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tree_sitter::{Language, Node, Parser, Query, QueryCursor, Range, StreamingIterator, Tree};

/// Language information
#[derive(Debug, Clone)]
struct LanguageInfo {
    #[allow(dead_code)]
    name: &'static str,
    language: Language,
    function_query: &'static str,
    class_query: &'static str,
    interface_query: &'static str,
    type_query: &'static str,
}

/// Result of incremental parsing
#[derive(Debug)]
pub struct IncrementalParseResult {
    /// All symbols in the file
    pub symbols: Vec<CodeSymbol>,
    /// Ranges that changed from previous parse
    pub changed_ranges: Vec<Range>,
}

pub struct TreeSitterParser {
    parser: Parser,
    languages: HashMap<&'static str, LanguageInfo>,
    /// Cache of parse trees for incremental parsing
    tree_cache: HashMap<PathBuf, Tree>,
}

impl TreeSitterParser {
    pub fn new() -> Result<Self> {
        let parser = Parser::new();

        let mut languages = HashMap::new();

        // Rust
        let rust_lang = tree_sitter_rust::LANGUAGE.into();
        languages.insert(
            "rust",
            LanguageInfo {
                name: "rust",
                language: rust_lang,
                function_query: r#"
                    (function_item
                        name: (identifier) @name
                        parameters: (parameters) @params
                        return_type: (type_identifier)? @return) @function
                "#,
                class_query: r#"
                    (struct_item
                        name: (type_identifier) @name) @struct
                    (enum_item
                        name: (type_identifier) @name) @enum
                "#,
                interface_query: r#"
                    (trait_item
                        name: (type_identifier) @name) @trait
                "#,
                type_query: r#"
                    (type_item
                        name: (type_identifier) @name) @type
                "#,
            },
        );

        // TypeScript
        let ts_lang = tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into();
        languages.insert(
            "typescript",
            LanguageInfo {
                name: "typescript",
                language: ts_lang,
                function_query: r#"
                    (function_declaration
                        name: (identifier) @name
                        parameters: (formal_parameters) @params) @function
                    (method_definition
                        name: (property_identifier) @name
                        parameters: (formal_parameters) @params) @method
                "#,
                class_query: r#"
                    (class_declaration
                        name: (type_identifier) @name) @class
                "#,
                interface_query: r#"
                    (interface_declaration
                        name: (type_identifier) @name) @interface
                "#,
                type_query: r#"
                    (type_alias_declaration
                        name: (type_identifier) @name) @type
                "#,
            },
        );

        // TSX (TypeScript with JSX)
        let tsx_lang = tree_sitter_typescript::LANGUAGE_TSX.into();
        languages.insert(
            "tsx",
            LanguageInfo {
                name: "tsx",
                language: tsx_lang,
                function_query: r#"
                    (function_declaration
                        name: (identifier) @name
                        parameters: (formal_parameters) @params) @function
                    (method_definition
                        name: (property_identifier) @name
                        parameters: (formal_parameters) @params) @method
                "#,
                class_query: r#"
                    (class_declaration
                        name: (type_identifier) @name) @class
                "#,
                interface_query: r#"
                    (interface_declaration
                        name: (type_identifier) @name) @interface
                "#,
                type_query: r#"
                    (type_alias_declaration
                        name: (type_identifier) @name) @type
                "#,
            },
        );

        // JavaScript
        let js_lang = tree_sitter_javascript::LANGUAGE.into();
        languages.insert(
            "javascript",
            LanguageInfo {
                name: "javascript",
                language: js_lang,
                function_query: r#"
                    (function_declaration
                        name: (identifier) @name
                        parameters: (formal_parameters) @params) @function
                    (method_definition
                        name: (property_identifier) @name
                        parameters: (formal_parameters) @params) @method
                "#,
                class_query: r#"
                    (class_declaration
                        name: (identifier) @name) @class
                "#,
                interface_query: "",
                type_query: "",
            },
        );

        // Python
        let py_lang = tree_sitter_python::LANGUAGE.into();
        languages.insert(
            "python",
            LanguageInfo {
                name: "python",
                language: py_lang,
                function_query: r#"
                    (function_definition
                        name: (identifier) @name
                        parameters: (parameters) @params) @function
                "#,
                class_query: r#"
                    (class_definition
                        name: (identifier) @name) @class
                "#,
                interface_query: "",
                type_query: "",
            },
        );

        // Go
        let go_lang = tree_sitter_go::LANGUAGE.into();
        languages.insert(
            "go",
            LanguageInfo {
                name: "go",
                language: go_lang,
                function_query: r#"
                    (function_declaration
                        name: (identifier) @name
                        parameters: (parameter_list) @params) @function
                    (method_declaration
                        name: (field_identifier) @name
                        parameters: (parameter_list) @params) @method
                "#,
                class_query: r#"
                    (type_declaration
                        (type_spec
                            name: (type_identifier) @name
                            type: (struct_type))) @struct
                "#,
                interface_query: r#"
                    (type_declaration
                        (type_spec
                            name: (type_identifier) @name
                            type: (interface_type))) @interface
                "#,
                type_query: r#"
                    (type_declaration
                        (type_spec
                            name: (type_identifier) @name)) @type
                "#,
            },
        );

        Ok(Self {
            parser,
            languages,
            tree_cache: HashMap::new(),
        })
    }

    /// Parse a file and extract symbols
    pub fn parse_file(&mut self, path: &Path, content: &str) -> Result<Vec<CodeSymbol>> {
        let language = self.detect_language(path)?;
        let lang_info = self
            .languages
            .get(language)
            .ok_or_else(|| anyhow!("Unsupported language: {}", language))?;

        // Set parser language
        self.parser
            .set_language(&lang_info.language)
            .map_err(|e| anyhow!("Failed to set language: {}", e))?;

        // Parse the file
        let tree = self
            .parser
            .parse(content, None)
            .ok_or_else(|| anyhow!("Failed to parse file"))?;

        // Cache the tree for future incremental parsing
        self.tree_cache.insert(path.to_path_buf(), tree.clone());

        // Extract symbols
        let mut symbols = Vec::new();

        // Extract functions
        symbols.extend(self.extract_functions(&tree, content, path, lang_info)?);

        // Extract classes/structs
        symbols.extend(self.extract_classes(&tree, content, path, lang_info)?);

        // Extract interfaces/traits
        symbols.extend(self.extract_interfaces(&tree, content, path, lang_info)?);

        // Extract types
        symbols.extend(self.extract_types(&tree, content, path, lang_info)?);

        // Extract references between symbols
        self.extract_references(&mut symbols, &tree, content)?;

        Ok(symbols)
    }

    /// Parse a file incrementally using cached tree
    pub fn parse_file_incremental(&mut self, path: &Path, content: &str) -> Result<IncrementalParseResult> {
        let language = self.detect_language(path)?;
        let lang_info = self
            .languages
            .get(language)
            .ok_or_else(|| anyhow!("Unsupported language: {}", language))?;

        // Set parser language
        self.parser
            .set_language(&lang_info.language)
            .map_err(|e| anyhow!("Failed to set language: {}", e))?;

        // Get old tree if available
        let old_tree_opt = self.tree_cache.get(&path.to_path_buf());

        // Parse incrementally
        let new_tree = self
            .parser
            .parse(content, old_tree_opt)
            .ok_or_else(|| anyhow!("Failed to parse file"))?;

        // Get changed ranges
        let changed_ranges = if let Some(old_tree) = old_tree_opt {
            // Calculate changed ranges between old and new tree
            let ranges: Vec<Range> = new_tree.changed_ranges(old_tree).collect();
            if ranges.is_empty() && content != "" {
                // Tree-sitter didn't detect changes, mark full file as changed
                // This can happen when structure changes significantly
                vec![Range {
                    start_byte: 0,
                    end_byte: content.len(),
                    start_point: tree_sitter::Point { row: 0, column: 0 },
                    end_point: tree_sitter::Point {
                        row: content.lines().count().saturating_sub(1),
                        column: content.lines().last().map(|l| l.len()).unwrap_or(0),
                    },
                }]
            } else {
                ranges
            }
        } else {
            // No old tree, everything is changed
            vec![Range {
                start_byte: 0,
                end_byte: content.len(),
                start_point: tree_sitter::Point { row: 0, column: 0 },
                end_point: tree_sitter::Point {
                    row: content.lines().count().saturating_sub(1),
                    column: content.lines().last().map(|l| l.len()).unwrap_or(0),
                },
            }]
        };

        // Cache the new tree
        self.tree_cache.insert(path.to_path_buf(), new_tree.clone());

        // Extract all symbols (we'll filter in the indexer)
        let mut symbols = Vec::new();

        // Extract functions
        symbols.extend(self.extract_functions(&new_tree, content, path, lang_info)?);

        // Extract classes/structs
        symbols.extend(self.extract_classes(&new_tree, content, path, lang_info)?);

        // Extract interfaces/traits
        symbols.extend(self.extract_interfaces(&new_tree, content, path, lang_info)?);

        // Extract types
        symbols.extend(self.extract_types(&new_tree, content, path, lang_info)?);

        // Extract references between symbols
        self.extract_references(&mut symbols, &new_tree, content)?;

        Ok(IncrementalParseResult {
            symbols,
            changed_ranges,
        })
    }

    /// Clear cached tree for a file
    pub fn clear_tree_cache(&mut self, path: &Path) {
        self.tree_cache.remove(path);
    }

    /// Clear all cached trees
    pub fn clear_all_tree_caches(&mut self) {
        self.tree_cache.clear();
    }

    /// Get number of cached trees
    pub fn cached_tree_count(&self) -> usize {
        self.tree_cache.len()
    }

    /// Check if a symbol overlaps with any changed range
    pub fn symbol_in_changed_range(symbol: &CodeSymbol, changed_ranges: &[Range]) -> bool {
        for range in changed_ranges {
            let symbol_start_line = symbol.location.line_start;
            let symbol_end_line = symbol.location.line_end;
            let range_start_line = range.start_point.row + 1; // tree-sitter uses 0-based
            let range_end_line = range.end_point.row + 1;

            // Check if ranges overlap
            if !(symbol_end_line < range_start_line || symbol_start_line > range_end_line) {
                return true;
            }
        }
        false
    }

    /// Detect language from file extension
    fn detect_language(&self, path: &Path) -> Result<&'static str> {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("No file extension"))?;

        match ext {
            "rs" => Ok("rust"),
            "ts" => Ok("typescript"),
            "tsx" => Ok("tsx"),
            "js" | "jsx" => Ok("javascript"),
            "py" => Ok("python"),
            "go" => Ok("go"),
            _ => Err(anyhow!("Unsupported file extension: {}", ext)),
        }
    }

    /// Extract function symbols
    fn extract_functions(
        &self,
        tree: &Tree,
        content: &str,
        path: &Path,
        lang_info: &LanguageInfo,
    ) -> Result<Vec<CodeSymbol>> {
        if lang_info.function_query.is_empty() {
            return Ok(Vec::new());
        }

        let query = Query::new(&lang_info.language, lang_info.function_query)
            .map_err(|e| anyhow!("Failed to create query: {}", e))?;

        let mut cursor = QueryCursor::new();
        let mut symbols = Vec::new();

        let mut matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

        while let Some(m) = matches.next() {
            if let Some(capture) = m.captures.first() {
                let node = capture.node;
                let symbol = self.node_to_symbol(
                    node,
                    content,
                    path,
                    if lang_info.function_query.contains("method_definition") {
                        SymbolKind::Method
                    } else {
                        SymbolKind::Function
                    },
                )?;
                symbols.push(symbol);
            }
        }

        Ok(symbols)
    }

    /// Extract class/struct symbols
    fn extract_classes(
        &self,
        tree: &Tree,
        content: &str,
        path: &Path,
        lang_info: &LanguageInfo,
    ) -> Result<Vec<CodeSymbol>> {
        if lang_info.class_query.is_empty() {
            return Ok(Vec::new());
        }

        let query = Query::new(&lang_info.language, lang_info.class_query)
            .map_err(|e| anyhow!("Failed to create query: {}", e))?;

        let mut cursor = QueryCursor::new();
        let mut symbols = Vec::new();

        let mut matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

        while let Some(m) = matches.next() {
            if let Some(capture) = m.captures.first() {
                let node = capture.node;
                let kind = if node.kind().contains("struct") {
                    SymbolKind::Struct
                } else if node.kind().contains("enum") {
                    SymbolKind::Enum
                } else {
                    SymbolKind::Class
                };
                let symbol = self.node_to_symbol(node, content, path, kind)?;
                symbols.push(symbol);
            }
        }

        Ok(symbols)
    }

    /// Extract interface/trait symbols
    fn extract_interfaces(
        &self,
        tree: &Tree,
        content: &str,
        path: &Path,
        lang_info: &LanguageInfo,
    ) -> Result<Vec<CodeSymbol>> {
        if lang_info.interface_query.is_empty() {
            return Ok(Vec::new());
        }

        let query = Query::new(&lang_info.language, lang_info.interface_query)
            .map_err(|e| anyhow!("Failed to create query: {}", e))?;

        let mut cursor = QueryCursor::new();
        let mut symbols = Vec::new();

        let mut matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

        while let Some(m) = matches.next() {
            if let Some(capture) = m.captures.first() {
                let node = capture.node;
                let kind = if node.kind().contains("trait") {
                    SymbolKind::Trait
                } else {
                    SymbolKind::Interface
                };
                let symbol = self.node_to_symbol(node, content, path, kind)?;
                symbols.push(symbol);
            }
        }

        Ok(symbols)
    }

    /// Extract type symbols
    fn extract_types(
        &self,
        tree: &Tree,
        content: &str,
        path: &Path,
        lang_info: &LanguageInfo,
    ) -> Result<Vec<CodeSymbol>> {
        if lang_info.type_query.is_empty() {
            return Ok(Vec::new());
        }

        let query = Query::new(&lang_info.language, lang_info.type_query)
            .map_err(|e| anyhow!("Failed to create query: {}", e))?;

        let mut cursor = QueryCursor::new();
        let mut symbols = Vec::new();

        let mut matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

        while let Some(m) = matches.next() {
            if let Some(capture) = m.captures.first() {
                let node = capture.node;
                let symbol = self.node_to_symbol(node, content, path, SymbolKind::Type)?;
                symbols.push(symbol);
            }
        }

        Ok(symbols)
    }

    /// Convert AST node to CodeSymbol
    fn node_to_symbol(
        &self,
        node: Node,
        content: &str,
        path: &Path,
        kind: SymbolKind,
    ) -> Result<CodeSymbol> {
        // Extract name
        let name = self.extract_name(node, content)?;

        // Extract signature
        let signature = self.extract_signature(node, content);

        // Calculate body hash
        let body = node.utf8_text(content.as_bytes())?;
        let body_hash = Hash::new(body.as_bytes());

        // Create location
        let start = node.start_position();
        let end = node.end_position();
        let location = Location::new(
            path.to_string_lossy().to_string(),
            start.row + 1, // tree-sitter uses 0-based rows
            end.row + 1,
            start.column,
            end.column,
        );

        // Extract doc comment
        let doc_comment = self.extract_doc_comment(node, content);

        // Calculate token cost (rough estimate: 1 token per 4 characters)
        let token_cost = TokenCount::new((body.len() / 4) as u32);

        // Calculate complexity (rough estimate based on node count)
        let complexity = self.calculate_complexity(node);

        Ok(CodeSymbol {
            id: SymbolId::generate(),
            name: name.to_string(),
            kind,
            signature,
            body_hash,
            location,
            references: Vec::new(),
            dependencies: Vec::new(),
            metadata: SymbolMetadata {
                complexity,
                token_cost,
                last_modified: None,
                authors: Vec::new(),
                doc_comment,
                test_coverage: 0.0,
                usage_frequency: 0,
            },
            embedding: None, // Will be populated by the indexer
        })
    }

    /// Extract symbol name from node
    fn extract_name(&self, node: Node, content: &str) -> Result<String> {
        // Try to find name node
        let name_node = node
            .children(&mut node.walk())
            .find(|n| {
                matches!(
                    n.kind(),
                    "identifier"
                        | "type_identifier"
                        | "property_identifier"
                        | "field_identifier"
                )
            })
            .ok_or_else(|| anyhow!("No name found for symbol"))?;

        Ok(name_node.utf8_text(content.as_bytes())?.to_string())
    }

    /// Extract signature from node
    fn extract_signature(&self, node: Node, content: &str) -> String {
        // Find signature parts (name + parameters + return type)
        let mut parts = Vec::new();

        for child in node.children(&mut node.walk()) {
            match child.kind() {
                "identifier" | "type_identifier" | "property_identifier" | "field_identifier"
                | "parameters" | "formal_parameters" | "parameter_list" | "return_type"
                | "type_annotation" => {
                    if let Ok(text) = child.utf8_text(content.as_bytes()) {
                        parts.push(text);
                    }
                }
                _ => {}
            }
        }

        parts.join(" ")
    }

    /// Extract doc comment
    fn extract_doc_comment(&self, node: Node, content: &str) -> Option<String> {
        // Look for comment nodes before this node
        let mut prev_sibling = node.prev_sibling()?;

        while prev_sibling.kind().contains("comment") {
            if let Ok(text) = prev_sibling.utf8_text(content.as_bytes()) {
                // Check if it's a doc comment (///, /**, etc.)
                if text.starts_with("///") || text.starts_with("/**") || text.starts_with("#") {
                    return Some(text.to_string());
                }
            }
            prev_sibling = prev_sibling.prev_sibling()?;
        }

        None
    }

    /// Calculate cyclomatic complexity
    fn calculate_complexity(&self, node: Node) -> u32 {
        let mut complexity = 1u32; // Base complexity

        let mut cursor = node.walk();
        let mut stack = vec![node];

        while let Some(current) = stack.pop() {
            // Count decision points
            match current.kind() {
                "if_expression" | "if_statement" | "while_statement" | "for_statement"
                | "match_expression" | "case_statement" | "catch_clause" => {
                    complexity += 1;
                }
                "||" | "&&" => {
                    complexity += 1;
                }
                _ => {}
            }

            // Add children to stack
            for child in current.children(&mut cursor) {
                stack.push(child);
            }
        }

        complexity
    }

    /// Extract references between symbols
    fn extract_references(
        &self,
        symbols: &mut [CodeSymbol],
        tree: &Tree,
        content: &str,
    ) -> Result<()> {
        // Build symbol name lookup
        let symbol_map: HashMap<String, usize> = symbols
            .iter()
            .enumerate()
            .map(|(idx, s)| (s.name.clone(), idx))
            .collect();

        // Walk the tree and find references
        let mut cursor = tree.root_node().walk();
        let mut stack = vec![tree.root_node()];

        while let Some(node) = stack.pop() {
            // Check if this is an identifier that references a symbol
            if matches!(
                node.kind(),
                "identifier" | "type_identifier" | "property_identifier"
            ) {
                if let Ok(name) = node.utf8_text(content.as_bytes()) {
                    if let Some(&target_idx) = symbol_map.get(name) {
                        // Determine reference kind
                        let kind = self.determine_reference_kind(node);

                        // Create reference
                        let start = node.start_position();
                        let end = node.end_position();
                        let reference = Reference {
                            symbol_id: symbols[target_idx].id.clone(),
                            location: Location::new(
                                symbols[0].location.file.clone(), // All from same file
                                start.row + 1,
                                end.row + 1,
                                start.column,
                                end.column,
                            ),
                            kind,
                        };

                        // Add reference to target symbol
                        symbols[target_idx].references.push(reference);
                    }
                }
            }

            // Add children to stack
            for child in node.children(&mut cursor) {
                stack.push(child);
            }
        }

        Ok(())
    }

    /// Determine reference kind from context
    fn determine_reference_kind(&self, node: Node) -> ReferenceKind {
        // Check parent node to determine context
        if let Some(parent) = node.parent() {
            match parent.kind() {
                "call_expression" | "call" => ReferenceKind::Call,
                "new_expression" => ReferenceKind::Instantiation,
                "type_identifier" | "type_annotation" => ReferenceKind::TypeReference,
                "use_declaration" | "import_statement" | "import_declaration" => {
                    ReferenceKind::Import
                }
                "impl_item" | "implementation" => ReferenceKind::Implementation,
                _ => ReferenceKind::TypeReference,
            }
        } else {
            ReferenceKind::TypeReference
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_rust_file() {
        let mut parser = TreeSitterParser::new().unwrap();

        let content = r#"
            /// A test function
            pub fn test_function(x: i32, y: i32) -> i32 {
                x + y
            }

            pub struct TestStruct {
                field: i32,
            }
        "#;

        let path = PathBuf::from("test.rs");
        let symbols = parser.parse_file(&path, content).unwrap();

        assert!(!symbols.is_empty());
        assert!(symbols.iter().any(|s| s.name == "test_function"));
        assert!(symbols.iter().any(|s| s.name == "TestStruct"));
    }

    #[test]
    fn test_parse_typescript_file() {
        let mut parser = TreeSitterParser::new().unwrap();

        let content = r#"
            function testFunction(x: number, y: number): number {
                return x + y;
            }

            class TestClass {
                field: number;
            }

            interface TestInterface {
                method(): void;
            }
        "#;

        let path = PathBuf::from("test.ts");
        let symbols = parser.parse_file(&path, content).unwrap();

        assert!(!symbols.is_empty());
        assert!(symbols.iter().any(|s| s.name == "testFunction"));
        assert!(symbols.iter().any(|s| s.name == "TestClass"));
        assert!(symbols.iter().any(|s| s.name == "TestInterface"));
    }

    #[test]
    fn test_complexity_calculation() {
        let mut parser = TreeSitterParser::new().unwrap();

        let content = r#"
            pub fn complex_function(x: i32) -> i32 {
                if x > 0 {
                    if x < 10 {
                        return x;
                    }
                }
                match x {
                    1 => 1,
                    2 => 2,
                    _ => 0,
                }
            }
        "#;

        let path = PathBuf::from("test.rs");
        let symbols = parser.parse_file(&path, content).unwrap();

        let func = symbols.iter().find(|s| s.name == "complex_function");
        assert!(func.is_some());
        assert!(func.unwrap().metadata.complexity > 1);
    }

    #[test]
    fn test_incremental_parsing_no_changes() {
        let mut parser = TreeSitterParser::new().unwrap();

        let content = r#"
            pub fn test_function(x: i32) -> i32 {
                x + 1
            }
        "#;

        let path = PathBuf::from("test.rs");

        // First parse - no cached tree
        let result1 = parser.parse_file_incremental(&path, content).unwrap();
        assert_eq!(result1.changed_ranges.len(), 1); // Everything changed on first parse

        // Second parse with same content
        let result2 = parser.parse_file_incremental(&path, content).unwrap();
        assert_eq!(result2.changed_ranges.len(), 0); // No changes
        assert_eq!(result1.symbols.len(), result2.symbols.len());
    }

    #[test]
    fn test_incremental_parsing_with_changes() {
        let mut parser = TreeSitterParser::new().unwrap();

        let initial_content = r#"
            pub fn test_function(x: i32) -> i32 {
                x + 1
            }
        "#;

        let modified_content = r#"
            pub fn test_function(x: i32) -> i32 {
                x + 2
            }

            pub fn new_function() -> i32 {
                42
            }
        "#;

        let path = PathBuf::from("test.rs");

        // First parse
        let result1 = parser.parse_file_incremental(&path, initial_content).unwrap();
        assert_eq!(result1.symbols.len(), 1);

        // Second parse with changes
        let result2 = parser.parse_file_incremental(&path, modified_content).unwrap();
        assert!(result2.changed_ranges.len() > 0); // Should have changes
        assert_eq!(result2.symbols.len(), 2); // Now has 2 functions
    }

    #[test]
    fn test_symbol_in_changed_range() {
        use crate::types::{Location, SymbolKind, SymbolMetadata, Hash, TokenCount};

        let symbol = CodeSymbol {
            id: SymbolId::generate(),
            name: "test".to_string(),
            kind: SymbolKind::Function,
            signature: "test()".to_string(),
            body_hash: Hash::new(b"test"),
            location: Location::new("test.rs".to_string(), 5, 10, 0, 0),
            references: Vec::new(),
            dependencies: Vec::new(),
            metadata: SymbolMetadata {
                complexity: 1,
                token_cost: TokenCount::new(10),
                last_modified: None,
                authors: Vec::new(),
                doc_comment: None,
                test_coverage: 0.0,
                usage_frequency: 0,
            },
            embedding: None,
        };

        // Range that overlaps
        let overlapping_range = Range {
            start_byte: 0,
            end_byte: 100,
            start_point: tree_sitter::Point { row: 4, column: 0 },
            end_point: tree_sitter::Point { row: 11, column: 0 },
        };

        // Range that doesn't overlap
        let non_overlapping_range = Range {
            start_byte: 0,
            end_byte: 50,
            start_point: tree_sitter::Point { row: 1, column: 0 },
            end_point: tree_sitter::Point { row: 3, column: 0 },
        };

        assert!(TreeSitterParser::symbol_in_changed_range(&symbol, &[overlapping_range]));
        assert!(!TreeSitterParser::symbol_in_changed_range(&symbol, &[non_overlapping_range]));
    }

    #[test]
    fn test_tree_cache_management() {
        let mut parser = TreeSitterParser::new().unwrap();

        let content = r#"
            pub fn test() {}
        "#;

        let path = PathBuf::from("test.rs");

        assert_eq!(parser.cached_tree_count(), 0);

        parser.parse_file(&path, content).unwrap();
        assert_eq!(parser.cached_tree_count(), 1);

        parser.clear_tree_cache(&path);
        assert_eq!(parser.cached_tree_count(), 0);

        parser.parse_file(&path, content).unwrap();
        assert_eq!(parser.cached_tree_count(), 1);

        parser.clear_all_tree_caches();
        assert_eq!(parser.cached_tree_count(), 0);
    }
}
