//! Python language parser implementation.
//!
//! This module provides comprehensive Python language support including:
//! - Decorator detection (@decorator)
//! - Class/function definition patterns
//! - Indentation-based scope analysis
//! - Async function handling
//! - List/dict/set comprehensions
//! - Context managers (with statements)
//! - Lambda expressions
//! - Generator expressions
//! - Keyword arguments
//! - Type hints (Python 3+)
//! - F-strings and string formatting
//! - All Python operators and literals

use num_derive::FromPrimitive;
use crate::lang::Lang;
use crate::traits::LanguageInfo;

/// Python language token types.
///
/// This enum represents all possible node types in the Python tree-sitter grammar.
/// Each variant corresponds to a specific Python language construct, from basic
/// tokens like identifiers and operators to complex structures like comprehensions
/// and decorated definitions.
#[derive(Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum Python {
    // Basic tokens (0-107)
    End = 0,
    Identifier = 1,
    SEMI = 2,
    Import = 3,
    DOT = 4,
    From = 5,
    Future = 6,
    LPAREN = 7,
    RPAREN = 8,
    COMMA = 9,
    As = 10,
    STAR = 11,
    Print = 12,
    GTGT = 13,
    Assert = 14,
    COLONEQ = 15,
    Return = 16,
    Del = 17,
    Raise = 18,
    Pass = 19,
    Break = 20,
    Continue = 21,
    If = 22,
    COLON = 23,
    Elif = 24,
    Else = 25,
    Match = 26,
    Case = 27,
    Async = 28,
    For = 29,
    In = 30,
    While = 31,
    Try = 32,
    Except = 33,
    ExceptSTAR = 34,
    Finally = 35,
    With = 36,
    Def = 37,
    DASHGT = 38,
    STARSTAR = 39,
    Global = 40,
    Nonlocal = 41,
    Exec = 42,
    Type2 = 43,
    EQ = 44,
    Class = 45,
    LBRACK = 46,
    RBRACK = 47,
    AT = 48,
    DASH = 49,
    UNDERSCORE = 50,
    PIPE = 51,
    LBRACE = 52,
    RBRACE = 53,
    PLUS = 54,
    Not = 55,
    And = 56,
    Or = 57,
    SLASH = 58,
    PERCENT = 59,
    SLASHSLASH = 60,
    AMP = 61,
    CARET = 62,
    LTLT = 63,
    TILDE = 64,
    Is = 65,
    LT = 66,
    LTEQ = 67,
    EQEQ = 68,
    BANGEQ = 69,
    GTEQ = 70,
    GT = 71,
    LTGT = 72,
    Lambda3 = 73,
    PLUSEQ = 74,
    DASHEQ = 75,
    STAREQ = 76,
    SLASHEQ = 77,
    ATEQ = 78,
    SLASHSLASHEQ = 79,
    PERCENTEQ = 80,
    STARSTAREQ = 81,
    GTGTEQ = 82,
    LTLTEQ = 83,
    AMPEQ = 84,
    CARETEQ = 85,
    PIPEEQ = 86,
    Yield2 = 87,
    Ellipsis = 88,
    EscapeSequence = 89,
    BSLASH = 90,
    FormatSpecifierToken1 = 91,
    TypeConversion = 92,
    Integer = 93,
    Float = 94,
    Await2 = 95,
    True = 96,
    False = 97,
    None = 98,
    Comment = 99,
    LineContinuation = 100,
    Newline = 101,
    Indent = 102,
    Dedent = 103,
    StringStart = 104,
    StringContent2 = 105,
    EscapeInterpolation = 106,
    StringEnd = 107,

    // Structure nodes (108-240)
    Module = 108,
    Statement = 109,
    SimpleStatements = 110,
    ImportStatement = 111,
    ImportPrefix = 112,
    RelativeImport = 113,
    FutureImportStatement = 114,
    ImportFromStatement = 115,
    ImportList = 116,
    AliasedImport = 117,
    WildcardImport = 118,
    PrintStatement = 119,
    Chevron = 120,
    AssertStatement = 121,
    ExpressionStatement = 122,
    NamedExpression = 123,
    NamedExpressionLhs = 124,
    ReturnStatement = 125,
    DeleteStatement = 126,
    RaiseStatement = 127,
    PassStatement = 128,
    BreakStatement = 129,
    ContinueStatement = 130,
    IfStatement = 131,
    ElifClause = 132,
    ElseClause = 133,
    MatchStatement = 134,
    Block = 135,
    CaseClause = 136,
    ForStatement = 137,
    WhileStatement = 138,
    TryStatement = 139,
    ExceptClause = 140,
    ExceptGroupClause = 141,
    FinallyClause = 142,
    WithStatement = 143,
    WithClause = 144,
    WithItem = 145,
    FunctionDefinition = 146,
    Parameters = 147,
    LambdaParameters = 148,
    ListSplat = 149,
    DictionarySplat = 150,
    GlobalStatement = 151,
    NonlocalStatement = 152,
    ExecStatement = 153,
    TypeAliasStatement = 154,
    ClassDefinition = 155,
    TypeParameter = 156,
    ParenthesizedListSplat = 157,
    ArgumentList = 158,
    DecoratedDefinition = 159,
    Decorator = 160,
    Block2 = 161,
    ExpressionList = 162,
    DottedName = 163,
    CasePattern = 164,
    SimplePattern = 165,
    AsPattern = 166,
    UnionPattern = 167,
    ListPattern = 168,
    TuplePattern = 169,
    DictPattern = 170,
    KeyValuePattern = 171,
    KeywordPattern = 172,
    SplatPattern = 173,
    ClassPattern = 174,
    ComplexPattern = 175,
    Parameters2 = 176,
    Patterns = 177,
    Parameter = 178,
    Pattern = 179,
    TuplePattern2 = 180,
    ListPattern2 = 181,
    DefaultParameter = 182,
    TypedDefaultParameter = 183,
    ListSplatPattern = 184,
    DictionarySplatPattern = 185,
    AsPattern2 = 186,
    ExpressionWithinForInClause = 187,
    Expression = 188,
    PrimaryExpression = 189,
    NotOperator = 190,
    BooleanOperator = 191,
    BinaryOperator = 192,
    UnaryOperator = 193,
    Notin = 194,
    Isnot = 195,
    ComparisonOperator = 196,
    Lambda = 197,
    Lambda2 = 198,
    Assignment = 199,
    AugmentedAssignment = 200,
    PatternList = 201,
    RightHandSide = 202,
    Yield = 203,
    Attribute = 204,
    Subscript = 205,
    Slice = 206,
    Call = 207,
    TypedParameter = 208,
    Type = 209,
    SplatType = 210,
    GenericType = 211,
    UnionType = 212,
    ConstrainedType = 213,
    MemberType = 214,
    KeywordArgument = 215,
    List = 216,
    Set = 217,
    Tuple = 218,
    Dictionary = 219,
    Pair = 220,
    ListComprehension = 221,
    DictionaryComprehension = 222,
    SetComprehension = 223,
    GeneratorExpression = 224,
    ComprehensionClauses = 225,
    ParenthesizedExpression = 226,
    CollectionElements = 227,
    ForInClause = 228,
    IfClause = 229,
    ConditionalExpression = 230,
    ConcatenatedString = 231,
    String = 232,
    StringContent = 233,
    Interpolation = 234,
    FExpression = 235,
    NotEscapeSequence = 236,
    FormatSpecifier = 237,
    Await = 238,
    PositionalSeparator = 239,
    KeywordSeparator = 240,

    // Repeat nodes (241-272)
    ModuleRepeat1 = 241,
    SimpleStatementsRepeat1 = 242,
    ImportPrefixRepeat1 = 243,
    ImportListRepeat1 = 244,
    PrintStatementRepeat1 = 245,
    AssertStatementRepeat1 = 246,
    IfStatementRepeat1 = 247,
    MatchStatementRepeat1 = 248,
    MatchBlockRepeat1 = 249,
    CaseClauseRepeat1 = 250,
    TryStatementRepeat1 = 251,
    TryStatementRepeat2 = 252,
    WithClauseRepeat1 = 253,
    GlobalStatementRepeat1 = 254,
    TypeParameterRepeat1 = 255,
    ArgumentListRepeat1 = 256,
    DecoratedDefinitionRepeat1 = 257,
    DottedNameRepeat1 = 258,
    UnionPatternRepeat1 = 259,
    DictPatternRepeat1 = 260,
    ParametersRepeat1 = 261,
    PatternsRepeat1 = 262,
    ComparisonOperatorRepeat1 = 263,
    SubscriptRepeat1 = 264,
    DictionaryRepeat1 = 265,
    ComprehensionClausesRepeat1 = 266,
    CollectionElementsRepeat1 = 267,
    ForInClauseRepeat1 = 268,
    ConcatenatedStringRepeat1 = 269,
    StringRepeat1 = 270,
    StringContentRepeat1 = 271,
    FormatSpecifierRepeat1 = 272,

    // Additional nodes (273-275)
    AsPatternTarget = 273,
    FormatExpression = 274,
    Error = 275,
}

