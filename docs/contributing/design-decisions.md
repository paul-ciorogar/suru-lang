# Design Decisions

> Key architectural choices and their rationale

## Overview

This document explains the major design decisions made in the Suru compiler implementation. Each decision is documented with its rationale and code location for reference.

## 1. Pure Recursive Descent Parser

**Decision:** Use pure recursive descent for all parsing (statements and expressions)

**Rationale:**
- **Simplicity**: Single consistent approach throughout the parser
- **Natural grammar mapping**: Each production rule is a method
- **Depth tracking**: Explicit depth parameter prevents stack overflow
- **Maintainability**: Easier to understand and extend than state machine
- **Best practices**: Standard approach used in most modern compilers

**Implementation:**
All parsing methods follow a consistent pattern with depth tracking:

```rust
fn parse_expression(&mut self, depth: usize, min_precedence: u8)
    -> Result<usize, ParseError>
{
    self.check_depth(depth)?;
    // Parsing logic...
}
```

**Code Location:** `src/parser/` (all modules use recursive descent)

**Benefits:**
- Each grammar rule maps to a single method
- Easy to add new syntax by adding new methods
- Stack overflow protection through explicit depth limits
- Consistent error handling throughout

## 2. First-Child/Next-Sibling AST

**Decision:** Use FCNS tree representation instead of `Vec<NodeId>` for children

**Rationale:**
- **Uniform node size**: Important for cache locality
- **Single vector storage**: All nodes in `Vec<AstNode>`
- **Index-based references**: No lifetimes, serialization-friendly
- **Memory efficient**: No dynamic allocations per node

**Implementation:**

```rust
pub struct AstNode {
    pub node_type: NodeType,
    pub token_index: Option<usize>,
    pub first_child: Option<usize>,
    pub next_sibling: Option<usize>,
}
```

**Code Location:**
- `src/ast.rs:27-39` (AstNode struct)
- `src/ast.rs:79-94` (tree manipulation)

**Benefits:**
- Predictable memory layout improves cache performance
- Single allocation for entire AST
- Easy to serialize/deserialize
- No lifetime annotations needed

**Trade-offs:**
- Slightly more complex traversal code
- Must track indices carefully

## 3. Zero-Copy Lexer

**Decision:** Store byte offsets into source string instead of copying text

**Rationale:**
- **Performance**: No string allocations during lexing
- **Memory efficiency**: Single source string shared
- **Simplicity**: `Token.text()` method provides access when needed

**Implementation:**

```rust
pub struct Token {
    pub kind: TokenKind,
    pub start: usize,  // Byte offset in source
    pub length: usize, // Length in bytes
    pub line: usize,
    pub column: usize,
}
```

**Code Location:** `src/lexer.rs:69-75` (Token struct)

**Benefits:**
- Fast lexing with minimal allocations
- Source string owned by lexer, tokens reference it
- Token text extracted on-demand only when needed

**Trade-offs:**
- Tokens must not outlive source string
- Text access requires source string reference

## 4. Configurable Recursion Depth

**Decision:** Make expression recursion depth configurable (default: 256)

**Rationale:**
- **Safety**: Prevents stack overflow attacks
- **Testing**: Can use small limits to test edge cases
- **Production**: Large limit allows reasonable nesting
- **Security**: Protects against malicious input

**Implementation:**

```rust
fn parse_expression(&mut self, depth: usize, min_precedence: u8)
    -> Result<usize, ParseError>
{
    self.check_depth(depth)?;
    // Parsing logic...
}
```

**Code Location:** `src/parser/helpers.rs` (check_depth method)

**Configuration:**
Can be set via `project.toml`:

```toml
[limits]
max_expression_depth = 256
```

**Benefits:**
- Prevents denial-of-service attacks
- Explicit limits better than implicit stack limits
- Easy to adjust for different use cases

## 5. Comprehensive Compiler Limits System

**Decision:** Create a dedicated limits module with TOML configuration support

