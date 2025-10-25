//! TypeScript language parser implementation.

use num_derive::FromPrimitive;
use crate::lang::Lang;
use crate::traits::LanguageInfo;

/// TypeScript language token types.
#[derive(Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum TypeScriptToken {
    End = 0,
    Identifier = 1,
    Program = 166,
    FunctionDeclaration = 224,
    ClassDeclaration = 221,
    InterfaceDeclaration = 288,
    Error = 383,
}

impl From<TypeScriptToken> for &'static str {
    fn from(tok: TypeScriptToken) -> Self {
        match tok {
            TypeScriptToken::End => "end",
            TypeScriptToken::Identifier => "identifier",
            TypeScriptToken::Program => "program",
            TypeScriptToken::FunctionDeclaration => "function_declaration",
            TypeScriptToken::ClassDeclaration => "class_declaration",
            TypeScriptToken::InterfaceDeclaration => "interface_declaration",
            TypeScriptToken::Error => "ERROR",
        }
    }
}

impl From<u16> for TypeScriptToken {
    fn from(x: u16) -> Self {
        num::FromPrimitive::from_u16(x).unwrap_or(Self::Error)
    }
}

/// TypeScript language implementation.
pub struct TypeScriptLanguage;

impl LanguageInfo for TypeScriptLanguage {
    fn get_lang() -> Lang {
        Lang::TypeScript
    }

    fn get_lang_name() -> &'static str {
        "typescript"
    }
}

