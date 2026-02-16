//! Method call type checking for semantic analysis
//!
//! This module handles:
//! - Looking up method types on struct types
//! - Type checking for method call expressions (receiver.method(args))
//! - Validating argument count and types against method signature
//! - Propagating method return type to the call expression
//! - Resolving the `this` keyword in method bodies
//!
//! Method calls in Suru:
//! - The receiver must be a struct type
//! - The method must exist on the struct
//! - Arguments must match the method's parameter count and types
//! - The result type of `receiver.method(args)` is the method's return type
//! - Privacy rules are enforced via helpers in struct_privacy.rs

use super::{DeferredMethodCheck, SemanticAnalyzer, SemanticError, Type, TypeId};

impl SemanticAnalyzer {
    /// Looks up a method's function type TypeId on a struct type
    ///
    /// Returns `Some(TypeId)` if the method exists on the struct,
    /// `None` if the method doesn't exist or the type is not a struct.
    pub(super) fn lookup_struct_method_type(
        &self,
        struct_type_id: TypeId,
        method_name: &str,
    ) -> Option<TypeId> {
        let ty = self.type_registry.resolve(struct_type_id);
        if let Type::Struct(struct_type) = ty {
            for method in &struct_type.methods {
                if method.name == method_name {
                    return Some(method.function_type);
                }
            }
        }
        None
    }

