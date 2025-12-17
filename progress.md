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
inkwell = { version = "0.6", features = ["llvm18-1"] }
```

## Build and Run

### Build the project
```bash
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo build
```

### Run the compiler
```bash
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo run
```

### Execute the generated program
```bash
docker run --rm -v $(pwd):/workspace suru-lang:dev ./hello
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
// [Module, Ident("main"), Return, Number(Decimal, "42"), Eof]
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
2. `ExpectColonAfterIdent` - Expecting `:` after variable name
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
   ├─ Ident "x" (nodes[2])
   └─ LiteralNumber "42" (nodes[3])
```

#### Node Types
- `Program` - Root node containing all declarations
- `VarDecl` - Variable declaration
- `Ident` - Identifier (terminal)
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

## Project Structure
```
suru-lang/
├── Cargo.toml          # Rust project manifest with Inkwell dependency
├── Cargo.lock          # Dependency lock file
├── Dockerfile          # Development environment (Ubuntu 24.04 + Rust + LLVM 18)
├── .dockerignore       # Docker build exclusions
├── README.md           # Project documentation
├── progress.md         # This file - development progress log
└── src/
    ├── main.rs         # Entry point with parser demo
    ├── codegen.rs      # LLVM code generation module
    ├── lexer.rs        # Lexer implementation
    ├── ast.rs          # AST data structures
    └── parser.rs       # Pure recursive descent parser with depth limiting
```

## Notes
- All development is done inside Docker container to ensure consistent LLVM environment
- LLVM 18 is explicitly used for latest features and stability
- Inkwell provides safe Rust bindings to LLVM C API
- Lexer follows Rust best practices with zero-copy design and comprehensive error handling
- Parser uses pure recursive descent approach with depth limiting for safety
- Recursion depth is configurable for safety and testing (default: 256)
- AST uses first-child/next-sibling representation for memory efficiency