impl From<Python> for &'static str {
    #[inline(always)]
    fn from(tok: Python) -> Self {
        match tok {
            Python::End => "end",
            Python::Identifier => "identifier",
            Python::SEMI => ";",
            Python::Import => "import",
            Python::DOT => ".",
            Python::From => "from",
            Python::Future => "__future__",
            Python::LPAREN => "(",
            Python::RPAREN => ")",
            Python::COMMA => ",",
            Python::As => "as",
            Python::STAR => "*",
            Python::Print => "print",
            Python::GTGT => ">>",
            Python::Assert => "assert",
            Python::COLONEQ => ":=",
            Python::Return => "return",
            Python::Del => "del",
            Python::Raise => "raise",
            Python::Pass => "pass",
            Python::Break => "break",
            Python::Continue => "continue",
            Python::If => "if",
            Python::COLON => ":",
            Python::Elif => "elif",
            Python::Else => "else",
            Python::Match => "match",
            Python::Case => "case",
            Python::Async => "async",
            Python::For => "for",
            Python::In => "in",
            Python::While => "while",
            Python::Try => "try",
            Python::Except => "except",
            Python::ExceptSTAR => "except*",
            Python::Finally => "finally",
            Python::With => "with",
            Python::Def => "def",
            Python::DASHGT => "->",
            Python::STARSTAR => "**",
            Python::Global => "global",
            Python::Nonlocal => "nonlocal",
            Python::Exec => "exec",
            Python::Type2 => "type",
            Python::EQ => "=",
            Python::Class => "class",
            Python::LBRACK => "[",
            Python::RBRACK => "]",
            Python::AT => "@",
            Python::DASH => "-",
            Python::UNDERSCORE => "_",
            Python::PIPE => "|",
            Python::LBRACE => "{",
            Python::RBRACE => "}",
            Python::PLUS => "+",
            Python::Not => "not",
            Python::And => "and",
            Python::Or => "or",
            Python::SLASH => "/",
            Python::PERCENT => "%",
            Python::SLASHSLASH => "//",
            Python::AMP => "&",
            Python::CARET => "^",
            Python::LTLT => "<<",
            Python::TILDE => "~",
            Python::Is => "is",
            Python::LT => "<",
            Python::LTEQ => "<=",
            Python::EQEQ => "==",
            Python::BANGEQ => "!=",
            Python::GTEQ => ">=",
            Python::GT => ">",
            Python::LTGT => "<>",
            Python::Lambda3 => "lambda",
            Python::PLUSEQ => "+=",
            Python::DASHEQ => "-=",
            Python::STAREQ => "*=",
            Python::SLASHEQ => "/=",
            Python::ATEQ => "@=",
            Python::SLASHSLASHEQ => "//=",
            Python::PERCENTEQ => "%=",
            Python::STARSTAREQ => "**=",
            Python::GTGTEQ => ">>=",
            Python::LTLTEQ => "<<=",
            Python::AMPEQ => "&=",
            Python::CARETEQ => "^=",
            Python::PIPEEQ => "|=",
            Python::Yield2 => "yield",
            Python::Ellipsis => "ellipsis",
            Python::EscapeSequence => "escape_sequence",
            Python::BSLASH => "\\",
            Python::FormatSpecifierToken1 => "format_specifier_token1",
            Python::TypeConversion => "type_conversion",
            Python::Integer => "integer",
            Python::Float => "float",
            Python::Await2 => "await",
            Python::True => "true",
            Python::False => "false",
            Python::None => "none",
            Python::Comment => "comment",
            Python::LineContinuation => "line_continuation",
            Python::Newline => "_newline",
            Python::Indent => "_indent",
            Python::Dedent => "_dedent",
            Python::StringStart => "string_start",
            Python::StringContent2 => "_string_content",
            Python::EscapeInterpolation => "escape_interpolation",
            Python::StringEnd => "string_end",
            Python::Module => "module",
            Python::Statement => "_statement",
            Python::SimpleStatements => "_simple_statements",
            Python::ImportStatement => "import_statement",
            Python::ImportPrefix => "import_prefix",
            Python::RelativeImport => "relative_import",
            Python::FutureImportStatement => "future_import_statement",
            Python::ImportFromStatement => "import_from_statement",
            Python::ImportList => "_import_list",
            Python::AliasedImport => "aliased_import",
            Python::WildcardImport => "wildcard_import",
            Python::PrintStatement => "print_statement",
            Python::Chevron => "chevron",
            Python::AssertStatement => "assert_statement",
            Python::ExpressionStatement => "expression_statement",
            Python::NamedExpression => "named_expression",
            Python::NamedExpressionLhs => "_named_expression_lhs",
            Python::ReturnStatement => "return_statement",
            Python::DeleteStatement => "delete_statement",
            Python::RaiseStatement => "raise_statement",
            Python::PassStatement => "pass_statement",
            Python::BreakStatement => "break_statement",
            Python::ContinueStatement => "continue_statement",
            Python::IfStatement => "if_statement",
            Python::ElifClause => "elif_clause",
            Python::ElseClause => "else_clause",
            Python::MatchStatement => "match_statement",
            Python::Block => "block",
            Python::CaseClause => "case_clause",
            Python::ForStatement => "for_statement",
            Python::WhileStatement => "while_statement",
            Python::TryStatement => "try_statement",
            Python::ExceptClause => "except_clause",
            Python::ExceptGroupClause => "except_group_clause",
            Python::FinallyClause => "finally_clause",
            Python::WithStatement => "with_statement",
            Python::WithClause => "with_clause",
            Python::WithItem => "with_item",
            Python::FunctionDefinition => "function_definition",
            Python::Parameters => "parameters",
            Python::LambdaParameters => "lambda_parameters",
            Python::ListSplat => "list_splat",
            Python::DictionarySplat => "dictionary_splat",
            Python::GlobalStatement => "global_statement",
            Python::NonlocalStatement => "nonlocal_statement",
            Python::ExecStatement => "exec_statement",
            Python::TypeAliasStatement => "type_alias_statement",
            Python::ClassDefinition => "class_definition",
            Python::TypeParameter => "type_parameter",
            Python::ParenthesizedListSplat => "parenthesized_list_splat",
            Python::ArgumentList => "argument_list",
            Python::DecoratedDefinition => "decorated_definition",
            Python::Decorator => "decorator",
            Python::Block2 => "block",
            Python::ExpressionList => "expression_list",
            Python::DottedName => "dotted_name",
            Python::CasePattern => "case_pattern",
            Python::SimplePattern => "_simple_pattern",
            Python::AsPattern => "as_pattern",
            Python::UnionPattern => "union_pattern",
            Python::ListPattern => "list_pattern",
            Python::TuplePattern => "tuple_pattern",
            Python::DictPattern => "dict_pattern",
            Python::KeyValuePattern => "_key_value_pattern",
            Python::KeywordPattern => "keyword_pattern",
            Python::SplatPattern => "splat_pattern",
            Python::ClassPattern => "class_pattern",
            Python::ComplexPattern => "complex_pattern",
            Python::Parameters2 => "_parameters",
            Python::Patterns => "_patterns",
            Python::Parameter => "parameter",
            Python::Pattern => "pattern",
            Python::TuplePattern2 => "tuple_pattern",
            Python::ListPattern2 => "list_pattern",
            Python::DefaultParameter => "default_parameter",
            Python::TypedDefaultParameter => "typed_default_parameter",
            Python::ListSplatPattern => "list_splat_pattern",
            Python::DictionarySplatPattern => "dictionary_splat_pattern",
            Python::AsPattern2 => "as_pattern",
            Python::ExpressionWithinForInClause => "_expression_within_for_in_clause",
            Python::Expression => "expression",
            Python::PrimaryExpression => "primary_expression",
            Python::NotOperator => "not_operator",
            Python::BooleanOperator => "boolean_operator",
            Python::BinaryOperator => "binary_operator",
            Python::UnaryOperator => "unary_operator",
            Python::Notin => "not in",
            Python::Isnot => "is not",
            Python::ComparisonOperator => "comparison_operator",
            Python::Lambda => "lambda",
            Python::Lambda2 => "lambda",
            Python::Assignment => "assignment",
            Python::AugmentedAssignment => "augmented_assignment",
            Python::PatternList => "pattern_list",
            Python::RightHandSide => "_right_hand_side",
            Python::Yield => "yield",
            Python::Attribute => "attribute",
            Python::Subscript => "subscript",
            Python::Slice => "slice",
            Python::Call => "call",
            Python::TypedParameter => "typed_parameter",
            Python::Type => "type",
            Python::SplatType => "splat_type",
            Python::GenericType => "generic_type",
            Python::UnionType => "union_type",
            Python::ConstrainedType => "constrained_type",
            Python::MemberType => "member_type",
            Python::KeywordArgument => "keyword_argument",
            Python::List => "list",
            Python::Set => "set",
            Python::Tuple => "tuple",
            Python::Dictionary => "dictionary",
            Python::Pair => "pair",
            Python::ListComprehension => "list_comprehension",
            Python::DictionaryComprehension => "dictionary_comprehension",
            Python::SetComprehension => "set_comprehension",
            Python::GeneratorExpression => "generator_expression",
            Python::ComprehensionClauses => "_comprehension_clauses",
            Python::ParenthesizedExpression => "parenthesized_expression",
            Python::CollectionElements => "_collection_elements",
            Python::ForInClause => "for_in_clause",
            Python::IfClause => "if_clause",
            Python::ConditionalExpression => "conditional_expression",
            Python::ConcatenatedString => "concatenated_string",
            Python::String => "string",
            Python::StringContent => "string_content",
            Python::Interpolation => "interpolation",
            Python::FExpression => "_f_expression",
            Python::NotEscapeSequence => "_not_escape_sequence",
            Python::FormatSpecifier => "format_specifier",
            Python::Await => "await",
            Python::PositionalSeparator => "positional_separator",
            Python::KeywordSeparator => "keyword_separator",
            Python::ModuleRepeat1 => "module_repeat1",
            Python::SimpleStatementsRepeat1 => "_simple_statements_repeat1",
            Python::ImportPrefixRepeat1 => "import_prefix_repeat1",
            Python::ImportListRepeat1 => "_import_list_repeat1",
            Python::PrintStatementRepeat1 => "print_statement_repeat1",
            Python::AssertStatementRepeat1 => "assert_statement_repeat1",
            Python::IfStatementRepeat1 => "if_statement_repeat1",
            Python::MatchStatementRepeat1 => "match_statement_repeat1",
            Python::MatchBlockRepeat1 => "_match_block_repeat1",
            Python::CaseClauseRepeat1 => "case_clause_repeat1",
            Python::TryStatementRepeat1 => "try_statement_repeat1",
            Python::TryStatementRepeat2 => "try_statement_repeat2",
            Python::WithClauseRepeat1 => "with_clause_repeat1",
            Python::GlobalStatementRepeat1 => "global_statement_repeat1",
            Python::TypeParameterRepeat1 => "type_parameter_repeat1",
            Python::ArgumentListRepeat1 => "argument_list_repeat1",
            Python::DecoratedDefinitionRepeat1 => "decorated_definition_repeat1",
            Python::DottedNameRepeat1 => "dotted_name_repeat1",
            Python::UnionPatternRepeat1 => "union_pattern_repeat1",
            Python::DictPatternRepeat1 => "dict_pattern_repeat1",
            Python::ParametersRepeat1 => "_parameters_repeat1",
            Python::PatternsRepeat1 => "_patterns_repeat1",
            Python::ComparisonOperatorRepeat1 => "comparison_operator_repeat1",
            Python::SubscriptRepeat1 => "subscript_repeat1",
            Python::DictionaryRepeat1 => "dictionary_repeat1",
            Python::ComprehensionClausesRepeat1 => "_comprehension_clauses_repeat1",
            Python::CollectionElementsRepeat1 => "_collection_elements_repeat1",
            Python::ForInClauseRepeat1 => "for_in_clause_repeat1",
            Python::ConcatenatedStringRepeat1 => "concatenated_string_repeat1",
            Python::StringRepeat1 => "string_repeat1",
            Python::StringContentRepeat1 => "string_content_repeat1",
            Python::FormatSpecifierRepeat1 => "format_specifier_repeat1",
            Python::AsPatternTarget => "as_pattern_target",
            Python::FormatExpression => "format_expression",
            Python::Error => "ERROR",
        }
    }
}

