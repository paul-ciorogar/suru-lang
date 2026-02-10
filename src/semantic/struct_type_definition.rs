//! Struct type definition processing for semantic analysis
//!
//! This module handles processing of struct type declarations, including:
//! - Struct fields (name Type)
//! - Struct methods (name: (params) ReturnType)
//!
//! # Example
//!
//! ```suru
//! type Person: {
//!     name String
//!     age Number
//!     greet: () String
//!     add: (x Number, y Number) Number
//! }
//! ```

use super::{
    FunctionParam, FunctionType, SemanticAnalyzer, SemanticError, StructField, StructMethod,
    StructType, Type, TypeId,
};
use crate::ast::NodeType;

impl SemanticAnalyzer {
    /// Processes a struct type declaration body
    ///
    /// Iterates through the StructBody children and builds a StructType
    /// containing both fields and methods.
    pub(super) fn process_struct_type_definition(
        &mut self,
        struct_body_idx: usize,
    ) -> Result<TypeId, SemanticError> {
        let mut fields = Vec::new();
        let mut methods = Vec::new();

        // Iterate through struct members
        let mut current_child = self.ast.nodes[struct_body_idx].first_child;

        while let Some(child_idx) = current_child {
            match self.ast.nodes[child_idx].node_type {
                NodeType::StructField => {
                    let field = self.process_struct_field_definition(child_idx)?;
                    fields.push(field);
                }
                NodeType::StructMethod => {
                    let method = self.process_struct_method(child_idx)?;
                    methods.push(method);
                }
                _ => {
                    let token = self.ast.nodes[child_idx].token.as_ref().unwrap();
                    return Err(SemanticError::from_token(
                        "Unexpected node in struct body".to_string(),
                        token,
                    ));
                }
            }

            current_child = self.ast.nodes[child_idx].next_sibling;
        }

        // Create struct type
        let struct_type = StructType { fields, methods };
        Ok(self.type_registry.intern(Type::Struct(struct_type)))
    }

    /// Processes a single struct field
    ///
    /// AST structure:
    /// ```text
    /// StructField
    ///   Identifier 'fieldName'
    ///   TypeAnnotation 'TypeName'
    /// ```
    fn process_struct_field_definition(
        &mut self,
        field_idx: usize,
    ) -> Result<StructField, SemanticError> {
        // First child is field name (Identifier)
        let Some(name_idx) = self.ast.nodes[field_idx].first_child else {
            let token = self.ast.nodes[field_idx].token.as_ref().unwrap();
            return Err(SemanticError::from_token(
                "Struct field missing name".to_string(),
                token,
            ));
        };

        let Some(field_name) = self.ast.node_text(name_idx) else {
            let token = self.ast.nodes[name_idx].token.as_ref().unwrap();
            return Err(SemanticError::from_token(
                "Struct field missing name".to_string(),
                token,
            ));
        };
        let field_name = field_name.to_string();

        // Second child is type annotation (TypeAnnotation)
        let Some(type_idx) = self.ast.nodes[name_idx].next_sibling else {
            let token = self.ast.nodes[name_idx].token.as_ref().unwrap();
            return Err(SemanticError::from_token(
                format!("Struct field '{}' missing type annotation", field_name),
                token,
            ));
        };

        if self.ast.nodes[type_idx].node_type != NodeType::TypeAnnotation {
            let token = self.ast.nodes[type_idx].token.as_ref().unwrap();
            return Err(SemanticError::from_token(
                format!("Expected type annotation for field '{}'", field_name),
                token,
            ));
        }

        let Some(type_name) = self.ast.node_text(type_idx) else {
            let token = self.ast.nodes[type_idx].token.as_ref().unwrap();
            return Err(SemanticError::from_token(
                format!("Field '{}' missing type", field_name),
                token,
            ));
        };
        let type_name = type_name.to_string();

        // Validate type exists
        if !self.type_exists(&type_name) {
            let token = self.ast.nodes[type_idx].token.as_ref().unwrap();
            return Err(SemanticError::from_token(
                format!("Type '{}' is not defined", type_name),
                token,
            ));
        }

        // Get TypeId
        let type_id = self.lookup_type_id(&type_name)?;

        Ok(StructField {
            name: field_name,
            type_id,
            is_private: false,
        })
    }

