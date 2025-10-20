//! Example Generator - Generates code examples from symbols
//!
//! This module provides functionality to generate basic and advanced code examples
//! from CodeSymbol definitions, supporting multiple languages.

use crate::types::CodeSymbol;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Generated code example with metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Example {
    pub code: String,
    pub description: String,
    pub language: String,
    pub complexity: ExampleComplexity,
}

/// Complexity level of generated examples
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExampleComplexity {
    Basic,
    Intermediate,
    Advanced,
}

impl ExampleComplexity {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExampleComplexity::Basic => "basic",
            ExampleComplexity::Intermediate => "intermediate",
            ExampleComplexity::Advanced => "advanced",
        }
    }
}

/// Validation result for examples
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn success() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn with_warning(mut self, warning: String) -> Self {
        self.warnings.push(warning);
        self
    }

    pub fn with_error(error: String) -> Self {
        Self {
            valid: false,
            errors: vec![error],
            warnings: Vec::new(),
        }
    }
}

/// Example generator for code symbols
pub struct ExampleGenerator {
    language: String,
}

impl ExampleGenerator {
    /// Create a new example generator for the specified language
    pub fn new(language: String) -> Self {
        Self { language }
    }

    /// Generate a basic usage example for a symbol
    pub fn generate_basic(&self, symbol: &CodeSymbol) -> Result<Example> {
        let code = match self.language.as_str() {
            "typescript" | "javascript" => self.generate_basic_typescript(symbol)?,
            "rust" => self.generate_basic_rust(symbol)?,
            "python" => self.generate_basic_python(symbol)?,
            _ => anyhow::bail!("Unsupported language: {}", self.language),
        };

        Ok(Example {
            code,
            description: format!("Basic usage example for {}", symbol.name),
            language: self.language.clone(),
            complexity: ExampleComplexity::Basic,
        })
    }

    /// Generate example with specific complexity level
    pub fn generate_with_complexity(
        &self,
        symbol: &CodeSymbol,
        complexity: ExampleComplexity,
    ) -> Result<Vec<Example>> {
        match complexity {
            ExampleComplexity::Basic => Ok(vec![self.generate_basic(symbol)?]),
            ExampleComplexity::Intermediate => self.generate_intermediate(symbol),
            ExampleComplexity::Advanced => self.generate_advanced(symbol),
        }
    }

    /// Generate intermediate examples for a symbol
    pub fn generate_intermediate(&self, symbol: &CodeSymbol) -> Result<Vec<Example>> {
        let mut examples = Vec::new();

        match self.language.as_str() {
            "typescript" | "javascript" => {
                examples.extend(self.generate_intermediate_typescript(symbol)?);
            }
            "rust" => {
                examples.extend(self.generate_intermediate_rust(symbol)?);
            }
            "python" => {
                examples.extend(self.generate_intermediate_python(symbol)?);
            }
            _ => anyhow::bail!("Unsupported language: {}", self.language),
        }

        Ok(examples)
    }

    /// Generate multiple advanced examples for a symbol
    pub fn generate_advanced(&self, symbol: &CodeSymbol) -> Result<Vec<Example>> {
        let mut examples = Vec::new();

        match self.language.as_str() {
            "typescript" | "javascript" => {
                examples.extend(self.generate_advanced_typescript(symbol)?);
            }
            "rust" => {
                examples.extend(self.generate_advanced_rust(symbol)?);
            }
            "python" => {
                examples.extend(self.generate_advanced_python(symbol)?);
            }
            _ => anyhow::bail!("Unsupported language: {}", self.language),
        }

        Ok(examples)
    }

    /// Extract parameters from symbol signature
    fn extract_parameters(&self, signature: &str) -> Vec<(String, String)> {
        let mut params = Vec::new();

        // Simple parameter extraction - in reality this would use proper parsing
        if let Some(start) = signature.find('(') {
            if let Some(end) = signature.rfind(')') {
                let params_str = &signature[start + 1..end];
                for param in params_str.split(',') {
                    let param = param.trim();
                    if param.is_empty() {
                        continue;
                    }

                    // TypeScript/JavaScript: name: type
                    if let Some(colon) = param.find(':') {
                        let name = param[..colon].trim().to_string();
                        let type_str = param[colon + 1..].trim().to_string();
                        params.push((name, type_str));
                    } else {
                        // Simple name
                        params.push((param.to_string(), "any".to_string()));
                    }
                }
            }
        }

        params
    }

