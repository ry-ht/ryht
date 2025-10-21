//! AST Editor - Tree-sitter based code manipulation and refactoring.
//!
//! This module provides high-level AST editing capabilities for safe code
//! transformations including insertion, replacement, deletion, and refactoring
//! operations like function extraction and renaming.

use anyhow::{anyhow, bail, Context, Result};
use std::cmp::min;
use std::collections::HashSet;
use tree_sitter::{Language, Node, Parser, Point, Tree};

/// Position in source code (line, column)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }

    pub fn to_point(&self) -> Point {
        Point {
            row: self.line,
            column: self.column,
        }
    }
}

/// Range in source code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    pub fn from_node(node: &Node) -> Self {
        Self {
            start: Position::new(node.start_position().row, node.start_position().column),
            end: Position::new(node.end_position().row, node.end_position().column),
        }
    }
}

/// Edit operation to be applied to source code
#[derive(Debug, Clone)]
pub struct Edit {
    pub range: Range,
    pub new_text: String,
}

impl Edit {
    pub fn new(range: Range, new_text: String) -> Self {
        Self { range, new_text }
    }

    pub fn insert(pos: Position, text: String) -> Self {
        Self {
            range: Range::new(pos, pos),
            new_text: text,
        }
    }

    pub fn delete(range: Range) -> Self {
        Self {
            range,
            new_text: String::new(),
        }
    }

    pub fn replace(range: Range, new_text: String) -> Self {
        Self { range, new_text }
    }
}

/// AST-based code editor
pub struct AstEditor {
    source: String,
    tree: Tree,
    parser: Parser,
    #[allow(dead_code)]
    language: Language,
    pub edits: Vec<Edit>,
}

impl AstEditor {
    /// Create a new AST editor for the given source code
    pub fn new(source: String, language: Language) -> Result<Self> {
        let mut parser = Parser::new();
        parser
            .set_language(&language)
            .context("Failed to set parser language")?;

        let tree = parser
            .parse(&source, None)
            .context("Failed to parse source code")?;

        Ok(Self {
            source,
            tree,
            parser,
            language,
            edits: Vec::new(),
        })
    }

    /// Get the current source code
    pub fn get_source(&self) -> &str {
        &self.source
    }

