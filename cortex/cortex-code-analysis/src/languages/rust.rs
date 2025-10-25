//! Rust language parser implementation.

use num_derive::FromPrimitive;
use crate::lang::Lang;
use crate::traits::LanguageInfo;

/// Rust language token types.
///
/// This enum represents all possible node types in the Rust tree-sitter grammar.
#[derive(Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum RustToken {
    End = 0,
    Identifier = 1,
    SEMI = 2,
    MacroRulesBANG = 3,
    LPAREN = 4,
    RPAREN = 5,
    LBRACK = 6,
    RBRACK = 7,
    LBRACE = 8,
    RBRACE = 9,
    // ... (truncated for brevity, use the full enum from experiments)
    SourceFile = 155,
    FunctionItem = 186,
    StructItem = 174,
    EnumItem = 176,
    TraitItem = 192,
    ImplItem = 191,
    UseDeclaration = 202,
    Error = 350,
}

impl From<RustToken> for &'static str {
    fn from(tok: RustToken) -> Self {
        match tok {
            RustToken::End => "end",
            RustToken::Identifier => "identifier",
            RustToken::SEMI => ";",
            RustToken::SourceFile => "source_file",
            RustToken::FunctionItem => "function_item",
            RustToken::StructItem => "struct_item",
            RustToken::EnumItem => "enum_item",
            RustToken::TraitItem => "trait_item",
            RustToken::ImplItem => "impl_item",
            RustToken::UseDeclaration => "use_declaration",
            RustToken::Error => "ERROR",
            _ => "unknown",
        }
    }
}

impl From<u16> for RustToken {
    fn from(x: u16) -> Self {
        num::FromPrimitive::from_u16(x).unwrap_or(Self::Error)
    }
}

impl PartialEq<u16> for RustToken {
    fn eq(&self, x: &u16) -> bool {
        *self == Into::<Self>::into(*x)
    }
}

impl PartialEq<RustToken> for u16 {
    fn eq(&self, x: &RustToken) -> bool {
        *x == *self
    }
}

/// Rust language implementation.
pub struct RustLanguage;

impl LanguageInfo for RustLanguage {
    fn get_lang() -> Lang {
        Lang::Rust
    }

    fn get_lang_name() -> &'static str {
        "rust"
    }
}
