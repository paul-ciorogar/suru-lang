// Pipe operator type checking
//
// Implements semantic analysis for the pipe operator |:
//   value | identifier       → identifier(value)       (bare fn, 1 param)
//   value | func(_, arg)     → func(value, arg)         (_ marks injection point)
//   value | func()           → (func())(value)          (no _ = call returns a function)

use super::{SemanticAnalyzer, SymbolKind, Type, TypeId};
use crate::ast::NodeType;

impl SemanticAnalyzer {
    /// Visits a Pipe node and type-checks it.
    ///
    /// Dispatches based on the right-hand side node type:
    /// - `Identifier`   → bare function name; piped value is the sole argument
    /// - `FunctionCall` → normal call checking, then placeholder constraint
    /// - other          → visit and propagate type (future extensibility)
    pub(super) fn visit_pipe(&mut self, node_idx: usize) {
        // Extract left (first_child) and right (first_child.next_sibling)
        let Some(left_idx) = self.ast.nodes[node_idx].first_child else {
            return;
        };
        let Some(right_idx) = self.ast.nodes[left_idx].next_sibling else {
            return;
        };

        // Visit left side to infer its type
        self.visit_node(left_idx);

        // Check left side is not Void and capture its type
        let piped_type = self.check_pipe_left_not_void(node_idx, left_idx);

        match self.ast.nodes[right_idx].node_type {
            NodeType::Identifier   => self.pipe_into_identifier(node_idx, right_idx, piped_type),
            NodeType::FunctionCall => self.pipe_into_function_call(node_idx, right_idx, piped_type),
            _                      => self.pipe_into_other(node_idx, right_idx),
        }
    }

    /// Checks that the left side of a pipe produces a value (not Void).
    ///
    /// Returns `Some(type_id)` when safe to proceed, `None` when Void (error recorded).
    fn check_pipe_left_not_void(&mut self, pipe_idx: usize, left_idx: usize) -> Option<TypeId> {
        let left_type_id = self.get_node_type(left_idx)?;
        let left_type = self.type_registry.resolve(left_type_id).clone();
        match left_type {
            Type::Void => {
                self.record_error(self.make_error(
                    "Pipe operator: left side produces no value (Void)".to_string(),
                    pipe_idx,
                ));
                None
            }
            // Type::Var / Type::Unknown — can't rule out at constraint-collection time
            _ => Some(left_type_id),
        }
    }

    /// Handles `value | identifier` — bare function reference as pipe target.
    ///
    /// 1. Visits the identifier (symbol resolution, sets identifier's type)
    /// 2. Looks up the function type
    /// 3. Validates it takes exactly 1 parameter
    /// 4. Constrains piped_type → first param type
    /// 5. Sets pipe node type to the function's return type
    fn pipe_into_identifier(
        &mut self,
        pipe_idx: usize,
        ident_idx: usize,
        piped_type: Option<TypeId>,
    ) {
        // 1. Visit identifier (resolves name, may record "not defined" error)
        self.visit_node(ident_idx);

        // 2. Get function name
        let Some(name) = self.ast.node_text(ident_idx) else {
            return;
        };
        let name = name.to_string();

        // 3. Look up symbol — if None, visit_identifier already reported the error
        let symbol_info = self.scopes.lookup(&name).map(|s| (s.kind, s.type_id));
        let Some((kind, Some(type_id))) = symbol_info else {
            return;
        };

        // 4. Validate it's a function
        if kind != SymbolKind::Function {
            self.record_error(self.make_error(
                format!("Pipe: '{}' is not callable", name),
                ident_idx,
            ));
            return;
        }

        let func_type = self.type_registry.resolve(type_id).clone();
        let Type::Function(ft) = func_type else {
            self.record_error(self.make_error(
                format!("Pipe: '{}' is not callable", name),
                ident_idx,
            ));
            return;
        };

        // 5. Validate exactly 1 parameter
        if ft.params.len() != 1 {
            self.record_error(self.make_error(
                format!(
                    "Pipe: function '{}' expects {} argument(s), but pipe provides 1",
                    name,
                    ft.params.len()
                ),
                ident_idx,
            ));
            // Set return type for error recovery so downstream type checks keep going
            self.set_node_type(pipe_idx, ft.return_type);
            return;
        }

        // 6. Constrain piped_type → first param type (skip if param is Unknown)
        if let Some(piped) = piped_type {
            let param_type = self.type_registry.resolve(ft.params[0].type_id).clone();
            if !matches!(param_type, Type::Unknown) {
                self.add_constraint(piped, ft.params[0].type_id, ident_idx);
            }
        }

        // 7. Set pipe return type
        self.set_node_type(pipe_idx, ft.return_type);
    }

