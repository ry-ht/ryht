//! Code Generation Tools - Documentation, Examples, and Tests
//!
//! This module provides code generation capabilities including documentation,
//! code examples, and test generation for TypeScript, JavaScript, Rust, and Python.

pub mod doc_generator;
pub mod doc_quality;
pub mod templates;
pub mod catalog;
pub mod cross_ref;
pub mod example_generator;
pub mod test_generator;
pub mod example_validator;
pub mod cross_monorepo;
pub mod dependency_parser;

pub use doc_generator::{DocumentationGenerator, GeneratedDoc, DocFormat, DocTransformOptions};
pub use doc_quality::{QualityValidator, QualityScore, QualityIssue, Suggestion};
pub use templates::{TemplateEngine, DocTemplate};
pub use catalog::{GlobalCatalog, ProjectMetadata, SearchScope, DocResult};
pub use cross_ref::{CrossReferenceManager, CrossReference, ReferenceType, DependencyGraph, DependencyNode, DependencyEdge};
pub use example_generator::{ExampleGenerator, Example, ExampleComplexity, ValidationResult};
pub use test_generator::{TestGenerator, GeneratedTest, TestFramework, TestType};
pub use example_validator::{ExampleValidator, QualityScore as ExampleQualityScore};
pub use cross_monorepo::{CrossMonorepoAccess, ExternalDocs, Usage, UsageType, AccessControl, SearchResult, MatchType};
pub use dependency_parser::{DependencyParser, Dependency, DependencyType, ManifestDependencies};
