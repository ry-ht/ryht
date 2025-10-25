//! Language-specific parser implementations.
//!
//! This module contains concrete implementations of the ParserTrait
//! for each supported programming language, along with comprehensive
//! token enums and helper methods for each language.
//!
//! # Language Support
//!
//! Each language module provides:
//! - Complete token/node type enumerations
//! - Comprehensive helper methods (is_function, is_class, etc.)
//! - Operator detection and classification
//! - Language-specific feature detection
//!
//! ## Available Languages
//!
//! - **Rust**: 175+ helper methods for traits, lifetimes, macros, patterns
//! - **TypeScript**: 95+ helper methods for types, interfaces, decorators
//! - **JavaScript**: 114+ helper methods for ES6+, JSX, async/await
//! - **Python**: 136+ helper methods for decorators, comprehensions, type hints
//! - **C++**: Complete C++ language support
//! - **Java**: Full Java language analysis
//! - **Kotlin**: Kotlin language support
//! - **TSX**: TypeScript with JSX support

pub mod rust;
pub mod typescript;
pub mod javascript;
pub mod python;
pub mod cpp;
pub mod java;
pub mod kotlin;
pub mod tsx;

// Re-export language analyzers (structs implementing LanguageInfo)
pub use rust::RustLanguage;
pub use typescript::TypeScriptLanguage;
pub use javascript::{JavaScriptLanguage, JsxLanguage};
pub use python::PythonLanguage;
pub use cpp::CppLanguage;
pub use java::JavaLanguage;
pub use kotlin::KotlinLanguage;
pub use tsx::TsxLanguage;

// Re-export token enums for direct use in pattern matching and analysis
pub use rust::Rust as RustToken;
pub use typescript::TypeScriptToken;
pub use javascript::JavaScriptToken;
pub use python::Python as PythonToken;
