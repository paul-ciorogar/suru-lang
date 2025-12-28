# Testing Guide

> Test coverage, testing patterns, and test organization

## Overview

The Suru compiler has comprehensive test coverage with 152+ passing tests. All tests are co-located with their implementation for easy maintenance.

## Test Organization

### Unit Tests

All unit tests are located in the same file as the code they test, using Rust's `#[cfg(test)]` module pattern.

**Test Locations:**
- `src/lexer.rs` - Lexer tests (bottom of file)
- `src/parser/expressions.rs` - Expression parsing tests
- `src/parser/statements.rs` - Statement parsing tests
- `src/parser/types.rs` - Type declaration parsing tests
- `src/limits.rs` - Compiler limits tests
- `src/ast.rs` - AST manipulation tests

**Rationale:**
- Easy to find tests for specific code
- Tests serve as usage examples
- Easier to keep tests updated when code changes
- Standard Rust practice

### Integration Tests

**TODO:** Integration tests will be added in `tests/` directory

**Planned tests:**
- End-to-end parsing of complete programs
- Multi-file compilation
- Error message formatting
- CLI command testing

## Current Test Coverage

### Lexer Tests

**Coverage:** All token types and error cases

**Test Categories:**
- Keywords (14 keywords)
- Identifiers (valid/invalid)
- Number literals (all bases: binary, octal, decimal, hex)
- Type suffixes (i8, i16, i32, i64, u8, u16, u32, u64, f32, f64)
- String literals (single, double, backtick quotes)
- String escape sequences
- Operators and punctuation
- Comments (line and block)
- Error cases (unterminated strings, invalid characters, etc.)

**Total Lexer Tests:** ~30 tests

**Code Location:** `src/lexer.rs` (bottom of file)

### Parser Tests

**Coverage:** 125+ tests covering all implemented syntax

#### Expression Tests

**Test Categories:**
- Literal expressions (numbers, strings, booleans)
- Identifier expressions
- Boolean operators (and, or, not)
- Operator precedence
- Function calls with arguments
- Function calls in expressions
- Nested expressions
- Precedence and associativity

**Test Count:** ~40 tests

**Code Location:** `src/parser/expressions.rs`

#### Statement Tests

**Test Categories:**
- Variable declarations with type inference
- Variable declarations with expressions
- Function declarations (typed/inferred parameters)
- Return statements (with/without values)
- Statement blocks
- Nested functions
- Parameter lists (typed/inferred/mixed)
- Return type annotations

**Test Count:** ~35 tests

**Code Location:** `src/parser/statements.rs`

#### Type Declaration Tests

**Test Categories:**
- Unit types (simple flags/states)
- Type aliases
- Union types (alternatives)
- Function types (with parameters and return types)
- Struct types (fields and methods)
- Intersection types (type composition with +)
- Generic types (with type parameters and constraints)
- Complex nested type structures

**Test Count:** ~45 tests

**Code Location:** `src/parser/types.rs`

#### Error Tests

**Test Categories:**
- Unexpected tokens
- Invalid syntax
- Recursion depth limiting
- Missing required elements (colons, braces, etc.)
- Tree structure validation

**Test Count:** ~10 tests

**Code Location:** Throughout parser modules

### Limits Tests

**Coverage:** All limit validation scenarios

**Test Categories:**
- Default values and validation
- Zero value detection (invalid)
- Too-large value detection (invalid)
- TOML parsing from valid config
- Partial TOML overrides (some limits specified)
- Missing file fallback to defaults
- Malformed TOML error handling
- Valid limit ranges

**Test Count:** 8 comprehensive tests

**Code Location:** `src/limits.rs`

## Test Statistics

**Total Tests:** 152+ passing
- Lexer: ~30 tests
- Parser (expressions): ~40 tests
- Parser (statements): ~35 tests
- Parser (types): ~45 tests
- Parser (errors): ~10 tests
- Limits: 8 tests
- AST: ~5 tests

**Coverage by Component:**
- Lexer: 100% of token types
- Parser: All implemented syntax (variables, functions, types, returns)
- AST: Basic tree operations
- Limits: All validation scenarios
- Codegen: Not yet tested (skeleton only)

## Running Tests

### All Tests

```bash
# Inside container
cargo test

# From host
docker run --rm -v $(pwd):/workspace suru-lang:dev cargo test
```

### Specific Test

```bash
# Run tests matching pattern
cargo test test_function_declarations

# Run tests in specific module
cargo test lexer::tests::

# Run single test
cargo test test_parse_function_with_typed_parameters
```

### With Output

```bash
# Show println! output
cargo test -- --nocapture

# Show test names as they run
cargo test -- --nocapture --test-threads=1
```

### Test Filtering

```bash
# Run only parser tests
cargo test parser::

# Run only type tests
cargo test types::

# Run only error tests
cargo test error
```

## Testing Patterns

### Basic Parse Test Template

```rust
#[test]
fn test_parse_feature() {
    let input = "x: 42";
    let limits = CompilerLimits::default();
    let tokens = Lexer::new(input, &limits).tokenize().unwrap();
    let mut parser = Parser::new(tokens, &limits);
    let ast = parser.parse().unwrap();

    // Verify AST structure
    assert_eq!(ast.nodes.len(), 2); // Program + VarDecl
    assert_eq!(ast.nodes[1].node_type, NodeType::VarDecl);
}
```

### Error Test Template

```rust
#[test]
fn test_parse_error_case() {
    let input = "x:"; // Missing value
    let limits = CompilerLimits::default();
    let tokens = Lexer::new(input, &limits).tokenize().unwrap();
    let mut parser = Parser::new(tokens, &limits);

    let result = parser.parse();
    assert!(result.is_err());

    // Optional: verify specific error type
    if let Err(ParseError::UnexpectedToken { expected, .. }) = result {
        assert!(expected.contains("expression"));
    }
}
```

