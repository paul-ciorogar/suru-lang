// Name resolution implementation
//
// This module implements visitor methods for:
// - Variable declaration resolution
// - Variable reference resolution
// - Function declaration resolution
// - Function call resolution

use super::{
    FunctionParam, FunctionType, SemanticAnalyzer, SemanticError, Symbol, SymbolKind, Type, TypeId,
};
use crate::ast::NodeType;

impl SemanticAnalyzer {
    /// Visits variable declaration
    /// Extracts variable name and optional type, then registers in current scope
    pub(super) fn visit_var_decl(&mut self, node_idx: usize) {
        let decl = self.ast.var_decl(node_idx);

        let Some(name) = decl.name() else {
            return; // Malformed AST - should not happen
        };
        let name = name.to_string();
        let type_annotation = decl.type_annotation().map(String::from);
        let value_expr_idx = decl.value_expr_idx();
        let ident_idx = decl.ident_idx().unwrap();

        // Assignment type checking
        // Check if variable already exists in CURRENT scope (not outer scopes)
        // This must happen BEFORE inserting the new symbol!
        let exists_in_current_scope = self
            .scopes
            .current_scope()
            .lookup_local(&name)
            .filter(|s| s.kind == SymbolKind::Variable)
            .is_some();

        // Get existing type BEFORE replacing the symbol (for reassignment checking)
        let existing_type_id: Option<TypeId> =
            if exists_in_current_scope && self.scopes.is_in_mutable_scope() {
                self.lookup_variable_type(&name)
            } else {
                None // New declaration (including shadowing)
            };

        // Check for constant redeclaration at file level
        if exists_in_current_scope && !self.scopes.is_in_mutable_scope() {
            // Global/Module scope: constants cannot be redeclared
            let token = self.ast.nodes[ident_idx].token.as_ref().unwrap();
            self.record_error(SemanticError::from_token(
                format!("Cannot redeclare constant '{}'", name),
                token,
            ));
            return; // Stop processing this declaration
        }
        // Note: If variable exists in OUTER scope only, this is shadowing (allowed)

        // Insert/update symbol in current scope
        let symbol = Symbol::new(name.clone(), type_annotation.clone(), SymbolKind::Variable);
        self.scopes
            .current_scope_mut()
            .symbols
            .insert_or_replace(symbol);

        // Type checking

        // 1. Resolve type annotation if present
        let declared_type_id: Option<TypeId> = if let Some(ref type_name_str) = type_annotation {
            match self.lookup_type_id(&type_name_str) {
                Ok(type_id) => Some(type_id),
                Err(error) => {
                    self.record_error(error);
                    None // Continue even on error
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
                // 4. Determine final type (from annotation or inference)
                let final_type = if let Some(declared_type) = declared_type_id {
                    // With annotation: constrain init_type = declared_type
                    self.add_constraint(init_type, declared_type, expr_idx);
                    declared_type
                } else {
                    // No annotation: use inferred type
                    init_type
                };

                // 5. Assignment type checking
                if let Some(existing_type) = existing_type_id {
                    // This is a reassignment: constrain value to match existing variable type
                    self.add_constraint(init_type, existing_type, expr_idx);
                } else {
                    // This is a new declaration: record the variable's type
                    self.record_variable_type(&name, final_type);
                }

                // Set node type for the declaration
                self.set_node_type(node_idx, final_type);
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
        // Extract symbol info before mutable borrows
        let symbol_info = self.scopes.lookup(name).map(|s| (s.kind, s.type_id));

        match symbol_info {
            None => {
                let token = self.ast.nodes[node_idx].token.as_ref().unwrap();
                let error =
                    SemanticError::from_token(format!("Variable '{}' is not defined", name), token);
                self.record_error(error);
            }
            Some((kind, type_id)) => {
                // Set the node type from the variable's type
                if let Some(var_type) = self.lookup_variable_type(name) {
                    self.set_node_type(node_idx, var_type);
                } else if kind == SymbolKind::Type {
                    // Named unit types can be used as values (e.g., x: Success)
                    if let Some(type_id) = type_id {
                        let ty = self.type_registry.resolve(type_id).clone();
                        if matches!(ty, Type::NamedUnit(_)) {
                            self.set_node_type(node_idx, type_id);
                        }
                    }
                } else if kind == SymbolKind::Function {
                    // Function names used as values expose their FunctionType
                    if let Some(func_type_id) = type_id {
                        self.set_node_type(node_idx, func_type_id);
                    }
                }
            }
        }
    }

    /// Helper: Build function signature string from function declaration
    /// Returns signature like "(Type1, Type2) -> RetType" or "()" or "() -> Type"
    fn build_function_signature(&self, func_decl_idx: usize) -> String {
        let decl = self.ast.function_decl(func_decl_idx);

        let param_types: Vec<String> = decl
            .params()
            .map(|p| {
                p.type_annotation()
                    .map(str::to_string)
                    .unwrap_or_else(|| "?".to_string())
            })
            .collect();

        let return_type = decl
            .return_type_annotation()
            .map(|t| format!(" -> {}", t))
            .unwrap_or_default();

        format!("({}){}", param_types.join(", "), return_type)
    }

    /// Builds a FunctionType from function declaration
    /// Returns TypeId for the interned function type
    /// Parameters without type annotations get Type::Unknown for later inference
    /// Return type without annotation gets Type::Unknown for later inference
    fn build_function_type(&mut self, func_decl_idx: usize) -> TypeId {
        let decl = self.ast.function_decl(func_decl_idx);

        // Collect param data before mutably borrowing self
        let param_data: Vec<(String, Option<String>)> = decl
            .params()
            .map(|p| {
                (
                    p.name().map(str::to_string).unwrap_or_default(),
                    p.type_annotation().map(str::to_string),
                )
            })
            .collect();
        let return_type_name = decl.return_type_annotation().map(str::to_string);

        // Build parameter list with TypeIds
        let mut params = Vec::new();
        for (param_name, type_name) in param_data {
            let type_id = match type_name {
                Some(name) => self
                    .lookup_type_id(&name)
                    .unwrap_or_else(|_| self.type_registry.intern(Type::Unknown)),
                None => self.type_registry.intern(Type::Unknown),
            };
            params.push(FunctionParam { name: param_name, type_id });
        }

        // Get return type
        let return_type = match return_type_name {
            Some(name) => self
                .lookup_type_id(&name)
                .unwrap_or_else(|_| self.type_registry.intern(Type::Unknown)),
            None => self.type_registry.intern(Type::Unknown),
        };

        let func_type = FunctionType { params, return_type };
        self.type_registry.intern(Type::Function(func_type))
    }

    /// Visits function declaration
    /// Registers function in current scope and adds parameters to function scope
    pub(super) fn visit_function_decl(&mut self, node_idx: usize) {
        let decl = self.ast.function_decl(node_idx);

        let Some(name) = decl.name() else {
            return;
        };
        let name = name.to_string();
        let ident_idx = decl.ident_idx().unwrap();
        let type_params_idx = decl.type_params_idx();

        // Build function signature string (for backward compatibility)
        let signature = self.build_function_signature(node_idx);

        // Build structured function type
        let func_type_id = self.build_function_type(node_idx);

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

        // Insert function symbol in current scope with structured type
        let symbol = Symbol::new(name.clone(), Some(signature), SymbolKind::Function)
            .with_type_id(func_type_id);
        self.scopes.insert(symbol);

        // Enter function context for return type tracking
        self.enter_function_context(node_idx);

        // Enter function scope
        self.scopes.enter_scope(super::ScopeKind::Function);

        // Check for TypeParams and register type parameters in scope
        if let Some(tp_idx) = type_params_idx {
            self.extract_type_params(tp_idx);
        }

        // Register parameters in function scope
        let param_data: Vec<(String, Option<String>)> = self
            .ast
            .function_decl(node_idx)
            .params()
            .map(|p| {
                (
                    p.name().map(str::to_string).unwrap_or_default(),
                    p.type_annotation().map(str::to_string),
                )
            })
            .collect();

        for (param_name, param_type) in param_data {
            let param_symbol = Symbol::new(param_name, param_type, SymbolKind::Variable);
            self.scopes.insert(param_symbol);
        }

        // Register parameter types in variable_types for type checking
        let func_type_clone = self.type_registry.resolve(func_type_id).clone();
        if let Type::Function(ref ft) = func_type_clone {
            for param in &ft.params {
                self.record_variable_type(&param.name, param.type_id);
            }
        }

        // Find and visit function body (Block node)
        let block_idx = self.ast.function_decl(node_idx).body_idx();

        if let Some(block_idx) = block_idx {
            // Visit block children directly (don't create another Block scope)
            self.visit_children(block_idx);
        }

        // Validate return types
        self.validate_function_returns(node_idx, func_type_id);

        // Clear type params if we set them
        if type_params_idx.is_some() {
            self.current_type_params.clear();
        }

        // Exit function scope
        self.scopes.exit_scope();

        // Exit function context
        self.exit_function_context();
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

        // Type check function call
        self.type_check_function_call(node_idx);
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
    fn test_var_decl_redeclaration_at_file_level() {
        // Variable redeclaration at file level should fail (constants cannot be redeclared)
        let source = "x Number: 42\nx String: \"hello\"";
        let result = analyze_source(source);
        assert!(result.is_err(), "Constant redeclaration should fail");
        let errors = result.unwrap_err();
        assert!(
            errors[0].message.contains("Cannot redeclare constant"),
            "Expected 'Cannot redeclare constant' error, got: {}",
            errors[0].message
        );
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
                return result
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Recursive function call should succeed: {:?}",
            result.err()
        );
    }
}

/// Tests for variable declaration type checking
#[cfg(test)]
mod variable_type_tests {
    use super::*;
    use crate::lexer::lex;
    use crate::limits::CompilerLimits;
    use crate::parser::parse;
    use crate::semantic::{FloatSize, IntSize, Type};

    /// Helper to analyze variable declaration and return its type
    fn analyze_var_decl(source: &str) -> Result<Type, Vec<SemanticError>> {
        let limits = CompilerLimits::default();
        let tokens =
            lex(source, &limits).map_err(|e| vec![SemanticError::new(format!("{:?}", e), 0, 0)])?;
        let ast = parse(tokens, &limits)
            .map_err(|e| vec![SemanticError::new(format!("{:?}", e), 0, 0)])?;

        // Navigate to VarDecl node
        let root_idx = ast
            .root
            .ok_or(vec![SemanticError::new("No root".to_string(), 0, 0)])?;
        let decl_idx = ast.nodes[root_idx]
            .first_child
            .ok_or(vec![SemanticError::new("No declaration".to_string(), 0, 0)])?;

        let mut analyzer = SemanticAnalyzer::new(ast);

        // Run 3-phase analysis
        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
            analyzer.solve_constraints()?;
            analyzer.apply_substitution();
        }

        let type_id = analyzer
            .get_node_type(decl_idx)
            .ok_or(vec![SemanticError::new("No type".to_string(), 0, 0)])?;
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

    // ========== Group 3: Constant Redeclaration ==========
    // File-level variables are constants and cannot be redeclared

    #[test]
    fn test_constant_redecl_same_type_fails() {
        // Constants cannot be redeclared even with same type
        let source = "x Number: 42\nx Number: 99";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let analyzer = SemanticAnalyzer::new(ast);
        let result = analyzer.analyze();
        assert!(result.is_err(), "Constant redeclaration should fail");
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("Cannot redeclare constant"));
    }

    #[test]
    fn test_constant_redecl_different_type_fails() {
        // Constants cannot be redeclared with different type
        let source = "x Number: 42\nx String: \"hello\"";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let analyzer = SemanticAnalyzer::new(ast);
        let result = analyzer.analyze();
        assert!(result.is_err(), "Constant redeclaration should fail");
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("Cannot redeclare constant"));
    }

    #[test]
    fn test_constant_redecl_annotation_to_inferred_fails() {
        // Constants cannot be redeclared even from annotated to inferred
        let source = "x Number: 42\nx: \"hello\"";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let analyzer = SemanticAnalyzer::new(ast);
        let result = analyzer.analyze();
        assert!(result.is_err(), "Constant redeclaration should fail");
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("Cannot redeclare constant"));
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

/// Tests for function signature analysis
#[cfg(test)]
mod function_signature_tests {
    use crate::lexer::lex;
    use crate::limits::CompilerLimits;
    use crate::parser::parse;
    use crate::semantic::{FunctionType, SemanticAnalyzer, SymbolKind, Type};

    /// Helper to get function type from analyzed source
    fn get_function_type(source: &str, func_name: &str) -> Option<FunctionType> {
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).ok()?;
        let ast = parse(tokens, &limits).ok()?;
        let mut analyzer = SemanticAnalyzer::new(ast);

        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
        }

        let symbol = analyzer.scopes.lookup(func_name)?;
        let type_id = symbol.type_id?;
        let ty = analyzer.type_registry.resolve(type_id).clone();

        match ty {
            Type::Function(ft) => Some(ft),
            _ => None,
        }
    }

