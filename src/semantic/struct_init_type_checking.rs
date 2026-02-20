//! Struct initialization type checking for semantic analysis
//!
//! This module handles type checking of struct literals (StructInit nodes), including:
//! - Type inference from field and method initializations
//! - Type checking against expected struct types
//! - Validation that required fields/methods are present
//!
//! # Example
//!
//! ```suru
//! type Person: {
//!     name String
//!     age Number
//!     greet: () String
//! }
//!
//! p Person: {
//!     name: "Paul"
//!     age: 30
//!     greet: () String { return "hello" }
//! }
//! ```

use super::{
    FunctionParam, FunctionType, SemanticAnalyzer, SemanticError, StructField, StructMethod,
    StructType, Type, TypeId,
};
use crate::ast::{NodeFlags, NodeType};

impl SemanticAnalyzer {
    /// Visits a struct initialization literal
    ///
    /// Infers the struct type from its field and method initializations,
    /// then associates the inferred TypeId with the node.
    ///
    /// AST structure:
    /// ```text
    /// StructInit
    ///   StructInitField
    ///     Identifier 'fieldName'
    ///     <Expression>  (value)
    ///   StructInitMethod
    ///     Identifier 'methodName'
    ///     FunctionDecl  (implementation)
    /// ```
    pub(super) fn visit_struct_init(&mut self, node_idx: usize) {
        // Two-pass approach for proper 'this' keyword support:
        // Pass 1: Collect field values and method signatures (without visiting method bodies)
        // Pass 2: Visit method bodies with 'this' set to the real struct type

        // Pass 1: Collect signatures
        let (fields, methods, method_func_decls) =
            self.collect_struct_init_signatures(node_idx);

        // Build inferred StructType
        let struct_type = StructType { fields, methods };
        let type_id = self.type_registry.intern(Type::Struct(struct_type));

        // Pass 2: Visit method bodies with 'this' context set to the real struct type
        let previous_struct_type = self.current_struct_type;
        self.current_struct_type = Some(type_id);

        for func_decl_idx in method_func_decls {
            self.visit_function_decl(func_decl_idx);
        }

        // Restore previous struct type context (handles nested structs)
        self.current_struct_type = previous_struct_type;

        // Set the node type for use by parent context
        self.set_node_type(node_idx, type_id);
    }

    /// Collects field types and method signatures from a struct init's children
    ///
    /// This is Pass 1: it visits field value expressions to infer their types and
    /// builds method signatures from FunctionDecl nodes, but does NOT visit method
    /// bodies. Returns the list of method FunctionDecl indices for Pass 2.
    fn collect_struct_init_signatures(
        &mut self,
        struct_init_idx: usize,
    ) -> (Vec<StructField>, Vec<StructMethod>, Vec<usize>) {
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut method_func_decls = Vec::new();

        let child_indices: Vec<usize> = self.ast.children(struct_init_idx).collect();

        for child_idx in child_indices {
            match self.ast.nodes[child_idx].node_type {
                NodeType::StructInitField => {
                    if let Some(field) = self.process_struct_init_field(child_idx) {
                        fields.push(field);
                    }
                }
                NodeType::StructInitMethod => {
                    if let Some((method, func_decl_idx)) =
                        self.process_struct_init_method_signature(child_idx)
                    {
                        methods.push(method);
                        method_func_decls.push(func_decl_idx);
                    }
                }
                _ => {
                    // Other node types - visit children for nested expressions
                    self.visit_children(child_idx);
                }
            }
        }

        (fields, methods, method_func_decls)
    }

    /// Processes a single field initialization in a struct literal
    ///
    /// AST structure:
    /// ```text
    /// StructInitField
    ///   Identifier 'fieldName'
    ///   <Expression>  (value)
    /// ```
    fn process_struct_init_field(&mut self, field_idx: usize) -> Option<StructField> {
        // Check privacy flag on the StructInitField node
        let is_private = self.ast.nodes[field_idx]
            .flags
            .contains(NodeFlags::IS_PRIVATE);

        // First child is field name (Identifier)
        let name_idx = self.ast.nodes[field_idx].first_child?;

        let field_name = self.ast.node_text(name_idx)?.to_string();

        // Second child is the value expression
        let value_idx = self.ast.nodes[name_idx].next_sibling?;

        // Visit the value expression to infer its type
        self.visit_node(value_idx);

        // Get the inferred type from the value
        let type_id = self.get_node_type(value_idx)?;

        Some(StructField {
            name: field_name,
            type_id,
            is_private,
        })
    }