impl From<u16> for Python {
    #[inline(always)]
    fn from(x: u16) -> Self {
        num::FromPrimitive::from_u16(x).unwrap_or(Self::Error)
    }
}

/// Python == u16 comparison
impl PartialEq<u16> for Python {
    #[inline(always)]
    fn eq(&self, x: &u16) -> bool {
        *self == Into::<Self>::into(*x)
    }
}

/// u16 == Python comparison
impl PartialEq<Python> for u16 {
    #[inline(always)]
    fn eq(&self, x: &Python) -> bool {
        *x == *self
    }
}

impl Python {
    /// Check if this token represents a function-like definition.
    ///
    /// Returns true for function definitions, lambda expressions, and async functions.
    #[inline]
    pub fn is_function(&self) -> bool {
        matches!(
            self,
            Python::FunctionDefinition | Python::Lambda | Python::Lambda2 | Python::Lambda3
        )
    }

    /// Check if this token represents a class definition.
    #[inline]
    pub fn is_class(&self) -> bool {
        matches!(self, Python::ClassDefinition)
    }

    /// Check if this token represents a decorator.
    ///
    /// Decorators are Python's mechanism for modifying functions and classes,
    /// denoted by the @ symbol.
    #[inline]
    pub fn is_decorator(&self) -> bool {
        matches!(
            self,
            Python::Decorator | Python::DecoratedDefinition | Python::AT
        )
    }

