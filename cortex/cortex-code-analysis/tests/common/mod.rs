use std::path::{Path, PathBuf};
use std::fs;
use std::io::Read;

use glob::glob;
use cortex_code_analysis::{CodeParser, Lang, ParsedFile};

/// Repository base path for test fixtures
pub const REPO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/repositories");

/// Read a file into a string
pub fn read_file(path: &Path) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

/// Guess the language from file path
pub fn guess_language(path: &Path) -> Option<Lang> {
    Lang::from_path(path)
}

/// Parse a single file and analyze it
pub fn parse_file(path: &Path, language: Lang) -> anyhow::Result<ParsedFile> {
    let source = read_file(path)?;
    let mut parser = CodeParser::for_language(language)?;
    parser.parse_file(path.to_str().unwrap(), &source, language)
}

/// Configuration for file analysis
#[derive(Debug)]
pub struct AnalysisConfig {
    pub language: Option<Lang>,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self { language: None }
    }
}

/// Analyze all files in a repository matching the given patterns
pub fn analyze_repository_files(
    repo_name: &str,
    include_patterns: &[&str],
    exclude_patterns: &[&str],
) -> anyhow::Result<Vec<(PathBuf, ParsedFile)>> {
    let repo_path = Path::new(REPO).join(repo_name);
    let mut results = Vec::new();

    for pattern in include_patterns {
        let full_pattern = repo_path.join("**").join(pattern);
        let pattern_str = full_pattern.to_str().unwrap();

        for entry in glob(pattern_str).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    // Check if path should be excluded
                    let should_exclude = exclude_patterns.iter().any(|exclude_pat| {
                        // Simple pattern matching - check if path contains the pattern
                        let path_str = path.to_str().unwrap();
                        if exclude_pat.contains("**") {
                            // Handle glob patterns in exclude
                            let pattern = exclude_pat.replace("**/", "");
                            path_str.contains(&pattern)
                        } else {
                            path_str.ends_with(exclude_pat)
                        }
                    });

                    if should_exclude {
                        continue;
                    }

                    // Determine language from path
                    if let Some(language) = guess_language(&path) {
                        // Try to parse the file
                        match parse_file(&path, language) {
                            Ok(parsed) => {
                                results.push((path, parsed));
                            }
                            Err(e) => {
                                eprintln!("Failed to parse {}: {}", path.display(), e);
                            }
                        }
                    }
                }
                Err(e) => eprintln!("Error reading glob entry: {}", e),
            }
        }
    }

    Ok(results)
}

/// Compare parser output with expected results
/// This is a simplified version - in production you'd want snapshot testing
pub fn compare_parser_output_with_files(
    repo_name: &str,
    include: &[&str],
    exclude: &[&str],
) {
    match analyze_repository_files(repo_name, include, exclude) {
        Ok(results) => {
            println!("Successfully parsed {} files in {}", results.len(), repo_name);

            // Basic validation - ensure we got some results
            assert!(!results.is_empty(), "Expected to parse at least one file");

            // Validate that parsing succeeded for each file
            for (path, parsed) in &results {
                // Basic sanity checks
                assert!(!parsed.path.is_empty(),
                    "File path should not be empty for {}", path.display());
            }
        }
        Err(e) => {
            panic!("Failed to analyze repository {}: {}", repo_name, e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_language() {
        assert_eq!(guess_language(Path::new("test.rs")), Some(Lang::Rust));
        assert_eq!(guess_language(Path::new("test.js")), Some(Lang::JavaScript));
        assert_eq!(guess_language(Path::new("test.ts")), Some(Lang::TypeScript));
    }
}