    /// Visits a method call node, checks method existence, validates arguments,
    /// enforces privacy, and propagates the return type.
    ///
    /// AST structure:
    /// ```text
    /// MethodCall
    ///   <Receiver Expression>
    ///   Identifier 'methodName'
    ///   ArgList
    ///     <arg1>
    ///     <arg2>
    /// ```
    pub(super) fn visit_method_call(&mut self, node_idx: usize) {
        // First child is the receiver expression
        let Some(receiver_idx) = self.ast.nodes[node_idx].first_child else {
            return;
        };

        // Visit receiver to resolve its type
        self.visit_node(receiver_idx);

        // Second child is the method name
        let Some(name_idx) = self.ast.nodes[receiver_idx].next_sibling else {
            return;
        };

        let Some(method_name) = self.ast.node_text(name_idx) else {
            return;
        };
        let method_name = method_name.to_string();

        // Get the receiver's type
        let Some(receiver_type_id) = self.get_node_type(receiver_idx) else {
            // Visit remaining children (ArgList) even if type unknown
            if let Some(arg_list_idx) = self.ast.nodes[name_idx].next_sibling {
                self.visit_children(arg_list_idx);
            }
            return;
        };

        // Check receiver type category before calling mutable methods
        let is_struct = matches!(self.type_registry.resolve(receiver_type_id), Type::Struct(_));
        let is_inference_type = matches!(
            self.type_registry.resolve(receiver_type_id),
            Type::Var(_) | Type::Unknown
        );
        let is_type_param = matches!(
            self.type_registry.resolve(receiver_type_id),
            Type::TypeParameter { .. }
        );

        if is_struct {
            // Check method existence
            if let Some(method_func_type_id) =
                self.lookup_struct_method_type(receiver_type_id, &method_name)
            {
                // Method exists - check privacy
                if let Some(true) = self.is_method_private(receiver_type_id, &method_name) {
                    let token = self.ast.nodes[name_idx].token.as_ref().unwrap();
                    self.record_error(SemanticError::from_token(
                        format!("Cannot access private method '{}'", method_name),
                        token,
                    ));
                }

                // Resolve the FunctionType from the method's TypeId
                let func_type = self.type_registry.resolve(method_func_type_id).clone();
                if let Type::Function(ft) = func_type {
                    // Get ArgList (third child)
                    let arg_list_idx = self.ast.nodes[name_idx].next_sibling;

                    // Count arguments
                    let arg_count = self.count_call_arguments(arg_list_idx);

                    // Validate argument count
                    if arg_count != ft.params.len() {
                        self.record_error(self.make_error(
                            format!(
                                "Method '{}' expects {} argument(s) but got {}",
                                method_name,
                                ft.params.len(),
                                arg_count
                            ),
                            node_idx,
                        ));
                    }

                    // Visit each argument and add type constraints
                    if let Some(arg_list_idx) = arg_list_idx {
                        let mut arg_idx = self.ast.nodes[arg_list_idx].first_child;
                        for param in ft.params.iter() {
                            if let Some(current_arg_idx) = arg_idx {
                                // Visit the argument to resolve its type
                                self.visit_node(current_arg_idx);
                                let param_type = self.type_registry.resolve(param.type_id);
                                // Only add constraint if parameter has a known type
                                if !matches!(param_type, Type::Unknown) {
                                    if let Some(arg_type) = self.get_node_type(current_arg_idx) {
                                        self.add_constraint(
                                            arg_type,
                                            param.type_id,
                                            current_arg_idx,
                                        );
                                    }
                                }
                                arg_idx = self.ast.nodes[current_arg_idx].next_sibling;
                            }
                        }
                        // Visit any remaining arguments beyond param count
                        while let Some(current_arg_idx) = arg_idx {
                            self.visit_node(current_arg_idx);
                            arg_idx = self.ast.nodes[current_arg_idx].next_sibling;
                        }
                    }

                    // Set return type on the MethodCall node
                    self.set_node_type(node_idx, ft.return_type);
                }
            } else {
                // Method does not exist on this struct
                let token = self.ast.nodes[name_idx].token.as_ref().unwrap();
                self.record_error(SemanticError::from_token(
                    format!("Method '{}' does not exist on struct type", method_name),
                    token,
                ));
                // Still visit arguments for error recovery
                if let Some(arg_list_idx) = self.ast.nodes[name_idx].next_sibling {
                    self.visit_children(arg_list_idx);
                }
            }
        } else if is_type_param {
            // TypeParameter receiver - defer check until after unification
            let arg_list_idx = self.ast.nodes[name_idx].next_sibling;
            let mut arg_type_ids = Vec::new();

            // Visit arguments and collect their types
            if let Some(arg_list_idx) = arg_list_idx {
                let mut arg_idx = self.ast.nodes[arg_list_idx].first_child;
                while let Some(current_arg_idx) = arg_idx {
                    self.visit_node(current_arg_idx);
                    if let Some(arg_type) = self.get_node_type(current_arg_idx) {
                        arg_type_ids.push(arg_type);
                    }
                    arg_idx = self.ast.nodes[current_arg_idx].next_sibling;
                }
            }

            // Set return type as fresh type variable (resolved later)
            let return_type_var = self.fresh_type_var();
            self.set_node_type(node_idx, return_type_var);

            // Record deferred check
            self.deferred_method_checks.push(DeferredMethodCheck {
                receiver_type_id,
                method_name: method_name.clone(),
                arg_type_ids,
                call_node_idx: node_idx,
                method_name_node_idx: name_idx,
            });
        } else if is_inference_type {
            // Type not yet known - visit args, skip checks
            if let Some(arg_list_idx) = self.ast.nodes[name_idx].next_sibling {
                self.visit_children(arg_list_idx);
            }
        } else {
            // Not a struct type - cannot call methods
            let token = self.ast.nodes[name_idx].token.as_ref().unwrap();
            self.record_error(SemanticError::from_token(
                format!(
                    "Cannot call method '{}' on non-struct type",
                    method_name
                ),
                token,
            ));
            if let Some(arg_list_idx) = self.ast.nodes[name_idx].next_sibling {
                self.visit_children(arg_list_idx);
            }
        }
    }

