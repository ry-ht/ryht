//! Language-specific parser implementations.
//!
//! This module contains concrete implementations of the ParserTrait
//! for each supported programming language.

pub mod rust;
pub mod typescript;
pub mod javascript;
pub mod python;

// Re-export language parsers
pub use rust::RustLanguage;
pub use typescript::TypeScriptLanguage;
pub use javascript::JavaScriptLanguage;
pub use python::PythonLanguage;
