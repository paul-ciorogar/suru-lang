# Changelog

All notable changes to Suru Lang will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.35.0] - 2026-02-05 - Struct Privacy Enforcement

### Added
- **Struct privacy enforcement**
  - Privacy tracking: `is_private` field on `StructField` and `StructMethod` in type system
  - Privacy propagation from AST `NodeFlags::IS_PRIVATE` during struct init processing
  - Property access privacy enforcement: `Cannot access private field 'X'`
  - Method call privacy enforcement: `Cannot access private method 'X'`
  - Privacy helper methods: `is_field_private()`, `is_method_private()`
  - Basic `visit_property_access` and `visit_method_call` visitors (privacy-only; full type checking in 6.4/6.5)
  - Comma-separated members in struct type definitions (parser enhancement)
  - 13 new tests (605 total tests passing)

### Technical Details
- **New module**: `src/semantic/struct_privacy.rs`
  - `is_field_private()` - Checks if a struct field is private by name
  - `is_method_private()` - Checks if a struct method is private by name
  - `visit_property_access()` - Visits receiver, resolves type, checks field privacy
  - `visit_method_call()` - Visits receiver, resolves type, checks method privacy
- **Updated** `src/semantic/types.rs`: Added `is_private: bool` to `StructField` and `StructMethod`
- **Updated** `src/semantic/struct_init_type_checking.rs`: Reads `NodeFlags::IS_PRIVATE` from AST nodes
- **Updated** `src/semantic/mod.rs`: Registered `PropertyAccess` and `MethodCall` in visitor dispatch
- **Updated** `src/parser/types.rs`: Added optional comma separator between struct body members

### Privacy Design
- Type definitions are interfaces — they do NOT have private members
- Only struct initializations can mark fields/methods as private using `_` prefix
- Private members are tracked in the type system via `is_private: bool`
- External access to private members produces semantic errors

### Examples
```suru
// Private fields and methods in struct init
user: {
    name: "Paul"
    _ password: "secret"
    greet: () String { return "hello" }
    _ validate: () Bool { return true }
}

// Public access OK
x: user.name           // OK
x: user.greet()        // OK

// Private access blocked
x: user.password       // Error: Cannot access private field 'password'
x: user.validate()     // Error: Cannot access private method 'validate'

// Private extras with typed struct (structural subtyping)
type Person: { name String }
p Person: { name: "Paul", _ secret: "password" }  // OK
```

---

## [0.34.0] - 2026-02-05 - Struct Initialization Type Checking

### Added
- **Struct initialization type checking**
  - Field type validation against declared struct type
  - Method signature validation (parameter types and return type)
  - Missing field/method detection
  - Structural subtyping: extra fields in literal allowed
  - Nested struct literal support
  - Struct-to-struct unification in Hindley-Milner system
  - 14 new tests (592 total tests passing)

### Technical Details
- **New module**: `src/semantic/struct_init_type_checking.rs`
  - `visit_struct_init()` - Infers struct type from field/method initializations
  - `build_function_type_from_decl()` - Constructs FunctionType from method FunctionDecl
- **Updated** `src/semantic/unification.rs`: Struct-to-struct unification (field/method existence and type checking)
- **Updated** `src/parser/struct_init.rs`: Added function name Identifier to FunctionDecl for AST consistency

### Examples
```suru
// Valid struct initialization
type Point: { x Number, y Number }
p Point: { x: 10, y: 20 }

// Missing field → Error: Missing field 'y' in struct literal
p Point: { x: 10 }

// Field type mismatch → Error: Type mismatch
p Point: { x: "hello" }

// Extra fields allowed (structural subtyping)
p Point: { x: 10, y: 20, z: 30 }  // OK

// Method signature validation
type Greeter: { greet: () String }
g Greeter: { greet: () String { return "hello" } }  // OK
g Greeter: { greet: () Number { return 42 } }        // Error
```

---

## [0.33.0] - 2026-02-01 - Struct Type Definition

### Added
- **Struct type definition processing**
  - Struct method parsing and type construction
  - Function type building from method signatures
  - Parameter type validation for method parameters
  - Return type validation for method return types
  - Support for methods with no parameters, single parameter, and multiple parameters
  - Mixed fields and methods in same struct
  - User-defined type references in method signatures
  - 14 new tests (578 total tests passing)

### Technical Details
- **New module**: `src/semantic/struct_type_definition.rs`
  - `process_struct_type_definition()` - Main entry point for struct body processing
  - `process_struct_field_definition()` - Handles struct fields (name Type)
  - `process_struct_method()` - Handles struct methods (name: (params) ReturnType)
  - `process_function_type_definition()` - Builds FunctionType from AST
  - `process_function_type_params()` - Extracts parameters from FunctionTypeParams
- **Updated** `src/semantic/type_resolution.rs`:
  - Delegates struct processing to new module
  - Removed duplicate field processing code
- **Type construction flow**:
  1. Iterate StructBody children
  2. Process StructField nodes → Vec<StructField>
  3. Process StructMethod nodes → build FunctionType → Vec<StructMethod>
  4. Create and intern StructType with both fields and methods

### Method Type Construction
```
// Input struct declaration
type Calculator: {
    add: (x Number, y Number) Number
}

// Resulting StructType
StructType {
    fields: [],
    methods: [
        StructMethod {
            name: "add",
            function_type: FunctionType {
                params: [
                    FunctionParam { name: "x", type_id: <Number> },
                    FunctionParam { name: "y", type_id: <Number> },
                ],
                return_type: <Number>
            }
        }
    ]
}
```

### Examples
```suru
// Method with no parameters
type Greeter: {
    greet: () String
}

// Method with multiple parameters
type Calculator: {
    add: (x Number, y Number) Number
    subtract: (x Number, y Number) Number
}

// Mixed fields and methods
type Person: {
    name String
    age Number
    greet: () String
}

// User-defined types in methods
type Point: { x Number }
type Factory: {
    createPoint: () Point
    process: (p Point) Number
}
```

