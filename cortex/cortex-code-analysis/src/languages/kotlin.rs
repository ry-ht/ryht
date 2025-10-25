//! Kotlin language parser implementation.
//!
//! This module provides comprehensive support for Kotlin code analysis including:
//! - Classes, objects, interfaces, data classes
//! - Functions, properties, constructors
//! - Lambdas, extension functions, coroutines
//! - Nullable types, sealed classes, inline functions
//! - All code metrics (Cyclomatic Complexity, Cognitive Complexity, LOC, Halstead)

use num_derive::FromPrimitive;
use crate::lang::Lang;
use crate::traits::LanguageInfo;

/// Kotlin language token types.
///
/// This enum represents all possible node types in the Kotlin tree-sitter grammar.
/// Generated from tree-sitter-kotlin grammar definitions.
#[derive(Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum KotlinToken {
    End = 0,
    Identifier = 1,

    // Keywords - declarations
    Class = 13,
    Fun = 14,
    Interface = 15,
    Object = 16,
    Val = 17,
    Var = 18,
    Typealias = 20,
    Companion = 21,
    Init = 22,
    Constructor = 23,
    Enum = 41,
    Sealed = 42,
    Annotation2 = 43,
    Data = 44,
    Inner = 45,
    Value = 46,

    // Keywords - modifiers
    Public = 54,
    Private = 55,
    Protected = 56,
    Internal = 57,
    Abstract = 58,
    Final = 59,
    Open = 60,
    Override = 61,
    Lateinit = 62,
    Tailrec = 47,
    Operator = 48,
    Infix = 49,
    Inline = 50,
    External = 51,
    Suspend = 52,
    Const = 53,
    Vararg = 63,
    Noinline = 64,
    Crossinline = 65,

    // Keywords - control flow
    If = 109,
    Else = 110,
    When = 111,
    For = 37,
    In = 38,
    While = 39,
    Do = 40,
    Try = 112,
    Catch = 113,
    Finally = 114,
    Return = 115,
    ReturnAT = 116,
    Throw = 117,
    Break = 118,
    Continue = 119,

    // Keywords - other
    Package = 7,
    Import2 = 9,
    As = 12,
    This = 24,
    Super = 25,
    By = 32,
    Get = 33,
    Set = 34,
    Where = 31,
    Is = 102,
    Dynamic = 75,

    // Operators and punctuation
    DOT = 10,
    STAR = 11,
    COLON = 4,
    LBRACK = 5,
    RBRACK = 6,
    SEMI = 8,
    LPAREN = 29,
    RPAREN = 30,
    LBRACE = 35,
    RBRACE = 36,
    COMMA = 27,
    LT = 26,
    GT = 28,
    AT = 2,
    EQ = 19,
    QMARK = 76,
    AMP = 77,
    DASHGT = 78,

    // Assignment operators
    PLUSEQ = 79,
    DASHEQ = 80,
    STAREQ = 135,
    SLASHEQ = 136,
    PERCENTEQ = 83,

    // Increment/decrement
    PLUSPLUS = 84,
    DASHDASH = 85,

    // Arithmetic operators
    PLUS = 86,
    DASH = 87,
    SLASH = 90,
    PERCENT = 91,

    // Logical operators
    BANG = 88,
    BANGBANG = 89,
    PIPEPIPE = 92,
    AMPAMP = 93,

    // Comparison operators
    BANGEQ = 94,
    BANGEQEQ = 95,
    EQEQ = 96,
    EQEQEQ = 97,
    GTEQ = 98,
    LTEQ = 99,

    // Other operators
    QMARKCOLON = 100,
    BANGin = 101,
    AsQMARK = 103,
    DOTDOT = 104,
    DOTDOTLT = 105,
    COLONCOLON = 138,
    QMARKDOT = 140,

    // Literals and strings
    DQUOTE = 139,
    StringContent = 120,
    DOLLAR = 122,
    DQUOTEDQUOTEDQUOTE = 123,
    DOLLARLBRACE = 125,
    SQUOTE = 126,
    CharacterLiteralToken1 = 127,
    EscapeSequence = 128,
    NumberLiteral = 129,
    FloatLiteral = 130,

    // Special labels and annotations
    ThisAT = 106,
    SuperAT = 107,
    AT2 = 108,
    Label = 132,

    // Comments and shebang
    Shebang = 133,
    LineComment = 134,
    BlockComment = 137,

    // Type modifiers
    Out = 131,
    Expect = 67,
    Actual = 68,
    ReificationModifier = 66,

    // Use site targets
    Field = 69,
    Property = 70,
    Receiver = 71,
    Param = 72,
    Setparam = 73,
    Delegate = 74,

    // AST Node types
    SourceFile = 142,
    FileAnnotation = 143,
    PackageHeader = 144,
    Import = 145,
    Declaration = 146,
    ClassDeclaration = 147,
    ObjectDeclaration = 148,
    PropertyDeclaration = 149,
    TypeAlias = 150,
    CompanionObject = 151,
    AnonymousInitializer = 152,
    SecondaryConstructor = 153,
    FunctionDeclaration = 163,
    PrimaryConstructor = 157,

    // Class structure
    ClassParameters = 158,
    ClassParameter = 159,
    TypeParameters = 155,
    TypeParameter = 156,
    TypeConstraints = 160,
    TypeConstraint = 161,
    ClassBody = 179,
    EnumClassBody = 181,
    EnumEntry = 182,

    // Type system
    Type = 200,
    UserType = 201,
    NullableType = 203,
    FunctionType = 208,
    FunctionTypeParameters = 209,
    ParenthesizedType = 210,

    // Parameters and delegation
    FunctionValueParameters = 164,
    Parameter = 165,
    DelegationSpecifiers = 166,
    DelegationSpecifier = 167,
    ExplicitDelegation = 171,

    // Properties
    VariableDeclaration = 168,
    MultiVariableDeclaration = 169,
    PropertyDelegate = 170,
    Getter = 172,
    Setter = 173,

    // Statements and blocks
    FunctionBody = 174,
    Block = 175,
    ForStatement = 176,
    WhileStatement = 177,
    DoWhileStatement = 178,
    Statement = 185,

    // Modifiers
    Modifiers = 186,
    ClassModifier = 187,
    FunctionModifier = 188,
    PropertyModifier = 189,
    VisibilityModifier = 190,
    InheritanceModifier = 191,
    MemberModifier = 192,
    ParameterModifiers = 193,
    ParameterModifier = 194,
    PlatformModifier = 195,
    TypeModifiers = 196,

    // Annotations
    Annotation = 197,
    UseSiteTarget = 198,

    // Expressions
    Assignment = 211,
    Expression = 212,
    PrimaryExpression = 213,
    UnaryExpression = 214,
    BinaryExpression = 217,
    InExpression = 218,
    IsExpression = 219,
    AsExpression = 220,
    RangeExpression = 222,
    InfixExpression = 223,
    CallExpression = 224,

    // Lambda and functions
    LambdaLiteral = 226,
    LambdaParameters = 227,
    AnonymousFunction = 229,

    // Control flow expressions
    IfExpression = 233,
    WhenExpression = 236,
    WhenSubject = 237,
    WhenEntry = 238,
    TryExpression = 242,
    CatchBlock = 243,
    FinallyBlock = 244,
    ReturnExpression = 245,
    ThrowExpression = 246,

    // Other expressions
    IndexExpression = 230,
    ThisExpression = 231,
    SuperExpression = 232,
    ParenthesizedExpression = 234,
    CollectionLiteral = 235,
    NavigationExpression = 248,
    ObjectLiteral = 249,
    CallableReference = 247,

    // String literals
    StringLiteral = 250,
    MultilineStringLiteral = 251,
    Interpolation = 252,
    CharacterLiteral = 253,

    // Value arguments
    ValueArguments = 183,
    ValueArgument = 184,

    Error = 289,
}

