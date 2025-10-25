//! Programming language enumeration and metadata.
//!
//! This module defines the supported programming languages and provides
//! utilities for language detection and metadata access.

use std::path::Path;
use tree_sitter::Language as TSLanguage;

/// The list of supported programming languages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lang {
    /// The Rust programming language
    Rust,
    /// The TypeScript language
    TypeScript,
    /// TypeScript with JSX (TSX)
    Tsx,
    /// The JavaScript language
    JavaScript,
    /// JavaScript with JSX
    Jsx,
    /// The Python language
    Python,
    /// The Java language
    Java,
    /// The Kotlin language
    Kotlin,
    /// The C/C++ languages
    Cpp,
}

impl Lang {
    /// Return an iterator over all supported languages.
    ///
    /// # Examples
    ///
    /// ```
    /// use cortex_code_analysis::Lang;
    ///
    /// for lang in Lang::into_enum_iter() {
    ///     println!("{:?}", lang);
    /// }
    /// ```
    pub fn into_enum_iter() -> impl Iterator<Item = Lang> {
        use Lang::*;
        [Rust, TypeScript, Tsx, JavaScript, Jsx, Python, Java, Kotlin, Cpp].into_iter()
    }

    /// Returns the name of a language as a `&str`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cortex_code_analysis::Lang;
    ///
    /// assert_eq!(Lang::Rust.get_name(), "rust");
    /// assert_eq!(Lang::TypeScript.get_name(), "typescript");
    /// ```
    pub fn get_name(&self) -> &'static str {
        match self {
            Lang::Rust => "rust",
            Lang::TypeScript => "typescript",
            Lang::Tsx => "tsx",
            Lang::JavaScript => "javascript",
            Lang::Jsx => "jsx",
            Lang::Python => "python",
            Lang::Java => "java",
            Lang::Kotlin => "kotlin",
            Lang::Cpp => "c/c++",
        }
    }

    /// Returns the display name for a language.
    pub fn display_name(&self) -> &'static str {
        match self {
            Lang::Rust => "Rust",
            Lang::TypeScript => "TypeScript",
            Lang::Tsx => "TSX",
            Lang::JavaScript => "JavaScript",
            Lang::Jsx => "JSX",
            Lang::Python => "Python",
            Lang::Java => "Java",
            Lang::Kotlin => "Kotlin",
            Lang::Cpp => "C/C++",
        }
    }

    /// Get the tree-sitter Language for this language.
    pub fn get_ts_language(&self) -> TSLanguage {
        match self {
            Lang::Rust => tree_sitter_rust::LANGUAGE.into(),
            Lang::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Lang::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Lang::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Lang::Jsx => tree_sitter_javascript::LANGUAGE.into(),
            Lang::Python => tree_sitter_python::LANGUAGE.into(),
            Lang::Java => tree_sitter_java::LANGUAGE.into(),
            Lang::Kotlin => tree_sitter_kotlin_ng::LANGUAGE.into(),
            Lang::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        }
    }

    /// Get file extensions for this language.
    ///
    /// # Examples
    ///
    /// ```
    /// use cortex_code_analysis::Lang;
    ///
    /// assert_eq!(Lang::Rust.extensions(), &["rs"]);
    /// assert_eq!(Lang::TypeScript.extensions(), &["ts"]);
    /// ```
    pub fn extensions(&self) -> &[&str] {
        match self {
            Lang::Rust => &["rs"],
            Lang::TypeScript => &["ts"],
            Lang::Tsx => &["tsx"],
            Lang::JavaScript => &["js", "mjs", "cjs"],
            Lang::Jsx => &["jsx"],
            Lang::Python => &["py"],
            Lang::Java => &["java"],
            Lang::Kotlin => &["kt", "kts"],
            Lang::Cpp => &["cpp", "cxx", "cc", "hxx", "hpp", "c", "h", "hh", "inc", "mm", "m"],
        }
    }

    /// Detect language from file extension.
    ///
    /// # Examples
    ///
    /// ```
    /// use cortex_code_analysis::Lang;
    /// use std::path::Path;
    ///
    /// assert_eq!(Lang::from_path(Path::new("test.rs")), Some(Lang::Rust));
    /// assert_eq!(Lang::from_path(Path::new("test.ts")), Some(Lang::TypeScript));
    /// assert_eq!(Lang::from_path(Path::new("test.tsx")), Some(Lang::Tsx));
    /// assert_eq!(Lang::from_path(Path::new("test.py")), Some(Lang::Python));
    /// ```
    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()?.to_str().and_then(Self::from_extension)
    }

    /// Detect language from file extension string.
    ///
    /// # Examples
    ///
    /// ```
    /// use cortex_code_analysis::Lang;
    ///
    /// assert_eq!(Lang::from_extension("rs"), Some(Lang::Rust));
    /// assert_eq!(Lang::from_extension("ts"), Some(Lang::TypeScript));
    /// assert_eq!(Lang::from_extension("tsx"), Some(Lang::Tsx));
    /// ```
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Lang::Rust),
            "ts" => Some(Lang::TypeScript),
            "tsx" => Some(Lang::Tsx),
            "js" | "mjs" | "cjs" => Some(Lang::JavaScript),
            "jsx" => Some(Lang::Jsx),
            "py" => Some(Lang::Python),
            "java" => Some(Lang::Java),
            "kt" | "kts" => Some(Lang::Kotlin),
            "cpp" | "cxx" | "cc" | "hxx" | "hpp" => Some(Lang::Cpp),
            "c" | "h" | "hh" | "inc" => Some(Lang::Cpp),
            "mm" | "m" => Some(Lang::Cpp), // Objective-C/C++
            _ => None,
        }
    }

    /// Check if this language supports a specific feature.
    pub fn supports_generics(&self) -> bool {
        matches!(
            self,
            Lang::Rust | Lang::TypeScript | Lang::Tsx | Lang::Java | Lang::Kotlin | Lang::Cpp
        )
    }

    /// Check if this language is statically typed.
    pub fn is_statically_typed(&self) -> bool {
        matches!(
            self,
            Lang::Rust | Lang::TypeScript | Lang::Tsx | Lang::Java | Lang::Kotlin | Lang::Cpp
        )
    }
}

