//! Unification algorithm for Hindley-Milner type inference
//!
//! Unification solves type constraints by finding a substitution that makes
//! two types equal. This is the core of Hindley-Milner type inference.
//!
//! # Algorithm
//!
//! The unification algorithm takes two types and attempts to make them equal:
//! - If both are the same type, unification succeeds trivially
//! - If one is a type variable, bind it to the other type (with occurs check)
//! - If both are compound types, recursively unify their components
//! - Otherwise, the types are incompatible and unification fails
//!
//! # Occurs Check
//!
//! The occurs check prevents infinite types like `'a = Array('a)`.
//! Before binding a type variable to a type, we check if the variable
//! occurs within that type.

use super::{SemanticAnalyzer, SemanticError, Type, TypeId, TypeVarId};

impl SemanticAnalyzer {
    /// Unifies two types, updating the substitution
    ///
    /// This implements the standard unification algorithm with occurs check.
    /// If unification fails, returns a SemanticError with location information.
    ///
    /// # Algorithm Steps
    ///
    /// 1. Apply current substitution to both types
    /// 2. If types are identical, succeed
    /// 3. If either is a type variable, bind it (with occurs check)
    /// 4. If both are compound types, recursively unify components
    /// 5. Otherwise, fail with type mismatch error
    pub(super) fn unify(
        &mut self,
        t1: TypeId,
        t2: TypeId,
        source: usize,
    ) -> Result<(), SemanticError> {
        // Apply current substitution to both types first
        let t1 = self.substitution.apply(t1, &self.type_registry);
        let t2 = self.substitution.apply(t2, &self.type_registry);

        // If same type, already unified
        if t1 == t2 {
            return Ok(());
        }

        let type1 = self.type_registry.resolve(t1).clone();
        let type2 = self.type_registry.resolve(t2).clone();

        match (&type1, &type2) {
            // ========== Type Variable Cases ==========

            // Var-Var: bind first to second
            (Type::Var(v1), Type::Var(v2)) => {
                self.substitution.insert(*v1, t2);
                Ok(())
            }

            // Var-Type: bind variable to type (with occurs check)
            (Type::Var(var), _) => {
                if self.occurs_check(*var, t2) {
                    Err(self.make_error(
                        format!("Infinite type: type variable '{}' occurs in type", var.id()),
                        source,
                    ))
                } else {
                    self.substitution.insert(*var, t2);
                    Ok(())
                }
            }

            // Type-Var: symmetric case
            (_, Type::Var(var)) => {
                if self.occurs_check(*var, t1) {
                    Err(self.make_error(
                        format!("Infinite type: type variable '{}' occurs in type", var.id()),
                        source,
                    ))
                } else {
                    self.substitution.insert(*var, t1);
                    Ok(())
                }
            }

            // ========== Primitive Types ==========
            (Type::Unit, Type::Unit)
            | (Type::Number, Type::Number)
            | (Type::String, Type::String)
            | (Type::Bool, Type::Bool) => Ok(()),

            // ========== Sized Types ==========
            (Type::Int(s1), Type::Int(s2)) if s1 == s2 => Ok(()),
            (Type::UInt(s1), Type::UInt(s2)) if s1 == s2 => Ok(()),
            (Type::Float(s1), Type::Float(s2)) if s1 == s2 => Ok(()),

            // ========== Collection Types ==========

            // Array-Array: unify element types
            (Type::Array(elem1), Type::Array(elem2)) => self.unify(*elem1, *elem2, source),

            // Option-Option: unify inner types
            (Type::Option(inner1), Type::Option(inner2)) => self.unify(*inner1, *inner2, source),

            // Result-Result: unify both Ok and Err types
            (Type::Result(ok1, err1), Type::Result(ok2, err2)) => {
                self.unify(*ok1, *ok2, source)?;
                self.unify(*err1, *err2, source)
            }

            // ========== Function Types ==========
            (Type::Function(f1), Type::Function(f2)) => {
                // Check parameter count matches
                if f1.params.len() != f2.params.len() {
                    return Err(self.make_error(
                        format!(
                            "Function parameter count mismatch: expected {}, found {}",
                            f1.params.len(),
                            f2.params.len()
                        ),
                        source,
                    ));
                }

                // Unify each parameter type
                for (p1, p2) in f1.params.iter().zip(&f2.params) {
                    self.unify(p1.type_id, p2.type_id, source)?;
                }

                // Unify return types
                self.unify(f1.return_type, f2.return_type, source)
            }

            // ========== Generic Types ==========
            (
                Type::Generic {
                    type_params: tp1,
                    inner: i1,
                },
                Type::Generic {
                    type_params: tp2,
                    inner: i2,
                },
            ) => {
                if tp1.len() != tp2.len() {
                    return Err(self.make_error(
                        "Generic type parameter count mismatch".to_string(),
                        source,
                    ));
                }
                for (p1, p2) in tp1.iter().zip(tp2.iter()) {
                    self.unify(*p1, *p2, source)?;
                }
                self.unify(*i1, *i2, source)
            }

            // ========== Named Unit Types ==========
            (Type::NamedUnit(n1), Type::NamedUnit(n2)) => {
                if n1 == n2 {
                    Ok(())
                } else {
                    Err(self.make_error(
                        format!("Type mismatch: cannot unify {:?} with {:?}", type1, type2),
                        source,
                    ))
                }
            }

            // ========== Union Types ==========

            // Concrete type vs Union: check if concrete type is one of the alternatives
            (_, Type::Union(alternatives)) => {
                if alternatives.iter().any(|alt| *alt == t1) {
                    Ok(())
                } else {
                    Err(self.make_error(
                        format!(
                            "Type mismatch: type is not a member of the union type"
                        ),
                        source,
                    ))
                }
            }

            // Union vs Concrete: symmetric case
            (Type::Union(alternatives), _) => {
                if alternatives.iter().any(|alt| *alt == t2) {
                    Ok(())
                } else {
                    Err(self.make_error(
                        format!(
                            "Type mismatch: type is not a member of the union type"
                        ),
                        source,
                    ))
                }
            }

            // ========== Struct Types ==========
            (Type::Struct(s1), Type::Struct(s2)) => {
                // Check all required fields from s2 exist in s1
                for expected_field in &s2.fields {
                    match s1.fields.iter().find(|f| f.name == expected_field.name) {
                        None => {
                            return Err(self.make_error(
                                format!("Missing field '{}' in struct literal", expected_field.name),
                                source,
                            ));
                        }
                        Some(actual_field) => {
                            // Unify field types
                            self.unify(actual_field.type_id, expected_field.type_id, source)?;
                        }
                    }
                }

                // Check all required methods from s2 exist in s1
                for expected_method in &s2.methods {
                    match s1.methods.iter().find(|m| m.name == expected_method.name) {
                        None => {
                            return Err(self.make_error(
                                format!("Missing method '{}' in struct literal", expected_method.name),
                                source,
                            ));
                        }
                        Some(actual_method) => {
                            // Unify method signatures (function types)
                            self.unify(
                                actual_method.function_type,
                                expected_method.function_type,
                                source,
                            )?;
                        }
                    }
                }

                // Extra fields in s1 are allowed (structural subtyping)
                Ok(())
            }

            // ========== Special Types ==========

            // Unknown can unify with anything (used for empty lists, etc.)
            (Type::Unknown, _) | (_, Type::Unknown) => Ok(()),

            // Error type unifies with anything (error recovery)
            (Type::Error, _) | (_, Type::Error) => Ok(()),

            // ========== Type Mismatch ==========
            _ => Err(self.make_error(
                format!("Type mismatch: cannot unify {:?} with {:?}", type1, type2),
                source,
            )),
        }
    }

