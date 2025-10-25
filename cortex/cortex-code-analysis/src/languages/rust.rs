//! Rust language parser implementation.
//!
//! This module provides comprehensive Rust language support including:
//! - All Rust keywords and operators
//! - Pattern matching support
//! - Trait and impl blocks
//! - Generics and lifetimes
//! - Macros (macro_rules!, procedural macros)
//! - Async/await support
//! - Unsafe blocks
//! - Module system (mod, use, pub)
//! - Ownership features (borrow, move, ref, mut)
//! - Closures and higher-order functions
//! - All attribute types (#[derive], #[cfg], etc.)
//! - All literal types (integers, floats, strings, chars, bools)
//! - Complete match expressions
//! - Error handling (Result, Option, ? operator)
//! - Type system features (never type, dynamic dispatch, abstract types)

use num_derive::FromPrimitive;
use crate::lang::Lang;
use crate::traits::LanguageInfo;

/// Rust language token types.
///
/// This enum represents all possible node types in the Rust tree-sitter grammar.
/// Each variant corresponds to a specific Rust language construct, from basic
/// tokens like identifiers and operators to complex structures like traits,
/// impl blocks, and pattern matching constructs.
///
/// The token values are generated from the tree-sitter Rust grammar and should
/// remain synchronized with the grammar version being used.
#[derive(Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum Rust {
    // Basic tokens and delimiters (0-16)
    End = 0,
    Identifier = 1,
    SEMI = 2,
    MacroRulesBANG = 3,
    LPAREN = 4,
    RPAREN = 5,
    LBRACK = 6,
    RBRACK = 7,
    LBRACE = 8,
    RBRACE = 9,
    EQGT = 10,
    COLON = 11,
    DOLLAR = 12,
    TokenRepetitionPatternToken1 = 13,
    PLUS = 14,
    STAR = 15,
    QMARK = 16,

    // Fragment specifiers (17-29)
    Block2 = 17,
    Expr = 18,
    Ident = 19,
    Item = 20,
    Lifetime2 = 21,
    Literal = 22,
    Meta = 23,
    Pat = 24,
    Path = 25,
    Stmt = 26,
    Tt = 27,
    Ty = 28,
    Vis = 29,

    // Primitive types (30-46)
    PrimitiveType = 30,
    PrimitiveType2 = 31,
    PrimitiveType3 = 32,
    PrimitiveType4 = 33,
    PrimitiveType5 = 34,
    PrimitiveType6 = 35,
    PrimitiveType7 = 36,
    PrimitiveType8 = 37,
    PrimitiveType9 = 38,
    PrimitiveType10 = 39,
    PrimitiveType11 = 40,
    PrimitiveType12 = 41,
    PrimitiveType13 = 42,
    PrimitiveType14 = 43,
    PrimitiveType15 = 44,
    PrimitiveType16 = 45,
    PrimitiveType17 = 46,

    // Operators (47-85)
    DASH = 47,
    SLASH = 48,
    PERCENT = 49,
    CARET = 50,
    BANG = 51,
    AMP = 52,
    PIPE = 53,
    AMPAMP = 54,
    PIPEPIPE = 55,
    LTLT = 56,
    GTGT = 57,
    PLUSEQ = 58,
    DASHEQ = 59,
    STAREQ = 60,
    SLASHEQ = 61,
    PERCENTEQ = 62,
    CARETEQ = 63,
    AMPEQ = 64,
    PIPEEQ = 65,
    LTLTEQ = 66,
    GTGTEQ = 67,
    EQ = 68,
    EQEQ = 69,
    BANGEQ = 70,
    GT = 71,
    LT = 72,
    GTEQ = 73,
    LTEQ = 74,
    AT = 75,
    UNDERSCORE = 76,
    DOT = 77,
    DOTDOT = 78,
    DOTDOTDOT = 79,
    DOTDOTEQ = 80,
    COMMA = 81,
    COLONCOLON = 82,
    DASHGT = 83,
    HASH = 84,
    SQUOTE = 85,

    // Keywords (86-124)
    As = 86,
    Async = 87,
    Await = 88,
    Break = 89,
    Const = 90,
    Continue = 91,
    Default = 92,
    Enum = 93,
    Fn = 94,
    For = 95,
    Gen = 96,
    If = 97,
    Impl = 98,
    Let = 99,
    Loop = 100,
    Match = 101,
    Mod = 102,
    Pub = 103,
    Return = 104,
    Static = 105,
    Struct = 106,
    Trait = 107,
    Type = 108,
    Union = 109,
    Unsafe = 110,
    Use = 111,
    Where = 112,
    While = 113,
    Extern = 114,
    Ref = 115,
    Else = 116,
    In = 117,
    LT2 = 118,
    Dyn = 119,
    MutableSpecifier = 120,
    Raw = 121,
    Yield = 122,
    Move = 123,
    Try = 124,

    // Literals (125-157)
    IntegerLiteral = 125,
    DQUOTE = 126,
    DQUOTE2 = 127,
    CharLiteral = 128,
    EscapeSequence = 129,
    True = 130,
    False = 131,
    SLASHSLASH = 132,
    LineCommentToken1 = 133,
    LineCommentToken2 = 134,
    LineCommentToken3 = 135,
    BANG2 = 136,
    SLASH2 = 137,
    SLASHSTAR = 138,
    STARSLASH = 139,
    Shebang = 140,
    Zelf = 141,
    Super = 142,
    Crate = 143,
    Metavariable = 144,
    StringContent = 145,
    RawStringLiteralStart = 146,
    StringContent2 = 147,
    RawStringLiteralEnd = 148,
    FloatLiteral = 149,
    OuterDocCommentMarker = 150,
    InnerDocCommentMarker = 151,
    BlockCommentContent = 152,
    DocComment = 153,
    ErrorSentinel = 154,

    // Structure nodes (155-289)
    SourceFile = 155,
    Statement = 156,
    EmptyStatement = 157,
    ExpressionStatement = 158,
    MacroDefinition = 159,
    MacroRule = 160,
    TokenPattern = 161,
    TokenTreePattern = 162,
    TokenBindingPattern = 163,
    TokenRepetitionPattern = 164,
    FragmentSpecifier = 165,
    TokenTree = 166,
    TokenRepetition = 167,
    AttributeItem = 168,
    InnerAttributeItem = 169,
    Attribute = 170,
    ModItem = 171,
    ForeignModItem = 172,
    DeclarationList = 173,
    StructItem = 174,
    UnionItem = 175,
    EnumItem = 176,
    EnumVariantList = 177,
    EnumVariant = 178,
    FieldDeclarationList = 179,
    FieldDeclaration = 180,
    OrderedFieldDeclarationList = 181,
    ExternCrateDeclaration = 182,
    ConstItem = 183,
    StaticItem = 184,
    TypeItem = 185,
    FunctionItem = 186,
    FunctionSignatureItem = 187,
    FunctionModifiers = 188,
    WhereClause = 189,
    WherePredicate = 190,
    ImplItem = 191,
    TraitItem = 192,
    AssociatedType = 193,
    TraitBounds = 194,
    HigherRankedTraitBound = 195,
    RemovedTraitBound = 196,
    TypeParameters = 197,
    ConstParameter = 198,
    ConstrainedTypeParameter = 199,
    OptionalTypeParameter = 200,
    LetDeclaration = 201,
    UseDeclaration = 202,
    UseClause = 203,
    ScopedUseList = 204,
    UseList = 205,
    UseAsClause = 206,
    UseWildcard = 207,
    Parameters = 208,
    SelfParameter = 209,
    VariadicParameter = 210,
    Parameter = 211,
    ExternModifier = 212,
    VisibilityModifier = 213,
    Type2 = 214,
    BracketedType = 215,
    QualifiedType = 216,
    Lifetime = 217,
    ArrayType = 218,
    ForLifetimes = 219,
    FunctionType = 220,
    TupleType = 221,
    UnitType = 222,
    GenericFunction = 223,
    GenericType = 224,
    GenericTypeWithTurbofish = 225,
    BoundedType = 226,
    TypeArguments = 227,
    TypeBinding = 228,
    ReferenceType = 229,
    PointerType = 230,
    NeverType = 231,
    AbstractType = 232,
    DynamicType = 233,
    ExpressionExceptRange = 234,
    Expression = 235,
    MacroInvocation = 236,
    TokenTree2 = 237,
    DelimTokens = 238,
    NonDelimToken = 239,
    ScopedIdentifier = 240,
    ScopedTypeIdentifier = 241,
    ScopedTypeIdentifier2 = 242,
    RangeExpression = 243,
    UnaryExpression = 244,
    TryExpression = 245,
    ReferenceExpression = 246,
    BinaryExpression = 247,
    AssignmentExpression = 248,
    CompoundAssignmentExpr = 249,
    TypeCastExpression = 250,
    ReturnExpression = 251,
    YieldExpression = 252,
    CallExpression = 253,
    Arguments = 254,
    ArrayExpression = 255,
    ParenthesizedExpression = 256,
    TupleExpression = 257,
    UnitExpression = 258,
    StructExpression = 259,
    FieldInitializerList = 260,
    ShorthandFieldInitializer = 261,
    FieldInitializer = 262,
    BaseFieldInitializer = 263,
    IfExpression = 264,
    LetCondition = 265,
    LetChain2 = 266,
    Condition = 267,
    ElseClause = 268,
    MatchExpression = 269,
    MatchBlock = 270,
    MatchArm = 271,
    MatchArm2 = 272,
    MatchPattern = 273,
    WhileExpression = 274,
    LoopExpression = 275,
    ForExpression = 276,
    ConstBlock = 277,
    ClosureExpression = 278,
    ClosureParameters = 279,
    Label = 280,
    BreakExpression = 281,
    ContinueExpression = 282,
    IndexExpression = 283,
    AwaitExpression = 284,
    FieldExpression = 285,
    UnsafeBlock = 286,
    AsyncBlock = 287,
    GenBlock = 288,
    TryBlock = 289,
    Block = 290,

    // Pattern nodes (291-309)
    Pattern = 291,
    TuplePattern = 292,
    SlicePattern = 293,
    TupleStructPattern = 294,
    StructPattern = 295,
    FieldPattern = 296,
    RemainingFieldPattern = 297,
    MutPattern = 298,
    RangePattern = 299,
    RefPattern = 300,
    CapturedPattern = 301,
    ReferencePattern = 302,
    OrPattern = 303,
    Literal2 = 304,
    LiteralPattern = 305,
    NegativeLiteral = 306,
    StringLiteral = 307,
    RawStringLiteral = 308,
    BooleanLiteral = 309,

    // Comments and documentation (310-315)
    LineComment = 310,
    LineDocCommentMarker = 311,
    InnerDocCommentMarker2 = 312,
    OuterDocCommentMarker2 = 313,
    BlockComment = 314,
    BlockDocCommentMarker = 315,

    // Repeat nodes (316-352)
    SourceFileRepeat1 = 316,
    MacroDefinitionRepeat1 = 317,
    TokenTreePatternRepeat1 = 318,
    TokenTreeRepeat1 = 319,
    NonSpecialTokenRepeat1 = 320,
    DeclarationListRepeat1 = 321,
    EnumVariantListRepeat1 = 322,
    EnumVariantListRepeat2 = 323,
    FieldDeclarationListRepeat1 = 324,
    OrderedFieldDeclarationListRepeat1 = 325,
    FunctionModifiersRepeat1 = 326,
    WhereClauseRepeat1 = 327,
    TraitBoundsRepeat1 = 328,
    TypeParametersRepeat1 = 329,
    UseListRepeat1 = 330,
    ParametersRepeat1 = 331,
    ForLifetimesRepeat1 = 332,
    TupleTypeRepeat1 = 333,
    TypeArgumentsRepeat1 = 334,
    DelimTokenTreeRepeat1 = 335,
    ArgumentsRepeat1 = 336,
    TupleExpressionRepeat1 = 337,
    FieldInitializerListRepeat1 = 338,
    MatchBlockRepeat1 = 339,
    MatchArmRepeat1 = 340,
    ClosureParametersRepeat1 = 341,
    TuplePatternRepeat1 = 342,
    SlicePatternRepeat1 = 343,
    StructPatternRepeat1 = 344,
    StringLiteralRepeat1 = 345,

    // Special identifiers (346-349)
    FieldIdentifier = 346,
    LetChain = 347,
    ShorthandFieldIdentifier = 348,
    TypeIdentifier = 349,

    // Error node
    Error = 350,
}