### Error Cases
```suru
// Undefined parameter type
type Foo: {
    bar: (x UndefinedType) Number  // Error: Type 'UndefinedType' is not defined
}

// Undefined return type
type Foo: {
    bar: (x Number) UndefinedType  // Error: Type 'UndefinedType' is not defined
}
```

---

## [0.32.0] - 2026-01-31 - Module Declaration Processing

### Added
- **Module declaration processing**
  - Main module registration: `module Calculator`, `module math.geometry`
  - Submodule support: `module .utils` (dot-prefixed names)
  - Module symbol table entries with `SymbolKind::Module`
  - Module scope creation for subsequent declarations
  - One-module-per-file validation
  - 17 new tests (564 total tests passing)

### Technical Details
- **New module**: `src/semantic/module_resolution.rs`
  - `visit_module_decl()` - Core visitor method for module declarations
  - Extracts module path from `ModulePath` child node
  - Distinguishes main modules from submodules via leading dot
  - Strips leading dot from submodule names for storage
  - Registers module symbol with type info ("module" or "submodule")
  - Enters `ScopeKind::Module` for subsequent declarations
- **New SymbolKind variant**: `SymbolKind::Module` for module symbols
- **New SemanticAnalyzer fields**:
  - `current_module: Option<String>` - Current module name (None if not in a module)
  - `is_submodule: bool` - Whether current module is a submodule
- **Updated dispatcher**: Added `NodeType::ModuleDecl` to `visit_node()`

### Module Types
```
Main module:
  module Calculator        → name: "Calculator", type: "module"
  module math.geometry     → name: "math.geometry", type: "module"

Submodule:
  module .utils            → name: "utils", type: "submodule"
  module .helpers          → name: "helpers", type: "submodule"
```

### Examples
```suru
// Main module with declarations
module Calculator

type CalcResult: Number
version: 1

add: (x Number, y Number) Number {
    return x
}

// Submodule
module .utils

helper: () { }
```

### Error Cases
```suru
// Multiple modules (error)
module First
module Second  // Error: Only one module declaration allowed per file

// Multiple modules with code between (error)
module First
x: 42
module Second  // Error: Only one module declaration allowed per file
```

---

## [0.31.0] - 2026-01-26 - Function Call Type Checking

### Added
- **Function call type checking**
  - Argument count validation: Error if argument count doesn't match parameter count
  - Argument type checking: Constraints added for each argument against parameter type
  - Return type propagation: Function call nodes get the function's return type
  - Variable reference type tracking: Identifiers now have their types set from scope
  - 27 new tests (547 total tests passing)

### Technical Details
- **New module**: `src/semantic/function_call_type_checking.rs`
  - `type_check_function_call()` - Core validation method
  - `count_call_arguments()` - Counts arguments in ArgList
- **Updated** `visit_identifier()` in `name_resolution.rs`:
  - Now sets node type for identifier references based on variable's type
  - Enables type checking for variable arguments in function calls
- **Integration**: Called from `visit_function_call()` after visiting arguments
- **Constraint-based**: Uses Hindley-Milner unification for type checking

### Validation Cases
```
1. Argument count mismatch:
   - Error: "Function 'foo' expects N argument(s) but got M"

2. Argument type mismatch:
   - Constraint added: arg_type = param_type
   - Unification reports type mismatch errors

3. Unknown parameter types:
   - Parameters with Type::Unknown skip type checking
   - Allows type inference from actual arguments
```

### Examples
```suru
// Valid - correct argument count and types
add: (x Number, y Number) Number { return 1 }
z: add(42, 99)

// Error - too few arguments
z: add(42)  // Function 'add' expects 2 argument(s) but got 1

// Error - too many arguments
z: add(1, 2, 3)  // Function 'add' expects 2 argument(s) but got 3

// Error - type mismatch
z: add(42, "hello")  // Type mismatch: String vs Number

// Valid - variable argument
n: 42
z: add(n, 10)

// Error - variable type mismatch
s: "hello"
z: add(s, 10)  // Type mismatch: String vs Number

// Valid - untyped parameter accepts any type
identity: (x) { return x }
z: identity(42)
z: identity("hello")
```

---

## [0.30.0] - 2026-01-26 - Return Type Validation

### Added
- **Return type validation**
  - Return type matching: All return statements validated against declared return type
  - Return type inference: Functions without return annotation infer type from returns
  - Missing return detection: Functions with declared return type must have returns
  - Void return handling: Bare `return` in non-void functions produces error
  - Nested function isolation: Each function tracks returns independently
  - 26 new tests (520 total tests passing)

### Technical Details
- **New module**: `src/semantic/return_type_validation.rs`
  - `validate_function_returns()` - Core validation method
  - `function_has_body_statements()` - Checks for empty function stubs
  - `get_function_name()` - Extracts function name for error messages
- **Integration**: Called from `visit_function_decl()` after body analysis
- **Constraint-based**: Uses Hindley-Milner unification for type checking
  - Declared type: `add_constraint(return_type, declared_type)`
  - Inferred type: All returns constrained to equal each other

### Validation Cases
```
1. Declared return type (e.g., Number):
   - Each return value must match declared type
   - Bare 'return' produces error
   - Missing returns produce error

2. No return annotation (Unknown):
   - Infer type from actual returns
   - Multiple returns must be consistent

3. Void return type:
   - Only bare 'return' allowed
   - Return with value produces error
```

