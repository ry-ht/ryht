//! C++ language parser implementation.
//!
//! This module provides comprehensive support for C++ code analysis including:
//! - Functions, classes, structs, templates, unions, enums
//! - Namespaces and scope resolution
//! - Modern C++ features (C++11/14/17/20/23)
//! - Preprocessor directives (#include, #define, #if, #ifdef, etc.)
//! - Template detection and analysis
//! - Static assertions
//! - Include guard patterns
//! - Using declarations vs typedefs
//! - C++ specific operators (::, ->, .*, ->*)
//! - Template operators (<>, <<, >>)
//! - User-defined operator overloads
//! - Raw string literals, character literals, wide/unicode strings
//! - String concatenation
//! - Virtual function analysis
//! - Class hierarchy metrics
//! - All code metrics (Cyclomatic Complexity, Cognitive Complexity, LOC, Halstead)

use num_derive::FromPrimitive;
use crate::lang::Lang;
use crate::traits::LanguageInfo;

/// C++ language token types.
///
/// This enum represents all essential node types in the C++ tree-sitter grammar.
/// Includes preprocessor directives, templates, modern C++ features, and all
/// operators and keywords necessary for comprehensive code analysis.
#[derive(Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum CppToken {
    End = 0,
    Identifier = 1,

    // ========================================
    // Preprocessor Directives
    // ========================================
    HASHinclude = 2,
    PreprocIncludeToken2 = 3,
    HASHdefine = 4,
    HASHif = 9,
    HASHendif = 11,
    HASHifdef = 12,
    HASHifndef = 13,
    HASHelse = 14,
    HASHelif = 15,
    HASHelifdef = 16,
    HASHelifndef = 17,
    PreprocArg = 18,
    PreprocDirective = 19,
    PreprocDefined = 21,

    // ========================================
    // Punctuation and Delimiters
    // ========================================
    LPAREN = 5,
    RPAREN = 8,
    LBRACE = 65,
    RBRACE = 66,
    LBRACK = 71,
    RBRACK = 73,
    SEMI = 42,
    COLON = 101,
    COLONCOLON = 49,        // Scope resolution operator ::
    COMMA = 7,
    DOT = 155,
    DOTDOTDOT = 6,          // Variadic templates/parameters ...
    DASHGT = 157,           // Arrow operator ->
    DASHGTSTAR = 211,       // Pointer-to-member arrow ->*
    DOTSTAR = 156,          // Pointer-to-member dot .*

    // ========================================
    // Operators - Arithmetic
    // ========================================
    PLUS = 25,
    DASH = 24,
    STAR = 26,
    SLASH = 27,
    PERCENT = 28,
    PLUSPLUS = 142,
    DASHDASH = 141,

    // ========================================
    // Operators - Logical
    // ========================================
    BANG = 22,
    AMPAMP = 30,
    PIPEPIPE = 29,
    TILDE = 23,

    // ========================================
    // Operators - Bitwise
    // ========================================
    AMP = 33,
    PIPE = 31,
    CARET = 32,
    LTLT = 40,              // Left shift or stream insertion
    GTGT = 41,              // Right shift or stream extraction

    // ========================================
    // Operators - Comparison
    // ========================================
    LT = 39,
    GT = 36,
    GT2 = 185,              // Template closing >
    LTEQ = 38,
    GTEQ = 37,
    EQEQ = 34,
    BANGEQ = 35,
    LTEQGT = 134,           // Three-way comparison (spaceship) <=>

    // ========================================
    // Operators - Assignment
    // ========================================
    EQ = 74,
    PLUSEQ = 122,
    DASHEQ = 123,
    STAREQ = 119,
    SLASHEQ = 120,
    PERCENTEQ = 121,
    LTLTEQ = 124,
    GTGTEQ = 125,
    AMPEQ = 126,
    CARETEQ = 127,
    PIPEEQ = 128,
    AndEq = 129,
    OrEq = 130,
    XorEq = 131,

    // ========================================
    // Operators - Conditional
    // ========================================
    QMARK = 118,

    // ========================================
    // Operators - Alternative Tokens
    // ========================================
    Not = 132,
    Compl = 133,
    Or = 135,
    And = 136,
    Bitor = 137,
    Xor = 138,
    Bitand = 139,
    NotEq = 140,

    // ========================================
    // Keywords - Type Declaration
    // ========================================
    Class = 98,
    Struct = 99,
    Union = 100,
    Enum = 97,
    Typedef = 44,
    Using = 197,
    Typename = 183,
    Namespace = 196,
    Template = 184,
    Friend = 190,

    // ========================================
    // Keywords - Access Specifiers
    // ========================================
    Public = 191,
    Private = 192,
    Protected = 193,

    // ========================================
    // Keywords - Type Qualifiers
    // ========================================
    Const = 82,
    Constexpr = 83,
    Constinit = 92,
    Consteval = 93,
    Volatile = 84,
    Volatile2 = 154,
    Static = 72,
    Extern = 46,
    Virtual = 45,
    Mutable = 91,
    Register = 75,
    Inline = 76,
    Inline2 = 77,
    Inline3 = 78,
    Forceinline = 79,
    ThreadLocal = 80,
    Thread = 81,

    // ========================================
    // Keywords - Type Modifiers
    // ========================================
    Signed = 67,
    Unsigned = 68,
    Long = 69,
    Short = 70,
    Restrict = 85,
    Restrict2 = 86,
    Atomic = 87,

    // ========================================
    // Keywords - Control Flow
    // ========================================
    If = 102,
    Else = 103,
    Switch = 104,
    Case = 105,
    Default = 106,
    While = 107,
    Do = 108,
    For = 109,
    Return = 110,
    Break = 111,
    Continue = 112,
    Goto = 113,

    // ========================================
    // Keywords - Exception Handling
    // ========================================
    Try = 114,              // SEH __try
    Try2 = 187,             // C++ try
    Catch = 202,
    Throw = 195,
    Except = 115,           // SEH __except
    Finally = 116,          // SEH __finally
    Leave = 117,            // SEH __leave
    Noexcept2 = 194,

    // ========================================
    // Keywords - Memory Management
    // ========================================
    New = 209,
    Delete = 188,

    // ========================================
    // Keywords - Modern C++
    // ========================================
    Auto = 178,
    Decltype3 = 179,
    Final = 180,
    Override = 181,
    Explicit = 182,
    Operator = 186,
    StaticAssert = 198,
    Concept = 199,
    Requires = 210,

    // ========================================
    // Keywords - Coroutines (C++20)
    // ========================================
    CoReturn = 200,
    CoYield = 201,
    CoAwait = 208,

    // ========================================
    // Keywords - Alignment
    // ========================================
    Alignas = 94,
    Alignas2 = 95,
    Alignof = 144,
    Alignof2 = 145,
    Alignof3 = 146,
    Alignof4 = 147,
    Alignof5 = 148,

    // ========================================
    // Keywords - Other
    // ========================================
    Sizeof = 143,
    Offsetof = 149,
    Generic = 150,
    Asm = 151,
    Asm2 = 152,
    Asm3 = 153,
    Extension = 43,
    Noreturn = 88,
    Noreturn2 = 89,
    Nonnull = 90,

    // ========================================
    // Attributes and Specifiers
    // ========================================
    Attribute2 = 47,
    Attribute3 = 48,
    LBRACKLBRACK = 50,      // [[ for attributes
    RBRACKRBRACK = 51,      // ]] for attributes
    Declspec = 52,
    Based = 53,

    // ========================================
    // Calling Conventions (MS-specific)
    // ========================================
    Cdecl = 54,
    Clrcall = 55,
    Stdcall = 56,
    Fastcall = 57,
    Thiscall = 58,
    Vectorcall = 59,

    // ========================================
    // MS-specific Modifiers
    // ========================================
    MsRestrictModifier = 60,
    MsUnsignedPtrModifier = 61,
    MsSignedPtrModifier = 62,
    Unaligned = 63,
    Unaligned2 = 64,

    // ========================================
    // Primitive Types
    // ========================================
    PrimitiveType = 96,

    // ========================================
    // Literals
    // ========================================
    NumberLiteral = 158,
    True = 173,
    False = 174,
    NULL = 175,
    Nullptr = 176,
    This = 215,

    // ========================================
    // String Literals
    // ========================================
    // Character literals
    SQUOTE = 163,           // '
    LSQUOTE = 159,          // L'
    USQUOTE = 160,          // u'
    USQUOTE2 = 161,         // U'
    U8SQUOTE = 162,         // u8'
    Character = 164,

    // String literals
    DQUOTE = 169,           // "
    LDQUOTE = 165,          // L"
    UDQUOTE = 166,          // u"
    UDQUOTE2 = 167,         // U"
    U8DQUOTE = 168,         // u8"
    StringContent = 170,
    EscapeSequence = 171,

    // Raw string literals
    RDQUOTE = 203,          // R"
    LRDQUOTE = 204,         // LR"
    URDQUOTE = 205,         // uR"
    URDQUOTE2 = 206,        // UR"
    U8RDQUOTE = 207,        // u8R"
    RawStringDelimiter = 306,
    RawStringContent = 307,

    // Special string tokens
    SystemLibString = 172,  // <header.h> in #include
    DQUOTEDQUOTE = 214,     // User-defined literal ""
    LiteralSuffix = 216,

    // ========================================
    // Comments
    // ========================================
    Comment = 177,

    // ========================================
    // Preprocessor AST Nodes
    // ========================================
    PreprocInclude = 311,
    PreprocDef = 312,
    PreprocFunctionDef = 313,
    PreprocParams = 314,
    PreprocCall = 315,
    PreprocIf = 316,
    PreprocIfdef = 317,
    PreprocElse = 318,
    PreprocElif = 319,
    PreprocElifdef = 320,
    PreprocExpression = 336,

    // ========================================
    // AST Nodes - Top Level
    // ========================================
    TranslationUnit = 308,
    TopLevelItem = 309,
    BlockItem = 310,

    // ========================================
    // AST Nodes - Declarations
    // ========================================
    Declaration = 344,
    FunctionDefinition = 343,
    TypeDefinition = 345,
    EmptyDeclaration = 462,
    LinkageSpecification = 350,
    AttributeDeclaration = 353,
    FriendDeclaration = 499,
    UsingDeclaration = 523,
    AliasDeclaration = 524,
    StaticAssertDeclaration = 525,

    // ========================================
    // AST Nodes - Class/Struct/Union
    // ========================================
    ClassDeclaration = 466,
    ClassSpecifier = 468,
    StructSpecifier = 392,
    UnionSpecifier = 393,
    EnumSpecifier = 390,
    ClassName = 469,
    BaseClassClause = 472,
    FieldDeclarationList = 394,
    FieldDeclaration = 396,
    AccessSpecifier = 500,
    VirtualSpecifier = 470,

    // ========================================
    // AST Nodes - Templates
    // ========================================
    TemplateDeclaration = 475,
    TemplateInstantiation = 476,
    TemplateParameterList = 477,
    TemplateArgumentList = 518,
    TemplateType = 515,
    TemplateMethod = 516,
    TemplateFunction = 517,
    TypeParameterDeclaration = 478,
    VariadicTypeParameterDeclaration = 479,
    OptionalTypeParameterDeclaration = 480,
    TemplateTemplateParameterDeclaration = 481,

    // ========================================
    // AST Nodes - Namespaces
    // ========================================
    NamespaceDefinition = 519,
    NamespaceAliasDefinition = 520,
    NamespaceSpecifier = 521,
    NestedNamespaceSpecifier = 522,

    // ========================================
    // AST Nodes - Types
    // ========================================
    TypeSpecifier = 388,
    SizedTypeSpecifier = 389,
    TypeDescriptor = 433,
    TypeQualifier = 386,
    AlignasQualifier = 387,
    PlaceholderTypeSpecifier = 463,
    Decltype = 464,
    Decltype2 = 465,
    DependentType = 474,
    EnumBaseClause = 473,

    // ========================================
    // AST Nodes - Declarators
    // ========================================
    Declarator = 360,
    FieldDeclarator = 361,
    TypeDeclarator = 362,
    AbstractDeclarator = 363,
    FunctionDeclarator = 375,
    PointerDeclarator = 371,
    ReferenceDeclarator = 485,
    ArrayDeclarator = 379,
    InitDeclarator = 383,
    VariadicDeclarator = 484,
    ParenthesizedDeclarator = 364,
    AttributedDeclarator = 368,
    StructuredBindingDeclarator = 505,

    // ========================================
    // AST Nodes - Function Components
    // ========================================
    ParameterList = 399,
    ParameterDeclaration = 400,
    OptionalParameterDeclaration = 482,
    VariadicParameterDeclaration = 483,
    FunctionDeclaratorSeq = 507,
    FunctionExceptionSpecification = 509,
    FunctionPostfix = 511,
    TrailingReturnType = 512,
    Noexcept = 513,
    ThrowSpecifier = 514,
    RefQualifier = 506,
    ConstructorSpecifiers = 490,
    ExplicitFunctionSpecifier = 471,

    // ========================================
    // AST Nodes - Initializers
    // ========================================
    FieldInitializerList = 487,
    FieldInitializer = 488,
    InitializerList = 453,
    InitializerPair = 454,
    SubscriptDesignator = 455,
    SubscriptRangeDesignator = 456,
    FieldDesignator = 457,

    // ========================================
    // AST Nodes - Statements
    // ========================================
    CompoundStatement = 384,
    Statement = 402,
    TopLevelStatement = 403,
    LabeledStatement = 404,
    ExpressionStatement = 405,
    AttributedStatement = 401,

    // Control flow statements
    IfStatement = 407,
    ElseClause = 408,
    SwitchStatement = 409,
    CaseStatement = 410,
    WhileStatement = 411,
    DoStatement = 412,
    ForStatement = 413,
    ForRangeLoop = 527,
    InitStatement = 529,
    ConditionClause = 530,

    // Jump statements
    ReturnStatement = 415,
    BreakStatement = 416,
    ContinueStatement = 417,
    GotoStatement = 418,

    // Exception statements
    TryStatement = 493,
    CatchClause = 536,
    ThrowStatement = 534,
    CoReturnStatement = 532,
    CoYieldStatement = 533,

    // SEH statements
    SehTryStatement = 419,
    SehExceptClause = 420,
    SehFinallyClause = 421,
    SehLeaveStatement = 422,

    // ========================================
    // AST Nodes - Expressions
    // ========================================
    Expression = 423,
    ParenthesizedExpression = 337,
    CommaExpression = 425,
    ConditionalExpression = 426,
    AssignmentExpression = 427,

    // Unary and Binary
    UnaryExpression = 339,
    BinaryExpression = 342,
    UpdateExpression = 431,
    PointerExpression = 428,
    CastExpression = 432,

    // Function calls and access
    CallExpression = 340,
    ArgumentList = 341,
    FieldExpression = 450,
    SubscriptExpression = 438,
    SubscriptArgumentList = 538,

    // Special expressions
    SizeofExpression = 434,
    AlignofExpression = 435,
    OffsetofExpression = 436,
    GenericExpression = 437,
    ExtensionExpression = 448,

    // Modern C++ expressions
    LambdaExpression = 553,
    LambdaCaptureSpecifier = 554,
    LambdaDefaultCapture = 555,
    LambdaCapture = 558,
    NewExpression = 540,
    NewDeclarator = 541,
    DeleteExpression = 542,
    CoAwaitExpression = 539,

    // Template and dependent expressions
    DependentName = 569,
    ScopeResolution = 572,
    QualifiedIdentifier = 573,
    DestructorName = 568,

    // Operator-related
    OperatorCast = 486,
    OperatorName = 578,
    UserDefinedLiteral = 579,

    // Fold expressions (C++17)
    FoldExpression = 564,
    ParameterPackExpansion = 565,

    // ========================================
    // AST Nodes - Concepts and Requirements (C++20)
    // ========================================
    ConceptDefinition = 526,
    RequiresClause = 550,
    RequiresExpression = 552,
    RequirementSeq = 546,
    TypeRequirement = 543,
    CompoundRequirement = 544,
    ConstraintConjunction = 547,
    ConstraintDisjunction = 548,

    // ========================================
    // AST Nodes - Strings and Literals
    // ========================================
    String = 424,
    StringLiteral = 460,
    ConcatenatedString = 459,
    RawStringLiteral = 537,
    CharLiteral = 458,
    Null = 461,

    // ========================================
    // AST Nodes - Other
    // ========================================
    DeclarationList = 359,
    DeclarationModifiers = 348,
    DeclarationSpecifiers = 349,
    StorageClassSpecifier = 385,
    AttributeSpecifier = 351,
    Attribute = 352,
    EnumeratorList = 391,
    Enumerator = 398,
    BitfieldClause = 397,
    CompoundLiteralExpression = 451,

    // ========================================
    // AST Nodes - GNU ASM
    // ========================================
    GnuAsmExpression = 440,
    GnuAsmQualifier = 441,
    GnuAsmOutputOperandList = 442,
    GnuAsmOutputOperand = 443,
    GnuAsmInputOperandList = 444,
    GnuAsmInputOperand = 445,
    GnuAsmClobberList = 446,
    GnuAsmGotoList = 447,

    // ========================================
    // AST Nodes - MS-specific
    // ========================================
    MsDeclspecModifier = 354,
    MsBasedModifier = 355,
    MsCallModifier = 356,
    MsUnalignedPtrModifier = 357,
    MsPointerModifier = 358,

    // ========================================
    // AST Nodes - Macros
    // ========================================
    AloneMacro = 217,
    AloneMacroCall = 580,
    MacroAnnotation = 582,
    MacroStatement = 236,

    // ========================================
    // Special Method Clauses
    // ========================================
    DefaultMethodClause = 496,
    DeleteMethodClause = 497,
    PureVirtualClause = 498,
    PureVirtualClauseToken1 = 189,

    // ========================================
    // Special Operators for User-defined Literals
    // ========================================
    LPARENRPAREN = 212,     // () operator
    LBRACKRBRACK = 213,     // [] operator

    // ========================================
    // Identifiers
    // ========================================
    FieldIdentifier = 633,
    NamespaceIdentifier = 634,
    StatementIdentifier = 636,
    TypeIdentifier = 637,

    // ========================================
    // Error Node
    // ========================================
    Error = 638,
}

