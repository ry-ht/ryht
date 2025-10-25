//! AST types for parsed code structures.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a parsed function.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FunctionInfo {
    /// Function name
    pub name: String,

    /// Fully qualified name (e.g., "module::MyStruct::my_method")
    pub qualified_name: String,

    /// Function parameters
    pub parameters: Vec<Parameter>,

    /// Return type (None for functions returning ())
    pub return_type: Option<String>,

    /// Visibility (pub, pub(crate), private)
    pub visibility: Visibility,

    /// Attributes/annotations (e.g., #[test], #[async])
    pub attributes: Vec<String>,

    /// Function body as text
    pub body: String,

    /// Starting line number (1-indexed)
    pub start_line: usize,

    /// Ending line number (1-indexed)
    pub end_line: usize,

    /// Documentation/docstring
    pub docstring: Option<String>,

    /// Whether the function is async
    pub is_async: bool,

    /// Whether the function is const
    pub is_const: bool,

    /// Whether the function is unsafe
    pub is_unsafe: bool,

    /// Generic type parameters (e.g., ["T", "U: Clone"])
    pub generics: Vec<String>,

    /// Where clause constraints
    pub where_clause: Option<String>,

    /// Cyclomatic complexity
    pub complexity: Option<u32>,
}

/// Represents a function parameter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Parameter {
    /// Parameter name
    pub name: String,

    /// Parameter type
    pub param_type: String,

    /// Default value (if any)
    pub default_value: Option<String>,

    /// Whether it's a self parameter
    pub is_self: bool,

    /// Whether it's mutable
    pub is_mut: bool,

    /// Whether it's a reference
    pub is_reference: bool,
}

/// Visibility of a code item.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    PublicCrate,
    PublicSuper,
    PublicIn,
    Private,
}

impl std::fmt::Display for Visibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Visibility::Public => write!(f, "pub"),
            Visibility::PublicCrate => write!(f, "pub(crate)"),
            Visibility::PublicSuper => write!(f, "pub(super)"),
            Visibility::PublicIn => write!(f, "pub(in ...)"),
            Visibility::Private => write!(f, "private"),
        }
    }
}

/// Represents a parsed struct.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructInfo {
    /// Struct name
    pub name: String,

    /// Fully qualified name
    pub qualified_name: String,

    /// Fields
    pub fields: Vec<Field>,

    /// Visibility
    pub visibility: Visibility,

    /// Attributes
    pub attributes: Vec<String>,

    /// Starting line number
    pub start_line: usize,

    /// Ending line number
    pub end_line: usize,

    /// Documentation
    pub docstring: Option<String>,

    /// Generic type parameters
    pub generics: Vec<String>,

    /// Where clause
    pub where_clause: Option<String>,

    /// Whether it's a tuple struct
    pub is_tuple_struct: bool,

    /// Whether it's a unit struct
    pub is_unit_struct: bool,
}

/// Represents a struct field.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Field {
    /// Field name
    pub name: String,

    /// Field type
    pub field_type: String,

    /// Visibility
    pub visibility: Visibility,

    /// Attributes
    pub attributes: Vec<String>,

    /// Documentation
    pub docstring: Option<String>,
}

/// Represents a parsed enum.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnumInfo {
    /// Enum name
    pub name: String,

    /// Fully qualified name
    pub qualified_name: String,

    /// Variants
    pub variants: Vec<EnumVariant>,

    /// Visibility
    pub visibility: Visibility,

    /// Attributes
    pub attributes: Vec<String>,

    /// Starting line number
    pub start_line: usize,

    /// Ending line number
    pub end_line: usize,

    /// Documentation
    pub docstring: Option<String>,

    /// Generic type parameters
    pub generics: Vec<String>,

    /// Where clause
    pub where_clause: Option<String>,
}

/// Represents an enum variant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnumVariant {
    /// Variant name
    pub name: String,

    /// Fields (for struct variants)
    pub fields: Vec<Field>,

    /// Tuple fields (for tuple variants)
    pub tuple_fields: Vec<String>,

    /// Discriminant value (for C-like enums)
    pub discriminant: Option<String>,

    /// Attributes
    pub attributes: Vec<String>,

    /// Documentation
    pub docstring: Option<String>,
}

/// Represents a parsed trait.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraitInfo {
    /// Trait name
    pub name: String,

    /// Fully qualified name
    pub qualified_name: String,

    /// Methods
    pub methods: Vec<FunctionInfo>,

    /// Associated types
    pub associated_types: Vec<String>,

    /// Visibility
    pub visibility: Visibility,

    /// Attributes
    pub attributes: Vec<String>,

    /// Starting line number
    pub start_line: usize,

    /// Ending line number
    pub end_line: usize,

    /// Documentation
    pub docstring: Option<String>,

    /// Generic type parameters
    pub generics: Vec<String>,

    /// Where clause
    pub where_clause: Option<String>,

    /// Supertraits
    pub supertraits: Vec<String>,

    /// Whether it's unsafe
    pub is_unsafe: bool,
}

/// Represents an impl block.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImplInfo {
    /// Type being implemented for
    pub type_name: String,

    /// Trait being implemented (None for inherent impl)
    pub trait_name: Option<String>,

    /// Methods
    pub methods: Vec<FunctionInfo>,

    /// Associated types
    pub associated_types: Vec<String>,

    /// Attributes
    pub attributes: Vec<String>,

    /// Starting line number
    pub start_line: usize,

    /// Ending line number
    pub end_line: usize,

    /// Generic type parameters
    pub generics: Vec<String>,

    /// Where clause
    pub where_clause: Option<String>,

    /// Whether it's unsafe
    pub is_unsafe: bool,
}

/// Represents a module.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModuleInfo {
    /// Module name
    pub name: String,

    /// Fully qualified name
    pub qualified_name: String,

    /// Visibility
    pub visibility: Visibility,

    /// Attributes
    pub attributes: Vec<String>,

    /// Starting line number
    pub start_line: usize,

    /// Ending line number
    pub end_line: usize,

    /// Documentation
    pub docstring: Option<String>,

    /// Whether it's an inline module
    pub is_inline: bool,
}

/// Complete parsed file information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParsedFile {
    /// File path
    pub path: String,

    /// Functions
    pub functions: Vec<FunctionInfo>,

    /// Structs
    pub structs: Vec<StructInfo>,

    /// Enums
    pub enums: Vec<EnumInfo>,

    /// Traits
    pub traits: Vec<TraitInfo>,

    /// Impl blocks
    pub impls: Vec<ImplInfo>,

    /// Modules
    pub modules: Vec<ModuleInfo>,

    /// Use statements/imports
    pub imports: Vec<String>,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl ParsedFile {
    pub fn new(path: String) -> Self {
        Self {
            path,
            functions: Vec::new(),
            structs: Vec::new(),
            enums: Vec::new(),
            traits: Vec::new(),
            impls: Vec::new(),
            modules: Vec::new(),
            imports: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}
