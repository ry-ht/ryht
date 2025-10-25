//! TSX (TypeScript with JSX) language parser implementation.
//!
//! This module provides comprehensive support for TSX code analysis including:
//! - React components, JSX elements
//! - TypeScript types, interfaces, generics
//! - Classes, functions, arrow functions
//! - Import/export statements
//! - All code metrics (Cyclomatic Complexity, Cognitive Complexity, LOC, Halstead)

use num_derive::FromPrimitive;
use crate::lang::Lang;
use crate::traits::LanguageInfo;

/// TSX language token types.
///
/// This enum represents all possible node types in the TSX tree-sitter grammar.
/// Generated from tree-sitter-tsx grammar definitions.
#[derive(Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum TsxToken {
    End = 0,
    Identifier = 1,
    HashBangLine = 2,

    // Keywords - exports and imports
    Export = 3,
    Default = 5,
    Type = 6,
    As = 8,
    Import2 = 14,
    From = 15,

    // Keywords - declarations
    Var = 18,
    Let = 19,
    Const = 20,
    Function = 59,
    Class2 = 57,
    Interface = 151,
    Enum = 152,
    Namespace = 9,
    Module2 = 137,

    // Keywords - types and modifiers
    Typeof = 13,
    Async = 58,
    Static = 127,
    Readonly = 128,
    Public = 133,
    Private = 134,
    Protected = 135,
    Abstract = 144,
    Declare = 132,
    Override = 136,

    // Keywords - control flow
    If = 23,
    Else = 22,
    Switch = 24,
    Case = 41,
    Default2 = 17,
    For = 25,
    While = 32,
    Do = 33,
    Try = 34,
    Catch = 42,
    Finally = 43,
    Break = 35,
    Continue = 36,
    Return = 38,
    Throw = 39,
    Debugger = 37,
    Yield = 44,
    Await = 29,

    // Keywords - other
    In = 30,
    Of = 31,
    With = 16,
    New = 62,
    This = 120,
    Super = 121,
    Void = 103,
    Delete = 104,
    Instanceof = 101,
    Extends = 148,
    Implements = 149,

    // Operators - arithmetic
    PLUS = 88,
    DASH = 89,
    STAR = 4,
    SLASH = 90,
    PERCENT = 91,
    STARSTAR = 92,
    PLUSPLUS = 105,
    DASHDASH = 106,

    // Operators - comparison
    LT = 93,
    GT = 48,
    LTEQ = 94,
    GTEQ = 99,
    EQEQ = 95,
    EQEQEQ = 96,
    BANGEQ = 97,
    BANGEQEQ = 98,

    // Operators - logical
    BANG = 21,
    AMPAMP = 80,
    PIPEPIPE = 81,
    QMARKQMARK = 100,

    // Operators - bitwise
    AMP = 85,
    PIPE = 87,
    CARET = 86,
    TILDE = 102,
    LTLT = 84,
    GTGT = 82,
    GTGTGT = 83,

    // Operators - assignment
    EQ = 7,
    PLUSEQ = 64,
    DASHEQ = 65,
    STAREQ = 66,
    SLASHEQ = 67,
    PERCENTEQ = 68,
    CARETEQ = 69,
    AMPEQ = 70,
    PIPEEQ = 71,
    GTGTEQ = 72,
    GTGTGTEQ = 73,
    LTLTEQ = 74,
    STARSTAREQ = 75,
    AMPAMPEQ = 76,
    PIPEPIPEEQ = 77,
    QMARKQMARKEQ = 78,

    // Operators - other
    EQGT = 60,
    QMARK = 131,
    QMARKDOT = 61,
    DASHGT = 56,

    // Punctuation
    LPAREN = 26,
    RPAREN = 28,
    LBRACE = 10,
    RBRACE = 12,
    LBRACK = 45,
    RBRACK = 46,
    SEMI = 27,
    COMMA = 11,
    DOT = 50,
    COLON = 40,
    COLONCOLON = 49,
    DOTDOTDOT = 79,
    AT = 126,

    // JSX specific
    HtmlCharacterReference = 47,
    LTSLASH = 51,
    SLASHGT = 52,
    JsxText = 169,

    // String literals
    DQUOTE = 53,
    SQUOTE = 54,
    StringFragment = 55,
    EscapeSequence = 109,
    BQUOTE = 111,
    DOLLARLBRACE = 112,

    // Regex
    SLASH2 = 113,
    RegexPattern = 114,
    RegexFlags = 115,

    // Numbers and literals
    Number = 116,
    True = 122,
    False = 123,
    Null = 124,
    Undefined = 125,

    // TypeScript specific
    Any = 138,
    Number2 = 139,
    Boolean = 140,
    String3 = 141,
    Symbol = 142,
    Object2 = 143,
    Satisfies = 146,
    Infer = 157,
    Is = 158,
    Keyof = 159,
    Uniquesymbol = 160,
    Unknown = 161,
    Never = 162,
    Asserts2 = 156,

    // AST Nodes - Program and structure
    Program = 172,
    ExportStatement = 173,
    ImportStatement = 180,
    Declaration = 178,

    // AST Nodes - Declarations
    ClassDeclaration = 235,
    Class = 234,
    FunctionDeclaration = 238,
    FunctionExpression = 237,
    GeneratorFunction = 239,
    GeneratorFunctionDeclaration = 240,
    ArrowFunction = 241,
    MethodDefinition = 275,
    InterfaceDeclaration = 301,
    EnumDeclaration = 303,
    TypeAliasDeclaration = 306,
    AmbientDeclaration = 294,
    AbstractClassDeclaration = 295,

    // AST Nodes - Statements
    Statement = 187,
    Block = 186,
    ExpressionStatement = 188,
    IfStatement = 194,
    SwitchStatement = 195,
    ForStatement = 196,
    ForInStatement = 197,
    WhileStatement = 199,
    DoStatement = 200,
    TryStatement = 201,
    ReturnStatement = 206,
    ThrowStatement = 207,
    BreakStatement = 203,
    ContinueStatement = 204,
    LabeledStatement = 209,

    // AST Nodes - Variable declarations
    VariableDeclaration = 189,
    LexicalDeclaration = 190,
    VariableDeclarator = 191,

    // AST Nodes - Expressions
    Expression = 216,
    PrimaryExpression = 217,
    ParenthesizedExpression = 215,
    YieldExpression = 218,
    Object = 219,
    Array = 223,
    CallExpression = 245,
    NewExpression = 246,
    AwaitExpression = 247,
    MemberExpression = 248,
    SubscriptExpression = 249,
    AssignmentExpression = 250,
    AugmentedAssignmentExpression = 252,
    TernaryExpression = 256,
    BinaryExpression = 257,
    UnaryExpression = 258,
    UpdateExpression = 259,
    SequenceExpression = 260,

    // AST Nodes - JSX
    JsxElement = 225,
    JsxExpression = 226,
    JsxOpeningElement = 227,
    JsxClosingElement = 230,
    JsxSelfClosingElement = 231,
    JsxAttribute = 232,

    // AST Nodes - TypeScript
    TypeAnnotation = 315,
    Type2 = 320,
    PrimaryType = 327,
    GenericType = 332,
    TypePredicate = 333,
    TypeArguments = 349,
    ObjectType = 350,
    TypeParameters = 353,
    TypeParameter = 354,
    ArrayType = 359,
    TupleType = 360,
    UnionType = 362,
    IntersectionType = 363,
    FunctionType = 364,

    // AST Nodes - Class structure
    ClassBody = 270,
    ClassHeritage = 236,
    FormalParameters = 271,

    // AST Nodes - Other
    String = 233,
    TemplateString = 262,
    Arguments = 266,
    Pair = 276,

    // Error handling
    Error = 400,
}

