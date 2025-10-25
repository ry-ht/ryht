//! Java language parser implementation.
//!
//! This module provides comprehensive support for Java code analysis including:
//! - Classes, interfaces, enums, records, annotations
//! - Methods, fields, constructors
//! - Generics, annotations, lambdas
//! - Pattern matching, switch expressions (modern Java)
//! - All code metrics (Cyclomatic Complexity, Cognitive Complexity, LOC, Halstead)

use num_derive::FromPrimitive;
use crate::lang::Lang;
use crate::traits::LanguageInfo;

/// Java language token types.
///
/// This enum represents all possible node types in the Java tree-sitter grammar.
/// Generated from tree-sitter-java grammar definitions.
#[derive(Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum JavaToken {
    End = 0,
    Identifier = 1,

    // Literals
    DecimalIntegerLiteral = 2,
    HexIntegerLiteral = 3,
    OctalIntegerLiteral = 4,
    BinaryIntegerLiteral = 5,
    DecimalFloatingPointLiteral = 6,
    HexFloatingPointLiteral = 7,
    True = 8,
    False = 9,
    CharacterLiteral = 10,
    NullLiteral = 20,

    // String tokens
    DQUOTE = 11,
    DQUOTEDQUOTEDQUOTE = 12,
    StringFragment = 13,
    EscapeSequence = 19,

    // Punctuation
    LPAREN = 21,
    RPAREN = 22,
    LBRACE = 72,
    RBRACE = 17,
    LBRACK = 65,
    RBRACK = 66,
    SEMI = 77,
    COMMA = 57,
    DOT = 67,
    COLON = 59,
    COLONCOLON = 69,
    DOTDOTDOT = 132,

    // Operators
    EQ = 24,
    PLUS = 44,
    DASH = 45,
    STAR = 46,
    SLASH = 47,
    PERCENT = 50,
    AMP = 23,
    PIPE = 48,
    CARET = 49,
    PLUSPLUS = 62,
    DASHDASH = 63,
    BANG = 60,
    TILDE = 61,
    LT = 37,
    GT = 36,
    LTEQ = 39,
    GTEQ = 38,
    EQEQ = 40,
    BANGEQ = 41,
    AMPAMP = 42,
    PIPEPIPE = 43,
    LTLT = 51,
    GTGT = 52,
    GTGTGT = 53,
    QMARK = 58,
    DASHGT = 56,

    // Assignment operators
    PLUSEQ = 25,
    DASHEQ = 26,
    STAREQ = 27,
    SLASHEQ = 28,
    AMPEQ = 29,
    PIPEEQ = 30,
    CARETEQ = 31,
    PERCENTEQ = 32,
    LTLTEQ = 33,
    GTGTEQ = 34,
    GTGTGTEQ = 35,

    // Keywords - declarations
    Class = 68,
    Interface = 122,
    Enum = 107,
    Record = 120,
    ATinterface = 121,

    // Keywords - modifiers
    Public = 108,
    Private = 110,
    Protected = 109,
    Static = 98,
    Final = 55,
    Abstract = 111,
    Synchronized = 85,
    Native = 113,
    Transient = 114,
    Volatile = 115,
    Strictfp = 112,
    Sealed = 116,
    NonDASHsealed = 117,

    // Keywords - control flow
    If = 90,
    Else = 91,
    Switch = 71,
    Case = 73,
    Default = 74,
    While = 80,
    Do = 79,
    For = 92,
    Break = 81,
    Continue = 82,
    Return = 83,
    Yield = 84,
    Throw = 86,
    Try = 87,
    Catch = 88,
    Finally = 89,
    Assert = 78,

    // Keywords - other
    Package = 105,
    Import = 106,
    Extends = 70,
    Implements = 118,
    Permits2 = 119,
    New = 64,
    This = 134,
    Super = 135,
    Instanceof = 54,
    Throws2 = 133,
    When = 76,
    With = 104,

    // Module system keywords
    Open = 94,
    Module = 95,
    Requires = 96,
    Transitive = 97,
    Exports = 99,
    To = 100,
    Opens = 101,
    Uses = 102,
    Provides = 103,

    // Primitive types
    Byte = 123,
    Short = 124,
    Int = 125,
    Long = 126,
    Char = 127,
    Float = 128,
    Double = 129,
    BooleanType = 130,
    VoidType = 131,

    // Annotations
    AT = 93,

    // Special patterns
    UnderscorePattern = 75,

    // Comments
    LineComment = 136,
    BlockComment = 137,

    // AST Nodes - Top level
    Program = 138,
    PackageDeclaration = 226,
    ImportDeclaration = 227,

    // AST Nodes - Declarations
    ClassDeclaration = 233,
    InterfaceDeclaration = 255,
    EnumDeclaration = 229,
    RecordDeclaration = 250,
    AnnotationTypeDeclaration = 251,
    MethodDeclaration = 279,
    FieldDeclaration = 249,
    ConstructorDeclaration = 244,

    // AST Nodes - Modifiers and parameters
    Modifiers = 234,
    TypeParameters = 235,
    TypeParameter = 236,
    TypeBound = 237,
    FormalParameters = 273,
    FormalParameter = 274,

    // AST Nodes - Class structure
    ClassBody = 242,
    InterfaceBody = 257,
    EnumBody = 230,
    AnnotationTypeBody = 252,

    // AST Nodes - Statements
    Block = 186,
    IfStatement = 205,
    SwitchStatement = 195,
    SwitchExpression = 174,
    WhileStatement = 206,
    DoStatement = 190,
    ForStatement = 207,
    EnhancedForStatement = 208,
    TryStatement = 197,
    CatchClause = 198,
    ThrowStatement = 196,
    ReturnStatement = 193,
    YieldStatement = 194,
    BreakStatement = 191,
    ContinueStatement = 192,
    AssertStatement = 189,
    SynchronizedStatement = 212,

    // AST Nodes - Expressions
    Expression = 147,
    BinaryExpression = 150,
    UnaryExpression = 155,
    UpdateExpression = 156,
    CastExpression = 148,
    AssignmentExpression = 149,
    TernaryExpression = 154,
    LambdaExpression = 152,
    InstanceofExpression = 151,
    MethodInvocation = 167,
    FieldAccess = 164,
    ArrayAccess = 166,
    ObjectCreationExpression = 162,
    ArrayCreationExpression = 158,
    MethodReference = 169,

    // AST Nodes - Patterns
    Pattern = 179,
    TypePattern = 180,
    RecordPattern = 181,

    // AST Nodes - Types
    Type = 263,
    GenericType = 267,
    ArrayType = 268,
    ScopedTypeIdentifier = 266,

    // AST Nodes - Annotations
    Annotation = 211,
    MarkerAnnotation = 210,

    // AST Nodes - Other
    ArgumentList = 168,
    TypeArguments = 170,
    Wildcard = 171,
    ScopedIdentifier = 248,
    Literal = 140,
    StringLiteral = 141,

    Error = 321,
}