### Examples
```suru
// Valid - matching return type
getNum: () Number {
    return 42
}

// Error - type mismatch
getNum: () Number {
    return "hello"  // Type mismatch: String vs Number
}

// Error - bare return in non-void function
getNum: () Number {
    return  // Cannot use bare 'return' in function with return type
}

// Error - missing return
getNum: () Number {
    x: 42  // Function must have at least one return statement
}

// Valid - inferred return type
getValue: () {
    return 42  // Return type inferred as Number
}

// Error - inconsistent inferred returns
getValue: () {
    return 42
    return "text"  // Type mismatch: String vs Number
}

// Valid - nested functions have separate return types
outer: () Number {
    inner: () String {
        return "hello"
    }
    return 42
}
```

---

## [0.29.0] - 2026-01-19 - Function Body Analysis

### Added
- **Function body analysis**
  - `Type::Void` variant for functions with no return value
  - Function context tracking with `current_function_stack` for nested functions
  - Return statement type inference and recording via `function_returns` map
  - Error detection for return statements outside functions
  - 18 new tests (499 total tests passing)

### Technical Details
- **New module**: `src/semantic/function_body_analysis.rs` with `visit_return_stmt()`
- **New fields** in `SemanticAnalyzer`: `function_returns`, `current_function_stack`
- **New helpers**: `enter_function_context()`, `exit_function_context()`, `current_function()`, `record_return()`, `get_function_returns()`
- **Updated** `visit_function_decl()` to track function context

### Examples
```suru
foo: () { return 42 }      // Records Number type
bar: () { return }         // Records Void type
return 42                  // Error: outside function
```

---

## [0.28.0] - 2026-01-19 - Function Signature Analysis

### Added
- **Function signature analysis**
  - Structured `FunctionType` construction from function declarations
  - Parameter type resolution with `FunctionParam` entries
  - Return type resolution from annotations
  - `Type::Unknown` for untyped parameters (enables future inference)
  - `Type::Unknown` for missing return types (enables future inference)
  - TypeId storage in Symbol via new `type_id` field
  - Backward compatibility: String signatures preserved alongside structured types
  - 11 new tests (482 total tests passing)

### Technical Details
- **Extended Symbol struct** in `src/semantic/mod.rs`:
  - Added `type_id: Option<TypeId>` field for structured type storage
  - Added `with_type_id()` builder method for fluent construction
- **New method** in `src/semantic/name_resolution.rs`:
  - `build_function_type()` creates `FunctionType` with proper `TypeId` references
  - Resolves type annotations via `lookup_type_id()`
  - Uses `Type::Unknown` for untyped/missing types
- **Updated `visit_function_decl()`**: Now builds and stores structured function type
- **Test module**: `function_signature_tests` with comprehensive coverage

### Function Type Construction
```
// Input function declaration
add: (x Number, y Number) Number { }

// Resulting FunctionType
FunctionType {
    params: [
        FunctionParam { name: "x", type_id: <Number> },
        FunctionParam { name: "y", type_id: <Number> },
    ],
    return_type: <Number>
}

// Untyped parameters get Unknown
identity: (x) { }
// → FunctionParam { name: "x", type_id: <Unknown> }

// Missing return type gets Unknown
doSomething: () { }
// → return_type: <Unknown>
```

### Design Decisions
- **Additive change**: New `type_id` field added without breaking existing code
- **String signature preserved**: `type_name` still contains signature string for display
- **Unknown for inference**: Untyped elements use `Type::Unknown` for future Hindley-Milner inference
- **User-defined types supported**: Type annotations resolve through symbol table lookup

---

## [0.27.0] - 2026-01-16 - Assignment Type Checking

### Added
- **Assignment type checking**
  - Constant immutability at file level: Variables declared at module/global scope are constants
  - Constant redeclaration errors: `Cannot redeclare constant 'x'`
  - Variable reassignment in mutable scopes (functions/blocks)
  - Type checking for reassignments: Reassigned values must match original variable type
  - Shadowing support: Inner scopes can shadow outer variables with any type
  - Scope-aware type tracking via `variable_types` map

### Technical Details
- **New module**: `src/semantic/assignment_type_checking.rs`
  - Added `variable_types: HashMap<(usize, String), TypeId>` for tracking variable types per scope
  - Added `is_in_mutable_scope()` to ScopeStack for detecting function/block contexts
  - Added `lookup_variable_type()` helper for scope chain lookup
  - Added `record_variable_type()` helper for recording declarations
  - Checks if variable exists in current scope before inserting
  - Generates constraint for type matching on reassignment
  - Records variable type on new declarations

### Scope Semantics
```
File/Module scope (immutable):
  x: 42           // Constant declaration
  x: 99           // Error: Cannot redeclare constant 'x'

Function/Block scope (mutable):
  foo: () {
      x: 42       // Variable declaration
      x: 99       // OK: Reassignment with same type
      x: "text"   // Error: Type mismatch (String vs Number)
  }

Shadowing (always allowed):
  x: 42           // Outer constant
  foo: () {
      x: "text"   // OK: Shadows outer, different type allowed
  }

  value: 100
  outer: () {
    value: "shadowed"        // OK: Shadows file-level constant
    inner: () {
        value: true          // OK: Shadows outer function variable
    }
  }
```

### Design Decisions
- **File-level immutability**
- **Mutable function scopes**: Enables practical imperative code within functions
- **Shadowing allowed**: Provides flexibility without breaking type safety
- **Constraint-based checking**: Leverages existing Hindley-Milner unification

---

## [0.26.0] - 2026-01-14 - Variable Declaration Type Checking

### Added
- **Variable declaration type checking**
  - Type annotation validation: Verifies declared types exist (e.g., `x Number: 42`)
  - Initializer type checking: Ensures initializer matches declared type
  - Type inference without annotation: Infers type from initializer (e.g., `x: 42` → Number)
  - Constraint generation for type checking:
    - With annotation: `init_type = declared_type`
    - Without annotation: variable gets inferred type
  - Support for all built-in types: Number, String, Bool, Int8-64, UInt8-64, Float32-64
  - Error reporting for undefined types and type mismatches

