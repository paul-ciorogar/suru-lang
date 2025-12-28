# Compiler Architecture

> Overview of the Suru compiler implementation

## Overview

Suru Lang is a minimalist, library-driven, general-purpose programming language implemented in Rust with LLVM 18 as the compilation backend.

**Language Characteristics:**
- Minimal syntax with maximum expressiveness
- Generic type constraint inference (duck typing)
- No garbage collection - simple ownership model
- Interactive development through LSP-first tooling
- Method-centric design - no traditional loop keywords

## Compilation Pipeline

### Current Pipeline

```
Source Code (.suru)
    ↓
Lexer (src/lexer.rs)
    ↓
Tokens
    ↓
Parser (src/parser/)
    ↓
AST (src/ast.rs)
    ↓
[TODO] Semantic Analysis
    ↓
[TODO] LLVM IR Generation (src/codegen.rs)
    ↓
Object File → Executable
```

### Implementation Status

**Completed:**
1. **Lexer** - Full tokenization of Suru syntax
2. **Parser** - Pure recursive descent with 2-token lookahead
3. **AST** - Uniform-size nodes with index-based tree structure
4. **CLI** - Command-line interface using clap
5. **Compiler Limits** - Comprehensive safety system

**In Progress:**
- Semantic analysis
- Type system implementation

**TODO:**
- LLVM IR code generation
- Error recovery
- LSP server
- Standard library

## Module Overview

### src/main.rs

**Purpose:** CLI entry point

**Responsibilities:**
- Routes commands to appropriate handlers
- Implements `suru parse <file>` command
- Error handling and user-friendly output

**Size:** ~67 lines

### src/cli.rs

**Purpose:** CLI argument definitions

**Responsibilities:**
- Command structure using clap
- Subcommands (parse, lex, compile, run, lsp)
- Type-safe argument parsing

**Size:** ~21 lines

### src/lexer.rs

**Purpose:** Tokenization

**Responsibilities:**
- Converts source code into `Vec<Token>`
- Zero-copy design using byte offsets
- Comprehensive error reporting with line/column info
- Handles all Suru literals and keywords

**Features:**
- 14 keywords
- Multiple number bases with type suffixes
- String literals (standard and interpolated)
- Operators and punctuation

**Size:** ~916 lines (complete)

### src/parser/

**Purpose:** Parse tokens into AST

**Structure (modular):**
- `mod.rs` - Parser struct, public API (~60 lines)
- `error.rs` - ParseError type (~50 lines)
- `helpers.rs` - Utilities and precedence (~90 lines)
- `expressions.rs` - Expression parsing (~450 lines with tests)
- `types.rs` - Type declaration parsing (~750 lines with tests)
- `statements.rs` - Statement parsing (~750 lines with tests)

**Total:** ~2,150 lines with ~200 tests

**Parsing Approach:**
- Pure recursive descent
- Precedence climbing for expressions
- 2-token lookahead for disambiguation
- Configurable recursion depth limit

**Currently Parses:**
- Variable declarations with expressions
- Function declarations with typed/inferred parameters
- Return type annotations
- Return statements
- Statement blocks
- Function calls (standalone and in expressions)
- Method calls and property access
- Boolean operators
- All 7 type declaration forms
- All literal types

### src/ast.rs

**Purpose:** AST data structures

**Key Features:**
- First-child/next-sibling representation
- Uniform node size (cache-friendly)
- Index-based references (no lifetimes)
- Single vector storage

**Node Types:**
- Declarations: Program, VarDecl, FunctionDecl, TypeDecl
- Type System: TypeName, TypeBody, TypeAnnotation, etc.
- Functions: ParamList, Param, Block, ReturnStmt
- Expressions: Identifier, Literals, Operators
- Method calls: MethodCall, PropertyAccess

**Size:** ~175 lines

### src/limits.rs

**Purpose:** Compiler safety limits

**Features:**
- Configurable resource limits
- TOML-based configuration via project.toml
- Prevents DoS attacks from pathological input
- Full validation and error handling

**Limits:**
- Input size: 10MB default
- Token count: 100k default
- Expression depth: 256 default
- AST nodes: 1M default
- String/identifier/comment lengths

**Size:** ~278 lines with 8 unit tests

