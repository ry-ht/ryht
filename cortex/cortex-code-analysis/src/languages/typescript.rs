//! TypeScript language parser implementation.
//!
//! This module provides comprehensive TypeScript parsing support with advanced
//! type system features including:
//! - Complete type system support (interfaces, types, generics, unions, intersections)
//! - All TypeScript-specific keywords (namespace, module, enum, implements, etc.)
//! - Decorators (experimental TypeScript feature)
//! - Type predicates and assertions
//! - Conditional types and mapped types
//! - Template literal types
//! - All TypeScript operators including type operators
//! - JSX/TSX support
//! - Ambient declarations
//! - Declaration merging patterns

use num_derive::FromPrimitive;
use crate::lang::Lang;
use crate::traits::LanguageInfo;

/// TypeScript language token types.
///
/// This enum represents all possible node types in the TypeScript tree-sitter grammar.
/// It provides complete coverage of TypeScript syntax including all type system features,
/// decorators, and JSX/TSX support.
#[derive(Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum TypeScriptToken {
    // ===== Terminals and basic tokens =====
    End = 0,
    Identifier = 1,
    HashBangLine = 2,

    // ===== Export/Import keywords =====
    Export = 3,
    STAR = 4,
    Default = 5,
    Type = 6,
    EQ = 7,
    As = 8,
    Namespace = 9,
    LBRACE = 10,
    COMMA = 11,
    RBRACE = 12,
    Typeof = 13,
    Import2 = 14,
    From = 15,
    With = 16,
    Assert = 17,

    // ===== Variable declarations =====
    Var = 18,
    Let = 19,
    Const = 20,
    BANG = 21,

    // ===== Control flow keywords =====
    Else = 22,
    If = 23,
    Switch = 24,
    For = 25,
    LPAREN = 26,
    SEMI = 27,
    RPAREN = 28,
    Await = 29,
    In = 30,
    Of = 31,
    While = 32,
    Do = 33,
    Try = 34,
    Break = 35,
    Continue = 36,
    Debugger = 37,
    Return = 38,
    Throw = 39,
    COLON = 40,
    Case = 41,
    Catch = 42,
    Finally = 43,
    Yield = 44,

    // ===== Brackets and operators =====
    LBRACK = 45,
    RBRACK = 46,
    DOT = 47,
    Class2 = 48,
    Async = 49,
    Function = 50,
    EQGT = 51,           // =>
    QMARKDOT = 52,       // ?.
    New = 53,
    Using = 54,

    // ===== Assignment operators =====
    PLUSEQ = 55,         // +=
    DASHEQ = 56,         // -=
    STAREQ = 57,         // *=
    SLASHEQ = 58,        // /=
    PERCENTEQ = 59,      // %=
    CARETEQ = 60,        // ^=
    AMPEQ = 61,          // &=
    PIPEEQ = 62,         // |=
    GTGTEQ = 63,         // >>=
    GTGTGTEQ = 64,       // >>>=
    LTLTEQ = 65,         // <<=
    STARSTAREQ = 66,     // **=
    AMPAMPEQ = 67,       // &&=
    PIPEPIPEEQ = 68,     // ||=
    QMARKQMARKEQ = 69,   // ??=

    // ===== Other operators =====
    DOTDOTDOT = 70,      // ...
    AMPAMP = 71,         // &&
    PIPEPIPE = 72,       // ||
    GTGT = 73,           // >>
    GTGTGT = 74,         // >>>
    LTLT = 75,           // <<
    AMP = 76,            // &
    CARET = 77,          // ^
    PIPE = 78,           // |
    PLUS = 79,           // +
    DASH = 80,           // -
    SLASH = 81,          // /
    PERCENT = 82,        // %
    STARSTAR = 83,       // **
    LT = 84,             // <
    LTEQ = 85,           // <=
    EQEQ = 86,           // ==
    EQEQEQ = 87,         // ===
    BANGEQ = 88,         // !=
    BANGEQEQ = 89,       // !==
    GTEQ = 90,           // >=
    GT = 91,             // >
    QMARKQMARK = 92,     // ??
    Instanceof = 93,
    TILDE = 94,          // ~
    Void = 95,
    Delete = 96,
    PLUSPLUS = 97,       // ++
    DASHDASH = 98,       // --

    // ===== String literals =====
    DQUOTE = 99,
    SQUOTE = 100,
    StringFragment = 101,
    StringFragment2 = 102,
    EscapeSequence = 103,
    Comment = 104,

    // ===== Template literals =====
    BQUOTE = 105,
    DOLLARLBRACE = 106,

    // ===== Regular expressions =====
    SLASH2 = 107,
    RegexPattern = 108,
    RegexFlags = 109,

    // ===== Literals and keywords =====
    Number = 110,
    PrivatePropertyIdentifier = 111,
    Target = 112,
    Meta = 113,
    This = 114,
    Super = 115,
    True = 116,
    False = 117,
    Null = 118,
    Undefined = 119,

    // ===== TypeScript-specific modifiers and keywords =====
    AT = 120,
    Static = 121,
    Readonly = 122,
    Get = 123,
    Set = 124,
    QMARK = 125,
    Declare = 126,
    Public = 127,
    Private = 128,
    Protected = 129,
    Override = 130,
    Module2 = 131,

    // ===== TypeScript predefined types =====
    Any = 132,
    Number2 = 133,
    Boolean = 134,
    String2 = 135,
    Symbol = 136,
    Object2 = 137,

    // ===== TypeScript advanced keywords =====
    Abstract = 138,
    Accessor = 139,
    Satisfies = 140,
    Require = 141,
    Extends = 142,
    Implements = 143,
    Global = 144,
    Interface = 145,
    Enum = 146,

    // ===== TypeScript type operators =====
    DASHQMARKCOLON = 147,    // -?:
    PLUSQMARKCOLON = 148,    // +?:
    QMARKCOLON = 149,        // ?:
    Asserts2 = 150,
    Infer = 151,
    Is = 152,
    Keyof = 153,
    Uniquesymbol = 154,
    Unknown = 155,
    Never = 156,

    // ===== Flow type operators =====
    LBRACEPIPE = 157,        // {|
    PIPERBRACE = 158,        // |}

    // ===== Special terminals =====
    AutomaticSemicolon = 159,
    StringFragment3 = 160,
    QMARK2 = 161,
    HtmlComment = 162,
    JsxText = 163,
    FunctionSignatureAutomaticSemicolon = 164,
    ErrorRecovery = 165,

    // ===== Non-terminal nodes (AST node types) =====

    /// Program root node
    Program = 166,

    // ===== Export/Import statements =====
    ExportStatement = 167,
    NamespaceExport = 168,
    ExportClause = 169,
    ExportSpecifier = 170,
    ModuleExportName = 171,
    Declaration = 172,
    Import = 173,
    ImportStatement = 174,
    ImportClause = 175,
    FromClause = 176,
    NamespaceImport = 177,
    NamedImports = 178,
    ImportSpecifier = 179,
    ImportAttribute = 180,

    // ===== Statements =====
    Statement = 181,
    ExpressionStatement = 182,
    VariableDeclaration = 183,
    LexicalDeclaration = 184,
    VariableDeclarator = 185,
    StatementBlock = 186,
    ElseClause = 187,
    IfStatement = 188,
    SwitchStatement = 189,
    ForStatement = 190,
    ForInStatement = 191,
    ForHeader = 192,
    WhileStatement = 193,
    DoStatement = 194,
    TryStatement = 195,
    WithStatement = 196,
    BreakStatement = 197,
    ContinueStatement = 198,
    DebuggerStatement = 199,
    ReturnStatement = 200,
    ThrowStatement = 201,
    EmptyStatement = 202,
    LabeledStatement = 203,

    // ===== Switch statement parts =====
    SwitchBody = 204,
    SwitchCase = 205,
    SwitchDefault = 206,
    CatchClause = 207,
    FinallyClause = 208,

    // ===== Expressions =====
    ParenthesizedExpression = 209,
    Expression = 210,
    PrimaryExpression = 211,
    YieldExpression = 212,

    // ===== Objects and patterns =====
    Object = 213,
    ObjectPattern = 214,
    AssignmentPattern = 215,
    ObjectAssignmentPattern = 216,

    // ===== Arrays and patterns =====
    Array = 217,
    ArrayPattern = 218,

    // ===== Identifiers and classes =====
    NestedIdentifier = 219,
    Class = 220,
    ClassDeclaration = 221,
    ClassHeritage = 222,

    // ===== Functions =====
    FunctionExpression = 223,
    FunctionDeclaration = 224,
    GeneratorFunction = 225,
    GeneratorFunctionDeclaration = 226,
    ArrowFunction = 227,
    CallSignature2 = 228,
    FormalParameter = 229,

    // ===== Call and member expressions =====
    OptionalChain = 230,
    CallExpression = 231,
    NewExpression = 232,
    AwaitExpression = 233,
    MemberExpression = 234,
    SubscriptExpression = 235,

    // ===== Assignment expressions =====
    AssignmentExpression = 236,
    AugmentedAssignmentLhs = 237,
    AugmentedAssignmentExpression = 238,
    Initializer = 239,
    DestructuringPattern = 240,
    SpreadElement = 241,

    // ===== Other expressions =====
    TernaryExpression = 242,
    BinaryExpression = 243,
    UnaryExpression = 244,
    UpdateExpression = 245,
    SequenceExpression = 246,

    // ===== Literals =====
    String = 247,
    TemplateString = 248,
    TemplateSubstitution = 249,
    Regex = 250,
    MetaProperty = 251,
    Arguments = 252,

    // ===== Decorators =====
    Decorator = 253,
    MemberExpression2 = 254,
    CallExpression2 = 255,

    // ===== Class members =====
    ClassBody = 256,
    FormalParameters = 257,
    ClassStaticBlock = 258,
    Pattern = 259,
    RestPattern = 260,
    MethodDefinition = 261,
    Pair = 262,
    PairPattern = 263,
    PropertyName = 264,
    ComputedPropertyName = 265,
    PublicFieldDefinition = 266,
    ImportIdentifier = 267,

    // ===== TypeScript-specific expressions =====
    NonNullExpression = 268,
    MethodSignature = 269,
    AbstractMethodSignature = 270,
    FunctionSignature = 271,
    ParenthesizedExpression2 = 272,
    TypeAssertion = 273,
    AsExpression = 274,
    SatisfiesExpression = 275,
    InstantiationExpression = 276,
    ImportRequireClause = 277,

    // ===== TypeScript clauses =====
    ExtendsClause = 278,
    ExtendsClauseSingle = 279,
    ImplementsClause = 280,

    // ===== TypeScript declarations =====
    AmbientDeclaration = 281,
    AbstractClassDeclaration = 282,
    Module = 283,
    InternalModule = 284,
    Module3 = 285,
    ImportAlias = 286,
    NestedTypeIdentifier = 287,
    InterfaceDeclaration = 288,
    ExtendsTypeClause = 289,
    EnumDeclaration = 290,
    EnumBody = 291,
    EnumAssignment = 292,
    TypeAliasDeclaration = 293,

    // ===== Modifiers =====
    AccessibilityModifier = 294,
    OverrideModifier = 295,

    // ===== Parameters =====
    RequiredParameter = 296,
    OptionalParameter = 297,
    ParameterName = 298,

    // ===== Type annotations =====
    OmittingTypeAnnotation = 299,
    AddingTypeAnnotation = 300,
    OptingTypeAnnotation = 301,
    TypeAnnotation = 302,
    MemberExpression3 = 303,
    CallExpression3 = 304,
    Asserts = 305,
    AssertsAnnotation = 306,

    // ===== Type system =====
    Type2 = 307,
    RequiredParameter2 = 308,
    OptionalParameter2 = 309,
    OptionalType = 310,
    RestType = 311,
    TupleTypeMember = 312,
    ConstructorType = 313,
    PrimaryType = 314,
    TemplateType = 315,
    TemplateLiteralType = 316,
    InferType = 317,
    ConditionalType = 318,
    GenericType = 319,
    TypePredicate = 320,
    TypePredicateAnnotation = 321,
    MemberExpression4 = 322,
    SubscriptExpression2 = 323,
    CallExpression4 = 324,
    InstantiationExpression2 = 325,
    TypeQuery = 326,
    IndexTypeQuery = 327,
    LookupType = 328,
    MappedTypeClause = 329,
    LiteralType = 330,
    UnaryExpression2 = 331,
    ExistentialType = 332,
    FlowMaybeType = 333,
    ParenthesizedType = 334,
    PredefinedType = 335,
    TypeArguments = 336,
    ObjectType = 337,
    CallSignature = 338,
    PropertySignature = 339,
    TypeParameters = 340,
    TypeParameter = 341,
    DefaultType = 342,
    Constraint = 343,
    ConstructSignature = 344,
    IndexSignature = 345,
    ArrayType = 346,
    TupleType = 347,
    ReadonlyType = 348,
    UnionType = 349,
    IntersectionType = 350,
    FunctionType = 351,

    // ===== Repeat nodes (for sequences) =====
    ProgramRepeat1 = 352,
    ExportStatementRepeat1 = 353,
    ExportClauseRepeat1 = 354,
    NamedImportsRepeat1 = 355,
    VariableDeclarationRepeat1 = 356,
    SwitchBodyRepeat1 = 357,
    ObjectRepeat1 = 358,
    ObjectPatternRepeat1 = 359,
    ArrayRepeat1 = 360,
    ArrayPatternRepeat1 = 361,
    SequenceExpressionRepeat1 = 362,
    StringRepeat1 = 363,
    StringRepeat2 = 364,
    TemplateStringRepeat1 = 365,
    ClassBodyRepeat1 = 366,
    FormalParametersRepeat1 = 367,
    ExtendsClauseRepeat1 = 368,
    ImplementsClauseRepeat1 = 369,
    ExtendsTypeClauseRepeat1 = 370,
    EnumBodyRepeat1 = 371,
    TemplateLiteralTypeRepeat1 = 372,
    ObjectTypeRepeat1 = 373,
    TypeParametersRepeat1 = 374,
    TupleTypeRepeat1 = 375,

    // ===== Special TypeScript nodes =====
    InterfaceBody = 376,
    PropertyIdentifier = 377,
    ShorthandPropertyIdentifier = 378,
    ShorthandPropertyIdentifierPattern = 379,
    StatementIdentifier = 380,
    ThisType = 381,
    TypeIdentifier = 382,

    /// Error node
    Error = 383,
}

