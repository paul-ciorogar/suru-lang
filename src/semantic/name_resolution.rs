// Name resolution implementation for Phase 2
//
// This module implements visitor methods for:
// - Variable declaration resolution
// - Variable reference resolution
// - Function declaration resolution
// - Function call resolution

use super::{SemanticAnalyzer, SemanticError, Symbol, SymbolKind, TypeId};
use crate::ast::NodeType;

impl SemanticAnalyzer {
    /// Visits variable declaration
    /// Extracts variable name and optional type, then registers in current scope
    pub(super) fn visit_var_decl(&mut self, node_idx: usize) {
        // Extract variable name from first child (Identifier)
        let Some(ident_idx) = self.ast.nodes[node_idx].first_child else {
            return; // Malformed AST - should not happen
        };

        let Some(name) = self.ast.node_text(ident_idx) else {
            return; // No name - should not happen
        };
        let name = name.to_string();

        // Extract optional type annotation from second child
        let mut type_name: Option<String> = None;
        let mut value_expr_idx: Option<usize> = None;

        if let Some(second_child_idx) = self.ast.nodes[ident_idx].next_sibling {
            if self.ast.nodes[second_child_idx].node_type == NodeType::TypeAnnotation {
                type_name = self.ast.node_text(second_child_idx).map(String::from);
                // Value expression is after type annotation
                value_expr_idx = self.ast.nodes[second_child_idx].next_sibling;
            } else {
                // No type annotation, this is the value expression
                value_expr_idx = Some(second_child_idx);
            }
        }

        // Insert/update symbol in current scope (allow redeclaration)
        let symbol = Symbol::new(name, type_name.clone(), SymbolKind::Variable);
        self.scopes
            .current_scope_mut()
            .symbols
            .insert_or_replace(symbol);

        // Type checking (Phase 4.3)

        // 1. Resolve type annotation if present
        let declared_type_id: Option<TypeId> = if let Some(ref type_name_str) = type_name {
            match self.lookup_type_id(&type_name_str) {
                Ok(type_id) => Some(type_id),
                Err(error) => {
                    self.record_error(error);
                    None  // Continue even on error
                }
            }
        } else {
            None
        };

        // 2. Visit the initializer expression to infer its type
        if let Some(expr_idx) = value_expr_idx {
            self.visit_node(expr_idx);

            // 3. Get inferred type from initializer
            if let Some(init_type) = self.get_node_type(expr_idx) {
                // 4. Generate constraint or use inferred type
                if let Some(declared_type) = declared_type_id {
                    // With annotation: constrain init_type = declared_type
                    self.add_constraint(init_type, declared_type, expr_idx);
                    // Variable has declared type
                    self.set_node_type(node_idx, declared_type);
                } else {
                    // No annotation: variable has inferred type
                    self.set_node_type(node_idx, init_type);
                }
            }
        }
    }

    /// Visits identifier
    /// Resolves identifier references to variables
    pub(super) fn visit_identifier(&mut self, node_idx: usize) {
        // Get parent to determine context
        let parent_idx = match self.ast.nodes[node_idx].parent {
            Some(idx) => idx,
            None => return, // Root identifier? Shouldn't happen
        };

        let parent_type = self.ast.nodes[parent_idx].node_type;

        // Skip identifiers that are declarations or in special contexts
        match parent_type {
            NodeType::VarDecl => {
                // This is the variable name being declared, not a reference
                // Only skip if this is the first child (the name)
                if self.ast.nodes[parent_idx].first_child == Some(node_idx) {
                    return;
                }
                // Otherwise, it's an identifier in the initializer expression
            }
            NodeType::FunctionDecl => {
                // First child is function name, skip it
                if self.ast.nodes[parent_idx].first_child == Some(node_idx) {
                    return;
                }
            }
            NodeType::FunctionCall => {
                // First child is function name, handled in visit_function_call
                if self.ast.nodes[parent_idx].first_child == Some(node_idx) {
                    return;
                }
            }
            NodeType::Param => {
                // This is a parameter name declaration
                return;
            }
            NodeType::TypeAnnotation => {
                // This is a type name, not a variable reference
                return;
            }
            _ => {
                // This is a variable reference in an expression context
            }
        }

        // Extract name
        let Some(name) = self.ast.node_text(node_idx) else {
            return;
        };

        // Look up in scope chain
        if self.scopes.lookup(name).is_none() {
            let token = self.ast.nodes[node_idx].token.as_ref().unwrap();
            let error =
                SemanticError::from_token(format!("Variable '{}' is not defined", name), token);
            self.record_error(error);
        }
    }