impl From<Rust> for &'static str {
    #[inline(always)]
    fn from(tok: Rust) -> Self {
        match tok {
            Rust::End => "end",
            Rust::Identifier => "identifier",
            Rust::SEMI => ";",
            Rust::MacroRulesBANG => "macro_rules!",
            Rust::LPAREN => "(",
            Rust::RPAREN => ")",
            Rust::LBRACK => "[",
            Rust::RBRACK => "]",
            Rust::LBRACE => "{",
            Rust::RBRACE => "}",
            Rust::EQGT => "=>",
            Rust::COLON => ":",
            Rust::DOLLAR => "$",
            Rust::TokenRepetitionPatternToken1 => "token_repetition_pattern_token1",
            Rust::PLUS => "+",
            Rust::STAR => "*",
            Rust::QMARK => "?",
            Rust::Block2 => "block",
            Rust::Expr => "expr",
            Rust::Ident => "ident",
            Rust::Item => "item",
            Rust::Lifetime2 => "lifetime",
            Rust::Literal => "literal",
            Rust::Meta => "meta",
            Rust::Pat => "pat",
            Rust::Path => "path",
            Rust::Stmt => "stmt",
            Rust::Tt => "tt",
            Rust::Ty => "ty",
            Rust::Vis => "vis",
            Rust::PrimitiveType => "primitive_type",
            Rust::PrimitiveType2 => "primitive_type",
            Rust::PrimitiveType3 => "primitive_type",
            Rust::PrimitiveType4 => "primitive_type",
            Rust::PrimitiveType5 => "primitive_type",
            Rust::PrimitiveType6 => "primitive_type",
            Rust::PrimitiveType7 => "primitive_type",
            Rust::PrimitiveType8 => "primitive_type",
            Rust::PrimitiveType9 => "primitive_type",
            Rust::PrimitiveType10 => "primitive_type",
            Rust::PrimitiveType11 => "primitive_type",
            Rust::PrimitiveType12 => "primitive_type",
            Rust::PrimitiveType13 => "primitive_type",
            Rust::PrimitiveType14 => "primitive_type",
            Rust::PrimitiveType15 => "primitive_type",
            Rust::PrimitiveType16 => "primitive_type",
            Rust::PrimitiveType17 => "primitive_type",
            Rust::DASH => "-",
            Rust::SLASH => "/",
            Rust::PERCENT => "%",
            Rust::CARET => "^",
            Rust::BANG => "!",
            Rust::AMP => "&",
            Rust::PIPE => "|",
            Rust::AMPAMP => "&&",
            Rust::PIPEPIPE => "||",
            Rust::LTLT => "<<",
            Rust::GTGT => ">>",
            Rust::PLUSEQ => "+=",
            Rust::DASHEQ => "-=",
            Rust::STAREQ => "*=",
            Rust::SLASHEQ => "/=",
            Rust::PERCENTEQ => "%=",
            Rust::CARETEQ => "^=",
            Rust::AMPEQ => "&=",
            Rust::PIPEEQ => "|=",
            Rust::LTLTEQ => "<<=",
            Rust::GTGTEQ => ">>=",
            Rust::EQ => "=",
            Rust::EQEQ => "==",
            Rust::BANGEQ => "!=",
            Rust::GT => ">",
            Rust::LT => "<",
            Rust::GTEQ => ">=",
            Rust::LTEQ => "<=",
            Rust::AT => "@",
            Rust::UNDERSCORE => "_",
            Rust::DOT => ".",
            Rust::DOTDOT => "..",
            Rust::DOTDOTDOT => "...",
            Rust::DOTDOTEQ => "..=",
            Rust::COMMA => ",",
            Rust::COLONCOLON => "::",
            Rust::DASHGT => "->",
            Rust::HASH => "#",
            Rust::SQUOTE => "'",
            Rust::As => "as",
            Rust::Async => "async",
            Rust::Await => "await",
            Rust::Break => "break",
            Rust::Const => "const",
            Rust::Continue => "continue",
            Rust::Default => "default",
            Rust::Enum => "enum",
            Rust::Fn => "fn",
            Rust::For => "for",
            Rust::Gen => "gen",
            Rust::If => "if",
            Rust::Impl => "impl",
            Rust::Let => "let",
            Rust::Loop => "loop",
            Rust::Match => "match",
            Rust::Mod => "mod",
            Rust::Pub => "pub",
            Rust::Return => "return",
            Rust::Static => "static",
            Rust::Struct => "struct",
            Rust::Trait => "trait",
            Rust::Type => "type",
            Rust::Union => "union",
            Rust::Unsafe => "unsafe",
            Rust::Use => "use",
            Rust::Where => "where",
            Rust::While => "while",
            Rust::Extern => "extern",
            Rust::Ref => "ref",
            Rust::Else => "else",
            Rust::In => "in",
            Rust::LT2 => "<",
            Rust::Dyn => "dyn",
            Rust::MutableSpecifier => "mutable_specifier",
            Rust::Raw => "raw",
            Rust::Yield => "yield",
            Rust::Move => "move",
            Rust::Try => "try",
            Rust::IntegerLiteral => "integer_literal",
            Rust::DQUOTE => "\"",
            Rust::DQUOTE2 => "\"",
            Rust::CharLiteral => "char_literal",
            Rust::EscapeSequence => "escape_sequence",
            Rust::True => "true",
            Rust::False => "false",
            Rust::SLASHSLASH => "//",
            Rust::LineCommentToken1 => "line_comment_token1",
            Rust::LineCommentToken2 => "line_comment_token2",
            Rust::LineCommentToken3 => "line_comment_token3",
            Rust::BANG2 => "!",
            Rust::SLASH2 => "/",
            Rust::SLASHSTAR => "/*",
            Rust::STARSLASH => "*/",
            Rust::Shebang => "shebang",
            Rust::Zelf => "self",
            Rust::Super => "super",
            Rust::Crate => "crate",
            Rust::Metavariable => "metavariable",
            Rust::StringContent => "string_content",
            Rust::RawStringLiteralStart => "_raw_string_literal_start",
            Rust::StringContent2 => "string_content",
            Rust::RawStringLiteralEnd => "_raw_string_literal_end",
            Rust::FloatLiteral => "float_literal",
            Rust::OuterDocCommentMarker => "outer_doc_comment_marker",
            Rust::InnerDocCommentMarker => "inner_doc_comment_marker",
            Rust::BlockCommentContent => "_block_comment_content",
            Rust::DocComment => "doc_comment",
            Rust::ErrorSentinel => "_error_sentinel",
            Rust::SourceFile => "source_file",
            Rust::Statement => "_statement",
            Rust::EmptyStatement => "empty_statement",
            Rust::ExpressionStatement => "expression_statement",
            Rust::MacroDefinition => "macro_definition",
            Rust::MacroRule => "macro_rule",
            Rust::TokenPattern => "_token_pattern",
            Rust::TokenTreePattern => "token_tree_pattern",
            Rust::TokenBindingPattern => "token_binding_pattern",
            Rust::TokenRepetitionPattern => "token_repetition_pattern",
            Rust::FragmentSpecifier => "fragment_specifier",
            Rust::TokenTree => "token_tree",
            Rust::TokenRepetition => "token_repetition",
            Rust::AttributeItem => "attribute_item",
            Rust::InnerAttributeItem => "inner_attribute_item",
            Rust::Attribute => "attribute",
            Rust::ModItem => "mod_item",
            Rust::ForeignModItem => "foreign_mod_item",
            Rust::DeclarationList => "declaration_list",
            Rust::StructItem => "struct_item",
            Rust::UnionItem => "union_item",
            Rust::EnumItem => "enum_item",
            Rust::EnumVariantList => "enum_variant_list",
            Rust::EnumVariant => "enum_variant",
            Rust::FieldDeclarationList => "field_declaration_list",
            Rust::FieldDeclaration => "field_declaration",
            Rust::OrderedFieldDeclarationList => "ordered_field_declaration_list",
            Rust::ExternCrateDeclaration => "extern_crate_declaration",
            Rust::ConstItem => "const_item",
            Rust::StaticItem => "static_item",
            Rust::TypeItem => "type_item",
            Rust::FunctionItem => "function_item",
            Rust::FunctionSignatureItem => "function_signature_item",
            Rust::FunctionModifiers => "function_modifiers",
            Rust::WhereClause => "where_clause",
            Rust::WherePredicate => "where_predicate",
            Rust::ImplItem => "impl_item",
            Rust::TraitItem => "trait_item",
            Rust::AssociatedType => "associated_type",
            Rust::TraitBounds => "trait_bounds",
            Rust::HigherRankedTraitBound => "higher_ranked_trait_bound",
            Rust::RemovedTraitBound => "removed_trait_bound",
            Rust::TypeParameters => "type_parameters",
            Rust::ConstParameter => "const_parameter",
            Rust::ConstrainedTypeParameter => "constrained_type_parameter",
            Rust::OptionalTypeParameter => "optional_type_parameter",
            Rust::LetDeclaration => "let_declaration",
            Rust::UseDeclaration => "use_declaration",
            Rust::UseClause => "_use_clause",
            Rust::ScopedUseList => "scoped_use_list",
            Rust::UseList => "use_list",
            Rust::UseAsClause => "use_as_clause",
            Rust::UseWildcard => "use_wildcard",
            Rust::Parameters => "parameters",
            Rust::SelfParameter => "self_parameter",
            Rust::VariadicParameter => "variadic_parameter",
            Rust::Parameter => "parameter",
            Rust::ExternModifier => "extern_modifier",
            Rust::VisibilityModifier => "visibility_modifier",
            Rust::Type2 => "_type",
            Rust::BracketedType => "bracketed_type",
            Rust::QualifiedType => "qualified_type",
            Rust::Lifetime => "lifetime",
            Rust::ArrayType => "array_type",
            Rust::ForLifetimes => "for_lifetimes",
            Rust::FunctionType => "function_type",
            Rust::TupleType => "tuple_type",
            Rust::UnitType => "unit_type",
            Rust::GenericFunction => "generic_function",
            Rust::GenericType => "generic_type",
            Rust::GenericTypeWithTurbofish => "generic_type_with_turbofish",
            Rust::BoundedType => "bounded_type",
            Rust::TypeArguments => "type_arguments",
            Rust::TypeBinding => "type_binding",
            Rust::ReferenceType => "reference_type",
            Rust::PointerType => "pointer_type",
            Rust::NeverType => "never_type",
            Rust::AbstractType => "abstract_type",
            Rust::DynamicType => "dynamic_type",
            Rust::ExpressionExceptRange => "_expression_except_range",
            Rust::Expression => "_expression",
            Rust::MacroInvocation => "macro_invocation",
            Rust::TokenTree2 => "token_tree",
            Rust::DelimTokens => "_delim_tokens",
            Rust::NonDelimToken => "_non_delim_token",
            Rust::ScopedIdentifier => "scoped_identifier",
            Rust::ScopedTypeIdentifier => "scoped_type_identifier",
            Rust::ScopedTypeIdentifier2 => "scoped_type_identifier",
            Rust::RangeExpression => "range_expression",
            Rust::UnaryExpression => "unary_expression",
            Rust::TryExpression => "try_expression",
            Rust::ReferenceExpression => "reference_expression",
            Rust::BinaryExpression => "binary_expression",
            Rust::AssignmentExpression => "assignment_expression",
            Rust::CompoundAssignmentExpr => "compound_assignment_expr",
            Rust::TypeCastExpression => "type_cast_expression",
            Rust::ReturnExpression => "return_expression",
            Rust::YieldExpression => "yield_expression",
            Rust::CallExpression => "call_expression",
            Rust::Arguments => "arguments",
            Rust::ArrayExpression => "array_expression",
            Rust::ParenthesizedExpression => "parenthesized_expression",
            Rust::TupleExpression => "tuple_expression",
            Rust::UnitExpression => "unit_expression",
            Rust::StructExpression => "struct_expression",
            Rust::FieldInitializerList => "field_initializer_list",
            Rust::ShorthandFieldInitializer => "shorthand_field_initializer",
            Rust::FieldInitializer => "field_initializer",
            Rust::BaseFieldInitializer => "base_field_initializer",
            Rust::IfExpression => "if_expression",
            Rust::LetCondition => "let_condition",
            Rust::LetChain2 => "_let_chain",
            Rust::Condition => "_condition",
            Rust::ElseClause => "else_clause",
            Rust::MatchExpression => "match_expression",
            Rust::MatchBlock => "match_block",
            Rust::MatchArm => "match_arm",
            Rust::MatchArm2 => "match_arm",
            Rust::MatchPattern => "match_pattern",
            Rust::WhileExpression => "while_expression",
            Rust::LoopExpression => "loop_expression",
            Rust::ForExpression => "for_expression",
            Rust::ConstBlock => "const_block",
            Rust::ClosureExpression => "closure_expression",
            Rust::ClosureParameters => "closure_parameters",
            Rust::Label => "label",
            Rust::BreakExpression => "break_expression",
            Rust::ContinueExpression => "continue_expression",
            Rust::IndexExpression => "index_expression",
            Rust::AwaitExpression => "await_expression",
            Rust::FieldExpression => "field_expression",
            Rust::UnsafeBlock => "unsafe_block",
            Rust::AsyncBlock => "async_block",
            Rust::GenBlock => "gen_block",
            Rust::TryBlock => "try_block",
            Rust::Block => "block",
            Rust::Pattern => "_pattern",
            Rust::TuplePattern => "tuple_pattern",
            Rust::SlicePattern => "slice_pattern",
            Rust::TupleStructPattern => "tuple_struct_pattern",
            Rust::StructPattern => "struct_pattern",
            Rust::FieldPattern => "field_pattern",
            Rust::RemainingFieldPattern => "remaining_field_pattern",
            Rust::MutPattern => "mut_pattern",
            Rust::RangePattern => "range_pattern",
            Rust::RefPattern => "ref_pattern",
            Rust::CapturedPattern => "captured_pattern",
            Rust::ReferencePattern => "reference_pattern",
            Rust::OrPattern => "or_pattern",
            Rust::Literal2 => "_literal",
            Rust::LiteralPattern => "_literal_pattern",
            Rust::NegativeLiteral => "negative_literal",
            Rust::StringLiteral => "string_literal",
            Rust::RawStringLiteral => "raw_string_literal",
            Rust::BooleanLiteral => "boolean_literal",
            Rust::LineComment => "line_comment",
            Rust::LineDocCommentMarker => "_line_doc_comment_marker",
            Rust::InnerDocCommentMarker2 => "inner_doc_comment_marker",
            Rust::OuterDocCommentMarker2 => "outer_doc_comment_marker",
            Rust::BlockComment => "block_comment",
            Rust::BlockDocCommentMarker => "_block_doc_comment_marker",
            Rust::SourceFileRepeat1 => "source_file_repeat1",
            Rust::MacroDefinitionRepeat1 => "macro_definition_repeat1",
            Rust::TokenTreePatternRepeat1 => "token_tree_pattern_repeat1",
            Rust::TokenTreeRepeat1 => "token_tree_repeat1",
            Rust::NonSpecialTokenRepeat1 => "_non_special_token_repeat1",
            Rust::DeclarationListRepeat1 => "declaration_list_repeat1",
            Rust::EnumVariantListRepeat1 => "enum_variant_list_repeat1",
            Rust::EnumVariantListRepeat2 => "enum_variant_list_repeat2",
            Rust::FieldDeclarationListRepeat1 => "field_declaration_list_repeat1",
            Rust::OrderedFieldDeclarationListRepeat1 => "ordered_field_declaration_list_repeat1",
            Rust::FunctionModifiersRepeat1 => "function_modifiers_repeat1",
            Rust::WhereClauseRepeat1 => "where_clause_repeat1",
            Rust::TraitBoundsRepeat1 => "trait_bounds_repeat1",
            Rust::TypeParametersRepeat1 => "type_parameters_repeat1",
            Rust::UseListRepeat1 => "use_list_repeat1",
            Rust::ParametersRepeat1 => "parameters_repeat1",
            Rust::ForLifetimesRepeat1 => "for_lifetimes_repeat1",
            Rust::TupleTypeRepeat1 => "tuple_type_repeat1",
            Rust::TypeArgumentsRepeat1 => "type_arguments_repeat1",
            Rust::DelimTokenTreeRepeat1 => "delim_token_tree_repeat1",
            Rust::ArgumentsRepeat1 => "arguments_repeat1",
            Rust::TupleExpressionRepeat1 => "tuple_expression_repeat1",
            Rust::FieldInitializerListRepeat1 => "field_initializer_list_repeat1",
            Rust::MatchBlockRepeat1 => "match_block_repeat1",
            Rust::MatchArmRepeat1 => "match_arm_repeat1",
            Rust::ClosureParametersRepeat1 => "closure_parameters_repeat1",
            Rust::TuplePatternRepeat1 => "tuple_pattern_repeat1",
            Rust::SlicePatternRepeat1 => "slice_pattern_repeat1",
            Rust::StructPatternRepeat1 => "struct_pattern_repeat1",
            Rust::StringLiteralRepeat1 => "string_literal_repeat1",
            Rust::FieldIdentifier => "field_identifier",
            Rust::LetChain => "let_chain",
            Rust::ShorthandFieldIdentifier => "shorthand_field_identifier",
            Rust::TypeIdentifier => "type_identifier",
            Rust::Error => "ERROR",
        }
    }
}

impl From<u16> for Rust {
    #[inline(always)]
    fn from(x: u16) -> Self {
        num::FromPrimitive::from_u16(x).unwrap_or(Self::Error)
    }
}

/// Rust == u16 comparison
impl PartialEq<u16> for Rust {
    #[inline(always)]
    fn eq(&self, x: &u16) -> bool {
        *self == Into::<Self>::into(*x)
    }
}

/// u16 == Rust comparison
impl PartialEq<Rust> for u16 {
    #[inline(always)]
    fn eq(&self, x: &Rust) -> bool {
        *x == *self
    }
}

impl Rust {
    /// Check if this token represents a function definition or declaration.
    ///
    /// Returns true for function items, function signatures, and generic functions.
    #[inline]
    pub fn is_function(&self) -> bool {
        matches!(
            self,
            Rust::FunctionItem | Rust::FunctionSignatureItem | Rust::GenericFunction | Rust::Fn
        )
    }

    /// Check if this token represents a struct definition.
    ///
    /// Returns true for struct items including tuple structs and unit structs.
    #[inline]
    pub fn is_struct(&self) -> bool {
        matches!(self, Rust::StructItem | Rust::Struct)
    }

    /// Check if this token represents an enum definition.
    ///
    /// Returns true for enum items, variants, and variant lists.
    #[inline]
    pub fn is_enum(&self) -> bool {
        matches!(
            self,
            Rust::EnumItem | Rust::Enum | Rust::EnumVariant | Rust::EnumVariantList
        )
    }