### Technical Details
- **Updated**: `src/semantic/name_resolution.rs`
  - Extended `visit_var_decl()` method with type checking logic (~30 lines)
  - Added `lookup_type_id()` helper already existed in SemanticAnalyzer
  - Resolves type annotation to TypeId before checking
  - Visits initializer expression to infer its type
  - Generates constraint or assigns inferred type
  - New test module `variable_type_tests` with 15 tests
- **Type checking flow**:
  1. Resolve type annotation (if present) to TypeId
  2. Visit initializer expression to infer its type
  3. Generate constraint: `init_type = declared_type` (with annotation)
  4. Or assign inferred type directly (without annotation)
  5. Unification phase validates constraints
- **Variable redeclaration**: Each declaration analyzed independently (redeclaration allowed)

### Type Rules
```
With annotation:
  init_expr : T1    declared_type : T2    T1 = T2
  ------------------------------------------------
         var name declared_type: init_expr : T2

Without annotation:
  init_expr : T
  ---------------------------------
  var name: init_expr : T
```

### Examples
```suru
// With type annotation
x Number: 42              // Success: 42 is Number
y Bool: true and false    // Success: expression is Bool
z Int64: 42               // Success: variable gets Int64 type

// Type annotation errors
invalid1 Number: "text"   // Error: Type mismatch (String vs Number)
invalid2 Foo: 42          // Error: Type 'Foo' is not defined

// Without type annotation (inference)
a: 42                     // Infers Number
b: "hello"                // Infers String
c: not true               // Infers Bool
d: -99                    // Infers Number

// Variable redeclaration (allowed)
x Number: 42              // x is Number
x String: "hello"         // x is now String (replaces previous)
```

### Design Decisions
- **In-place extension**: Type checking added directly to `visit_var_decl()` (not separate module)
- **Graceful error handling**: Continue analysis even when type annotation fails
- **Constraint-based**: Leverages existing Hindley-Milner unification
- **Built-in types only**: User-defined types in annotations not yet supported

---

## [0.25.0] - 2026-01-14 - Expression Type Checking

### Added
- **Operator type checking**
  - Binary boolean operators (`and`, `or`):
    - Both operands must be `Bool` → result is `Bool`
    - Generates constraints for operand types
    - Type errors for non-boolean operands
  - Unary `not` operator:
    - Operand must be `Bool` → result is `Bool`
    - Generates constraint for operand type
    - Type errors for non-boolean operands
  - Unary negate operator (`-`):
    - Operand must be `Number` → result is `Number`
    - Uses universal `Number` type
    - Type errors for non-numeric operands
  - Full integration with Hindley-Milner constraint system

### Technical Details
- **New module**: `src/semantic/expression_type_inference.rs` (~260 lines)
  - `visit_binary_bool_op()` - Type inference for `and` and `or` operators
  - `visit_not()` - Type inference for `not` operator
  - `visit_negate()` - Type inference for negate operator
  - All methods generate constraints integrated with existing unification system
- **Updated**: `src/semantic/mod.rs`
  - Added module declaration for `expression_type_inference`
  - Updated `visit_node()` dispatcher to route operator nodes to new visitor methods
- **Type checking approach**: Constraint-based using Hindley-Milner
  - Operators visit children first (bottom-up)
  - Get operand types from child nodes
  - Generate equality constraints (e.g., `operand_type = Bool`)
  - Result types set on operator nodes
  - Unification phase solves constraints and reports type errors
- **Error reporting**: Leverages existing unification error messages
  - "Type mismatch: cannot unify Number with Bool"
  - Precise source location tracking via AST node tokens

### Type Rules
```
Binary Boolean (and, or):
  e1 : Bool    e2 : Bool
  ------------------------
      e1 op e2 : Bool

Unary Not:
  e : Bool
  ------------
  not e : Bool

Unary Negate:
  e : Number
  -------------
    -e : Number
```

### Examples
```suru
// Valid operator expressions
flag: true and false           // Bool
result: not true               // Bool
negated: -42                   // Number
complex: true and not false    // Bool
nested: - -100                 // Number

// Type errors (detected during unification)
invalid1: 42 and true          // Error: Type mismatch (Number vs Bool)
invalid2: not "hello"          // Error: Type mismatch (String vs Bool)
invalid3: -true                // Error: Type mismatch (Bool vs Number)
invalid4: "text" or false      // Error: Type mismatch (String vs Bool)
```

### Design Decisions
- **Constraint-based approach**: Integrates naturally with Hindley-Milner system
- **Universal Number type**: Defers specific numeric types (Int8-64, etc.)
- **Separate module**: Keeps `type_inference.rs` focused on literals
- **Type variable support**: Operators work with inferred types through unification

---

## [0.24.0] - 2026-01-13 - Hindley-Milner Type Inference Foundation

### Added
- **Hindley-Milner type inference foundation**
  - Type variables (`Type::Var(TypeVarId)`) for representing unknowns during inference
  - Constraint system for collecting type equality constraints
  - Unification algorithm with occurs check to solve constraints
  - Substitution mechanism for mapping type variables to concrete types
  - Literal type inference:
    - Number literals → `Number` type
    - String literals → `String` type
    - Boolean literals → `Bool` type
    - Empty lists → `Array('a)` where 'a is a fresh type variable
  - Three-phase analysis: constraint collection, unification, substitution application

### Technical Details
- **New type system components** in `src/semantic/types.rs`:
  - `TypeVarId` struct for unique type variable identifiers
  - `Type::Var(TypeVarId)` variant for inference type variables
  - `Constraint` struct representing type equality constraints
  - `Substitution` struct for storing type variable bindings
- **New module**: `src/semantic/unification.rs` 
  - `unify()` - Core unification algorithm handling all type forms
  - `occurs_check()` - Prevents infinite types like `'a = Array('a)`
  - Support for primitives, arrays, functions, options, results, unions, intersections