impl From<TypeScriptToken> for &'static str {
    #[inline(always)]
    fn from(tok: TypeScriptToken) -> Self {
        match tok {
            TypeScriptToken::End => "end",
            TypeScriptToken::Identifier => "identifier",
            TypeScriptToken::HashBangLine => "hash_bang_line",
            TypeScriptToken::Export => "export",
            TypeScriptToken::STAR => "*",
            TypeScriptToken::Default => "default",
            TypeScriptToken::Type => "type",
            TypeScriptToken::EQ => "=",
            TypeScriptToken::As => "as",
            TypeScriptToken::Namespace => "namespace",
            TypeScriptToken::LBRACE => "{",
            TypeScriptToken::COMMA => ",",
            TypeScriptToken::RBRACE => "}",
            TypeScriptToken::Typeof => "typeof",
            TypeScriptToken::Import2 => "import",
            TypeScriptToken::From => "from",
            TypeScriptToken::With => "with",
            TypeScriptToken::Assert => "assert",
            TypeScriptToken::Var => "var",
            TypeScriptToken::Let => "let",
            TypeScriptToken::Const => "const",
            TypeScriptToken::BANG => "!",
            TypeScriptToken::Else => "else",
            TypeScriptToken::If => "if",
            TypeScriptToken::Switch => "switch",
            TypeScriptToken::For => "for",
            TypeScriptToken::LPAREN => "(",
            TypeScriptToken::SEMI => ";",
            TypeScriptToken::RPAREN => ")",
            TypeScriptToken::Await => "await",
            TypeScriptToken::In => "in",
            TypeScriptToken::Of => "of",
            TypeScriptToken::While => "while",
            TypeScriptToken::Do => "do",
            TypeScriptToken::Try => "try",
            TypeScriptToken::Break => "break",
            TypeScriptToken::Continue => "continue",
            TypeScriptToken::Debugger => "debugger",
            TypeScriptToken::Return => "return",
            TypeScriptToken::Throw => "throw",
            TypeScriptToken::COLON => ":",
            TypeScriptToken::Case => "case",
            TypeScriptToken::Catch => "catch",
            TypeScriptToken::Finally => "finally",
            TypeScriptToken::Yield => "yield",
            TypeScriptToken::LBRACK => "[",
            TypeScriptToken::RBRACK => "]",
            TypeScriptToken::DOT => ".",
            TypeScriptToken::Class2 => "class",
            TypeScriptToken::Async => "async",
            TypeScriptToken::Function => "function",
            TypeScriptToken::EQGT => "=>",
            TypeScriptToken::QMARKDOT => "?.",
            TypeScriptToken::New => "new",
            TypeScriptToken::Using => "using",
            TypeScriptToken::PLUSEQ => "+=",
            TypeScriptToken::DASHEQ => "-=",
            TypeScriptToken::STAREQ => "*=",
            TypeScriptToken::SLASHEQ => "/=",
            TypeScriptToken::PERCENTEQ => "%=",
            TypeScriptToken::CARETEQ => "^=",
            TypeScriptToken::AMPEQ => "&=",
            TypeScriptToken::PIPEEQ => "|=",
            TypeScriptToken::GTGTEQ => ">>=",
            TypeScriptToken::GTGTGTEQ => ">>>=",
            TypeScriptToken::LTLTEQ => "<<=",
            TypeScriptToken::STARSTAREQ => "**=",
            TypeScriptToken::AMPAMPEQ => "&&=",
            TypeScriptToken::PIPEPIPEEQ => "||=",
            TypeScriptToken::QMARKQMARKEQ => "??=",
            TypeScriptToken::DOTDOTDOT => "...",
            TypeScriptToken::AMPAMP => "&&",
            TypeScriptToken::PIPEPIPE => "||",
            TypeScriptToken::GTGT => ">>",
            TypeScriptToken::GTGTGT => ">>>",
            TypeScriptToken::LTLT => "<<",
            TypeScriptToken::AMP => "&",
            TypeScriptToken::CARET => "^",
            TypeScriptToken::PIPE => "|",
            TypeScriptToken::PLUS => "+",
            TypeScriptToken::DASH => "-",
            TypeScriptToken::SLASH => "/",
            TypeScriptToken::PERCENT => "%",
            TypeScriptToken::STARSTAR => "**",
            TypeScriptToken::LT => "<",
            TypeScriptToken::LTEQ => "<=",
            TypeScriptToken::EQEQ => "==",
            TypeScriptToken::EQEQEQ => "===",
            TypeScriptToken::BANGEQ => "!=",
            TypeScriptToken::BANGEQEQ => "!==",
            TypeScriptToken::GTEQ => ">=",
            TypeScriptToken::GT => ">",
            TypeScriptToken::QMARKQMARK => "??",
            TypeScriptToken::Instanceof => "instanceof",
            TypeScriptToken::TILDE => "~",
            TypeScriptToken::Void => "void",
            TypeScriptToken::Delete => "delete",
            TypeScriptToken::PLUSPLUS => "++",
            TypeScriptToken::DASHDASH => "--",
            TypeScriptToken::DQUOTE => "\"",
            TypeScriptToken::SQUOTE => "'",
            TypeScriptToken::StringFragment => "string_fragment",
            TypeScriptToken::StringFragment2 => "string_fragment",
            TypeScriptToken::EscapeSequence => "escape_sequence",
            TypeScriptToken::Comment => "comment",
            TypeScriptToken::BQUOTE => "`",
            TypeScriptToken::DOLLARLBRACE => "${",
            TypeScriptToken::SLASH2 => "/",
            TypeScriptToken::RegexPattern => "regex_pattern",
            TypeScriptToken::RegexFlags => "regex_flags",
            TypeScriptToken::Number => "number",
            TypeScriptToken::PrivatePropertyIdentifier => "private_property_identifier",
            TypeScriptToken::Target => "target",
            TypeScriptToken::Meta => "meta",
            TypeScriptToken::This => "this",
            TypeScriptToken::Super => "super",
            TypeScriptToken::True => "true",
            TypeScriptToken::False => "false",
            TypeScriptToken::Null => "null",
            TypeScriptToken::Undefined => "undefined",
            TypeScriptToken::AT => "@",
            TypeScriptToken::Static => "static",
            TypeScriptToken::Readonly => "readonly",
            TypeScriptToken::Get => "get",
            TypeScriptToken::Set => "set",
            TypeScriptToken::QMARK => "?",
            TypeScriptToken::Declare => "declare",
            TypeScriptToken::Public => "public",
            TypeScriptToken::Private => "private",
            TypeScriptToken::Protected => "protected",
            TypeScriptToken::Override => "override",
            TypeScriptToken::Module2 => "module",
            TypeScriptToken::Any => "any",
            TypeScriptToken::Number2 => "number",
            TypeScriptToken::Boolean => "boolean",
            TypeScriptToken::String2 => "string",
            TypeScriptToken::Symbol => "symbol",
            TypeScriptToken::Object2 => "object",
            TypeScriptToken::Abstract => "abstract",
            TypeScriptToken::Accessor => "accessor",
            TypeScriptToken::Satisfies => "satisfies",
            TypeScriptToken::Require => "require",
            TypeScriptToken::Extends => "extends",
            TypeScriptToken::Implements => "implements",
            TypeScriptToken::Global => "global",
            TypeScriptToken::Interface => "interface",
            TypeScriptToken::Enum => "enum",
            TypeScriptToken::DASHQMARKCOLON => "-?:",
            TypeScriptToken::PLUSQMARKCOLON => "+?:",
            TypeScriptToken::QMARKCOLON => "?:",
            TypeScriptToken::Asserts2 => "asserts",
            TypeScriptToken::Infer => "infer",
            TypeScriptToken::Is => "is",
            TypeScriptToken::Keyof => "keyof",
            TypeScriptToken::Uniquesymbol => "unique symbol",
            TypeScriptToken::Unknown => "unknown",
            TypeScriptToken::Never => "never",
            TypeScriptToken::LBRACEPIPE => "{|",
            TypeScriptToken::PIPERBRACE => "|}",
            TypeScriptToken::AutomaticSemicolon => "_automatic_semicolon",
            TypeScriptToken::StringFragment3 => "string_fragment",
            TypeScriptToken::QMARK2 => "?",
            TypeScriptToken::HtmlComment => "html_comment",
            TypeScriptToken::JsxText => "jsx_text",
            TypeScriptToken::FunctionSignatureAutomaticSemicolon => {
                "_function_signature_automatic_semicolon"
            }
            TypeScriptToken::ErrorRecovery => "__error_recovery",
            TypeScriptToken::Program => "program",
            TypeScriptToken::ExportStatement => "export_statement",
            TypeScriptToken::NamespaceExport => "namespace_export",
            TypeScriptToken::ExportClause => "export_clause",
            TypeScriptToken::ExportSpecifier => "export_specifier",
            TypeScriptToken::ModuleExportName => "_module_export_name",
            TypeScriptToken::Declaration => "declaration",
            TypeScriptToken::Import => "import",
            TypeScriptToken::ImportStatement => "import_statement",
            TypeScriptToken::ImportClause => "import_clause",
            TypeScriptToken::FromClause => "_from_clause",
            TypeScriptToken::NamespaceImport => "namespace_import",
            TypeScriptToken::NamedImports => "named_imports",
            TypeScriptToken::ImportSpecifier => "import_specifier",
            TypeScriptToken::ImportAttribute => "import_attribute",
            TypeScriptToken::Statement => "statement",
            TypeScriptToken::ExpressionStatement => "expression_statement",
            TypeScriptToken::VariableDeclaration => "variable_declaration",
            TypeScriptToken::LexicalDeclaration => "lexical_declaration",
            TypeScriptToken::VariableDeclarator => "variable_declarator",
            TypeScriptToken::StatementBlock => "statement_block",
            TypeScriptToken::ElseClause => "else_clause",
            TypeScriptToken::IfStatement => "if_statement",
            TypeScriptToken::SwitchStatement => "switch_statement",
            TypeScriptToken::ForStatement => "for_statement",
            TypeScriptToken::ForInStatement => "for_in_statement",
            TypeScriptToken::ForHeader => "_for_header",
            TypeScriptToken::WhileStatement => "while_statement",
            TypeScriptToken::DoStatement => "do_statement",
            TypeScriptToken::TryStatement => "try_statement",
            TypeScriptToken::WithStatement => "with_statement",
            TypeScriptToken::BreakStatement => "break_statement",
            TypeScriptToken::ContinueStatement => "continue_statement",
            TypeScriptToken::DebuggerStatement => "debugger_statement",
            TypeScriptToken::ReturnStatement => "return_statement",
            TypeScriptToken::ThrowStatement => "throw_statement",
            TypeScriptToken::EmptyStatement => "empty_statement",
            TypeScriptToken::LabeledStatement => "labeled_statement",
            TypeScriptToken::SwitchBody => "switch_body",
            TypeScriptToken::SwitchCase => "switch_case",
            TypeScriptToken::SwitchDefault => "switch_default",
            TypeScriptToken::CatchClause => "catch_clause",
            TypeScriptToken::FinallyClause => "finally_clause",
            TypeScriptToken::ParenthesizedExpression => "parenthesized_expression",
            TypeScriptToken::Expression => "expression",
            TypeScriptToken::PrimaryExpression => "primary_expression",
            TypeScriptToken::YieldExpression => "yield_expression",
            TypeScriptToken::Object => "object",
            TypeScriptToken::ObjectPattern => "object_pattern",
            TypeScriptToken::AssignmentPattern => "assignment_pattern",
            TypeScriptToken::ObjectAssignmentPattern => "object_assignment_pattern",
            TypeScriptToken::Array => "array",
            TypeScriptToken::ArrayPattern => "array_pattern",
            TypeScriptToken::NestedIdentifier => "nested_identifier",
            TypeScriptToken::Class => "class",
            TypeScriptToken::ClassDeclaration => "class_declaration",
            TypeScriptToken::ClassHeritage => "class_heritage",
            TypeScriptToken::FunctionExpression => "function_expression",
            TypeScriptToken::FunctionDeclaration => "function_declaration",
            TypeScriptToken::GeneratorFunction => "generator_function",
            TypeScriptToken::GeneratorFunctionDeclaration => "generator_function_declaration",
            TypeScriptToken::ArrowFunction => "arrow_function",
            TypeScriptToken::CallSignature2 => "_call_signature",
            TypeScriptToken::FormalParameter => "_formal_parameter",
            TypeScriptToken::OptionalChain => "optional_chain",
            TypeScriptToken::CallExpression => "call_expression",
            TypeScriptToken::NewExpression => "new_expression",
            TypeScriptToken::AwaitExpression => "await_expression",
            TypeScriptToken::MemberExpression => "member_expression",
            TypeScriptToken::SubscriptExpression => "subscript_expression",
            TypeScriptToken::AssignmentExpression => "assignment_expression",
            TypeScriptToken::AugmentedAssignmentLhs => "_augmented_assignment_lhs",
            TypeScriptToken::AugmentedAssignmentExpression => "augmented_assignment_expression",
            TypeScriptToken::Initializer => "_initializer",
            TypeScriptToken::DestructuringPattern => "_destructuring_pattern",
            TypeScriptToken::SpreadElement => "spread_element",
            TypeScriptToken::TernaryExpression => "ternary_expression",
            TypeScriptToken::BinaryExpression => "binary_expression",
            TypeScriptToken::UnaryExpression => "unary_expression",
            TypeScriptToken::UpdateExpression => "update_expression",
            TypeScriptToken::SequenceExpression => "sequence_expression",
            TypeScriptToken::String => "string",
            TypeScriptToken::TemplateString => "template_string",
            TypeScriptToken::TemplateSubstitution => "template_substitution",
            TypeScriptToken::Regex => "regex",
            TypeScriptToken::MetaProperty => "meta_property",
            TypeScriptToken::Arguments => "arguments",
            TypeScriptToken::Decorator => "decorator",
            TypeScriptToken::MemberExpression2 => "member_expression",
            TypeScriptToken::CallExpression2 => "call_expression",
            TypeScriptToken::ClassBody => "class_body",
            TypeScriptToken::FormalParameters => "formal_parameters",
            TypeScriptToken::ClassStaticBlock => "class_static_block",
            TypeScriptToken::Pattern => "pattern",
            TypeScriptToken::RestPattern => "rest_pattern",
            TypeScriptToken::MethodDefinition => "method_definition",
            TypeScriptToken::Pair => "pair",
            TypeScriptToken::PairPattern => "pair_pattern",
            TypeScriptToken::PropertyName => "_property_name",
            TypeScriptToken::ComputedPropertyName => "computed_property_name",
            TypeScriptToken::PublicFieldDefinition => "public_field_definition",
            TypeScriptToken::ImportIdentifier => "_import_identifier",
            TypeScriptToken::NonNullExpression => "non_null_expression",
            TypeScriptToken::MethodSignature => "method_signature",
            TypeScriptToken::AbstractMethodSignature => "abstract_method_signature",
            TypeScriptToken::FunctionSignature => "function_signature",
            TypeScriptToken::ParenthesizedExpression2 => "parenthesized_expression",
            TypeScriptToken::TypeAssertion => "type_assertion",
            TypeScriptToken::AsExpression => "as_expression",
            TypeScriptToken::SatisfiesExpression => "satisfies_expression",
            TypeScriptToken::InstantiationExpression => "instantiation_expression",
            TypeScriptToken::ImportRequireClause => "import_require_clause",
            TypeScriptToken::ExtendsClause => "extends_clause",
            TypeScriptToken::ExtendsClauseSingle => "_extends_clause_single",
            TypeScriptToken::ImplementsClause => "implements_clause",
            TypeScriptToken::AmbientDeclaration => "ambient_declaration",
            TypeScriptToken::AbstractClassDeclaration => "abstract_class_declaration",
            TypeScriptToken::Module => "module",
            TypeScriptToken::InternalModule => "internal_module",
            TypeScriptToken::Module3 => "_module",
            TypeScriptToken::ImportAlias => "import_alias",
            TypeScriptToken::NestedTypeIdentifier => "nested_type_identifier",
            TypeScriptToken::InterfaceDeclaration => "interface_declaration",
            TypeScriptToken::ExtendsTypeClause => "extends_type_clause",
            TypeScriptToken::EnumDeclaration => "enum_declaration",
            TypeScriptToken::EnumBody => "enum_body",
            TypeScriptToken::EnumAssignment => "enum_assignment",
            TypeScriptToken::TypeAliasDeclaration => "type_alias_declaration",
            TypeScriptToken::AccessibilityModifier => "accessibility_modifier",
            TypeScriptToken::OverrideModifier => "override_modifier",
            TypeScriptToken::RequiredParameter => "required_parameter",
            TypeScriptToken::OptionalParameter => "optional_parameter",
            TypeScriptToken::ParameterName => "_parameter_name",
            TypeScriptToken::OmittingTypeAnnotation => "omitting_type_annotation",
            TypeScriptToken::AddingTypeAnnotation => "adding_type_annotation",
            TypeScriptToken::OptingTypeAnnotation => "opting_type_annotation",
            TypeScriptToken::TypeAnnotation => "type_annotation",
            TypeScriptToken::MemberExpression3 => "member_expression",
            TypeScriptToken::CallExpression3 => "call_expression",
            TypeScriptToken::Asserts => "asserts",
            TypeScriptToken::AssertsAnnotation => "asserts_annotation",
            TypeScriptToken::Type2 => "type",
            TypeScriptToken::RequiredParameter2 => "required_parameter",
            TypeScriptToken::OptionalParameter2 => "optional_parameter",
            TypeScriptToken::OptionalType => "optional_type",
            TypeScriptToken::RestType => "rest_type",
            TypeScriptToken::TupleTypeMember => "_tuple_type_member",
            TypeScriptToken::ConstructorType => "constructor_type",
            TypeScriptToken::PrimaryType => "primary_type",
            TypeScriptToken::TemplateType => "template_type",
            TypeScriptToken::TemplateLiteralType => "template_literal_type",
            TypeScriptToken::InferType => "infer_type",
            TypeScriptToken::ConditionalType => "conditional_type",
            TypeScriptToken::GenericType => "generic_type",
            TypeScriptToken::TypePredicate => "type_predicate",
            TypeScriptToken::TypePredicateAnnotation => "type_predicate_annotation",
            TypeScriptToken::MemberExpression4 => "member_expression",
            TypeScriptToken::SubscriptExpression2 => "subscript_expression",
            TypeScriptToken::CallExpression4 => "call_expression",
            TypeScriptToken::InstantiationExpression2 => "instantiation_expression",
            TypeScriptToken::TypeQuery => "type_query",
            TypeScriptToken::IndexTypeQuery => "index_type_query",
            TypeScriptToken::LookupType => "lookup_type",
            TypeScriptToken::MappedTypeClause => "mapped_type_clause",
            TypeScriptToken::LiteralType => "literal_type",
            TypeScriptToken::UnaryExpression2 => "unary_expression",
            TypeScriptToken::ExistentialType => "existential_type",
            TypeScriptToken::FlowMaybeType => "flow_maybe_type",
            TypeScriptToken::ParenthesizedType => "parenthesized_type",
            TypeScriptToken::PredefinedType => "predefined_type",
            TypeScriptToken::TypeArguments => "type_arguments",
            TypeScriptToken::ObjectType => "object_type",
            TypeScriptToken::CallSignature => "call_signature",
            TypeScriptToken::PropertySignature => "property_signature",
            TypeScriptToken::TypeParameters => "type_parameters",
            TypeScriptToken::TypeParameter => "type_parameter",
            TypeScriptToken::DefaultType => "default_type",
            TypeScriptToken::Constraint => "constraint",
            TypeScriptToken::ConstructSignature => "construct_signature",
            TypeScriptToken::IndexSignature => "index_signature",
            TypeScriptToken::ArrayType => "array_type",
            TypeScriptToken::TupleType => "tuple_type",
            TypeScriptToken::ReadonlyType => "readonly_type",
            TypeScriptToken::UnionType => "union_type",
            TypeScriptToken::IntersectionType => "intersection_type",
            TypeScriptToken::FunctionType => "function_type",
            TypeScriptToken::ProgramRepeat1 => "program_repeat1",
            TypeScriptToken::ExportStatementRepeat1 => "export_statement_repeat1",
            TypeScriptToken::ExportClauseRepeat1 => "export_clause_repeat1",
            TypeScriptToken::NamedImportsRepeat1 => "named_imports_repeat1",
            TypeScriptToken::VariableDeclarationRepeat1 => "variable_declaration_repeat1",
            TypeScriptToken::SwitchBodyRepeat1 => "switch_body_repeat1",
            TypeScriptToken::ObjectRepeat1 => "object_repeat1",
            TypeScriptToken::ObjectPatternRepeat1 => "object_pattern_repeat1",
            TypeScriptToken::ArrayRepeat1 => "array_repeat1",
            TypeScriptToken::ArrayPatternRepeat1 => "array_pattern_repeat1",
            TypeScriptToken::SequenceExpressionRepeat1 => "sequence_expression_repeat1",
            TypeScriptToken::StringRepeat1 => "string_repeat1",
            TypeScriptToken::StringRepeat2 => "string_repeat2",
            TypeScriptToken::TemplateStringRepeat1 => "template_string_repeat1",
            TypeScriptToken::ClassBodyRepeat1 => "class_body_repeat1",
            TypeScriptToken::FormalParametersRepeat1 => "formal_parameters_repeat1",
            TypeScriptToken::ExtendsClauseRepeat1 => "extends_clause_repeat1",
            TypeScriptToken::ImplementsClauseRepeat1 => "implements_clause_repeat1",
            TypeScriptToken::ExtendsTypeClauseRepeat1 => "extends_type_clause_repeat1",
            TypeScriptToken::EnumBodyRepeat1 => "enum_body_repeat1",
            TypeScriptToken::TemplateLiteralTypeRepeat1 => "template_literal_type_repeat1",
            TypeScriptToken::ObjectTypeRepeat1 => "object_type_repeat1",
            TypeScriptToken::TypeParametersRepeat1 => "type_parameters_repeat1",
            TypeScriptToken::TupleTypeRepeat1 => "tuple_type_repeat1",
            TypeScriptToken::InterfaceBody => "interface_body",
            TypeScriptToken::PropertyIdentifier => "property_identifier",
            TypeScriptToken::ShorthandPropertyIdentifier => "shorthand_property_identifier",
            TypeScriptToken::ShorthandPropertyIdentifierPattern => {
                "shorthand_property_identifier_pattern"
            }
            TypeScriptToken::StatementIdentifier => "statement_identifier",
            TypeScriptToken::ThisType => "this_type",
            TypeScriptToken::TypeIdentifier => "type_identifier",
            TypeScriptToken::Error => "ERROR",
        }
    }
}

