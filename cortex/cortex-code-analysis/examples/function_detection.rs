//! Example demonstrating function detection across multiple languages.
//!
//! This example shows how to use the `detect_functions` API to find all functions
//! in source code for various programming languages.
//!
//! Run with: cargo run --example function_detection

use cortex_code_analysis::{detect_functions, Lang};

fn main() -> anyhow::Result<()> {
    println!("=== Function Detection Demo ===\n");

    // Detect functions in Rust code
    detect_rust_functions()?;

    // Detect functions in TypeScript code
    detect_typescript_functions()?;

    // Detect functions in JavaScript code
    detect_javascript_functions()?;

    // Detect functions in Python code
    detect_python_functions()?;

    Ok(())
}

fn detect_rust_functions() -> anyhow::Result<()> {
    println!("--- Rust Functions ---");

    let rust_code = r#"
/// Main entry point
fn main() {
    println!("Hello, world!");
}

/// Adds two numbers together
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// Multiplies two numbers
pub fn multiply(x: i32, y: i32) -> i32 {
    x * y
}

/// An async function
async fn fetch_data() -> Result<String, Box<dyn std::error::Error>> {
    Ok("data".to_string())
}

mod utils {
    /// A nested module function
    pub fn helper() -> bool {
        true
    }
}
"#;

    let functions = detect_functions(rust_code, Lang::Rust)?;

    println!("Found {} functions:", functions.len());
    for func in &functions {
        println!(
            "  - {} (lines {}-{})",
            func.name, func.start_line, func.end_line
        );
    }
    println!();

    Ok(())
}

fn detect_typescript_functions() -> anyhow::Result<()> {
    println!("--- TypeScript Functions ---");

    let ts_code = r#"
// A simple greeting function
function greet(name: string): void {
    console.log(`Hello, ${name}!`);
}

// Arrow function assigned to variable
const add = (a: number, b: number): number => {
    return a + b;
};

// A class with methods
class Calculator {
    private value: number = 0;

    // Constructor is a method too
    constructor(initialValue: number) {
        this.value = initialValue;
    }

    // Method to add
    add(n: number): number {
        return this.value + n;
    }

    // Method to multiply
    multiply(n: number): number {
        return this.value * n;
    }
}

// Async function
async function fetchData(): Promise<string> {
    return "data";
}
"#;

    let functions = detect_functions(ts_code, Lang::TypeScript)?;

    println!("Found {} functions:", functions.len());
    for func in &functions {
        println!(
            "  - {} (lines {}-{})",
            func.name, func.start_line, func.end_line
        );
    }
    println!();

    Ok(())
}

fn detect_javascript_functions() -> anyhow::Result<()> {
    println!("--- JavaScript Functions ---");

    let js_code = r#"
// Traditional function declaration
function greet(name) {
    console.log(`Hello, ${name}!`);
}

// Arrow function
const add = (a, b) => {
    return a + b;
};

// Short arrow function
const multiply = (x, y) => x * y;

// Object with methods
const calculator = {
    value: 0,

    add: function(n) {
        return this.value + n;
    },

    multiply(n) {
        return this.value * n;
    }
};

// Class with methods
class Person {
    constructor(name) {
        this.name = name;
    }

    greet() {
        console.log(`Hi, I'm ${this.name}`);
    }
}
"#;

    let functions = detect_functions(js_code, Lang::JavaScript)?;

    println!("Found {} functions:", functions.len());
    for func in &functions {
        println!(
            "  - {} (lines {}-{})",
            func.name, func.start_line, func.end_line
        );
    }
    println!();

    Ok(())
}

fn detect_python_functions() -> anyhow::Result<()> {
    println!("--- Python Functions ---");

    let python_code = r#"
def greet(name):
    """Greet someone by name."""
    print(f"Hello, {name}!")

def add(a, b):
    """Add two numbers."""
    return a + b

def multiply(x, y):
    """Multiply two numbers."""
    return x * y

class Calculator:
    """A simple calculator class."""

    def __init__(self, initial_value=0):
        """Initialize the calculator."""
        self.value = initial_value

    def add(self, n):
        """Add to the current value."""
        return self.value + n

    def multiply(self, n):
        """Multiply the current value."""
        return self.value * n

async def fetch_data():
    """Fetch data asynchronously."""
    return "data"
"#;

    let functions = detect_functions(python_code, Lang::Python)?;

    println!("Found {} functions:", functions.len());
    for func in &functions {
        println!(
            "  - {} (lines {}-{}, {} lines)",
            func.name, func.start_line, func.end_line, func.line_count()
        );
    }
    println!();

    Ok(())
}
