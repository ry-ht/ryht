//! Special C/C++ Keywords and Type Names
//!
//! This module contains a comprehensive list of C/C++ keywords, built-in types,
//! and special identifiers that should not be treated as user-defined macros
//! during code analysis.
//!
//! # Categories
//!
//! The special keywords include:
//! - Language keywords (const, static, inline, etc.)
//! - Built-in types (int, char, double, etc.)
//! - Standard library types (size_t, ptrdiff_t, wchar_t, etc.)
//! - Fixed-width integer types (int8_t, uint32_t, etc.)
//! - C++ specific keywords (namespace, constexpr, nullptr, etc.)
//! - Boolean literals (true, false)
//! - Special values (NULL)
//!
//! # Usage
//!
//! These identifiers should be excluded when collecting user-defined macros
//! to avoid counting language features as custom macros.

/// Complete list of special C/C++ keywords and type names.
///
/// These identifiers are part of the C/C++ language or standard library
/// and should not be counted as user-defined macros.
const SPECIAL_KEYWORDS: &[&str] = &[
    // Special constants
    "NULL",

    // Boolean type and values
    "bool", "true", "false",

    // Basic types
    "char", "short", "int", "long",
    "float", "double",
    "signed", "unsigned",
    "void",

    // Character types
    "char8_t", "char16_t", "char32_t", "char64_t",
    "wchar_t", "charptr_t",

    // Fixed-width integer types from stdint.h
    "int8_t", "int16_t", "int32_t", "int64_t",
    "uint8_t", "uint16_t", "uint32_t", "uint64_t",

    // Fast minimum-width integer types
    "int_fast8_t", "int_fast16_t", "int_fast32_t", "int_fast64_t",
    "uint_fast8_t", "uint_fast16_t", "uint_fast32_t", "uint_fast64_t",

    // Smallest minimum-width integer types
    "int_least8_t", "int_least16_t", "int_least32_t", "int_least64_t",
    "uint_least8_t", "uint_least16_t", "uint_least32_t", "uint_least64_t",

    // Maximum-width integer types
    "intmax_t", "uintmax_t",

    // Pointer-width integer types
    "intptr_t", "uintptr_t",

    // Standard library types
    "size_t", "ssize_t", "ptrdiff_t",
    "max_align_t",

    // Type qualifiers
    "const", "volatile", "restrict",

    // Storage class specifiers
    "static", "extern", "auto", "register",

    // Function specifiers
    "inline", "explicit",

    // C++ specific keywords
    "constexpr", "mutable", "namespace", "nullptr",
];

/// Checks if a given identifier is a special C/C++ keyword or type.
///
/// This function determines whether an identifier should be excluded from
/// user-defined macro collection during code analysis.
///
/// # Arguments
///
/// * `name` - The identifier name to check
///
/// # Returns
///
/// `true` if the identifier is a special keyword/type, `false` otherwise.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::c_specials::is_special_keyword;
///
/// assert!(is_special_keyword("int"));
/// assert!(is_special_keyword("NULL"));
/// assert!(is_special_keyword("size_t"));
/// assert!(is_special_keyword("constexpr"));
/// assert!(!is_special_keyword("MY_MACRO"));
/// ```
#[inline]
pub fn is_special_keyword(name: &str) -> bool {
    SPECIAL_KEYWORDS.contains(&name)
}

/// Returns the complete list of special C/C++ keywords and types.
///
/// This can be useful for documentation, tooling, or batch processing.
///
/// # Returns
///
/// A slice containing all special keyword names.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::c_specials::get_all_special_keywords;
///
/// let keywords = get_all_special_keywords();
/// assert!(keywords.len() > 50);
/// assert!(keywords.contains(&"int"));
/// assert!(keywords.contains(&"NULL"));
/// ```
pub fn get_all_special_keywords() -> &'static [&'static str] {
    SPECIAL_KEYWORDS
}

/// Returns the number of special C/C++ keywords and types.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::c_specials::special_keyword_count;
///
/// let count = special_keyword_count();
/// assert!(count > 50);
/// ```
pub fn special_keyword_count() -> usize {
    SPECIAL_KEYWORDS.len()
}

