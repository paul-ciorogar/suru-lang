use suru_lang::lexer;
use suru_lang::limits::CompilerLimits;
use suru_lang::parser;
use suru_lang::semantic::SemanticAnalyzer;

fn analyze_source(source: &str) -> Result<suru_lang::ast::Ast, Vec<suru_lang::semantic::SemanticError>> {
    let limits = CompilerLimits::default();
    let tokens = lexer::lex(source, &limits).expect("lexer failed");
    let ast = parser::parse(tokens, &limits).expect("parser failed");
    SemanticAnalyzer::new(ast).analyze()
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
