//! Type inference using Hindley-Milner algorithm
//!
//! This module implements type inference for Suru expressions:
//! 1. Assign type variables to unknown types
//! 2. Collect constraints from AST
//! 3. Solve constraints via unification
//! 4. Apply final substitution to get concrete types
//!
//!

use super::{SemanticAnalyzer, SemanticError, Type};

impl SemanticAnalyzer {
    /// Infers type for number literal
    pub(super) fn visit_literal_number(&mut self, node_idx: usize) {
        let number_type = self.type_registry.intern(Type::Number);
        self.set_node_type(node_idx, number_type);
    }

    /// Infers type for string literal
    pub(super) fn visit_literal_string(&mut self, node_idx: usize) {
        let string_type = self.type_registry.intern(Type::String);
        self.set_node_type(node_idx, string_type);
    }

    /// Infers type for boolean literal
    pub(super) fn visit_literal_boolean(&mut self, node_idx: usize) {
        let bool_type = self.type_registry.intern(Type::Bool);
        self.set_node_type(node_idx, bool_type);
    }

    /// Infers type for list literal
    pub(super) fn visit_list(&mut self, node_idx: usize) {
        // Create Array('a) where 'a is a fresh type variable
        let elem_var = self.fresh_type_var();
        let array_type = self.type_registry.intern(Type::Array(elem_var));
        self.set_node_type(node_idx, array_type);

        // For non-empty lists, we need to collect element type constraints
        if self.ast.nodes[node_idx].first_child.is_some() {
            // TODO: Walk children and add constraints
            // For now, list elements are not type-checked
        }
    }

    /// Solves all collected constraints via unification
    ///
    /// This is called after the constraint collection phase (AST traversal).
    /// Each constraint is processed by the unification algorithm, which
    /// updates the substitution to make the types equal.
    ///
    /// If any unification fails, collects all errors and returns them.
    pub(super) fn solve_constraints(&mut self) -> Result<(), Vec<SemanticError>> {
        let mut errors = Vec::new();

        // Process each constraint
        // Clone constraints to avoid borrow checker issues
        let constraints = self.constraints.clone();
        for constraint in constraints {
            if let Err(e) = self.unify(constraint.left, constraint.right, constraint.source) {
                errors.push(e);
            }
        }

        // Clear constraints to avoid re-processing on subsequent passes
        self.constraints.clear();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Applies final substitution to all node types
    ///
    /// After unification, type variables may be bound to concrete types
    /// in the substitution. This method walks through all inferred types
    /// and applies the substitution to replace type variables with their
    /// concrete types.
    ///
    /// # Example
    ///
    /// ```text
    /// Before: node 1 → 'a, node 2 → Array('a)
    /// Substitution: 'a → Number
    /// After: node 1 → Number, node 2 → Array(Number)
    /// ```
    pub(super) fn apply_substitution(&mut self) {
        let keys: Vec<usize> = self.node_types.keys().copied().collect();
        for node_idx in keys {
            if let Some(ty) = self.node_types.get(&node_idx) {
                let final_ty = self.substitution.apply(*ty, &self.type_registry);
                self.node_types.insert(node_idx, final_ty);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use crate::limits::CompilerLimits;
    use crate::parser::parse;

    /// Helper to analyze source and get type of first literal in variable declaration
    fn analyze_literal(source: &str) -> Result<Type, String> {
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).map_err(|e| format!("{:?}", e))?;
        let ast = parse(tokens, &limits).map_err(|e| format!("{:?}", e))?;

        // Get root and first literal before moving ast into analyzer
        let root_idx = ast.root.ok_or("No root")?;
        let decl_idx = ast.nodes[root_idx].first_child.ok_or("No declaration")?;
        let ident_idx = ast.nodes[decl_idx].first_child.ok_or("No identifier")?;
        let literal_idx = ast.nodes[ident_idx].next_sibling.ok_or("No literal")?;

        let mut analyzer = SemanticAnalyzer::new(ast);

        // Run analysis - this traverses AST and infers types
        // Note: analyze() consumes self, so we need to extract info first
        // or visit manually
        if let Some(root) = analyzer.ast.root {
            analyzer.visit_node(root);
            analyzer
                .solve_constraints()
                .map_err(|e| format!("{:?}", e))?;
            analyzer.apply_substitution();
        }

        let type_id = analyzer.get_node_type(literal_idx).ok_or("No type")?;
        let ty = analyzer.type_registry.resolve(type_id);
        Ok(ty.clone())
    }

    #[test]
    fn test_number_literal_decimal() {
        assert_eq!(analyze_literal("x: 42").unwrap(), Type::Number);
    }

    #[test]
    fn test_number_literal_float() {
        assert_eq!(analyze_literal("y: 3.14").unwrap(), Type::Number);
    }

    #[test]
    fn test_number_literal_hex() {
        assert_eq!(analyze_literal("z: 0xFF").unwrap(), Type::Number);
    }

    #[test]
    fn test_number_literal_binary() {
        assert_eq!(analyze_literal("a: 0b1010").unwrap(), Type::Number);
    }

    #[test]
    fn test_number_literal_octal() {
        assert_eq!(analyze_literal("b: 0o755").unwrap(), Type::Number);
    }

    #[test]
    fn test_string_literal() {
        assert_eq!(analyze_literal(r#"s: "hello""#).unwrap(), Type::String);
    }

    #[test]
    fn test_boolean_literal_true() {
        assert_eq!(analyze_literal("flag: true").unwrap(), Type::Bool);
    }

    #[test]
    fn test_boolean_literal_false() {
        assert_eq!(analyze_literal("flag: false").unwrap(), Type::Bool);
    }

    #[test]
    fn test_empty_list() {
        let ty = analyze_literal("xs: []").unwrap();
        // Should be Array(type_variable)
        match ty {
            Type::Array(_) => {
                // Success - element type is a type variable
            }
            _ => panic!("Expected Array type, got {:?}", ty),
        }
    }

    #[test]
    fn test_non_empty_list() {
        let ty = analyze_literal("xs: [1, 2, 3]").unwrap();
        match ty {
            Type::Array(_) => {
                // Success - element type is a type variable
            }
            _ => panic!("Expected Array type, got {:?}", ty),
        }
    }

    #[test]
    fn test_multiple_literals() {
        // Test that multiple literals in sequence all get typed correctly
        let source = r#"
            a: 42
            b: "hello"
            c: true
        "#;
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let analyzer = SemanticAnalyzer::new(ast);

        // analyze() consumes self and returns Ok(Ast) or Err(errors)
        let result = analyzer.analyze();

        // Verify analysis succeeded
        assert!(result.is_ok());
    }

    #[test]
    fn test_constraint_solving_empty() {
        // Test that constraint solving works with no constraints
        let source = "x: 42";
        let limits = CompilerLimits::default();
        let tokens = lex(source, &limits).unwrap();
        let ast = parse(tokens, &limits).unwrap();
        let analyzer = SemanticAnalyzer::new(ast);

        // Should succeed with no errors
        assert!(analyzer.analyze().is_ok());
    }
}
