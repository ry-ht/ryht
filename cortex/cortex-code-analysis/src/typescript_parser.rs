//! TypeScript-specific parsing using tree-sitter.

use crate::extractor::NodeExtractor;
use crate::tree_sitter_wrapper::TreeSitterWrapper;
use crate::types::*;
use anyhow::{Context, Result};
use tree_sitter::Node;

/// TypeScript/JavaScript parser using tree-sitter.
pub struct TypeScriptParser {
    wrapper: TreeSitterWrapper,
}

impl TypeScriptParser {
    /// Create a new TypeScript parser.
    pub fn new() -> Result<Self> {
        let wrapper = TreeSitterWrapper::new(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())?;
        Ok(Self { wrapper })
    }

    /// Create a new JavaScript parser.
    pub fn new_javascript() -> Result<Self> {
        let wrapper = TreeSitterWrapper::new(tree_sitter_typescript::LANGUAGE_TSX.into())?;
        Ok(Self { wrapper })
    }

    /// Parse a TypeScript source file.
    pub fn parse_file(&mut self, path: &str, source: &str) -> Result<ParsedFile> {
        let tree = self.wrapper.parse(source)?;
        let root = tree.root_node();

        let mut parsed = ParsedFile::new(path.to_string());

        // Walk the tree and extract items
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            self.process_item(child, source, &mut parsed)?;
        }

