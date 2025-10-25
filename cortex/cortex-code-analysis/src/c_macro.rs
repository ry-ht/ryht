//! C/C++ Macro Replacement and Code Preparation
//!
//! This module provides functionality to replace C/C++ macros with placeholder tokens
//! before parsing, which helps tree-sitter parse the code more accurately by preventing
//! macro-expanded code from confusing the parser.
//!
//! The replacement strategy uses dollar signs ($) to replace macro identifiers while
//! preserving the overall structure of the code. This allows the parser to correctly
//! identify the syntactic structure without being affected by macro definitions.
//!
//! # Examples
//!
//! ```
//! use cortex_code_analysis::c_macro::replace_macros;
//! use std::collections::HashSet;
//!
//! let mut macros = HashSet::new();
//! macros.insert("MAX_SIZE".to_string());
//! macros.insert("MIN_SIZE".to_string());
//!
//! let code = b"int size = MAX_SIZE + MIN_SIZE;";
//! let result = replace_macros(code, &macros);
//!
//! assert!(result.is_some());
//! let new_code = result.unwrap();
//! // Macros are replaced with $ characters
//! assert_eq!(&new_code, b"int size = $$$$$$$$ + $$$$$$$$;");
//! ```
//!
//! # Design
//!
//! The module uses several optimization techniques:
//! - Fast ASCII character checking with inline functions
//! - Pre-allocated buffer of dollar signs for quick replacement
//! - Single-pass scanning of the source code
//! - Early return when no macros are found

use std::collections::HashSet;

/// Pre-allocated buffer of dollar signs for macro replacement.
/// This avoids repeated allocations during macro replacement operations.
const DOLLARS: [u8; 2048] = [b'$'; 2048];

/// Checks if a byte is a valid identifier continuation character.
///
/// An identifier can contain uppercase letters, lowercase letters,
/// digits, or underscores.
#[inline(always)]
fn is_identifier_part(c: u8) -> bool {
    c.is_ascii_uppercase() || c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_'
}

/// Checks if a byte is a valid identifier start character.
///
/// An identifier must start with an uppercase letter, lowercase letter,
/// or underscore (not a digit).
#[inline(always)]
fn is_identifier_starter(c: u8) -> bool {
    c.is_ascii_uppercase() || c.is_ascii_lowercase() || c == b'_'
}

/// Checks if a macro name is in the known macro set or is a predefined macro.
///
/// This function checks both user-defined macros and standard C/C++ predefined
/// macros (like INT32_MAX, UINT64_C, etc.).
#[inline(always)]
fn is_macro<S: ::std::hash::BuildHasher>(mac: &str, macros: &HashSet<String, S>) -> bool {
    macros.contains(mac) || is_predefined_macro(mac)
}

/// Replaces all occurrences of macros in the code with dollar signs.
///
/// This function scans through the source code byte by byte, identifies
/// macro identifiers, and replaces them with an equal number of dollar signs.
/// This preserves the byte offsets and positions in the source code, which is
/// important for accurate source mapping.
///
/// # Arguments
///
/// * `code` - The source code as a byte slice
/// * `macros` - A set of macro names to replace
///
/// # Returns
///
/// * `Some(Vec<u8>)` - A new code buffer with macros replaced, if any macros were found
/// * `None` - If no macros were found in the code
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::c_macro::replace_macros;
/// use std::collections::HashSet;
///
/// let mut macros = HashSet::new();
/// macros.insert("MAX".to_string());
///
/// let code = b"int x = MAX;";
/// let result = replace_macros(code, &macros);
/// assert!(result.is_some());
/// ```
pub fn replace_macros<S: ::std::hash::BuildHasher>(
    code: &[u8],
    macros: &HashSet<String, S>,
) -> Option<Vec<u8>> {
    let mut new_code = Vec::with_capacity(code.len());
    let mut code_start = 0;
    let mut k_start = 0;

    for (i, c) in code.iter().enumerate() {
        if k_start != 0 {
            // We're currently scanning an identifier
            if !is_identifier_part(*c) {
                // End of identifier found
                let start = k_start - 1;
                k_start = 0;
                let keyword = String::from_utf8(code[start..i].to_vec()).unwrap();
                if is_macro(&keyword, macros) {
                    // This identifier is a macro, replace it
                    new_code.extend(&code[code_start..start]);
                    new_code.extend(&DOLLARS[..(i - start)]);
                    code_start = i;
                }
            }
        } else if is_identifier_starter(*c) {
            // Start of a potential identifier
            k_start = i + 1;
        }
    }

    // Handle identifier at end of code
    if k_start != 0 {
        let start = k_start - 1;
        let i = code.len();
        let keyword = String::from_utf8(code[start..].to_vec()).unwrap();
        if is_macro(&keyword, macros) {
            new_code.extend(&code[code_start..start]);
            new_code.extend(&DOLLARS[..(i - start)]);
            code_start = i;
        }
    }

    // If no macros were replaced, return None
    if code_start == 0 {
        None
    } else {
        // Append any remaining code after the last macro
        if code_start < code.len() {
            new_code.extend(&code[code_start..]);
        }
        Some(new_code)
    }
}

/// Prepares C/C++ code for parsing by replacing macros.
///
/// This is a convenience function that wraps `replace_macros` and returns
/// the original code if no macros need to be replaced.
///
/// # Arguments
///
/// * `code` - The source code as a byte slice
/// * `macros` - A set of macro names to replace
///
/// # Returns
///
/// A `Vec<u8>` containing either the modified code (with macros replaced)
/// or the original code if no replacements were needed.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::c_macro::prepare_file;
/// use std::collections::HashSet;
///
/// let macros = HashSet::new();
/// let code = b"int main() { return 0; }";
/// let result = prepare_file(code, &macros);
/// assert_eq!(result, code.to_vec());
/// ```
pub fn prepare_file<S: ::std::hash::BuildHasher>(
    code: &[u8],
    macros: &HashSet<String, S>,
) -> Vec<u8> {
    replace_macros(code, macros).unwrap_or_else(|| code.to_vec())
}

