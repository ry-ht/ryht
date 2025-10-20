use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

/// Unique identifier for semantic links
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LinkId(String);

impl LinkId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for LinkId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for LinkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type of semantic link
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkType {
    /// Spec → Code: Specification is implemented by code
    ImplementedBy,
    /// Code → Spec: Code realizes specification
    Realizes,
    /// Code → Docs: Code is documented in documentation
    DocumentedIn,
    /// Docs → Code: Documentation documents code
    Documents,
    /// Code → Examples: Code is demonstrated in examples
    DemonstratedIn,
    /// Examples → Code: Example demonstrates code
    Demonstrates,
    /// Code → Tests: Code is tested by test
    TestedBy,
    /// Tests → Code: Test verifies code
    Tests,
    /// Spec → Docs: Specification has user guide
    UserGuideFor,
    /// Docs → Spec: Documentation specifies feature
    Specifies,
    /// Docs → Examples: Documentation shows example
    ShowsExample,
    /// Examples → Docs: Example illustrated in documentation
    IllustratedIn,
    /// Generic dependency relationship
    DependsOn,
    /// Derived from another entity
    DerivedFrom,
    /// Related to another entity
    RelatesTo,
    /// Supersedes another entity
    Supersedes,
    /// Referenced by another entity
    ReferencedBy,
}

impl LinkType {
    pub fn as_str(&self) -> &'static str {
        match self {
            LinkType::ImplementedBy => "implemented_by",
            LinkType::Realizes => "realizes",
            LinkType::DocumentedIn => "documented_in",
            LinkType::Documents => "documents",
            LinkType::DemonstratedIn => "demonstrated_in",
            LinkType::Demonstrates => "demonstrates",
            LinkType::TestedBy => "tested_by",
            LinkType::Tests => "tests",
            LinkType::UserGuideFor => "user_guide_for",
            LinkType::Specifies => "specifies",
            LinkType::ShowsExample => "shows_example",
            LinkType::IllustratedIn => "illustrated_in",
            LinkType::DependsOn => "depends_on",
            LinkType::DerivedFrom => "derived_from",
            LinkType::RelatesTo => "relates_to",
            LinkType::Supersedes => "supersedes",
            LinkType::ReferencedBy => "referenced_by",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "implemented_by" => Some(LinkType::ImplementedBy),
            "realizes" => Some(LinkType::Realizes),
            "documented_in" => Some(LinkType::DocumentedIn),
            "documents" => Some(LinkType::Documents),
            "demonstrated_in" => Some(LinkType::DemonstratedIn),
            "demonstrates" => Some(LinkType::Demonstrates),
            "tested_by" => Some(LinkType::TestedBy),
            "tests" => Some(LinkType::Tests),
            "user_guide_for" => Some(LinkType::UserGuideFor),
            "specifies" => Some(LinkType::Specifies),
            "shows_example" => Some(LinkType::ShowsExample),
            "illustrated_in" => Some(LinkType::IllustratedIn),
            "depends_on" => Some(LinkType::DependsOn),
            "derived_from" => Some(LinkType::DerivedFrom),
            "relates_to" => Some(LinkType::RelatesTo),
            "supersedes" => Some(LinkType::Supersedes),
            "referenced_by" => Some(LinkType::ReferencedBy),
            _ => None,
        }
    }

    /// Get the inverse link type if bidirectional
    pub fn inverse(&self) -> Option<LinkType> {
        match self {
            LinkType::ImplementedBy => Some(LinkType::Realizes),
            LinkType::Realizes => Some(LinkType::ImplementedBy),
            LinkType::DocumentedIn => Some(LinkType::Documents),
            LinkType::Documents => Some(LinkType::DocumentedIn),
            LinkType::DemonstratedIn => Some(LinkType::Demonstrates),
            LinkType::Demonstrates => Some(LinkType::DemonstratedIn),
            LinkType::TestedBy => Some(LinkType::Tests),
            LinkType::Tests => Some(LinkType::TestedBy),
            LinkType::UserGuideFor => Some(LinkType::Specifies),
            LinkType::Specifies => Some(LinkType::UserGuideFor),
            LinkType::ShowsExample => Some(LinkType::IllustratedIn),
            LinkType::IllustratedIn => Some(LinkType::ShowsExample),
            _ => None,
        }
    }

    /// Check if this link type is bidirectional
    pub fn is_bidirectional(&self) -> bool {
        self.inverse().is_some()
    }
}