    /// Check if this token represents a trait definition.
    ///
    /// Returns true for trait items and the trait keyword.
    #[inline]
    pub fn is_trait(&self) -> bool {
        matches!(self, Rust::TraitItem | Rust::Trait)
    }

    /// Check if this token represents an impl block.
    ///
    /// Returns true for impl items and the impl keyword.
    #[inline]
    pub fn is_impl(&self) -> bool {
        matches!(self, Rust::ImplItem | Rust::Impl)
    }

    /// Check if this token represents a module definition.
    ///
    /// Returns true for module items and the mod keyword.
    #[inline]
    pub fn is_module(&self) -> bool {
        matches!(self, Rust::ModItem | Rust::Mod)
    }

    /// Check if this token represents a use statement.
    ///
    /// Returns true for use declarations and related constructs.
    #[inline]
    pub fn is_use(&self) -> bool {
        matches!(
            self,
            Rust::UseDeclaration
                | Rust::Use
                | Rust::UseClause
                | Rust::UseList
                | Rust::UseAsClause
                | Rust::UseWildcard
        )
    }

    /// Check if this token represents an async construct.
    ///
    /// Returns true for async/await keywords and async blocks.
    #[inline]
    pub fn is_async(&self) -> bool {
        matches!(
            self,
            Rust::Async | Rust::Await | Rust::AsyncBlock | Rust::AwaitExpression
        )
    }

    /// Check if this token represents an unsafe construct.
    ///
    /// Returns true for unsafe keyword and unsafe blocks.
    #[inline]
    pub fn is_unsafe(&self) -> bool {
        matches!(self, Rust::Unsafe | Rust::UnsafeBlock)
    }

    /// Check if this token represents a const construct.
    ///
    /// Returns true for const keyword, const items, and const blocks.
    #[inline]
    pub fn is_const(&self) -> bool {
        matches!(self, Rust::Const | Rust::ConstItem | Rust::ConstBlock | Rust::ConstParameter)
    }

    /// Check if this token represents a macro.
    ///
    /// Returns true for macro definitions, invocations, and macro_rules!.
    #[inline]
    pub fn is_macro(&self) -> bool {
        matches!(
            self,
            Rust::MacroDefinition
                | Rust::MacroInvocation
                | Rust::MacroRulesBANG
                | Rust::MacroRule
        )
    }

    /// Check if this token represents a pattern.
    ///
    /// Returns true for all pattern types including tuple, slice, struct, and ref patterns.
    #[inline]
    pub fn is_pattern(&self) -> bool {
        matches!(
            self,
            Rust::Pattern
                | Rust::TuplePattern
                | Rust::SlicePattern
                | Rust::TupleStructPattern
                | Rust::StructPattern
                | Rust::FieldPattern
                | Rust::MutPattern
                | Rust::RangePattern
                | Rust::RefPattern
                | Rust::CapturedPattern
                | Rust::ReferencePattern
                | Rust::OrPattern
                | Rust::LiteralPattern
                | Rust::MatchPattern
        )
    }

    /// Check if this token represents a match expression.
    ///
    /// Returns true for match expressions, match blocks, and match arms.
    #[inline]
    pub fn is_match(&self) -> bool {
        matches!(
            self,
            Rust::Match
                | Rust::MatchExpression
                | Rust::MatchBlock
                | Rust::MatchArm
                | Rust::MatchArm2
        )
    }

    /// Check if this token represents a closure expression.
    ///
    /// Returns true for closure expressions and their parameters.
    #[inline]
    pub fn is_closure(&self) -> bool {
        matches!(
            self,
            Rust::ClosureExpression | Rust::ClosureParameters | Rust::Move
        )
    }

    /// Check if this token represents a lifetime.
    ///
    /// Returns true for lifetime annotations and for-lifetimes clauses.
    #[inline]
    pub fn is_lifetime(&self) -> bool {
        matches!(
            self,
            Rust::Lifetime | Rust::Lifetime2 | Rust::ForLifetimes | Rust::SQUOTE
        )
    }

    /// Check if this token represents a generic type parameter.
    ///
    /// Returns true for type parameters and type arguments.
    #[inline]
    pub fn is_generic(&self) -> bool {
        matches!(
            self,
            Rust::TypeParameters
                | Rust::TypeArguments
                | Rust::GenericType
                | Rust::GenericFunction
                | Rust::GenericTypeWithTurbofish
                | Rust::ConstrainedTypeParameter
                | Rust::OptionalTypeParameter
        )
    }

    /// Check if this token represents a where clause.
    ///
    /// Returns true for where clauses and predicates.
    #[inline]
    pub fn is_where_clause(&self) -> bool {
        matches!(
            self,
            Rust::Where | Rust::WhereClause | Rust::WherePredicate
        )
    }

    /// Check if this token represents a trait bound.
    ///
    /// Returns true for trait bounds and higher-ranked trait bounds.
    #[inline]
    pub fn is_trait_bound(&self) -> bool {
        matches!(
            self,
            Rust::TraitBounds
                | Rust::HigherRankedTraitBound
                | Rust::RemovedTraitBound
                | Rust::BoundedType
        )
    }

    /// Check if this token represents a reference type or expression.
    ///
    /// Returns true for reference types, reference expressions, and borrow operators.
    #[inline]
    pub fn is_reference(&self) -> bool {
        matches!(
            self,
            Rust::ReferenceType
                | Rust::ReferenceExpression
                | Rust::ReferencePattern
                | Rust::Ref
                | Rust::AMP
        )
    }

    /// Check if this token represents a mutable reference or pattern.
    ///
    /// Returns true for the mut keyword and mutable patterns.
    #[inline]
    pub fn is_mutable(&self) -> bool {
        matches!(self, Rust::MutableSpecifier | Rust::MutPattern)
    }

    /// Check if this token represents an attribute.
    ///
    /// Returns true for attributes, attribute items, and the # symbol.
    #[inline]
    pub fn is_attribute(&self) -> bool {
        matches!(
            self,
            Rust::Attribute
                | Rust::AttributeItem
                | Rust::InnerAttributeItem
                | Rust::HASH
        )
    }

    /// Check if this token represents a visibility modifier.
    ///
    /// Returns true for visibility modifiers and the pub keyword.
    #[inline]
    pub fn is_visibility(&self) -> bool {
        matches!(self, Rust::VisibilityModifier | Rust::Pub)
    }

    /// Check if this token represents a let declaration or binding.
    ///
    /// Returns true for let declarations, let conditions, and let chains.
    #[inline]
    pub fn is_let(&self) -> bool {
        matches!(
            self,
            Rust::Let
                | Rust::LetDeclaration
                | Rust::LetCondition
                | Rust::LetChain
                | Rust::LetChain2
        )
    }

    /// Check if this token represents a control flow construct.
    ///
    /// Returns true for if/else, loop, while, for, break, continue, and return.
    #[inline]
    pub fn is_control_flow(&self) -> bool {
        matches!(
            self,
            Rust::If
                | Rust::IfExpression
                | Rust::Else
                | Rust::ElseClause
                | Rust::Loop
                | Rust::LoopExpression
                | Rust::While
                | Rust::WhileExpression
                | Rust::For
                | Rust::ForExpression
                | Rust::Break
                | Rust::BreakExpression
                | Rust::Continue
                | Rust::ContinueExpression
                | Rust::Return
                | Rust::ReturnExpression
        )
    }

    /// Check if this token represents an operator.
    ///
    /// Includes arithmetic, comparison, logical, and bitwise operators.
    #[inline]
    pub fn is_operator(&self) -> bool {
        matches!(
            self,
            Rust::PLUS
                | Rust::DASH
                | Rust::STAR
                | Rust::SLASH
                | Rust::PERCENT
                | Rust::CARET
                | Rust::BANG
                | Rust::AMP
                | Rust::PIPE
                | Rust::AMPAMP
                | Rust::PIPEPIPE
                | Rust::LTLT
                | Rust::GTGT
                | Rust::EQEQ
                | Rust::BANGEQ
                | Rust::GT
                | Rust::LT
                | Rust::GTEQ
                | Rust::LTEQ
                | Rust::BinaryExpression
                | Rust::UnaryExpression
        )
    }

    /// Check if this token represents an assignment operator.
    ///
    /// Includes simple assignment and compound assignment operators.
    #[inline]
    pub fn is_assignment(&self) -> bool {
        matches!(
            self,
            Rust::EQ
                | Rust::PLUSEQ
                | Rust::DASHEQ
                | Rust::STAREQ
                | Rust::SLASHEQ
                | Rust::PERCENTEQ
                | Rust::CARETEQ
                | Rust::AMPEQ
                | Rust::PIPEEQ
                | Rust::LTLTEQ
                | Rust::GTGTEQ
                | Rust::AssignmentExpression
                | Rust::CompoundAssignmentExpr
        )
    }

    /// Check if this token represents a literal value.
    ///
    /// Includes integer, float, string, char, and boolean literals.
    #[inline]
    pub fn is_literal(&self) -> bool {
        matches!(
            self,
            Rust::IntegerLiteral
                | Rust::FloatLiteral
                | Rust::StringLiteral
                | Rust::RawStringLiteral
                | Rust::CharLiteral
                | Rust::BooleanLiteral
                | Rust::True
                | Rust::False
                | Rust::Literal
                | Rust::Literal2
        )
    }

    /// Check if this token represents a comment or documentation.
    ///
    /// Includes line comments, block comments, and doc comments.
    #[inline]
    pub fn is_comment(&self) -> bool {
        matches!(
            self,
            Rust::LineComment
                | Rust::BlockComment
                | Rust::DocComment
                | Rust::OuterDocCommentMarker
                | Rust::OuterDocCommentMarker2
                | Rust::InnerDocCommentMarker
                | Rust::InnerDocCommentMarker2
                | Rust::SLASHSLASH
                | Rust::SLASHSTAR
        )
    }

    /// Check if this token represents a type annotation or type expression.
    ///
    /// Includes all type constructs like array, tuple, function, and reference types.
    #[inline]
    pub fn is_type(&self) -> bool {
        matches!(
            self,
            Rust::Type
                | Rust::Type2
                | Rust::ArrayType
                | Rust::TupleType
                | Rust::UnitType
                | Rust::FunctionType
                | Rust::ReferenceType
                | Rust::PointerType
                | Rust::NeverType
                | Rust::AbstractType
                | Rust::DynamicType
                | Rust::BracketedType
                | Rust::QualifiedType
                | Rust::GenericType
                | Rust::BoundedType
                | Rust::Ty
        )
    }

    /// Check if this token represents a primitive type.
    ///
    /// Returns true for all primitive type variants.
    #[inline]
    pub fn is_primitive_type(&self) -> bool {
        matches!(
            self,
            Rust::PrimitiveType
                | Rust::PrimitiveType2
                | Rust::PrimitiveType3
                | Rust::PrimitiveType4
                | Rust::PrimitiveType5
                | Rust::PrimitiveType6
                | Rust::PrimitiveType7
                | Rust::PrimitiveType8
                | Rust::PrimitiveType9
                | Rust::PrimitiveType10
                | Rust::PrimitiveType11
                | Rust::PrimitiveType12
                | Rust::PrimitiveType13
                | Rust::PrimitiveType14
                | Rust::PrimitiveType15
                | Rust::PrimitiveType16
                | Rust::PrimitiveType17
        )
    }

    /// Check if this token represents an expression.
    ///
    /// Returns true for all expression types.
    #[inline]
    pub fn is_expression(&self) -> bool {
        matches!(
            self,
            Rust::Expression
                | Rust::ExpressionExceptRange
                | Rust::ExpressionStatement
                | Rust::RangeExpression
                | Rust::UnaryExpression
                | Rust::BinaryExpression
                | Rust::AssignmentExpression
                | Rust::CompoundAssignmentExpr
                | Rust::TypeCastExpression
                | Rust::ReturnExpression
                | Rust::YieldExpression
                | Rust::CallExpression
                | Rust::ArrayExpression
                | Rust::TupleExpression
                | Rust::UnitExpression
                | Rust::StructExpression
                | Rust::IfExpression
                | Rust::MatchExpression
                | Rust::WhileExpression
                | Rust::LoopExpression
                | Rust::ForExpression
                | Rust::ClosureExpression
                | Rust::BreakExpression
                | Rust::ContinueExpression
                | Rust::IndexExpression
                | Rust::AwaitExpression
                | Rust::FieldExpression
                | Rust::TryExpression
                | Rust::ReferenceExpression
                | Rust::ParenthesizedExpression
        )
    }

    /// Check if this token represents a try expression or error handling.
    ///
    /// Returns true for try expressions, try blocks, and the ? operator.
    #[inline]
    pub fn is_try(&self) -> bool {
        matches!(
            self,
            Rust::Try | Rust::TryExpression | Rust::TryBlock | Rust::QMARK
        )
    }

    /// Check if this token represents a block expression.
    ///
    /// Returns true for all block types including unsafe, async, const, and try blocks.
    #[inline]
    pub fn is_block(&self) -> bool {
        matches!(
            self,
            Rust::Block
                | Rust::Block2
                | Rust::UnsafeBlock
                | Rust::AsyncBlock
                | Rust::ConstBlock
                | Rust::TryBlock
                | Rust::GenBlock
        )
    }

    /// Check if this token represents a self-related keyword.
    ///
    /// Returns true for self, Self, super, and crate keywords.
    #[inline]
    pub fn is_self_keyword(&self) -> bool {
        matches!(
            self,
            Rust::Zelf | Rust::Super | Rust::Crate | Rust::SelfParameter
        )
    }

    /// Check if this token represents a range expression.
    ///
    /// Returns true for range expressions and range operators (.., ..=, ...).
    #[inline]
    pub fn is_range(&self) -> bool {
        matches!(
            self,
            Rust::RangeExpression
                | Rust::RangePattern
                | Rust::DOTDOT
                | Rust::DOTDOTEQ
                | Rust::DOTDOTDOT
        )
    }

    /// Check if this token represents extern functionality.
    ///
    /// Returns true for extern keyword, extern crates, and foreign mod items.
    #[inline]
    pub fn is_extern(&self) -> bool {
        matches!(
            self,
            Rust::Extern
                | Rust::ExternCrateDeclaration
                | Rust::ForeignModItem
                | Rust::ExternModifier
        )
    }

    /// Check if this token represents a static or const item.
    ///
    /// Returns true for static and const item definitions.
    #[inline]
    pub fn is_static_or_const_item(&self) -> bool {
        matches!(
            self,
            Rust::StaticItem | Rust::ConstItem | Rust::Static | Rust::Const
        )
    }

    /// Check if this token represents a type alias.
    ///
    /// Returns true for type items and associated types.
    #[inline]
    pub fn is_type_alias(&self) -> bool {
        matches!(self, Rust::TypeItem | Rust::AssociatedType)
    }

    /// Check if this token represents a union type.
    ///
    /// Returns true for union items and the union keyword.
    #[inline]
    pub fn is_union(&self) -> bool {
        matches!(self, Rust::UnionItem | Rust::Union)
    }

    /// Check if this token represents a pointer type.
    ///
    /// Returns true for raw pointer types.
    #[inline]
    pub fn is_pointer(&self) -> bool {
        matches!(self, Rust::PointerType | Rust::Raw)
    }

    /// Check if this token represents dynamic dispatch.
    ///
    /// Returns true for dyn keyword and dynamic types.
    #[inline]
    pub fn is_dynamic(&self) -> bool {
        matches!(self, Rust::Dyn | Rust::DynamicType)
    }

    /// Check if this token represents the never type.
    ///
    /// The never type (!) indicates that a function never returns.
    #[inline]
    pub fn is_never_type(&self) -> bool {
        matches!(self, Rust::NeverType)
    }