impl From<u16> for TypeScriptToken {
    #[inline(always)]
    fn from(x: u16) -> Self {
        num::FromPrimitive::from_u16(x).unwrap_or(Self::Error)
    }
}

// TypeScriptToken == u16
impl PartialEq<u16> for TypeScriptToken {
    #[inline(always)]
    fn eq(&self, x: &u16) -> bool {
        *self == Into::<Self>::into(*x)
    }
}

// u16 == TypeScriptToken
impl PartialEq<TypeScriptToken> for u16 {
    #[inline(always)]
    fn eq(&self, x: &TypeScriptToken) -> bool {
        *x == *self
    }
}

/// TypeScript language implementation.
///
/// Provides metadata and configuration for TypeScript parsing with full type system
/// support including generics, union/intersection types, mapped types, conditional types,
/// and all TypeScript-specific features.
pub struct TypeScriptLanguage;

impl LanguageInfo for TypeScriptLanguage {
    fn get_lang() -> Lang {
        Lang::TypeScript
    }

    fn get_lang_name() -> &'static str {
        "typescript"
    }
}

// Helper methods for TypeScript token detection
impl TypeScriptToken {
    /// Check if this token represents a function-like construct.
    ///
    /// Includes regular functions, arrow functions, generators, async functions,
    /// and function signatures.
    #[inline]
    pub fn is_function(&self) -> bool {
        matches!(
            self,
            TypeScriptToken::FunctionDeclaration
                | TypeScriptToken::FunctionExpression
                | TypeScriptToken::ArrowFunction
                | TypeScriptToken::GeneratorFunction
                | TypeScriptToken::GeneratorFunctionDeclaration
                | TypeScriptToken::MethodDefinition
                | TypeScriptToken::FunctionSignature
                | TypeScriptToken::MethodSignature
                | TypeScriptToken::AbstractMethodSignature
        )
    }

