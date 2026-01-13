//! Internal type representation for Suru's type system
//!
//! This module provides the foundational type representation used by the semantic
//! analyzer. It implements a type interning system for efficient type comparison
//! and structural deduplication.
//!
//! # Architecture
//!
//! - [`Type`]: Enum representing all type forms in Suru
//! - [`TypeId`]: Opaque handle to a type in the registry
//! - [`TypeRegistry`]: Central type storage with interning
//!
//! # Type Interning
//!
//! Identical types receive the same TypeId, enabling:
//! - Fast type comparison (compare TypeIds, not deep structures)
//! - Memory efficiency (shared types)
//! - Support for recursive types
//!
//! # Example
//!
//! ```
//! use suru_lang::semantic::{Type, TypeRegistry};
//!
//! let mut registry = TypeRegistry::new();
//! let num1 = registry.intern(Type::Number);
//! let num2 = registry.intern(Type::Number);
//! assert_eq!(num1, num2); // Same type, same ID
//! ```

use std::collections::HashMap;

// ========== Type Size Enums ==========

/// Integer size variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntSize {
    /// 8-bit signed integer
    I8,
    /// 16-bit signed integer
    I16,
    /// 32-bit signed integer
    I32,
    /// 64-bit signed integer
    I64,
}

/// Unsigned integer size variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UIntSize {
    /// 8-bit unsigned integer
    U8,
    /// 16-bit unsigned integer
    U16,
    /// 32-bit unsigned integer
    U32,
    /// 64-bit unsigned integer
    U64,
}

/// Float size variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatSize {
    /// 32-bit floating point
    F32,
    /// 64-bit floating point
    F64,
}

// ========== Composite Type Structures ==========

/// Field in a struct type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructField {
    /// Field name
    pub name: String,
    /// Field type
    pub type_id: TypeId,
}

/// Method in a struct type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructMethod {
    /// Method name
    pub name: String,
    /// Method type (must be a Function type)
    pub function_type: TypeId,
}

/// Complete struct type definition
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructType {
    /// Struct fields (order preserved from source)
    pub fields: Vec<StructField>,
    /// Struct methods
    pub methods: Vec<StructMethod>,
}

/// Parameter in a function type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionParam {
    /// Parameter name (part of function type signature in Suru)
    pub name: String,
    /// Parameter type
    pub type_id: TypeId,
}

/// Function type signature
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionType {
    /// Function parameters (with names)
    pub params: Vec<FunctionParam>,
    /// Return type
    pub return_type: TypeId,
}

// ========== TypeId ==========

/// Opaque identifier for types in the registry
///
/// TypeIds are created by [`TypeRegistry::intern`] and can be used to
/// retrieve types via [`TypeRegistry::get`]. TypeIds enable efficient
/// type comparison and support recursive types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(usize);

impl TypeId {
    /// Creates a new TypeId (internal use only)
    pub(crate) fn new(id: usize) -> Self {
        TypeId(id)
    }

    /// Gets the raw index (for debugging)
    pub fn index(&self) -> usize {
        self.0
    }
}

// ========== Type Enum ==========

/// Internal representation of types in the Suru type system
///
/// This enum represents all possible type forms in Suru. Composite types
/// use [`TypeId`] references to enable recursive types and efficient
/// structural sharing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    // Primitive types
    /// Unit type (empty type)
    Unit,
    /// Universal numeric type
    Number,
    /// String type
    String,
    /// Boolean type
    Bool,
    /// Sized signed integer
    Int(IntSize),
    /// Sized unsigned integer
    UInt(UIntSize),
    /// Sized floating point
    Float(FloatSize),

    // Composite types
    /// Struct type with fields and methods
    Struct(StructType),
    /// Union type (A, B, C)
    Union(Vec<TypeId>),
    /// Intersection type (A + B) - binary, chains for A+B+C
    Intersection(TypeId, TypeId),

    // Function types
    /// Function type with parameters and return type
    Function(FunctionType),

    // Generic types
    /// Type variable (unbound generic)
    TypeVar(String),
    /// Type parameter with optional constraint
    TypeParameter {
        /// Parameter name
        name: String,
        /// Optional constraint type
        constraint: Option<TypeId>,
    },

    // Collection types
    /// Array type
    Array(TypeId),
    /// Optional type
    Option(TypeId),
    /// Result type (Ok, Err)
    Result(TypeId, TypeId),

    // Special types
    /// Unknown type (for inference)
    Unknown,
    /// Error sentinel (for error recovery)
    Error,
}