impl std::fmt::Display for Lang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_extension() {
        assert_eq!(Lang::from_extension("rs"), Some(Lang::Rust));
        assert_eq!(Lang::from_extension("ts"), Some(Lang::TypeScript));
        assert_eq!(Lang::from_extension("tsx"), Some(Lang::Tsx));
        assert_eq!(Lang::from_extension("js"), Some(Lang::JavaScript));
        assert_eq!(Lang::from_extension("py"), Some(Lang::Python));
        assert_eq!(Lang::from_extension("unknown"), None);
    }

    #[test]
    fn test_from_path() {
        assert_eq!(
            Lang::from_path(Path::new("test.rs")),
            Some(Lang::Rust)
        );
        assert_eq!(
            Lang::from_path(Path::new("test.ts")),
            Some(Lang::TypeScript)
        );
        assert_eq!(
            Lang::from_path(Path::new("test.tsx")),
            Some(Lang::Tsx)
        );
        assert_eq!(
            Lang::from_path(Path::new("test.unknown")),
            None
        );
    }

    #[test]
    fn test_extensions() {
        assert_eq!(Lang::Rust.extensions(), &["rs"]);
        assert_eq!(Lang::TypeScript.extensions(), &["ts"]);
        assert!(Lang::JavaScript.extensions().contains(&"js"));
    }

    #[test]
    fn test_get_name() {
        assert_eq!(Lang::Rust.get_name(), "rust");
        assert_eq!(Lang::TypeScript.get_name(), "typescript");
        assert_eq!(Lang::Python.get_name(), "python");
    }
}