        Ok(parsed)
    }

    /// Process a top-level item.
    fn process_item(&self, node: Node, source: &str, parsed: &mut ParsedFile) -> Result<()> {
        match node.kind() {
            "function_declaration" | "method_definition" => {
                if let Ok(func) = self.extract_function(node, source) {
                    parsed.functions.push(func);
                }
            }
            "class_declaration" => {
                if let Ok(class) = self.extract_class(node, source) {
                    parsed.structs.push(class);
                }
            }
            "interface_declaration" => {
                if let Ok(interface) = self.extract_interface(node, source) {
                    parsed.traits.push(interface);
                }
            }
            "import_statement" => {
                parsed.imports.push(node.text(source).to_string());
            }
            _ => {}
        }

        Ok(())
    }

    /// Extract function information.
    fn extract_function(&self, node: Node, source: &str) -> Result<FunctionInfo> {
        let name = node
            .child_by_field_name("name")
            .map(|n| n.text(source).to_string())
            .context("Function missing name")?;

        let parameters = self.extract_ts_parameters(node, source)?;
        let return_type = self.extract_ts_return_type(node, source);

        let body = node
            .child_by_field_name("body")
            .map(|b| b.text(source).to_string())
            .unwrap_or_default();

        let is_async = node
            .children(&mut node.walk())
            .any(|c| c.kind() == "async");

        Ok(FunctionInfo {
            name: name.clone(),
            qualified_name: name,
            parameters,
            return_type,
            visibility: Visibility::Public, // TypeScript doesn't have same visibility
            attributes: Vec::new(),
            body,
            start_line: node.start_line(),
            end_line: node.end_line(),
            docstring: None,
            is_async,
            is_const: false,
            is_unsafe: false,
            generics: Vec::new(),
            where_clause: None,
            complexity: None,
        })
    }

    /// Extract TypeScript parameters.
    fn extract_ts_parameters(&self, node: Node, source: &str) -> Result<Vec<Parameter>> {
        let mut params = Vec::new();

        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                if child.kind() == "required_parameter" || child.kind() == "optional_parameter" {
                    let name = child
                        .child_by_field_name("pattern")
                        .or_else(|| child.child(0))
                        .map(|n| n.text(source).to_string())
                        .unwrap_or_default();

                    let param_type = child
                        .child_by_field_name("type")
                        .map(|t| t.text(source).to_string())
                        .unwrap_or_else(|| "any".to_string());

                    params.push(Parameter {
                        name,
                        param_type,
                        default_value: None,
                        is_self: false,
                        is_mut: false,
                        is_reference: false,
                    });
                }
            }
        }

        Ok(params)
    }

    /// Extract TypeScript return type.
    fn extract_ts_return_type(&self, node: Node, source: &str) -> Option<String> {
        node.child_by_field_name("return_type")
            .map(|rt| rt.text(source).trim_start_matches(':').trim().to_string())
    }

    /// Extract class as a struct.
    fn extract_class(&self, node: Node, source: &str) -> Result<StructInfo> {
        let name = node
            .child_by_field_name("name")
            .map(|n| n.text(source).to_string())
            .context("Class missing name")?;

        let fields = if let Some(body) = node.child_by_field_name("body") {
            self.extract_class_fields(body, source)?
        } else {
            Vec::new()
        };

        Ok(StructInfo {
            name: name.clone(),
            qualified_name: name,
            fields,
            visibility: Visibility::Public,
            attributes: Vec::new(),
            start_line: node.start_line(),
            end_line: node.end_line(),
            docstring: None,
            generics: Vec::new(),
            where_clause: None,
            is_tuple_struct: false,
            is_unit_struct: false,
        })
    }

    /// Extract class fields.
    fn extract_class_fields(&self, body: Node, source: &str) -> Result<Vec<Field>> {
        let mut fields = Vec::new();
        let mut cursor = body.walk();

        for child in body.children(&mut cursor) {
            if child.kind() == "field_definition" || child.kind() == "public_field_definition" {
                let name = child
                    .child_by_field_name("property")
                    .map(|n| n.text(source).to_string())
                    .unwrap_or_default();

                let field_type = child
                    .child_by_field_name("type")
                    .map(|t| t.text(source).to_string())
                    .unwrap_or_else(|| "any".to_string());

                let visibility = if child.kind() == "public_field_definition" {
                    Visibility::Public
                } else {
                    Visibility::Private
                };

                fields.push(Field {
                    name,
                    field_type,
                    visibility,
                    attributes: Vec::new(),
                    docstring: None,
                });
            }
        }

        Ok(fields)
    }

    /// Extract interface as a trait.
    fn extract_interface(&self, node: Node, source: &str) -> Result<TraitInfo> {
        let name = node
            .child_by_field_name("name")
            .map(|n| n.text(source).to_string())
            .context("Interface missing name")?;

        Ok(TraitInfo {
            name: name.clone(),
            qualified_name: name,
            methods: Vec::new(),
            associated_types: Vec::new(),
            visibility: Visibility::Public,
            attributes: Vec::new(),
            start_line: node.start_line(),
            end_line: node.end_line(),
            docstring: None,
            generics: Vec::new(),
            where_clause: None,
            supertraits: Vec::new(),
            is_unsafe: false,
        })
    }
}

impl Default for TypeScriptParser {
    fn default() -> Self {
        Self::new().expect("Failed to create TypeScriptParser")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_typescript_function() {
        let source = "function add(a: number, b: number): number { return a + b; }";
        let mut parser = TypeScriptParser::new().unwrap();
        let result = parser.parse_file("test.ts", source).unwrap();

        assert_eq!(result.functions.len(), 1);
        let func = &result.functions[0];
        assert_eq!(func.name, "add");
        assert_eq!(func.parameters.len(), 2);
    }

    #[test]
    fn test_parse_typescript_class() {
        let source = r#"
class Person {
    name: string;
    age: number;
}
"#;
        let mut parser = TypeScriptParser::new().unwrap();
        let result = parser.parse_file("test.ts", source).unwrap();

        assert_eq!(result.structs.len(), 1);
        assert_eq!(result.structs[0].name, "Person");
    }

    #[test]
    fn test_parse_async_function() {
        let source = "async function fetchData() { return data; }";
        let mut parser = TypeScriptParser::new().unwrap();
        let result = parser.parse_file("test.ts", source).unwrap();

        assert_eq!(result.functions.len(), 1);
        assert!(result.functions[0].is_async);
    }
}