    /// Processes a single method initialization's signature in a struct literal
    ///
    /// Builds the method signature (StructMethod) and returns the FunctionDecl index
    /// for later body analysis. Does NOT visit the method body - that happens in Pass 2
    /// after the struct type is built, so 'this' can resolve to the real struct type.
    ///
    /// AST structure:
    /// ```text
    /// StructInitMethod
    ///   Identifier 'methodName'
    ///   FunctionDecl
    ///     Identifier (function name)
    ///     ParamList
    ///     TypeAnnotation (return type, optional)
    ///     Block
    /// ```
    fn process_struct_init_method_signature(
        &mut self,
        method_idx: usize,
    ) -> Option<(StructMethod, usize)> {
        // Check privacy flag on the StructInitMethod node
        let is_private = self.ast.nodes[method_idx]
            .flags
            .contains(NodeFlags::IS_PRIVATE);

        // First child is method name (Identifier)
        let name_idx = self.ast.nodes[method_idx].first_child?;

        let method_name = self.ast.node_text(name_idx)?.to_string();

        // Second child is FunctionDecl
        let func_decl_idx = self.ast.nodes[name_idx].next_sibling?;

        if self.ast.nodes[func_decl_idx].node_type != NodeType::FunctionDecl {
            // Unexpected node type - record error and return None
            if let Some(token) = &self.ast.nodes[func_decl_idx].token {
                self.record_error(SemanticError::from_token(
                    format!(
                        "Expected function declaration for method '{}', found {:?}",
                        method_name, self.ast.nodes[func_decl_idx].node_type
                    ),
                    token,
                ));
            }
            return None;
        }

        // Build the function type from the FunctionDecl (signature only, no body visit)
        let function_type_id = self.build_function_type_from_decl(func_decl_idx);

        let method = StructMethod {
            name: method_name,
            function_type: function_type_id,
            is_private,
        };

        Some((method, func_decl_idx))
    }