impl fmt::Display for LinkType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Knowledge level in the documentation hierarchy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KnowledgeLevel {
    /// Specification (what to build)
    Spec,
    /// Implementation code (how it's built)
    Code,
    /// Documentation (how to use it)
    Docs,
    /// Usage examples
    Examples,
    /// Tests (verification)
    Tests,
}

impl KnowledgeLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            KnowledgeLevel::Spec => "spec",
            KnowledgeLevel::Code => "code",
            KnowledgeLevel::Docs => "docs",
            KnowledgeLevel::Examples => "examples",
            KnowledgeLevel::Tests => "tests",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "spec" => Some(KnowledgeLevel::Spec),
            "code" => Some(KnowledgeLevel::Code),
            "docs" => Some(KnowledgeLevel::Docs),
            "examples" => Some(KnowledgeLevel::Examples),
            "tests" => Some(KnowledgeLevel::Tests),
            _ => None,
        }
    }
}

impl fmt::Display for KnowledgeLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Location within a source file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub line_start: Option<usize>,
    pub line_end: Option<usize>,
    pub column_start: Option<usize>,
    pub column_end: Option<usize>,
    /// Optional fragment identifier (e.g., "#section-name" for markdown)
    pub fragment: Option<String>,
}

impl SourceLocation {
    pub fn new(file: String) -> Self {
        Self {
            file,
            line_start: None,
            line_end: None,
            column_start: None,
            column_end: None,
            fragment: None,
        }
    }

    pub fn with_lines(mut self, start: usize, end: usize) -> Self {
        self.line_start = Some(start);
        self.line_end = Some(end);
        self
    }

    pub fn with_fragment(mut self, fragment: String) -> Self {
        self.fragment = Some(fragment);
        self
    }
}

/// Target of a semantic link
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkTarget {
    /// Knowledge level
    pub level: KnowledgeLevel,
    /// Entity identifier (e.g., "Application", "spec.md#memory-model")
    pub id: String,
    /// Optional precise location
    pub location: Option<SourceLocation>,
}

impl LinkTarget {
    pub fn new(level: KnowledgeLevel, id: String) -> Self {
        Self {
            level,
            id,
            location: None,
        }
    }

    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }

    /// Create a spec target
    pub fn spec(id: String) -> Self {
        Self::new(KnowledgeLevel::Spec, id)
    }

    /// Create a code target
    pub fn code(id: String) -> Self {
        Self::new(KnowledgeLevel::Code, id)
    }

    /// Create a docs target
    pub fn docs(id: String) -> Self {
        Self::new(KnowledgeLevel::Docs, id)
    }

    /// Create an examples target
    pub fn examples(id: String) -> Self {
        Self::new(KnowledgeLevel::Examples, id)
    }

    /// Create a tests target
    pub fn tests(id: String) -> Self {
        Self::new(KnowledgeLevel::Tests, id)
    }

    /// Get a string representation for indexing
    pub fn key(&self) -> String {
        format!("{}:{}", self.level.as_str(), self.id)
    }
}

impl fmt::Display for LinkTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.key())
    }
}

/// Method used to extract the link
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtractionMethod {
    /// Explicitly annotated in code/docs/specs
    Annotation,
    /// Inferred from code structure, imports, naming
    Inference,
    /// Manually created via MCP tools
    Manual,
}