- **New module**: `src/semantic/type_inference.rs` 
  - `visit_list()` - Creates `Array('a)` with fresh type variable
  - `solve_constraints()` - Unifies all collected constraints
  - `apply_substitution()` - Replaces type variables with concrete types
- **Enhanced SemanticAnalyzer** in `src/semantic/mod.rs`:
  - Added `node_types: HashMap<usize, TypeId>` for tracking inferred types
  - Added `constraints: Vec<Constraint>` for constraint collection
  - Added `substitution: Substitution` for unification results
  - Added `next_type_var: u32` counter for generating fresh type variables
  - Helper methods: `fresh_type_var()`, `set_node_type()`, `get_node_type()`, `add_constraint()`
- **Updated analyze() method**: Now runs three-phase HM algorithm
  1. Constraint collection via AST traversal
  2. Constraint solving via unification
  3. Final substitution application to all nodes

### Algorithm Details
The implementation follows the classic Hindley-Milner algorithm:
1. **Type Variable Generation**: Assigns fresh type variables to unknowns
2. **Constraint Collection**: Walks AST generating equality constraints
3. **Unification**: Solves constraints via Robinson's unification algorithm
4. **Substitution**: Maps type variables to concrete types

Occurs check prevents infinite types, ensuring type soundness.


### Examples
```suru
// Literal type inference
x: 42          // Inferred: Number
s: "hello"     // Inferred: String
flag: true     // Inferred: Bool
xs: []         // Inferred: Array('a) where 'a is type variable

// Future
// nums: [1, 2, 3]        // Will infer: Array(Number)
// identity: (x) { x }    // Will infer: ∀a. (a) -> a
```

---

## [0.23.0] - 2026-01-13 - Type Declaration Processing

### Added
- **Type declaration processing**
  - Type aliases with transparent aliasing (`type UserId: Number`)
  - Unit types (`type Success`)
  - Union types (`type Status: Success, Error`)
  - Struct types with field validation (`type Person: { name String }`)
  - Intersection types with validation (`type Admin: Person + { role String }`)
  - Built-in types: Number, String, Bool, Int8-Int64, UInt8-UInt64, Float32-Float64
  - TypeRegistry integration for type interning
  - 37 new tests (408 total tests passing)

### Technical Details
- **New module**: `src/semantic/type_resolution.rs` with visitor methods for each type form
- **Enhanced SemanticAnalyzer**: Added `TypeRegistry` field and helper methods
- **Type validation**: All type references validated, no forward references allowed
- **Deferred features**: Generic types, function types and struct methods

### Examples
```suru
type UserId: Number              // Type alias
type Success                     // Unit type
type Result: Success, Error      // Union type
type User: { id UserId }         // Struct type
type Admin: User + { role String }  // Intersection type
```

---

## [0.22.0] - 2026-01-13 - Name Resolution

### Added
- **Name resolution for variables and functions**
  - Variable declaration resolution with redeclaration support
  - Variable reference resolution with scope chain lookup
  - Function declaration resolution with signature tracking
  - Function call resolution with kind validation
  - Context-aware identifier resolution (distinguishes declarations from references)
  - Support for variable shadowing across scopes
  - Recursive function support (function visible to its own body)
  - 19 comprehensive semantic analysis tests (329 total tests)

### Technical Details
- **New module**: `src/semantic/name_resolution.rs` (~520 lines)
  - `visit_var_decl()` - Registers variables in current scope
  - `visit_identifier()` - Resolves variable references in scope chain
  - `visit_function_decl()` - Registers functions with parameter handling
  - `visit_function_call()` - Validates function calls and resolves arguments
  - `build_function_signature()` - Constructs signature strings like `"(Number, String) -> Bool"`
- **Enhanced SymbolTable**: Added `insert_or_replace()` method for variable redeclaration support
- **Dispatcher updates**: Added `Identifier` and `FunctionCall` to `visit_node()` dispatcher
- Error messages: Simple and direct with precise location tracking
  - "Variable 'x' is not defined"
  - "Function 'foo' is not defined"
  - "Duplicate declaration of function 'bar'"
  - "'x' is not a function"

### Language Semantics
- **Variable redeclaration allowed**: The `:` operator acts as both declaration and reassignment
  ```suru
  x Number: 42
  x String: "hello"  // Valid - replaces previous declaration
  ```
- **Function redeclaration prohibited**: Duplicate function names produce errors
  ```suru
  foo: () { }
  foo: () { }  // Error: Duplicate declaration of function 'foo'
  ```
- **Scope chain resolution**: Variables and functions resolved from innermost to outermost scope
- **Variable shadowing**: Inner scopes can shadow outer scope variables
  ```suru
  x: 42
  foo: () {
      x String: "shadowed"  // Different variable, shadows outer x
  }
  ```

### Examples
```suru
// Variable declaration and reference
x Number: 42
y: x  // Valid reference

// Function declaration and call
add: (a Number, b Number) Number {
    result: a  // Parameters in scope
}
sum: add(5, 10)  // Valid call

// Recursive functions
factorial: (n Number) Number {
    result: factorial(n)  // Function visible to itself
}

// Nested scopes
outer: () {
    x: 1
    inner: () {
        y: x  // Outer variable visible
    }
}
```

## [0.21.0] - 2026-01-12 - Semantic Analyzer Foundation