impl From<JavaToken> for &'static str {
    fn from(tok: JavaToken) -> Self {
        match tok {
            JavaToken::End => "end",
            JavaToken::Identifier => "identifier",

            // Literals
            JavaToken::DecimalIntegerLiteral => "decimal_integer_literal",
            JavaToken::HexIntegerLiteral => "hex_integer_literal",
            JavaToken::OctalIntegerLiteral => "octal_integer_literal",
            JavaToken::BinaryIntegerLiteral => "binary_integer_literal",
            JavaToken::DecimalFloatingPointLiteral => "decimal_floating_point_literal",
            JavaToken::HexFloatingPointLiteral => "hex_floating_point_literal",
            JavaToken::True => "true",
            JavaToken::False => "false",
            JavaToken::CharacterLiteral => "character_literal",
            JavaToken::NullLiteral => "null_literal",

            // String tokens
            JavaToken::DQUOTE => "\"",
            JavaToken::DQUOTEDQUOTEDQUOTE => "\"\"\"",
            JavaToken::StringFragment => "string_fragment",
            JavaToken::EscapeSequence => "escape_sequence",

            // Punctuation
            JavaToken::LPAREN => "(",
            JavaToken::RPAREN => ")",
            JavaToken::LBRACE => "{",
            JavaToken::RBRACE => "}",
            JavaToken::LBRACK => "[",
            JavaToken::RBRACK => "]",
            JavaToken::SEMI => ";",
            JavaToken::COMMA => ",",
            JavaToken::DOT => ".",
            JavaToken::COLON => ":",
            JavaToken::COLONCOLON => "::",
            JavaToken::DOTDOTDOT => "...",

            // Operators
            JavaToken::EQ => "=",
            JavaToken::PLUS => "+",
            JavaToken::DASH => "-",
            JavaToken::STAR => "*",
            JavaToken::SLASH => "/",
            JavaToken::PERCENT => "%",
            JavaToken::AMP => "&",
            JavaToken::PIPE => "|",
            JavaToken::CARET => "^",
            JavaToken::PLUSPLUS => "++",
            JavaToken::DASHDASH => "--",
            JavaToken::BANG => "!",
            JavaToken::TILDE => "~",
            JavaToken::LT => "<",
            JavaToken::GT => ">",
            JavaToken::LTEQ => "<=",
            JavaToken::GTEQ => ">=",
            JavaToken::EQEQ => "==",
            JavaToken::BANGEQ => "!=",
            JavaToken::AMPAMP => "&&",
            JavaToken::PIPEPIPE => "||",
            JavaToken::LTLT => "<<",
            JavaToken::GTGT => ">>",
            JavaToken::GTGTGT => ">>>",
            JavaToken::QMARK => "?",
            JavaToken::DASHGT => "->",

            // Assignment operators
            JavaToken::PLUSEQ => "+=",
            JavaToken::DASHEQ => "-=",
            JavaToken::STAREQ => "*=",
            JavaToken::SLASHEQ => "/=",
            JavaToken::AMPEQ => "&=",
            JavaToken::PIPEEQ => "|=",
            JavaToken::CARETEQ => "^=",
            JavaToken::PERCENTEQ => "%=",
            JavaToken::LTLTEQ => "<<=",
            JavaToken::GTGTEQ => ">>=",
            JavaToken::GTGTGTEQ => ">>>=",

            // Keywords - declarations
            JavaToken::Class => "class",
            JavaToken::Interface => "interface",
            JavaToken::Enum => "enum",
            JavaToken::Record => "record",
            JavaToken::ATinterface => "@interface",

            // Keywords - modifiers
            JavaToken::Public => "public",
            JavaToken::Private => "private",
            JavaToken::Protected => "protected",
            JavaToken::Static => "static",
            JavaToken::Final => "final",
            JavaToken::Abstract => "abstract",
            JavaToken::Synchronized => "synchronized",
            JavaToken::Native => "native",
            JavaToken::Transient => "transient",
            JavaToken::Volatile => "volatile",
            JavaToken::Strictfp => "strictfp",
            JavaToken::Sealed => "sealed",
            JavaToken::NonDASHsealed => "non-sealed",

            // Keywords - control flow
            JavaToken::If => "if",
            JavaToken::Else => "else",
            JavaToken::Switch => "switch",
            JavaToken::Case => "case",
            JavaToken::Default => "default",
            JavaToken::While => "while",
            JavaToken::Do => "do",
            JavaToken::For => "for",
            JavaToken::Break => "break",
            JavaToken::Continue => "continue",
            JavaToken::Return => "return",
            JavaToken::Yield => "yield",
            JavaToken::Throw => "throw",
            JavaToken::Try => "try",
            JavaToken::Catch => "catch",
            JavaToken::Finally => "finally",
            JavaToken::Assert => "assert",

            // Keywords - other
            JavaToken::Package => "package",
            JavaToken::Import => "import",
            JavaToken::Extends => "extends",
            JavaToken::Implements => "implements",
            JavaToken::Permits2 => "permits",
            JavaToken::New => "new",
            JavaToken::This => "this",
            JavaToken::Super => "super",
            JavaToken::Instanceof => "instanceof",
            JavaToken::Throws2 => "throws",
            JavaToken::When => "when",
            JavaToken::With => "with",

            // Module system
            JavaToken::Open => "open",
            JavaToken::Module => "module",
            JavaToken::Requires => "requires",
            JavaToken::Transitive => "transitive",
            JavaToken::Exports => "exports",
            JavaToken::To => "to",
            JavaToken::Opens => "opens",
            JavaToken::Uses => "uses",
            JavaToken::Provides => "provides",

            // Primitive types
            JavaToken::Byte => "byte",
            JavaToken::Short => "short",
            JavaToken::Int => "int",
            JavaToken::Long => "long",
            JavaToken::Char => "char",
            JavaToken::Float => "float",
            JavaToken::Double => "double",
            JavaToken::BooleanType => "boolean_type",
            JavaToken::VoidType => "void_type",

            // Annotations
            JavaToken::AT => "@",

            // Special patterns
            JavaToken::UnderscorePattern => "underscore_pattern",

            // Comments
            JavaToken::LineComment => "line_comment",
            JavaToken::BlockComment => "block_comment",

            // AST Nodes - Top level
            JavaToken::Program => "program",
            JavaToken::PackageDeclaration => "package_declaration",
            JavaToken::ImportDeclaration => "import_declaration",

            // AST Nodes - Declarations
            JavaToken::ClassDeclaration => "class_declaration",
            JavaToken::InterfaceDeclaration => "interface_declaration",
            JavaToken::EnumDeclaration => "enum_declaration",
            JavaToken::RecordDeclaration => "record_declaration",
            JavaToken::AnnotationTypeDeclaration => "annotation_type_declaration",
            JavaToken::MethodDeclaration => "method_declaration",
            JavaToken::FieldDeclaration => "field_declaration",
            JavaToken::ConstructorDeclaration => "constructor_declaration",

            // AST Nodes - Modifiers and parameters
            JavaToken::Modifiers => "modifiers",
            JavaToken::TypeParameters => "type_parameters",
            JavaToken::TypeParameter => "type_parameter",
            JavaToken::TypeBound => "type_bound",
            JavaToken::FormalParameters => "formal_parameters",
            JavaToken::FormalParameter => "formal_parameter",

            // AST Nodes - Class structure
            JavaToken::ClassBody => "class_body",
            JavaToken::InterfaceBody => "interface_body",
            JavaToken::EnumBody => "enum_body",
            JavaToken::AnnotationTypeBody => "annotation_type_body",

            // AST Nodes - Statements
            JavaToken::Block => "block",
            JavaToken::IfStatement => "if_statement",
            JavaToken::SwitchStatement => "switch_statement",
            JavaToken::SwitchExpression => "switch_expression",
            JavaToken::WhileStatement => "while_statement",
            JavaToken::DoStatement => "do_statement",
            JavaToken::ForStatement => "for_statement",
            JavaToken::EnhancedForStatement => "enhanced_for_statement",
            JavaToken::TryStatement => "try_statement",
            JavaToken::CatchClause => "catch_clause",
            JavaToken::ThrowStatement => "throw_statement",
            JavaToken::ReturnStatement => "return_statement",
            JavaToken::YieldStatement => "yield_statement",
            JavaToken::BreakStatement => "break_statement",
            JavaToken::ContinueStatement => "continue_statement",
            JavaToken::AssertStatement => "assert_statement",
            JavaToken::SynchronizedStatement => "synchronized_statement",

            // AST Nodes - Expressions
            JavaToken::Expression => "expression",
            JavaToken::BinaryExpression => "binary_expression",
            JavaToken::UnaryExpression => "unary_expression",
            JavaToken::UpdateExpression => "update_expression",
            JavaToken::CastExpression => "cast_expression",
            JavaToken::AssignmentExpression => "assignment_expression",
            JavaToken::TernaryExpression => "ternary_expression",
            JavaToken::LambdaExpression => "lambda_expression",
            JavaToken::InstanceofExpression => "instanceof_expression",
            JavaToken::MethodInvocation => "method_invocation",
            JavaToken::FieldAccess => "field_access",
            JavaToken::ArrayAccess => "array_access",
            JavaToken::ObjectCreationExpression => "object_creation_expression",
            JavaToken::ArrayCreationExpression => "array_creation_expression",
            JavaToken::MethodReference => "method_reference",

            // AST Nodes - Patterns
            JavaToken::Pattern => "pattern",
            JavaToken::TypePattern => "type_pattern",
            JavaToken::RecordPattern => "record_pattern",

            // AST Nodes - Types
            JavaToken::Type => "_type",
            JavaToken::GenericType => "generic_type",
            JavaToken::ArrayType => "array_type",
            JavaToken::ScopedTypeIdentifier => "scoped_type_identifier",

            // AST Nodes - Annotations
            JavaToken::Annotation => "annotation",
            JavaToken::MarkerAnnotation => "marker_annotation",

            // AST Nodes - Other
            JavaToken::ArgumentList => "argument_list",
            JavaToken::TypeArguments => "type_arguments",
            JavaToken::Wildcard => "wildcard",
            JavaToken::ScopedIdentifier => "scoped_identifier",
            JavaToken::Literal => "_literal",
            JavaToken::StringLiteral => "string_literal",

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
///
/// Provides language-specific information and tree-sitter grammar access for Java.
pub struct JavaLanguage;

impl LanguageInfo for JavaLanguage {
    fn get_lang() -> Lang {
        Lang::Java
    }

    fn get_lang_name() -> &'static str {
        "java"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_java_language_info() {
        assert_eq!(JavaLanguage::get_lang(), Lang::Java);
        assert_eq!(JavaLanguage::get_lang_name(), "java");
    }

    #[test]
    fn test_java_token_conversions() {
        let tok: JavaToken = 1.into();
        assert_eq!(tok, JavaToken::Identifier);

        let tok: JavaToken = 138.into();
        assert_eq!(tok, JavaToken::Program);

        let tok: JavaToken = 233.into();
        assert_eq!(tok, JavaToken::ClassDeclaration);

        let tok: JavaToken = 279.into();
        assert_eq!(tok, JavaToken::MethodDeclaration);
    }

    #[test]
    fn test_java_token_to_string() {
        assert_eq!(<&str>::from(JavaToken::Class), "class");
        assert_eq!(<&str>::from(JavaToken::Interface), "interface");
        assert_eq!(<&str>::from(JavaToken::ClassDeclaration), "class_declaration");
        assert_eq!(<&str>::from(JavaToken::MethodDeclaration), "method_declaration");
    }
}
