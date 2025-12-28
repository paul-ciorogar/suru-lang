# Module Responsibilities

> Detailed breakdown of what each source file does

## Overview

This document describes the responsibilities and internal structure of each module in the Suru compiler. Use this as a reference when deciding where to add new code.

## Source File Organization

```
src/
├── main.rs          # Entry point with CLI (~67 lines)
├── cli.rs           # CLI interface using clap (~21 lines)
├── lexer.rs         # Tokenizer (~916 lines, complete)
├── parser/          # Parser module (modular structure)
│   ├── mod.rs       # Module root, Parser struct, public API (~60 lines)
│   ├── error.rs     # ParseError type (~50 lines)
│   ├── helpers.rs   # Utilities and precedence (~90 lines)
│   ├── expressions.rs  # Expression parsing (~450 lines with tests)
│   ├── types.rs     # Type declaration parsing (~750 lines with tests)
│   └── statements.rs   # Statement parsing (~750 lines with tests)
├── ast.rs           # AST data structures (~175 lines)
├── limits.rs        # Compiler safety limits (~278 lines)
└── codegen.rs       # LLVM code generation (skeleton, ~106 lines)
```

## Module Details

### src/main.rs

**Size:** ~67 lines

**Purpose:** CLI entry point

**Responsibilities:**
- Parse command-line arguments using clap
- Route commands to appropriate handlers
- Implement `suru parse <file>` command
- Error handling and user-friendly output
- Exit with appropriate status codes

**Key Functions:**
- `main()` - Entry point, parses CLI and routes commands
- Command handlers for each CLI subcommand

**Dependencies:**
- `cli` - Command definitions
- `lexer` - Tokenization
- `parser` - Parsing
- `limits` - Compiler limits loading

**Future Extensions:**
- `suru lex <file>` command
- `suru compile <file>` command
- `suru run <file>` command
- `suru lsp` command

**Code Location:** `src/main.rs`

### src/cli.rs

**Size:** ~21 lines

**Purpose:** CLI argument definitions

**Responsibilities:**
- Define CLI structure using clap
- Declare subcommands (parse, lex, compile, run, lsp)
- Type-safe argument parsing
- Help text generation

**Key Types:**
- `Cli` - Main CLI structure with version info
- `Commands` - Enum of all subcommands
- `ParseArgs` - Arguments for parse command
- (Future: `LexArgs`, `CompileArgs`, `RunArgs`, `LspArgs`)

**Dependencies:**
- `clap` - CLI parsing library

**Design Notes:**
- Uses derive macros for automatic CLI generation
- Each command has its own args struct
- Help text auto-generated from doc comments

**Code Location:** `src/cli.rs`

### src/lexer.rs

**Size:** ~916 lines (complete implementation)

**Purpose:** Tokenization of Suru source code

**Responsibilities:**
- Convert source code into `Vec<Token>`
- Handle all Suru syntax elements:
  - 14 keywords (module, import, export, return, match, type, try, and, or, not, true, false, this, partial)
  - Numbers (binary 0b, octal 0o, hex 0x, decimal, float)
  - Type suffixes (i8, i16, i32, i64, u8, u16, u32, u64, f32, f64)
  - String literals (standard "..." / '...', interpolated `...`)
  - Operators and punctuation
  - Comments (line `#` and block `#* ... *#`)
- Zero-copy design using byte offsets
- Comprehensive error reporting with line/column info
- Respect compiler limits (max tokens, max string length, etc.)

**Key Types:**
- `Token` - Single token with kind, position, and byte offset
- `TokenKind` - Enum of all token types
- `Lexer` - Main lexer struct

**Key Methods:**
- `new(source, limits)` - Create lexer
- `tokenize()` - Tokenize entire input
- `next_token()` - Get next single token
- `lex_number()`, `lex_string()`, `lex_identifier()` - Specific token types

**Design Notes:**
- Tokens store byte offsets, not copied text
- `token.text(source)` provides text access
- Single source string shared across all tokens
- No allocations during tokenization

**Dependencies:**
- `limits::CompilerLimits` - Resource limits

**Testing:**
- ~30 comprehensive tests
- Tests for all token types
- Error case testing
- All tests in same file (bottom)

**Code Location:** `src/lexer.rs`

### src/parser/ (Module)

**Total Size:** ~2,150 lines including tests

**Purpose:** Parse tokens into Abstract Syntax Tree

**Module Organization:**
Parser is split into multiple files by responsibility:
- `mod.rs` - Public API and Parser struct
- `error.rs` - Error types
- `helpers.rs` - Utilities and precedence
- `expressions.rs` - Expression parsing
- `types.rs` - Type declaration parsing
- `statements.rs` - Statement parsing