    /// Visits a `this` keyword node
    ///
    /// Sets the node's type to the current struct type if inside a method body.
    /// Reports an error if `this` is used outside a method context.
    pub(super) fn visit_this(&mut self, node_idx: usize) {
        if let Some(struct_type_id) = self.current_struct_type {
            self.set_node_type(node_idx, struct_type_id);
        } else {
            let token = self.ast.nodes[node_idx].token.as_ref().unwrap();
            self.record_error(SemanticError::from_token(
                "'this' can only be used inside a method body".to_string(),
                token,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::lex;
    use crate::limits::CompilerLimits;
    use crate::parser::parse;
    use crate::semantic::{
        FunctionParam, FunctionType, SemanticAnalyzer, SemanticError, StructMethod, StructType,
        Type,
    };

    fn analyze_source(source: &str) -> Result<crate::ast::Ast, Vec<SemanticError>> {
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let analyzer = SemanticAnalyzer::new(ast);
        analyzer.analyze()
    }

    // ========== Helper Unit Tests ==========

    #[test]
    fn test_lookup_struct_method_type_found() {
        let limits = CompilerLimits::default();
        let tokens = lex("", &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let mut analyzer = SemanticAnalyzer::new(ast);

        let unit_id = analyzer.type_registry.intern(Type::Unit);
        let str_id = analyzer.type_registry.intern(Type::String);
        let func_type = FunctionType {
            params: vec![],
            return_type: str_id,
        };
        let func_id = analyzer.type_registry.intern(Type::Function(func_type));

        let struct_type = StructType {
            fields: vec![],
            methods: vec![StructMethod {
                name: "greet".to_string(),
                function_type: func_id,
                is_private: false,
            }],
        };
        let struct_id = analyzer.type_registry.intern(Type::Struct(struct_type));

        assert_eq!(
            analyzer.lookup_struct_method_type(struct_id, "greet"),
            Some(func_id)
        );
        assert_eq!(
            analyzer.lookup_struct_method_type(struct_id, "nonexistent"),
            None
        );
        assert_eq!(
            analyzer.lookup_struct_method_type(unit_id, "anything"),
            None
        );
    }

    #[test]
    fn test_lookup_struct_method_type_multiple() {
        let limits = CompilerLimits::default();
        let tokens = lex("", &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let mut analyzer = SemanticAnalyzer::new(ast);

        let str_id = analyzer.type_registry.intern(Type::String);
        let num_id = analyzer.type_registry.intern(Type::Number);

        let greet_func = FunctionType {
            params: vec![],
            return_type: str_id,
        };
        let greet_id = analyzer.type_registry.intern(Type::Function(greet_func));

        let add_func = FunctionType {
            params: vec![
                FunctionParam {
                    name: "x".to_string(),
                    type_id: num_id,
                },
                FunctionParam {
                    name: "y".to_string(),
                    type_id: num_id,
                },
            ],
            return_type: num_id,
        };
        let add_id = analyzer.type_registry.intern(Type::Function(add_func));

        let struct_type = StructType {
            fields: vec![],
            methods: vec![
                StructMethod {
                    name: "greet".to_string(),
                    function_type: greet_id,
                    is_private: false,
                },
                StructMethod {
                    name: "add".to_string(),
                    function_type: add_id,
                    is_private: false,
                },
            ],
        };
        let struct_id = analyzer.type_registry.intern(Type::Struct(struct_type));

        assert_eq!(
            analyzer.lookup_struct_method_type(struct_id, "greet"),
            Some(greet_id)
        );
        assert_eq!(
            analyzer.lookup_struct_method_type(struct_id, "add"),
            Some(add_id)
        );
    }

    // ========== Method Existence Tests ==========

    #[test]
    fn test_method_call_existing_method_succeeds() {
        let source = r#"
            type Greeter: { greet: () String }
            g Greeter: { greet: () String { return "hello" } }
            x: g.greet()
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Calling existing method should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_method_call_nonexistent_method_error() {
        let source = "p: { name: \"Paul\" }\nx: p.greet()\n";
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Calling nonexistent method should fail"
        );
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Method 'greet' does not exist on struct type")),
            "Error should mention nonexistent method: {:?}",
            errors
        );
    }

    #[test]
    fn test_method_call_field_as_method_error() {
        let source = "p: { name: \"Paul\" }\nx: p.name()\n";
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Calling a field as a method should fail"
        );
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Method 'name' does not exist on struct type")),
            "Error should mention nonexistent method: {:?}",
            errors
        );
    }

    #[test]
    fn test_method_call_multiple_nonexistent() {
        let source = "p: { name: \"Paul\" }\nx: p.greet()\ny: p.wave()\n";
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Calling nonexistent methods should fail"
        );
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Method 'greet' does not exist")),
            "Should report greet error: {:?}",
            errors
        );
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Method 'wave' does not exist")),
            "Should report wave error: {:?}",
            errors
        );
    }

