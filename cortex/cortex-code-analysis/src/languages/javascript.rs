//! JavaScript language parser implementation.

use num_derive::FromPrimitive;
use crate::lang::Lang;
use crate::traits::LanguageInfo;

/// JavaScript language token types.
#[derive(Clone, Debug, PartialEq, Eq, FromPrimitive)]
pub enum JavaScriptToken {
    End = 0,
    Identifier = 1,
    Program = 2,
    FunctionDeclaration = 3,
    Error = 100,
}

impl From<JavaScriptToken> for &'static str {
    fn from(tok: JavaScriptToken) -> Self {
        match tok {
            JavaScriptToken::End => "end",
            JavaScriptToken::Identifier => "identifier",
            JavaScriptToken::Program => "program",
            JavaScriptToken::FunctionDeclaration => "function_declaration",
            JavaScriptToken::Error => "ERROR",
        }
    }
}

impl From<u16> for JavaScriptToken {
    fn from(x: u16) -> Self {
        num::FromPrimitive::from_u16(x).unwrap_or(Self::Error)
    }
}

/// JavaScript language implementation.
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
pub struct JsxLanguage;

impl LanguageInfo for JsxLanguage {
    fn get_lang() -> Lang {
        Lang::Jsx
    }

    fn get_lang_name() -> &'static str {
        "jsx"
    }
}