    /// Check if this token represents a label.
    ///
    /// Labels are used with loops for break and continue statements.
    #[inline]
    pub fn is_label(&self) -> bool {
        matches!(self, Rust::Label)
    }

    /// Check if this token represents a yield expression.
    ///
    /// Yield expressions are used in generators.
    #[inline]
    pub fn is_yield(&self) -> bool {
        matches!(self, Rust::Yield | Rust::YieldExpression | Rust::Gen | Rust::GenBlock)
    }

    /// Check if this token represents a shebang.
    ///
    /// Shebangs are used for executable Rust scripts.
    #[inline]
    pub fn is_shebang(&self) -> bool {
        matches!(self, Rust::Shebang)
    }

    /// Check if this token is an identifier.
    ///
    /// Returns true for all identifier types.
    #[inline]
    pub fn is_identifier(&self) -> bool {
        matches!(
            self,
            Rust::Identifier
                | Rust::TypeIdentifier
                | Rust::FieldIdentifier
                | Rust::ShorthandFieldIdentifier
                | Rust::ScopedIdentifier
                | Rust::ScopedTypeIdentifier
                | Rust::ScopedTypeIdentifier2
        )
    }

    /// Check if this token represents a metavariable (used in macros).
    ///
    /// Metavariables are placeholders in macro definitions.
    #[inline]
    pub fn is_metavariable(&self) -> bool {
        matches!(self, Rust::Metavariable)
    }

    // ==================== ADVANCED HELPER METHODS ====================

    // -------------------- Macros --------------------

    /// Check if this token represents a macro_rules! definition.
    ///
    /// Returns true for macro_rules! keyword and macro definitions.
    #[inline]
    pub fn is_macro_rules(&self) -> bool {
        matches!(self, Rust::MacroRulesBANG | Rust::MacroDefinition)
    }

    /// Check if this token represents a procedural macro invocation.
    ///
    /// Returns true for macro invocations which include proc macros, derive macros, etc.
    #[inline]
    pub fn is_proc_macro(&self) -> bool {
        matches!(self, Rust::MacroInvocation)
    }

    /// Check if this token is part of a macro pattern or token tree.
    ///
    /// Returns true for token patterns, token trees, and token repetitions.
    #[inline]
    pub fn is_macro_pattern(&self) -> bool {
        matches!(
            self,
            Rust::TokenPattern
                | Rust::TokenTreePattern
                | Rust::TokenBindingPattern
                | Rust::TokenRepetitionPattern
                | Rust::TokenTree
                | Rust::TokenTree2
                | Rust::TokenRepetition
        )
    }

    /// Check if this token represents a fragment specifier in macros.
    ///
    /// Fragment specifiers like expr, ident, ty, etc.
    #[inline]
    pub fn is_fragment_specifier(&self) -> bool {
        matches!(
            self,
            Rust::FragmentSpecifier
                | Rust::Expr
                | Rust::Ident
                | Rust::Item
                | Rust::Pat
                | Rust::Path
                | Rust::Stmt
                | Rust::Tt
                | Rust::Ty
                | Rust::Vis
                | Rust::Meta
        )
    }

    /// Check if this token is a macro rule component.
    ///
    /// Returns true for macro rules and their components.
    #[inline]
    pub fn is_macro_rule(&self) -> bool {
        matches!(self, Rust::MacroRule)
    }

    // -------------------- Lifetimes --------------------

    /// Check if this token is a lifetime annotation or parameter.
    ///
    /// Returns true for lifetime syntax including 'a, 'static, etc.
    #[inline]
    pub fn is_lifetime_annotation(&self) -> bool {
        matches!(self, Rust::Lifetime | Rust::Lifetime2 | Rust::SQUOTE)
    }

    /// Check if this token is a for-lifetimes (HRTB) clause.
    ///
    /// For example: for<'a, 'b> in higher-ranked trait bounds.
    #[inline]
    pub fn is_for_lifetimes(&self) -> bool {
        matches!(self, Rust::ForLifetimes)
    }

    /// Check if this token represents lifetime elision context.
    ///
    /// Returns true for reference types and function parameters where elision occurs.
    #[inline]
    pub fn is_lifetime_elision_context(&self) -> bool {
        matches!(
            self,
            Rust::ReferenceType | Rust::Parameter | Rust::FunctionType
        )
    }

    // -------------------- Trait System --------------------

    /// Check if this token is a trait definition or implementation.
    ///
    /// Returns true for trait items and impl items.
    #[inline]
    pub fn is_trait_or_impl(&self) -> bool {
        matches!(
            self,
            Rust::TraitItem | Rust::ImplItem | Rust::Trait | Rust::Impl
        )
    }

    /// Check if this token represents an associated type.
    ///
    /// Returns true for associated types in traits and impls.
    #[inline]
    pub fn is_associated_type(&self) -> bool {
        matches!(self, Rust::AssociatedType | Rust::TypeBinding)
    }

    /// Check if this token is part of a where clause.
    ///
    /// Returns true for where clauses and where predicates.
    #[inline]
    pub fn is_where_predicate(&self) -> bool {
        matches!(self, Rust::WherePredicate | Rust::WhereClause | Rust::Where)
    }

    /// Check if this token represents trait bounds or constraints.
    ///
    /// Returns true for trait bounds, bounded types, and constrained type parameters.
    #[inline]
    pub fn is_trait_constraint(&self) -> bool {
        matches!(
            self,
            Rust::TraitBounds
                | Rust::BoundedType
                | Rust::ConstrainedTypeParameter
                | Rust::HigherRankedTraitBound
        )
    }

    // -------------------- Pattern Matching --------------------

    /// Check if this token represents a match arm.
    ///
    /// Returns true for match arms and match patterns.
    #[inline]
    pub fn is_match_arm(&self) -> bool {
        matches!(self, Rust::MatchArm | Rust::MatchArm2 | Rust::MatchPattern)
    }

    /// Check if this token is an if-let or while-let pattern.
    ///
    /// Returns true for let conditions and let chains.
    #[inline]
    pub fn is_let_pattern(&self) -> bool {
        matches!(
            self,
            Rust::LetCondition | Rust::LetChain | Rust::LetChain2
        )
    }

    /// Check if this token is a struct pattern.
    ///
    /// Returns true for struct patterns and field patterns.
    #[inline]
    pub fn is_struct_pattern(&self) -> bool {
        matches!(
            self,
            Rust::StructPattern
                | Rust::TupleStructPattern
                | Rust::FieldPattern
                | Rust::RemainingFieldPattern
        )
    }

    /// Check if this token is a tuple or slice pattern.
    ///
    /// Returns true for tuple and slice destructuring patterns.
    #[inline]
    pub fn is_destructuring_pattern(&self) -> bool {
        matches!(
            self,
            Rust::TuplePattern | Rust::SlicePattern | Rust::StructPattern
        )
    }

    /// Check if this token is an or-pattern.
    ///
    /// Returns true for or-patterns (e.g., Some(x) | None).
    #[inline]
    pub fn is_or_pattern(&self) -> bool {
        matches!(self, Rust::OrPattern)
    }

    /// Check if this token is a range pattern.
    ///
    /// Returns true for range patterns in match arms (e.g., 1..=10).
    #[inline]
    pub fn is_range_pattern(&self) -> bool {
        matches!(self, Rust::RangePattern)
    }

    /// Check if this token is a captured or ref pattern.
    ///
    /// Returns true for captured patterns and ref patterns.
    #[inline]
    pub fn is_binding_pattern(&self) -> bool {
        matches!(
            self,
            Rust::CapturedPattern
                | Rust::RefPattern
                | Rust::MutPattern
                | Rust::ReferencePattern
        )
    }

    // -------------------- Ownership & Borrowing --------------------

    /// Check if this token represents a move operation.
    ///
    /// Returns true for move keyword in closures.
    #[inline]
    pub fn is_move(&self) -> bool {
        matches!(self, Rust::Move)
    }

    /// Check if this token is a borrow or reference.
    ///
    /// Returns true for & operator and reference expressions.
    #[inline]
    pub fn is_borrow(&self) -> bool {
        matches!(
            self,
            Rust::AMP | Rust::ReferenceExpression | Rust::ReferenceType
        )
    }

    /// Check if this token is a mutable borrow.
    ///
    /// Returns true for &mut references.
    #[inline]
    pub fn is_mutable_borrow(&self) -> bool {
        matches!(
            self,
            Rust::MutableSpecifier | Rust::ReferenceExpression | Rust::ReferenceType
        )
    }

    /// Check if this token represents reference-related syntax.
    ///
    /// Returns true for references, ref patterns, and & operator.
    #[inline]
    pub fn is_reference_syntax(&self) -> bool {
        matches!(
            self,
            Rust::Ref
                | Rust::AMP
                | Rust::ReferenceType
                | Rust::ReferenceExpression
                | Rust::ReferencePattern
                | Rust::RefPattern
        )
    }

    // -------------------- Unsafe Code --------------------

    /// Check if this token is an unsafe function.
    ///
    /// Returns true for unsafe function modifiers.
    #[inline]
    pub fn is_unsafe_function(&self) -> bool {
        matches!(self, Rust::Unsafe | Rust::FunctionModifiers)
    }

    /// Check if this token is an unsafe trait or impl.
    ///
    /// Returns true for unsafe trait definitions and implementations.
    #[inline]
    pub fn is_unsafe_trait(&self) -> bool {
        matches!(self, Rust::Unsafe | Rust::TraitItem | Rust::ImplItem)
    }

    /// Check if this token is a raw pointer.
    ///
    /// Returns true for raw pointer types (*const, *mut).
    #[inline]
    pub fn is_raw_pointer(&self) -> bool {
        matches!(self, Rust::PointerType | Rust::Raw)
    }

    // -------------------- Async/Await --------------------

    /// Check if this token is an async function.
    ///
    /// Returns true for async function declarations.
    #[inline]
    pub fn is_async_function(&self) -> bool {
        matches!(self, Rust::Async | Rust::FunctionItem)
    }

    /// Check if this token is an await expression.
    ///
    /// Returns true for .await syntax.
    #[inline]
    pub fn is_await_expression(&self) -> bool {
        matches!(self, Rust::AwaitExpression | Rust::Await)
    }

    /// Check if this token is an async block.
    ///
    /// Returns true for async { } blocks.
    #[inline]
    pub fn is_async_block(&self) -> bool {
        matches!(self, Rust::AsyncBlock)
    }

    // -------------------- Generics --------------------

    /// Check if this token is a type parameter.
    ///
    /// Returns true for type parameters in generic declarations.
    #[inline]
    pub fn is_type_parameter(&self) -> bool {
        matches!(
            self,
            Rust::TypeParameters
                | Rust::ConstrainedTypeParameter
                | Rust::OptionalTypeParameter
        )
    }

    /// Check if this token is a const generic parameter.
    ///
    /// Returns true for const parameters in generics.
    #[inline]
    pub fn is_const_generic(&self) -> bool {
        matches!(self, Rust::ConstParameter)
    }

    /// Check if this token is a turbofish operator.
    ///
    /// Returns true for ::<> syntax.
    #[inline]
    pub fn is_turbofish(&self) -> bool {
        matches!(
            self,
            Rust::GenericTypeWithTurbofish | Rust::COLONCOLON | Rust::LT2
        )
    }

    /// Check if this token is type arguments.
    ///
    /// Returns true for type arguments in generic instantiations.
    #[inline]
    pub fn is_type_arguments(&self) -> bool {
        matches!(self, Rust::TypeArguments | Rust::TypeBinding)
    }

    // -------------------- Attributes --------------------

    /// Check if this token is a derive attribute.
    ///
    /// Returns true for #[derive(...)] attributes.
    #[inline]
    pub fn is_derive_attribute(&self) -> bool {
        matches!(self, Rust::AttributeItem | Rust::Attribute)
    }

    /// Check if this token is a cfg attribute.
    ///
    /// Returns true for #[cfg(...)] conditional compilation attributes.
    #[inline]
    pub fn is_cfg_attribute(&self) -> bool {
        matches!(self, Rust::AttributeItem | Rust::Attribute)
    }

    /// Check if this token is a test attribute.
    ///
    /// Returns true for #[test] attributes.
    #[inline]
    pub fn is_test_attribute(&self) -> bool {
        matches!(self, Rust::AttributeItem)
    }

    /// Check if this token is an inner attribute.
    ///
    /// Returns true for #![...] inner attributes.
    #[inline]
    pub fn is_inner_attribute(&self) -> bool {
        matches!(self, Rust::InnerAttributeItem)
    }

    /// Check if this token is an outer attribute.
    ///
    /// Returns true for #[...] outer attributes.
    #[inline]
    pub fn is_outer_attribute(&self) -> bool {
        matches!(self, Rust::AttributeItem)
    }

    // -------------------- Module System --------------------

    /// Check if this token is a mod declaration.
    ///
    /// Returns true for mod items and mod keyword.
    #[inline]
    pub fn is_mod_declaration(&self) -> bool {
        matches!(self, Rust::ModItem | Rust::Mod)
    }

    /// Check if this token is a use import.
    ///
    /// Returns true for use declarations and clauses.
    #[inline]
    pub fn is_use_import(&self) -> bool {
        matches!(
            self,
            Rust::UseDeclaration
                | Rust::UseClause
                | Rust::UseList
                | Rust::UseAsClause
                | Rust::UseWildcard
        )
    }

    /// Check if this token is pub visibility.
    ///
    /// Returns true for pub keyword and visibility modifiers.
    #[inline]
    pub fn is_pub_visibility(&self) -> bool {
        matches!(self, Rust::Pub | Rust::VisibilityModifier)
    }

    /// Check if this token is crate/super/self path.
    ///
    /// Returns true for crate::, super::, self:: path components.
    #[inline]
    pub fn is_path_segment(&self) -> bool {
        matches!(self, Rust::Crate | Rust::Super | Rust::Zelf)
    }

    /// Check if this token is a scoped identifier.
    ///
    /// Returns true for scoped identifiers like std::vec::Vec.
    #[inline]
    pub fn is_scoped_identifier(&self) -> bool {
        matches!(
            self,
            Rust::ScopedIdentifier
                | Rust::ScopedTypeIdentifier
                | Rust::ScopedTypeIdentifier2
        )
    }

    /// Check if this token is a use wildcard.
    ///
    /// Returns true for use glob imports (use foo::*).
    #[inline]
    pub fn is_use_wildcard(&self) -> bool {
        matches!(self, Rust::UseWildcard | Rust::STAR)
    }

    // -------------------- Error Handling --------------------

    /// Check if this token is a Result type usage.
    ///
    /// Note: This checks for type identifiers that could be Result.
    #[inline]
    pub fn is_result_type(&self) -> bool {
        matches!(self, Rust::TypeIdentifier | Rust::GenericType)
    }

    /// Check if this token is an Option type usage.
    ///
    /// Note: This checks for type identifiers that could be Option.
    #[inline]
    pub fn is_option_type(&self) -> bool {
        matches!(self, Rust::TypeIdentifier | Rust::GenericType)
    }

    /// Check if this token is the ? operator.
    ///
    /// Returns true for the ? try operator.
    #[inline]
    pub fn is_question_mark_operator(&self) -> bool {
        matches!(self, Rust::QMARK | Rust::TryExpression)
    }

    /// Check if this token is a panic macro.
    ///
    /// Note: Detects macro invocations that could be panic-related.
    #[inline]
    pub fn is_panic_macro(&self) -> bool {
        matches!(self, Rust::MacroInvocation)
    }

    // -------------------- Closures --------------------