    /// Check if this token represents an async construct.
    ///
    /// Returns true for async/await keywords and async function definitions.
    #[inline]
    pub fn is_async(&self) -> bool {
        matches!(self, Python::Async | Python::Await | Python::Await2)
    }

    /// Check if this token represents a comprehension.
    ///
    /// Python comprehensions provide concise ways to create collections.
    /// Includes list, dict, set comprehensions and generator expressions.
    #[inline]
    pub fn is_comprehension(&self) -> bool {
        matches!(
            self,
            Python::ListComprehension
                | Python::DictionaryComprehension
                | Python::SetComprehension
                | Python::GeneratorExpression
        )
    }

    /// Check if this token represents a context manager (with statement).
    ///
    /// Context managers handle resource management through __enter__ and __exit__ methods.
    #[inline]
    pub fn is_context_manager(&self) -> bool {
        matches!(
            self,
            Python::WithStatement | Python::WithClause | Python::WithItem
        )
    }

    /// Check if this token represents a type hint or annotation.
    ///
    /// Type hints provide optional static typing in Python 3+.
    #[inline]
    pub fn is_type_annotation(&self) -> bool {
        matches!(
            self,
            Python::Type
                | Python::TypedParameter
                | Python::TypedDefaultParameter
                | Python::GenericType
                | Python::UnionType
                | Python::SplatType
                | Python::ConstrainedType
                | Python::MemberType
                | Python::TypeAliasStatement
                | Python::TypeParameter
        )
    }