impl From<CppToken> for &'static str {
    fn from(tok: CppToken) -> Self {
        match tok {
            CppToken::End => "end",
            CppToken::Identifier => "identifier",

            // Preprocessor
            CppToken::HASHinclude => "#include",
            CppToken::PreprocIncludeToken2 => "preproc_include_token2",
            CppToken::HASHdefine => "#define",
            CppToken::HASHif => "#if",
            CppToken::HASHendif => "#endif",
            CppToken::HASHifdef => "#ifdef",
            CppToken::HASHifndef => "#ifndef",
            CppToken::HASHelse => "#else",
            CppToken::HASHelif => "#elif",
            CppToken::HASHelifdef => "#elifdef",
            CppToken::HASHelifndef => "#elifndef",
            CppToken::PreprocArg => "preproc_arg",
            CppToken::PreprocDirective => "preproc_directive",
            CppToken::PreprocDefined => "preproc_defined",

            // Punctuation
            CppToken::LPAREN => "(",
            CppToken::RPAREN => ")",
            CppToken::LBRACE => "{",
            CppToken::RBRACE => "}",
            CppToken::LBRACK => "[",
            CppToken::RBRACK => "]",
            CppToken::SEMI => ";",
            CppToken::COLON => ":",
            CppToken::COLONCOLON => "::",
            CppToken::COMMA => ",",
            CppToken::DOT => ".",
            CppToken::DOTDOTDOT => "...",
            CppToken::DASHGT => "->",
            CppToken::DASHGTSTAR => "->*",
            CppToken::DOTSTAR => ".*",

            // Arithmetic operators
            CppToken::PLUS => "+",
            CppToken::DASH => "-",
            CppToken::STAR => "*",
            CppToken::SLASH => "/",
            CppToken::PERCENT => "%",
            CppToken::PLUSPLUS => "++",
            CppToken::DASHDASH => "--",

            // Logical operators
            CppToken::BANG => "!",
            CppToken::AMPAMP => "&&",
            CppToken::PIPEPIPE => "||",
            CppToken::TILDE => "~",

            // Bitwise operators
            CppToken::AMP => "&",
            CppToken::PIPE => "|",
            CppToken::CARET => "^",
            CppToken::LTLT => "<<",
            CppToken::GTGT => ">>",

            // Comparison operators
            CppToken::LT => "<",
            CppToken::GT => ">",
            CppToken::GT2 => ">",
            CppToken::LTEQ => "<=",
            CppToken::GTEQ => ">=",
            CppToken::EQEQ => "==",
            CppToken::BANGEQ => "!=",
            CppToken::LTEQGT => "<=>",

            // Assignment operators
            CppToken::EQ => "=",
            CppToken::PLUSEQ => "+=",
            CppToken::DASHEQ => "-=",
            CppToken::STAREQ => "*=",
            CppToken::SLASHEQ => "/=",
            CppToken::PERCENTEQ => "%=",
            CppToken::LTLTEQ => "<<=",
            CppToken::GTGTEQ => ">>=",
            CppToken::AMPEQ => "&=",
            CppToken::CARETEQ => "^=",
            CppToken::PIPEEQ => "|=",
            CppToken::AndEq => "and_eq",
            CppToken::OrEq => "or_eq",
            CppToken::XorEq => "xor_eq",

            // Conditional
            CppToken::QMARK => "?",

            // Alternative tokens
            CppToken::Not => "not",
            CppToken::Compl => "compl",
            CppToken::Or => "or",
            CppToken::And => "and",
            CppToken::Bitor => "bitor",
            CppToken::Xor => "xor",
            CppToken::Bitand => "bitand",
            CppToken::NotEq => "not_eq",

            // Type declaration keywords
            CppToken::Class => "class",
            CppToken::Struct => "struct",
            CppToken::Union => "union",
            CppToken::Enum => "enum",
            CppToken::Typedef => "typedef",
            CppToken::Using => "using",
            CppToken::Typename => "typename",
            CppToken::Namespace => "namespace",
            CppToken::Template => "template",
            CppToken::Friend => "friend",

            // Access specifiers
            CppToken::Public => "public",
            CppToken::Private => "private",
            CppToken::Protected => "protected",

            // Type qualifiers
            CppToken::Const => "const",
            CppToken::Constexpr => "constexpr",
            CppToken::Constinit => "constinit",
            CppToken::Consteval => "consteval",
            CppToken::Volatile => "volatile",
            CppToken::Volatile2 => "__volatile__",
            CppToken::Static => "static",
            CppToken::Extern => "extern",
            CppToken::Virtual => "virtual",
            CppToken::Mutable => "mutable",
            CppToken::Register => "register",
            CppToken::Inline => "inline",
            CppToken::Inline2 => "__inline",
            CppToken::Inline3 => "__inline__",
            CppToken::Forceinline => "__forceinline",
            CppToken::ThreadLocal => "thread_local",
            CppToken::Thread => "__thread",

            // Type modifiers
            CppToken::Signed => "signed",
            CppToken::Unsigned => "unsigned",
            CppToken::Long => "long",
            CppToken::Short => "short",
            CppToken::Restrict => "restrict",
            CppToken::Restrict2 => "__restrict__",
            CppToken::Atomic => "_Atomic",

            // Control flow
            CppToken::If => "if",
            CppToken::Else => "else",
            CppToken::Switch => "switch",
            CppToken::Case => "case",
            CppToken::Default => "default",
            CppToken::While => "while",
            CppToken::Do => "do",
            CppToken::For => "for",
            CppToken::Return => "return",
            CppToken::Break => "break",
            CppToken::Continue => "continue",
            CppToken::Goto => "goto",

            // Exception handling
            CppToken::Try => "__try",
            CppToken::Try2 => "try",
            CppToken::Catch => "catch",
            CppToken::Throw => "throw",
            CppToken::Except => "__except",
            CppToken::Finally => "__finally",
            CppToken::Leave => "__leave",
            CppToken::Noexcept2 => "noexcept",

            // Memory management
            CppToken::New => "new",
            CppToken::Delete => "delete",

            // Modern C++
            CppToken::Auto => "auto",
            CppToken::Decltype3 => "decltype",
            CppToken::Final => "final",
            CppToken::Override => "override",
            CppToken::Explicit => "explicit",
            CppToken::Operator => "operator",
            CppToken::StaticAssert => "static_assert",
            CppToken::Concept => "concept",
            CppToken::Requires => "requires",

            // Coroutines
            CppToken::CoReturn => "co_return",
            CppToken::CoYield => "co_yield",
            CppToken::CoAwait => "co_await",

            // Alignment
            CppToken::Alignas => "alignas",
            CppToken::Alignas2 => "_Alignas",
            CppToken::Alignof => "__alignof__",
            CppToken::Alignof2 => "__alignof",
            CppToken::Alignof3 => "_alignof",
            CppToken::Alignof4 => "alignof",
            CppToken::Alignof5 => "_Alignof",

            // Other keywords
            CppToken::Sizeof => "sizeof",
            CppToken::Offsetof => "offsetof",
            CppToken::Generic => "_Generic",
            CppToken::Asm => "asm",
            CppToken::Asm2 => "__asm__",
            CppToken::Asm3 => "__asm",
            CppToken::Extension => "__extension__",
            CppToken::Noreturn => "_Noreturn",
            CppToken::Noreturn2 => "noreturn",
            CppToken::Nonnull => "_Nonnull",

            // Attributes
            CppToken::Attribute2 => "__attribute__",
            CppToken::Attribute3 => "__attribute",
            CppToken::LBRACKLBRACK => "[[",
            CppToken::RBRACKRBRACK => "]]",
            CppToken::Declspec => "__declspec",
            CppToken::Based => "__based",

            // Calling conventions
            CppToken::Cdecl => "__cdecl",
            CppToken::Clrcall => "__clrcall",
            CppToken::Stdcall => "__stdcall",
            CppToken::Fastcall => "__fastcall",
            CppToken::Thiscall => "__thiscall",
            CppToken::Vectorcall => "__vectorcall",

            // MS-specific
            CppToken::MsRestrictModifier => "ms_restrict_modifier",
            CppToken::MsUnsignedPtrModifier => "ms_unsigned_ptr_modifier",
            CppToken::MsSignedPtrModifier => "ms_signed_ptr_modifier",
            CppToken::Unaligned => "_unaligned",
            CppToken::Unaligned2 => "__unaligned",

            // Primitive types
            CppToken::PrimitiveType => "primitive_type",

            // Literals
            CppToken::NumberLiteral => "number_literal",
            CppToken::True => "true",
            CppToken::False => "false",
            CppToken::NULL => "NULL",
            CppToken::Nullptr => "nullptr",
            CppToken::This => "this",

            // Character literals
            CppToken::SQUOTE => "'",
            CppToken::LSQUOTE => "L'",
            CppToken::USQUOTE => "u'",
            CppToken::USQUOTE2 => "U'",
            CppToken::U8SQUOTE => "u8'",
            CppToken::Character => "character",

            // String literals
            CppToken::DQUOTE => "\"",
            CppToken::LDQUOTE => "L\"",
            CppToken::UDQUOTE => "u\"",
            CppToken::UDQUOTE2 => "U\"",
            CppToken::U8DQUOTE => "u8\"",
            CppToken::StringContent => "string_content",
            CppToken::EscapeSequence => "escape_sequence",

            // Raw string literals
            CppToken::RDQUOTE => "R\"",
            CppToken::LRDQUOTE => "LR\"",
            CppToken::URDQUOTE => "uR\"",
            CppToken::URDQUOTE2 => "UR\"",
            CppToken::U8RDQUOTE => "u8R\"",
            CppToken::RawStringDelimiter => "raw_string_delimiter",
            CppToken::RawStringContent => "raw_string_content",

            // Special strings
            CppToken::SystemLibString => "system_lib_string",
            CppToken::DQUOTEDQUOTE => "\"\"",
            CppToken::LiteralSuffix => "literal_suffix",

            // Comments
            CppToken::Comment => "comment",

            // Preprocessor nodes
            CppToken::PreprocInclude => "preproc_include",
            CppToken::PreprocDef => "preproc_def",
            CppToken::PreprocFunctionDef => "preproc_function_def",
            CppToken::PreprocParams => "preproc_params",
            CppToken::PreprocCall => "preproc_call",
            CppToken::PreprocIf => "preproc_if",
            CppToken::PreprocIfdef => "preproc_ifdef",
            CppToken::PreprocElse => "preproc_else",
            CppToken::PreprocElif => "preproc_elif",
            CppToken::PreprocElifdef => "preproc_elifdef",
            CppToken::PreprocExpression => "_preproc_expression",

            // Top level
            CppToken::TranslationUnit => "translation_unit",
            CppToken::TopLevelItem => "_top_level_item",
            CppToken::BlockItem => "_block_item",

            // Declarations
            CppToken::Declaration => "declaration",
            CppToken::FunctionDefinition => "function_definition",
            CppToken::TypeDefinition => "type_definition",
            CppToken::EmptyDeclaration => "_empty_declaration",
            CppToken::LinkageSpecification => "linkage_specification",
            CppToken::AttributeDeclaration => "attribute_declaration",
            CppToken::FriendDeclaration => "friend_declaration",
            CppToken::UsingDeclaration => "using_declaration",
            CppToken::AliasDeclaration => "alias_declaration",
            CppToken::StaticAssertDeclaration => "static_assert_declaration",

            // Class/Struct/Union
            CppToken::ClassDeclaration => "_class_declaration",
            CppToken::ClassSpecifier => "class_specifier",
            CppToken::StructSpecifier => "struct_specifier",
            CppToken::UnionSpecifier => "union_specifier",
            CppToken::EnumSpecifier => "enum_specifier",
            CppToken::ClassName => "_class_name",
            CppToken::BaseClassClause => "base_class_clause",
            CppToken::FieldDeclarationList => "field_declaration_list",
            CppToken::FieldDeclaration => "field_declaration",
            CppToken::AccessSpecifier => "access_specifier",
            CppToken::VirtualSpecifier => "virtual_specifier",

            // Templates
            CppToken::TemplateDeclaration => "template_declaration",
            CppToken::TemplateInstantiation => "template_instantiation",
            CppToken::TemplateParameterList => "template_parameter_list",
            CppToken::TemplateArgumentList => "template_argument_list",
            CppToken::TemplateType => "template_type",
            CppToken::TemplateMethod => "template_method",
            CppToken::TemplateFunction => "template_function",
            CppToken::TypeParameterDeclaration => "type_parameter_declaration",
            CppToken::VariadicTypeParameterDeclaration => "variadic_type_parameter_declaration",
            CppToken::OptionalTypeParameterDeclaration => "optional_type_parameter_declaration",
            CppToken::TemplateTemplateParameterDeclaration => "template_template_parameter_declaration",

            // Namespaces
            CppToken::NamespaceDefinition => "namespace_definition",
            CppToken::NamespaceAliasDefinition => "namespace_alias_definition",
            CppToken::NamespaceSpecifier => "_namespace_specifier",
            CppToken::NestedNamespaceSpecifier => "nested_namespace_specifier",

            // Types
            CppToken::TypeSpecifier => "type_specifier",
            CppToken::SizedTypeSpecifier => "sized_type_specifier",
            CppToken::TypeDescriptor => "type_descriptor",
            CppToken::TypeQualifier => "type_qualifier",
            CppToken::AlignasQualifier => "alignas_qualifier",
            CppToken::PlaceholderTypeSpecifier => "placeholder_type_specifier",
            CppToken::Decltype => "decltype",
            CppToken::Decltype2 => "decltype",
            CppToken::DependentType => "dependent_type",
            CppToken::EnumBaseClause => "_enum_base_clause",

            // Declarators
            CppToken::Declarator => "_declarator",
            CppToken::FieldDeclarator => "_field_declarator",
            CppToken::TypeDeclarator => "_type_declarator",
            CppToken::AbstractDeclarator => "_abstract_declarator",
            CppToken::FunctionDeclarator => "function_declarator",
            CppToken::PointerDeclarator => "pointer_declarator",
            CppToken::ReferenceDeclarator => "reference_declarator",
            CppToken::ArrayDeclarator => "array_declarator",
            CppToken::InitDeclarator => "init_declarator",
            CppToken::VariadicDeclarator => "variadic_declarator",
            CppToken::ParenthesizedDeclarator => "parenthesized_declarator",
            CppToken::AttributedDeclarator => "attributed_declarator",
            CppToken::StructuredBindingDeclarator => "structured_binding_declarator",

            // Function components
            CppToken::ParameterList => "parameter_list",
            CppToken::ParameterDeclaration => "parameter_declaration",
            CppToken::OptionalParameterDeclaration => "optional_parameter_declaration",
            CppToken::VariadicParameterDeclaration => "variadic_parameter_declaration",
            CppToken::FunctionDeclaratorSeq => "_function_declarator_seq",
            CppToken::FunctionExceptionSpecification => "_function_exception_specification",
            CppToken::FunctionPostfix => "_function_postfix",
            CppToken::TrailingReturnType => "trailing_return_type",
            CppToken::Noexcept => "noexcept",
            CppToken::ThrowSpecifier => "throw_specifier",
            CppToken::RefQualifier => "ref_qualifier",
            CppToken::ConstructorSpecifiers => "_constructor_specifiers",
            CppToken::ExplicitFunctionSpecifier => "explicit_function_specifier",

            // Initializers
            CppToken::FieldInitializerList => "field_initializer_list",
            CppToken::FieldInitializer => "field_initializer",
            CppToken::InitializerList => "initializer_list",
            CppToken::InitializerPair => "initializer_pair",
            CppToken::SubscriptDesignator => "subscript_designator",
            CppToken::SubscriptRangeDesignator => "subscript_range_designator",
            CppToken::FieldDesignator => "field_designator",

            // Statements
            CppToken::CompoundStatement => "compound_statement",
            CppToken::Statement => "statement",
            CppToken::TopLevelStatement => "_top_level_statement",
            CppToken::LabeledStatement => "labeled_statement",
            CppToken::ExpressionStatement => "expression_statement",
            CppToken::AttributedStatement => "attributed_statement",

            // Control flow statements
            CppToken::IfStatement => "if_statement",
            CppToken::ElseClause => "else_clause",
            CppToken::SwitchStatement => "switch_statement",
            CppToken::CaseStatement => "case_statement",
            CppToken::WhileStatement => "while_statement",
            CppToken::DoStatement => "do_statement",
            CppToken::ForStatement => "for_statement",
            CppToken::ForRangeLoop => "for_range_loop",
            CppToken::InitStatement => "init_statement",
            CppToken::ConditionClause => "condition_clause",

            // Jump statements
            CppToken::ReturnStatement => "return_statement",
            CppToken::BreakStatement => "break_statement",
            CppToken::ContinueStatement => "continue_statement",
            CppToken::GotoStatement => "goto_statement",

            // Exception statements
            CppToken::TryStatement => "try_statement",
            CppToken::CatchClause => "catch_clause",
            CppToken::ThrowStatement => "throw_statement",
            CppToken::CoReturnStatement => "co_return_statement",
            CppToken::CoYieldStatement => "co_yield_statement",

            // SEH statements
            CppToken::SehTryStatement => "seh_try_statement",
            CppToken::SehExceptClause => "seh_except_clause",
            CppToken::SehFinallyClause => "seh_finally_clause",
            CppToken::SehLeaveStatement => "seh_leave_statement",

            // Expressions
            CppToken::Expression => "expression",
            CppToken::ParenthesizedExpression => "parenthesized_expression",
            CppToken::CommaExpression => "comma_expression",
            CppToken::ConditionalExpression => "conditional_expression",
            CppToken::AssignmentExpression => "assignment_expression",

            // Unary and Binary
            CppToken::UnaryExpression => "unary_expression",
            CppToken::BinaryExpression => "binary_expression",
            CppToken::UpdateExpression => "update_expression",
            CppToken::PointerExpression => "pointer_expression",
            CppToken::CastExpression => "cast_expression",

            // Function calls and access
            CppToken::CallExpression => "call_expression",
            CppToken::ArgumentList => "argument_list",
            CppToken::FieldExpression => "field_expression",
            CppToken::SubscriptExpression => "subscript_expression",
            CppToken::SubscriptArgumentList => "subscript_argument_list",

            // Special expressions
            CppToken::SizeofExpression => "sizeof_expression",
            CppToken::AlignofExpression => "alignof_expression",
            CppToken::OffsetofExpression => "offsetof_expression",
            CppToken::GenericExpression => "generic_expression",
            CppToken::ExtensionExpression => "extension_expression",

            // Modern C++ expressions
            CppToken::LambdaExpression => "lambda_expression",
            CppToken::LambdaCaptureSpecifier => "lambda_capture_specifier",
            CppToken::LambdaDefaultCapture => "lambda_default_capture",
            CppToken::LambdaCapture => "_lambda_capture",
            CppToken::NewExpression => "new_expression",
            CppToken::NewDeclarator => "new_declarator",
            CppToken::DeleteExpression => "delete_expression",
            CppToken::CoAwaitExpression => "co_await_expression",

            // Template and dependent
            CppToken::DependentName => "dependent_name",
            CppToken::ScopeResolution => "_scope_resolution",
            CppToken::QualifiedIdentifier => "qualified_identifier",
            CppToken::DestructorName => "destructor_name",

            // Operator-related
            CppToken::OperatorCast => "operator_cast",
            CppToken::OperatorName => "operator_name",
            CppToken::UserDefinedLiteral => "user_defined_literal",

            // Fold expressions
            CppToken::FoldExpression => "fold_expression",
            CppToken::ParameterPackExpansion => "parameter_pack_expansion",

            // Concepts and requirements
            CppToken::ConceptDefinition => "concept_definition",
            CppToken::RequiresClause => "requires_clause",
            CppToken::RequiresExpression => "requires_expression",
            CppToken::RequirementSeq => "requirement_seq",
            CppToken::TypeRequirement => "type_requirement",
            CppToken::CompoundRequirement => "compound_requirement",
            CppToken::ConstraintConjunction => "constraint_conjunction",
            CppToken::ConstraintDisjunction => "constraint_disjunction",

            // Strings and literals
            CppToken::String => "_string",
            CppToken::StringLiteral => "string_literal",
            CppToken::ConcatenatedString => "concatenated_string",
            CppToken::RawStringLiteral => "raw_string_literal",
            CppToken::CharLiteral => "char_literal",
            CppToken::Null => "null",

            // Other
            CppToken::DeclarationList => "declaration_list",
            CppToken::DeclarationModifiers => "_declaration_modifiers",
            CppToken::DeclarationSpecifiers => "_declaration_specifiers",
            CppToken::StorageClassSpecifier => "storage_class_specifier",
            CppToken::AttributeSpecifier => "attribute_specifier",
            CppToken::Attribute => "attribute",
            CppToken::EnumeratorList => "enumerator_list",
            CppToken::Enumerator => "enumerator",
            CppToken::BitfieldClause => "bitfield_clause",
            CppToken::CompoundLiteralExpression => "compound_literal_expression",

            // GNU ASM
            CppToken::GnuAsmExpression => "gnu_asm_expression",
            CppToken::GnuAsmQualifier => "gnu_asm_qualifier",
            CppToken::GnuAsmOutputOperandList => "gnu_asm_output_operand_list",
            CppToken::GnuAsmOutputOperand => "gnu_asm_output_operand",
            CppToken::GnuAsmInputOperandList => "gnu_asm_input_operand_list",
            CppToken::GnuAsmInputOperand => "gnu_asm_input_operand",
            CppToken::GnuAsmClobberList => "gnu_asm_clobber_list",
            CppToken::GnuAsmGotoList => "gnu_asm_goto_list",

            // MS-specific
            CppToken::MsDeclspecModifier => "ms_declspec_modifier",
            CppToken::MsBasedModifier => "ms_based_modifier",
            CppToken::MsCallModifier => "ms_call_modifier",
            CppToken::MsUnalignedPtrModifier => "ms_unaligned_ptr_modifier",
            CppToken::MsPointerModifier => "ms_pointer_modifier",

            // Macros
            CppToken::AloneMacro => "alone_macro",
            CppToken::AloneMacroCall => "alone_macro_call",
            CppToken::MacroAnnotation => "macro_annotation",
            CppToken::MacroStatement => "macro_statement",

            // Special method clauses
            CppToken::DefaultMethodClause => "default_method_clause",
            CppToken::DeleteMethodClause => "delete_method_clause",
            CppToken::PureVirtualClause => "pure_virtual_clause",
            CppToken::PureVirtualClauseToken1 => "pure_virtual_clause_token1",

            // Special operators
            CppToken::LPARENRPAREN => "()",
            CppToken::LBRACKRBRACK => "[]",

            // Identifiers
            CppToken::FieldIdentifier => "field_identifier",
            CppToken::NamespaceIdentifier => "namespace_identifier",
            CppToken::StatementIdentifier => "statement_identifier",
            CppToken::TypeIdentifier => "type_identifier",

            CppToken::Error => "ERROR",
        }
    }
}