    /// Generate realistic parameter values based on type
    fn generate_param_value(&self, param_type: &str) -> String {
        match self.language.as_str() {
            "typescript" | "javascript" => match param_type {
                "string" | "String" => "\"example\"".to_string(),
                "number" | "Number" => "42".to_string(),
                "boolean" | "Boolean" => "true".to_string(),
                "any" => "{}".to_string(),
                t if t.ends_with("[]") => "[]".to_string(),
                _ => "{}".to_string(),
            },
            "rust" => match param_type {
                "String" | "&str" => "\"example\".to_string()".to_string(),
                "i32" | "i64" | "u32" | "u64" | "usize" => "42".to_string(),
                "f32" | "f64" => "3.14".to_string(),
                "bool" => "true".to_string(),
                _ => "Default::default()".to_string(),
            },
            "python" => match param_type {
                "str" => "\"example\"".to_string(),
                "int" => "42".to_string(),
                "float" => "3.14".to_string(),
                "bool" => "True".to_string(),
                "list" => "[]".to_string(),
                "dict" => "{}".to_string(),
                _ => "None".to_string(),
            },
            _ => "".to_string(),
        }
    }

    /// Validate an example (syntax and structure check)
    pub fn validate(&self, example: &Example) -> Result<ValidationResult> {
        // Check language matches
        if example.language != self.language {
            return Ok(ValidationResult::with_error(format!(
                "Language mismatch: expected {}, got {}",
                self.language, example.language
            )));
        }

        // Basic validation checks
        if example.code.is_empty() {
            return Ok(ValidationResult::with_error(
                "Example code is empty".to_string(),
            ));
        }

        // Language-specific validation
        match self.language.as_str() {
            "typescript" | "javascript" => self.validate_typescript(&example.code),
            "rust" => self.validate_rust(&example.code),
            "python" => self.validate_python(&example.code),
            _ => Ok(ValidationResult::success().with_warning(format!(
                "No validator available for language: {}",
                self.language
            ))),
        }
    }

    // Private helper methods for TypeScript/JavaScript
    fn generate_basic_typescript(&self, symbol: &CodeSymbol) -> Result<String> {
        use crate::types::SymbolKind;

        let params = self.extract_parameters(&symbol.signature);
        let param_values: Vec<String> = params.iter()
            .map(|(_, ptype)| self.generate_param_value(ptype))
            .collect();
        let args = param_values.join(", ");

        let code = match symbol.kind {
            SymbolKind::Function | SymbolKind::Method => {
                if params.is_empty() {
                    format!(
                        "// Basic usage of {}\nconst result = {}();\nconsole.log(result);",
                        symbol.name, symbol.name
                    )
                } else {
                    format!(
                        "// Basic usage of {}\nconst result = {}({});\nconsole.log(result);",
                        symbol.name, symbol.name, args
                    )
                }
            }
            SymbolKind::Class => {
                if params.is_empty() {
                    format!(
                        "// Basic usage of {}\nconst instance = new {}();\nconsole.log(instance);",
                        symbol.name, symbol.name
                    )
                } else {
                    format!(
                        "// Basic usage of {}\nconst instance = new {}({});\nconsole.log(instance);",
                        symbol.name, symbol.name, args
                    )
                }
            }
            SymbolKind::Interface => {
                let mut impl_lines = Vec::new();
                for (name, ptype) in &params {
                    impl_lines.push(format!("  {}: {}", name, self.generate_param_value(ptype)));
                }
                format!(
                    "// Implementing {}\nconst obj: {} = {{\n{}\n}};",
                    symbol.name, symbol.name, impl_lines.join(",\n")
                )
            }
            SymbolKind::Constant | SymbolKind::Variable => {
                format!("// Using {}\nconsole.log({});", symbol.name, symbol.name)
            }
            _ => {
                format!("// Example for {}\n// Usage example", symbol.name)
            }
        };

        Ok(code)
    }

