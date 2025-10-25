//! Rust-specific parsing using tree-sitter.

use crate::extractor::{
    calculate_complexity, extract_attributes, extract_docstring, extract_generics,
    extract_where_clause, NodeExtractor,
};
use crate::tree_sitter_wrapper::TreeSitterWrapper;
use crate::types::*;
use anyhow::{Context, Result};
use tree_sitter::Node;

/// Rust parser using tree-sitter.
pub struct RustParser {
    wrapper: TreeSitterWrapper,
}

impl RustParser {
    /// Create a new Rust parser.
    pub fn new() -> Result<Self> {
        let wrapper = TreeSitterWrapper::new(tree_sitter_rust::LANGUAGE.into())?;
        Ok(Self { wrapper })
    }

    /// Parse a Rust source file.
    pub fn parse_file(&mut self, path: &str, source: &str) -> Result<ParsedFile> {
        let tree = self.wrapper.parse(source)?;
        let root = tree.root_node();

        let mut parsed = ParsedFile::new(path.to_string());

        // Walk the tree and extract items
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            self.process_item(child, source, &mut parsed, vec![])?;
        }

        Ok(parsed)
    }

    /// Process a top-level item.
    fn process_item(
        &self,
        node: Node,
        source: &str,
        parsed: &mut ParsedFile,
        module_path: Vec<String>,
    ) -> Result<()> {
        match node.kind() {
            "function_item" => {
                let func = self.extract_function(node, source, &module_path)?;
                parsed.functions.push(func);
            }
            "struct_item" => {
                let struct_info = self.extract_struct(node, source, &module_path)?;
                parsed.structs.push(struct_info);
            }
            "enum_item" => {
                let enum_info = self.extract_enum(node, source, &module_path)?;
                parsed.enums.push(enum_info);
            }
            "trait_item" => {
                let trait_info = self.extract_trait(node, source, &module_path)?;
                parsed.traits.push(trait_info);
            }
            "impl_item" => {
                let impl_info = self.extract_impl(node, source, &module_path)?;
                // Also add the methods to the functions list for easier access
                for method in &impl_info.methods {
                    parsed.functions.push(method.clone());
                }
                parsed.impls.push(impl_info);
            }
            "mod_item" => {
                let mod_info = self.extract_module(node, source, &module_path)?;
                parsed.modules.push(mod_info);
            }
            "use_declaration" => {
                parsed.imports.push(node.text(source).to_string());
            }
            _ => {}
        }

        Ok(())
    }

    /// Extract function information.
    fn extract_function(
        &self,
        node: Node,
        source: &str,
        module_path: &[String],
    ) -> Result<FunctionInfo> {
        let name = node
            .child_by_field_name("name")
            .map(|n| n.text(source).to_string())
            .context("Function missing name")?;

        let qualified_name = if module_path.is_empty() {
            name.clone()
        } else {
            format!("{}::{}", module_path.join("::"), name)
        };

        let parameters = self.extract_parameters(node, source)?;
        let return_type = self.extract_return_type(node, source);
        let visibility = self.extract_visibility(node, source);
        let attributes = extract_attributes(node, source);
        let docstring = extract_docstring(node, source);

        let body = node
            .child_by_field_name("body")
            .map(|b| b.text(source).to_string())
            .unwrap_or_default();

        let start_line = node.start_line();
        let end_line = node.end_line();

        let is_async = node
            .children(&mut node.walk())
            .any(|c| c.kind() == "async");
        let is_const = node
            .children(&mut node.walk())
            .any(|c| c.kind() == "const");
        let is_unsafe = node
            .children(&mut node.walk())
            .any(|c| c.kind() == "unsafe");

        let generics = extract_generics(node, source);
        let where_clause = extract_where_clause(node, source);

        let complexity = node
            .child_by_field_name("body")
            .map(calculate_complexity);

        Ok(FunctionInfo {
            name,
            qualified_name,
            parameters,
            return_type,
            visibility,
            attributes,
            body,
            start_line,
            end_line,
            docstring,
            is_async,
            is_const,
            is_unsafe,
            generics,
            where_clause,
            complexity,
        })
    }

    /// Extract function parameters.
    fn extract_parameters(&self, node: Node, source: &str) -> Result<Vec<Parameter>> {
        let mut params = Vec::new();

        if let Some(params_node) = node.child_by_field_name("parameters") {
            let mut cursor = params_node.walk();
            for child in params_node.children(&mut cursor) {
                if child.kind() == "parameter" || child.kind() == "self_parameter" {
                    let param = self.extract_parameter(child, source)?;
                    params.push(param);
                }
            }
        }

        Ok(params)
    }

    /// Extract a single parameter.
    fn extract_parameter(&self, node: Node, source: &str) -> Result<Parameter> {
        if node.kind() == "self_parameter" {
            let text = node.text(source);
            return Ok(Parameter {
                name: "self".to_string(),
                param_type: text.to_string(),
                default_value: None,
                is_self: true,
                is_mut: text.contains("mut"),
                is_reference: text.contains('&'),
            });
        }

        let name = node
            .child_by_field_name("pattern")
            .map(|n| {
                // Handle mutable patterns
                if n.kind() == "mut_pattern" {
                    n.child(1).map(|c| c.text(source)).unwrap_or("")
                } else {
                    n.text(source)
                }
            })
            .unwrap_or("")
            .to_string();

        let param_type = node
            .child_by_field_name("type")
            .map(|n| n.text(source).to_string())
            .unwrap_or_default();

        let is_mut = node
            .child_by_field_name("pattern")
            .map(|n| n.kind() == "mut_pattern")
            .unwrap_or(false);

        let is_reference = param_type.starts_with('&');

        Ok(Parameter {
            name,
            param_type,
            default_value: None,
            is_self: false,
            is_mut,
            is_reference,
        })
    }

    /// Extract return type.
    fn extract_return_type(&self, node: Node, source: &str) -> Option<String> {
        node.child_by_field_name("return_type")
            .map(|rt| rt.text(source).to_string())
    }

    /// Extract visibility.
    fn extract_visibility(&self, node: Node, source: &str) -> Visibility {
        if let Some(vis_node) = node
            .children(&mut node.walk())
            .find(|c| c.kind() == "visibility_modifier")
        {
            let vis_text = vis_node.text(source);
            match vis_text {
                "pub" => Visibility::Public,
                text if text.contains("pub(crate)") => Visibility::PublicCrate,
                text if text.contains("pub(super)") => Visibility::PublicSuper,
                text if text.starts_with("pub(in") => Visibility::PublicIn,
                _ => Visibility::Private,
            }
        } else {
            Visibility::Private
        }
    }

    /// Extract struct information.
    fn extract_struct(
        &self,
        node: Node,
        source: &str,
        module_path: &[String],
    ) -> Result<StructInfo> {
        let name = node
            .child_by_field_name("name")
            .map(|n| n.text(source).to_string())
            .context("Struct missing name")?;

        let qualified_name = if module_path.is_empty() {
            name.clone()
        } else {
            format!("{}::{}", module_path.join("::"), name)
        };

        let body = node.child_by_field_name("body");
        let is_tuple_struct = body.map(|b| b.kind() == "tuple_struct").unwrap_or(false);
        let is_unit_struct = body.is_none();

        let fields = if let Some(body_node) = body {
            self.extract_fields(body_node, source)?
        } else {
            Vec::new()
        };

        Ok(StructInfo {
            name,
            qualified_name,
            fields,
            visibility: self.extract_visibility(node, source),
            attributes: extract_attributes(node, source),
            start_line: node.start_line(),
            end_line: node.end_line(),
            docstring: extract_docstring(node, source),
            generics: extract_generics(node, source),
            where_clause: extract_where_clause(node, source),
            is_tuple_struct,
            is_unit_struct,
        })
    }

    /// Extract struct fields.
    fn extract_fields(&self, body_node: Node, source: &str) -> Result<Vec<Field>> {
        let mut fields = Vec::new();
        let mut cursor = body_node.walk();

        for child in body_node.children(&mut cursor) {
            if child.kind() == "field_declaration" {
                let name = child
                    .child_by_field_name("name")
                    .map(|n| n.text(source).to_string())
                    .unwrap_or_default();

                let field_type = child
                    .child_by_field_name("type")
                    .map(|t| t.text(source).to_string())
                    .unwrap_or_default();

                fields.push(Field {
                    name,
                    field_type,
                    visibility: self.extract_visibility(child, source),
                    attributes: extract_attributes(child, source),
                    docstring: extract_docstring(child, source),
                });
            }
        }

        Ok(fields)
    }

    /// Extract enum information.
    fn extract_enum(
        &self,
        node: Node,
        source: &str,
        module_path: &[String],
    ) -> Result<EnumInfo> {
        let name = node
            .child_by_field_name("name")
            .map(|n| n.text(source).to_string())
            .context("Enum missing name")?;

        let qualified_name = if module_path.is_empty() {
            name.clone()
        } else {
            format!("{}::{}", module_path.join("::"), name)
        };

        let variants = if let Some(body) = node.child_by_field_name("body") {
            self.extract_enum_variants(body, source)?
        } else {
            Vec::new()
        };

        Ok(EnumInfo {
            name,
            qualified_name,
            variants,
            visibility: self.extract_visibility(node, source),
            attributes: extract_attributes(node, source),
            start_line: node.start_line(),
            end_line: node.end_line(),
            docstring: extract_docstring(node, source),
            generics: extract_generics(node, source),
            where_clause: extract_where_clause(node, source),
        })
    }

    /// Extract enum variants.
    fn extract_enum_variants(&self, body: Node, source: &str) -> Result<Vec<EnumVariant>> {
        let mut variants = Vec::new();
        let mut cursor = body.walk();

        for child in body.children(&mut cursor) {
            if child.kind() == "enum_variant" {
                let name = child
                    .child_by_field_name("name")
                    .map(|n| n.text(source).to_string())
                    .unwrap_or_default();

                let fields = if let Some(body) = child.child_by_field_name("body") {
                    self.extract_fields(body, source)?
                } else {
                    Vec::new()
                };

                variants.push(EnumVariant {
                    name,
                    fields,
                    tuple_fields: Vec::new(),
                    discriminant: None,
                    attributes: extract_attributes(child, source),
                    docstring: extract_docstring(child, source),
                });
            }
        }

        Ok(variants)
    }

    /// Extract trait information.
    fn extract_trait(
        &self,
        node: Node,
        source: &str,
        module_path: &[String],
    ) -> Result<TraitInfo> {
        let name = node
            .child_by_field_name("name")
            .map(|n| n.text(source).to_string())
            .context("Trait missing name")?;

        let qualified_name = if module_path.is_empty() {
            name.clone()
        } else {
            format!("{}::{}", module_path.join("::"), name)
        };

        let mut methods = Vec::new();
        let mut associated_types = Vec::new();

        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                match child.kind() {
                    "function_item" | "function_signature_item" => {
                        if let Ok(func) = self.extract_function(child, source, module_path) {
                            methods.push(func);
                        }
                    }
                    "associated_type" => {
                        associated_types.push(child.text(source).to_string());
                    }
                    _ => {}
                }
            }
        }

        let is_unsafe = node
            .children(&mut node.walk())
            .any(|c| c.kind() == "unsafe");

        // Extract supertraits (trait bounds)
        let supertraits = self.extract_supertraits(node, source);

        Ok(TraitInfo {
            name,
            qualified_name,
            methods,
            associated_types,
            visibility: self.extract_visibility(node, source),
            attributes: extract_attributes(node, source),
            start_line: node.start_line(),
            end_line: node.end_line(),
            docstring: extract_docstring(node, source),
            generics: extract_generics(node, source),
            where_clause: extract_where_clause(node, source),
            supertraits,
            is_unsafe,
        })
    }

    /// Extract supertraits from a trait declaration.
    /// For example: `trait Extended: Base + Clone` -> ["Base", "Clone"]
    fn extract_supertraits(&self, node: Node, source: &str) -> Vec<String> {
        let mut supertraits = Vec::new();

        // Look for bounds field which contains the trait_bounds node
        if let Some(bounds) = node.child_by_field_name("bounds") {
            let mut cursor = bounds.walk();
            for child in bounds.children(&mut cursor) {
                match child.kind() {
                    "type_identifier" => {
                        // Simple trait name like "Base" or "Clone"
                        supertraits.push(child.text(source).to_string());
                    }
                    "scoped_type_identifier" => {
                        // Qualified trait name like "std::fmt::Debug"
                        supertraits.push(child.text(source).to_string());
                    }
                    "generic_type" => {
                        // Generic trait like "Iterator<Item=T>"
                        // Extract just the base type name
                        if let Some(type_node) = child.child_by_field_name("type") {
                            supertraits.push(type_node.text(source).to_string());
                        } else {
                            supertraits.push(child.text(source).to_string());
                        }
                    }
                    "lifetime" => {
                        // Lifetime bounds like 'static - include them
                        supertraits.push(child.text(source).to_string());
                    }
                    _ => {
                        // Skip operators like '+' and ':'
                        // These are separate nodes in the AST
                    }
                }
            }
        }

        supertraits
    }

    /// Extract impl block information.
    fn extract_impl(
        &self,
        node: Node,
        source: &str,
        module_path: &[String],
    ) -> Result<ImplInfo> {
        let type_name = node
            .child_by_field_name("type")
            .map(|n| n.text(source).to_string())
            .context("Impl missing type")?;

        let trait_name = node
            .child_by_field_name("trait")
            .map(|n| n.text(source).to_string());

        let mut methods = Vec::new();
        let mut associated_types = Vec::new();

        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            for child in body.children(&mut cursor) {
                match child.kind() {
                    "function_item" => {
                        let mut path = module_path.to_vec();
                        path.push(type_name.clone());
                        if let Ok(func) = self.extract_function(child, source, &path) {
                            methods.push(func);
                        }
                    }
                    "associated_type" => {
                        associated_types.push(child.text(source).to_string());
                    }
                    _ => {}
                }
            }
        }

        let is_unsafe = node
            .children(&mut node.walk())
            .any(|c| c.kind() == "unsafe");

        Ok(ImplInfo {
            type_name,
            trait_name,
            methods,
            associated_types,
            attributes: extract_attributes(node, source),
            start_line: node.start_line(),
            end_line: node.end_line(),
            generics: extract_generics(node, source),
            where_clause: extract_where_clause(node, source),
            is_unsafe,
        })
    }

    /// Extract module information.
    fn extract_module(
        &self,
        node: Node,
        source: &str,
        module_path: &[String],
    ) -> Result<ModuleInfo> {
        let name = node
            .child_by_field_name("name")
            .map(|n| n.text(source).to_string())
            .context("Module missing name")?;

        let qualified_name = if module_path.is_empty() {
            name.clone()
        } else {
            format!("{}::{}", module_path.join("::"), name)
        };

        let is_inline = node.child_by_field_name("body").is_some();

        Ok(ModuleInfo {
            name,
            qualified_name,
            visibility: self.extract_visibility(node, source),
            attributes: extract_attributes(node, source),
            start_line: node.start_line(),
            end_line: node.end_line(),
            docstring: extract_docstring(node, source),
            is_inline,
        })
    }
}

