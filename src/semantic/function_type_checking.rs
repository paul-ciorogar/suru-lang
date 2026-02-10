//! Function type checking tests for semantic analysis
//!
//! Tests for function type declarations (e.g., `type AddFn: (a Number, b Number) Number`)
//! and checking that function values match declared function types.

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

    // ========== Valid Function Type Declarations ==========

    #[test]
    fn test_function_type_no_params() {
        let result = analyze_source("type GetNum: () Number\n");
        assert!(result.is_ok());
    }

    #[test]
    fn test_function_type_one_param() {
        let result = analyze_source("type IntFn: (a Number) Number\n");
        assert!(result.is_ok());
    }

    #[test]
    fn test_function_type_two_params() {
        let result = analyze_source("type AddFn: (a Number, b Number) Number\n");
        assert!(result.is_ok());
    }

    #[test]
    fn test_function_type_string_return() {
        let result = analyze_source("type GetName: (id Number) String\n");
        assert!(result.is_ok());
    }

    #[test]
    fn test_function_type_bool_param_and_return() {
        let result = analyze_source("type Predicate: (value String) Bool\n");
        assert!(result.is_ok());
    }

    #[test]
    fn test_function_type_with_sized_types() {
        let result = analyze_source("type IntAdder: (a Int64, b Int64) Int64\n");
        assert!(result.is_ok());
    }

    // ========== Error Cases: Undefined Types ==========

    #[test]
    fn test_function_type_undefined_param_type() {
        let result = analyze_source("type BadFn: (a Foo, b Number) Number\n");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("Foo"));
        assert!(errors[0].message.contains("not defined"));
    }

    #[test]
    fn test_function_type_undefined_return_type() {
        let result = analyze_source("type BadFn: (a Number) Foo\n");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("Foo"));
        assert!(errors[0].message.contains("not defined"));
    }

    // ========== Function Values Matching Function Types ==========

    #[test]
    fn test_assign_matching_function_to_fn_typed_var() {
        let source = r#"
            type AddFn: (a Number, b Number) Number
            add: (a Number, b Number) Number { return a }
            myAdd AddFn: add
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Matching function should satisfy function type: {:?}", result.err());
    }

    #[test]
    fn test_assign_mismatched_param_count() {
        let source = r#"
            type BinaryFn: (a Number, b Number) Number
            unary: (a Number) Number { return a }
            myFn BinaryFn: unary
        "#;
        let result = analyze_source(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("parameter count mismatch"));
    }

    #[test]
    fn test_assign_mismatched_param_type() {
        let source = r#"
            type NumberFn: (a Number) Number
            strFn: (a String) Number { return 42 }
            myFn NumberFn: strFn
        "#;
        let result = analyze_source(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("Type mismatch"));
    }

    #[test]
    fn test_assign_mismatched_return_type() {
        let source = r#"
            type NumberFn: (a Number) Number
            boolFn: (a Number) Bool { return true }
            myFn NumberFn: boolFn
        "#;
        let result = analyze_source(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("Type mismatch"));
    }

    // ========== Integration Tests ==========

    #[test]
    fn test_function_type_in_union() {
        let source = r#"
            type AddFn: (a Number, b Number) Number
            type SubFn: (a Number, b Number) Number
            type BinaryOp: AddFn, SubFn
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_duplicate_function_type_declaration() {
        let source = "type AddFn: (a Number) Number\ntype AddFn: (b String) Bool\n";
        let result = analyze_source(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("Duplicate declaration"));
    }
}
