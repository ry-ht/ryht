//! Documentation templates
use crate::types::{CodeSymbol, SymbolKind};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocTemplate { pub name: String, pub format: String, pub template: String, pub description: String }
impl DocTemplate { pub fn render(&self, ctx: &HashMap<String, String>) -> String { let mut r = self.template.clone(); for (k, v) in ctx { r = r.replace(&format!("{{{{{}}}}}", k), v); } r } }

pub struct TemplateEngine { templates: HashMap<String, DocTemplate> }
impl TemplateEngine {
    pub fn new() -> Self { 
        let mut t = HashMap::new();
        t.insert("tsdoc_function".to_string(), DocTemplate { name: "tsdoc_function".to_string(), format: "TSDoc".to_string(), template: "/** {{description}} */".to_string(), description: "TSDoc function".to_string() });
        t.insert("rustdoc_function".to_string(), DocTemplate { name: "rustdoc_function".to_string(), format: "rustdoc".to_string(), template: "/// {{description}}".to_string(), description: "rustdoc function".to_string() });
        Self { templates: t }
    }
    pub fn add_template(&mut self, t: DocTemplate) { self.templates.insert(t.name.clone(), t); }
    pub fn get_template(&self, n: &str) -> Option<&DocTemplate> { self.templates.get(n) }
    pub fn select_template(&self, s: &CodeSymbol, f: &str) -> Option<&DocTemplate> {
        match (f.to_lowercase().as_str(), s.kind) {
            ("tsdoc" | "jsdoc", SymbolKind::Function | SymbolKind::Method) => self.templates.get("tsdoc_function"),
            ("rustdoc", SymbolKind::Function | SymbolKind::Method) => self.templates.get("rustdoc_function"),
            _ => None
        }
    }
    pub fn generate_context(&self, s: &CodeSymbol) -> HashMap<String, String> { let mut c = HashMap::new(); c.insert("description".to_string(), s.name.clone()); c }
    pub fn render_for_symbol(&self, s: &CodeSymbol, f: &str) -> Result<String> { let t = self.select_template(s, f).ok_or_else(|| anyhow::anyhow!("No template"))?; Ok(t.render(&self.generate_context(s))) }
}
impl Default for TemplateEngine { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{SymbolId, Hash, Location, SymbolMetadata};
    fn cs(n: &str, k: SymbolKind, s: &str) -> CodeSymbol { CodeSymbol { id: SymbolId::new(format!("t::{}", n)), name: n.to_string(), kind: k, signature: s.to_string(), body_hash: Hash("t".to_string()), location: Location { file: "/t.ts".to_string(), line_start: 1, line_end: 10, column_start: 0, column_end: 0 }, references: vec![], dependencies: vec![], metadata: SymbolMetadata::default(), embedding: None } }
    #[test] fn test_template_rendering() { let t = DocTemplate { name: "t".to_string(), format: "t".to_string(), template: "{{name}}".to_string(), description: "t".to_string() }; let mut c = HashMap::new(); c.insert("name".to_string(), "test".to_string()); assert_eq!(t.render(&c), "test"); }
    #[test] fn test_template_engine_has_builtin_templates() { let e = TemplateEngine::new(); assert!(e.get_template("tsdoc_function").is_some()); }
    #[test] fn test_select_tsdoc_function_template() { let e = TemplateEngine::new(); let t = e.select_template(&cs("f", SymbolKind::Function, "f()"), "tsdoc"); assert!(t.is_some()); }
    #[test] fn test_select_rustdoc_function_template() { let e = TemplateEngine::new(); let t = e.select_template(&cs("f", SymbolKind::Function, "f()"), "rustdoc"); assert!(t.is_some()); }
    #[test] fn test_generate_context() { let e = TemplateEngine::new(); let c = e.generate_context(&cs("f", SymbolKind::Function, "f()")); assert!(c.contains_key("description")); }
    #[test] fn test_render_for_symbol() { let e = TemplateEngine::new(); let r = e.render_for_symbol(&cs("f", SymbolKind::Function, "f()"), "tsdoc"); assert!(r.is_ok()); }
    #[test] fn test_add_custom_template() { let mut e = TemplateEngine::new(); e.add_template(DocTemplate { name: "c".to_string(), format: "c".to_string(), template: "c".to_string(), description: "c".to_string() }); assert!(e.get_template("c").is_some()); }
}
