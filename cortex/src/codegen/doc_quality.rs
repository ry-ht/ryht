//! Documentation quality validation with comprehensive scoring and suggestions

use crate::types::{CodeSymbol, SymbolKind};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityIssue {
    pub severity: Severity,
    pub category: String,
    pub message: String,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub suggestion_type: String,
    pub description: String,
    pub example: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityScore {
    pub overall: f32,
    pub completeness: f32,
    pub clarity: f32,
    pub accuracy: f32,
    pub compliance: f32,
    pub issues: Vec<QualityIssue>,
    pub suggestions: Vec<Suggestion>,
}

impl QualityScore {
    pub fn perfect() -> Self {
        Self {
            overall: 1.0,
            completeness: 1.0,
            clarity: 1.0,
            accuracy: 1.0,
            compliance: 1.0,
            issues: vec![],
            suggestions: vec![],
        }
    }

    /// Score is acceptable if overall >= 70%
    pub fn is_acceptable(&self) -> bool {
        self.overall >= 0.7
    }

    /// Score is good if overall >= 85%
    pub fn is_good(&self) -> bool {
        self.overall >= 0.85
    }

    /// Convert to 0-100 scale
    pub fn as_percentage(&self) -> u8 {
        (self.overall * 100.0) as u8
    }
}

pub struct QualityValidator {
    strict_mode: bool,
}

impl QualityValidator {
    pub fn new() -> Self {
        Self { strict_mode: false }
    }

    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    /// Assess documentation quality with comprehensive scoring
    pub fn assess(&self, doc: &str, symbol: &CodeSymbol) -> QualityScore {
        let mut issues = Vec::new();
        let mut suggestions = Vec::new();

        // Completeness score (0-1)
        let completeness = self.assess_completeness(doc, symbol, &mut issues);

        // Clarity score (0-1)
        let clarity = self.assess_clarity(doc, &mut issues);

        // Accuracy score (0-1)
        let accuracy = self.assess_accuracy(doc, symbol, &mut issues);

        // Compliance score (0-1)
        let compliance = self.assess_compliance(doc, symbol, &mut issues);

        // Calculate overall score (weighted average)
        let overall = (completeness * 0.35 + clarity * 0.25 + accuracy * 0.25 + compliance * 0.15)
            .max(0.0)
            .min(1.0);

        // Generate suggestions based on issues
        suggestions.extend(self.generate_suggestions(doc, symbol, &issues));

        QualityScore {
            overall,
            completeness,
            clarity,
            accuracy,
            compliance,
            issues,
            suggestions,
        }
    }

    /// Suggest improvements for documentation
    pub fn suggest_improvements(&self, doc: &str, symbol: &CodeSymbol) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        // Check for missing description
        if self.is_description_missing(doc) {
            suggestions.push(Suggestion {
                suggestion_type: "add_description".to_string(),
                description: "Add a clear description of what this symbol does".to_string(),
                example: Some(self.generate_description_example(symbol)),
            });
        }

        // Check for missing parameters
        if self.has_parameters(symbol) && !self.has_parameter_docs(doc) {
            suggestions.push(Suggestion {
                suggestion_type: "add_parameters".to_string(),
                description: "Document all parameters with their types and purposes".to_string(),
                example: Some("@param {Type} paramName - Description of parameter".to_string()),
            });
        }

        // Check for missing return documentation
        if self.has_return_type(symbol) && !self.has_return_docs(doc) {
            suggestions.push(Suggestion {
                suggestion_type: "add_return".to_string(),
                description: "Document the return value and its meaning".to_string(),
                example: Some("@returns {Type} Description of return value".to_string()),
            });
        }

        // Check for missing examples
        if !self.has_examples(doc)
            && matches!(
                symbol.kind,
                SymbolKind::Function | SymbolKind::Method | SymbolKind::Class
            )
        {
            suggestions.push(Suggestion {
                suggestion_type: "add_example".to_string(),
                description: "Add usage examples to help users understand how to use this symbol"
                    .to_string(),
                example: Some("@example\nconst result = myFunction(arg1, arg2);".to_string()),
            });
        }

        // Check for vague descriptions
        if self.has_vague_description(doc) {
            suggestions.push(Suggestion {
                suggestion_type: "improve_description".to_string(),
                description: "Make the description more specific and detailed".to_string(),
                example: None,
            });
        }

        // Check for missing error documentation
        if self.might_throw_errors(symbol) && !self.has_error_docs(doc) {
            suggestions.push(Suggestion {
                suggestion_type: "add_throws".to_string(),
                description: "Document possible errors or exceptions this symbol may throw"
                    .to_string(),
                example: Some("@throws {ErrorType} When invalid input is provided".to_string()),
            });
        }

        suggestions
    }

    /// Assess completeness of documentation
    fn assess_completeness(
        &self,
        doc: &str,
        symbol: &CodeSymbol,
        issues: &mut Vec<QualityIssue>,
    ) -> f32 {
        let mut score: f32 = 1.0;

        // Description is always required
        if !self.is_description_missing(doc) {
            // Description present
        } else {
            score -= 0.4;
            issues.push(QualityIssue {
                severity: Severity::Error,
                category: "completeness".to_string(),
                message: "Missing description".to_string(),
                line: None,
            });
        }

        // Parameters documentation
        if self.has_parameters(symbol) {
            if self.has_parameter_docs(doc) {
                // Parameters documented
            } else {
                score -= 0.3;
                issues.push(QualityIssue {
                    severity: Severity::Error,
                    category: "completeness".to_string(),
                    message: "Missing parameter documentation".to_string(),
                    line: None,
                });
            }
        }

        // Return type documentation
        if self.has_return_type(symbol) {
            if self.has_return_docs(doc) {
                // Return documented
            } else {
                score -= 0.2;
                issues.push(QualityIssue {
                    severity: if self.strict_mode {
                        Severity::Error
                    } else {
                        Severity::Warning
                    },
                    category: "completeness".to_string(),
                    message: "Missing return value documentation".to_string(),
                    line: None,
                });
            }
        }

        // Examples (optional but recommended for functions/methods)
        if matches!(
            symbol.kind,
            SymbolKind::Function | SymbolKind::Method | SymbolKind::Class
        ) {
            if self.has_examples(doc) {
                // Examples present
            } else {
                score -= 0.1;
                issues.push(QualityIssue {
                    severity: Severity::Info,
                    category: "completeness".to_string(),
                    message: "Missing usage examples".to_string(),
                    line: None,
                });
            }
        }

        score.max(0.0)
    }

    /// Assess clarity of documentation
    fn assess_clarity(&self, doc: &str, issues: &mut Vec<QualityIssue>) -> f32 {
        let mut score: f32 = 1.0;

        // Check minimum length
        let cleaned = self.strip_markers(doc);
        if cleaned.len() < 10 {
            score -= 0.3;
            issues.push(QualityIssue {
                severity: Severity::Warning,
                category: "clarity".to_string(),
                message: "Documentation is too brief".to_string(),
                line: None,
            });
        }

        // Check for vague terms
        if self.has_vague_description(doc) {
            score -= 0.2;
            issues.push(QualityIssue {
                severity: Severity::Info,
                category: "clarity".to_string(),
                message: "Description contains vague terms. Be more specific.".to_string(),
                line: None,
            });
        }

        // Check for proper sentence structure
        if !self.has_proper_sentences(doc) {
            score -= 0.1;
            issues.push(QualityIssue {
                severity: Severity::Info,
                category: "clarity".to_string(),
                message: "Description should use complete sentences".to_string(),
                line: None,
            });
        }

        // Check line length (too long is hard to read)
        if self.has_excessive_line_length(doc) {
            score -= 0.1;
            issues.push(QualityIssue {
                severity: Severity::Info,
                category: "clarity".to_string(),
                message: "Some lines are too long. Consider breaking into multiple lines."
                    .to_string(),
                line: None,
            });
        }

        score.max(0.0)
    }

    /// Assess accuracy of documentation
    fn assess_accuracy(&self, doc: &str, symbol: &CodeSymbol, issues: &mut Vec<QualityIssue>) -> f32 {
        let mut score: f32 = 1.0;

        // Check parameter count matches
        if self.has_parameters(symbol) {
            let sig_param_count = self.count_signature_parameters(symbol);
            let doc_param_count = self.count_documented_parameters(doc);

            if sig_param_count != doc_param_count {
                score -= 0.4;
                issues.push(QualityIssue {
                    severity: Severity::Error,
                    category: "accuracy".to_string(),
                    message: format!(
                        "Parameter count mismatch: signature has {} but docs have {}",
                        sig_param_count, doc_param_count
                    ),
                    line: None,
                });
            }
        }

        // Check for inconsistent naming
        if self.has_inconsistent_naming(doc, symbol) {
            score -= 0.2;
            issues.push(QualityIssue {
                severity: Severity::Warning,
                category: "accuracy".to_string(),
                message: "Documentation uses different names than the symbol signature".to_string(),
                line: None,
            });
        }

        // Check for outdated information markers
        if self.has_outdated_markers(doc) {
            score -= 0.1;
            issues.push(QualityIssue {
                severity: Severity::Warning,
                category: "accuracy".to_string(),
                message: "Documentation may contain outdated information (TODO, FIXME markers)"
                    .to_string(),
                line: None,
            });
        }

        score.max(0.0)
    }

    /// Assess compliance with documentation standards
    fn assess_compliance(
        &self,
        doc: &str,
        symbol: &CodeSymbol,
        issues: &mut Vec<QualityIssue>,
    ) -> f32 {
        let mut score: f32 = 1.0;

        // Detect format
        let format = self.detect_format(doc);

        // Check for proper comment markers
        if !self.has_proper_markers(doc, &format) {
            score -= 0.3;
            issues.push(QualityIssue {
                severity: Severity::Error,
                category: "compliance".to_string(),
                message: format!(
                    "Documentation does not follow {} format conventions",
                    format
                ),
                line: None,
            });
        }

        // Check for proper tag usage (TSDoc/JSDoc)
        if (format == "tsdoc" || format == "jsdoc") && !self.has_proper_tags(doc) {
            score -= 0.2;
            issues.push(QualityIssue {
                severity: Severity::Warning,
                category: "compliance".to_string(),
                message: "Use standard JSDoc/TSDoc tags (@param, @returns, etc.)".to_string(),
                line: None,
            });
        }

        // Check for public API documentation requirements
        if self.is_public_api(symbol) && !self.meets_public_api_requirements(doc) {
            score -= 0.2;
            issues.push(QualityIssue {
                severity: if self.strict_mode {
                    Severity::Error
                } else {
                    Severity::Warning
                },
                category: "compliance".to_string(),
                message: "Public API requires comprehensive documentation with examples"
                    .to_string(),
                line: None,
            });
        }

        score.max(0.0)
    }

    /// Generate suggestions based on issues
    fn generate_suggestions(
        &self,
        _doc: &str,
        symbol: &CodeSymbol,
        issues: &[QualityIssue],
    ) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        for issue in issues {
            match issue.category.as_str() {
                "completeness" => {
                    if issue.message.contains("description") {
                        suggestions.push(Suggestion {
                            suggestion_type: "add_description".to_string(),
                            description: "Add a clear, concise description".to_string(),
                            example: Some(self.generate_description_example(symbol)),
                        });
                    }
                    if issue.message.contains("parameter") {
                        suggestions.push(Suggestion {
                            suggestion_type: "add_parameters".to_string(),
                            description: "Document each parameter with type and description"
                                .to_string(),
                            example: Some("@param {Type} name - Description".to_string()),
                        });
                    }
                }
                "clarity" => {
                    if issue.message.contains("brief") {
                        suggestions.push(Suggestion {
                            suggestion_type: "expand_description".to_string(),
                            description: "Provide more details about behavior and use cases"
                                .to_string(),
                            example: None,
                        });
                    }
                }
                "accuracy" => {
                    if issue.message.contains("mismatch") {
                        suggestions.push(Suggestion {
                            suggestion_type: "fix_parameters".to_string(),
                            description: "Ensure all parameters are documented correctly".to_string(),
                            example: None,
                        });
                    }
                }
                _ => {}
            }
        }

        // Avoid duplicates
        suggestions.sort_by(|a, b| a.suggestion_type.cmp(&b.suggestion_type));
        suggestions.dedup_by(|a, b| a.suggestion_type == b.suggestion_type);

        suggestions
    }

    // Helper methods

    fn is_description_missing(&self, doc: &str) -> bool {
        let cleaned = self.strip_markers(doc);
        cleaned.trim().is_empty() || cleaned.len() < 5
    }

    fn has_parameters(&self, symbol: &CodeSymbol) -> bool {
        symbol.signature.contains('(')
            && !symbol.signature.contains("()")
            && self.count_signature_parameters(symbol) > 0
    }

    fn has_return_type(&self, symbol: &CodeSymbol) -> bool {
        if !matches!(symbol.kind, SymbolKind::Function | SymbolKind::Method) {
            return false;
        }

        // Check for explicit return type markers
        let has_return_marker = symbol.signature.contains("): ")
            || symbol.signature.contains("-> ")
            || (symbol.signature.contains(": ") && !symbol.signature.contains("("));

        // Exclude void/unit returns
        let is_void = symbol.signature.contains("void") || symbol.signature.ends_with("()");

        has_return_marker && !is_void
    }

    fn has_parameter_docs(&self, doc: &str) -> bool {
        doc.contains("@param")
            || doc.contains("# Arguments")
            || doc.contains("**Parameters:**")
    }

    fn has_return_docs(&self, doc: &str) -> bool {
        doc.contains("@returns")
            || doc.contains("@return")
            || doc.contains("# Returns")
            || doc.contains("**Returns:**")
    }

    fn has_examples(&self, doc: &str) -> bool {
        doc.contains("@example")
            || doc.contains("# Example")
            || doc.contains("## Example")
            || doc.contains("```")
    }

    fn has_error_docs(&self, doc: &str) -> bool {
        doc.contains("@throws") || doc.contains("@error") || doc.contains("# Errors")
    }

    fn has_vague_description(&self, doc: &str) -> bool {
        static VAGUE_TERMS: OnceLock<Regex> = OnceLock::new();
        let regex = VAGUE_TERMS
            .get_or_init(|| Regex::new(r"\b(does stuff|handles|manages|etc\.?|TODO|FIXME)\b").unwrap());

        regex.is_match(doc)
    }

    fn has_proper_sentences(&self, doc: &str) -> bool {
        let cleaned = self.strip_markers(doc);
        let first_line = cleaned.lines().next().unwrap_or("");

        // Check if first sentence starts with capital and ends with period
        if let Some(first_char) = first_line.chars().next() {
            first_char.is_uppercase() && first_line.contains('.')
        } else {
            false
        }
    }

    fn has_excessive_line_length(&self, doc: &str) -> bool {
        doc.lines().any(|line| line.len() > 120)
    }

    fn count_signature_parameters(&self, symbol: &CodeSymbol) -> usize {
        static PARAM_REGEX: OnceLock<Regex> = OnceLock::new();
        let regex = PARAM_REGEX.get_or_init(|| Regex::new(r"\w+\s*:\s*[^,)]+").unwrap());

        if let Some(start) = symbol.signature.find('(') {
            if let Some(end) = symbol.signature.rfind(')') {
                let param_str = &symbol.signature[start + 1..end];
                if param_str.trim().is_empty() {
                    return 0;
                }
                return regex.find_iter(param_str).count();
            }
        }
        0
    }

    fn count_documented_parameters(&self, doc: &str) -> usize {
        static PARAM_DOC_REGEX: OnceLock<Regex> = OnceLock::new();
        let regex = PARAM_DOC_REGEX.get_or_init(|| Regex::new(r"@param\s+").unwrap());

        regex.find_iter(doc).count()
    }

    fn has_inconsistent_naming(&self, _doc: &str, _symbol: &CodeSymbol) -> bool {
        // TODO: Implement parameter name matching
        false
    }

    fn has_outdated_markers(&self, doc: &str) -> bool {
        doc.contains("TODO") || doc.contains("FIXME") || doc.contains("XXX")
    }

    fn detect_format(&self, doc: &str) -> String {
        if doc.contains("/**") {
            "tsdoc".to_string()
        } else if doc.contains("///") {
            "rustdoc".to_string()
        } else if doc.contains("@param") || doc.contains("@returns") {
            "jsdoc".to_string()
        } else {
            "markdown".to_string()
        }
    }

    fn has_proper_markers(&self, doc: &str, format: &str) -> bool {
        match format {
            "tsdoc" | "jsdoc" => doc.contains("/**") && doc.contains("*/"),
            "rustdoc" => doc.contains("///"),
            _ => true,
        }
    }

    fn has_proper_tags(&self, doc: &str) -> bool {
        // If it has parameter docs, they should use @param
        if doc.contains("param") && !doc.contains("@param") {
            return false;
        }
        // If it has return docs, they should use @returns
        if doc.contains("return") && !doc.contains("@return") {
            return false;
        }
        true
    }

    fn is_public_api(&self, _symbol: &CodeSymbol) -> bool {
        // TODO: Check for public/private modifiers
        true // Assume public for now
    }

    fn meets_public_api_requirements(&self, doc: &str) -> bool {
        !self.is_description_missing(doc) && self.has_examples(doc)
    }

    fn might_throw_errors(&self, symbol: &CodeSymbol) -> bool {
        matches!(symbol.kind, SymbolKind::Function | SymbolKind::Method)
    }

    fn strip_markers(&self, doc: &str) -> String {
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
            .join(" ")
    }

    fn generate_description_example(&self, symbol: &CodeSymbol) -> String {
        match symbol.kind {
            SymbolKind::Function => {
                format!("Performs {} operation and returns the result", symbol.name)
            }
            SymbolKind::Method => {
                format!("Method to execute {} operation", symbol.name)
            }
            SymbolKind::Class => format!("Represents a {} in the system", symbol.name),
            SymbolKind::Interface => format!("Interface defining {} contract", symbol.name),
            _ => format!("Description for {}", symbol.name),
        }
    }
}