    /// Builds a FunctionType from a FunctionDecl node
    ///
    /// This is similar to build_function_type in name_resolution.rs
    /// but adapted for struct init method context.
    ///
    /// AST structure:
    /// ```text
    /// FunctionDecl
    ///   Identifier (function name)
    ///   ParamList
    ///     Param
    ///       Identifier 'paramName'
    ///       TypeAnnotation 'ParamType' (optional)
    ///   TypeAnnotation 'ReturnType' (optional)
    ///   Block
    /// ```
    fn build_function_type_from_decl(&mut self, func_decl_idx: usize) -> TypeId {
        // Get ParamList (second child after function name)
        let Some(ident_idx) = self.ast.nodes[func_decl_idx].first_child else {
            return self.type_registry.intern(Type::Unknown);
        };

        let Some(param_list_idx) = self.ast.nodes[ident_idx].next_sibling else {
            return self.type_registry.intern(Type::Unknown);
        };

        if self.ast.nodes[param_list_idx].node_type != NodeType::ParamList {
            return self.type_registry.intern(Type::Unknown);
        }

        // Build parameter list with TypeIds
        let mut params = Vec::new();
        if let Some(first_param_idx) = self.ast.nodes[param_list_idx].first_child {
            let mut current_param_idx = first_param_idx;
            loop {
                if let Some(param_ident_idx) = self.ast.nodes[current_param_idx].first_child {
                    // Get parameter name
                    let param_name = self
                        .ast
                        .node_text(param_ident_idx)
                        .map(|s| s.to_string())
                        .unwrap_or_default();

                    // Get type: annotation or Unknown for inference
                    let type_id =
                        if let Some(type_ann_idx) = self.ast.nodes[param_ident_idx].next_sibling {
                            if self.ast.nodes[type_ann_idx].node_type == NodeType::TypeAnnotation {
                                if let Some(type_name) =
                                    self.ast.node_text(type_ann_idx).map(|s| s.to_string())
                                {
                                    self.lookup_type_id(&type_name)
                                        .unwrap_or_else(|_| self.type_registry.intern(Type::Unknown))
                                } else {
                                    self.type_registry.intern(Type::Unknown)
                                }
                            } else {
                                self.type_registry.intern(Type::Unknown)
                            }
                        } else {
                            self.type_registry.intern(Type::Unknown)
                        };

                    params.push(FunctionParam {
                        name: param_name,
                        type_id,
                    });
                }

                if let Some(next) = self.ast.nodes[current_param_idx].next_sibling {
                    current_param_idx = next;
                } else {
                    break;
                }
            }
        }

        // Get return type (after ParamList, if TypeAnnotation exists)
        let return_type = if let Some(after_params_idx) = self.ast.nodes[param_list_idx].next_sibling
        {
            if self.ast.nodes[after_params_idx].node_type == NodeType::TypeAnnotation {
                if let Some(type_name) = self.ast.node_text(after_params_idx).map(|s| s.to_string())
                {
                    self.lookup_type_id(&type_name)
                        .unwrap_or_else(|_| self.type_registry.intern(Type::Unknown))
                } else {
                    self.type_registry.intern(Type::Unknown)
                }
            } else {
                // No return type annotation - Unit type for void functions
                self.type_registry.intern(Type::Unit)
            }
        } else {
            // No return type annotation - Unit type for void functions
            self.type_registry.intern(Type::Unit)
        };

        // Create and intern the function type
        let func_type = FunctionType {
            params,
            return_type,
        };
        self.type_registry.intern(Type::Function(func_type))
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::lex;
    use crate::limits::CompilerLimits;
    use crate::parser::parse;
    use crate::semantic::{SemanticAnalyzer, SemanticError};

    /// Helper function to analyze source code
    fn analyze_source(source: &str) -> Result<crate::ast::Ast, Vec<SemanticError>> {
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let analyzer = SemanticAnalyzer::new(ast);
        analyzer.analyze()
    }

    // ========== Basic Field Tests ==========

    #[test]
    fn test_struct_init_single_field() {
        let source = r#"
            type Point: { x Number }
            p Point: { x: 10 }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Should accept struct init with single field: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_struct_init_multiple_fields() {
        let source = "type Point: {\n    x Number\n    y Number\n}\np Point: { x: 10, y: 20 }\n";
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Should accept struct init with multiple fields: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_struct_init_field_type_mismatch() {
        let source = r#"
            type Point: { x Number }
            p Point: { x: "hello" }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Should reject field type mismatch");
        let errors = result.unwrap_err();
        assert!(
            errors[0].message.contains("mismatch")
                || errors[0].message.contains("unify")
                || errors[0].message.contains("Missing"),
            "Error should mention type mismatch: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_struct_init_missing_field() {
        let source = "type Point: {\n    x Number\n    y Number\n}\np Point: { x: 10 }\n";
        let result = analyze_source(source);
        assert!(result.is_err(), "Should reject missing required field");
        let errors = result.unwrap_err();
        assert!(
            errors[0].message.contains("Missing field")
                || errors[0].message.contains("missing")
                || errors[0].message.contains("y"),
            "Error should mention missing field: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_struct_init_extra_field_allowed() {
        let source = r#"
            type Point: { x Number }
            p Point: { x: 10, y: 20 }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Should allow extra fields (structural subtyping): {:?}",
            result.err()
        );
    }

    // ========== Method Tests ==========

    #[test]
    fn test_struct_init_with_method() {
        let source = r#"
            type Greeter: { greet: () String }
            g Greeter: { greet: () String { return "hello" } }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Should accept struct init with method: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_struct_init_method_missing() {
        let source = r#"
            type Greeter: { greet: () String }
            g Greeter: {}
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Should reject missing required method");
        let errors = result.unwrap_err();
        assert!(
            errors[0].message.contains("Missing method")
                || errors[0].message.contains("missing")
                || errors[0].message.contains("greet"),
            "Error should mention missing method: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_struct_init_method_param_count_mismatch() {
        let source = r#"
            type Adder: { add: (x Number, y Number) Number }
            a Adder: { add: (x Number) Number { return x } }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Should reject method parameter count mismatch");
        let errors = result.unwrap_err();
        assert!(
            errors[0].message.contains("parameter count")
                || errors[0].message.contains("mismatch")
                || errors[0].message.contains("expected"),
            "Error should mention parameter count mismatch: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_struct_init_method_return_type_mismatch() {
        let source = r#"
            type Greeter: { greet: () String }
            g Greeter: { greet: () Number { return 42 } }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Should reject method return type mismatch");
        let errors = result.unwrap_err();
        assert!(
            errors[0].message.contains("mismatch")
                || errors[0].message.contains("unify")
                || errors[0].message.contains("String")
                || errors[0].message.contains("Number"),
            "Error should mention type mismatch: {}",
            errors[0].message
        );
    }

    // ========== Mixed Fields and Methods ==========

    #[test]
    fn test_struct_init_mixed_fields_and_methods() {
        let source = r#"
            type Person: {
                name String
                greet: () String
            }
            p Person: {
                name: "Paul"
                greet: () String { return "hello" }
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Should accept struct with both fields and methods: {:?}",
            result.err()
        );
    }

    // ========== Type Inference Tests ==========

    #[test]
    fn test_struct_init_inferred_type() {
        let source = r#"
            p: { x: 10, y: 20 }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Should accept struct init without type annotation: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_struct_init_nested() {
        let source = r#"
            type Point: {
                x Number
                y Number
            }
            type Line: {
                start Point
                end Point
            }
            l Line: {
                start: { x: 0, y: 0 },
                end: { x: 10, y: 10 }
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Should accept nested struct literals: {:?}",
            result.err()
        );
    }

    // ========== Complex Tests ==========

    #[test]
    fn test_struct_init_complex() {
        let source = r#"
            type Calculator: {
                value Number
                add: (x Number) Number
                reset: () Number
            }
            calc Calculator: {
                value: 0
                add: (x Number) Number { return x }
                reset: () Number { return 0 }
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Should accept complex struct with multiple methods: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_struct_init_empty() {
        let source = r#"
            type Empty: {}
            e Empty: {}
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Should accept empty struct init: {:?}",
            result.err()
        );
    }
}