    /// Check if this token represents an arrow function.
    #[inline]
    pub fn is_arrow_function(&self) -> bool {
        matches!(self, TypeScriptToken::ArrowFunction)
    }

    /// Check if this token represents a class-related construct.
    #[inline]
    pub fn is_class(&self) -> bool {
        matches!(
            self,
            TypeScriptToken::Class
                | TypeScriptToken::ClassDeclaration
                | TypeScriptToken::AbstractClassDeclaration
        )
    }

    /// Check if this token represents an interface declaration.
    #[inline]
    pub fn is_interface(&self) -> bool {
        matches!(self, TypeScriptToken::InterfaceDeclaration)
    }

    /// Check if this token represents a type alias.
    #[inline]
    pub fn is_type_alias(&self) -> bool {
        matches!(self, TypeScriptToken::TypeAliasDeclaration)
    }

    /// Check if this token represents an enum declaration.
    #[inline]
    pub fn is_enum(&self) -> bool {
        matches!(self, TypeScriptToken::EnumDeclaration)
    }

    /// Check if this token represents a namespace or module.
    #[inline]
    pub fn is_namespace(&self) -> bool {
        matches!(
            self,
            TypeScriptToken::Module | TypeScriptToken::InternalModule
        )
    }

    /// Check if this token represents an async construct.
    #[inline]
    pub fn is_async(&self) -> bool {
        matches!(self, TypeScriptToken::Async | TypeScriptToken::AwaitExpression)
    }

