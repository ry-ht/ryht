//! Comprehensive tests for the analysis module.

#[cfg(test)]
mod tests {
    use crate::analysis::{
        DefaultNodeChecker, DefaultNodeGetter, HalsteadType, NodeChecker, NodeGetter, SpaceKind,
    };
    use crate::{Lang, Node};
    use tree_sitter::Parser as TSParser;

    fn parse_code(code: &str, lang: Lang) -> (Vec<u8>, tree_sitter::Tree) {
        let mut parser = TSParser::new();
        let ts_lang = lang.get_ts_language();
        parser.set_language(&ts_lang).unwrap();
        let code_bytes = code.as_bytes().to_vec();
        let tree = parser.parse(&code_bytes, None).unwrap();
        (code_bytes, tree)
    }

    // ===== Rust Tests =====

    #[test]
    fn test_rust_comment_detection() {
        let (_code_bytes, tree) = parse_code(
            "// This is a line comment\n/* block comment */\nfn main() {}",
            Lang::Rust,
        );
        let root = Node::new(tree.root_node());

        let mut found_comment = false;
        for node in root.children() {
            if DefaultNodeChecker::is_comment(&node, Lang::Rust) {
                found_comment = true;
                break;
            }
        }
        assert!(found_comment, "Should detect Rust comments");
    }

    #[test]
    fn test_rust_function_detection() {
        let (_code_bytes, tree) =
            parse_code("fn add(a: i32, b: i32) -> i32 { a + b }", Lang::Rust);
        let root = Node::new(tree.root_node());

        let mut found_func = false;
        for node in root.children() {
            if DefaultNodeChecker::is_func(&node, Lang::Rust) {
                found_func = true;
                break;
            }
        }
        assert!(found_func, "Should detect Rust function");
    }

    #[test]
    fn test_rust_closure_detection() {
        let (_code_bytes, tree) = parse_code("fn main() { let f = |x| x + 1; }", Lang::Rust);
        let root = Node::new(tree.root_node());

        let mut found_closure = false;
        fn check_node(node: &Node, found: &mut bool) {
            if DefaultNodeChecker::is_closure(node, Lang::Rust) {
                *found = true;
                return;
            }
            for child in node.children() {
                check_node(&child, found);
            }
        }
        check_node(&root, &mut found_closure);
        assert!(found_closure, "Should detect Rust closure");
    }

    #[test]
    fn test_rust_space_kind() {
        let (_code_bytes, tree) = parse_code("fn test() {}", Lang::Rust);
        let root = Node::new(tree.root_node());

        for node in root.children() {
            let kind = DefaultNodeGetter::get_space_kind(&node, Lang::Rust);
            if kind == SpaceKind::Function {
                return; // Test passed
            }
        }
        panic!("Should identify function space kind");
    }

    #[test]
    fn test_rust_func_name_extraction() {
        let (code_bytes, tree) = parse_code("fn calculate() {}", Lang::Rust);
        let root = Node::new(tree.root_node());

        for node in root.children() {
            if DefaultNodeChecker::is_func(&node, Lang::Rust) {
                let name = DefaultNodeGetter::get_func_name(&node, &code_bytes, Lang::Rust);
                assert_eq!(name, Some("calculate"));
                return;
            }
        }
        panic!("Should extract function name");
    }

    // ===== Python Tests =====

    #[test]
    fn test_python_comment_detection() {
        let (_code_bytes, tree) =
            parse_code("# This is a comment\ndef main():\n    pass", Lang::Python);
        let root = Node::new(tree.root_node());

        let mut found_comment = false;
        for node in root.children() {
            if DefaultNodeChecker::is_comment(&node, Lang::Python) {
                found_comment = true;
                break;
            }
        }
        assert!(found_comment, "Should detect Python comments");
    }

    #[test]
    fn test_python_function_detection() {
        let (_code_bytes, tree) = parse_code("def greet(name):\n    print(name)", Lang::Python);
        let root = Node::new(tree.root_node());

        let mut found_func = false;
        for node in root.children() {
            if DefaultNodeChecker::is_func(&node, Lang::Python) {
                found_func = true;
                break;
            }
        }
        assert!(found_func, "Should detect Python function");
    }

    #[test]
    fn test_python_lambda_detection() {
        let (_code_bytes, tree) = parse_code("f = lambda x: x + 1", Lang::Python);
        let root = Node::new(tree.root_node());

        let mut found_lambda = false;
        fn check_node(node: &Node, found: &mut bool) {
            if DefaultNodeChecker::is_closure(node, Lang::Python) {
                *found = true;
                return;
            }
            for child in node.children() {
                check_node(&child, found);
            }
        }
        check_node(&root, &mut found_lambda);
        assert!(found_lambda, "Should detect Python lambda");
    }

