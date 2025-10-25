//! Analysis types and enumerations.
//!
//! This module defines core types used in code analysis operations,
//! including Halstead complexity types and code space classifications.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Halstead metric type classification.
///
/// Used to classify AST nodes as operators, operands, or unknown
/// for Halstead complexity analysis.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum HalsteadType {
    /// Node is classified as an operator
    Operator,
    /// Node is classified as an operand
    Operand,
    /// Node type is unknown or not applicable
    #[default]
    Unknown,
}

impl fmt::Display for HalsteadType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HalsteadType::Operator => write!(f, "operator"),
            HalsteadType::Operand => write!(f, "operand"),
            HalsteadType::Unknown => write!(f, "unknown"),
        }
    }
}

/// Code space kind classification.
///
/// Represents different types of code spaces/scopes in a program,
/// such as functions, classes, traits, etc.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpaceKind {
    /// An unknown space
    #[default]
    Unknown,
    /// A function space
    Function,
    /// A class space
    Class,
    /// A struct space
    Struct,
    /// A Rust trait space
    Trait,
    /// A Rust implementation space
    Impl,
    /// A general/top-level unit space
    Unit,
    /// A C/C++ namespace
    Namespace,
    /// An interface (TypeScript, Java, etc.)
    Interface,
}

impl fmt::Display for SpaceKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            SpaceKind::Unknown => "unknown",
            SpaceKind::Function => "function",
            SpaceKind::Class => "class",
            SpaceKind::Struct => "struct",
            SpaceKind::Trait => "trait",
            SpaceKind::Impl => "impl",
            SpaceKind::Unit => "unit",
            SpaceKind::Namespace => "namespace",
            SpaceKind::Interface => "interface",
        };
        write!(f, "{}", s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_halstead_type_display() {
        assert_eq!(HalsteadType::Operator.to_string(), "operator");
        assert_eq!(HalsteadType::Operand.to_string(), "operand");
        assert_eq!(HalsteadType::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_space_kind_display() {
        assert_eq!(SpaceKind::Function.to_string(), "function");
        assert_eq!(SpaceKind::Class.to_string(), "class");
        assert_eq!(SpaceKind::Trait.to_string(), "trait");
        assert_eq!(SpaceKind::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_default_values() {
        assert_eq!(HalsteadType::default(), HalsteadType::Unknown);
        assert_eq!(SpaceKind::default(), SpaceKind::Unknown);
    }
}