### Added
- **Semantic analyzer skeleton** - Foundation for semantic analysis phase
  - `SemanticError` struct with message and location tracking
  - `SemanticAnalyzer` struct with AST traversal and error collection
  - `analyze()` entry point returning `Result<Ast, Vec<SemanticError>>`
  - Visitor pattern implementation for AST node traversal
  - Automatic scope management for blocks
  - Error collection (collects all errors, doesn't stop on first)
  - Integration with existing ScopeStack infrastructure
  - 3 integration tests (test_empty_program, test_analyzer_initialization, test_simple_program_with_declarations)

### Technical Details
- Implemented in `src/semantic/mod.rs` 
- Visitor methods: `visit_node()`, `visit_children()`, `visit_program()`, `visit_block()`
- Stub methods for future phases: `visit_var_decl()`, `visit_function_decl()`, `visit_type_decl()`
- First-child/next-sibling AST traversal pattern
- Scope entry/exit demonstrated in `visit_block()`
- Error pattern follows ParseError design (message + line + column)
- Implements `Display` and `Error` traits for SemanticError

## [0.20.0] - 2025-12-29 - Unary Negation Operator

### Added
- **Unary negation operator (`-`)** for all expressions
  - Precedence level 3 (same as `not`, `try`, `partial`)
  - Works with literals, identifiers, function/method calls, and complex expressions
  - Supports chaining: `--42`, `---value`
  - Integration with all operators: `not -value`, `-a and b`, `data | -getValue()`
  - New AST node type: `Negate`
  - 35 comprehensive tests (283 total)

### Examples
```suru
// Basic negation
x: -42
y: -getValue()
z: -obj.method()

// With operators
a: -x and y        // (-x) and y
b: not -value      // not (-value)
c: data | -process // pipe with negation

// In arguments
d: add(-5, 10)
```

## [0.19.0] - 2025-12-29 - Module System Parsing

### Added
- **Module declarations**: `module Calculator`, `module math.geometry`, `module .utils` (submodule)
- **Import statements** with three forms:
  - Full: `import { math }`
  - Aliased: `import { m: math }`
  - Selective: `import { {sin, cos}: math }`
  - Star: `import { *: math }`
- **Export statements**: `export { Calculator, add, subtract }`
- Dotted module paths: `math.geometry`, `math.trigonometry`
- Flexible separators: comma-separated or newline-separated lists
- New AST nodes: `ModuleDecl`, `ModulePath`, `Import`, `ImportList`, `ImportItem`, `ImportAlias`, `ImportSelective`, `ImportSelector`, `Export`, `ExportList`
- New parser module: `src/parser/module.rs` (~745 lines)
- 37 comprehensive tests (252 total)

### Examples
```suru
module Calculator

import {
    math
    m: trigonometry
    {sin, cos}: angles
    *: io
}

export {
    Calculator
    add
}
```

## [0.18.0] - 2025-12-29 - Composition Operator

### Added
- **Composition operator (`+`)** for data and method composition
  - Binary infix operator (precedence 1, same as `|` and `or`)
  - Left-associative: `a + b + c` → `(a + b) + c`
  - Works in all expression contexts
  - New AST node: `Compose`
  - 18 comprehensive tests (215 total)
- **Struct literals as expressions**
  - Struct literals `{...}` now usable in any expression context
  - Enables: `base + {extra: value}`

### Examples
```suru
// Data composition
user: {name: "Paul"} + {age: 30}
enhanced: person + contact + location

// With pipes
result: getData() | transform + enhance
```

## [0.17.0] - 2025-12-29 - Partial Keyword Support

### Added
- **`partial` keyword** for explicit partial application
  - Unary prefix operator (precedence 3, same as `not` and `try`)
  - Syntactic sugar to avoid writing many `_` placeholders
  - Usage: `partial functionWithManyArguments(arg1)` instead of `func(_, _, _, _, _, _, _, _, _)`
  - Works with function calls: `partial getValue()`
  - Works with method calls: `partial obj.method(arg)`
  - Composable in pipes: `data | partial filter(active)`
  - New AST node type: `Partial`
  - 5 essential tests (197 tests total)

### Technical Details
- Added `Partial` to `NodeType` enum in `src/ast.rs`
- Parsing logic in `src/parser/expressions.rs` (follows `try` operator pattern)
- Same precedence as other unary operators (3)
- Right-to-left associativity
- Accepts any expression as operand (semantic validation deferred)

### Examples
```suru
// Avoid many underscores
curry: partial functionWithManyArguments(2_283i32)

// With method calls
validator: partial user.validate()

// In pipelines
result: data | partial filter(active) | partial sort()

// Composition with other operators
checked: try partial getValue()
```

### Context
The `partial` keyword complements the existing `_` placeholder syntax. While `_` is ideal for functions with few parameters (`add(_, 5)`), `partial` provides cleaner syntax when many arguments would require many placeholders.

## [0.16.0] - 2025-12-29 - List Literals

### Added
- **List literal parsing** with square bracket syntax `[...]`
  - Empty lists, trailing commas, and nested lists
  - Any expression as elements: literals, identifiers, function/method calls
  - Method chaining on list literals: `[1, 2, 3].length()`
  - New AST node type: `List`
  - New parser module: `src/parser/list.rs` (~420 lines)
  - 19 comprehensive tests (192 tests total)

### Examples
```suru
// Basic lists
empty: []
numbers: [1, 2, 3]
mixed: [1, "text", true]

// Nested and method calls
nested: [[1, 2], [3, 4]]
length: [1, 2, 3].length()
computed: [getValue(), x | transform]
```

## [0.15.0] - 2025-12-28 - Struct Initialization, Type Annotations, and `this` Keyword

### Added
- **Struct initialization literals** for creating struct instances
  - Empty structs: `{}`
  - Field initialization: `{ name: "Paul", age: 30 }`
  - Method implementation: `{ greet: () { return "Hello!" } }`
  - Private members with `_` prefix: `{ _ secret: "password" }`
  - Privacy stored as bitflags, not in identifier names
  - New AST node types: `StructInit`, `StructInitField`, `StructInitMethod`
  - Separate nodes for fields vs methods (better semantic clarity)
- **Type annotations** for variable declarations
  - Works with any expression: `count Int16: 42`
  - Function call results: `name String: getName(person)`
  - Boolean expressions: `result Bool: x and y`
  - Struct literals: `user User: { name: "Paul" }`
  - Pattern: `identifier [Type] : expression`
  - Handled at statement level, not expression level
- **`this` keyword** for self-reference in methods
  - Separate node type (not Identifier) for better semantic analysis
  - Property access: `this.name`
  - Method calls: `this.getValue()`
  - Works in struct literal methods and type declarations
- **Privacy system** using bitflags
  - NodeFlags bitflags struct (extensible to 8 flags)
  - IS_PRIVATE flag for private members
  - Privacy constructors: `new_private()`, `new_private_terminal()`
  - Privacy markers in AST tree display: `[private]`
  - Only 1 byte overhead per node

### Technical Details
- Added bitflags dependency (v2.4) for metadata flags
- Extended AstNode with flags field (maintains uniform node size)
- Privacy flag set on both container and identifier nodes
- Type annotation lookahead in statement parser (handles optional types)
- Struct literals parsed in limited contexts (var decls only for now)
- Comma-separated or newline-separated struct members
- Modular implementation in `src/parser/struct_init.rs`
- 16 new tests (157 → 173 total)

### Examples
```suru
// Type annotations
count Int16: 42
name String: getName(person)
result Bool: x and y

// Simple struct
user: {
    name: "Paul"
    age: 30
}

// Struct with type annotation
user User: {
    username: "Paul"
}

// Struct with methods
user: {
    name: "Paul"
    greet: () {
        return `Hello, I'm {this.name}!`
    }
}