    /// Check if this token represents a string or string-related construct.
    ///
    /// Includes regular strings, f-strings, string formatting, and concatenation.
    #[inline]
    pub fn is_string(&self) -> bool {
        matches!(
            self,
            Python::String
                | Python::StringStart
                | Python::StringEnd
                | Python::StringContent
                | Python::StringContent2
                | Python::ConcatenatedString
                | Python::FExpression
                | Python::Interpolation
                | Python::FormatSpecifier
        )
    }

    /// Check if this token represents an operator.
    ///
    /// Includes arithmetic, comparison, logical, and bitwise operators.
    #[inline]
    pub fn is_operator(&self) -> bool {
        matches!(
            self,
            Python::PLUS
                | Python::DASH
                | Python::STAR
                | Python::SLASH
                | Python::PERCENT
                | Python::STARSTAR
                | Python::SLASHSLASH
                | Python::AMP
                | Python::PIPE
                | Python::CARET
                | Python::TILDE
                | Python::LTLT
                | Python::GTGT
                | Python::LT
                | Python::GT
                | Python::LTEQ
                | Python::GTEQ
                | Python::EQEQ
                | Python::BANGEQ
                | Python::LTGT
                | Python::And
                | Python::Or
                | Python::Not
                | Python::Is
                | Python::In
                | Python::BinaryOperator
                | Python::UnaryOperator
                | Python::BooleanOperator
                | Python::ComparisonOperator
                | Python::NotOperator
        )
    }