impl From<u16> for CppToken {
    fn from(x: u16) -> Self {
        num::FromPrimitive::from_u16(x).unwrap_or(Self::Error)
    }
}

impl PartialEq<u16> for CppToken {
    fn eq(&self, x: &u16) -> bool {
        *self == Into::<Self>::into(*x)
    }
}

impl PartialEq<CppToken> for u16 {
    fn eq(&self, x: &CppToken) -> bool {
        *x == *self
    }
}

/// C++ language implementation.
///
/// Provides language-specific information and tree-sitter grammar access for C++.
/// Supports comprehensive C++ analysis including:
/// - Template metaprogramming and variadic templates
/// - Modern C++ features (concepts, coroutines, ranges, constexpr)
/// - Preprocessor directive analysis
/// - Virtual functions and polymorphism
/// - RAII patterns and smart pointers
/// - Operator overloading
pub struct CppLanguage;

impl LanguageInfo for CppLanguage {
    fn get_lang() -> Lang {
        Lang::Cpp
    }

    fn get_lang_name() -> &'static str {
        "c++"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpp_language_info() {
        assert_eq!(CppLanguage::get_lang(), Lang::Cpp);
        assert_eq!(CppLanguage::get_lang_name(), "c++");
    }

    #[test]
    fn test_cpp_token_conversions() {
        let tok: CppToken = 1.into();
        assert_eq!(tok, CppToken::Identifier);

        let tok: CppToken = 308.into();
        assert_eq!(tok, CppToken::TranslationUnit);

        let tok: CppToken = 343.into();
        assert_eq!(tok, CppToken::FunctionDefinition);

        let tok: CppToken = 468.into();
        assert_eq!(tok, CppToken::ClassSpecifier);
    }