    #[test]
    fn test_python_space_kind() {
        let (_code_bytes, tree) = parse_code("class MyClass:\n    pass", Lang::Python);
        let root = Node::new(tree.root_node());

        for node in root.children() {
            let kind = DefaultNodeGetter::get_space_kind(&node, Lang::Python);
            if kind == SpaceKind::Class {
                return; // Test passed
            }
        }
        panic!("Should identify class space kind");
    }

    // ===== TypeScript Tests =====

    #[test]
    fn test_typescript_comment_detection() {
        let (_code_bytes, tree) =
            parse_code("// comment\nfunction test() {}", Lang::TypeScript);
        let root = Node::new(tree.root_node());

        let mut found_comment = false;
        for node in root.children() {
            if DefaultNodeChecker::is_comment(&node, Lang::TypeScript) {
                found_comment = true;
                break;
            }
        }
        assert!(found_comment, "Should detect TypeScript comments");
    }

    #[test]
    fn test_typescript_function_detection() {
        let (_code_bytes, tree) = parse_code(
            "function add(a: number, b: number): number { return a + b; }",
            Lang::TypeScript,
        );
        let root = Node::new(tree.root_node());

        let mut found_func = false;
        for node in root.children() {
            if DefaultNodeChecker::is_func(&node, Lang::TypeScript) {
                found_func = true;
                break;
            }
        }
        assert!(found_func, "Should detect TypeScript function");
    }

    #[test]
    fn test_typescript_arrow_function() {
        let (_code_bytes, tree) =
            parse_code("const add = (a: number, b: number) => a + b;", Lang::TypeScript);
        let root = Node::new(tree.root_node());

        let mut found_arrow = false;
        fn check_node(node: &Node, found: &mut bool) {
            if node.kind() == "arrow_function" {
                *found = true;
                return;
            }
            for child in node.children() {
                check_node(&child, found);
            }
        }
        check_node(&root, &mut found_arrow);
        assert!(found_arrow, "Should detect TypeScript arrow function");
    }

    #[test]
    fn test_typescript_interface_space_kind() {
        let (_code_bytes, tree) =
            parse_code("interface Person { name: string; }", Lang::TypeScript);
        let root = Node::new(tree.root_node());

        for node in root.children() {
            let kind = DefaultNodeGetter::get_space_kind(&node, Lang::TypeScript);
            if kind == SpaceKind::Interface {
                return; // Test passed
            }
        }
        panic!("Should identify interface space kind");
    }

    // ===== JavaScript Tests =====

    #[test]
    fn test_javascript_function_detection() {
        let (_code_bytes, tree) =
            parse_code("function test() { return 42; }", Lang::JavaScript);
        let root = Node::new(tree.root_node());

        let mut found_func = false;
        for node in root.children() {
            if DefaultNodeChecker::is_func(&node, Lang::JavaScript) {
                found_func = true;
                break;
            }
        }
        assert!(found_func, "Should detect JavaScript function");
    }

    #[test]
    fn test_javascript_call_detection() {
        let (_code_bytes, tree) = parse_code("console.log('hello');", Lang::JavaScript);
        let root = Node::new(tree.root_node());

        let mut found_call = false;
        fn check_node(node: &Node, found: &mut bool) {
            if DefaultNodeChecker::is_call(node, Lang::JavaScript) {
                *found = true;
                return;
            }
            for child in node.children() {
                check_node(&child, found);
            }
        }
        check_node(&root, &mut found_call);
        assert!(found_call, "Should detect JavaScript function call");
    }

    // ===== Java Tests =====

    #[test]
    fn test_java_method_detection() {
        let (_code_bytes, tree) = parse_code("class Test { public void run() {} }", Lang::Java);
        let root = Node::new(tree.root_node());

        let mut found_method = false;
        fn check_node(node: &Node, found: &mut bool) {
            if DefaultNodeChecker::is_func(node, Lang::Java) {
                *found = true;
                return;
            }
            for child in node.children() {
                check_node(&child, found);
            }
        }
        check_node(&root, &mut found_method);
        assert!(found_method, "Should detect Java method");
    }