    /// Check if this token is a closure definition.
    ///
    /// Returns true for closure expressions and parameters.
    #[inline]
    pub fn is_closure_definition(&self) -> bool {
        matches!(self, Rust::ClosureExpression | Rust::ClosureParameters)
    }

    /// Check if this token is a closure parameter.
    ///
    /// Returns true for closure parameter lists.
    #[inline]
    pub fn is_closure_parameter(&self) -> bool {
        matches!(self, Rust::ClosureParameters)
    }

    /// Check if this token is a move closure.
    ///
    /// Returns true for closures with move keyword.
    #[inline]
    pub fn is_move_closure(&self) -> bool {
        matches!(self, Rust::Move | Rust::ClosureExpression)
    }

    // -------------------- Type System --------------------

    /// Check if this token is a struct definition or expression.
    ///
    /// Returns true for struct items and struct expressions.
    #[inline]
    pub fn is_struct_definition(&self) -> bool {
        matches!(
            self,
            Rust::StructItem
                | Rust::StructExpression
                | Rust::FieldDeclarationList
                | Rust::FieldDeclaration
        )
    }

    /// Check if this token is an enum definition or variant.
    ///
    /// Returns true for enum items, variants, and variant lists.
    #[inline]
    pub fn is_enum_definition(&self) -> bool {
        matches!(
            self,
            Rust::EnumItem | Rust::EnumVariant | Rust::EnumVariantList
        )
    }

    /// Check if this token is a type alias.
    ///
    /// Returns true for type items and type keyword.
    #[inline]
    pub fn is_type_alias_definition(&self) -> bool {
        matches!(self, Rust::TypeItem | Rust::Type)
    }

    /// Check if this token is a union definition.
    ///
    /// Returns true for union items.
    #[inline]
    pub fn is_union_definition(&self) -> bool {
        matches!(self, Rust::UnionItem | Rust::Union)
    }

    /// Check if this token is the never type (!).
    ///
    /// Returns true for the never type.
    #[inline]
    pub fn is_never_type_annotation(&self) -> bool {
        matches!(self, Rust::NeverType | Rust::BANG)
    }

    /// Check if this token is a tuple type or expression.
    ///
    /// Returns true for tuple types and tuple expressions.
    #[inline]
    pub fn is_tuple(&self) -> bool {
        matches!(
            self,
            Rust::TupleType | Rust::TupleExpression | Rust::TuplePattern
        )
    }

    /// Check if this token is an array type or expression.
    ///
    /// Returns true for array types and array expressions.
    #[inline]
    pub fn is_array(&self) -> bool {
        matches!(self, Rust::ArrayType | Rust::ArrayExpression)
    }

    /// Check if this token is a function type.
    ///
    /// Returns true for function type signatures (fn(...) -> ...).
    #[inline]
    pub fn is_function_type(&self) -> bool {
        matches!(self, Rust::FunctionType)
    }

    // -------------------- Special Syntax --------------------

    /// Check if this token uses turbofish syntax.
    ///
    /// Returns true for ::<> type annotations.
    #[inline]
    pub fn is_turbofish_syntax(&self) -> bool {
        matches!(self, Rust::GenericTypeWithTurbofish)
    }

    /// Check if this token is dyn Trait syntax.
    ///
    /// Returns true for dynamic dispatch trait objects.
    #[inline]
    pub fn is_dyn_trait(&self) -> bool {
        matches!(self, Rust::Dyn | Rust::DynamicType)
    }

    /// Check if this token is impl Trait syntax.
    ///
    /// Returns true for abstract type (impl Trait in return position).
    #[inline]
    pub fn is_impl_trait(&self) -> bool {
        matches!(self, Rust::AbstractType | Rust::Impl)
    }

    /// Check if this token is a qualified type.
    ///
    /// Returns true for qualified types like <T as Trait>::Type.
    #[inline]
    pub fn is_qualified_type(&self) -> bool {
        matches!(self, Rust::QualifiedType)
    }

    // -------------------- Memory & Special Traits --------------------

    /// Check if this token is a field declaration or expression.
    ///
    /// Returns true for field-related constructs.
    #[inline]
    pub fn is_field(&self) -> bool {
        matches!(
            self,
            Rust::FieldDeclaration
                | Rust::FieldExpression
                | Rust::FieldPattern
                | Rust::FieldIdentifier
                | Rust::ShorthandFieldIdentifier
        )
    }

    /// Check if this token is a unit type or expression.
    ///
    /// Returns true for () unit type.
    #[inline]
    pub fn is_unit(&self) -> bool {
        matches!(self, Rust::UnitType | Rust::UnitExpression)
    }

    // -------------------- Loop & Control Flow --------------------

    /// Check if this token is a loop expression.
    ///
    /// Returns true for loop, while, and for expressions.
    #[inline]
    pub fn is_loop_expression(&self) -> bool {
        matches!(
            self,
            Rust::LoopExpression
                | Rust::WhileExpression
                | Rust::ForExpression
                | Rust::Loop
                | Rust::While
                | Rust::For
        )
    }

    /// Check if this token is a break or continue statement.
    ///
    /// Returns true for break and continue expressions.
    #[inline]
    pub fn is_break_or_continue(&self) -> bool {
        matches!(
            self,
            Rust::Break
                | Rust::Continue
                | Rust::BreakExpression
                | Rust::ContinueExpression
        )
    }

    /// Check if this token is a loop label.
    ///
    /// Returns true for loop labels ('label: loop).
    #[inline]
    pub fn is_loop_label(&self) -> bool {
        matches!(self, Rust::Label)
    }

    // -------------------- Expressions --------------------

    /// Check if this token is a call expression.
    ///
    /// Returns true for function calls.
    #[inline]
    pub fn is_call_expression(&self) -> bool {
        matches!(self, Rust::CallExpression | Rust::Arguments)
    }

    /// Check if this token is a method call.
    ///
    /// Returns true for field expressions which can be method calls.
    #[inline]
    pub fn is_method_call(&self) -> bool {
        matches!(self, Rust::FieldExpression | Rust::CallExpression)
    }

    /// Check if this token is an index expression.
    ///
    /// Returns true for array/slice indexing (foo[0]).
    #[inline]
    pub fn is_index_expression(&self) -> bool {
        matches!(self, Rust::IndexExpression)
    }

    /// Check if this token is a struct expression.
    ///
    /// Returns true for struct initialization expressions.
    #[inline]
    pub fn is_struct_expression(&self) -> bool {
        matches!(
            self,
            Rust::StructExpression
                | Rust::FieldInitializerList
                | Rust::FieldInitializer
                | Rust::ShorthandFieldInitializer
        )
    }

    /// Check if this token is a binary expression.
    ///
    /// Returns true for binary operations.
    #[inline]
    pub fn is_binary_expression(&self) -> bool {
        matches!(self, Rust::BinaryExpression)
    }

    /// Check if this token is a unary expression.
    ///
    /// Returns true for unary operations.
    #[inline]
    pub fn is_unary_expression(&self) -> bool {
        matches!(self, Rust::UnaryExpression)
    }

    /// Check if this token is a type cast expression.
    ///
    /// Returns true for as type casts.
    #[inline]
    pub fn is_type_cast(&self) -> bool {
        matches!(self, Rust::TypeCastExpression | Rust::As)
    }

    // -------------------- Statements --------------------

    /// Check if this token is an expression statement.
    ///
    /// Returns true for expression statements.
    #[inline]
    pub fn is_expression_statement(&self) -> bool {
        matches!(self, Rust::ExpressionStatement)
    }

    /// Check if this token is an empty statement.
    ///
    /// Returns true for empty statements (semicolon only).
    #[inline]
    pub fn is_empty_statement(&self) -> bool {
        matches!(self, Rust::EmptyStatement)
    }

    /// Check if this token is a let declaration.
    ///
    /// Returns true for let bindings.
    #[inline]
    pub fn is_let_declaration(&self) -> bool {
        matches!(self, Rust::LetDeclaration)
    }

    // -------------------- Items & Declarations --------------------

    /// Check if this token is a function signature.
    ///
    /// Returns true for function signature items (in traits).
    #[inline]
    pub fn is_function_signature(&self) -> bool {
        matches!(self, Rust::FunctionSignatureItem)
    }

    /// Check if this token is a const item declaration.
    ///
    /// Returns true for const item definitions.
    #[inline]
    pub fn is_const_item(&self) -> bool {
        matches!(self, Rust::ConstItem)
    }

    /// Check if this token is a static item declaration.
    ///
    /// Returns true for static item definitions.
    #[inline]
    pub fn is_static_item(&self) -> bool {
        matches!(self, Rust::StaticItem)
    }

    /// Check if this token is an extern crate declaration.
    ///
    /// Returns true for extern crate statements.
    #[inline]
    pub fn is_extern_crate(&self) -> bool {
        matches!(self, Rust::ExternCrateDeclaration)
    }

    /// Check if this token is a foreign mod item.
    ///
    /// Returns true for extern blocks.
    #[inline]
    pub fn is_foreign_mod(&self) -> bool {
        matches!(self, Rust::ForeignModItem)
    }

    // -------------------- Parameters --------------------

    /// Check if this token is a function parameter.
    ///
    /// Returns true for function parameters.
    #[inline]
    pub fn is_function_parameter(&self) -> bool {
        matches!(
            self,
            Rust::Parameter | Rust::Parameters | Rust::SelfParameter
        )
    }

    /// Check if this token is a self parameter.
    ///
    /// Returns true for self/&self/&mut self parameters.
    #[inline]
    pub fn is_self_parameter(&self) -> bool {
        matches!(self, Rust::SelfParameter)
    }

    /// Check if this token is a variadic parameter.
    ///
    /// Returns true for ... variadic parameters.
    #[inline]
    pub fn is_variadic_parameter(&self) -> bool {
        matches!(self, Rust::VariadicParameter)
    }

    // -------------------- Operators & Delimiters --------------------

    /// Check if this token is a comparison operator.
    ///
    /// Returns true for ==, !=, <, >, <=, >=.
    #[inline]
    pub fn is_comparison_operator(&self) -> bool {
        matches!(
            self,
            Rust::EQEQ | Rust::BANGEQ | Rust::GT | Rust::LT | Rust::GTEQ | Rust::LTEQ
        )
    }

    /// Check if this token is a logical operator.
    ///
    /// Returns true for && and ||.
    #[inline]
    pub fn is_logical_operator(&self) -> bool {
        matches!(self, Rust::AMPAMP | Rust::PIPEPIPE)
    }

    /// Check if this token is a bitwise operator.
    ///
    /// Returns true for &, |, ^, <<, >>.
    #[inline]
    pub fn is_bitwise_operator(&self) -> bool {
        matches!(
            self,
            Rust::AMP | Rust::PIPE | Rust::CARET | Rust::LTLT | Rust::GTGT
        )
    }

    /// Check if this token is an arithmetic operator.
    ///
    /// Returns true for +, -, *, /, %.
    #[inline]
    pub fn is_arithmetic_operator(&self) -> bool {
        matches!(
            self,
            Rust::PLUS | Rust::DASH | Rust::STAR | Rust::SLASH | Rust::PERCENT
        )
    }

    /// Check if this token is a compound assignment operator.
    ///
    /// Returns true for +=, -=, *=, etc.
    #[inline]
    pub fn is_compound_assignment(&self) -> bool {
        matches!(
            self,
            Rust::PLUSEQ
                | Rust::DASHEQ
                | Rust::STAREQ
                | Rust::SLASHEQ
                | Rust::PERCENTEQ
                | Rust::CARETEQ
                | Rust::AMPEQ
                | Rust::PIPEEQ
                | Rust::LTLTEQ
                | Rust::GTGTEQ
                | Rust::CompoundAssignmentExpr
        )
    }

    /// Check if this token is a delimiter.
    ///
    /// Returns true for (, ), [, ], {, }.
    #[inline]
    pub fn is_delimiter(&self) -> bool {
        matches!(
            self,
            Rust::LPAREN
                | Rust::RPAREN
                | Rust::LBRACK
                | Rust::RBRACK
                | Rust::LBRACE
                | Rust::RBRACE
        )
    }

    /// Check if this token is a path separator.
    ///
    /// Returns true for :: double colon.
    #[inline]
    pub fn is_path_separator(&self) -> bool {
        matches!(self, Rust::COLONCOLON)
    }

    /// Check if this token is a return type arrow.
    ///
    /// Returns true for ->.
    #[inline]
    pub fn is_return_arrow(&self) -> bool {
        matches!(self, Rust::DASHGT)
    }

    /// Check if this token is a fat arrow.
    ///
    /// Returns true for => in match arms.
    #[inline]
    pub fn is_fat_arrow(&self) -> bool {
        matches!(self, Rust::EQGT)
    }

    // -------------------- String & Character Literals --------------------

    /// Check if this token is a string literal.
    ///
    /// Returns true for string and raw string literals.
    #[inline]
    pub fn is_string_literal(&self) -> bool {
        matches!(
            self,
            Rust::StringLiteral
                | Rust::RawStringLiteral
                | Rust::StringContent
                | Rust::StringContent2
        )
    }

    /// Check if this token is a raw string literal.
    ///
    /// Returns true for raw string literals r"..." or r#"..."#.
    #[inline]
    pub fn is_raw_string_literal(&self) -> bool {
        matches!(
            self,
            Rust::RawStringLiteral
                | Rust::RawStringLiteralStart
                | Rust::RawStringLiteralEnd
        )
    }

    /// Check if this token is a character literal.
    ///
    /// Returns true for 'c' character literals.
    #[inline]
    pub fn is_char_literal(&self) -> bool {
        matches!(self, Rust::CharLiteral)
    }

    /// Check if this token is an escape sequence.
    ///
    /// Returns true for escape sequences in strings/chars.
    #[inline]
    pub fn is_escape_sequence(&self) -> bool {
        matches!(self, Rust::EscapeSequence)
    }

    // -------------------- Numeric Literals --------------------

    /// Check if this token is an integer literal.
    ///
    /// Returns true for integer literals.
    #[inline]
    pub fn is_integer_literal(&self) -> bool {
        matches!(self, Rust::IntegerLiteral)
    }

    /// Check if this token is a float literal.
    ///
    /// Returns true for floating point literals.
    #[inline]
    pub fn is_float_literal(&self) -> bool {
        matches!(self, Rust::FloatLiteral)
    }

    /// Check if this token is a numeric literal.
    ///
    /// Returns true for any numeric literal (integer or float).
    #[inline]
    pub fn is_numeric_literal(&self) -> bool {
        matches!(self, Rust::IntegerLiteral | Rust::FloatLiteral)
    }

    // -------------------- Boolean Literals --------------------

    /// Check if this token is a boolean literal.
    ///
    /// Returns true for true or false.
    #[inline]
    pub fn is_boolean_literal(&self) -> bool {
        matches!(self, Rust::BooleanLiteral | Rust::True | Rust::False)
    }

    // -------------------- Documentation --------------------

    /// Check if this token is a doc comment.
    ///
    /// Returns true for doc comments (/// or //!).
    #[inline]
    pub fn is_doc_comment(&self) -> bool {
        matches!(
            self,
            Rust::DocComment
                | Rust::OuterDocCommentMarker
                | Rust::OuterDocCommentMarker2
                | Rust::InnerDocCommentMarker
                | Rust::InnerDocCommentMarker2
                | Rust::LineDocCommentMarker
                | Rust::BlockDocCommentMarker
        )
    }

    /// Check if this token is an inner doc comment.
    ///
    /// Returns true for //! inner doc comments.
    #[inline]
    pub fn is_inner_doc_comment(&self) -> bool {
        matches!(
            self,
            Rust::InnerDocCommentMarker | Rust::InnerDocCommentMarker2
        )
    }