impl ExtractionMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExtractionMethod::Annotation => "annotation",
            ExtractionMethod::Inference => "inference",
            ExtractionMethod::Manual => "manual",
        }
    }
}

impl fmt::Display for ExtractionMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Validation status of a link
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationStatus {
    /// Link is valid
    Valid,
    /// Link is broken (target doesn't exist)
    Broken,
    /// Link may be stale (target modified since creation)
    Stale,
    /// Link hasn't been validated yet
    Unchecked,
}

impl ValidationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ValidationStatus::Valid => "valid",
            ValidationStatus::Broken => "broken",
            ValidationStatus::Stale => "stale",
            ValidationStatus::Unchecked => "unchecked",
        }
    }
}

impl fmt::Display for ValidationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A semantic link between two entities in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticLink {
    /// Unique identifier
    pub id: LinkId,
    /// Type of link
    pub link_type: LinkType,
    /// Source entity
    pub source: LinkTarget,
    /// Target entity
    pub target: LinkTarget,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// How the link was extracted
    pub extraction_method: ExtractionMethod,
    /// Optional context explaining the link
    pub context: Option<String>,
    /// When the link was created
    pub created_at: DateTime<Utc>,
    /// Who/what created the link
    pub created_by: String,
    /// When the link was last validated
    pub last_validated: Option<DateTime<Utc>>,
    /// Validation status
    pub validation_status: ValidationStatus,
}

impl SemanticLink {
    /// Create a new semantic link
    pub fn new(
        link_type: LinkType,
        source: LinkTarget,
        target: LinkTarget,
        confidence: f32,
        extraction_method: ExtractionMethod,
        created_by: String,
    ) -> Self {
        Self {
            id: LinkId::new(),
            link_type,
            source,
            target,
            metadata: HashMap::new(),
            confidence: confidence.clamp(0.0, 1.0),
            extraction_method,
            context: None,
            created_at: Utc::now(),
            created_by,
            last_validated: None,
            validation_status: ValidationStatus::Unchecked,
        }
    }

    /// Add metadata to the link
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Add context to the link
    pub fn with_context(mut self, context: String) -> Self {
        self.context = Some(context);
        self
    }

    /// Validate the link
    pub fn validate(&mut self, status: ValidationStatus) {
        self.validation_status = status;
        self.last_validated = Some(Utc::now());
    }

    /// Check if the link is valid
    pub fn is_valid(&self) -> bool {
        matches!(self.validation_status, ValidationStatus::Valid)
    }

    /// Check if the link is broken
    pub fn is_broken(&self) -> bool {
        matches!(self.validation_status, ValidationStatus::Broken)
    }

    /// Check if the link is stale
    pub fn is_stale(&self) -> bool {
        matches!(self.validation_status, ValidationStatus::Stale)
    }

