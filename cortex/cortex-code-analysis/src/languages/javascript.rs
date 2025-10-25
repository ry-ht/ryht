//! JavaScript language parser implementation.
//!
//! This module provides comprehensive JavaScript parsing support with advanced
//! token detection for all JavaScript language features including:
//! - Function declarations and expressions (including arrow functions)
//! - Class declarations and methods
//! - Generator and async functions
//! - Object literals and methods
//! - All JavaScript operators and expressions
//! - JSX support
//! - Modern JavaScript syntax (ES2015+)

use num_derive::FromPrimitive;
use crate::lang::Lang;
use crate::traits::LanguageInfo;

/// JavaScript language token types.
///
/// This enum represents all possible node types in the JavaScript tree-sitter grammar.
/// It provides complete coverage of JavaScript syntax from ES5 through modern ES2023+,
/// including JSX extensions.
#[derive(Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum JavaScriptToken {
    // Terminals and basic tokens
    End = 0,
    Identifier = 1,
    HashBangLine = 2,

    // Export/Import keywords
    Export = 3,
    STAR = 4,
    Default = 5,
    As = 6,
    Import2 = 10,
    From = 11,
    With = 12,

    // Variable declarations
    Var = 13,
    Let = 14,
    Const = 15,

    // Control flow keywords
    Else = 16,
    If = 17,
    Switch = 18,
    For = 19,
    While = 26,
    Do = 27,
    Try = 28,
    Break = 29,
    Continue = 30,
    Debugger = 31,
    Return = 32,
    Throw = 33,
    Case = 35,
    Catch = 36,
    Finally = 37,

    // Async/Await/Yield
    Await = 23,
    Yield = 38,

    // Operators and punctuation
    LBRACE = 7,
    COMMA = 8,
    RBRACE = 9,
    LPAREN = 20,
    SEMI = 21,
    RPAREN = 22,
    COLON = 34,
    EQ = 39,
    LBRACK = 40,
    RBRACK = 41,
    DOT = 46,
    EQGT = 57,          // =>
    OptionalChain = 58,  // ?.

    // Loop keywords
    In = 24,
    Of = 25,

    // Assignment operators
    PLUSEQ = 60,        // +=
    DASHEQ = 61,        // -=
    STAREQ = 62,        // *=
    SLASHEQ = 63,       // /=
    PERCENTEQ = 64,     // %=
    CARETEQ = 65,       // ^=
    AMPEQ = 66,         // &=
    PIPEEQ = 67,        // |=
    GTGTEQ = 68,        // >>=
    GTGTGTEQ = 69,      // >>>=
    LTLTEQ = 70,        // <<=
    STARSTAREQ = 71,    // **=
    AMPAMPEQ = 72,      // &&=
    PIPEPIPEEQ = 73,    // ||=
    QMARKQMARKEQ = 74,  // ??=

    // Other operators
    DOTDOTDOT = 75,     // ...
    AMPAMP = 76,        // &&
    PIPEPIPE = 77,      // ||
    GTGT = 78,          // >>
    GTGTGT = 79,        // >>>
    LTLT = 80,          // <<
    AMP = 81,           // &
    CARET = 82,         // ^
    PIPE = 83,          // |
    PLUS = 84,          // +
    DASH = 85,          // -
    SLASH = 86,         // /
    PERCENT = 87,       // %
    STARSTAR = 88,      // **
    LTEQ = 89,          // <=
    EQEQ = 90,          // ==
    EQEQEQ = 91,        // ===
    BANGEQ = 92,        // !=
    BANGEQEQ = 93,      // !==
    GTEQ = 94,          // >=
    QMARKQMARK = 95,    // ??
    BANG = 97,          // !
    TILDE = 98,         // ~
    PLUSPLUS = 102,     // ++
    DASHDASH = 103,     // --
    QMARK = 130,        // ?

    // Comparison and type operators
    LT = 43,            // <
    GT = 44,            // >
    Instanceof = 96,
    Typeof = 99,
    Void = 100,
    Delete = 101,

    // JSX-specific tokens
    HtmlCharacterReference = 42,
    Identifier2 = 45,
    LTSLASH = 47,       // </
    SLASHGT = 48,       // />
    JsxText = 132,

    // String literals
    DQUOTE = 49,
    SQUOTE = 50,
    StringFragment = 51,
    StringFragment2 = 52,
    StringFragment3 = 104,
    StringFragment4 = 105,
    StringFragment5 = 129,
    EscapeSequence = 106,

    // Template literals
    BQUOTE = 108,
    DOLLARLBRACE = 109,

    // Regular expressions
    SLASH2 = 110,
    RegexPattern = 111,
    RegexFlags = 112,

    // Keywords and literals
    Class2 = 53,
    Extends = 54,
    Async = 55,
    Function = 56,
    New = 59,

    // Numeric literals
    Number = 113,

    // Special identifiers
    PrivatePropertyIdentifier = 114,
    Target = 115,
    Meta = 116,
    This = 117,
    Super = 118,
    True = 119,
    False = 120,
    Null = 121,
    Undefined = 122,

    // Decorators and modifiers
    AT = 123,
    Static = 124,
    Staticget = 125,
    Get = 126,
    Set = 127,

    // Comments
    Comment = 107,
    HtmlComment = 131,

    // Special
    AutomaticSemicolon = 128,

    // --- Non-terminal nodes (AST node types) ---

    /// Program root node
    Program = 133,

    // Export/Import statements
    ExportStatement = 134,
    NamespaceExport = 135,
    ExportClause = 136,
    ExportSpecifier = 137,
    ModuleExportName = 138,
    Import = 140,
    ImportStatement = 141,
    ImportClause = 142,
    FromClause = 143,
    NamespaceImport = 144,
    NamedImports = 145,
    ImportSpecifier = 146,
    ImportAttribute = 147,

    // Declarations
    Declaration = 139,
    VariableDeclaration = 150,
    LexicalDeclaration = 151,
    VariableDeclarator = 152,

    // Statements
    Statement = 148,
    ExpressionStatement = 149,
    StatementBlock = 153,
    ElseClause = 154,
    IfStatement = 155,
    SwitchStatement = 156,
    ForStatement = 157,
    ForInStatement = 158,
    ForHeader = 159,
    WhileStatement = 160,
    DoStatement = 161,
    TryStatement = 162,
    WithStatement = 163,
    BreakStatement = 164,
    ContinueStatement = 165,
    DebuggerStatement = 166,
    ReturnStatement = 167,
    ThrowStatement = 168,
    EmptyStatement = 169,
    LabeledStatement = 170,

    // Switch statement parts
    SwitchBody = 171,
    SwitchCase = 172,
    SwitchDefault = 173,
    CatchClause = 174,
    FinallyClause = 175,

    // Expressions
    ParenthesizedExpression = 176,
    Expression = 177,
    PrimaryExpression = 178,
    YieldExpression = 179,

    // Objects and patterns
    Object = 180,
    ObjectPattern = 181,
    AssignmentPattern = 182,
    ObjectAssignmentPattern = 183,

    // Arrays and patterns
    Array = 184,
    ArrayPattern = 185,

    // JSX elements
    JsxElement = 186,
    JsxExpression = 187,
    JsxOpeningElement = 188,
    JsxNamespaceName = 190,
    JsxClosingElement = 191,
    JsxSelfClosingElement = 192,
    JsxAttribute = 193,

    // Member expressions
    MemberExpression = 189,
    MemberExpression2 = 206,
    MemberExpression3 = 226,

    // Strings
    String = 194,
    String2 = 219,

    // Classes
    Class = 195,
    ClassDeclaration = 196,
    ClassHeritage = 197,
    ClassBody = 228,
    FieldDefinition = 229,
    ClassStaticBlock = 231,

    // Functions
    FunctionExpression = 198,
    FunctionDeclaration = 199,
    GeneratorFunction = 200,
    GeneratorFunctionDeclaration = 201,
    ArrowFunction = 202,
    FormalParameters = 230,

    // Method definitions
    MethodDefinition = 234,

    // Call expressions
    CallExpression = 203,
    CallExpression2 = 227,
    NewExpression = 204,

    // Other expressions
    AwaitExpression = 205,
    SubscriptExpression = 207,
    AssignmentExpression = 208,
    AugmentedAssignmentLhs = 209,
    AugmentedAssignmentExpression = 210,
    Initializer = 211,
    DestructuringPattern = 212,
    SpreadElement = 213,
    TernaryExpression = 214,
    BinaryExpression = 215,
    UnaryExpression = 216,
    UpdateExpression = 217,
    SequenceExpression = 218,
    TemplateString = 220,
    TemplateSubstitution = 221,
    Regex = 222,
    MetaProperty = 223,
    Arguments = 224,
    Decorator = 225,

    // Patterns
    Pattern = 232,
    RestPattern = 233,
    Pair = 235,
    PairPattern = 236,
    PropertyName = 237,
    ComputedPropertyName = 238,

    // Repeat nodes (for sequences)
    ProgramRepeat1 = 239,
    ExportStatementRepeat1 = 240,
    ExportClauseRepeat1 = 241,
    NamedImportsRepeat1 = 242,
    VariableDeclarationRepeat1 = 243,
    SwitchBodyRepeat1 = 244,
    ObjectRepeat1 = 245,
    ObjectPatternRepeat1 = 246,
    ArrayRepeat1 = 247,
    ArrayPatternRepeat1 = 248,
    JsxElementRepeat1 = 249,
    JsxOpeningElementRepeat1 = 250,
    JsxStringRepeat1 = 251,
    JsxStringRepeat2 = 252,
    SequenceExpressionRepeat1 = 253,
    StringRepeat1 = 254,
    StringRepeat2 = 255,
    TemplateStringRepeat1 = 256,
    ClassBodyRepeat1 = 257,
    FormalParametersRepeat1 = 258,

    // Special identifiers
    PropertyIdentifier = 259,
    ShorthandPropertyIdentifier = 260,
    ShorthandPropertyIdentifierPattern = 261,
    StatementIdentifier = 262,

    /// Error node
    Error = 263,
}