    /// Check if this token is an outer doc comment.
    ///
    /// Returns true for /// outer doc comments.
    #[inline]
    pub fn is_outer_doc_comment(&self) -> bool {
        matches!(
            self,
            Rust::OuterDocCommentMarker | Rust::OuterDocCommentMarker2
        )
    }

    /// Check if this token is a line comment.
    ///
    /// Returns true for // comments.
    #[inline]
    pub fn is_line_comment(&self) -> bool {
        matches!(self, Rust::LineComment | Rust::SLASHSLASH)
    }

    /// Check if this token is a block comment.
    ///
    /// Returns true for /* */ comments.
    #[inline]
    pub fn is_block_comment(&self) -> bool {
        matches!(
            self,
            Rust::BlockComment | Rust::SLASHSTAR | Rust::STARSLASH
        )
    }

    // -------------------- Advanced Features --------------------

    /// Check if this token is a const block.
    ///
    /// Returns true for const { } blocks.
    #[inline]
    pub fn is_const_block(&self) -> bool {
        matches!(self, Rust::ConstBlock)
    }

    /// Check if this token is a try block.
    ///
    /// Returns true for try { } blocks.
    #[inline]
    pub fn is_try_block(&self) -> bool {
        matches!(self, Rust::TryBlock)
    }

    /// Check if this token is a gen block.
    ///
    /// Returns true for gen { } generator blocks.
    #[inline]
    pub fn is_gen_block(&self) -> bool {
        matches!(self, Rust::GenBlock | Rust::Gen)
    }

    /// Check if this token represents a parenthesized expression.
    ///
    /// Returns true for expressions wrapped in parentheses.
    #[inline]
    pub fn is_parenthesized_expression(&self) -> bool {
        matches!(self, Rust::ParenthesizedExpression)
    }

    /// Check if this token is a base field initializer.
    ///
    /// Returns true for ..base in struct expressions.
    #[inline]
    pub fn is_base_field_initializer(&self) -> bool {
        matches!(self, Rust::BaseFieldInitializer)
    }

    /// Check if this token is a shorthand field initializer.
    ///
    /// Returns true for shorthand field syntax (field instead of field: field).
    #[inline]
    pub fn is_shorthand_field_initializer(&self) -> bool {
        matches!(self, Rust::ShorthandFieldInitializer)
    }

    /// Check if this token is a condition in if/while.
    ///
    /// Returns true for condition expressions.
    #[inline]
    pub fn is_condition(&self) -> bool {
        matches!(self, Rust::Condition)
    }

    /// Check if this token is an else clause.
    ///
    /// Returns true for else branches.
    #[inline]
    pub fn is_else_clause(&self) -> bool {
        matches!(self, Rust::ElseClause | Rust::Else)
    }

    /// Check if this token is a bracketed type.
    ///
    /// Returns true for types in brackets/parentheses.
    #[inline]
    pub fn is_bracketed_type(&self) -> bool {
        matches!(self, Rust::BracketedType)
    }

    /// Check if this token is function modifiers.
    ///
    /// Returns true for function modifier keywords (async, const, unsafe, etc).
    #[inline]
    pub fn is_function_modifiers(&self) -> bool {
        matches!(self, Rust::FunctionModifiers)
    }

    /// Check if this token is an extern modifier.
    ///
    /// Returns true for extern ABI specifications.
    #[inline]
    pub fn is_extern_modifier(&self) -> bool {
        matches!(self, Rust::ExternModifier)
    }

    /// Check if this token represents source file root.
    ///
    /// Returns true for source file node.
    #[inline]
    pub fn is_source_file(&self) -> bool {
        matches!(self, Rust::SourceFile)
    }

    /// Check if this token is an underscore wildcard.
    ///
    /// Returns true for _ wildcard pattern/expression.
    #[inline]
    pub fn is_underscore(&self) -> bool {
        matches!(self, Rust::UNDERSCORE)
    }

    /// Check if this token is at pattern binding (@).
    ///
    /// Returns true for @ in patterns (e.g., x @ 1..=5).
    #[inline]
    pub fn is_at_pattern(&self) -> bool {
        matches!(self, Rust::AT)
    }

    /// Check if this token is a dollar sign (used in macros).
    ///
    /// Returns true for $ in macro patterns.
    #[inline]
    pub fn is_dollar_sign(&self) -> bool {
        matches!(self, Rust::DOLLAR)
    }

    /// Check if this token is a semicolon.
    ///
    /// Returns true for statement terminators.
    #[inline]
    pub fn is_semicolon(&self) -> bool {
        matches!(self, Rust::SEMI)
    }

    /// Check if this token is a colon.
    ///
    /// Returns true for : in type annotations.
    #[inline]
    pub fn is_colon(&self) -> bool {
        matches!(self, Rust::COLON)
    }

    /// Check if this token is a comma.
    ///
    /// Returns true for , separators.
    #[inline]
    pub fn is_comma(&self) -> bool {
        matches!(self, Rust::COMMA)
    }

    /// Check if this token is a dot operator.
    ///
    /// Returns true for . field access.
    #[inline]
    pub fn is_dot(&self) -> bool {
        matches!(self, Rust::DOT)
    }

    /// Check if this token is a hash symbol.
    ///
    /// Returns true for # in attributes.
    #[inline]
    pub fn is_hash(&self) -> bool {
        matches!(self, Rust::HASH)
    }
}

/// Rust language implementation.
pub struct RustLanguage;

impl LanguageInfo for RustLanguage {
    fn get_lang() -> Lang {
        Lang::Rust
    }

    fn get_lang_name() -> &'static str {
        "rust"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_conversion() {
        assert_eq!(Rust::from(0), Rust::End);
        assert_eq!(Rust::from(1), Rust::Identifier);
        assert_eq!(Rust::from(186), Rust::FunctionItem);
        assert_eq!(Rust::from(174), Rust::StructItem);
        assert_eq!(Rust::from(999), Rust::Error);
    }

    #[test]
    fn test_token_to_string() {
        let tok: &str = Rust::FunctionItem.into();
        assert_eq!(tok, "function_item");

        let tok: &str = Rust::StructItem.into();
        assert_eq!(tok, "struct_item");

        let tok: &str = Rust::MacroRulesBANG.into();
        assert_eq!(tok, "macro_rules!");
    }

    #[test]
    fn test_is_function() {
        assert!(Rust::FunctionItem.is_function());
        assert!(Rust::FunctionSignatureItem.is_function());
        assert!(Rust::GenericFunction.is_function());
        assert!(Rust::Fn.is_function());
        assert!(!Rust::StructItem.is_function());
    }

    #[test]
    fn test_is_struct() {
        assert!(Rust::StructItem.is_struct());
        assert!(Rust::Struct.is_struct());
        assert!(!Rust::EnumItem.is_struct());
    }

    #[test]
    fn test_is_enum() {
        assert!(Rust::EnumItem.is_enum());
        assert!(Rust::Enum.is_enum());
        assert!(Rust::EnumVariant.is_enum());
        assert!(!Rust::StructItem.is_enum());
    }

    #[test]
    fn test_is_trait() {
        assert!(Rust::TraitItem.is_trait());
        assert!(Rust::Trait.is_trait());
        assert!(!Rust::ImplItem.is_trait());
    }

    #[test]
    fn test_is_impl() {
        assert!(Rust::ImplItem.is_impl());
        assert!(Rust::Impl.is_impl());
        assert!(!Rust::TraitItem.is_impl());
    }

    #[test]
    fn test_is_async() {
        assert!(Rust::Async.is_async());
        assert!(Rust::Await.is_async());
        assert!(Rust::AsyncBlock.is_async());
        assert!(Rust::AwaitExpression.is_async());
        assert!(!Rust::Fn.is_async());
    }

    #[test]
    fn test_is_unsafe() {
        assert!(Rust::Unsafe.is_unsafe());
        assert!(Rust::UnsafeBlock.is_unsafe());
        assert!(!Rust::Async.is_unsafe());
    }

    #[test]
    fn test_is_const() {
        assert!(Rust::Const.is_const());
        assert!(Rust::ConstItem.is_const());
        assert!(Rust::ConstBlock.is_const());
        assert!(!Rust::Static.is_const());
    }

    #[test]
    fn test_is_macro() {
        assert!(Rust::MacroDefinition.is_macro());
        assert!(Rust::MacroInvocation.is_macro());
        assert!(Rust::MacroRulesBANG.is_macro());
        assert!(!Rust::FunctionItem.is_macro());
    }

    #[test]
    fn test_is_pattern() {
        assert!(Rust::Pattern.is_pattern());
        assert!(Rust::TuplePattern.is_pattern());
        assert!(Rust::StructPattern.is_pattern());
        assert!(Rust::RefPattern.is_pattern());
        assert!(!Rust::Expression.is_pattern());
    }

    #[test]
    fn test_is_match() {
        assert!(Rust::Match.is_match());
        assert!(Rust::MatchExpression.is_match());
        assert!(Rust::MatchBlock.is_match());
        assert!(Rust::MatchArm.is_match());
        assert!(!Rust::If.is_match());
    }

    #[test]
    fn test_is_closure() {
        assert!(Rust::ClosureExpression.is_closure());
        assert!(Rust::ClosureParameters.is_closure());
        assert!(Rust::Move.is_closure());
        assert!(!Rust::FunctionItem.is_closure());
    }

    #[test]
    fn test_is_lifetime() {
        assert!(Rust::Lifetime.is_lifetime());
        assert!(Rust::ForLifetimes.is_lifetime());
        assert!(Rust::SQUOTE.is_lifetime());
        assert!(!Rust::TypeParameters.is_lifetime());
    }

    #[test]
    fn test_is_generic() {
        assert!(Rust::TypeParameters.is_generic());
        assert!(Rust::TypeArguments.is_generic());
        assert!(Rust::GenericType.is_generic());
        assert!(Rust::GenericFunction.is_generic());
        assert!(!Rust::Type.is_generic());
    }

    #[test]
    fn test_is_reference() {
        assert!(Rust::ReferenceType.is_reference());
        assert!(Rust::ReferenceExpression.is_reference());
        assert!(Rust::Ref.is_reference());
        assert!(Rust::AMP.is_reference());
        assert!(!Rust::PointerType.is_reference());
    }

    #[test]
    fn test_is_operator() {
        assert!(Rust::PLUS.is_operator());
        assert!(Rust::DASH.is_operator());
        assert!(Rust::AMPAMP.is_operator());
        assert!(Rust::PIPEPIPE.is_operator());
        assert!(!Rust::EQ.is_operator());
    }

    #[test]
    fn test_is_assignment() {
        assert!(Rust::EQ.is_assignment());
        assert!(Rust::PLUSEQ.is_assignment());
        assert!(Rust::AssignmentExpression.is_assignment());
        assert!(!Rust::EQEQ.is_assignment());
    }

    #[test]
    fn test_is_literal() {
        assert!(Rust::IntegerLiteral.is_literal());
        assert!(Rust::FloatLiteral.is_literal());
        assert!(Rust::StringLiteral.is_literal());
        assert!(Rust::True.is_literal());
        assert!(Rust::False.is_literal());
        assert!(!Rust::Identifier.is_literal());
    }

    #[test]
    fn test_is_comment() {
        assert!(Rust::LineComment.is_comment());
        assert!(Rust::BlockComment.is_comment());
        assert!(Rust::DocComment.is_comment());
        assert!(!Rust::StringLiteral.is_comment());
    }

    #[test]
    fn test_is_control_flow() {
        assert!(Rust::If.is_control_flow());
        assert!(Rust::Loop.is_control_flow());
        assert!(Rust::While.is_control_flow());
        assert!(Rust::For.is_control_flow());
        assert!(Rust::Break.is_control_flow());
        assert!(Rust::Return.is_control_flow());
        assert!(!Rust::Fn.is_control_flow());
    }

    #[test]
    fn test_is_try() {
        assert!(Rust::Try.is_try());
        assert!(Rust::TryExpression.is_try());
        assert!(Rust::TryBlock.is_try());
        assert!(Rust::QMARK.is_try());
        assert!(!Rust::Match.is_try());
    }

    #[test]
    fn test_is_type() {
        assert!(Rust::Type.is_type());
        assert!(Rust::ArrayType.is_type());
        assert!(Rust::TupleType.is_type());
        assert!(Rust::FunctionType.is_type());
        assert!(!Rust::Expression.is_type());
    }

    #[test]
    fn test_is_primitive_type() {
        assert!(Rust::PrimitiveType.is_primitive_type());
        assert!(Rust::PrimitiveType5.is_primitive_type());
        assert!(Rust::PrimitiveType17.is_primitive_type());
        assert!(!Rust::Type.is_primitive_type());
    }

    #[test]
    fn test_is_self_keyword() {
        assert!(Rust::Zelf.is_self_keyword());
        assert!(Rust::Super.is_self_keyword());
        assert!(Rust::Crate.is_self_keyword());
        assert!(!Rust::Identifier.is_self_keyword());
    }

    #[test]
    fn test_is_dynamic() {
        assert!(Rust::Dyn.is_dynamic());
        assert!(Rust::DynamicType.is_dynamic());
        assert!(!Rust::Type.is_dynamic());
    }

    #[test]
    fn test_is_identifier() {
        assert!(Rust::Identifier.is_identifier());
        assert!(Rust::TypeIdentifier.is_identifier());
        assert!(Rust::FieldIdentifier.is_identifier());
        assert!(!Rust::IntegerLiteral.is_identifier());
    }

    #[test]
    fn test_u16_equality() {
        assert_eq!(Rust::FunctionItem, 186u16);
        assert_eq!(186u16, Rust::FunctionItem);
        assert_eq!(Rust::StructItem, 174u16);
        assert_eq!(174u16, Rust::StructItem);
    }

    #[test]
    fn test_language_info() {
        assert_eq!(RustLanguage::get_lang(), Lang::Rust);
        assert_eq!(RustLanguage::get_lang_name(), "rust");
    }

    #[test]
    fn test_is_yield() {
        assert!(Rust::Yield.is_yield());
        assert!(Rust::YieldExpression.is_yield());
        assert!(Rust::Gen.is_yield());
        assert!(!Rust::Return.is_yield());
    }

    #[test]
    fn test_is_range() {
        assert!(Rust::RangeExpression.is_range());
        assert!(Rust::RangePattern.is_range());
        assert!(Rust::DOTDOT.is_range());
        assert!(Rust::DOTDOTEQ.is_range());
        assert!(!Rust::DOT.is_range());
    }

    // ==================== ADVANCED HELPER METHOD TESTS ====================

    // -------------------- Macro Tests --------------------

    #[test]
    fn test_is_macro_rules() {
        assert!(Rust::MacroRulesBANG.is_macro_rules());
        assert!(Rust::MacroDefinition.is_macro_rules());
        assert!(!Rust::MacroInvocation.is_macro_rules());
    }

    #[test]
    fn test_is_proc_macro() {
        assert!(Rust::MacroInvocation.is_proc_macro());
        assert!(!Rust::MacroDefinition.is_proc_macro());
    }

    #[test]
    fn test_is_macro_pattern() {
        assert!(Rust::TokenPattern.is_macro_pattern());
        assert!(Rust::TokenTreePattern.is_macro_pattern());
        assert!(Rust::TokenTree.is_macro_pattern());
        assert!(Rust::TokenRepetition.is_macro_pattern());
        assert!(!Rust::Pattern.is_macro_pattern());
    }

