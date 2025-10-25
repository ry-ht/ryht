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
}