impl From<JavaScriptToken> for &'static str {
    #[inline(always)]
    fn from(tok: JavaScriptToken) -> Self {
        match tok {
            JavaScriptToken::End => "end",
            JavaScriptToken::Identifier => "identifier",
            JavaScriptToken::HashBangLine => "hash_bang_line",
            JavaScriptToken::Export => "export",
            JavaScriptToken::STAR => "*",
            JavaScriptToken::Default => "default",
            JavaScriptToken::As => "as",
            JavaScriptToken::LBRACE => "{",
            JavaScriptToken::COMMA => ",",
            JavaScriptToken::RBRACE => "}",
            JavaScriptToken::Import2 => "import",
            JavaScriptToken::From => "from",
            JavaScriptToken::With => "with",
            JavaScriptToken::Var => "var",
            JavaScriptToken::Let => "let",
            JavaScriptToken::Const => "const",
            JavaScriptToken::Else => "else",
            JavaScriptToken::If => "if",
            JavaScriptToken::Switch => "switch",
            JavaScriptToken::For => "for",
            JavaScriptToken::LPAREN => "(",
            JavaScriptToken::SEMI => ";",
            JavaScriptToken::RPAREN => ")",
            JavaScriptToken::Await => "await",
            JavaScriptToken::In => "in",
            JavaScriptToken::Of => "of",
            JavaScriptToken::While => "while",
            JavaScriptToken::Do => "do",
            JavaScriptToken::Try => "try",
            JavaScriptToken::Break => "break",
            JavaScriptToken::Continue => "continue",
            JavaScriptToken::Debugger => "debugger",
            JavaScriptToken::Return => "return",
            JavaScriptToken::Throw => "throw",
            JavaScriptToken::COLON => ":",
            JavaScriptToken::Case => "case",
            JavaScriptToken::Catch => "catch",
            JavaScriptToken::Finally => "finally",
            JavaScriptToken::Yield => "yield",
            JavaScriptToken::EQ => "=",
            JavaScriptToken::LBRACK => "[",
            JavaScriptToken::RBRACK => "]",
            JavaScriptToken::HtmlCharacterReference => "html_character_reference",
            JavaScriptToken::LT => "<",
            JavaScriptToken::GT => ">",
            JavaScriptToken::Identifier2 => "identifier",
            JavaScriptToken::DOT => ".",
            JavaScriptToken::LTSLASH => "</",
            JavaScriptToken::SLASHGT => "/>",
            JavaScriptToken::DQUOTE => "\"",
            JavaScriptToken::SQUOTE => "'",
            JavaScriptToken::StringFragment => "string_fragment",
            JavaScriptToken::StringFragment2 => "string_fragment",
            JavaScriptToken::Class2 => "class",
            JavaScriptToken::Extends => "extends",
            JavaScriptToken::Async => "async",
            JavaScriptToken::Function => "function",
            JavaScriptToken::EQGT => "=>",
            JavaScriptToken::OptionalChain => "optional_chain",
            JavaScriptToken::New => "new",
            JavaScriptToken::PLUSEQ => "+=",
            JavaScriptToken::DASHEQ => "-=",
            JavaScriptToken::STAREQ => "*=",
            JavaScriptToken::SLASHEQ => "/=",
            JavaScriptToken::PERCENTEQ => "%=",
            JavaScriptToken::CARETEQ => "^=",
            JavaScriptToken::AMPEQ => "&=",
            JavaScriptToken::PIPEEQ => "|=",
            JavaScriptToken::GTGTEQ => ">>=",
            JavaScriptToken::GTGTGTEQ => ">>>=",
            JavaScriptToken::LTLTEQ => "<<=",
            JavaScriptToken::STARSTAREQ => "**=",
            JavaScriptToken::AMPAMPEQ => "&&=",
            JavaScriptToken::PIPEPIPEEQ => "||=",
            JavaScriptToken::QMARKQMARKEQ => "??=",
            JavaScriptToken::DOTDOTDOT => "...",
            JavaScriptToken::AMPAMP => "&&",
            JavaScriptToken::PIPEPIPE => "||",
            JavaScriptToken::GTGT => ">>",
            JavaScriptToken::GTGTGT => ">>>",
            JavaScriptToken::LTLT => "<<",
            JavaScriptToken::AMP => "&",
            JavaScriptToken::CARET => "^",
            JavaScriptToken::PIPE => "|",
            JavaScriptToken::PLUS => "+",
            JavaScriptToken::DASH => "-",
            JavaScriptToken::SLASH => "/",
            JavaScriptToken::PERCENT => "%",
            JavaScriptToken::STARSTAR => "**",
            JavaScriptToken::LTEQ => "<=",
            JavaScriptToken::EQEQ => "==",
            JavaScriptToken::EQEQEQ => "===",
            JavaScriptToken::BANGEQ => "!=",
            JavaScriptToken::BANGEQEQ => "!==",
            JavaScriptToken::GTEQ => ">=",
            JavaScriptToken::QMARKQMARK => "??",
            JavaScriptToken::Instanceof => "instanceof",
            JavaScriptToken::BANG => "!",
            JavaScriptToken::TILDE => "~",
            JavaScriptToken::Typeof => "typeof",
            JavaScriptToken::Void => "void",
            JavaScriptToken::Delete => "delete",
            JavaScriptToken::PLUSPLUS => "++",
            JavaScriptToken::DASHDASH => "--",
            JavaScriptToken::StringFragment3 => "string_fragment",
            JavaScriptToken::StringFragment4 => "string_fragment",
            JavaScriptToken::EscapeSequence => "escape_sequence",
            JavaScriptToken::Comment => "comment",
            JavaScriptToken::BQUOTE => "`",
            JavaScriptToken::DOLLARLBRACE => "${",
            JavaScriptToken::SLASH2 => "/",
            JavaScriptToken::RegexPattern => "regex_pattern",
            JavaScriptToken::RegexFlags => "regex_flags",
            JavaScriptToken::Number => "number",
            JavaScriptToken::PrivatePropertyIdentifier => "private_property_identifier",
            JavaScriptToken::Target => "target",
            JavaScriptToken::Meta => "meta",
            JavaScriptToken::This => "this",
            JavaScriptToken::Super => "super",
            JavaScriptToken::True => "true",
            JavaScriptToken::False => "false",
            JavaScriptToken::Null => "null",
            JavaScriptToken::Undefined => "undefined",
            JavaScriptToken::AT => "@",
            JavaScriptToken::Static => "static",
            JavaScriptToken::Staticget => "static get",
            JavaScriptToken::Get => "get",
            JavaScriptToken::Set => "set",
            JavaScriptToken::AutomaticSemicolon => "_automatic_semicolon",
            JavaScriptToken::StringFragment5 => "string_fragment",
            JavaScriptToken::QMARK => "?",
            JavaScriptToken::HtmlComment => "html_comment",
            JavaScriptToken::JsxText => "jsx_text",
            JavaScriptToken::Program => "program",
            JavaScriptToken::ExportStatement => "export_statement",
            JavaScriptToken::NamespaceExport => "namespace_export",
            JavaScriptToken::ExportClause => "export_clause",
            JavaScriptToken::ExportSpecifier => "export_specifier",
            JavaScriptToken::ModuleExportName => "_module_export_name",
            JavaScriptToken::Declaration => "declaration",
            JavaScriptToken::Import => "import",
            JavaScriptToken::ImportStatement => "import_statement",
            JavaScriptToken::ImportClause => "import_clause",
            JavaScriptToken::FromClause => "_from_clause",
            JavaScriptToken::NamespaceImport => "namespace_import",
            JavaScriptToken::NamedImports => "named_imports",
            JavaScriptToken::ImportSpecifier => "import_specifier",
            JavaScriptToken::ImportAttribute => "import_attribute",
            JavaScriptToken::Statement => "statement",
            JavaScriptToken::ExpressionStatement => "expression_statement",
            JavaScriptToken::VariableDeclaration => "variable_declaration",
            JavaScriptToken::LexicalDeclaration => "lexical_declaration",
            JavaScriptToken::VariableDeclarator => "variable_declarator",
            JavaScriptToken::StatementBlock => "statement_block",
            JavaScriptToken::ElseClause => "else_clause",
            JavaScriptToken::IfStatement => "if_statement",
            JavaScriptToken::SwitchStatement => "switch_statement",
            JavaScriptToken::ForStatement => "for_statement",
            JavaScriptToken::ForInStatement => "for_in_statement",
            JavaScriptToken::ForHeader => "_for_header",
            JavaScriptToken::WhileStatement => "while_statement",
            JavaScriptToken::DoStatement => "do_statement",
            JavaScriptToken::TryStatement => "try_statement",
            JavaScriptToken::WithStatement => "with_statement",
            JavaScriptToken::BreakStatement => "break_statement",
            JavaScriptToken::ContinueStatement => "continue_statement",
            JavaScriptToken::DebuggerStatement => "debugger_statement",
            JavaScriptToken::ReturnStatement => "return_statement",
            JavaScriptToken::ThrowStatement => "throw_statement",
            JavaScriptToken::EmptyStatement => "empty_statement",
            JavaScriptToken::LabeledStatement => "labeled_statement",
            JavaScriptToken::SwitchBody => "switch_body",
            JavaScriptToken::SwitchCase => "switch_case",
            JavaScriptToken::SwitchDefault => "switch_default",
            JavaScriptToken::CatchClause => "catch_clause",
            JavaScriptToken::FinallyClause => "finally_clause",
            JavaScriptToken::ParenthesizedExpression => "parenthesized_expression",
            JavaScriptToken::Expression => "expression",
            JavaScriptToken::PrimaryExpression => "primary_expression",
            JavaScriptToken::YieldExpression => "yield_expression",
            JavaScriptToken::Object => "object",
            JavaScriptToken::ObjectPattern => "object_pattern",
            JavaScriptToken::AssignmentPattern => "assignment_pattern",
            JavaScriptToken::ObjectAssignmentPattern => "object_assignment_pattern",
            JavaScriptToken::Array => "array",
            JavaScriptToken::ArrayPattern => "array_pattern",
            JavaScriptToken::JsxElement => "jsx_element",
            JavaScriptToken::JsxExpression => "jsx_expression",
            JavaScriptToken::JsxOpeningElement => "jsx_opening_element",
            JavaScriptToken::MemberExpression => "member_expression",
            JavaScriptToken::JsxNamespaceName => "jsx_namespace_name",
            JavaScriptToken::JsxClosingElement => "jsx_closing_element",
            JavaScriptToken::JsxSelfClosingElement => "jsx_self_closing_element",
            JavaScriptToken::JsxAttribute => "jsx_attribute",
            JavaScriptToken::String => "string",
            JavaScriptToken::Class => "class",
            JavaScriptToken::ClassDeclaration => "class_declaration",
            JavaScriptToken::ClassHeritage => "class_heritage",
            JavaScriptToken::FunctionExpression => "function_expression",
            JavaScriptToken::FunctionDeclaration => "function_declaration",
            JavaScriptToken::GeneratorFunction => "generator_function",
            JavaScriptToken::GeneratorFunctionDeclaration => "generator_function_declaration",
            JavaScriptToken::ArrowFunction => "arrow_function",
            JavaScriptToken::CallExpression => "call_expression",
            JavaScriptToken::NewExpression => "new_expression",
            JavaScriptToken::AwaitExpression => "await_expression",
            JavaScriptToken::MemberExpression2 => "member_expression",
            JavaScriptToken::SubscriptExpression => "subscript_expression",
            JavaScriptToken::AssignmentExpression => "assignment_expression",
            JavaScriptToken::AugmentedAssignmentLhs => "_augmented_assignment_lhs",
            JavaScriptToken::AugmentedAssignmentExpression => "augmented_assignment_expression",
            JavaScriptToken::Initializer => "_initializer",
            JavaScriptToken::DestructuringPattern => "_destructuring_pattern",
            JavaScriptToken::SpreadElement => "spread_element",
            JavaScriptToken::TernaryExpression => "ternary_expression",
            JavaScriptToken::BinaryExpression => "binary_expression",
            JavaScriptToken::UnaryExpression => "unary_expression",
            JavaScriptToken::UpdateExpression => "update_expression",
            JavaScriptToken::SequenceExpression => "sequence_expression",
            JavaScriptToken::String2 => "string",
            JavaScriptToken::TemplateString => "template_string",
            JavaScriptToken::TemplateSubstitution => "template_substitution",
            JavaScriptToken::Regex => "regex",
            JavaScriptToken::MetaProperty => "meta_property",
            JavaScriptToken::Arguments => "arguments",
            JavaScriptToken::Decorator => "decorator",
            JavaScriptToken::MemberExpression3 => "member_expression",
            JavaScriptToken::CallExpression2 => "call_expression",
            JavaScriptToken::ClassBody => "class_body",
            JavaScriptToken::FieldDefinition => "field_definition",
            JavaScriptToken::FormalParameters => "formal_parameters",
            JavaScriptToken::ClassStaticBlock => "class_static_block",
            JavaScriptToken::Pattern => "pattern",
            JavaScriptToken::RestPattern => "rest_pattern",
            JavaScriptToken::MethodDefinition => "method_definition",
            JavaScriptToken::Pair => "pair",
            JavaScriptToken::PairPattern => "pair_pattern",
            JavaScriptToken::PropertyName => "_property_name",
            JavaScriptToken::ComputedPropertyName => "computed_property_name",
            JavaScriptToken::ProgramRepeat1 => "program_repeat1",
            JavaScriptToken::ExportStatementRepeat1 => "export_statement_repeat1",
            JavaScriptToken::ExportClauseRepeat1 => "export_clause_repeat1",
            JavaScriptToken::NamedImportsRepeat1 => "named_imports_repeat1",
            JavaScriptToken::VariableDeclarationRepeat1 => "variable_declaration_repeat1",
            JavaScriptToken::SwitchBodyRepeat1 => "switch_body_repeat1",
            JavaScriptToken::ObjectRepeat1 => "object_repeat1",
            JavaScriptToken::ObjectPatternRepeat1 => "object_pattern_repeat1",
            JavaScriptToken::ArrayRepeat1 => "array_repeat1",
            JavaScriptToken::ArrayPatternRepeat1 => "array_pattern_repeat1",
            JavaScriptToken::JsxElementRepeat1 => "jsx_element_repeat1",
            JavaScriptToken::JsxOpeningElementRepeat1 => "jsx_opening_element_repeat1",
            JavaScriptToken::JsxStringRepeat1 => "_jsx_string_repeat1",
            JavaScriptToken::JsxStringRepeat2 => "_jsx_string_repeat2",
            JavaScriptToken::SequenceExpressionRepeat1 => "sequence_expression_repeat1",
            JavaScriptToken::StringRepeat1 => "string_repeat1",
            JavaScriptToken::StringRepeat2 => "string_repeat2",
            JavaScriptToken::TemplateStringRepeat1 => "template_string_repeat1",
            JavaScriptToken::ClassBodyRepeat1 => "class_body_repeat1",
            JavaScriptToken::FormalParametersRepeat1 => "formal_parameters_repeat1",
            JavaScriptToken::PropertyIdentifier => "property_identifier",
            JavaScriptToken::ShorthandPropertyIdentifier => "shorthand_property_identifier",
            JavaScriptToken::ShorthandPropertyIdentifierPattern => {
                "shorthand_property_identifier_pattern"
            }
            JavaScriptToken::StatementIdentifier => "statement_identifier",
            JavaScriptToken::Error => "ERROR",
        }
    }
}