    fn generate_intermediate_typescript(&self, symbol: &CodeSymbol) -> Result<Vec<Example>> {
        use crate::types::SymbolKind;

        let mut examples = Vec::new();
        let params = self.extract_parameters(&symbol.signature);

        match symbol.kind {
            SymbolKind::Function | SymbolKind::Method => {
                // Async/await pattern
                examples.push(Example {
                    code: if params.is_empty() {
                        format!(
                            "// Async usage\nasync function example() {{\n  const result = await {}();\n  return result;\n}}\n\nexample().then(console.log);",
                            symbol.name
                        )
                    } else {
                        let param_values: Vec<String> = params.iter()
                            .map(|(_, ptype)| self.generate_param_value(ptype))
                            .collect();
                        format!(
                            "// Async usage with parameters\nasync function example() {{\n  const result = await {}({});\n  return result;\n}}\n\nexample().then(console.log);",
                            symbol.name, param_values.join(", ")
                        )
                    },
                    description: format!("Async pattern for {}", symbol.name),
                    language: self.language.clone(),
                    complexity: ExampleComplexity::Intermediate,
                });

                // With type checking
                if !params.is_empty() {
                    let param_defs: Vec<String> = params.iter()
                        .map(|(name, ptype)| format!("{}: {}", name, ptype))
                        .collect();
                    examples.push(Example {
                        code: format!(
                            "// Type-safe usage\nfunction use{}({}) {{\n  return {}({});\n}}",
                            symbol.name,
                            param_defs.join(", "),
                            symbol.name,
                            params.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join(", ")
                        ),
                        description: format!("Type-safe wrapper for {}", symbol.name),
                        language: self.language.clone(),
                        complexity: ExampleComplexity::Intermediate,
                    });
                }
            }
            SymbolKind::Class => {
                // With dependency injection
                examples.push(Example {
                    code: format!(
                        "// Using with dependency injection\nclass Service {{\n  private {}: {};\n\n  constructor({}: {}) {{\n    this.{} = {};\n  }}\n\n  use() {{\n    // Use the injected instance\n    return this.{};\n  }}\n}}",
                        symbol.name.to_lowercase(),
                        symbol.name,
                        symbol.name.to_lowercase(),
                        symbol.name,
                        symbol.name.to_lowercase(),
                        symbol.name.to_lowercase(),
                        symbol.name.to_lowercase()
                    ),
                    description: format!("Dependency injection pattern for {}", symbol.name),
                    language: self.language.clone(),
                    complexity: ExampleComplexity::Intermediate,
                });
            }
            _ => {}
        }

        Ok(examples)
    }

    fn generate_advanced_typescript(&self, symbol: &CodeSymbol) -> Result<Vec<Example>> {
        use crate::types::SymbolKind;

        let mut examples = Vec::new();

        match symbol.kind {
            SymbolKind::Function | SymbolKind::Method => {
                // Error handling example
                examples.push(Example {
                    code: format!(
                        "// Error handling\ntry {{\n  const result = {}();\n  console.log(result);\n}} catch (error) {{\n  console.error('Error:', error);\n}}",
                        symbol.name
                    ),
                    description: format!("Error handling for {}", symbol.name),
                    language: self.language.clone(),
                    complexity: ExampleComplexity::Advanced,
                });

                // Async example
                examples.push(Example {
                    code: format!(
                        "// Async usage\nasync function example() {{\n  const result = await {}();\n  return result;\n}}",
                        symbol.name
                    ),
                    description: format!("Async pattern for {}", symbol.name),
                    language: self.language.clone(),
                    complexity: ExampleComplexity::Intermediate,
                });
            }
            SymbolKind::Class => {
                // Inheritance example
                examples.push(Example {
                    code: format!(
                        "// Extending {}\nclass Extended{} extends {} {{\n  constructor() {{\n    super();\n  }}\n}}",
                        symbol.name, symbol.name, symbol.name
                    ),
                    description: format!("Inheritance pattern for {}", symbol.name),
                    language: self.language.clone(),
                    complexity: ExampleComplexity::Advanced,
                });
            }
            _ => {}
        }

        Ok(examples)
    }