    /// Check if this token represents a generator.
    #[inline]
    pub fn is_generator(&self) -> bool {
        matches!(
            self,
            TypeScriptToken::GeneratorFunction | TypeScriptToken::GeneratorFunctionDeclaration
        )
    }

    /// Check if this token represents a decorator.
    #[inline]
    pub fn is_decorator(&self) -> bool {
        matches!(self, TypeScriptToken::Decorator)
    }

    /// Check if this token represents a type annotation.
    #[inline]
    pub fn is_type_annotation(&self) -> bool {
        matches!(
            self,
            TypeScriptToken::TypeAnnotation
                | TypeScriptToken::TypeAssertion
                | TypeScriptToken::AsExpression
                | TypeScriptToken::SatisfiesExpression
        )
    }

    /// Check if this token represents a type predicate.
    #[inline]
    pub fn is_type_predicate(&self) -> bool {
        matches!(
            self,
            TypeScriptToken::TypePredicate | TypeScriptToken::TypePredicateAnnotation
        )
    }

    /// Check if this token represents a generic type.
    #[inline]
    pub fn is_generic(&self) -> bool {
        matches!(
            self,
            TypeScriptToken::GenericType
                | TypeScriptToken::TypeParameters
                | TypeScriptToken::TypeArguments
        )
    }

