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

    // ============================================================================
    // ADVANCED HELPER METHODS
    // ============================================================================

    // ------------------------------------------------------------------------
    // Decorators
    // ------------------------------------------------------------------------

    /// Check if this token is a decorator application.
    ///
    /// Decorators modify the behavior of functions and classes using the @ syntax.
    /// Example: `@property`, `@staticmethod`, `@dataclass`
    #[inline]
    pub fn is_decorator_application(&self) -> bool {
        matches!(self, Python::Decorator | Python::DecoratedDefinition)
    }

    /// Check if this token is a function decorator.
    ///
    /// Function decorators wrap and modify function behavior.
    /// Example: `@property`, `@lru_cache`, `@staticmethod`
    #[inline]
    pub fn is_function_decorator(&self) -> bool {
        matches!(self, Python::Decorator | Python::AT)
    }

    /// Check if this token is a class decorator.
    ///
    /// Class decorators modify class definitions.
    /// Example: `@dataclass`, `@singleton`, `@register`
    #[inline]
    pub fn is_class_decorator(&self) -> bool {
        matches!(self, Python::DecoratedDefinition)
    }

    // ------------------------------------------------------------------------
    // Type System
    // ------------------------------------------------------------------------

    /// Check if this token represents a generic type.
    ///
    /// Generic types enable parameterized types using square brackets.
    /// Example: `List[int]`, `Dict[str, Any]`, `Optional[T]`
    #[inline]
    pub fn is_generic_type(&self) -> bool {
        matches!(self, Python::GenericType)
    }

    /// Check if this token represents a union type.
    ///
    /// Union types represent multiple possible types.
    /// Example: `int | str`, `Union[int, str]`, `Optional[int]`
    #[inline]
    pub fn is_union_type(&self) -> bool {
        matches!(self, Python::UnionType | Python::PIPE)
    }

    /// Check if this token represents a type parameter.
    ///
    /// Type parameters define generic type variables.
    /// Example: `T`, `KT`, `VT` in `class MyClass[T]: ...`
    #[inline]
    pub fn is_type_parameter(&self) -> bool {
        matches!(self, Python::TypeParameter)
    }

    /// Check if this token represents a typed parameter.
    ///
    /// Typed parameters include type hints in function signatures.
    /// Example: `def foo(x: int, y: str) -> bool:`
    #[inline]
    pub fn is_typed_parameter(&self) -> bool {
        matches!(
            self,
            Python::TypedParameter | Python::TypedDefaultParameter
        )
    }

    /// Check if this token represents a type alias.
    ///
    /// Type aliases create new names for types.
    /// Example: `type Vector = list[float]`
    #[inline]
    pub fn is_type_alias(&self) -> bool {
        matches!(self, Python::TypeAliasStatement | Python::Type2)
    }

    /// Check if this token represents a splat type.
    ///
    /// Splat types represent variable-length argument types.
    /// Example: `*args: int`, `**kwargs: str`
    #[inline]
    pub fn is_splat_type(&self) -> bool {
        matches!(self, Python::SplatType)
    }

    /// Check if this token represents a constrained type.
    ///
    /// Constrained types limit type variables to specific types.
    /// Example: `TypeVar('T', int, str)`
    #[inline]
    pub fn is_constrained_type(&self) -> bool {
        matches!(self, Python::ConstrainedType)
    }

    /// Check if this token represents a member type.
    ///
    /// Member types access types from modules or classes.
    /// Example: `typing.List`, `collections.abc.Mapping`
    #[inline]
    pub fn is_member_type(&self) -> bool {
        matches!(self, Python::MemberType)
    }

    // ------------------------------------------------------------------------
    // Pattern Matching (Python 3.10+)
    // ------------------------------------------------------------------------

    /// Check if this token is a match statement.
    ///
    /// Match statements provide structural pattern matching.
    /// Example: `match value: case 1: ...`
    #[inline]
    pub fn is_match_statement(&self) -> bool {
        matches!(self, Python::MatchStatement | Python::Match)
    }

    /// Check if this token is a case clause.
    ///
    /// Case clauses define patterns in match statements.
    /// Example: `case [x, y]:`, `case {"key": value}:`
    #[inline]
    pub fn is_case_clause(&self) -> bool {
        matches!(self, Python::CaseClause | Python::Case)
    }

    /// Check if this token is a pattern guard.
    ///
    /// Pattern guards add conditions to case patterns.
    /// Example: `case x if x > 0:`
    #[inline]
    pub fn is_pattern_guard(&self) -> bool {
        matches!(self, Python::IfClause)
    }

    /// Check if this token is a union pattern.
    ///
    /// Union patterns match multiple alternatives.
    /// Example: `case 1 | 2 | 3:`
    #[inline]
    pub fn is_union_pattern(&self) -> bool {
        matches!(self, Python::UnionPattern)
    }

    /// Check if this token is an as pattern.
    ///
    /// As patterns capture matched values.
    /// Example: `case [x, y] as point:`
    #[inline]
    pub fn is_as_pattern(&self) -> bool {
        matches!(self, Python::AsPattern | Python::AsPattern2)
    }

    /// Check if this token is a list pattern.
    ///
    /// List patterns match list structures.
    /// Example: `case [first, *rest]:`
    #[inline]
    pub fn is_list_pattern(&self) -> bool {
        matches!(self, Python::ListPattern | Python::ListPattern2)
    }

    /// Check if this token is a tuple pattern.
    ///
    /// Tuple patterns match tuple structures.
    /// Example: `case (x, y):`
    #[inline]
    pub fn is_tuple_pattern(&self) -> bool {
        matches!(self, Python::TuplePattern | Python::TuplePattern2)
    }

    /// Check if this token is a dict pattern.
    ///
    /// Dict patterns match dictionary structures.
    /// Example: `case {"key": value}:`
    #[inline]
    pub fn is_dict_pattern(&self) -> bool {
        matches!(self, Python::DictPattern)
    }

    /// Check if this token is a class pattern.
    ///
    /// Class patterns match class instances.
    /// Example: `case Point(x=0, y=0):`
    #[inline]
    pub fn is_class_pattern(&self) -> bool {
        matches!(self, Python::ClassPattern)
    }

    /// Check if this token is a splat pattern.
    ///
    /// Splat patterns capture remaining elements.
    /// Example: `case [first, *rest, last]:`
    #[inline]
    pub fn is_splat_pattern(&self) -> bool {
        matches!(self, Python::SplatPattern)
    }

    // ------------------------------------------------------------------------
    // Comprehensions
    // ------------------------------------------------------------------------

    /// Check if this token is a list comprehension.
    ///
    /// List comprehensions create lists from iterables.
    /// Example: `[x**2 for x in range(10)]`
    #[inline]
    pub fn is_list_comprehension(&self) -> bool {
        matches!(self, Python::ListComprehension)
    }

    /// Check if this token is a dictionary comprehension.
    ///
    /// Dict comprehensions create dictionaries from iterables.
    /// Example: `{k: v for k, v in items}`
    #[inline]
    pub fn is_dict_comprehension(&self) -> bool {
        matches!(self, Python::DictionaryComprehension)
    }

    /// Check if this token is a set comprehension.
    ///
    /// Set comprehensions create sets from iterables.
    /// Example: `{x for x in items if x > 0}`
    #[inline]
    pub fn is_set_comprehension(&self) -> bool {
        matches!(self, Python::SetComprehension)
    }

    /// Check if this token is a generator expression.
    ///
    /// Generator expressions create lazy iterators.
    /// Example: `(x**2 for x in range(10))`
    #[inline]
    pub fn is_generator_expression(&self) -> bool {
        matches!(self, Python::GeneratorExpression)
    }

    /// Check if this token is a for-in clause (used in comprehensions).
    ///
    /// For-in clauses iterate over sequences in comprehensions.
    /// Example: `for x in items` in `[x for x in items]`
    #[inline]
    pub fn is_for_in_clause(&self) -> bool {
        matches!(self, Python::ForInClause)
    }

    /// Check if this token is an if clause (used in comprehensions).
    ///
    /// If clauses filter items in comprehensions.
    /// Example: `if x > 0` in `[x for x in items if x > 0]`
    #[inline]
    pub fn is_comprehension_if_clause(&self) -> bool {
        matches!(self, Python::IfClause)
    }

    // ------------------------------------------------------------------------
    // Context Managers
    // ------------------------------------------------------------------------

    /// Check if this token is a with statement.
    ///
    /// With statements manage resources using context managers.
    /// Example: `with open('file.txt') as f:`
    #[inline]
    pub fn is_with_statement(&self) -> bool {
        matches!(self, Python::WithStatement | Python::With)
    }

    /// Check if this token is a with clause.
    ///
    /// With clauses contain context manager expressions.
    /// Example: The `open('file.txt') as f` part in with statements
    #[inline]
    pub fn is_with_clause(&self) -> bool {
        matches!(self, Python::WithClause)
    }

    /// Check if this token is a with item.
    ///
    /// With items are individual context managers in with statements.
    /// Example: `open('file.txt') as f` in `with open('file.txt') as f:`
    #[inline]
    pub fn is_with_item(&self) -> bool {
        matches!(self, Python::WithItem)
    }

    /// Check if this token is an async with statement.
    ///
    /// Async with statements handle async context managers.
    /// Example: `async with aiofiles.open('file.txt') as f:`
    #[inline]
    pub fn is_async_with(&self) -> bool {
        matches!(self, Python::Async)
    }

    // ------------------------------------------------------------------------
    // Advanced Operators
    // ------------------------------------------------------------------------

    /// Check if this token is the walrus operator (:=).
    ///
    /// The walrus operator assigns values within expressions.
    /// Example: `if (n := len(items)) > 10:`
    #[inline]
    pub fn is_walrus_operator(&self) -> bool {
        matches!(self, Python::COLONEQ | Python::NamedExpression)
    }

    /// Check if this token is the matrix multiplication operator (@).
    ///
    /// The @ operator is used for matrix multiplication.
    /// Example: `A @ B` for matrix multiplication
    #[inline]
    pub fn is_matrix_multiply(&self) -> bool {
        matches!(self, Python::AT | Python::ATEQ)
    }

    /// Check if this token is the floor division operator (//).
    ///
    /// Floor division returns the integer quotient.
    /// Example: `7 // 2` returns 3
    #[inline]
    pub fn is_floor_division(&self) -> bool {
        matches!(self, Python::SLASHSLASH | Python::SLASHSLASHEQ)
    }

    /// Check if this token is the power operator (**).
    ///
    /// The power operator raises to exponents.
    /// Example: `2 ** 3` returns 8
    #[inline]
    pub fn is_power_operator(&self) -> bool {
        matches!(self, Python::STARSTAR | Python::STARSTAREQ)
    }

    /// Check if this token is a bitwise operator.
    ///
    /// Bitwise operators perform bit-level operations.
    /// Example: `&` (AND), `|` (OR), `^` (XOR), `~` (NOT), `<<` (shift left), `>>` (shift right)
    #[inline]
    pub fn is_bitwise_operator(&self) -> bool {
        matches!(
            self,
            Python::AMP
                | Python::PIPE
                | Python::CARET
                | Python::TILDE
                | Python::LTLT
                | Python::GTGT
                | Python::AMPEQ
                | Python::PIPEEQ
                | Python::CARETEQ
                | Python::LTLTEQ
                | Python::GTGTEQ
        )
    }

    /// Check if this token is a comparison operator.
    ///
    /// Comparison operators compare values.
    /// Example: `<`, `<=`, `>`, `>=`, `==`, `!=`, `is`, `in`
    #[inline]
    pub fn is_comparison_operator(&self) -> bool {
        matches!(
            self,
            Python::LT
                | Python::LTEQ
                | Python::GT
                | Python::GTEQ
                | Python::EQEQ
                | Python::BANGEQ
                | Python::LTGT
                | Python::Is
                | Python::In
                | Python::Isnot
                | Python::Notin
                | Python::ComparisonOperator
        )
    }

    /// Check if this token is a boolean operator.
    ///
    /// Boolean operators perform logical operations.
    /// Example: `and`, `or`, `not`
    #[inline]
    pub fn is_boolean_operator(&self) -> bool {
        matches!(
            self,
            Python::And | Python::Or | Python::Not | Python::BooleanOperator
        )
    }

    /// Check if this token is an augmented assignment operator.
    ///
    /// Augmented assignment combines an operation with assignment.
    /// Example: `+=`, `-=`, `*=`, `/=`, `//=`, `**=`, `%=`
    #[inline]
    pub fn is_augmented_assignment(&self) -> bool {
        matches!(
            self,
            Python::PLUSEQ
                | Python::DASHEQ
                | Python::STAREQ
                | Python::SLASHEQ
                | Python::SLASHSLASHEQ
                | Python::PERCENTEQ
                | Python::STARSTAREQ
                | Python::ATEQ
                | Python::AMPEQ
                | Python::PIPEEQ
                | Python::CARETEQ
                | Python::LTLTEQ
                | Python::GTGTEQ
                | Python::AugmentedAssignment
        )
    }

    // ------------------------------------------------------------------------
    // String Features
    // ------------------------------------------------------------------------

    /// Check if this token is an f-string (formatted string literal).
    ///
    /// F-strings embed expressions inside string literals.
    /// Example: `f"Hello {name}!"`, `f"Value: {x:.2f}"`
    #[inline]
    pub fn is_fstring(&self) -> bool {
        matches!(
            self,
            Python::FExpression | Python::Interpolation | Python::FormatExpression
        )
    }

    /// Check if this token is a format specifier.
    ///
    /// Format specifiers control formatting in f-strings.
    /// Example: `:.2f` in `f"{value:.2f}"`
    #[inline]
    pub fn is_format_specifier(&self) -> bool {
        matches!(
            self,
            Python::FormatSpecifier
                | Python::FormatSpecifierToken1
                | Python::TypeConversion
        )
    }

    /// Check if this token is a string interpolation.
    ///
    /// String interpolation embeds expressions in strings.
    /// Example: `{name}` in `f"Hello {name}!"`
    #[inline]
    pub fn is_string_interpolation(&self) -> bool {
        matches!(self, Python::Interpolation)
    }

    /// Check if this token is a concatenated string.
    ///
    /// Concatenated strings combine multiple string literals.
    /// Example: `"Hello" "World"` becomes `"HelloWorld"`
    #[inline]
    pub fn is_concatenated_string(&self) -> bool {
        matches!(self, Python::ConcatenatedString)
    }

    /// Check if this token is a string literal.
    ///
    /// String literals include regular, raw, byte, and f-strings.
    /// Example: `"text"`, `r"raw"`, `b"bytes"`, `f"formatted"`
    #[inline]
    pub fn is_string_literal(&self) -> bool {
        matches!(
            self,
            Python::String | Python::StringStart | Python::StringEnd
        )
    }

    /// Check if this token is string content.
    ///
    /// String content is the text inside string literals.
    #[inline]
    pub fn is_string_content(&self) -> bool {
        matches!(self, Python::StringContent | Python::StringContent2)
    }

    /// Check if this token is an escape sequence.
    ///
    /// Escape sequences represent special characters.
    /// Example: `\n`, `\t`, `\\`, `\"`
    #[inline]
    pub fn is_escape_sequence(&self) -> bool {
        matches!(
            self,
            Python::EscapeSequence | Python::EscapeInterpolation
        )
    }

    // ------------------------------------------------------------------------
    // Async Features
    // ------------------------------------------------------------------------

    /// Check if this token is an async function definition.
    ///
    /// Async functions can use await and are executed asynchronously.
    /// Example: `async def fetch_data():`
    #[inline]
    pub fn is_async_function(&self) -> bool {
        matches!(self, Python::Async)
    }

    /// Check if this token is an await expression.
    ///
    /// Await expressions wait for async operations to complete.
    /// Example: `await fetch_data()`
    #[inline]
    pub fn is_await_expression(&self) -> bool {
        matches!(self, Python::Await | Python::Await2)
    }

    /// Check if this token is an async for statement.
    ///
    /// Async for iterates over async iterators.
    /// Example: `async for item in async_iterator:`
    #[inline]
    pub fn is_async_for(&self) -> bool {
        matches!(self, Python::Async)
    }

    /// Check if this token relates to async operations.
    ///
    /// Includes async def, await, async for, async with.
    #[inline]
    pub fn is_async_operation(&self) -> bool {
        matches!(self, Python::Async | Python::Await | Python::Await2)
    }

    // ------------------------------------------------------------------------
    // Control Flow
    // ------------------------------------------------------------------------

    /// Check if this token is a try statement.
    ///
    /// Try statements handle exceptions.
    /// Example: `try: ... except ValueError: ...`
    #[inline]
    pub fn is_try_statement(&self) -> bool {
        matches!(self, Python::TryStatement | Python::Try)
    }

    /// Check if this token is an except clause.
    ///
    /// Except clauses catch and handle exceptions.
    /// Example: `except ValueError as e:`
    #[inline]
    pub fn is_except_clause(&self) -> bool {
        matches!(
            self,
            Python::ExceptClause | Python::Except | Python::ExceptGroupClause | Python::ExceptSTAR
        )
    }

    /// Check if this token is a finally clause.
    ///
    /// Finally clauses execute cleanup code regardless of exceptions.
    /// Example: `finally: file.close()`
    #[inline]
    pub fn is_finally_clause(&self) -> bool {
        matches!(self, Python::FinallyClause | Python::Finally)
    }

    /// Check if this token is an else clause.
    ///
    /// Else clauses execute when no exception occurs (in try) or condition is false (in if/for/while).
    /// Example: `else: print("Success")`
    #[inline]
    pub fn is_else_clause(&self) -> bool {
        matches!(self, Python::ElseClause | Python::Else)
    }

    /// Check if this token is a raise statement.
    ///
    /// Raise statements throw exceptions.
    /// Example: `raise ValueError("Invalid input")`
    #[inline]
    pub fn is_raise_statement(&self) -> bool {
        matches!(self, Python::RaiseStatement | Python::Raise)
    }

    /// Check if this token is an assert statement.
    ///
    /// Assert statements check conditions and raise AssertionError if false.
    /// Example: `assert x > 0, "x must be positive"`
    #[inline]
    pub fn is_assert_statement(&self) -> bool {
        matches!(self, Python::AssertStatement | Python::Assert)
    }

    /// Check if this token is a pass statement.
    ///
    /// Pass statements are null operations (placeholders).
    /// Example: `def foo(): pass`
    #[inline]
    pub fn is_pass_statement(&self) -> bool {
        matches!(self, Python::PassStatement | Python::Pass)
    }

    /// Check if this token is a break statement.
    ///
    /// Break statements exit loops early.
    /// Example: `if found: break`
    #[inline]
    pub fn is_break_statement(&self) -> bool {
        matches!(self, Python::BreakStatement | Python::Break)
    }

    /// Check if this token is a continue statement.
    ///
    /// Continue statements skip to the next loop iteration.
    /// Example: `if skip: continue`
    #[inline]
    pub fn is_continue_statement(&self) -> bool {
        matches!(self, Python::ContinueStatement | Python::Continue)
    }

    /// Check if this token is a return statement.
    ///
    /// Return statements exit functions and return values.
    /// Example: `return result`
    #[inline]
    pub fn is_return_statement(&self) -> bool {
        matches!(self, Python::ReturnStatement | Python::Return)
    }

    /// Check if this token is a yield statement.
    ///
    /// Yield statements produce values in generators.
    /// Example: `yield value`
    #[inline]
    pub fn is_yield_statement(&self) -> bool {
        matches!(self, Python::Yield | Python::Yield2)
    }

    /// Check if this token is an if statement.
    ///
    /// If statements conditionally execute code.
    /// Example: `if condition: ...`
    #[inline]
    pub fn is_if_statement(&self) -> bool {
        matches!(self, Python::IfStatement | Python::If)
    }

    /// Check if this token is an elif clause.
    ///
    /// Elif clauses provide alternative conditions in if statements.
    /// Example: `elif other_condition: ...`
    #[inline]
    pub fn is_elif_clause(&self) -> bool {
        matches!(self, Python::ElifClause | Python::Elif)
    }

    /// Check if this token is a for statement.
    ///
    /// For statements iterate over sequences.
    /// Example: `for item in items: ...`
    #[inline]
    pub fn is_for_statement(&self) -> bool {
        matches!(self, Python::ForStatement | Python::For)
    }

    /// Check if this token is a while statement.
    ///
    /// While statements loop while a condition is true.
    /// Example: `while condition: ...`
    #[inline]
    pub fn is_while_statement(&self) -> bool {
        matches!(self, Python::WhileStatement | Python::While)
    }

    /// Check if this token is a conditional expression (ternary operator).
    ///
    /// Conditional expressions inline if/else logic.
    /// Example: `x if condition else y`
    #[inline]
    pub fn is_conditional_expression(&self) -> bool {
        matches!(self, Python::ConditionalExpression)
    }

    // ------------------------------------------------------------------------
    // Special Methods (Dunder Methods)
    // ------------------------------------------------------------------------

    /// Check if this token could be a dunder method name.
    ///
    /// Dunder methods (double underscore) implement special behavior.
    /// Example: `__init__`, `__str__`, `__repr__`, `__eq__`, `__len__`
    /// Note: This checks for identifiers; actual validation requires string content.
    #[inline]
    pub fn is_potential_dunder_method(&self) -> bool {
        matches!(self, Python::Identifier | Python::FunctionDefinition)
    }

    // ------------------------------------------------------------------------
    // Module System
    // ------------------------------------------------------------------------

    /// Check if this token is an import statement.
    ///
    /// Import statements bring in modules.
    /// Example: `import os`, `import sys`
    #[inline]
    pub fn is_import_statement(&self) -> bool {
        matches!(self, Python::ImportStatement | Python::Import)
    }

    /// Check if this token is an import-from statement.
    ///
    /// From-import statements import specific items from modules.
    /// Example: `from os import path`, `from typing import List`
    #[inline]
    pub fn is_import_from_statement(&self) -> bool {
        matches!(self, Python::ImportFromStatement | Python::From)
    }

    /// Check if this token is a future import.
    ///
    /// Future imports enable upcoming Python features.
    /// Example: `from __future__ import annotations`
    #[inline]
    pub fn is_future_import(&self) -> bool {
        matches!(self, Python::FutureImportStatement | Python::Future)
    }

    /// Check if this token is an aliased import.
    ///
    /// Aliased imports rename imported items.
    /// Example: `import numpy as np`, `from foo import bar as baz`
    #[inline]
    pub fn is_aliased_import(&self) -> bool {
        matches!(self, Python::AliasedImport | Python::As)
    }

    /// Check if this token is a wildcard import.
    ///
    /// Wildcard imports import all items from a module.
    /// Example: `from module import *`
    #[inline]
    pub fn is_wildcard_import(&self) -> bool {
        matches!(self, Python::WildcardImport)
    }

    /// Check if this token is a relative import.
    ///
    /// Relative imports use dots to import from relative paths.
    /// Example: `from . import module`, `from .. import module`
    #[inline]
    pub fn is_relative_import(&self) -> bool {
        matches!(self, Python::RelativeImport | Python::ImportPrefix)
    }

    /// Check if this token is a dotted name (module path).
    ///
    /// Dotted names represent nested module/attribute access.
    /// Example: `os.path`, `typing.Optional`
    #[inline]
    pub fn is_dotted_name(&self) -> bool {
        matches!(self, Python::DottedName | Python::DOT)
    }

    // ------------------------------------------------------------------------
    // Class Features
    // ------------------------------------------------------------------------

    /// Check if this token is a class definition.
    ///
    /// Class definitions create new classes.
    /// Example: `class MyClass:`, `class Derived(Base):`
    #[inline]
    pub fn is_class_definition(&self) -> bool {
        matches!(self, Python::ClassDefinition | Python::Class)
    }

    /// Check if this token relates to class methods.
    ///
    /// Note: Detecting @classmethod/@staticmethod requires decorator analysis.
    #[inline]
    pub fn is_class_method_marker(&self) -> bool {
        matches!(self, Python::Decorator)
    }

    /// Check if this token is a property decorator marker.
    ///
    /// Properties provide getter/setter functionality.
    /// Example: `@property`, `@foo.setter`
    #[inline]
    pub fn is_property_marker(&self) -> bool {
        matches!(self, Python::Decorator)
    }

    // ------------------------------------------------------------------------
    // Functional Features
    // ------------------------------------------------------------------------

    /// Check if this token is a lambda expression.
    ///
    /// Lambda expressions create anonymous functions.
    /// Example: `lambda x: x * 2`, `lambda x, y: x + y`
    #[inline]
    pub fn is_lambda(&self) -> bool {
        matches!(self, Python::Lambda | Python::Lambda2 | Python::Lambda3)
    }

    /// Check if this token is a lambda parameter list.
    ///
    /// Lambda parameters define inputs to lambda functions.
    /// Example: `x, y` in `lambda x, y: x + y`
    #[inline]
    pub fn is_lambda_parameters(&self) -> bool {
        matches!(self, Python::LambdaParameters)
    }

    // ------------------------------------------------------------------------
    // Iterators & Generators
    // ------------------------------------------------------------------------

    /// Check if this token is a yield expression.
    ///
    /// Yield expressions produce values in generators.
    /// Example: `yield value`, `yield from iterator`
    #[inline]
    pub fn is_yield_expression(&self) -> bool {
        matches!(self, Python::Yield | Python::Yield2)
    }

    /// Check if this token is part of a generator.
    ///
    /// Generators are functions that yield values.
    #[inline]
    pub fn is_generator_related(&self) -> bool {
        matches!(
            self,
            Python::Yield | Python::Yield2 | Python::GeneratorExpression
        )
    }

    // ------------------------------------------------------------------------
    // Parameters & Arguments
    // ------------------------------------------------------------------------

    /// Check if this token is a parameter.
    ///
    /// Parameters define function inputs.
    /// Example: `x, y` in `def foo(x, y):`
    #[inline]
    pub fn is_parameter(&self) -> bool {
        matches!(
            self,
            Python::Parameter
                | Python::TypedParameter
                | Python::DefaultParameter
                | Python::TypedDefaultParameter
        )
    }

    /// Check if this token is a default parameter.
    ///
    /// Default parameters have default values.
    /// Example: `def foo(x=10):`
    #[inline]
    pub fn is_default_parameter(&self) -> bool {
        matches!(
            self,
            Python::DefaultParameter | Python::TypedDefaultParameter
        )
    }

    /// Check if this token is a list splat (*args).
    ///
    /// List splat captures variable positional arguments.
    /// Example: `*args` in `def foo(*args):`
    #[inline]
    pub fn is_list_splat(&self) -> bool {
        matches!(
            self,
            Python::ListSplat | Python::ListSplatPattern | Python::STAR
        )
    }

    /// Check if this token is a dictionary splat (**kwargs).
    ///
    /// Dictionary splat captures variable keyword arguments.
    /// Example: `**kwargs` in `def foo(**kwargs):`
    #[inline]
    pub fn is_dict_splat(&self) -> bool {
        matches!(
            self,
            Python::DictionarySplat | Python::DictionarySplatPattern | Python::STARSTAR
        )
    }

    /// Check if this token is a keyword argument.
    ///
    /// Keyword arguments are named arguments in function calls.
    /// Example: `foo(x=10, y=20)`
    #[inline]
    pub fn is_keyword_argument(&self) -> bool {
        matches!(self, Python::KeywordArgument)
    }

    /// Check if this token is a positional separator.
    ///
    /// Positional separators enforce positional-only parameters.
    /// Example: `/` in `def foo(x, y, /, z):`
    #[inline]
    pub fn is_positional_separator(&self) -> bool {
        matches!(self, Python::PositionalSeparator)
    }

    /// Check if this token is a keyword separator.
    ///
    /// Keyword separators enforce keyword-only parameters.
    /// Example: `*` in `def foo(x, *, y):`
    #[inline]
    pub fn is_keyword_separator(&self) -> bool {
        matches!(self, Python::KeywordSeparator)
    }

    /// Check if this token is an argument list.
    ///
    /// Argument lists contain function call arguments.
    /// Example: `(1, 2, x=3)` in `foo(1, 2, x=3)`
    #[inline]
    pub fn is_argument_list(&self) -> bool {
        matches!(self, Python::ArgumentList)
    }

    // ------------------------------------------------------------------------
    // Expressions
    // ------------------------------------------------------------------------

    /// Check if this token is an expression.
    ///
    /// Expressions compute values.
    #[inline]
    pub fn is_expression(&self) -> bool {
        matches!(
            self,
            Python::Expression
                | Python::PrimaryExpression
                | Python::BinaryOperator
                | Python::UnaryOperator
                | Python::ConditionalExpression
                | Python::NamedExpression
        )
    }

    /// Check if this token is a primary expression.
    ///
    /// Primary expressions are atomic values and operations.
    #[inline]
    pub fn is_primary_expression(&self) -> bool {
        matches!(self, Python::PrimaryExpression)
    }

    /// Check if this token is a binary operator expression.
    ///
    /// Binary operators take two operands.
    /// Example: `x + y`, `a * b`, `p and q`
    #[inline]
    pub fn is_binary_operator(&self) -> bool {
        matches!(self, Python::BinaryOperator)
    }

    /// Check if this token is a unary operator expression.
    ///
    /// Unary operators take one operand.
    /// Example: `-x`, `not p`, `~n`
    #[inline]
    pub fn is_unary_operator(&self) -> bool {
        matches!(self, Python::UnaryOperator | Python::NotOperator)
    }

    /// Check if this token is a named expression (walrus operator).
    ///
    /// Named expressions assign within expressions.
    /// Example: `if (n := len(data)) > 10:`
    #[inline]
    pub fn is_named_expression(&self) -> bool {
        matches!(self, Python::NamedExpression)
    }

    /// Check if this token is a call expression.
    ///
    /// Call expressions invoke functions.
    /// Example: `foo(1, 2)`, `obj.method(x=10)`
    #[inline]
    pub fn is_call_expression(&self) -> bool {
        matches!(self, Python::Call)
    }

    /// Check if this token is an attribute access.
    ///
    /// Attribute access retrieves object attributes.
    /// Example: `obj.attr`, `module.function`
    #[inline]
    pub fn is_attribute(&self) -> bool {
        matches!(self, Python::Attribute)
    }

    /// Check if this token is a subscript expression.
    ///
    /// Subscript expressions access collection items.
    /// Example: `list[0]`, `dict['key']`, `arr[1:5]`
    #[inline]
    pub fn is_subscript(&self) -> bool {
        matches!(self, Python::Subscript)
    }

    /// Check if this token is a slice expression.
    ///
    /// Slice expressions extract subsequences.
    /// Example: `[1:5]`, `[::2]`, `[::-1]`
    #[inline]
    pub fn is_slice(&self) -> bool {
        matches!(self, Python::Slice)
    }

    /// Check if this token is a parenthesized expression.
    ///
    /// Parenthesized expressions group operations.
    /// Example: `(x + y) * z`
    #[inline]
    pub fn is_parenthesized_expression(&self) -> bool {
        matches!(self, Python::ParenthesizedExpression)
    }

    // ------------------------------------------------------------------------
    // Literals
    // ------------------------------------------------------------------------

    /// Check if this token is an integer literal.
    ///
    /// Integer literals represent whole numbers.
    /// Example: `42`, `0x2A`, `0o52`, `0b101010`
    #[inline]
    pub fn is_integer(&self) -> bool {
        matches!(self, Python::Integer)
    }

    /// Check if this token is a float literal.
    ///
    /// Float literals represent decimal numbers.
    /// Example: `3.14`, `1.0e-5`, `.5`
    #[inline]
    pub fn is_float(&self) -> bool {
        matches!(self, Python::Float)
    }

    /// Check if this token is a boolean literal.
    ///
    /// Boolean literals are True or False.
    #[inline]
    pub fn is_boolean(&self) -> bool {
        matches!(self, Python::True | Python::False)
    }

    /// Check if this token is None.
    ///
    /// None represents the absence of a value.
    #[inline]
    pub fn is_none(&self) -> bool {
        matches!(self, Python::None)
    }

    /// Check if this token is an ellipsis literal (...).
    ///
    /// Ellipsis is used in type hints and slicing.
    /// Example: `...` in `def foo(x: int) -> ...:`
    #[inline]
    pub fn is_ellipsis(&self) -> bool {
        matches!(self, Python::Ellipsis)
    }

    // ------------------------------------------------------------------------
    // Collections
    // ------------------------------------------------------------------------

    /// Check if this token is a list literal.
    ///
    /// List literals create lists.
    /// Example: `[1, 2, 3]`
    #[inline]
    pub fn is_list(&self) -> bool {
        matches!(self, Python::List)
    }

    /// Check if this token is a tuple literal.
    ///
    /// Tuple literals create immutable sequences.
    /// Example: `(1, 2, 3)`, `(1,)`
    #[inline]
    pub fn is_tuple(&self) -> bool {
        matches!(self, Python::Tuple)
    }

    /// Check if this token is a set literal.
    ///
    /// Set literals create unordered collections.
    /// Example: `{1, 2, 3}`
    #[inline]
    pub fn is_set(&self) -> bool {
        matches!(self, Python::Set)
    }

    /// Check if this token is a dictionary literal.
    ///
    /// Dictionary literals create key-value mappings.
    /// Example: `{'key': 'value', 'x': 10}`
    #[inline]
    pub fn is_dictionary(&self) -> bool {
        matches!(self, Python::Dictionary)
    }

    /// Check if this token is a key-value pair.
    ///
    /// Key-value pairs are dictionary entries.
    /// Example: `'key': 'value'` in `{'key': 'value'}`
    #[inline]
    pub fn is_pair(&self) -> bool {
        matches!(self, Python::Pair)
    }

    // ------------------------------------------------------------------------
    // Scope & Blocks
    // ------------------------------------------------------------------------

    /// Check if this token is a block.
    ///
    /// Blocks contain multiple statements.
    #[inline]
    pub fn is_block(&self) -> bool {
        matches!(self, Python::Block | Python::Block2)
    }

    /// Check if this token is a global statement.
    ///
    /// Global statements declare variables as global.
    /// Example: `global x, y`
    #[inline]
    pub fn is_global_statement(&self) -> bool {
        matches!(self, Python::GlobalStatement | Python::Global)
    }

    /// Check if this token is a nonlocal statement.
    ///
    /// Nonlocal statements declare variables from enclosing scope.
    /// Example: `nonlocal x, y`
    #[inline]
    pub fn is_nonlocal_statement(&self) -> bool {
        matches!(self, Python::NonlocalStatement | Python::Nonlocal)
    }

    /// Check if this token is a delete statement.
    ///
    /// Delete statements remove variables or items.
    /// Example: `del x`, `del list[0]`
    #[inline]
    pub fn is_delete_statement(&self) -> bool {
        matches!(self, Python::DeleteStatement | Python::Del)
    }

    // ------------------------------------------------------------------------
    // Comments & Documentation
    // ------------------------------------------------------------------------

    /// Check if this token is a comment.
    ///
    /// Comments are annotations in code.
    /// Example: `# This is a comment`
    #[inline]
    pub fn is_comment(&self) -> bool {
        matches!(self, Python::Comment)
    }

    // ------------------------------------------------------------------------
    // Miscellaneous
    // ------------------------------------------------------------------------

    /// Check if this token is an identifier.
    ///
    /// Identifiers are variable/function/class names.
    #[inline]
    pub fn is_identifier(&self) -> bool {
        matches!(self, Python::Identifier)
    }

    /// Check if this token is a line continuation.
    ///
    /// Line continuations split statements across lines.
    /// Example: `\` at end of line
    #[inline]
    pub fn is_line_continuation(&self) -> bool {
        matches!(self, Python::LineContinuation | Python::BSLASH)
    }

    /// Check if this token is a module.
    ///
    /// Module is the root node of a Python file.
    #[inline]
    pub fn is_module(&self) -> bool {
        matches!(self, Python::Module)
    }

    /// Check if this token is an error node.
    ///
    /// Error nodes represent parsing failures.
    #[inline]
    pub fn is_error(&self) -> bool {
        matches!(self, Python::Error)
    }

    /// Check if this token is an expression statement.
    ///
    /// Expression statements are expressions used as statements.
    #[inline]
    pub fn is_expression_statement(&self) -> bool {
        matches!(self, Python::ExpressionStatement)
    }

    /// Check if this token is a print statement (Python 2).
    ///
    /// Print statements output text (Python 2 only).
    /// Example: `print "Hello"`
    #[inline]
    pub fn is_print_statement(&self) -> bool {
        matches!(self, Python::PrintStatement | Python::Print)
    }

    /// Check if this token is an exec statement (Python 2).
    ///
    /// Exec statements execute code dynamically (Python 2 only).
    /// Example: `exec "x = 1"`
    #[inline]
    pub fn is_exec_statement(&self) -> bool {
        matches!(self, Python::ExecStatement | Python::Exec)
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

    // ========================================================================
    // COMPREHENSIVE TESTS FOR ADVANCED HELPER METHODS
    // ========================================================================

    // ------------------------------------------------------------------------
    // Decorator Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_decorator_application() {
        assert!(Python::Decorator.is_decorator_application());
        assert!(Python::DecoratedDefinition.is_decorator_application());
        assert!(!Python::FunctionDefinition.is_decorator_application());
    }

    #[test]
    fn test_is_function_decorator() {
        assert!(Python::Decorator.is_function_decorator());
        assert!(Python::AT.is_function_decorator());
        assert!(!Python::FunctionDefinition.is_function_decorator());
    }

    #[test]
    fn test_is_class_decorator() {
        assert!(Python::DecoratedDefinition.is_class_decorator());
        assert!(!Python::Decorator.is_class_decorator());
    }

    // ------------------------------------------------------------------------
    // Type System Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_generic_type() {
        assert!(Python::GenericType.is_generic_type());
        assert!(!Python::Type.is_generic_type());
    }

    #[test]
    fn test_is_union_type() {
        assert!(Python::UnionType.is_union_type());
        assert!(Python::PIPE.is_union_type());
        assert!(!Python::Type.is_union_type());
    }

    #[test]
    fn test_is_type_parameter() {
        assert!(Python::TypeParameter.is_type_parameter());
        assert!(!Python::Parameter.is_type_parameter());
    }

    #[test]
    fn test_is_typed_parameter() {
        assert!(Python::TypedParameter.is_typed_parameter());
        assert!(Python::TypedDefaultParameter.is_typed_parameter());
        assert!(!Python::Parameter.is_typed_parameter());
    }

    #[test]
    fn test_is_type_alias() {
        assert!(Python::TypeAliasStatement.is_type_alias());
        assert!(Python::Type2.is_type_alias());
        assert!(!Python::Type.is_type_alias());
    }

    #[test]
    fn test_is_splat_type() {
        assert!(Python::SplatType.is_splat_type());
        assert!(!Python::ListSplat.is_splat_type());
    }

    #[test]
    fn test_is_constrained_type() {
        assert!(Python::ConstrainedType.is_constrained_type());
        assert!(!Python::Type.is_constrained_type());
    }

    #[test]
    fn test_is_member_type() {
        assert!(Python::MemberType.is_member_type());
        assert!(!Python::Type.is_member_type());
    }

    // ------------------------------------------------------------------------
    // Pattern Matching Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_match_statement() {
        assert!(Python::MatchStatement.is_match_statement());
        assert!(Python::Match.is_match_statement());
        assert!(!Python::Case.is_match_statement());
    }

    #[test]
    fn test_is_case_clause() {
        assert!(Python::CaseClause.is_case_clause());
        assert!(Python::Case.is_case_clause());
        assert!(!Python::Match.is_case_clause());
    }

    #[test]
    fn test_is_pattern_guard() {
        assert!(Python::IfClause.is_pattern_guard());
        assert!(!Python::If.is_pattern_guard());
    }

    #[test]
    fn test_is_union_pattern() {
        assert!(Python::UnionPattern.is_union_pattern());
        assert!(!Python::UnionType.is_union_pattern());
    }

    #[test]
    fn test_is_as_pattern() {
        assert!(Python::AsPattern.is_as_pattern());
        assert!(Python::AsPattern2.is_as_pattern());
        assert!(!Python::As.is_as_pattern());
    }

    #[test]
    fn test_is_list_pattern() {
        assert!(Python::ListPattern.is_list_pattern());
        assert!(Python::ListPattern2.is_list_pattern());
        assert!(!Python::List.is_list_pattern());
    }

    #[test]
    fn test_is_tuple_pattern() {
        assert!(Python::TuplePattern.is_tuple_pattern());
        assert!(Python::TuplePattern2.is_tuple_pattern());
        assert!(!Python::Tuple.is_tuple_pattern());
    }

    #[test]
    fn test_is_dict_pattern() {
        assert!(Python::DictPattern.is_dict_pattern());
        assert!(!Python::Dictionary.is_dict_pattern());
    }

    #[test]
    fn test_is_class_pattern() {
        assert!(Python::ClassPattern.is_class_pattern());
        assert!(!Python::ClassDefinition.is_class_pattern());
    }

    #[test]
    fn test_is_splat_pattern() {
        assert!(Python::SplatPattern.is_splat_pattern());
        assert!(!Python::ListSplat.is_splat_pattern());
    }

    // ------------------------------------------------------------------------
    // Comprehension Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_list_comprehension() {
        assert!(Python::ListComprehension.is_list_comprehension());
        assert!(!Python::List.is_list_comprehension());
    }

    #[test]
    fn test_is_dict_comprehension() {
        assert!(Python::DictionaryComprehension.is_dict_comprehension());
        assert!(!Python::Dictionary.is_dict_comprehension());
    }

    #[test]
    fn test_is_set_comprehension() {
        assert!(Python::SetComprehension.is_set_comprehension());
        assert!(!Python::Set.is_set_comprehension());
    }

    #[test]
    fn test_is_generator_expression() {
        assert!(Python::GeneratorExpression.is_generator_expression());
        assert!(!Python::ListComprehension.is_generator_expression());
    }

    #[test]
    fn test_is_for_in_clause() {
        assert!(Python::ForInClause.is_for_in_clause());
        assert!(!Python::ForStatement.is_for_in_clause());
    }

    #[test]
    fn test_is_comprehension_if_clause() {
        assert!(Python::IfClause.is_comprehension_if_clause());
        assert!(!Python::IfStatement.is_comprehension_if_clause());
    }

    // ------------------------------------------------------------------------
    // Context Manager Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_with_statement() {
        assert!(Python::WithStatement.is_with_statement());
        assert!(Python::With.is_with_statement());
        assert!(!Python::WithClause.is_with_statement());
    }

    #[test]
    fn test_is_with_clause() {
        assert!(Python::WithClause.is_with_clause());
        assert!(!Python::WithStatement.is_with_clause());
    }

    #[test]
    fn test_is_with_item() {
        assert!(Python::WithItem.is_with_item());
        assert!(!Python::WithClause.is_with_item());
    }

    #[test]
    fn test_is_async_with() {
        assert!(Python::Async.is_async_with());
        assert!(!Python::With.is_async_with());
    }

    // ------------------------------------------------------------------------
    // Advanced Operator Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_walrus_operator() {
        assert!(Python::COLONEQ.is_walrus_operator());
        assert!(Python::NamedExpression.is_walrus_operator());
        assert!(!Python::COLON.is_walrus_operator());
    }

    #[test]
    fn test_is_matrix_multiply() {
        assert!(Python::AT.is_matrix_multiply());
        assert!(Python::ATEQ.is_matrix_multiply());
        assert!(!Python::STAR.is_matrix_multiply());
    }

    #[test]
    fn test_is_floor_division() {
        assert!(Python::SLASHSLASH.is_floor_division());
        assert!(Python::SLASHSLASHEQ.is_floor_division());
        assert!(!Python::SLASH.is_floor_division());
    }

    #[test]
    fn test_is_power_operator() {
        assert!(Python::STARSTAR.is_power_operator());
        assert!(Python::STARSTAREQ.is_power_operator());
        assert!(!Python::STAR.is_power_operator());
    }

    #[test]
    fn test_is_bitwise_operator() {
        assert!(Python::AMP.is_bitwise_operator());
        assert!(Python::PIPE.is_bitwise_operator());
        assert!(Python::CARET.is_bitwise_operator());
        assert!(Python::TILDE.is_bitwise_operator());
        assert!(Python::LTLT.is_bitwise_operator());
        assert!(Python::GTGT.is_bitwise_operator());
        assert!(!Python::And.is_bitwise_operator());
    }

    #[test]
    fn test_is_comparison_operator() {
        assert!(Python::LT.is_comparison_operator());
        assert!(Python::LTEQ.is_comparison_operator());
        assert!(Python::GT.is_comparison_operator());
        assert!(Python::GTEQ.is_comparison_operator());
        assert!(Python::EQEQ.is_comparison_operator());
        assert!(Python::BANGEQ.is_comparison_operator());
        assert!(Python::Is.is_comparison_operator());
        assert!(Python::In.is_comparison_operator());
        assert!(!Python::EQ.is_comparison_operator());
    }

    #[test]
    fn test_is_boolean_operator() {
        assert!(Python::And.is_boolean_operator());
        assert!(Python::Or.is_boolean_operator());
        assert!(Python::Not.is_boolean_operator());
        assert!(Python::BooleanOperator.is_boolean_operator());
        assert!(!Python::AMP.is_boolean_operator());
    }

    #[test]
    fn test_is_augmented_assignment() {
        assert!(Python::PLUSEQ.is_augmented_assignment());
        assert!(Python::DASHEQ.is_augmented_assignment());
        assert!(Python::STAREQ.is_augmented_assignment());
        assert!(Python::SLASHEQ.is_augmented_assignment());
        assert!(Python::AugmentedAssignment.is_augmented_assignment());
        assert!(!Python::EQ.is_augmented_assignment());
    }

    // ------------------------------------------------------------------------
    // String Feature Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_fstring() {
        assert!(Python::FExpression.is_fstring());
        assert!(Python::Interpolation.is_fstring());
        assert!(Python::FormatExpression.is_fstring());
        assert!(!Python::String.is_fstring());
    }

    #[test]
    fn test_is_format_specifier() {
        assert!(Python::FormatSpecifier.is_format_specifier());
        assert!(Python::FormatSpecifierToken1.is_format_specifier());
        assert!(Python::TypeConversion.is_format_specifier());
        assert!(!Python::String.is_format_specifier());
    }

    #[test]
    fn test_is_string_interpolation() {
        assert!(Python::Interpolation.is_string_interpolation());
        assert!(!Python::String.is_string_interpolation());
    }

    #[test]
    fn test_is_concatenated_string() {
        assert!(Python::ConcatenatedString.is_concatenated_string());
        assert!(!Python::String.is_concatenated_string());
    }

    #[test]
    fn test_is_string_literal() {
        assert!(Python::String.is_string_literal());
        assert!(Python::StringStart.is_string_literal());
        assert!(Python::StringEnd.is_string_literal());
        assert!(!Python::StringContent.is_string_literal());
    }

    #[test]
    fn test_is_string_content() {
        assert!(Python::StringContent.is_string_content());
        assert!(Python::StringContent2.is_string_content());
        assert!(!Python::String.is_string_content());
    }

    #[test]
    fn test_is_escape_sequence() {
        assert!(Python::EscapeSequence.is_escape_sequence());
        assert!(Python::EscapeInterpolation.is_escape_sequence());
        assert!(!Python::String.is_escape_sequence());
    }

    // ------------------------------------------------------------------------
    // Async Feature Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_async_function() {
        assert!(Python::Async.is_async_function());
        assert!(!Python::Def.is_async_function());
    }

    #[test]
    fn test_is_await_expression() {
        assert!(Python::Await.is_await_expression());
        assert!(Python::Await2.is_await_expression());
        assert!(!Python::Async.is_await_expression());
    }

    #[test]
    fn test_is_async_for() {
        assert!(Python::Async.is_async_for());
        assert!(!Python::For.is_async_for());
    }

    #[test]
    fn test_is_async_operation() {
        assert!(Python::Async.is_async_operation());
        assert!(Python::Await.is_async_operation());
        assert!(Python::Await2.is_async_operation());
        assert!(!Python::Def.is_async_operation());
    }

    // ------------------------------------------------------------------------
    // Control Flow Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_try_statement() {
        assert!(Python::TryStatement.is_try_statement());
        assert!(Python::Try.is_try_statement());
        assert!(!Python::Except.is_try_statement());
    }

    #[test]
    fn test_is_except_clause() {
        assert!(Python::ExceptClause.is_except_clause());
        assert!(Python::Except.is_except_clause());
        assert!(Python::ExceptGroupClause.is_except_clause());
        assert!(Python::ExceptSTAR.is_except_clause());
        assert!(!Python::Try.is_except_clause());
    }

    #[test]
    fn test_is_finally_clause() {
        assert!(Python::FinallyClause.is_finally_clause());
        assert!(Python::Finally.is_finally_clause());
        assert!(!Python::Except.is_finally_clause());
    }

    #[test]
    fn test_is_else_clause() {
        assert!(Python::ElseClause.is_else_clause());
        assert!(Python::Else.is_else_clause());
        assert!(!Python::If.is_else_clause());
    }

    #[test]
    fn test_is_raise_statement() {
        assert!(Python::RaiseStatement.is_raise_statement());
        assert!(Python::Raise.is_raise_statement());
        assert!(!Python::Try.is_raise_statement());
    }

    #[test]
    fn test_is_assert_statement() {
        assert!(Python::AssertStatement.is_assert_statement());
        assert!(Python::Assert.is_assert_statement());
        assert!(!Python::Raise.is_assert_statement());
    }

    #[test]
    fn test_is_pass_statement() {
        assert!(Python::PassStatement.is_pass_statement());
        assert!(Python::Pass.is_pass_statement());
        assert!(!Python::Break.is_pass_statement());
    }

    #[test]
    fn test_is_break_statement() {
        assert!(Python::BreakStatement.is_break_statement());
        assert!(Python::Break.is_break_statement());
        assert!(!Python::Continue.is_break_statement());
    }

    #[test]
    fn test_is_continue_statement() {
        assert!(Python::ContinueStatement.is_continue_statement());
        assert!(Python::Continue.is_continue_statement());
        assert!(!Python::Break.is_continue_statement());
    }

    #[test]
    fn test_is_return_statement() {
        assert!(Python::ReturnStatement.is_return_statement());
        assert!(Python::Return.is_return_statement());
        assert!(!Python::Yield.is_return_statement());
    }

    #[test]
    fn test_is_yield_statement() {
        assert!(Python::Yield.is_yield_statement());
        assert!(Python::Yield2.is_yield_statement());
        assert!(!Python::Return.is_yield_statement());
    }

    #[test]
    fn test_is_if_statement() {
        assert!(Python::IfStatement.is_if_statement());
        assert!(Python::If.is_if_statement());
        assert!(!Python::Elif.is_if_statement());
    }

    #[test]
    fn test_is_elif_clause() {
        assert!(Python::ElifClause.is_elif_clause());
        assert!(Python::Elif.is_elif_clause());
        assert!(!Python::If.is_elif_clause());
    }

    #[test]
    fn test_is_for_statement() {
        assert!(Python::ForStatement.is_for_statement());
        assert!(Python::For.is_for_statement());
        assert!(!Python::While.is_for_statement());
    }

    #[test]
    fn test_is_while_statement() {
        assert!(Python::WhileStatement.is_while_statement());
        assert!(Python::While.is_while_statement());
        assert!(!Python::For.is_while_statement());
    }

    #[test]
    fn test_is_conditional_expression() {
        assert!(Python::ConditionalExpression.is_conditional_expression());
        assert!(!Python::IfStatement.is_conditional_expression());
    }

    // ------------------------------------------------------------------------
    // Special Methods Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_potential_dunder_method() {
        assert!(Python::Identifier.is_potential_dunder_method());
        assert!(Python::FunctionDefinition.is_potential_dunder_method());
        assert!(!Python::ClassDefinition.is_potential_dunder_method());
    }

    // ------------------------------------------------------------------------
    // Module System Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_import_statement() {
        assert!(Python::ImportStatement.is_import_statement());
        assert!(Python::Import.is_import_statement());
        assert!(!Python::From.is_import_statement());
    }

    #[test]
    fn test_is_import_from_statement() {
        assert!(Python::ImportFromStatement.is_import_from_statement());
        assert!(Python::From.is_import_from_statement());
        assert!(!Python::Import.is_import_from_statement());
    }

    #[test]
    fn test_is_future_import() {
        assert!(Python::FutureImportStatement.is_future_import());
        assert!(Python::Future.is_future_import());
        assert!(!Python::Import.is_future_import());
    }

    #[test]
    fn test_is_aliased_import() {
        assert!(Python::AliasedImport.is_aliased_import());
        assert!(Python::As.is_aliased_import());
        assert!(!Python::Import.is_aliased_import());
    }

    #[test]
    fn test_is_wildcard_import() {
        assert!(Python::WildcardImport.is_wildcard_import());
        assert!(!Python::STAR.is_wildcard_import());
    }

    #[test]
    fn test_is_relative_import() {
        assert!(Python::RelativeImport.is_relative_import());
        assert!(Python::ImportPrefix.is_relative_import());
        assert!(!Python::Import.is_relative_import());
    }

    #[test]
    fn test_is_dotted_name() {
        assert!(Python::DottedName.is_dotted_name());
        assert!(Python::DOT.is_dotted_name());
        assert!(!Python::Identifier.is_dotted_name());
    }

    // ------------------------------------------------------------------------
    // Class Feature Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_class_definition() {
        assert!(Python::ClassDefinition.is_class_definition());
        assert!(Python::Class.is_class_definition());
        assert!(!Python::FunctionDefinition.is_class_definition());
    }

    #[test]
    fn test_is_class_method_marker() {
        assert!(Python::Decorator.is_class_method_marker());
        assert!(!Python::FunctionDefinition.is_class_method_marker());
    }

    #[test]
    fn test_is_property_marker() {
        assert!(Python::Decorator.is_property_marker());
        assert!(!Python::FunctionDefinition.is_property_marker());
    }

    // ------------------------------------------------------------------------
    // Functional Feature Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_lambda() {
        assert!(Python::Lambda.is_lambda());
        assert!(Python::Lambda2.is_lambda());
        assert!(Python::Lambda3.is_lambda());
        assert!(!Python::FunctionDefinition.is_lambda());
    }

    #[test]
    fn test_is_lambda_parameters() {
        assert!(Python::LambdaParameters.is_lambda_parameters());
        assert!(!Python::Parameters.is_lambda_parameters());
    }

    // ------------------------------------------------------------------------
    // Iterator & Generator Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_yield_expression() {
        assert!(Python::Yield.is_yield_expression());
        assert!(Python::Yield2.is_yield_expression());
        assert!(!Python::Return.is_yield_expression());
    }

    #[test]
    fn test_is_generator_related() {
        assert!(Python::Yield.is_generator_related());
        assert!(Python::Yield2.is_generator_related());
        assert!(Python::GeneratorExpression.is_generator_related());
        assert!(!Python::ListComprehension.is_generator_related());
    }

    // ------------------------------------------------------------------------
    // Parameter & Argument Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_parameter() {
        assert!(Python::Parameter.is_parameter());
        assert!(Python::TypedParameter.is_parameter());
        assert!(Python::DefaultParameter.is_parameter());
        assert!(Python::TypedDefaultParameter.is_parameter());
        assert!(!Python::ArgumentList.is_parameter());
    }

    #[test]
    fn test_is_default_parameter() {
        assert!(Python::DefaultParameter.is_default_parameter());
        assert!(Python::TypedDefaultParameter.is_default_parameter());
        assert!(!Python::Parameter.is_default_parameter());
    }

    #[test]
    fn test_is_list_splat() {
        assert!(Python::ListSplat.is_list_splat());
        assert!(Python::ListSplatPattern.is_list_splat());
        assert!(Python::STAR.is_list_splat());
        assert!(!Python::STARSTAR.is_list_splat());
    }

    #[test]
    fn test_is_dict_splat() {
        assert!(Python::DictionarySplat.is_dict_splat());
        assert!(Python::DictionarySplatPattern.is_dict_splat());
        assert!(Python::STARSTAR.is_dict_splat());
        assert!(!Python::STAR.is_dict_splat());
    }

    #[test]
    fn test_is_keyword_argument() {
        assert!(Python::KeywordArgument.is_keyword_argument());
        assert!(!Python::Parameter.is_keyword_argument());
    }

    #[test]
    fn test_is_positional_separator() {
        assert!(Python::PositionalSeparator.is_positional_separator());
        assert!(!Python::KeywordSeparator.is_positional_separator());
    }

    #[test]
    fn test_is_keyword_separator() {
        assert!(Python::KeywordSeparator.is_keyword_separator());
        assert!(!Python::PositionalSeparator.is_keyword_separator());
    }

    #[test]
    fn test_is_argument_list() {
        assert!(Python::ArgumentList.is_argument_list());
        assert!(!Python::Parameters.is_argument_list());
    }

    // ------------------------------------------------------------------------
    // Expression Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_expression() {
        assert!(Python::Expression.is_expression());
        assert!(Python::PrimaryExpression.is_expression());
        assert!(Python::BinaryOperator.is_expression());
        assert!(Python::UnaryOperator.is_expression());
        assert!(Python::ConditionalExpression.is_expression());
        assert!(Python::NamedExpression.is_expression());
        assert!(!Python::Statement.is_expression());
    }

    #[test]
    fn test_is_primary_expression() {
        assert!(Python::PrimaryExpression.is_primary_expression());
        assert!(!Python::BinaryOperator.is_primary_expression());
    }

    #[test]
    fn test_is_binary_operator() {
        assert!(Python::BinaryOperator.is_binary_operator());
        assert!(!Python::UnaryOperator.is_binary_operator());
    }

    #[test]
    fn test_is_unary_operator() {
        assert!(Python::UnaryOperator.is_unary_operator());
        assert!(Python::NotOperator.is_unary_operator());
        assert!(!Python::BinaryOperator.is_unary_operator());
    }

    #[test]
    fn test_is_named_expression() {
        assert!(Python::NamedExpression.is_named_expression());
        assert!(!Python::Expression.is_named_expression());
    }

    #[test]
    fn test_is_call_expression() {
        assert!(Python::Call.is_call_expression());
        assert!(!Python::FunctionDefinition.is_call_expression());
    }

    #[test]
    fn test_is_attribute() {
        assert!(Python::Attribute.is_attribute());
        assert!(!Python::Identifier.is_attribute());
    }

    #[test]
    fn test_is_subscript() {
        assert!(Python::Subscript.is_subscript());
        assert!(!Python::Slice.is_subscript());
    }

    #[test]
    fn test_is_slice() {
        assert!(Python::Slice.is_slice());
        assert!(!Python::Subscript.is_slice());
    }

    #[test]
    fn test_is_parenthesized_expression() {
        assert!(Python::ParenthesizedExpression.is_parenthesized_expression());
        assert!(!Python::Expression.is_parenthesized_expression());
    }

    // ------------------------------------------------------------------------
    // Literal Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_integer() {
        assert!(Python::Integer.is_integer());
        assert!(!Python::Float.is_integer());
    }

    #[test]
    fn test_is_float() {
        assert!(Python::Float.is_float());
        assert!(!Python::Integer.is_float());
    }

    #[test]
    fn test_is_boolean() {
        assert!(Python::True.is_boolean());
        assert!(Python::False.is_boolean());
        assert!(!Python::None.is_boolean());
    }

    #[test]
    fn test_is_none() {
        assert!(Python::None.is_none());
        assert!(!Python::False.is_none());
    }

    #[test]
    fn test_is_ellipsis() {
        assert!(Python::Ellipsis.is_ellipsis());
        assert!(!Python::None.is_ellipsis());
    }

    // ------------------------------------------------------------------------
    // Collection Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_list() {
        assert!(Python::List.is_list());
        assert!(!Python::ListComprehension.is_list());
    }

    #[test]
    fn test_is_tuple() {
        assert!(Python::Tuple.is_tuple());
        assert!(!Python::List.is_tuple());
    }

    #[test]
    fn test_is_set() {
        assert!(Python::Set.is_set());
        assert!(!Python::SetComprehension.is_set());
    }

    #[test]
    fn test_is_dictionary() {
        assert!(Python::Dictionary.is_dictionary());
        assert!(!Python::DictionaryComprehension.is_dictionary());
    }

    #[test]
    fn test_is_pair() {
        assert!(Python::Pair.is_pair());
        assert!(!Python::Dictionary.is_pair());
    }

    // ------------------------------------------------------------------------
    // Scope & Block Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_block() {
        assert!(Python::Block.is_block());
        assert!(Python::Block2.is_block());
        assert!(!Python::FunctionDefinition.is_block());
    }

    #[test]
    fn test_is_global_statement() {
        assert!(Python::GlobalStatement.is_global_statement());
        assert!(Python::Global.is_global_statement());
        assert!(!Python::Nonlocal.is_global_statement());
    }

    #[test]
    fn test_is_nonlocal_statement() {
        assert!(Python::NonlocalStatement.is_nonlocal_statement());
        assert!(Python::Nonlocal.is_nonlocal_statement());
        assert!(!Python::Global.is_nonlocal_statement());
    }

    #[test]
    fn test_is_delete_statement() {
        assert!(Python::DeleteStatement.is_delete_statement());
        assert!(Python::Del.is_delete_statement());
        assert!(!Python::Pass.is_delete_statement());
    }

    // ------------------------------------------------------------------------
    // Comment Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_comment() {
        assert!(Python::Comment.is_comment());
        assert!(!Python::String.is_comment());
    }

    // ------------------------------------------------------------------------
    // Miscellaneous Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_is_identifier() {
        assert!(Python::Identifier.is_identifier());
        assert!(!Python::Integer.is_identifier());
    }

    #[test]
    fn test_is_line_continuation() {
        assert!(Python::LineContinuation.is_line_continuation());
        assert!(Python::BSLASH.is_line_continuation());
        assert!(!Python::Newline.is_line_continuation());
    }

    #[test]
    fn test_is_module() {
        assert!(Python::Module.is_module());
        assert!(!Python::Block.is_module());
    }

    #[test]
    fn test_is_error() {
        assert!(Python::Error.is_error());
        assert!(!Python::Module.is_error());
    }

    #[test]
    fn test_is_expression_statement() {
        assert!(Python::ExpressionStatement.is_expression_statement());
        assert!(!Python::Expression.is_expression_statement());
    }

    #[test]
    fn test_is_print_statement() {
        assert!(Python::PrintStatement.is_print_statement());
        assert!(Python::Print.is_print_statement());
        assert!(!Python::ExpressionStatement.is_print_statement());
    }

    #[test]
    fn test_is_exec_statement() {
        assert!(Python::ExecStatement.is_exec_statement());
        assert!(Python::Exec.is_exec_statement());
        assert!(!Python::ExpressionStatement.is_exec_statement());
    }

    // ------------------------------------------------------------------------
    // Integration Tests - Testing multiple methods together
    // ------------------------------------------------------------------------

    #[test]
    fn test_comprehension_integration() {
        // All comprehensions should be comprehensions but not collections
        assert!(Python::ListComprehension.is_comprehension());
        assert!(!Python::ListComprehension.is_collection());

        assert!(Python::DictionaryComprehension.is_dict_comprehension());
        assert!(!Python::DictionaryComprehension.is_dictionary());

        assert!(Python::SetComprehension.is_set_comprehension());
        assert!(!Python::SetComprehension.is_set());
    }

    #[test]
    fn test_async_integration() {
        // Async token should work with multiple async features
        assert!(Python::Async.is_async());
        assert!(Python::Async.is_async_function());
        assert!(Python::Async.is_async_for());
        assert!(Python::Async.is_async_with());
        assert!(Python::Async.is_async_operation());
    }

    #[test]
    fn test_type_system_integration() {
        // Type-related tokens should be recognized as type annotations
        assert!(Python::GenericType.is_type_annotation());
        assert!(Python::UnionType.is_type_annotation());
        assert!(Python::TypedParameter.is_type_annotation());
        assert!(Python::TypeAliasStatement.is_type_annotation());
    }

    #[test]
    fn test_pattern_matching_integration() {
        // All pattern types should be recognized as patterns
        assert!(Python::ListPattern.is_pattern());
        assert!(Python::TuplePattern.is_pattern());
        assert!(Python::DictPattern.is_pattern());
        assert!(Python::ClassPattern.is_pattern());
        assert!(Python::UnionPattern.is_pattern());
    }

    #[test]
    fn test_operator_integration() {
        // Test operator categorization
        assert!(Python::PLUS.is_operator());
        assert!(!Python::PLUS.is_assignment());
        assert!(!Python::PLUS.is_comparison_operator());

        assert!(Python::PLUSEQ.is_assignment());
        assert!(Python::PLUSEQ.is_augmented_assignment());
        assert!(!Python::PLUSEQ.is_operator());

        assert!(Python::EQEQ.is_comparison_operator());
        assert!(Python::EQEQ.is_operator());
        assert!(!Python::EQEQ.is_assignment());
    }

    #[test]
    fn test_control_flow_integration() {
        // Test control flow statement recognition
        assert!(Python::IfStatement.is_control_flow());
        assert!(Python::IfStatement.is_if_statement());
        assert!(!Python::IfStatement.is_for_statement());

        assert!(Python::TryStatement.is_control_flow());
        assert!(Python::TryStatement.is_try_statement());
        assert!(!Python::TryStatement.is_if_statement());
    }
}
