//! Predefined C/C++ Macros
//!
//! This module contains a comprehensive list of standard C/C++ predefined macros
//! from headers like `<stdint.h>` and `<inttypes.h>`. These macros are part of
//! the C99/C11 and C++11 standards and should be treated specially during code analysis.
//!
//! # Categories
//!
//! The predefined macros include:
//! - Integer type limits (INT8_MAX, UINT32_MAX, etc.)
//! - Integer type constructors (INT16_C, UINT64_C, etc.)
//! - Printf format specifiers (PRId32, PRIu64, PRIxMAX, etc.)
//! - Scanf format specifiers (SCNd32, SCNu64, SCNxMAX, etc.)
//! - Fast and least integer types (INT_FAST16_MAX, INT_LEAST32_MIN, etc.)
//!
//! # Note
//!
//! This file is auto-generated or should be treated as such. Do not manually edit
//! the macro list unless you're adding new standard macros from C/C++ specifications.

/// Complete list of standard C/C++ predefined macros.
///
/// These macros are defined by the C/C++ standard library headers and should
/// not be counted as user-defined macros during code analysis.
const PREDEFINED_MACROS: &[&str] = &[
    // Integer limits for exact-width types
    "INT8_MIN", "INT8_MAX", "INT8_C",
    "INT16_MIN", "INT16_MAX", "INT16_C",
    "INT32_MIN", "INT32_MAX", "INT32_C",
    "INT64_MIN", "INT64_MAX", "INT64_C",

    "UINT8_MIN", "UINT8_MAX", "UINT8_C",
    "UINT16_MIN", "UINT16_MAX", "UINT16_C",
    "UINT32_MIN", "UINT32_MAX", "UINT32_C",
    "UINT64_MIN", "UINT64_MAX", "UINT64_C",

    // Integer limits for fastest minimum-width types
    "INT_FAST8_MIN", "INT_FAST8_MAX",
    "INT_FAST16_MIN", "INT_FAST16_MAX",
    "INT_FAST32_MIN", "INT_FAST32_MAX",
    "INT_FAST64_MIN", "INT_FAST64_MAX",

    "UINT_FAST8_MIN", "UINT_FAST8_MAX",
    "UINT_FAST16_MIN", "UINT_FAST16_MAX",
    "UINT_FAST32_MIN", "UINT_FAST32_MAX",
    "UINT_FAST64_MIN", "UINT_FAST64_MAX",

    // Integer limits for smallest minimum-width types
    "INT_LEAST8_MIN", "INT_LEAST8_MAX",
    "INT_LEAST16_MIN", "INT_LEAST16_MAX",
    "INT_LEAST32_MIN", "INT_LEAST32_MAX",
    "INT_LEAST64_MIN", "INT_LEAST64_MAX",

    "UINT_LEAST8_MIN", "UINT_LEAST8_MAX",
    "UINT_LEAST16_MIN", "UINT_LEAST16_MAX",
    "UINT_LEAST32_MIN", "UINT_LEAST32_MAX",
    "UINT_LEAST64_MIN", "UINT_LEAST64_MAX",

    // Integer limits for maximum-width types
    "INTMAX_MIN", "INTMAX_MAX",
    "UINTMAX_MIN", "UINTMAX_MAX",

    // Integer limits for pointer-width types
    "INTPTR_MIN", "INTPTR_MAX",
    "UINTPTR_MIN", "UINTPTR_MAX",

    // Printf format specifiers - decimal
    "PRId8", "PRId16", "PRId32", "PRId64",
    "PRIdFAST8", "PRIdFAST16", "PRIdFAST32", "PRIdFAST64",
    "PRIdLEAST8", "PRIdLEAST16", "PRIdLEAST32", "PRIdLEAST64",
    "PRIdMAX", "PRIdPTR",

    // Printf format specifiers - integer
    "PRIi8", "PRIi16", "PRIi32", "PRIi64",
    "PRIiFAST8", "PRIiFAST16", "PRIiFAST32", "PRIiFAST64",
    "PRIiLEAST8", "PRIiLEAST16", "PRIiLEAST32", "PRIiLEAST64",
    "PRIiMAX", "PRIiPTR",

    // Printf format specifiers - unsigned
    "PRIu8", "PRIu16", "PRIu32", "PRIu64",
    "PRIuFAST8", "PRIuFAST16", "PRIuFAST32", "PRIuFAST64",
    "PRIuLEAST8", "PRIuLEAST16", "PRIuLEAST32", "PRIuLEAST64",
    "PRIuMAX", "PRIuPTR",

    // Printf format specifiers - octal
    "PRIo8", "PRIo16", "PRIo32", "PRIo64",
    "PRIoFAST8", "PRIoFAST16", "PRIoFAST32", "PRIoFAST64",
    "PRIoLEAST8", "PRIoLEAST16", "PRIoLEAST32", "PRIoLEAST64",
    "PRIoMAX", "PRIoPTR",

    // Printf format specifiers - hexadecimal (lowercase)
    "PRIx8", "PRIx16", "PRIx32", "PRIx64",
    "PRIxFAST8", "PRIxFAST16", "PRIxFAST32", "PRIxFAST64",
    "PRIxLEAST8", "PRIxLEAST16", "PRIxLEAST32", "PRIxLEAST64",
    "PRIxMAX", "PRIxPTR",

    // Printf format specifiers - hexadecimal (uppercase)
    "PRIX8", "PRIX16", "PRIX32", "PRIX64",
    "PRIXFAST8", "PRIXFAST16", "PRIXFAST32", "PRIXFAST64",
    "PRIXLEAST8", "PRIXLEAST16", "PRIXLEAST32", "PRIXLEAST64",
    "PRIXMAX", "PRIXPTR",

    // Scanf format specifiers - decimal
    "SCNd8", "SCNd16", "SCNd32", "SCNd64",
    "SCNdFAST8", "SCNdFAST16", "SCNdFAST32", "SCNdFAST64",
    "SCNdLEAST8", "SCNdLEAST16", "SCNdLEAST32", "SCNdLEAST64",
    "SCNdMAX", "SCNdPTR",

    // Scanf format specifiers - integer
    "SCNi8", "SCNi16", "SCNi32", "SCNi64",
    "SCNiFAST8", "SCNiFAST16", "SCNiFAST32", "SCNiFAST64",
    "SCNiLEAST8", "SCNiLEAST16", "SCNiLEAST32", "SCNiLEAST64",
    "SCNiMAX", "SCNiPTR",

    // Scanf format specifiers - unsigned
    "SCNu8", "SCNu16", "SCNu32", "SCNu64",
    "SCNuFAST8", "SCNuFAST16", "SCNuFAST32", "SCNuFAST64",
    "SCNuLEAST8", "SCNuLEAST16", "SCNuLEAST32", "SCNuLEAST64",
    "SCNuMAX", "SCNuPTR",

    // Scanf format specifiers - octal
    "SCNo8", "SCNo16", "SCNo32", "SCNo64",
    "SCNoFAST8", "SCNoFAST16", "SCNoFAST32", "SCNoFAST64",
    "SCNoLEAST8", "SCNoLEAST16", "SCNoLEAST32", "SCNoLEAST64",
    "SCNoMAX", "SCNoPTR",

    // Scanf format specifiers - hexadecimal
    "SCNx8", "SCNx16", "SCNx32", "SCNx64",
    "SCNxFAST8", "SCNxFAST16", "SCNxFAST32", "SCNxFAST64",
    "SCNxLEAST8", "SCNxLEAST16", "SCNxLEAST32", "SCNxLEAST64",
    "SCNxMAX", "SCNxPTR",
];