impl Default for RustParser {
    fn default() -> Self {
        Self::new().expect("Failed to create RustParser")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_function() {
        let source = "fn add(a: i32, b: i32) -> i32 { a + b }";
        let mut parser = RustParser::new().unwrap();
        let result = parser.parse_file("test.rs", source).unwrap();

        assert_eq!(result.functions.len(), 1);
        let func = &result.functions[0];
        assert_eq!(func.name, "add");
        assert_eq!(func.parameters.len(), 2);
        assert_eq!(func.parameters[0].name, "a");
        assert_eq!(func.parameters[0].param_type, "i32");
        assert_eq!(func.return_type, Some("i32".to_string()));
    }

    #[test]
    fn test_parse_pub_function() {
        let source = "pub fn hello() {}";
        let mut parser = RustParser::new().unwrap();
        let result = parser.parse_file("test.rs", source).unwrap();

        assert_eq!(result.functions.len(), 1);
        assert_eq!(result.functions[0].visibility, Visibility::Public);
    }

    #[test]
    fn test_parse_struct() {
        let source = r#"
struct Person {
    name: String,
    age: u32,
}
"#;
        let mut parser = RustParser::new().unwrap();
        let result = parser.parse_file("test.rs", source).unwrap();

        assert_eq!(result.structs.len(), 1);
        let s = &result.structs[0];
        assert_eq!(s.name, "Person");
        assert_eq!(s.fields.len(), 2);
        assert_eq!(s.fields[0].name, "name");
        assert_eq!(s.fields[0].field_type, "String");
    }

    #[test]
    fn test_parse_with_docstring() {
        let source = r#"
/// This is a test function.
/// It does something cool.
fn test() {}
"#;
        let mut parser = RustParser::new().unwrap();
        let result = parser.parse_file("test.rs", source).unwrap();

        assert_eq!(result.functions.len(), 1);
        assert!(result.functions[0].docstring.is_some());
        let doc = result.functions[0].docstring.as_ref().unwrap();
        assert!(doc.contains("test function"));
    }

    #[test]
    fn test_parse_with_generics() {
        let source = "fn generic<T, U>(x: T, y: U) -> T { x }";
        let mut parser = RustParser::new().unwrap();
        let result = parser.parse_file("test.rs", source).unwrap();

        assert_eq!(result.functions.len(), 1);
        let func = &result.functions[0];
        assert!(func.generics.len() >= 1); // Should have T and U
    }

    #[test]
    fn test_parse_trait_with_supertraits() {
        let source = r#"
trait Base {
    fn base_method(&self);
}

trait Extended: Base {
    fn extended_method(&self);
}

trait MultiExtended: Base + Clone {
    fn multi_method(&self);
}
"#;
        let mut parser = RustParser::new().unwrap();
        let result = parser.parse_file("test.rs", source).unwrap();

        assert_eq!(result.traits.len(), 3);

        // Base trait should have no supertraits
        let base = &result.traits[0];
        assert_eq!(base.name, "Base");
        assert_eq!(base.supertraits.len(), 0);

        // Extended trait should have one supertrait: Base
        let extended = &result.traits[1];
        assert_eq!(extended.name, "Extended");
        assert_eq!(extended.supertraits.len(), 1);
        assert_eq!(extended.supertraits[0], "Base");

        // MultiExtended trait should have two supertraits: Base and Clone
        let multi = &result.traits[2];
        assert_eq!(multi.name, "MultiExtended");
        assert_eq!(multi.supertraits.len(), 2);
        assert!(multi.supertraits.contains(&"Base".to_string()));
        assert!(multi.supertraits.contains(&"Clone".to_string()));
    }
}