**Rationale:**
- **Security**: Prevents DoS attacks from pathological input (huge files, deep nesting)
- **Flexibility**: Project-specific limits via `project.toml` (overrides defaults)
- **Developer-friendly**: Permissive defaults (10MB files, 256 depth) don't block normal work
- **Validation**: Built-in checks prevent unreasonable limits
- **Transparency**: Explicit, documented limits rather than implicit language constraints

**Default Limits:**
- Input size: 10MB
- Token count: 100,000
- Expression depth: 256
- AST nodes: 1,000,000
- String/identifier/comment lengths: Various per-type limits

**Implementation:**

```rust
pub struct CompilerLimits {
    pub max_input_size: usize,
    pub max_tokens: usize,
    pub max_expression_depth: usize,
    pub max_ast_nodes: usize,
    // ... other limits
}
```

**Code Location:**
- `src/limits.rs:14-44` (struct definition)
- `src/limits.rs:55-101` (TOML loading)

**Usage:**

```rust
let limits = CompilerLimits::from_project_toml("project.toml")?;
limits.validate()?;
let lexer = Lexer::new(source, &limits);
```

**Benefits:**
- Prevents resource exhaustion
- Project-specific customization
- Clear error messages when limits exceeded
- Tested with 8 comprehensive unit tests

## 6. CLI-First Architecture

**Decision:** Use `clap` for structured CLI with subcommands

**Rationale:**
- **Ergonomics**: Standard `suru parse <file>` interface familiar to developers
- **Extensibility**: Easy to add new commands (lex, compile, run, lsp)
- **Self-documenting**: Help text auto-generated from code
- **Type-safe**: Compile-time validation of arguments

**Implementation:**

```rust
#[derive(clap::Parser)]
#[command(name = "suru")]
#[command(about = "Suru programming language compiler")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    Parse(ParseArgs),
    // Future: Lex, Compile, Run, Lsp
}
```

**Code Location:**
- `src/cli.rs:3-21` (CLI structure)
- `src/main.rs` (entry point)

**Benefits:**
- Professional command-line interface
- Automatic help generation
- Easy to add new commands
- Follows Unix conventions

**Future Extensions:**
- `suru lex <file>` - Show tokens
- `suru compile <file>` - Compile to binary
- `suru run <file>` - Compile and run
- `suru lsp` - Start LSP server

## Alternative Approaches Considered

### Parser: Pratt vs Recursive Descent

**Considered:** Pratt parsing for expressions

**Chosen:** Recursive descent with precedence climbing

**Reason:** Precedence climbing provides the same power as Pratt while maintaining consistency with statement parsing. Pure recursive descent is simpler to understand and extend.

### AST: Multiple Vectors vs Single Vector

**Considered:** Separate vectors for each node type (typed arena pattern)

**Chosen:** Single vector with uniform nodes

**Reason:** Simpler indexing, better cache locality for traversal, easier to implement

### Limits: Hard-coded vs Configurable

**Considered:** Hard-coded limits in source code

**Chosen:** TOML-based configuration

**Reason:** Allows projects to adjust limits without recompiling compiler, better testing flexibility

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

## Future Considerations

### Incremental Parsing

For LSP support, we may need to add:
- Edit tracking
- Partial re-parsing
- AST diffing

**Note:** The current AST structure (index-based) supports this well.

### Error Recovery

Current parser fails fast on first error. Future improvements:
- Synchronization points
- Error recovery strategies
- Multiple error reporting

### Parallel Parsing

The stateless nature of recursive descent allows:
- Parallel parsing of independent modules
- Concurrent lexing and parsing
- Multi-threaded compilation

## Lessons Learned

1. **Explicit limits are better than implicit**: The compiler limits system prevents entire classes of bugs
2. **Uniform data structures simplify code**: FCNS AST is easier to work with than complex pointer structures
3. **Zero-copy when possible**: Lexer performance benefits significantly from avoiding allocations
4. **Type safety prevents bugs**: Using `clap` for CLI caught many argument handling bugs at compile time

---

**See also:**
- [Architecture](architecture.md) - Overall compiler structure
- [Development Workflow](development.md) - Build and test instructions
- [Roadmap](roadmap.md) - Future plans
