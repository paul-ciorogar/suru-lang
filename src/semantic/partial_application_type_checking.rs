// Partial application type checking
//
// Implements semantic analysis for two partial application forms:
//
//   partial func              → bare identifier; propagates function type unchanged
//   partial func()            → no args; same type as func
//   partial func(arg)         → apply first arg; remaining params become new function type
//   partial func(_, arg)      → _ marks unfilled positions; stays in remaining params
//   partial func(arg1, arg2)  → all args applied; new 0-param function type
//
// Standalone `_` (e.g., `x: _`) is invalid and triggers an error.

use super::{SemanticAnalyzer, SymbolKind, Type, TypeId};
use crate::ast::NodeType;
use crate::semantic::types::{FunctionParam, FunctionType};

impl SemanticAnalyzer {
    /// Visits a Placeholder node (`_`).
    ///
    /// Valid only as a direct child of ArgList or MatchPattern.
    /// Any other position (e.g., `x: _`) records an error.
    pub(super) fn visit_placeholder(&mut self, node_idx: usize) {
        // Validate parent context
        let valid = match self.ast.nodes[node_idx].parent {
            Some(parent_idx) => {
                let parent_type = self.ast.nodes[parent_idx].node_type;
                matches!(parent_type, NodeType::ArgList | NodeType::MatchPattern)
            }
            None => false,
        };

        if !valid {
            self.record_error(self.make_error(
                "Placeholder '_' can only appear as a function argument or match pattern"
                    .to_string(),
                node_idx,
            ));
        }

        // Assign a fresh type variable so type inference can still propagate
        let tv = self.fresh_type_var();
        self.set_node_type(node_idx, tv);
    }

    /// Visits a Partial node (`partial <operand>`).
    ///
    /// Dispatches based on operand node type:
    /// - `Identifier`   → `partial_from_identifier` (bare function reference)
    /// - `FunctionCall` → `partial_from_call` (partially applied call)
    /// - other          → visit operand and propagate its type (deferred)
    pub(super) fn visit_partial(&mut self, node_idx: usize) {
        let Some(operand_idx) = self.ast.nodes[node_idx].first_child else {
            return;
        };

        match self.ast.nodes[operand_idx].node_type {
            NodeType::Identifier => self.partial_from_identifier(node_idx, operand_idx),
            NodeType::FunctionCall => self.partial_from_call(node_idx, operand_idx),
            _ => {
                // Unknown operand form — visit it and propagate whatever type it has
                self.visit_node(operand_idx);
                if let Some(ty) = self.get_node_type(operand_idx) {
                    self.set_node_type(node_idx, ty);
                } else {
                    let tv = self.fresh_type_var();
                    self.set_node_type(node_idx, tv);
                }
            }
        }
    }

    /// Handles `partial identifier` — bare function reference.
    ///
    /// Visits the identifier (resolves name, sets its type), then propagates
    /// the function type to the Partial node unchanged.  If the identifier
    /// does not resolve to a function, records an error.
    fn partial_from_identifier(&mut self, partial_idx: usize, ident_idx: usize) {
        // 1. Visit identifier (may report "Variable 'X' is not defined" if absent)
        self.visit_node(ident_idx);

        // 2. Get function name for diagnostics
        let Some(name) = self.ast.node_text(ident_idx) else {
            return;
        };
        let name = name.to_string();

        // 3. Look up symbol
        let symbol_info = self.scopes.lookup(&name).map(|s| (s.kind, s.type_id));
        let Some((kind, type_id_opt)) = symbol_info else {
            // visit_node already reported "not defined"
            let tv = self.fresh_type_var();
            self.set_node_type(partial_idx, tv);
            return;
        };

        // 4. Validate it's a function (before checking type_id — variables have type_id: None)
        if kind != SymbolKind::Function {
            self.record_error(self.make_error(
                format!("partial: '{}' is not a function", name),
                ident_idx,
            ));
            let tv = self.fresh_type_var();
            self.set_node_type(partial_idx, tv);
            return;
        }

        let Some(type_id) = type_id_opt else {
            // Symbol exists but has no type info yet — defer
            let tv = self.fresh_type_var();
            self.set_node_type(partial_idx, tv);
            return;
        };

        // 5. Propagate function type (or defer if not yet resolved)
        let resolved = self.type_registry.resolve(type_id).clone();
        match resolved {
            Type::Function(_) => self.set_node_type(partial_idx, type_id),
            Type::Var(_) | Type::Unknown => {
                // Cannot determine yet — propagate as-is and let unification handle it
                self.set_node_type(partial_idx, type_id);
            }
            _ => {
                self.record_error(self.make_error(
                    format!("partial: '{}' is not a function", name),
                    ident_idx,
                ));
                let tv = self.fresh_type_var();
                self.set_node_type(partial_idx, tv);
            }
        }
    }