#### src/parser/mod.rs

**Size:** ~60 lines

**Purpose:** Module root and public API

**Responsibilities:**
- Declare parser module structure
- Export public types
- Define `Parser` struct
- Provide main `parse()` entry point

**Key Types:**
- `Parser` - Main parser struct
- Public re-exports of error types

**Code Location:** `src/parser/mod.rs`

#### src/parser/error.rs

**Size:** ~50 lines

**Purpose:** Parser error types

**Responsibilities:**
- Define all parser error types
- Implement `Display` for user-friendly error messages
- Implement `Error` trait

**Key Types:**
- `ParseError` - Enum of all possible parse errors
- Error variants:
  - `UnexpectedToken` - Got wrong token
  - `RecursionLimitExceeded` - Too deep nesting
  - `InvalidSyntax` - Malformed syntax
  - `UnexpectedEof` - Unexpected end of file

**Code Location:** `src/parser/error.rs`

#### src/parser/helpers.rs

**Size:** ~90 lines

**Purpose:** Parsing utilities and operator precedence

**Responsibilities:**
- Token navigation (advance, peek, current)
- Operator precedence mapping
- Depth checking for recursion limits
- Common parsing utilities

**Key Methods:**
- `advance()` - Move to next token
- `peek()` - Look ahead one token
- `current_token()` - Get current token
- `check_depth(depth)` - Verify recursion depth
- `get_precedence(operator)` - Get operator precedence

**Operator Precedence:**
1. `or` - lowest (precedence 1)
2. `and` - medium (precedence 2)
3. `not` - highest (precedence 3)
4. `.` - highest (precedence 4) - method/property access

**Code Location:** `src/parser/helpers.rs`

#### src/parser/expressions.rs

**Size:** ~450 lines including tests

**Purpose:** Expression parsing

**Responsibilities:**
- Parse all expression types:
  - Literals (numbers, strings, booleans)
  - Identifiers
  - Boolean operators (and, or, not)
  - Function calls
  - Method calls
  - Property access
- Precedence climbing algorithm
- Operator associativity

**Key Methods:**
- `parse_expression(depth, min_precedence)` - Main expression parser
- `parse_primary(depth)` - Primary expressions (literals, identifiers)
- `parse_function_call(depth)` - Function call expressions
- `parse_method_call(depth)` - Method call and property access

**Testing:**
- ~40 comprehensive tests
- Tests for all expression types
- Precedence and associativity tests
- Function call tests

**Code Location:** `src/parser/expressions.rs`

#### src/parser/types.rs

**Size:** ~750 lines including tests

**Purpose:** Type declaration parsing

**Responsibilities:**
- Parse all 7 type forms:
  1. Unit types (`type Success`)
  2. Type aliases (`type UserId: Number`)
  3. Union types (`type Status: Success, Error`)
  4. Function types (`type Fn: (x Number) Number`)
  5. Struct types (`type Person: { name String }`)
  6. Intersection types (`type Employee: Person + Manager`)
  7. Generic types (`type List<T>: { items Array }`)
- Parse type parameters and constraints
- Parse struct fields and methods

**Key Methods:**
- `parse_type_decl(depth)` - Main type declaration parser
- `parse_type_body(depth)` - Type body (after colon)
- `parse_struct_body(depth)` - Struct fields and methods
- `parse_type_params(depth)` - Generic type parameters
- `parse_union_type(depth)` - Union alternatives
- `parse_intersection_type(depth)` - Type composition

**Testing:**
- ~45 comprehensive tests
- Tests for all 7 type forms
- Tests for nested and complex types
- Generic constraint tests

**Code Location:** `src/parser/types.rs`

#### src/parser/statements.rs

**Size:** ~750 lines including tests

**Purpose:** Statement parsing

**Responsibilities:**
- Parse variable declarations (`x: 42`)
- Parse function declarations
- Parse function parameters (typed/inferred)
- Parse return statements
- Parse statement blocks
- Disambiguate functions from variables (2-token lookahead)

**Key Methods:**
- `parse_statement(depth)` - Main statement parser
- `parse_var_or_function_decl(depth)` - Disambiguate variable vs function
- `parse_function_decl(depth)` - Function declarations
- `parse_param_list(depth)` - Function parameters
- `parse_block(depth)` - Statement blocks
- `parse_return_stmt(depth)` - Return statements

**Testing:**
- ~35 comprehensive tests
- Variable declaration tests
- Function declaration tests
- Parameter tests (typed/inferred/mixed)
- Return statement tests
- Block tests

