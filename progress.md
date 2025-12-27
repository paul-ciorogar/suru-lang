# Suru Lang Development Progress

## Project Overview
Building a programming language compiler using Rust and LLVM 18.

## Completed Milestones

### Milestone 1: Development Environment Setup
- **Date**: 2025-12-12
- **Details**:
  - Docker development environment configured with Ubuntu 24.04 LTS
  - Rust stable toolchain with edition 2024 support
  - LLVM 18 with full development libraries
  - All build tools properly configured

### Milestone 2: Hello World LLVM IR Generation
- **Date**: 2025-12-12
- **Details**:
  - Successfully implemented LLVM IR generation using Inkwell
  - Created a complete compilation pipeline:
    1. LLVM IR generation
    2. Object file compilation
    3. Executable linking
    4. Program execution

#### Implementation Highlights
- **File**: `src/main.rs`
- **Key Components**:
  - LLVM context and module creation
  - External function declaration (printf)
  - Global string constant creation
  - Function definition (main)
  - Basic block and instruction generation
  - Module verification
  - Native code generation via LLVM target machine
  - Linking with clang-18

#### Generated LLVM IR
```llvm
; ModuleID = 'hello_world'
source_filename = "hello_world"

@.str = private unnamed_addr constant [15 x i8] c"Hello, world!\0A\00", align 1

declare i32 @printf(ptr, ...)

define i32 @main() {
entry:
  %printf_call = call i32 (ptr, ...) @printf(ptr @.str)
  ret i32 0
}
```

#### Output
```
Hello, world!
```

### Dependencies
```toml
[dependencies]
clap = { version = "4.5", features = ["derive"] }
inkwell = { version = "0.6.0", features = ["llvm18-1"] }
toml = "0.8"
serde = { version = "1.0", features = ["derive"] }
```

## Build and Run

### Build the project
```bash
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo build
```

### Run tests
```bash
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo test
```

### Parse a Suru file
```bash
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo run -- parse test.suru
```

### Show CLI help
```bash
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo run -- --help
```

### Milestone 3: Lexer Implementation
- **Date**: 2025-12-12
- **Details**:
  - Complete lexer implementation for the Suru language
  - Project restructured into modular architecture

#### Implementation Highlights
- **Files**:
  - `src/lexer.rs` - Complete lexer implementation
  - `src/codegen.rs` - Extracted LLVM code generation
  - `src/main.rs` - Minimal entry point

#### Token Types
- **14 Keywords** (lowercase): module, import, export, return, match, type, try, and, or, not, true, false, this, partial
- **Numbers**: Binary (0b), Octal (0o), Hex (0x), Decimal, Float with type suffixes (i8, u32, f64, etc.)
- **Strings**: Standard ("..." or '...') with escape sequences, Interpolated (`...`)
- **Operators**: : ; , . | * + - ( ) { } [ ] < >
- **Special**: Underscore (_), Newline, EOF

#### Example Usage
```rust
use lexer::lex;

let source = "module main return 42";
let tokens = lex(source)?;

// Tokens:
// [Module, Identifier("main"), Return, Number(Decimal, "42"), Eof]
```

### Milestone 4: Stack-Based Parser Implementation
- **Date**: 2025-12-14
- **Details**:
  - Complete stack-based parser with no recursion
  - First-child/next-sibling AST representation
  - All nodes stored in single vector for cache efficiency

#### Implementation Highlights
- **Files**:
  - `src/ast.rs` - AST data structures with uniform-size nodes
  - `src/parser.rs` - Stack-based parser with state machine
  - `src/main.rs` - Updated with parser integration and demo

#### Architecture
- **Parser Style**: State machine with explicit stack (no recursion)
- **AST Storage**: Single `Vec<AstNode>` with first-child/next-sibling tree
- **Tree Structure**: Indices instead of pointers (cache-friendly)
- **Error Handling**: Fail-fast with precise position information

#### Parser States
1. `ExpectStatement` - Waiting for identifier or EOF
2. `ExpectColonAfterIdentifier` - Expecting `:` after variable name
3. `ExpectValue` - Expecting literal value (boolean, number, string)
4. `ExpectStatementEnd` - Expecting newline or EOF

#### Supported Syntax
Currently parses simple variable declarations:
```suru
x: 42
name: "Alice"
flag: true
count: 0xFF
pi: 3.14159
```