    /// Handles `partial func(args...)` — partial function application.
    ///
    /// Does NOT delegate to `visit_function_call` (which would flag short argument
    /// lists as errors).  Instead:
    ///
    /// 1. Resolves the function name and its FunctionType.
    /// 2. Checks the provided argument count does not exceed the parameter count.
    /// 3. For each provided argument:
    ///    - `_` placeholder → keeps the corresponding param in the remaining list.
    ///    - concrete arg    → visits + constrains against param type; param is consumed.
    /// 4. Appends all uncovered parameters (beyond provided args) to remaining params.
    /// 5. Interns a new FunctionType for the remaining params and sets it on the Partial node.
    fn partial_from_call(&mut self, partial_idx: usize, call_idx: usize) {
        // 1. Extract Identifier child from FunctionCall
        let Some(ident_idx) = self.ast.nodes[call_idx].first_child else {
            return;
        };

        // 2. Get function name
        let Some(name) = self.ast.node_text(ident_idx) else {
            return;
        };
        let name = name.to_string();

        // 3. Look up symbol — report error if not defined
        let symbol_info = self.scopes.lookup(&name).map(|s| (s.kind, s.type_id));
        let Some((kind, type_id_opt)) = symbol_info else {
            // Report the same message as visit_function_call for consistency
            if let Some(token) = &self.ast.nodes[ident_idx].token {
                let error = crate::semantic::SemanticError::from_token(
                    format!("Function '{}' is not defined", name),
                    token,
                );
                self.record_error(error);
            }
            let tv = self.fresh_type_var();
            self.set_node_type(partial_idx, tv);
            return;
        };

        let Some(func_type_id) = type_id_opt else {
            let tv = self.fresh_type_var();
            self.set_node_type(partial_idx, tv);
            return;
        };

        // 4. Validate it's a function
        if kind != SymbolKind::Function {
            self.record_error(self.make_error(
                format!("partial: '{}' is not a function", name),
                ident_idx,
            ));
            let tv = self.fresh_type_var();
            self.set_node_type(partial_idx, tv);
            return;
        }

        let func_type = self.type_registry.resolve(func_type_id).clone();
        let Type::Function(ft) = func_type else {
            self.record_error(self.make_error(
                format!("partial: '{}' is not a function", name),
                ident_idx,
            ));
            let tv = self.fresh_type_var();
            self.set_node_type(partial_idx, tv);
            return;
        };

        // 5. Get ArgList and collect argument node indices
        let arg_list_idx = self.ast.nodes[ident_idx].next_sibling;
        let args: Vec<usize> = match arg_list_idx {
            Some(al_idx) => self.ast.children(al_idx).collect(),
            None => Vec::new(),
        };

        // 6. Error if too many args provided
        if args.len() > ft.params.len() {
            self.record_error(self.make_error(
                format!(
                    "partial: '{}' expects {} parameter(s) but {} argument(s) were provided",
                    name,
                    ft.params.len(),
                    args.len()
                ),
                call_idx,
            ));
            let tv = self.fresh_type_var();
            self.set_node_type(partial_idx, tv);
            return;
        }

        // 7. Process provided arguments, building remaining params
        let mut remaining_params: Vec<FunctionParam> = Vec::new();
        for (i, &arg_idx) in args.iter().enumerate() {
            let param = &ft.params[i];
            if self.ast.nodes[arg_idx].node_type == NodeType::Placeholder {
                // Placeholder keeps this param open; set its type for downstream inference
                self.set_node_type(arg_idx, param.type_id);
                remaining_params.push(param.clone());
            } else {
                // Concrete arg: visit it and constrain against the parameter type
                self.visit_node(arg_idx);
                if let Some(arg_type) = self.get_node_type(arg_idx) {
                    let param_type = self.type_registry.resolve(param.type_id).clone();
                    if !matches!(param_type, Type::Unknown) {
                        self.add_constraint(arg_type, param.type_id, arg_idx);
                    }
                }
                // This param is consumed; not carried forward to remaining
            }
        }

        // 8. Append all params not covered by provided args
        for param in ft.params[args.len()..].iter() {
            remaining_params.push(param.clone());
        }

        // 9. Intern the resulting partial function type and set it on the Partial node
        let new_func_type = Type::Function(FunctionType {
            params: remaining_params,
            return_type: ft.return_type,
        });
        let new_type_id = self.type_registry.intern(new_func_type);
        self.set_node_type(partial_idx, new_type_id);
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

    // ========== Group 1: partial of bare identifier ==========

    #[test]
    fn test_partial_bare_identifier_ok() {
        // partial add — bare identifier; propagates function type unchanged
        let source = r#"
            add: (x Number, y Number) Number { return x }
            f: partial add
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "partial add should succeed: {:?}",
            result.err()
        );
    }

    // ========== Group 2: partial func() — no args ==========

