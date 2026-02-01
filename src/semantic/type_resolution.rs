//! Type declaration processing for semantic analysis
//!
//! This module implements type declaration processing for the Suru
//! semantic analyzer. It handles registration and validation of:
//! - Unit types (e.g., `type Success`)
//! - Type aliases (e.g., `type UserId: Number`)
//! - Union types (e.g., `type Status: Success, Error`)
//! - Struct types (e.g., `type Person: { name String, age Number }`)
//! - Intersection types (e.g., `type Admin: Person + Manager`)

use super::{SemanticAnalyzer, SemanticError, Symbol, SymbolKind, Type, TypeId};
use crate::ast::NodeType;

impl SemanticAnalyzer {
    /// Visits a type declaration node
    ///
    /// Processes type declarations following a three-phase approach:
    /// 1. Extract: Get type name and body structure from AST
    /// 2. Validate: Check for duplicates and undefined references
    /// 3. Register: Create Type, intern in TypeRegistry, add Symbol to scope
    pub(super) fn visit_type_decl(&mut self, node_idx: usize) {
        // PHASE 1: EXTRACT INFORMATION

        // Get type name (first child)
        let Some(type_name_idx) = self.ast.nodes[node_idx].first_child else {
            return; // Malformed AST
        };

        if self.ast.nodes[type_name_idx].node_type != NodeType::TypeName {
            return; // Expected TypeName
        }

        let Some(type_name) = self.ast.node_text(type_name_idx) else {
            return; // No name
        };
        let type_name = type_name.to_string();

        // Check if next child is TypeParams
        let current_child = self.ast.nodes[type_name_idx].next_sibling;

        if let Some(child_idx) = current_child {
            if self.ast.nodes[child_idx].node_type == NodeType::TypeParams {
                // Generic types not yet supported
                let token = self.ast.nodes[type_name_idx].token.as_ref().unwrap();
                let error = SemanticError::from_token(
                    format!("Generic types not yet supported (type '{}')", type_name),
                    token,
                );
                self.record_error(error);
                return;
            }
        }

        // Get TypeBody (should be next sibling after name)
        let Some(type_body_idx) = current_child else {
            return; // No type body
        };

        if self.ast.nodes[type_body_idx].node_type != NodeType::TypeBody {
            return; // Expected TypeBody
        }

        // PHASE 2: VALIDATE

        // Check for duplicate type declaration in current scope
        if self
            .scopes
            .current_scope()
            .lookup_local(&type_name)
            .is_some()
        {
            let token = self.ast.nodes[type_name_idx].token.as_ref().unwrap();
            let error = SemanticError::from_token(
                format!("Duplicate declaration of type '{}'", type_name),
                token,
            );
            self.record_error(error);
            return;
        }

        // PHASE 3: PROCESS TYPE BODY AND REGISTER

        // Determine type form based on TypeBody children
        let type_id = match self.process_type_body(type_body_idx) {
            Ok(id) => id,
            Err(error) => {
                self.record_error(error);
                return;
            }
        };

        // Register type in symbol table
        let symbol = Symbol::new(
            type_name.clone(),
            Some(format!("TypeId({})", type_id.index())), // Store TypeId reference
            SymbolKind::Type,
        );
        self.scopes.insert(symbol);
    }

    /// Processes the body of a type declaration and returns its TypeId
    fn process_type_body(&mut self, type_body_idx: usize) -> Result<TypeId, SemanticError> {
        // Check what's inside TypeBody
        let first_child = self.ast.nodes[type_body_idx].first_child;

        match first_child {
            None => {
                // Unit type - empty TypeBody
                Ok(self.type_registry.intern(Type::Unit))
            }
            Some(child_idx) => match self.ast.nodes[child_idx].node_type {
                NodeType::TypeAnnotation => {
                    // Type alias: type UserId: Number
                    self.process_type_alias(child_idx)
                }
                NodeType::UnionTypeList => {
                    // Union type: type Status: Success, Error
                    self.process_union_type(child_idx)
                }
                NodeType::StructBody => {
                    // Struct type: type Person: { name String, age Number }
                    self.process_struct_type(child_idx)
                }
                NodeType::IntersectionType => {
                    // Intersection type: type Admin: Person + Manager
                    self.process_intersection_type(child_idx)
                }
                NodeType::FunctionType => {
                    let token = self.ast.nodes[child_idx]
                        .token
                        .as_ref()
                        .or_else(|| self.ast.nodes[type_body_idx].token.as_ref())
                        .unwrap();
                    Err(SemanticError::from_token(
                        "Function types not yet supported".to_string(),
                        token,
                    ))
                }
                _ => {
                    let token = self.ast.nodes[child_idx]
                        .token
                        .as_ref()
                        .or_else(|| self.ast.nodes[type_body_idx].token.as_ref())
                        .unwrap();
                    Err(SemanticError::from_token(
                        "Unknown type form".to_string(),
                        token,
                    ))
                }
            },
        }
    }