/// Checks if a given macro name is a predefined C/C++ standard macro.
///
/// This function performs a linear search through the list of predefined macros.
/// For better performance with frequent lookups, consider using a HashSet or
/// perfect hash function in production code.
///
/// # Arguments
///
/// * `mac` - The macro name to check
///
/// # Returns
///
/// `true` if the macro is a standard predefined macro, `false` otherwise.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::c_predefined_macros::is_predefined_macro;
///
/// assert!(is_predefined_macro("INT32_MAX"));
/// assert!(is_predefined_macro("PRIu64"));
/// assert!(is_predefined_macro("SCNd32"));
/// assert!(!is_predefined_macro("MY_CUSTOM_MACRO"));
/// ```
#[inline]
pub fn is_predefined_macro(mac: &str) -> bool {
    PREDEFINED_MACROS.contains(&mac)
}

/// Returns the complete list of predefined C/C++ macros.
///
/// This can be useful for generating documentation, building autocomplete
/// systems, or performing batch checks.
///
/// # Returns
///
/// A slice containing all predefined macro names.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::c_predefined_macros::get_all_predefined_macros;
///
/// let macros = get_all_predefined_macros();
/// assert!(macros.len() > 200); // Should have many predefined macros
/// assert!(macros.contains(&"INT32_MAX"));
/// ```
pub fn get_all_predefined_macros() -> &'static [&'static str] {
    PREDEFINED_MACROS
}