// Privacy and this keyword
user User: {
    username: "Paul"              // Public field
    _ passwordHash: "hash123"     // Private field

    authenticate: (password) {    // Public method
        return this.passwordHash.equals(password)
    }

    _ hashPassword: (pass) {      // Private method
        return pass
    }
}
```

### Design Decisions
- Privacy via bitflags (not name mangling) for clean AST and extensibility
- Type annotations as general feature (not struct-specific)
- Separate StructInitField and StructInitMethod node types
- `this` as separate node type (better for later semantic analysis)

## [0.14.0] - 2025-12-28 - Match Statements and Match Expressions

### Added
- **Match expression** parsing for pattern matching control flow
  - Pattern matching on types, values, and wildcards
  - Match on identifiers: `Success`, `Error`, `Pending`
  - Match on literals: numbers (`0`, `1`), strings (`"admin"`), booleans (`true`, `false`)
  - Wildcard pattern: `_` for catch-all cases
  - Nested matches: match expressions inside result expressions
  - Match as expression: works in variables, returns, and pipes
  - Complex subjects: function calls, method calls, property access, pipes
  - Complex results: function calls, method calls, boolean expressions, pipes
  - New AST node types: `Match`, `MatchSubject`, `MatchArms`, `MatchArm`, `MatchPattern`
  - 28 comprehensive tests covering all patterns and edge cases
- **Match statement** support for pattern matching as standalone control flow
  - Works anywhere statements are allowed: program root, function bodies, blocks
  - Wrapped in `ExprStmt` for statement context

### Technical Details
- Match is a primary expression (like literals)
- Recursive parsing with depth tracking (default limit: 256)
- First-child/next-sibling AST structure
- Requires at least one arm
- Pattern wrapper nodes for type safety
- Implemented in separate module: `src/parser/match.rs`

### Examples
```suru
// Match on types
status: match result {
    Success: "ok"
    Error: "fail"
    _: "unknown"
}

// Match on values
message: match n {
    0: "zero"
    1: "one"
    _: "other number"
}

// Nested match
result: match outer {
    Ok: match inner {
        TypeA: "A"
        _: "other"
    }
    Error: "error"
}

// Match in return
check: () {
    return match x {
        0: "zero"
        _: "other"
    }
}

// Match as statement
match status {
    Success: print("success")
    Error: exit()
}

// In function
handleResult: (result Result) {
    match result {
        Ok: processSuccess()
        Error: logError()
    }
}

// Mixed with other statements
x: 42
match x {
    0: print("zero")
    _: print("other")
}
print("done")
```

## [0.13.0] - 2025-12-28 - Placeholder & Try Operator

### Added
- **Try operator** (`try`) for error handling expressions
  - Unary prefix operator with precedence 3 (same as `not`)
  - Works with expressions, function/method calls, pipes
  - Chaining: `try try getValue()`, `input | try parse | try validate`
  - New AST node: `Try`
  - 17 tests

- **Placeholder** (`_`) for partial application
  - Terminal expression for function/method arguments
  - Multiple placeholders: `func(_, 42, _)`
  - In pipes: `100 | multiply(_, 2) | add(_, 50)`
  - New AST node: `Placeholder`
  - 12 tests

### Examples
```suru
// Try operator
result: try parseNumber(input)
safe: input | try parse | try validate

// Placeholder
result: add(_, 5)
chain: data | filter(_, active) | map(_, transform)
```

## [0.12.0] - 2025-12-28 - Pipe Expressions

### Added
- Pipe operator parsing (`|`) for functional composition
- Basic piping: `value | transform`
- Pipe chaining with left-associativity: `a | b | c` → `((a | b) | c)`
- Pipes with function calls: `data | filter(active) | sort()`
- Pipes with method calls: `obj.method() | func`
- Complex pipeline chains: `data | filter(active) | sort() | take(10)`
- New AST node type: `Pipe`
- 17 comprehensive tests for pipe operator

### Technical Details
- Pipe has precedence level 1 (same as `or`)
- Lower precedence than `and` (2), `not` (3), and `.` (4)
- Left-associative for natural chaining
- Parser creates AST nodes only; semantic transformation deferred to later phases
- Placeholder (`_`) support intentionally deferred to future release

### Examples
```suru
// Basic pipe
result: value | transform

// Chaining
processed: data | filter(active) | sort() | take(10)

