//! Expression type inference
//!
//! Type checking for operators:
//! - Binary boolean operators (and, or): Both operands Bool → result Bool
//! - Unary not operator: Operand Bool → result Bool
//! - Unary negate operator: Operand Number → result Number
//!
//! Uses Hindley-Milner constraint generation approach integrated with
//! the existing type inference system.

use super::{SemanticAnalyzer, Type};

impl SemanticAnalyzer {
    /// Infers type for binary boolean operator (and, or)
    ///
    /// Type rule: e1 : Bool, e2 : Bool → (e1 op e2) : Bool
    /// Generates constraints that both operands must be Bool.
    pub(super) fn visit_binary_bool_op(&mut self, node_idx: usize) {
        // Get left and right children
        let node = &self.ast.nodes[node_idx];
        let left_idx = node.first_child.expect("Binary op must have left child");
        let right_idx = self.ast.nodes[left_idx]
            .next_sibling
            .expect("Binary op must have right child");

        // Visit children to infer their types
        self.visit_node(left_idx);
        self.visit_node(right_idx);

        // Get operand types
        let left_type = self
            .get_node_type(left_idx)
            .expect("Left operand should have type");
        let right_type = self
            .get_node_type(right_idx)
            .expect("Right operand should have type");

        // Generate constraints: both operands must be Bool
        let bool_type = self.type_registry.intern(Type::Bool);
        self.add_constraint(left_type, bool_type, left_idx);
        self.add_constraint(right_type, bool_type, right_idx);

        // Result type is Bool
        self.set_node_type(node_idx, bool_type);
    }

    /// Infers type for unary not operator
    ///
    /// Type rule: e : Bool → (not e) : Bool
    /// Generates constraint that operand must be Bool.
    pub(super) fn visit_not(&mut self, node_idx: usize) {
        // Get operand
        let operand_idx = self.ast.nodes[node_idx]
            .first_child
            .expect("Not must have operand");

        // Visit operand to infer its type
        self.visit_node(operand_idx);

        // Get operand type
        let operand_type = self
            .get_node_type(operand_idx)
            .expect("Operand should have type");

        // Generate constraint: operand must be Bool
        let bool_type = self.type_registry.intern(Type::Bool);
        self.add_constraint(operand_type, bool_type, operand_idx);

        // Result type is Bool
        self.set_node_type(node_idx, bool_type);
    }

    /// Infers type for unary negate operator
    ///
    /// Type rule: e : Number → (-e) : Number
    ///
    pub(super) fn visit_negate(&mut self, node_idx: usize) {
        // Get operand
        let operand_idx = self.ast.nodes[node_idx]
            .first_child
            .expect("Negate must have operand");

        // Visit operand to infer its type
        self.visit_node(operand_idx);

        // Get operand type
        let operand_type = self
            .get_node_type(operand_idx)
            .expect("Operand should have type");

        // Generate constraint: operand must be Number
        let number_type = self.type_registry.intern(Type::Number);
        self.add_constraint(operand_type, number_type, operand_idx);

        // Result type is Number
        self.set_node_type(node_idx, number_type);
    }
}

#[cfg(test)]
mod tests {
    use super::super::SemanticError;
    use super::*;
    use crate::lexer::lex;
    use crate::limits::CompilerLimits;
    use crate::parser::parse;

    /// Helper to analyze expression and return its type or errors
    fn analyze_expression(source: &str) -> Result<Type, Vec<SemanticError>> {
        let limits = CompilerLimits::default();
        let tokens =
            lex(source, &limits).map_err(|e| vec![SemanticError::new(format!("{:?}", e), 0, 0)])?;
        let ast = parse(tokens, &limits)
            .map_err(|e| vec![SemanticError::new(format!("{:?}", e), 0, 0)])?;

        // Navigate to expression node (source format: "x: <expression>")
        let root_idx = ast
            .root
            .ok_or(vec![SemanticError::new("No root".to_string(), 0, 0)])?;
        let decl_idx = ast.nodes[root_idx]
            .first_child
            .ok_or(vec![SemanticError::new("No declaration".to_string(), 0, 0)])?;
        let ident_idx = ast.nodes[decl_idx]
            .first_child
            .ok_or(vec![SemanticError::new("No identifier".to_string(), 0, 0)])?;
        let expr_idx = ast.nodes[ident_idx]
            .next_sibling
            .ok_or(vec![SemanticError::new("No expression".to_string(), 0, 0)])?;

        let mut analyzer = SemanticAnalyzer::new(ast);

        // Run 3-phase analysis
        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
            analyzer.solve_constraints()?;
            analyzer.apply_substitution();
        }

        let type_id = analyzer
            .get_node_type(expr_idx)
            .ok_or(vec![SemanticError::new("No type".to_string(), 0, 0)])?;
        let ty = analyzer.type_registry.resolve(type_id);
        Ok(ty.clone())
    }

    // ========== Binary Boolean Operators ==========

    #[test]
    fn test_and_operator_both_bool() {
        let ty = analyze_expression("x: true and false").unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_and_operator_type_error_left() {
        let err = analyze_expression("x: 42 and true").unwrap_err();
        assert!(!err.is_empty());
        assert!(err[0].message.contains("Type mismatch"));
    }

    #[test]
    fn test_and_operator_type_error_right() {
        let err = analyze_expression("x: true and 42").unwrap_err();
        assert!(!err.is_empty());
        assert!(err[0].message.contains("Type mismatch"));
    }

    #[test]
    fn test_or_operator_both_bool() {
        let ty = analyze_expression("x: true or false").unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_or_operator_type_error() {
        let err = analyze_expression("x: true or \"hello\"").unwrap_err();
        assert!(!err.is_empty());
        assert!(err[0].message.contains("Type mismatch"));
    }

    #[test]
    fn test_nested_and_or() {
        let ty = analyze_expression("x: true and false or true").unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_and_operator_type_error_both() {
        let err = analyze_expression("x: 1 and 2").unwrap_err();
        assert!(!err.is_empty());
    }

    // ========== Unary Not Operator ==========

    #[test]
    fn test_not_operator_bool() {
        let ty = analyze_expression("x: not true").unwrap();
        assert_eq!(ty, Type::Bool);
    }

    #[test]
    fn test_not_operator_type_error() {
        let err = analyze_expression("x: not 42").unwrap_err();
        assert!(!err.is_empty());
        assert!(err[0].message.contains("Type mismatch"));
    }

    #[test]
    fn test_not_operator_string_error() {
        let err = analyze_expression("x: not \"hello\"").unwrap_err();
        assert!(!err.is_empty());
        assert!(err[0].message.contains("Type mismatch"));
    }

    #[test]
    fn test_double_not() {
        let ty = analyze_expression("x: not not false").unwrap();
        assert_eq!(ty, Type::Bool);
    }

    // ========== Unary Negate Operator ==========

    #[test]
    fn test_negate_operator_number() {
        let ty = analyze_expression("x: -42").unwrap();
        assert_eq!(ty, Type::Number);
    }

    #[test]
    fn test_negate_operator_type_error_bool() {
        let err = analyze_expression("x: -true").unwrap_err();
        assert!(!err.is_empty());
        assert!(err[0].message.contains("Type mismatch"));
    }

    #[test]
    fn test_negate_operator_type_error_string() {
        let err = analyze_expression("x: -\"hello\"").unwrap_err();
        assert!(!err.is_empty());
        assert!(err[0].message.contains("Type mismatch"));
    }

    #[test]
    fn test_double_negate() {
        let ty = analyze_expression("x: - -42").unwrap();
        assert_eq!(ty, Type::Number);
    }
}
