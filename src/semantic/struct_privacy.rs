//! Struct privacy enforcement for semantic analysis
//!
//! This module handles:
//! - Privacy checking helpers for struct fields and methods
//! - Privacy enforcement for property access (field reads)
//! - Privacy enforcement for method calls
//!
//! Privacy in Suru:
//! - Type definitions are interfaces and do NOT have private members
//! - Only struct initializations can mark fields/methods as private using `_` prefix
//! - Private fields/methods cannot be accessed from outside the struct

use super::{SemanticAnalyzer, SemanticError, Type};

impl SemanticAnalyzer {
    /// Checks if a field is private on a struct type
    ///
    /// Returns `Some(true)` if the field exists and is private,
    /// `Some(false)` if the field exists and is public,
    /// `None` if the field doesn't exist or the type is not a struct.
    pub(super) fn is_field_private(
        &self,
        struct_type_id: super::TypeId,
        field_name: &str,
    ) -> Option<bool> {
        let ty = self.type_registry.resolve(struct_type_id);
        if let Type::Struct(struct_type) = ty {
            for field in &struct_type.fields {
                if field.name == field_name {
                    return Some(field.is_private);
                }
            }
        }
        None
    }

    /// Checks if a method is private on a struct type
    ///
    /// Returns `Some(true)` if the method exists and is private,
    /// `Some(false)` if the method exists and is public,
    /// `None` if the method doesn't exist or the type is not a struct.
    pub(super) fn is_method_private(
        &self,
        struct_type_id: super::TypeId,
        method_name: &str,
    ) -> Option<bool> {
        let ty = self.type_registry.resolve(struct_type_id);
        if let Type::Struct(struct_type) = ty {
            for method in &struct_type.methods {
                if method.name == method_name {
                    return Some(method.is_private);
                }
            }
        }
        None
    }

