use suru_lang::lexer;
use suru_lang::limits::CompilerLimits;
use suru_lang::parser;
use suru_lang::semantic::{AnalysisError, AnalysisOutput, SemanticAnalyzer, SemanticError};

fn analyze_source(source: &str) -> Result<suru_lang::ast::Ast, Vec<SemanticError>> {
    let limits = CompilerLimits::default();
    let tokens = lexer::lex(source, &limits).expect("lexer failed");
    let ast = parser::parse(tokens, &limits).expect("parser failed");
    SemanticAnalyzer::new(ast).analyze()
}

fn analyze_with_types(source: &str) -> Result<AnalysisOutput, AnalysisError> {
    let limits = CompilerLimits::default();
    let tokens = lexer::lex(source, &limits).expect("lexer failed");
    let ast = parser::parse(tokens, &limits).expect("parser failed");
    SemanticAnalyzer::new(ast).analyze_with_types()
}

#[test]
fn test_check_valid_program() {
    let source = r#"
x: 42
y: "hello"
"#;
    assert!(analyze_source(source).is_ok());
}

#[test]
fn test_check_valid_function() {
    let source = r#"
add: (x Number, y Number) Number {
    return x
}
"#;
    assert!(analyze_source(source).is_ok());
}

#[test]
fn test_check_undefined_variable() {
    let source = r#"
x: undefined_var
"#;
    let result = analyze_source(source);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(!errors.is_empty());
    let msg = &errors[0].message;
    assert!(
        msg.contains("undefined_var") || msg.contains("Undefined") || msg.contains("not found"),
        "Expected error about undefined_var, got: {msg}"
    );
}

#[test]
fn test_check_type_declaration() {
    let source = r#"
type Person: {
    name String
    age Number
}
"#;
    assert!(analyze_source(source).is_ok());
}

// ========== Annotated Output Tests ==========

#[test]
fn test_annotated_number_literal() {
    let output = analyze_with_types("x: 42\n").expect("analysis failed");
    let annotated = output.to_annotated_string();
    assert!(
        annotated.contains("[Number]"),
        "Expected [Number] annotation, got:\n{annotated}"
    );
}

#[test]
fn test_annotated_string_variable() {
    let output = analyze_with_types("greeting: \"hello\"\n").expect("analysis failed");
    let annotated = output.to_annotated_string();
    assert!(
        annotated.contains("[String]"),
        "Expected [String] annotation, got:\n{annotated}"
    );
}

#[test]
fn test_annotated_bool_variable() {
    let output = analyze_with_types("flag: true\n").expect("analysis failed");
    let annotated = output.to_annotated_string();
    assert!(
        annotated.contains("[Bool]"),
        "Expected [Bool] annotation, got:\n{annotated}"
    );
}

#[test]
fn test_annotated_function_decl() {
    let source = "double: (n Number) Number {\n    return n\n}\n";
    let output = analyze_with_types(source).expect("analysis failed");
    let annotated = output.to_annotated_string();
    // The return statement and parameter references inside the body are annotated
    assert!(
        annotated.contains("[Number]"),
        "Expected [Number] annotation inside function body, got:\n{annotated}"
    );
    assert!(
        annotated.contains("FunctionDecl"),
        "Expected FunctionDecl node, got:\n{annotated}"
    );
}

#[test]
fn test_annotated_output_structure() {
    let output = analyze_with_types("x: 42\n").expect("analysis failed");
    let annotated = output.to_annotated_string();
    // Should still have the AST node names
    assert!(annotated.contains("Program"));
    assert!(annotated.contains("VarDecl"));
    assert!(annotated.contains("Identifier 'x'"));
    assert!(annotated.contains("LiteralNumber '42'"));
}