    #[test]
    fn test_java_class_space_kind() {
        let (_code_bytes, tree) = parse_code("class MyClass {}", Lang::Java);
        let root = Node::new(tree.root_node());

        for node in root.children() {
            let kind = DefaultNodeGetter::get_space_kind(&node, Lang::Java);
            if kind == SpaceKind::Class {
                return; // Test passed
            }
        }
        // Java wraps everything in a program node, so check children
        for node in root.children() {
            for child in node.children() {
                let kind = DefaultNodeGetter::get_space_kind(&child, Lang::Java);
                if kind == SpaceKind::Class {
                    return;
                }
            }
        }
        panic!("Should identify class space kind");
    }

    // ===== C++ Tests =====

    #[test]
    fn test_cpp_function_detection() {
        let (_code_bytes, tree) =
            parse_code("int add(int a, int b) { return a + b; }", Lang::Cpp);
        let root = Node::new(tree.root_node());

        let mut found_func = false;
        for node in root.children() {
            if DefaultNodeChecker::is_func(&node, Lang::Cpp) {
                found_func = true;
                break;
            }
        }
        assert!(found_func, "Should detect C++ function");
    }

    #[test]
    fn test_cpp_namespace_space_kind() {
        let (_code_bytes, tree) = parse_code("namespace test { void func() {} }", Lang::Cpp);
        let root = Node::new(tree.root_node());

        for node in root.children() {
            let kind = DefaultNodeGetter::get_space_kind(&node, Lang::Cpp);
            if kind == SpaceKind::Namespace {
                return; // Test passed
            }
        }
        panic!("Should identify namespace space kind");
    }

    // ===== Halstead Type Tests =====

    #[test]
    fn test_halstead_operator_detection() {
        let (_code_bytes, tree) = parse_code("fn test() { let x = 1 + 2; }", Lang::Rust);
        let root = Node::new(tree.root_node());

        let mut found_operator = false;
        fn check_node(node: &Node, found: &mut bool) {
            let op_type = DefaultNodeGetter::get_op_type(node, Lang::Rust);
            if op_type == HalsteadType::Operator {
                *found = true;
                return;
            }
            for child in node.children() {
                check_node(&child, found);
            }
        }
        check_node(&root, &mut found_operator);
        assert!(found_operator, "Should detect Halstead operators");
    }

    #[test]
    fn test_halstead_operand_detection() {
        let (_code_bytes, tree) = parse_code("x = 42", Lang::Python);
        let root = Node::new(tree.root_node());

        let mut found_operand = false;
        fn check_node(node: &Node, found: &mut bool) {
            let op_type = DefaultNodeGetter::get_op_type(node, Lang::Python);
            if op_type == HalsteadType::Operand {
                *found = true;
                return;
            }
            for child in node.children() {
                check_node(&child, found);
            }
        }
        check_node(&root, &mut found_operand);
        assert!(found_operand, "Should detect Halstead operands");
    }

    // ===== String Detection Tests =====

    #[test]
    fn test_string_detection_rust() {
        let (_code_bytes, tree) = parse_code(r#"let s = "hello";"#, Lang::Rust);
        let root = Node::new(tree.root_node());

        let mut found_string = false;
        fn check_node(node: &Node, found: &mut bool) {
            if DefaultNodeChecker::is_string(node, Lang::Rust) {
                *found = true;
                return;
            }
            for child in node.children() {
                check_node(&child, found);
            }
        }
        check_node(&root, &mut found_string);
        assert!(found_string, "Should detect Rust string literals");
    }

    #[test]
    fn test_string_detection_python() {
        let (_code_bytes, tree) = parse_code(r#"s = "hello""#, Lang::Python);
        let root = Node::new(tree.root_node());

        let mut found_string = false;
        fn check_node(node: &Node, found: &mut bool) {
            if DefaultNodeChecker::is_string(node, Lang::Python) {
                *found = true;
                return;
            }
            for child in node.children() {
                check_node(&child, found);
            }
        }
        check_node(&root, &mut found_string);
        assert!(found_string, "Should detect Python string literals");
    }

    // ===== Error Detection Tests =====

    #[test]
    fn test_error_detection() {
        let (_code_bytes, tree) = parse_code("fn test( { }", Lang::Rust); // Intentionally malformed
        let root = Node::new(tree.root_node());

        assert!(
            DefaultNodeChecker::is_error(&root),
            "Should detect syntax errors"
        );
    }

    // ===== Basic Sanity Tests =====

    #[test]
    fn test_types_exist() {
        let _ = HalsteadType::Operator;
        let _ = SpaceKind::Function;
    }

    #[test]
    fn test_trait_implementations() {
        // Test that we can use the traits
        fn _use_checker<T: NodeChecker>() {}
        fn _use_getter<T: NodeGetter>() {}

        _use_checker::<DefaultNodeChecker>();
        _use_getter::<DefaultNodeGetter>();
    }
}