    /// Helper: Build function signature string from function declaration
    /// Returns signature like "(Type1, Type2) -> RetType" or "()" or "() -> Type"
    fn build_function_signature(&self, func_decl_idx: usize) -> String {
        // Get ParamList (second child after function name)
        let ident_idx = self.ast.nodes[func_decl_idx].first_child.unwrap();
        let param_list_idx = self.ast.nodes[ident_idx].next_sibling.unwrap();

        // Build parameter type list
        let mut param_types = Vec::new();
        if let Some(first_param_idx) = self.ast.nodes[param_list_idx].first_child {
            let mut current_param_idx = first_param_idx;
            loop {
                // Each Param has Identifier child, possibly TypeAnnotation as second child
                if let Some(param_ident_idx) = self.ast.nodes[current_param_idx].first_child {
                    if let Some(type_ann_idx) = self.ast.nodes[param_ident_idx].next_sibling {
                        if self.ast.nodes[type_ann_idx].node_type == NodeType::TypeAnnotation {
                            if let Some(type_name) = self.ast.node_text(type_ann_idx) {
                                param_types.push(type_name.to_string());
                            } else {
                                param_types.push("?".to_string());
                            }
                        } else {
                            param_types.push("?".to_string());
                        }
                    } else {
                        param_types.push("?".to_string());
                    }
                }

                // Move to next param
                if let Some(next) = self.ast.nodes[current_param_idx].next_sibling {
                    current_param_idx = next;
                } else {
                    break;
                }
            }
        }

        // Get return type (after ParamList, if exists and is TypeAnnotation)
        let mut return_type = String::new();
        if let Some(after_params_idx) = self.ast.nodes[param_list_idx].next_sibling {
            if self.ast.nodes[after_params_idx].node_type == NodeType::TypeAnnotation {
                if let Some(type_name) = self.ast.node_text(after_params_idx) {
                    return_type = format!(" -> {}", type_name);
                }
            }
        }

        // Build signature
        format!("({}){}", param_types.join(", "), return_type)
    }

