//! Kotlin language parser implementation.

use num_derive::FromPrimitive;
use crate::lang::Lang;
use crate::traits::LanguageInfo;

/// Kotlin language token types.
///
/// This enum represents all possible node types in the Kotlin tree-sitter grammar.
#[derive(Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum KotlinToken {
    End = 0,
    Identifier = 1,
    SourceFile = 142,
    ClassDeclaration = 147,
    ObjectDeclaration = 148,
    FunctionDeclaration = 163,
    PropertyDeclaration = 149,
    InterfaceDeclaration = 150,
    PackageHeader = 144,
    Import = 145,
    Error = 289,
}

impl From<KotlinToken> for &'static str {
    fn from(tok: KotlinToken) -> Self {
        match tok {
            KotlinToken::End => "end",
            KotlinToken::Identifier => "identifier",
            KotlinToken::SourceFile => "source_file",
            KotlinToken::ClassDeclaration => "class_declaration",
            KotlinToken::ObjectDeclaration => "object_declaration",
            KotlinToken::FunctionDeclaration => "function_declaration",
            KotlinToken::PropertyDeclaration => "property_declaration",
            KotlinToken::InterfaceDeclaration => "class_declaration",
            KotlinToken::PackageHeader => "package_header",
            KotlinToken::Import => "import",
            KotlinToken::Error => "ERROR",
        }
    }
}

impl From<u16> for KotlinToken {
    fn from(x: u16) -> Self {
        num::FromPrimitive::from_u16(x).unwrap_or(Self::Error)
    }
}

impl PartialEq<u16> for KotlinToken {
    fn eq(&self, x: &u16) -> bool {
        *self == Into::<Self>::into(*x)
    }
}

impl PartialEq<KotlinToken> for u16 {
    fn eq(&self, x: &KotlinToken) -> bool {
        *x == *self
    }
}

/// Kotlin language implementation.
pub struct KotlinLanguage;

impl LanguageInfo for KotlinLanguage {
    fn get_lang() -> Lang {
        Lang::Kotlin
    }

    fn get_lang_name() -> &'static str {
        "kotlin"
    }
}
