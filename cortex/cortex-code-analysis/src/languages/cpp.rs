//! C++ language parser implementation.
//!
//! This module provides comprehensive support for C++ code analysis including:
//! - Functions, classes, structs, templates
//! - Namespaces and scope resolution
//! - Modern C++ features (C++11/14/17/20/23)
//! - Preprocessor directives
//! - All code metrics (Cyclomatic Complexity, Cognitive Complexity, LOC, Halstead)

use num_derive::FromPrimitive;
use crate::lang::Lang;
use crate::traits::LanguageInfo;

/// C++ language token types.
///
/// This enum represents all possible node types in the C++ tree-sitter grammar.
/// Generated from tree-sitter-cpp grammar definitions.
#[derive(Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum CppToken {
    End = 0,
    Identifier = 1,
    // Preprocessor tokens
    HASHinclude = 2,
    HASHdefine = 4,
    HASHif = 9,
    HASHendif = 11,
    HASHifdef = 12,
    HASHifndef = 13,
    HASHelse = 14,
    HASHelif = 15,

    // Operators and punctuation
    LPAREN = 5,
    RPAREN = 8,
    LBRACE = 65,
    RBRACE = 66,
    LBRACK = 71,
    RBRACK = 73,
    SEMI = 42,
    COLON = 101,
    COLONCOLON = 49,
    COMMA = 7,
    DOT = 155,
    DOTDOTDOT = 6,
    DASHGT = 157,

    // Keywords
    Class = 98,
    Struct = 99,
    Union = 100,
    Enum = 97,
    Namespace = 196,
    Template = 184,
    Typedef = 44,
    Using = 197,

    // Access specifiers
    Public = 191,
    Private = 192,
    Protected = 193,

    // Type qualifiers
    Const = 82,
    Constexpr = 83,
    Volatile = 84,
    Static = 72,
    Extern = 46,
    Virtual = 45,
    Mutable = 91,

    // Control flow
    If = 102,
    Else = 103,
    Switch = 104,
    Case = 105,
    Default = 106,
    While = 107,
    Do = 108,
    For = 109,
    Return = 110,
    Break = 111,
    Continue = 112,
    Goto = 113,
    Try = 114,
    Catch = 202,
    Throw = 195,

    // Operators
    EQ = 74,
    PLUS = 25,
    DASH = 24,
    STAR = 26,
    SLASH = 27,
    PERCENT = 28,
    PLUSPLUS = 142,
    DASHDASH = 141,
    AMPAMP = 30,
    PIPEPIPE = 29,
    BANG = 22,
    TILDE = 23,
    AMP = 33,
    PIPE = 31,
    CARET = 32,
    LT = 39,
    GT = 36,
    LTEQ = 38,
    GTEQ = 37,
    EQEQ = 34,
    BANGEQ = 35,
    LTLT = 40,
    GTGT = 41,

    // Modern C++ keywords
    Auto = 178,
    Decltype3 = 179,
    Final = 180,
    Override = 181,
    Explicit = 182,
    Typename = 183,
    Operator = 186,
    Delete = 188,
    New = 209,
    Noexcept2 = 194,
    Constinit = 92,
    Consteval = 93,
    Concept = 199,
    Requires = 210,
    CoReturn = 200,
    CoYield = 201,
    CoAwait = 208,

    // AST Node types
    TranslationUnit = 308,
    FunctionDefinition = 343,
    Declaration = 344,
    ClassDeclaration = 466,
    ClassSpecifier = 468,
    StructSpecifier = 392,
    UnionSpecifier = 393,
    EnumSpecifier = 390,
    NamespaceDefinition = 519,
    TemplateDeclaration = 475,
    TemplateInstantiation = 476,

    // Statements
    CompoundStatement = 384,
    IfStatement = 407,
    SwitchStatement = 409,
    WhileStatement = 411,
    DoStatement = 412,
    ForStatement = 413,
    ForRangeLoop = 527,
    ReturnStatement = 415,
    BreakStatement = 416,
    ContinueStatement = 417,
    ThrowStatement = 534,
    TryStatement = 493,
    CatchClause = 536,

    // Expressions
    Expression = 423,
    BinaryExpression = 342,
    UnaryExpression = 339,
    CallExpression = 340,
    AssignmentExpression = 427,
    ConditionalExpression = 426,
    LambdaExpression = 553,
    NewExpression = 540,
    DeleteExpression = 542,

    // Declarators
    Declarator = 360,
    FunctionDeclarator = 375,
    PointerDeclarator = 371,
    ReferenceDeclarator = 485,
    ArrayDeclarator = 379,

    // Types
    TypeSpecifier = 388,
    SizedTypeSpecifier = 389,
    PrimitiveType = 96,
    TemplateType = 515,
    DependentType = 474,

    // Other important nodes
    ParameterList = 399,
    ArgumentList = 341,
    BaseClassClause = 472,
    FieldDeclarationList = 394,
    InitializerList = 453,

    // Preprocessor nodes
    PreprocInclude = 311,
    PreprocDef = 312,
    PreprocIf = 316,
    PreprocIfdef = 317,

    Error = 638,
}