### src/codegen.rs

**Purpose:** LLVM IR generation

**Status:** Skeleton implementation

**Future:**
- Generate LLVM IR from AST
- Function compilation
- Type lowering to LLVM types
- Memory management code generation

**Size:** ~106 lines

## Data Flow

### Lexing

```
Source String
    ↓
Lexer::new(source, limits)
    ↓
Token Stream (Vec<Token>)
    ↓
Zero-copy: tokens reference source via byte offsets
```

### Parsing

```
Token Stream
    ↓
Parser::new(tokens, limits)
    ↓
Recursive Descent Parsing
    ↓
AST (Vec<AstNode> with FCNS structure)
```

### AST Traversal

```
AST Root (nodes[0])
    ↓
First Child → Next Sibling → Next Sibling → ...
    ↓              ↓
  Children     More siblings
```

## Key Design Patterns

### Zero-Copy Lexer

Tokens store byte offsets instead of copying text:

```rust
pub struct Token {
    pub kind: TokenKind,
    pub start: usize,  // Byte offset in source
    pub length: usize, // Length in bytes
    pub line: usize,
    pub column: usize,
}
```

**Benefits:**
- No string allocations during lexing
- Single source string shared
- `token.text()` provides access when needed

### First-Child/Next-Sibling AST

```rust
pub struct AstNode {
    pub node_type: NodeType,
    pub token_index: Option<usize>,
    pub first_child: Option<usize>,
    pub next_sibling: Option<usize>,
}
```

**Benefits:**
- Uniform node size
- Single vector storage
- Index-based references (no lifetimes)
- Cache-friendly

### Recursive Descent with Depth Tracking

```rust
fn parse_expression(&mut self, depth: usize, min_precedence: u8)
    -> Result<usize, ParseError>
{
    self.check_depth(depth)?;
    // Parsing logic...
}
```

**Benefits:**
- Prevents stack overflow
- Configurable limits
- Natural grammar mapping

## Testing Strategy

### Unit Tests

**Location:** Co-located with implementation

- Lexer tests in `src/lexer.rs`
- Parser tests in `src/parser/*.rs`
- Limits tests in `src/limits.rs`

**Total:** 152+ tests passing

**Coverage:**
- All token types
- All AST node types
- Error cases
- Edge cases
- Recursion depth limits
- Tree structure validation

### Integration Tests

**TODO:** Integration tests in `tests/` directory

## Error Handling

### Lexer Errors

```rust
pub enum LexError {
    UnexpectedCharacter { line, column, char },
    UnterminatedString { line, column },
    InvalidNumberLiteral { line, column, reason },
    // ...
}
```

### Parser Errors

```rust
pub enum ParseError {
    UnexpectedToken { expected, got, location },
    RecursionLimitExceeded { limit, location },
    InvalidSyntax { message, location },
    // ...
}
```

## Performance Considerations

### Memory Efficiency

- Zero-copy lexer
- Single vector AST storage
- Index-based references
- No heap allocations per node

### Parse Speed

- Pure recursive descent (fast)
- Minimal lookahead (2 tokens max)
- Direct grammar mapping
- No backtracking

### Compiler Limits

Configurable safety limits prevent:
- Excessive memory usage
- Stack overflow
- Infinite loops
- DoS attacks

## Future Architecture

### Planned Components

1. **Semantic Analyzer**
   - Symbol table
   - Scope tracking
   - Type checking
   - Generic type resolution

2. **Type System**
   - Generic type constraint inference
   - Duck typing validation
   - Type compatibility checking

3. **Code Generator**
   - LLVM IR generation
   - Memory management
   - Optimization passes

4. **LSP Server**
   - Incremental parsing
   - Real-time error checking
   - Code completion
   - Hover information

## Development Environment

**Docker-based:**
- Ubuntu 24.04 LTS
- Rust stable (edition 2024)
- LLVM 18 with full dev libraries
- Inkwell 0.6 (Rust LLVM bindings)

See [Development Workflow](development.md) for setup instructions.

---

**See also:**
- [Design Decisions](design-decisions.md) - Key architectural choices
- [Development Workflow](development.md) - Build and test instructions
- [Roadmap](roadmap.md) - Future plans