    /// Visits a property access node, checks field existence, enforces privacy,
    /// and propagates the field's type to the PropertyAccess node.
    ///
    /// AST structure:
    /// ```text
    /// PropertyAccess
    ///   <Receiver Expression>
    ///   Identifier 'propertyName'
    /// ```
    pub(super) fn visit_property_access(&mut self, node_idx: usize) {
        // First child is the receiver expression
        let Some(receiver_idx) = self.ast.nodes[node_idx].first_child else {
            return;
        };

        // Visit receiver to resolve its type
        self.visit_node(receiver_idx);

        // Second child is the property name
        let Some(name_idx) = self.ast.nodes[receiver_idx].next_sibling else {
            return;
        };

        let Some(property_name) = self.ast.node_text(name_idx) else {
            return;
        };
        let property_name = property_name.to_string();

        // Get the receiver's type
        let Some(receiver_type_id) = self.get_node_type(receiver_idx) else {
            return; // Type unknown - skip checks (will be resolved during inference)
        };

        // Check receiver type category before calling mutable methods
        let is_struct = matches!(self.type_registry.resolve(receiver_type_id), Type::Struct(_));
        let is_inference_type = matches!(
            self.type_registry.resolve(receiver_type_id),
            Type::Var(_) | Type::Unknown
        );

        if is_struct {
            // Check field existence and get its type
            if let Some(field_type_id) =
                self.lookup_struct_field_type(receiver_type_id, &property_name)
            {
                // Field exists - check privacy
                if let Some(true) = self.is_field_private(receiver_type_id, &property_name) {
                    let token = self.ast.nodes[name_idx].token.as_ref().unwrap();
                    self.record_error(SemanticError::from_token(
                        format!("Cannot access private field '{}'", property_name),
                        token,
                    ));
                }
                // Set the PropertyAccess node's type to the field's type
                self.set_node_type(node_idx, field_type_id);
            } else {
                // Field does not exist on this struct
                let token = self.ast.nodes[name_idx].token.as_ref().unwrap();
                self.record_error(SemanticError::from_token(
                    format!(
                        "Field '{}' does not exist on struct type",
                        property_name
                    ),
                    token,
                ));
            }
        } else if is_inference_type {
            // Type not yet known - skip checks, will be resolved during inference
        } else {
            // Not a struct type - cannot access properties
            let token = self.ast.nodes[name_idx].token.as_ref().unwrap();
            self.record_error(SemanticError::from_token(
                format!(
                    "Cannot access property '{}' on non-struct type",
                    property_name
                ),
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
        FunctionType, SemanticAnalyzer, SemanticError, StructField, StructMethod, StructType, Type,
    };

    fn analyze_source(source: &str) -> Result<crate::ast::Ast, Vec<SemanticError>> {
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let analyzer = SemanticAnalyzer::new(ast);
        analyzer.analyze()
    }

    // ========== Privacy Tracking Tests ==========

    #[test]
    fn test_struct_init_private_field_tracked() {
        let source = "p: { name: \"Paul\"\n_ secret: \"password\" }\n";
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Struct init with private field should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_struct_init_private_method_tracked() {
        let source = "obj: { _ internal: () { return 42 } }\n";
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Struct init with private method should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_struct_init_mixed_public_private() {
        let source = "user: {\n    name: \"Paul\"\n    _ password: \"secret\"\n    greet: () String { return \"hello\" }\n    _ validate: () Bool { return true }\n}\n";
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Mixed public/private should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_struct_init_multiple_private_fields() {
        let source = "config: {\n    _ dbHost: \"localhost\"\n    _ dbPort: 5432\n    _ apiKey: \"abc123\"\n}\n";
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Multiple private fields should succeed: {:?}",
            result.err()
        );
    }

    // ========== Privacy Enforcement: Field Access ==========

    #[test]
    fn test_private_field_access_error() {
        let source = "user: { name: \"Paul\", _ secret: \"password\" }\nx: user.secret\n";
        let result = analyze_source(source);
        assert!(result.is_err(), "Accessing private field should fail");
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Cannot access private field 'secret'")),
            "Error should mention private field: {:?}",
            errors
        );
    }

    #[test]
    fn test_public_field_access_allowed() {
        let source = "user: { name: \"Paul\", _ secret: \"password\" }\nx: user.name\n";
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Accessing public field should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_private_field_access_multiple() {
        let source = "config: { _ host: \"localhost\", _ port: 5432 }\nh: config.host\n";
        let result = analyze_source(source);
        assert!(result.is_err(), "Accessing private field should fail");
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Cannot access private field 'host'")),
            "Error should mention private field: {:?}",
            errors
        );
    }

    // ========== Structural Subtyping with Private Fields ==========

    #[test]
    fn test_typed_struct_init_with_private_extra_field() {
        let source = "type Person: { name String }\np Person: { name: \"Paul\", _ secret: \"password\" }\n";
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Private extra fields in typed struct init should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_typed_struct_init_with_private_extra_method() {
        let source = "type Greeter: { greet: () String }\ng Greeter: {\n    greet: () String { return \"hello\" }\n    _ validate: () Bool { return true }\n}\n";
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Private extra methods in typed struct init should succeed: {:?}",
            result.err()
        );
    }

    // ========== Helper Method Unit Tests ==========

    #[test]
    fn test_is_field_private_helper() {
        let limits = CompilerLimits::default();
        let tokens = lex("", &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let mut analyzer = SemanticAnalyzer::new(ast);

        let str_id = analyzer.type_registry.intern(Type::String);
        let num_id = analyzer.type_registry.intern(Type::Number);

        let struct_type = StructType {
            fields: vec![
                StructField {
                    name: "name".to_string(),
                    type_id: str_id,
                    is_private: false,
                },
                StructField {
                    name: "secret".to_string(),
                    type_id: str_id,
                    is_private: true,
                },
            ],
            methods: vec![],
        };
        let struct_id = analyzer.type_registry.intern(Type::Struct(struct_type));

        assert_eq!(analyzer.is_field_private(struct_id, "name"), Some(false));
        assert_eq!(analyzer.is_field_private(struct_id, "secret"), Some(true));
        assert_eq!(analyzer.is_field_private(struct_id, "nonexistent"), None);
        assert_eq!(analyzer.is_field_private(num_id, "anything"), None);
    }

    #[test]
    fn test_is_method_private_helper() {
        let limits = CompilerLimits::default();
        let tokens = lex("", &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let mut analyzer = SemanticAnalyzer::new(ast);

        let unit_id = analyzer.type_registry.intern(Type::Unit);
        let func_type = FunctionType {
            params: vec![],
            return_type: unit_id,
        };
        let func_id = analyzer.type_registry.intern(Type::Function(func_type));

        let struct_type = StructType {
            fields: vec![],
            methods: vec![
                StructMethod {
                    name: "greet".to_string(),
                    function_type: func_id,
                    is_private: false,
                },
                StructMethod {
                    name: "validate".to_string(),
                    function_type: func_id,
                    is_private: true,
                },
            ],
        };
        let struct_id = analyzer.type_registry.intern(Type::Struct(struct_type));

        assert_eq!(analyzer.is_method_private(struct_id, "greet"), Some(false));
        assert_eq!(
            analyzer.is_method_private(struct_id, "validate"),
            Some(true)
        );
        assert_eq!(analyzer.is_method_private(struct_id, "nonexistent"), None);
    }
}
