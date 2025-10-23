//! Code Generators for Testing
//!
//! Generate realistic code samples in various languages for testing

use rand::Rng;

/// Generate a random Rust function
pub fn generate_rust_function(name: &str, complexity: usize) -> String {
    let params = (0..complexity).map(|i| format!("arg{}: i32", i)).collect::<Vec<_>>().join(", ");
    let body = (0..complexity)
        .map(|i| format!("    let result{} = arg{} * 2;", i, i))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"/// Generated function: {}
pub fn {}({}) -> i32 {{
{}
    result0
}}
"#,
        name, name, params, body
    )
}

/// Generate a random TypeScript class
pub fn generate_typescript_class(name: &str, method_count: usize) -> String {
    let methods = (0..method_count)
        .map(|i| {
            format!(
                r#"    method{}(arg: number): number {{
        return arg * {};
    }}
"#,
                i,
                i + 1
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"export class {} {{
    private value: number = 0;

{}
}}
"#,
        name, methods
    )
}

/// Generate a random React component
pub fn generate_react_component(name: &str, prop_count: usize) -> String {
    let props = (0..prop_count)
        .map(|i| format!("    prop{}: string;", i))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"import React from 'react';

interface {}Props {{
{}
}}

export const {}: React.FC<{}Props> = (props) => {{
    return (
        <div>
            <h1>{{props.prop0}}</h1>
        </div>
    );
}};
"#,
        name, props, name, name
    )
}

/// Generate a large file with many functions
pub fn generate_large_rust_file(function_count: usize) -> String {
    let functions = (0..function_count)
        .map(|i| generate_rust_function(&format!("function_{}", i), 2))
        .collect::<Vec<_>>()
        .join("\n");

    format!("// Large generated file with {} functions\n\n{}", function_count, functions)
}

/// Generate random file content of specific size
pub fn generate_content_of_size(size_bytes: usize) -> String {
    let mut rng = rand::thread_rng();
    let lines = size_bytes / 50; // ~50 bytes per line

    (0..lines)
        .map(|i| {
            let random_content: String = (0..40)
                .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
                .collect();
            format!("// Line {}: {}\n", i, random_content)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_rust_function() {
        let code = generate_rust_function("test_func", 3);
        assert!(code.contains("pub fn test_func"));
        assert!(code.contains("arg0"));
        assert!(code.contains("arg1"));
        assert!(code.contains("arg2"));
    }

    #[test]
    fn test_generate_typescript_class() {
        let code = generate_typescript_class("TestClass", 2);
        assert!(code.contains("export class TestClass"));
        assert!(code.contains("method0"));
        assert!(code.contains("method1"));
    }

    #[test]
    fn test_generate_content_of_size() {
        let content = generate_content_of_size(5000);
        assert!(content.len() >= 4500); // Allow some variance
        assert!(content.len() <= 5500);
    }
}