    /// Helper to check if a TypeId resolves to Unknown
    fn is_unknown_type(analyzer: &SemanticAnalyzer, type_id: crate::semantic::TypeId) -> bool {
        matches!(analyzer.type_registry.resolve(type_id), Type::Unknown)
    }

    /// Helper to check if a TypeId resolves to a specific type name
    fn type_resolves_to(
        analyzer: &SemanticAnalyzer,
        type_id: crate::semantic::TypeId,
        expected: &Type,
    ) -> bool {
        analyzer.type_registry.resolve(type_id) == expected
    }

    // ========== Group 1: Basic Function Types ==========

    #[test]
    fn test_function_no_params_no_return() {
        let ft = get_function_type("foo: () { }", "foo").unwrap();
        assert!(ft.params.is_empty(), "Expected no parameters");
    }

    #[test]
    fn test_function_single_typed_param() {
        let source = "greet: (name String) { }";
        let ft = get_function_type(source, "greet").unwrap();

        assert_eq!(ft.params.len(), 1);
        assert_eq!(ft.params[0].name, "name");
    }

    #[test]
    fn test_function_multiple_typed_params() {
        let source = "add: (x Number, y Number) { }";
        let ft = get_function_type(source, "add").unwrap();

        assert_eq!(ft.params.len(), 2);
        assert_eq!(ft.params[0].name, "x");
        assert_eq!(ft.params[1].name, "y");
    }

