// Assignment type checking tests (Phase 4.4)
//
// This module tests:
// - Constant redeclaration errors at file level
// - Variable reassignment type checking in mutable scopes
// - Shadowing behavior

#[cfg(test)]
mod tests {
    use crate::lexer::lex;
    use crate::limits::CompilerLimits;
    use crate::parser::parse;
    use crate::semantic::SemanticAnalyzer;
    use crate::semantic::SemanticError;

    /// Helper function to analyze source code
    fn analyze_source(source: &str) -> Result<crate::ast::Ast, Vec<SemanticError>> {
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let analyzer = SemanticAnalyzer::new(ast);
        analyzer.analyze()
    }

    // ========== Constant Redeclaration Tests ==========

    #[test]
    fn test_redeclare_constant_same_type() {
        // Redeclaring a constant at file level should fail
        let source = "x: 42\nx: 99";
        let result = analyze_source(source);
        assert!(result.is_err(), "Constant redeclaration should fail");
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
        assert!(
            errors[0].message.contains("Cannot redeclare constant"),
            "Expected 'Cannot redeclare constant' error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_redeclare_constant_different_type() {
        // Redeclaring a constant with different type should fail
        let source = "x: 42\nx: \"hello\"";
        let result = analyze_source(source);
        assert!(result.is_err(), "Constant redeclaration should fail");
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
        assert!(
            errors[0].message.contains("Cannot redeclare constant"),
            "Expected 'Cannot redeclare constant' error, got: {}",
            errors[0].message
        );
    }

    // ========== Shadowing Tests ==========

    #[test]
    fn test_shadow_constant_in_function() {
        // Creating a local variable that shadows a constant is allowed
        let source = r#"
            x: 42
            foo: () {
                x: "hello"
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Shadowing constant in function should succeed: {:?}", result.unwrap_err());
    }

    #[test]
    fn test_shadow_constant_same_type() {
        // Shadowing with same type is also allowed
        let source = r#"
            x: 42
            foo: () {
                x: 99
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Shadowing constant with same type should succeed: {:?}", result.unwrap_err());
    }

    #[test]
    fn test_nested_function_shadows_outer() {
        // Nested function can shadow outer function's variable
        let source = r#"
            foo: () {
                x: 42
                bar: () {
                    x: 99
                }
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Nested function shadowing should succeed: {:?}", result.unwrap_err());
    }

    #[test]
    fn test_nested_function_shadows_different_type() {
        // Nested function can shadow with different type
        let source = r#"
            foo: () {
                x: 42
                bar: () {
                    x: "hello"
                }
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Nested function shadowing with different type should succeed: {:?}", result.unwrap_err());
    }

    // ========== Reassignment Tests (Same Scope) ==========

    #[test]
    fn test_reassign_same_type() {
        // Reassigning with same type in function scope should succeed
        let source = r#"
            foo: () {
                x: 42
                x: 99
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Reassigning with same type should succeed: {:?}", result.unwrap_err());
    }

    #[test]
    fn test_reassign_wrong_type() {
        // Reassigning with different type in same scope should fail
        let source = r#"
            foo: () {
                x: 42
                x: "hello"
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Reassigning with wrong type should fail");
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
        assert!(
            errors[0].message.contains("Type mismatch"),
            "Expected 'Type mismatch' error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_reassign_inferred_type_success() {
        // Reassigning inferred type with matching type should succeed
        let source = r#"
            foo: () {
                x: true
                x: false
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Reassigning inferred bool with bool should succeed: {:?}", result.unwrap_err());
    }

    #[test]
    fn test_reassign_inferred_type_mismatch() {
        // Reassigning inferred type with wrong type should fail
        let source = r#"
            foo: () {
                x: true
                x: 42
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Reassigning inferred bool with number should fail");
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
        assert!(
            errors[0].message.contains("Type mismatch"),
            "Expected 'Type mismatch' error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_reassign_with_annotation_success() {
        // Reassigning annotated variable with matching type
        let source = r#"
            foo: () {
                x Number: 42
                x: 99
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Reassigning annotated number with number should succeed: {:?}", result.unwrap_err());
    }

    #[test]
    fn test_reassign_with_annotation_mismatch() {
        // Reassigning annotated variable with wrong type
        let source = r#"
            foo: () {
                x Number: 42
                x: "hello"
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Reassigning annotated number with string should fail");
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
        assert!(
            errors[0].message.contains("Type mismatch"),
            "Expected 'Type mismatch' error, got: {}",
            errors[0].message
        );
    }

    // ========== Multiple Reassignments ==========

    #[test]
    fn test_multiple_reassignments_same_type() {
        // Multiple reassignments with same type should succeed
        let source = r#"
            foo: () {
                x: 1
                x: 2
                x: 3
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Multiple reassignments should succeed: {:?}", result.unwrap_err());
    }

    #[test]
    fn test_multiple_reassignments_fails_on_mismatch() {
        // Should fail at the point of type mismatch
        let source = r#"
            foo: () {
                x: 1
                x: 2
                x: "oops"
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Third reassignment should fail due to type mismatch");
    }
}