    // ========== Argument Count Tests ==========

    #[test]
    fn test_method_call_correct_arg_count() {
        let source = r#"
            type Adder: { add: (x Number, y Number) Number }
            a Adder: { add: (x Number, y Number) Number { return 1 } }
            result: a.add(1, 2)
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Method call with correct arg count should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_method_call_too_few_args() {
        let source = r#"
            type Adder: { add: (x Number, y Number) Number }
            a Adder: { add: (x Number, y Number) Number { return 1 } }
            result: a.add(1)
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Too few arguments should fail");
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Method 'add' expects 2 argument(s) but got 1")),
            "Expected argument count error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_method_call_too_many_args() {
        let source = r#"
            type Adder: { add: (x Number, y Number) Number }
            a Adder: { add: (x Number, y Number) Number { return 1 } }
            result: a.add(1, 2, 3)
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Too many arguments should fail");
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Method 'add' expects 2 argument(s) but got 3")),
            "Expected argument count error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_method_call_no_args_when_expected() {
        let source = r#"
            type Greeter: { greet: (name String) String }
            g Greeter: { greet: (name String) String { return "hello" } }
            result: g.greet()
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "No args when expected should fail");
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Method 'greet' expects 1 argument(s) but got 0")),
            "Expected argument count error, got: {:?}",
            errors
        );
    }

    // ========== Argument Type Tests ==========

    #[test]
    fn test_method_call_correct_arg_types() {
        let source = r#"
            type Greeter: { greet: (name String) String }
            g Greeter: { greet: (name String) String { return "hello" } }
            x: g.greet("Paul")
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Method call with correct arg types should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_method_call_wrong_arg_type() {
        let source = r#"
            type Greeter: { greet: (name String) String }
            g Greeter: { greet: (name String) String { return "hello" } }
            x: g.greet(42)
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Wrong argument type should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Type mismatch")),
            "Expected type mismatch error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_method_call_multiple_wrong_types() {
        let source = r#"
            type Adder: { add: (x Number, y Number) Number }
            a Adder: { add: (x Number, y Number) Number { return 1 } }
            result: a.add("hello", true)
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Multiple wrong arg types should fail");
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Type mismatch")),
            "Expected type mismatch error, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_method_call_untyped_params_accept_anything() {
        let source = "obj: { process: (x) { return x } }\ny: obj.process(42)\n";
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Untyped params should accept any arg: {:?}",
            result.err()
        );
    }

    // ========== Return Type Propagation Tests ==========

    #[test]
    fn test_method_call_return_type_propagated() {
        let source = r#"
            type Greeter: { greet: () String }
            g Greeter: { greet: () String { return "hello" } }
            x: g.greet()
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Method call return type should propagate: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_method_call_return_type_in_expression() {
        let source = r#"
            type Checker: { check: () Bool }
            c Checker: { check: () Bool { return true } }
            result: c.check() and true
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Method return type should work in expressions: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_method_call_return_type_mismatch_with_annotation() {
        let source = r#"
            type GetNum: { get: () Number }
            g GetNum: { get: () Number { return 42 } }
            x Bool: g.get()
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Return type mismatch with annotation should fail"
        );
        let errors = result.unwrap_err();
        assert!(
            errors.iter().any(|e| e.message.contains("Type mismatch")
                || e.message.contains("mismatch")
                || e.message.contains("unify")),
            "Expected type mismatch error, got: {:?}",
            errors
        );
    }

    // ========== Non-Struct Receiver Tests ==========

    #[test]
    fn test_method_call_on_number_error() {
        let source = "x: 42\ny: x.double()\n";
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Method call on number should fail"
        );
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Cannot call method 'double' on non-struct type")),
            "Error should mention non-struct type: {:?}",
            errors
        );
    }

    #[test]
    fn test_method_call_on_string_error() {
        let source = "x: \"hello\"\ny: x.upper()\n";
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Method call on string should fail"
        );
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Cannot call method 'upper' on non-struct type")),
            "Error should mention non-struct type: {:?}",
            errors
        );
    }

    #[test]
    fn test_method_call_on_bool_error() {
        let source = "x: true\ny: x.toggle()\n";
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Method call on bool should fail"
        );
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Cannot call method 'toggle' on non-struct type")),
            "Error should mention non-struct type: {:?}",
            errors
        );
    }

    // ========== Privacy Integration Tests ==========

    #[test]
    fn test_private_method_call_error() {
        let source = "obj: {\n    greet: () String { return \"hello\" }\n    _ validate: () Bool { return true }\n}\nx: obj.validate()\n";
        let result = analyze_source(source);
        assert!(result.is_err(), "Calling private method should fail");
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Cannot access private method 'validate'")),
            "Error should mention private method: {:?}",
            errors
        );
    }

    #[test]
    fn test_public_method_call_allowed() {
        let source = "obj: {\n    greet: () String { return \"hello\" }\n    _ validate: () Bool { return true }\n}\nx: obj.greet()\n";
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Calling public method should succeed: {:?}",
            result.err()
        );
    }

    // ========== this Keyword Tests ==========

    #[test]
    fn test_this_outside_method_error() {
        let source = "x: this\n";
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "'this' outside method should fail"
        );
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("'this' can only be used inside a method body")),
            "Error should mention this context: {:?}",
            errors
        );
    }

    #[test]
    fn test_this_in_regular_function_error() {
        let source = "foo: () { x: this }\n";
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "'this' in regular function should fail"
        );
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("'this' can only be used inside a method body")),
            "Error should mention this context: {:?}",
            errors
        );
    }

    #[test]
    fn test_this_in_method_body_field_access() {
        let source = "obj: {\n    name: \"Paul\"\n    getName: () String { return this.name }\n}\n";
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "'this' in method body should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_this_property_nonexistent_field_error() {
        let source = "obj: {\n    name: \"Paul\"\n    getBad: () String { return this.email }\n}\n";
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "'this.email' on struct without email should fail"
        );
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Field 'email' does not exist")),
            "Error should mention nonexistent field: {:?}",
            errors
        );
    }

    // ========== Chained Call Tests ==========

    #[test]
    fn test_method_call_on_property_access() {
        let source = "outer: { inner: { greet: () String { return \"hello\" } } }\nx: outer.inner.greet()\n";
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Method call on property access result should succeed: {:?}",
            result.err()
        );
    }

    // ========== Typed Struct Method Calls ==========

    #[test]
    fn test_typed_struct_method_call() {
        let source = r#"
            type Calculator: {
                add: (x Number, y Number) Number
                reset: () Number
            }
            calc Calculator: {
                add: (x Number, y Number) Number { return 1 }
                reset: () Number { return 0 }
            }
            a: calc.add(1, 2)
            b: calc.reset()
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Typed struct method calls should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_method_call_no_args_succeeds() {
        let source = r#"
            type Greeter: { greet: () String }
            g Greeter: { greet: () String { return "hello" } }
            x: g.greet()
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Method call with no args should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_inferred_struct_method_call() {
        let source = "obj: { greet: () String { return \"hello\" } }\nx: obj.greet()\n";
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Method call on inferred struct should succeed: {:?}",
            result.err()
        );
    }
}