    // ========== Group 2: Untyped Parameters ==========

    #[test]
    fn test_function_untyped_param_marked_unknown() {
        let source = "identity: (x) { }";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let mut analyzer = SemanticAnalyzer::new(ast);

        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
        }

        let symbol = analyzer.scopes.lookup("identity").unwrap();
        let type_id = symbol.type_id.unwrap();
        let ty = analyzer.type_registry.resolve(type_id).clone();

        if let Type::Function(ft) = ty {
            assert_eq!(ft.params.len(), 1);
            assert_eq!(ft.params[0].name, "x");
            // Parameter should be Unknown for inference
            assert!(is_unknown_type(&analyzer, ft.params[0].type_id));
        } else {
            panic!("Expected Function type");
        }
    }

    #[test]
    fn test_function_mixed_typed_untyped_params() {
        let source = "mixed: (x Number, y) { }";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let mut analyzer = SemanticAnalyzer::new(ast);

        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
        }

        let symbol = analyzer.scopes.lookup("mixed").unwrap();
        let type_id = symbol.type_id.unwrap();
        let ty = analyzer.type_registry.resolve(type_id).clone();

        if let Type::Function(ft) = ty {
            assert_eq!(ft.params.len(), 2);
            assert_eq!(ft.params[0].name, "x");
            assert_eq!(ft.params[1].name, "y");
            // First param should be Number
            assert!(type_resolves_to(
                &analyzer,
                ft.params[0].type_id,
                &Type::Number
            ));
            // Second param should be Unknown
            assert!(is_unknown_type(&analyzer, ft.params[1].type_id));
        } else {
            panic!("Expected Function type");
        }
    }

    // ========== Group 3: Return Types ==========

    #[test]
    fn test_function_with_return_type() {
        let source = "getNum: () Number { }";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let mut analyzer = SemanticAnalyzer::new(ast);

        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
        }

        let symbol = analyzer.scopes.lookup("getNum").unwrap();
        let type_id = symbol.type_id.unwrap();
        let ty = analyzer.type_registry.resolve(type_id).clone();

        if let Type::Function(ft) = ty {
            assert!(ft.params.is_empty());
            // Return type should be Number
            assert!(type_resolves_to(&analyzer, ft.return_type, &Type::Number));
        } else {
            panic!("Expected Function type");
        }
    }

    #[test]
    fn test_function_no_return_type_marked_unknown() {
        let source = "doSomething: () { }";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let mut analyzer = SemanticAnalyzer::new(ast);

        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
        }

        let symbol = analyzer.scopes.lookup("doSomething").unwrap();
        let type_id = symbol.type_id.unwrap();
        let ty = analyzer.type_registry.resolve(type_id).clone();

        if let Type::Function(ft) = ty {
            // Return type should be Unknown for inference
            assert!(is_unknown_type(&analyzer, ft.return_type));
        } else {
            panic!("Expected Function type");
        }
    }

    // ========== Group 4: Complex Function Signatures ==========

    #[test]
    fn test_function_full_signature() {
        let source = "compute: (a Number, b String, c Bool) Number { }";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let mut analyzer = SemanticAnalyzer::new(ast);

        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
        }

        let symbol = analyzer.scopes.lookup("compute").unwrap();
        let type_id = symbol.type_id.unwrap();
        let ty = analyzer.type_registry.resolve(type_id).clone();

        if let Type::Function(ft) = ty {
            assert_eq!(ft.params.len(), 3);
            assert_eq!(ft.params[0].name, "a");
            assert_eq!(ft.params[1].name, "b");
            assert_eq!(ft.params[2].name, "c");
            assert!(type_resolves_to(
                &analyzer,
                ft.params[0].type_id,
                &Type::Number
            ));
            assert!(type_resolves_to(
                &analyzer,
                ft.params[1].type_id,
                &Type::String
            ));
            assert!(type_resolves_to(
                &analyzer,
                ft.params[2].type_id,
                &Type::Bool
            ));
            assert!(type_resolves_to(&analyzer, ft.return_type, &Type::Number));
        } else {
            panic!("Expected Function type");
        }
    }

    // ========== Group 5: User-Defined Types ==========

    #[test]
    fn test_function_with_user_defined_type() {
        let source = r#"
            type UserId: Number
            getUser: (id UserId) String { }
        "#;
        let ft = get_function_type(source, "getUser").unwrap();

        assert_eq!(ft.params.len(), 1);
        assert_eq!(ft.params[0].name, "id");
        // Type should be resolved (not Unknown)
    }

    // ========== Group 6: Backward Compatibility ==========

    #[test]
    fn test_string_signature_still_present() {
        let source = "add: (x Number, y Number) Number { }";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let mut analyzer = SemanticAnalyzer::new(ast);

        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
        }

        let symbol = analyzer.scopes.lookup("add").unwrap();

        // String signature should still be present
        assert!(symbol.type_name.is_some());
        let sig = symbol.type_name.as_ref().unwrap();
        assert!(
            sig.contains("Number"),
            "Signature should contain 'Number': {}",
            sig
        );

        // TypeId should also be present
        assert!(symbol.type_id.is_some());
        assert_eq!(symbol.kind, SymbolKind::Function);
    }

    // ========== Group 7: Edge Cases ==========

    #[test]
    fn test_nested_function_has_type() {
        let source = r#"
            outer: () {
                inner: (x Number) Bool { }
            }
        "#;
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let mut analyzer = SemanticAnalyzer::new(ast);

        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
        }

        // Outer function should have type
        let outer_symbol = analyzer.scopes.lookup("outer").unwrap();
        assert!(outer_symbol.type_id.is_some());
    }
}
