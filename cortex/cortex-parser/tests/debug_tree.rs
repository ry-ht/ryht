//! Debug test to understand tree structure

#[test]
fn debug_simple_function() {
    let source = "fn add(a: i32, b: i32) -> i32 { a + b }";
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).unwrap();
    let tree = parser.parse(source, None).unwrap();
    let root = tree.root_node();

    println!("Source: {}", source);
    println!("Root kind: {}", root.kind());

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        println!("Child: {} = {}", child.kind(), child.utf8_text(source.as_bytes()).unwrap());

        // Check specific field names
        if child.kind() == "function_item" {
            if let Some(name) = child.child_by_field_name("name") {
                println!("  name field: {}", name.utf8_text(source.as_bytes()).unwrap());
            }
            if let Some(params) = child.child_by_field_name("parameters") {
                println!("  parameters field: {}", params.utf8_text(source.as_bytes()).unwrap());
            }
            if let Some(ret) = child.child_by_field_name("return_type") {
                println!("  return_type field: {}", ret.utf8_text(source.as_bytes()).unwrap());
            }
            if let Some(body) = child.child_by_field_name("body") {
                println!("  body field: {}", body.utf8_text(source.as_bytes()).unwrap());
            }
        }
    }
}