    fn validate_typescript(&self, code: &str) -> Result<ValidationResult> {
        let mut result = ValidationResult::success();

        // Basic syntax checks
        let open_braces = code.matches('{').count();
        let close_braces = code.matches('}').count();

        if open_braces != close_braces {
            result.valid = false;
            result.errors.push("Mismatched braces".to_string());
        }

        let open_parens = code.matches('(').count();
        let close_parens = code.matches(')').count();

        if open_parens != close_parens {
            result.valid = false;
            result.errors.push("Mismatched parentheses".to_string());
        }

        // Check for common patterns
        if !code.contains("//") && !code.contains("/*") {
            result = result.with_warning("Example has no comments".to_string());
        }

        Ok(result)
    }

    // Private helper methods for Rust
    fn generate_basic_rust(&self, symbol: &CodeSymbol) -> Result<String> {
        use crate::types::SymbolKind;

        let code = match symbol.kind {
            SymbolKind::Function => {
                format!(
                    "// Basic usage of {}\nlet result = {}();\nprintln!(\"{{:?}}\", result);",
                    symbol.name, symbol.name
                )
            }
            SymbolKind::Struct => {
                format!(
                    "// Basic usage of {}\nlet instance = {}::new();\nprintln!(\"{{:?}}\", instance);",
                    symbol.name, symbol.name
                )
            }
            SymbolKind::Trait => {
                format!(
                    "// Implementing {}\nimpl {} for MyType {{\n  // implementation\n}}",
                    symbol.name, symbol.name
                )
            }
            SymbolKind::Enum => {
                format!(
                    "// Using {}\nlet value = {}::Variant;\nmatch value {{\n  // patterns\n  _ => {{}}\n}}",
                    symbol.name, symbol.name
                )
            }
            _ => {
                format!("// Example for {}\n// Usage example", symbol.name)
            }
        };

        Ok(code)
    }

    fn generate_intermediate_rust(&self, symbol: &CodeSymbol) -> Result<Vec<Example>> {
        use crate::types::SymbolKind;

        let mut examples = Vec::new();
        let params = self.extract_parameters(&symbol.signature);

        match symbol.kind {
            SymbolKind::Function => {
                // With error propagation
                if !params.is_empty() {
                    let param_values: Vec<String> = params.iter()
                        .map(|(_, ptype)| self.generate_param_value(ptype))
                        .collect();
                    examples.push(Example {
                        code: format!(
                            "// Error propagation\nfn use_{}() -> Result<(), Box<dyn std::error::Error>> {{\n  let result = {}({});\n  println!(\"{{:?}}\", result);\n  Ok(())\n}}",
                            symbol.name,
                            symbol.name,
                            param_values.join(", ")
                        ),
                        description: format!("Error propagation for {}", symbol.name),
                        language: self.language.clone(),
                        complexity: ExampleComplexity::Intermediate,
                    });
                }

                // With generics
                examples.push(Example {
                    code: format!(
                        "// Generic wrapper\nfn process_with<F>(f: F) -> impl FnOnce()\nwhere\n  F: Fn(),\n{{\n  move || {{\n    f();\n    {}();\n  }}\n}}",
                        symbol.name
                    ),
                    description: format!("Generic wrapper for {}", symbol.name),
                    language: self.language.clone(),
                    complexity: ExampleComplexity::Intermediate,
                });
            }
            SymbolKind::Struct => {
                // Builder pattern
                examples.push(Example {
                    code: format!(
                        "// Builder pattern\nlet instance = {}::builder()\n  .field(value)\n  .build()?;",
                        symbol.name
                    ),
                    description: format!("Builder pattern for {}", symbol.name),
                    language: self.language.clone(),
                    complexity: ExampleComplexity::Intermediate,
                });

                // With trait bounds
                examples.push(Example {
                    code: format!(
                        "// Generic usage with trait bounds\nfn use_generic<T>(item: T)\nwhere\n  T: Into<{}>,\n{{\n  let instance: {} = item.into();\n  // Use instance\n}}",
                        symbol.name,
                        symbol.name
                    ),
                    description: format!("Generic usage with trait bounds for {}", symbol.name),
                    language: self.language.clone(),
                    complexity: ExampleComplexity::Intermediate,
                });
            }
            _ => {}
        }

        Ok(examples)
    }

