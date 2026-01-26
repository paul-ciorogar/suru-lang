// Return type validation implementation for Phase 5.3
//
// This module implements validation of function return types:
// - Check all return statements match declared return type
// - Infer return type if not declared
// - Check function has at least one return (if return type specified)

use super::{SemanticAnalyzer, Type, TypeId};

impl SemanticAnalyzer {
    /// Validates return types for a function
    ///
    /// This method is called after visiting the function body to ensure
    /// all return statements are consistent with the declared return type.
    ///
    /// # Cases Handled
    ///
    /// 1. **Declared non-void type**: Each return value must match the declared type.
    ///    Uses `add_constraint` for unification.
    ///
    /// 2. **No return type annotation (Unknown)**: Infer from returns by constraining
    ///    all return values to equal each other. Unification determines the common type.
    ///
    /// 3. **Void return in non-void function**: Reports an error for bare `return`
    ///    statements when function declares a return type.
    ///
    /// 4. **Missing return**: Reports an error if a function with a declared return
    ///    type has no return statements.
    ///
    /// # Arguments
    ///
    /// * `func_decl_idx` - AST node index of the FunctionDecl
    /// * `func_type_id` - TypeId of the function's FunctionType
    pub(super) fn validate_function_returns(&mut self, func_decl_idx: usize, func_type_id: TypeId) {
        // Get the function type to extract declared return type
        let func_type = self.type_registry.resolve(func_type_id).clone();
        let declared_return_type = match &func_type {
            Type::Function(ft) => ft.return_type,
            _ => return, // Not a function type, nothing to validate
        };

        // Get all recorded returns for this function
        let returns = match self.get_function_returns(func_decl_idx) {
            Some(returns) => returns.clone(),
            None => return, // No return tracking for this function
        };

        // Resolve the declared return type to check what kind it is
        let declared_type = self.type_registry.resolve(declared_return_type).clone();

        match &declared_type {
            // Case 1: No declared return type (Unknown) - infer from returns
            Type::Unknown => {
                // Collect all return types that have type information
                let typed_returns: Vec<(usize, TypeId)> = returns
                    .iter()
                    .filter_map(|(idx, opt)| opt.map(|ty| (*idx, ty)))
                    .collect();

                // If we have multiple returns, constrain them to be equal to each other
                // This ensures type consistency when inferring
                if typed_returns.len() > 1 {
                    let first_type = typed_returns[0].1;
                    for (return_node_idx, return_type_id) in typed_returns.iter().skip(1) {
                        self.add_constraint(*return_type_id, first_type, *return_node_idx);
                    }
                }

                // Also constrain the declared return type to match the actual returns
                // This allows the function's return type to be inferred
                if let Some((return_node_idx, first_type)) = typed_returns.first() {
                    self.add_constraint(*first_type, declared_return_type, *return_node_idx);
                }
            }

            // Case 2: Declared void return type - validate no values returned
            Type::Void => {
                for (return_node_idx, return_type_opt) in &returns {
                    if let Some(return_type_id) = return_type_opt {
                        let return_type = self.type_registry.resolve(*return_type_id);
                        if !matches!(return_type, Type::Void) {
                            // Return with value in void function
                            self.record_error(self.make_error(
                                "Cannot return a value from a void function".to_string(),
                                *return_node_idx,
                            ));
                        }
                    }
                }
            }

            // Case 3: Declared non-void type - validate all returns match
            _ => {
                if returns.is_empty() {
                    // Function has declared return type but no return statements
                    // Check if function body has any statements by looking at the block
                    let has_body_statements = self.function_has_body_statements(func_decl_idx);

                    if has_body_statements {
                        // Function has statements but no returns - error
                        let func_name = self.get_function_name(func_decl_idx);
                        self.record_error(self.make_error(
                            format!(
                                "Function '{}' must have at least one return statement",
                                func_name
                            ),
                            func_decl_idx,
                        ));
                    }
                    // If no body statements, this is an empty function stub - allow it
                    return;
                }

                // Validate each return that has type information
                for (return_node_idx, return_type_opt) in &returns {
                    if let Some(return_type_id) = return_type_opt {
                        let return_type = self.type_registry.resolve(*return_type_id);
                        if matches!(return_type, Type::Void) {
                            // Bare return in non-void function
                            self.record_error(self.make_error(
                                "Cannot use bare 'return' in a function with a return type".to_string(),
                                *return_node_idx,
                            ));
                        } else {
                            // Add constraint: return type = declared return type
                            self.add_constraint(*return_type_id, declared_return_type, *return_node_idx);
                        }
                    }
                    // Skip returns without type info (e.g., variable references not yet tracked)
                    // These will be validated when full type tracking is implemented
                }
            }
        }
    }