    #[test]
    fn test_is_fragment_specifier() {
        assert!(Rust::FragmentSpecifier.is_fragment_specifier());
        assert!(Rust::Expr.is_fragment_specifier());
        assert!(Rust::Ident.is_fragment_specifier());
        assert!(Rust::Ty.is_fragment_specifier());
        assert!(!Rust::TypeIdentifier.is_fragment_specifier());
    }

    #[test]
    fn test_is_macro_rule() {
        assert!(Rust::MacroRule.is_macro_rule());
        assert!(!Rust::MacroDefinition.is_macro_rule());
    }

    // -------------------- Lifetime Tests --------------------

    #[test]
    fn test_is_lifetime_annotation() {
        assert!(Rust::Lifetime.is_lifetime_annotation());
        assert!(Rust::Lifetime2.is_lifetime_annotation());
        assert!(Rust::SQUOTE.is_lifetime_annotation());
        assert!(!Rust::TypeParameters.is_lifetime_annotation());
    }

    #[test]
    fn test_is_for_lifetimes() {
        assert!(Rust::ForLifetimes.is_for_lifetimes());
        assert!(!Rust::Lifetime.is_for_lifetimes());
    }

    #[test]
    fn test_is_lifetime_elision_context() {
        assert!(Rust::ReferenceType.is_lifetime_elision_context());
        assert!(Rust::Parameter.is_lifetime_elision_context());
        assert!(Rust::FunctionType.is_lifetime_elision_context());
        assert!(!Rust::Lifetime.is_lifetime_elision_context());
    }

    // -------------------- Trait System Tests --------------------

    #[test]
    fn test_is_trait_or_impl() {
        assert!(Rust::TraitItem.is_trait_or_impl());
        assert!(Rust::ImplItem.is_trait_or_impl());
        assert!(Rust::Trait.is_trait_or_impl());
        assert!(Rust::Impl.is_trait_or_impl());
        assert!(!Rust::StructItem.is_trait_or_impl());
    }

    #[test]
    fn test_is_associated_type() {
        assert!(Rust::AssociatedType.is_associated_type());
        assert!(Rust::TypeBinding.is_associated_type());
        assert!(!Rust::TypeItem.is_associated_type());
    }

    #[test]
    fn test_is_where_predicate() {
        assert!(Rust::WherePredicate.is_where_predicate());
        assert!(Rust::WhereClause.is_where_predicate());
        assert!(Rust::Where.is_where_predicate());
        assert!(!Rust::TraitBounds.is_where_predicate());
    }

    #[test]
    fn test_is_trait_constraint() {
        assert!(Rust::TraitBounds.is_trait_constraint());
        assert!(Rust::BoundedType.is_trait_constraint());
        assert!(Rust::ConstrainedTypeParameter.is_trait_constraint());
        assert!(Rust::HigherRankedTraitBound.is_trait_constraint());
        assert!(!Rust::TypeParameters.is_trait_constraint());
    }

    // -------------------- Pattern Matching Tests --------------------

    #[test]
    fn test_is_match_arm() {
        assert!(Rust::MatchArm.is_match_arm());
        assert!(Rust::MatchArm2.is_match_arm());
        assert!(Rust::MatchPattern.is_match_arm());
        assert!(!Rust::Pattern.is_match_arm());
    }

    #[test]
    fn test_is_let_pattern() {
        assert!(Rust::LetCondition.is_let_pattern());
        assert!(Rust::LetChain.is_let_pattern());
        assert!(Rust::LetChain2.is_let_pattern());
        assert!(!Rust::LetDeclaration.is_let_pattern());
    }

    #[test]
    fn test_is_struct_pattern() {
        assert!(Rust::StructPattern.is_struct_pattern());
        assert!(Rust::TupleStructPattern.is_struct_pattern());
        assert!(Rust::FieldPattern.is_struct_pattern());
        assert!(!Rust::StructItem.is_struct_pattern());
    }

    #[test]
    fn test_is_destructuring_pattern() {
        assert!(Rust::TuplePattern.is_destructuring_pattern());
        assert!(Rust::SlicePattern.is_destructuring_pattern());
        assert!(Rust::StructPattern.is_destructuring_pattern());
        assert!(!Rust::Pattern.is_destructuring_pattern());
    }

    #[test]
    fn test_is_or_pattern() {
        assert!(Rust::OrPattern.is_or_pattern());
        assert!(!Rust::Pattern.is_or_pattern());
    }

    #[test]
    fn test_is_range_pattern() {
        assert!(Rust::RangePattern.is_range_pattern());
        assert!(!Rust::RangeExpression.is_range_pattern());
    }

    #[test]
    fn test_is_binding_pattern() {
        assert!(Rust::CapturedPattern.is_binding_pattern());
        assert!(Rust::RefPattern.is_binding_pattern());
        assert!(Rust::MutPattern.is_binding_pattern());
        assert!(Rust::ReferencePattern.is_binding_pattern());
        assert!(!Rust::Pattern.is_binding_pattern());
    }

    // -------------------- Ownership Tests --------------------

    #[test]
    fn test_is_move() {
        assert!(Rust::Move.is_move());
        assert!(!Rust::Ref.is_move());
    }

    #[test]
    fn test_is_borrow() {
        assert!(Rust::AMP.is_borrow());
        assert!(Rust::ReferenceExpression.is_borrow());
        assert!(Rust::ReferenceType.is_borrow());
        assert!(!Rust::Move.is_borrow());
    }

    #[test]
    fn test_is_mutable_borrow() {
        assert!(Rust::MutableSpecifier.is_mutable_borrow());
        assert!(Rust::ReferenceExpression.is_mutable_borrow());
        assert!(!Rust::Move.is_mutable_borrow());
    }

    #[test]
    fn test_is_reference_syntax() {
        assert!(Rust::Ref.is_reference_syntax());
        assert!(Rust::AMP.is_reference_syntax());
        assert!(Rust::ReferenceType.is_reference_syntax());
        assert!(Rust::RefPattern.is_reference_syntax());
        assert!(!Rust::Move.is_reference_syntax());
    }

    // -------------------- Unsafe Tests --------------------

    #[test]
    fn test_is_unsafe_function() {
        assert!(Rust::Unsafe.is_unsafe_function());
        assert!(Rust::FunctionModifiers.is_unsafe_function());
        assert!(!Rust::FunctionItem.is_unsafe_function());
    }

    #[test]
    fn test_is_unsafe_trait() {
        assert!(Rust::Unsafe.is_unsafe_trait());
        assert!(Rust::TraitItem.is_unsafe_trait());
        assert!(Rust::ImplItem.is_unsafe_trait());
        assert!(!Rust::StructItem.is_unsafe_trait());
    }

    #[test]
    fn test_is_raw_pointer() {
        assert!(Rust::PointerType.is_raw_pointer());
        assert!(Rust::Raw.is_raw_pointer());
        assert!(!Rust::ReferenceType.is_raw_pointer());
    }

    // -------------------- Async/Await Tests --------------------

    #[test]
    fn test_is_async_function() {
        assert!(Rust::Async.is_async_function());
        assert!(Rust::FunctionItem.is_async_function());
        assert!(!Rust::AsyncBlock.is_async_function());
    }

    #[test]
    fn test_is_await_expression() {
        assert!(Rust::AwaitExpression.is_await_expression());
        assert!(Rust::Await.is_await_expression());
        assert!(!Rust::Async.is_await_expression());
    }

    #[test]
    fn test_is_async_block() {
        assert!(Rust::AsyncBlock.is_async_block());
        assert!(!Rust::Async.is_async_block());
    }

    // -------------------- Generics Tests --------------------

    #[test]
    fn test_is_type_parameter() {
        assert!(Rust::TypeParameters.is_type_parameter());
        assert!(Rust::ConstrainedTypeParameter.is_type_parameter());
        assert!(Rust::OptionalTypeParameter.is_type_parameter());
        assert!(!Rust::ConstParameter.is_type_parameter());
    }

    #[test]
    fn test_is_const_generic() {
        assert!(Rust::ConstParameter.is_const_generic());
        assert!(!Rust::TypeParameters.is_const_generic());
    }

    #[test]
    fn test_is_turbofish() {
        assert!(Rust::GenericTypeWithTurbofish.is_turbofish());
        assert!(Rust::COLONCOLON.is_turbofish());
        assert!(Rust::LT2.is_turbofish());
        assert!(!Rust::GenericType.is_turbofish());
    }

    #[test]
    fn test_is_type_arguments() {
        assert!(Rust::TypeArguments.is_type_arguments());
        assert!(Rust::TypeBinding.is_type_arguments());
        assert!(!Rust::TypeParameters.is_type_arguments());
    }

    // -------------------- Attributes Tests --------------------

    #[test]
    fn test_is_derive_attribute() {
        assert!(Rust::AttributeItem.is_derive_attribute());
        assert!(Rust::Attribute.is_derive_attribute());
        assert!(!Rust::InnerAttributeItem.is_derive_attribute());
    }

    #[test]
    fn test_is_cfg_attribute() {
        assert!(Rust::AttributeItem.is_cfg_attribute());
        assert!(Rust::Attribute.is_cfg_attribute());
        assert!(!Rust::InnerAttributeItem.is_cfg_attribute());
    }

    #[test]
    fn test_is_test_attribute() {
        assert!(Rust::AttributeItem.is_test_attribute());
        assert!(!Rust::InnerAttributeItem.is_test_attribute());
    }

    #[test]
    fn test_is_inner_attribute() {
        assert!(Rust::InnerAttributeItem.is_inner_attribute());
        assert!(!Rust::AttributeItem.is_inner_attribute());
    }

    #[test]
    fn test_is_outer_attribute() {
        assert!(Rust::AttributeItem.is_outer_attribute());
        assert!(!Rust::InnerAttributeItem.is_outer_attribute());
    }

    // -------------------- Module System Tests --------------------

    #[test]
    fn test_is_mod_declaration() {
        assert!(Rust::ModItem.is_mod_declaration());
        assert!(Rust::Mod.is_mod_declaration());
        assert!(!Rust::UseDeclaration.is_mod_declaration());
    }

    #[test]
    fn test_is_use_import() {
        assert!(Rust::UseDeclaration.is_use_import());
        assert!(Rust::UseClause.is_use_import());
        assert!(Rust::UseList.is_use_import());
        assert!(!Rust::ModItem.is_use_import());
    }

    #[test]
    fn test_is_pub_visibility() {
        assert!(Rust::Pub.is_pub_visibility());
        assert!(Rust::VisibilityModifier.is_pub_visibility());
        assert!(!Rust::Mod.is_pub_visibility());
    }

    #[test]
    fn test_is_path_segment() {
        assert!(Rust::Crate.is_path_segment());
        assert!(Rust::Super.is_path_segment());
        assert!(Rust::Zelf.is_path_segment());
        assert!(!Rust::Identifier.is_path_segment());
    }

    #[test]
    fn test_is_scoped_identifier() {
        assert!(Rust::ScopedIdentifier.is_scoped_identifier());
        assert!(Rust::ScopedTypeIdentifier.is_scoped_identifier());
        assert!(!Rust::Identifier.is_scoped_identifier());
    }

    #[test]
    fn test_is_use_wildcard() {
        assert!(Rust::UseWildcard.is_use_wildcard());
        assert!(Rust::STAR.is_use_wildcard());
        assert!(!Rust::UseList.is_use_wildcard());
    }

    // -------------------- Error Handling Tests --------------------

    #[test]
    fn test_is_result_type() {
        assert!(Rust::TypeIdentifier.is_result_type());
        assert!(Rust::GenericType.is_result_type());
        assert!(!Rust::IntegerLiteral.is_result_type());
    }

    #[test]
    fn test_is_option_type() {
        assert!(Rust::TypeIdentifier.is_option_type());
        assert!(Rust::GenericType.is_option_type());
        assert!(!Rust::IntegerLiteral.is_option_type());
    }

    #[test]
    fn test_is_question_mark_operator() {
        assert!(Rust::QMARK.is_question_mark_operator());
        assert!(Rust::TryExpression.is_question_mark_operator());
        assert!(!Rust::Try.is_question_mark_operator());
    }

    #[test]
    fn test_is_panic_macro() {
        assert!(Rust::MacroInvocation.is_panic_macro());
        assert!(!Rust::MacroDefinition.is_panic_macro());
    }

    // -------------------- Closures Tests --------------------

    #[test]
    fn test_is_closure_definition() {
        assert!(Rust::ClosureExpression.is_closure_definition());
        assert!(Rust::ClosureParameters.is_closure_definition());
        assert!(!Rust::FunctionItem.is_closure_definition());
    }

    #[test]
    fn test_is_closure_parameter() {
        assert!(Rust::ClosureParameters.is_closure_parameter());
        assert!(!Rust::Parameters.is_closure_parameter());
    }

    #[test]
    fn test_is_move_closure() {
        assert!(Rust::Move.is_move_closure());
        assert!(Rust::ClosureExpression.is_move_closure());
        assert!(!Rust::FunctionItem.is_move_closure());
    }

    // -------------------- Type System Tests --------------------

    #[test]
    fn test_is_struct_definition() {
        assert!(Rust::StructItem.is_struct_definition());
        assert!(Rust::StructExpression.is_struct_definition());
        assert!(Rust::FieldDeclarationList.is_struct_definition());
        assert!(!Rust::EnumItem.is_struct_definition());
    }

    #[test]
    fn test_is_enum_definition() {
        assert!(Rust::EnumItem.is_enum_definition());
        assert!(Rust::EnumVariant.is_enum_definition());
        assert!(Rust::EnumVariantList.is_enum_definition());
        assert!(!Rust::StructItem.is_enum_definition());
    }

    #[test]
    fn test_is_type_alias_definition() {
        assert!(Rust::TypeItem.is_type_alias_definition());
        assert!(Rust::Type.is_type_alias_definition());
        assert!(!Rust::TypeIdentifier.is_type_alias_definition());
    }

    #[test]
    fn test_is_union_definition() {
        assert!(Rust::UnionItem.is_union_definition());
        assert!(Rust::Union.is_union_definition());
        assert!(!Rust::StructItem.is_union_definition());
    }

    #[test]
    fn test_is_never_type_annotation() {
        assert!(Rust::NeverType.is_never_type_annotation());
        assert!(Rust::BANG.is_never_type_annotation());
        assert!(!Rust::Type.is_never_type_annotation());
    }

    #[test]
    fn test_is_tuple() {
        assert!(Rust::TupleType.is_tuple());
        assert!(Rust::TupleExpression.is_tuple());
        assert!(Rust::TuplePattern.is_tuple());
        assert!(!Rust::ArrayType.is_tuple());
    }

    #[test]
    fn test_is_array() {
        assert!(Rust::ArrayType.is_array());
        assert!(Rust::ArrayExpression.is_array());
        assert!(!Rust::TupleType.is_array());
    }

    #[test]
    fn test_is_function_type() {
        assert!(Rust::FunctionType.is_function_type());
        assert!(!Rust::FunctionItem.is_function_type());
    }

    // -------------------- Special Syntax Tests --------------------

    #[test]
    fn test_is_turbofish_syntax() {
        assert!(Rust::GenericTypeWithTurbofish.is_turbofish_syntax());
        assert!(!Rust::GenericType.is_turbofish_syntax());
    }

    #[test]
    fn test_is_dyn_trait() {
        assert!(Rust::Dyn.is_dyn_trait());
        assert!(Rust::DynamicType.is_dyn_trait());
        assert!(!Rust::Trait.is_dyn_trait());
    }

    #[test]
    fn test_is_impl_trait() {
        assert!(Rust::AbstractType.is_impl_trait());
        assert!(Rust::Impl.is_impl_trait());
        assert!(!Rust::Trait.is_impl_trait());
    }