    fn generate_advanced_rust(&self, symbol: &CodeSymbol) -> Result<Vec<Example>> {
        use crate::types::SymbolKind;

        let mut examples = Vec::new();

        match symbol.kind {
            SymbolKind::Function => {
                // Error handling example
                examples.push(Example {
                    code: format!(
                        "// Advanced error handling\nmatch {}() {{\n  Ok(result) => println!(\"Success: {{:?}}\", result),\n  Err(e) => {{\n    eprintln!(\"Error: {{:?}}\", e);\n    // Handle specific error types\n  }}\n}}",
                        symbol.name
                    ),
                    description: format!("Advanced error handling for {}", symbol.name),
                    language: self.language.clone(),
                    complexity: ExampleComplexity::Advanced,
                });

                // Async with tokio
                examples.push(Example {
                    code: format!(
                        "// Async with concurrency\n#[tokio::main]\nasync fn main() {{\n  let handle = tokio::spawn(async move {{\n    {}().await\n  }});\n  \n  match handle.await {{\n    Ok(result) => println!(\"{{:?}}\", result),\n    Err(e) => eprintln!(\"Task failed: {{:?}}\", e),\n  }}\n}}",
                        symbol.name
                    ),
                    description: format!("Async concurrency pattern for {}", symbol.name),
                    language: self.language.clone(),
                    complexity: ExampleComplexity::Advanced,
                });
            }
            SymbolKind::Struct => {
                // Arc/Mutex pattern
                examples.push(Example {
                    code: format!(
                        "// Thread-safe shared state\nuse std::sync::{{Arc, Mutex}};\n\nlet shared = Arc::new(Mutex::new({}::new()));\nlet shared_clone = Arc::clone(&shared);\n\nstd::thread::spawn(move || {{\n  let mut data = shared_clone.lock().unwrap();\n  // Modify data\n}});",
                        symbol.name
                    ),
                    description: format!("Thread-safe shared state for {}", symbol.name),
                    language: self.language.clone(),
                    complexity: ExampleComplexity::Advanced,
                });
            }
            SymbolKind::Trait => {
                // Trait object pattern
                examples.push(Example {
                    code: format!(
                        "// Dynamic dispatch with trait objects\nfn use_dyn(obj: &dyn {}) {{\n  // Use trait methods dynamically\n}}\n\n// Or with Box\nfn use_boxed(obj: Box<dyn {}>) {{\n  // Owned trait object\n}}",
                        symbol.name,
                        symbol.name
                    ),
                    description: format!("Dynamic dispatch pattern for {}", symbol.name),
                    language: self.language.clone(),
                    complexity: ExampleComplexity::Advanced,
                });
            }
            _ => {}
        }

        Ok(examples)
    }

    fn validate_rust(&self, code: &str) -> Result<ValidationResult> {
        let mut result = ValidationResult::success();

        // Basic syntax checks
        let open_braces = code.matches('{').count();
        let close_braces = code.matches('}').count();

        if open_braces != close_braces {
            result.valid = false;
            result.errors.push("Mismatched braces".to_string());
        }

        // Check for Rust-specific patterns
        if code.contains("unwrap()") {
            result = result.with_warning("Consider using proper error handling instead of unwrap()".to_string());
        }

        Ok(result)
    }

    // Private helper methods for Python
    fn generate_basic_python(&self, symbol: &CodeSymbol) -> Result<String> {
        use crate::types::SymbolKind;

        let code = match symbol.kind {
            SymbolKind::Function | SymbolKind::Method => {
                format!(
                    "# Basic usage of {}\nresult = {}()\nprint(result)",
                    symbol.name, symbol.name
                )
            }
            SymbolKind::Class => {
                format!(
                    "# Basic usage of {}\ninstance = {}()\nprint(instance)",
                    symbol.name, symbol.name
                )
            }
            _ => {
                format!("# Example for {}\n# Usage example", symbol.name)
            }
        };

        Ok(code)
    }