    /// Get the syntax tree
    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    /// Get the root node
    pub fn root_node(&self) -> Node<'_> {
        self.tree.root_node()
    }

    /// Insert code at a specific position
    pub fn insert_at(&mut self, line: usize, col: usize, code: &str) -> Result<()> {
        let pos = Position::new(line, col);
        self.edits.push(Edit::insert(pos, code.to_string()));
        Ok(())
    }

    /// Replace a node with new code
    pub fn replace_node(&mut self, node: &Node, new_code: &str) -> Result<()> {
        let range = Range::from_node(node);
        self.edits.push(Edit::replace(range, new_code.to_string()));
        Ok(())
    }

    /// Delete a node
    pub fn delete_node(&mut self, node: &Node) -> Result<()> {
        let range = Range::from_node(node);
        self.edits.push(Edit::delete(range));
        Ok(())
    }

    /// Rename a symbol (all occurrences in the current file)
    pub fn rename_symbol(&mut self, old_name: &str, new_name: &str) -> Result<Vec<Edit>> {
        let mut rename_edits = Vec::new();

        // Use simple tree traversal to find identifiers
        self.find_identifiers_recursive(self.root_node(), old_name, new_name, &mut rename_edits);

        self.edits.extend(rename_edits.clone());
        Ok(rename_edits)
    }

    /// Helper to recursively find and rename identifiers
    fn find_identifiers_recursive(
        &self,
        node: Node,
        old_name: &str,
        new_name: &str,
        edits: &mut Vec<Edit>,
    ) {
        // Match both "identifier" and "type_identifier" nodes
        // type_identifier is used for type names (struct, enum, type aliases, etc.)
        // identifier is used for variable/function names
        if node.kind() == "identifier" || node.kind() == "type_identifier" {
            let text = &self.source[node.byte_range()];
            if text == old_name {
                let range = Range::from_node(&node);
                edits.push(Edit::replace(range, new_name.to_string()));
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.find_identifiers_recursive(child, old_name, new_name, edits);
        }
    }

    /// Extract a code block into a new function
    pub fn extract_function(
        &mut self,
        start_line: usize,
        end_line: usize,
        function_name: &str,
    ) -> Result<ExtractFunctionResult> {
        // Get the code block to extract
        let lines: Vec<&str> = self.source.lines().collect();
        if start_line >= lines.len() || end_line >= lines.len() || start_line > end_line {
            bail!("Invalid line range");
        }

        let extracted_lines = &lines[start_line..=end_line];
        let extracted_code = extracted_lines.join("\n");

        // Analyze variables used in the extracted code
        let (params, return_vars) = self.analyze_variables(&extracted_code, start_line, end_line)?;

        // Build the new function
        let mut new_function = String::new();
        new_function.push_str(&format!("fn {}(", function_name));

        // Add parameters
        let param_list: Vec<String> = params
            .iter()
            .map(|(name, type_)| format!("{}: {}", name, type_))
            .collect();
        new_function.push_str(&param_list.join(", "));

        // Add return type if needed
        if !return_vars.is_empty() {
            if return_vars.len() == 1 {
                new_function.push_str(&format!(") -> {} {{\n", return_vars[0].1));
            } else {
                let return_types: Vec<String> = return_vars
                    .iter()
                    .map(|(_, type_)| type_.clone())
                    .collect();
                new_function.push_str(&format!(") -> ({}) {{\n", return_types.join(", ")));
            }
        } else {
            new_function.push_str(") {\n");
        }

        // Add the extracted code
        new_function.push_str(&extracted_code);
        new_function.push('\n');

        // Add return statement if needed
        if !return_vars.is_empty() {
            if return_vars.len() == 1 {
                new_function.push_str(&format!("    {}\n", return_vars[0].0));
            } else {
                let return_names: Vec<String> = return_vars
                    .iter()
                    .map(|(name, _)| name.clone())
                    .collect();
                new_function.push_str(&format!("    ({})\n", return_names.join(", ")));
            }
        }

        new_function.push_str("}\n\n");

        // Replace the extracted code with a function call
        let call_args: Vec<String> = params.iter().map(|(name, _)| name.clone()).collect();
        let function_call = format!("{}({})", function_name, call_args.join(", "));

        let call_statement = if !return_vars.is_empty() {
            if return_vars.len() == 1 {
                format!("let {} = {};", return_vars[0].0, function_call)
            } else {
                let return_names: Vec<String> = return_vars
                    .iter()
                    .map(|(name, _)| name.clone())
                    .collect();
                format!("let ({}) = {};", return_names.join(", "), function_call)
            }
        } else {
            format!("{};", function_call)
        };

        // Create edits
        let start_pos = Position::new(start_line, 0);
        let end_pos = Position::new(end_line + 1, 0);
        self.edits.push(Edit::replace(
            Range::new(start_pos, end_pos),
            format!("    {}\n", call_statement),
        ));

        // Insert the new function (we'll insert it before the containing function)
        let insert_pos = self.find_function_insert_position(start_line)?;
        self.edits
            .push(Edit::insert(insert_pos, new_function.clone()));

        Ok(ExtractFunctionResult {
            function_name: function_name.to_string(),
            parameters: params,
            return_type: if return_vars.is_empty() {
                None
            } else if return_vars.len() == 1 {
                Some(return_vars[0].1.clone())
            } else {
                let types: Vec<String> = return_vars.iter().map(|(_, t)| t.clone()).collect();
                Some(format!("({})", types.join(", ")))
            },
            function_code: new_function,
        })
    }

    /// Find the best position to insert a new function
    fn find_function_insert_position(&self, _near_line: usize) -> Result<Position> {
        // For simplicity, insert at the beginning of the file
        // In a real implementation, this would find the appropriate module/impl block
        Ok(Position::new(0, 0))
    }

    /// Analyze variables in a code block to determine parameters and return values
    fn analyze_variables(
        &self,
        _code: &str,
        _start_line: usize,
        _end_line: usize,
    ) -> Result<(Vec<(String, String)>, Vec<(String, String)>)> {
        // This is a simplified analysis
        // In a real implementation, this would use tree-sitter to analyze scope and usage

        let params = Vec::new();
        let return_vars = Vec::new();

        // For now, return empty lists (this would be filled in with real analysis)
        // A real implementation would:
        // 1. Find all variables used in the code block
        // 2. Determine which are defined outside (parameters)
        // 3. Determine which are used after the block (return values)
        // 4. Infer or look up types

        Ok((params, return_vars))
    }

    /// Apply all pending edits and update the AST
    pub fn apply_edits(&mut self) -> Result<()> {
        if self.edits.is_empty() {
            return Ok(());
        }

        // Convert position-based edits to byte-based edits
        let mut byte_edits: Vec<(usize, usize, String)> = Vec::new();

        for edit in &self.edits {
            let start_byte = self.position_to_byte(edit.range.start)?;
            let end_byte = self.position_to_byte(edit.range.end)?;
            byte_edits.push((start_byte, end_byte, edit.new_text.clone()));
        }

        // Sort edits by position (reverse order so we can apply them without shifting positions)
        byte_edits.sort_by(|a, b| b.0.cmp(&a.0));

        let mut new_source = self.source.clone();

        // Apply edits in reverse order
        for (start_byte, end_byte, new_text) in byte_edits {
            let start = min(start_byte, new_source.len());
            let end = min(end_byte, new_source.len());

            new_source.replace_range(start..end, &new_text);
        }

        // Re-parse the modified source
        self.tree = self
            .parser
            .parse(&new_source, Some(&self.tree))
            .context("Failed to re-parse after edits")?;

        self.source = new_source;
        self.edits.clear();

        Ok(())
    }

    /// Convert a position (line, column) to a byte offset
    fn position_to_byte(&self, pos: Position) -> Result<usize> {
        let lines: Vec<&str> = self.source.lines().collect();

        let mut byte_offset = 0;

        // Add bytes for all complete lines before the target line
        for (i, line) in lines.iter().enumerate() {
            if i < pos.line {
                byte_offset += line.len() + 1; // +1 for newline
            } else {
                break;
            }
        }

        // If the line doesn't exist yet, return the end of the file
        if pos.line >= lines.len() {
            return Ok(self.source.len());
        }

        // Add the column offset within the target line
        let line = lines[pos.line];
        let col = min(pos.column, line.len());
        byte_offset += col;

        Ok(byte_offset)
    }

    /// Get a node by path (e.g., "function_item:0.identifier:0")
    pub fn find_node_by_path(&self, path: &str) -> Result<Node<'_>> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = self.root_node();

        for part in parts {
            let (kind, index) = if let Some(pos) = part.find(':') {
                let kind = &part[..pos];
                let index: usize = part[pos + 1..]
                    .parse()
                    .context("Invalid node index in path")?;
                (kind, index)
            } else {
                (part, 0)
            };

            let mut found = None;
            let mut count = 0;

            let mut cursor = current.walk();
            for child in current.children(&mut cursor) {
                if child.kind() == kind {
                    if count == index {
                        found = Some(child);
                        break;
                    }
                    count += 1;
                }
            }

            current = found.ok_or_else(|| anyhow!("Node not found in path: {}", path))?;
        }

        Ok(current)
    }

    /// Find all nodes matching a query (simplified version using tree traversal)
    pub fn query(&self, query_str: &str) -> Result<Vec<Node<'_>>> {
        // For now, support simple node kind matching like "(function_item) @func"
        // Extract the node kind from the query
        let node_kind = if query_str.starts_with('(') {
            let end = query_str.find(')').unwrap_or(query_str.len());
            &query_str[1..end]
        } else {
            return Ok(Vec::new());
        };

        let mut nodes = Vec::new();
        self.find_nodes_by_kind_recursive(self.root_node(), node_kind, &mut nodes);
        Ok(nodes)
    }

    /// Helper to recursively find nodes by kind
    fn find_nodes_by_kind_recursive<'a>(&self, node: Node<'a>, kind: &str, nodes: &mut Vec<Node<'a>>) {
        if node.kind() == kind {
            nodes.push(node);
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.find_nodes_by_kind_recursive(child, kind, nodes);
        }
    }

    /// Get the text of a node
    pub fn node_text(&self, node: &Node) -> &str {
        &self.source[node.byte_range()]
    }

    /// Add an import statement (Rust-specific)
    pub fn add_import_rust(&mut self, import_path: &str) -> Result<()> {
        // Find existing imports
        let import_query = "(use_declaration) @import";
        let imports = self.query(import_query)?;

        let import_stmt = format!("use {};\n", import_path);

        if imports.is_empty() {
            // No imports yet, add at the beginning
            self.insert_at(0, 0, &import_stmt)?;
        } else {
            // Add after the last import
            let last_import = imports.last().unwrap();
            let line = last_import.end_position().row + 1;
            self.insert_at(line, 0, &import_stmt)?;
        }

        Ok(())
    }

    /// Optimize imports (remove duplicates, sort)
    pub fn optimize_imports_rust(&mut self) -> Result<OptimizeImportsResult> {
        let import_query = "(use_declaration) @import";
        let imports = self.query(import_query)?;

        // Collect import info before mutating
        let import_data: Vec<(Range, String)> = imports
            .iter()
            .map(|node| (Range::from_node(node), self.node_text(node).to_string()))
            .collect();

        let mut import_texts: Vec<String> = import_data.iter().map(|(_, text)| text.clone()).collect();

        let original_count = import_texts.len();

        // Remove duplicates
        let mut seen = HashSet::new();
        import_texts.retain(|import| seen.insert(import.clone()));

        let removed_count = original_count - import_texts.len();

        // Sort imports
        import_texts.sort();

        // Delete all old imports (in reverse order to avoid position shifts)
        for (range, _) in import_data.iter().rev() {
            self.edits.push(Edit::delete(*range));
        }

        // Insert sorted imports
        if !import_texts.is_empty() {
            let sorted_imports = import_texts.join("\n") + "\n";
            self.insert_at(0, 0, &sorted_imports)?;
        }

        Ok(OptimizeImportsResult {
            removed: removed_count,
            sorted: true,
            grouped: false,
        })
    }

    /// Change function signature
    pub fn change_signature_rust(
        &mut self,
        function_name: &str,
        new_params: Vec<(String, String)>,
        new_return_type: Option<String>,
    ) -> Result<()> {
        // Find the function - first get all functions, then filter by name
        let function_query = "(function_item) @function";
        let all_functions = self.query(&function_query)?;

        // Filter to find the function with the matching name
        let mut function_node = None;
        for func in all_functions {
            // Look for the name field of the function
            let mut cursor = func.walk();
            for child in func.children(&mut cursor) {
                if child.kind() == "identifier" {
                    let name = self.node_text(&child);
                    if name == function_name {
                        function_node = Some(func);
                        break;
                    }
                }
            }
            if function_node.is_some() {
                break;
            }
        }

        let function_node = function_node.ok_or_else(|| anyhow!("Function '{}' not found", function_name))?;

        // Collect node info before mutating
        let function_range = Range::from_node(&function_node);
        let function_text = self.node_text(&function_node).to_string();

        // Build new signature
        let mut new_sig = format!("fn {}(", function_name);
        let params: Vec<String> = new_params
            .iter()
            .map(|(name, type_)| format!("{}: {}", name, type_))
            .collect();
        new_sig.push_str(&params.join(", "));
        new_sig.push(')');

        if let Some(ret_type) = new_return_type {
            new_sig.push_str(&format!(" -> {}", ret_type));
        }

        // Find the parameters and return type nodes to replace
        // This is simplified - a real implementation would precisely replace only the signature part
        if let Some(body_start) = function_text.find('{') {
            let body = &function_text[body_start..];
            let new_function = format!("{} {}", new_sig, body);
            self.edits.push(Edit::replace(function_range, new_function));
        }

        Ok(())
    }
}