impl From<u16> for JavaScriptToken {
    #[inline(always)]
    fn from(x: u16) -> Self {
        num::FromPrimitive::from_u16(x).unwrap_or(Self::Error)
    }
}

// JavaScriptToken == u16
impl PartialEq<u16> for JavaScriptToken {
    #[inline(always)]
    fn eq(&self, x: &u16) -> bool {
        *self == Into::<Self>::into(*x)
    }
}

// u16 == JavaScriptToken
impl PartialEq<JavaScriptToken> for u16 {
    #[inline(always)]
    fn eq(&self, x: &JavaScriptToken) -> bool {
        *x == *self
    }
}

/// JavaScript language implementation.
///
/// Provides metadata and configuration for JavaScript parsing with full ES2015+
/// and JSX support.
pub struct JavaScriptLanguage;

impl LanguageInfo for JavaScriptLanguage {
    fn get_lang() -> Lang {
        Lang::JavaScript
    }

    fn get_lang_name() -> &'static str {
        "javascript"
    }
}

/// JSX (JavaScript with JSX) language implementation.
///
/// Provides metadata and configuration for JavaScript files that include JSX syntax.
pub struct JsxLanguage;

impl LanguageInfo for JsxLanguage {
    fn get_lang() -> Lang {
        Lang::Jsx
    }

    fn get_lang_name() -> &'static str {
        "jsx"
    }
}