// ========== TypeRegistry ==========

/// Registry for type interning and deduplication
///
/// The TypeRegistry stores all types and ensures that structurally identical
/// types receive the same [`TypeId`]. This enables efficient type comparison
/// and memory usage.
///
/// # Example
///
/// ```
/// use suru_lang::semantic::{Type, TypeRegistry, IntSize};
///
/// let mut registry = TypeRegistry::new();
///
/// // Intern types
/// let i32_1 = registry.intern(Type::Int(IntSize::I32));
/// let i32_2 = registry.intern(Type::Int(IntSize::I32));
///
/// // Identical types get same ID
/// assert_eq!(i32_1, i32_2);
/// ```
pub struct TypeRegistry {
    /// Storage: TypeId -> Type
    types: Vec<Type>,
    /// Interning cache: Type -> TypeId
    cache: HashMap<Type, TypeId>,
}

impl TypeRegistry {
    /// Creates a new empty type registry
    pub fn new() -> Self {
        TypeRegistry {
            types: Vec::new(),
            cache: HashMap::new(),
        }
    }

    /// Interns a type, returning its TypeId
    ///
    /// If the type already exists in the registry, returns the existing TypeId.
    /// If the type is new, allocates a new TypeId and stores the type.
    ///
    /// # Example
    ///
    /// ```
    /// use suru_lang::semantic::{Type, TypeRegistry};
    ///
    /// let mut registry = TypeRegistry::new();
    /// let num = registry.intern(Type::Number);
    /// assert_eq!(registry.get(num), &Type::Number);
    /// ```
    pub fn intern(&mut self, ty: Type) -> TypeId {
        // Check cache first
        if let Some(&type_id) = self.cache.get(&ty) {
            return type_id;
        }

        // New type - allocate ID
        let type_id = TypeId::new(self.types.len());
        self.types.push(ty.clone());
        self.cache.insert(ty, type_id);
        type_id
    }

    /// Gets type by ID
    ///
    /// # Panics
    ///
    /// Panics if the TypeId is invalid. This should never happen if TypeIds
    /// are only created via [`TypeRegistry::intern`].
    pub fn get(&self, type_id: TypeId) -> &Type {
        &self.types[type_id.0]
    }

    /// Returns the number of unique types in the registry
    pub fn len(&self) -> usize {
        self.types.len()
    }

    /// Checks if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ========== Tests ==========

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Test Group 1: Primitives ==========

