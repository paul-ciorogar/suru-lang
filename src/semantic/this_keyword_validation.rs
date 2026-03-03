//! `this` keyword validation for semantic analysis
//!
//! This module handles:
//! - Checking that `this` is only used inside a method body
//! - Resolving `this` to the correct struct type of the enclosing struct literal
//!
//! # Behaviour
//!
//! The `this` keyword is valid only inside a method body within a struct literal.
//! The analyzer tracks the current struct type via `current_struct_type`, which is
//! set by the struct initialisation visitor before visiting each method body and
//! cleared (restored) afterwards.
//!
//! ```suru
//! obj: {
//!     name: "Paul"
//!     getName: () String { return this.name }   # OK – resolves to obj's struct type
//! }
//!
//! x: this                 # Error: 'this' outside method context
//! foo: () { y: this }     # Error: 'this' in a plain function, not a method
//! ```

use super::{SemanticAnalyzer, SemanticError};

impl SemanticAnalyzer {
    /// Visits a `this` keyword node.
    ///
    /// Sets the node's type to the enclosing struct's `TypeId` when inside a method
    /// body. Reports an error when `this` appears outside a method context.
    pub(super) fn visit_this(&mut self, node_idx: usize) {
        if let Some(struct_type_id) = self.current_struct_type {
            self.set_node_type(node_idx, struct_type_id);
        } else {
            let token = self.ast.nodes[node_idx].token.as_ref().unwrap();
            self.record_error(SemanticError::from_token(
                "'this' can only be used inside a method body".to_string(),
                token,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::lex;
    use crate::limits::CompilerLimits;
    use crate::parser::parse;
    use crate::semantic::{SemanticAnalyzer, SemanticError};

    fn analyze_source(source: &str) -> Result<crate::ast::Ast, Vec<SemanticError>> {
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let analyzer = SemanticAnalyzer::new(ast);
        analyzer.analyze()
    }

    // ========== Invalid `this` Usage ==========

    #[test]
    fn test_this_outside_method_error() {
        let source = "x: this\n";
        let result = analyze_source(source);
        assert!(result.is_err(), "'this' outside method should fail");
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("'this' can only be used inside a method body")),
            "Error should mention this context: {:?}",
            errors
        );
    }

    #[test]
    fn test_this_in_regular_function_error() {
        let source = "foo: () { x: this }\n";
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "'this' in regular function should fail"
        );
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("'this' can only be used inside a method body")),
            "Error should mention this context: {:?}",
            errors
        );
    }

    #[test]
    fn test_this_in_return_of_regular_function_error() {
        let source = "bar: () { return this }\n";
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "'this' in return of regular function should fail"
        );
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("'this' can only be used inside a method body")),
            "Expected 'this' context error, got: {:?}",
            errors
        );
    }

    // ========== Valid `this` Usage ==========

    #[test]
    fn test_this_in_method_body_field_access() {
        let source =
            "obj: {\n    name: \"Paul\"\n    getName: () String { return this.name }\n}\n";
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "'this' in method body should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_this_returned_from_method() {
        // Returning `this` itself should succeed – the node receives the struct type.
        let source = "obj: {\n    value: 42\n    getSelf: () { return this }\n}\n";
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Returning 'this' from a method should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_this_method_call() {
        // Calling another method via `this` inside a method body.
        let source = r#"
            obj: {
                greet: () String { return "hello" }
                greetTwice: () String { return this.greet() }
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "'this.method()' inside a method body should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_this_field_used_in_expression() {
        // `this.field` used as part of a boolean expression.
        let source = r#"
            obj: {
                active: true
                isActive: () Bool { return this.active and true }
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "'this.field' in expression should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_multiple_methods_using_this() {
        // Several methods on the same struct all using `this`.
        let source = r#"
            obj: {
                x: 10
                y: 20
                getX: () Number { return this.x }
                getY: () Number { return this.y }
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Multiple methods using 'this' should all succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_this_in_typed_struct_method() {
        // `this` used inside a method of a typed struct variable.
        let source = r#"
            type Person: {
                name String
                greet: () String
            }
            p Person: {
                name: "Paul"
                greet: () String { return this.name }
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "'this' in typed struct method should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_this_nested_struct_resolves_to_inner() {
        // In a nested struct literal, `this` inside the inner method
        // should refer to the inner struct, not the outer one.
        let source = r#"
            outer: {
                inner: {
                    value: 42
                    getValue: () Number { return this.value }
                }
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "'this' in nested struct method should resolve to inner struct: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_this_method_call_with_args() {
        // Calling `this.method(arg)` with the correct argument type.
        let source = r#"
            obj: {
                add: (x Number, y Number) Number { return x }
                compute: () Number { return this.add(1, 2) }
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "'this.method(args)' should succeed: {:?}",
            result.err()
        );
    }

    // ========== `this` Field / Method Error Cases ==========

    #[test]
    fn test_this_property_nonexistent_field_error() {
        let source =
            "obj: {\n    name: \"Paul\"\n    getBad: () String { return this.email }\n}\n";
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "'this.email' on struct without email should fail"
        );
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Field 'email' does not exist")),
            "Error should mention nonexistent field: {:?}",
            errors
        );
    }

    #[test]
    fn test_this_method_call_wrong_arg_type() {
        // `this.add("hello")` when `add` expects a Number.
        let source = r#"
            obj: {
                add: (x Number) Number { return x }
                bad: () Number { return this.add("hello") }
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "'this.method' with wrong arg type should fail"
        );
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Type mismatch")),
            "Expected type mismatch error, got: {:?}",
            errors
        );
    }
}