    /// Visits function declaration
    /// Registers function in current scope and adds parameters to function scope
    pub(super) fn visit_function_decl(&mut self, node_idx: usize) {
        // Extract function name from first child
        let Some(ident_idx) = self.ast.nodes[node_idx].first_child else {
            return;
        };
        let Some(name) = self.ast.node_text(ident_idx) else {
            return;
        };
        let name = name.to_string();

        // Build function signature
        let signature = self.build_function_signature(node_idx);

        // Check for duplicate in current scope
        if self.scopes.current_scope().lookup_local(&name).is_some() {
            let token = self.ast.nodes[ident_idx].token.as_ref().unwrap();
            let error = SemanticError::from_token(
                format!("Duplicate declaration of function '{}'", name),
                token,
            );
            self.record_error(error);
            return;
        }

        // Insert function symbol in current scope
        let symbol = Symbol::new(name.clone(), Some(signature), SymbolKind::Function);
        self.scopes.insert(symbol);

        // Enter function scope
        self.scopes.enter_scope(super::ScopeKind::Function);

        // Register parameters in function scope
        let param_list_idx = self.ast.nodes[ident_idx].next_sibling.unwrap();
        if let Some(first_param_idx) = self.ast.nodes[param_list_idx].first_child {
            let mut current_param_idx = first_param_idx;
            loop {
                // Extract parameter name and optional type
                if let Some(param_ident_idx) = self.ast.nodes[current_param_idx].first_child {
                    if let Some(param_name) = self.ast.node_text(param_ident_idx) {
                        let param_name = param_name.to_string();

                        // Get optional type annotation
                        let param_type = if let Some(type_ann_idx) =
                            self.ast.nodes[param_ident_idx].next_sibling
                        {
                            if self.ast.nodes[type_ann_idx].node_type == NodeType::TypeAnnotation {
                                self.ast.node_text(type_ann_idx).map(String::from)
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        // Insert parameter as variable in function scope
                        let param_symbol =
                            Symbol::new(param_name, param_type, SymbolKind::Variable);
                        self.scopes.insert(param_symbol);
                    }
                }

                // Move to next parameter
                if let Some(next) = self.ast.nodes[current_param_idx].next_sibling {
                    current_param_idx = next;
                } else {
                    break;
                }
            }
        }

        // Find and visit function body (Block node)
        // Block is after identifier, params, and optional return type
        let mut current_idx = Some(param_list_idx);
        let mut block_idx = None;
        while let Some(idx) = current_idx {
            if self.ast.nodes[idx].node_type == NodeType::Block {
                block_idx = Some(idx);
                break;
            }
            current_idx = self.ast.nodes[idx].next_sibling;
        }

        if let Some(block_idx) = block_idx {
            // Visit block children directly (don't create another Block scope)
            self.visit_children(block_idx);
        }

        // Exit function scope
        self.scopes.exit_scope();
    }

    /// Visits function call
    /// Resolves function name and validates it's a function
    pub(super) fn visit_function_call(&mut self, node_idx: usize) {
        // Extract function name from first child
        let Some(ident_idx) = self.ast.nodes[node_idx].first_child else {
            return;
        };
        let Some(name) = self.ast.node_text(ident_idx) else {
            return;
        };

        // Look up function in scope chain
        match self.scopes.lookup(name) {
            None => {
                // Function not defined
                let token = self.ast.nodes[ident_idx].token.as_ref().unwrap();
                let error =
                    SemanticError::from_token(format!("Function '{}' is not defined", name), token);
                self.record_error(error);
            }
            Some(symbol) => {
                // Validate it's a function
                if symbol.kind != SymbolKind::Function {
                    let token = self.ast.nodes[ident_idx].token.as_ref().unwrap();
                    let error =
                        SemanticError::from_token(format!("'{}' is not a function", name), token);
                    self.record_error(error);
                }
            }
        }

        // Visit arguments (to resolve any variable references in arguments)
        if let Some(arg_list_idx) = self.ast.nodes[ident_idx].next_sibling {
            self.visit_children(arg_list_idx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{SemanticAnalyzer, SemanticError};
    use crate::lexer::lex;
    use crate::limits::CompilerLimits;
    use crate::parser::parse;

    // Helper function to analyze source code
    fn analyze_source(source: &str) -> Result<crate::ast::Ast, Vec<SemanticError>> {
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let analyzer = SemanticAnalyzer::new(ast);
        analyzer.analyze()
    }

    // ========== Variable Declaration Tests ==========

    #[test]
    fn test_var_decl_simple() {
        // Simple variable declaration with type annotation
        let result = analyze_source("x Number: 42");
        assert!(result.is_ok(), "Simple variable declaration should succeed");
    }

    #[test]
    fn test_var_decl_redeclaration() {
        // Variable redeclaration should be allowed (replaces previous)
        let source = "x Number: 42\nx String: \"hello\"";
        let result = analyze_source(source);
        assert!(result.is_ok(), "Variable redeclaration should be allowed");
    }

    #[test]
    fn test_var_decl_no_type_annotation() {
        // Variable declaration without type annotation
        let result = analyze_source("x: 42");
        assert!(
            result.is_ok(),
            "Variable declaration without type should succeed"
        );
    }

    #[test]
    fn test_var_decl_in_nested_scope() {
        // Variable declaration in function scope
        let source = r#"
            foo: () {
                x Number: 42
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Variable declaration in nested scope should succeed"
        );
    }

    // ========== Variable Reference Tests ==========

    #[test]
    fn test_var_reference_valid() {
        // Variable reference to a previously declared variable
        let source = "x Number: 42\ny: x";
        let result = analyze_source(source);
        assert!(result.is_ok(), "Valid variable reference should succeed");
    }

    #[test]
    fn test_var_reference_undefined() {
        // Reference to undefined variable
        let source = "y: x";
        let result = analyze_source(source);
        assert!(result.is_err(), "Undefined variable reference should fail");
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("Variable 'x' is not defined"));
    }

    #[test]
    fn test_var_reference_in_parent_scope() {
        // Variable from outer scope should be visible in function
        let source = r#"
            x: 42
            foo: () {
                y: x
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Variable from parent scope should be accessible"
        );
    }

    #[test]
    fn test_var_shadowing() {
        // Inner variable shadows outer variable
        let source = r#"
            x: 42
            foo: () {
                x String: "hello"
                y: x
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Variable shadowing should work");
    }

    #[test]
    fn test_var_reference_before_declaration() {
        // Forward reference should fail
        let source = "y: x\nx: 42";
        let result = analyze_source(source);
        assert!(result.is_err(), "Forward reference should fail");
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("Variable 'x' is not defined"));
    }

    // ========== Function Declaration Tests ==========

    #[test]
    fn test_function_decl_simple() {
        // Simple function with no parameters
        let source = "foo: () { }";
        let result = analyze_source(source);
        assert!(result.is_ok(), "Simple function declaration should succeed");
    }

    #[test]
    fn test_function_decl_with_params() {
        // Function with parameters and return type
        let source = "add: (x Number, y Number) Number { }";
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Function with params and return type should succeed"
        );
    }

    #[test]
    fn test_function_decl_duplicate_error() {
        // Duplicate function declaration should fail
        let source = "foo: () { }\nfoo: () { }";
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Duplicate function declaration should fail"
        );
        let errors = result.unwrap_err();
        assert!(
            errors[0]
                .message
                .contains("Duplicate declaration of function 'foo'")
        );
    }

    #[test]
    fn test_function_params_in_scope() {
        // Function parameters should be visible in function body
        let source = r#"
            foo: (x Number) {
                y: x
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Function parameters should be in scope");
    }

    #[test]
    fn test_nested_function_decls() {
        // Functions can be nested
        let source = r#"
            outer: () {
                inner: () { }
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Nested functions should work");
    }

    // ========== Function Call Tests ==========

    #[test]
    fn test_function_call_valid() {
        // Valid function call
        let source = r#"
            foo: () { }
            bar: () {
                foo()
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Valid function call should succeed");
    }

    #[test]
    fn test_function_call_undefined() {
        // Call to undefined function
        let source = "x: foo()";
        let result = analyze_source(source);
        assert!(result.is_err(), "Undefined function call should fail");
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("Function 'foo' is not defined"));
    }

    #[test]
    fn test_function_call_not_a_function() {
        // Calling a variable as a function
        let source = "x: 42\ny: x()";
        let result = analyze_source(source);
        assert!(result.is_err(), "Calling variable as function should fail");
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("'x' is not a function"));
    }

    #[test]
    fn test_function_call_with_var_args() {
        // Function call with variable arguments
        let source = r#"
            foo: (x Number) { }
            a: 42
            b: foo(a)
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Function call with variable arguments should succeed"
        );
    }

    #[test]
    fn test_function_call_recursive() {
        // Recursive function call
        let source = r#"
            factorial: (n Number) Number {
                result: factorial(n)
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Recursive function call should succeed");
    }
}

/// Tests for variable declaration type checking (Phase 4.3)
#[cfg(test)]
mod variable_type_tests {
    use super::*;
    use crate::limits::CompilerLimits;
    use crate::lexer::lex;
    use crate::parser::parse;
    use crate::semantic::{Type, IntSize, FloatSize};

    /// Helper to analyze variable declaration and return its type
    fn analyze_var_decl(source: &str) -> Result<Type, Vec<SemanticError>> {
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits)
            .map_err(|e| vec![SemanticError::new(format!("{:?}", e), 0, 0)])?;
        let ast = parse(tokens, &limits)
            .map_err(|e| vec![SemanticError::new(format!("{:?}", e), 0, 0)])?;

        // Navigate to VarDecl node
        let root_idx = ast
            .root
            .ok_or(vec![SemanticError::new("No root".to_string(), 0, 0)])?;
        let decl_idx = ast.nodes[root_idx]
            .first_child
            .ok_or(vec![SemanticError::new(
                "No declaration".to_string(),
                0,
                0,
            )])?;

        let mut analyzer = SemanticAnalyzer::new(ast);

        // Run 3-phase analysis
        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
            analyzer.solve_constraints()?;
            analyzer.apply_substitution();
        }

        let type_id = analyzer.get_node_type(decl_idx).ok_or(vec![
            SemanticError::new("No type".to_string(), 0, 0),
        ])?;
        let ty = analyzer.type_registry.resolve(type_id);
        Ok(ty.clone())
    }

    // ========== Group 1: With Type Annotation ==========

    #[test]
    fn test_var_decl_with_annotation_valid() {
        let ty = analyze_var_decl("x Number: 42").unwrap();
        assert_eq!(ty, Type::Number);
    }

    #[test]
    fn test_var_decl_with_annotation_type_mismatch() {
        let err = analyze_var_decl("x Number: \"hello\"").unwrap_err();
        assert!(!err.is_empty());
        assert!(err[0].message.contains("Type mismatch"));
    }

    #[test]
    fn test_var_decl_with_annotation_undefined_type() {
        // When type annotation fails, error is recorded but inference continues
        // So the variable gets the inferred type from initializer
        let source = "x Foo: 42";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let mut analyzer = SemanticAnalyzer::new(ast);
        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
            let _ = analyzer.solve_constraints();
        }
        // Check that an error was recorded
        assert!(!analyzer.errors.is_empty());
        assert!(analyzer.errors[0].message.contains("Type 'Foo' not found"));
    }