    /// Check if this token represents a union or intersection type.
    #[inline]
    pub fn is_composite_type(&self) -> bool {
        matches!(
            self,
            TypeScriptToken::UnionType | TypeScriptToken::IntersectionType
        )
    }

    /// Check if this token represents a conditional type.
    #[inline]
    pub fn is_conditional_type(&self) -> bool {
        matches!(self, TypeScriptToken::ConditionalType)
    }

    /// Check if this token represents a mapped type.
    #[inline]
    pub fn is_mapped_type(&self) -> bool {
        matches!(self, TypeScriptToken::MappedTypeClause)
    }

    /// Check if this token represents a template literal type.
    #[inline]
    pub fn is_template_literal_type(&self) -> bool {
        matches!(self, TypeScriptToken::TemplateLiteralType)
    }

    /// Check if this token represents an infer type.
    #[inline]
    pub fn is_infer_type(&self) -> bool {
        matches!(self, TypeScriptToken::InferType)
    }

    /// Check if this token represents an index type query.
    #[inline]
    pub fn is_index_type(&self) -> bool {
        matches!(
            self,
            TypeScriptToken::IndexTypeQuery | TypeScriptToken::LookupType
        )
    }

    /// Check if this token represents a readonly modifier.
    #[inline]
    pub fn is_readonly(&self) -> bool {
        matches!(self, TypeScriptToken::Readonly | TypeScriptToken::ReadonlyType)
    }

