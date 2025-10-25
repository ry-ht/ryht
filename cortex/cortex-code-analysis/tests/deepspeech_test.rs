mod common;

use common::compare_parser_output_with_files;

/// Test parsing the DeepSpeech repository
/// This validates that our C/C++ parser can handle a large, complex real-world codebase
#[test]
#[ignore] // This test requires the DeepSpeech repository to be cloned
fn test_deepspeech() {
    // FIXME: Ignoring these files temporarily due to parsing errors (see issue: https://github.com/mozilla/rust-code-analysis/issues/1142),
    // in order to allow CI to pass until the issue is resolved.
    let exclude = &[
        "**/DeepSpeech/native_client/deepspeech.cc",
        "**/DeepSpeech/native_client/getopt_win.h",
        "**/DeepSpeech/native_client/kenlm/util/mmap.cc",
        "**/DeepSpeech/native_client/deepspeech.h",
        "**/DeepSpeech/native_client/kenlm/util/double-conversion/fast-dtoa.cc",
        "**/DeepSpeech/native_client/kenlm/lm/left_test.cc",
        "**/DeepSpeech/native_client/ctcdecode/third_party/openfst-1.6.7/src/test/fst_test.h",
        "**/DeepSpeech/native_client/ctcdecode/third_party/openfst-1.6.9-win/src/include/fst/test/fst_test.h",
    ];

    compare_parser_output_with_files("DeepSpeech", &["*.cc", "*.cpp", "*.h", "*.hh"], exclude);
}
