//! Union type checking tests
//!
//! Tests for Phase 7.1: Union Type Support
//! Validates that union type checking works correctly including:
//! - Named unit types as values
//! - Union type annotation on variables
//! - Function parameters and return types with unions
//! - Error cases for type mismatches

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

    // ========== Named Unit Type as Value ==========

    #[test]
    fn test_named_unit_type_as_value() {
        let source = "type Success\nx: Success\n";
        let result = analyze_source(source);
        assert!(result.is_ok(), "Named unit type should be usable as a value: {:?}", result.err());
    }

    #[test]
    fn test_two_distinct_named_unit_types() {
        let source = "type Success\ntype Error\nx: Success\ny: Error\n";
        let result = analyze_source(source);
        assert!(result.is_ok(), "Different named unit types should work: {:?}", result.err());
    }

    #[test]
    fn test_named_unit_type_mismatch() {
        // Assigning one named unit type to a variable annotated with a different one
        let source = "type A\ntype B\nx A: B\n";
        let result = analyze_source(source);
        assert!(result.is_err(), "Different named unit types should not be assignable");
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("Type mismatch"), "Expected type mismatch, got: {}", errors[0].message);
    }

    // ========== Union Type Annotation on Variables ==========

    #[test]
    fn test_union_annotation_with_first_alternative() {
        let source = "type Success\ntype Error\ntype Status: Success, Error\nx Status: Success\n";
        let result = analyze_source(source);
        assert!(result.is_ok(), "First union alternative should be valid: {:?}", result.err());
    }

    #[test]
    fn test_union_annotation_with_second_alternative() {
        let source = "type Success\ntype Error\ntype Status: Success, Error\nx Status: Error\n";
        let result = analyze_source(source);
        assert!(result.is_ok(), "Second union alternative should be valid: {:?}", result.err());
    }

    #[test]
    fn test_union_annotation_with_wrong_type() {
        let source = "type Success\ntype Error\ntype Status: Success, Error\nx Status: 42\n";
        let result = analyze_source(source);
        assert!(result.is_err(), "Number should not be assignable to union of named units");
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("not a member of the union"), "Expected union membership error, got: {}", errors[0].message);
    }

    #[test]
    fn test_union_annotation_with_non_member_named_type() {
        let source = "type A\ntype B\ntype C\ntype AB: A, B\nx AB: C\n";
        let result = analyze_source(source);
        assert!(result.is_err(), "Non-member named type should not be assignable to union");
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("not a member of the union"), "Expected union membership error, got: {}", errors[0].message);
    }

    // ========== Union with Built-in Types ==========

    #[test]
    fn test_union_with_builtins_valid_number() {
        let source = "type Value: Number, String\nx Value: 42\n";
        let result = analyze_source(source);
        assert!(result.is_ok(), "Number should be valid for Number|String union: {:?}", result.err());
    }

    #[test]
    fn test_union_with_builtins_valid_string() {
        let source = "type Value: Number, String\nx Value: \"hello\"\n";
        let result = analyze_source(source);
        assert!(result.is_ok(), "String should be valid for Number|String union: {:?}", result.err());
    }

    #[test]
    fn test_union_with_builtins_wrong_type() {
        let source = "type Value: Number, String\nx Value: true\n";
        let result = analyze_source(source);
        assert!(result.is_err(), "Bool should not be assignable to Number|String union");
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("not a member of the union"), "Expected union membership error, got: {}", errors[0].message);
    }

    // ========== Union with Three Alternatives ==========

    #[test]
    fn test_union_three_alternatives_all_valid() {
        let source = r#"
            type A
            type B
            type C
            type ABC: A, B, C
            x ABC: A
            y ABC: B
            z ABC: C
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "All three alternatives should be valid: {:?}", result.err());
    }

    // ========== Function Parameters with Union Types ==========

    #[test]
    fn test_function_param_union_valid() {
        let source = r#"
            type Success
            type Error
            type Status: Success, Error
            handle: (s Status) { }
            main: () {
                handle(Success)
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Passing union member to function should work: {:?}", result.err());
    }

    #[test]
    fn test_function_param_union_wrong_type() {
        let source = r#"
            type Success
            type Error
            type Status: Success, Error
            handle: (s Status) { }
            main: () {
                handle(42)
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Passing wrong type to union param should fail");
    }

    // ========== Function Return Type with Union ==========

    #[test]
    fn test_function_return_union_valid() {
        let source = r#"
            type Success
            type Error
            type Status: Success, Error
            getStatus: () Status {
                return Success
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Returning union member should work: {:?}", result.err());
    }

    // ========== Mixed Named and Built-in Union ==========

    #[test]
    fn test_union_mixed_named_and_builtin() {
        let source = r#"
            type None
            type MaybeNumber: Number, None
            x MaybeNumber: 42
            y MaybeNumber: None
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Union of Number and None should work: {:?}", result.err());
    }
}