#### AST Structure Example
For `x: 42`:
```
Program (nodes[0])
└─ VarDecl (nodes[1])
   ├─ Identifier "x" (nodes[2])
   └─ LiteralNumber "42" (nodes[3])
```

#### Node Types
- `Program` - Root node containing all declarations
- `VarDecl` - Variable declaration
- `Identifier` - Identifier (terminal)
- `LiteralBoolean` - Boolean literal (terminal)
- `LiteralNumber` - Number literal (terminal)
- `LiteralString` - String literal (terminal)

### Milestone 5: Full Recursive Descent Parser Refactoring
- **Date**: 2025-12-17
- **Details**:
  - Converted from hybrid stack-based/recursive to pure recursive descent parser
  - Removed state machine and explicit stack completely
  - Unified depth tracking across all parsing
  - Simplified codebase by ~80 lines
- All parsing is recursive (statements and expressions)
- Depth passed as parameter to each function
- No state machine or stack
- Simpler `Parser` struct with no state tracking


#### Key Design Decisions
- **Depth as parameter**: Passed to each recursive function, not stored in struct
- **Depth increment pattern**: Always call with `depth + 1` when recursing
- **Unified checking**: Same `check_depth()` used for all recursion
- **Direct grammar mapping**: Each production rule is a method

#### Updated Method Signatures
```rust
// Before
fn parse_expression(&mut self, min_precedence: u8) -> Result<usize, ParseError>

// After
fn parse_expression(&mut self, depth: usize, min_precedence: u8) -> Result<usize, ParseError>
fn parse_var_decl(&mut self, depth: usize) -> Result<usize, ParseError>
fn parse_statement(&mut self, depth: usize) -> Result<Option<usize>, ParseError>
```

### Milestone 6: CLI Infrastructure & Function Call Support
- **Date**: 2025-12-18
- **Details**:
  - Added complete CLI interface using clap
  - Implemented function call parsing
  - Enhanced AST with tree printing methods
  - Improved compiler limits defaults

#### CLI Implementation
- **New Module**: `src/cli.rs` - Command-line interface definitions
- **Dependency**: Added clap 4.5 with derive features
- **Commands**:
  - `suru parse <file>` - Parse a Suru source file and print AST
  - Future: lex, compile, run, lsp commands planned
- **Error Handling**: Proper exit codes and user-friendly error messages

#### Function Call Parsing
- **New AST Node**: `FunctionCall` - Represents function invocations
- **Syntax Support**:
  - Zero arguments: `print()`
  - Single argument: `print(42)`
  - Multiple arguments: `add(1, 2, 3)`
  - Mixed types: `add(42, x, "test", true)`
  - Function calls in expressions: `not print()`, `f() and g()`
- **Design Decisions**:
  - Nested function calls are **explicitly disallowed** (error: "Nested function calls are not supported") for the moment
  - Identifiers can be used standalone or as function names
  - Arguments are comma-separated with optional trailing comma support

#### Example Usage
```bash
# Parse a Suru file and display AST
suru parse test.suru
```

**Input** (`test.suru`):
```suru
x: add(1, 2, 3)
y: print("hello")
z: not test(true, false)
```

**Output**:
```
Program
  VarDecl
    Identifier "x"
    FunctionCall
      Identifier "add"
      LiteralNumber "1"
      LiteralNumber "2"
      LiteralNumber "3"
  VarDecl
    Identifier "y"
    FunctionCall
      Identifier "print"
      LiteralString ""hello""
  VarDecl
    Identifier "z"
    Not
      FunctionCall
        Identifier "test"
        LiteralBoolean "true"
        LiteralBoolean "false"
```

### Milestone 7: Function Declaration Parsing & Block Support
- **Date**: 2025-12-18
- **Details**:
  - Implemented function declaration parsing with empty parameter lists
  - Added block support for statement grouping
  - Implemented standalone function calls as statements
  - Enhanced parser with 2-token lookahead for disambiguation
  - 22 comprehensive tests added (78 total tests passing)

#### Implementation Highlights
- **New AST Node Types** (src/ast.rs):
  - `FunctionDecl` - Function declaration node
  - `ParamList` - Parameter list container (empty for now)
  - `Block` - Statement block `{ ... }`
  - `ExprStmt` - Expression statement wrapper (for standalone calls)

- **New Parser Methods** (src/parser.rs):
  - `parse_function_decl()` - Parses complete function declarations
  - `parse_block()` - Parses statement blocks with `{ }`
  - `parse_param_list()` - Parses empty parameter lists `()`
  - Updated `parse_statement()` - Lookahead logic to distinguish functions vs variables