    #[test]
    fn test_cpp_token_to_string() {
        assert_eq!(<&str>::from(CppToken::Class), "class");
        assert_eq!(<&str>::from(CppToken::Namespace), "namespace");
        assert_eq!(<&str>::from(CppToken::Template), "template");
        assert_eq!(<&str>::from(CppToken::FunctionDefinition), "function_definition");
        assert_eq!(<&str>::from(CppToken::COLONCOLON), "::");
        assert_eq!(<&str>::from(CppToken::DASHGT), "->");
        assert_eq!(<&str>::from(CppToken::TemplateDeclaration), "template_declaration");
    }

    #[test]
    fn test_cpp_preprocessor_tokens() {
        assert_eq!(<&str>::from(CppToken::HASHinclude), "#include");
        assert_eq!(<&str>::from(CppToken::HASHdefine), "#define");
        assert_eq!(<&str>::from(CppToken::HASHif), "#if");
        assert_eq!(<&str>::from(CppToken::HASHifdef), "#ifdef");
        assert_eq!(<&str>::from(CppToken::HASHifndef), "#ifndef");
        assert_eq!(<&str>::from(CppToken::HASHendif), "#endif");
    }

    #[test]
    fn test_cpp_modern_features() {
        assert_eq!(<&str>::from(CppToken::Auto), "auto");
        assert_eq!(<&str>::from(CppToken::Decltype3), "decltype");
        assert_eq!(<&str>::from(CppToken::Constexpr), "constexpr");
        assert_eq!(<&str>::from(CppToken::Concept), "concept");
        assert_eq!(<&str>::from(CppToken::Requires), "requires");
        assert_eq!(<&str>::from(CppToken::CoAwait), "co_await");
        assert_eq!(<&str>::from(CppToken::CoReturn), "co_return");
        assert_eq!(<&str>::from(CppToken::CoYield), "co_yield");
    }