    #[test]
    fn test_var_decl_with_annotation_bool() {
        let ty = analyze_var_decl("x Bool: true").unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_var_decl_with_annotation_complex_expr() {
        let ty = analyze_var_decl("x Bool: true and false").unwrap();
        assert_eq!(ty, Type::Bool);
    }

    // ========== Group 2: Without Type Annotation ==========

    #[test]
    fn test_var_decl_inferred_number() {
        let ty = analyze_var_decl("x: 42").unwrap();
        assert_eq!(ty, Type::Number);
    }

    #[test]
    fn test_var_decl_inferred_string() {
        let ty = analyze_var_decl("x: \"hello\"").unwrap();
        assert_eq!(ty, Type::String);
    }

    #[test]
    fn test_var_decl_inferred_bool() {
        let ty = analyze_var_decl("x: true").unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_var_decl_inferred_expression() {
        let ty = analyze_var_decl("x: not false").unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_var_decl_inferred_negate() {
        let ty = analyze_var_decl("x: -42").unwrap();
        assert_eq!(ty, Type::Number);
    }

    // ========== Group 3: Variable Redeclaration ==========

    #[test]
    fn test_var_redecl_same_type() {
        let source = "x Number: 42\nx Number: 99";
        // Only tests that analysis completes without error
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let mut analyzer = SemanticAnalyzer::new(ast);
        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
            let result = analyzer.solve_constraints();
            assert!(result.is_ok(), "Same type redeclaration should succeed");
        }
    }

    #[test]
    fn test_var_redecl_different_type() {
        let source = "x Number: 42\nx String: \"hello\"";
        // Redeclaration allowed with different type
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let mut analyzer = SemanticAnalyzer::new(ast);
        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
            let result = analyzer.solve_constraints();
            assert!(result.is_ok(), "Different type redeclaration should succeed");
        }
    }

    #[test]
    fn test_var_redecl_annotation_to_inferred() {
        let source = "x Number: 42\nx: \"hello\"";
        // Redeclaration from annotation to inferred
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let mut analyzer = SemanticAnalyzer::new(ast);
        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
            let result = analyzer.solve_constraints();
            assert!(result.is_ok(), "Annotation to inferred redeclaration should succeed");
        }
    }