**Code Location:** `src/parser/statements.rs`

### src/ast.rs

**Size:** ~175 lines

**Purpose:** AST data structures and tree manipulation

**Responsibilities:**
- Define AST node types
- Implement first-child/next-sibling tree structure
- Provide tree manipulation methods
- Store all nodes in single vector

**Key Types:**
- `Ast` - Container for all nodes
- `AstNode` - Single node with type and tree links
- `NodeType` - Enum of all node types (50+ variants)

**Node Categories:**
- Declarations: Program, VarDecl, FunctionDecl, TypeDecl
- Type System: TypeName, TypeBody, TypeAnnotation, TypeParams, etc.
- Functions: ParamList, Param, Block, ReturnStmt, FunctionCall
- Expressions: Identifier, Literals, Operators
- Method calls: MethodCall, PropertyAccess

**Key Methods:**
- `new()` - Create empty AST
- `create_node(type, token)` - Add node to tree
- `set_first_child(parent, child)` - Link child
- `add_sibling(node, sibling)` - Link sibling

**Design:**
- Uniform node size (cache-friendly)
- Index-based references (no lifetimes)
- Single vector storage
- First-child/next-sibling tree representation

**Code Location:** `src/ast.rs`

### src/limits.rs

**Size:** ~278 lines including tests

**Purpose:** Compiler safety limits

**Responsibilities:**
- Define configurable resource limits
- Load limits from `project.toml` (TOML configuration)
- Validate limits are reasonable
- Prevent DoS attacks from pathological input

**Key Types:**
- `CompilerLimits` - All limit values

**Limits:**
- `max_input_size`: 10MB default (max file size)
- `max_tokens`: 100,000 default
- `max_expression_depth`: 256 default (recursion limit)
- `max_ast_nodes`: 1,000,000 default
- `max_string_length`: 1MB default
- `max_identifier_length`: 256 default
- `max_comment_length`: 10KB default

**Key Methods:**
- `default()` - Create with default limits
- `from_project_toml(path)` - Load from TOML file
- `validate()` - Check limits are reasonable

**Testing:**
- 8 comprehensive tests
- Default value tests
- TOML parsing tests
- Validation tests
- Error handling tests

**Code Location:** `src/limits.rs`

### src/codegen.rs

**Size:** ~106 lines (skeleton implementation)

**Purpose:** LLVM IR code generation

**Status:** Early development (currently has "Hello World" demo)

**Responsibilities (Future):**
- Generate LLVM IR from AST
- Function compilation
- Type lowering to LLVM types
- Memory management code generation
- Optimization passes

**Dependencies:**
- `inkwell` - Safe Rust bindings for LLVM

**Planned Structure:**
- `CodeGenerator` - Main code generation struct
- Type lowering (Suru types → LLVM types)
- Function compilation
- Expression code generation
- Module compilation

**Code Location:** `src/codegen.rs`

## Module Dependencies

```
main.rs
  ├─→ cli.rs
  ├─→ lexer.rs
  │     └─→ limits.rs
  ├─→ parser/
  │     ├─→ limits.rs
  │     └─→ ast.rs
  └─→ codegen.rs (future)
        └─→ ast.rs
```

## Adding New Code - Decision Tree

**Where should I add my code?**

1. **New CLI command?** → `src/cli.rs` (struct), `src/main.rs` (handler)

2. **New token type?** → `src/lexer.rs` (TokenKind enum, lexing logic)

3. **New syntax for expressions?** → `src/parser/expressions.rs`

4. **New syntax for statements?** → `src/parser/statements.rs`

5. **New syntax for types?** → `src/parser/types.rs`

6. **New AST node type?** → `src/ast.rs` (NodeType enum)

7. **New compiler limit?** → `src/limits.rs` (CompilerLimits struct)

8. **New error type?** → `src/parser/error.rs` (ParseError enum)

9. **Code generation?** → `src/codegen.rs`

10. **Parsing utility?** → `src/parser/helpers.rs`

## Module Best Practices

1. **Keep modules focused:** Each module should have a single, clear responsibility
2. **Co-locate tests:** Tests go in same file as implementation
3. **Minimize dependencies:** Don't create circular dependencies
4. **Document public APIs:** Add doc comments to public functions
5. **Use private by default:** Only expose what's needed
6. **Keep files manageable:** Split large modules (like parser) into submodules

---

**See also:**
- [Architecture](../docs/contributing/architecture.md) - Overall compiler structure
- [Common Patterns](patterns.md) - Code patterns for each module
- [Testing Guide](testing.md) - Test organization