    #[test]
    fn test_cpp_operators() {
        assert_eq!(<&str>::from(CppToken::COLONCOLON), "::");
        assert_eq!(<&str>::from(CppToken::DASHGT), "->");
        assert_eq!(<&str>::from(CppToken::DASHGTSTAR), "->*");
        assert_eq!(<&str>::from(CppToken::DOTSTAR), ".*");
        assert_eq!(<&str>::from(CppToken::LTEQGT), "<=>");
        assert_eq!(<&str>::from(CppToken::LTLT), "<<");
        assert_eq!(<&str>::from(CppToken::GTGT), ">>");
    }

    #[test]
    fn test_cpp_string_literals() {
        assert_eq!(<&str>::from(CppToken::RDQUOTE), "R\"");
        assert_eq!(<&str>::from(CppToken::LDQUOTE), "L\"");
        assert_eq!(<&str>::from(CppToken::U8DQUOTE), "u8\"");
        assert_eq!(<&str>::from(CppToken::RawStringLiteral), "raw_string_literal");
        assert_eq!(<&str>::from(CppToken::ConcatenatedString), "concatenated_string");
    }

    #[test]
    fn test_cpp_template_tokens() {
        assert_eq!(<&str>::from(CppToken::Template), "template");
        assert_eq!(<&str>::from(CppToken::TemplateDeclaration), "template_declaration");
        assert_eq!(<&str>::from(CppToken::TemplateType), "template_type");
        assert_eq!(<&str>::from(CppToken::TemplateFunction), "template_function");
        assert_eq!(<&str>::from(CppToken::TemplateArgumentList), "template_argument_list");
    }
}