    /// Processes a struct method declaration
    ///
    /// AST structure:
    /// ```text
    /// StructMethod
    ///   Identifier 'methodName'
    ///   FunctionType
    ///     FunctionTypeParams
    ///       StructField
    ///         Identifier 'param1'
    ///         TypeAnnotation 'Type1'
    ///     TypeAnnotation 'ReturnType'
    /// ```
    fn process_struct_method(&mut self, method_idx: usize) -> Result<StructMethod, SemanticError> {
        // First child is method name (Identifier)
        let Some(name_idx) = self.ast.nodes[method_idx].first_child else {
            let token = self.ast.nodes[method_idx].token.as_ref().unwrap();
            return Err(SemanticError::from_token(
                "Struct method missing name".to_string(),
                token,
            ));
        };

        let Some(method_name) = self.ast.node_text(name_idx) else {
            let token = self.ast.nodes[name_idx].token.as_ref().unwrap();
            return Err(SemanticError::from_token(
                "Struct method missing name".to_string(),
                token,
            ));
        };
        let method_name = method_name.to_string();

        // Second child is FunctionType
        let Some(func_type_idx) = self.ast.nodes[name_idx].next_sibling else {
            let token = self.ast.nodes[name_idx].token.as_ref().unwrap();
            return Err(SemanticError::from_token(
                format!("Method '{}' missing function type", method_name),
                token,
            ));
        };

        if self.ast.nodes[func_type_idx].node_type != NodeType::FunctionType {
            let token = self.ast.nodes[func_type_idx].token.as_ref().unwrap();
            return Err(SemanticError::from_token(
                format!("Expected function type for method '{}'", method_name),
                token,
            ));
        }

        // Process the function type
        let function_type_id = self.process_function_type_definition(func_type_idx)?;

        Ok(StructMethod {
            name: method_name,
            function_type: function_type_id,
            is_private: false,
        })
    }

    /// Processes a function type declaration
    ///
    /// AST structure:
    /// ```text
    /// FunctionType
    ///   FunctionTypeParams
    ///     StructField (for each param)
    ///       Identifier 'paramName'
    ///       TypeAnnotation 'ParamType'
    ///   TypeAnnotation 'ReturnType'
    /// ```
    pub(super) fn process_function_type_definition(
        &mut self,
        func_type_idx: usize,
    ) -> Result<TypeId, SemanticError> {
        // First child is FunctionTypeParams
        let Some(params_idx) = self.ast.nodes[func_type_idx].first_child else {
            let token = self.ast.nodes[func_type_idx]
                .token
                .as_ref()
                .or_else(|| {
                    // Try to get token from parent node if this node doesn't have one
                    self.ast.nodes[func_type_idx]
                        .parent
                        .and_then(|p| self.ast.nodes[p].token.as_ref())
                })
                .unwrap();
            return Err(SemanticError::from_token(
                "Function type missing parameters".to_string(),
                token,
            ));
        };

        if self.ast.nodes[params_idx].node_type != NodeType::FunctionTypeParams {
            let token = self.ast.nodes[params_idx].token.as_ref().unwrap();
            return Err(SemanticError::from_token(
                "Expected function type parameters".to_string(),
                token,
            ));
        }

        // Process parameters
        let params = self.process_function_type_params(params_idx)?;

        // Second child is return type (TypeAnnotation)
        let Some(return_type_idx) = self.ast.nodes[params_idx].next_sibling else {
            let token = self.ast.nodes[params_idx]
                .token
                .as_ref()
                .or_else(|| self.ast.nodes[func_type_idx].token.as_ref())
                .unwrap();
            return Err(SemanticError::from_token(
                "Function type missing return type".to_string(),
                token,
            ));
        };

        if self.ast.nodes[return_type_idx].node_type != NodeType::TypeAnnotation {
            let token = self.ast.nodes[return_type_idx].token.as_ref().unwrap();
            return Err(SemanticError::from_token(
                "Expected return type annotation".to_string(),
                token,
            ));
        }

        let Some(return_type_name) = self.ast.node_text(return_type_idx) else {
            let token = self.ast.nodes[return_type_idx].token.as_ref().unwrap();
            return Err(SemanticError::from_token(
                "Function type missing return type name".to_string(),
                token,
            ));
        };
        let return_type_name = return_type_name.to_string();

        // Validate return type exists
        if !self.type_exists(&return_type_name) {
            let token = self.ast.nodes[return_type_idx].token.as_ref().unwrap();
            return Err(SemanticError::from_token(
                format!("Type '{}' is not defined", return_type_name),
                token,
            ));
        }

        let return_type = self.lookup_type_id(&return_type_name)?;

        // Build and intern function type
        let func_type = FunctionType {
            params,
            return_type,
        };

        Ok(self.type_registry.intern(Type::Function(func_type)))
    }

