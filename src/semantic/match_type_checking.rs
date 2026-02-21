//! Match expression type checking and pattern validation
//!
//! Type rules for match expressions:
//! - The subject expression is visited and its type is inferred
//! - All arm result expressions must have a compatible (unified) type
//! - The match expression's result type is the common arm type
//!
//! Pattern validation rules:
//! - Each non-wildcard pattern must be type-compatible with the subject
//! - Patterns after a wildcard `_` are unreachable
//! - Duplicate literal patterns are unreachable
//! - Match must be exhaustive (cover all possible values)
//!
//! AST structure:
//! ```text
//! Match
//!   ├─ MatchSubject
//!   │   └─ <expression>
//!   └─ MatchArms
//!       ├─ MatchArm
//!       │   ├─ MatchPattern
//!       │   │   └─ <pattern>       (Placeholder | LiteralNumber | LiteralString | LiteralBoolean)
//!       │   └─ <result expression>
//!       └─ ...
//! ```

use super::{DeferredMatchExhaustivenessCheck, SemanticAnalyzer, SemanticError, Type};
use crate::ast::NodeType;
use crate::lexer::TokenKind;

impl SemanticAnalyzer {
    /// Type-checks a match expression and validates its patterns.
    ///
    /// all arm result expressions must have the same type T,
    ///            and the match expression itself has type T.
    /// each non-wildcard pattern is constrained to the subject type;
    ///            unreachable and duplicate patterns are reported immediately;
    ///            exhaustiveness is deferred until after unification.
    pub(super) fn visit_match(&mut self, node_idx: usize) {
        // Extract structure upfront via view (borrow released before any mutable calls)
        let (subject_expr_idx, arm_indices) = {
            let m = self.ast.match_expr(node_idx);
            (m.subject_expr_idx(), m.arm_indices().collect::<Vec<_>>())
        };

        // Visit the subject expression and capture its TypeId
        let subject_type_id = if let Some(expr_idx) = subject_expr_idx {
            self.visit_node(expr_idx);
            self.get_node_type(expr_idx)
        } else {
            None
        };

        // Create a fresh type variable for the match result type.
        // All arm result types will be constrained to equal this variable.
        let result_type_var = self.fresh_type_var();

        // State for unreachability and exhaustiveness tracking
        let mut saw_wildcard = false;
        let mut seen_bool_true = false;
        let mut seen_bool_false = false;
        let mut seen_number_literals: Vec<String> = Vec::new();
        let mut seen_string_literals: Vec<String> = Vec::new();

        // Iterate over MatchArm nodes
        for arm_idx in arm_indices {
            // Extract arm indices via view (borrow released before mutable calls)
            let (pattern_child_idx, result_expr_idx) = {
                let arm = self.ast.match_arm(arm_idx);
                (
                    arm.pattern_child_idx(),
                    arm.result_expr_idx().expect("MatchArm must have result expression"),
                )
            };

            // Process the pattern child (inside MatchPattern)
            if let Some(pattern_child_idx) = pattern_child_idx {
                if saw_wildcard {
                    // Any pattern after a wildcard is unreachable
                    self.record_error(self.make_error(
                        "Unreachable pattern: wildcard already covers all cases".to_string(),
                        pattern_child_idx,
                    ));
                } else {
                    // Read node type before any mutable calls
                    let pattern_node_type = self.ast.nodes[pattern_child_idx].node_type;

                    match pattern_node_type {
                        NodeType::Placeholder => {
                            saw_wildcard = true;
                            // Wildcard needs no type constraint
                        }
                        NodeType::LiteralBoolean => {
                            // Determine true/false from token kind
                            let is_true = matches!(
                                self.ast.nodes[pattern_child_idx]
                                    .token
                                    .as_ref()
                                    .map(|t| &t.kind),
                                Some(TokenKind::True)
                            );
                            let already_seen = if is_true { seen_bool_true } else { seen_bool_false };
                            if already_seen {
                                self.record_error(self.make_error(
                                    format!(
                                        "Unreachable pattern: '{}' already covered",
                                        if is_true { "true" } else { "false" }
                                    ),
                                    pattern_child_idx,
                                ));
                            } else if is_true {
                                seen_bool_true = true;
                            } else {
                                seen_bool_false = true;
                            }
                            // Add type constraint: pattern type == subject type
                            self.visit_node(pattern_child_idx);
                            if let (Some(pat_type), Some(subj_type)) =
                                (self.get_node_type(pattern_child_idx), subject_type_id)
                            {
                                self.add_constraint(pat_type, subj_type, pattern_child_idx);
                            }
                        }
                        NodeType::LiteralNumber => {
                            let num_text = self
                                .ast
                                .node_text(pattern_child_idx)
                                .unwrap_or("")
                                .to_string();
                            if seen_number_literals.contains(&num_text) {
                                self.record_error(self.make_error(
                                    format!(
                                        "Unreachable pattern: '{}' already covered",
                                        num_text
                                    ),
                                    pattern_child_idx,
                                ));
                            } else {
                                seen_number_literals.push(num_text);
                            }
                            self.visit_node(pattern_child_idx);
                            if let (Some(pat_type), Some(subj_type)) =
                                (self.get_node_type(pattern_child_idx), subject_type_id)
                            {
                                self.add_constraint(pat_type, subj_type, pattern_child_idx);
                            }
                        }
                        NodeType::LiteralString => {
                            let str_text = self
                                .ast
                                .node_text(pattern_child_idx)
                                .unwrap_or("")
                                .to_string();
                            if seen_string_literals.contains(&str_text) {
                                self.record_error(self.make_error(
                                    format!(
                                        "Unreachable pattern: '{}' already covered",
                                        str_text
                                    ),
                                    pattern_child_idx,
                                ));
                            } else {
                                seen_string_literals.push(str_text);
                            }
                            self.visit_node(pattern_child_idx);
                            if let (Some(pat_type), Some(subj_type)) =
                                (self.get_node_type(pattern_child_idx), subject_type_id)
                            {
                                self.add_constraint(pat_type, subj_type, pattern_child_idx);
                            }
                        }
                        _ => {
                            // Unknown pattern kind — skip validation
                        }
                    }
                }
            }

            // Visit the result expression to infer its type
            self.visit_node(result_expr_idx);

            // Constrain arm result type == match result type
            if let Some(arm_result_type) = self.get_node_type(result_expr_idx) {
                self.add_constraint(arm_result_type, result_type_var, result_expr_idx);
            }
        }

        // Defer exhaustiveness check until after unification resolves subject type
        self.deferred_match_checks
            .push(DeferredMatchExhaustivenessCheck {
                subject_expr_idx,
                has_wildcard: saw_wildcard,
                has_true: seen_bool_true,
                has_false: seen_bool_false,
                match_node_idx: node_idx,
            });

        // The match expression's type is the common arm result type
        self.set_node_type(node_idx, result_type_var);
    }