/// Checks if a macro name is a predefined C/C++ macro.
///
/// This function checks against a comprehensive list of standard C/C++
/// predefined macros from headers like `<stdint.h>` and `<inttypes.h>`.
///
/// # Examples
///
/// ```
/// use cortex_code_analysis::c_macro::is_predefined_macro;
///
/// assert!(is_predefined_macro("INT32_MAX"));
/// assert!(is_predefined_macro("UINT64_C"));
/// assert!(!is_predefined_macro("MY_CUSTOM_MACRO"));
/// ```
pub fn is_predefined_macro(mac: &str) -> bool {
    crate::c_predefined_macros::is_predefined_macro(mac)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_macros_single() {
        let mut mac = HashSet::new();
        mac.insert("abc".to_string());

        // No macros in code
        assert!(replace_macros(b"def ghi jkl", &mac).is_none());

        // Macro at start
        assert_eq!(
            b"$$$ def ghi jkl".to_vec(),
            replace_macros(b"abc def ghi jkl", &mac).unwrap()
        );

        // Macro in middle
        assert_eq!(
            b"def $$$ ghi jkl".to_vec(),
            replace_macros(b"def abc ghi jkl", &mac).unwrap()
        );

        // Macro near end
        assert_eq!(
            b"def ghi $$$ jkl".to_vec(),
            replace_macros(b"def ghi abc jkl", &mac).unwrap()
        );

        // Macro at end
        assert_eq!(
            b"def ghi jkl $$$".to_vec(),
            replace_macros(b"def ghi jkl abc", &mac).unwrap()
        );
    }

    #[test]
    fn test_replace_macros_multiple() {
        let mut mac = HashSet::new();
        mac.insert("abc".to_string());
        mac.insert("z9_".to_string());

        assert_eq!(
            b"$$$ def ghi $$$ jkl".to_vec(),
            replace_macros(b"abc def ghi z9_ jkl", &mac).unwrap()
        );
    }

    #[test]
    fn test_replace_macros_realistic() {
        let mut mac = HashSet::new();
        mac.insert("MAX_SIZE".to_string());
        mac.insert("MIN_SIZE".to_string());

        let code = b"int size = MAX_SIZE + MIN_SIZE;";
        let result = replace_macros(code, &mac);
        assert!(result.is_some());

        let new_code = result.unwrap();
        // Each macro name is replaced with equal number of $ chars
        assert!(new_code.contains(&b'$'));
        assert_eq!(code.len(), new_code.len());
    }

    #[test]
    fn test_is_identifier_part() {
        assert!(is_identifier_part(b'a'));
        assert!(is_identifier_part(b'Z'));
        assert!(is_identifier_part(b'0'));
        assert!(is_identifier_part(b'_'));
        assert!(!is_identifier_part(b' '));
        assert!(!is_identifier_part(b'+'));
        assert!(!is_identifier_part(b'('));
    }

    #[test]
    fn test_is_identifier_starter() {
        assert!(is_identifier_starter(b'a'));
        assert!(is_identifier_starter(b'Z'));
        assert!(is_identifier_starter(b'_'));
        assert!(!is_identifier_starter(b'0')); // Digit can't start identifier
        assert!(!is_identifier_starter(b' '));
        assert!(!is_identifier_starter(b'+'));
    }

    #[test]
    fn test_prepare_file_no_macros() {
        let macros = HashSet::new();
        let code = b"int main() { return 0; }";
        let result = prepare_file(code, &macros);
        assert_eq!(result, code.to_vec());
    }

    #[test]
    fn test_prepare_file_with_macros() {
        let mut macros = HashSet::new();
        macros.insert("MAX".to_string());

        let code = b"int x = MAX;";
        let result = prepare_file(code, &macros);
        assert_ne!(result, code.to_vec());
        assert_eq!(result.len(), code.len());
    }

    #[test]
    fn test_macro_boundary_detection() {
        let mut mac = HashSet::new();
        mac.insert("MAX".to_string());

        // Should not match MAXIMAL (not a word boundary)
        let code = b"int MAXIMAL = 100;";
        assert!(replace_macros(code, &mac).is_none());

        // Should match MAX with parentheses
        let code = b"int x = MAX(10);";
        let result = replace_macros(code, &mac);
        assert!(result.is_some());
        assert_eq!(&result.unwrap(), b"int x = $$$(10);");
    }

    #[test]
    fn test_predefined_macros() {
        assert!(is_predefined_macro("INT32_MAX"));
        assert!(is_predefined_macro("UINT64_C"));
        assert!(is_predefined_macro("PRId64"));
        assert!(is_predefined_macro("SCNu32"));
        assert!(!is_predefined_macro("MY_MACRO"));
        assert!(!is_predefined_macro("custom"));
    }

    #[test]
    fn test_is_macro_with_predefined() {
        let mut macros = HashSet::new();
        macros.insert("CUSTOM".to_string());

        // Should find custom macro
        assert!(is_macro("CUSTOM", &macros));

        // Should find predefined macro even if not in set
        assert!(is_macro("INT32_MAX", &macros));

        // Should not find unknown macro
        assert!(!is_macro("UNKNOWN", &macros));
    }
}