### Recursion Limit Test Template

```rust
#[test]
fn test_recursion_limit() {
    let mut limits = CompilerLimits::default();
    limits.max_expression_depth = 5; // Small limit for testing

    // Create deeply nested expression
    let input = "x: not not not not not not true";
    let tokens = Lexer::new(input, &limits).tokenize().unwrap();
    let mut parser = Parser::new(tokens, &limits);

    let result = parser.parse();
    assert!(matches!(result, Err(ParseError::RecursionLimitExceeded { .. })));
}
```

### Tree Structure Validation Template

```rust
#[test]
fn test_ast_structure() {
    let input = "add: (x Number, y Number) Number { return x }";
    let ast = parse_to_ast(input);

    // Verify tree structure
    let func_node = &ast.nodes[1];
    assert_eq!(func_node.node_type, NodeType::FunctionDecl);

    // Check for parameter list child
    let param_list_idx = func_node.first_child.unwrap();
    assert_eq!(ast.nodes[param_list_idx].node_type, NodeType::ParamList);

    // Check parameters are siblings
    let first_param_idx = ast.nodes[param_list_idx].first_child.unwrap();
    assert_eq!(ast.nodes[first_param_idx].node_type, NodeType::Param);

    let second_param_idx = ast.nodes[first_param_idx].next_sibling.unwrap();
    assert_eq!(ast.nodes[second_param_idx].node_type, NodeType::Param);
}
```

## Test Naming Conventions

**Pattern:** `test_<component>_<feature>_<variant>`

**Examples:**
- `test_parse_variable_declaration`
- `test_parse_function_with_typed_parameters`
- `test_parse_union_type`
- `test_lex_hex_number_with_suffix`
- `test_error_unterminated_string`
- `test_limit_recursion_depth_exceeded`

**Guidelines:**
- Use descriptive names that explain what is tested
- Include the component being tested (parse, lex, etc.)
- Include the feature being tested
- Include variant if testing specific case
- Use snake_case for test names

## Test Data Organization

### Small Inline Test Data

For simple cases, use string literals directly in tests:

```rust
#[test]
fn test_simple_case() {
    let input = "x: 42";
    // ...
}
```

### Complex Test Data

For complex test cases, use multi-line strings:

```rust
#[test]
fn test_complex_type() {
    let input = r#"
        type Person: {
            name String
            age Number
            greet: () String
        }
    "#;
    // ...
}
```

### Shared Test Data

For data used across multiple tests, define constants:

```rust
#[cfg(test)]
mod tests {
    const SAMPLE_FUNCTION: &str = "add: (x Number, y Number) Number { return x }";

    #[test]
    fn test_function_parsing() {
        let ast = parse_to_ast(SAMPLE_FUNCTION);
        // ...
    }

    #[test]
    fn test_function_parameters() {
        let ast = parse_to_ast(SAMPLE_FUNCTION);
        // ...
    }
}
```

## Planned Test Improvements

### Short-term
1. Add more error recovery tests
2. Test all error message formatting
3. Add performance benchmarks
4. Test memory usage under limits

### Medium-term
1. Integration tests for complete programs
2. Property-based testing (using proptest)
3. Fuzzing tests for parser robustness
4. Code coverage measurement

### Long-term
1. Regression test suite
2. Performance regression tracking
3. Cross-platform testing
4. Compiler correctness tests (compare output with spec)

## Test Quality Standards

### Every Test Should:
1. Have a clear, descriptive name
2. Test one specific thing
3. Be independent (not rely on other tests)
4. Be deterministic (same result every time)
5. Run quickly (< 100ms for unit tests)
6. Have clear assertions with helpful failure messages

### Every Feature Should Have:
1. Happy path test (valid input)
2. Error case tests (invalid input)
3. Edge case tests (boundary conditions)
4. Regression tests (if bugs were found)

### Test Documentation:
1. Complex tests should have comments explaining intent
2. Test modules should have a module-level doc comment
3. Unusual test data should be explained
4. Expected failures should be documented with TODO

## Debugging Failed Tests

### View Test Output

```bash
# Show all output
cargo test -- --nocapture

# Run single test with output
cargo test test_name -- --nocapture

# Show test binary location
cargo test --no-run -v
```

### Debugging in Tests

```rust
#[test]
fn test_debug_example() {
    let ast = parse_to_ast(input);

    // Print AST for debugging
    dbg!(&ast);

    // Print specific nodes
    for (i, node) in ast.nodes.iter().enumerate() {
        println!("{}: {:?}", i, node.node_type);
    }

    // Assertions...
}
```

### Using Rust Debugger

```bash
# Build test binary
cargo test --no-run

# Run with debugger (example with gdb)
gdb target/debug/deps/suru_lang_rs-<hash>

# Set breakpoint and run
(gdb) break test_name
(gdb) run test_name
```

## Continuous Integration

**TODO:** CI/CD pipeline will run:
- `cargo fmt -- --check` (formatting)
- `cargo clippy` (linting)
- `cargo test` (all tests)
- `cargo build --release` (release build)

**Planned CI Jobs:**
- Run tests on every PR
- Run tests on multiple platforms (Linux, macOS, Windows)
- Measure code coverage
- Check for performance regressions

## Contributing Tests

When contributing code, always include tests:

1. Add unit tests in the same file as implementation
2. Follow existing test naming conventions
3. Test both success and error cases
4. Ensure all tests pass before submitting PR
5. Add comments for complex test logic

---

**See also:**
- [Common Patterns](patterns.md) - Code patterns including test patterns
- [Architecture](../docs/contributing/architecture.md) - Overall structure
- [Development Workflow](../docs/contributing/development.md) - Running tests