    /// Returns `true` if the given `FunctionCall` node has a `_` (Placeholder) in its arg list.
    fn call_has_placeholder(&self, call_idx: usize) -> bool {
        let Some(ident_idx) = self.ast.nodes[call_idx].first_child else {
            return false;
        };
        let Some(arg_list_idx) = self.ast.nodes[ident_idx].next_sibling else {
            return false;
        };
        self.ast
            .children(arg_list_idx)
            .any(|child_idx| self.ast.nodes[child_idx].node_type == NodeType::Placeholder)
    }

    fn pipe_into_function_call(
        &mut self,
        pipe_idx: usize,
        call_idx: usize,
        piped_type: Option<TypeId>,
    ) {
        let has_placeholder = self.call_has_placeholder(call_idx);
        // Normal function call checking (validates declared arg count vs params)
        self.visit_node(call_idx);
        if has_placeholder {
            self.pipe_into_call_with_placeholder(pipe_idx, call_idx, piped_type);
        } else {
            self.pipe_into_call_without_placeholder(pipe_idx, call_idx, piped_type);
        }
    }

    /// Form 2: `value | func(_, arg)` — inject piped value at `_` placeholder position.
    fn pipe_into_call_with_placeholder(
        &mut self,
        pipe_idx: usize,
        call_idx: usize,
        piped_type: Option<TypeId>,
    ) {
        if let Some(piped) = piped_type {
            self.pipe_constrain_placeholder(call_idx, piped);
        }
        if let Some(call_type) = self.get_node_type(call_idx) {
            self.set_node_type(pipe_idx, call_type);
        }
    }

    /// Form 3: `value | func()` — call's return value must itself be a function,
    /// which is then called with the piped value: `(func())(value)`.
    fn pipe_into_call_without_placeholder(
        &mut self,
        pipe_idx: usize,
        call_idx: usize,
        piped_type: Option<TypeId>,
    ) {
        let Some(call_type) = self.get_node_type(call_idx) else {
            return;
        };
        let resolved = self.type_registry.resolve(call_type).clone();
        match resolved {
            Type::Function(ft) => {
                // Constrain piped value to function's first param
                if let (Some(piped), Some(first_param)) = (piped_type, ft.params.first()) {
                    let param_type = self.type_registry.resolve(first_param.type_id).clone();
                    if !matches!(param_type, Type::Unknown) {
                        self.add_constraint(piped, first_param.type_id, call_idx);
                    }
                }
                self.set_node_type(pipe_idx, ft.return_type);
            }
            // Cannot determine yet (type var / unknown) — defer
            Type::Var(_) | Type::Unknown => self.set_node_type(pipe_idx, call_type),
            _ => self.record_error(self.make_error(
                "Pipe operator: right-hand side call must return a function; \
                 use _ to inject the piped value into the call directly"
                    .to_string(),
                call_idx,
            )),
        }
    }

    /// Fallback arm: visit RHS and propagate its type to the pipe node.
    fn pipe_into_other(&mut self, pipe_idx: usize, right_idx: usize) {
        self.visit_node(right_idx);
        if let Some(right_type) = self.get_node_type(right_idx) {
            self.set_node_type(pipe_idx, right_type);
        }
    }

