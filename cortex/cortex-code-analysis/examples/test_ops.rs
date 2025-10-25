use cortex_code_analysis::{extract_ops, Lang};

fn main() {
    println!("Testing simple code:");
    let code = "let x = 5 + 3;";
    match extract_ops(code, Lang::Rust) {
        Ok(ops) => {
            println!("Operators: {:?}", ops.operators);
            println!("Operands: {:?}", ops.operands);
            println!("\nDetails:");
            println!("Name: {:?}", ops.name);
            println!("Kind: {:?}", ops.kind);
            println!("Start: {}, End: {}", ops.start_line, ops.end_line);
            println!("Spaces: {}", ops.spaces.len());

            // Check subspaces
            for (i, space) in ops.spaces.iter().enumerate() {
                println!("\nSubspace {}:", i);
                println!("  Name: {:?}", space.name);
                println!("  Kind: {:?}", space.kind);
                println!("  Operators: {:?}", space.operators);
                println!("  Operands: {:?}", space.operands);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }

    println!("\n\nTesting function:");
    let code2 = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
    match extract_ops(code2, Lang::Rust) {
        Ok(ops) => {
            println!("Operators: {:?}", ops.operators);
            println!("Operands: {:?}", ops.operands);
            println!("Spaces: {}", ops.spaces.len());

            for (i, space) in ops.spaces.iter().enumerate() {
                println!("\nSubspace {}:", i);
                println!("  Name: {:?}", space.name);
                println!("  Kind: {:?}", space.kind);
                println!("  Operators: {:?}", space.operators);
                println!("  Operands: {:?}", space.operands);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}
