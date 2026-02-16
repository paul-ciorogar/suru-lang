//! Structural type compatibility for semantic analysis
//!
//! This module implements:
//! - Deferred method check verification for generic type parameters
//! - Structural subtyping: if a type has the required methods, it's compatible
//!
//! When a method is called on a generic TypeParameter receiver (e.g., `obj.add(value)`
//! where obj is of type T), the check is deferred until after unification resolves T
//! to a concrete type. This module verifies those deferred checks.

use super::{SemanticAnalyzer, SemanticError, Type};

impl SemanticAnalyzer {
    /// Verifies all deferred method checks after unification
    ///
    /// For each deferred check:
    /// - Resolves the receiver TypeParameter via substitution
    /// - If resolved to Struct: verifies method exists, checks arg count, adds constraints
    /// - If still TypeParameter: allows it (duck typing promise)
    /// - If resolved to non-struct: reports error
    pub(super) fn verify_deferred_checks(&mut self) {
        let checks = self.deferred_method_checks.clone();

        for check in &checks {
            // Resolve the receiver type through substitution
            let resolved_type_id =
                self.substitution.apply(check.receiver_type_id, &self.type_registry);
            let resolved_type = self.type_registry.resolve(resolved_type_id).clone();

            match resolved_type {
                Type::Struct(_) => {
                    // Resolved to a concrete struct - verify the method exists
                    if let Some(method_func_type_id) =
                        self.lookup_struct_method_type(resolved_type_id, &check.method_name)
                    {
                        let func_type = self.type_registry.resolve(method_func_type_id).clone();
                        if let Type::Function(ft) = func_type {
                            // Check argument count
                            if check.arg_type_ids.len() != ft.params.len() {
                                self.record_error(self.make_error(
                                    format!(
                                        "Method '{}' expects {} argument(s) but got {}",
                                        check.method_name,
                                        ft.params.len(),
                                        check.arg_type_ids.len()
                                    ),
                                    check.call_node_idx,
                                ));
                            }

                            // Add type constraints for arguments
                            for (arg_type_id, param) in
                                check.arg_type_ids.iter().zip(ft.params.iter())
                            {
                                let param_type = self.type_registry.resolve(param.type_id);
                                if !matches!(param_type, Type::Unknown) {
                                    self.add_constraint(
                                        *arg_type_id,
                                        param.type_id,
                                        check.call_node_idx,
                                    );
                                }
                            }

                            // Update return type: unify the placeholder with actual return type
                            if let Some(call_type) = self.get_node_type(check.call_node_idx) {
                                self.add_constraint(
                                    call_type,
                                    ft.return_type,
                                    check.call_node_idx,
                                );
                            }
                        }
                    } else {
                        let token = self.ast.nodes[check.method_name_node_idx]
                            .token
                            .as_ref()
                            .unwrap();
                        self.record_error(SemanticError::from_token(
                            format!(
                                "Method '{}' does not exist on struct type",
                                check.method_name
                            ),
                            token,
                        ));
                    }
                }
                Type::TypeParameter { .. } => {
                    // Still a TypeParameter after unification - this is fine
                    // The method will be checked when the generic is instantiated
                }
                _ => {
                    // Resolved to a non-struct, non-TypeParameter type
                    let token = self.ast.nodes[check.method_name_node_idx]
                        .token
                        .as_ref()
                        .unwrap();
                    self.record_error(SemanticError::from_token(
                        format!(
                            "Cannot call method '{}' on non-struct type",
                            check.method_name
                        ),
                        token,
                    ));
                }
            }
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

    // ========== Generic Function Declaration Tests ==========

    #[test]
    fn test_generic_function_declaration_succeeds() {
        let source = r#"
            identity<T>: (x T) T {
                return x
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Generic function declaration should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_generic_function_multiple_type_params() {
        let source = r#"
            pair<T, U>: (a T, b U) T {
                return a
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Generic function with multiple type params should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_generic_function_with_method_call_on_type_param() {
        let source = r#"
            acceptsAdd<T>: (obj T, value Number) T {
                return obj.add(value)
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Method call on TypeParameter should be deferred and succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_generic_function_with_property_access_on_type_param() {
        let source = r#"
            getName<T>: (obj T) {
                x: obj.name
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Property access on TypeParameter should succeed: {:?}",
            result.err()
        );
    }

    // ========== Non-Generic Regression Tests ==========

    #[test]
    fn test_non_generic_function_still_works() {
        let source = r#"
            add: (x Number, y Number) Number {
                return x
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Non-generic functions should still work: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_non_generic_method_call_still_errors() {
        let source = "x: 42\ny: x.double()\n";
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Method call on non-struct should still fail"
        );
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Cannot call method 'double' on non-struct type")),
            "Error should mention non-struct type: {:?}",
            errors
        );
    }

    // ========== Parameter Type Registration Tests ==========

    #[test]
    fn test_typed_parameter_method_call() {
        let source = r#"
            type Greeter: { greet: () String }
            callGreet: (g Greeter) String {
                return g.greet()
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Method call on typed parameter should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_typed_parameter_property_access() {
        let source = r#"
            type Person: { name String }
            getName: (p Person) String {
                return p.name
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Property access on typed parameter should succeed: {:?}",
            result.err()
        );
    }

    // ========== Parser Generic Function Tests ==========

    #[test]
    fn test_parser_generic_function_single_param() {
        let limits = CompilerLimits::default();
        let tokens = lex("f<T>: (x T) T { return x }\n", &limits).unwrap();
        let ast = parse(tokens, &limits);
        assert!(
            ast.is_ok(),
            "Parser should handle single generic param: {:?}",
            ast.err()
        );
    }

    #[test]
    fn test_parser_generic_function_multiple_params() {
        let limits = CompilerLimits::default();
        let tokens = lex("f<T, U>: (x T, y U) { }\n", &limits).unwrap();
        let ast = parse(tokens, &limits);
        assert!(
            ast.is_ok(),
            "Parser should handle multiple generic params: {:?}",
            ast.err()
        );
    }

    #[test]
    fn test_parser_generic_function_with_constraint() {
        let limits = CompilerLimits::default();
        let tokens = lex("f<T: Number>: (x T) { }\n", &limits).unwrap();
        let ast = parse(tokens, &limits);
        assert!(
            ast.is_ok(),
            "Parser should handle constrained generic param: {:?}",
            ast.err()
        );
    }
}