    /// Check if this token represents an assignment operator.
    ///
    /// Includes basic assignment and all augmented assignment operators.
    #[inline]
    pub fn is_assignment(&self) -> bool {
        matches!(
            self,
            Python::EQ
                | Python::PLUSEQ
                | Python::DASHEQ
                | Python::STAREQ
                | Python::SLASHEQ
                | Python::PERCENTEQ
                | Python::STARSTAREQ
                | Python::SLASHSLASHEQ
                | Python::AMPEQ
                | Python::PIPEEQ
                | Python::CARETEQ
                | Python::LTLTEQ
                | Python::GTGTEQ
                | Python::ATEQ
                | Python::COLONEQ
                | Python::Assignment
                | Python::AugmentedAssignment
        )
    }

    /// Check if this token represents a control flow statement.
    ///
    /// Includes if/elif/else, for, while, try/except/finally, match/case.
    #[inline]
    pub fn is_control_flow(&self) -> bool {
        matches!(
            self,
            Python::If
                | Python::Elif
                | Python::Else
                | Python::For
                | Python::While
                | Python::Try
                | Python::Except
                | Python::Finally
                | Python::Match
                | Python::Case
                | Python::IfStatement
                | Python::ForStatement
                | Python::WhileStatement
                | Python::TryStatement
                | Python::MatchStatement
        )
    }