    #[test]
    fn test_intern_primitives() {
        let mut registry = TypeRegistry::new();

        let num1 = registry.intern(Type::Number);
        let num2 = registry.intern(Type::Number);

        // Same primitive type should get same TypeId
        assert_eq!(num1, num2);
        assert_eq!(registry.len(), 1);

        let str_id = registry.intern(Type::String);
        assert_ne!(num1, str_id);
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_intern_sized_ints() {
        let mut registry = TypeRegistry::new();

        let i32_1 = registry.intern(Type::Int(IntSize::I32));
        let i32_2 = registry.intern(Type::Int(IntSize::I32));
        let i64 = registry.intern(Type::Int(IntSize::I64));

        assert_eq!(i32_1, i32_2);
        assert_ne!(i32_1, i64);
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_get_primitive() {
        let mut registry = TypeRegistry::new();
        let num_id = registry.intern(Type::Number);

        assert_eq!(registry.get(num_id), &Type::Number);
    }

    #[test]
    fn test_all_primitive_types() {
        let mut registry = TypeRegistry::new();

        let unit = registry.intern(Type::Unit);
        let num = registry.intern(Type::Number);
        let str = registry.intern(Type::String);
        let bool = registry.intern(Type::Bool);

        assert_eq!(registry.get(unit), &Type::Unit);
        assert_eq!(registry.get(num), &Type::Number);
        assert_eq!(registry.get(str), &Type::String);
        assert_eq!(registry.get(bool), &Type::Bool);
    }

    #[test]
    fn test_all_int_sizes() {
        let mut registry = TypeRegistry::new();

        let i8 = registry.intern(Type::Int(IntSize::I8));
        let i16 = registry.intern(Type::Int(IntSize::I16));
        let i32 = registry.intern(Type::Int(IntSize::I32));
        let i64 = registry.intern(Type::Int(IntSize::I64));

        assert_ne!(i8, i16);
        assert_ne!(i16, i32);
        assert_ne!(i32, i64);
        assert_eq!(registry.len(), 4);
    }

    #[test]
    fn test_all_uint_sizes() {
        let mut registry = TypeRegistry::new();

        let u8 = registry.intern(Type::UInt(UIntSize::U8));
        let u16 = registry.intern(Type::UInt(UIntSize::U16));
        let u32 = registry.intern(Type::UInt(UIntSize::U32));
        let u64 = registry.intern(Type::UInt(UIntSize::U64));

        assert_ne!(u8, u16);
        assert_ne!(u16, u32);
        assert_ne!(u32, u64);
        assert_eq!(registry.len(), 4);
    }

    #[test]
    fn test_all_float_sizes() {
        let mut registry = TypeRegistry::new();

        let f32 = registry.intern(Type::Float(FloatSize::F32));
        let f64 = registry.intern(Type::Float(FloatSize::F64));

        assert_ne!(f32, f64);
        assert_eq!(registry.len(), 2);
    }

    // ========== Test Group 2: Unions ==========

    #[test]
    fn test_intern_union_same_order() {
        let mut registry = TypeRegistry::new();

        let num = registry.intern(Type::Number);
        let str = registry.intern(Type::String);

        let union1 = registry.intern(Type::Union(vec![num, str]));
        let union2 = registry.intern(Type::Union(vec![num, str]));

        // Identical unions should deduplicate
        assert_eq!(union1, union2);
        assert_eq!(registry.len(), 3); // Number, String, Union
    }

    #[test]
    fn test_intern_union_different_order() {
        let mut registry = TypeRegistry::new();

        let num = registry.intern(Type::Number);
        let str = registry.intern(Type::String);

        let union1 = registry.intern(Type::Union(vec![num, str]));
        let union2 = registry.intern(Type::Union(vec![str, num]));

        // Different order = different union (no normalization)
        assert_ne!(union1, union2);
    }

    #[test]
    fn test_intern_union_three_types() {
        let mut registry = TypeRegistry::new();

        let num = registry.intern(Type::Number);
        let str = registry.intern(Type::String);
        let bool = registry.intern(Type::Bool);

        let union = registry.intern(Type::Union(vec![num, str, bool]));

        assert_eq!(registry.get(union), &Type::Union(vec![num, str, bool]));
    }

    #[test]
    fn test_intern_empty_union() {
        let mut registry = TypeRegistry::new();

        let union1 = registry.intern(Type::Union(vec![]));
        let union2 = registry.intern(Type::Union(vec![]));

        assert_eq!(union1, union2);
    }

    // ========== Test Group 3: Intersections ==========

    #[test]
    fn test_intern_intersection() {
        let mut registry = TypeRegistry::new();

        let num = registry.intern(Type::Number);
        let str = registry.intern(Type::String);

        let inter1 = registry.intern(Type::Intersection(num, str));
        let inter2 = registry.intern(Type::Intersection(num, str));

        assert_eq!(inter1, inter2);
    }

    #[test]
    fn test_intern_intersection_chained() {
        // Simulates: A + B + C (parsed as (A + B) + C)
        let mut registry = TypeRegistry::new();

        let a = registry.intern(Type::Number);
        let b = registry.intern(Type::String);
        let c = registry.intern(Type::Bool);

        let ab = registry.intern(Type::Intersection(a, b));
        let abc = registry.intern(Type::Intersection(ab, c));

        // Verify structure
        match registry.get(abc) {
            Type::Intersection(left, right) => {
                assert_eq!(*left, ab);
                assert_eq!(*right, c);
            }
            _ => panic!("Expected intersection"),
        }
    }

    #[test]
    fn test_intern_intersection_different_order() {
        let mut registry = TypeRegistry::new();

        let a = registry.intern(Type::Number);
        let b = registry.intern(Type::String);

        let ab = registry.intern(Type::Intersection(a, b));
        let ba = registry.intern(Type::Intersection(b, a));

        // Different order = different intersection
        assert_ne!(ab, ba);
    }

    // ========== Test Group 4: Structs ==========

    #[test]
    fn test_intern_empty_struct() {
        let mut registry = TypeRegistry::new();

        let empty = StructType {
            fields: vec![],
            methods: vec![],
        };

        let struct_id = registry.intern(Type::Struct(empty.clone()));
        assert_eq!(registry.get(struct_id), &Type::Struct(empty));
    }

    #[test]
    fn test_intern_struct_with_fields() {
        let mut registry = TypeRegistry::new();

        let num = registry.intern(Type::Number);
        let str = registry.intern(Type::String);

        let person = StructType {
            fields: vec![
                StructField {
                    name: "name".to_string(),
                    type_id: str,
                },
                StructField {
                    name: "age".to_string(),
                    type_id: num,
                },
            ],
            methods: vec![],
        };

        let person_id = registry.intern(Type::Struct(person.clone()));
        assert_eq!(registry.get(person_id), &Type::Struct(person));
    }

    #[test]
    fn test_intern_struct_deduplication() {
        let mut registry = TypeRegistry::new();

        let num = registry.intern(Type::Number);

        let struct1 = StructType {
            fields: vec![StructField {
                name: "x".to_string(),
                type_id: num,
            }],
            methods: vec![],
        };

        let struct2 = struct1.clone();

        let id1 = registry.intern(Type::Struct(struct1));
        let id2 = registry.intern(Type::Struct(struct2));

        // Identical structs should deduplicate
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_intern_struct_different_field_order() {
        let mut registry = TypeRegistry::new();

        let num = registry.intern(Type::Number);
        let str = registry.intern(Type::String);

        let struct1 = StructType {
            fields: vec![
                StructField {
                    name: "a".to_string(),
                    type_id: num,
                },
                StructField {
                    name: "b".to_string(),
                    type_id: str,
                },
            ],
            methods: vec![],
        };

        let struct2 = StructType {
            fields: vec![
                StructField {
                    name: "b".to_string(),
                    type_id: str,
                },
                StructField {
                    name: "a".to_string(),
                    type_id: num,
                },
            ],
            methods: vec![],
        };

        let id1 = registry.intern(Type::Struct(struct1));
        let id2 = registry.intern(Type::Struct(struct2));

        // Different field order = different struct
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_intern_struct_with_methods() {
        let mut registry = TypeRegistry::new();

        let num = registry.intern(Type::Number);
        let unit = registry.intern(Type::Unit);

        let func_type = FunctionType {
            params: vec![],
            return_type: unit,
        };
        let func_id = registry.intern(Type::Function(func_type));

        let struct_type = StructType {
            fields: vec![StructField {
                name: "x".to_string(),
                type_id: num,
            }],
            methods: vec![StructMethod {
                name: "reset".to_string(),
                function_type: func_id,
            }],
        };

        let struct_id = registry.intern(Type::Struct(struct_type.clone()));
        assert_eq!(registry.get(struct_id), &Type::Struct(struct_type));
    }

    // ========== Test Group 5: Functions ==========

    #[test]
    fn test_intern_function_no_params() {
        let mut registry = TypeRegistry::new();

        let unit = registry.intern(Type::Unit);

        let func = FunctionType {
            params: vec![],
            return_type: unit,
        };

        let func_id = registry.intern(Type::Function(func.clone()));
        assert_eq!(registry.get(func_id), &Type::Function(func));
    }

    #[test]
    fn test_intern_function_with_params() {
        let mut registry = TypeRegistry::new();

        let num = registry.intern(Type::Number);

        let func = FunctionType {
            params: vec![
                FunctionParam {
                    name: "x".to_string(),
                    type_id: num,
                },
                FunctionParam {
                    name: "y".to_string(),
                    type_id: num,
                },
            ],
            return_type: num,
        };

        let func_id = registry.intern(Type::Function(func.clone()));
        assert_eq!(registry.get(func_id), &Type::Function(func));
    }

    #[test]
    fn test_intern_function_deduplication() {
        let mut registry = TypeRegistry::new();

        let num = registry.intern(Type::Number);

        let func1 = FunctionType {
            params: vec![FunctionParam {
                name: "x".to_string(),
                type_id: num,
            }],
            return_type: num,
        };

        let func2 = func1.clone();

        let id1 = registry.intern(Type::Function(func1));
        let id2 = registry.intern(Type::Function(func2));

        assert_eq!(id1, id2);
    }

    #[test]
    fn test_intern_function_different_param_names() {
        let mut registry = TypeRegistry::new();

        let num = registry.intern(Type::Number);

        let func1 = FunctionType {
            params: vec![FunctionParam {
                name: "x".to_string(),
                type_id: num,
            }],
            return_type: num,
        };

        let func2 = FunctionType {
            params: vec![FunctionParam {
                name: "y".to_string(),
                type_id: num,
            }],
            return_type: num,
        };

        let id1 = registry.intern(Type::Function(func1));
        let id2 = registry.intern(Type::Function(func2));

        // Different param names = different function type
        assert_ne!(id1, id2);
    }

    // ========== Test Group 6: Recursive Types ==========

    #[test]
    fn test_recursive_struct() {
        // Simulates: type Node: { value Number, next Node }
        let mut registry = TypeRegistry::new();

        let num = registry.intern(Type::Number);

        // Create placeholder for recursive reference
        let node_id = TypeId::new(999);

        let node_struct = StructType {
            fields: vec![
                StructField {
                    name: "value".to_string(),
                    type_id: num,
                },
                StructField {
                    name: "next".to_string(),
                    type_id: node_id,
                },
            ],
            methods: vec![],
        };

        // Verify the structure supports recursive references
        let _actual_node_id = registry.intern(Type::Struct(node_struct));
        // In real usage, we'd fix up the recursive reference
    }

    // ========== Test Group 7: Generics ==========

    #[test]
    fn test_intern_type_var() {
        let mut registry = TypeRegistry::new();

        let t1 = registry.intern(Type::TypeVar("T".to_string()));
        let t2 = registry.intern(Type::TypeVar("T".to_string()));
        let k = registry.intern(Type::TypeVar("K".to_string()));

        assert_eq!(t1, t2);
        assert_ne!(t1, k);
    }

    #[test]
    fn test_intern_type_parameter_no_constraint() {
        let mut registry = TypeRegistry::new();

        let param = Type::TypeParameter {
            name: "T".to_string(),
            constraint: None,
        };

        let id = registry.intern(param.clone());
        assert_eq!(registry.get(id), &param);
    }

    #[test]
    fn test_intern_type_parameter_with_constraint() {
        let mut registry = TypeRegistry::new();

        let number = registry.intern(Type::Number);

        let param = Type::TypeParameter {
            name: "T".to_string(),
            constraint: Some(number),
        };

        let id = registry.intern(param.clone());
        assert_eq!(registry.get(id), &param);
    }

    #[test]
    fn test_type_parameter_different_constraints() {
        let mut registry = TypeRegistry::new();

        let number = registry.intern(Type::Number);
        let string = registry.intern(Type::String);

        let param1 = Type::TypeParameter {
            name: "T".to_string(),
            constraint: Some(number),
        };

        let param2 = Type::TypeParameter {
            name: "T".to_string(),
            constraint: Some(string),
        };

        let id1 = registry.intern(param1);
        let id2 = registry.intern(param2);

        // Different constraints = different type parameters
        assert_ne!(id1, id2);
    }

    // ========== Test Group 8: Collections ==========

    #[test]
    fn test_intern_array() {
        let mut registry = TypeRegistry::new();

        let num = registry.intern(Type::Number);
        let arr = registry.intern(Type::Array(num));

        assert_eq!(registry.get(arr), &Type::Array(num));
    }

    #[test]
    fn test_intern_option() {
        let mut registry = TypeRegistry::new();

        let str = registry.intern(Type::String);
        let opt = registry.intern(Type::Option(str));

        assert_eq!(registry.get(opt), &Type::Option(str));
    }

    #[test]
    fn test_intern_result() {
        let mut registry = TypeRegistry::new();

        let num = registry.intern(Type::Number);
        let str = registry.intern(Type::String);
        let res = registry.intern(Type::Result(num, str));

        assert_eq!(registry.get(res), &Type::Result(num, str));
    }

    #[test]
    fn test_nested_collections() {
        let mut registry = TypeRegistry::new();

        let num = registry.intern(Type::Number);
        let arr_num = registry.intern(Type::Array(num));
        let opt_arr_num = registry.intern(Type::Option(arr_num));

        assert_eq!(registry.get(opt_arr_num), &Type::Option(arr_num));
    }

    // ========== Test Group 9: Special Types ==========

    #[test]
    fn test_intern_unknown() {
        let mut registry = TypeRegistry::new();

        let u1 = registry.intern(Type::Unknown);
        let u2 = registry.intern(Type::Unknown);

        assert_eq!(u1, u2);
    }

    #[test]
    fn test_intern_error() {
        let mut registry = TypeRegistry::new();

        let e1 = registry.intern(Type::Error);
        let e2 = registry.intern(Type::Error);

        assert_eq!(e1, e2);
    }

    // ========== Test Group 10: Registry Operations ==========

    #[test]
    fn test_registry_empty() {
        let registry = TypeRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_len() {
        let mut registry = TypeRegistry::new();

        registry.intern(Type::Number);
        assert_eq!(registry.len(), 1);

        registry.intern(Type::Number); // Duplicate
        assert_eq!(registry.len(), 1); // No change

        registry.intern(Type::String);
        assert_eq!(registry.len(), 2);
    }

    #[test]
    #[should_panic]
    fn test_get_invalid_type_id() {
        let registry = TypeRegistry::new();
        let invalid_id = TypeId::new(999);
        registry.get(invalid_id); // Should panic
    }

    #[test]
    fn test_type_id_equality() {
        let id1 = TypeId::new(0);
        let id2 = TypeId::new(0);
        let id3 = TypeId::new(1);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_type_id_index() {
        let id = TypeId::new(42);
        assert_eq!(id.index(), 42);
    }

    #[test]
    fn test_type_id_copy() {
        let id1 = TypeId::new(42);
        let id2 = id1; // Should copy, not move
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_default_registry() {
        let registry = TypeRegistry::default();
        assert!(registry.is_empty());
    }

    // ========== Test Group 11: Integration ==========

    #[test]
    fn test_complex_type_scenario() {
        // Simulates realistic type system usage:
        // type Person: { name String, age Number }
        // type Manager: Person + { reports Array }
        // type Result: Success, Error

        let mut registry = TypeRegistry::new();

        // Basic types
        let str = registry.intern(Type::String);
        let num = registry.intern(Type::Number);

        // Person struct
        let person = StructType {
            fields: vec![
                StructField {
                    name: "name".to_string(),
                    type_id: str,
                },
                StructField {
                    name: "age".to_string(),
                    type_id: num,
                },
            ],
            methods: vec![],
        };
        let person_id = registry.intern(Type::Struct(person));

        // Manager struct (will be intersected with Person)
        let manager_extra = StructType {
            fields: vec![StructField {
                name: "reports".to_string(),
                type_id: registry.intern(Type::Array(person_id)),
            }],
            methods: vec![],
        };
        let manager_extra_id = registry.intern(Type::Struct(manager_extra));

        // Manager = Person + ManagerExtra
        let manager = registry.intern(Type::Intersection(person_id, manager_extra_id));

        // Result union
        let success = registry.intern(Type::Unit);
        let error = registry.intern(Type::String);
        let result = registry.intern(Type::Union(vec![success, error]));

        // Verify all types are distinct and retrievable
        assert_ne!(person_id, manager);
        assert_ne!(person_id, result);
        assert!(matches!(registry.get(manager), Type::Intersection(_, _)));
        assert!(matches!(registry.get(result), Type::Union(_)));
    }
}
