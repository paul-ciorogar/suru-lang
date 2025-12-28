# Changelog

All notable changes to Suru Lang will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