    /// Processes a type alias declaration
    fn process_type_alias(&mut self, type_ann_idx: usize) -> Result<TypeId, SemanticError> {
        // TypeAnnotation node contains the target type name
        let Some(target_type_name) = self.ast.node_text(type_ann_idx) else {
            let token = self.ast.nodes[type_ann_idx].token.as_ref().unwrap();
            return Err(SemanticError::from_token(
                "Type alias missing target type".to_string(),
                token,
            ));
        };
        let target_type_name = target_type_name.to_string(); // Convert to owned String

        // Validate that the target type exists
        if !self.type_exists(&target_type_name) {
            let token = self.ast.nodes[type_ann_idx].token.as_ref().unwrap();
            return Err(SemanticError::from_token(
                format!("Type '{}' is not defined", target_type_name),
                token,
            ));
        }

        // For aliases, we return the TypeId of the target type
        // This achieves transparent aliasing (UserId and Number share the same TypeId)
        self.lookup_type_id(&target_type_name)
    }

    /// Processes a union type declaration
    fn process_union_type(&mut self, union_list_idx: usize) -> Result<TypeId, SemanticError> {
        let mut type_ids = Vec::new();

        // Iterate through all children of UnionTypeList
        let mut current_child = self.ast.nodes[union_list_idx].first_child;

        while let Some(child_idx) = current_child {
            // Each child should be a TypeAnnotation
            if self.ast.nodes[child_idx].node_type != NodeType::TypeAnnotation {
                let token = self.ast.nodes[child_idx].token.as_ref().unwrap();
                return Err(SemanticError::from_token(
                    "Expected type name in union".to_string(),
                    token,
                ));
            }

            let Some(type_name) = self.ast.node_text(child_idx) else {
                let token = self.ast.nodes[child_idx].token.as_ref().unwrap();
                return Err(SemanticError::from_token(
                    "Missing type name in union".to_string(),
                    token,
                ));
            };
            let type_name = type_name.to_string(); // Convert to owned String

            // Validate type exists
            if !self.type_exists(&type_name) {
                let token = self.ast.nodes[child_idx].token.as_ref().unwrap();
                return Err(SemanticError::from_token(
                    format!("Type '{}' is not defined", type_name),
                    token,
                ));
            }

            // Get TypeId and add to vector
            let type_id = self.lookup_type_id(&type_name)?;
            type_ids.push(type_id);

            // Move to next sibling
            current_child = self.ast.nodes[child_idx].next_sibling;
        }

        // Create union type and intern it
        Ok(self.type_registry.intern(Type::Union(type_ids)))
    }

    /// Processes a struct type declaration
    ///
    /// Delegates to the struct_type_definition module which handles both
    /// fields and methods.
    fn process_struct_type(&mut self, struct_body_idx: usize) -> Result<TypeId, SemanticError> {
        self.process_struct_type_definition(struct_body_idx)
    }

    /// Processes an intersection type declaration
    fn process_intersection_type(
        &mut self,
        intersection_idx: usize,
    ) -> Result<TypeId, SemanticError> {
        // IntersectionType has exactly two children: left and right
        let Some(left_idx) = self.ast.nodes[intersection_idx].first_child else {
            let token = self.ast.nodes[intersection_idx].token.as_ref().unwrap();
            return Err(SemanticError::from_token(
                "Intersection missing left type".to_string(),
                token,
            ));
        };

        let Some(right_idx) = self.ast.nodes[left_idx].next_sibling else {
            let token = self.ast.nodes[left_idx].token.as_ref().unwrap();
            return Err(SemanticError::from_token(
                "Intersection missing right type".to_string(),
                token,
            ));
        };

        // Process left type (can be TypeAnnotation, StructBody, or nested IntersectionType)
        let left_type_id = self.process_intersection_operand(left_idx)?;

        // Process right type
        let right_type_id = self.process_intersection_operand(right_idx)?;

        // VALIDATION: Left-hand type must be a struct type or intersection
        let left_type = self.type_registry.get(left_type_id);
        if !matches!(left_type, Type::Struct(_) | Type::Intersection(_, _)) {
            let token = self.ast.nodes[left_idx].token.as_ref().unwrap();
            return Err(SemanticError::from_token(
                "Left side of intersection must be a struct type".to_string(),
                token,
            ));
        }

        // Create intersection type
        Ok(self
            .type_registry
            .intern(Type::Intersection(left_type_id, right_type_id)))
    }