    /// Adds a type constraint for the piped value at the `_` placeholder position.
    ///
    /// Called after `visit_node(call_idx)` so the function is already resolved.
    /// If no `_` placeholder is present, the piped value is not injected (no-op).
    fn pipe_constrain_placeholder(&mut self, call_idx: usize, piped_type: TypeId) {
        // 1. Get function identifier (first child of FunctionCall)
        let Some(ident_idx) = self.ast.nodes[call_idx].first_child else {
            return;
        };
        let Some(name) = self.ast.node_text(ident_idx) else {
            return;
        };
        let name = name.to_string();

        // 2. Get ArgList (sibling after identifier)
        let Some(arg_list_idx) = self.ast.nodes[ident_idx].next_sibling else {
            return;
        };

        // 3. Find first Placeholder child in ArgList and its position
        let children: Vec<usize> = self.ast.children(arg_list_idx).collect();
        let mut placeholder_info: Option<(usize, usize)> = None;
        for (pos, child_idx) in children.iter().copied().enumerate() {
            if self.ast.nodes[child_idx].node_type == NodeType::Placeholder {
                placeholder_info = Some((pos, child_idx));
                break;
            }
        }

        let Some((position, placeholder_idx)) = placeholder_info else {
            return; // No placeholder → piped value not injected
        };

        // 4. Look up function type
        let Some(Some(func_type_id)) = self.scopes.lookup(&name).map(|s| s.type_id) else {
            return;
        };

        let func_type = self.type_registry.resolve(func_type_id).clone();
        let Type::Function(ft) = func_type else {
            return;
        };

        // 5. Add constraint if the position is within the param list
        if position < ft.params.len() {
            let param_type = self.type_registry.resolve(ft.params[position].type_id).clone();
            if !matches!(param_type, Type::Unknown) {
                self.add_constraint(piped_type, ft.params[position].type_id, placeholder_idx);
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

    // ========== Group 1: Bare identifier RHS ==========

    #[test]
    fn test_pipe_bare_identifier_ok() {
        // value | identifier where identifier expects 1 arg of matching type
        let source = r#"
            double: (n Number) Number { return 1 }
            more: 42 | double
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "42 | double should succeed: {:?}", result.err());
    }

    #[test]
    fn test_pipe_bare_identifier_type_mismatch() {
        // Piped value type doesn't match the function's parameter type
        let source = r#"
            double: (n Number) Number { return 1 }
            more: "hello" | double
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "String piped to Number param should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Type mismatch")),
            "Expected type mismatch error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_pipe_bare_identifier_arity_mismatch() {
        // Function expects 2 args, pipe provides 1
        let source = r#"
            add: (x Number, y Number) Number { return 1 }
            more: 42 | add
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Piping into 2-arg function should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| {
                e.message.contains("expects 2 argument(s), but pipe provides 1")
            }),
            "Expected arity error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_pipe_bare_identifier_return_type_propagates() {
        // Result type of the pipe should be the function's return type (String)
        let source = r#"
            numToStr: (n Number) String { return "hi" }
            more: 42 | numToStr
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "42 | numToStr should succeed: {:?}",
            result.err()
        );
    }

    // ========== Group 2: Placeholder in FunctionCall RHS ==========

    #[test]
    fn test_pipe_call_placeholder_first_arg_ok() {
        // value | sub(_, 5) — piped value at position 0
        let source = r#"
            sub: (x Number, y Number) Number { return 1 }
            more: 10 | sub(_, 5)
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "10 | sub(_, 5) should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_pipe_call_placeholder_second_arg_ok() {
        // value | sub(20, _) — piped value at position 1
        let source = r#"
            sub: (x Number, y Number) Number { return 1 }
            more: 10 | sub(20, _)
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "10 | sub(20, _) should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_pipe_call_placeholder_type_mismatch() {
        // String piped into Number param position via _
        let source = r#"
            sub: (x Number, y Number) Number { return 1 }
            more: "hello" | sub(_, 5)
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "String piped into Number placeholder should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Type mismatch")),
            "Expected type mismatch error, got: {:?}",
            errors
        );
    }

    // ========== Group 3: FunctionCall without placeholder (piped value NOT injected) ==========

    #[test]
    fn test_pipe_call_no_placeholder_returns_non_function_err() {
        // zero() returns Number (not a function), so 42 | zero() is invalid.
        // 42 | zero() means (zero())(42), but Number is not callable.
        let source = r#"
            zero: () Number { return 0 }
            more: 42 | zero()
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "42 | zero() should fail: zero() returns Number, not a function"
        );
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("must return a function")),
            "Expected pipe return-type error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_pipe_call_no_placeholder_arg_count_error() {
        // value | double() — no placeholder, double expects 1 arg but gets 0
        let source = r#"
            double: (n Number) Number { return 1 }
            more: 42 | double()
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "42 | double() should fail (0 args, 1 expected)");
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("expects 1 argument(s) but got 0")),
            "Expected argument count error, got: {:?}",
            errors
        );
    }

    // ========== Group 4: Pipe chaining ==========

    #[test]
    fn test_pipe_chain_ok() {
        // 10 | double | numToStr — type propagates Number → Number → String
        let source = r#"
            double: (n Number) Number { return 1 }
            numToStr: (n Number) String { return "hi" }
            result: 10 | double | numToStr
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "10 | double | numToStr should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_pipe_chain_type_mismatch() {
        // "hello" | double — String piped into Number param (first pipe fails)
        let source = r#"
            double: (n Number) Number { return 1 }
            numToStr: (n Number) String { return "hi" }
            result: "hello" | double | numToStr
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "String | double should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Type mismatch")),
            "Expected type mismatch error, got: {:?}",
            errors
        );
    }

    // ========== Group 5: Result type used in annotation ==========

    #[test]
    fn test_pipe_result_type_annotation_ok() {
        // result Number: 10 | double — annotation matches return type
        let source = r#"
            double: (n Number) Number { return 1 }
            result Number: 10 | double
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "result Number: 10 | double should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_pipe_result_type_annotation_mismatch() {
        // result String: 10 | double — double returns Number, annotation says String
        let source = r#"
            double: (n Number) Number { return 1 }
            result String: 10 | double
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "String annotation on Number return should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Type mismatch")),
            "Expected type mismatch error, got: {:?}",
            errors
        );
    }

    // ========== Group 6: Undefined function ==========

    #[test]
    fn test_pipe_undefined_function() {
        // Bare identifier that doesn't exist in scope
        let source = r#"
            result: 42 | nonexistent
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Piping to undefined function should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("not defined")),
            "Expected 'not defined' error, got: {:?}",
            errors
        );
    }
}
