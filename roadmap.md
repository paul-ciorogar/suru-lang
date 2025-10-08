# Suru Language Compiler Roadmap

## Phase 1: Foundation (v0.1.0 - v0.3.0)
**Goal**: Core language features working end-to-end

### v0.1.0 - Minimal Viable Compiler
- **Lexer**: Tokenize basic syntax
  - Identifiers, keywords, operators
  - Numbers (decimal, binary `0b`, octal `0o`, hex `0x`)
  - Strings (single, double quotes, basic backtick interpolation)
  - Booleans (`true`, `false`)
- **Parser**: Build AST for basic constructs
  - Variable declarations with type inference (`name: value`)
  - Simple expressions (logical)
  - Function definitions with explicit types
  - Basic types (Number, String, Bool)
- **AST Interpreter**: Execute AST directly
- **Standard Library Basics**: Print function
- **Milestone**: Run "Hello World" and basic arithmetic

### v0.2.0 - Pattern Matching & Control Flow
- **Pattern Matching**: `match` expressions
  - Type matching
  - Value matching
  - Default case (`_`)
  - Member access patterns (`.equals()`)
- **Method Calls**: Dot notation for methods
- **Built-in Methods**: Basic string and number methods
- **Milestone**: Compile programs using pattern matching for control flow

### v0.3.0 - Functions & Lexical Scoping
- **Function Definitions**: 
  - With and without type annotations
  - Return type inference
  - Generic functions (simple cases)
- **Function Calls**: Positional arguments
- **Lexical Scoping**: Strict function-level scoping
- **Access Rules**: Parameters, local vars, global constants only
- **Milestone**: Compile programs with multiple functions and proper scoping

## Phase 2: Type System & Core Features (v0.4.0 - v0.7.0)
**Goal**: Implement Suru's unique type system and structural typing

### v0.4.0 - Structural Type System
- **Type Definitions**: 
  - Simple types (flags like `type Success`)
  - Type aliases (`type UserId: Number`)
  - Alternative types (`type Status: Success, Error, Loading`)
  - Record types with fields and methods
- **Structural Typing**: Type compatibility by shape, not name
- **Type Checking**: Structural equivalence checking
- **Better Error Messages**: Type mismatch explanations
- **Milestone**: Programs using structural typing with multiple record types

### v0.5.0 - Advanced Types & Composition
- **Type Composition**: Using `+` operator
  - Struct composition
  - Type extension
  - Method composition with `+` prefix
- **Generic Types**: 
  - Single type parameters (`List<T>`)
  - Multiple type parameters (`Map<K, V>`)
  - Basic constraints
- **Union Types**: Multi-variant types
- **Milestone**: Generic collections and type composition working

### v0.6.0 - Collections & Iteration
- **Collection Types**: `List<T>`, `Set<T>`, `Map<K,V>`
- **Unified Syntax**: `[]` for all collections, type-driven interpretation
- **Collection Methods**: 
  - `.each()`, `.map()`, `.filter()`
  - Iteration with continuation types
- **Number Iteration**: `.times()` method
- **Continuation Types**: `Continue`, `Break<T>`, `Produce<T>`
- **Milestone**: Complex data processing with collections

### v0.7.0 - Privacy, Encapsulation & LSP
- **Private Members**: `_` prefix for private fields and methods
- **Public Interface**: Type declarations define public API
- **Constructor Functions**: Type-named constructors
- **Instance Methods**: Per-instance implementations
- **`this` Reference**: Self-reference in methods
- **Language Server Protocol (LSP)**: Basic implementation
  - Syntax highlighting
  - Go to definition
  - Hover information
  - Basic diagnostics
- **Milestone**: Properly encapsulated types with IDE support

## Phase 3: Advanced Language Features (v0.8.0 - v0.10.0)
**Goal**: Currying, pipelines, and error handling

### v0.8.0 - Currying & Partial Application
- **Placeholder Currying**: Using `_` for partial application
- **Explicit Partial**: `partial` keyword for many-argument functions
- **Method Currying**: Currying on type methods
- **Function Composition**: Curried functions in pipelines
- **Milestone**: Complex functional programming patterns working