impl From<KotlinToken> for &'static str {
    fn from(tok: KotlinToken) -> Self {
        match tok {
            KotlinToken::End => "end",
            KotlinToken::Identifier => "identifier",

            // Keywords - declarations
            KotlinToken::Class => "class",
            KotlinToken::Fun => "fun",
            KotlinToken::Interface => "interface",
            KotlinToken::Object => "object",
            KotlinToken::Val => "val",
            KotlinToken::Var => "var",
            KotlinToken::Typealias => "typealias",
            KotlinToken::Companion => "companion",
            KotlinToken::Init => "init",
            KotlinToken::Constructor => "constructor",
            KotlinToken::Enum => "enum",
            KotlinToken::Sealed => "sealed",
            KotlinToken::Annotation2 => "annotation",
            KotlinToken::Data => "data",
            KotlinToken::Inner => "inner",
            KotlinToken::Value => "value",

            // Keywords - modifiers
            KotlinToken::Public => "public",
            KotlinToken::Private => "private",
            KotlinToken::Protected => "protected",
            KotlinToken::Internal => "internal",
            KotlinToken::Abstract => "abstract",
            KotlinToken::Final => "final",
            KotlinToken::Open => "open",
            KotlinToken::Override => "override",
            KotlinToken::Lateinit => "lateinit",
            KotlinToken::Tailrec => "tailrec",
            KotlinToken::Operator => "operator",
            KotlinToken::Infix => "infix",
            KotlinToken::Inline => "inline",
            KotlinToken::External => "external",
            KotlinToken::Suspend => "suspend",
            KotlinToken::Const => "const",
            KotlinToken::Vararg => "vararg",
            KotlinToken::Noinline => "noinline",
            KotlinToken::Crossinline => "crossinline",

            // Keywords - control flow
            KotlinToken::If => "if",
            KotlinToken::Else => "else",
            KotlinToken::When => "when",
            KotlinToken::For => "for",
            KotlinToken::In => "in",
            KotlinToken::While => "while",
            KotlinToken::Do => "do",
            KotlinToken::Try => "try",
            KotlinToken::Catch => "catch",
            KotlinToken::Finally => "finally",
            KotlinToken::Return => "return",
            KotlinToken::ReturnAT => "return@",
            KotlinToken::Throw => "throw",
            KotlinToken::Break => "break",
            KotlinToken::Continue => "continue",

            // Keywords - other
            KotlinToken::Package => "package",
            KotlinToken::Import2 => "import",
            KotlinToken::As => "as",
            KotlinToken::This => "this",
            KotlinToken::Super => "super",
            KotlinToken::By => "by",
            KotlinToken::Get => "get",
            KotlinToken::Set => "set",
            KotlinToken::Where => "where",
            KotlinToken::Is => "is",
            KotlinToken::Dynamic => "dynamic",

            // Operators and punctuation
            KotlinToken::DOT => ".",
            KotlinToken::STAR => "*",
            KotlinToken::COLON => ":",
            KotlinToken::LBRACK => "[",
            KotlinToken::RBRACK => "]",
            KotlinToken::SEMI => ";",
            KotlinToken::LPAREN => "(",
            KotlinToken::RPAREN => ")",
            KotlinToken::LBRACE => "{",
            KotlinToken::RBRACE => "}",
            KotlinToken::COMMA => ",",
            KotlinToken::LT => "<",
            KotlinToken::GT => ">",
            KotlinToken::AT => "@",
            KotlinToken::EQ => "=",
            KotlinToken::QMARK => "?",
            KotlinToken::AMP => "&",
            KotlinToken::DASHGT => "->",

            // Assignment operators
            KotlinToken::PLUSEQ => "+=",
            KotlinToken::DASHEQ => "-=",
            KotlinToken::STAREQ => "*=",
            KotlinToken::SLASHEQ => "/=",
            KotlinToken::PERCENTEQ => "%=",

            // Increment/decrement
            KotlinToken::PLUSPLUS => "++",
            KotlinToken::DASHDASH => "--",

            // Arithmetic operators
            KotlinToken::PLUS => "+",
            KotlinToken::DASH => "-",
            KotlinToken::SLASH => "/",
            KotlinToken::PERCENT => "%",

            // Logical operators
            KotlinToken::BANG => "!",
            KotlinToken::BANGBANG => "!!",
            KotlinToken::PIPEPIPE => "||",
            KotlinToken::AMPAMP => "&&",

            // Comparison operators
            KotlinToken::BANGEQ => "!=",
            KotlinToken::BANGEQEQ => "!==",
            KotlinToken::EQEQ => "==",
            KotlinToken::EQEQEQ => "===",
            KotlinToken::GTEQ => ">=",
            KotlinToken::LTEQ => "<=",

            // Other operators
            KotlinToken::QMARKCOLON => "?:",
            KotlinToken::BANGin => "!in",
            KotlinToken::AsQMARK => "as?",
            KotlinToken::DOTDOT => "..",
            KotlinToken::DOTDOTLT => "..<",
            KotlinToken::COLONCOLON => "::",
            KotlinToken::QMARKDOT => "?.",

            // Literals and strings
            KotlinToken::DQUOTE => "\"",
            KotlinToken::StringContent => "string_content",
            KotlinToken::DOLLAR => "$",
            KotlinToken::DQUOTEDQUOTEDQUOTE => "\"\"\"",
            KotlinToken::DOLLARLBRACE => "${",
            KotlinToken::SQUOTE => "'",
            KotlinToken::CharacterLiteralToken1 => "character_literal_token1",
            KotlinToken::EscapeSequence => "escape_sequence",
            KotlinToken::NumberLiteral => "number_literal",
            KotlinToken::FloatLiteral => "float_literal",

            // Special labels and annotations
            KotlinToken::ThisAT => "this@",
            KotlinToken::SuperAT => "super@",
            KotlinToken::AT2 => "@",
            KotlinToken::Label => "label",

            // Comments and shebang
            KotlinToken::Shebang => "shebang",
            KotlinToken::LineComment => "line_comment",
            KotlinToken::BlockComment => "block_comment",

            // Type modifiers
            KotlinToken::Out => "out",
            KotlinToken::Expect => "expect",
            KotlinToken::Actual => "actual",
            KotlinToken::ReificationModifier => "reification_modifier",

            // Use site targets
            KotlinToken::Field => "field",
            KotlinToken::Property => "property",
            KotlinToken::Receiver => "receiver",
            KotlinToken::Param => "param",
            KotlinToken::Setparam => "setparam",
            KotlinToken::Delegate => "delegate",

            // AST Node types
            KotlinToken::SourceFile => "source_file",
            KotlinToken::FileAnnotation => "file_annotation",
            KotlinToken::PackageHeader => "package_header",
            KotlinToken::Import => "import",
            KotlinToken::Declaration => "declaration",
            KotlinToken::ClassDeclaration => "class_declaration",
            KotlinToken::ObjectDeclaration => "object_declaration",
            KotlinToken::PropertyDeclaration => "property_declaration",
            KotlinToken::TypeAlias => "type_alias",
            KotlinToken::CompanionObject => "companion_object",
            KotlinToken::AnonymousInitializer => "anonymous_initializer",
            KotlinToken::SecondaryConstructor => "secondary_constructor",
            KotlinToken::FunctionDeclaration => "function_declaration",
            KotlinToken::PrimaryConstructor => "primary_constructor",

            // Class structure
            KotlinToken::ClassParameters => "class_parameters",
            KotlinToken::ClassParameter => "class_parameter",
            KotlinToken::TypeParameters => "type_parameters",
            KotlinToken::TypeParameter => "type_parameter",
            KotlinToken::TypeConstraints => "type_constraints",
            KotlinToken::TypeConstraint => "type_constraint",
            KotlinToken::ClassBody => "class_body",
            KotlinToken::EnumClassBody => "enum_class_body",
            KotlinToken::EnumEntry => "enum_entry",

            // Type system
            KotlinToken::Type => "type",
            KotlinToken::UserType => "user_type",
            KotlinToken::NullableType => "nullable_type",
            KotlinToken::FunctionType => "function_type",
            KotlinToken::FunctionTypeParameters => "function_type_parameters",
            KotlinToken::ParenthesizedType => "parenthesized_type",

            // Parameters and delegation
            KotlinToken::FunctionValueParameters => "function_value_parameters",
            KotlinToken::Parameter => "parameter",
            KotlinToken::DelegationSpecifiers => "delegation_specifiers",
            KotlinToken::DelegationSpecifier => "delegation_specifier",
            KotlinToken::ExplicitDelegation => "explicit_delegation",

            // Properties
            KotlinToken::VariableDeclaration => "variable_declaration",
            KotlinToken::MultiVariableDeclaration => "multi_variable_declaration",
            KotlinToken::PropertyDelegate => "property_delegate",
            KotlinToken::Getter => "getter",
            KotlinToken::Setter => "setter",

            // Statements and blocks
            KotlinToken::FunctionBody => "function_body",
            KotlinToken::Block => "block",
            KotlinToken::ForStatement => "for_statement",
            KotlinToken::WhileStatement => "while_statement",
            KotlinToken::DoWhileStatement => "do_while_statement",
            KotlinToken::Statement => "statement",

            // Modifiers
            KotlinToken::Modifiers => "modifiers",
            KotlinToken::ClassModifier => "class_modifier",
            KotlinToken::FunctionModifier => "function_modifier",
            KotlinToken::PropertyModifier => "property_modifier",
            KotlinToken::VisibilityModifier => "visibility_modifier",
            KotlinToken::InheritanceModifier => "inheritance_modifier",
            KotlinToken::MemberModifier => "member_modifier",
            KotlinToken::ParameterModifiers => "parameter_modifiers",
            KotlinToken::ParameterModifier => "parameter_modifier",
            KotlinToken::PlatformModifier => "platform_modifier",
            KotlinToken::TypeModifiers => "type_modifiers",

            // Annotations
            KotlinToken::Annotation => "annotation",
            KotlinToken::UseSiteTarget => "use_site_target",

            // Expressions
            KotlinToken::Assignment => "assignment",
            KotlinToken::Expression => "expression",
            KotlinToken::PrimaryExpression => "primary_expression",
            KotlinToken::UnaryExpression => "unary_expression",
            KotlinToken::BinaryExpression => "binary_expression",
            KotlinToken::InExpression => "in_expression",
            KotlinToken::IsExpression => "is_expression",
            KotlinToken::AsExpression => "as_expression",
            KotlinToken::RangeExpression => "range_expression",
            KotlinToken::InfixExpression => "infix_expression",
            KotlinToken::CallExpression => "call_expression",

            // Lambda and functions
            KotlinToken::LambdaLiteral => "lambda_literal",
            KotlinToken::LambdaParameters => "lambda_parameters",
            KotlinToken::AnonymousFunction => "anonymous_function",

            // Control flow expressions
            KotlinToken::IfExpression => "if_expression",
            KotlinToken::WhenExpression => "when_expression",
            KotlinToken::WhenSubject => "when_subject",
            KotlinToken::WhenEntry => "when_entry",
            KotlinToken::TryExpression => "try_expression",
            KotlinToken::CatchBlock => "catch_block",
            KotlinToken::FinallyBlock => "finally_block",
            KotlinToken::ReturnExpression => "return_expression",
            KotlinToken::ThrowExpression => "throw_expression",

            // Other expressions
            KotlinToken::IndexExpression => "index_expression",
            KotlinToken::ThisExpression => "this_expression",
            KotlinToken::SuperExpression => "super_expression",
            KotlinToken::ParenthesizedExpression => "parenthesized_expression",
            KotlinToken::CollectionLiteral => "collection_literal",
            KotlinToken::NavigationExpression => "navigation_expression",
            KotlinToken::ObjectLiteral => "object_literal",
            KotlinToken::CallableReference => "callable_reference",

            // String literals
            KotlinToken::StringLiteral => "string_literal",
            KotlinToken::MultilineStringLiteral => "multiline_string_literal",
            KotlinToken::Interpolation => "interpolation",
            KotlinToken::CharacterLiteral => "character_literal",

            // Value arguments
            KotlinToken::ValueArguments => "value_arguments",
            KotlinToken::ValueArgument => "value_argument",

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
///
/// Provides language-specific information and tree-sitter grammar access for Kotlin.
pub struct KotlinLanguage;

impl LanguageInfo for KotlinLanguage {
    fn get_lang() -> Lang {
        Lang::Kotlin
    }

    fn get_lang_name() -> &'static str {
        "kotlin"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kotlin_language_info() {
        assert_eq!(KotlinLanguage::get_lang(), Lang::Kotlin);
        assert_eq!(KotlinLanguage::get_lang_name(), "kotlin");
    }

    #[test]
    fn test_kotlin_token_conversions() {
        let tok: KotlinToken = 1.into();
        assert_eq!(tok, KotlinToken::Identifier);

        let tok: KotlinToken = 142.into();
        assert_eq!(tok, KotlinToken::SourceFile);

        let tok: KotlinToken = 147.into();
        assert_eq!(tok, KotlinToken::ClassDeclaration);

        let tok: KotlinToken = 163.into();
        assert_eq!(tok, KotlinToken::FunctionDeclaration);
    }

    #[test]
    fn test_kotlin_token_to_string() {
        assert_eq!(<&str>::from(KotlinToken::Class), "class");
        assert_eq!(<&str>::from(KotlinToken::Fun), "fun");
        assert_eq!(<&str>::from(KotlinToken::Data), "data");
        assert_eq!(<&str>::from(KotlinToken::FunctionDeclaration), "function_declaration");
    }
}