    /// Occurs check: does type variable occur in type?
    ///
    /// Prevents infinite types like `'a = Array('a)` by checking if
    /// a type variable appears within the type it's being bound to.
    ///
    /// # Example
    ///
    /// ```text
    /// occurs_check('a, Array('a)) -> true  (would create infinite type)
    /// occurs_check('a, Array(Number)) -> false  (safe to bind)
    /// ```
    fn occurs_check(&self, var: TypeVarId, ty: TypeId) -> bool {
        // Apply substitution first
        let resolved = self.substitution.apply(ty, &self.type_registry);
        let typ = self.type_registry.resolve(resolved);

        match typ {
            // If it's the same type variable, it occurs
            Type::Var(v) => *v == var,

            // For compound types, recursively check components
            Type::Array(elem) => self.occurs_check(var, *elem),
            Type::Option(inner) => self.occurs_check(var, *inner),
            Type::Result(ok, err) => self.occurs_check(var, *ok) || self.occurs_check(var, *err),
            Type::Function(func) => {
                // Check parameters and return type
                func.params
                    .iter()
                    .any(|p| self.occurs_check(var, p.type_id))
                    || self.occurs_check(var, func.return_type)
            }
            Type::Union(types) => types.iter().any(|t| self.occurs_check(var, *t)),
            Type::Generic { type_params, inner } => {
                type_params.iter().any(|tp| self.occurs_check(var, *tp))
                    || self.occurs_check(var, *inner)
            }
            Type::Struct(struct_type) => {
                // Check fields and methods
                struct_type
                    .fields
                    .iter()
                    .any(|f| self.occurs_check(var, f.type_id))
                    || struct_type
                        .methods
                        .iter()
                        .any(|m| self.occurs_check(var, m.function_type))
            }

            // Primitives and type variables don't contain the variable
            _ => false,
        }
    }

