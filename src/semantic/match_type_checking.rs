//! Match expression type checking
//!
//! Type rules for match expressions:
//! - The subject expression is visited and its type is inferred
//! - All arm result expressions must have a compatible (unified) type
//! - The match expression's result type is the common arm type
//!
//! AST structure:
//! ```text
//! Match
//!   ├─ MatchSubject
//!   │   └─ <expression>
//!   └─ MatchArms
//!       ├─ MatchArm
//!       │   ├─ MatchPattern
//!       │   │   └─ <pattern>
//!       │   └─ <result expression>
//!       └─ ...
//! ```

use super::SemanticAnalyzer;

impl SemanticAnalyzer {
    /// Type-checks a match expression.
    ///
    /// Type rule: all arm result expressions must have the same type T,
    /// and the match expression itself has type T.
    pub(super) fn visit_match(&mut self, node_idx: usize) {
        // Match has two children: MatchSubject and MatchArms
        let subject_node_idx = self.ast.nodes[node_idx]
            .first_child
            .expect("Match must have MatchSubject child");
        let arms_node_idx = self.ast.nodes[subject_node_idx]
            .next_sibling
            .expect("Match must have MatchArms child");

        // Visit the subject expression (inside MatchSubject)
        if let Some(subject_expr_idx) = self.ast.nodes[subject_node_idx].first_child {
            self.visit_node(subject_expr_idx);
        }

        // Create a fresh type variable for the match result type.
        // All arm result types will be constrained to equal this variable.
        let result_type_var = self.fresh_type_var();

        // Iterate over MatchArm children of MatchArms
        if let Some(first_arm_idx) = self.ast.nodes[arms_node_idx].first_child {
            let mut current_arm = first_arm_idx;
            loop {
                // Each MatchArm: first child = MatchPattern, second child = result expression
                let pattern_idx = self.ast.nodes[current_arm]
                    .first_child
                    .expect("MatchArm must have MatchPattern child");
                let result_expr_idx = self.ast.nodes[pattern_idx]
                    .next_sibling
                    .expect("MatchArm must have result expression");

                // Visit the result expression to infer its type
                self.visit_node(result_expr_idx);

                // Constrain arm result type == match result type
                if let Some(arm_result_type) = self.get_node_type(result_expr_idx) {
                    self.add_constraint(arm_result_type, result_type_var, result_expr_idx);
                }

                // Move to next sibling arm
                if let Some(next_arm) = self.ast.nodes[current_arm].next_sibling {
                    current_arm = next_arm;
                } else {
                    break;
                }
            }
        }

        // The match expression's type is the common arm result type
        self.set_node_type(node_idx, result_type_var);
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
            "Match with single arm should succeed: {:?}",
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

    // ========== Group 3: Type Mismatch Errors ==========

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
        let errors = result.unwrap_err();
        assert!(
            !errors.is_empty(),
            "Should report at least one type error"
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
}
