// Function body analysis implementation for Phase 5.2
//
// This module implements visitor methods for:
// - Return statement type tracking

use super::{SemanticAnalyzer, SemanticError, Type};

impl SemanticAnalyzer {
    /// Visits return statement
    ///
    /// Analyzes the return statement's expression (if any) and records
    /// the return type for the current function. This information is used
    /// in Phase 5.3 for return type validation.
    ///
    /// # Errors
    /// - Reports an error if return statement is outside a function
    pub(super) fn visit_return_stmt(&mut self, node_idx: usize) {
        // Check if we're inside a function
        if self.current_function().is_none() {
            // Get location info from the node's token
            let (line, column) = if let Some(ref token) = self.ast.nodes[node_idx].token {
                (token.line, token.column)
            } else {
                (0, 0)
            };
            self.record_error(SemanticError::new(
                "Return statement outside of function".to_string(),
                line,
                column,
            ));
            return;
        }

        // Check if there's a return expression
        let return_type_id = if let Some(expr_idx) = self.ast.nodes[node_idx].first_child {
            // Visit the return expression to infer its type
            self.visit_node(expr_idx);

            // Get the inferred type from the expression
            self.get_node_type(expr_idx)
        } else {
            // No return expression - this is a void return
            Some(self.type_registry.intern(Type::Void))
        };

        // Record the return for this function
        self.record_return(node_idx, return_type_id);

        // Set node type on the ReturnStmt node
        if let Some(type_id) = return_type_id {
            self.set_node_type(node_idx, type_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{SemanticAnalyzer, SemanticError, Type};
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

    /// Helper to get analyzer with visited AST (for inspecting internal state)
    fn get_analyzer_after_visit(source: &str) -> SemanticAnalyzer {
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let mut analyzer = SemanticAnalyzer::new(ast);

        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
        }

        analyzer
    }

    // ========== Group 1: Return Outside Function ==========

    #[test]
    fn test_return_outside_function_error() {
        let source = "return 42";
        let result = analyze_source(source);
        assert!(result.is_err(), "Return outside function should fail");
        let errors = result.unwrap_err();
        assert!(
            errors[0].message.contains("Return statement outside of function"),
            "Expected 'outside of function' error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_void_return_outside_function_error() {
        let source = "return";
        let result = analyze_source(source);
        assert!(result.is_err(), "Void return outside function should fail");
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("Return statement outside of function"));
    }

    // ========== Group 2: Return Inside Function ==========

    #[test]
    fn test_return_in_function_succeeds() {
        let source = r#"
            foo: () {
                return 42
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Return in function should succeed: {:?}", result.err());
    }

    #[test]
    fn test_void_return_in_function_succeeds() {
        let source = r#"
            foo: () {
                return
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Void return in function should succeed: {:?}", result.err());
    }

    #[test]
    fn test_return_with_expression_succeeds() {
        let source = r#"
            add: (x Number, y Number) Number {
                return x
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Return with expression should succeed: {:?}", result.err());
    }

    // ========== Group 3: Return Type Tracking ==========

    #[test]
    fn test_return_type_recorded_for_function() {
        let source = r#"
            foo: () {
                return 42
            }
        "#;
        let analyzer = get_analyzer_after_visit(source);

        // Find the function declaration node
        let root_idx = analyzer.ast.root.unwrap();
        let func_idx = analyzer.ast.nodes[root_idx].first_child.unwrap();

        // Check that returns were recorded
        let returns = analyzer.get_function_returns(func_idx);
        assert!(returns.is_some(), "Returns should be recorded for function");
        let returns = returns.unwrap();
        assert_eq!(returns.len(), 1, "Should have one return statement");

        // Check the return type is Number
        let (_, return_type) = returns[0];
        assert!(return_type.is_some(), "Return type should be recorded");
        let ty = analyzer.type_registry.resolve(return_type.unwrap());
        assert_eq!(ty, &Type::Number, "Return type should be Number");
    }

    #[test]
    fn test_void_return_type_recorded() {
        let source = r#"
            foo: () {
                return
            }
        "#;
        let analyzer = get_analyzer_after_visit(source);

        // Find the function declaration node
        let root_idx = analyzer.ast.root.unwrap();
        let func_idx = analyzer.ast.nodes[root_idx].first_child.unwrap();

        // Check that returns were recorded
        let returns = analyzer.get_function_returns(func_idx);
        assert!(returns.is_some(), "Returns should be recorded for function");
        let returns = returns.unwrap();
        assert_eq!(returns.len(), 1, "Should have one return statement");

        // Check the return type is Void
        let (_, return_type) = returns[0];
        assert!(return_type.is_some(), "Return type should be recorded");
        let ty = analyzer.type_registry.resolve(return_type.unwrap());
        assert_eq!(ty, &Type::Void, "Return type should be Void");
    }

    #[test]
    fn test_multiple_returns_recorded() {
        let source = r#"
            foo: (x Bool) Number {
                return 1
                return 2
            }
        "#;
        let analyzer = get_analyzer_after_visit(source);

        // Find the function declaration node
        let root_idx = analyzer.ast.root.unwrap();
        let func_idx = analyzer.ast.nodes[root_idx].first_child.unwrap();

        // Check that multiple returns were recorded
        let returns = analyzer.get_function_returns(func_idx);
        assert!(returns.is_some(), "Returns should be recorded for function");
        let returns = returns.unwrap();
        assert_eq!(returns.len(), 2, "Should have two return statements");
    }

    // ========== Group 4: Nested Functions ==========

    #[test]
    fn test_nested_function_returns_tracked_separately() {
        let source = r#"
            outer: () {
                inner: () {
                    return "hello"
                }
                return 42
            }
        "#;
        let analyzer = get_analyzer_after_visit(source);

        // Find the outer function declaration node
        let root_idx = analyzer.ast.root.unwrap();
        let outer_func_idx = analyzer.ast.nodes[root_idx].first_child.unwrap();

        // Check outer function returns
        let outer_returns = analyzer.get_function_returns(outer_func_idx);
        assert!(outer_returns.is_some(), "Outer function returns should be recorded");
        let outer_returns = outer_returns.unwrap();
        assert_eq!(outer_returns.len(), 1, "Outer should have one return");

        // Check the return type is Number
        let (_, return_type) = outer_returns[0];
        assert!(return_type.is_some());
        let ty = analyzer.type_registry.resolve(return_type.unwrap());
        assert_eq!(ty, &Type::Number, "Outer return should be Number");
    }

    // ========== Group 5: Return Expression Types ==========

    #[test]
    fn test_return_string_type() {
        let source = r#"
            greet: () {
                return "hello"
            }
        "#;
        let analyzer = get_analyzer_after_visit(source);

        let root_idx = analyzer.ast.root.unwrap();
        let func_idx = analyzer.ast.nodes[root_idx].first_child.unwrap();

        let returns = analyzer.get_function_returns(func_idx).unwrap();
        let (_, return_type) = returns[0];
        let ty = analyzer.type_registry.resolve(return_type.unwrap());
        assert_eq!(ty, &Type::String, "Return type should be String");
    }

    #[test]
    fn test_return_bool_type() {
        let source = r#"
            check: () {
                return true
            }
        "#;
        let analyzer = get_analyzer_after_visit(source);

        let root_idx = analyzer.ast.root.unwrap();
        let func_idx = analyzer.ast.nodes[root_idx].first_child.unwrap();

        let returns = analyzer.get_function_returns(func_idx).unwrap();
        let (_, return_type) = returns[0];
        let ty = analyzer.type_registry.resolve(return_type.unwrap());
        assert_eq!(ty, &Type::Bool, "Return type should be Bool");
    }

    #[test]
    fn test_return_expression_type() {
        let source = r#"
            compute: () {
                return not false
            }
        "#;
        let analyzer = get_analyzer_after_visit(source);

        let root_idx = analyzer.ast.root.unwrap();
        let func_idx = analyzer.ast.nodes[root_idx].first_child.unwrap();

        let returns = analyzer.get_function_returns(func_idx).unwrap();
        let (_, return_type) = returns[0];
        let ty = analyzer.type_registry.resolve(return_type.unwrap());
        assert_eq!(ty, &Type::Bool, "Return type should be Bool");
    }

    // ========== Group 6: Return Node Type Set ==========

    #[test]
    fn test_return_node_has_type() {
        let source = r#"
            foo: () {
                return 42
            }
        "#;
        let analyzer = get_analyzer_after_visit(source);

        // Find the return statement node
        let root_idx = analyzer.ast.root.unwrap();
        let func_idx = analyzer.ast.nodes[root_idx].first_child.unwrap();
        let returns = analyzer.get_function_returns(func_idx).unwrap();
        let (return_node_idx, _) = returns[0];

        // Check that the return node has a type set
        let return_type = analyzer.get_node_type(return_node_idx);
        assert!(return_type.is_some(), "Return node should have type set");
        let ty = analyzer.type_registry.resolve(return_type.unwrap());
        assert_eq!(ty, &Type::Number);
    }

    #[test]
    fn test_void_return_node_has_void_type() {
        let source = r#"
            foo: () {
                return
            }
        "#;
        let analyzer = get_analyzer_after_visit(source);

        // Find the return statement node
        let root_idx = analyzer.ast.root.unwrap();
        let func_idx = analyzer.ast.nodes[root_idx].first_child.unwrap();
        let returns = analyzer.get_function_returns(func_idx).unwrap();
        let (return_node_idx, _) = returns[0];

        // Check that the return node has Void type set
        let return_type = analyzer.get_node_type(return_node_idx);
        assert!(return_type.is_some(), "Void return node should have type set");
        let ty = analyzer.type_registry.resolve(return_type.unwrap());
        assert_eq!(ty, &Type::Void);
    }

    // ========== Group 7: Function Without Returns ==========

    #[test]
    fn test_function_without_returns_has_empty_vec() {
        let source = r#"
            foo: () {
                x: 42
            }
        "#;
        let analyzer = get_analyzer_after_visit(source);

        let root_idx = analyzer.ast.root.unwrap();
        let func_idx = analyzer.ast.nodes[root_idx].first_child.unwrap();

        let returns = analyzer.get_function_returns(func_idx);
        assert!(returns.is_some(), "Function should have returns entry");
        let returns = returns.unwrap();
        assert!(returns.is_empty(), "Function without returns should have empty vec");
    }

    // ========== Group 8: Return with Variable Reference ==========

    #[test]
    fn test_return_variable_reference() {
        let source = r#"
            foo: () {
                x: 42
                return x
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Return with variable reference should succeed: {:?}", result.err());
    }

    #[test]
    fn test_return_undefined_variable_error() {
        let source = r#"
            foo: () {
                return undefined_var
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Return with undefined variable should fail");
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("not defined"));
    }
}
