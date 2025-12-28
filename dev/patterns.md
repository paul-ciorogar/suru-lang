# Common Code Patterns

> Reusable patterns and best practices for working with the Suru compiler codebase

## Overview

This document contains common code patterns used throughout the Suru compiler implementation. These patterns represent tested, working solutions to recurring problems.

## Compiler Limits Pattern

### Using Compiler Limits

All compiler components should respect configurable limits to prevent resource exhaustion:

```rust
use crate::limits::CompilerLimits;

// Load from project.toml (or use defaults if missing)
let limits = CompilerLimits::from_project_toml("project.toml")?;
limits.validate()?; // Ensure limits are reasonable

// Use in lexer/parser
let lexer = Lexer::new(source, &limits);
let parser = Parser::new(tokens, &limits);
```

**Key Points:**
- Always validate limits after loading
- Fall back to defaults if project.toml is missing
- Pass limits by reference to avoid copying

**Code Location:** `src/limits.rs`

## CLI Extension Pattern

### Adding a New CLI Command

Follow this pattern when adding new commands to the CLI:

**Step 1:** Add command variant to `Commands` enum in `src/cli.rs`

```rust
#[derive(clap::Subcommand)]
enum Commands {
    Parse(ParseArgs),
    Lex(LexArgs),     // New command
    // ... other commands
}
```

**Step 2:** Create args struct using `#[derive(clap::Args)]`

```rust
#[derive(clap::Args)]
struct LexArgs {
    /// Input file to lex
    file: PathBuf,

    /// Show detailed token information
    #[arg(short, long)]
    verbose: bool,
}
```

**Step 3:** Add handler in `src/main.rs` match statement

```rust
match cli.command {
    Commands::Parse(args) => { /* ... */ },
    Commands::Lex(args) => handle_lex_command(args)?,
    // ... other commands
}
```

**Step 4:** Update `todo.md` to track implementation status

**Code Location:** `src/cli.rs`, `src/main.rs`

## AST Extension Pattern

### Adding a New AST Node Type

When adding new syntax that requires a new AST node:

**Step 1:** Add variant to `NodeType` enum in `src/ast.rs`

```rust
pub enum NodeType {
    // ... existing variants
    MatchExpr,      // New node type
    MatchArm,
    Pattern,
}
```

**Step 2:** Add parsing logic in appropriate `src/parser/*.rs` file

```rust
fn parse_match_expr(&mut self, depth: usize) -> Result<usize, ParseError> {
    self.check_depth(depth)?;

    let match_node = self.ast.create_node(
        NodeType::MatchExpr,
        Some(self.current),
    );

    // ... parsing logic

    Ok(match_node)
}
```

**Step 3:** Add comprehensive tests

```rust
#[test]
fn test_parse_match_expression() {
    let input = "match value { Ok: 1, Error: 0 }";
    let tokens = lex(input);
    let mut parser = Parser::new(tokens, &default_limits());
    let ast = parser.parse().unwrap();

    // Verify AST structure
    assert_eq!(ast.nodes[0].node_type, NodeType::MatchExpr);
}
```

**Step 4:** Update code generation in `src/codegen.rs` (future work)

**Code Location:** `src/ast.rs`, `src/parser/`, `src/codegen.rs`

## Parser Recursive Descent Pattern

### Standard Parsing Method Signature

All parsing methods follow this consistent pattern:

```rust
fn parse_statement(&mut self, depth: usize) -> Result<Option<usize>, ParseError> {
    self.check_depth(depth)?;

    match &self.current_token().kind {
        TokenKind::Return => self.parse_return_stmt(depth + 1),
        TokenKind::Ident => {
            // Disambiguate function vs variable
            if self.peek_token().kind == TokenKind::Colon {
                self.parse_var_or_function_decl(depth + 1)
            } else {
                self.parse_expr_stmt(depth + 1)
            }
        },
        _ => Err(ParseError::unexpected_token(...)),
    }
}
```

**Key Points:**
- Always call `check_depth(depth)?` first
- Pass `depth + 1` to recursive calls
- Return `Result<Option<usize>, ParseError>` for optional elements
- Return `Result<usize, ParseError>` for required elements

**Code Location:** `src/parser/statements.rs`, `src/parser/expressions.rs`

### Expression Parsing with Precedence

Expression parsing uses precedence climbing:

```rust
fn parse_expression(&mut self, depth: usize, min_precedence: u8)
    -> Result<usize, ParseError>
{
    self.check_depth(depth)?;

    // Parse primary expression (literal, identifier, etc.)
    let mut left = self.parse_primary(depth + 1)?;

    // Precedence climbing loop
    while self.current_token_precedence() >= min_precedence {
        let operator = self.current_token();
        let op_precedence = self.get_precedence(operator);

        self.advance();

        let right = self.parse_expression(depth + 1, op_precedence + 1)?;
        left = self.create_binary_op(operator, left, right);
    }

    Ok(left)
}
```

**Key Points:**
- Start with primary expression
- Use precedence climbing for operators
- Always increment depth for recursive calls
- Check depth before any recursive call

**Code Location:** `src/parser/expressions.rs`, `src/parser/helpers.rs`

## AST Traversal Pattern

### Iterating Through Children

Use the first-child/next-sibling pattern to traverse the tree:

```rust
// Iterate through all children of a node
if let Some(child_idx) = node.first_child {
    let mut current = child_idx;
    loop {
        // Process current node
        process_node(&ast.nodes[current]);

        // Move to next sibling
        if let Some(next) = ast.nodes[current].next_sibling {
            current = next;
        } else {
            break;
        }
    }
}
```

**Recursive Version:**