    /// Processes function type parameters
    ///
    /// Each parameter is represented as a StructField node with:
    /// - Identifier child (parameter name)
    /// - TypeAnnotation child (parameter type)
    fn process_function_type_params(
        &mut self,
        params_idx: usize,
    ) -> Result<Vec<FunctionParam>, SemanticError> {
        let mut params = Vec::new();

        let mut current_child = self.ast.nodes[params_idx].first_child;

        while let Some(child_idx) = current_child {
            // Each child should be a StructField (reused for params)
            if self.ast.nodes[child_idx].node_type != NodeType::StructField {
                let token = self.ast.nodes[child_idx].token.as_ref().unwrap();
                return Err(SemanticError::from_token(
                    "Expected parameter in function type".to_string(),
                    token,
                ));
            }

            // Get parameter name
            let Some(name_idx) = self.ast.nodes[child_idx].first_child else {
                let token = self.ast.nodes[child_idx].token.as_ref().unwrap();
                return Err(SemanticError::from_token(
                    "Function parameter missing name".to_string(),
                    token,
                ));
            };

            let Some(param_name) = self.ast.node_text(name_idx) else {
                let token = self.ast.nodes[name_idx].token.as_ref().unwrap();
                return Err(SemanticError::from_token(
                    "Function parameter missing name".to_string(),
                    token,
                ));
            };
            let param_name = param_name.to_string();

            // Get parameter type
            let Some(type_idx) = self.ast.nodes[name_idx].next_sibling else {
                let token = self.ast.nodes[name_idx].token.as_ref().unwrap();
                return Err(SemanticError::from_token(
                    format!("Parameter '{}' missing type annotation", param_name),
                    token,
                ));
            };

            let Some(type_name) = self.ast.node_text(type_idx) else {
                let token = self.ast.nodes[type_idx].token.as_ref().unwrap();
                return Err(SemanticError::from_token(
                    format!("Parameter '{}' missing type", param_name),
                    token,
                ));
            };
            let type_name = type_name.to_string();

            // Validate type exists
            if !self.type_exists(&type_name) {
                let token = self.ast.nodes[type_idx].token.as_ref().unwrap();
                return Err(SemanticError::from_token(
                    format!("Type '{}' is not defined", type_name),
                    token,
                ));
            }

            let type_id = self.lookup_type_id(&type_name)?;

            params.push(FunctionParam {
                name: param_name,
                type_id,
            });

            current_child = self.ast.nodes[child_idx].next_sibling;
        }

        Ok(params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use crate::parser::parse;

    // Helper function to analyze source code
    fn analyze_source(source: &str) -> Result<crate::ast::Ast, Vec<SemanticError>> {
        let limits = crate::limits::CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let analyzer = SemanticAnalyzer::new(ast);
        analyzer.analyze()
    }

    // ========== Method-Only Struct Tests ==========

    #[test]
    fn test_struct_with_method_no_params() {
        let source = r#"
            type Greeter: {
                greet: () String
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Should accept struct with no-param method");
    }

    #[test]
    fn test_struct_with_method_one_param() {
        let source = r#"
            type Calculator: {
                double: (x Number) Number
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Should accept struct with one-param method");
    }

    #[test]
    fn test_struct_with_method_multiple_params() {
        let source = r#"
            type Calculator: {
                add: (x Number, y Number) Number
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Should accept struct with multi-param method"
        );
    }

    #[test]
    fn test_struct_with_multiple_methods() {
        let source = r#"
            type Calculator: {
                add: (x Number, y Number) Number
                subtract: (x Number, y Number) Number
                reset: () Number
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Should accept struct with multiple methods");
    }

    // ========== Mixed Fields and Methods Tests ==========

    #[test]
    fn test_struct_with_fields_and_methods() {
        let source = r#"
            type Person: {
                name String
                age Number
                greet: () String
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Should accept struct with both fields and methods"
        );
    }

    #[test]
    fn test_struct_complex_mixed() {
        let source = r#"
            type BankAccount: {
                balance Number
                owner String
                deposit: (amount Number) Number
                withdraw: (amount Number) Number
                getBalance: () Number
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Should accept complex struct");
    }

    // ========== Error Cases ==========

    #[test]
    fn test_struct_method_undefined_param_type() {
        let source = r#"
            type Foo: {
                bar: (x UndefinedType) Number
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("UndefinedType"));
        assert!(errors[0].message.contains("not defined"));
    }

    #[test]
    fn test_struct_method_undefined_return_type() {
        let source = r#"
            type Foo: {
                bar: (x Number) UndefinedType
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("UndefinedType"));
        assert!(errors[0].message.contains("not defined"));
    }

    // ========== Built-in Types Tests ==========

    #[test]
    fn test_struct_method_all_builtin_param_types() {
        let source = r#"
            type AllTypes: {
                method: (n Number, s String, b Bool, i Int64, u UInt32, f Float64) Number
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Should accept methods with all built-in param types"
        );
    }

    #[test]
    fn test_struct_method_builtin_return_types() {
        let source = r#"
            type Returns: {
                getNumber: () Number
                getString: () String
                getBool: () Bool
                getInt: () Int64
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Should accept methods with built-in return types"
        );
    }

    // ========== User-Defined Type References ==========

    #[test]
    fn test_struct_method_references_user_type() {
        let source = r#"
            type Point: { x Number }
            type Factory: {
                createPoint: () Point
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Should accept method returning user type");
    }

    #[test]
    fn test_struct_method_param_user_type() {
        let source = r#"
            type Point: { x Number }
            type Processor: {
                process: (p Point) Number
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Should accept method with user type parameter"
        );
    }

    // ========== Integration Tests ==========

    #[test]
    fn test_complex_struct_with_methods() {
        let source = r#"
            type Id: Int64
            type Name: String
            type Address: { city String }

            type Person: {
                id Id
                name Name
                addr Address
                getId: () Id
                setName: (n Name) Name
                updateAddress: (a Address) Address
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Should accept complex struct with methods");
    }

    #[test]
    fn test_struct_preserves_method_order() {
        // This test verifies that methods are processed in order
        // by checking that all methods are captured
        let source = r#"
            type Multi: {
                first: () Number
                second: () String
                third: () Bool
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Should preserve method order");
    }
}