    #[test]
    fn test_partial_no_args_ok() {
        // partial add() — zero provided args; type is same as add
        let source = r#"
            add: (x Number, y Number) Number { return x }
            f: partial add()
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "partial add() should succeed: {:?}",
            result.err()
        );
    }

    // ========== Group 3: partial func(arg) — one concrete arg ==========

    #[test]
    fn test_partial_one_concrete_arg_ok() {
        // partial add(2) — first param consumed; remaining: (Number) → Number
        let source = r#"
            add: (x Number, y Number) Number { return x }
            f: partial add(2)
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "partial add(2) should succeed: {:?}",
            result.err()
        );
    }

    // ========== Group 4: partial func(arg, _) — concrete arg + placeholder ==========

    #[test]
    fn test_partial_concrete_and_placeholder_ok() {
        // partial add(2, _) — first param consumed, _ keeps second param open
        // Result is still (Number) → Number
        let source = r#"
            add: (x Number, y Number) Number { return x }
            f: partial add(2, _)
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "partial add(2, _) should succeed: {:?}",
            result.err()
        );
    }

    // ========== Group 5: partial func(arg1, arg2) — all args provided ==========

    #[test]
    fn test_partial_all_args_provided_ok() {
        // partial add(1, 2) — both params consumed; result is () → Number
        let source = r#"
            add: (x Number, y Number) Number { return x }
            f: partial add(1, 2)
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "partial add(1, 2) should succeed: {:?}",
            result.err()
        );
    }

    // ========== Group 6: type mismatch in provided arg ==========

    #[test]
    fn test_partial_arg_type_mismatch_err() {
        // partial add("hello") — first param expects Number but gets String
        let source = r#"
            add: (x Number, y Number) Number { return x }
            f: partial add("hello")
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "partial add(\"hello\") should fail (String vs Number)"
        );
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Type mismatch")),
            "Expected type mismatch error, got: {:?}",
            errors
        );
    }

    // ========== Group 7: too many arguments ==========

    #[test]
    fn test_partial_too_many_args_err() {
        // partial add(1, 2, 3) — add only has 2 params; 3 is too many
        let source = r#"
            add: (x Number, y Number) Number { return x }
            f: partial add(1, 2, 3)
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "partial add(1, 2, 3) should fail");
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("expects 2 parameter(s) but 3 argument(s)")),
            "Expected too-many-args error, got: {:?}",
            errors
        );
    }

    // ========== Group 8: partial of a non-function ==========

    #[test]
    fn test_partial_non_function_err() {
        // x: 42; f: partial x — x is a Number, not a function
        let source = r#"
            x: 42
            f: partial x
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "partial of non-function should fail");
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("is not a function")),
            "Expected 'is not a function' error, got: {:?}",
            errors
        );
    }

    // ========== Group 9: undefined function ==========

    #[test]
    fn test_partial_undefined_function_err() {
        // partial unknown(1) — 'unknown' is not defined
        let source = r#"
            f: partial unknown(1)
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "partial of undefined should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("not defined")),
            "Expected 'not defined' error, got: {:?}",
            errors
        );
    }

    // ========== Group 10: valid _ in ArgList ==========

    #[test]
    fn test_placeholder_in_arg_list_ok() {
        // partial add(_, 2) — _ in ArgList is a valid placeholder position
        let source = r#"
            add: (x Number, y Number) Number { return x }
            f: partial add(_, 2)
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "partial add(_, 2) should succeed: {:?}",
            result.err()
        );
    }

    // ========== Group 11: invalid _ standalone ==========

    #[test]
    fn test_placeholder_standalone_err() {
        // x: _ — placeholder used outside of arg list or match pattern
        let source = r#"
            x: _
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "standalone _ should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| {
                e.message
                    .contains("Placeholder '_' can only appear as a function argument or match pattern")
            }),
            "Expected placeholder error, got: {:?}",
            errors
        );
    }

    // ========== Group 12: partial result used correctly (type annotation integration) ==========

    #[test]
    fn test_partial_result_type_matches() {
        // partial add(2, _) keeps one param open; the whole expression is valid
        // Integration test: all type constraints are satisfiable
        let source = r#"
            add: (x Number, y Number) Number { return x }
            f: partial add(2, _)
            g: partial add(_, 3)
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "multiple partial applications should succeed: {:?}",
            result.err()
        );
    }

    // ========== Group 13: type annotation on partial result mismatches ==========

    #[test]
    fn test_partial_result_type_annotation_mismatch() {
        // Annotating the var holding 'partial add' with Number fails:
        // partial add is a (Number, Number) → Number function, not a Number value.
        let source = r#"
            add: (x Number, y Number) Number { return x }
            f Number: partial add
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "annotating partial result as Number should fail"
        );
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Type mismatch")),
            "Expected type mismatch error, got: {:?}",
            errors
        );
    }
}