impl Default for QualityValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Hash, Location, SymbolId, SymbolKind, SymbolMetadata};

    fn cs(n: &str, k: SymbolKind, s: &str) -> CodeSymbol {
        CodeSymbol {
            id: SymbolId::new(format!("t::{}", n)),
            name: n.to_string(),
            kind: k,
            signature: s.to_string(),
            body_hash: Hash("t".to_string()),
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
    fn test_empty_documentation_score() {
        let v = QualityValidator::new();
        let s = v.assess("", &cs("f", SymbolKind::Function, "f()"));
        assert!(!s.is_acceptable());
        assert!(s.completeness < 0.7);
    }

    #[test]
    fn test_complete_documentation_score() {
        let v = QualityValidator::new();
        let doc = "/**\n * A complete function that does something useful.\n * @param {number} x - The input value\n * @returns {string} The result\n * @example\n * f(42)\n */";
        let s = v.assess(doc, &cs("f", SymbolKind::Function, "f(x: number): string"));
        assert!(s.is_acceptable());
        assert!(s.overall > 0.7);
    }

    #[test]
    fn test_missing_parameters_issue() {
        let v = QualityValidator::new();
        let s = v.assess(
            "/** Does something */",
            &cs("f", SymbolKind::Function, "f(x: i32, y: i32)"),
        );
        assert!(s.issues.iter().any(|i| i.message.contains("parameter")));
    }

    #[test]
    fn test_missing_return_issue() {
        let v = QualityValidator::new();
        let s = v.assess(
            "/** Does something */",
            &cs("f", SymbolKind::Function, "f(): String"),
        );
        assert!(s
            .issues
            .iter()
            .any(|i| i.message.contains("return") || i.category == "completeness"));
    }

    #[test]
    fn test_suggest_improvements() {
        let v = QualityValidator::new();
        let s = v.suggest_improvements("", &cs("f", SymbolKind::Function, "f(x: number)"));
        assert!(!s.is_empty());
        assert!(s.iter().any(|s| s.suggestion_type == "add_description"));
    }

    #[test]
    fn test_suggest_parameter_documentation() {
        let v = QualityValidator::new();
        let s = v.suggest_improvements(
            "/** Basic doc */",
            &cs("f", SymbolKind::Function, "f(x: i32)"),
        );
        assert!(s.iter().any(|s| s.suggestion_type == "add_parameters"));
    }

    #[test]
    fn test_severity_levels() {
        let v = QualityValidator::new();
        let s = v.assess("", &cs("f", SymbolKind::Function, "f()"));
        assert!(!s.issues.is_empty());
        assert!(s.issues.iter().any(|i| i.severity == Severity::Error));
    }

    #[test]
    fn test_quality_score_thresholds() {
        let p = QualityScore::perfect();
        assert!(p.is_good());
        assert!(p.is_acceptable());
        assert_eq!(p.as_percentage(), 100);
    }

    #[test]
    fn test_parameter_count_validation() {
        let v = QualityValidator::new();
        let doc = "/**\n * Function\n * @param {number} x\n */";
        let s = v.assess(doc, &cs("f", SymbolKind::Function, "f(x: number, y: number)"));
        assert!(s.issues.iter().any(|i| i.message.contains("mismatch")));
    }

    #[test]
    fn test_vague_description_detection() {
        let v = QualityValidator::new();
        let s = v.assess(
            "/** Does stuff */",
            &cs("f", SymbolKind::Function, "f()"),
        );
        assert!(s.issues.iter().any(|i| i.category == "clarity"));
    }

    #[test]
    fn test_compliance_checking() {
        let v = QualityValidator::new();
        let s = v.assess(
            "Just a comment without markers",
            &cs("f", SymbolKind::Function, "f()"),
        );
        assert!(s.compliance < 1.0);
    }

    #[test]
    fn test_strict_mode() {
        let v = QualityValidator::new().with_strict_mode(true);
        let s = v.assess(
            "/** Basic */",
            &cs("f", SymbolKind::Function, "f(): string"),
        );
        assert!(s
            .issues
            .iter()
            .any(|i| i.severity == Severity::Error && i.message.contains("return")));
    }

    #[test]
    fn test_example_suggestion() {
        let v = QualityValidator::new();
        let s = v.suggest_improvements(
            "/** Good description */",
            &cs("add", SymbolKind::Function, "add(a: number, b: number)"),
        );
        assert!(s.iter().any(|s| s.suggestion_type == "add_example"));
    }

    #[test]
    fn test_clarity_score() {
        let v = QualityValidator::new();
        let short_doc = "/** A */";
        let s = v.assess(short_doc, &cs("f", SymbolKind::Function, "f()"));
        assert!(s.clarity < 1.0);
    }

    #[test]
    fn test_accuracy_score() {
        let v = QualityValidator::new();
        let doc = "/** Function with TODO */";
        let s = v.assess(doc, &cs("f", SymbolKind::Function, "f()"));
        assert!(s.accuracy < 1.0);
    }

    #[test]
    fn test_high_quality_documentation() {
        let v = QualityValidator::new();
        let doc = "/**\n * Adds two numbers together and returns the sum.\n * This is a pure function with no side effects.\n * @param {number} a - The first number\n * @param {number} b - The second number\n * @returns {number} The sum of a and b\n * @example\n * add(2, 3) // returns 5\n */";
        let s = v.assess(doc, &cs("add", SymbolKind::Function, "add(a: number, b: number): number"));
        assert!(s.is_good());
        assert!(s.overall >= 0.85);
    }

    #[test]
    fn test_format_detection() {
        let v = QualityValidator::new();
        assert_eq!(v.detect_format("/** JSDoc */"), "tsdoc");
        assert_eq!(v.detect_format("/// Rust doc"), "rustdoc");
    }

    #[test]
    fn test_percentage_conversion() {
        let mut score = QualityScore::perfect();
        assert_eq!(score.as_percentage(), 100);

        score.overall = 0.75;
        assert_eq!(score.as_percentage(), 75);

        score.overall = 0.0;
        assert_eq!(score.as_percentage(), 0);
    }
}