    /// Verifies match exhaustiveness for all deferred checks after unification.
    ///
    /// For each match expression:
    /// - If a wildcard arm exists → exhaustive (skip)
    /// - If subject resolves to `Bool` → both `true` and `false` must be present
    /// - If subject resolves to `Number`, `String`, etc. → wildcard is required
    /// - If subject type is still unknown (Var/Unknown/TypeParameter) → skip
    pub(super) fn verify_match_exhaustiveness(&mut self) {
        let checks = self.deferred_match_checks.clone();

        for check in &checks {
            // Wildcard covers all cases — always exhaustive
            if check.has_wildcard {
                continue;
            }

            // Resolve the subject type through substitution
            let resolved_type = if let Some(expr_idx) = check.subject_expr_idx {
                if let Some(type_id) = self.get_node_type(expr_idx) {
                    let resolved_id = self.substitution.apply(type_id, &self.type_registry);
                    Some(self.type_registry.resolve(resolved_id).clone())
                } else {
                    None
                }
            } else {
                None
            };

            match resolved_type {
                Some(Type::Bool) => {
                    // For Bool: need both true and false
                    if check.has_true && check.has_false {
                        // Exhaustive
                    } else {
                        let missing = if !check.has_true { "true" } else { "false" };
                        self.record_error(self.make_error(
                            format!(
                                "Non-exhaustive match: missing '{}' pattern (or add '_')",
                                missing
                            ),
                            check.match_node_idx,
                        ));
                    }
                }
                Some(Type::Var(_))
                | Some(Type::Unknown)
                | Some(Type::TypeParameter { .. })
                | None => {
                    // Cannot determine exhaustiveness — skip
                }
                Some(_) => {
                    // Number, String, struct, union, etc. — infinite domain requires wildcard
                    self.record_error(
                        self.make_error(
                            "Non-exhaustive match: add a wildcard arm '_' to handle all cases"
                                .to_string(),
                            check.match_node_idx,
                        ),
                    );
                }
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

    /// Helper function to analyze source code
    fn analyze_source(source: &str) -> Result<crate::ast::Ast, Vec<SemanticError>> {
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let analyzer = SemanticAnalyzer::new(ast);
        analyzer.analyze()
    }

    // ========== Group 1: Valid Match Expressions ==========

    #[test]
    fn test_match_number_arms() {
        let source = r#"
            x: 1
            result: match x {
                1: 100
                _: 0
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Match with all Number arms should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_match_bool_arms() {
        let source = r#"
            flag: true
            result: match flag {
                true: false
                _: true
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Match with all Bool arms should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_match_string_arms() {
        let source = r#"
            role: "admin"
            result: match role {
                "admin": "allowed"
                _: "denied"
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Match with all String arms should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_match_single_arm() {
        let source = r#"
            x: 42
            result: match x {
                _: 99
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Match with single wildcard arm should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_match_multiple_number_arms() {
        let source = r#"
            n: 2
            result: match n {
                0: 10
                1: 20
                2: 30
                _: 0
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Match with multiple Number arms should succeed: {:?}",
            result.err()
        );
    }

    // ========== Group 2: Match in Different Contexts ==========

    #[test]
    fn test_match_in_variable_declaration() {
        let source = r#"
            status: 1
            label: match status {
                1: "active"
                _: "inactive"
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Match in variable declaration should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_match_in_function_return() {
        let source = r#"
            describe: (n Number) String {
                return match n {
                    0: "zero"
                    _: "nonzero"
                }
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Match in function return should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_match_subject_is_function_call() {
        let source = r#"
            getValue: () Number { return 42 }
            result: match getValue() {
                0: false
                _: true
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Match with function call subject should succeed: {:?}",
            result.err()
        );
    }

    // ========== Group 3: Arm Type Mismatch Errors ==========

    #[test]
    fn test_match_incompatible_arm_types() {
        let source = r#"
            x: 1
            result: match x {
                1: 42
                _: "hello"
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Match with Number and String arms should fail"
        );
    }

    #[test]
    fn test_match_number_and_bool_arms() {
        let source = r#"
            x: 0
            result: match x {
                0: true
                _: 99
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Match with Bool and Number arms should fail"
        );
    }

    #[test]
    fn test_match_string_and_bool_arms() {
        let source = r#"
            x: "val"
            result: match x {
                "yes": true
                _: "no"
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Match with String and Bool arms should fail"
        );
    }

    // ========== Group 4: Pattern Type Validation ==========

    #[test]
    fn test_pattern_number_on_string_subject() {
        let source = r#"
            s: "hello"
            result: match s {
                1: "one"
                _: "other"
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Number pattern on String subject should fail"
        );
    }

    #[test]
    fn test_pattern_bool_on_number_subject() {
        let source = r#"
            n: 42
            result: match n {
                true: 1
                _: 0
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Bool pattern on Number subject should fail"
        );
    }

    #[test]
    fn test_pattern_string_on_bool_subject() {
        let source = r#"
            flag: true
            result: match flag {
                "yes": 1
                _: 0
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "String pattern on Bool subject should fail"
        );
    }

    // ========== Group 5: Unreachable Patterns ==========

    #[test]
    fn test_unreachable_after_wildcard() {
        let source = r#"
            x: 1
            result: match x {
                _: 0
                1: 10
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Pattern after wildcard should be unreachable"
        );
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Unreachable")),
            "Should report unreachable pattern error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_unreachable_two_wildcards() {
        let source = r#"
            x: 1
            result: match x {
                _: 0
                _: 99
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Second wildcard should be unreachable");
    }

    #[test]
    fn test_unreachable_duplicate_number_literal() {
        let source = r#"
            n: 5
            result: match n {
                1: 10
                1: 20
                _: 0
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Duplicate number literal should be unreachable"
        );
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Unreachable")),
            "Should report unreachable pattern error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_unreachable_duplicate_string_literal() {
        let source = r#"
            s: "a"
            result: match s {
                "a": 1
                "a": 2
                _: 0
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Duplicate string literal should be unreachable"
        );
    }

    #[test]
    fn test_unreachable_duplicate_bool_literal() {
        let source = r#"
            flag: true
            result: match flag {
                true: 1
                true: 2
                false: 3
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Duplicate bool literal should be unreachable"
        );
    }

    // ========== Group 6: Exhaustiveness ==========

    #[test]
    fn test_exhaustive_bool_both_cases() {
        let source = r#"
            flag: true
            result: match flag {
                true: 1
                false: 0
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Bool match with true + false should be exhaustive: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_non_exhaustive_bool_missing_false() {
        let source = r#"
            flag: true
            result: match flag {
                true: 1
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Bool match with only 'true' should be non-exhaustive"
        );
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Non-exhaustive")),
            "Should report non-exhaustive error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_non_exhaustive_bool_missing_true() {
        let source = r#"
            flag: false
            result: match flag {
                false: 0
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Bool match with only 'false' should be non-exhaustive"
        );
    }

    #[test]
    fn test_non_exhaustive_number_no_wildcard() {
        let source = r#"
            n: 5
            result: match n {
                1: "one"
                2: "two"
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Number match without wildcard should be non-exhaustive"
        );
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Non-exhaustive")),
            "Should report non-exhaustive error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_non_exhaustive_string_no_wildcard() {
        let source = r#"
            s: "hello"
            result: match s {
                "a": 1
                "b": 2
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "String match without wildcard should be non-exhaustive"
        );
    }
}
