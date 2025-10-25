mod common;

use common::compare_parser_output_with_files;

/// Test parsing the serde repository
/// This validates that our Rust parser can handle a large, complex real-world codebase
#[test]
#[ignore = "Requires serde repository to be cloned"]
fn test_serde() {
    compare_parser_output_with_files("serde", &["*.rs"], &[]);
}