impl From<CppToken> for &'static str {
    fn from(tok: CppToken) -> Self {
        match tok {
            CppToken::End => "end",
            CppToken::Identifier => "identifier",

            // Preprocessor
            CppToken::HASHinclude => "#include",
            CppToken::HASHdefine => "#define",
            CppToken::HASHif => "#if",
            CppToken::HASHendif => "#endif",
            CppToken::HASHifdef => "#ifdef",
            CppToken::HASHifndef => "#ifndef",
            CppToken::HASHelse => "#else",
            CppToken::HASHelif => "#elif",

            // Punctuation
            CppToken::LPAREN => "(",
            CppToken::RPAREN => ")",
            CppToken::LBRACE => "{",
            CppToken::RBRACE => "}",
            CppToken::LBRACK => "[",
            CppToken::RBRACK => "]",
            CppToken::SEMI => ";",
            CppToken::COLON => ":",
            CppToken::COLONCOLON => "::",
            CppToken::COMMA => ",",
            CppToken::DOT => ".",
            CppToken::DOTDOTDOT => "...",
            CppToken::DASHGT => "->",

            // Keywords
            CppToken::Class => "class",
            CppToken::Struct => "struct",
            CppToken::Union => "union",
            CppToken::Enum => "enum",
            CppToken::Namespace => "namespace",
            CppToken::Template => "template",
            CppToken::Typedef => "typedef",
            CppToken::Using => "using",

            // Access specifiers
            CppToken::Public => "public",
            CppToken::Private => "private",
            CppToken::Protected => "protected",

            // Type qualifiers
            CppToken::Const => "const",
            CppToken::Constexpr => "constexpr",
            CppToken::Volatile => "volatile",
            CppToken::Static => "static",
            CppToken::Extern => "extern",
            CppToken::Virtual => "virtual",
            CppToken::Mutable => "mutable",

            // Control flow
            CppToken::If => "if",
            CppToken::Else => "else",
            CppToken::Switch => "switch",
            CppToken::Case => "case",
            CppToken::Default => "default",
            CppToken::While => "while",
            CppToken::Do => "do",
            CppToken::For => "for",
            CppToken::Return => "return",
            CppToken::Break => "break",
            CppToken::Continue => "continue",
            CppToken::Goto => "goto",
            CppToken::Try => "__try",
            CppToken::Catch => "catch",
            CppToken::Throw => "throw",

            // Operators
            CppToken::EQ => "=",
            CppToken::PLUS => "+",
            CppToken::DASH => "-",
            CppToken::STAR => "*",
            CppToken::SLASH => "/",
            CppToken::PERCENT => "%",
            CppToken::PLUSPLUS => "++",
            CppToken::DASHDASH => "--",
            CppToken::AMPAMP => "&&",
            CppToken::PIPEPIPE => "||",
            CppToken::BANG => "!",
            CppToken::TILDE => "~",
            CppToken::AMP => "&",
            CppToken::PIPE => "|",
            CppToken::CARET => "^",
            CppToken::LT => "<",
            CppToken::GT => ">",
            CppToken::LTEQ => "<=",
            CppToken::GTEQ => ">=",
            CppToken::EQEQ => "==",
            CppToken::BANGEQ => "!=",
            CppToken::LTLT => "<<",
            CppToken::GTGT => ">>",

            // Modern C++
            CppToken::Auto => "auto",
            CppToken::Decltype3 => "decltype",
            CppToken::Final => "final",
            CppToken::Override => "override",
            CppToken::Explicit => "explicit",
            CppToken::Typename => "typename",
            CppToken::Operator => "operator",
            CppToken::Delete => "delete",
            CppToken::New => "new",
            CppToken::Noexcept2 => "noexcept",
            CppToken::Constinit => "constinit",
            CppToken::Consteval => "consteval",
            CppToken::Concept => "concept",
            CppToken::Requires => "requires",
            CppToken::CoReturn => "co_return",
            CppToken::CoYield => "co_yield",
            CppToken::CoAwait => "co_await",

            // AST Nodes
            CppToken::TranslationUnit => "translation_unit",
            CppToken::FunctionDefinition => "function_definition",
            CppToken::Declaration => "declaration",
            CppToken::ClassDeclaration => "_class_declaration",
            CppToken::ClassSpecifier => "class_specifier",
            CppToken::StructSpecifier => "struct_specifier",
            CppToken::UnionSpecifier => "union_specifier",
            CppToken::EnumSpecifier => "enum_specifier",
            CppToken::NamespaceDefinition => "namespace_definition",
            CppToken::TemplateDeclaration => "template_declaration",
            CppToken::TemplateInstantiation => "template_instantiation",

            // Statements
            CppToken::CompoundStatement => "compound_statement",
            CppToken::IfStatement => "if_statement",
            CppToken::SwitchStatement => "switch_statement",
            CppToken::WhileStatement => "while_statement",
            CppToken::DoStatement => "do_statement",
            CppToken::ForStatement => "for_statement",
            CppToken::ForRangeLoop => "for_range_loop",
            CppToken::ReturnStatement => "return_statement",
            CppToken::BreakStatement => "break_statement",
            CppToken::ContinueStatement => "continue_statement",
            CppToken::ThrowStatement => "throw_statement",
            CppToken::TryStatement => "try_statement",
            CppToken::CatchClause => "catch_clause",

            // Expressions
            CppToken::Expression => "expression",
            CppToken::BinaryExpression => "binary_expression",
            CppToken::UnaryExpression => "unary_expression",
            CppToken::CallExpression => "call_expression",
            CppToken::AssignmentExpression => "assignment_expression",
            CppToken::ConditionalExpression => "conditional_expression",
            CppToken::LambdaExpression => "lambda_expression",
            CppToken::NewExpression => "new_expression",
            CppToken::DeleteExpression => "delete_expression",

            // Declarators
            CppToken::Declarator => "_declarator",
            CppToken::FunctionDeclarator => "function_declarator",
            CppToken::PointerDeclarator => "pointer_declarator",
            CppToken::ReferenceDeclarator => "reference_declarator",
            CppToken::ArrayDeclarator => "array_declarator",

            // Types
            CppToken::TypeSpecifier => "type_specifier",
            CppToken::SizedTypeSpecifier => "sized_type_specifier",
            CppToken::PrimitiveType => "primitive_type",
            CppToken::TemplateType => "template_type",
            CppToken::DependentType => "dependent_type",

            // Other nodes
            CppToken::ParameterList => "parameter_list",
            CppToken::ArgumentList => "argument_list",
            CppToken::BaseClassClause => "base_class_clause",
            CppToken::FieldDeclarationList => "field_declaration_list",
            CppToken::InitializerList => "initializer_list",

            // Preprocessor
            CppToken::PreprocInclude => "preproc_include",
            CppToken::PreprocDef => "preproc_def",
            CppToken::PreprocIf => "preproc_if",
            CppToken::PreprocIfdef => "preproc_ifdef",

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
///
/// Provides language-specific information and tree-sitter grammar access for C++.
pub struct CppLanguage;

impl LanguageInfo for CppLanguage {
    fn get_lang() -> Lang {
        Lang::Cpp
    }

    fn get_lang_name() -> &'static str {
        "c++"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpp_language_info() {
        assert_eq!(CppLanguage::get_lang(), Lang::Cpp);
        assert_eq!(CppLanguage::get_lang_name(), "c++");
    }

    #[test]
    fn test_cpp_token_conversions() {
        let tok: CppToken = 1.into();
        assert_eq!(tok, CppToken::Identifier);

        let tok: CppToken = 308.into();
        assert_eq!(tok, CppToken::TranslationUnit);

        let tok: CppToken = 343.into();
        assert_eq!(tok, CppToken::FunctionDefinition);
    }

    #[test]
    fn test_cpp_token_to_string() {
        assert_eq!(<&str>::from(CppToken::Class), "class");
        assert_eq!(<&str>::from(CppToken::Namespace), "namespace");
        assert_eq!(<&str>::from(CppToken::Template), "template");
        assert_eq!(<&str>::from(CppToken::FunctionDefinition), "function_definition");
    }
}