    /// Get the inverse link if bidirectional
    pub fn inverse(&self) -> Option<SemanticLink> {
        self.link_type.inverse().map(|inv_type| {
            let mut inv = SemanticLink::new(
                inv_type,
                self.target.clone(),
                self.source.clone(),
                self.confidence,
                self.extraction_method,
                self.created_by.clone(),
            );
            inv.metadata = self.metadata.clone();
            inv.context = self.context.clone();
            inv.created_at = self.created_at;
            inv.last_validated = self.last_validated;
            inv.validation_status = self.validation_status;
            inv
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_type_as_str() {
        assert_eq!(LinkType::ImplementedBy.as_str(), "implemented_by");
        assert_eq!(LinkType::Realizes.as_str(), "realizes");
        assert_eq!(LinkType::DocumentedIn.as_str(), "documented_in");
    }

    #[test]
    fn test_link_type_from_str() {
        assert_eq!(
            LinkType::from_str("implemented_by"),
            Some(LinkType::ImplementedBy)
        );
        assert_eq!(LinkType::from_str("realizes"), Some(LinkType::Realizes));
        assert_eq!(LinkType::from_str("invalid"), None);
    }

    #[test]
    fn test_link_type_inverse() {
        assert_eq!(
            LinkType::ImplementedBy.inverse(),
            Some(LinkType::Realizes)
        );
        assert_eq!(
            LinkType::DocumentedIn.inverse(),
            Some(LinkType::Documents)
        );
        assert_eq!(LinkType::DependsOn.inverse(), None);
    }

    #[test]
    fn test_link_type_bidirectional() {
        assert!(LinkType::ImplementedBy.is_bidirectional());
        assert!(LinkType::DocumentedIn.is_bidirectional());
        assert!(!LinkType::DependsOn.is_bidirectional());
    }

    #[test]
    fn test_knowledge_level() {
        assert_eq!(KnowledgeLevel::Spec.as_str(), "spec");
        assert_eq!(KnowledgeLevel::Code.as_str(), "code");
        assert_eq!(
            KnowledgeLevel::from_str("spec"),
            Some(KnowledgeLevel::Spec)
        );
    }

    #[test]
    fn test_link_target_key() {
        let target = LinkTarget::code("Application".to_string());
        assert_eq!(target.key(), "code:Application");

        let target = LinkTarget::spec("spec.md#memory-model".to_string());
        assert_eq!(target.key(), "spec:spec.md#memory-model");
    }

    #[test]
    fn test_semantic_link_creation() {
        let source = LinkTarget::spec("spec.md#feature".to_string());
        let target = LinkTarget::code("Implementation".to_string());

        let link = SemanticLink::new(
            LinkType::ImplementedBy,
            source.clone(),
            target.clone(),
            0.95,
            ExtractionMethod::Annotation,
            "test".to_string(),
        );

        assert_eq!(link.link_type, LinkType::ImplementedBy);
        assert_eq!(link.source, source);
        assert_eq!(link.target, target);
        assert_eq!(link.confidence, 0.95);
        assert_eq!(link.extraction_method, ExtractionMethod::Annotation);
        assert_eq!(link.validation_status, ValidationStatus::Unchecked);
    }

    #[test]
    fn test_semantic_link_validation() {
        let source = LinkTarget::code("A".to_string());
        let target = LinkTarget::docs("a.md".to_string());

        let mut link = SemanticLink::new(
            LinkType::DocumentedIn,
            source,
            target,
            1.0,
            ExtractionMethod::Manual,
            "test".to_string(),
        );

        assert!(!link.is_valid());
        assert!(!link.is_broken());

        link.validate(ValidationStatus::Valid);
        assert!(link.is_valid());
        assert!(link.last_validated.is_some());
    }

    #[test]
    fn test_semantic_link_inverse() {
        let source = LinkTarget::code("Application".to_string());
        let target = LinkTarget::spec("spec.md#app".to_string());

        let link = SemanticLink::new(
            LinkType::Realizes,
            source.clone(),
            target.clone(),
            0.9,
            ExtractionMethod::Annotation,
            "test".to_string(),
        );

        let inverse = link.inverse().unwrap();
        assert_eq!(inverse.link_type, LinkType::ImplementedBy);
        assert_eq!(inverse.source, target);
        assert_eq!(inverse.target, source);
        assert_eq!(inverse.confidence, link.confidence);
    }

    #[test]
    fn test_confidence_clamping() {
        let source = LinkTarget::code("A".to_string());
        let target = LinkTarget::docs("a.md".to_string());

        let link1 = SemanticLink::new(
            LinkType::DocumentedIn,
            source.clone(),
            target.clone(),
            1.5,
            ExtractionMethod::Manual,
            "test".to_string(),
        );
        assert_eq!(link1.confidence, 1.0);

        let link2 = SemanticLink::new(
            LinkType::DocumentedIn,
            source,
            target,
            -0.5,
            ExtractionMethod::Manual,
            "test".to_string(),
        );
        assert_eq!(link2.confidence, 0.0);
    }
}
