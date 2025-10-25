//! C++ language parser implementation.

use num_derive::FromPrimitive;
use crate::lang::Lang;
use crate::traits::LanguageInfo;

/// C++ language token types.
///
/// This enum represents all possible node types in the C++ tree-sitter grammar.
#[derive(Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum CppToken {
    End = 0,
    Identifier = 1,
    TranslationUnit = 308,
    FunctionDefinition = 343,
    ClassDeclaration = 466,
    ClassSpecifier = 468,
    StructSpecifier = 392,
    NamespaceDefinition = 519,
    TemplateDeclaration = 475,
    Declaration = 344,
    Error = 638,
}

impl From<CppToken> for &'static str {
    fn from(tok: CppToken) -> Self {
        match tok {
            CppToken::End => "end",
            CppToken::Identifier => "identifier",
            CppToken::TranslationUnit => "translation_unit",
            CppToken::FunctionDefinition => "function_definition",
            CppToken::ClassDeclaration => "_class_declaration",
            CppToken::ClassSpecifier => "class_specifier",
            CppToken::StructSpecifier => "struct_specifier",
            CppToken::NamespaceDefinition => "namespace_definition",
            CppToken::TemplateDeclaration => "template_declaration",
            CppToken::Declaration => "declaration",
            CppToken::Error => "ERROR",
        }
    }
}

impl From<u16> for CppToken {
    fn from(x: u16) -> Self {
        num::FromPrimitive::from_u16(x).unwrap_or(Self::Error)
    }
}

impl PartialEq<u16> for CppToken {
    fn eq(&self, x: &u16) -> bool {
        *self == Into::<Self>::into(*x)
    }
}

impl PartialEq<CppToken> for u16 {
    fn eq(&self, x: &CppToken) -> bool {
        *x == *self
    }
}

/// C++ language implementation.
pub struct CppLanguage;

impl LanguageInfo for CppLanguage {
    fn get_lang() -> Lang {
        Lang::Cpp
    }

    fn get_lang_name() -> &'static str {
        "c++"
    }
}
