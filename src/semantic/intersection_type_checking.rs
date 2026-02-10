//! Intersection type checking for semantic analysis
//!
//! This module handles merging struct types for intersection type declarations.
//! When `type Employee: Person + { salary Number }` is declared, the fields and
//! methods from both sides are merged into a single `Type::Struct`.
//!
//! Validation rules:
//! - Duplicate field names with different types produce an error
//! - Duplicate field names with different privacy produce an error
//! - Duplicate method names with different signatures produce an error
//! - Duplicate method names with different privacy produce an error
//! - Identical duplicates (same name, type, and privacy) are allowed and deduplicated

use super::types::StructType;
use super::{SemanticAnalyzer, SemanticError};

impl SemanticAnalyzer {
    /// Merges two struct types into one, validating for conflicts.
    ///
    /// Fields and methods from `right` are added to those from `left`.
    /// If a field/method name already exists in `left`:
    /// - Same type and privacy: skip (deduplicate)
    /// - Different type: error
    /// - Different privacy: error
    pub(super) fn merge_struct_types(
        &self,
        left: &StructType,
        right: &StructType,
        source_node_idx: usize,
    ) -> Result<StructType, SemanticError> {
        let mut fields = left.fields.clone();
        let mut methods = left.methods.clone();

        // Merge fields from right
        for right_field in &right.fields {
            if let Some(existing) = fields.iter().find(|f| f.name == right_field.name) {
                // Duplicate name - check for conflicts
                if existing.type_id != right_field.type_id {
                    return Err(self.make_error(
                        format!(
                            "Field '{}' has conflicting types in intersection",
                            right_field.name
                        ),
                        source_node_idx,
                    ));
                }
                if existing.is_private != right_field.is_private {
                    return Err(self.make_error(
                        format!(
                            "Field '{}' has conflicting privacy in intersection",
                            right_field.name
                        ),
                        source_node_idx,
                    ));
                }
                // Same name, type, and privacy - skip duplicate
            } else {
                fields.push(right_field.clone());
            }
        }

        // Merge methods from right
        for right_method in &right.methods {
            if let Some(existing) = methods.iter().find(|m| m.name == right_method.name) {
                // Duplicate name - check for conflicts
                if existing.function_type != right_method.function_type {
                    return Err(self.make_error(
                        format!(
                            "Method '{}' has conflicting signatures in intersection",
                            right_method.name
                        ),
                        source_node_idx,
                    ));
                }
                if existing.is_private != right_method.is_private {
                    return Err(self.make_error(
                        format!(
                            "Method '{}' has conflicting privacy in intersection",
                            right_method.name
                        ),
                        source_node_idx,
                    ));
                }
                // Same name, signature, and privacy - skip duplicate
            } else {
                methods.push(right_method.clone());
            }
        }

        Ok(StructType { fields, methods })
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

    // ========== Merge Basics ==========

    #[test]
    fn test_intersection_disjoint_fields() {
        let source = r#"
            type Person: { name String }
            type HasAge: { age Number }
            type Full: Person + HasAge
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Expected success: {:?}", result.err());
    }

    #[test]
    fn test_intersection_chained_three_types() {
        let source = r#"
            type A: { a Number }
            type B: { b String }
            type C: { c Bool }
            type ABC: A + B + C
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Expected success: {:?}", result.err());
    }

    #[test]
    fn test_intersection_inline_struct() {
        let source = r#"
            type Person: { name String }
            type Employee: Person + { salary Number }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Expected success: {:?}", result.err());
    }

    #[test]
    fn test_intersection_duplicate_field_same_type_ok() {
        let source = r#"
            type A: { x Number }
            type B: { x Number }
            type AB: A + B
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Expected success: {:?}", result.err());
    }

    // ========== Conflict Detection ==========

    #[test]
    fn test_intersection_field_type_conflict() {
        let source = r#"
            type A: { x Number }
            type B: { x String }
            type AB: A + B
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Expected error for conflicting field types");
        let errors = result.unwrap_err();
        assert!(
            errors[0].message.contains("conflicting types"),
            "Expected 'conflicting types' error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_intersection_method_signature_conflict() {
        let source = r#"
            type A: { greet: () String }
            type B: { greet: () Number }
            type AB: A + B
        "#;
        let result = analyze_source(source);
        assert!(
            result.is_err(),
            "Expected error for conflicting method signatures"
        );
        let errors = result.unwrap_err();
        assert!(
            errors[0].message.contains("conflicting signatures"),
            "Expected 'conflicting signatures' error, got: {}",
            errors[0].message
        );
    }

    // ========== Right Operand Validation ==========

    #[test]
    fn test_intersection_right_non_struct_error() {
        let source = r#"
            type Person: { name String }
            type Bad: Person + Number
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Expected error for non-struct right operand");
        let errors = result.unwrap_err();
        assert!(
            errors[0].message.contains("must be a struct type"),
            "Expected 'must be a struct type' error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_intersection_left_non_struct_error() {
        let source = "type Invalid: Number + String\n";
        let result = analyze_source(source);
        assert!(result.is_err(), "Expected error for non-struct left operand");
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("must be a struct type"));
    }

    // ========== Property Access on Intersection ==========

    #[test]
    fn test_property_access_left_field() {
        let source = r#"
            type Person: { name String, age Number }
            type Employee: Person + { salary Number }
            make_employee: () Employee {
                return { name: "Alice", age: 30, salary: 50000 }
            }
            emp: make_employee()
            n: emp.name
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Expected success: {:?}", result.err());
    }

    #[test]
    fn test_property_access_right_field() {
        let source = r#"
            type Person: { name String }
            type Employee: Person + { salary Number }
            make_employee: () Employee {
                return { name: "Alice", salary: 50000 }
            }
            emp: make_employee()
            s: emp.salary
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Expected success: {:?}", result.err());
    }

    #[test]
    fn test_property_access_nonexistent_field() {
        let source = r#"
            type Person: { name String }
            type Employee: Person + { salary Number }
            make_employee: () Employee {
                return { name: "Alice", salary: 50000 }
            }
            emp: make_employee()
            x: emp.nonexistent
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Expected error for nonexistent field");
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("does not exist"));
    }

    // ========== Method Calls on Intersection ==========

    #[test]
    fn test_method_call_on_intersection_left_method() {
        let source = r#"
            type Greeter: { greet: () String }
            type Worker: Greeter + { work: () Number }
            make_worker: () Worker {
                return {
                    greet: () String { return "hi" }
                    work: () Number { return 42 }
                }
            }
            w: make_worker()
            g: w.greet()
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Expected success: {:?}", result.err());
    }

    #[test]
    fn test_method_call_on_intersection_right_method() {
        let source = r#"
            type Greeter: { greet: () String }
            type Worker: Greeter + { work: () Number }
            make_worker: () Worker {
                return {
                    greet: () String { return "hi" }
                    work: () Number { return 42 }
                }
            }
            w: make_worker()
            r: w.work()
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Expected success: {:?}", result.err());
    }

    #[test]
    fn test_method_call_nonexistent_method() {
        let source = r#"
            type Greeter: { greet: () String }
            type Worker: Greeter + { work: () Number }
            make_worker: () Worker {
                return {
                    greet: () String { return "hi" }
                    work: () Number { return 42 }
                }
            }
            w: make_worker()
            r: w.fly()
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Expected error for nonexistent method");
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("does not exist"));
    }