    /// Processes one operand of an intersection type
    fn process_intersection_operand(
        &mut self,
        operand_idx: usize,
    ) -> Result<TypeId, SemanticError> {
        match self.ast.nodes[operand_idx].node_type {
            NodeType::TypeAnnotation => {
                // Simple type reference
                let Some(type_name) = self.ast.node_text(operand_idx) else {
                    let token = self.ast.nodes[operand_idx].token.as_ref().unwrap();
                    return Err(SemanticError::from_token(
                        "Missing type name".to_string(),
                        token,
                    ));
                };
                let type_name = type_name.to_string(); // Convert to owned String

                if !self.type_exists(&type_name) {
                    let token = self.ast.nodes[operand_idx].token.as_ref().unwrap();
                    return Err(SemanticError::from_token(
                        format!("Type '{}' is not defined", type_name),
                        token,
                    ));
                }

                self.lookup_type_id(&type_name)
            }
            NodeType::StructBody => {
                // Inline struct body (e.g., Person + { salary Int64 })
                self.process_struct_type(operand_idx)
            }
            NodeType::IntersectionType => {
                // Nested intersection (for chaining: A + B + C)
                self.process_intersection_type(operand_idx)
            }
            _ => {
                let token = self.ast.nodes[operand_idx].token.as_ref().unwrap();
                Err(SemanticError::from_token(
                    "Invalid type in intersection".to_string(),
                    token,
                ))
            }
        }
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

    // ========== Built-in Types Tests ==========

    #[test]
    fn test_type_alias_to_builtin_number() {
        let result = analyze_source("type UserId: Number\n");
        assert!(result.is_ok());
    }

    #[test]
    fn test_type_alias_to_builtin_string() {
        let result = analyze_source("type Username: String\n");
        assert!(result.is_ok());
    }

    #[test]
    fn test_type_alias_to_builtin_bool() {
        let result = analyze_source("type Flag: Bool\n");
        assert!(result.is_ok());
    }

    #[test]
    fn test_type_alias_to_sized_int() {
        let result = analyze_source("type Age: Int64\n");
        assert!(result.is_ok());
    }

    #[test]
    fn test_type_alias_to_sized_uint() {
        let result = analyze_source("type Count: UInt32\n");
        assert!(result.is_ok());
    }

    #[test]
    fn test_type_alias_to_float() {
        let result = analyze_source("type Price: Float64\n");
        assert!(result.is_ok());
    }

    // ========== Unit Types Tests ==========

    #[test]
    fn test_unit_type_declaration() {
        let result = analyze_source("type Success\n");
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_unit_types() {
        let source = "type Success\ntype Error\ntype Loading\n";
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    // ========== Type Aliases Tests ==========

    #[test]
    fn test_type_alias_simple() {
        let result = analyze_source("type UserId: Number\n");
        assert!(result.is_ok());
    }

    #[test]
    fn test_type_alias_to_undefined() {
        let result = analyze_source("type UserId: UnknownType\n");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("UnknownType"));
        assert!(errors[0].message.contains("not defined"));
    }

    #[test]
    fn test_type_alias_chain() {
        let source = "type UserId: Number\ntype AdminId: UserId\n";
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_type_alias_forward_reference() {
        // Forward references not supported yet
        let source = "type A: B\ntype B: Number\n";
        let result = analyze_source(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("'B' is not defined"));
    }

    // ========== Union Types Tests ==========

    #[test]
    fn test_union_type_two_alternatives() {
        let source = "type Success\ntype Error\ntype Status: Success, Error\n";
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_union_type_three_alternatives() {
        let source = "type Value: Number, String, Bool\n";
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_union_with_undefined_type() {
        let result = analyze_source("type Status: Success, Error\n");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("'Success' is not defined"));
    }

    #[test]
    fn test_union_with_one_undefined() {
        let source = "type Success\ntype Status: Success, Error\n";
        let result = analyze_source(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("'Error' is not defined"));
    }

    // ========== Struct Types Tests ==========

    #[test]
    fn test_struct_type_empty() {
        let result = analyze_source("type Empty: {}\n");
        assert!(result.is_ok());
    }

    #[test]
    fn test_struct_type_single_field() {
        let result = analyze_source("type Point: { x Number }\n");
        assert!(result.is_ok());
    }

    #[test]
    fn test_struct_type_multiple_fields() {
        let source = "type Person: {\n    name String\n    age Number\n}\n";
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_struct_with_undefined_field_type() {
        let result = analyze_source("type Person: { name UnknownType }\n");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("UnknownType"));
        assert!(errors[0].message.contains("not defined"));
    }

    #[test]
    fn test_struct_nested_types() {
        let source = "type Address: { city String }\ntype Person: { addr Address }\n";
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_struct_with_all_builtin_types() {
        let source = r#"
            type AllTypes: {
                n Number
                s String
                b Bool
                i8 Int8
                i16 Int16
                i32 Int32
                i64 Int64
                u8 UInt8
                u16 UInt16
                u32 UInt32
                u64 UInt64
                f32 Float32
                f64 Float64
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    // ========== Intersection Types Tests ==========

    #[test]
    fn test_intersection_two_types() {
        let source = r#"
            type Person: { name String }
            type Manager: { team String }
            type Admin: Person + Manager
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_intersection_chained() {
        let source = r#"
            type A: { a Number }
            type B: { b Number }
            type C: { c Number }
            type ABC: A + B + C
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_intersection_inline_struct() {
        let source = r#"
            type Person: { name String }
            type Manager: Person + { salary Int64 }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_intersection_invalid_left_type() {
        let source = "type Invalid: Number + String\n";
        let result = analyze_source(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("must be a struct type"));
    }

    #[test]
    fn test_intersection_undefined_left() {
        let source = "type Admin: Person + Manager\n";
        let result = analyze_source(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("'Person' is not defined"));
    }

    #[test]
    fn test_intersection_undefined_right() {
        let source = "type Person: { name String }\ntype Admin: Person + Manager\n";
        let result = analyze_source(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("'Manager' is not defined"));
    }

    // ========== Error Cases Tests ==========

    #[test]
    fn test_duplicate_type_declaration() {
        let source = "type Point: { x Number }\ntype Point: { y Number }\n";
        let result = analyze_source(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("Duplicate declaration"));
        assert!(errors[0].message.contains("'Point'"));
    }

    #[test]
    fn test_generic_types_deferred() {
        let result = analyze_source("type List<T>: { items Array }\n");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors[0]
                .message
                .contains("Generic types not yet supported")
        );
    }

    #[test]
    fn test_generic_with_constraint_deferred() {
        let result = analyze_source("type Container<T: Number>: { value T }\n");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors[0]
                .message
                .contains("Generic types not yet supported")
        );
    }

    // ========== Integration Tests ==========

    #[test]
    fn test_complex_type_system() {
        let source = r#"
            type UserId: Number
            type Success
            type Error
            type Result: Success, Error
            type User: {
                id UserId
                name String
                age Number
            }
            type Admin: User + { permissions String }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_deeply_nested_types() {
        let source = r#"
            type Id: Int64
            type Name: String
            type Address: { street String
                            city String }
            type Contact: { email String
                            phone String }
            type Person: {
                id Id
                name Name
                addr Address
            }
            type Employee: Person + {
                contact Contact
                salary Float64
            }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_union_of_structs() {
        let source = r#"
            type Cat: { meow String }
            type Dog: { bark String }
            type Pet: Cat, Dog
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_type_can_reference_earlier_declarations() {
        let source = r#"
            type A: Number
            type B: A
            type C: B
            type D: C
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mixed_unit_and_struct_union() {
        let source = r#"
            type Loading
            type Data: { value String }
            type State: Loading, Data
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_error_messages_contain_location() {
        let result = analyze_source("type Foo: Bar\n");
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].line > 0);
        assert!(errors[0].column > 0);
    }
}
