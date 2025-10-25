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
}