    fn generate_intermediate_python(&self, symbol: &CodeSymbol) -> Result<Vec<Example>> {
        use crate::types::SymbolKind;

        let mut examples = Vec::new();
        let params = self.extract_parameters(&symbol.signature);

        match symbol.kind {
            SymbolKind::Function | SymbolKind::Method => {
                // With type hints
                if !params.is_empty() {
                    let param_defs: Vec<String> = params.iter()
                        .map(|(name, ptype)| format!("{}: {}", name, ptype))
                        .collect();
                    examples.push(Example {
                        code: format!(
                            "# Type-safe usage with hints\ndef use_{}({}) -> None:\n    result = {}({})\n    print(result)",
                            symbol.name,
                            param_defs.join(", "),
                            symbol.name,
                            params.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>().join(", ")
                        ),
                        description: format!("Type-hinted wrapper for {}", symbol.name),
                        language: self.language.clone(),
                        complexity: ExampleComplexity::Intermediate,
                    });
                }

                // With context manager
                examples.push(Example {
                    code: format!(
                        "# Using as context manager\nfrom contextlib import contextmanager\n\n@contextmanager\ndef {}_context():\n    result = {}()\n    try:\n        yield result\n    finally:\n        # Cleanup\n        pass\n\nwith {}_context() as ctx:\n    # Use context\n    pass",
                        symbol.name,
                        symbol.name,
                        symbol.name
                    ),
                    description: format!("Context manager pattern for {}", symbol.name),
                    language: self.language.clone(),
                    complexity: ExampleComplexity::Intermediate,
                });
            }
            SymbolKind::Class => {
                // Decorator pattern
                examples.push(Example {
                    code: format!(
                        "# Decorator pattern\nfrom functools import wraps\n\ndef with_{}(f):\n    @wraps(f)\n    def wrapper(*args, **kwargs):\n        instance = {}()\n        return f(instance, *args, **kwargs)\n    return wrapper\n\n@with_{}\ndef use_instance(instance):\n    # Use the instance\n    pass",
                        symbol.name.to_lowercase(),
                        symbol.name,
                        symbol.name.to_lowercase()
                    ),
                    description: format!("Decorator pattern for {}", symbol.name),
                    language: self.language.clone(),
                    complexity: ExampleComplexity::Intermediate,
                });
            }
            _ => {}
        }

        Ok(examples)
    }

    fn generate_advanced_python(&self, symbol: &CodeSymbol) -> Result<Vec<Example>> {
        use crate::types::SymbolKind;

        let mut examples = Vec::new();

        match symbol.kind {
            SymbolKind::Function | SymbolKind::Method => {
                // Advanced error handling
                examples.push(Example {
                    code: format!(
                        "# Advanced error handling with logging\nimport logging\n\nlogger = logging.getLogger(__name__)\n\ntry:\n    result = {}()\n    logger.info(f'Success: {{result}}')\n    print(result)\nexcept ValueError as ve:\n    logger.error(f'Value error: {{ve}}')\n    raise\nexcept Exception as e:\n    logger.exception(f'Unexpected error: {{e}}')\n    # Handle or re-raise\n    raise",
                        symbol.name
                    ),
                    description: format!("Advanced error handling for {}", symbol.name),
                    language: self.language.clone(),
                    complexity: ExampleComplexity::Advanced,
                });

                // Async with asyncio
                examples.push(Example {
                    code: format!(
                        "# Async/await pattern\nimport asyncio\n\nasync def async_{}():\n    # Async implementation\n    result = await some_async_call()\n    return result\n\nasync def main():\n    tasks = [async_{}() for _ in range(10)]\n    results = await asyncio.gather(*tasks)\n    print(results)\n\nasyncio.run(main())",
                        symbol.name,
                        symbol.name
                    ),
                    description: format!("Async concurrency for {}", symbol.name),
                    language: self.language.clone(),
                    complexity: ExampleComplexity::Advanced,
                });
            }
            SymbolKind::Class => {
                // Metaclass pattern
                examples.push(Example {
                    code: format!(
                        "# Metaclass pattern\nclass {}Meta(type):\n    def __new__(mcs, name, bases, namespace):\n        # Customize class creation\n        return super().__new__(mcs, name, bases, namespace)\n\nclass Enhanced{}({}, metaclass={}Meta):\n    def __init__(self):\n        super().__init__()\n        # Enhanced functionality\n        pass",
                        symbol.name,
                        symbol.name,
                        symbol.name,
                        symbol.name
                    ),
                    description: format!("Metaclass pattern for {}", symbol.name),
                    language: self.language.clone(),
                    complexity: ExampleComplexity::Advanced,
                });
            }
            _ => {}
        }

        Ok(examples)
    }