### v0.9.0 - Pipeline Operator & Error Handling
- **Pipeline Operator**: `|` for chaining operations
- **Result Types**: `Result<T, E>`, `Option<T>`, `Response<T, E>`, `Either<L, R>`
- **Try Operator**: `try` keyword for error short-circuiting
- **Try Compatibility**: Works with any 2-variant union type
- **Pipeline + Try**: Combining operators for clean error handling
- **Milestone**: Complex data pipelines with proper error propagation

### v0.10.0 - Modules & Organization
- **Module System**: 
  - Module declarations
  - Import statements (aliased, selective, wildcard)
  - Export declarations
- **File Structure**: `.suru` files, module directories
- **Module Resolution**: Finding and loading modules
- **Main Function**: Entry point in main module
- **Milestone**: Multi-file projects with proper module structure

## Phase 4: Polish & Advanced Features (v0.11.0 - v0.13.0)
**Goal**: String interpolation, overloading, and documentation

### v0.11.0 - String Interpolation
- **Basic Interpolation**: Single backtick with `{expr}`
- **Multi-line Strings**: Backtick followed by newline
- **Nested Interpolation**: Multiple backtick levels
  - `{{}}` for double backticks
  - `{{{}}}` for triple backticks
  - `{{{{}}}}` for quadruple backticks
- **Escape Sequences**: `\n`, `\t`, `\xNN`, `\uNNNN`, `\UNNNNNNNN`
- **Milestone**: Complex string templating and formatting

### v0.12.0 - Function & Method Overloading
- **Function Overloading**: Same name, different parameter types
- **Return Type Overloading**: Same signature, different return types
- **Method Overloading**: Overloaded methods in types
- **Overload Resolution**: Choose correct overload at compile time
- **Milestone**: Type-safe overloading across the language

### v0.13.0 - Documentation System
- **Doc Comments**: Markdown between `====` delimiters
- **Doc Annotations**: 
  - `@param`, `@return`, `@example`
  - `@deprecated`, `@experimental`, `@todo`
  - `@see`, `@link`, `@author`, `@since`
- **Doc Generation**: Extract documentation to readable format
- **Milestone**: Syntax is final

## Phase 5: Code Generation (v0.14.0)
**Goal**: Native compilation target

### v0.14.0 - x86-64 Linux Code Generation
- **Assembly Generation**: Emit x86-64 assembly code
- **Calling Conventions**: System V AMD64 ABI
- **Register Allocation**: Basic register allocation strategy
- **System Calls**: Linux syscalls for I/O
- **Linking**: Generate object files and link with system linker
- **Binary Output**: Produce standalone native executables
- **Performance Baseline**: Establish benchmarks vs interpreter
- **Milestone**: Native executables running on Linux x86-64

## Phase 6: Standard Library & tooling (v0.15.0 - v0.16.0)
**Goal**: Implement and document a minimal viable standar library

### v0.15.0 - Implementation
- **Standard Library**: 
  - Complete collection implementations
  - String manipulation functions
  - Math library
  - File I/O basics
  - Async/concurrency primitives
  - FFI (Foreign Function Interface) for C interop
- **Tooling**:
  - Format tool for code formatting
- **Documentation**: Complete language guide and API docs
- **Examples**: Real-world example programs
- **Milestone**: Well-documented standard library and examples

### v0.15.0 - Optimization Pass
- **Constant Folding**: Compile-time expression evaluation
- **Dead Code Elimination**: Remove unused functions and types
- **Type Specialization**: Monomorphization of generics
- **Method Inlining**: Inline small methods for performance
- **Pipeline Optimization**: Optimize chained operations
- **Milestone**: Significant performance improvements on benchmarks

### v0.16.0 - Refinemnet

- **Documentation**: 
  - Complete language reference
  - Tutorial series
  - API documentation

---

## Implementation Notes

### Current Status
- A working build system (`builder.c`, `./builder`)
- Basic project structure in place