    /// Check if this token represents an accessibility modifier.
    #[inline]
    pub fn is_accessibility_modifier(&self) -> bool {
        matches!(
            self,
            TypeScriptToken::Public
                | TypeScriptToken::Private
                | TypeScriptToken::Protected
                | TypeScriptToken::AccessibilityModifier
        )
    }

    /// Check if this token represents a binary operator.
    #[inline]
    pub fn is_binary_operator(&self) -> bool {
        matches!(
            self,
            TypeScriptToken::PLUS
                | TypeScriptToken::DASH
                | TypeScriptToken::SLASH
                | TypeScriptToken::PERCENT
                | TypeScriptToken::STARSTAR
                | TypeScriptToken::AMP
                | TypeScriptToken::PIPE
                | TypeScriptToken::CARET
                | TypeScriptToken::AMPAMP
                | TypeScriptToken::PIPEPIPE
                | TypeScriptToken::LTLT
                | TypeScriptToken::GTGT
                | TypeScriptToken::GTGTGT
                | TypeScriptToken::EQEQ
                | TypeScriptToken::EQEQEQ
                | TypeScriptToken::BANGEQ
                | TypeScriptToken::BANGEQEQ
                | TypeScriptToken::LT
                | TypeScriptToken::LTEQ
                | TypeScriptToken::GT
                | TypeScriptToken::GTEQ
                | TypeScriptToken::QMARKQMARK
                | TypeScriptToken::Instanceof
        )
    }

    /// Check if this token represents a unary operator.
    #[inline]
    pub fn is_unary_operator(&self) -> bool {
        matches!(
            self,
            TypeScriptToken::BANG
                | TypeScriptToken::TILDE
                | TypeScriptToken::Typeof
                | TypeScriptToken::Void
                | TypeScriptToken::Delete
                | TypeScriptToken::PLUS
                | TypeScriptToken::DASH
        )
    }

    /// Check if this token represents an assignment operator.
    #[inline]
    pub fn is_assignment_operator(&self) -> bool {
        matches!(
            self,
            TypeScriptToken::EQ
                | TypeScriptToken::PLUSEQ
                | TypeScriptToken::DASHEQ
                | TypeScriptToken::STAREQ
                | TypeScriptToken::SLASHEQ
                | TypeScriptToken::PERCENTEQ
                | TypeScriptToken::CARETEQ
                | TypeScriptToken::AMPEQ
                | TypeScriptToken::PIPEEQ
                | TypeScriptToken::GTGTEQ
                | TypeScriptToken::GTGTGTEQ
                | TypeScriptToken::LTLTEQ
                | TypeScriptToken::STARSTAREQ
                | TypeScriptToken::AMPAMPEQ
                | TypeScriptToken::PIPEPIPEEQ
                | TypeScriptToken::QMARKQMARKEQ
        )
    }

    /// Check if this token represents a type operator.
    #[inline]
    pub fn is_type_operator(&self) -> bool {
        matches!(
            self,
            TypeScriptToken::Keyof | TypeScriptToken::Typeof | TypeScriptToken::Infer
        )
    }

    /// Check if this token represents a literal value.
    #[inline]
    pub fn is_literal(&self) -> bool {
        matches!(
            self,
            TypeScriptToken::Number
                | TypeScriptToken::String
                | TypeScriptToken::True
                | TypeScriptToken::False
                | TypeScriptToken::Null
                | TypeScriptToken::Undefined
                | TypeScriptToken::TemplateString
                | TypeScriptToken::Regex
        )
    }

    /// Check if this token represents a predefined type.
    #[inline]
    pub fn is_predefined_type(&self) -> bool {
        matches!(
            self,
            TypeScriptToken::Any
                | TypeScriptToken::Number2
                | TypeScriptToken::Boolean
                | TypeScriptToken::String2
                | TypeScriptToken::Symbol
                | TypeScriptToken::Object2
                | TypeScriptToken::Unknown
                | TypeScriptToken::Never
                | TypeScriptToken::Void
                | TypeScriptToken::Uniquesymbol
                | TypeScriptToken::PredefinedType
        )
    }

    /// Check if this token represents a JSX-related construct.
    #[inline]
    pub fn is_jsx(&self) -> bool {
        matches!(self, TypeScriptToken::JsxText)
    }

    /// Check if this token represents a loop statement.
    #[inline]
    pub fn is_loop(&self) -> bool {
        matches!(
            self,
            TypeScriptToken::ForStatement
                | TypeScriptToken::ForInStatement
                | TypeScriptToken::WhileStatement
                | TypeScriptToken::DoStatement
        )
    }

    /// Check if this token represents an import/export statement.
    #[inline]
    pub fn is_module(&self) -> bool {
        matches!(
            self,
            TypeScriptToken::ImportStatement
                | TypeScriptToken::ExportStatement
                | TypeScriptToken::Import
                | TypeScriptToken::Export
        )
    }

    /// Check if this token represents an ambient declaration.
    #[inline]
    pub fn is_ambient(&self) -> bool {
        matches!(
            self,
            TypeScriptToken::Declare | TypeScriptToken::AmbientDeclaration
        )
    }

    /// Check if this token represents an abstract construct.
    #[inline]
    pub fn is_abstract(&self) -> bool {
        matches!(
            self,
            TypeScriptToken::Abstract
                | TypeScriptToken::AbstractClassDeclaration
                | TypeScriptToken::AbstractMethodSignature
        )
    }

    /// Check if this token represents a declaration.
    #[inline]
    pub fn is_declaration(&self) -> bool {
        matches!(
            self,
            TypeScriptToken::FunctionDeclaration
                | TypeScriptToken::ClassDeclaration
                | TypeScriptToken::InterfaceDeclaration
                | TypeScriptToken::TypeAliasDeclaration
                | TypeScriptToken::EnumDeclaration
                | TypeScriptToken::VariableDeclaration
                | TypeScriptToken::LexicalDeclaration
                | TypeScriptToken::AbstractClassDeclaration
        )
    }

