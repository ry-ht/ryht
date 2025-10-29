//! Core traits for the parser abstraction layer.
//!
//! This module provides the fundamental traits that define the parser interface
//! and enable language-agnostic code analysis operations.

use std::path::Path;
use tree_sitter::Node as TSNode;

use crate::node::Node;
use crate::lang::Lang;

/// A trait for callback functions.
///
/// Allows to call a private library function, getting as result
/// its output value. This enables extensible operations on parsers
/// without tight coupling.
pub trait Callback {
    /// The output type returned by the callee
    type Res;
    /// The input type used by the caller to pass the arguments to the callee
    type Cfg;

    /// Calls a function inside the library and returns its value
    fn call<T: ParserTrait>(cfg: Self::Cfg, parser: &T) -> Self::Res;
}

/// Language information provider trait.
///
/// Provides static language metadata for parser implementations.
pub trait LanguageInfo {
    /// Get the language enumeration value
    fn get_lang() -> Lang;

    /// Get the language name as a string
    fn get_lang_name() -> &'static str;
}

/// Core parser trait defining the interface for all language parsers.
///
/// This trait abstracts over tree-sitter based parsers for different languages,
/// providing a uniform interface for parsing and analyzing code.
#[doc(hidden)]
pub trait ParserTrait: Sized {
    /// Create a new parser instance for the given code
    fn new(code: Vec<u8>, path: &Path) -> anyhow::Result<Self>;

    /// Get the language this parser handles
    fn get_language(&self) -> Lang;

    /// Get the root node of the parsed tree
    fn get_root(&self) -> Node;

    /// Get the source code as bytes
    fn get_code(&self) -> &[u8];

    /// Get a text slice from the source code
    fn get_text(&self, node: &TSNode) -> Option<&str> {
        let start = node.start_byte();
        let end = node.end_byte();
        let bytes = &self.get_code()[start..end];
        std::str::from_utf8(bytes).ok()
    }
}

/// Search operations on AST nodes.
pub(crate) trait Search<'a> {
    /// Find first node matching the predicate in depth-first order
    #[allow(dead_code)]
    fn first_occurrence(&self, pred: fn(u16) -> bool) -> Option<Node<'a>>;

    /// Execute an action on every node in the tree
    fn act_on_node(&self, pred: &mut dyn FnMut(&Node<'a>));

    /// Find first child matching the predicate
    #[allow(dead_code)]
    fn first_child(&self, pred: fn(u16) -> bool) -> Option<Node<'a>>;

    /// Execute an action on every direct child
    #[allow(dead_code)]
    fn act_on_child(&self, action: &mut dyn FnMut(&Node<'a>));
}