/// Categorizes special keywords by type for better analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordCategory {
    /// Language keywords (const, static, etc.)
    LanguageKeyword,
    /// Basic types (int, char, etc.)
    BasicType,
    /// Standard library types (size_t, etc.)
    StdLibType,
    /// Fixed-width integer types (int32_t, etc.)
    FixedWidthType,
    /// C++ specific features
    CppSpecific,
    /// Special constants (NULL, true, false)
    Constant,
}

/// Gets the category of a special keyword.
///
/// # Arguments
///
/// * `name` - The keyword name to categorize
///
/// # Returns
///
/// An `Option<KeywordCategory>` - `Some(category)` if it's a special keyword,
/// `None` if it's not recognized.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::c_specials::{get_keyword_category, KeywordCategory};
///
/// assert_eq!(get_keyword_category("int"), Some(KeywordCategory::BasicType));
/// assert_eq!(get_keyword_category("size_t"), Some(KeywordCategory::StdLibType));
/// assert_eq!(get_keyword_category("nullptr"), Some(KeywordCategory::CppSpecific));
/// assert_eq!(get_keyword_category("MY_MACRO"), None);
/// ```
pub fn get_keyword_category(name: &str) -> Option<KeywordCategory> {
    match name {
        // Special constants
        "NULL" | "true" | "false" => Some(KeywordCategory::Constant),

        // Basic types
        "bool" | "char" | "short" | "int" | "long" |
        "float" | "double" | "signed" | "unsigned" | "void" => {
            Some(KeywordCategory::BasicType)
        }

        // Character types
        "char8_t" | "char16_t" | "char32_t" | "char64_t" |
        "wchar_t" | "charptr_t" => {
            Some(KeywordCategory::BasicType)
        }

        // Fixed-width integer types
        "int8_t" | "int16_t" | "int32_t" | "int64_t" |
        "uint8_t" | "uint16_t" | "uint32_t" | "uint64_t" |
        "int_fast8_t" | "int_fast16_t" | "int_fast32_t" | "int_fast64_t" |
        "uint_fast8_t" | "uint_fast16_t" | "uint_fast32_t" | "uint_fast64_t" |
        "int_least8_t" | "int_least16_t" | "int_least32_t" | "int_least64_t" |
        "uint_least8_t" | "uint_least16_t" | "uint_least32_t" | "uint_least64_t" |
        "intmax_t" | "uintmax_t" | "intptr_t" | "uintptr_t" => {
            Some(KeywordCategory::FixedWidthType)
        }

        // Standard library types
        "size_t" | "ssize_t" | "ptrdiff_t" | "max_align_t" => {
            Some(KeywordCategory::StdLibType)
        }

        // Type qualifiers and storage class specifiers
        "const" | "volatile" | "restrict" |
        "static" | "extern" | "auto" | "register" |
        "inline" | "explicit" => {
            Some(KeywordCategory::LanguageKeyword)
        }

        // C++ specific
        "constexpr" | "mutable" | "namespace" | "nullptr" => {
            Some(KeywordCategory::CppSpecific)
        }

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_special_keyword() {
        // Test basic types
        assert!(is_special_keyword("int"));
        assert!(is_special_keyword("char"));
        assert!(is_special_keyword("double"));
        assert!(is_special_keyword("void"));

        // Test fixed-width types
        assert!(is_special_keyword("int32_t"));
        assert!(is_special_keyword("uint64_t"));
        assert!(is_special_keyword("int_fast16_t"));
        assert!(is_special_keyword("uint_least32_t"));

        // Test standard library types
        assert!(is_special_keyword("size_t"));
        assert!(is_special_keyword("ptrdiff_t"));
        assert!(is_special_keyword("wchar_t"));

        // Test language keywords
        assert!(is_special_keyword("const"));
        assert!(is_special_keyword("static"));
        assert!(is_special_keyword("inline"));
        assert!(is_special_keyword("restrict"));

        // Test C++ specific
        assert!(is_special_keyword("constexpr"));
        assert!(is_special_keyword("nullptr"));
        assert!(is_special_keyword("namespace"));

        // Test special constants
        assert!(is_special_keyword("NULL"));
        assert!(is_special_keyword("true"));
        assert!(is_special_keyword("false"));
    }

    #[test]
    fn test_not_special_keyword() {
        assert!(!is_special_keyword("MY_MACRO"));
        assert!(!is_special_keyword("MAX_SIZE"));
        assert!(!is_special_keyword("BUFFER_LENGTH"));
        assert!(!is_special_keyword(""));
        assert!(!is_special_keyword("random_identifier"));
    }

    #[test]
    fn test_case_sensitive() {
        // Keywords are case-sensitive
        assert!(is_special_keyword("NULL"));
        assert!(!is_special_keyword("null"));
        assert!(!is_special_keyword("Null"));

        assert!(is_special_keyword("int"));
        assert!(!is_special_keyword("INT"));
        assert!(!is_special_keyword("Int"));
    }

    #[test]
    fn test_get_all_special_keywords() {
        let keywords = get_all_special_keywords();

        // Should have a reasonable number of keywords
        assert!(keywords.len() > 50);

        // Check for presence of various categories
        assert!(keywords.contains(&"int"));
        assert!(keywords.contains(&"NULL"));
        assert!(keywords.contains(&"size_t"));
        assert!(keywords.contains(&"int32_t"));
        assert!(keywords.contains(&"constexpr"));
    }

    #[test]
    fn test_special_keyword_count() {
        let count = special_keyword_count();
        assert!(count > 50);
        assert_eq!(count, get_all_special_keywords().len());
    }

    #[test]
    fn test_no_duplicates() {
        let keywords = get_all_special_keywords();
        let mut seen = std::collections::HashSet::new();

        for &keyword in keywords {
            assert!(seen.insert(keyword), "Duplicate keyword found: {}", keyword);
        }
    }

    #[test]
    fn test_get_keyword_category() {
        // Test basic types
        assert_eq!(get_keyword_category("int"), Some(KeywordCategory::BasicType));
        assert_eq!(get_keyword_category("double"), Some(KeywordCategory::BasicType));
        assert_eq!(get_keyword_category("bool"), Some(KeywordCategory::BasicType));

        // Test fixed-width types
        assert_eq!(get_keyword_category("int32_t"), Some(KeywordCategory::FixedWidthType));
        assert_eq!(get_keyword_category("uint64_t"), Some(KeywordCategory::FixedWidthType));

        // Test standard library types
        assert_eq!(get_keyword_category("size_t"), Some(KeywordCategory::StdLibType));
        assert_eq!(get_keyword_category("ptrdiff_t"), Some(KeywordCategory::StdLibType));

        // Test language keywords
        assert_eq!(get_keyword_category("const"), Some(KeywordCategory::LanguageKeyword));
        assert_eq!(get_keyword_category("static"), Some(KeywordCategory::LanguageKeyword));

        // Test C++ specific
        assert_eq!(get_keyword_category("constexpr"), Some(KeywordCategory::CppSpecific));
        assert_eq!(get_keyword_category("nullptr"), Some(KeywordCategory::CppSpecific));

        // Test constants
        assert_eq!(get_keyword_category("NULL"), Some(KeywordCategory::Constant));
        assert_eq!(get_keyword_category("true"), Some(KeywordCategory::Constant));

        // Test non-keywords
        assert_eq!(get_keyword_category("MY_MACRO"), None);
        assert_eq!(get_keyword_category("custom"), None);
    }

    #[test]
    fn test_all_keywords_have_category() {
        let keywords = get_all_special_keywords();

        for &keyword in keywords {
            assert!(
                get_keyword_category(keyword).is_some(),
                "Keyword '{}' has no category assigned",
                keyword
            );
        }
    }

    #[test]
    fn test_coverage_of_fixed_width_types() {
        let widths = ["8", "16", "32", "64"];

        for width in &widths {
            // Exact-width types
            assert!(is_special_keyword(&format!("int{width}_t")));
            assert!(is_special_keyword(&format!("uint{width}_t")));

            // Fast types
            assert!(is_special_keyword(&format!("int_fast{width}_t")));
            assert!(is_special_keyword(&format!("uint_fast{width}_t")));

            // Least types
            assert!(is_special_keyword(&format!("int_least{width}_t")));
            assert!(is_special_keyword(&format!("uint_least{width}_t")));
        }
    }
}
