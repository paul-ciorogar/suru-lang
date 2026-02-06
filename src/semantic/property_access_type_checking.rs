//! Property access type checking for semantic analysis
//!
//! This module handles:
//! - Looking up field types on struct types
//! - Type checking for property access expressions (receiver.field)
//!
//! Property access in Suru:
//! - The receiver must be a struct type
//! - The field must exist on the struct
//! - The result type of `receiver.field` is the field's declared type
//! - Privacy rules are enforced separately in struct_privacy.rs

use super::{SemanticAnalyzer, Type, TypeId};

impl SemanticAnalyzer {
    /// Looks up a field's TypeId on a struct type
    ///
    /// Returns `Some(TypeId)` if the field exists on the struct,
    /// `None` if the field doesn't exist or the type is not a struct.
    pub(super) fn lookup_struct_field_type(
        &self,
        struct_type_id: TypeId,
        field_name: &str,
    ) -> Option<TypeId> {
        let ty = self.type_registry.resolve(struct_type_id);
        if let Type::Struct(struct_type) = ty {
            for field in &struct_type.fields {
                if field.name == field_name {
                    return Some(field.type_id);
                }
            }
        }
        None
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

    // ========== Field Existence Tests ==========

    #[test]
    fn test_property_access_existing_field_succeeds() {
        let source = "p: { name: \"Paul\", age: 42 }\nx: p.name\n";
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Accessing existing field should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_property_access_nonexistent_field_error() {
        let source = "p: { name: \"Paul\" }\nx: p.email\n";
        let result = analyze_source(source);
        assert!(result.is_err(), "Accessing nonexistent field should fail");
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Field 'email' does not exist on struct type")),
            "Error should mention nonexistent field: {:?}",
            errors
        );
    }

    #[test]
    fn test_property_access_multiple_nonexistent_fields() {
        let source = "p: { name: \"Paul\" }\nx: p.email\ny: p.phone\n";
        let result = analyze_source(source);
        assert!(result.is_err(), "Accessing nonexistent fields should fail");
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Field 'email' does not exist")),
            "Should report email error: {:?}",
            errors
        );
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Field 'phone' does not exist")),
            "Should report phone error: {:?}",
            errors
        );
    }

    // ========== Result Type Propagation Tests ==========

    #[test]
    fn test_property_access_result_type_string() {
        let source = "p: { name: \"Paul\" }\nx: p.name\n";
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Property access should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_property_access_result_type_number() {
        let source = "p: { x: 10, y: 20 }\nval: p.x\n";
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Property access should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_property_access_result_type_bool() {
        let source = "config: { enabled: true }\nflag: config.enabled\n";
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Property access should succeed: {:?}",
            result.err()
        );
    }

    // ========== Non-Struct Receiver Tests ==========

    #[test]
    fn test_property_access_on_number_error() {
        let source = "x: 42\ny: x.name\n";
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Property access on number should fail"
        );
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Cannot access property 'name' on non-struct type")),
            "Error should mention non-struct type: {:?}",
            errors
        );
    }

    #[test]
    fn test_property_access_on_string_error() {
        let source = "x: \"hello\"\ny: x.length\n";
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Property access on string should fail"
        );
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Cannot access property 'length' on non-struct type")),
            "Error should mention non-struct type: {:?}",
            errors
        );
    }

    #[test]
    fn test_property_access_on_bool_error() {
        let source = "x: true\ny: x.value\n";
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Property access on bool should fail"
        );
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Cannot access property 'value' on non-struct type")),
            "Error should mention non-struct type: {:?}",
            errors
        );
    }

    // ========== Chained Property Access Tests ==========

    #[test]
    fn test_property_access_chained() {
        let source = "outer: { inner: { value: 42 } }\nx: outer.inner.value\n";
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Chained property access should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_property_access_chained_nonexistent() {
        let source = "outer: { inner: { value: 42 } }\nx: outer.inner.missing\n";
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Chained access to nonexistent field should fail"
        );
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Field 'missing' does not exist")),
            "Error should mention missing field: {:?}",
            errors
        );
    }

    // ========== Typed Struct Tests ==========

    #[test]
    fn test_property_access_typed_struct() {
        let source = "type Person: { name String, age Number }\np Person: { name: \"Paul\", age: 30 }\nx: p.name\ny: p.age\n";
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Property access on typed struct should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_property_access_typed_struct_nonexistent_field() {
        let source = "type Person: { name String }\np Person: { name: \"Paul\" }\nx: p.email\n";
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Accessing nonexistent field on typed struct should fail"
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

    // ========== Privacy + Type Integration Tests ==========

    #[test]
    fn test_property_access_private_field_error_preserved() {
        let source = "user: { name: \"Paul\", _ secret: \"password\" }\nx: user.secret\n";
        let result = analyze_source(source);
        assert!(result.is_err(), "Accessing private field should fail");
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Cannot access private field 'secret'")),
            "Privacy error should be preserved: {:?}",
            errors
        );
    }

    #[test]
    fn test_property_access_public_field_with_private_sibling() {
        let source = "user: { name: \"Paul\", _ secret: \"password\" }\nx: user.name\n";
        let result = analyze_source(source);
        assert!(
            result.is_ok(),
            "Accessing public field should succeed even with private siblings: {:?}",
            result.err()
        );
    }

    // ========== Helper Unit Tests ==========

    #[test]
    fn test_lookup_struct_field_type_helper() {
        use crate::semantic::{StructField, StructType, Type};

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
                    name: "age".to_string(),
                    type_id: num_id,
                    is_private: false,
                },
            ],
            methods: vec![],
        };
        let struct_id = analyzer.type_registry.intern(Type::Struct(struct_type));

        assert_eq!(
            analyzer.lookup_struct_field_type(struct_id, "name"),
            Some(str_id)
        );
        assert_eq!(
            analyzer.lookup_struct_field_type(struct_id, "age"),
            Some(num_id)
        );
        assert_eq!(
            analyzer.lookup_struct_field_type(struct_id, "nonexistent"),
            None
        );
        assert_eq!(analyzer.lookup_struct_field_type(num_id, "anything"), None);
    }
}
