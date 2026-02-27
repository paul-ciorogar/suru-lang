// Try operator type checking
//
// Implements semantic analysis for the `try` keyword:
//   try expr       → unwraps the success variant; propagates failure variant on short-circuit
//
// Works with:
//   Union<A, B>       → success is A, failure is B
//   Result(ok, err)   → success is ok, failure is err
//   Option(inner)     → success is inner, failure is best-effort
//   Var / Unknown     → defer (fresh type var; constraint-based inference handles it later)
//   anything else     → error: "try operator requires a 2-variant union type"

use super::{SemanticAnalyzer, Type, TypeId};

impl SemanticAnalyzer {
    /// Visits a Try node and type-checks it.
    ///
    /// 1. Visits the single child (operand) to infer its type.
    /// 2. Resolves the operand type and extracts success/failure variants.
    /// 3. Sets the Try node's type to the success variant.
    /// 4. Validates that the containing function's return type is compatible
    ///    with the failure variant.
    pub(super) fn visit_try(&mut self, node_idx: usize) {
        // Get the single child (operand expression)
        let Some(operand_idx) = self.ast.nodes[node_idx].first_child else {
            return;
        };

        // Visit operand to infer its type
        self.visit_node(operand_idx);

        // Get operand type; if unknown, assign a fresh type variable and defer
        let Some(operand_type_id) = self.get_node_type(operand_idx) else {
            let tv = self.fresh_type_var();
            self.set_node_type(node_idx, tv);
            return;
        };

        // Resolve operand type and dispatch
        let resolved = self.type_registry.resolve(operand_type_id).clone();

        match resolved {
            // Result(ok, err) → success is ok, failure is err
            Type::Result(ok_type, err_type) => {
                self.set_node_type(node_idx, ok_type);
                self.check_try_return_type_compatible(node_idx, err_type);
            }

            // Option(inner) → success is inner; failure is best-effort
            Type::Option(inner_type) => {
                self.set_node_type(node_idx, inner_type);
                self.check_try_return_type_compatible(node_idx, inner_type);
            }

            // Union with exactly 2 variants → success is v[0], failure is v[1]
            // Union with != 2 variants → error
            Type::Union(variants) => {
                let count = variants.len();
                if count == 2 {
                    let success_type = variants[0];
                    let failure_type = variants[1];
                    self.set_node_type(node_idx, success_type);
                    self.check_try_return_type_compatible(node_idx, failure_type);
                } else {
                    self.record_error(self.make_error(
                        format!(
                            "try requires a 2-variant union, found {} variants",
                            count
                        ),
                        node_idx,
                    ));
                    let tv = self.fresh_type_var();
                    self.set_node_type(node_idx, tv);
                }
            }

            // Type variable / Unknown — cannot check at constraint-collection time; defer
            Type::Var(_) | Type::Unknown => {
                let tv = self.fresh_type_var();
                self.set_node_type(node_idx, tv);
            }

            // Any other concrete type → error
            _ => {
                self.record_error(self.make_error(
                    "try operator requires a 2-variant union type (e.g., Result, Option)"
                        .to_string(),
                    node_idx,
                ));
                let tv = self.fresh_type_var();
                self.set_node_type(node_idx, tv);
            }
        }
    }