    /// Calculate quality score for an example
    pub fn calculate_quality_score(&self, example: &Example) -> f32 {
        let mut score: f32 = 100.0;

        // Check for comments
        let has_comments = example.code.contains("//") || example.code.contains("/*") || example.code.contains('#');
        if !has_comments {
            score -= 10.0;
        }

        // Check for error handling
        let has_error_handling = example.code.contains("try")
            || example.code.contains("catch")
            || example.code.contains("Result")
            || example.code.contains("except");
        if !has_error_handling && example.complexity != ExampleComplexity::Basic {
            score -= 15.0;
        }

        // Check length - too short or too long is bad
        let lines = example.code.lines().count();
        if lines < 3 {
            score -= 20.0;
        } else if lines > 50 {
            score -= 10.0;
        }

        // Bonus for intermediate/advanced examples
        match example.complexity {
            ExampleComplexity::Basic => {},
            ExampleComplexity::Intermediate => score += 10.0,
            ExampleComplexity::Advanced => score += 20.0,
        }

        // Check description quality
        if example.description.len() < 10 {
            score -= 5.0;
        }

        score.max(0.0).min(100.0)
    }

    fn validate_python(&self, code: &str) -> Result<ValidationResult> {
        let mut result = ValidationResult::success();

        // Check for balanced parentheses
        let open_parens = code.matches('(').count();
        let close_parens = code.matches(')').count();

        if open_parens != close_parens {
            result.valid = false;
            result.errors.push("Mismatched parentheses".to_string());
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Hash, Location, SymbolId, SymbolKind, SymbolMetadata};

    fn create_test_symbol(kind: SymbolKind, name: &str) -> CodeSymbol {
        CodeSymbol {
            id: SymbolId::generate(),
            name: name.to_string(),
            kind,
            signature: format!("{}()", name),
            body_hash: Hash::from_string("test"),
            location: Location::new("test.ts".to_string(), 1, 10, 0, 100),
            references: Vec::new(),
            dependencies: Vec::new(),
            metadata: SymbolMetadata::default(),
            embedding: None,
        }
    }

    #[test]
    fn test_basic_function_example_typescript() {
        let generator = ExampleGenerator::new("typescript".to_string());
        let symbol = create_test_symbol(SymbolKind::Function, "testFunc");

        let example = generator.generate_basic(&symbol).unwrap();

        assert_eq!(example.language, "typescript");
        assert_eq!(example.complexity, ExampleComplexity::Basic);
        assert!(example.code.contains("testFunc"));
        assert!(example.description.contains("Basic usage"));
    }

    #[test]
    fn test_basic_class_example_typescript() {
        let generator = ExampleGenerator::new("typescript".to_string());
        let symbol = create_test_symbol(SymbolKind::Class, "TestClass");

        let example = generator.generate_basic(&symbol).unwrap();

        assert!(example.code.contains("new TestClass"));
        assert_eq!(example.complexity, ExampleComplexity::Basic);
    }

    #[test]
    fn test_basic_function_example_rust() {
        let generator = ExampleGenerator::new("rust".to_string());
        let symbol = create_test_symbol(SymbolKind::Function, "test_func");

        let example = generator.generate_basic(&symbol).unwrap();

        assert_eq!(example.language, "rust");
        assert!(example.code.contains("test_func"));
        assert!(example.code.contains("println!"));
    }

    #[test]
    fn test_basic_struct_example_rust() {
        let generator = ExampleGenerator::new("rust".to_string());
        let symbol = create_test_symbol(SymbolKind::Struct, "TestStruct");

        let example = generator.generate_basic(&symbol).unwrap();

        assert!(example.code.contains("TestStruct::new"));
    }

    #[test]
    fn test_advanced_function_examples_typescript() {
        let generator = ExampleGenerator::new("typescript".to_string());
        let symbol = create_test_symbol(SymbolKind::Function, "testFunc");

        let examples = generator.generate_advanced(&symbol).unwrap();

        assert!(!examples.is_empty());
        assert!(examples.iter().any(|e| e.description.contains("Error handling")));
        assert!(examples.iter().any(|e| e.description.contains("Async")));
    }

    #[test]
    fn test_advanced_class_examples_typescript() {
        let generator = ExampleGenerator::new("typescript".to_string());
        let symbol = create_test_symbol(SymbolKind::Class, "TestClass");

        let examples = generator.generate_advanced(&symbol).unwrap();

        assert!(!examples.is_empty());
        assert!(examples.iter().any(|e| e.description.contains("Inheritance")));
    }

    #[test]
    fn test_validation_success() {
        let generator = ExampleGenerator::new("typescript".to_string());
        let example = Example {
            code: "const x = { test: 1 };".to_string(),
            description: "Test".to_string(),
            language: "typescript".to_string(),
            complexity: ExampleComplexity::Basic,
        };

        let result = generator.validate(&example).unwrap();
        assert!(result.valid);
    }

    #[test]
    fn test_validation_mismatched_braces() {
        let generator = ExampleGenerator::new("typescript".to_string());
        let example = Example {
            code: "const x = { test: 1;".to_string(),
            description: "Test".to_string(),
            language: "typescript".to_string(),
            complexity: ExampleComplexity::Basic,
        };

        let result = generator.validate(&example).unwrap();
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("braces")));
    }

    #[test]
    fn test_validation_empty_code() {
        let generator = ExampleGenerator::new("typescript".to_string());
        let example = Example {
            code: "".to_string(),
            description: "Test".to_string(),
            language: "typescript".to_string(),
            complexity: ExampleComplexity::Basic,
        };

        let result = generator.validate(&example).unwrap();
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("empty")));
    }

    #[test]
    fn test_validation_language_mismatch() {
        let generator = ExampleGenerator::new("typescript".to_string());
        let example = Example {
            code: "test".to_string(),
            description: "Test".to_string(),
            language: "rust".to_string(),
            complexity: ExampleComplexity::Basic,
        };

        let result = generator.validate(&example).unwrap();
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("Language mismatch")));
    }

    #[test]
    fn test_unsupported_language() {
        let generator = ExampleGenerator::new("cobol".to_string());
        let symbol = create_test_symbol(SymbolKind::Function, "test");

        let result = generator.generate_basic(&symbol);
        assert!(result.is_err());
    }

    #[test]
    fn test_python_basic_function() {
        let generator = ExampleGenerator::new("python".to_string());
        let symbol = create_test_symbol(SymbolKind::Function, "test_func");

        let example = generator.generate_basic(&symbol).unwrap();

        assert_eq!(example.language, "python");
        assert!(example.code.contains("test_func"));
        assert!(example.code.contains("print"));
    }

    #[test]
    fn test_rust_validation_unwrap_warning() {
        let generator = ExampleGenerator::new("rust".to_string());
        let example = Example {
            code: "let x = some_func().unwrap();".to_string(),
            description: "Test".to_string(),
            language: "rust".to_string(),
            complexity: ExampleComplexity::Basic,
        };

        let result = generator.validate(&example).unwrap();
        assert!(result.valid);
        assert!(!result.warnings.is_empty());
        assert!(result.warnings.iter().any(|w| w.contains("unwrap")));
    }
}