    /// Check if this token represents a type construct.
    #[inline]
    pub fn is_type_construct(&self) -> bool {
        matches!(
            self,
            TypeScriptToken::InterfaceDeclaration
                | TypeScriptToken::TypeAliasDeclaration
                | TypeScriptToken::EnumDeclaration
                | TypeScriptToken::ObjectType
                | TypeScriptToken::ArrayType
                | TypeScriptToken::TupleType
                | TypeScriptToken::FunctionType
                | TypeScriptToken::ConstructorType
                | TypeScriptToken::UnionType
                | TypeScriptToken::IntersectionType
                | TypeScriptToken::ConditionalType
                | TypeScriptToken::GenericType
        )
    }

    /// Check if this token is an optional chain operator.
    #[inline]
    pub fn is_optional_chain(&self) -> bool {
        matches!(self, TypeScriptToken::QMARKDOT | TypeScriptToken::OptionalChain)
    }

    /// Check if this token represents a non-null assertion.
    #[inline]
    pub fn is_non_null_assertion(&self) -> bool {
        matches!(self, TypeScriptToken::NonNullExpression)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_conversion() {
        assert_eq!(
            TypeScriptToken::from(224),
            TypeScriptToken::FunctionDeclaration
        );
        assert_eq!(TypeScriptToken::from(227), TypeScriptToken::ArrowFunction);
        assert_eq!(TypeScriptToken::from(221), TypeScriptToken::ClassDeclaration);
        assert_eq!(
            TypeScriptToken::from(288),
            TypeScriptToken::InterfaceDeclaration
        );
        assert_eq!(
            TypeScriptToken::from(290),
            TypeScriptToken::EnumDeclaration
        );
    }

    #[test]
    fn test_function_detection() {
        assert!(TypeScriptToken::FunctionDeclaration.is_function());
        assert!(TypeScriptToken::ArrowFunction.is_function());
        assert!(TypeScriptToken::ArrowFunction.is_arrow_function());
        assert!(TypeScriptToken::GeneratorFunction.is_generator());
        assert!(TypeScriptToken::FunctionSignature.is_function());
        assert!(TypeScriptToken::MethodSignature.is_function());
    }

    #[test]
    fn test_class_detection() {
        assert!(TypeScriptToken::ClassDeclaration.is_class());
        assert!(TypeScriptToken::AbstractClassDeclaration.is_class());
        assert!(TypeScriptToken::AbstractClassDeclaration.is_abstract());
    }

    #[test]
    fn test_type_detection() {
        assert!(TypeScriptToken::InterfaceDeclaration.is_interface());
        assert!(TypeScriptToken::TypeAliasDeclaration.is_type_alias());
        assert!(TypeScriptToken::EnumDeclaration.is_enum());
        assert!(TypeScriptToken::UnionType.is_composite_type());
        assert!(TypeScriptToken::IntersectionType.is_composite_type());
        assert!(TypeScriptToken::ConditionalType.is_conditional_type());
    }

    #[test]
    fn test_type_operator_detection() {
        assert!(TypeScriptToken::Keyof.is_type_operator());
        assert!(TypeScriptToken::Typeof.is_type_operator());
        assert!(TypeScriptToken::Infer.is_type_operator());
        assert!(TypeScriptToken::InferType.is_infer_type());
    }

    #[test]
    fn test_operator_detection() {
        assert!(TypeScriptToken::PLUS.is_binary_operator());
        assert!(TypeScriptToken::BANG.is_unary_operator());
        assert!(TypeScriptToken::PLUSEQ.is_assignment_operator());
        assert!(TypeScriptToken::QMARKDOT.is_optional_chain());
    }

    #[test]
    fn test_literal_detection() {
        assert!(TypeScriptToken::Number.is_literal());
        assert!(TypeScriptToken::String.is_literal());
        assert!(TypeScriptToken::True.is_literal());
        assert!(TypeScriptToken::Null.is_literal());
    }

    #[test]
    fn test_predefined_type_detection() {
        assert!(TypeScriptToken::Any.is_predefined_type());
        assert!(TypeScriptToken::Unknown.is_predefined_type());
        assert!(TypeScriptToken::Never.is_predefined_type());
        assert!(TypeScriptToken::Void.is_predefined_type());
        assert!(TypeScriptToken::Uniquesymbol.is_predefined_type());
    }

    #[test]
    fn test_modifier_detection() {
        assert!(TypeScriptToken::Public.is_accessibility_modifier());
        assert!(TypeScriptToken::Private.is_accessibility_modifier());
        assert!(TypeScriptToken::Protected.is_accessibility_modifier());
        assert!(TypeScriptToken::Readonly.is_readonly());
        assert!(TypeScriptToken::Abstract.is_abstract());
    }

    #[test]
    fn test_decorator_detection() {
        assert!(TypeScriptToken::Decorator.is_decorator());
    }

    #[test]
    fn test_namespace_detection() {
        assert!(TypeScriptToken::Module.is_namespace());
        assert!(TypeScriptToken::InternalModule.is_namespace());
    }

    #[test]
    fn test_ambient_detection() {
        assert!(TypeScriptToken::Declare.is_ambient());
        assert!(TypeScriptToken::AmbientDeclaration.is_ambient());
    }

    #[test]
    fn test_type_annotation_detection() {
        assert!(TypeScriptToken::TypeAnnotation.is_type_annotation());
        assert!(TypeScriptToken::TypeAssertion.is_type_annotation());
        assert!(TypeScriptToken::AsExpression.is_type_annotation());
        assert!(TypeScriptToken::SatisfiesExpression.is_type_annotation());
    }

    #[test]
    fn test_type_construct_detection() {
        assert!(TypeScriptToken::InterfaceDeclaration.is_type_construct());
        assert!(TypeScriptToken::TypeAliasDeclaration.is_type_construct());
        assert!(TypeScriptToken::UnionType.is_type_construct());
        assert!(TypeScriptToken::IntersectionType.is_type_construct());
        assert!(TypeScriptToken::ConditionalType.is_type_construct());
    }

    #[test]
    fn test_advanced_type_detection() {
        assert!(TypeScriptToken::MappedTypeClause.is_mapped_type());
        assert!(TypeScriptToken::TemplateLiteralType.is_template_literal_type());
        assert!(TypeScriptToken::TypePredicate.is_type_predicate());
        assert!(TypeScriptToken::IndexTypeQuery.is_index_type());
        assert!(TypeScriptToken::LookupType.is_index_type());
    }

    #[test]
    fn test_language_info() {
        assert_eq!(TypeScriptLanguage::get_lang(), Lang::TypeScript);
        assert_eq!(TypeScriptLanguage::get_lang_name(), "typescript");
    }

    #[test]
    fn test_equality_with_u16() {
        let token = TypeScriptToken::FunctionDeclaration;
        assert_eq!(token, 224u16);
        assert_eq!(224u16, token);
    }

    #[test]
    fn test_declaration_detection() {
        assert!(TypeScriptToken::FunctionDeclaration.is_declaration());
        assert!(TypeScriptToken::ClassDeclaration.is_declaration());
        assert!(TypeScriptToken::InterfaceDeclaration.is_declaration());
        assert!(TypeScriptToken::TypeAliasDeclaration.is_declaration());
        assert!(TypeScriptToken::EnumDeclaration.is_declaration());
    }

    #[test]
    fn test_non_null_assertion() {
        assert!(TypeScriptToken::NonNullExpression.is_non_null_assertion());
    }
}