```rust
fn visit_node(ast: &Ast, node_idx: usize) {
    let node = &ast.nodes[node_idx];

    // Process this node
    println!("Visiting {:?}", node.node_type);

    // Visit children
    if let Some(child_idx) = node.first_child {
        let mut current = child_idx;
        loop {
            visit_node(ast, current);

            if let Some(next) = ast.nodes[current].next_sibling {
                current = next;
            } else {
                break;
            }
        }
    }
}
```

**Key Points:**
- Use indices, not references (avoids lifetime issues)
- Check for `first_child` before iterating
- Use loop with break instead of while for clarity
- Recursive version is cleaner but uses stack

**Code Location:** `src/ast.rs`

## Error Handling Pattern

### Creating Parse Errors

Use consistent error creation throughout the parser:

```rust
// Unexpected token
return Err(ParseError::UnexpectedToken {
    expected: "expected description",
    got: self.current_token().kind.clone(),
    location: self.current_token().location(),
});

// Recursion limit exceeded
return Err(ParseError::RecursionLimitExceeded {
    limit: self.limits.max_expression_depth,
    location: self.current_token().location(),
});

// Invalid syntax
return Err(ParseError::InvalidSyntax {
    message: "description of what's wrong",
    location: self.current_token().location(),
});
```

**Key Points:**
- Always include location information
- Use descriptive error messages
- Reference limits when applicable
- Clone token kind for error (tokens are consumed)

**Code Location:** `src/parser/error.rs`

## Testing Patterns

### Basic Parse Test

```rust
#[test]
fn test_parse_feature() {
    let input = "x: 42";
    let limits = CompilerLimits::default();
    let tokens = Lexer::new(input, &limits).tokenize().unwrap();
    let mut parser = Parser::new(tokens, &limits);
    let ast = parser.parse().unwrap();

    // Verify structure
    assert_eq!(ast.nodes.len(), 2); // Program + VarDecl
    assert_eq!(ast.nodes[1].node_type, NodeType::VarDecl);
}
```

### Error Test

```rust
#[test]
fn test_parse_error_case() {
    let input = "x:"; // Missing value
    let limits = CompilerLimits::default();
    let tokens = Lexer::new(input, &limits).tokenize().unwrap();
    let mut parser = Parser::new(tokens, &limits);

    let result = parser.parse();
    assert!(result.is_err());

    if let Err(ParseError::UnexpectedToken { expected, .. }) = result {
        assert!(expected.contains("expression"));
    }
}
```

### Recursion Limit Test

```rust
#[test]
fn test_recursion_limit() {
    let mut limits = CompilerLimits::default();
    limits.max_expression_depth = 5; // Small limit for testing

    // Create deeply nested expression: not not not not not true
    let input = "x: not not not not not not true";
    let tokens = Lexer::new(input, &limits).tokenize().unwrap();
    let mut parser = Parser::new(tokens, &limits);

    let result = parser.parse();
    assert!(matches!(result, Err(ParseError::RecursionLimitExceeded { .. })));
}
```

**Code Location:** `src/parser/statements.rs`, `src/parser/expressions.rs`, `src/parser/types.rs`

## Token Navigation Pattern

### Safe Token Access

```rust
// Current token (always valid)
let current = self.current_token();

// Peek ahead (returns EOF if past end)
let next = self.peek_token();

// Advance to next token
self.advance();

// Check and consume expected token
if self.current_token().kind == TokenKind::Colon {
    self.advance();
} else {
    return Err(ParseError::expected_token(":", self.current_token()));
}

// Expect and consume specific token (helper method)
self.expect(TokenKind::Colon)?;
```

**Key Points:**
- `current_token()` never panics (returns EOF if past end)
- `peek_token()` looks ahead one token safely
- Always check token kind before consuming
- Use helper methods for common patterns

**Code Location:** `src/parser/helpers.rs`

## Zero-Copy Token Text Pattern

### Accessing Token Text

```rust
// Tokens store byte offsets, not text
let token = &tokens[index];

// Get text when needed (requires source string)
let text = token.text(source);

// For identifiers
if token.kind == TokenKind::Ident {
    let name = token.text(source);
    println!("Identifier: {}", name);
}
```

**Key Points:**
- Tokens reference source string by offset
- Only extract text when actually needed
- Source string must outlive tokens
- Zero allocation during lexing

**Code Location:** `src/lexer.rs`

## Best Practices

1. **Always validate depth:** Call `check_depth(depth)?` at the start of every recursive parsing method
2. **Use type-safe enums:** Prefer `TokenKind` and `NodeType` enums over strings
3. **Test error cases:** Every feature should have error tests
4. **Keep tests close to code:** Co-locate tests with implementation
5. **Document public APIs:** Add doc comments to public functions
6. **Use descriptive names:** `parse_var_decl` is better than `parse_vd`
7. **Return early on errors:** Use `?` operator for clean error propagation
8. **Avoid unwrap in production:** Use proper error handling

## Anti-Patterns to Avoid

1. **Don't skip depth checks:** Stack overflow is a security issue
2. **Don't use `unwrap()` in parser:** Always handle errors properly
3. **Don't mutate AST after creation:** Build tree during parsing
4. **Don't store string copies in tokens:** Use byte offsets
5. **Don't use recursion without limits:** Always track and limit depth
6. **Don't hardcode limits:** Use `CompilerLimits` configuration
7. **Don't ignore test failures:** Fix or document failing tests

---

**See also:**
- [Testing Guide](testing.md) - Test patterns and coverage
- [Module Responsibilities](modules.md) - What each module does
- [Architecture](../docs/contributing/architecture.md) - Overall structure