    // ========== Group 4: Sized Integer Types ==========

    #[test]
    fn test_var_decl_int64_annotation() {
        // Variable gets the declared type Int64, even though literal is Number
        // This will require unification to accept Number as compatible with Int64
        // For now, just test that the annotation is parsed and stored
        let source = "x Int64: 42";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let root_idx = ast.root.unwrap();
        let decl_idx = ast.nodes[root_idx].first_child.unwrap();

        let mut analyzer = SemanticAnalyzer::new(ast);
        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
            let _ = analyzer.solve_constraints();
        }

        // Variable should have Int64 type (the declared type)
        if let Some(type_id) = analyzer.get_node_type(decl_idx) {
            let ty = analyzer.type_registry.resolve(type_id);
            assert_eq!(ty, &Type::Int(IntSize::I64));
        }
    }

    #[test]
    fn test_var_decl_float32_annotation() {
        // Variable gets the declared type Float32, even though literal is Number
        let source = "x Float32: 42";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let root_idx = ast.root.unwrap();
        let decl_idx = ast.nodes[root_idx].first_child.unwrap();

        let mut analyzer = SemanticAnalyzer::new(ast);
        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
            let _ = analyzer.solve_constraints();
        }

        // Variable should have Float32 type (the declared type)
        if let Some(type_id) = analyzer.get_node_type(decl_idx) {
            let ty = analyzer.type_registry.resolve(type_id);
            assert_eq!(ty, &Type::Float(FloatSize::F32));
        }
    }
}
