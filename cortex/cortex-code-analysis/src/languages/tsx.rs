//! TSX (TypeScript with JSX) language parser implementation.

use num_derive::FromPrimitive;
use crate::lang::Lang;
use crate::traits::LanguageInfo;

/// TSX language token types.
///
/// This enum represents all possible node types in the TSX tree-sitter grammar.
#[derive(Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum TsxToken {
    End = 0,
    Identifier = 1,
    Program = 172,
    ClassDeclaration = 235,
    FunctionDeclaration = 238,
    InterfaceDeclaration = 301,
    ArrowFunction = 241,
    ExportStatement = 173,
    ImportStatement = 180,
    Error = 400,
}

impl From<TsxToken> for &'static str {
    fn from(tok: TsxToken) -> Self {
        match tok {
            TsxToken::End => "end",
            TsxToken::Identifier => "identifier",
            TsxToken::Program => "program",
            TsxToken::ClassDeclaration => "class_declaration",
            TsxToken::FunctionDeclaration => "function_declaration",
            TsxToken::InterfaceDeclaration => "interface_declaration",
            TsxToken::ArrowFunction => "arrow_function",
            TsxToken::ExportStatement => "export_statement",
            TsxToken::ImportStatement => "import_statement",
            TsxToken::Error => "ERROR",
        }
    }
}

impl From<u16> for TsxToken {
    fn from(x: u16) -> Self {
        num::FromPrimitive::from_u16(x).unwrap_or(Self::Error)
    }
}

impl PartialEq<u16> for TsxToken {
    fn eq(&self, x: &u16) -> bool {
        *self == Into::<Self>::into(*x)
    }
}

impl PartialEq<TsxToken> for u16 {
    fn eq(&self, x: &TsxToken) -> bool {
        *x == *self
    }
}

/// TSX language implementation.
pub struct TsxLanguage;

impl LanguageInfo for TsxLanguage {
    fn get_lang() -> Lang {
        Lang::Tsx
    }

    fn get_lang_name() -> &'static str {
        "tsx"
    }
}