/// Result of extracting a function
#[derive(Debug, Clone)]
pub struct ExtractFunctionResult {
    pub function_name: String,
    pub parameters: Vec<(String, String)>,
    pub return_type: Option<String>,
    pub function_code: String,
}

/// Result of optimizing imports
#[derive(Debug, Clone)]
pub struct OptimizeImportsResult {
    pub removed: usize,
    pub sorted: bool,
    pub grouped: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_ast_editor_rust() {
        let source = "fn main() {\n    println!(\"Hello\");\n}".to_string();
        let editor = AstEditor::new(source, tree_sitter_rust::LANGUAGE.into());
        assert!(editor.is_ok());
    }

    #[test]
    fn test_insert_at() {
        let source = "fn main() {}".to_string();
        let mut editor = AstEditor::new(source, tree_sitter_rust::LANGUAGE.into()).unwrap();
        let result = editor.insert_at(0, 0, "// Comment\n");
        assert!(result.is_ok());
    }

    #[test]
    fn test_rename_symbol() {
        let source = r#"
fn calculate(x: i32) -> i32 {
    let y = x + 1;
    y
}
        "#
        .to_string();
        let mut editor = AstEditor::new(source, tree_sitter_rust::LANGUAGE.into()).unwrap();
        let edits = editor.rename_symbol("calculate", "compute");
        assert!(edits.is_ok());
    }

    #[test]
    fn test_add_import_rust() {
        let source = "fn main() {}".to_string();
        let mut editor = AstEditor::new(source, tree_sitter_rust::LANGUAGE.into()).unwrap();
        let result = editor.add_import_rust("std::collections::HashMap");
        assert!(result.is_ok());
    }

    #[test]
    fn test_query_functions() {
        let source = r#"
fn foo() {}
fn bar() {}
        "#
        .to_string();
        let editor = AstEditor::new(source, tree_sitter_rust::LANGUAGE.into()).unwrap();
        let functions = editor.query("(function_item) @func");
        assert!(functions.is_ok());
        assert_eq!(functions.unwrap().len(), 2);
    }
}
