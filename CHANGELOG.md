# Changelog

All notable changes to Suru Lang will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
