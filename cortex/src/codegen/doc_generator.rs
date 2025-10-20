//! Documentation generation engine with comprehensive format support

use crate::types::{CodeSymbol, SymbolKind};
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocFormat {
    TSDoc,
    JSDoc,
    RustDoc,
    Markdown,
}

impl DocFormat {
    pub fn comment_prefix(&self) -> &'static str {
        match self {
            DocFormat::TSDoc | DocFormat::JSDoc => "/**",
            DocFormat::RustDoc => "///",
            DocFormat::Markdown => "",
        }
    }

    pub fn comment_suffix(&self) -> &'static str {
        match self {
            DocFormat::TSDoc | DocFormat::JSDoc => " */",
            _ => "",
        }
    }

    pub fn line_prefix(&self) -> &'static str {
        match self {
            DocFormat::TSDoc | DocFormat::JSDoc => " * ",
            DocFormat::RustDoc => "/// ",
            DocFormat::Markdown => "",
        }
    }

    pub fn param_tag(&self) -> &'static str {
        match self {
            DocFormat::TSDoc | DocFormat::JSDoc => "@param",
            DocFormat::RustDoc => "#",
            DocFormat::Markdown => "-",
        }
    }

    pub fn returns_tag(&self) -> &'static str {
        match self {
            DocFormat::TSDoc | DocFormat::JSDoc => "@returns",
            DocFormat::RustDoc => "#",
            DocFormat::Markdown => "**Returns:**",
        }
    }

    pub fn example_tag(&self) -> &'static str {
        match self {
            DocFormat::TSDoc | DocFormat::JSDoc => "@example",
            DocFormat::RustDoc => "# Examples",
            DocFormat::Markdown => "## Example",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DocTransformOptions {
    pub preserve_examples: bool,
    pub preserve_links: bool,
    pub preserve_formatting: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedDoc {
    pub content: String,
    pub format: DocFormat,
    pub is_enhanced: bool,
    pub metadata: DocMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocMetadata {
    pub symbol_name: String,
    pub symbol_kind: String,
    pub generated_at: String,
    pub has_parameters: bool,
    pub has_return: bool,
    pub has_examples: bool,
    pub parameter_count: usize,
    pub description_lines: usize,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub type_annotation: Option<String>,
    pub is_optional: bool,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReturnType {
    pub type_annotation: String,
    pub is_async: bool,
}

pub struct DocumentationGenerator {
    format: DocFormat,
}

impl DocumentationGenerator {
    pub fn new(format: DocFormat) -> Self {
        Self { format }
    }

    /// Generate comprehensive documentation for a symbol
    pub fn generate(&self, symbol: &CodeSymbol) -> Result<GeneratedDoc> {
        let parameters = self.extract_parameters(&symbol.signature);
        let return_type = self.extract_return_type(&symbol.signature);
        let description = self.generate_description(symbol);

        let mut lines = Vec::new();
        let description_lines = description.lines().count();

        // Add description
        if !description.is_empty() {
            lines.push(description);
            lines.push(String::new());
        }

        // Add parameters
        if !parameters.is_empty() {
            for param in &parameters {
                lines.push(self.format_parameter(param));
            }
            lines.push(String::new());
        }

        // Add return type
        if let Some(ret) = &return_type {
            lines.push(self.format_return_type(ret));
            lines.push(String::new());
        }

        // Add example template
        if matches!(
            symbol.kind,
            SymbolKind::Function | SymbolKind::Method | SymbolKind::Class
        ) {
            lines.push(self.format_example(symbol, &parameters));
        }

        let content = self.format_documentation(&lines.join("\n"));

        Ok(GeneratedDoc {
            content,
            format: self.format,
            is_enhanced: false,
            metadata: DocMetadata {
                symbol_name: symbol.name.clone(),
                symbol_kind: symbol.kind.as_str().to_string(),
                generated_at: chrono::Utc::now().to_rfc3339(),
                has_parameters: !parameters.is_empty(),
                has_return: return_type.is_some(),
                has_examples: true,
                parameter_count: parameters.len(),
                description_lines,
            },
        })
    }

    /// Enhance existing documentation with missing elements
    pub fn enhance(&self, existing: &str, symbol: &CodeSymbol) -> Result<GeneratedDoc> {
        let parameters = self.extract_parameters(&symbol.signature);
        let return_type = self.extract_return_type(&symbol.signature);

        let mut enhanced_lines = Vec::new();
        let existing_clean = self.strip_comment_markers(existing);

        // Check what's missing
        let has_params = existing.contains("@param") || existing.contains("# Arguments");
        let has_return = existing.contains("@returns")
            || existing.contains("@return")
            || existing.contains("# Returns");
        let has_example = existing.contains("@example")
            || existing.contains("# Example")
            || existing.contains("```");

        // Keep existing content
        enhanced_lines.push(existing_clean.clone());

        // Add missing parameters
        if !has_params && !parameters.is_empty() {
            enhanced_lines.push(String::new());
            for param in &parameters {
                enhanced_lines.push(self.format_parameter(param));
            }
        }

        // Add missing return type
        if !has_return {
            if let Some(ret) = &return_type {
                enhanced_lines.push(String::new());
                enhanced_lines.push(self.format_return_type(ret));
            }
        }

        // Add missing example
        if !has_example
            && matches!(
                symbol.kind,
                SymbolKind::Function | SymbolKind::Method | SymbolKind::Class
            )
        {
            enhanced_lines.push(String::new());
            enhanced_lines.push(self.format_example(symbol, &parameters));
        }

        let content = self.format_documentation(&enhanced_lines.join("\n"));
        let description_lines = existing_clean.lines().count();

        Ok(GeneratedDoc {
            content,
            format: self.format,
            is_enhanced: true,
            metadata: DocMetadata {
                symbol_name: symbol.name.clone(),
                symbol_kind: symbol.kind.as_str().to_string(),
                generated_at: chrono::Utc::now().to_rfc3339(),
                has_parameters: !parameters.is_empty(),
                has_return: return_type.is_some(),
                has_examples: true,
                parameter_count: parameters.len(),
                description_lines,
            },
        })
    }

    /// Transform documentation from one format to another
    pub fn transform(
        &self,
        doc: &str,
        target_format: DocFormat,
        options: &DocTransformOptions,
    ) -> Result<String> {
        let clean_doc = self.strip_comment_markers(doc);
        let generator = DocumentationGenerator::new(target_format);

        let mut lines = Vec::new();

        for line in clean_doc.lines() {
            let trimmed = line.trim();

            // Transform parameter tags
            if trimmed.starts_with("@param") || trimmed.contains("# Arguments") {
                if options.preserve_formatting {
                    lines.push(self.transform_parameter_line(trimmed, target_format));
                } else {
                    lines.push(trimmed.to_string());
                }
            }
            // Transform return tags
            else if trimmed.starts_with("@returns")
                || trimmed.starts_with("@return")
                || trimmed.contains("# Returns")
            {
                if options.preserve_formatting {
                    lines.push(self.transform_return_line(trimmed, target_format));
                } else {
                    lines.push(trimmed.to_string());
                }
            }
            // Transform example tags
            else if trimmed.starts_with("@example") || trimmed.contains("# Example") {
                if options.preserve_examples {
                    lines.push(target_format.example_tag().to_string());
                }
            }
            // Preserve links if requested
            else if options.preserve_links && (trimmed.contains("http") || trimmed.contains("[")) {
                lines.push(trimmed.to_string());
            }
            // Keep other content
            else {
                lines.push(trimmed.to_string());
            }
        }

        Ok(generator.format_documentation(&lines.join("\n")))
    }

    /// Extract parameters from signature
    fn extract_parameters(&self, signature: &str) -> Vec<Parameter> {
        let mut params = Vec::new();

        // Find the parameter list
        if let Some(start) = signature.find('(') {
            if let Some(end) = signature.rfind(')') {
                let param_str = &signature[start + 1..end];
                if param_str.trim().is_empty() {
                    return params;
                }

                // Split by commas (simple approach)
                for param in param_str.split(',') {
                    let param = param.trim();
                    if param.is_empty() {
                        continue;
                    }

                    // Check for default value
                    let (param_part, default_value) = if let Some(eq_pos) = param.find('=') {
                        let default = param[eq_pos + 1..].trim().to_string();
                        (param[..eq_pos].trim(), Some(default))
                    } else {
                        (param, None)
                    };

                    // Extract name and type
                    if let Some(colon_pos) = param_part.find(':') {
                        let name_part = param_part[..colon_pos].trim();
                        let type_part = param_part[colon_pos + 1..].trim();

                        // Check for optional marker (?) in name like "b?"
                        let (name, name_optional) = if name_part.ends_with('?') {
                            (name_part[..name_part.len() - 1].trim().to_string(), true)
                        } else {
                            (name_part.to_string(), false)
                        };

                        // Check for optional marker in type
                        let (type_annotation, type_optional) = if type_part.ends_with('?') {
                            (type_part[..type_part.len() - 1].trim().to_string(), true)
                        } else if type_part.contains('?') {
                            (type_part.replace('?', "").trim().to_string(), true)
                        } else {
                            (type_part.to_string(), false)
                        };

                        let is_optional = name_optional || type_optional || default_value.is_some();

                        params.push(Parameter {
                            name,
                            type_annotation: Some(type_annotation),
                            is_optional,
                            default_value,
                        });
                    }
                }
            }
        }

        params
    }

    /// Extract return type from signature
    fn extract_return_type(&self, signature: &str) -> Option<ReturnType> {
        let is_async = signature.contains("async");

        // TypeScript/JavaScript style: ): type or -> type
        if let Some(pos) = signature.rfind("): ") {
            let after = &signature[pos + 3..];
            let type_str = after
                .split(['{', ';'])
                .next()
                .unwrap_or("")
                .trim();
            if !type_str.is_empty() && type_str != "void" {
                return Some(ReturnType {
                    type_annotation: type_str.to_string(),
                    is_async,
                });
            }
        }

        // Rust style: -> Type
        if let Some(pos) = signature.rfind("-> ") {
            let after = &signature[pos + 3..];
            let type_str = after
                .split(['{', ';', ' '])
                .next()
                .unwrap_or("")
                .trim();
            if !type_str.is_empty() && type_str != "()" {
                return Some(ReturnType {
                    type_annotation: type_str.to_string(),
                    is_async,
                });
            }
        }

        None
    }

    /// Generate a description for the symbol
    fn generate_description(&self, symbol: &CodeSymbol) -> String {
        match symbol.kind {
            SymbolKind::Function | SymbolKind::Method => {
                format!("{} {}", self.get_kind_verb(symbol.kind), symbol.name)
            }
            SymbolKind::Class | SymbolKind::Interface | SymbolKind::Struct => {
                format!("{} representing {}", symbol.kind.as_str(), symbol.name)
            }
            SymbolKind::Type => format!("Type alias for {}", symbol.name),
            SymbolKind::Enum => format!("Enumeration of {} variants", symbol.name),
            SymbolKind::Constant | SymbolKind::Variable => {
                format!("{} {}", symbol.kind.as_str(), symbol.name)
            }
            _ => symbol.name.clone(),
        }
    }

    fn get_kind_verb(&self, kind: SymbolKind) -> &'static str {
        match kind {
            SymbolKind::Function => "Function to",
            SymbolKind::Method => "Method to",
            _ => "",
        }
    }

    /// Format a parameter for documentation
    fn format_parameter(&self, param: &Parameter) -> String {
        match self.format {
            DocFormat::TSDoc | DocFormat::JSDoc => {
                let type_str = param
                    .type_annotation
                    .as_ref()
                    .map(|t| format!("{{{}}}", t.replace('?', "")))
                    .unwrap_or_else(|| "{any}".to_string());
                let optional = if param.is_optional { " (optional)" } else { "" };
                format!("@param {} {}{}", type_str, param.name, optional)
            }
            DocFormat::RustDoc => {
                let type_str = param
                    .type_annotation
                    .as_ref()
                    .map(|t| format!(": {}", t))
                    .unwrap_or_default();
                format!("# Arguments\n* `{}{}` - Description", param.name, type_str)
            }
            DocFormat::Markdown => {
                let type_str = param
                    .type_annotation
                    .as_ref()
                    .map(|t| format!(" ({})", t))
                    .unwrap_or_default();
                format!("- **{}**{} - Description", param.name, type_str)
            }
        }
    }

    /// Format return type for documentation
    fn format_return_type(&self, return_type: &ReturnType) -> String {
        match self.format {
            DocFormat::TSDoc | DocFormat::JSDoc => {
                let async_prefix = if return_type.is_async {
                    "Promise<"
                } else {
                    ""
                };
                let async_suffix = if return_type.is_async { ">" } else { "" };
                format!(
                    "@returns {{{}{}{}}} Description",
                    async_prefix, return_type.type_annotation, async_suffix
                )
            }
            DocFormat::RustDoc => {
                format!("# Returns\n{}", return_type.type_annotation)
            }
            DocFormat::Markdown => {
                format!("**Returns:** `{}`", return_type.type_annotation)
            }
        }
    }

    /// Format an example for documentation
    fn format_example(&self, symbol: &CodeSymbol, params: &[Parameter]) -> String {
        match self.format {
            DocFormat::TSDoc | DocFormat::JSDoc => {
                let param_str = params
                    .iter()
                    .map(|p| {
                        p.default_value
                            .as_ref()
                            .cloned()
                            .unwrap_or_else(|| "value".to_string())
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                match symbol.kind {
                    SymbolKind::Function => {
                        format!("@example\n{}({})", symbol.name, param_str)
                    }
                    SymbolKind::Method => {
                        format!("@example\nobj.{}({})", symbol.name, param_str)
                    }
                    SymbolKind::Class => {
                        format!("@example\nconst instance = new {}()", symbol.name)
                    }
                    _ => "@example\n// Example usage".to_string(),
                }
            }
            DocFormat::RustDoc => {
                let param_str = params
                    .iter()
                    .map(|p| {
                        p.default_value
                            .as_ref()
                            .cloned()
                            .unwrap_or_else(|| "value".to_string())
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                format!(
                    "# Examples\n```\nlet result = {}({});\n```",
                    symbol.name, param_str
                )
            }
            DocFormat::Markdown => {
                "## Example\n```\n// Example usage\n```".to_string()
            }
        }
    }

    /// Format documentation with proper comment markers
    fn format_documentation(&self, content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();

        result.push(self.format.comment_prefix().to_string());

        for line in lines {
            if line.trim().is_empty() {
                result.push(self.format.line_prefix().trim_end().to_string());
            } else {
                result.push(format!("{}{}", self.format.line_prefix(), line.trim()));
            }
        }

        if !self.format.comment_suffix().is_empty() {
            result.push(self.format.comment_suffix().to_string());
        }

        result.join("\n")
    }

    /// Strip comment markers from documentation
    fn strip_comment_markers(&self, doc: &str) -> String {
        doc.lines()
            .map(|line| {
                line.trim()
                    .trim_start_matches("/**")
                    .trim_start_matches("*/")
                    .trim_start_matches("*")
                    .trim_start_matches("///")
                    .trim()
            })
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Transform parameter line to target format
    fn transform_parameter_line(&self, line: &str, target: DocFormat) -> String {
        // Extract param name and type
        static PARAM_LINE_REGEX: OnceLock<Regex> = OnceLock::new();
        let regex =
            PARAM_LINE_REGEX.get_or_init(|| Regex::new(r"@param\s+\{([^}]+)\}\s+(\w+)").unwrap());

        if let Some(cap) = regex.captures(line) {
            let type_str = cap.get(1).map(|m| m.as_str()).unwrap_or("any");
            let name = cap.get(2).map(|m| m.as_str()).unwrap_or("param");

            match target {
                DocFormat::TSDoc | DocFormat::JSDoc => {
                    format!("@param {{{}}} {}", type_str, name)
                }
                DocFormat::RustDoc => {
                    format!("# Arguments\n* `{}: {}` - Description", name, type_str)
                }
                DocFormat::Markdown => {
                    format!("- **{}** ({}) - Description", name, type_str)
                }
            }
        } else {
            line.to_string()
        }
    }

    /// Transform return line to target format
    fn transform_return_line(&self, line: &str, target: DocFormat) -> String {
        static RETURN_LINE_REGEX: OnceLock<Regex> = OnceLock::new();
        let regex =
            RETURN_LINE_REGEX.get_or_init(|| Regex::new(r"@returns?\s+\{([^}]+)\}").unwrap());

        if let Some(cap) = regex.captures(line) {
            let type_str = cap.get(1).map(|m| m.as_str()).unwrap_or("any");

            match target {
                DocFormat::TSDoc | DocFormat::JSDoc => {
                    format!("@returns {{{}}} Description", type_str)
                }
                DocFormat::RustDoc => {
                    format!("# Returns\n{}", type_str)
                }
                DocFormat::Markdown => {
                    format!("**Returns:** `{}`", type_str)
                }
            }
        } else {
            line.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Hash, Location, SymbolId, SymbolKind, SymbolMetadata};

    fn create_test_symbol(name: &str, kind: SymbolKind, sig: &str) -> CodeSymbol {
        CodeSymbol {
            id: SymbolId::new(format!("test::{}", name)),
            name: name.to_string(),
            kind,
            signature: sig.to_string(),
            body_hash: Hash("test".to_string()),
            location: Location {
                file: "/t.ts".to_string(),
                line_start: 1,
                line_end: 10,
                column_start: 0,
                column_end: 0,
            },
            references: vec![],
            dependencies: vec![],
            metadata: SymbolMetadata::default(),
            embedding: None,
        }
    }

    #[test]
    fn test_tsdoc_generation() {
        let g = DocumentationGenerator::new(DocFormat::TSDoc);
        let r = g
            .generate(&create_test_symbol("f", SymbolKind::Function, "f()"))
            .unwrap();
        assert_eq!(r.format, DocFormat::TSDoc);
        assert!(r.content.contains("/**"));
        assert!(r.content.contains("*/"));
    }

    #[test]
    fn test_rustdoc_generation() {
        let g = DocumentationGenerator::new(DocFormat::RustDoc);
        let r = g
            .generate(&create_test_symbol("f", SymbolKind::Function, "fn f()"))
            .unwrap();
        assert_eq!(r.format, DocFormat::RustDoc);
        assert!(r.content.contains("///"));
    }

    #[test]
    fn test_parameter_extraction() {
        let g = DocumentationGenerator::new(DocFormat::TSDoc);
        let r = g
            .generate(&create_test_symbol(
                "f",
                SymbolKind::Function,
                "f(a: number, b: string)",
            ))
            .unwrap();
        assert!(r.metadata.has_parameters);
        assert_eq!(r.metadata.parameter_count, 2);
        assert!(r.content.contains("@param"));
    }

    #[test]
    fn test_return_type_extraction() {
        let g = DocumentationGenerator::new(DocFormat::TSDoc);
        let r = g
            .generate(&create_test_symbol(
                "f",
                SymbolKind::Function,
                "f(): string",
            ))
            .unwrap();
        assert!(r.metadata.has_return);
        assert!(r.content.contains("@returns"));
    }

    #[test]
    fn test_enhance() {
        let g = DocumentationGenerator::new(DocFormat::TSDoc);
        let r = g
            .enhance(
                "/** Basic doc */",
                &create_test_symbol("f", SymbolKind::Function, "f(x: number): string"),
            )
            .unwrap();
        assert!(r.is_enhanced);
        assert!(r.content.contains("@param"));
        assert!(r.content.contains("@returns"));
    }

    #[test]
    fn test_transform() {
        let g = DocumentationGenerator::new(DocFormat::RustDoc);
        let r = g
            .transform(
                "/** @param {number} x */",
                DocFormat::RustDoc,
                &DocTransformOptions::default(),
            )
            .unwrap();
        assert!(r.contains("///"));
    }

    #[test]
    fn test_class_documentation() {
        let g = DocumentationGenerator::new(DocFormat::TSDoc);
        let r = g
            .generate(&create_test_symbol("C", SymbolKind::Class, "class C"))
            .unwrap();
        assert!(r.content.contains("C"));
        assert!(r.content.contains("@example"));
    }

    #[test]
    fn test_optional_parameters() {
        let g = DocumentationGenerator::new(DocFormat::TSDoc);
        let params = g.extract_parameters("f(a: number, b?: string)");
        assert_eq!(params.len(), 2);
        assert!(!params[0].is_optional);
        assert!(params[1].is_optional);
    }

    #[test]
    fn test_default_parameters() {
        let g = DocumentationGenerator::new(DocFormat::TSDoc);
        let params = g.extract_parameters("f(a: number = 5)");
        assert_eq!(params.len(), 1);
        assert!(params[0].is_optional);
        assert_eq!(params[0].default_value, Some("5".to_string()));
    }

    #[test]
    fn test_async_return_type() {
        let g = DocumentationGenerator::new(DocFormat::TSDoc);
        let ret = g.extract_return_type("async function f(): Promise<string>");
        assert!(ret.is_some());
        let ret = ret.unwrap();
        assert!(ret.is_async);
    }

    #[test]
    fn test_rust_return_type() {
        let g = DocumentationGenerator::new(DocFormat::RustDoc);
        let ret = g.extract_return_type("fn f() -> Result<String>");
        assert!(ret.is_some());
        assert_eq!(ret.unwrap().type_annotation, "Result<String>");
    }

    #[test]
    fn test_empty_parameters() {
        let g = DocumentationGenerator::new(DocFormat::TSDoc);
        let r = g
            .generate(&create_test_symbol("f", SymbolKind::Function, "f()"))
            .unwrap();
        assert!(!r.metadata.has_parameters);
        assert_eq!(r.metadata.parameter_count, 0);
    }

    #[test]
    fn test_doc_format_markers() {
        assert_eq!(DocFormat::TSDoc.comment_prefix(), "/**");
        assert_eq!(DocFormat::RustDoc.comment_prefix(), "///");
        assert_eq!(DocFormat::JSDoc.comment_suffix(), " */");
    }

    #[test]
    fn test_strip_comment_markers() {
        let g = DocumentationGenerator::new(DocFormat::TSDoc);
        let stripped = g.strip_comment_markers("/**\n * Hello\n * World\n */");
        assert_eq!(stripped, "Hello\nWorld");
    }

    #[test]
    fn test_example_generation() {
        let g = DocumentationGenerator::new(DocFormat::TSDoc);
        let r = g
            .generate(&create_test_symbol(
                "add",
                SymbolKind::Function,
                "add(a: number, b: number): number",
            ))
            .unwrap();
        assert!(r.metadata.has_examples);
        assert!(r.content.contains("@example"));
    }

    #[test]
    fn test_interface_documentation() {
        let g = DocumentationGenerator::new(DocFormat::TSDoc);
        let r = g
            .generate(&create_test_symbol(
                "User",
                SymbolKind::Interface,
                "interface User",
            ))
            .unwrap();
        assert!(r.content.contains("User"));
    }

    #[test]
    fn test_enum_documentation() {
        let g = DocumentationGenerator::new(DocFormat::RustDoc);
        let r = g
            .generate(&create_test_symbol("Status", SymbolKind::Enum, "enum Status"))
            .unwrap();
        assert!(r.content.contains("Status"));
    }

    #[test]
    fn test_markdown_format() {
        let g = DocumentationGenerator::new(DocFormat::Markdown);
        let r = g
            .generate(&create_test_symbol(
                "f",
                SymbolKind::Function,
                "f(x: number): string",
            ))
            .unwrap();
        assert_eq!(r.format, DocFormat::Markdown);
        assert!(r.content.contains("**Returns:**"));
    }
}