#### Syntax Support
Function declarations with empty parameters:
```suru
main: () {
    print("Hello, world")
}

add: () {
    x: 42
    y: 100
    sum(x, y)
}
```

Standalone function calls (not just in expressions):
```suru
print("test")
initialize()
cleanup()
```

Nested functions:
```suru
outer: () {
    inner: () {
        x: 1
    }
}
```

#### AST Structure Example
For `main: () { print("Hello, world") }`:
```
Program
  FunctionDecl
    Identifier "main"
    ParamList
    Block
      ExprStmt
        FunctionCall
          Identifier "print"
          LiteralString ""Hello, world""
```

#### Key Design Decisions
- **2-token lookahead**: Distinguish `ident : ()` (function) from `ident : expr` (variable) from `ident()` (standalone call)
- **ExprStmt wrapper**: Separates expression-as-statement from expression-as-value contexts
- **Empty ParamList node**: Consistent tree structure (always 3 children for FunctionDecl), easy to extend later
- **RBrace handling**: `parse_statement()` returns `Ok(None)` when seeing `}` to signal end of block
- **Newline skipping**: Allows flexible formatting with newlines after `:`, `()`, and `{`

#### Test Coverage
Added 22 comprehensive tests:
- **Basic functions** (4 tests): Empty functions, functions with calls, multiple statements, multiple functions
- **Standalone calls** (3 tests): Simple calls, multiple calls, no-arg calls
- **Error cases** (6 tests): Missing parens/braces, unclosed blocks, invalid syntax
- **Tree structure** (3 tests): Validate exact FCNS tree layout, sibling chaining
- **Edge cases** (6 tests): Formatting variations, nested blocks, depth limits, mixed declarations

#### Example Usage
```bash
# Parse a function declaration
echo 'main: () {
    print("Hello, world")
}' > test.suru

docker run --rm -v $(pwd):/workspace suru-lang:dev cargo run -- parse test.suru
```

**Output**:
```
Program
  FunctionDecl
    Identifier "main"
    ParamList
    Block
      ExprStmt
        FunctionCall
          Identifier "print"
          LiteralString ""Hello, world""
```

#### Future Extensions (Designed For)
The implementation is structured to easily support:
- Generic functions: `<T>` type parameters
- Method calls as statements: `obj.method()`

### Milestone 8: Function Parameters, Return Types & Return Statements
- **Date**: 2025-12-19
- **Details**:
  - Implemented complete function parameter parsing with optional types
  - Added return type annotations for functions
  - Implemented return statement parsing
  - 18 comprehensive tests added (101 total tests passing)

#### Syntax Support

**Function Parameters:**
```suru
// Typed parameters
add: (x Number, y Number) {}

// Inferred parameters (no types)
identity: (value) {}

// Mixed types
process: (id String, count) {}

// Multiple parameters
combine: (a, b, c) {}
```

**Return Type Annotations:**
```suru
// Return type specified
add: (x Number, y Number) Number {}

// No return type (inferred)
process: (data String) {}

// Custom types
getUser: (id String) User {}
```

**Return Statements:**
```suru
// Empty return
f: () {
    return
}

// Return literal
getValue: () Number {
    return 42
}

// Return expression
add: (x Number, y Number) Number {
    return x
}

// Return function call
process: () {
    return getData()
}

// Return boolean expression
check: () Bool {
    return true and false
}
```

#### Complete Function Example
```suru
add: (x Number, y Number) Number {
    result: 42
    return result
}

main: () {
    sum: add(5, 10)
    print(sum)
}
```

#### AST Structure Example
For `add: (x Number, y Number) Number { return x }`:
```
Program
  FunctionDecl
    Identifier "add"
    ParamList
      Param
        Identifier "x"
        TypeAnnotation "Number"
      Param
        Identifier "y"
        TypeAnnotation "Number"
    TypeAnnotation "Number"
    Block
      ReturnStmt
        Identifier "x"
```

#### Key Design Decisions
- **Type annotations are optional**: Parameters and return types can be omitted for type inference
- **TypeAnnotation as terminal node**: Points to token for type name (Number, String, custom types)
- **Param node structure**: Always has Identifier child, optionally has TypeAnnotation child
- **Return stmt flexibility**: Works with or without expression (void returns)
- **Trailing comma support**: Parameters allow trailing commas like function call arguments

