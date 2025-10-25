//! Java language parser implementation.

use num_derive::FromPrimitive;
use crate::lang::Lang;
use crate::traits::LanguageInfo;

/// Java language token types.
///
/// This enum represents all possible node types in the Java tree-sitter grammar.
#[derive(Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum JavaToken {
    End = 0,
    Identifier = 1,
    Program = 138,
    ClassDeclaration = 233,
    InterfaceDeclaration = 255,
    MethodDeclaration = 279,
    FieldDeclaration = 249,
    EnumDeclaration = 229,
    AnnotationTypeDeclaration = 251,
    PackageDeclaration = 226,
    ImportDeclaration = 227,
    Error = 321,
}

impl From<JavaToken> for &'static str {
    fn from(tok: JavaToken) -> Self {
        match tok {
            JavaToken::End => "end",
            JavaToken::Identifier => "identifier",
            JavaToken::Program => "program",
            JavaToken::ClassDeclaration => "class_declaration",
            JavaToken::InterfaceDeclaration => "interface_declaration",
            JavaToken::MethodDeclaration => "method_declaration",
            JavaToken::FieldDeclaration => "field_declaration",
            JavaToken::EnumDeclaration => "enum_declaration",
            JavaToken::AnnotationTypeDeclaration => "annotation_type_declaration",
            JavaToken::PackageDeclaration => "package_declaration",
            JavaToken::ImportDeclaration => "import_declaration",
            JavaToken::Error => "ERROR",
        }
    }
}

impl From<u16> for JavaToken {
    fn from(x: u16) -> Self {
        num::FromPrimitive::from_u16(x).unwrap_or(Self::Error)
    }
}

impl PartialEq<u16> for JavaToken {
    fn eq(&self, x: &u16) -> bool {
        *self == Into::<Self>::into(*x)
    }
}

impl PartialEq<JavaToken> for u16 {
    fn eq(&self, x: &JavaToken) -> bool {
        *x == *self
    }
}

/// Java language implementation.
pub struct JavaLanguage;

impl LanguageInfo for JavaLanguage {
    fn get_lang() -> Lang {
        Lang::Java
    }

    fn get_lang_name() -> &'static str {
        "java"
    }
}
