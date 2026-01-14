# Changelog

All notable changes to Suru Lang will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.26.0] - 2026-01-14 - Variable Declaration Type Checking

### Added
- **Variable declaration type checking** - Complete Phase 4.3 semantic analysis
  - Type annotation validation: Verifies declared types exist (e.g., `x Number: 42`)
  - Initializer type checking: Ensures initializer matches declared type
  - Type inference without annotation: Infers type from initializer (e.g., `x: 42` → Number)
  - Constraint generation for type checking:
    - With annotation: `init_type = declared_type`
    - Without annotation: variable gets inferred type
  - Support for all built-in types: Number, String, Bool, Int8-64, UInt8-64, Float32-64
  - Error reporting for undefined types and type mismatches
  - 15 comprehensive variable type checking tests (457 total tests passing)

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

### Next Steps
Phase 4.4 (Assignment Type Checking) will implement:
- Type checking for assignment statements
- Ensuring assigned values match variable types

---

## [0.25.0] - 2026-01-14 - Expression Type Checking

### Added
- **Operator type checking** - Complete Phase 4.2 semantic analysis
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
    - Uses universal `Number` type (consistent with Phase 4.1a)
    - Type errors for non-numeric operands
  - Full integration with Hindley-Milner constraint system
  - 15 comprehensive operator type checking tests (442 total tests passing)

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
- **Universal Number type**: Defers specific numeric types (Int8-64, etc.) to Phase 4.1c
- **Separate module**: Keeps `type_inference.rs` focused on literals
- **Type variable support**: Operators work with inferred types through unification

### Next Steps
Phase 4.3 (Variable Declaration Type Checking) will implement:
- Type annotation validation
- Initializer expression type checking
- Type inference from initializers

---

## [0.24.0] - 2026-01-13 - Hindley-Milner Type Inference Foundation

### Added
- **Hindley-Milner type inference foundation** - Complete Phase 4.1a semantic analysis
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

### Future Phases
This foundation enables:
- **Phase 4.1b**: Binary/unary operators, non-empty lists, variable references
- **Phase 4.1c**: Function inference, generalization, let-polymorphism

### Examples
```suru
// Literal type inference
x: 42          // Inferred: Number
s: "hello"     // Inferred: String
flag: true     // Inferred: Bool
xs: []         // Inferred: Array('a) where 'a is type variable

// Future (Phase 4.1b+):
// nums: [1, 2, 3]        // Will infer: Array(Number)
// identity: (x) { x }    // Will infer: ∀a. (a) -> a
```

---

## [0.23.0] - 2026-01-13 - Type Declaration Processing

### Added
- **Type declaration processing** - Complete Phase 3.2 semantic analysis
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
- **Deferred features**: Generic types (Phase 8), function types and struct methods (Phase 5)

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
- **Name resolution for variables and functions** - Complete Phase 2 semantic analysis
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

### Next Steps
Phase 3 (Type System Foundation) will implement:
- Internal type representation (3.1)
- Type declaration processing (3.2)
- Built-in types registration (3.3)

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
- Implemented in `src/semantic/mod.rs` (builds on phases 1.1 and 1.2)
- Visitor methods: `visit_node()`, `visit_children()`, `visit_program()`, `visit_block()`
- Stub methods for future phases: `visit_var_decl()`, `visit_function_decl()`, `visit_type_decl()`
- First-child/next-sibling AST traversal pattern
- Scope entry/exit demonstrated in `visit_block()`
- Error pattern follows ParseError design (message + line + column)
- Implements `Display` and `Error` traits for SemanticError

### Next Steps
Phase 2 (Name Resolution) will implement:
- Variable declaration resolution (2.1)
- Variable reference resolution (2.2)
- Function declaration resolution (2.3)
- Function call resolution (2.4)

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