    // ========== Unification ==========

    #[test]
    fn test_struct_init_satisfies_intersection() {
        let source = r#"
            type Person: { name String, age Number }
            type Employee: Person + { salary Number }
            emp Employee: { name: "Alice", age: 30, salary: 50000 }
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Expected success: {:?}", result.err());
    }

    #[test]
    fn test_struct_init_missing_intersection_field() {
        let source = r#"
            type Person: { name String, age Number }
            type Employee: Person + { salary Number }
            emp Employee: { name: "Alice", salary: 50000 }
        "#;
        let result = analyze_source(source);
        assert!(result.is_err(), "Expected error for missing field");
        let errors = result.unwrap_err();
        assert!(
            errors[0].message.contains("Missing field"),
            "Expected 'Missing field' error, got: {}",
            errors[0].message
        );
    }

    // ========== Chained Intersection Property Access ==========

    #[test]
    fn test_chained_intersection_access_all_fields() {
        let source = r#"
            type A: { a Number }
            type B: { b String }
            type C: { c Bool }
            type ABC: A + B + C
            make_abc: () ABC {
                return { a: 1, b: "two", c: true }
            }
            x: make_abc()
            va: x.a
            vb: x.b
            vc: x.c
        "#;
        let result = analyze_source(source);
        assert!(result.is_ok(), "Expected success: {:?}", result.err());
    }

    // ========== Undefined Types in Intersection ==========

    #[test]
    fn test_intersection_undefined_left() {
        let source = "type Admin: Person + Manager\n";
        let result = analyze_source(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("is not defined"));
    }

    #[test]
    fn test_intersection_undefined_right() {
        let source = r#"
            type Person: { name String }
            type Admin: Person + Manager
        "#;
        let result = analyze_source(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors[0].message.contains("is not defined"));
    }
}