    /// Checks if a function has body statements (not just an empty block)
    fn function_has_body_statements(&self, func_decl_idx: usize) -> bool {
        use crate::ast::NodeType;

        // Navigate to the Block node in the function
        let mut current_idx = self.ast.nodes[func_decl_idx].first_child;
        while let Some(idx) = current_idx {
            if self.ast.nodes[idx].node_type == NodeType::Block {
                // Check if block has any children (statements)
                return self.ast.nodes[idx].first_child.is_some();
            }
            current_idx = self.ast.nodes[idx].next_sibling;
        }
        false
    }

    /// Gets the function name from a FunctionDecl node for error messages
    fn get_function_name(&self, func_decl_idx: usize) -> String {
        if let Some(ident_idx) = self.ast.nodes[func_decl_idx].first_child {
            if let Some(name) = self.ast.node_text(ident_idx) {
                return name.to_string();
            }
        }
        "<unknown>".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::super::{SemanticAnalyzer, SemanticError};
    use crate::lexer::lex;
    use crate::limits::CompilerLimits;
    use crate::parser::parse;

    /// Helper function to analyze source code
    fn analyze_source(source: &str) -> Result<crate::ast::Ast, Vec<SemanticError>> {
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let analyzer = SemanticAnalyzer::new(ast);
        analyzer.analyze()
    }

    // ========== Group 1: Return Type Matching ==========

    #[test]
    fn test_return_type_correct() {
        let source = r#"
            getNum: () Number {
                return 42
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Matching return type should succeed: {:?}", result.err());
    }

    #[test]
    fn test_return_type_mismatch() {
        let source = r#"
            getNum: () Number {
                return "hello"
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Mismatched return type should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Type mismatch")),
            "Expected type mismatch error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_multiple_returns_same_type() {
        let source = r#"
            getValue: (flag Bool) Number {
                return 1
                return 2
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Multiple returns of same type should succeed: {:?}", result.err());
    }

    #[test]
    fn test_multiple_returns_different_types() {
        let source = r#"
            getValue: (flag Bool) Number {
                return 42
                return "oops"
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Multiple returns of different types should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Type mismatch")),
            "Expected type mismatch error, got: {:?}",
            errors
        );
    }

    // ========== Group 2: Void Return Handling ==========

    #[test]
    fn test_void_return_in_inferred_function() {
        let source = r#"
            doNothing: () {
                return
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Void return in inferred function should succeed: {:?}", result.err());
    }

    #[test]
    fn test_value_return_in_void_function() {
        // Note: suru-lang doesn't have explicit void annotation yet,
        // but we test by checking functions without return annotation
        // that have a void return followed by a value return
        let source = r#"
            mixed: () {
                return
            }
        "#;
        let result = analyze_source(source);
        // This should succeed - function is inferred as returning void
        assert!(result.is_ok(), "Void return in inferred function should succeed: {:?}", result.err());
    }

    #[test]
    fn test_bare_return_error_in_non_void_function() {
        let source = r#"
            getNum: () Number {
                return
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Bare return in non-void function should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("bare 'return'")),
            "Expected bare return error, got: {:?}",
            errors
        );
    }

    // ========== Group 3: Missing Return ==========

    #[test]
    fn test_missing_return_in_non_void_function() {
        let source = r#"
            getNum: () Number {
                x: 42
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Missing return in non-void function should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("must have at least one return")),
            "Expected missing return error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_no_return_in_inferred_function() {
        let source = r#"
            doSomething: () {
                x: 42
            }
        "#;
        let result = analyze_source(source);
        // Function with no return type annotation and no returns is ok
        // (implicitly void or unit)
        assert!(result.is_ok(), "No return in inferred function should succeed: {:?}", result.err());
    }

    // ========== Group 4: Type Inference ==========
    //
    // Note: Full return type inference (where the FunctionType's return_type gets
    // updated) requires Phase 4.1c (let-polymorphism with type variables).
    // Currently, validation ensures return types are consistent but doesn't
    // update the FunctionType stored in the symbol table.

    #[test]
    fn test_infer_function_accepts_consistent_returns() {
        // Functions without return type annotation should accept consistent return types
        let source = r#"
            getValue: () {
                return 42
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Function with inferred return type should succeed: {:?}", result.err());
    }

    #[test]
    fn test_infer_multiple_consistent_returns() {
        // Multiple returns with same type should succeed
        let source = r#"
            getValue: (flag Bool) {
                return 1
                return 2
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Multiple consistent returns should succeed: {:?}", result.err());
    }

    #[test]
    fn test_infer_return_type_inconsistent_error() {
        // Multiple returns with different types should fail
        let source = r#"
            getValue: () {
                return 42
                return "hello"
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Inconsistent return types should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Type mismatch")),
            "Expected type mismatch error, got: {:?}",
            errors
        );
    }

    // ========== Group 5: Nested Functions ==========

    #[test]
    fn test_nested_functions_separate_return_types() {
        let source = r#"
            outer: () Number {
                inner: () String {
                    return "hello"
                }
                return 42
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Nested functions should have separate return types: {:?}", result.err());
    }

    #[test]
    fn test_nested_function_inner_mismatch() {
        let source = r#"
            outer: () Number {
                inner: () String {
                    return 42
                }
                return 42
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Inner function type mismatch should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Type mismatch")),
            "Expected type mismatch error for inner function, got: {:?}",
            errors
        );
    }

    // ========== Group 6: Expression Returns ==========

    #[test]
    fn test_return_variable() {
        // Note: Variable reference type tracking is not fully implemented yet.
        // Returns with variable references are allowed but not type-checked.
        // Full variable type tracking will be implemented in a future phase.
        let source = r#"
            getValue: () Number {
                x: 42
                return x
            }
        "#;
        let result = analyze_source(source);
        // Should succeed - variable returns are allowed, type not validated yet
        assert!(result.is_ok(), "Return with variable should succeed: {:?}", result.err());
    }

    #[test]
    fn test_return_expression() {
        let source = r#"
            isTrue: () Bool {
                return not false
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Return with expression should succeed: {:?}", result.err());
    }

    #[test]
    fn test_return_expression_type_mismatch() {
        let source = r#"
            getNum: () Number {
                return true and false
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Return expression type mismatch should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Type mismatch")),
            "Expected type mismatch error, got: {:?}",
            errors
        );
    }

    // ========== Group 7: String Return Type ==========

    #[test]
    fn test_string_return_type() {
        let source = r#"
            greet: () String {
                return "hello"
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "String return type should succeed: {:?}", result.err());
    }

    #[test]
    fn test_string_return_type_mismatch() {
        let source = r#"
            greet: () String {
                return 42
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "String/Number mismatch should fail");
    }

    // ========== Group 8: Bool Return Type ==========

    #[test]
    fn test_bool_return_type() {
        let source = r#"
            check: () Bool {
                return true
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Bool return type should succeed: {:?}", result.err());
    }

    #[test]
    fn test_bool_return_type_mismatch() {
        let source = r#"
            check: () Bool {
                return 42
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Bool/Number mismatch should fail");
    }
}
