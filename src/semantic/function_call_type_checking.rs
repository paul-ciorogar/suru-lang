// Function call type checking implementation
//
// This module implements type checking for function calls:
// - Check argument count matches parameter count
// - Check argument types match parameter types
// - Determine call expression result type

use super::{SemanticAnalyzer, Type};

impl SemanticAnalyzer {
    /// Type checks a function call
    ///
    /// This method is called after visiting the function call's arguments
    /// to validate argument count and types against the function signature.
    ///
    /// # Cases Handled
    ///
    /// 1. **Argument count mismatch**: Reports error if argument count != parameter count
    ///
    /// 2. **Argument type mismatch**: Adds constraints for each argument type against
    ///    parameter type. Unification handles type error reporting.
    ///
    /// 3. **Unknown parameter types**: Skips type checking for parameters with
    ///    `Type::Unknown` (allows inference from actual arguments).
    ///
    /// 4. **Return type**: Sets the function call node's type to the function's
    ///    return type, enabling type propagation through expressions.
    ///
    /// # Arguments
    ///
    /// * `node_idx` - AST node index of the FunctionCall
    pub(super) fn type_check_function_call(&mut self, node_idx: usize) {
        // 1. Extract function name from first child (Identifier)
        let Some(ident_idx) = self.ast.nodes[node_idx].first_child else {
            return;
        };
        let Some(name) = self.ast.node_text(ident_idx) else {
            return;
        };

        // 2. Look up function and get FunctionType
        let Some(symbol) = self.scopes.lookup(name) else {
            return; // Already reported as undefined in visit_function_call
        };
        let Some(func_type_id) = symbol.type_id else {
            return; // No type info, skip validation
        };
        let func_type = self.type_registry.resolve(func_type_id).clone();
        let Type::Function(ft) = func_type else {
            return; // Not a function type
        };

        // 3. Get ArgList and count arguments
        let arg_list_idx = self.ast.nodes[ident_idx].next_sibling;
        let arg_count = self.count_call_arguments(arg_list_idx);

        // 4. Validate argument count
        if arg_count != ft.params.len() {
            self.record_error(self.make_error(
                format!(
                    "Function '{}' expects {} argument(s) but got {}",
                    name,
                    ft.params.len(),
                    arg_count
                ),
                node_idx,
            ));
        }

        // 5. Type check each argument against parameter type
        if let Some(arg_list_idx) = arg_list_idx {
            let mut arg_idx = self.ast.nodes[arg_list_idx].first_child;
            for param in ft.params.iter() {
                if let Some(current_arg_idx) = arg_idx {
                    let param_type = self.type_registry.resolve(param.type_id);
                    // Only add constraint if parameter has a known type
                    if !matches!(param_type, Type::Unknown) {
                        if let Some(arg_type) = self.get_node_type(current_arg_idx) {
                            self.add_constraint(arg_type, param.type_id, current_arg_idx);
                        }
                    }
                    arg_idx = self.ast.nodes[current_arg_idx].next_sibling;
                }
            }
        }

        // 6. Set return type on the call node
        self.set_node_type(node_idx, ft.return_type);
    }