impl From<TsxToken> for &'static str {
    fn from(tok: TsxToken) -> Self {
        match tok {
            TsxToken::End => "end",
            TsxToken::Identifier => "identifier",
            TsxToken::HashBangLine => "hash_bang_line",

            // Keywords - exports and imports
            TsxToken::Export => "export",
            TsxToken::Default => "default",
            TsxToken::Type => "type",
            TsxToken::As => "as",
            TsxToken::Import2 => "import",
            TsxToken::From => "from",

            // Keywords - declarations
            TsxToken::Var => "var",
            TsxToken::Let => "let",
            TsxToken::Const => "const",
            TsxToken::Function => "function",
            TsxToken::Class2 => "class",
            TsxToken::Interface => "interface",
            TsxToken::Enum => "enum",
            TsxToken::Namespace => "namespace",
            TsxToken::Module2 => "module",

            // Keywords - types and modifiers
            TsxToken::Typeof => "typeof",
            TsxToken::Async => "async",
            TsxToken::Static => "static",
            TsxToken::Readonly => "readonly",
            TsxToken::Public => "public",
            TsxToken::Private => "private",
            TsxToken::Protected => "protected",
            TsxToken::Abstract => "abstract",
            TsxToken::Declare => "declare",
            TsxToken::Override => "override",

            // Keywords - control flow
            TsxToken::If => "if",
            TsxToken::Else => "else",
            TsxToken::Switch => "switch",
            TsxToken::Case => "case",
            TsxToken::Default2 => "default",
            TsxToken::For => "for",
            TsxToken::While => "while",
            TsxToken::Do => "do",
            TsxToken::Try => "try",
            TsxToken::Catch => "catch",
            TsxToken::Finally => "finally",
            TsxToken::Break => "break",
            TsxToken::Continue => "continue",
            TsxToken::Return => "return",
            TsxToken::Throw => "throw",
            TsxToken::Debugger => "debugger",
            TsxToken::Yield => "yield",
            TsxToken::Await => "await",

            // Keywords - other
            TsxToken::In => "in",
            TsxToken::Of => "of",
            TsxToken::With => "with",
            TsxToken::New => "new",
            TsxToken::This => "this",
            TsxToken::Super => "super",
            TsxToken::Void => "void",
            TsxToken::Delete => "delete",
            TsxToken::Instanceof => "instanceof",
            TsxToken::Extends => "extends",
            TsxToken::Implements => "implements",

            // Operators - arithmetic
            TsxToken::PLUS => "+",
            TsxToken::DASH => "-",
            TsxToken::STAR => "*",
            TsxToken::SLASH => "/",
            TsxToken::PERCENT => "%",
            TsxToken::STARSTAR => "**",
            TsxToken::PLUSPLUS => "++",
            TsxToken::DASHDASH => "--",

            // Operators - comparison
            TsxToken::LT => "<",
            TsxToken::GT => ">",
            TsxToken::LTEQ => "<=",
            TsxToken::GTEQ => ">=",
            TsxToken::EQEQ => "==",
            TsxToken::EQEQEQ => "===",
            TsxToken::BANGEQ => "!=",
            TsxToken::BANGEQEQ => "!==",

            // Operators - logical
            TsxToken::BANG => "!",
            TsxToken::AMPAMP => "&&",
            TsxToken::PIPEPIPE => "||",
            TsxToken::QMARKQMARK => "??",

            // Operators - bitwise
            TsxToken::AMP => "&",
            TsxToken::PIPE => "|",
            TsxToken::CARET => "^",
            TsxToken::TILDE => "~",
            TsxToken::LTLT => "<<",
            TsxToken::GTGT => ">>",
            TsxToken::GTGTGT => ">>>",

            // Operators - assignment
            TsxToken::EQ => "=",
            TsxToken::PLUSEQ => "+=",
            TsxToken::DASHEQ => "-=",
            TsxToken::STAREQ => "*=",
            TsxToken::SLASHEQ => "/=",
            TsxToken::PERCENTEQ => "%=",
            TsxToken::CARETEQ => "^=",
            TsxToken::AMPEQ => "&=",
            TsxToken::PIPEEQ => "|=",
            TsxToken::GTGTEQ => ">>=",
            TsxToken::GTGTGTEQ => ">>>=",
            TsxToken::LTLTEQ => "<<=",
            TsxToken::STARSTAREQ => "**=",
            TsxToken::AMPAMPEQ => "&&=",
            TsxToken::PIPEPIPEEQ => "||=",
            TsxToken::QMARKQMARKEQ => "??=",

            // Operators - other
            TsxToken::EQGT => "=>",
            TsxToken::QMARK => "?",
            TsxToken::QMARKDOT => "?.",
            TsxToken::DASHGT => "->",

            // Punctuation
            TsxToken::LPAREN => "(",
            TsxToken::RPAREN => ")",
            TsxToken::LBRACE => "{",
            TsxToken::RBRACE => "}",
            TsxToken::LBRACK => "[",
            TsxToken::RBRACK => "]",
            TsxToken::SEMI => ";",
            TsxToken::COMMA => ",",
            TsxToken::DOT => ".",
            TsxToken::COLON => ":",
            TsxToken::COLONCOLON => "::",
            TsxToken::DOTDOTDOT => "...",
            TsxToken::AT => "@",

            // JSX specific
            TsxToken::HtmlCharacterReference => "html_character_reference",
            TsxToken::LTSLASH => "</",
            TsxToken::SLASHGT => "/>",
            TsxToken::JsxText => "jsx_text",

            // String literals
            TsxToken::DQUOTE => "\"",
            TsxToken::SQUOTE => "'",
            TsxToken::StringFragment => "string_fragment",
            TsxToken::EscapeSequence => "escape_sequence",
            TsxToken::BQUOTE => "`",
            TsxToken::DOLLARLBRACE => "${",

            // Regex
            TsxToken::SLASH2 => "/",
            TsxToken::RegexPattern => "regex_pattern",
            TsxToken::RegexFlags => "regex_flags",

            // Numbers and literals
            TsxToken::Number => "number",
            TsxToken::True => "true",
            TsxToken::False => "false",
            TsxToken::Null => "null",
            TsxToken::Undefined => "undefined",

            // TypeScript specific
            TsxToken::Any => "any",
            TsxToken::Number2 => "number",
            TsxToken::Boolean => "boolean",
            TsxToken::String3 => "string",
            TsxToken::Symbol => "symbol",
            TsxToken::Object2 => "object",
            TsxToken::Satisfies => "satisfies",
            TsxToken::Infer => "infer",
            TsxToken::Is => "is",
            TsxToken::Keyof => "keyof",
            TsxToken::Uniquesymbol => "unique symbol",
            TsxToken::Unknown => "unknown",
            TsxToken::Never => "never",
            TsxToken::Asserts2 => "asserts",

            // AST Nodes - Program and structure
            TsxToken::Program => "program",
            TsxToken::ExportStatement => "export_statement",
            TsxToken::ImportStatement => "import_statement",
            TsxToken::Declaration => "declaration",

            // AST Nodes - Declarations
            TsxToken::ClassDeclaration => "class_declaration",
            TsxToken::Class => "class",
            TsxToken::FunctionDeclaration => "function_declaration",
            TsxToken::FunctionExpression => "function_expression",
            TsxToken::GeneratorFunction => "generator_function",
            TsxToken::GeneratorFunctionDeclaration => "generator_function_declaration",
            TsxToken::ArrowFunction => "arrow_function",
            TsxToken::MethodDefinition => "method_definition",
            TsxToken::InterfaceDeclaration => "interface_declaration",
            TsxToken::EnumDeclaration => "enum_declaration",
            TsxToken::TypeAliasDeclaration => "type_alias_declaration",
            TsxToken::AmbientDeclaration => "ambient_declaration",
            TsxToken::AbstractClassDeclaration => "abstract_class_declaration",

            // AST Nodes - Statements
            TsxToken::Statement => "statement",
            TsxToken::Block => "block",
            TsxToken::ExpressionStatement => "expression_statement",
            TsxToken::IfStatement => "if_statement",
            TsxToken::SwitchStatement => "switch_statement",
            TsxToken::ForStatement => "for_statement",
            TsxToken::ForInStatement => "for_in_statement",
            TsxToken::WhileStatement => "while_statement",
            TsxToken::DoStatement => "do_statement",
            TsxToken::TryStatement => "try_statement",
            TsxToken::ReturnStatement => "return_statement",
            TsxToken::ThrowStatement => "throw_statement",
            TsxToken::BreakStatement => "break_statement",
            TsxToken::ContinueStatement => "continue_statement",
            TsxToken::LabeledStatement => "labeled_statement",

            // AST Nodes - Variable declarations
            TsxToken::VariableDeclaration => "variable_declaration",
            TsxToken::LexicalDeclaration => "lexical_declaration",
            TsxToken::VariableDeclarator => "variable_declarator",

            // AST Nodes - Expressions
            TsxToken::Expression => "expression",
            TsxToken::PrimaryExpression => "primary_expression",
            TsxToken::ParenthesizedExpression => "parenthesized_expression",
            TsxToken::YieldExpression => "yield_expression",
            TsxToken::Object => "object",
            TsxToken::Array => "array",
            TsxToken::CallExpression => "call_expression",
            TsxToken::NewExpression => "new_expression",
            TsxToken::AwaitExpression => "await_expression",
            TsxToken::MemberExpression => "member_expression",
            TsxToken::SubscriptExpression => "subscript_expression",
            TsxToken::AssignmentExpression => "assignment_expression",
            TsxToken::AugmentedAssignmentExpression => "augmented_assignment_expression",
            TsxToken::TernaryExpression => "ternary_expression",
            TsxToken::BinaryExpression => "binary_expression",
            TsxToken::UnaryExpression => "unary_expression",
            TsxToken::UpdateExpression => "update_expression",
            TsxToken::SequenceExpression => "sequence_expression",

            // AST Nodes - JSX
            TsxToken::JsxElement => "jsx_element",
            TsxToken::JsxExpression => "jsx_expression",
            TsxToken::JsxOpeningElement => "jsx_opening_element",
            TsxToken::JsxClosingElement => "jsx_closing_element",
            TsxToken::JsxSelfClosingElement => "jsx_self_closing_element",
            TsxToken::JsxAttribute => "jsx_attribute",

            // AST Nodes - TypeScript
            TsxToken::TypeAnnotation => "type_annotation",
            TsxToken::Type2 => "type",
            TsxToken::PrimaryType => "primary_type",
            TsxToken::GenericType => "generic_type",
            TsxToken::TypePredicate => "type_predicate",
            TsxToken::TypeArguments => "type_arguments",
            TsxToken::ObjectType => "object_type",
            TsxToken::TypeParameters => "type_parameters",
            TsxToken::TypeParameter => "type_parameter",
            TsxToken::ArrayType => "array_type",
            TsxToken::TupleType => "tuple_type",
            TsxToken::UnionType => "union_type",
            TsxToken::IntersectionType => "intersection_type",
            TsxToken::FunctionType => "function_type",

            // AST Nodes - Class structure
            TsxToken::ClassBody => "class_body",
            TsxToken::ClassHeritage => "class_heritage",
            TsxToken::FormalParameters => "formal_parameters",

            // AST Nodes - Other
            TsxToken::String => "string",
            TsxToken::TemplateString => "template_string",
            TsxToken::Arguments => "arguments",
            TsxToken::Pair => "pair",

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
///
/// Provides language-specific information and tree-sitter grammar access for TSX.
pub struct TsxLanguage;

impl LanguageInfo for TsxLanguage {
    fn get_lang() -> Lang {
        Lang::Tsx
    }

    fn get_lang_name() -> &'static str {
        "tsx"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tsx_language_info() {
        assert_eq!(TsxLanguage::get_lang(), Lang::Tsx);
        assert_eq!(TsxLanguage::get_lang_name(), "tsx");
    }

    #[test]
    fn test_tsx_token_conversions() {
        let tok: TsxToken = 1.into();
        assert_eq!(tok, TsxToken::Identifier);

        let tok: TsxToken = 172.into();
        assert_eq!(tok, TsxToken::Program);

        let tok: TsxToken = 235.into();
        assert_eq!(tok, TsxToken::ClassDeclaration);

        let tok: TsxToken = 238.into();
        assert_eq!(tok, TsxToken::FunctionDeclaration);
    }

    #[test]
    fn test_tsx_token_to_string() {
        assert_eq!(<&str>::from(TsxToken::Function), "function");
        assert_eq!(<&str>::from(TsxToken::Class2), "class");
        assert_eq!(<&str>::from(TsxToken::Interface), "interface");
        assert_eq!(<&str>::from(TsxToken::JsxElement), "jsx_element");
    }
}