    /// Validates that the containing function's return type is compatible with
    /// the failure type that `try` would propagate on short-circuit.
    ///
    /// Records an error if:
    /// - `try` is used outside of a function
    /// - the containing function's return type is not a 2-variant union
    fn check_try_return_type_compatible(&mut self, try_idx: usize, failure_type_id: TypeId) {
        // Must be inside a function
        let Some(func_decl_idx) = self.current_function() else {
            self.record_error(self.make_error(
                "try operator: cannot be used outside of a function".to_string(),
                try_idx,
            ));
            return;
        };

        // Get function name to look it up in scope
        let func_name = match self.ast.function_decl(func_decl_idx).name() {
            Some(n) => n.to_string(),
            None => return,
        };

        // Look up function's TypeId from scope
        let func_type_id = match self.scopes.lookup(&func_name).and_then(|s| s.type_id) {
            Some(id) => id,
            None => return,
        };

        // Resolve to FunctionType and extract return type
        let func_type = self.type_registry.resolve(func_type_id).clone();
        let Type::Function(ft) = func_type else {
            return;
        };
        let return_type_id = ft.return_type;
        let return_type = self.type_registry.resolve(return_type_id).clone();

        match return_type {
            // Result(_, fn_err): constrain failure ~ fn_err
            Type::Result(_, fn_err) => {
                self.add_constraint(failure_type_id, fn_err, try_idx);
            }

            // Option(_): best-effort — try is Option-compatible within an Option-returning function
            Type::Option(_) => {
                // No constraint needed; any Option return is compatible
            }

            // Union with exactly 2 variants: constrain failure ~ second variant
            Type::Union(variants) if variants.len() == 2 => {
                let fn_failure_type = variants[1];
                self.add_constraint(failure_type_id, fn_failure_type, try_idx);
            }

            // Type variable / Unknown: defer — inference will resolve it later
            Type::Var(_) | Type::Unknown => {}

            // Any other type (including Union != 2 variants): error
            _ => {
                self.record_error(self.make_error(
                    "try operator: containing function must return a 2-variant union type"
                        .to_string(),
                    try_idx,
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{SemanticAnalyzer, SemanticError};
    use crate::lexer::lex;
    use crate::limits::CompilerLimits;
    use crate::parser::parse;

    fn analyze_source(source: &str) -> Result<crate::ast::Ast, Vec<SemanticError>> {
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let analyzer = SemanticAnalyzer::new(ast);
        analyzer.analyze()
    }

    // ========== Group 1: Happy path ==========

    #[test]
    fn test_try_result_ok() {
        // Simulates Result<Number, String> via a 2-variant union
        let source = r#"
            type Ok
            type Err
            type MyResult: Ok, Err
            getResult: () MyResult { return Ok }
            process: () MyResult {
                value: try getResult()
                return Ok
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "try on 2-variant union should succeed: {:?}", result.err());
    }

    #[test]
    fn test_try_option_ok() {
        // Simulates Option<Number> via a 2-variant union
        let source = r#"
            type Some
            type None
            type MyOption: Some, None
            findValue: () MyOption { return Some }
            process: () MyOption {
                value: try findValue()
                return Some
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "try on Option-like union should succeed: {:?}", result.err());
    }

    #[test]
    fn test_try_2variant_union_ok() {
        // Custom 2-variant union used as both return type and operand type
        let source = r#"
            type Success
            type Failure
            type MyResult: Success, Failure
            makeResult: () MyResult { return Success }
            process: () MyResult {
                value: try makeResult()
                return Success
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "try on custom 2-variant union should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_try_type_propagates() {
        // try on MyResult unwraps to Ok (first variant); annotation Ok matches
        let source = r#"
            type Ok
            type Err
            type MyResult: Ok, Err
            getResult: () MyResult { return Ok }
            process: () MyResult {
                value Ok: try getResult()
                return Ok
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "try type should propagate as success variant (Ok): {:?}",
            result.err()
        );
    }

    #[test]
    fn test_try_chained() {
        // Two try calls in the same function body
        let source = r#"
            type Ok
            type Err
            type MyResult: Ok, Err
            step1: () MyResult { return Ok }
            step2: () MyResult { return Ok }
            process: () MyResult {
                a: try step1()
                b: try step2()
                return Ok
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "two chained try calls should succeed: {:?}",
            result.err()
        );
    }

    // ========== Group 2: Error cases — wrong operand type ==========

    #[test]
    fn test_try_on_number() {
        let source = r#"
            getValue: () Number { return 42 }
            process: () Number {
                value: try getValue()
                return 42
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "try on Number should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("try operator requires a 2-variant union type")),
            "Expected 'try operator requires a 2-variant union type', got: {:?}",
            errors
        );
    }

    #[test]
    fn test_try_on_string() {
        let source = r#"
            getValue: () String { return "hello" }
            process: () String {
                value: try getValue()
                return "hi"
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "try on String should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("try operator requires a 2-variant union type")),
            "Expected 'try operator requires a 2-variant union type', got: {:?}",
            errors
        );
    }

    #[test]
    fn test_try_on_bool() {
        let source = r#"
            getValue: () Bool { return true }
            process: () Bool {
                value: try getValue()
                return true
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "try on Bool should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("try operator requires a 2-variant union type")),
            "Expected 'try operator requires a 2-variant union type', got: {:?}",
            errors
        );
    }

    #[test]
    fn test_try_on_3variant_union() {
        let source = r#"
            type A
            type B
            type C
            type Tri: A, B, C
            getTri: () Tri { return A }
            process: () Tri {
                value: try getTri()
                return A
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "try on 3-variant union should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("try requires a 2-variant union, found 3 variants")),
            "Expected '3 variants' error, got: {:?}",
            errors
        );
    }

    // ========== Group 3: Error cases — context / return type ==========

    #[test]
    fn test_try_outside_function() {
        // try at global scope (not inside any function)
        let source = r#"
            type Ok
            type Err
            type MyResult: Ok, Err
            getResult: () MyResult { return Ok }
            result: try getResult()
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "try outside a function should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("cannot be used outside of a function")),
            "Expected 'cannot be used outside of a function', got: {:?}",
            errors
        );
    }

    #[test]
    fn test_try_return_type_not_union() {
        // Containing function returns Number (not a 2-variant union)
        let source = r#"
            type Ok
            type Err
            type MyResult: Ok, Err
            getResult: () MyResult { return Ok }
            process: () Number {
                value: try getResult()
                return 42
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "try in function returning Number should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| {
                e.message.contains("containing function must return a 2-variant union type")
            }),
            "Expected 'containing function must return a 2-variant union type', got: {:?}",
            errors
        );
    }
}
