//! Generic type parameter processing for semantic analysis
//!
//! This module handles processing of generic type declarations:
//! - Extracting type parameters from TypeParams AST nodes
//! - Validating constraint types exist
//! - Detecting duplicate type parameter names
//!
//! # Example
//!
//! ```suru
//! type Container<T: Number>: { value T }
//! type Map<K, V>: { size Number }
//! ```

use super::{SemanticAnalyzer, SemanticError, Type, TypeId};
use crate::ast::NodeType;

impl SemanticAnalyzer {
    /// Extracts type parameters from a TypeParams AST node
    ///
    /// Iterates through TypeParam children, creating Type::TypeParameter for each.
    /// Validates constraints exist as types and checks for duplicate parameter names.
    ///
    /// Returns a Vec of (param_name, TypeId) pairs where each TypeId points to
    /// a Type::TypeParameter in the registry.
    ///
    /// AST structure:
    /// ```text
    /// TypeParams
    ///   TypeParam
    ///     Identifier 'T'
    ///     [TypeConstraint 'Orderable']  // optional
    ///   TypeParam
    ///     Identifier 'V'
    /// ```
    pub(super) fn extract_type_params(
        &mut self,
        type_params_idx: usize,
    ) -> Result<Vec<(String, TypeId)>, SemanticError> {
        let mut params: Vec<(String, TypeId)> = Vec::new();
        let mut current_child = self.ast.nodes[type_params_idx].first_child;

        while let Some(param_idx) = current_child {
            if self.ast.nodes[param_idx].node_type != NodeType::TypeParam {
                let token = self.ast.nodes[param_idx].token.as_ref().unwrap();
                return Err(SemanticError::from_token(
                    "Expected type parameter".to_string(),
                    token,
                ));
            }

            // Get parameter name (first child is Identifier)
            let Some(name_idx) = self.ast.nodes[param_idx].first_child else {
                let token = self.ast.nodes[param_idx]
                    .token
                    .as_ref()
                    .or_else(|| self.ast.nodes[type_params_idx].token.as_ref())
                    .unwrap();
                return Err(SemanticError::from_token(
                    "Type parameter missing name".to_string(),
                    token,
                ));
            };

            let Some(param_name) = self.ast.node_text(name_idx) else {
                let token = self.ast.nodes[name_idx].token.as_ref().unwrap();
                return Err(SemanticError::from_token(
                    "Type parameter missing name".to_string(),
                    token,
                ));
            };
            let param_name = param_name.to_string();

            // Check for duplicate parameter names
            if params.iter().any(|(n, _)| n == &param_name) {
                let token = self.ast.nodes[name_idx].token.as_ref().unwrap();
                return Err(SemanticError::from_token(
                    format!("Duplicate type parameter '{}'", param_name),
                    token,
                ));
            }

            // Check for optional constraint (next sibling is TypeConstraint)
            let constraint: Option<TypeId> =
                if let Some(constraint_idx) = self.ast.nodes[name_idx].next_sibling {
                    if self.ast.nodes[constraint_idx].node_type == NodeType::TypeConstraint {
                        let Some(constraint_name) = self.ast.node_text(constraint_idx) else {
                            let token = self.ast.nodes[constraint_idx].token.as_ref().unwrap();
                            return Err(SemanticError::from_token(
                                "Type constraint missing name".to_string(),
                                token,
                            ));
                        };
                        let constraint_name = constraint_name.to_string();

                        // Validate constraint type exists
                        if !self.type_exists(&constraint_name) {
                            let token = self.ast.nodes[constraint_idx].token.as_ref().unwrap();
                            return Err(SemanticError::from_token(
                                format!("Constraint type '{}' is not defined", constraint_name),
                                token,
                            ));
                        }

                        Some(self.lookup_type_id(&constraint_name)?)
                    } else {
                        None
                    }
                } else {
                    None
                };

            // Create and intern the TypeParameter type
            let type_param_id = self.type_registry.intern(Type::TypeParameter {
                name: param_name.clone(),
                constraint,
            });

            params.push((param_name, type_param_id));

            current_child = self.ast.nodes[param_idx].next_sibling;
        }

        Ok(params)
    }
}

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

    // ========== Valid Generic Declarations ==========

    #[test]
    fn test_generic_struct_single_param() {
        let result = analyze_source("type Box<T>: { value T }\n");
        assert!(
            result.is_ok(),
            "Basic generic struct should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_generic_struct_multiple_params() {
        let result = analyze_source("type Pair<A, B>: { first A, second B }\n");
        assert!(
            result.is_ok(),
            "Multi-param generic should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_generic_struct_three_params() {
        let result = analyze_source("type Triple<A, B, C>: { count Number }\n");
        assert!(
            result.is_ok(),
            "Three-param generic should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_generic_with_constraint_builtin() {
        let result = analyze_source("type Container<T: Number>: { value T }\n");
        assert!(
            result.is_ok(),
            "Constrained generic should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_generic_with_constraint_user_type() {
        let source = r#"
            type Orderable: { compare: (other Number) Number }
            type Comparable<T: Orderable>: { value T }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "User-type constraint should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_generic_method_uses_type_param() {
        let source = r#"
            type Comparable<T>: {
                value T
                compare: (other T) Number
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Method using type param should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_generic_mixed_field_types() {
        let source = r#"
            type Container<T>: {
                value T
                label String
                count Number
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Mixed field types should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_generic_mixed_constrained_and_unconstrained() {
        let source = r#"
            type KeyValue<K: String, V>: {
                key K
                value V
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Mixed constraints should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_generic_union_type() {
        let source = r#"
            type Ok
            type Error
            type Result<T, E>: Ok, Error
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Generic union should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_generic_type_alias() {
        let result = analyze_source("type Wrapper<T>: T\n");
        assert!(
            result.is_ok(),
            "Generic alias should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_generic_phantom_type() {
        let result = analyze_source("type Phantom<T>\n");
        assert!(
            result.is_ok(),
            "Phantom generic should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_generic_and_non_generic_coexist() {
        let source = r#"
            type Name: String
            type Box<T>: { value T }
            type Status: { code Number }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Mixed generic/non-generic should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_generic_constraint_references_earlier_type() {
        let source = r#"
            type Printable: { print: () String }
            type Logger<T: Printable>: {
                count Number
            }
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Constraint referencing earlier type should succeed: {:?}",
            result.err()
        );
    }

    // ========== Error Cases ==========

    #[test]
    fn test_generic_constraint_undefined_type() {
        let result = analyze_source("type Bad<T: UndefinedType>: { value T }\n");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("UndefinedType"));
        assert!(errors[0].message.contains("not defined"));
    }

    #[test]
    fn test_generic_duplicate_param_names() {
        let result = analyze_source("type Dup<T, T>: { x T }\n");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("Duplicate type parameter"));
        assert!(errors[0].message.contains("'T'"));
    }

    #[test]
    fn test_generic_type_params_dont_leak() {
        let source = r#"
            type Box<T>: { value T }
            type Bad: { value T }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("'T' is not defined"));
    }

    #[test]
    fn test_generic_duplicate_type_name() {
        let source = r#"
            type List<T>: { count Number }
            type List<T>: { size Number }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("Duplicate declaration"));
    }
}
