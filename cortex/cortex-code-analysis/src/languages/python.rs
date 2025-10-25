//! Python language parser implementation.

use num_derive::FromPrimitive;
use crate::lang::Lang;
use crate::traits::LanguageInfo;

/// Python language token types.
#[derive(Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum PythonToken {
    End = 0,
    Identifier = 1,
    Module = 2,
    FunctionDefinition = 3,
    ClassDefinition = 4,
    Error = 100,
}

impl From<PythonToken> for &'static str {
    fn from(tok: PythonToken) -> Self {
        match tok {
            PythonToken::End => "end",
            PythonToken::Identifier => "identifier",
            PythonToken::Module => "module",
            PythonToken::FunctionDefinition => "function_definition",
            PythonToken::ClassDefinition => "class_definition",
            PythonToken::Error => "ERROR",
        }
    }
}

impl From<u16> for PythonToken {
    fn from(x: u16) -> Self {
        num::FromPrimitive::from_u16(x).unwrap_or(Self::Error)
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