    /// Counts the number of arguments in an ArgList node
    pub(super) fn count_call_arguments(&self, arg_list_idx: Option<usize>) -> usize {
        let Some(arg_list_idx) = arg_list_idx else {
            return 0;
        };
        let mut count = 0;
        let mut arg_idx = self.ast.nodes[arg_list_idx].first_child;
        while let Some(idx) = arg_idx {
            count += 1;
            arg_idx = self.ast.nodes[idx].next_sibling;
        }
        count
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

    // ========== Group 1: Correct Function Calls ==========

    #[test]
    fn test_function_call_correct_args() {
        let source = r#"
            add: (x Number, y Number) Number { return 1 }
            z: add(42, 99)
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Valid function call should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_function_call_no_args() {
        let source = r#"
            getNum: () Number { return 42 }
            z: getNum()
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Function call with no args should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_function_call_single_arg() {
        let source = r#"
            double: (x Number) Number { return 1 }
            z: double(21)
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Function call with single arg should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_function_call_string_arg() {
        let source = r#"
            greet: (name String) String { return "hi" }
            z: greet("Paul")
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Function call with string arg should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_function_call_bool_arg() {
        let source = r#"
            check: (flag Bool) Bool { return true }
            z: check(false)
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Function call with bool arg should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_function_call_mixed_arg_types() {
        let source = r#"
            format: (name String, age Number, active Bool) String { return "ok" }
            z: format("Paul", 30, true)
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Function call with mixed types should succeed: {:?}",
            result.err()
        );
    }

    // ========== Group 2: Argument Count Errors ==========

    #[test]
    fn test_function_call_too_few_args() {
        let source = r#"
            add: (x Number, y Number) Number { return 1 }
            z: add(42)
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Too few arguments should fail");
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("expects 2 argument(s) but got 1")),
            "Expected argument count error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_function_call_too_many_args() {
        let source = r#"
            add: (x Number, y Number) Number { return 1 }
            z: add(1, 2, 3)
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Too many arguments should fail");
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("expects 2 argument(s) but got 3")),
            "Expected argument count error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_function_call_args_when_none_expected() {
        let source = r#"
            getNum: () Number { return 42 }
            z: getNum(1)
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Args when none expected should fail");
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("expects 0 argument(s) but got 1")),
            "Expected argument count error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_function_call_no_args_when_expected() {
        let source = r#"
            double: (x Number) Number { return 1 }
            z: double()
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "No args when expected should fail");
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("expects 1 argument(s) but got 0")),
            "Expected argument count error, got: {:?}",
            errors
        );
    }

    // ========== Group 3: Argument Type Errors ==========

    #[test]
    fn test_function_call_wrong_arg_type() {
        let source = r#"
            double: (x Number) Number { return 1 }
            z: double("hello")
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Wrong argument type should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Type mismatch")),
            "Expected type mismatch error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_function_call_string_instead_of_number() {
        let source = r#"
            add: (x Number, y Number) Number { return 1 }
            z: add(42, "hello")
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "String instead of Number should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Type mismatch")),
            "Expected type mismatch error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_function_call_number_instead_of_string() {
        let source = r#"
            greet: (name String) String { return "hi" }
            z: greet(42)
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Number instead of String should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Type mismatch")),
            "Expected type mismatch error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_function_call_number_instead_of_bool() {
        let source = r#"
            check: (flag Bool) Bool { return true }
            z: check(42)
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Number instead of Bool should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Type mismatch")),
            "Expected type mismatch error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_function_call_multiple_type_errors() {
        let source = r#"
            format: (name String, age Number) String { return "ok" }
            z: format(42, "thirty")
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Multiple type errors should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Type mismatch")),
            "Expected type mismatch error, got: {:?}",
            errors
        );
    }

    // ========== Group 4: Expression Arguments ==========

    #[test]
    fn test_function_call_with_expression_arg() {
        let source = r#"
            check: (flag Bool) Bool { return flag }
            z: check(true and false)
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Expression argument should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_function_call_with_negation_arg() {
        let source = r#"
            double: (x Number) Number { return 1 }
            z: double(-42)
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Negation argument should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_function_call_with_not_arg() {
        let source = r#"
            check: (flag Bool) Bool { return flag }
            z: check(not true)
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Not expression argument should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_function_call_expression_type_mismatch() {
        let source = r#"
            double: (x Number) Number { return 1 }
            z: double(true and false)
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Bool expression for Number param should fail"
        );
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Type mismatch")),
            "Expected type mismatch error, got: {:?}",
            errors
        );
    }

    // ========== Group 5: Unknown Parameter Types ==========

    #[test]
    fn test_function_call_untyped_param() {
        let source = r#"
            identity: (x) { return x }
            z: identity(42)
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Call with untyped param should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_function_call_untyped_param_string() {
        let source = r#"
            identity: (x) { return x }
            z: identity("hello")
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Call with untyped param and string should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_function_call_mixed_typed_untyped() {
        let source = r#"
            process: (x Number, y) Number { return x }
            z: process(42, "anything")
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Call with mixed typed/untyped should succeed: {:?}",
            result.err()
        );
    }

    // ========== Group 6: Recursive Calls ==========
    // Note: Nested function calls like foo(bar(x)) are not supported by the parser yet

    #[test]
    fn test_recursive_function_call() {
        let source = r#"
            factorial: (n Number) Number {
                return factorial(1)
            }
            z: factorial(5)
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Recursive function call should succeed: {:?}",
            result.err()
        );
    }

    // ========== Group 7: Variable Arguments ==========

    #[test]
    fn test_function_call_with_variable_arg() {
        let source = r#"
            double: (x Number) Number { return 1 }
            n: 42
            z: double(n)
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Variable argument should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_function_call_variable_type_mismatch() {
        let source = r#"
            double: (x Number) Number { return 1 }
            s: "hello"
            z: double(s)
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Variable type mismatch should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Type mismatch")),
            "Expected type mismatch error, got: {:?}",
            errors
        );
    }

    // ========== Group 8: Multiple Calls ==========

    #[test]
    fn test_multiple_valid_calls() {
        let source = r#"
            add: (x Number, y Number) Number { return 1 }
            a: add(1, 2)
            b: add(3, 4)
            c: add(5, 6)
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Multiple valid calls should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_multiple_calls_one_invalid() {
        let source = r#"
            add: (x Number, y Number) Number { return 1 }
            a: add(1, 2)
            b: add("wrong", 4)
            c: add(5, 6)
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "One invalid call should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Type mismatch")),
            "Expected type mismatch error, got: {:?}",
            errors
        );
    }
}