// With methods
output: obj.process() | validate() | format()
```

## [0.11.0] - 2025-12-27 - Method Calls & Property Access

### Added
- Method call parsing with dot notation (`person.greet()`)
- Property access parsing (`person.name`)
- Method chaining support (`numbers.add(6).add(7).set(0, 0)`)
- Works on any expression including literals (`"hello".toUpper()`, `42.toString()`)
- New AST node types: `MethodCall`, `PropertyAccess`, `ArgList`
- 14 comprehensive tests for method calls and property access

### Technical Details
- Dot operator has highest precedence (4)
- Separate AST nodes for method calls vs property access
- Postfix loop enables chaining

## [0.10.0] - 2025-12-23 - Parser Module Refactoring

### Changed
- Refactored monolithic 3,427-line parser.rs into modular structure
- Split into 6 organized modules by parsing domain
- Zero breaking changes - public API remains identical

### Added
- `parser/mod.rs` - Parser struct, public API, module coordination
- `parser/error.rs` - ParseError type with Display/Error implementations
- `parser/helpers.rs` - Operator precedence, token navigation utilities
- `parser/expressions.rs` - Expression parsing (~60 tests)
- `parser/types.rs` - Type declaration parsing (~70 tests)
- `parser/statements.rs` - Statement parsing (~60 tests)

### Improved
- Better code organization and maintainability
- Easier to understand and extend

## [0.9.0] - 2025-12-19 - Type Declarations Complete

### Added
- Complete type system declarations (all 7 forms)
- Unit types (`type Success`)
- Type aliases (`type UserId: Number`)
- Union types (`type Status: Success, Error, Loading`)
- Function types (`type AddFunction: (a Number, b Number) Number`)
- Struct types with fields and methods
- Intersection types with `+` operator
- Generic types with constraints (`type List<T>`, `type Comparable<T: Orderable>`)
- 51 tests covering all type declaration forms

### Technical Details
- Unified TypeDecl node for all forms
- TypeBody abstraction separates name/generics from definition
- Support for generic constraints (`<T: Constraint>`)
- Intersection composition using `+` operator

## [0.8.0] - 2025-12-19 - Function Parameters & Return Types

### Added
- Function parameter parsing with optional types
- Return type annotations for functions
- Return statement parsing (with/without expressions)
- Support for typed parameters (`x Number`)
- Support for inferred parameters (`value`)
- Mixed parameter types in same function
- 18 comprehensive tests

### Examples
```suru
add: (x Number, y Number) Number {
    return x
}

identity: (value) {
    return value
}
```

## [0.7.0] - 2025-12-18 - Functions & Blocks

### Added
- Function declaration parsing with parameter lists
- Block support for statement grouping (`{ ... }`)
- Standalone function calls as statements
- 2-token lookahead for disambiguation
- New AST nodes: `FunctionDecl`, `ParamList`, `Block`, `ExprStmt`
- 22 comprehensive tests

### Examples
```suru
main: () {
    print("Hello, world")
}

nested: () {
    inner: () {
        x: 1
    }
}
```

## [0.6.0] - 2025-12-18 - Function Calls & CLI

### Added
- Complete CLI interface using clap
- Function call parsing in expressions
- Zero-argument, single-argument, and multi-argument calls
- Function calls with boolean operators
- `suru parse <file>` command
- Enhanced AST with tree printing methods

### Changed
- Improved compiler limits defaults
- Better error messages

## [0.5.0] - 2025-12-17 - Pure Recursive Descent Parser

### Changed
- Converted from hybrid stack-based/recursive to pure recursive descent parser
- Removed state machine and explicit stack completely
- Unified depth tracking across all parsing
- Simplified codebase by ~80 lines

### Improved
- Simpler `Parser` struct with no state tracking
- Depth passed as parameter to each function
- Better code readability and maintainability

## [0.4.0] - 2025-12-14 - Stack-Based Parser

### Added
- Complete stack-based parser with no recursion
- First-child/next-sibling AST representation
- All nodes stored in single vector for cache efficiency
- State machine with explicit stack
- AST nodes: `Program`, `VarDecl`, `Identifier`, `LiteralBoolean`, `LiteralNumber`, `LiteralString`

### Technical Details
- Uniform node size for cache locality
- Index-based references (no lifetimes, serialization-friendly)
- Fail-fast error handling with precise position information

## [0.3.0] - 2025-12-12 - Lexer Implementation

### Added
- Complete lexer implementation for Suru language
- 14 keywords: module, import, export, return, match, type, try, and, or, not, true, false, this, partial
- Number literals with multiple bases (binary, octal, hex, decimal, float)
- Type suffixes for numbers (i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, f16, f32, f64, f128)
- String literals (standard `"..."` / `'...'`, interpolated `` `...` ``)
- Operators and punctuation
- Zero-copy design using byte offsets

### Technical Details
- Comprehensive error reporting with line/column info
- Project restructured into modular architecture
- `src/lexer.rs` - Complete lexer implementation
- `src/codegen.rs` - Extracted LLVM code generation
- `src/main.rs` - Minimal entry point

## [0.2.0] - 2025-12-12 - Hello World LLVM IR

### Added
- LLVM IR generation using Inkwell
- Complete compilation pipeline (IR → object file → executable)
- External function declaration (printf)
- Global string constant creation
- Module verification
- Native code generation via LLVM target machine
- Linking with clang-18

### Dependencies Added
- inkwell = { version = "0.6.0", features = ["llvm18-1"] }
- clap = { version = "4.5", features = ["derive"] }
- toml = "0.8"
- serde = { version = "1.0", features = ["derive"] }

## [0.1.0] - 2025-12-12 - Development Environment

### Added
- Docker development environment with Ubuntu 24.04 LTS
- Rust stable toolchain with edition 2024 support
- LLVM 18 with full development libraries
- All build tools properly configured
- Basic Rust project structure with Cargo.toml

---

For detailed development log, see [dev/progress.md](dev/progress.md).