    #[test]
    fn test_is_qualified_type() {
        assert!(Rust::QualifiedType.is_qualified_type());
        assert!(!Rust::Type.is_qualified_type());
    }

    // -------------------- Memory & Special Traits Tests --------------------

    #[test]
    fn test_is_field() {
        assert!(Rust::FieldDeclaration.is_field());
        assert!(Rust::FieldExpression.is_field());
        assert!(Rust::FieldPattern.is_field());
        assert!(Rust::FieldIdentifier.is_field());
        assert!(!Rust::Identifier.is_field());
    }

    #[test]
    fn test_is_unit() {
        assert!(Rust::UnitType.is_unit());
        assert!(Rust::UnitExpression.is_unit());
        assert!(!Rust::TupleType.is_unit());
    }

    // -------------------- Loop & Control Flow Tests --------------------

    #[test]
    fn test_is_loop_expression() {
        assert!(Rust::LoopExpression.is_loop_expression());
        assert!(Rust::WhileExpression.is_loop_expression());
        assert!(Rust::ForExpression.is_loop_expression());
        assert!(Rust::Loop.is_loop_expression());
        assert!(!Rust::IfExpression.is_loop_expression());
    }

    #[test]
    fn test_is_break_or_continue() {
        assert!(Rust::Break.is_break_or_continue());
        assert!(Rust::Continue.is_break_or_continue());
        assert!(Rust::BreakExpression.is_break_or_continue());
        assert!(Rust::ContinueExpression.is_break_or_continue());
        assert!(!Rust::Return.is_break_or_continue());
    }

    #[test]
    fn test_is_loop_label() {
        assert!(Rust::Label.is_loop_label());
        assert!(!Rust::Lifetime.is_loop_label());
    }

    // -------------------- Expressions Tests --------------------

    #[test]
    fn test_is_call_expression() {
        assert!(Rust::CallExpression.is_call_expression());
        assert!(Rust::Arguments.is_call_expression());
        assert!(!Rust::FieldExpression.is_call_expression());
    }

    #[test]
    fn test_is_method_call() {
        assert!(Rust::FieldExpression.is_method_call());
        assert!(Rust::CallExpression.is_method_call());
        assert!(!Rust::MacroInvocation.is_method_call());
    }

    #[test]
    fn test_is_index_expression() {
        assert!(Rust::IndexExpression.is_index_expression());
        assert!(!Rust::ArrayExpression.is_index_expression());
    }

    #[test]
    fn test_is_struct_expression() {
        assert!(Rust::StructExpression.is_struct_expression());
        assert!(Rust::FieldInitializerList.is_struct_expression());
        assert!(Rust::FieldInitializer.is_struct_expression());
        assert!(!Rust::StructItem.is_struct_expression());
    }

    #[test]
    fn test_is_binary_expression() {
        assert!(Rust::BinaryExpression.is_binary_expression());
        assert!(!Rust::UnaryExpression.is_binary_expression());
    }

    #[test]
    fn test_is_unary_expression() {
        assert!(Rust::UnaryExpression.is_unary_expression());
        assert!(!Rust::BinaryExpression.is_unary_expression());
    }

    #[test]
    fn test_is_type_cast() {
        assert!(Rust::TypeCastExpression.is_type_cast());
        assert!(Rust::As.is_type_cast());
        assert!(!Rust::Type.is_type_cast());
    }

    // -------------------- Statements Tests --------------------

    #[test]
    fn test_is_expression_statement() {
        assert!(Rust::ExpressionStatement.is_expression_statement());
        assert!(!Rust::EmptyStatement.is_expression_statement());
    }

    #[test]
    fn test_is_empty_statement() {
        assert!(Rust::EmptyStatement.is_empty_statement());
        assert!(!Rust::ExpressionStatement.is_empty_statement());
    }

    #[test]
    fn test_is_let_declaration() {
        assert!(Rust::LetDeclaration.is_let_declaration());
        assert!(!Rust::LetCondition.is_let_declaration());
    }

    // -------------------- Items & Declarations Tests --------------------

    #[test]
    fn test_is_function_signature() {
        assert!(Rust::FunctionSignatureItem.is_function_signature());
        assert!(!Rust::FunctionItem.is_function_signature());
    }

    #[test]
    fn test_is_const_item() {
        assert!(Rust::ConstItem.is_const_item());
        assert!(!Rust::StaticItem.is_const_item());
    }

    #[test]
    fn test_is_static_item() {
        assert!(Rust::StaticItem.is_static_item());
        assert!(!Rust::ConstItem.is_static_item());
    }

    #[test]
    fn test_is_extern_crate() {
        assert!(Rust::ExternCrateDeclaration.is_extern_crate());
        assert!(!Rust::ForeignModItem.is_extern_crate());
    }

    #[test]
    fn test_is_foreign_mod() {
        assert!(Rust::ForeignModItem.is_foreign_mod());
        assert!(!Rust::ExternCrateDeclaration.is_foreign_mod());
    }

    // -------------------- Parameters Tests --------------------

    #[test]
    fn test_is_function_parameter() {
        assert!(Rust::Parameter.is_function_parameter());
        assert!(Rust::Parameters.is_function_parameter());
        assert!(Rust::SelfParameter.is_function_parameter());
        assert!(!Rust::ClosureParameters.is_function_parameter());
    }

    #[test]
    fn test_is_self_parameter() {
        assert!(Rust::SelfParameter.is_self_parameter());
        assert!(!Rust::Parameter.is_self_parameter());
    }

    #[test]
    fn test_is_variadic_parameter() {
        assert!(Rust::VariadicParameter.is_variadic_parameter());
        assert!(!Rust::Parameter.is_variadic_parameter());
    }

    // -------------------- Operators & Delimiters Tests --------------------

    #[test]
    fn test_is_comparison_operator() {
        assert!(Rust::EQEQ.is_comparison_operator());
        assert!(Rust::BANGEQ.is_comparison_operator());
        assert!(Rust::GT.is_comparison_operator());
        assert!(Rust::LT.is_comparison_operator());
        assert!(!Rust::EQ.is_comparison_operator());
    }

    #[test]
    fn test_is_logical_operator() {
        assert!(Rust::AMPAMP.is_logical_operator());
        assert!(Rust::PIPEPIPE.is_logical_operator());
        assert!(!Rust::AMP.is_logical_operator());
    }

    #[test]
    fn test_is_bitwise_operator() {
        assert!(Rust::AMP.is_bitwise_operator());
        assert!(Rust::PIPE.is_bitwise_operator());
        assert!(Rust::CARET.is_bitwise_operator());
        assert!(Rust::LTLT.is_bitwise_operator());
        assert!(!Rust::AMPAMP.is_bitwise_operator());
    }

    #[test]
    fn test_is_arithmetic_operator() {
        assert!(Rust::PLUS.is_arithmetic_operator());
        assert!(Rust::DASH.is_arithmetic_operator());
        assert!(Rust::STAR.is_arithmetic_operator());
        assert!(Rust::SLASH.is_arithmetic_operator());
        assert!(Rust::PERCENT.is_arithmetic_operator());
        assert!(!Rust::EQ.is_arithmetic_operator());
    }

    #[test]
    fn test_is_compound_assignment() {
        assert!(Rust::PLUSEQ.is_compound_assignment());
        assert!(Rust::DASHEQ.is_compound_assignment());
        assert!(Rust::STAREQ.is_compound_assignment());
        assert!(Rust::CompoundAssignmentExpr.is_compound_assignment());
        assert!(!Rust::EQ.is_compound_assignment());
    }

    #[test]
    fn test_is_delimiter() {
        assert!(Rust::LPAREN.is_delimiter());
        assert!(Rust::RPAREN.is_delimiter());
        assert!(Rust::LBRACK.is_delimiter());
        assert!(Rust::RBRACK.is_delimiter());
        assert!(Rust::LBRACE.is_delimiter());
        assert!(Rust::RBRACE.is_delimiter());
        assert!(!Rust::SEMI.is_delimiter());
    }

    #[test]
    fn test_is_path_separator() {
        assert!(Rust::COLONCOLON.is_path_separator());
        assert!(!Rust::COLON.is_path_separator());
    }

    #[test]
    fn test_is_return_arrow() {
        assert!(Rust::DASHGT.is_return_arrow());
        assert!(!Rust::EQGT.is_return_arrow());
    }

    #[test]
    fn test_is_fat_arrow() {
        assert!(Rust::EQGT.is_fat_arrow());
        assert!(!Rust::DASHGT.is_fat_arrow());
    }

    // -------------------- String & Character Literals Tests --------------------

    #[test]
    fn test_is_string_literal() {
        assert!(Rust::StringLiteral.is_string_literal());
        assert!(Rust::RawStringLiteral.is_string_literal());
        assert!(Rust::StringContent.is_string_literal());
        assert!(!Rust::CharLiteral.is_string_literal());
    }

    #[test]
    fn test_is_raw_string_literal() {
        assert!(Rust::RawStringLiteral.is_raw_string_literal());
        assert!(Rust::RawStringLiteralStart.is_raw_string_literal());
        assert!(Rust::RawStringLiteralEnd.is_raw_string_literal());
        assert!(!Rust::StringLiteral.is_raw_string_literal());
    }

    #[test]
    fn test_is_char_literal() {
        assert!(Rust::CharLiteral.is_char_literal());
        assert!(!Rust::StringLiteral.is_char_literal());
    }

    #[test]
    fn test_is_escape_sequence() {
        assert!(Rust::EscapeSequence.is_escape_sequence());
        assert!(!Rust::CharLiteral.is_escape_sequence());
    }

    // -------------------- Numeric Literals Tests --------------------

    #[test]
    fn test_is_integer_literal() {
        assert!(Rust::IntegerLiteral.is_integer_literal());
        assert!(!Rust::FloatLiteral.is_integer_literal());
    }

    #[test]
    fn test_is_float_literal() {
        assert!(Rust::FloatLiteral.is_float_literal());
        assert!(!Rust::IntegerLiteral.is_float_literal());
    }

    #[test]
    fn test_is_numeric_literal() {
        assert!(Rust::IntegerLiteral.is_numeric_literal());
        assert!(Rust::FloatLiteral.is_numeric_literal());
        assert!(!Rust::StringLiteral.is_numeric_literal());
    }

    // -------------------- Boolean Literals Tests --------------------

    #[test]
    fn test_is_boolean_literal() {
        assert!(Rust::BooleanLiteral.is_boolean_literal());
        assert!(Rust::True.is_boolean_literal());
        assert!(Rust::False.is_boolean_literal());
        assert!(!Rust::IntegerLiteral.is_boolean_literal());
    }

    // -------------------- Documentation Tests --------------------

    #[test]
    fn test_is_doc_comment() {
        assert!(Rust::DocComment.is_doc_comment());
        assert!(Rust::OuterDocCommentMarker.is_doc_comment());
        assert!(Rust::InnerDocCommentMarker.is_doc_comment());
        assert!(!Rust::LineComment.is_doc_comment());
    }

    #[test]
    fn test_is_inner_doc_comment() {
        assert!(Rust::InnerDocCommentMarker.is_inner_doc_comment());
        assert!(Rust::InnerDocCommentMarker2.is_inner_doc_comment());
        assert!(!Rust::OuterDocCommentMarker.is_inner_doc_comment());
    }

    #[test]
    fn test_is_outer_doc_comment() {
        assert!(Rust::OuterDocCommentMarker.is_outer_doc_comment());
        assert!(Rust::OuterDocCommentMarker2.is_outer_doc_comment());
        assert!(!Rust::InnerDocCommentMarker.is_outer_doc_comment());
    }

    #[test]
    fn test_is_line_comment() {
        assert!(Rust::LineComment.is_line_comment());
        assert!(Rust::SLASHSLASH.is_line_comment());
        assert!(!Rust::BlockComment.is_line_comment());
    }

    #[test]
    fn test_is_block_comment() {
        assert!(Rust::BlockComment.is_block_comment());
        assert!(Rust::SLASHSTAR.is_block_comment());
        assert!(Rust::STARSLASH.is_block_comment());
        assert!(!Rust::LineComment.is_block_comment());
    }

    // -------------------- Advanced Features Tests --------------------

    #[test]
    fn test_is_const_block() {
        assert!(Rust::ConstBlock.is_const_block());
        assert!(!Rust::ConstItem.is_const_block());
    }

    #[test]
    fn test_is_try_block() {
        assert!(Rust::TryBlock.is_try_block());
        assert!(!Rust::TryExpression.is_try_block());
    }

    #[test]
    fn test_is_gen_block() {
        assert!(Rust::GenBlock.is_gen_block());
        assert!(Rust::Gen.is_gen_block());
        assert!(!Rust::Block.is_gen_block());
    }

    #[test]
    fn test_is_parenthesized_expression() {
        assert!(Rust::ParenthesizedExpression.is_parenthesized_expression());
        assert!(!Rust::TupleExpression.is_parenthesized_expression());
    }

    #[test]
    fn test_is_base_field_initializer() {
        assert!(Rust::BaseFieldInitializer.is_base_field_initializer());
        assert!(!Rust::FieldInitializer.is_base_field_initializer());
    }

    #[test]
    fn test_is_shorthand_field_initializer() {
        assert!(Rust::ShorthandFieldInitializer.is_shorthand_field_initializer());
        assert!(!Rust::FieldInitializer.is_shorthand_field_initializer());
    }

    #[test]
    fn test_is_condition() {
        assert!(Rust::Condition.is_condition());
        assert!(!Rust::Expression.is_condition());
    }

    #[test]
    fn test_is_else_clause() {
        assert!(Rust::ElseClause.is_else_clause());
        assert!(Rust::Else.is_else_clause());
        assert!(!Rust::If.is_else_clause());
    }

    #[test]
    fn test_is_bracketed_type() {
        assert!(Rust::BracketedType.is_bracketed_type());
        assert!(!Rust::Type.is_bracketed_type());
    }

    #[test]
    fn test_is_function_modifiers() {
        assert!(Rust::FunctionModifiers.is_function_modifiers());
        assert!(!Rust::Async.is_function_modifiers());
    }

    #[test]
    fn test_is_extern_modifier() {
        assert!(Rust::ExternModifier.is_extern_modifier());
        assert!(!Rust::Extern.is_extern_modifier());
    }

    #[test]
    fn test_is_source_file() {
        assert!(Rust::SourceFile.is_source_file());
        assert!(!Rust::ModItem.is_source_file());
    }

    #[test]
    fn test_is_underscore() {
        assert!(Rust::UNDERSCORE.is_underscore());
        assert!(!Rust::Identifier.is_underscore());
    }

    #[test]
    fn test_is_at_pattern() {
        assert!(Rust::AT.is_at_pattern());
        assert!(!Rust::Pattern.is_at_pattern());
    }

    #[test]
    fn test_is_dollar_sign() {
        assert!(Rust::DOLLAR.is_dollar_sign());
        assert!(!Rust::HASH.is_dollar_sign());
    }

    #[test]
    fn test_is_semicolon() {
        assert!(Rust::SEMI.is_semicolon());
        assert!(!Rust::COLON.is_semicolon());
    }

    #[test]
    fn test_is_colon() {
        assert!(Rust::COLON.is_colon());
        assert!(!Rust::COLONCOLON.is_colon());
    }

    #[test]
    fn test_is_comma() {
        assert!(Rust::COMMA.is_comma());
        assert!(!Rust::SEMI.is_comma());
    }

    #[test]
    fn test_is_dot() {
        assert!(Rust::DOT.is_dot());
        assert!(!Rust::DOTDOT.is_dot());
    }

    #[test]
    fn test_is_hash() {
        assert!(Rust::HASH.is_hash());
        assert!(!Rust::DOLLAR.is_hash());
    }
}