## Project Structure
```
suru-lang/
├── Cargo.toml          # Rust project manifest (inkwell, clap, toml, serde)
├── Cargo.lock          # Dependency lock file
├── Dockerfile          # Development environment (Ubuntu 24.04 + Rust + LLVM 18)
├── .dockerignore       # Docker build exclusions
├── README.md           # Language specification and documentation
├── CLAUDE.md           # Project context and architecture guide
├── progress.md         # This file - development progress log
├── todo.md             # Task tracking for upcoming features
└── src/
    ├── main.rs         # CLI entry point with command routing
    ├── cli.rs          # Command-line interface using clap
    ├── lexer.rs        # Lexer implementation (zero-copy tokenization)
    ├── parser/         # Modular parser structure (refactored from single file)
    │   ├── mod.rs      # Parser struct, public API, module coordination
    │   ├── error.rs    # ParseError type with Display/Error impls
    │   ├── helpers.rs  # Utilities and operator precedence
    │   ├── expressions.rs  # Expression parsing + tests
    │   ├── types.rs    # Type declaration parsing + tests
    │   └── statements.rs   # Statement parsing + tests
    ├── ast.rs          # AST data structures with tree utilities
    ├── limits.rs       # Compiler safety limits (TOML configuration)
    └── codegen.rs      # LLVM code generation module (skeleton)
```

### Milestone 9: Type Declarations - All 7 Forms Implemented
- **Date**: 2025-12-19
- **Details**:
  - Implemented complete type system declarations (7 distinct forms)
  - Added support for unit types, type aliases, union types, function types, struct types, intersection types, and generic types
  - Type system foundation ready for semantic analysis phase

#### Syntax Support

**1. Unit Types (Simple Flags/States):**
```suru
type Success
type Error
type Loading
```

**2. Type Aliases:**
```suru
type UserId: Number
type Username: String
type Age: Int64
```

**3. Union Types (Alternatives):**
```suru
type Status: Success, Error, Loading
type Value: Int64, String, Bool
type Result: Ok, Error
```

**4. Function Types:**
```suru
type AddFunction: (a Number, b Number) Number
type Predicate: (value String) Bool
type VoidCallback: () void
type NoParamsReturnsInt: () Int64
```

**5. Struct Types (Records with Fields and Methods):**
```suru
type Person: {
    name String
    age Number
    greet: () String
}

type Calculator: {
    add: (x Number, y Number) Number
    subtract: (x Number, y Number) Number
}
```

**6. Intersection Types (Type Composition):**
```suru
type Employee: Person + {
    salary Int64
    department String
}

type Manager: Person + Employee + {
    level Number
}
```

**7. Generic Types:**
```suru
type List<T>: {
    items Array
    size Number
}

type Map<K, V>: {
    entries Array
}

type Comparable<T: Orderable>: {
    value T
    compare: (other T) Number
}

type Result<T, E>: Ok, Error
```

#### AST Structure Example
For `type Person: { name String, age Number }`:
```
Program
  TypeDecl
    TypeName
      Identifier "Person"
    TypeBody
      StructBody
        StructField
          Identifier "name"
          TypeAnnotation "String"
        StructField
          Identifier "age"
          TypeAnnotation "Number"
```

For `type Result<T, E>: Ok, Error`:
```
Program
  TypeDecl
    TypeName
      Identifier "Result"
      TypeParams
        TypeParam
          Identifier "T"
        TypeParam
          Identifier "E"
    TypeBody
      UnionTypeList
        TypeAnnotation "Ok"
        TypeAnnotation "Error"
```

#### New AST Node Types
- **Type Declaration Nodes** (src/ast.rs):
  - `TypeDecl` - Type declaration root
  - `TypeName` - Type name with optional generic parameters
  - `TypeBody` - Type body container
  - `TypeAnnotation` - Type reference (terminal)
  - `TypeParams` - Generic parameter list
  - `TypeParam` - Single generic parameter with optional constraint
  - `TypeConstraint` - Generic constraint (terminal)
  - `UnionTypeList` - Union type alternatives
  - `StructBody` - Struct definition container
  - `StructField` - Struct field (name + type)
  - `StructMethod` - Struct method declaration
  - `IntersectionType` - Type intersection with `+`
  - `FunctionType` - Function type signature
  - `FunctionTypeParams` - Function type parameter list