    /// Helper to create a SemanticError with AST node location
    pub(super) fn make_error(&self, message: String, node_idx: usize) -> SemanticError {
        // Check if node_idx is valid
        if node_idx < self.ast.nodes.len() {
            if let Some(token) = &self.ast.nodes[node_idx].token {
                return SemanticError::from_token(message, token);
            }
        }
        // Fallback if node has no token or index is invalid
        SemanticError::new(message, 0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::Type;

    // Helper to create a test analyzer
    fn test_analyzer() -> SemanticAnalyzer {
        use crate::ast::Ast;
        use crate::limits::CompilerLimits;
        use crate::string_storage::StringStorage;

        let limits = CompilerLimits::default();
        let string_storage = StringStorage::new();
        let ast = Ast::new(string_storage, limits);
        SemanticAnalyzer::new(ast)
    }

    #[test]
    fn test_unify_same_types() {
        let mut analyzer = test_analyzer();

        let num1 = analyzer.type_registry.intern(Type::Number);
        let num2 = analyzer.type_registry.intern(Type::Number);

        // Unifying identical types should succeed
        assert!(analyzer.unify(num1, num2, 0).is_ok());
    }

    #[test]
    fn test_unify_var_to_type() {
        let mut analyzer = test_analyzer();

        let var = analyzer.fresh_type_var();
        let num = analyzer.type_registry.intern(Type::Number);

        // Unifying 'a with Number should bind 'a -> Number
        assert!(analyzer.unify(var, num, 0).is_ok());

        // Check substitution was created
        let Type::Var(var_id) = analyzer.type_registry.resolve(var) else {
            panic!("Expected type variable");
        };
        assert_eq!(analyzer.substitution.lookup(*var_id), Some(num));
    }

    #[test]
    fn test_unify_type_mismatch() {
        let mut analyzer = test_analyzer();

        let num = analyzer.type_registry.intern(Type::Number);
        let str = analyzer.type_registry.intern(Type::String);

        // Unifying Number with String should fail
        assert!(analyzer.unify(num, str, 0).is_err());
    }

    #[test]
    fn test_unify_arrays() {
        let mut analyzer = test_analyzer();

        let num = analyzer.type_registry.intern(Type::Number);
        let arr_num1 = analyzer.type_registry.intern(Type::Array(num));
        let arr_num2 = analyzer.type_registry.intern(Type::Array(num));

        // Array(Number) should unify with Array(Number)
        assert!(analyzer.unify(arr_num1, arr_num2, 0).is_ok());
    }

    #[test]
    fn test_occurs_check() {
        let mut analyzer = test_analyzer();

        let var = analyzer.fresh_type_var();

        // Create Array('a) where 'a is the type variable
        let arr_var = analyzer.type_registry.intern(Type::Array(var));

        // Unifying 'a with Array('a) should fail (infinite type)
        let Type::Var(var_id) = analyzer.type_registry.resolve(var) else {
            panic!("Expected type variable");
        };
        assert!(analyzer.occurs_check(*var_id, arr_var));
        assert!(analyzer.unify(var, arr_var, 0).is_err());
    }

    #[test]
    fn test_unify_var_var() {
        let mut analyzer = test_analyzer();

        let var1 = analyzer.fresh_type_var();
        let var2 = analyzer.fresh_type_var();

        // Unifying two type variables should bind one to the other
        assert!(analyzer.unify(var1, var2, 0).is_ok());
    }

    #[test]
    fn test_transitive_unification() {
        let mut analyzer = test_analyzer();

        let var = analyzer.fresh_type_var();
        let num = analyzer.type_registry.intern(Type::Number);

        // First unify 'a with Number
        analyzer.unify(var, num, 0).unwrap();

        // Now create Array('a) and Array(Number)
        let arr_var = analyzer.type_registry.intern(Type::Array(var));
        let arr_num = analyzer.type_registry.intern(Type::Array(num));

        // These should unify because 'a is already bound to Number
        assert!(analyzer.unify(arr_var, arr_num, 0).is_ok());
    }
}