/// Returns the number of predefined C/C++ macros.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::c_predefined_macros::predefined_macro_count;
///
/// let count = predefined_macro_count();
/// assert!(count > 200);
/// ```
pub fn predefined_macro_count() -> usize {
    PREDEFINED_MACROS.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_predefined_macro() {
        // Test exact-width integer limits
        assert!(is_predefined_macro("INT8_MAX"));
        assert!(is_predefined_macro("INT16_MIN"));
        assert!(is_predefined_macro("INT32_C"));
        assert!(is_predefined_macro("UINT64_MAX"));

        // Test fast types
        assert!(is_predefined_macro("INT_FAST16_MAX"));
        assert!(is_predefined_macro("UINT_FAST32_MIN"));

        // Test least types
        assert!(is_predefined_macro("INT_LEAST8_MIN"));
        assert!(is_predefined_macro("UINT_LEAST64_MAX"));

        // Test printf format specifiers
        assert!(is_predefined_macro("PRId32"));
        assert!(is_predefined_macro("PRIu64"));
        assert!(is_predefined_macro("PRIxMAX"));
        assert!(is_predefined_macro("PRIX16"));

        // Test scanf format specifiers
        assert!(is_predefined_macro("SCNd32"));
        assert!(is_predefined_macro("SCNu64"));
        assert!(is_predefined_macro("SCNxMAX"));

        // Test max/ptr types
        assert!(is_predefined_macro("INTMAX_MAX"));
        assert!(is_predefined_macro("UINTPTR_MAX"));
        assert!(is_predefined_macro("PRIdPTR"));
        assert!(is_predefined_macro("SCNuPTR"));
    }

    #[test]
    fn test_not_predefined_macro() {
        assert!(!is_predefined_macro("MY_CUSTOM_MACRO"));
        assert!(!is_predefined_macro("MAX_SIZE"));
        assert!(!is_predefined_macro("BUFFER_SIZE"));
        assert!(!is_predefined_macro(""));
        assert!(!is_predefined_macro("random_text"));
    }

    #[test]
    fn test_case_sensitive() {
        // Predefined macros are case-sensitive
        assert!(is_predefined_macro("INT32_MAX"));
        assert!(!is_predefined_macro("int32_max"));
        assert!(!is_predefined_macro("Int32_Max"));
    }

    #[test]
    fn test_get_all_predefined_macros() {
        let macros = get_all_predefined_macros();

        // Should have a substantial number of macros
        assert!(macros.len() > 200);

        // Check for presence of various categories
        assert!(macros.contains(&"INT32_MAX"));
        assert!(macros.contains(&"PRId64"));
        assert!(macros.contains(&"SCNu32"));
        assert!(macros.contains(&"UINT_FAST16_MAX"));
        assert!(macros.contains(&"INT_LEAST32_MIN"));
    }

    #[test]
    fn test_predefined_macro_count() {
        let count = predefined_macro_count();
        assert!(count > 200);
        assert_eq!(count, get_all_predefined_macros().len());
    }

    #[test]
    fn test_no_duplicates() {
        let macros = get_all_predefined_macros();
        let mut seen = std::collections::HashSet::new();

        for &mac in macros {
            assert!(seen.insert(mac), "Duplicate macro found: {}", mac);
        }
    }

    #[test]
    fn test_coverage_of_types() {
        // Ensure we have coverage of all standard types
        let int_widths = ["8", "16", "32", "64"];
        let suffixes = ["MIN", "MAX", "C"];

        for width in &int_widths {
            for suffix in &suffixes {
                let signed_macro = format!("INT{width}_{suffix}");
                let unsigned_macro = format!("UINT{width}_{suffix}");

                assert!(
                    is_predefined_macro(&signed_macro),
                    "Missing macro: {signed_macro}"
                );
                assert!(
                    is_predefined_macro(&unsigned_macro),
                    "Missing macro: {unsigned_macro}"
                );
            }
        }
    }

    #[test]
    fn test_printf_scanf_coverage() {
        let format_types = ["d", "i", "u", "o", "x"];
        let widths = ["8", "16", "32", "64"];

        for fmt in &format_types {
            for width in &widths {
                let pri_macro = format!("PRI{fmt}{width}");
                let scn_macro = format!("SCN{fmt}{width}");

                assert!(
                    is_predefined_macro(&pri_macro),
                    "Missing PRI macro: {pri_macro}"
                );
                assert!(
                    is_predefined_macro(&scn_macro),
                    "Missing SCN macro: {scn_macro}"
                );
            }
        }
    }
}