    /// Check if this token represents a collection literal.
    ///
    /// Includes lists, tuples, sets, and dictionaries.
    #[inline]
    pub fn is_collection(&self) -> bool {
        matches!(
            self,
            Python::List | Python::Tuple | Python::Set | Python::Dictionary
        )
    }

    /// Check if this token represents an import statement.
    #[inline]
    pub fn is_import(&self) -> bool {
        matches!(
            self,
            Python::Import
                | Python::ImportStatement
                | Python::ImportFromStatement
                | Python::FutureImportStatement
        )
    }

    /// Check if this token represents indentation.
    ///
    /// Python uses indentation for block structure instead of braces.
    #[inline]
    pub fn is_indent(&self) -> bool {
        matches!(self, Python::Indent | Python::Dedent)
    }

    /// Check if this token represents a pattern (for pattern matching).
    ///
    /// Python 3.10+ introduced structural pattern matching.
    #[inline]
    pub fn is_pattern(&self) -> bool {
        matches!(
            self,
            Python::Pattern
                | Python::CasePattern
                | Python::SimplePattern
                | Python::AsPattern
                | Python::AsPattern2
                | Python::UnionPattern
                | Python::ListPattern
                | Python::ListPattern2
                | Python::TuplePattern
                | Python::TuplePattern2
                | Python::DictPattern
                | Python::ClassPattern
                | Python::SplatPattern
        )
    }
}

/// Python language implementation.
pub struct PythonLanguage;

impl LanguageInfo for PythonLanguage {
    fn get_lang() -> Lang {
        Lang::Python
    }

    fn get_lang_name() -> &'static str {
        "python"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_conversion() {
        assert_eq!(Python::from(0), Python::End);
        assert_eq!(Python::from(1), Python::Identifier);
        assert_eq!(Python::from(146), Python::FunctionDefinition);
        assert_eq!(Python::from(155), Python::ClassDefinition);
        assert_eq!(Python::from(999), Python::Error);
    }

    #[test]
    fn test_token_to_string() {
        let tok: &str = Python::FunctionDefinition.into();
        assert_eq!(tok, "function_definition");

        let tok: &str = Python::ClassDefinition.into();
        assert_eq!(tok, "class_definition");

        let tok: &str = Python::Decorator.into();
        assert_eq!(tok, "decorator");
    }

    #[test]
    fn test_is_function() {
        assert!(Python::FunctionDefinition.is_function());
        assert!(Python::Lambda.is_function());
        assert!(Python::Lambda2.is_function());
        assert!(!Python::ClassDefinition.is_function());
    }

    #[test]
    fn test_is_class() {
        assert!(Python::ClassDefinition.is_class());
        assert!(!Python::FunctionDefinition.is_class());
    }

