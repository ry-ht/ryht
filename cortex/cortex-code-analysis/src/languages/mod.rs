//! Language-specific parser implementations.
//!
//! This module contains concrete implementations of the ParserTrait
//! for each supported programming language.

pub mod rust;
pub mod typescript;
pub mod javascript;
pub mod python;
pub mod cpp;
pub mod java;
pub mod kotlin;
pub mod tsx;

// Re-export language parsers
pub use rust::RustLanguage;
pub use typescript::TypeScriptLanguage;
pub use javascript::JavaScriptLanguage;
pub use python::PythonLanguage;
pub use cpp::CppLanguage;
pub use java::JavaLanguage;
pub use kotlin::KotlinLanguage;
pub use tsx::TsxLanguage;