// Helper methods for token detection
impl JavaScriptToken {
    /// Check if this token represents a function-like construct.
    ///
    /// Includes regular functions, arrow functions, generators, and async functions.
    #[inline]
    pub fn is_function(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::FunctionDeclaration
                | JavaScriptToken::FunctionExpression
                | JavaScriptToken::ArrowFunction
                | JavaScriptToken::GeneratorFunction
                | JavaScriptToken::GeneratorFunctionDeclaration
                | JavaScriptToken::MethodDefinition
        )
    }

    /// Check if this token represents an arrow function.
    #[inline]
    pub fn is_arrow_function(&self) -> bool {
        matches!(self, JavaScriptToken::ArrowFunction)
    }

    /// Check if this token represents a class-related construct.
    #[inline]
    pub fn is_class(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::Class | JavaScriptToken::ClassDeclaration
        )
    }

    /// Check if this token represents an async construct.
    #[inline]
    pub fn is_async(&self) -> bool {
        matches!(self, JavaScriptToken::Async | JavaScriptToken::AwaitExpression)
    }

    /// Check if this token represents a generator.
    #[inline]
    pub fn is_generator(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::GeneratorFunction | JavaScriptToken::GeneratorFunctionDeclaration
        )
    }

    /// Check if this token represents a binary operator.
    #[inline]
    pub fn is_binary_operator(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::PLUS
                | JavaScriptToken::DASH
                | JavaScriptToken::SLASH
                | JavaScriptToken::PERCENT
                | JavaScriptToken::STARSTAR
                | JavaScriptToken::AMP
                | JavaScriptToken::PIPE
                | JavaScriptToken::CARET
                | JavaScriptToken::AMPAMP
                | JavaScriptToken::PIPEPIPE
                | JavaScriptToken::LTLT
                | JavaScriptToken::GTGT
                | JavaScriptToken::GTGTGT
                | JavaScriptToken::EQEQ
                | JavaScriptToken::EQEQEQ
                | JavaScriptToken::BANGEQ
                | JavaScriptToken::BANGEQEQ
                | JavaScriptToken::LT
                | JavaScriptToken::LTEQ
                | JavaScriptToken::GT
                | JavaScriptToken::GTEQ
                | JavaScriptToken::QMARKQMARK
                | JavaScriptToken::Instanceof
        )
    }

    /// Check if this token represents a unary operator.
    #[inline]
    pub fn is_unary_operator(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::BANG
                | JavaScriptToken::TILDE
                | JavaScriptToken::Typeof
                | JavaScriptToken::Void
                | JavaScriptToken::Delete
                | JavaScriptToken::PLUS
                | JavaScriptToken::DASH
        )
    }

    /// Check if this token represents an assignment operator.
    #[inline]
    pub fn is_assignment_operator(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::EQ
                | JavaScriptToken::PLUSEQ
                | JavaScriptToken::DASHEQ
                | JavaScriptToken::STAREQ
                | JavaScriptToken::SLASHEQ
                | JavaScriptToken::PERCENTEQ
                | JavaScriptToken::CARETEQ
                | JavaScriptToken::AMPEQ
                | JavaScriptToken::PIPEEQ
                | JavaScriptToken::GTGTEQ
                | JavaScriptToken::GTGTGTEQ
                | JavaScriptToken::LTLTEQ
                | JavaScriptToken::STARSTAREQ
                | JavaScriptToken::AMPAMPEQ
                | JavaScriptToken::PIPEPIPEEQ
                | JavaScriptToken::QMARKQMARKEQ
        )
    }

    /// Check if this token represents a literal value.
    #[inline]
    pub fn is_literal(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::Number
                | JavaScriptToken::String
                | JavaScriptToken::String2
                | JavaScriptToken::True
                | JavaScriptToken::False
                | JavaScriptToken::Null
                | JavaScriptToken::Undefined
                | JavaScriptToken::TemplateString
                | JavaScriptToken::Regex
        )
    }

    /// Check if this token represents a JSX-related construct.
    #[inline]
    pub fn is_jsx(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::JsxElement
                | JavaScriptToken::JsxExpression
                | JavaScriptToken::JsxOpeningElement
                | JavaScriptToken::JsxClosingElement
                | JavaScriptToken::JsxSelfClosingElement
                | JavaScriptToken::JsxAttribute
                | JavaScriptToken::JsxText
        )
    }

    /// Check if this token represents a loop statement.
    #[inline]
    pub fn is_loop(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::ForStatement
                | JavaScriptToken::ForInStatement
                | JavaScriptToken::WhileStatement
                | JavaScriptToken::DoStatement
        )
    }

    /// Check if this token represents an import/export statement.
    #[inline]
    pub fn is_module(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::ImportStatement
                | JavaScriptToken::ExportStatement
                | JavaScriptToken::Import
                | JavaScriptToken::Export
        )
    }

    // ===== ES6+ Features =====

    /// Check if this token represents an ES6 class construct.
    #[inline]
    pub fn is_class_declaration(&self) -> bool {
        matches!(self, JavaScriptToken::ClassDeclaration)
    }

    /// Check if this token represents a class body or class-related construct.
    #[inline]
    pub fn is_class_body(&self) -> bool {
        matches!(self, JavaScriptToken::ClassBody)
    }

    /// Check if this token represents a class member (method or field).
    #[inline]
    pub fn is_class_member(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::MethodDefinition | JavaScriptToken::FieldDefinition
        )
    }

    /// Check if this token represents a class heritage (extends clause).
    #[inline]
    pub fn is_class_heritage(&self) -> bool {
        matches!(self, JavaScriptToken::ClassHeritage)
    }

    /// Check if this token represents a destructuring pattern.
    #[inline]
    pub fn is_destructuring(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::DestructuringPattern
                | JavaScriptToken::ObjectPattern
                | JavaScriptToken::ArrayPattern
        )
    }

    /// Check if this token represents a spread or rest element.
    #[inline]
    pub fn is_spread_or_rest(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::SpreadElement | JavaScriptToken::RestPattern
        )
    }

    /// Check if this token represents a template literal or template string.
    #[inline]
    pub fn is_template_literal(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::TemplateString | JavaScriptToken::TemplateSubstitution
        )
    }

    /// Check if this token represents a template string token (backtick or substitution).
    #[inline]
    pub fn is_template_token(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::BQUOTE | JavaScriptToken::DOLLARLBRACE
        )
    }

    // ===== Async Features =====

    /// Check if this token represents an await expression.
    #[inline]
    pub fn is_await(&self) -> bool {
        matches!(self, JavaScriptToken::AwaitExpression | JavaScriptToken::Await)
    }

    /// Check if this token represents a yield expression.
    #[inline]
    pub fn is_yield(&self) -> bool {
        matches!(self, JavaScriptToken::YieldExpression | JavaScriptToken::Yield)
    }

    /// Check if this token represents a Promise-related construct.
    /// Note: This checks for async/await which typically work with Promises.
    #[inline]
    pub fn is_promise_related(&self) -> bool {
        self.is_async() || self.is_await()
    }

    // ===== Module System =====

    /// Check if this token represents an import statement or clause.
    #[inline]
    pub fn is_import(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::ImportStatement
                | JavaScriptToken::Import
                | JavaScriptToken::Import2
                | JavaScriptToken::ImportClause
                | JavaScriptToken::ImportSpecifier
                | JavaScriptToken::NamespaceImport
                | JavaScriptToken::NamedImports
        )
    }

    /// Check if this token represents an export statement or clause.
    #[inline]
    pub fn is_export(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::ExportStatement
                | JavaScriptToken::Export
                | JavaScriptToken::ExportClause
                | JavaScriptToken::ExportSpecifier
                | JavaScriptToken::NamespaceExport
        )
    }

    /// Check if this token represents a default export/import.
    #[inline]
    pub fn is_default(&self) -> bool {
        matches!(self, JavaScriptToken::Default)
    }

    /// Check if this token represents a from clause in import/export.
    #[inline]
    pub fn is_from_clause(&self) -> bool {
        matches!(self, JavaScriptToken::FromClause | JavaScriptToken::From)
    }

    /// Check if this token represents a namespace import/export.
    #[inline]
    pub fn is_namespace(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::NamespaceImport | JavaScriptToken::NamespaceExport
        )
    }

    // ===== Advanced Operators =====

    /// Check if this token represents optional chaining (?.operator).
    #[inline]
    pub fn is_optional_chain(&self) -> bool {
        matches!(self, JavaScriptToken::OptionalChain)
    }

    /// Check if this token represents nullish coalescing (??operator).
    #[inline]
    pub fn is_nullish_coalescing(&self) -> bool {
        matches!(self, JavaScriptToken::QMARKQMARK)
    }

    /// Check if this token represents a private field or property.
    #[inline]
    pub fn is_private_field(&self) -> bool {
        matches!(self, JavaScriptToken::PrivatePropertyIdentifier)
    }

    /// Check if this token represents the nullish coalescing assignment (??=).
    #[inline]
    pub fn is_nullish_coalescing_assignment(&self) -> bool {
        matches!(self, JavaScriptToken::QMARKQMARKEQ)
    }

    /// Check if this token represents logical assignment (&&= or ||=).
    #[inline]
    pub fn is_logical_assignment(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::AMPAMPEQ | JavaScriptToken::PIPEPIPEEQ
        )
    }

    // ===== Control Flow =====

    /// Check if this token represents a try-catch-finally statement.
    #[inline]
    pub fn is_try_catch_finally(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::TryStatement
                | JavaScriptToken::Try
                | JavaScriptToken::CatchClause
                | JavaScriptToken::Catch
                | JavaScriptToken::FinallyClause
                | JavaScriptToken::Finally
        )
    }

    /// Check if this token represents a switch statement.
    #[inline]
    pub fn is_switch(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::SwitchStatement
                | JavaScriptToken::Switch
                | JavaScriptToken::SwitchBody
                | JavaScriptToken::SwitchCase
                | JavaScriptToken::SwitchDefault
        )
    }

    /// Check if this token represents a for-in statement.
    #[inline]
    pub fn is_for_in(&self) -> bool {
        matches!(self, JavaScriptToken::ForInStatement | JavaScriptToken::In)
    }

    /// Check if this token represents a for-of statement.
    #[inline]
    pub fn is_for_of(&self) -> bool {
        matches!(self, JavaScriptToken::Of)
    }

    /// Check if this token represents an if-else statement.
    #[inline]
    pub fn is_if_else(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::IfStatement | JavaScriptToken::If | JavaScriptToken::ElseClause | JavaScriptToken::Else
        )
    }

    /// Check if this token represents a while loop.
    #[inline]
    pub fn is_while(&self) -> bool {
        matches!(self, JavaScriptToken::WhileStatement | JavaScriptToken::While)
    }

    /// Check if this token represents a do-while loop.
    #[inline]
    pub fn is_do_while(&self) -> bool {
        matches!(self, JavaScriptToken::DoStatement | JavaScriptToken::Do)
    }

    /// Check if this token represents break or continue.
    #[inline]
    pub fn is_break_or_continue(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::BreakStatement
                | JavaScriptToken::Break
                | JavaScriptToken::ContinueStatement
                | JavaScriptToken::Continue
        )
    }

    /// Check if this token represents return or throw.
    #[inline]
    pub fn is_return_or_throw(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::ReturnStatement
                | JavaScriptToken::Return
                | JavaScriptToken::ThrowStatement
                | JavaScriptToken::Throw
        )
    }

    // ===== Object Features =====

    /// Check if this token represents a computed property name.
    #[inline]
    pub fn is_computed_property(&self) -> bool {
        matches!(self, JavaScriptToken::ComputedPropertyName)
    }

    /// Check if this token represents a shorthand property.
    #[inline]
    pub fn is_shorthand_property(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::ShorthandPropertyIdentifier
                | JavaScriptToken::ShorthandPropertyIdentifierPattern
        )
    }

    /// Check if this token represents a getter or setter.
    #[inline]
    pub fn is_accessor(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::Get | JavaScriptToken::Set | JavaScriptToken::Staticget
        )
    }

    /// Check if this token represents a pair (property: value).
    #[inline]
    pub fn is_pair(&self) -> bool {
        matches!(self, JavaScriptToken::Pair | JavaScriptToken::PairPattern)
    }

    /// Check if this token represents an object literal or pattern.
    #[inline]
    pub fn is_object(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::Object
                | JavaScriptToken::ObjectPattern
                | JavaScriptToken::ObjectAssignmentPattern
        )
    }

    // ===== Pattern Matching =====

    /// Check if this token represents an array pattern.
    #[inline]
    pub fn is_array_pattern(&self) -> bool {
        matches!(self, JavaScriptToken::ArrayPattern)
    }

    /// Check if this token represents an assignment pattern.
    #[inline]
    pub fn is_assignment_pattern(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::AssignmentPattern | JavaScriptToken::ObjectAssignmentPattern
        )
    }

    /// Check if this token represents a rest pattern.
    #[inline]
    pub fn is_rest_pattern(&self) -> bool {
        matches!(self, JavaScriptToken::RestPattern)
    }

    // ===== JSX Support =====

    /// Check if this token represents a JSX element.
    #[inline]
    pub fn is_jsx_element(&self) -> bool {
        matches!(self, JavaScriptToken::JsxElement)
    }

    /// Check if this token represents a JSX opening element.
    #[inline]
    pub fn is_jsx_opening(&self) -> bool {
        matches!(self, JavaScriptToken::JsxOpeningElement)
    }

    /// Check if this token represents a JSX closing element.
    #[inline]
    pub fn is_jsx_closing(&self) -> bool {
        matches!(self, JavaScriptToken::JsxClosingElement)
    }

    /// Check if this token represents a JSX self-closing element.
    #[inline]
    pub fn is_jsx_self_closing(&self) -> bool {
        matches!(self, JavaScriptToken::JsxSelfClosingElement)
    }

    /// Check if this token represents a JSX attribute.
    #[inline]
    pub fn is_jsx_attribute(&self) -> bool {
        matches!(self, JavaScriptToken::JsxAttribute)
    }

    /// Check if this token represents a JSX expression.
    #[inline]
    pub fn is_jsx_expression(&self) -> bool {
        matches!(self, JavaScriptToken::JsxExpression)
    }

    /// Check if this token represents JSX text content.
    #[inline]
    pub fn is_jsx_text(&self) -> bool {
        matches!(self, JavaScriptToken::JsxText)
    }

    // ===== Special Constructs =====

    /// Check if this token represents a with statement.
    #[inline]
    pub fn is_with_statement(&self) -> bool {
        matches!(self, JavaScriptToken::WithStatement | JavaScriptToken::With)
    }

    /// Check if this token represents a debugger statement.
    #[inline]
    pub fn is_debugger(&self) -> bool {
        matches!(self, JavaScriptToken::DebuggerStatement | JavaScriptToken::Debugger)
    }

    /// Check if this token represents a labeled statement.
    #[inline]
    pub fn is_labeled(&self) -> bool {
        matches!(self, JavaScriptToken::LabeledStatement)
    }

    // ===== Expression Types =====

    /// Check if this token represents a member expression.
    #[inline]
    pub fn is_member_expression(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::MemberExpression
                | JavaScriptToken::MemberExpression2
                | JavaScriptToken::MemberExpression3
        )
    }

    /// Check if this token represents a subscript expression.
    #[inline]
    pub fn is_subscript_expression(&self) -> bool {
        matches!(self, JavaScriptToken::SubscriptExpression)
    }

    /// Check if this token represents a call expression.
    #[inline]
    pub fn is_call_expression(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::CallExpression | JavaScriptToken::CallExpression2
        )
    }

    /// Check if this token represents a new expression.
    #[inline]
    pub fn is_new_expression(&self) -> bool {
        matches!(self, JavaScriptToken::NewExpression | JavaScriptToken::New)
    }

    /// Check if this token represents a ternary expression.
    #[inline]
    pub fn is_ternary_expression(&self) -> bool {
        matches!(self, JavaScriptToken::TernaryExpression)
    }

    /// Check if this token represents a binary expression.
    #[inline]
    pub fn is_binary_expression(&self) -> bool {
        matches!(self, JavaScriptToken::BinaryExpression)
    }

    /// Check if this token represents a unary expression.
    #[inline]
    pub fn is_unary_expression(&self) -> bool {
        matches!(self, JavaScriptToken::UnaryExpression)
    }

    /// Check if this token represents an update expression (++ or --).
    #[inline]
    pub fn is_update_expression(&self) -> bool {
        matches!(self, JavaScriptToken::UpdateExpression)
    }

    /// Check if this token represents a sequence expression.
    #[inline]
    pub fn is_sequence_expression(&self) -> bool {
        matches!(self, JavaScriptToken::SequenceExpression)
    }

    /// Check if this token represents a parenthesized expression.
    #[inline]
    pub fn is_parenthesized_expression(&self) -> bool {
        matches!(self, JavaScriptToken::ParenthesizedExpression)
    }

    /// Check if this token represents an assignment expression.
    #[inline]
    pub fn is_assignment_expression(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::AssignmentExpression | JavaScriptToken::AugmentedAssignmentExpression
        )
    }

    // ===== Additional Operators =====

    /// Check if this token represents an increment operator.
    #[inline]
    pub fn is_increment(&self) -> bool {
        matches!(self, JavaScriptToken::PLUSPLUS)
    }

    /// Check if this token represents a decrement operator.
    #[inline]
    pub fn is_decrement(&self) -> bool {
        matches!(self, JavaScriptToken::DASHDASH)
    }

    /// Check if this token represents a comparison operator.
    #[inline]
    pub fn is_comparison_operator(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::EQEQ
                | JavaScriptToken::EQEQEQ
                | JavaScriptToken::BANGEQ
                | JavaScriptToken::BANGEQEQ
                | JavaScriptToken::LT
                | JavaScriptToken::LTEQ
                | JavaScriptToken::GT
                | JavaScriptToken::GTEQ
        )
    }

    /// Check if this token represents a logical operator.
    #[inline]
    pub fn is_logical_operator(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::AMPAMP | JavaScriptToken::PIPEPIPE | JavaScriptToken::BANG
        )
    }

    /// Check if this token represents a bitwise operator.
    #[inline]
    pub fn is_bitwise_operator(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::AMP
                | JavaScriptToken::PIPE
                | JavaScriptToken::CARET
                | JavaScriptToken::TILDE
                | JavaScriptToken::LTLT
                | JavaScriptToken::GTGT
                | JavaScriptToken::GTGTGT
        )
    }

    /// Check if this token represents an arithmetic operator.
    #[inline]
    pub fn is_arithmetic_operator(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::PLUS
                | JavaScriptToken::DASH
                | JavaScriptToken::STAR
                | JavaScriptToken::SLASH
                | JavaScriptToken::PERCENT
                | JavaScriptToken::STARSTAR
        )
    }

    /// Check if this token represents the spread operator.
    #[inline]
    pub fn is_spread_operator(&self) -> bool {
        matches!(self, JavaScriptToken::DOTDOTDOT)
    }

    // ===== Identifiers and Keywords =====

    /// Check if this token represents an identifier.
    #[inline]
    pub fn is_identifier(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::Identifier
                | JavaScriptToken::Identifier2
                | JavaScriptToken::PropertyIdentifier
                | JavaScriptToken::StatementIdentifier
        )
    }

    /// Check if this token represents a property identifier.
    #[inline]
    pub fn is_property_identifier(&self) -> bool {
        matches!(self, JavaScriptToken::PropertyIdentifier)
    }

    /// Check if this token represents a statement identifier.
    #[inline]
    pub fn is_statement_identifier(&self) -> bool {
        matches!(self, JavaScriptToken::StatementIdentifier)
    }

    /// Check if this token represents 'this' or 'super'.
    #[inline]
    pub fn is_this_or_super(&self) -> bool {
        matches!(self, JavaScriptToken::This | JavaScriptToken::Super)
    }

    /// Check if this token represents a meta property (new.target, import.meta).
    #[inline]
    pub fn is_meta_property(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::MetaProperty | JavaScriptToken::Target | JavaScriptToken::Meta
        )
    }

    // ===== Variable Declarations =====

    /// Check if this token represents a variable declaration.
    #[inline]
    pub fn is_variable_declaration(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::VariableDeclaration
                | JavaScriptToken::LexicalDeclaration
                | JavaScriptToken::VariableDeclarator
        )
    }

    /// Check if this token represents a var keyword.
    #[inline]
    pub fn is_var(&self) -> bool {
        matches!(self, JavaScriptToken::Var)
    }

    /// Check if this token represents a let keyword.
    #[inline]
    pub fn is_let(&self) -> bool {
        matches!(self, JavaScriptToken::Let)
    }

    /// Check if this token represents a const keyword.
    #[inline]
    pub fn is_const(&self) -> bool {
        matches!(self, JavaScriptToken::Const)
    }

    // ===== Regular Expressions =====

    /// Check if this token represents a regular expression.
    #[inline]
    pub fn is_regex(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::Regex | JavaScriptToken::RegexPattern | JavaScriptToken::RegexFlags
        )
    }

    // ===== Strings and Templates =====

    /// Check if this token represents a string literal.
    #[inline]
    pub fn is_string(&self) -> bool {
        matches!(self, JavaScriptToken::String | JavaScriptToken::String2)
    }

    /// Check if this token represents a string fragment.
    #[inline]
    pub fn is_string_fragment(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::StringFragment
                | JavaScriptToken::StringFragment2
                | JavaScriptToken::StringFragment3
                | JavaScriptToken::StringFragment4
                | JavaScriptToken::StringFragment5
        )
    }

    /// Check if this token represents an escape sequence.
    #[inline]
    pub fn is_escape_sequence(&self) -> bool {
        matches!(self, JavaScriptToken::EscapeSequence)
    }

    // ===== Statements =====

    /// Check if this token represents a statement.
    #[inline]
    pub fn is_statement(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::Statement
                | JavaScriptToken::ExpressionStatement
                | JavaScriptToken::IfStatement
                | JavaScriptToken::SwitchStatement
                | JavaScriptToken::ForStatement
                | JavaScriptToken::ForInStatement
                | JavaScriptToken::WhileStatement
                | JavaScriptToken::DoStatement
                | JavaScriptToken::TryStatement
                | JavaScriptToken::WithStatement
                | JavaScriptToken::BreakStatement
                | JavaScriptToken::ContinueStatement
                | JavaScriptToken::DebuggerStatement
                | JavaScriptToken::ReturnStatement
                | JavaScriptToken::ThrowStatement
                | JavaScriptToken::EmptyStatement
                | JavaScriptToken::LabeledStatement
        )
    }

    /// Check if this token represents a statement block.
    #[inline]
    pub fn is_block(&self) -> bool {
        matches!(self, JavaScriptToken::StatementBlock)
    }

    /// Check if this token represents an expression statement.
    #[inline]
    pub fn is_expression_statement(&self) -> bool {
        matches!(self, JavaScriptToken::ExpressionStatement)
    }

    /// Check if this token represents an empty statement.
    #[inline]
    pub fn is_empty_statement(&self) -> bool {
        matches!(self, JavaScriptToken::EmptyStatement)
    }

    // ===== Arrays =====

    /// Check if this token represents an array literal.
    #[inline]
    pub fn is_array(&self) -> bool {
        matches!(self, JavaScriptToken::Array)
    }

    // ===== Decorators =====

    /// Check if this token represents a decorator.
    #[inline]
    pub fn is_decorator(&self) -> bool {
        matches!(self, JavaScriptToken::Decorator | JavaScriptToken::AT)
    }

    // ===== Static Members =====

    /// Check if this token represents a static keyword.
    #[inline]
    pub fn is_static(&self) -> bool {
        matches!(self, JavaScriptToken::Static | JavaScriptToken::Staticget)
    }

    /// Check if this token represents a class static block.
    #[inline]
    pub fn is_static_block(&self) -> bool {
        matches!(self, JavaScriptToken::ClassStaticBlock)
    }

    // ===== Parameters =====

    /// Check if this token represents formal parameters.
    #[inline]
    pub fn is_formal_parameters(&self) -> bool {
        matches!(self, JavaScriptToken::FormalParameters)
    }

    /// Check if this token represents function arguments.
    #[inline]
    pub fn is_arguments(&self) -> bool {
        matches!(self, JavaScriptToken::Arguments)
    }

    // ===== Comments =====

    /// Check if this token represents a comment.
    #[inline]
    pub fn is_comment(&self) -> bool {
        matches!(self, JavaScriptToken::Comment | JavaScriptToken::HtmlComment)
    }

    // ===== Special Values =====

    /// Check if this token represents a boolean literal.
    #[inline]
    pub fn is_boolean(&self) -> bool {
        matches!(self, JavaScriptToken::True | JavaScriptToken::False)
    }

    /// Check if this token represents null.
    #[inline]
    pub fn is_null(&self) -> bool {
        matches!(self, JavaScriptToken::Null)
    }

    /// Check if this token represents undefined.
    #[inline]
    pub fn is_undefined(&self) -> bool {
        matches!(self, JavaScriptToken::Undefined)
    }

    /// Check if this token represents a number literal.
    #[inline]
    pub fn is_number(&self) -> bool {
        matches!(self, JavaScriptToken::Number)
    }

    // ===== Type Operators =====

    /// Check if this token represents the typeof operator.
    #[inline]
    pub fn is_typeof(&self) -> bool {
        matches!(self, JavaScriptToken::Typeof)
    }

    /// Check if this token represents the instanceof operator.
    #[inline]
    pub fn is_instanceof(&self) -> bool {
        matches!(self, JavaScriptToken::Instanceof)
    }

    /// Check if this token represents the void operator.
    #[inline]
    pub fn is_void(&self) -> bool {
        matches!(self, JavaScriptToken::Void)
    }

    /// Check if this token represents the delete operator.
    #[inline]
    pub fn is_delete(&self) -> bool {
        matches!(self, JavaScriptToken::Delete)
    }

    // ===== Declarations =====

    /// Check if this token represents a declaration.
    #[inline]
    pub fn is_declaration(&self) -> bool {
        matches!(
            self,
            JavaScriptToken::Declaration
                | JavaScriptToken::FunctionDeclaration
                | JavaScriptToken::GeneratorFunctionDeclaration
                | JavaScriptToken::ClassDeclaration
                | JavaScriptToken::VariableDeclaration
                | JavaScriptToken::LexicalDeclaration
        )
    }

    // ===== Patterns =====

    /// Check if this token represents a pattern.
    #[inline]
    pub fn is_pattern(&self) -> bool {
        matches!(self, JavaScriptToken::Pattern)
    }

    // ===== Program =====

    /// Check if this token represents the program root.
    #[inline]
    pub fn is_program(&self) -> bool {
        matches!(self, JavaScriptToken::Program)
    }

    // ===== Error Handling =====

    /// Check if this token represents an error node.
    #[inline]
    pub fn is_error(&self) -> bool {
        matches!(self, JavaScriptToken::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_conversion() {
        assert_eq!(JavaScriptToken::from(199), JavaScriptToken::FunctionDeclaration);
        assert_eq!(JavaScriptToken::from(202), JavaScriptToken::ArrowFunction);
        assert_eq!(JavaScriptToken::from(196), JavaScriptToken::ClassDeclaration);
    }

    #[test]
    fn test_token_detection() {
        assert!(JavaScriptToken::FunctionDeclaration.is_function());
        assert!(JavaScriptToken::ArrowFunction.is_function());
        assert!(JavaScriptToken::ArrowFunction.is_arrow_function());
        assert!(JavaScriptToken::ClassDeclaration.is_class());
        assert!(JavaScriptToken::GeneratorFunction.is_generator());
    }

    #[test]
    fn test_operator_detection() {
        assert!(JavaScriptToken::PLUS.is_binary_operator());
        assert!(JavaScriptToken::BANG.is_unary_operator());
        assert!(JavaScriptToken::PLUSEQ.is_assignment_operator());
    }

    #[test]
    fn test_literal_detection() {
        assert!(JavaScriptToken::Number.is_literal());
        assert!(JavaScriptToken::String.is_literal());
        assert!(JavaScriptToken::True.is_literal());
        assert!(JavaScriptToken::Null.is_literal());
    }

    #[test]
    fn test_language_info() {
        assert_eq!(JavaScriptLanguage::get_lang(), Lang::JavaScript);
        assert_eq!(JavaScriptLanguage::get_lang_name(), "javascript");
        assert_eq!(JsxLanguage::get_lang(), Lang::Jsx);
        assert_eq!(JsxLanguage::get_lang_name(), "jsx");
    }

    // ===== ES6+ Features Tests =====

    #[test]
    fn test_class_features() {
        assert!(JavaScriptToken::ClassDeclaration.is_class_declaration());
        assert!(JavaScriptToken::ClassBody.is_class_body());
        assert!(JavaScriptToken::MethodDefinition.is_class_member());
        assert!(JavaScriptToken::FieldDefinition.is_class_member());
        assert!(JavaScriptToken::ClassHeritage.is_class_heritage());
    }

    #[test]
    fn test_destructuring() {
        assert!(JavaScriptToken::DestructuringPattern.is_destructuring());
        assert!(JavaScriptToken::ObjectPattern.is_destructuring());
        assert!(JavaScriptToken::ArrayPattern.is_destructuring());
        assert!(JavaScriptToken::ArrayPattern.is_array_pattern());
    }

    #[test]
    fn test_spread_rest() {
        assert!(JavaScriptToken::SpreadElement.is_spread_or_rest());
        assert!(JavaScriptToken::RestPattern.is_spread_or_rest());
        assert!(JavaScriptToken::RestPattern.is_rest_pattern());
        assert!(JavaScriptToken::DOTDOTDOT.is_spread_operator());
    }

    #[test]
    fn test_template_literals() {
        assert!(JavaScriptToken::TemplateString.is_template_literal());
        assert!(JavaScriptToken::TemplateSubstitution.is_template_literal());
        assert!(JavaScriptToken::BQUOTE.is_template_token());
        assert!(JavaScriptToken::DOLLARLBRACE.is_template_token());
    }

    // ===== Async Features Tests =====

    #[test]
    fn test_async_features() {
        assert!(JavaScriptToken::Async.is_async());
        assert!(JavaScriptToken::AwaitExpression.is_await());
        assert!(JavaScriptToken::Await.is_await());
        assert!(JavaScriptToken::AwaitExpression.is_promise_related());
    }

    #[test]
    fn test_yield() {
        assert!(JavaScriptToken::YieldExpression.is_yield());
        assert!(JavaScriptToken::Yield.is_yield());
    }

    // ===== Module System Tests =====

    #[test]
    fn test_import() {
        assert!(JavaScriptToken::ImportStatement.is_import());
        assert!(JavaScriptToken::Import.is_import());
        assert!(JavaScriptToken::ImportClause.is_import());
        assert!(JavaScriptToken::ImportSpecifier.is_import());
        assert!(JavaScriptToken::NamespaceImport.is_import());
    }

    #[test]
    fn test_export() {
        assert!(JavaScriptToken::ExportStatement.is_export());
        assert!(JavaScriptToken::Export.is_export());
        assert!(JavaScriptToken::ExportClause.is_export());
        assert!(JavaScriptToken::ExportSpecifier.is_export());
        assert!(JavaScriptToken::NamespaceExport.is_export());
    }

    #[test]
    fn test_module_keywords() {
        assert!(JavaScriptToken::Default.is_default());
        assert!(JavaScriptToken::FromClause.is_from_clause());
        assert!(JavaScriptToken::From.is_from_clause());
        assert!(JavaScriptToken::NamespaceImport.is_namespace());
    }

    // ===== Advanced Operators Tests =====

    #[test]
    fn test_optional_chaining() {
        assert!(JavaScriptToken::OptionalChain.is_optional_chain());
    }

    #[test]
    fn test_nullish_coalescing() {
        assert!(JavaScriptToken::QMARKQMARK.is_nullish_coalescing());
        assert!(JavaScriptToken::QMARKQMARKEQ.is_nullish_coalescing_assignment());
    }

    #[test]
    fn test_private_fields() {
        assert!(JavaScriptToken::PrivatePropertyIdentifier.is_private_field());
    }

    #[test]
    fn test_logical_assignment() {
        assert!(JavaScriptToken::AMPAMPEQ.is_logical_assignment());
        assert!(JavaScriptToken::PIPEPIPEEQ.is_logical_assignment());
    }

    // ===== Control Flow Tests =====

    #[test]
    fn test_try_catch_finally() {
        assert!(JavaScriptToken::TryStatement.is_try_catch_finally());
        assert!(JavaScriptToken::CatchClause.is_try_catch_finally());
        assert!(JavaScriptToken::FinallyClause.is_try_catch_finally());
    }

    #[test]
    fn test_switch() {
        assert!(JavaScriptToken::SwitchStatement.is_switch());
        assert!(JavaScriptToken::SwitchBody.is_switch());
        assert!(JavaScriptToken::SwitchCase.is_switch());
        assert!(JavaScriptToken::SwitchDefault.is_switch());
    }

    #[test]
    fn test_loops() {
        assert!(JavaScriptToken::ForInStatement.is_for_in());
        assert!(JavaScriptToken::In.is_for_in());
        assert!(JavaScriptToken::Of.is_for_of());
        assert!(JavaScriptToken::WhileStatement.is_while());
        assert!(JavaScriptToken::DoStatement.is_do_while());
    }

    #[test]
    fn test_if_else() {
        assert!(JavaScriptToken::IfStatement.is_if_else());
        assert!(JavaScriptToken::ElseClause.is_if_else());
    }

    #[test]
    fn test_break_continue() {
        assert!(JavaScriptToken::BreakStatement.is_break_or_continue());
        assert!(JavaScriptToken::ContinueStatement.is_break_or_continue());
    }

    #[test]
    fn test_return_throw() {
        assert!(JavaScriptToken::ReturnStatement.is_return_or_throw());
        assert!(JavaScriptToken::ThrowStatement.is_return_or_throw());
    }

    // ===== Object Features Tests =====

    #[test]
    fn test_computed_property() {
        assert!(JavaScriptToken::ComputedPropertyName.is_computed_property());
    }

    #[test]
    fn test_shorthand_property() {
        assert!(JavaScriptToken::ShorthandPropertyIdentifier.is_shorthand_property());
        assert!(JavaScriptToken::ShorthandPropertyIdentifierPattern.is_shorthand_property());
    }

    #[test]
    fn test_accessor() {
        assert!(JavaScriptToken::Get.is_accessor());
        assert!(JavaScriptToken::Set.is_accessor());
        assert!(JavaScriptToken::Staticget.is_accessor());
    }

    #[test]
    fn test_object() {
        assert!(JavaScriptToken::Object.is_object());
        assert!(JavaScriptToken::ObjectPattern.is_object());
        assert!(JavaScriptToken::Pair.is_pair());
    }

    // ===== Pattern Matching Tests =====

    #[test]
    fn test_assignment_pattern() {
        assert!(JavaScriptToken::AssignmentPattern.is_assignment_pattern());
        assert!(JavaScriptToken::ObjectAssignmentPattern.is_assignment_pattern());
    }

    // ===== JSX Tests =====

    #[test]
    fn test_jsx_elements() {
        assert!(JavaScriptToken::JsxElement.is_jsx());
        assert!(JavaScriptToken::JsxElement.is_jsx_element());
        assert!(JavaScriptToken::JsxOpeningElement.is_jsx_opening());
        assert!(JavaScriptToken::JsxClosingElement.is_jsx_closing());
        assert!(JavaScriptToken::JsxSelfClosingElement.is_jsx_self_closing());
    }

    #[test]
    fn test_jsx_attributes() {
        assert!(JavaScriptToken::JsxAttribute.is_jsx_attribute());
        assert!(JavaScriptToken::JsxExpression.is_jsx_expression());
        assert!(JavaScriptToken::JsxText.is_jsx_text());
    }

    // ===== Special Constructs Tests =====

    #[test]
    fn test_with_statement() {
        assert!(JavaScriptToken::WithStatement.is_with_statement());
    }

    #[test]
    fn test_debugger() {
        assert!(JavaScriptToken::DebuggerStatement.is_debugger());
    }

    #[test]
    fn test_labeled() {
        assert!(JavaScriptToken::LabeledStatement.is_labeled());
    }

    // ===== Expression Types Tests =====

    #[test]
    fn test_member_expression() {
        assert!(JavaScriptToken::MemberExpression.is_member_expression());
        assert!(JavaScriptToken::MemberExpression2.is_member_expression());
        assert!(JavaScriptToken::MemberExpression3.is_member_expression());
    }

    #[test]
    fn test_subscript_expression() {
        assert!(JavaScriptToken::SubscriptExpression.is_subscript_expression());
    }

    #[test]
    fn test_call_expression() {
        assert!(JavaScriptToken::CallExpression.is_call_expression());
        assert!(JavaScriptToken::CallExpression2.is_call_expression());
    }

    #[test]
    fn test_new_expression() {
        assert!(JavaScriptToken::NewExpression.is_new_expression());
        assert!(JavaScriptToken::New.is_new_expression());
    }

    #[test]
    fn test_expressions() {
        assert!(JavaScriptToken::TernaryExpression.is_ternary_expression());
        assert!(JavaScriptToken::BinaryExpression.is_binary_expression());
        assert!(JavaScriptToken::UnaryExpression.is_unary_expression());
        assert!(JavaScriptToken::UpdateExpression.is_update_expression());
        assert!(JavaScriptToken::SequenceExpression.is_sequence_expression());
        assert!(JavaScriptToken::ParenthesizedExpression.is_parenthesized_expression());
    }

    #[test]
    fn test_assignment_expression() {
        assert!(JavaScriptToken::AssignmentExpression.is_assignment_expression());
        assert!(JavaScriptToken::AugmentedAssignmentExpression.is_assignment_expression());
    }

    // ===== Additional Operators Tests =====

    #[test]
    fn test_increment_decrement() {
        assert!(JavaScriptToken::PLUSPLUS.is_increment());
        assert!(JavaScriptToken::DASHDASH.is_decrement());
    }

    #[test]
    fn test_comparison_operators() {
        assert!(JavaScriptToken::EQEQ.is_comparison_operator());
        assert!(JavaScriptToken::EQEQEQ.is_comparison_operator());
        assert!(JavaScriptToken::BANGEQ.is_comparison_operator());
        assert!(JavaScriptToken::BANGEQEQ.is_comparison_operator());
        assert!(JavaScriptToken::LT.is_comparison_operator());
        assert!(JavaScriptToken::GT.is_comparison_operator());
    }

    #[test]
    fn test_logical_operators() {
        assert!(JavaScriptToken::AMPAMP.is_logical_operator());
        assert!(JavaScriptToken::PIPEPIPE.is_logical_operator());
        assert!(JavaScriptToken::BANG.is_logical_operator());
    }

    #[test]
    fn test_bitwise_operators() {
        assert!(JavaScriptToken::AMP.is_bitwise_operator());
        assert!(JavaScriptToken::PIPE.is_bitwise_operator());
        assert!(JavaScriptToken::CARET.is_bitwise_operator());
        assert!(JavaScriptToken::TILDE.is_bitwise_operator());
        assert!(JavaScriptToken::LTLT.is_bitwise_operator());
        assert!(JavaScriptToken::GTGT.is_bitwise_operator());
        assert!(JavaScriptToken::GTGTGT.is_bitwise_operator());
    }

    #[test]
    fn test_arithmetic_operators() {
        assert!(JavaScriptToken::PLUS.is_arithmetic_operator());
        assert!(JavaScriptToken::DASH.is_arithmetic_operator());
        assert!(JavaScriptToken::SLASH.is_arithmetic_operator());
        assert!(JavaScriptToken::PERCENT.is_arithmetic_operator());
        assert!(JavaScriptToken::STARSTAR.is_arithmetic_operator());
    }

    // ===== Identifiers and Keywords Tests =====

    #[test]
    fn test_identifiers() {
        assert!(JavaScriptToken::Identifier.is_identifier());
        assert!(JavaScriptToken::PropertyIdentifier.is_identifier());
        assert!(JavaScriptToken::PropertyIdentifier.is_property_identifier());
        assert!(JavaScriptToken::StatementIdentifier.is_statement_identifier());
    }

    #[test]
    fn test_this_super() {
        assert!(JavaScriptToken::This.is_this_or_super());
        assert!(JavaScriptToken::Super.is_this_or_super());
    }

    #[test]
    fn test_meta_property() {
        assert!(JavaScriptToken::MetaProperty.is_meta_property());
        assert!(JavaScriptToken::Target.is_meta_property());
        assert!(JavaScriptToken::Meta.is_meta_property());
    }

    // ===== Variable Declarations Tests =====

    #[test]
    fn test_variable_declarations() {
        assert!(JavaScriptToken::VariableDeclaration.is_variable_declaration());
        assert!(JavaScriptToken::LexicalDeclaration.is_variable_declaration());
        assert!(JavaScriptToken::Var.is_var());
        assert!(JavaScriptToken::Let.is_let());
        assert!(JavaScriptToken::Const.is_const());
    }

    // ===== Regular Expressions Tests =====

    #[test]
    fn test_regex() {
        assert!(JavaScriptToken::Regex.is_regex());
        assert!(JavaScriptToken::RegexPattern.is_regex());
        assert!(JavaScriptToken::RegexFlags.is_regex());
    }

    // ===== Strings and Templates Tests =====

    #[test]
    fn test_strings() {
        assert!(JavaScriptToken::String.is_string());
        assert!(JavaScriptToken::String2.is_string());
        assert!(JavaScriptToken::StringFragment.is_string_fragment());
        assert!(JavaScriptToken::EscapeSequence.is_escape_sequence());
    }

    // ===== Statements Tests =====

    #[test]
    fn test_statements() {
        assert!(JavaScriptToken::Statement.is_statement());
        assert!(JavaScriptToken::ExpressionStatement.is_statement());
        assert!(JavaScriptToken::IfStatement.is_statement());
        assert!(JavaScriptToken::StatementBlock.is_block());
        assert!(JavaScriptToken::EmptyStatement.is_empty_statement());
    }

    // ===== Arrays Tests =====

    #[test]
    fn test_array() {
        assert!(JavaScriptToken::Array.is_array());
    }

    // ===== Decorators Tests =====

    #[test]
    fn test_decorator() {
        assert!(JavaScriptToken::Decorator.is_decorator());
        assert!(JavaScriptToken::AT.is_decorator());
    }

    // ===== Static Members Tests =====

    #[test]
    fn test_static() {
        assert!(JavaScriptToken::Static.is_static());
        assert!(JavaScriptToken::ClassStaticBlock.is_static_block());
    }

    // ===== Parameters Tests =====

    #[test]
    fn test_parameters() {
        assert!(JavaScriptToken::FormalParameters.is_formal_parameters());
        assert!(JavaScriptToken::Arguments.is_arguments());
    }

    // ===== Comments Tests =====

    #[test]
    fn test_comments() {
        assert!(JavaScriptToken::Comment.is_comment());
        assert!(JavaScriptToken::HtmlComment.is_comment());
    }

    // ===== Special Values Tests =====

    #[test]
    fn test_boolean() {
        assert!(JavaScriptToken::True.is_boolean());
        assert!(JavaScriptToken::False.is_boolean());
    }

    #[test]
    fn test_null_undefined() {
        assert!(JavaScriptToken::Null.is_null());
        assert!(JavaScriptToken::Undefined.is_undefined());
    }

    #[test]
    fn test_number() {
        assert!(JavaScriptToken::Number.is_number());
    }

    // ===== Type Operators Tests =====

    #[test]
    fn test_type_operators() {
        assert!(JavaScriptToken::Typeof.is_typeof());
        assert!(JavaScriptToken::Instanceof.is_instanceof());
        assert!(JavaScriptToken::Void.is_void());
        assert!(JavaScriptToken::Delete.is_delete());
    }

    // ===== Declarations Tests =====

    #[test]
    fn test_declarations() {
        assert!(JavaScriptToken::Declaration.is_declaration());
        assert!(JavaScriptToken::FunctionDeclaration.is_declaration());
        assert!(JavaScriptToken::ClassDeclaration.is_declaration());
    }

    // ===== Patterns Tests =====

    #[test]
    fn test_pattern() {
        assert!(JavaScriptToken::Pattern.is_pattern());
    }

    // ===== Program Tests =====

    #[test]
    fn test_program() {
        assert!(JavaScriptToken::Program.is_program());
    }

    // ===== Error Handling Tests =====

    #[test]
    fn test_error() {
        assert!(JavaScriptToken::Error.is_error());
    }

    // ===== Cross-feature Tests =====

    #[test]
    fn test_loop_detection() {
        assert!(JavaScriptToken::ForStatement.is_loop());
        assert!(JavaScriptToken::ForInStatement.is_loop());
        assert!(JavaScriptToken::WhileStatement.is_loop());
        assert!(JavaScriptToken::DoStatement.is_loop());
    }

    #[test]
    fn test_module_detection() {
        assert!(JavaScriptToken::ImportStatement.is_module());
        assert!(JavaScriptToken::ExportStatement.is_module());
    }
}