    #[test]
    fn test_is_decorator() {
        assert!(Python::Decorator.is_decorator());
        assert!(Python::DecoratedDefinition.is_decorator());
        assert!(Python::AT.is_decorator());
        assert!(!Python::FunctionDefinition.is_decorator());
    }

    #[test]
    fn test_is_async() {
        assert!(Python::Async.is_async());
        assert!(Python::Await.is_async());
        assert!(Python::Await2.is_async());
        assert!(!Python::Def.is_async());
    }

    #[test]
    fn test_is_comprehension() {
        assert!(Python::ListComprehension.is_comprehension());
        assert!(Python::DictionaryComprehension.is_comprehension());
        assert!(Python::SetComprehension.is_comprehension());
        assert!(Python::GeneratorExpression.is_comprehension());
        assert!(!Python::List.is_comprehension());
    }

    #[test]
    fn test_is_context_manager() {
        assert!(Python::WithStatement.is_context_manager());
        assert!(Python::WithClause.is_context_manager());
        assert!(Python::WithItem.is_context_manager());
        assert!(!Python::TryStatement.is_context_manager());
    }

    #[test]
    fn test_is_type_annotation() {
        assert!(Python::Type.is_type_annotation());
        assert!(Python::TypedParameter.is_type_annotation());
        assert!(Python::GenericType.is_type_annotation());
        assert!(Python::UnionType.is_type_annotation());
        assert!(!Python::Parameter.is_type_annotation());
    }

    #[test]
    fn test_is_string() {
        assert!(Python::String.is_string());
        assert!(Python::FExpression.is_string());
        assert!(Python::Interpolation.is_string());
        assert!(Python::ConcatenatedString.is_string());
        assert!(!Python::Integer.is_string());
    }

    #[test]
    fn test_is_operator() {
        assert!(Python::PLUS.is_operator());
        assert!(Python::DASH.is_operator());
        assert!(Python::And.is_operator());
        assert!(Python::Or.is_operator());
        assert!(Python::Not.is_operator());
        assert!(!Python::EQ.is_operator());
    }

    #[test]
    fn test_is_assignment() {
        assert!(Python::EQ.is_assignment());
        assert!(Python::PLUSEQ.is_assignment());
        assert!(Python::Assignment.is_assignment());
        assert!(Python::AugmentedAssignment.is_assignment());
        assert!(!Python::EQEQ.is_assignment());
    }

    #[test]
    fn test_is_control_flow() {
        assert!(Python::If.is_control_flow());
        assert!(Python::For.is_control_flow());
        assert!(Python::While.is_control_flow());
        assert!(Python::Try.is_control_flow());
        assert!(Python::Match.is_control_flow());
        assert!(!Python::FunctionDefinition.is_control_flow());
    }

    #[test]
    fn test_is_collection() {
        assert!(Python::List.is_collection());
        assert!(Python::Tuple.is_collection());
        assert!(Python::Set.is_collection());
        assert!(Python::Dictionary.is_collection());
        assert!(!Python::ListComprehension.is_collection());
    }

    #[test]
    fn test_is_import() {
        assert!(Python::Import.is_import());
        assert!(Python::ImportStatement.is_import());
        assert!(Python::ImportFromStatement.is_import());
        assert!(!Python::From.is_import());
    }

    #[test]
    fn test_is_indent() {
        assert!(Python::Indent.is_indent());
        assert!(Python::Dedent.is_indent());
        assert!(!Python::Newline.is_indent());
    }

    #[test]
    fn test_is_pattern() {
        assert!(Python::Pattern.is_pattern());
        assert!(Python::CasePattern.is_pattern());
        assert!(Python::AsPattern.is_pattern());
        assert!(Python::UnionPattern.is_pattern());
        assert!(!Python::Case.is_pattern());
    }

    #[test]
    fn test_u16_equality() {
        assert_eq!(Python::FunctionDefinition, 146u16);
        assert_eq!(146u16, Python::FunctionDefinition);
        assert_eq!(Python::ClassDefinition, 155u16);
        assert_eq!(155u16, Python::ClassDefinition);
    }

    #[test]
    fn test_language_info() {
        assert_eq!(PythonLanguage::get_lang(), Lang::Python);
        assert_eq!(PythonLanguage::get_lang_name(), "python");
    }
}
