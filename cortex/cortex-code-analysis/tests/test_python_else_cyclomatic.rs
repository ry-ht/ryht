//! Tests for Python cyclomatic complexity else clause enhancement
//!
//! This test file specifically tests the enhancement where Python else clauses
//! are only counted in cyclomatic complexity when they follow for/while loops,
//! not when they follow if statements.

use anyhow::Result;
use cortex_code_analysis::{
    Parser, PythonLanguage, Lang,
    spaces::compute_spaces,
    traits::ParserTrait,
};
use std::path::Path;

#[test]
fn test_python_else_after_if_not_counted() -> Result<()> {
    // else after if should NOT count towards cyclomatic complexity
    let source = r#"
def test_if_else(x):
    if x > 0:
        return "positive"
    else:
        return "non-positive"
"#;

    let parser = Parser::<PythonLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.py")
    )?;

    let root_space = compute_spaces(
        parser.get_root(),
        parser.get_code(),
        Lang::Python,
        "test.py"
    )?;

    // Find the function space in nested spaces
    let func_space = root_space.spaces.iter()
        .find(|s| s.name.as_ref().map(|n| n.contains("test_if_else")).unwrap_or(false))
        .expect("Function space not found");

    // Cyclomatic complexity should be 2: base 1 + if statement 1
    // else after if should NOT be counted
    assert_eq!(func_space.metrics.cyclomatic.cyclomatic(), 2.0,
        "else after if should not count towards cyclomatic complexity");

    Ok(())
}

#[test]
fn test_python_else_after_for_counted() -> Result<()> {
    // else after for SHOULD count towards cyclomatic complexity
    let source = r#"
def search_list(items, target):
    for item in items:
        if item == target:
            break
    else:
        return "not found"
    return "found"
"#;

    let parser = Parser::<PythonLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.py")
    )?;

    let root_space = compute_spaces(
        parser.get_root(),
        parser.get_code(),
        Lang::Python,
        "test.py"
    )?;

    // Find the function space
    let func_space = root_space.spaces.iter()
        .find(|s| s.name.as_ref().map(|n| n.contains("search_list")).unwrap_or(false))
        .expect("Function space not found");

    // Cyclomatic complexity should be 4: base 1 + for 1 + if 1 + else (after for) 1
    assert_eq!(func_space.metrics.cyclomatic.cyclomatic(), 4.0,
        "else after for should count towards cyclomatic complexity");

    Ok(())
}

#[test]
fn test_python_else_after_while_counted() -> Result<()> {
    // else after while SHOULD count towards cyclomatic complexity
    let source = r#"
def wait_for_condition(condition):
    attempts = 0
    while attempts < 10:
        if condition():
            break
        attempts += 1
    else:
        return "timeout"
    return "success"
"#;

    let parser = Parser::<PythonLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.py")
    )?;

    let root_space = compute_spaces(
        parser.get_root(),
        parser.get_code(),
        Lang::Python,
        "test.py"
    )?;

    // Find the function space
    let func_space = root_space.spaces.iter()
        .find(|s| s.name.as_ref().map(|n| n.contains("wait_for_condition")).unwrap_or(false))
        .expect("Function space not found");

    // Cyclomatic complexity should be 4: base 1 + while 1 + if 1 + else (after while) 1
    assert_eq!(func_space.metrics.cyclomatic.cyclomatic(), 4.0,
        "else after while should count towards cyclomatic complexity");

    Ok(())
}

#[test]
fn test_python_elif_chain_not_double_counted() -> Result<()> {
    // elif should count but else after if should not
    let source = r#"
def categorize(value):
    if value < 0:
        return "negative"
    elif value == 0:
        return "zero"
    elif value < 10:
        return "small"
    else:
        return "large"
"#;

    let parser = Parser::<PythonLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.py")
    )?;

    let root_space = compute_spaces(
        parser.get_root(),
        parser.get_code(),
        Lang::Python,
        "test.py"
    )?;

    // Find the function space
    let func_space = root_space.spaces.iter()
        .find(|s| s.name.as_ref().map(|n| n.contains("categorize")).unwrap_or(false))
        .expect("Function space not found");

    // Cyclomatic complexity should be 4: base 1 + if 1 + elif 1 + elif 1
    // else after if should NOT be counted
    assert_eq!(func_space.metrics.cyclomatic.cyclomatic(), 4.0,
        "elif chains should not double-count else clauses after if");

    Ok(())
}

#[test]
fn test_python_complex_else_combinations() -> Result<()> {
    // Test complex combinations of else clauses
    let source = r#"
def complex_function(items, threshold):
    # for-else should count
    for item in items:
        if item > threshold:
            # while-else should count
            while item > 0:
                item -= 1
            else:
                break
    else:
        return "all processed"

    # if-else should not count
    if threshold > 0:
        return "positive"
    else:
        return "negative"
"#;

    let parser = Parser::<PythonLanguage>::new(
        source.as_bytes().to_vec(),
        Path::new("test.py")
    )?;

    let root_space = compute_spaces(
        parser.get_root(),
        parser.get_code(),
        Lang::Python,
        "test.py"
    )?;

    // Find the function space
    let func_space = root_space.spaces.iter()
        .find(|s| s.name.as_ref().map(|n| n.contains("complex_function")).unwrap_or(false))
        .expect("Function space not found");

    // Cyclomatic complexity should be:
    // base 1 + for 1 + if 1 + while 1 + else(after while) 1 + else(after for) 1 + if 1
    // = 7 (else after second if should NOT count)
    assert_eq!(func_space.metrics.cyclomatic.cyclomatic(), 7.0,
        "Complex else combinations should be counted correctly");

    Ok(())
}