#### Key Design Decisions
- **7 distinct forms**: Each type declaration form has specific syntax and semantics
- **Unified TypeDecl node**: All forms start with `type` keyword and use same root node
- **TypeBody abstraction**: Separates type name/generics from type definition
- **Struct duality**: Structs can have both fields (data) and methods (behavior)
- **Generic constraints**: Support `<T: Constraint>` syntax for bounded type parameters
- **Intersection with +**: Compose types using `+` operator
- **Function type syntax**: Reuses parameter list parsing for consistency
- **No semicolons in struct**: Fields and methods separated by newlines (Suru style)

#### Example Usage
```bash
# Parse type declarations
echo 'type Person: {
    name String
    age Number
    greet: () String
}' > types.suru

docker run --rm -v $(pwd):/workspace suru-lang:dev cargo run -- parse types.suru
```

**Output**:
```
Program
  TypeDecl
    TypeName
      Identifier "Person"
    TypeBody
      StructBody
        StructField
          Identifier "name"
          TypeAnnotation "String"
        StructField
          Identifier "age"
          TypeAnnotation "Number"
        StructMethod
          Identifier "greet"
          FunctionType
            FunctionTypeParams
            TypeAnnotation "String"
```

#### Future Extensions (Ready For)
The implementation sets the foundation for:
- **Semantic analysis**: Type checking and validation
- **Generic type resolution**: Instantiation of generic types
- **Structural typing**: Duck typing based on struct shapes
- **Type inference**: Inferring types from usage patterns

### Milestone 10: Parser Module Refactoring
- **Date**: 2025-12-23
- **Details**:
  - Refactored monolithic 3,427-line parser.rs into modular structure
  - Split into 6 organized modules by parsing domain
  - Zero breaking changes - public API remains identical
  - Improved maintainability and code organization

#### Refactoring Overview
The large `src/parser.rs` file was split into a clean module hierarchy:
```
src/parser/
├── mod.rs             - Parser struct, public API, module coordination
├── error.rs           - ParseError type with Display/Error impls
├── helpers.rs         - Operator precedence, token navigation utilities
├── expressions.rs     - Expression parsing + tests
├── types.rs           - Type declaration parsing + tests
└── statements.rs      - Statement parsing + tests
```

#### Module Responsibilities
- **mod.rs**: Central coordinator, exports public API (`parse()`, `Parser`, `ParseError`)
- **error.rs**: Independent error type, no dependencies on other parser modules
- **helpers.rs**: Shared utilities - `get_precedence()`, `check_depth()`, `skip_newlines()`, etc.
- **expressions.rs**: Boolean operators, function calls, precedence climbing, ~60 tests
- **types.rs**: All 7 type forms (unit, alias, union, struct, intersection, function, generic), ~70 tests
- **statements.rs**: Variables, functions, returns, blocks, parameters, ~60 tests

## Current Parser Capabilities
The parser currently supports:
```suru
# ========== Type Declarations ==========

# Unit types
type Success
type Error

# Type aliases
type UserId: Number
type Username: String

# Union types
type Status: Success, Error, Loading
type Value: Int64, String, Bool

# Function types
type AddFunction: (a Number, b Number) Number
type Predicate: (value String) Bool

# Struct types
type Person: {
    name String
    age Number
    greet: () String
}

# Intersection types
type Employee: Person + {
    salary Int64
}

# Generic types
type List<T>: {
    items Array
}

type Result<T, E>: Ok, Error

# ========== Variable Declarations ==========

x: 42
name: "Alice"
flag: true and false

# ========== Function Declarations ==========

# Function declarations with parameters and return types
add: (x Number, y Number) Number {
    return x
}

# Functions with inferred parameters
identity: (value) {
    return value
}

# Standalone function calls
initialize()
print("test")

# Function calls in expressions
x: add(1, 2, 3)
y: not test(true, false)

# Return statements
getValue: () Number {
    result: 42
    return result
}

# Nested functions with parameters
outer: (x Number) {
    inner: (y String) Number {
        return 1
    }
}
```

## Notes
- All development is done inside Docker container to ensure consistent LLVM environment
- LLVM 18 is explicitly used for latest features and stability
- Inkwell provides safe Rust bindings to LLVM C API
- Lexer follows Rust best practices with zero-copy design and comprehensive error handling
- Parser uses pure recursive descent approach with 2-token lookahead for disambiguation
- Recursion depth is configurable for safety and testing (default: 256)
- AST uses first-child/next-sibling representation for memory efficiency
